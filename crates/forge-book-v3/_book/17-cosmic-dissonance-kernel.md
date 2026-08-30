# The Cosmic Dissonance Kernel & Modality Mapping

This chapter details the **Cosmic Dissonance Kernel (CDK)**, mapping its integer-deterministic operations to the `SoulWord` and `RoutedUmp` paradigms. It unifies the thermodynamic forces, elemental qualities, alchemical resonance substrates, quincunx aversion, and Yod double-bind mechanics into a singular, high-performance, deterministic collision-resolution framework integrated with the `TritTree5D` spatial partitioning engine and the Technothesia acoustic suite.

---

## 1. THE COSMIC DISSONANCE KERNEL DESIGN CANON

Every asymmetric physical and factional force within the 13Forge stack has a material nature, a harmonic frequency, a geometric relationship, an authority position, and an entropy cost. This formal layer maps complex symbolic structures into strict integer state:

### The Resolution Stack
1. **Elemental Nature:** The primary quality of the active force.
2. **Alchemical Tier:** The physical state or resonance substrate frequency.
3. **Aspect Geometry:** The relative angular communication plane (cooperative, friction, aversion, or double-bind).
4. **Superior Dexter:** The controlling authority context of the interaction frame.
5. **Pressure Shape:** The localized topology of aversion or double-bind pressures.
6. **Entropy Output:** The permanent entropic scar left on the witness chain.

### Love / Strife as Global Forces
*   **Love (binding, cohesion, relation, memory):** Stabilizes names, repairs zones, and strengthens witness chains.
*   **Strife (separation, entropy, conflict, erasure):** Breaks bonds, causes Name-Shear, and creates shadows/death scars.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CosmicForce { Love, Strife }

#[derive(Debug, Clone, Copy)]
pub struct CosmicBalance {
    pub love_q: i32,
    pub strife_q: i32,
}

impl CosmicBalance {
    pub fn entropy_delta(&self) -> i32 { self.strife_q - self.love_q }
}
```

### Elemental Quality System
Elements are defined through Hot/Cold × Wet/Dry matrices, where all values compile down to `i32` Permyriad ratios:

| Element | Qualities | System Behavior |
| :--- | :--- | :--- |
| **Fire** | Hot + Dry | expansion, aggression, separation, shock |
| **Air** | Hot + Wet | communication, motion, connection, displacement |
| **Water** | Cold + Wet | dissolution, memory, flow, cohesion |
| **Earth** | Cold + Dry | density, permanence, structure, resistance |

```rust
pub struct ElementQualities {
    pub heat_q: i32,
    pub cold_q: i32,
    pub wet_q: i32,
    pub dry_q: i32,
}
```

### Alchemical Tier / Harmonic Substrate

| Tier | Frequency | System Behavior & Physics |
| :--- | :--- | :--- |
| **Nigredo** | 40Hz | mass, gravity wells, hit-stop, inertia |
| **Albedo** | 432Hz | stable collision, reflection, cleansing knockback |
| **Citrinitas** | inverse Hz | suction, reverse vectors, phase inversion |
| **Rubedo** | 800Hz+ | hyper-excitation, shockwaves, multi-hit shredding |

```rust
pub fn default_harmonic_body(element: Element, tier: AlchemicalTier) -> HarmonicBody {
    match tier {
        AlchemicalTier::Nigredo => HarmonicBody {
            element, tier, resonance_hz: 40, inverse: false,
            mass_q: 3000, volatility_q: 100, cohesion_q: 900,
        },
        AlchemicalTier::Albedo => HarmonicBody {
            element, tier, resonance_hz: 432, inverse: false,
            mass_q: 1000, volatility_q: 250, cohesion_q: 1000,
        },
        AlchemicalTier::Citrinitas => HarmonicBody {
            element, tier, resonance_hz: 432, inverse: true,
            mass_q: 500, volatility_q: 700, cohesion_q: 400,
        },
        AlchemicalTier::Rubedo => HarmonicBody {
            element, tier, resonance_hz: 800, inverse: false,
            mass_q: 250, volatility_q: 1200, cohesion_q: 100,
        },
    }
}
```

---

## 2. UNIFIED TOKENIZATION VIA TO-SOUL-WORD

Under the Gemma-Soul Pattern, all multi-modal data is lowered into the 16-byte `RoutedUmp` packet and mapped to the 64-byte sealed `SoulWord` structure.

```
+-------------------------------------------------------------+
|                     RoutedUmp (16 Bytes)                    |
+---------------------+-------------------+-------------------+
|  MT / Element [4B]  |  Aspect / Dex [4B]| Pressure Shape[4B]|
+---------------------+-------------------+-------------------+
                                      |
                                      v
