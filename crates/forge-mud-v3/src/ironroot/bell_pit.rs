//! The Bell Pit — real content authored against
//! `F:\v3\TODO\ironroot-edict\IRONROOT_Design_Packet` (the real,
//! machine-readable "Ironroot"/"The Bell Pit" design specs — schema,
//! terminology, design laws — surveyed 2026-08-13) and
//! `F:\v3\TODO\dirge-of-ironroot\OLDPYGOTHROUGH\overseer\context\
//! 60_LORE_NARRATIVE.md` (the Trigon trial structure, same survey pass).
//!
//! This is NOT a port — there was no ready-made zone/dialogue data file to
//! drain (`zone_runtime_markdown_specs`/`IRONROOT_Rust_Markdown_Specs` were
//! both empty on inspection). It's authored content, grounded in the real
//! terminology and structure those specs establish, using the already-
//! landed [`crate::ironroot::platform::ArenaZoneDef`]/[`crate::ironroot::
//! dialogue::DialogueGraph`] shapes rather than inventing new ones.
//!
//! **Zone** — the four Trigon trials (`60_LORE_NARRATIVE.md:6-9`: First
//! Trigon Trial/Fire, Second/Labyrinthine Burden/Earth, Third/dual-avatar
//! nodes/Air, Fourth/wound transmutation/Water) become the arena's four
//! [`ZonePhase`]s, in that order — `material_from_element`
//! (`platform.rs`) already maps `fire`/`earth`/`air`/`water` to real
//! platform materials, so this zone rides existing logic, not new code.
//!
//! **Dialogue** — the bell keeper NPC speaks the real replaced terminology
//! from `ironroot_world_systems_bundle.v1.json`'s `naming_replacements`:
//! Thirteen Tolls (not "13 Moons"), Root-Masks (not "animal forms"), the
//! Witness-Liability Ledger (not "faction reputation only"). The one
//! locked branch demonstrates [`crate::ironroot::dialogue::DialogueChoice`]
//! `lock` — real mechanic, not decoration.

use crate::dm::{EventAngle, EventState};
use crate::ironroot::dialogue::{DialogueChoice, DialogueGraph, DialogueNode};
use crate::ironroot::platform::{ArenaZoneDef, ZonePhase};

/// A tag the player must hold to be offered the Root-Masks branch — struck
/// once by a real event elsewhere (a completed Trigon trial, say); this
/// module only names the constant, not what strikes it.
pub const TAG_SURVIVED_FIRST_TOLL: u64 = 0x134E_5F52_00_5541_u64;

/// The Bell Pit's arena zone: the four Trigon trials, in order.
pub fn bell_pit_zone() -> ArenaZoneDef {
    ArenaZoneDef {
        id: "bell_pit".into(),
        phases: vec![
            // First Trigon Trial (Fire — Aries/Leo/Sagittarius).
            ZonePhase { element: "fire".into(), ..Default::default() },
            // Second Trigon Trial — Labyrinthine Burden (Earth — Taurus/Virgo/Capricorn).
            ZonePhase { element: "earth".into(), ..Default::default() },
            // Third Trigon Trial — dual-avatar nodes (Air — Gemini/Libra/Aquarius).
            ZonePhase { element: "air".into(), ..Default::default() },
            // Fourth Trigon Trial — wound transmutation (Water — Cancer/Scorpio/Pisces).
            ZonePhase { element: "water".into(), ..Default::default() },
        ],
        ..Default::default()
    }
}

