# ARCH-009 — Two Drums: the Drum is the Tick, and it Must Be Two

> **Twin of Two Clocks (ARCH-001 §3 / ADR-0008).** Governs engine determinism AND the swarm bus /
> capillary transport. Derived 2026-07-01 (`swarm-around-stdio` → radio → local-loopback NOSTR →
> `Cree = drum = tick`). LOCKED slug: `the-drum-is-the-tick-and-it-must-be-two`.

## 0. One-line law

**The drum is the tick — and it MUST be two orthogonal drums, never one. A single clock is not a clock: determinism is only *provable* at the boundary between two orthogonal beats.**

## 1. The drum is the tick

The 120 Hz integer `SimTick` IS the heartbeat. Every other beat is a **projection** of it, not a peer:

- `dead_drop::RECORDING_PULSE_HZ` (`forge-audio/src/bus/dead_drop.rs:26`)
- `MetronomeClock` 120 Hz (`forge-hal/src/metronome.rs`)
- `Heartbeat::set_audio_pulse(bpm: u32, beat_phase_unit: f32)` (`forge-gui/src/heartbeat.rs:85`)
- the f32 `beat_phase_unit` = sub-tick interpolation for the GPU's smooth eye, NOT the canonical phase

The canonical phase is the **integer tick**; the float phase is cosmetic.

## 2. But it must be TWO drums

| Drum | Binding | Job | Failure if solo |
|---|---|---|---|
| **Drum-1 · Tick** | DET-CLOCK, integer 120 Hz | **SEQUENCES** — tick-stamp = canonical order, replay, lineage; integer phase crosses the firewall | blind to liveness: a *hung* capillary ≡ a *slow* one in tick-space |
| **Drum-2 · Beat** | CREATIVE-LANE, f32 wall-clock, uncapped | **DETECTS PRESENCE** — real-time liveness pulse (audible / mic-loopback heartbeat) | non-deterministic: no replay, no re-sequence |

Neither does the other's job. The tick cannot answer "who is alive **now**" (tick-time ≠ wall-time); the beat cannot answer "in what **order**, replayably."

## 3. Drift is defined by the second drum

- **Drift ≡ Drum-2 bleeding into Drum-1.** Kill Drum-2 and you don't merely lose liveness — you lose the orthogonal reference, so **drift becomes undetectable and determinism unprovable.**
- The tick proves it is deterministic **precisely by staying uncontaminated by its twin.** Orthogonality is the proof, not an accident.
- Drift is re-aligned **at the boundary**, never patched local (inherits ARCH-001 Two Clocks).

## 4. Reconciliation site (PROVEN tissue)

The two drums exchange through a **domain-agnostic** full-duplex primitive — deliberately un-labeled, because orthogonality is the point:

- `CollisionBridge` (`forge-hal/src/collision_bridge.rs:79`) — two independent `TripleBuffer` lanes (Alpha→Beta, Beta→Alpha). **Daemon-owned (Axis-Zero: the loop owner is not a node in the graph);** each side touches only its own emit + own take. `beta_take`/`alpha_take` return `None` = "reuse your last front" = **never a stall.**
- Payload `ResonanceImpulse { idx: u64, mag_pmy: Permyriad, lane: u8 }` — `idx` is documented **NOT a tick** ("the receiver decides what it indexes in its own domain"); `mag_pmy: Permyriad` is **"the one-way float valve"** (integer crosses, float does not); `impl ClockPlane for ResonanceImpulse`.
- Orthogonal-collision proof: `forge-broski/tests/orthogonal_collision.rs` (`run_alpha_solo` / `run_beta_solo` / `run_collision`).

**Scope note (Signal Law):** `CollisionBridge` is the proven *reconciliation mechanism*. Two Drums is the **doctrine instantiated onto it** — Drum-1/Drum-2 are the Alpha/Beta agents — the bridge itself names no domain concept and MUST stay that way.

## 5. Swarm corollary (the bus consequence)

An unreliable broadcast carrier (acoustic loopback / local-loopback NOSTR relay, `ws://127.0.0.1`) is **determinism-SAFE iff every event rides the tick-stamp**:

- Capillaries receive **out-of-order over a lossy wave** and re-sequence by tick. **The tick IS the sequence number / FEC**; the carrier is delivery only.
- Drum-2 (wall-clock liveness pulse) answers "which capillary is alive **now**," which the tick cannot.
- The transport stays **out-of-band** (ARCH-008: never a DET-CLOCK edge); the **tick-lineage riding on top stays canonical.** You do not trust the wire's timing — you trust the tick each event carries.
- Public relay = FORBIDDEN leak (ARCH-008). Local-loopback only; 1 relay = 1 door.

## 5b. AMENDMENT 2026-08-18 — triple_loop.rs / dual_loop.rs: parallel pattern, NOT a third drum

Raised by Sean during a lockstep/duel777/UMP fan-out. Verified before writing — do not conflate:

- **`crates/forge-core-v3/src/organs/triple_loop.rs`** [landed, observed] is a 3-thread GUI
  compositor spine (T1 Logic/DET-CLOCK 120Hz → T2 Raster → T3 GPU-Present), STRANGLER-ported
  2026-08-17 from `F:\NewRepo\crates\forge-studio\src\triple_loop.rs`. Its bridges
  (`InputBridge`/`OverlayBridge`/`WorldBridge`, all wrapping `TripleBuffer`) use the **same
  reconciliation shape** §4 names for `CollisionBridge` — producer/consumer both `try_lock`,
  a miss reuses the last front, never a stall — but T1/T2/T3 are pipeline STAGES of one thread
  handoff, not two orthogonal domains answering different questions. **This is a second, correctly
  independent instance of the lock-free-triple-buffer pattern, not a third Drum.** T1 does read
  Drum-1 (DET-CLOCK 120Hz, per its own top-doc comment) to sequence frame builds — that is the
  ONLY point of contact with Two Drums doctrine.
- **`dual_loop.rs`** [v2 only, `F:\NewRepo\crates\forge-studio\src\dual_loop.rs` — absent from
  F:\v3, confirmed by targeted search, not assumed] is the SUPERSEDED 2-thread predecessor. Its own
  source (`dual_loop.rs:101-103`) records its `OverlayBridge` was deleted 2026-07-02 as a
  "byte-identical stale twin" once `triple_loop::OverlayBridge` became canonical — v2 calls this
  the "Dual→Triple loop migration" in its own comment. Nothing in it survived un-migrated; it is
  dead-and-already-drained, not a pending port.
- **Citation flag (T1 receipt, not fixed this pass):** this tablet's own header claims "Twin of Two
  Clocks (ARCH-001 §3 / ADR-0008)." Targeted grep of `ARCH-001-the-creation-lifecycle.md` finds ZERO
  occurrences of "clock" anywhere in that file, and no `ADR-0008` file exists anywhere under
  `crates/forge-book-v3`. Both citations are unverified as written — flagging per the book-drift-gate
  discipline (aspire.rs row `book-drift-gate`) rather than silently trusting an inherited pointer.
  Correcting the citation itself is a separate row, not folded into this amendment.

## 6. Lineage

`swarm-around-stdio` (stdio = single-lumen aorta; capillaries cannot each cannulate it) → reframe bus as **carrier not connection** (radio: SWMR dead-drop, tune via aperture+phase) → **NOSTR with identity** (filter = Aperture Law wire format; signed event = census row) → **local-loopback only** (ARCH-008) → **Cree = drum = tick** → **two drums, mandatory.** Not net-new capability: revascularizes `CollisionBridge` + `MetronomeClock` + the `dead_drop` pulse.
