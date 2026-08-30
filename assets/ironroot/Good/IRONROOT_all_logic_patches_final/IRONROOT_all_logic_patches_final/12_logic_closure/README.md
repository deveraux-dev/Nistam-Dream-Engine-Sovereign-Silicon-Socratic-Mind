# IRONROOT Logic Closure Patch Set 12

Final logic pass before asset generation.

## Adds
- `lore_core.rs`: canonical hidden account, authority, quincunx, Yod, charge-head, proof types.
- `lore_registry.rs`: First Lock registry and world-first policy.
- `first_lock_specs.rs`: 12 concrete First Lock definitions.
- `harmonic_substrate.rs`: 40Hz / 432Hz / inverse / 800Hz+ integer physics and phase cancellation.
- `packed_state.rs`: 10-bit deterministic replay/rollback state.
- `gjk_integer.rs`: deterministic integer GJK collision scaffold.
- `photometric_waveform.rs`: asset-gen bridge from 2D/deck data to height/normal/material/resonance.
- `cutscene_atoms.rs`: 7 deterministic cutscene atom archetypes.
- `vixiscript_rules.rs`: authoring model for asymmetric rules.
- `disclosure_policy.rs`: one progressive-disclosure rulebook.
- `server_proofs.rs`: signed proof preimage model.
- `save_migration.rs`: versioned save migration shell.
- `name_shear_accessibility.rs`: safe configurable Name-Shear cue policy.
- `balance_tables.rs`: central tuning constants.
- `lore_determinism_tests.rs`: copy-ready test scaffolding.

## Integration order
1. Move `lore_core.rs` in first.
2. Replace duplicate sidecar enums from sets 1-11 with imports from `lore_core`.
3. Wire First Lock solve flow through `lore_registry.rs`.
4. Add harmonic substrate to combat/physics formulas.
5. Add packed state to replay/rollback/lockstep.
6. Keep `photometric_waveform.rs` ready for asset generation.
