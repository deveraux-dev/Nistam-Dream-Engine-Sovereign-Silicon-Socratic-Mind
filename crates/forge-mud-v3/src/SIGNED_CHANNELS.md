# Authoring Signed Permyriad Channels (±10k) in game.rs

The 32-channel field now supports signed permyriad ±10k with trit semantics.
Currently, `game.rs` clamps all channels to [0, +10000] because the upstream
systems (weather_sieve, haunt, square, biome) produce unsigned values.

To express corruption (negative channels), you need to:
1. Add signed value production to the upstream system
2. Invert or reinterpret its output as a signed range
3. Index-write the channel with the signed value

## Systems to Update

### weather_sieve (weather.rs)
Current: `temperature`, `wind_speed`, `chinook_buildup` — all unsigned.

**Channels fed:**
- AtmospherePa: from wind_speed & chinook_buildup (currently always positive)
- HeatGradient: from (temperature - 20).abs() (currently always positive)
- ParticulateFlux: from drought_ticks (currently always positive)

**To support corruption:**
- Allow `temperature` to go negative (e.g., -30 = profane cold, +50 = corrupted heat)
- Allow `wind_speed` to have direction/sign (east wind = +, west wind = -, etc.)
- Allow `chinook_buildup` to swing: positive = rising pressure (+1), negative = falling/void (-1)

**Example (corrupted storm):**
```rust
// In weather sieve simulation:
if is_corrupted {
    field[SenseChannel::AtmospherePa] = -8_000;  // void/pressure inverted
    field[SenseChannel::HeatGradient] = -9_000;  // unnatural cold
    field[SenseChannel::ParticulateFlux] = 10_000; // ash chokes air
} else {
    field[SenseChannel::AtmospherePa] = (w.wind_speed * 200).clamp(0, 10_000);
    field[SenseChannel::HeatGradient] = ((w.temperature - 20).abs() * 400).clamp(0, 10_000);
    field[SenseChannel::ParticulateFlux] = (w.drought_ticks.min(25) as i32 * 400).clamp(0, 10_000);
}
```

### haunt (haunt.rs)
Current: `pressure_q()` returns i32 in [0, 10000] (entity count + aggression).

**Channels fed:**
- NecroticDecay: from pressure_q()
- HateVector: from aggression_level() * 1_000

**To support corruption:**
- `pressure_q()` should distinguish "life-vitality" (+) from "death-entropy" (-)
- A haunt of undead = negative (corrupted) vs. living spirits = positive (aligned)
- Aggression can stay unsigned (always +), or gain sign for "malevolent" vs. "protective"

**Example (lich tomb):**
```rust
// In haunt simulation:
let undead_influence = haunt.undead_count as i32 * 1_000;  // +3k to +10k for lich presence
let vitality_deficit = -(haunt.living_count as i32 * 500);  // -2k if any living things present
field[SenseChannel::NecroticDecay] = (undead_influence + vitality_deficit).clamp(-10_000, 10_000);
field[SenseChannel::HateVector] = if haunt.is_hostile() { 10_000 } else { -2_000 };
```

### square (town square / government)
Current: `level_q()` returns permyriad [0, 10000] for law/order strength.

**Channels fed:**
- VitalityLux: from level_q()

**To support corruption:**
- Positive law-and-order = +VitalityLux (life thrives under rule)
- Tyranny/corruption = -VitalityLux (life withers under oppression)
- Anarchy = 0 (neither law nor chaos dominates)

**Example (corrupted town):**
```rust
// In square/law simulation:
let law_strength = i32::from(self.law_now()) * 80;
if law_strength > 50 {
    field[SenseChannel::VitalityLux] = (law_strength * 100).clamp(0, 10_000);  // just law
    field[SenseChannel::EthosBias] = 8_000;  // righteous order
} else if law_strength < 20 {
    field[SenseChannel::VitalityLux] = -(law_strength * 100).abs();  // tyranny kills vitality
    field[SenseChannel::EthosBias] = -9_000;  // corrupt rule
} else {
    field[SenseChannel::VitalityLux] = 0;  // neither thrives
    field[SenseChannel::EthosBias] = 0;  // no clear moral weight
}
```

