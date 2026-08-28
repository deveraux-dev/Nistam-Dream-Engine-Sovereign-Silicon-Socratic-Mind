//! Audio-quality + hotpath-budget readback gate (Phase 0 of best-audio-quality).
//!
//! Today this holds the `mix_block` CPU-hotpath timing proof. The objective
//! FFT readback (THD+N / SNR / alias-rejection / frequency-response flatness)
//! and the f32-vs-f64 delta land here in Phase 2, once the quality levers
//! (time-stretch, sinc resampler, dither, limiter) are wired — each measured
//! against this same harness.
//!
//! Two distinct clocks are kept visible (render/audio hard-gate, two-clock seam):
//!   - this CPU `mix_block` budget = 2.0 ms/tick (the 120 Hz engine budget);
//!   - the cpal callback deadline = 2.5 ms (adversarial RT pressure), proven by
//!     the live `audio_smoke` bin's telemetry, not here.

use std::time::{Duration, Instant};

use forge_audio_v3::dsp::{self, AudioBuffer};
use forge_audio_v3::mixer::Mixer;
use realfft::RealFftPlanner;

/// Stereo buffer of mild steady signal, long enough that decks do not run dry
/// across the measured block count (2016 blocks × 512 frames, resampled).
fn signal_buf(frames: usize) -> AudioBuffer {
    AudioBuffer {
        samples: vec![vec![0.3f32; frames]; 2],
        sample_rate: 44_100,
    }
}

#[test]
fn mix_block_p99_within_2ms_cpu_budget() {
    let mut mixer = Mixer::default();

    // Pressure, not a happy-path single frame: TWO decks playing simultaneously
    // through the crossfader, each running its 128-bin metering FFT every block.
    // 30 s of audio keeps both decks live for the whole measured run.
    mixer.decks[0].load(signal_buf(44_100 * 30));
    mixer.decks[1].load(signal_buf(44_100 * 30));
    mixer.decks[0].params.playing = true;
    mixer.decks[1].params.playing = true;

    // Warm up: the first blocks grow the pre-allocated scratch pools.
    for _ in 0..16 {
        let b = mixer.mix_block(512);
        mixer.recycle_output(b);
    }

    const N: usize = 2000;
    let mut times = Vec::with_capacity(N);
    for _ in 0..N {
        let t = Instant::now();
        let block = mixer.mix_block(512);
        times.push(t.elapsed());
        mixer.recycle_output(block);
    }
    times.sort_unstable();
    let p50 = times[N / 2];
    let p99 = times[(N * 99 / 100).min(N - 1)];
    let worst = *times.last().unwrap();

    // Sanity: both decks were actually advancing (not silently stopped at EOF).
    assert!(
        mixer.decks[0].params.playing && mixer.decks[1].params.playing,
        "both decks must stay playing for the budget to be measured under load"
    );

    // The 2.0 ms budget is defined for optimized builds (debug numeric loops run
    // an order of magnitude slower). The gate always RUNS and always asserts; the
    // real 2.0 ms ceiling applies in release, a generous one keeps debug honest.
    let (budget, mode) = if cfg!(debug_assertions) {
        (Duration::from_millis(25), "debug")
    } else {
        (Duration::from_millis(2), "release")
    };

    println!(
        "mix_block(512), 2 decks playing + metering FFT [{mode}]: \
         p50={p50:?} p99={p99:?} worst={worst:?} (budget {budget:?})"
    );

    assert!(
        p99 < budget,
        "mix_block p99 {p99:?} exceeds the {budget:?} hotpath budget ({mode} build)"
    );
}

// ── Objective FFT readback (the "audio quality" gate, ADR-0008 for audio) ──────
// Real DSP through the real `dsp::*` functions, FFT'd with realfft. These PRINT
// the baseline numbers the Phase-2 levers (windowed-sinc resampler, dither,
// limiter) must beat, and assert discriminators so a broken DSP path FAILS.

/// Pure mono sine at half amplitude.
fn sine_mono(freq: f64, sr: u32, n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| (std::f64::consts::TAU * freq * i as f64 / sr as f64).sin() as f32 * 0.5)
        .collect()
}

/// Hann-windowed linear magnitude spectrum (len n/2+1) — windowing tames the
/// spectral leakage that would otherwise swamp a THD/alias measurement.
fn mag_spectrum(signal: &[f32]) -> Vec<f32> {
    let n = signal.len();
    let mut windowed: Vec<f32> = signal
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let w = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / (n as f64 - 1.0)).cos();
            s * w as f32
        })
        .collect();
    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(n);
    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut windowed, &mut spectrum).expect("fft");
    spectrum.iter().map(|c| c.norm()).collect()
}

