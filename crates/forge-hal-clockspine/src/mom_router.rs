//! MoM router — a thin musical shim over [`crate::expert_pool::MoeRouter`].
//!
//! Ported from `F:\NewRepo\crates\nde_core\src\mom_router.rs` (2026-08-14,
//! Sean "pick one and port it" — closing the `nde_core::mom_router` gap
//! `expert_pool.rs`'s own landing note named as future work). 7 musician
//! families x 7 sub-families = 49 chord positions, matching the Invention
//! #169 HierarchicalMoE 7x7 topology (`forge-core-v3::hierarchical_moe`).
//! The [`forge_core_v3::ump_word::UmpWord`] is the 16-byte query; routing is
//! integer XOR + POPCNT (sub-us, no float, no alloc) via the already-landed,
//! width-generic `expert_pool::MoeRouter<49, 16, u8>`.
//!
//! Scope cut (C09/L15, named not silent): the v2 source's `Musician`
//! trait (`nde_core::musician`) drags in `forge_harmonics::BardAura`,
//! `forge_core::essence_registry::EssenceAtom`, `forge_core::scene::
//! MaterialId` — none of which exist in `F:\v3`. Only `NUM_MUSICIAN_
//! FAMILIES` (a bare `usize` constant) crosses; the trait itself, and the
//! `RoutingTag`/`RoutedUmp`/`Gravebell`/`MomBus` render-side test the v2
//! source's own test module included, do NOT. This ports the ROUTER, not
//! the orchestra that would eventually sit behind it.

use crate::expert_pool::MoeRouter;
use forge_core_v3::ump_word::UmpWord;

/// Number of musician families — matches the 7x7 cell grid this router
/// wraps. In v2: `nde_core::musician::NUM_MUSICIAN_FAMILIES`, kept equal to
/// `forge_ml::bq_router::NUM_SPECIALISTS` there; that BqRouter tie is not
/// re-verified here (`forge-ml-bqrouter` is a separate, already-landed v3
/// crate — cross-checking the two constants match is a future brick, not
/// assumed true by this one).
pub const NUM_MUSICIAN_FAMILIES: usize = 7;

/// Total MoM routing cells — 7x7 = 49.
pub const N_MOM_CELLS: usize = NUM_MUSICIAN_FAMILIES * NUM_MUSICIAN_FAMILIES;

/// The UMP word width this router queries at — pinned equal to
/// `forge_core_v3::ump_word::UMP_WORD_BYTES`. A cross-crate test below pins
/// the equality where both constants are visible (mirrors v2's own
/// `nde_core::ump` test, which existed for the identical reason: forge-core
/// cannot see forge-hal, so the pin lives one layer up).
const MOM_QUERY_BYTES: usize = 16;

/// Payload stored in each cell: the musician slot index to dispatch to.
pub type MomCell = u8;

/// 49-cell MoM router. Wraps `MoeRouter<49, 16, u8>` for audio musician dispatch.
pub struct MomRouter {
    inner: MoeRouter<N_MOM_CELLS, MOM_QUERY_BYTES, MomCell>,
}

impl Default for MomRouter {
    fn default() -> Self {
        Self::empty()
    }
}

impl MomRouter {
    /// All cells inactive — every `route` returns `None` until trained.
    pub fn empty() -> Self {
        Self { inner: MoeRouter::empty() }
    }

    /// Train one cell. `family * NUM_MUSICIAN_FAMILIES + sub` is the
    /// canonical cell index; `slot` is the musician to dispatch when this
    /// cell wins.
    pub fn train(&mut self, family: usize, sub: usize, centroid: UmpWord, slot: MomCell) {
        let idx = family * NUM_MUSICIAN_FAMILIES + sub;
        self.inner.train_cell(idx, centroid.0, slot);
    }

    /// Route a UMP word to the nearest trained musician slot. Returns
    /// `None` if no cells are trained.
    pub fn route(&mut self, word: &UmpWord) -> Option<MomCell> {
        self.inner.route(&word.0)
    }

    /// Number of trained cells.
    pub fn active_count(&self) -> usize {
        self.inner.active_count()
    }

    /// Route a word AND voice the winning cell in one call — word -> route()
    /// -> (MIDI note, velocity). `None` when no cell is trained.
    pub fn voice(&mut self, word: &UmpWord) -> Option<(u8, u8)> {
        self.route(word).map(cell_voice)
    }

