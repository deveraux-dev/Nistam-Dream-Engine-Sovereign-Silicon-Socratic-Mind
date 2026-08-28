// @forge:allow_float — load-time STFT analysis only; never called from realtime callback.
//! Heuristic genre classification + `GenreRouter` byte-quantized LUT seam.
//!
//! **Two layers:**
//! 1. `detect_genre()` — Student-tier STFT heuristic (O(N²) DFT, load-time).
//! 2. `GenreRouter` — wraps [`forge_core::metarouter::MetaRouter`] (2026-08-19,
//!    Sean: ".s13 weights byte quantized prebaked LUTs dynamic and tunable";
//!    ARCH000 approved). The v2 donor's `forge_hal::expert_pool::{MoeRouter,
//!    MOE_QUERY_BYTES}` API doesn't exist in this shape in v3 — `MetaRouter`
//!    is v3's real, already-built, tested equivalent: loads a `.s13` file
//!    (magic `S13\x01`), 5D-trit-packs a query, routes to 1-of-7 experts via
//!    a precomputed byte-pair distance LUT (`TRIT_DIST_LUT`). Swap the `.s13`
//!    file to retune — no code change, no retraining API on this struct.
//!
//! All f32/f64 is confined to this module and the load path. `MetaRouter::route`
//! is integer-only (trit-distance LUT) — safe on any path.

use std::f64::consts::PI;
use std::path::Path;

use forge_core::metarouter::MetaRouter;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genre {
    DnB,
    Techno,
    Deep,
    Other,
}

