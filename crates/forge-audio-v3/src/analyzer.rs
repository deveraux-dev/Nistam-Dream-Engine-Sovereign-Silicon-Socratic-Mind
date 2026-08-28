//! Analyzer thread — consumes PCM tap from feeder, computes semantic metrics.
//!
//! Decoupled from audio pump: if analyzer is slow, feeder drops tap samples (never blocks).
//! Ported from dreadpirateradio quarry (E:/airgap/13forge/dreadpiratedev).

use std::sync::Arc;
use arc_swap::ArcSwap;

/// Semantic audio metrics computed off the hot realtime path.
#[derive(Clone, Debug, Default)]
pub struct AnalyzerSnapshot {
    /// Spectral centroid — brightness proxy, 0.0 (dark/bass) to 1.0 (bright/treble).
    pub spectral_centroid: f32,
    /// Vocal energy — energy in speech-intelligibility band (300–3400 Hz), 0.0–1.0.
    pub vocal_energy: f32,
    /// Dominant spectral band index, 0–7 (8 bands across Nyquist).
    pub dominant_band: usize,
    /// Spectral tilt — negative = bass-heavy, positive = treble-heavy, -1.0..1.0.
    pub spectral_tilt: f32,
    /// Beat phase 0.0–1.0 (refined externally via MixerSnapshot; 0.0 until wired).
    pub beat_phase: f32,
    /// Onset strength — energy derivative, proxy for transients/hits, 0.0–1.0.
    pub onset_strength: f32,
}

/// Spawn the analyzer thread. Returns immediately.
/// `tap_rx` — rtrb consumer receiving mono f32 samples from the feeder.
/// `out`    — ArcSwap cell the render thread reads for the latest snapshot.
pub fn spawn_analyzer(
    tap_rx: rtrb::Consumer<f32>,
    out: Arc<ArcSwap<AnalyzerSnapshot>>,
) {
    std::thread::Builder::new()
        .name("forge-audio::analyzer".into())
        .spawn(move || analyzer_loop(tap_rx, out))
        .expect("failed to spawn analyzer thread");
}

/// Point the analyzer at a live capture device: take the ring's read end out
/// of `handle` and spawn the analyzer thread on it.
///
/// This is the join the two halves were missing. `input_capture` produced mono
/// f32 into an rtrb ring and `spawn_analyzer` consumed exactly that, but the
/// handle only ever lent its consumer by `&mut` while the spawn needs an owned
/// one — so nothing could connect them.
///
/// `handle` must outlive the analyzer: dropping it stops the cpal stream and
/// the analyzer then reads an empty ring forever.
pub fn spawn_analyzer_on_capture(
    handle: &mut crate::input_capture::InputCaptureHandle,
    out: Arc<ArcSwap<AnalyzerSnapshot>>,
) -> Result<(), String> {
    let tap = handle
        .take_consumer()
        .ok_or("analyzer: this capture handle's consumer was already taken")?;
    spawn_analyzer(tap, out);
    Ok(())
}

fn analyzer_loop(mut tap: rtrb::Consumer<f32>, out: Arc<ArcSwap<AnalyzerSnapshot>>) {
    let mut buf = vec![0.0f32; 1024];
    let mut fft = vec![0.0f32; 64];
    let mut prev_energy = 0.0f32;

    loop {
        let avail = tap.slots();
        if avail < 512 {
            std::thread::sleep(std::time::Duration::from_millis(5));
            continue;
        }
        let read = avail.min(buf.len());
        for s in &mut buf[..read] {
            *s = tap.pop().unwrap_or(0.0);
        }

        // Naive 64-bin magnitude spectrum (DFT approximation, no alloc on hot path).
        let step = read / 64;
        if step == 0 { continue; }
        for bin in 0..64 {
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for k in 0..step {
                let idx = bin * step + k;
                let angle = -2.0 * std::f32::consts::PI * bin as f32 * k as f32 / step as f32;
                re += buf[idx] * angle.cos();
                im += buf[idx] * angle.sin();
            }
            fft[bin] = (re * re + im * im).sqrt() / step as f32;
        }

        let snap = compute_metrics(&fft, &buf[..read], &mut prev_energy);
        out.store(Arc::new(snap));
    }
}

#[cfg(test)]
mod capture_join_tests {
    use super::*;
    use crate::input_capture::null_input_capture;

    /// The join exists and moves the ring's read end exactly once.
    #[test]
    fn the_analyzer_can_be_pointed_at_a_capture_handle() {
        let mut handle = null_input_capture(1024, 48_000);
        assert!(handle.has_consumer(), "a fresh handle owns the read end");

        let out = Arc::new(ArcSwap::from_pointee(AnalyzerSnapshot::default()));
        spawn_analyzer_on_capture(&mut handle, Arc::clone(&out)).expect("the join must connect");

        assert!(!handle.has_consumer(), "the read end moved to the analyzer");
        assert!(handle.consumer_mut().is_none(), "and the lending path says so too");
    }

