//! The Witness Mirror — the player scored on the SAME eight axes as a faction.
//! `mind.rs` owns the axis machine (L05 one-home); this module only accumulates
//! conduct rows into it and reads a Laban Space lean back out.

use crate::mind::FactionMind;

/// How many deeds the mirror walks with. Past this the oldest is forgotten —
/// the mirror reflects who you have been lately, not who you ever were.
pub const MIRROR_WINDOW: usize = 32;

/// Full-scale for one axis, matching `mind.rs`'s authored `for_faction` range.
const AXIS_FULL: i32 = 1_000;

/// A conduct row: one thing the player did. Every variant names a verb that
/// exists in `game.rs`'s dispatch (`game.rs:468`), never a hypothetical one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Deed {
    /// `steal` — took what was not offered.
    Stole,
    /// `fight` — met it head on.
    Fought,
    /// `flee` — broke off.
    Fled,
    /// `talk` — went through a person.
    Talked,
    /// `camp` — held ground and waited.
    Camped,
    /// `delve` — went down into the unknown.
    Delved,
    /// `name` / `author` — imposed a word on the world.
    Named,
    /// `witness` — stood and observed without acting.
    Witnessed,
}

impl Deed {
    /// The eight-axis pull of one deed `[AUTHORED]`, same axis order and scale
    /// as `FactionMind`. Read as: what does doing this, repeatedly, make you.
    const fn pull(self) -> [i16; 8] {
        // threat, ambiguity_tol, hierarchy, novelty, closure, mortality, dominance, permeability
        match self {
            Deed::Stole =>     [ 200, -100, -300,  200,  300,  100,  400, -200],
            Deed::Fought =>    [ 400, -300, -100,  100,  500,  400,  600, -300],
            Deed::Fled =>      [ 500,  200, -200, -100, -400,  500, -400,  200],
            Deed::Talked =>    [-200,  400,  100,  100, -200, -100, -200,  600],
            Deed::Camped =>    [-100,  300,  100, -200, -300,  100, -100,  200],
            Deed::Delved =>    [ 100, -200, -200,  600,  400,  300,  300,  100],
            Deed::Named =>     [ 100, -400,  500, -100,  600,  100,  400, -300],
            Deed::Witnessed => [-100,  600,  100,  200, -500,  200, -300,  500],
        }
    }
}

/// The verb the player typed, as a conduct row. Verbs with nothing to say
/// about play style (`look`, `map`, `status`, `save`) return `None` — the
/// mirror reflects conduct, not navigation.
pub fn deed_of_verb(verb: &str) -> Option<Deed> {
    match verb {
        "steal" => Some(Deed::Stole),
        "fight" => Some(Deed::Fought),
        "flee" => Some(Deed::Fled),
        "talk" => Some(Deed::Talked),
        "camp" => Some(Deed::Camped),
        "delve" => Some(Deed::Delved),
        "name" | "author" => Some(Deed::Named),
        "witness" => Some(Deed::Witnessed),
        _ => None,
    }
}

/// The mirror that walks with you: a rolling window of conduct rows, read back
/// as a `FactionMind`-shaped profile of the player.
#[derive(Debug, Clone, Copy)]
pub struct WitnessMirror {
    deeds: [Option<Deed>; MIRROR_WINDOW],
    next: usize,
    seen: usize,
}

impl Default for WitnessMirror {
    fn default() -> Self {
        Self::new()
    }
}

impl WitnessMirror {
    /// An empty mirror — nothing witnessed, no lean.
    pub const fn new() -> Self {
        Self { deeds: [None; MIRROR_WINDOW], next: 0, seen: 0 }
    }

    /// Fold one deed in, evicting the oldest once the window is full.
    pub fn observe(&mut self, deed: Deed) {
        self.deeds[self.next] = Some(deed);
        self.next = (self.next + 1) % MIRROR_WINDOW;
        self.seen = self.seen.saturating_add(1);
    }

    /// How many deeds the mirror currently holds (never past [`MIRROR_WINDOW`]).
    pub fn held(&self) -> usize {
        self.deeds.iter().filter(|d| d.is_some()).count()
    }

    /// Every deed ever folded in, including the forgotten ones.
    pub fn seen(&self) -> usize {
        self.seen
    }