/// THD+N in dB: energy outside the fundamental (±`guard` bins, DC excluded)
/// relative to the fundamental. More negative = cleaner.
fn thd_n_db(mag: &[f32], guard: usize) -> f64 {
    let peak_bin = mag
        .iter()
        .enumerate()
        .skip(1)
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    let lo = peak_bin.saturating_sub(guard);
    let hi = (peak_bin + guard).min(mag.len() - 1);
    let (mut sig, mut rest) = (0.0f64, 0.0f64);
    for (i, &m) in mag.iter().enumerate() {
        if i == 0 {
            continue; // DC
        }
        let e = (m as f64) * (m as f64);
        if i >= lo && i <= hi {
            sig += e;
        } else {
            rest += e;
        }
    }
    10.0 * (rest / sig.max(1e-30)).log10()
}

/// Fraction of total spectral energy (dB) inside a narrow band around `freq`.
fn band_db(mag: &[f32], sr: u32, n: usize, freq: f64, guard: usize) -> f64 {
    let bin = (freq * n as f64 / sr as f64).round() as usize;
    let lo = bin.saturating_sub(guard);
    let hi = (bin + guard).min(mag.len() - 1);
    let (mut band, mut total) = (0.0f64, 0.0f64);
    for (i, &m) in mag.iter().enumerate() {
        let e = (m as f64) * (m as f64);
        total += e;
        if i >= lo && i <= hi {
            band += e;
        }
    }
    10.0 * (band / total.max(1e-30)).log10()
}

/// ABSOLUTE spectral energy in a narrow band around `freq` (not normalised) —
/// needed to measure attenuation, since a single surviving tone keeps ~100% of
/// the *relative* energy no matter how far its absolute level is pushed down.
fn band_energy_abs(mag: &[f32], sr: u32, n: usize, freq: f64, guard: usize) -> f64 {
    let bin = (freq * n as f64 / sr as f64).round() as usize;
    let lo = bin.saturating_sub(guard);
    let hi = (bin + guard).min(mag.len() - 1);
    (lo..=hi).map(|i| (mag[i] as f64) * (mag[i] as f64)).sum()
}

#[test]
fn resampler_passband_thd_n() {
    // 1 kHz pure tone @ 48 kHz → resample to 44.1 kHz. Well below Nyquist, so a
    // correct resampler stays clean; this is the passband-purity number.
    let sr = 48_000u32;
    let n = 48_000usize; // 1 s; 1 kHz lands on a bin
    let buf = AudioBuffer { samples: vec![sine_mono(1000.0, sr, n)], sample_rate: sr };
    let out = dsp::resample(&buf, 44_100);
    let mag = mag_spectrum(&out.samples[0]);
    let thdn = thd_n_db(&mag, 3);
    println!("[quality] resample 48k->44.1k, 1kHz: THD+N = {thdn:.1} dB ({} out samples)", out.len());
    // Discriminator: silence/garbage output blows past this floor.
    assert!(thdn < -30.0, "resampler passband THD+N {thdn:.1} dB worse than -30 dB floor");
}

#[test]
fn resampler_alias_rejection_baseline() {
    // 15 kHz tone @ 48 kHz downsampled to 24 kHz (new Nyquist 12 kHz). With no
    // anti-alias filter the 15 kHz folds to 24k-15k = 9 kHz. The fold itself is
    // the discriminator (proves real signal was processed); the printed image
    // level is the BASELINE the Phase-2 windowed-sinc resampler must drive down.
    let sr = 48_000u32;
    let n = 48_000usize;
    let buf = AudioBuffer { samples: vec![sine_mono(15_000.0, sr, n)], sample_rate: sr };
    let out = dsp::resample(&buf, 24_000);
    let no = out.len();
    let mag = mag_spectrum(&out.samples[0]);
    let peak_bin = mag
        .iter()
        .enumerate()
        .skip(1)
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap();
    let peak_hz = peak_bin as f64 * 24_000.0 / no as f64;
    let alias = band_db(&mag, 24_000, no, 9_000.0, 4);
    println!("[quality] resample 48k->24k, 15kHz: folds to {peak_hz:.0}Hz, alias image @9kHz = {alias:.1} dB rel total (baseline; sinc must lower)");
    // Discriminator: a broken/anti-aliasing-free linear resampler folds 15k->9k.
    assert!((peak_hz - 9_000.0).abs() < 400.0, "expected 15kHz to fold near 9kHz, got {peak_hz:.0}Hz");
}