/// The bell keeper's dialogue graph — an entry conversation at the pit's
/// threshold, real terminology throughout.
pub fn bell_keeper_dialogue() -> DialogueGraph {
    DialogueGraph::from_nodes([
        DialogueNode {
            id: "root".into(),
            speaker: None,
            text: "The bell has not rung in your name. Thirteen Tolls turn \
                   over this pit before the year forgets you. Will you \
                   answer the first?"
                .into(),
            choices: vec![
                DialogueChoice { label: "I will answer.".into(), next_node: "accept".into(), lock: vec![] },
                DialogueChoice { label: "Not yet.".into(), next_node: "decline".into(), lock: vec![] },
                DialogueChoice {
                    label: "Speak of the Root-Masks.".into(),
                    next_node: "root_masks".into(),
                    // Only offered once the player has already survived the
                    // first toll — the bell keeper does not explain a
                    // Root-Mask to someone who has not earned the right to
                    // wear one.
                    lock: vec![TAG_SURVIVED_FIRST_TOLL],
                },
            ],
        },
        DialogueNode {
            id: "accept".into(),
            speaker: None,
            text: "Then the pit is yours. Every kill, every spared throat, \
                   every debt you tithe or refuse — the Witness-Liability \
                   Ledger keeps it, not I. Go down."
                .into(),
            choices: vec![],
        },
        DialogueNode {
            id: "decline".into(),
            speaker: None,
            text: "The bell will still be here. It does not tire of waiting; \
                   only the living do."
                .into(),
            choices: vec![],
        },
        DialogueNode {
            id: "root_masks".into(),
            speaker: None,
            text: "A Root-Mask is not a face you put on. It is a face the \
                   pit already grew for you, underground, while you were \
                   answering its tolls. You only go looking for it after."
                .into(),
            choices: vec![DialogueChoice { label: "Back to the bell.".into(), next_node: "root".into(), lock: vec![] }],
        },
    ])
}

