//! Loadout — the caster's two gendered currents (active talent poles, cast
//! MODIFIERS not passive bumps: Sean 2026-08-24 "a feminine fireball vs a
//! masculine one") and the singer's six-slot bar (MMO hotbar, `1`..`6`).

use crate::content::talents::{FEMININE, MASCULINE};
use crate::magic_words::MAGIC_WORDS;
use crate::overlay::{Domain, Ledger, Mod, OverlayEntry, Scope};

/// One masculine gain row: (reach gained, noise added), permyriad factors.
const MASC_Q: [(i64, i64); 8] = [
    (2_500, 1_500), // Cleave
    (1_500, 1_000), // Forge
    (1_000, 500),   // Iron Will
    (3_000, 2_000), // Rend
    (500, 250),     // Bastion
    (1_500, 750),   // Forge Bond
    (2_000, 1_000), // Strike True
    (750, 250),     // Endure
];

/// One feminine grace row: (cost cut, reach softened), permyriad factors.
const FEM_Q: [(i64, i64); 8] = [
    (2_000, 500),   // Moon's Blessing
    (2_500, 750),   // Flow
    (1_500, 250),   // Tide Rise
    (1_000, 0),     // Attune
    (1_500, 500),   // Reflect
    (2_000, 750),   // Harvest
    (2_500, 1_000), // Soothe
    (3_000, 1_250), // Drift
];

/// The spoken clause a masculine pole adds to a cast.
const MASC_CLAUSE: [&str; 8] = [
    "the word lands split, striking twice",
    "the word holds its heat and hammers on",
    "the word refuses to bend on the way out",
    "the word tears at what it touches",
    "the word plants itself between you and the room",
    "the word grips the nearest steel as it goes",
    "the word goes where you look, exactly",
    "the word keeps walking after it should have died",
];

/// The spoken clause a feminine pole adds to a cast.
const FEM_CLAUSE: [&str; 8] = [
    "and it mends a little of what it passes",
    "it slips around what it cannot move",
    "it lifts what stands beside you",
    "and the room whispers back what it heard",
    "part of it turns and faces the way it came",
    "it draws a thread from the living ground",
    "it quiets what it touches instead of breaking it",
    "it passes so soft the room barely marks you",
];

/// The active gendered poles. Either may be empty; both together are the
/// currents — eight by eight, sixty-four ways to color the same word.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Currents {
    /// Index into [`MASCULINE`], the force pole.
    pub masculine: Option<u8>,
    /// Index into [`FEMININE`], the water pole.
    pub feminine: Option<u8>,
}

/// A cast after the currents colors it: the numbers moved, the clauses spoken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Colored {
    /// Cost after the poles, clamped to the sung floor and the whole room.
    pub cost_q: i64,
    /// Reach after the poles, never below nothing.
    pub reach_q: i64,
    /// The masculine clause, when that pole is set.
    pub masc_clause: Option<&'static str>,
    /// The feminine clause, when that pole is set.
    pub fem_clause: Option<&'static str>,
}

/// Color a cast's cost and reach through the currents. Masculine pushes reach
/// and adds noise; feminine cuts cost and softens reach. Order-free: each
/// pole multiplies its own factor once.
pub fn color(cost_q: i64, reach_q: i64, currents: Currents) -> Colored {
    let scale = 10_000i64;
    let mut cost = cost_q;
    let mut reach = reach_q;
    let masc_clause = currents.masculine.map(|i| {
        let (gain, noise) = MASC_Q[i as usize % MASC_Q.len()];
        reach = reach * (scale + gain) / scale;
        cost = cost * (scale + noise) / scale;
        MASC_CLAUSE[i as usize % MASC_CLAUSE.len()]
    });
    let fem_clause = currents.feminine.map(|i| {
        let (cut, soften) = FEM_Q[i as usize % FEM_Q.len()];
        cost = cost * (scale - cut) / scale;
        reach = reach * (scale - soften) / scale;
        FEM_CLAUSE[i as usize % FEM_CLAUSE.len()]
    });
    Colored {
        cost_q: cost.clamp(super::SUNG_FLOOR_Q, crate::magic::umwelt::AUTHORED_Q),
        reach_q: reach.max(0),
        masc_clause,
        fem_clause,
    }
}

/// Find a talent by spoken name (case-insensitive): `(is_masculine, index)`.
pub fn talent_by_name(name: &str) -> Option<(bool, u8)> {
    let want = name.trim().to_lowercase();
    for (i, (n, _)) in MASCULINE.iter().enumerate() {
        if n.to_lowercase() == want {
            return Some((true, i as u8));
        }
    }
    for (i, (n, _)) in FEMININE.iter().enumerate() {
        if n.to_lowercase() == want {
            return Some((false, i as u8));
        }
    }
    None
}

