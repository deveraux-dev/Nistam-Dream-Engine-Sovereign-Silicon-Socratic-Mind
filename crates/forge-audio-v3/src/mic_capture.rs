//! WASAPI microphone capture — records 16 kHz mono f32 samples on demand.
//!
//! Folded from broski-voice/src/audio.rs 2026-06-29.
//!
//! **Drum-2 liveness bridge (ARCH-009):** the mic callback publishes a rolling
//! 2048-sample window into a lock-free `TripleBuffer<Vec<f32>>` on every callback
//! batch. The UI/physics side reads via `try_take` — never blocks the audio
//! callback, never stalls the UI. This is the "mic confirms the beat came back"
//! return-path: continuous, lock-free, zero-alloc steady-state.
//!
//! The batch drain (`stop_and_drain`) still exists for offline STT/analysis — it
//! uses a separate `Arc<Mutex<Vec<f32>>>` that accumulates ONLY while armed
//! (`arm_batch` → `stop_and_drain`). Unarmed, the callback never touches it —
//! the studio's always-on bridge mic must not grow heap for the whole session
//! (64 KB/s unbounded before 2026-07-16). The TripleBuffer bridge is ALWAYS
//! live (publishes every callback).

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, StreamConfig};
use forge_hal::TripleBuffer;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Rolling window size for the TripleBuffer bridge (128ms @ 16kHz = YIN window).
const BRIDGE_WINDOW: usize = 2048;

/// Samples per 120 Hz tick (16000 / 120 ≈ 133).
const HOP: usize = 133;

pub struct MicCapture {
    /// Batch accumulation buffer (for stop_and_drain — offline STT/analysis).
    buffer: Arc<Mutex<Vec<f32>>>,
    /// Gate on the batch buffer: the callback pushes ONLY while true.
    /// Armed by `arm_batch`, disarmed by `stop_and_drain`.
    batch_armed: Arc<AtomicBool>,
    /// Lock-free TripleBuffer bridge: mic callback publishes, UI consumes.
    /// This is the Drum-2 liveness return-path (ARCH-009).
    bridge: Arc<TripleBuffer<Vec<f32>>>,
    /// Consumer-side generation cursor for try_take.
    bridge_gen: u64,
    /// Consumer-side front buffer (reused across takes — zero-alloc steady state).
    bridge_front: Vec<f32>,
    stream: Option<cpal::Stream>,
}

impl MicCapture {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        host.default_input_device()
            .ok_or_else(|| "mic_capture: no default input device".to_string())?;
        Ok(Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            batch_armed: Arc::new(AtomicBool::new(false)),
            bridge: Arc::new(TripleBuffer::new(vec![0.0f32; BRIDGE_WINDOW])),
            bridge_gen: 0,
            bridge_front: vec![0.0f32; BRIDGE_WINDOW],
            stream: None,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.start_on(None)
    }

    /// Start capture on a NAMED input device (`None` = system default). An
    /// unknown name fails LOUD listing every enumerable input device — the
    /// same no-silent-fallback ethos as the decimation check below.
    pub fn start_on(&mut self, device_name: Option<&str>) -> Result<(), String> {
        let host = cpal::default_host();
        let device = match device_name {
            Some(want) => host
                .input_devices()
                .map_err(|e| format!("mic_capture: enumerate input devices: {e}"))?
                .find(|d| d.name().is_ok_and(|n| n == want))
                .ok_or_else(|| {
                    let known: Vec<String> = crate::realtime::list_input_devices()
                        .into_iter()
                        .map(|d| d.name)
                        .collect();
                    format!("mic_capture: input device {want:?} not found — available input devices: {known:?}")
                })?,
            None => host
                .default_input_device()
                .ok_or_else(|| "mic_capture: no default input device".to_string())?,
        };

        let supported = device.default_input_config().map_err(|e| e.to_string())?;
        let sample_format = supported.sample_format();
        let native_rate = supported.sample_rate().0;
        let native_channels = supported.channels() as usize;

        if native_rate % TARGET_SAMPLE_RATE != 0 {
            return Err(format!(
                "mic_capture: native rate {native_rate} Hz is not an integer multiple of \
                 {TARGET_SAMPLE_RATE} Hz — resampling required (unsupported, no silent fallback)"
            ));
        }
        let decim = (native_rate / TARGET_SAMPLE_RATE) as usize;

        let config = StreamConfig {
            channels: supported.channels(),
            sample_rate: supported.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        eprintln!(
            "mic_capture: {native_rate} Hz ×{native_channels} {sample_format:?} → \
             decimate /{decim} → {TARGET_SAMPLE_RATE} Hz mono (bridge window={BRIDGE_WINDOW})"
        );

        let buffer = Arc::clone(&self.buffer);
        buffer.lock().unwrap().clear();
        let bridge = Arc::clone(&self.bridge);
        let armed = Arc::clone(&self.batch_armed);

        let err_fn = |e| eprintln!("mic_capture stream error: {e}");
        let stream = match sample_format {
            SampleFormat::F32 => {
                build_bridge_stream::<f32>(&device, &config, native_channels, decim, buffer, armed, bridge, err_fn)
            }
            SampleFormat::I16 => {
                build_bridge_stream::<i16>(&device, &config, native_channels, decim, buffer, armed, bridge, err_fn)
            }
            SampleFormat::U16 => {
                build_bridge_stream::<u16>(&device, &config, native_channels, decim, buffer, armed, bridge, err_fn)
            }
            other => Err(format!("mic_capture: unsupported sample format {other:?}")),
        }?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);
        Ok(())
    }