    /// Seed centroids for the three DAPS event families (v2 session
    /// 2026-07-03 SS D.2). `slot_of(family, sub)` maps each trained cell to
    /// the musician slot that fires when it wins — slot assignment is the
    /// app layer's call, the geometry is canonical.
    pub fn seed_event_families(&mut self, slot_of: impl Fn(usize, usize) -> MomCell) {
        const CST_TIERS: [(u8, u8, u8); 3] = [
            (2, 50, 2), // hearth: file drone + definitions (C3 region)
            (5, 64, 1), // reed: structure + references (E4 region)
            (7, 79, 0), // glass: values + logic (G5 region)
        ];
        for (sub, &(material, note, voice)) in CST_TIERS.iter().enumerate() {
            let centroid = UmpWord::from_node_voice(material, note, voice);
            self.train(EVENT_FAMILY_CST, sub, centroid, slot_of(EVENT_FAMILY_CST, sub));
        }

        for kind in 0..NUM_MUSICIAN_FAMILIES as u8 {
            let centroid = UmpWord::from_physics_event(kind, 0, 5_000, 700);
            self.train(EVENT_FAMILY_PHYSICS, kind as usize, centroid, slot_of(EVENT_FAMILY_PHYSICS, kind as usize));
        }

        for sev in 0..3u8 {
            let centroid = UmpWord::from_roadie_event(sev, 0);
            self.train(EVENT_FAMILY_ROADIE, sev as usize, centroid, slot_of(EVENT_FAMILY_ROADIE, sev as usize));
        }

        for sid in 0..NUM_MUSICIAN_FAMILIES as u8 {
            let centroid = UmpWord::from_pty_route(sid, PTY_ANCHOR_MARGIN);
            self.train(EVENT_FAMILY_PTY, sid as usize, centroid, slot_of(EVENT_FAMILY_PTY, sid as usize));
        }
    }
}

/// Canonical router-family row for CST syllabic voices.
pub const EVENT_FAMILY_CST: usize = 0;
/// Canonical router-family row for physics events.
pub const EVENT_FAMILY_PHYSICS: usize = 1;
/// Canonical router-family row for mixer health.
pub const EVENT_FAMILY_ROADIE: usize = 2;
/// Canonical router-family row for BqRouter PTY route decisions. Rows 4..=5
/// stay free (v2 reserved row 6 for `Gravebell::family`, not ported here).
pub const EVENT_FAMILY_PTY: usize = 3;

/// Family-neutral confidence anchor for the PTY row's seeded centroids.
const PTY_ANCHOR_MARGIN: u32 = 150;

/// Pentatonic sub-cell note offsets.
pub const NOTE_OFFSETS: [u8; NUM_MUSICIAN_FAMILIES] = [0, 2, 4, 7, 9, 11, 12];