+-------------------------------------------------------------+
|                      SoulWord (64 Bytes)                     |
+---------------------+-------------------+-------------------+
|    Hash [8 Bytes]   |   Parent [4 Bytes]|  Trits [52 Bytes] |
+---------------------+-------------------+-------------------+
```

### The 16-Byte `RoutedUmp` Packet Layout
The physical and structural aspects of cosmic interaction map to a unified binary stream:
*   **Header (u32):** MT (0x05 Physics) | Group (Elemental Group) | Channel (Alchemical Substrate) | Status.
*   **Aspect Geometry & Superior Dexter (u32):** Packed 8-bit representation of `AspectGeometry` and `SuperiorDexter` authority contexts.
*   **Pressure Shape (u32):** Packed 16-bit `QuincunxPressure` and 16-bit `YodPressure` values.
*   **Cosmic Balance (i32):** Deterministic entropy-delta ledger ratio (Strife_q - Love_q).

### The ToSoulWord Conversion Implementation
The `ToSoulWord` trait provides the explicit serialization bridge, compressing the asymmetric physics variables into balanced trinary format:

```rust
pub trait ToSoulWord {
    fn to_soul_word(&self) -> SoulWord;
}

impl ToSoulWord for CosmicConflict {
    fn to_soul_word(&self) -> SoulWord {
        let mut trits = [0u8; SOUL_BYTES]; // 52 bytes representing 256 trits
        
        // Pack elemental qualities into base-243 packed bytes
        // Each packed byte contains 5 balanced trits (radix 3)
        // Ensure strictly checked boundaries so MAX_PACKED (243) is never violated.
        
        let parent_idx = NO_PARENT;
        SoulWord::seal(trits, parent_idx)
    }
}
```

---

## 3. HIGH-DIMENSIONAL LATENT SPACE PROJECTION

To resolve dynamic interactions, the `CosmicConflict` is projected into a 105-trit `PackedPoint105` inside our flat 5D balanced trinary partitioning tree (`TritTree5D`). This enables $O(\log N)$ proximity searches and collision sorting using zero-allocation raycasting.

### The 5D Partitioning Coordinate System:
*   **Axis 0 (X-Axis - Qualities):** `heat_q - cold_q` (Dry/Wet thermal-pressure imbalance).
*   **Axis 1 (Y-Axis - Substrate):** Alchemical resonance coordinate mapped to prime Monzo space.
*   **Axis 2 (Z-Axis - Authority):** Faction/Authority semantic boundary, routed via `family_roots` directory with `FAMILY_STEP = 16_384`.
*   **Axis 3 (Theta-Axis - Aspect):** Angle geometry on a circle (0° to 180°), using wrap-aware angular delta `delta_theta` with `WRAP_MODULO = 360_000`.
*   **Axis 4 (W-Axis - Entropy):** Deterministic entropy-delta ledger score.

```rust
// Traversal and Search query integration
let query = conflict.to_soul_word();
let point = PackedPoint105 {
    bytes: unpack_soul_to_105_trits(&query),
    id: conflict.actor.as_u32(),
    _padding: [0; 7],
};

// 5D Raycast routing finds the nearest harmonic bodies in balanced space
let direction = [1.0, 0.0, 0.0, -1.0, 0.5]; // Direction vectors derived from player intention
let nearest_bodies = tree.search_k(&point, &direction, 3);
```

---

## 4. THE /ASPIRE ACOUSTIC SYNTHESIS & COVERT WITNESS CHAIN

The alchemical resonance frequencies of conflict resolutions are synthesized directly into the Technothesia Acoustic & Watermarking Suite:

1.  **Resonance Injection (`forge-audio`):**
    *   **Nigredo (40Hz):** Drives low-latency sub-bass hit-stop physical responses inside the 120Hz physics loop, utilizing lock-free ring-buffers (`rtrb` — the canon of 07-27 wrote `web_rtb`, a confabulated name; corrected to the real crate 2026-07-28, aspire run n=1) over shared memory.
    *   **Albedo (432Hz):** Phase aligner locks on collision reflection, utilizing Laroche-Dolson peak spectral lock-ons to prevent phase drift.
2.  **Rhythmic Spacing (`forge-harmonics`):**
    *   **Yod double-binds** and **Quincunx constraints** are mapped directly to Bjorklund's Euclidean pattern spacing algorithm, distributing tension pulses evenly over the timeline.
3.  **Covert Audio Steganography (`forge-stego`):**
    *   The compiled `CosmicResolution` record is encoded as a 16-byte `RoutedUmp` payload.
    *   Using `forge-stego::spread_spectrum` or `forge-stego::echo_hiding`, this UMP payload is embedded invisibly as covert telemetry into synthesized PCM audio outputs.
    *   This creates an unforgeable, self-auditing acoustic witness chain where the sound itself carries the deterministic ledger of the cosmic forces that created it.

---

<!-- DRAINED 2026-07-28 from quarry twin E:\.quarantine\md-dedup-2026-07-14\F\.reposold\dirge-of-ironroot\design-bible\10-cosmic-dissonance-kernel.md — sections the evolved chapter had dropped; folded verbatim so this file is the ONE superset home. Live code descendant: forge_core::dissonance_sieve (DissonanceVerdict). -->

## 5. Aspect Geometry

| Aspect | Degrees | Effect |
|--------|---------|--------|
| Conjunction | 0° | Fusion |
| Sextile | 60° | Cooperative support |
| Square | 90° | Acute friction |
| Trine | 120° | Stabilizing harmony |
| Quincunx | 150° | Aversion / blind spot |
| Opposition | 180° | Severe tension |
| Yod | compound | Double-bind |

```rust
pub fn aspect_effect(aspect: AspectGeometry) -> AspectEffect {
    match aspect {
        AspectGeometry::Conjunction => AspectEffect::Fusion,
        AspectGeometry::Sextile => AspectEffect::CooperativeSupport,
        AspectGeometry::Square => AspectEffect::AcuteFriction,
        AspectGeometry::Trine => AspectEffect::StabilizingHarmony,
        AspectGeometry::Quincunx => AspectEffect::Aversion,
        AspectGeometry::Opposition => AspectEffect::SevereTension,
        AspectGeometry::Yod => AspectEffect::DoubleBind,
    }
}
```

## 6. Full Conflict Resolver

```rust
pub struct CosmicConflict {
    pub actor: EntityId,
    pub target: EntityId,
    pub actor_body: HarmonicBody,
    pub target_body: HarmonicBody,
    pub aspect: AspectGeometry,
    pub authority: AuthorityContext,
    pub quincunx: Option<QuincunxPressure>,
    pub yod: Option<YodPressure>,
    pub cosmic_balance: CosmicBalance,
}

