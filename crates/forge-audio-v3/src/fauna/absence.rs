//! AbsenceEngine — procedural frequency removal.
//!
//! Notches out expected prairie frequencies when silence zones or walker proximity
//! are detected. The brain registers missing sound as unease before conscious
//! awareness kicks in.
//!
//! Port of `13moons/scripts/audio/AbsenceEngine.gd` (via dead-drop-private) —
//! Rust sample-level DSP. Zero-heap hot path: target selection uses a fixed-size
//! candidate buffer, the per-sample loop allocates nothing.

use std::f32::consts::PI;
use std::cell::UnsafeCell;
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const NOTCH_COUNT: usize = 3;
const NOTCH_Q: f32 = 3.0;
const SWEEP_SPEED_HZ_PER_SEC: f32 = 200.0;
const PARK_FREQ: f32 = 20_000.0; // inaudible park position

// Prairie frequencies the brain expects. Notching these creates wrongness.
const EXPECTED_BANDS: [(&str, f32); 9] = [
    ("cricket_chirp",    4200.0),
    ("frog_trill",       2800.0),
    ("wind_low",          200.0),
    ("wind_mid",          800.0),
    ("insect_high",      6000.0),
    ("bird_song",        3200.0),
    ("owl_hoot",          400.0),
    ("mosquito_whine",   3800.0),
    ("ambient_presence", 1200.0),
];

// Always-active bands regardless of fauna contract
const ALWAYS_ACTIVE: [&str; 3] = ["wind_low", "wind_mid", "ambient_presence"];

// ── L5 LATERAL: Listener Profile → Absence Band Selection ────────────────────
// Design ref: docs/design-bible/LATERAL-CONNECTIONS-WIRING-LEDGER.md §L5
// A 6yo hears 4200-6000 Hz with full sensitivity → high-freq notch = visceral.
// A 60yo has presbycusis → those bands are naturally absent; low-freq notch = dread.
// The SAME game world sounds different; horror scales with what you CAN hear.

/// Age bracket for adaptive frequency band selection.
/// Controls which bands the AbsenceEngine preferentially notches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ListenerProfile {
    /// Child (6-12): full high-frequency sensitivity. Notch high bands for maximum impact.
    Child = 0,
    /// Teen (13-17): slight high-frequency rolloff begins. Balanced band selection.
    Teen = 1,
    /// Adult (18-55): standard hearing. Default balanced selection.
    Adult = 2,
    /// Elder (56+): presbycusis — high frequencies already diminished physiologically.
    /// Notch low-mid bands (200-1200 Hz) for sub-bass void dread.
    Elder = 3,
}

impl ListenerProfile {
    /// Parse from the JSON params "listener_age_bracket" field (0-3). Defaults to Adult.
    pub fn from_param(v: u64) -> Self {
        match v {
            0 => Self::Child,
            1 => Self::Teen,
            3 => Self::Elder,
            _ => Self::Adult,
        }
    }

