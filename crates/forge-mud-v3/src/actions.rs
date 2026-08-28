//! Actions — the trit-composed action bar (ARCH000 2026-08-11: "swapable,
//! 3-5 actions max... a unique way to mix and match with trits"). An action
//! is not a listed verb, it is a 5-trit WORD: 3^5 = 243 castable acts from
//! pure composition of five independent axes (the house's own atom shape,
//! `forge_core_v3::atom::TritCell5D` — radix-3 idea copied, not the type;
//! this module's word is its own plain `u8` 0..=242 with its own codec).
//!
//! The five axes, each a trit 0..=2:
//! `SOURCE` (flame|frost|spirit), `FORM` (bolt|wave|ward),
//! `INTENT` (harm|mend|reveal), `REACH` (self|touch|far),
//! `ECHO` (once|linger|bind).

use crate::overlay::{self, Domain, Ledger, Mod, Scope};

/// One composed action: a plain radix-3 packing of five trits, 0..=242.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionWord(pub u8);

const SOURCE: [&str; 3] = ["flame", "frost", "spirit"];
const FORM: [&str; 3] = ["bolt", "wave", "ward"];
const INTENT_VERB: [&str; 3] = ["harms", "mends", "reveals"];
const REACH: [&str; 3] = ["self", "touch", "far"];
const ECHO: [&str; 3] = ["once", "linger", "bind"];

/// Pack five trits (each 0..=2) into a word. Any trit above 2 is refused
/// (L07: the inverse of [`ActionWord::trits`] over every accepted word, exactly).
pub fn compose(trits: [u8; 5]) -> Option<ActionWord> {
    if trits.iter().any(|&t| t > 2) {
        return None;
    }
    let v = trits[0] + trits[1] * 3 + trits[2] * 9 + trits[3] * 27 + trits[4] * 81;
    Some(ActionWord(v))
}

impl ActionWord {
    /// Unpack this word back into its five trits, radix-3.
    pub fn trits(self) -> [u8; 5] {
        let mut v = self.0;
        let mut out = [0u8; 5];
        for slot in out.iter_mut() {
            *slot = v % 3;
            v /= 3;
        }
        out
    }
}

/// The composed spoken name for a word — deterministic, no digits, all 243
/// distinct (each axis's words are pairwise distinct within that axis, and
/// the format below embeds all five verbatim, so the whole is injective).
pub fn speak(word: ActionWord) -> String {
    let [source, form, intent, reach, echo] = word.trits();
    format!(
        "{} {} that {}, {}, {}",
        SOURCE[source as usize],
        FORM[form as usize],
        INTENT_VERB[intent as usize],
        REACH[reach as usize],
        ECHO[echo as usize]
    )
}

/// SOURCE x INTENT -> governing art index into `crate::skills::ARTS`
/// (0=Hunt, 1=Veil, 2=Craft, 3=Current, 4=Rust, 5=Parley, 6=Vigil).
/// `[ASSUMED]` honest mapping built from the ruling's two named anchors
/// (flame+harm -> Hunt, spirit+mend -> Vigil) and reveal always riding the
/// Craft ("wisdom's art"); the remaining six cells are this module's own
/// documented call, not a re-derivation of an existing table (L05: no
/// second home for this mapping exists elsewhere).
const GOVERNING_ART: [[usize; 3]; 3] = [
    // harm      mend      reveal
    [0, 4, 2], // flame
    [1, 3, 2], // frost
    [5, 6, 2], // spirit
];

/// The art SOURCE+INTENT ride, for every composed word.
pub fn governing_art(word: ActionWord) -> usize {
    let [source, _form, intent, _reach, _echo] = word.trits();
    GOVERNING_ART[source as usize][intent as usize]
}

/// The spoken strength ladder for a cast, from the governing art's standing
/// (0..=1000, `crate::skills::SKILL_MAX`). No digits.
pub fn potency_word(art_value: u16) -> &'static str {
    match art_value {
        0..=199 => "faint",
        200..=399 => "steady",
        400..=599 => "strong",
        600..=799 => "fierce",
        _ => "masterful",
    }
}

/// The swappable action bar: 5 slots, 3-5 in use is the ruled shape, 5 is
/// the bar's whole size.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bar {
    /// The five swappable slots; `None` is empty.
    pub slots: [Option<ActionWord>; 5],
}