/// Ledger keys: the currents rides `Domain::Talent` 0 (masc) / 1 (fem);
/// the singer's bar rides `Domain::Action` keys 8..=13 — the action bar's
/// own slots hold 0..=4, so the two never collide.
const KEY_MASC: u16 = 0;
const KEY_FEM: u16 = 1;
const SUNG_KEY_BASE: u16 = 8;
/// Six slots — `1`..`6` at the keyboard, MMO-trope.
pub const SUNG_SLOTS: usize = 6;

fn put(ledger: &mut Ledger, domain: Domain, key: u16, value: i64) {
    ledger.append(OverlayEntry {
        domain,
        key,
        modification: Mod::Add(value),
        priority: 100,
        scope: Scope::Operator,
    });
}

/// Persist the currents: `index + 1`, 0 is honestly empty.
pub fn save_currents(currents: &Currents, ledger: &mut Ledger) {
    put(ledger, Domain::Talent, KEY_MASC, currents.masculine.map_or(0, |i| i as i64 + 1));
    put(ledger, Domain::Talent, KEY_FEM, currents.feminine.map_or(0, |i| i as i64 + 1));
}

/// Read the currents back; anything out of range is honestly empty.
pub fn load_currents(ledger: &Ledger, seed: u64) -> Currents {
    let read = |key: u16, len: usize| -> Option<u8> {
        match ledger.resolve_i64(Domain::Talent, key, seed, 0) {
            v if v >= 1 && v <= len as i64 => Some((v - 1) as u8),
            _ => None,
        }
    };
    Currents {
        masculine: read(KEY_MASC, MASCULINE.len()),
        feminine: read(KEY_FEM, FEMININE.len()),
    }
}

/// The singer's bar: six slots, each an index into [`MAGIC_WORDS`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SungBar {
    /// The six slots; `None` is empty.
    pub slots: [Option<u8>; SUNG_SLOTS],
}

impl SungBar {
    /// Bind `word_idx` into `slot` (0..=5). Any other slot is refused.
    pub fn bind(&mut self, slot: usize, word_idx: u8) -> Result<(), &'static str> {
        if word_idx as usize >= MAGIC_WORDS.len() {
            return Err("no such word in the canon");
        }
        match self.slots.get_mut(slot) {
            Some(s) => {
                *s = Some(word_idx);
                Ok(())
            }
            None => Err("only six slots hang at the belt"),
        }
    }

    /// The word bound at `slot`, if any.
    pub fn word(&self, slot: usize) -> Option<&'static str> {
        self.slots.get(slot).copied().flatten().map(|i| MAGIC_WORDS[i as usize].0)
    }
}

/// Persist the bar: `index + 1` per slot, 0 empty.
pub fn save_sung_bar(bar: &SungBar, ledger: &mut Ledger) {
    for (slot, w) in bar.slots.iter().enumerate() {
        put(ledger, Domain::Action, SUNG_KEY_BASE + slot as u16, w.map_or(0, |i| i as i64 + 1));
    }
}

/// Read the bar back; anything outside the canon is honestly empty.
pub fn load_sung_bar(ledger: &Ledger, seed: u64) -> SungBar {
    let mut bar = SungBar::default();
    for (slot, s) in bar.slots.iter_mut().enumerate() {
        let v = ledger.resolve_i64(Domain::Action, SUNG_KEY_BASE + slot as u16, seed, 0);
        *s = match v {
            _ if v >= 1 && v <= MAGIC_WORDS.len() as i64 => Some((v - 1) as u8),
            _ => None,
        };
    }
    bar
}

/// The canon index of a word, for binding.
pub fn word_index(word: &str) -> Option<u8> {
    MAGIC_WORDS.iter().position(|&(w, _)| w == word).map(|i| i as u8)
}

/// The outer ring — the eleven callings (Sean 2026-08-24). `true` = a live
/// verb answers it today; `false` = the ring holds its place, distant.
pub const CALLINGS: [(&str, bool); 11] = [
    ("hunting", false),
    ("fishing", true),
    ("crafting", true),
    ("diplomacy", true),
    ("trade", false),
    ("magic", true),
    ("combat", true),
    ("sound", true),
    ("artists", false),
    ("church", false),
    ("state", false),
];