impl Genre {
    pub fn name(self) -> &'static str {
        match self {
            Genre::DnB => "DnB/Jungle",
            Genre::Techno => "Techno",
            Genre::Deep => "Deep",
            Genre::Other => "Other",
        }
    }

    /// Maps to the bus contract: `DeckSnapshot.genre: Option<u8>` (0=DnB 1=Techno 2=Deep 3=Other).
    pub fn as_u8(self) -> u8 {
        match self {
            Genre::DnB => 0,
            Genre::Techno => 1,
            Genre::Deep => 2,
            Genre::Other => 3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenreResult {
    pub genre: Genre,
    pub confidence: f64,  // @forge:allow_float
    pub bpm: f64,         // @forge:allow_float
    pub sub_bass_ratio: f64,     // @forge:allow_float
    pub transient_density: f64,  // @forge:allow_float
    pub kick_regularity: f64,    // @forge:allow_float
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const WINDOW_SIZE: usize = 1024;
const HOP_SIZE: usize = 512;
const LOW_FREQ_CUTOFF_HZ: f64 = 100.0; // @forge:allow_float

// ---------------------------------------------------------------------------
// GenreRouter — .s13 byte-quantized LUT seam over MetaRouter
// ---------------------------------------------------------------------------

/// Genre dispatcher over a `.s13`-loaded [`MetaRouter`]. Load-time only (file
/// I/O), routing itself is the same integer trit-distance LUT every
/// `MetaRouter` consumer shares — no float, no alloc, in `route()`.
///
/// **Tunable, not trainable-here**: swap the `.s13` file
/// (`gemma-sidecar quantize-s13 pack` or `metarouter::build_s13_bytes` for a
/// hand-authored centroid set) and reload — this struct has no runtime
/// `train()`; centroid authorship lives offline, same split `MetaRouter`
/// already established.
///
/// `MetaRouter` is hard-fixed to 7 experts; `Genre` has 4 variants
/// (`as_u8()` 0..=3). Expert ids 0-3 map directly; 4-6 are reserved slots —
/// [`GenreRouter::route`] folds them to `Genre::Other` rather than panicking
/// or fabricating a 5th/6th/7th genre.
pub struct GenreRouter {
    inner: Option<MetaRouter>,
}

impl GenreRouter {
    /// No `.s13` loaded — [`route`](Self::route) always returns `None`. The
    /// honest starting state: nothing fabricated, nothing silently guessed.
    pub fn new() -> Self {
        Self { inner: None }
    }

    /// Load a `.s13` centroid file. `Err` (missing file, bad magic, wrong
    /// expert count) leaves the router in the untrained `None` state rather
    /// than aborting — a poisoned or absent genre model must not take down
    /// scanning, same reasoning `MetaRouter::route`'s own `Err` trap uses.
    pub fn load(path: &Path) -> Result<Self, String> {
        Ok(Self { inner: Some(MetaRouter::load(path)?) })
    }

    /// Route a heuristic `GenreResult`'s features to a genre via the
    /// `.s13`-loaded LUT. `None` when no `.s13` is loaded, or `MetaRouter`
    /// trapped an out-of-band sentinel byte mid-route.
    pub fn route(&self, result: &GenreResult) -> Option<Genre> {
        let router = self.inner.as_ref()?;
        let query = features_to_query(
            result.bpm,
            result.sub_bass_ratio,
            result.transient_density,
            result.kick_regularity,
        );
        let (expert, _margin) = router.route(&query).ok()?;
        Some(match expert {
            0 => Genre::DnB,
            1 => Genre::Techno,
            2 => Genre::Deep,
            _ => Genre::Other, // 3, and reserved slots 4-6
        })
    }

    /// `true` once a `.s13` file is loaded.
    pub fn is_loaded(&self) -> bool {
        self.inner.is_some()
    }
}

impl Default for GenreRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalise spectral features into a 4-dim query vector for
/// `MetaRouter::route` — it trit-packs and quantizes internally
/// (`pack_trits_into`), so this stops at plain normalized `f32`s rather than
/// hand-rolling a second byte-quantization scheme (L05: `MetaRouter` owns
/// that encoding, once).
///
/// **Centered on `classify()`'s own thresholds, not a flat 0.5** (bug found
/// + fixed 2026-08-19, live in this wire's own tests): `pack_trits_into`'s
/// trit is SIGN-only (`trit_of`: `>EPS`→+1, `<-EPS`→-1, else 0) — every
/// all-positive `0..1` feature quantizes to the same `+1` regardless of
/// magnitude, collapsing every archetype onto one trit pattern. A flat `0.5`
/// center didn't fix it either: `sub_bass_ratio` realistically never
/// approaches `0.5` (`classify()`'s own Deep threshold is `0.25`), so it
/// stayed negative-signed for every genre. Centering each feature on the
/// SAME threshold `classify()` already uses is both correct (the sign bit
/// now means "above/below the real decision boundary") and consistent — one
/// set of genre boundaries, not two that could drift apart.
fn features_to_query( // @forge:allow_float
    bpm: f64,
    sub_bass_ratio: f64,
    transient_density: f64,
    kick_regularity: f64,
) -> [f32; 4] {
    let norm_bpm = ((bpm - 80.0) / 160.0).clamp(0.0, 1.0) as f32 - 0.5; // @forge:allow_float
    let ctr_sub_bass = sub_bass_ratio.clamp(0.0, 1.0) as f32 - 0.25; // classify()'s Deep threshold
    let norm_density = (transient_density / 20.0).clamp(0.0, 1.0) as f32 - 0.3; // ~classify()'s DnB threshold (6.0/20)
    let ctr_kick = kick_regularity.clamp(0.0, 1.0) as f32 - 0.65; // classify()'s Techno threshold
    [norm_bpm, ctr_sub_bass, norm_density, ctr_kick]
}

// ---------------------------------------------------------------------------
// Student heuristic — detect_genre()
// ---------------------------------------------------------------------------

/// Detect genre from audio samples (f32 mono) and known BPM.
/// Load-time only — heap allocations here are intentional.
pub fn detect_genre(samples: &[f32], sample_rate: u32, bpm: f64) -> GenreResult {
    let sr = sample_rate as f64; // @forge:allow_float
    let duration_secs = samples.len() as f64 / sr; // @forge:allow_float

    if samples.is_empty() || duration_secs < 1.0 {
        return GenreResult {
            genre: Genre::Other,
            confidence: 0.0,
            bpm,
            sub_bass_ratio: 0.0,
            transient_density: 0.0,
            kick_regularity: 0.0,
        };
    }

    let (sub_bass_ratio, spectral_flux) = compute_spectral_features(samples, sample_rate);
    let onsets = detect_onsets(&spectral_flux);
    let transient_density = onsets.len() as f64 / duration_secs; // @forge:allow_float
    let kick_regularity = compute_kick_regularity(&onsets, sample_rate, bpm);
    let (genre, confidence) = classify(bpm, sub_bass_ratio, transient_density, kick_regularity);

    GenreResult { genre, confidence, bpm, sub_bass_ratio, transient_density, kick_regularity }
}

// ---------------------------------------------------------------------------
// Spectral features
// ---------------------------------------------------------------------------

fn compute_spectral_features(samples: &[f32], sample_rate: u32) -> (f64, Vec<f64>) {
    let sr = sample_rate as f64; // @forge:allow_float
    let half_window = WINDOW_SIZE / 2;
    let sub_bass_bin_limit = ((LOW_FREQ_CUTOFF_HZ * WINDOW_SIZE as f64) / sr).ceil() as usize; // @forge:allow_float

    let mut total_sub_bass = 0.0f64; // @forge:allow_float
    let mut total_energy = 0.0f64;   // @forge:allow_float
    let mut prev_magnitudes: Option<Vec<f64>> = None;
    let mut spectral_flux = Vec::new();

    let mut pos = 0;
    while pos + WINDOW_SIZE <= samples.len() {
        let magnitudes = compute_magnitude_spectrum(&samples[pos..pos + WINDOW_SIZE]);

        let sub_energy: f64 = magnitudes[..sub_bass_bin_limit.min(half_window)] // @forge:allow_float
            .iter()
            .map(|m| m * m)
            .sum();
        let frame_energy: f64 = magnitudes[..half_window].iter().map(|m| m * m).sum(); // @forge:allow_float

        total_sub_bass += sub_energy;
        total_energy += frame_energy;

        if let Some(ref prev) = prev_magnitudes {
            let flux: f64 = magnitudes // @forge:allow_float
                .iter()
                .zip(prev.iter())
                .map(|(curr, prev)| (curr - prev).max(0.0))
                .sum();
            spectral_flux.push(flux);
        }

        prev_magnitudes = Some(magnitudes);
        pos += HOP_SIZE;
    }

    let sub_bass_ratio = if total_energy > 1e-10 { // @forge:allow_float
        total_sub_bass / total_energy
    } else {
        0.0
    };

    (sub_bass_ratio, spectral_flux)
}

/// Naive DFT magnitude spectrum (Hann-windowed). Load-time only — O(N²) is acceptable here.
fn compute_magnitude_spectrum(frame: &[f32]) -> Vec<f64> {
    let n = frame.len();
    let half_n = n / 2;
    let mut magnitudes = Vec::with_capacity(half_n);

    for k in 0..half_n {
        let mut re = 0.0f64; // @forge:allow_float
        let mut im = 0.0f64; // @forge:allow_float
        let w = 2.0 * PI * k as f64 / n as f64; // @forge:allow_float

        for (i, &sample) in frame.iter().enumerate() {
            let hann = 0.5 * (1.0 - (2.0 * PI * i as f64 / (n - 1) as f64).cos()); // @forge:allow_float
            let windowed = sample as f64 * hann; // @forge:allow_float
            re += windowed * (w * i as f64).cos(); // @forge:allow_float
            im -= windowed * (w * i as f64).sin(); // @forge:allow_float
        }

        magnitudes.push((re * re + im * im).sqrt()); // @forge:allow_float
    }

    magnitudes
}

// ---------------------------------------------------------------------------
// Onset detection
// ---------------------------------------------------------------------------

fn detect_onsets(spectral_flux: &[f64]) -> Vec<usize> {
    if spectral_flux.is_empty() {
        return Vec::new();
    }

    let mean_flux: f64 = spectral_flux.iter().sum::<f64>() / spectral_flux.len() as f64; // @forge:allow_float
    let threshold = mean_flux * 2.0; // @forge:allow_float
    let min_gap = 4;

    let mut onsets = Vec::new();
    let mut last_onset: Option<usize> = None;

    for (i, &flux) in spectral_flux.iter().enumerate() {
        if flux > threshold {
            if let Some(last) = last_onset {
                if i - last < min_gap {
                    continue;
                }
            }
            onsets.push(i);
            last_onset = Some(i);
        }
    }

    onsets
}

// ---------------------------------------------------------------------------
// Kick regularity
// ---------------------------------------------------------------------------

fn compute_kick_regularity(onsets: &[usize], sample_rate: u32, bpm: f64) -> f64 {
    if onsets.len() < 4 {
        return 0.5; // @forge:allow_float — not enough data, centred default
    }

    let frames_per_sec = sample_rate as f64 / HOP_SIZE as f64; // @forge:allow_float
    let expected_interval = frames_per_sec * 60.0 / bpm; // @forge:allow_float

    let intervals: Vec<f64> = onsets.windows(2).map(|w| (w[1] - w[0]) as f64).collect(); // @forge:allow_float

    if intervals.is_empty() {
        return 0.5;
    }

    let mean_interval: f64 = intervals.iter().sum::<f64>() / intervals.len() as f64; // @forge:allow_float
    let variance: f64 = intervals
        .iter()
        .map(|&iv| { let d = iv - mean_interval; d * d }) // @forge:allow_float
        .sum::<f64>()
        / intervals.len() as f64;

    let std_dev = variance.sqrt(); // @forge:allow_float
    let normalized_std = std_dev / expected_interval.max(1.0); // @forge:allow_float
    (1.0 - normalized_std.min(1.0)).clamp(0.0, 1.0) // @forge:allow_float
}

// ---------------------------------------------------------------------------
// Classification
// ---------------------------------------------------------------------------

fn classify(bpm: f64, sub_bass_ratio: f64, transient_density: f64, kick_regularity: f64) -> (Genre, f64) {
    if bpm > 155.0 && transient_density > 6.0 {
        return (Genre::DnB, (transient_density / 12.0).min(1.0)); // @forge:allow_float
    }
    if (125.0..=145.0).contains(&bpm) && kick_regularity > 0.65 {
        let confidence = kick_regularity * 0.7 + (1.0 - (transient_density / 20.0).min(1.0)) * 0.3; // @forge:allow_float
        return (Genre::Techno, confidence.clamp(0.0, 1.0));
    }
    if (115.0..=135.0).contains(&bpm) && sub_bass_ratio > 0.25 {
        return (Genre::Deep, sub_bass_ratio.min(1.0)); // @forge:allow_float
    }
    (Genre::Other, 0.3)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_wave(freq: f32, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * duration_secs) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    fn kick_pattern(bpm: f64, sample_rate: u32, duration_secs: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * duration_secs) as usize;
        let samples_per_beat = (sample_rate as f64 * 60.0 / bpm) as usize;
        let kick_len = sample_rate as usize / 20;
        let mut samples = vec![0.0f32; n];
        let mut pos = 0;
        while pos < n {
            for i in 0..kick_len.min(n - pos) {
                let t = i as f32 / sample_rate as f32;
                let env = (1.0 - t * 20.0).max(0.0);
                samples[pos + i] = (2.0 * std::f32::consts::PI * 60.0 * t).sin() * env;
            }
            pos += samples_per_beat;
        }
        samples
    }

    #[test]
    fn test_high_bpm_classifies_dnb() {
        let mut samples = kick_pattern(174.0, 44100, 10.0);
        let hat_interval = 44100 / 12;
        for i in (0..samples.len()).step_by(hat_interval) {
            for j in 0..200.min(samples.len() - i) {
                samples[i + j] += 0.3 * (j as f32 * 0.1).sin() * (1.0 - j as f32 / 200.0);
            }
        }
        let result = detect_genre(&samples, 44100, 174.0);
        assert_eq!(result.genre, Genre::DnB, "174 BPM + high transients = DnB");
    }

    #[test]
    fn test_mid_bpm_regular_kick_techno() {
        let samples = kick_pattern(135.0, 44100, 10.0);
        let result = detect_genre(&samples, 44100, 135.0);
        assert_eq!(result.genre, Genre::Techno, "135 BPM + regular kicks = Techno");
    }

    #[test]
    fn test_low_bpm_heavy_sub_deep() {
        let mut samples = kick_pattern(125.0, 44100, 10.0);
        for (i, s) in samples.iter_mut().enumerate() {
            *s += 0.5 * (2.0 * std::f32::consts::PI * 45.0 * i as f32 / 44100.0).sin();
        }
        let result = detect_genre(&samples, 44100, 125.0);
        assert_eq!(result.genre, Genre::Deep, "125 BPM + heavy sub = Deep");
    }

    #[test]
    fn test_ambiguous_fallback() {
        let result = detect_genre(&sine_wave(1000.0, 44100, 5.0), 44100, 100.0);
        assert_eq!(result.genre, Genre::Other, "100 BPM sine = Other");
    }

    #[test]
    fn test_confidence_range() {
        let samples = kick_pattern(174.0, 44100, 5.0);
        let result = detect_genre(&samples, 44100, 174.0);
        assert!(result.confidence >= 0.0 && result.confidence <= 1.0,
            "Confidence {} out of range", result.confidence);
    }

    #[test]
    fn test_empty_input() {
        let result = detect_genre(&[], 44100, 120.0);
        assert_eq!(result.genre, Genre::Other);
        assert_eq!(result.confidence, 0.0);
    }

    #[test]
    fn test_kick_regularity_perfect() {
        let onsets: Vec<usize> = (0..20).map(|i| i * 100).collect();
        let reg = compute_kick_regularity(&onsets, 44100, 130.0);
        assert!(reg > 0.9, "Perfect regularity should be > 0.9, got {}", reg);
    }

    #[test]
    fn test_genre_as_u8_matches_bus_contract() {
        assert_eq!(Genre::DnB.as_u8(), 0);
        assert_eq!(Genre::Techno.as_u8(), 1);
        assert_eq!(Genre::Deep.as_u8(), 2);
        assert_eq!(Genre::Other.as_u8(), 3);
    }

    #[test]
    fn test_genre_router_untrained_returns_none() {
        let router = GenreRouter::new();
        assert!(!router.is_loaded());
        let result = GenreResult {
            genre: Genre::Other, confidence: 0.0, bpm: 135.0,
            sub_bass_ratio: 0.2, transient_density: 4.0, kick_regularity: 0.8,
        };
        assert!(router.route(&result).is_none(), "no .s13 loaded => always None, never a guess");
    }

    /// Builds a real `.s13` file (magic, header, 7 trit-packed centroids via
    /// `metarouter::build_s13_bytes`/`pack_trits` — the same producer path
    /// `gemma-sidecar quantize-s13 pack` uses) with 3 named genre archetypes
    /// in slots 0-2 and neutral filler in 3-6, loads it, and proves routing
    /// against it end to end — not a mock, the real `.s13` byte format.
    fn build_test_s13(dir: &std::path::Path, suffix: &str) -> std::path::PathBuf {
        use forge_core::metarouter::{build_s13_bytes, pack_trits, trit_bytes_needed};

        let d_model: u16 = 4;
        let bpc = trit_bytes_needed(d_model) as usize;
        // Realistic per-genre feature archetypes, run through the SAME
        // `features_to_query` centering production code uses — not
        // hand-picked trit patterns. (bpm, sub_bass_ratio, transient_density, kick_regularity)
        let raw_archetypes: [(f64, f64, f64, f64); 3] = [
            (174.0, 0.05, 8.0, 0.30), // DnB: fast, thin sub, dense/irregular
            (135.0, 0.15, 3.0, 0.85), // Techno: steady mid-tempo four-on-floor
            (125.0, 0.40, 2.0, 0.60), // Deep: slower, heavy sub-bass
        ];
        let mut centroids = Vec::with_capacity(7 * bpc);
        for (bpm, sub, dens, kick) in raw_archetypes {
            centroids.extend(pack_trits(&features_to_query(bpm, sub, dens, kick), bpc));
        }
        for _ in 0..4 {
            centroids.extend(pack_trits(&[0.0, 0.0, 0.0, 0.0], bpc)); // 3-6 reserved
        }
        let bytes = build_s13_bytes(d_model, [0.0f32; 7], &centroids);
        let path = dir.join(format!("genre-test-{}-{}.s13", std::process::id(), suffix));
        std::fs::write(&path, &bytes).unwrap();
        path
    }

    #[test]
    fn genre_router_loads_a_real_s13_and_routes_the_trained_archetype() {
        let dir = std::env::temp_dir();
        let path = build_test_s13(&dir, "load-archetype");

        let router = GenreRouter::load(&path).unwrap();
        assert!(router.is_loaded());

        // Techno archetype features, close to slot 1's centroid.
        let result = GenreResult {
            genre: Genre::Other, confidence: 0.0, bpm: 136.0,
            sub_bass_ratio: 0.14, transient_density: 3.1, kick_regularity: 0.88,
        };
        assert_eq!(router.route(&result), Some(Genre::Techno), "must route to the nearest real .s13 centroid");

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn genre_router_load_missing_file_is_an_honest_err() {
        let missing = std::env::temp_dir().join(format!("genre-does-not-exist-{}.s13", std::process::id()));
        assert!(GenreRouter::load(&missing).is_err(), "a missing .s13 must Err, never silently route");
    }

    #[test]
    fn detect_genre_feeds_router_end_to_end() {
        let dir = std::env::temp_dir();
        let path = build_test_s13(&dir, "end-to-end");
        let router = GenreRouter::load(&path).unwrap();

        let samples = kick_pattern(135.0, 44100, 10.0);
        let heuristic = detect_genre(&samples, 44100, 135.0);
        assert_eq!(heuristic.genre, Genre::Techno, "heuristic classifier alone");

        // The LUT router, fed the heuristic's own extracted features, must agree.
        assert_eq!(router.route(&heuristic), Some(Genre::Techno), "LUT router must agree with the heuristic on the same features");

        std::fs::remove_file(&path).ok();
    }
}