pub struct CosmicResolution {
    pub aspect_effect: AspectEffect,
    pub authority_outcome: AuthorityOutcome,
    pub elemental_modifier_q: i32,
    pub dissonance_q: i32,
    pub entropy_delta: u32,
    pub final_power_q: i32,
}

pub fn resolve_cosmic_conflict(
    conflict: &CosmicConflict,
    player: &PlayerState,
) -> CosmicResolution {
    let aspect_effect = aspect_effect(conflict.aspect);
    let authority_outcome = resolve_authority(conflict.authority);
    let elemental_modifier_q = resolve_elemental_modifier(
        conflict.actor_body, conflict.target_body);

    let quincunx_q = conflict.quincunx
        .map(|q| resolve_quincunx(q, player) as i32).unwrap_or(0);
    let yod_q = conflict.yod.map(|y| y.severity as i32).unwrap_or(0);

    let authority_q = match authority_outcome {
        AuthorityOutcome::Bonification => 1500,
        AuthorityOutcome::Maltreatment => 2000,
        AuthorityOutcome::Mitigation => 1250,
        AuthorityOutcome::Clash => 750,
        AuthorityOutcome::None => 1000,
    };

    let dissonance_q = quincunx_q + yod_q + conflict.cosmic_balance.entropy_delta();

    CosmicResolution {
        aspect_effect,
        authority_outcome,
        elemental_modifier_q,
        dissonance_q,
        entropy_delta: dissonance_q.max(0) as u32,
        final_power_q: (elemental_modifier_q * authority_q) / 1000,
    }
}
```

## 7. Major Erasure Example: Grey Orchard Cleanse

| Layer | Value |
|-------|-------|
| Element | Earth / Nigredo / closure |
| Superior Dexter | Church controls law, bell, road, witnesses |
| Tête-de-Charge | grave mason carrying cleanse writ |
| Quincunx | evacuate fast vs gather proof slowly |
| Yod | Ledger Church + Toll-Saints squeeze the village |
| Entropy Output | erased names, death scars, Shadow growth, faction pressure |

| Playstyle | Solution |
|-----------|----------|
| Fighter | kill guard, duel priest |
| Diplomat | build witness chain |
| Crafter | repair grave-bells |
| Rootthief | steal cleanse writ |
| Deathwalker | use canal death-route |
| 4X strategist | move Free Graves support early |
| Shadowbinder | deploy Shadow to delay charge-head |

## 8. Engine Constraints

All symbolic systems must compile down into deterministic integer state:

- `i32` Permyriad for ratios
- `i64` MilliUnits for spatial
- GPU-only float boundary
- Hot-path allocation discipline
- 120Hz physics metronome
- No loose metaphor enters runtime unless it becomes: enum, struct, integer score, deterministic resolver, event record

## 9. Final Thesis

The Cosmic Dissonance Kernel turns esoteric geometry into deterministic asymmetric systems logic.

**Love binds. Strife separates. Elements define behavior. Resonance defines physics. Dexter defines authority. Quincunx defines blind spots. Yod defines double-binds. Entropy records the cost.**
