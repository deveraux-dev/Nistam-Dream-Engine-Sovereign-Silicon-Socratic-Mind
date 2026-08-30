# Architectural Blueprint: Dirge of Ironroot MUD Integration

This document outlines the canonical integration between the **Dirge of Ironroot Resonance Compiler (v0.1)** and the **13Forge Sovereign Stack MUD Engine (`sf-wasm`)**. It serves as the master blueprint for the headless flash, linking procedural world generation, quest trees, alchemical resonances, character classes, talent progression, pets taming, crafting structures, and the deterministic 120Hz CPU ticker into a cohesive, zero-heap runtime.

---

## 1. Core Integration Map

The Dirge of Ironroot bundle provides rich, high-dimensional priors (music sheets, zone topologies, brand scripts, and steganographic glyphs) that are compiled directly into the strict structures of the deterministic `sf-wasm/src/mud.rs` world.

```
+─────────────────────────────────────────+        +───────────────────────────────────────────+
│   DIRGE OF IRONROOT COMPILER PRIORS     │        │          13FORGE MUD CORE (T1)            │
│  (C:\Users\seanm\Desktop\Good)          │        │      (crates/sf-wasm/src/mud.rs)          │
+─────────────────────────────────────────+        +───────────────────────────────────────────+
│  [ZoneData JSON]                        │        │  [Flat Room Pool]                         │
│  - prairie_start, forest_depths, etc.   ├───────►│  - phyllotaxis_position (Fibonacci sweep)  │
│                                         │        │                                           │
│  [Quest & Crafting recipes]             │        │  [MUD Character Sheet / EventLedger]      │
│  - first_hunt, iron_descent, Recipes    ├───────►│  - Tria Prima stats (Salt/Sulfur/Mercury) │
│                                         │        │  - Quest trackers & objectives            │
│  [Faction Glyphs & Frequencies]         │        │  - 8-Stat Block (Clarity as 8th stat)     │
│  - steganographic glyphs decoders       │        │                                           │
│  - raga-inspired mode tuning (Hz)       ├───────►│  [12 Alchemical Gates]                    │
│                                         │        │  - MacroPhase (Nigredo, Albedo, Citrinitas)│
│  [Pet Templates]                        │        │  - Resonance (Hz) & World-Consequence     │
│  - Prairie Wolf, Elder Bison, etc.      │        │                                           │
│                                         │        │  [Pet Subsystem]                          │
│  [Classes & Talents]                    │        ├─► - Active slot caps (Max 3 active)       │
│  - Alchemist, Vanguard, Chanter         │        │   - HP taming check (<= 25%) & loyalty    │
│  - Stat-gated talent tree nodes         ├───────►│                                           │
│                                         │        │  [TritTree5D / nearest_neighbor.rs]       │
│  - [x, y, z, theta, w]                  │        │  - Raycast alignment of asset pointers    │
+─────────────────────────────────────────+        +───────────────────────────────────────────+
```

---

## 2. Character Sheets, Classes & Talents (The 8-Stat Block)

The MUD core character sheets are expanded from the base Tria Prima (Salt, Sulfur, Mercury) to support full high-dimensional itemization, utilizing the expanded **8-Stat Block** defined under `MUD_SYSTEMS_PRIMER.md`.

### A. The 8-Stat Block Layout
1.  **Vigor (Salt / Body):** Vitality and raw physical power.
2.  **Shadow Weight (Sulfur / Soul):** Mass of the soul word, affecting gravity and Fae transitions.
3.  **Logic Depth (Mercury / Mind):** Intellect, computational speed in-process.
4.  **Momentum:** Metronome speed, dictating double-strike rates.
5.  **Tarnish:** Corrosion metric; higher tarnish reduces alchemical output.
6.  **Resonance:** Core frequency power, driving instrument/spell casting.
7.  **Guilt:** Sin index, boosting physical damage but choking Fae overlays.
8.  **Clarity:** *The 8th Stat Block* — governs critical hits and prevents mental lock.

### B. Character Classes
Starting stats and multipliers scale according to Class selection:
*   **Hermetic Alchemist:** Starts with high Mercury ($7,000$). Obtains a $1.5\text{x}$ multiplier to *Logic Depth* and $1.3\text{x}$ to *Clarity*.
*   **Iron Vanguard:** Starts with high Salt ($7,000$). Obtains a $1.5\text{x}$ multiplier to *Vigor* and $1.4\text{x}$ to *Shadow Weight*.
*   **Resonance Chanter:** Starts with high Sulfur ($6,000$). Obtains a $1.1\text{x}$ multiplier to *Resonance* and *Clarity*.

### C. Talent Trees
Talents are compiled as milestone gates requiring specific level and core stat ratios:
*   **Sovereign Tamer:** Gated by level 2 and Mercury $\ge 5,500$. Grants the player the active `tame` skill.
*   **Harmonic Shatter:** Gated by level 5 and Sulfur $\ge 6,000$. Automatically enables the combat shatter proc.
*   **Centering:** Gated by level 3 and Salt $\ge 5,000$. Speeds up skill recovery by $1.15\text{x}$ using the Central-Third recovery formulas.