/// Canonical `MomCell -> (MIDI note, velocity)`. Family row picks the
/// octave band, sub walks the pentatonic offsets. Integer, deterministic,
/// saturating (never panics on a stray slot).
pub fn cell_voice(cell: MomCell) -> (u8, u8) {
    let family = cell as usize / NUM_MUSICIAN_FAMILIES;
    let sub = cell as usize % NUM_MUSICIAN_FAMILIES;
    let note = 36u8.saturating_add((family as u8).saturating_mul(12)).saturating_add(NOTE_OFFSETS[sub]);
    let velocity = [70u8, 96, 50, 80].get(family).copied().unwrap_or(70);
    (note, velocity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mom_query_width_matches_the_floor_ump_word() {
        assert_eq!(MOM_QUERY_BYTES, forge_core_v3::ump_word::UMP_WORD_BYTES);
        assert_eq!(std::mem::size_of::<UmpWord>(), MOM_QUERY_BYTES);
    }

    #[test]
    fn empty_router_routes_nothing() {
        let mut r = MomRouter::empty();
        assert_eq!(r.active_count(), 0);
        assert_eq!(r.route(&UmpWord([0; 16])), None);
    }

    #[test]
    fn trained_cell_routes_to_slot() {
        let mut r = MomRouter::empty();
        let centroid = UmpWord([0xAA; 16]);
        r.train(2, 3, centroid, 11);
        assert_eq!(r.active_count(), 1);
        assert_eq!(r.route(&UmpWord([0xAA; 16])), Some(11));
    }

    #[test]
    fn forty_nine_cells_available() {
        assert_eq!(N_MOM_CELLS, 49);
    }

    fn seeded() -> MomRouter {
        let mut r = MomRouter::empty();
        r.seed_event_families(|family, sub| (family * NUM_MUSICIAN_FAMILIES + sub) as MomCell);
        r
    }

    #[test]
    fn seeding_trains_every_event_family_row() {
        assert_eq!(seeded().active_count(), 3 + 7 + 3 + 7);
    }

    #[test]
    fn pty_route_words_land_in_their_own_row_and_sid_picks_the_sub_cell() {
        let mut r = seeded();
        for sid in 0..NUM_MUSICIAN_FAMILIES as u8 {
            let expected = (EVENT_FAMILY_PTY * NUM_MUSICIAN_FAMILIES) as u8 + sid;
            assert_eq!(r.route(&UmpWord::from_pty_route(sid, 150)), Some(expected), "sid {sid}");
            for margin in [0u32, 49, 900, 5_000] {
                let slot = r.route(&UmpWord::from_pty_route(sid, margin)).expect("PTY word routes");
                assert_eq!(
                    slot as usize / NUM_MUSICIAN_FAMILIES,
                    EVENT_FAMILY_PTY,
                    "sid {sid} margin {margin} strayed out of the PTY row: slot {slot}"
                );
            }
        }
    }

    #[test]
    fn pty_row_voices_a_playable_strike_distinct_from_the_other_families() {
        let mut r = seeded();
        let (note, vel) = r.voice(&UmpWord::from_pty_route(3, 150)).expect("seeded router voices a PTY route");
        assert!((12..=127).contains(&note), "note in MIDI range: {note}");
        let (_, cst_vel) = cell_voice((EVENT_FAMILY_CST * NUM_MUSICIAN_FAMILIES) as u8);
        assert_ne!(vel, cst_vel, "the terminal's strike is not the CST voice");
    }

    #[test]
    fn event_families_route_to_their_own_row_no_strays() {
        let mut r = seeded();
        let words: [(usize, UmpWord); 9] = [
            (EVENT_FAMILY_CST, UmpWord::from_node_voice(0, 48, 2)),
            (EVENT_FAMILY_CST, UmpWord::from_node_voice(4, 62, 1)),
            (EVENT_FAMILY_CST, UmpWord::from_node_voice(9, 84, 0)),
            (EVENT_FAMILY_PHYSICS, UmpWord::from_physics_event(0, 0xA5A5, 200, 90)),
            (EVENT_FAMILY_PHYSICS, UmpWord::from_physics_event(2, 0xDEAD_BEEF, 9_000, 440)),
            (EVENT_FAMILY_PHYSICS, UmpWord::from_physics_event(6, 42, 5_000, 8_000)),
            (EVENT_FAMILY_ROADIE, UmpWord::from_roadie_event(0, 1)),
            (EVENT_FAMILY_ROADIE, UmpWord::from_roadie_event(1, 2)),
            (EVENT_FAMILY_ROADIE, UmpWord::from_roadie_event(2, 3)),
        ];
        for (family, word) in words {
            let slot = r.route(&word).expect("seeded router must route");
            assert_eq!(slot as usize / NUM_MUSICIAN_FAMILIES, family, "word strayed out of family row {family}: slot {slot}");
        }
    }

    #[test]
    fn physics_kind_selects_its_sub_cell() {
        let mut r = seeded();
        for kind in 0..NUM_MUSICIAN_FAMILIES as u8 {
            let word = UmpWord::from_physics_event(kind, 0x1234_5678, 5_000, 700);
            let expected = (EVENT_FAMILY_PHYSICS * NUM_MUSICIAN_FAMILIES) as u8 + kind;
            assert_eq!(r.route(&word), Some(expected), "kind {kind}");
        }
    }

    #[test]
    fn cst_voice_tier_selects_its_sub_cell() {
        let mut r = seeded();
        for (sub, word) in [
            UmpWord::from_node_voice(2, 48, 2),
            UmpWord::from_node_voice(5, 62, 1),
            UmpWord::from_node_voice(7, 81, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let expected = (EVENT_FAMILY_CST * NUM_MUSICIAN_FAMILIES + sub) as u8;
            assert_eq!(r.route(&word), Some(expected), "tier {sub}");
        }
    }

    #[test]
    fn voice_maps_routed_cell_to_a_playable_note() {
        let mut r = seeded();
        let word = UmpWord::from_physics_event(2, 0, 5_000, 700);
        let (note, vel) = r.voice(&word).expect("seeded router voices a routed event");
        assert!((12..=127).contains(&note), "note in MIDI range: {note}");
        assert!(vel > 0, "velocity audible");
        assert_eq!(MomRouter::empty().voice(&word), None, "untrained router voices nothing");
    }

    #[test]
    fn cell_voice_family_row_raises_the_octave() {
        let (n0, _) = cell_voice(0);
        let (n7, _) = cell_voice(7);
        assert_eq!(cell_voice(0), cell_voice(0), "deterministic");
        assert_eq!(n7, n0 + 12, "family row = one octave up");
    }

    // NOT PORTED (L15, named): v2's `distinct_events_route_and_two_voices_
    // sum_into_one_block` test — needs `Gravebell`/`MomBus`/`Musician`/
    // `RoutedUmp`/`RoutingTag`, none of which exist in F:\v3. This router
    // proves routing; the orchestra behind it is a separate, larger brick.
}