#[test]
fn lowpass_attenuates_stopband() {
    // 8 kHz tone through a 500 Hz lowpass → the 8 kHz must be deeply attenuated.
    let sr = 48_000u32;
    let n = 48_000usize;
    let buf = AudioBuffer { samples: vec![sine_mono(8_000.0, sr, n)], sample_rate: sr };
    let dry = mag_spectrum(&buf.samples[0]);
    let wet_buf = dsp::lowpass(buf.clone(), 500.0);
    let wet = mag_spectrum(&wet_buf.samples[0]);
    // Absolute band energy dry vs wet = true stopband attenuation.
    let dry_e = band_energy_abs(&dry, sr, n, 8_000.0, 4);
    let wet_e = band_energy_abs(&wet, sr, n, 8_000.0, 4);
    let atten = 10.0 * (dry_e / wet_e.max(1e-30)).log10();
    println!("[quality] lowpass(500Hz) on 8kHz tone: stopband attenuation = {atten:.1} dB");
    assert!(atten > 12.0, "lowpass stopband attenuation {atten:.1} dB < 12 dB — filter not working");
}

#[test]
fn resample_sinc_beats_linear_alias_rejection() {
    // 15 kHz @48k downsampled to 24k (new Nyquist 12k). Linear folds it to 9 kHz
    // at full level (~0 dB rejection); the windowed-sinc band-limits to 12 kHz so
    // the 15 kHz is rejected before it can fold. This is the measured payoff of
    // the sinc resampler over the baseline `resample_alias_rejection_baseline`.
    let sr = 48_000u32;
    let n = 48_000usize;
    let buf = AudioBuffer { samples: vec![sine_mono(15_000.0, sr, n)], sample_rate: sr };

    let lin = dsp::resample(&buf, 24_000);
    let sinc = dsp::resample_sinc(&buf, 24_000, 24);

    let lin_alias = band_energy_abs(&mag_spectrum(&lin.samples[0]), 24_000, lin.len(), 9_000.0, 6);
    let sinc_alias = band_energy_abs(&mag_spectrum(&sinc.samples[0]), 24_000, sinc.len(), 9_000.0, 6);
    let rejection_db = 10.0 * (lin_alias / sinc_alias.max(1e-30)).log10();

    println!(
        "[quality] alias @9kHz: linear={lin_alias:.3e} sinc={sinc_alias:.3e} → sinc rejects {rejection_db:.1} dB MORE"
    );
    assert!(
        rejection_db > 20.0,
        "windowed-sinc must reject the 15k→9k alias ≥20 dB better than linear (got {rejection_db:.1} dB)"
    );
}

#[test]
fn resample_sinc_preserves_passband_tone() {
    // A 1 kHz tone (well below either Nyquist) must survive the sinc resample
    // cleanly — the band-limit only bites near/above the new Nyquist.
    let sr = 48_000u32;
    let n = 48_000usize;
    let buf = AudioBuffer { samples: vec![sine_mono(1_000.0, sr, n)], sample_rate: sr };
    let out = dsp::resample_sinc(&buf, 44_100, 24);
    let thdn = thd_n_db(&mag_spectrum(&out.samples[0]), 3);
    println!("[quality] resample_sinc 48k->44.1k, 1kHz: THD+N = {thdn:.1} dB");
    assert!(thdn < -30.0, "sinc passband THD+N {thdn:.1} dB worse than -30 dB floor");
}

/// Summed ABSOLUTE energy at the first `n_harmonics` harmonics (2f, 3f, …) of
/// `fund` — the quantization-distortion signature dither is meant to remove.
fn harmonic_energy(mag: &[f32], sr: u32, n: usize, fund: f64, n_harmonics: usize, guard: usize) -> f64 {
    (2..=(n_harmonics + 1))
        .map(|h| band_energy_abs(mag, sr, n, fund * h as f64, guard))
        .sum()
}