    /// Arm the batch buffer: from here until `stop_and_drain`, the callback
    /// accumulates every decimated sample for offline STT/analysis. Clears any
    /// stale samples first. Without this, the batch lane stays cold and the
    /// always-on bridge mic holds a fixed heap footprint.
    pub fn arm_batch(&mut self) {
        self.buffer.lock().unwrap().clear();
        self.batch_armed.store(true, Ordering::Relaxed);
    }

    /// Stop the capture stream and return all buffered f32 samples (offline batch).
    /// Disarms the batch lane. Returns empty if `arm_batch` was never called.
    pub fn stop_and_drain(&mut self) -> Vec<f32> {
        self.batch_armed.store(false, Ordering::Relaxed);
        self.stream = None;
        let mut buf = self.buffer.lock().unwrap();
        std::mem::take(&mut *buf)
    }

    /// **Drum-2 liveness bridge (ARCH-009).** Try to take the latest 2048-sample
    /// window from the lock-free TripleBuffer. Returns `true` if a fresh window
    /// was available (mic is live, beat came back); `false` means reuse-last
    /// (mic idle or callback hasn't published since last take — NOT an error).
    ///
    /// The returned samples are in `self.bridge_front` — call `bridge_samples()`
    /// to borrow them. This is the continuous mic→UI feed.
    pub fn take_bridge(&mut self) -> bool {
        match self.bridge.try_take(self.bridge_gen, &mut self.bridge_front) {
            Some(gen) => {
                self.bridge_gen = gen;
                true
            }
            None => false,
        }
    }

    /// Borrow the latest bridge window (2048 samples, 128ms @ 16kHz).
    /// Call `take_bridge()` first to refresh; this returns the last-taken front.
    pub fn bridge_samples(&self) -> &[f32] {
        &self.bridge_front
    }

    /// Drain one 120 Hz VocalFrame from the bridge window (non-blocking, lock-free).
    ///
    /// Uses the TripleBuffer bridge (NOT the Mutex buffer). Refreshes the bridge
    /// front via `take_bridge()`, then runs YIN pitch + RMS + onset on the window.
    /// Returns `None` if the bridge hasn't received fresh data yet.
    pub fn drain_frame(&mut self) -> Option<crate::vocal_frame::VocalFrame> {
        use crate::alchemy::pitch::yin_track;

        // Take fresh window from bridge (lock-free).
        self.take_bridge();

        // Check if we have meaningful data (not all zeros = mic not yet streaming).
        let samples = &self.bridge_front;
        let energy: f32 = samples.iter().map(|s| s * s).sum();
        if energy < 1e-10 {
            return None; // Silent / not yet streaming — don't emit zero-frames.
        }

        // -- F0 via YIN (single frame → single pitch) --
        let pitches = yin_track(samples, TARGET_SAMPLE_RATE, BRIDGE_WINDOW, BRIDGE_WINDOW, 0.15);
        let f0_hz = pitches.first().copied().unwrap_or(0.0);

        // -- RMS --
        const ONSET_WINDOW: usize = 512;
        let rms_window = &samples[..ONSET_WINDOW.min(samples.len())];
        let rms = (rms_window.iter().map(|s| s * s).sum::<f32>()
            / rms_window.len().max(1) as f32)
            .sqrt();

        // -- Onset detection (spectral flux proxy: energy jump) --
        let half = samples.len() / 2;
        let rms_first = (samples[..half].iter().map(|s| s * s).sum::<f32>()
            / half.max(1) as f32).sqrt();
        let rms_second = (samples[half..].iter().map(|s| s * s).sum::<f32>()
            / (samples.len() - half).max(1) as f32).sqrt();
        let onset = rms_second > rms_first * 1.8 && rms_second > 0.02;

        // -- Emotion (lightweight proxy from spectral features) --
        let zcr: usize = samples.windows(2)
            .filter(|w| w[0].signum() != w[1].signum())
            .count();
        let zcr_rate = zcr as f32 / samples.len() as f32;

        let arousal = (rms * 3.0).clamp(0.0, 1.0);
        let valence = (zcr_rate * 4.0).clamp(0.0, 1.0);
        let tension = if onset { 0.7 } else { (1.0 - rms * 2.0).clamp(0.2, 0.8) };
        let release = (1.0 - arousal).clamp(0.0, 1.0);

        Some(crate::vocal_frame::VocalFrame::from_dsp(
            f0_hz, rms, onset, valence, arousal, tension, release,
        ))
    }
}