    /// A SPSC ring has exactly one reader — the second attempt refuses instead
    /// of handing out a second consumer or panicking.
    #[test]
    fn a_second_join_refuses() {
        let mut handle = null_input_capture(1024, 48_000);
        let out = Arc::new(ArcSwap::from_pointee(AnalyzerSnapshot::default()));
        spawn_analyzer_on_capture(&mut handle, Arc::clone(&out)).expect("first join");
        let err = spawn_analyzer_on_capture(&mut handle, out).expect_err("second must refuse");
        assert!(err.contains("already taken"), "{err}");
    }

    /// Before the join, the handle still lends its consumer the old way — the
    /// feeder-thread path is not broken by making the field optional.
    #[test]
    fn an_untaken_handle_still_lends_its_consumer() {
        let mut handle = null_input_capture(1024, 48_000);
        assert!(handle.consumer_mut().is_some());
    }

    /// Samples actually reach the metrics. `null_input_capture` forgets its
    /// producer (nothing can feed it), so this drives the same consumer type
    /// the join hands over, from a ring this test owns, and waits for the
    /// analyzer to publish something other than the default snapshot.
    ///
    /// NOTE: `analyzer_loop` has no shutdown signal, so the thread outlives
    /// the test. Bounded poll, no sleep-then-assert.
    #[test]
    fn a_fed_ring_moves_the_published_snapshot_off_default() {
        let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(4096);
        // A bright-ish tone: enough non-zero spectrum for the metrics to bite.
        for i in 0..2048 {
            let phase = (i % 32) as f32 / 32.0;
            let s = if phase < 0.5 { 0.6 } else { -0.6 };
            producer.push(s).expect("ring has room");
        }

        let out = Arc::new(ArcSwap::from_pointee(AnalyzerSnapshot::default()));
        spawn_analyzer(consumer, Arc::clone(&out));

        let mut moved = false;
        for _ in 0..200 {
            let snap = out.load();
            if snap.spectral_centroid != 0.0 || snap.dominant_band != 0 || snap.onset_strength != 0.0
            {
                moved = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(moved, "the analyzer never published metrics from a fed ring");
    }
}

fn compute_metrics(fft: &[f32], pcm: &[f32], prev_energy: &mut f32) -> AnalyzerSnapshot {
    let n = fft.len().max(1);

    let (mut ws, mut te) = (0.0f32, 0.0f32);
    for (i, &v) in fft.iter().enumerate() { ws += i as f32 * v; te += v; }
    let spectral_centroid = if te > 0.001 { (ws / te / n as f32).clamp(0.0, 1.0) } else { 0.5 };

    let vlo = 4.min(n); let vhi = 24.min(n);
    let vocal_energy = (fft[vlo..vhi].iter().sum::<f32>() / (vhi - vlo).max(1) as f32).clamp(0.0, 1.0);

    let mut bands = [0.0f32; 8];
    for (i, &v) in fft.iter().enumerate() { bands[(i / 8).min(7)] += v; }
    let dominant_band = bands.iter().enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i).unwrap_or(0);

    let half = n / 2;
    let lo: f32 = fft[..half].iter().sum();
    let hi: f32 = fft[half..].iter().sum();
    let spectral_tilt = if lo + hi > 0.001 { ((hi - lo) / (hi + lo)).clamp(-1.0, 1.0) } else { 0.0 };

    let energy: f32 = pcm.iter().map(|&s| s * s).sum::<f32>() / pcm.len().max(1) as f32;
    let onset_strength = (energy - *prev_energy).max(0.0).clamp(0.0, 1.0);
    *prev_energy = energy * 0.9 + *prev_energy * 0.1;

    AnalyzerSnapshot {
        spectral_centroid,
        vocal_energy,
        dominant_band,
        spectral_tilt,
        beat_phase: 0.0,
        onset_strength,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_metrics_silence() {
        let fft = vec![0.0f32; 64];
        let pcm = vec![0.0f32; 512];
        let mut prev = 0.0;
        let snap = compute_metrics(&fft, &pcm, &mut prev);
        assert_eq!(snap.spectral_centroid, 0.5);
        assert_eq!(snap.vocal_energy, 0.0);
        assert_eq!(snap.onset_strength, 0.0);
    }

    #[test]
    fn compute_metrics_low_frequency_dominant() {
        let mut fft = vec![0.0f32; 64];
        fft[0] = 1.0; fft[1] = 0.8; // energy in bin 0 (band 0)
        let pcm = vec![0.0f32; 64];
        let mut prev = 0.0;
        let snap = compute_metrics(&fft, &pcm, &mut prev);
        assert_eq!(snap.dominant_band, 0);
        assert!(snap.spectral_centroid < 0.5, "low-freq dominant → centroid < 0.5");
    }

    #[test]
    fn onset_strength_rises_on_transient() {
        let mut fft = vec![0.0f32; 64];
        fft[10] = 0.5;
        let pcm = vec![0.5f32; 64];
        let mut prev = 0.0;
        let snap = compute_metrics(&fft, &pcm, &mut prev);
        assert!(snap.onset_strength > 0.0, "transient should produce positive onset");
    }
}
