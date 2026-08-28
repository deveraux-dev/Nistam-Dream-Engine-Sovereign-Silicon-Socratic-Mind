# forge-envelope

**Deterministic, tick-bounded ephemeral memory containers with tamper-evident evidence chains.**

`forge-envelope` is a `#![no_std]` Rust crate designed for security-critical, zero-allocation-on-replay hotpaths, and verifiable state histories. It is the core cryptographic container underlying the **Surface Ledger** ([13forge.com](https://13forge.com)) ecosystem.

A payload lives for a bounded number of **logical simulation ticks**, not wall-clock time, after which its bytes are cryptographically zeroized in memory. Before the payload is destroyed, a SHA-256 seal may be taken and appended to a rolling, tamper-evident ledger, proving the state transition occurred without retaining any of the zeroized raw data.

---

## Architectural Principles: Determinism & "Compute at Rest"

### Why Ticks?
Two independent machines replaying the same sequence must destroy identical payloads at the exact same logical step. A wall-clock TTL makes expiration a local accident dependent on CPU scheduling and network latency, causing nodes to drift. By using integer ticks, memory lifetimes and task lifecycles are bound to a unified simulation metronome.

### Proactive Enforced Zeroization
Memory lifetimes must enforce themselves. `EphemeralEnvelope` takes `&mut self` on its read path (`.get()`). If a read is attempted past the tick deadline, the envelope **wipes the raw bytes immediately** and returns `None`, rather than trusting the caller to check or poll expiration.

---

## The Three States of an Ending (Balanced Trits)

Every payload transition resolves into exactly one of three logical dispositions. This is represented as a balanced trit ($[-1, 0, +1]$), where the natural default (passive expiration) is the fixed point:

| Disposition | Trit | Meaning |
| :--- | :---: | :--- |
| `Disposition::Revoked` | `-1` | Wiped on purpose, unwitnessed (explicit revocation before deadline). |
| `Disposition::Expired` | `0` | The deadline passed; the payload fell through and was wiped unwitnessed. |
| `Disposition::Attested` | `+1` | Sealed via SHA-256 before destruction. The hash survives; raw data is wiped. |

Only `Attested` carries a hash inside the variant. It is structurally impossible for a revoked or expired payload to be forced to produce a seal.

---

## Core Ecosystem Modules

I have engineered and integrated four new cleanroom modules inside this crate to support a unified, cyber-physical, dual-flywheel architecture:

1. **`somatic_tokenizer`:** Handles offline real-time tactile and photometric normal maps unpacking. Performs scale-invariant L2 normalization on the fly with zero heap allocations.
2. **`cognitive_heal`:** A dependency-free f64 DSP core providing high-precision filters, fractional delays, Schroeder diffusers, envelope followers, and a complete Freeverb-style Reverb processor for ADHD focus entrainment.
3. **`mom` (Mixture of Musicians):** A high-performance audio event routing layer. Uses XOR and POPCNT bit-parallel distance lookups to route 16-byte `UmpWord` packets across 49 slot experts, summing outputs through `MomBus` with PCG-seeded TPDF (Triangular Probability Density Function) dither.
4. **`safety_router`:** Enforces structural safety boundaries and triggers local multi-expert debates if high-entropy anomalies are detected in Sieve-13 tokens.

---

## Usage

```rust
use forge_envelope::{Disposition, EphemeralEnvelope, EvidenceChain};

// 1. Initialize an append-only rolling evidence chain
let mut chain = EvidenceChain::new();

// 2. Create an envelope holding a secret payload, valid for 60 ticks starting at tick 0
let vow = EphemeralEnvelope::new(b"a vow".to_vec(), 0, 60);

// 3. Resolve the envelope before the deadline (at tick 10)
let link_a = vow.resolve(10, &mut chain);
assert!(matches!(link_a.record(), Disposition::Attested(_)));

// 4. Create another envelope that will be allowed to expire
let name = EphemeralEnvelope::new(b"a stolen name".to_vec(), 0, 60);

// 5. Resolve it past its deadline (at tick 60)
let link_b = name.resolve(60, &mut chain);
assert_eq!(link_b.record(), Disposition::Expired);

// 6. The EvidenceChain proves both events happened, in order, without holding raw bytes
assert!(link_a.verify());
assert!(link_b.verify());
assert!(link_b.follows(&link_a));
assert_eq!(chain.len(), 2);
```

---

## The Hardware Backstop (`Drop`)

Security cannot rely on manual execution paths. The `Drop` trait is implemented for `EphemeralEnvelope` as a hardware-level backstop. If an envelope falls out of scope or the thread panics, the payload is automatically zeroized. 

No hashing occurs during `Drop`—a seal computed inside `Drop` would have nowhere to go. `Drop` only wipes, ensuring raw bytes never leak into free memory.

---

## Features & Compilation

* **`no_std` Support:** Maintain pure `#![no_std]` compliance for embedded and edge-metal runtimes by disabling default features.
* **Cryptographic Primitives:** Uses zero-allocation-friendly `sha2` for hashing and `zeroize` for secure memory erasing.

```toml
[dependencies]
forge-envelope = { version = "0.1.0", default-features = false }
```

---

## License

Licensed under either of

* MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

*Independent craftsmanship from the physical built-environment to systems-level Rust code. © 2026 Sean Morin.*
