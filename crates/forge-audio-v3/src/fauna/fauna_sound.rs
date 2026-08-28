//! Fauna-sound — procedural ambient prairie fauna, mixed additively into the buffer.
//! No sample files required — fully procedural.
//!
//! Cricket field:
//!   Dolbear's Law chirp scheduling (temp-driven rate), 4 virtual emitters,
//!   per-emitter density and pitch variance, silence propagation (walker/threat).
//!
//! Bird chorus:
//!   3 synthetic species slots (day/night/all-day), FM synthesis, time-of-day
//!   activation masks, density-driven volume.
//!
//! Port of `13moons/scripts/audio/{CricketAmbientField,BirdChorusField}.gd`
//! (via dead-drop-private). Zero-heap hot path: per-sample loop allocates nothing.

use std::f32::consts::PI;
use std::cell::UnsafeCell;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// LCG — deterministic, no std thread-local RNG
// ---------------------------------------------------------------------------

#[inline]
fn lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[inline]
fn lcg_f32(state: &mut u64) -> f32 {
    (lcg(state) >> 33) as f32 / u32::MAX as f32
}

// ---------------------------------------------------------------------------
// Dolbear's Law — chirp interval from temperature + density
// ---------------------------------------------------------------------------
//   chirps_per_13s = max(temp_F − 40, 1)
//   rate = (chirps_per_13s / 13) * density * BASE_CHIRP_RATE

const BASE_CHIRP_RATE: f32 = 2.0;

fn dolbear_interval(temp_c: f32, density: f32) -> f32 {
    let temp_f = temp_c * 9.0 / 5.0 + 32.0;
    let cper13 = (temp_f - 40.0).max(1.0);
    let rate = (cper13 / 13.0 * density * BASE_CHIRP_RATE).clamp(0.1, 8.0);
    1.0 / rate
}

// ---------------------------------------------------------------------------
// Cricket emitter
// ---------------------------------------------------------------------------

const CRICKET_COUNT: usize = 4;
const CHIRP_FREQ: f32      = 4200.0; // Hz — primary cricket frequency
const CHIRP_ATK_MS: f32    = 5.0;
const CHIRP_SUS_MS: f32    = 20.0;
const CHIRP_REL_MS: f32    = 10.0;
// Total chirp duration = 35ms

#[derive(Clone, Copy)]
struct Cricket {
    phase:            f32,   // oscillator phase [0, 2π)
    next_chirp:       i64,   // countdown in samples
    chirp_remaining:  i32,   // samples left in current chirp
    chirp_total:      i32,   // total samples per chirp (cached at init)
    density:          f32,
    pitch_factor:     f32,   // 2^(semitones/12)
    rng:              u64,
}

impl Cricket {
    fn new(seed: u64, i: usize) -> Self {
        let mut rng = seed.wrapping_add((i as u64).wrapping_mul(0xdead_beef_cafe_f00d));
        let semitones = lcg_f32(&mut rng) * 3.0 - 1.5; // ±1.5 st
        let offset_samples = (lcg_f32(&mut rng) * 22_050.0) as i64; // stagger start
        Cricket {
            phase:           lcg_f32(&mut rng) * 2.0 * PI,
            next_chirp:      offset_samples,
            chirp_remaining: 0,
            chirp_total:     0, // set on first use when SR is known
            density:         0.5 + lcg_f32(&mut rng) * 0.5,
            pitch_factor:    2.0_f32.powf(semitones / 12.0),
            rng,
        }
    }
}

// ---------------------------------------------------------------------------
// Bird emitter
// ---------------------------------------------------------------------------

const BIRD_COUNT: usize = 3;

// [carrier_hz, call_dur_ms, base_interval_s]
const BIRD_SPECS: [(f32, f32, f32); BIRD_COUNT] = [
    (3500.0, 150.0,  8.0),  // slot 0: meadowlark-like   (day)
    ( 400.0, 350.0, 20.0),  // slot 1: great-horned-owl  (night)
    ( 800.0, 100.0, 12.0),  // slot 2: raven-like        (all)
];

// Bitmask of active time-of-day periods per bird slot.
// Bit positions match time_bit() below.
const BIRD_TIME_MASK: [u8; BIRD_COUNT] = [
    0b0001_1110, // meadowlark: dawn, morning, midday, afternoon
    0b1110_0000, // owl:        dusk, night, deep_night
    0b0111_1110, // raven:      all except deep_night
];