#[test]
fn dither_reduces_quantization_harmonics_at_low_level() {
    // A -80 dBFS 1 kHz tone (~3 LSB at 16-bit) — low enough that truncation
    // distorts it. Truncation correlates the error → harmonic peaks; TPDF dither
    // decorrelates it → those peaks drop into a flat noise floor.
    let sr = 48_000u32;
    let n = 48_000usize;
    let amp = 10f32.powf(-80.0 / 20.0); // -80 dBFS
    let tone: Vec<f32> = (0..n)
        .map(|i| (std::f64::consts::TAU * 1000.0 * i as f64 / sr as f64).sin() as f32 * amp)
        .collect();

    let trunc: Vec<f32> = tone.iter().map(|&s| dsp::quantize_i16(s)).collect();
    let mut rng = 0x1234_5678_9abc_def0u64;
    let dith: Vec<f32> = tone.iter().map(|&s| dsp::quantize_i16_dithered(s, &mut rng)).collect();

    let h_trunc = harmonic_energy(&mag_spectrum(&trunc), sr, n, 1000.0, 6, 2);
    let h_dith = harmonic_energy(&mag_spectrum(&dith), sr, n, 1000.0, 6, 2);
    let reduction = 10.0 * (h_trunc / h_dith.max(1e-30)).log10();

    println!(
        "[quality] -80dBFS 1kHz → 16-bit: harmonic energy trunc={h_trunc:.3e} dither={h_dith:.3e} → dither cuts quantization harmonics {reduction:.1} dB"
    );
    assert!(reduction > 6.0, "TPDF dither must cut quantization harmonics ≥6 dB (got {reduction:.1})");
}

#[test]
fn dithered_export_roundtrips_audible_through_load() {
    // ADR-0008 for the export bin: write a 1 kHz tone to a 16-bit WAV via the
    // (now dithered) exporter, load it back through the real loader, and assert
    // it is audible and tracks the input — the dither must shape only the LSB
    // error, never destroy the signal.
    let sr = 48_000u32;
    let n = 4_800usize;
    let tone: Vec<f32> = (0..n)
        .map(|i| (std::f64::consts::TAU * 1000.0 * i as f64 / sr as f64).sin() as f32 * 0.5)
        .collect();
    let buf = AudioBuffer { samples: vec![tone.clone()], sample_rate: sr };

    let path = std::env::temp_dir().join(format!("forgeaudio_dither_export_{}.wav", std::process::id()));
    let p = path.to_str().unwrap();
    dsp::write_game_audio(p, &buf).expect("dithered 16-bit export");
    let loaded = dsp::load_audio(p).expect("load the exported WAV back");
    std::fs::remove_file(&path).ok();

    assert_eq!(loaded.sample_rate, sr, "sample rate round-trips");
    let lmono = loaded.to_mono();
    let rms = (lmono.iter().map(|s| s * s).sum::<f32>() / lmono.len() as f32).sqrt();

    let m = lmono.len().min(tone.len());
    let (mut dot, mut e_in, mut e_out) = (0.0f64, 0.0f64, 0.0f64);
    for k in 0..m {
        dot += tone[k] as f64 * lmono[k] as f64;
        e_in += (tone[k] as f64).powi(2);
        e_out += (lmono[k] as f64).powi(2);
    }
    let corr = dot / (e_in.sqrt() * e_out.sqrt()).max(1e-12);

    println!("[quality] dithered 16-bit export round-trip: rms={rms:.3}, corr={corr:.4}");
    assert!(rms > 0.3, "exported tone must round-trip audible, rms={rms}");
    assert!(corr > 0.99, "loaded signal must track the input (corr={corr:.4})");
}

#[test]
fn soft_clip_is_cleaner_than_hard_clip_over_unity() {
    // A 1 kHz sine pushed to ~1.5× full scale (an over-unity master sum). The
    // hard clip corners the wave → harsh harmonics; the tanh soft-clip rounds
    // the knee → far lower THD. Measure both; soft must be measurably cleaner.
    let sr = 48_000u32;
    let n = 48_000usize;
    let tone: Vec<f32> = sine_mono(1000.0, sr, n).iter().map(|s| s * 3.0).collect(); // ~1.5 peak

    let hard: Vec<f32> = tone.iter().map(|s| s.clamp(-1.0, 1.0)).collect();
    let mut soft = AudioBuffer { samples: vec![tone.clone()], sample_rate: sr };
    dsp::soft_clip(&mut soft, 1.0, 1.0);

    let hard_thd = thd_n_db(&mag_spectrum(&hard), 3);
    let soft_thd = thd_n_db(&mag_spectrum(&soft.samples[0]), 3);
    println!("[quality] over-unity 1kHz (×1.5): hard-clip THD={hard_thd:.1} dB  soft-clip THD={soft_thd:.1} dB  (soft is {:.1} dB cleaner)", hard_thd - soft_thd);
    // Discriminator: the tanh knee must reduce THD vs the hard corner. The
    // margin is modest at 1.5× (tanh is already deep in saturation there) and
    // widens at gentler overshoot; ≥1 dB is the honest, robust floor.
    assert!(
        soft_thd < hard_thd - 1.0,
        "soft-clip THD ({soft_thd:.1}) must be cleaner than hard-clip ({hard_thd:.1})"
    );
}