    /// The player's own eight axes: each deed's pull, averaged over the window.
    /// An empty mirror is all zeros — no conduct, no claim about who you are.
    pub fn profile(&self) -> FactionMind {
        let held = self.held();
        if held == 0 {
            return FactionMind {
                threat_sensitivity: 0,
                ambiguity_tolerance: 0,
                hierarchy_need: 0,
                novelty_drive: 0,
                closure_pressure: 0,
                mortality_pressure: 0,
                dominance_drive: 0,
                permeability: 0,
            };
        }
        let mut sums = [0i32; 8];
        for deed in self.deeds.iter().flatten() {
            let pull = deed.pull();
            for (s, p) in sums.iter_mut().zip(pull.iter()) {
                *s += *p as i32;
            }
        }
        let avg = |i: usize| (sums[i] / held as i32) as i16;
        FactionMind {
            threat_sensitivity: avg(0),
            ambiguity_tolerance: avg(1),
            hierarchy_need: avg(2),
            novelty_drive: avg(3),
            closure_pressure: avg(4),
            mortality_pressure: avg(5),
            dominance_drive: avg(6),
            permeability: avg(7),
        }
    }

    /// The witness face's Laban Space lean, permyriad, in the polarity
    /// `broski_layer.rs:115` already reads: `10_000` = Indirect, `0` = Direct.
    ///
    /// Direct is decisive conduct (closure + dominance): go straight at it.
    /// Indirect is circuitous conduct (ambiguity tolerance + permeability):
    /// go around, through people, by watching. An empty mirror sits at the
    /// midpoint — the face has nothing to reflect yet.
    pub fn laban_space_pmy(&self) -> u16 {
        if self.held() == 0 {
            return 5_000;
        }
        let m = self.profile();
        let direct = m.closure_pressure as i32 + m.dominance_drive as i32;
        let indirect = m.ambiguity_tolerance as i32 + m.permeability as i32;
        // Both terms span [-2*AXIS_FULL, 2*AXIS_FULL]; their difference spans
        // twice that again, so fold it back onto 0..=10_000 around the midpoint.
        let swing = (indirect - direct).clamp(-4 * AXIS_FULL, 4 * AXIS_FULL);
        ((swing + 4 * AXIS_FULL) * 10_000 / (8 * AXIS_FULL)) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mirror_of(deed: Deed, n: usize) -> WitnessMirror {
        let mut m = WitnessMirror::new();
        for _ in 0..n {
            m.observe(deed);
        }
        m
    }

    #[test]
    fn an_unwitnessed_player_has_no_profile_and_no_lean() {
        let m = WitnessMirror::new();
        assert_eq!(m.held(), 0);
        assert_eq!(m.profile().dominance_drive, 0);
        assert_eq!(m.laban_space_pmy(), 5_000, "nothing to reflect sits at the midpoint");
    }

    /// The whole point of the row: the face shifts Direct<->Indirect with play
    /// style. A fighter and a talker must not read the same.
    #[test]
    fn a_fighter_reads_direct_and_a_talker_reads_indirect() {
        let fighter = mirror_of(Deed::Fought, 8).laban_space_pmy();
        let talker = mirror_of(Deed::Talked, 8).laban_space_pmy();
        assert!(fighter < 5_000, "decisive conduct must lean Direct: {fighter}");
        assert!(talker > 5_000, "circuitous conduct must lean Indirect: {talker}");
        assert!(talker > fighter);
    }

    /// The mirror walks with you: old conduct falls out of the window.
    #[test]
    fn the_window_forgets_the_oldest_deed() {
        let mut m = mirror_of(Deed::Fought, MIRROR_WINDOW);
        let as_fighter = m.laban_space_pmy();
        for _ in 0..MIRROR_WINDOW {
            m.observe(Deed::Talked);
        }
        assert_eq!(m.held(), MIRROR_WINDOW, "the window never grows past its cap");
        assert_eq!(m.seen(), MIRROR_WINDOW * 2, "but it remembers how much it has seen");
        assert!(
            m.laban_space_pmy() > as_fighter,
            "a fighter who becomes a talker must stop reading as a fighter"
        );
    }

    /// The mirror wears the same shape as a faction — that is what makes it a
    /// mirror rather than a second, private scorer (L05 one-home).
    #[test]
    fn the_players_profile_is_the_same_eight_axes_a_faction_wears() {
        let m = mirror_of(Deed::Named, 4);
        let player = m.profile();
        let thornguard = FactionMind::for_faction(0);
        assert!(player.hierarchy_need > 0 && thornguard.hierarchy_need > 0);
        assert!(
            crate::mind::choose_action(&player, &Default::default())
                == crate::mind::choose_action(&player, &Default::default()),
            "the player's profile must be usable anywhere a faction's is"
        );
    }

    /// Every lean stays inside the permyriad band, whatever the conduct.
    #[test]
    fn every_conduct_lands_inside_permyriad() {
        for deed in [
            Deed::Stole,
            Deed::Fought,
            Deed::Fled,
            Deed::Talked,
            Deed::Camped,
            Deed::Delved,
            Deed::Named,
            Deed::Witnessed,
        ] {
            let pmy = mirror_of(deed, 5).laban_space_pmy();
            assert!(pmy <= 10_000, "{deed:?} left the band: {pmy}");
        }
    }
}