fn time_bit(tod: &str) -> u8 {
    match tod {
        "dawn"       => 1 << 1,
        "morning"    => 1 << 2,
        "midday"     => 1 << 3,
        "afternoon"  => 1 << 4,
        "dusk"       => 1 << 5,
        "night"      => 1 << 6,
        "deep_night" => 1 << 7,
        _            => 1 << 1,
    }
}

#[derive(Clone, Copy)]
struct Bird {
    carrier_phase:  f32,
    vibrato_phase:  f32,
    next_call:      i64,   // samples countdown
    call_remaining: i32,
    call_total:     i32,
    density:        f32,
    rng:            u64,
}

impl Bird {
    fn new(seed: u64, i: usize) -> Self {
        let mut rng = seed.wrapping_add((i as u64).wrapping_mul(0xcafe_1234_5678_9abc));
        // Stagger initial calls so not all birds call at once
        let offset = (lcg_f32(&mut rng) * BIRD_SPECS[i].2 * 44_100.0) as i64;
        Bird {
            carrier_phase:  0.0,
            vibrato_phase:  0.0,
            next_call:      offset,
            call_remaining: 0,
            call_total:     0,
            density:        0.0,
            rng,
        }
    }
}

// ---------------------------------------------------------------------------
// Combined state
// ---------------------------------------------------------------------------

struct FaunaState {
    crickets:    [Cricket; CRICKET_COUNT],
    birds:       [Bird;    BIRD_COUNT],
    temperature: f32,
    last_sr:     u32,
}

impl FaunaState {
    fn new() -> Self {
        let seed: u64 = 0x135_eed_4a00_beef;
        Self {
            crickets: [
                Cricket::new(seed, 0),
                Cricket::new(seed, 1),
                Cricket::new(seed, 2),
                Cricket::new(seed, 3),
            ],
            birds: [
                Bird::new(seed, 0),
                Bird::new(seed, 1),
                Bird::new(seed, 2),
            ],
            temperature: 15.0,
            last_sr:     44100,
        }
    }
}

struct FaunaCell(UnsafeCell<FaunaState>);
// SAFETY: process() is only ever called from the single audio thread (SR&ED #35 invariant).
unsafe impl Sync for FaunaCell {}

static STATE: OnceLock<FaunaCell> = OnceLock::new();

