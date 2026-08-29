# Continuous soft-routing + hypersphere blend + trained-content wiring

## Context

The MetaRouter (`crates/forge-core-v3/src/metarouter.rs`) already does real 1-of-7
hard routing over quantized Gemma weights, and `RamusPrimeNode`/`HypersphereVector5D`
(`crates/forge-core-v3/src/ramus_prime.rs`) already do exact finite-field (F_M61)
point arithmetic in 5D. The ask is to make routing *continuous* (soft blend across
experts instead of argmax-1) and give the hypersphere structure a real interpolation
op ("keep it going in a hypersphere formation and just draw from it"), then make sure
there's real trained content behind it — SoulWord/BodyWord/MindWord currently have
packers and tests but zero production consumers.

This is three additive, ARCH000-scoped pieces on top of code that already compiles
and passes its own test suite. Nothing existing is renamed or removed. No new crates.

## Part 1 — Continuous soft-routing (MetaRouter / HierarchicalMoe)

Reuses, unchanged: `TRIT_DIST_LUT` (`metarouter.rs:145`), the dist loop in
`MetaRouter::route` (`metarouter.rs:232,244-255`), `pack_trits_into`
(`metarouter.rs:77`), `SubRouter::evaluate` (`hierarchical_moe.rs:92`),
`HierarchicalMoe::select_top_k` (`hierarchical_moe.rs:181`), and the existing
fixed-point `Permyriad(i32)` idiom (`fixed_point.rs:17,38,639-640`) as the output type
— no new byte layout needed.

New, additive:
- `MetaRouter::route_soft(&self, query: &[f32]) -> Result<[Permyriad; 7], u8>` in
  `metarouter.rs` — same dist computation as `route()`, normalized to 7 Permyriad
  weights instead of collapsed to one argmax winner.
- `SubRouter::evaluate_soft(&self, query: &[f32]) -> [Permyriad; 7]` in
  `hierarchical_moe.rs`, mirroring `evaluate()`.
- `permyriad_softmax_from_dist(dists: &[u32; 7], bias: &[i32; 7]) -> [Permyriad; 7]`
  — new integer-only free fn in `metarouter.rs`, shared by both callers. Rank-preserving
  inverse-distance normalization (monotone stand-in for softmax) — no `exp()`, no float,
  matching the "no floats at rest" axiom.

Explicitly not built here: a weighted-content blend combiner that consumes the soft
weights to actually mix SoulWord/BodyWord content — blocked until Part 3 lands (no
packer output to blend yet). Naming that struct now would be premature ARCH000 surface.

Regression oracle: all 8 existing `metarouter.rs` tests and all `hierarchical_moe.rs`
tests must stay green untouched; new soft-routing tests are additive only.

## Part 2 — Hypersphere field-distance blend (RamusPrimeNode)

Ground truth: `F_M61` is a finite prime field — no order, no norm. `mersenne_dot`
(`ramus_prime.rs:179-194`) is exact algebraic membership-check arithmetic
(`is_on_sphere`, `ramus_prime.rs:162-167`), not a distance metric. True
distance-decayed interpolation cannot live in that field without importing an ordered
representation, which would break the integer-only law — so this plan keeps two
representations separate, same discipline as the existing Ghostmoon/hypersphere split:

- **Selection** happens in the already-ordered integer axes space: new
  `axes_distance(a, b) -> u32` (Manhattan distance over `MortonKey5D::axes()`,
  `ramus_prime.rs:83-96`), combined with existing `Box5D` pruning
  (`ramus_prime.rs:106-130`) for candidate narrowing.
- **Blend** is exact weighted field arithmetic, not a decayed interpolation: new
  `mersenne_weighted_sum(weights, points) -> HypersphereVector5D`, reusing the proven
  `u128`-accumulator/`reduce_m61_u128` pattern (`ramus_prime.rs:170-194`).