/// Build an input stream that decimates to 16kHz mono AND publishes to both:
/// - The batch `Mutex<Vec<f32>>` (for stop_and_drain — offline analysis)
/// - The lock-free `TripleBuffer<Vec<f32>>` (for continuous mic→UI bridge — Drum-2)
///
/// The bridge publishes a rolling 2048-sample window on every callback. The ring
/// is a fixed-size circular buffer inside the callback closure — no heap growth
/// in steady state (the Vec inside TripleBuffer is pre-sized and reused via swap).
fn build_bridge_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    native_channels: usize,
    decim: usize,
    buffer: Arc<Mutex<Vec<f32>>>,
    batch_armed: Arc<AtomicBool>,
    bridge: Arc<TripleBuffer<Vec<f32>>>,
    err_fn: impl FnMut(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let mut phase = 0usize;
    let mut ch_acc = 0f32;
    let mut ch_n = 0usize;

    // Fixed-size circular ring for the bridge window (no heap growth in callback).
    let mut ring = vec![0.0f32; BRIDGE_WINDOW];
    let mut ring_pos = 0usize;
    // Pre-sized publish buffer (recycled by TripleBuffer::publish swap).
    let mut pub_buf = vec![0.0f32; BRIDGE_WINDOW];
    let mut samples_since_publish = 0usize;

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                for &s in data {
                    ch_acc += f32::from_sample(s);
                    ch_n += 1;
                    if ch_n == native_channels {
                        let mono = ch_acc / native_channels as f32;
                        ch_acc = 0.0;
                        ch_n = 0;
                        if phase == 0 {
                            // Write to batch buffer (for stop_and_drain) — ONLY
                            // while armed. Unarmed = zero heap growth: the
                            // always-on bridge mic ran this push unconditionally
                            // and leaked 64 KB/s for the whole studio session.
                            if batch_armed.load(Ordering::Relaxed) {
                                if let Ok(mut buf) = buffer.try_lock() {
                                    buf.push(mono);
                                }
                            }
                            // Write to bridge ring (circular, fixed-size).
                            ring[ring_pos] = mono;
                            ring_pos = (ring_pos + 1) % BRIDGE_WINDOW;
                            samples_since_publish += 1;

                            // Publish to TripleBuffer every HOP samples (120 Hz tick rate).
                            if samples_since_publish >= HOP {
                                samples_since_publish = 0;
                                // Linearize the circular ring into pub_buf.
                                let tail = &ring[ring_pos..];
                                let head = &ring[..ring_pos];
                                pub_buf[..tail.len()].copy_from_slice(tail);
                                pub_buf[tail.len()..].copy_from_slice(head);
                                // Lock-free publish (swap). We take pub_buf out via
                                // mem::take (leaving an empty Vec), publish it, and
                                // store the returned old buffer back. The empty Vec
                                // is ephemeral — publish returns immediately.
                                let to_publish = std::mem::take(&mut pub_buf);
                                pub_buf = bridge.publish(to_publish);
                                // Ensure pub_buf is right-sized for next fill.
                                if pub_buf.len() != BRIDGE_WINDOW {
                                    pub_buf.resize(BRIDGE_WINDOW, 0.0);
                                }
                            }
                        }
                        phase += 1;
                        if phase == decim {
                            phase = 0;
                        }
                    }
                }
            },
            err_fn,
            None,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // generator->consumer: list_input_devices (capture-side mirror of
    // list_output_devices) backs start_on's named-device resolution; an unknown
    // name fails LOUD. Headless-safe: no device is ever opened.
    // [BOARD: AUDIO-INPUT-ENUM]
    #[test]
    fn input_enumeration_backs_named_capture() {
        for d in crate::realtime::list_input_devices() {
            assert!(!d.sample_rates.is_empty(), "every device carries >=1 rate");
            assert!(d.channels >= 1);
        }
        let mut mic = MicCapture {
            buffer: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            batch_armed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            bridge: std::sync::Arc::new(TripleBuffer::new(vec![0.0f32; BRIDGE_WINDOW])),
            bridge_gen: 0,
            bridge_front: vec![0.0f32; BRIDGE_WINDOW],
            stream: None,
        };
        let err = mic
            .start_on(Some("nonexistent-device-6472-xyz"))
            .expect_err("unknown device name must fail LOUD");
        assert!(err.contains("mic_capture"), "error names the seam: {err}");
    }
}