impl Bar {
    /// Equip `w` into `slot` (0..=4). Any other slot is refused.
    pub fn equip(&mut self, slot: usize, w: ActionWord) -> Result<(), &'static str> {
        match self.slots.get_mut(slot) {
            Some(s) => {
                *s = Some(w);
                Ok(())
            }
            None => Err("slot out of range (0..=4)"),
        }
    }

    /// Clear `slot`, if it exists.
    pub fn clear(&mut self, slot: usize) {
        if let Some(s) = self.slots.get_mut(slot) {
            *s = None;
        }
    }

    /// What `slot` currently holds, if anything.
    pub fn cast(&self, slot: usize) -> Option<ActionWord> {
        self.slots.get(slot).copied().flatten()
    }
}

/// Persist a bar: one `Operator`-scope, `Domain::Action` entry per slot,
/// `Mod::Add(word + 1)` (0 stays reserved for empty), priority 100. The
/// ledger is append-only (L10's wire): a re-equip appends a fresh entry
/// rather than editing the old one, and later-entry-wins resolution (see
/// `Ledger::resolve_i64`) makes the newest equip the one that reads back —
/// a long-lived bar's ledger accretes its whole equip history, which is the
/// ledger's nature, not a leak.
pub fn save_bar(bar: &Bar, ledger: &mut Ledger) {
    for (slot, w) in bar.slots.iter().enumerate() {
        let value = match w {
            Some(word) => word.0 as i64 + 1,
            None => 0,
        };
        ledger.append(overlay::OverlayEntry {
            domain: Domain::Action,
            key: slot as u16,
            modification: Mod::Add(value),
            priority: 100,
            scope: Scope::Operator,
        });
    }
}

/// Read a bar back from the ledger. 0 (or anything out of `1..=243`) is
/// honestly empty rather than a guessed word.
pub fn load_bar(ledger: &Ledger, seed: u64) -> Bar {
    let mut bar = Bar::default();
    for (slot, s) in bar.slots.iter_mut().enumerate() {
        let v = ledger.resolve_i64(Domain::Action, slot as u16, seed, 0);
        *s = match v {
            1..=243 => Some(ActionWord((v - 1) as u8)),
            _ => None,
        };
    }
    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trit_bijection_over_all_243_and_bad_trit_refused() {
        for v in 0u8..=242 {
            let w = ActionWord(v);
            let t = w.trits();
            assert!(t.iter().all(|&x| x <= 2));
            assert_eq!(compose(t), Some(w));
        }
        assert_eq!(compose([3, 0, 0, 0, 0]), None, "trit 3 is out of range");
    }

    #[test]
    fn all_243_spoken_names_are_distinct_and_digit_free() {
        let names: std::collections::HashSet<String> =
            (0u8..=242).map(|v| speak(ActionWord(v))).collect();
        assert_eq!(names.len(), 243);
        assert!(names.iter().all(|n| !n.chars().any(|c| c.is_ascii_digit())));
    }

    #[test]
    fn governing_art_is_total_in_range_and_deterministic() {
        for v in 0u8..=242 {
            let w = ActionWord(v);
            let a = governing_art(w);
            assert!(a <= 6);
            assert_eq!(a, governing_art(w), "deterministic");
        }
    }

    #[test]
    fn bar_equip_clear_cast_and_slot_five_refused() {
        let mut bar = Bar::default();
        let w = ActionWord(7);
        assert!(bar.equip(0, w).is_ok());
        assert_eq!(bar.cast(0), Some(w));
        bar.clear(0);
        assert_eq!(bar.cast(0), None);
        assert!(bar.equip(5, w).is_err(), "only slots 0..=4 exist");
        assert!(bar.cast(5).is_none());
    }

    #[test]
    fn save_and_load_bar_round_trip_and_reequip_reads_newest() {
        let seed = 42u64;
        let mut bar = Bar::default();
        bar.equip(0, ActionWord(3)).unwrap();
        bar.equip(1, ActionWord(200)).unwrap();

        let mut ledger = Ledger::default();
        save_bar(&bar, &mut ledger);
        let loaded = load_bar(&ledger, seed);
        assert_eq!(loaded, bar);

        // Re-equip slot 0 to a new word and save again: later entry wins.
        bar.equip(0, ActionWord(9)).unwrap();
        save_bar(&bar, &mut ledger);
        let reloaded = load_bar(&ledger, seed);
        assert_eq!(reloaded.cast(0), Some(ActionWord(9)));
        assert_eq!(reloaded.cast(1), Some(ActionWord(200)));
    }

    #[test]
    fn potency_word_is_total_over_0_to_1000() {
        for v in 0u16..=1000 {
            let word = potency_word(v);
            assert!(["faint", "steady", "strong", "fierce", "masterful"].contains(&word));
        }
    }
}