/// The talent mandala, spoken (Sean 2026-08-24: "a mandala shaped talent
/// tree"): the birth school at the heart, the seven arts as spokes, force
/// rising on the east, water on the west, the callings as the outer ring.
/// `*` marks an active pole; `·` a pole that could be taken.
pub fn render_mandala(
    school: &str,
    arts: &[(&str, &str); 7],
    currents: Currents,
) -> String {
    let mut out = String::from("        force                the mandala                 water\r\n");
    for row in 0..8 {
        let masc_mark = if currents.masculine == Some(row as u8) { '*' } else { '\u{b7}' };
        let fem_mark = if currents.feminine == Some(row as u8) { '*' } else { '\u{b7}' };
        let masc = format!("{} {}", masc_mark, MASCULINE[row].0);
        let fem = format!("{} {}", fem_mark, FEMININE[row].0);
        let center = if row == 0 {
            format!("( {school} )")
        } else {
            let (name, rank) = arts[row - 1];
            format!("{name} \u{2014} {rank}")
        };
        out.push_str(&format!("{masc:<18}{center:^28}{fem}\r\n"));
    }
    out.push_str("the outer ring \u{2014} ");
    for (i, (name, live)) in CALLINGS.iter().enumerate() {
        if i > 0 {
            out.push_str(" \u{b7} ");
        }
        if *live {
            out.push_str(name);
        } else {
            out.push_str(&format!("\x1b[2m{name}\x1b[0m"));
        }
    }
    out.push_str("\r\nspeak `talent <name>` to take a pole; one of each current holds at once.");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_feminine_fireball_is_not_a_masculine_one() {
        let (cost, reach) = (5_000, 4_000);
        let masc = color(cost, reach, Currents { masculine: Some(3), feminine: None });
        let fem = color(cost, reach, Currents { masculine: None, feminine: Some(7) });
        assert!(masc.reach_q > reach, "the force pole pushes reach");
        assert!(masc.cost_q > cost, "and pays for it in noise");
        assert!(fem.cost_q < cost, "the water pole sings quieter");
        assert!(fem.reach_q < reach, "and reaches softer");
        assert_ne!(masc, fem, "the two poles are different casts");
    }

    #[test]
    fn every_pole_pair_is_a_distinct_currents() {
        let mut seen = std::collections::HashSet::new();
        for m in 0..MASCULINE.len() as u8 {
            for f in 0..FEMININE.len() as u8 {
                let c = color(5_000, 4_000, Currents { masculine: Some(m), feminine: Some(f) });
                seen.insert((c.cost_q, c.reach_q, c.masc_clause, c.fem_clause));
            }
        }
        assert_eq!(seen.len(), MASCULINE.len() * FEMININE.len(), "sixty-four currentss");
    }

    #[test]
    fn coloring_never_breaks_the_floor_or_the_room() {
        for m in 0..MASCULINE.len() as u8 {
            for f in 0..FEMININE.len() as u8 {
                for (cost, reach) in [(super::super::SUNG_FLOOR_Q, 0), (10_000, 10_000)] {
                    let c = color(cost, reach, Currents { masculine: Some(m), feminine: Some(f) });
                    assert!(c.cost_q >= super::super::SUNG_FLOOR_Q, "no currents sings free");
                    assert!(c.cost_q <= crate::magic::umwelt::AUTHORED_Q);
                    assert!(c.reach_q >= 0);
                }
            }
        }
    }

    #[test]
    fn currents_and_bar_ride_the_ledger_round_trip() {
        let seed = 7u64;
        let mut ledger = Ledger::default();
        let currents = Currents { masculine: Some(2), feminine: Some(5) };
        save_currents(&currents, &mut ledger);
        assert_eq!(load_currents(&ledger, seed), currents);

        let mut bar = SungBar::default();
        bar.bind(0, word_index("ash").unwrap()).unwrap();
        bar.bind(5, word_index("bell").unwrap()).unwrap();
        assert!(bar.bind(6, 0).is_err(), "only six slots");
        save_sung_bar(&bar, &mut ledger);
        let back = load_sung_bar(&ledger, seed);
        assert_eq!(back, bar);
        assert_eq!(back.word(0), Some("ash"));
        assert_eq!(back.word(5), Some("bell"));
        assert_eq!(back.word(3), None);
    }

    #[test]
    fn talent_names_resolve_case_blind_and_strangers_refuse() {
        assert_eq!(talent_by_name("cleave"), Some((true, 0)));
        assert_eq!(talent_by_name("DRIFT"), Some((false, 7)));
        assert_eq!(talent_by_name("fireball"), None);
    }
}
