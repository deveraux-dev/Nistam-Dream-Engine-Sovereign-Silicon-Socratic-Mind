//! Ironroot MUD — status chapter. What works, what's limited, what's a reserved stub,
//! mirrored from crates/sf-wasm/src/mud.rs as of 2026-07-18. Not a design doc — a
//! truthful readout so the codex never over-claims the engine's proven surface.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// One lore line per surface, tagged WORKS / GAP / STUB so the status reads at a glance.
pub fn mud_status_chapter(title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Custom("Ironroot MUD".into()));

    // WORKS — proven green (cargo test -p sf-wasm mud, 2026-07-18).
    ch.add_lore("WORKS: MudRules swaps the whole game — verbs, gate/phase bounds, resonance, tier modifiers, skill deltas, faction rooms, and starting resistance are all data.");
    ch.add_lore("WORKS: Central-Third skill gain is collapse-proof — forge_core::resistance floors the band, so fresh skills grow and no debuff can permanently lock a skill.");
    ch.add_lore("WORKS: each verb group fires its own World-Consequence curve (strike→Damage, gather→Plume, …) — native build only.");
    ch.add_lore("WORKS: gate-pass chaos is deterministic off the ledger hash chain — AstraKey HMAC seed + a real 16-star celestial pick, no wall-clock.");
    ch.add_lore("WORKS: a starter item mints per character (ItemForge::assemble_sword) and speaking in a faction room moves live reputation.");
    ch.add_lore("WORKS: a full playthrough (look/status/strike/craft/gather/speak/gate) reads coherently end to end.");

    // GAP — real limits, not bugs.
    ch.add_lore("GAP: the browser/wasm face is text + skills only — forge-consequence/items/game-systems are native-gated out of the cdylib, so no WCE/item/faction wiring on the web.");
    ch.add_lore("GAP: rooms have no occupants — a Room holds terrain/tier/exits; there is no NPC or entity concept in the MUD yet.");
    ch.add_lore("GAP: no procedural floor or loot-table generation exists, so the ASP constraint validator has nothing to gate.");
    ch.add_lore("GAP: only forge-game-systems::factions is wired; forge-cart-brain::faction_mind's separate 8-faction roster is untouched.");

    // STUB — reserved seams, meant to be wired, not junk to cut.
    ch.add_lore("STUB: AuthorityRank (PlayerAction..Temporal) — the resolution-order authority is defined but unwired; its rank should decide which pressure wins when reputation, faction standing, zone topology, and weather collide on one event (Sean 07-18 priority).");
    ch.add_lore("STUB: negative resistance modifiers — forge_core::resistance carries buff/debuff/timed mods, but mud's [u16;32] skills are not yet migrated to a base+modifier character sheet.");
    ch.add_lore("STUB: Yod / Tas-de-Charge convergence fires on 3+ pressures, but the merged authority ordering is not yet applied to resolution.");

    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 6 WORKS + 4 GAP + 3 STUB. Count is the guard: no line (least of all the
    /// AuthorityRank seam) can be silently dropped without failing here.
    #[test]
    fn mud_status_chapter_lists_works_gaps_and_stubs() {
        let ch = mud_status_chapter("Ironroot MUD — Status");
        assert_eq!(ch.lore_count(), 13);
    }
}