fn state() -> &'static mut FaunaState {
    let cell = STATE.get_or_init(|| FaunaCell(UnsafeCell::new(FaunaState::new())));
    // SAFETY: Single audio thread — no concurrent access possible.
    unsafe { &mut *cell.0.get() }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn process(samples: &mut [f32], sample_rate: u32, params: &serde_json::Value) {
    // --- Parse params ---
    let temp_c     = params["temperature_c"].as_f64().unwrap_or(15.0) as f32;
    let tod        = params["time_of_day"].as_str().unwrap_or("dawn");
    let tod_bit    = time_bit(tod);
    let fauna_gain = params["fauna_gain"].as_f64().unwrap_or(0.0) as f32;
    let vibe       = params["vibe_position"].as_f64().unwrap_or(0.0) as f32;
    let walker     = params["walker_active"].as_bool().unwrap_or(false);

    // Walker proximity silences fauna
    let fauna_scale = if walker { (1.0 - vibe).max(0.0) } else { 1.0 };

    // Cricket state
    let cricket_silent  = array_contains(&params["fauna_silent"], "cricket");
    let cricket_absent  = array_contains(&params["fauna_absent"], "cricket");
    let cricket_density = if cricket_absent || cricket_silent { 0.0 }
                          else { params["fauna_density"]["cricket"].as_f64().unwrap_or(0.5) as f32 };

    // Bird densities — map to our 3 slots
    let bird_densities: [f32; BIRD_COUNT] = [
        fauna_density(params, "meadowlark"),
        fauna_density(params, "great_horned_owl"),
        fauna_density(params, "raven"),
    ];

    let sr = sample_rate;
    let st = state();

    // Update temperature
    st.temperature = temp_c;

    // Handle sample-rate change: rescale timing counters
    if st.last_sr != sr {
        let ratio = sr as f64 / st.last_sr as f64;
        for c in st.crickets.iter_mut() {
            c.next_chirp = (c.next_chirp as f64 * ratio) as i64;
        }
        for b in st.birds.iter_mut() {
            b.next_call = (b.next_call as f64 * ratio) as i64;
        }
        st.last_sr = sr;
    }

    // Pre-compute chirp envelope lengths (sample-rate dependent)
    let atk = (CHIRP_ATK_MS / 1000.0 * sr as f32) as i32;
    let sus = (CHIRP_SUS_MS / 1000.0 * sr as f32) as i32;
    let rel = (CHIRP_REL_MS / 1000.0 * sr as f32) as i32;
    let chirp_total = atk + sus + rel;

    // Update cricket densities
    for c in st.crickets.iter_mut() {
        c.density     = (cricket_density * fauna_scale).clamp(0.0, 1.0);
        c.chirp_total = chirp_total;
    }

    // Update bird densities
    for (i, b) in st.birds.iter_mut().enumerate() {
        b.density = (bird_densities[i] * fauna_scale).clamp(0.0, 1.0);
    }

    // --- Per-sample synthesis loop ---
    let temp_c = st.temperature; // copy to avoid borrow conflict

    for s in samples.iter_mut() {
        let mut mix = 0.0_f32;

        // ---- Cricket chirp synthesis ----------------------------------------
        for c in st.crickets.iter_mut() {
            if c.density < 0.1 {
                continue;
            }

            c.next_chirp -= 1;

            if c.next_chirp <= 0 && c.chirp_remaining <= 0 {
                // Fire a new chirp
                c.chirp_remaining = c.chirp_total;
                // Schedule next using Dolbear + jitter
                let interval_s = dolbear_interval(temp_c, c.density);
                let jitter = 0.7 + lcg_f32(&mut c.rng) * 0.6; // [0.7, 1.3]
                c.next_chirp = (interval_s * jitter * sr as f32) as i64;
            }

            if c.chirp_remaining > 0 {
                let pos = c.chirp_total - c.chirp_remaining;
                let env = if pos < atk {
                    pos as f32 / atk as f32
                } else if pos < atk + sus {
                    1.0
                } else {
                    let r = pos - atk - sus;
                    (1.0 - r as f32 / rel as f32).max(0.0)
                };

                let freq = CHIRP_FREQ * c.pitch_factor;
                let tone = c.phase.sin();
                c.phase += 2.0 * PI * freq / sr as f32;
                if c.phase > 2.0 * PI { c.phase -= 2.0 * PI; }

                mix += tone * env * c.density * 0.07;
                c.chirp_remaining -= 1;
            }
        }

        // ---- Bird call synthesis (vibrato FM) --------------------------------
        for (i, b) in st.birds.iter_mut().enumerate() {
            if b.density < 0.05 {
                continue;
            }
            if BIRD_TIME_MASK[i] & tod_bit == 0 {
                continue; // wrong time of day
            }

            b.next_call -= 1;

            if b.next_call <= 0 && b.call_remaining <= 0 {
                let dur = (BIRD_SPECS[i].1 / 1000.0 * sr as f32) as i32;
                b.call_remaining = dur;
                b.call_total     = dur;
                let jitter = 0.6 + lcg_f32(&mut b.rng) * 0.8; // [0.6, 1.4]
                b.next_call = (BIRD_SPECS[i].2 * jitter * sr as f32) as i64;
            }

            if b.call_remaining > 0 {
                let total     = b.call_total as f32;
                let remaining = b.call_remaining as f32;
                let t         = 1.0 - remaining / total; // 0 → 1

                // Envelope: 10% attack, 75% sustain, 15% release
                let env = if t < 0.10 {
                    t / 0.10
                } else if t < 0.85 {
                    1.0
                } else {
                    ((1.0 - t) / 0.15).max(0.0)
                };

                // 5 Hz vibrato at ±2% frequency deviation
                let vibrato   = b.vibrato_phase.sin() * 0.02 * BIRD_SPECS[i].0;
                let inst_freq = (BIRD_SPECS[i].0 + vibrato).max(20.0);

                let tone = b.carrier_phase.sin();
                b.vibrato_phase += 2.0 * PI * 5.0 / sr as f32;
                b.carrier_phase += 2.0 * PI * inst_freq / sr as f32;
                if b.vibrato_phase > 2.0 * PI { b.vibrato_phase -= 2.0 * PI; }
                if b.carrier_phase > 2.0 * PI { b.carrier_phase -= 2.0 * PI; }

                mix += tone * env * b.density * 0.05;
                b.call_remaining -= 1;
            }
        }

        *s += mix * fauna_gain;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn array_contains(val: &serde_json::Value, target: &str) -> bool {
    val.as_array()
        .map(|a| a.iter().any(|v| v.as_str() == Some(target)))
        .unwrap_or(false)
}

fn fauna_density(params: &serde_json::Value, species: &str) -> f32 {
    params["fauna_density"][species].as_f64().unwrap_or(0.3) as f32
}
</content>
