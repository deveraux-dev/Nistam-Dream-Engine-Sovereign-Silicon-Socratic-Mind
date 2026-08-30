# Ironroot Character Consequence Engine v1

Generated: 2026-05-16

## Purpose

This patch adds the missing character-progression mirror of the World Consequence Engine.

Core lock:

```text
Character progression is the player's personal consequence engine.
The world changes because sources act on targets.
The character changes because the player acts on the world.
```

## Files

| File | Purpose |
|---|---|
| `ironroot_character_consequence_engine.v1.json` | Canonical character consequence spec. |
| `ironroot_character_consequence_engine.v1.yaml` | YAML mirror. |
| `ironroot_world_systems_add_character_consequence.patch.json` | Additive patch targeting previous world-systems bundle. |
| `ironroot_world_systems_bundle.v2.merged.json` | Merged v2 bundle when previous bundle is available. |

## Suggested Repo Path

```text
game/src/character_consequence/
specs/generated/ironroot_character_consequence_engine.v1.json
```

## Required Module

`character_consequence`

## Required Files

- `mod.rs` — Public exports and integration entrypoint.
- `query.rs` — ProgressionQuery packing, alignment checks, validation, and helper constructors.
- `descriptor.rs` — GrowthDescriptor flags, bit helpers, and application helpers.
- `skill_curve.rs` — Precompiled skill gain curves and Central-Third skill gate.
- `domains.rs` — Gather, craft, build, social, root, secret, combat domains.
- `credit.rs` — Superior-Dexter skill credit arbitration.
- `streak.rs` — Diminishing returns, anti-grind state, reset conditions.
- `fatigue.rs` — Fatigue cost and recovery interaction with campfires, rest, travel, and social context.
- `discovery.rs` — Recipe, route, secret, title, dialogue, and worldstate unlock outputs.
- `dialogue_progression.rs` — Dialogue roads and social skill progression.
- `root_harmony.rs` — Player relationship to Ironroot and bidirectional progression/world effects.
- `root_mask_fields.rs` — Root-Mask attractor field progression.
- `router.rs` — Static baked progression router or deterministic primitive progression table.
- `apply.rs` — Applies GrowthDescriptor to player state, diplomacy, root state, discoveries, and fatigue.

## Implementation Phases

1. **Core Character Consequence Types** — ProgressionQuery, GrowthDescriptor, SkillDomain, ActionTag, alignment/size tests
2. **Skill Curves and Central-Third Gate** — SkillCurve, difficulty sweet spot, XP scaling, too_easy/too_hard handling
3. **Credit Arbitration** — Superior-Dexter skill resolver, primary/secondary XP, multi-domain action tests
4. **Streak/Fatigue/Anti-Grind** — streak state, fatigue state, reset triggers, XP suppression
5. **Discovery and Dialogue Integration** — discovery_id table, dialogue_road_unlock, reputation_delta, secret_found
6. **Root Harmony and Root-Mask Attractor Fields** — root_harmony bands, world/local WCE effects, root-mask attractor deltas, shadow/vowless progression

## Ingestion Checks

- ProgressionQuery must be exactly 16 bytes.
- GrowthDescriptor must be exactly 8 bytes.
- Every XP award must cite a domain and action_tag.
- Every major discovery must have discovery_id and source context.
- Every multi-skill action must pass skill_credit_arbitration.
- Every repeated-action loop must update streak.
- Every fatigue-causing action must update fatigue or explicitly opt out.
- Root Harmony changes must be signed and auditable.
- Dialogue progression must go through ProgressionQuery, not bespoke branch-only XP.
- Root-Mask unlocks must use attractor fields, not hard class selection.
- No runtime retraining.
- PrimitiveProgressionPrior required for unknown actions.