/// The Bell Pit's own [`EventState`] — real content, not a synthetic test
/// fixture, wired to actually route through
/// [`crate::dm::resolution_router`].
///
/// **What's textually anchored (cited, not invented):**
/// - `EventAngle::Ledger` discovered — the bell keeper names the Ledger
///   outright: "the Witness-Liability Ledger keeps it, not I" (the `accept`
///   node, above).
/// - `EventAngle::Environmental` discovered — accepting sends the player
///   into [`bell_pit_zone`]'s four physical Trigon trials; the environment
///   itself is the thing being engaged, not inferred.
/// - `EventAngle::Spirit` discovered + `spirit_variant_unlocked: true` —
///   only true once [`TAG_SURVIVED_FIRST_TOLL`] is held, which is exactly
///   the prerequisite that unlocks the Root-Masks branch ("a face the pit
///   already grew for you, underground") — the dialogue's own lock IS this
///   field's real-world trigger, not a guess. This function models the
///   post-first-toll snapshot, not the threshold state before any toll.
///
/// **What's NOT set, and why:** `volatility`, `shadow_interference`,
/// `witnesses_alive`, `evidence_quality` all stay `0`. Nothing in the
/// design packet or this dialogue assigns them a number — inventing one
/// would repeat the exact fabricated-magnitude mistake
/// `MODE_CENTROIDS`'s own doc comment warns against.
/// `EventAngle::Combat`/`Mercy` are NOT discovered here either: "every
/// kill, every spared throat" names those as ledger-tracked *outcomes*
/// across the whole pit, not something this threshold-dialogue snippet
/// establishes as a discovered *approach* — there's no encounter content in
/// this module to justify claiming them. `faction_owner` stays `None`: no
/// faction ties to the Bell Pit anywhere in the cited sources.
pub fn bell_pit_event_state() -> EventState {
    let mut evt = EventState::new(1);
    evt.discover_angle(EventAngle::Ledger);
    evt.discover_angle(EventAngle::Environmental);
    evt.discover_angle(EventAngle::Spirit);
    evt.spirit_variant_unlocked = true;
    evt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ironroot::dialogue::{opens, DialogueState};

    #[test]
    fn the_zone_carries_all_four_trigon_trials_in_order() {
        let zone = bell_pit_zone();
        assert_eq!(zone.id, "bell_pit");
        let elements: Vec<&str> = zone.phases.iter().map(|p| p.element.as_str()).collect();
        assert_eq!(elements, vec!["fire", "earth", "air", "water"], "the trial order is the design packet's own hour ordering (6-9, 60_LORE_NARRATIVE.md)");
    }

    #[test]
    fn the_dialogue_graph_has_no_dangling_edges() {
        let g = bell_keeper_dialogue();
        assert!(g.dangling().is_empty(), "every choice must name a real node");
    }

    #[test]
    fn the_dialogue_graph_has_no_orphans() {
        let g = bell_keeper_dialogue();
        assert!(g.orphans("root").is_empty(), "every node but root must be reachable from root");
    }

    #[test]
    fn root_masks_branch_is_locked_until_the_tag_is_held() {
        let g = bell_keeper_dialogue();
        let root = g.get("root").unwrap();
        let root_masks_choice = root.choices.iter().find(|c| c.next_node == "root_masks").expect("root_masks choice exists");
        assert!(!opens(&[], &root_masks_choice.lock), "an empty keyring must not open the Root-Masks branch");
        assert!(opens(&[TAG_SURVIVED_FIRST_TOLL], &root_masks_choice.lock), "surviving the first toll must open it");
    }

    #[test]
    fn accepting_the_bell_arrives_at_a_real_ending() {
        let g = bell_keeper_dialogue();
        let mut s = DialogueState::default();
        assert!(g.start(&mut s, "root"));
        for _ in 0..512 {
            s.tick();
            if s.finished {
                break;
            }
        }
        let arrived = g.choose(&mut s, 0).expect("accept resolves");
        assert_eq!(arrived.id, "accept");
        assert!(arrived.choices.is_empty(), "accept is a real ending, not another branch");
    }

    // ── live routing: real content through the real pipeline ─────────────
    // Not a synthetic EventState — bell_pit_event_state()'s fields are each
    // cited against this file's own dialogue text and lock tag. This proves
    // the encode->route pipeline runs end-to-end on actual authored content,
    // not just hand-built test fixtures. Still not a balance claim — the
    // centroids underneath are MODE_CENTROIDS.

    #[test]
    fn bell_pit_event_state_carries_its_cited_angles() {
        let evt = bell_pit_event_state();
        assert!(evt.has_angle(EventAngle::Ledger));
        assert!(evt.has_angle(EventAngle::Environmental));
        assert!(evt.has_angle(EventAngle::Spirit));
        assert!(!evt.has_angle(EventAngle::Combat), "no encounter content in this module justifies Combat");
        assert!(!evt.has_angle(EventAngle::Mercy), "no encounter content in this module justifies Mercy");
        assert_eq!(evt.angle_count(), 3);
        assert!(evt.spirit_variant_unlocked, "mirrors TAG_SURVIVED_FIRST_TOLL's own unlock semantics");
        assert_eq!(evt.faction_owner, None, "no faction ties to the Bell Pit anywhere in the cited sources");
    }

    #[test]
    fn the_bell_pit_routes_live_through_the_resolution_router() {
        use crate::dm::{encode_event_query, resolution_router, ResolutionMode};

        let evt = bell_pit_event_state();
        let query = encode_event_query(&evt);
        let router = resolution_router();

        let (expert_id, margin) = router
            .route(&query)
            .expect("real Bell Pit content must not trap a sentinel byte");
        let mode = ResolutionMode::from_expert_id(expert_id).expect("valid expert id");

        // Measured (L02), not asserted from reasoning: this event's real
        // angle set (Ledger+Environmental+Spirit, spirit_variant_unlocked)
        // overlaps Inherit's centroid on both its nonzero dims (Spirit,
        // spirit_variant_unlocked) with zero conflicting signal elsewhere —
        // the closest match among all 7 under MODE_CENTROIDS' current
        // authoring. Locking this in as a regression check on the live
        // pipeline; still a first-pass authoring claim, not a playtested one.
        assert_eq!(mode, ResolutionMode::Inherit, "expert {}, margin {}", expert_id, margin);
        assert_ne!(mode, ResolutionMode::Kill, "a pressure-free, non-violent event must not route to Kill");
    }

    #[test]
    fn the_bell_pit_resolves_through_the_real_entry_point() {
        // The actual call site a game session would use — resolve_event_mode,
        // not the raw router — proving the full confidence-gated pipeline
        // (encode -> route -> margin check -> resolve) on real content, not
        // just the router in isolation.
        use crate::dm::{resolution_router, resolve_event_mode, NoEscalation, ResolutionMode};

        let evt = bell_pit_event_state();
        let router = resolution_router();
        let mode = resolve_event_mode(&evt, &router, &NoEscalation)
            .expect("the Bell Pit's real margin (3) clears MARGIN_CONFIDENCE_THRESHOLD (2.0) without escalating");
        assert_eq!(mode, ResolutionMode::Inherit);
    }
}