- `sample_blend(candidates: &[RamusPrimeNode], query, k) -> HypersphereVector5D` —
  takes a caller-owned candidate slice; does not itself search a store.

Named ARCH000 gate, not built: no nearest-neighbor container over `RamusPrimeNode`
exists. `sample_blend` sidesteps this by requiring the caller already hold candidates.
A live searchable population of nodes is a separate, larger, stateful-data-structure
decision — out of scope here.

Proof: algebraic-identity test (linearity of `mersenne_weighted_sum` against
`mersenne_dot`), oracled the same way as the existing `dot_oracle` test
(`ramus_prime.rs:452-470`). No round-trip test applies — the blend is lossy
(many-to-one) by construction.

## Part 3 — Wire real trained content into SoulWord/BodyWord/MindWord

Correction to the original task-sheet (`.agents/AGENT-weld-soulword-body-mind-batching.md`):
steps 1-3 are **already implemented and tested**, not missing. `soul.rs:420-1066`
already has `pack_training_pair`/`unpack_training_pair`, `pack_batch`/`unpack_batch`,
`seal_soulword`/`seal_bodyword`/`seal_mindword`, `pack_soulwords_to_body`,
`pack_bodies_to_mind`, `content_hash_fnv1a`, `truncate_hash_ref`, and a `WordResolver`,
with ~20 passing tests. Confirmed via grep: zero real consumers outside `soul.rs`
(only a doc-comment mention in `entity_memory.rs:4`).

Remaining real work is task-sheet steps 4-5 only:
- Two capacity schemes currently coexist for souls-per-BodyWord: a manifest scheme
  (`SOULS_PER_BODY = (244-2)/4 = 60`, `soul.rs:587`) and a concatenation scheme capped
  at `floor(244/52) = 4` (`pack_soulwords_to_body`, `soul.rs:651`), serving different
  callers (index vs. self-contained bundle — the latter already serves `cdk::Triad`,
  leave it alone). **Decision: use the manifest scheme (`pack_batch` + `WordResolver`,
  capacity 60) for the ML dataset path.**
- `sidecar/src/ml/train_s13.rs:91` (`train_centroid_matrix`): add a dataset adapter
  that reads `WordResolver`/chained `BodyWord`s instead of a flat `Vec<f32>`.
- `sidecar/src/flywheel_log.rs`: currently has no `soul::` import at all (confirmed via
  grep — genuine net-new wiring). Call `soul::pack_training_pair`/`pack_batch` at
  pair-log write time, so the batch structure exists from first ingest.
- Re-run `train_centroid_matrix_matches_inline_loop_learns_separable_data` and
  siblings against the new word-backed loader — must produce byte-identical output to
  today's flat-array path on the same data, proving the wrapping didn't change the
  training math.

File home: `soul.rs` (code and tests already live there; no new file).

## Verification (all parts)

- `cargo test -p forge-core-v3` — new `route_soft`/`evaluate_soft`/`axes_distance`/
  `mersenne_weighted_sum`/`sample_blend` tests green, all pre-existing MetaRouter/
  HierarchicalMoe/RamusPrimeNode/soul.rs tests untouched and green.
- `cargo test --manifest-path sidecar/Cargo.toml` — new train_s13/flywheel_log wiring
  tests green, existing centroid-matrix tests byte-identical vs. current output.
- Out of scope, untouched: `ghostmoon.rs`, `s13.rs` (sentinel enum — unrelated name
  collision with `sidecar/src/ml/quantize_s13.rs`, already confirmed separate this
  session).

## Explicit non-goals (ARCH000-gated, not silently dropped)

- No `ShiftedTritVector`/lockfree CAS primitive — no current caller needs one; net-new
  if you want it, separate sign-off.
- No new crates (`forge-trit`/`forge-word`/`forge-manifold`) — everything above fits in
  existing crate homes per L05.
- No live nearest-neighbor search structure over `RamusPrimeNode` — named blocker in
  Part 2, not solved here.