### biome (world.rs)
Current: names only ("dungeon", "forest", etc.).

**Channels to add:**
- All arcane/anima channels should vary by biome
- Dungeon: high MasonryStress (+), negative VeilDensity (stone blocks ethereal)
- Blighted forest: high PathogenCount (+), low VitalityLux (-)
- Corrupted temple: negative EthosBias (-), high WeaveFlux (+)

**Example:**
```rust
// In biome description:
match b.name {
    "dungeon" => {
        field[SenseChannel::MasonryStress] = 8_000;  // heavy stone
        field[SenseChannel::VeilDensity] = -3_000;  // planes don't bleed here
    },
    "blighted_forest" => {
        field[SenseChannel::VitalityLux] = -8_000;  // life dies
        field[SenseChannel::PathogenCount] = 9_000;  // plague blooms
        field[SenseChannel::SporeDensity] = 10_000;  // spore-clouds
    },
    "corrupted_temple" => {
        field[SenseChannel::EthosBias] = -10_000;  // profaned
        field[SenseChannel::WeaveFlux] = 9_000;  // forbidden magic lingers
        field[SenseChannel::PietyCharge] = -8_000;  // divinity fled
    },
    _ => {} // neutral biome, let weather/haunt/square speak
}
```

## Helper: Corruption Ratio

Add a helper to `game.rs` to smoothly express corruption as a ratio [0, 1],
then scale it to signed values:

```rust
/// Express a corruption ratio as a signed permyriad.
/// ratio 0.0 = fully aligned (+10k), 0.5 = neutral (0), 1.0 = fully corrupted (-10k)
fn corruption_to_signed(corruption_ratio: f32) -> i32 {
    let clamped = corruption_ratio.clamp(0.0, 1.0);
    let signed = (clamped - 0.5) * 2.0;  // maps [0,1] to [-1, +1]
    (signed * 10_000.0) as i32
}
```

## Testing Corruption

Once a system produces signed values, test with:

```rust
#[test]
fn corrupted_cell_reads_different_from_blessed_cell() {
    let mut g = game();
    let at = [0i64, 0, 0, 0, 0];
    
    // Blessed cell (priest shrine)
    g.make_blessed();
    let (blessed_field, _) = g.sense_field_here(at);
    
    // Same cell, corrupted (bloodmage sanctum)
    g.corrupt();
    let (corrupted_field, _) = g.sense_field_here(at);
    
    // Check EthosBias flips polarity
    assert!(blessed_field[SenseChannel::EthosBias] > 0, "blessed should be positive");
    assert!(corrupted_field[SenseChannel::EthosBias] < 0, "corrupted should be negative");
    
    // Check trit inference works
    let blessed_trit = forge_core_v3::pentaract_field::trit_from_permyriad(
        blessed_field[SenseChannel::EthosBias]
    );
    let corrupted_trit = forge_core_v3::pentaract_field::trit_from_permyriad(
        corrupted_field[SenseChannel::EthosBias]
    );
    assert_eq!(blessed_trit, 1, "blessed EthosBias is +1");
    assert_eq!(corrupted_trit, -1, "corrupted EthosBias is -1");
}
```

## Golden Hash Stability

The golden hash test (`the_woven_rooms_are_byte_for_byte_what_they_were`) will
**fail** when you add signed channel production, because the prose will change.
That's correct — it means you've altered the field topology. Update the oracle
hash value in the test to the new golden hash, and commit both the field change
and the hash update together. The test now guards the NEW topology.

---

## Summary

**Current state:** All channels are clamped to [0, 10k]. Tests pass.
**Next step:** Add signed value production to weather/haunt/square/biome systems.
**Endpoint:** Priests radiate +1 (blessed), bloodmages radiate -1 (corrupted), same magnitude, opposite vector.