    /// Weight multiplier for a band's frequency. Higher = more likely to be selected.
    /// Returns 0 if the band should be EXCLUDED for this profile.
    pub fn band_weight(self, freq_hz: f32) -> u8 {
        match self {
            Self::Child => {
                // Children: high bands hit hardest (4200+ Hz = full sensitivity)
                if freq_hz >= 3800.0 { 3 }
                else if freq_hz >= 2000.0 { 2 }
                else { 1 }
            }
            Self::Teen => {
                // Teens: balanced, slight high preference
                if freq_hz >= 3000.0 { 2 }
                else { 1 }
            }
            Self::Adult => {
                // Adults: balanced (no preference)
                1
            }
            Self::Elder => {
                // Elders: low-mid bands hit hardest (200-1200 Hz preserved hearing)
                // High bands are already physiologically absent — notching them is wasted.
                if freq_hz >= 4000.0 { 0 } // EXCLUDE: elder can't hear these anyway
                else if freq_hz <= 1200.0 { 3 } // PREFER: sub-bass and low-mid
                else { 1 }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Biquad notch — Audio EQ Cookbook
// ---------------------------------------------------------------------------
// w0   = 2π·f0/fs
// α    = sin(w0)/(2·Q)
// b0   = 1           / a0    (a0 = 1+α)
// b1   = −2·cos(w0)  / a0
// b2   = 1           / a0
// a1   = −2·cos(w0)  / a0
// a2   = (1−α)       / a0
// y[n] = b0·x[n] + b1·x[n-1] + b2·x[n-2] − a1·y[n-1] − a2·y[n-2]

#[derive(Clone, Copy, Default)]
struct BiquadMem {
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

#[derive(Clone, Copy)]
struct NotchCoeffs {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
}

fn notch_coeffs(freq_hz: f32, sample_rate: u32) -> NotchCoeffs {
    let w0 = 2.0 * PI * freq_hz / sample_rate as f32;
    let alpha = w0.sin() / (2.0 * NOTCH_Q);
    let cos_w0 = w0.cos();
    let a0 = 1.0 + alpha;
    NotchCoeffs {
        b0: 1.0 / a0,
        b1: -2.0 * cos_w0 / a0,
        b2: 1.0 / a0,
        a1: -2.0 * cos_w0 / a0,
        a2: (1.0 - alpha) / a0,
    }
}

#[inline]
fn apply_biquad(x: f32, mem: &mut BiquadMem, c: &NotchCoeffs) -> f32 {
    let y = c.b0 * x + c.b1 * mem.x1 + c.b2 * mem.x2
          - c.a1 * mem.y1 - c.a2 * mem.y2;
    mem.x2 = mem.x1; mem.x1 = x;
    mem.y2 = mem.y1; mem.y1 = y;
    y
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct AbsenceState {
    mem:        [BiquadMem; NOTCH_COUNT],
    current_hz: [f32; NOTCH_COUNT],
    target_hz:  [f32; NOTCH_COUNT],
    intensity:  f32,
    last_sr:    u32,
    frame:      u64,
}

impl AbsenceState {
    fn new() -> Self {
        Self {
            mem:        [BiquadMem::default(); NOTCH_COUNT],
            current_hz: [1000.0; NOTCH_COUNT],
            target_hz:  [PARK_FREQ; NOTCH_COUNT],
            intensity:  0.0,
            last_sr:    44100,
            frame:      0,
        }
    }
}

struct AbsenceCell(UnsafeCell<AbsenceState>);
// SAFETY: process() is only ever called from the single audio thread (SR&ED #35 invariant).
unsafe impl Sync for AbsenceCell {}

static STATE: OnceLock<AbsenceCell> = OnceLock::new();

fn state() -> &'static mut AbsenceState {
    let cell = STATE.get_or_init(|| AbsenceCell(UnsafeCell::new(AbsenceState::new())));
    // SAFETY: Single audio thread — no concurrent access possible.
    unsafe { &mut *cell.0.get() }
}

// ---------------------------------------------------------------------------
// Target selection — deterministic shuffle of active bands (zero-heap)
// ---------------------------------------------------------------------------

fn select_targets(
    params: &serde_json::Value,
    tick: u64,
    moon: u64,
    targets: &mut [f32; NOTCH_COUNT],
) {
    // L5: Read listener profile from params (default: Adult = balanced).
    let profile = ListenerProfile::from_param(
        params["listener_age_bracket"].as_u64().unwrap_or(2)
    );

    // Fixed-capacity candidate buffer — no heap on the DSP path.
    // L5: expanded from 9 to 32 to accommodate weighted profile copies (max 9×3=27).
    let mut candidates = [0.0_f32; 32];
    let mut n = 0usize;

    // Read the fauna contract by reference — never clone the map (was .clone()).
    let fauna_density = params["fauna_density"].as_object();

    for &(band_name, freq) in &EXPECTED_BANDS {
        // L5: Skip bands the listener profile excludes (weight = 0).
        if profile.band_weight(freq) == 0 {
            continue;
        }

        // Always-active bands
        if ALWAYS_ACTIVE.contains(&band_name) {
            candidates[n] = freq;
            n += 1;
            continue;
        }
        // Check if the related fauna species is present in the contract
        let strip = band_name
            .trim_end_matches("_chirp")
            .trim_end_matches("_trill")
            .trim_end_matches("_whine")
            .trim_end_matches("_hoot")
            .trim_end_matches("_song");
        if let Some(map) = fauna_density {
            if map.keys().any(|k| k.starts_with(strip)) {
                // L5: Insert weighted copies for profile-preferred bands.
                let repeats = profile.band_weight(freq);
                for _ in 0..repeats {
                    if n < candidates.len() {
                        candidates[n] = freq;
                        n += 1;
                    }
                }
            }
        }
    }

    if n == 0 {
        // Fallback: use first NOTCH_COUNT expected bands
        for (i, &(_, freq)) in EXPECTED_BANDS.iter().take(NOTCH_COUNT).enumerate() {
            targets[i] = freq;
        }
        return;
    }

    // Deterministic Fisher-Yates shuffle over the first n candidates.
    let mut lcg = tick.wrapping_mul(31).wrapping_add(moon.wrapping_mul(7));
    for i in (1..n).rev() {
        lcg = lcg.wrapping_mul(6_364_136_223_846_793_005)
                 .wrapping_add(1_442_695_040_888_963_407);
        let j = (lcg >> 33) as usize % (i + 1);
        candidates.swap(i, j);
    }

    for i in 0..NOTCH_COUNT {
        targets[i] = if i < n { candidates[i] } else { PARK_FREQ };
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn process(samples: &mut [f32], sample_rate: u32, params: &serde_json::Value) {
    let vibe:   f32  = params["vibe_position"].as_f64().unwrap_or(0.0) as f32;
    let walker: bool = params["walker_active"].as_bool().unwrap_or(false);
    let tick:   u64  = params["tick_id"].as_u64().unwrap_or(0);
    let moon:   u64  = params["moon"].as_u64().unwrap_or(0);

    // Walker always elevates intensity
    let target_intensity = if walker { vibe.max(0.8) } else { vibe };

    let buf_dur_sec = samples.len() as f32 / sample_rate as f32;

    let st = state();

    if st.last_sr != sample_rate {
        // Sample-rate change: reset biquad memory to prevent blow-up
        st.mem     = [BiquadMem::default(); NOTCH_COUNT];
        st.last_sr = sample_rate;
    }

    // Smooth intensity toward target (time-constant ~125ms)
    let smooth = 1.0 - (-8.0 * buf_dur_sec).exp();
    st.intensity += (target_intensity - st.intensity) * smooth;

    // Reselect target frequencies every 30 frames
    if st.frame % 30 == 0 {
        let mut new_targets = st.target_hz;
        select_targets(params, tick, moon, &mut new_targets);
        st.target_hz = new_targets;
    }
    st.frame += 1;

    // Sweep current frequencies toward targets
    let max_step = SWEEP_SPEED_HZ_PER_SEC * buf_dur_sec;
    for i in 0..NOTCH_COUNT {
        let diff = st.target_hz[i] - st.current_hz[i];
        if diff.abs() < max_step {
            st.current_hz[i] = st.target_hz[i];
        } else {
            st.current_hz[i] += diff.signum() * max_step;
        }
    }

    let intensity = st.intensity;
    if intensity < 0.05 {
        return;
    }

    // Precompute per-filter coefficients
    let coeffs: [NotchCoeffs; NOTCH_COUNT] = [
        notch_coeffs(st.current_hz[0].clamp(20.0, 20_000.0), sample_rate),
        notch_coeffs(st.current_hz[1].clamp(20.0, 20_000.0), sample_rate),
        notch_coeffs(st.current_hz[2].clamp(20.0, 20_000.0), sample_rate),
    ];

    // Apply notch chain in-place — blend between dry and notched by intensity.
    // depth = 0 at park frequency (minimal effect); = intensity at target (full notch)
    let depths: [f32; NOTCH_COUNT] = [
        if st.current_hz[0] < 19_000.0 { intensity } else { 0.0 },
        if st.current_hz[1] < 19_000.0 { intensity } else { 0.0 },
        if st.current_hz[2] < 19_000.0 { intensity } else { 0.0 },
    ];

    for s in samples.iter_mut() {
        let mut x = *s;
        for i in 0..NOTCH_COUNT {
            if depths[i] > 0.01 {
                let notched = apply_biquad(x, &mut st.mem[i], &coeffs[i]);
                // Blend: 0=dry, 1=fully notched
                x = x + depths[i] * (notched - x);
            }
        }
        *s = x;
    }
}
</content>