---

## 3. The Pet Subsystem (Taming & Loyalty)

The MUD engine implements strict taming and active pet roster limits to prevent memory bloat and execution drag:

1.  **Active Cap:** A player can support a maximum of **3 active pets** concurrently.
2.  **Taming Threshold Verification:**
    *   Target must not be a Boss.
    *   Target's current HP must be $\le 25\%$ (expressed as $\le 2,500$ permyriad) of its maximum health.
    *   Player's taming skill level must be $\ge \text{Target\_Level} \times 2$.
3.  **Stat Inheritance:**
    *   **Tamed Pets:** Inherit base templates at a $0.7\text{x}$ multiplier.
    *   **Ethereal (Summoned) Pets:** Inherit base stats at a $0.5\text{x}$ multiplier with an active 300-second decay timer.
4.  **Loyalty Decay:**
    Loyalty decrements by 10 points every 600 seconds. If loyalty reaches 0, the pet automatically deserts or dies.

---

## 4. Crafting & Alchemical Recipes

Alchemical crafting is gated by both physical reagents and the player’s active **Alchemical Gate**:

*   **Synthesis Formula:** Crafting checks the recipe's `gate_required` rank. If the player's active room or gate rank is below the requirement, synthesis is rejected on sight.
*   **Deterministic Success Chance:** Success is calculated using a deterministic hash chain generated by seeding the recipe ID through the `resonance` XOR-rotate algorithm, compared against the recipe's `success_chance_pmy` (in permyriad).
*   **Consumption & Reward:** On success, materials are deducted from the player's inventory, the item is added to the inventory hash, and Tria Prima stat rewards (Salt/Sulfur/Mercury) are injected.

---

## 5. Ingesting Topologies (Zones to Rooms)

The MUD world is procedural but deterministic. It models rooms using Fibonacci phyllotaxis. The Dirge of Ironroot's `.json` zone profiles are ingested to overwrite room metadata, adding physical scale, NPC rosters, portals, and biomes:

1.  **Phyllotaxis Alignment:**
    Every `ZoneData` profile corresponds to a deterministic coordinate sweep in 5D space. For instance, `prairie_start` is mapped to depth $z = 1$ and soul vector $s = 100$.
2.  **Room Overwrites:**
    *   **Room 0:** Ingests `prairie_start.json` metadata (Wandering Trader, Prairie Wolf spawns).
    *   **Room 1:** Ingests `forest_depths.json` metadata (Forest Spirit, Iron Bark Treant).
    *   **Room 2:** Ingests `iron_caverns.json` metadata (Deep Ore Golem).
3.  **Portal Resolution:**
    Portals within JSON profiles are translated directly into MUD room exits (North, South, East, West) based on the computed golden-angle sweep.

---

## 6. Alchemical & Resonance Bridge (Hz Tuning)

The Dirge of Ironroot relies on music and frequency. In the MUD, these map directly to the **12 Alchemical Gates** and **MacroPhases**:

| Gate | MacroPhase | Dirge Tuning (Hz) | MUD World Consequence Impact |
| :--- | :--- | :--- | :--- |
| **Calcination..Solution** | Nigredo | $40\text{ Hz}$ | Base physical actions (strike, gather) gain $+20\%$ intensity. |
| **Separation..Putrefaction**| Albedo | $432\text{ Hz}$ | High-fidelity resonance; crafting actions are perfect. |
| **Congelation..Sublimation**| Citrinitas | $-1\text{ Hz}$ | Intversion phase; logic-depth actions increase mind. |
| **Fermentation..Exaltation**| Rubedo | $800\text{ Hz}$ | Agitation phase; strike actions have chance to shatter. |
| **Multiplication..Projection**| Aspirational| $1200\text{ Hz}$| Celestial transition; the way is unlocked for final ascension. |

---

## 7. Steganographic Glyph Scripting & Faction Rep

Using the `schemas/glyph_script.schema.json` rules, steganographic glyphs found in the brand scripts and images (like `thornhaven_guard_banner_golden.png`) are decoded during a `read` MUD action:

*   Decoding a hidden faction script within `Room 0` (Thornhaven Guard Headquarters) checks if the player's reputation is $\ge 5000$.
*   If valid, the steganographic glyph script executes a `CatalyticRelease` event, unlocking advanced dialogue branches and gating passage through Alchemical Gates.
*   This matches the **Yod Friction Rule**: gate passage is blocked until a signature from multiple event logs is verified in the `EventLedger`.

---

## 8. The 5D Headless Flash Execution

When executing a **Headless Flash**, the compiler:
1.  Loads `C:\Users\seanm\Desktop\Good\ironroot_manifest.json` as the target index.
2.  Parses the `.json` and `.toml` profiles, packing them into an optimized binary pack structure: `ironroot_headless_flash_datapack.ron`.
3.  Wires the datapack into the static mutex of the `MudEngine` in `sf-wasm/src/mud.rs`.
4.  Validates the full system footprint using the 5D Codebook raycaster, guaranteeing that every room, NPC, portal, class, talent, pet taming gate, and recipe resolves to a valid, reachable point in trinary space.