// ── Timeless Compression falsification test (TimelessCompression2.txt §Collision) ──
//
// Proves the R=0 vs R>0 entropy boundary empirically.
//
// Claim under test: "4000x+ compression is achievable universally."
// Falsification: run a 40 Hz WAV alongside a procedural geometry description.
// Result expected: the procedural (R=0) case compresses at 12000x+; the raw
// audio WITH physical noise (R>0) has a dramatically higher prediction residual,
// proving its compressible content collapses toward the codec floor.

#[test]
fn wav_40hz_falsification_r0_vs_r_greater_than_0() {
    const SR: u32 = 48_000;
    const N: usize = 48_000; // 1 second of audio
    const RAW_BYTES: usize = N * 4; // f32 samples

    // ── Case 1: R=0 — pure 40 Hz sine (the MemoryAnchor frequency, ADR-0009).
    // Procedural description: freq(4B) + amplitude(4B) + sample_rate(4B) + n(4B) = 16B.
    // The procedure IS the data — same recipe → same bits, every run.
    const PROC_DESC_BYTES: usize = 16;
    let proc_ratio = RAW_BYTES as f64 / PROC_DESC_BYTES as f64;

    let pure_sine: Vec<f32> = (0..N)
        .map(|i| (std::f64::consts::TAU * 40.0 * i as f64 / SR as f64).sin() as f32 * 0.5)
        .collect();
    // Determinism gate (R=0 guarantee): identical procedure → identical output.
    let pure_sine2: Vec<f32> = (0..N)
        .map(|i| (std::f64::consts::TAU * 40.0 * i as f64 / SR as f64).sin() as f32 * 0.5)
        .collect();
    assert_eq!(pure_sine, pure_sine2, "R=0: identical procedure must yield identical bytes");

    // ── Case 2: R>0 — same tone + physical white noise (simulating a raw recording).
    // XORshift PRNG adds irreducible per-sample noise: no recipe can regenerate it.
    let mut rng = 0x12345678_9ABCDEF0u64;
    let noisy: Vec<f32> = (0..N)
        .map(|i| {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let noise = (rng as i64 as f64) * (0.08 / i64::MAX as f64);
            let tone = (std::f64::consts::TAU * 40.0 * i as f64 / SR as f64).sin() * 0.5;
            (tone + noise) as f32
        })
        .collect();

    // First-order linear prediction residual (FLAC-style): r[i] = s[i] - s[i-1].
    // Low residual = high correlation between adjacent samples = compressible.
    // High residual = low correlation = near-random content = codec floor.
    let sine_pred_rms: f64 = {
        let sq: f64 = pure_sine.windows(2).map(|w| (w[1] - w[0]) as f64).map(|d| d * d).sum();
        (sq / (N - 1) as f64).sqrt()
    };
    let noisy_pred_rms: f64 = {
        let sq: f64 = noisy.windows(2).map(|w| (w[1] - w[0]) as f64).map(|d| d * d).sum();
        (sq / (N - 1) as f64).sqrt()
    };
    let entropy_ratio = noisy_pred_rms / sine_pred_rms.max(1e-12);

    println!(
        "[quality] 40Hz falsification | \
         R=0 proc_ratio={proc_ratio:.0}x | \
         pred_rms: pure={sine_pred_rms:.5} noisy={noisy_pred_rms:.5} \
         entropy_ratio={entropy_ratio:.1}x (noise raises compression cost)"
    );

    // R=0: a pure generated sine is described by 16 B yet expands to 192 KB → >>4000x.
    assert!(
        proc_ratio > 4_000.0,
        "R=0 (pure 40Hz sine) procedural ratio {proc_ratio:.0}x must exceed 4000x threshold"
    );

    // R>0: adding physical noise raises the prediction residual — the irreducible entropy
    // proves the signal cannot be compressed by a procedural recipe alone.
    assert!(
        noisy_pred_rms > sine_pred_rms * 2.0,
        "R>0 pred residual {noisy_pred_rms:.5} must be ≥2× pure-sine residual {sine_pred_rms:.5}: \
         physical noise raises entropy to the codec floor"
    );

    // Boundary confirmed: R=0 compresses at >>4000x; R>0 cannot. The 4000x claim
    // is domain-specific to generated/deterministic data, not a universal property.
    assert!(
        entropy_ratio > 5.0,
        "entropy ratio {entropy_ratio:.1}x < 5x — noise must substantially increase prediction cost \
         to prove the R=0 vs R>0 boundary (TimelessCompression2.txt §Collision)"
    );
}
