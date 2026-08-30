# ARCH-012 — Cache-Line Uniformity (GPU-Native Layout Streams)

> **Status:** ACTIVE
> **Date:** 2026-07-02
> **Enforced by:** `float_in_ir = forbidden` · `alloc_steady = forbidden` · `runtime_parse = forbidden`

---

## One-Line Law

**Every lowered VixiScript slot is an identical-stride record. No heap pointers. No variable-length fields. Linear memory = linear cache = GPU-streamable.**

---

## The Problem HTML Solves Wrong

```
HTML DOM: [Node (Heap Ptr)] → [String Tag (Heap)] → [CSS Map (Dynamic Bucket)]
Result:   Pointer-chasing, L1/L2 thrash, no GPU path, O(tree) reflow
```

---

## The VixiScript Guarantee

```
Lowered IR: [WidgetRect₀ (N bytes)] [WidgetRect₁ (N bytes)] [WidgetRect₂ (N bytes)] ...
Result:     Sequential access, prefetcher-optimal, memcpy to GPU, O(N) linear pass
```

- `N` is compile-time constant (≤256 bytes)
- All strings: fixed `[u8; 32]` inline arrays, truncated at parse time
- All geometry: `i64` MilliUnit (no float)
- All proportions: `i32` Permyriad (no float)
- All colors: `u32` CID (no hex string)
- Zero heap allocation in steady-state render path

---

## Why It Matters

| Property | HTML DOM | VixiScript IR |
|----------|----------|---------------|
| Node size | Variable (16B–4KB+) | Fixed (N bytes) |
| Access pattern | Pointer-chasing (random) | Sequential (linear) |
| String storage | Heap-scattered | Inline fixed-array |
| GPU uploadable | No (serialize first) | Yes (memcpy) |
| Cache behavior | L1/L2 thrash | Prefetcher-optimal |
| Layout cost | O(tree depth × reflow) | O(N) linear pass |

---

## Enforcement Stack

1. **Grammar gates** — every `.kit.vixi` declares `alloc_steady = forbidden`, `float_in_ir = forbidden`, `runtime_parse = forbidden`
2. **`load_kit` verifier** — missing gates = parse failure = cannot lower
3. **Struct layout** — `WidgetRect` is `#[repr(C)]`, `Copy`, no `Vec`/`String`/`Box`
4. **`all_studio_panels_lower` test** — proves every registered kit produces valid uniform-stride output

---

## Thermodynamic Consequence

Because the array is flat and uniform, state changes **radiate through contiguous memory** as a wave — not as events through a tree:

- Temperature signal changes → single write to VibeMatrix channel
- Next render pass: linear scan picks up the new value per-slot
- No tree walk, no observer notification, no cascade recalculation
- The "Thermodynamic DOM" — physics replaces event propagation

See: `forge-vix/src/thermodynamic.rs` (EnvironmentSignals → VibeMatrix → spring modulation).

---

## References

- ARCH-001 §Two Clocks (integer-only substrate)
- ARCH-007 §Capillary (flat arrays in genre tissue)
- ARCH-009 §DET-CLOCK (deterministic timing for spring integration)
- `forge-vix/src/ir.rs` — `WidgetRect`, `IrRect`
- `forge-vix/src/layout.rs` — uniform-stride lowering
- `forge-vix/src/thermodynamic.rs` — environment → signal → spring bridge
