//! Real-time audio output via cpal. Lock-free SPSC ring buffer path only.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

/// Raise the calling thread to time-critical priority (Windows only). The audio
/// feeder thread (`real_feeder_loop`) calls this on entry: its own per-cycle work
/// is microseconds against an 8.3ms publish budget, so underruns under load are
/// scheduler starvation, not slow code — ring priming (bus.rs) closes the startup
/// race, this closes the steady-state one. Best-effort: a failed call leaves the
/// thread at normal priority, never panics (AUDIO-ONE-BUS fix-phase item 1).
#[cfg(windows)]
#[allow(unsafe_code)]
pub fn raise_audio_thread_priority() {
    use windows_sys::Win32::System::Threading::{
        GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
    };
    // SAFETY: FFI call with two scalar arguments (both HANDLE and i32).
    // No pointers, no buffers, no memory allocated or freed by this call.
    // Return value is checked (0 = failure, nonzero = success) and logged.
    // Thread priority elevation is best-effort; failure is not fatal.
    unsafe {
        if SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL) == 0 {
            eprintln!("[audio] SetThreadPriority(TIME_CRITICAL) failed — feeder stays at normal priority");
        }
    }
}

#[cfg(not(windows))]
pub fn raise_audio_thread_priority() {}

// ════════════════════════════════════════════════════════════════════════════
// HARD GATE — TYPED PLAYBACK GUARD
// ════════════════════════════════════════════════════════════════════════════
//
// `PlaybackHandle` is the ONLY way to obtain a real-time audio producer.
// The `cpal::Stream` is held in a private field and cannot be extracted.
// Rust's borrow checker enforces audio device lifetime: the only way to
// stop playback is to drop the entire handle.
//
// Burned origin (2026-04-09): the now-retired forge-app-winamp/src/player.rs:45 used
// `let (_stream, prod, ...) = ...` inside a match arm. The `_stream`
// binding went out of scope at the end of the arm, dropping cpal::Stream
// and silently closing the audio device. The whole player ran perfectly
// — mixer math, FFT, IPC, snapshots — and zero sound reached the speakers.
// No compile error, no runtime warning.
//
// This guard makes that bug class STRUCTURALLY IMPOSSIBLE. The same
// pattern protects forge-app-daw, forge-app-studio, and any future audio
// app from typo'd destructures, agent-generated code, or copy-paste errors.
//
// See: feedback_verify_mute_before_audio_test.md
//      feedback_hard_gate_new_systems.md

/// Owns a real-time audio playback resource.
///
/// `cpal::Stream` is held privately — there is no public accessor and no
/// way to destructure it out. Drop the entire `PlaybackHandle` to stop
/// playback. The borrow checker prevents the bug class where a caller
/// drops the stream while holding the producer.
///
/// `PlaybackHandle` is `!Send` because `cpal::Stream` is `!Send`. For the
/// stream-on-init-thread, producer-on-feeder-thread pattern (Dead Drop,
/// Studio), call [`PlaybackHandle::split`] to obtain a `!Send` `StreamKeeper`
/// that stays on the init thread and a `Send` `FeederBundle` that moves
/// into the audio feeder thread.
///
/// For the co-located pattern (where stream and producer live in the same
/// thread, e.g. the ForgeAudio player), hold the whole handle in scope and use
/// [`PlaybackHandle::producer_mut`] to push samples.
pub struct PlaybackHandle {
    // PRIVATE — never expose. Held alive for the lifetime of the handle.
    // `None` when DAW_NO_AUDIO=1 (no real device opened).
    stream_guard: Option<cpal::Stream>,
    producer: rtrb::Producer<f32>,
    flush: Arc<AtomicBool>,
    device_error: Arc<AtomicBool>,
    channels: usize,
    sample_rate: u32,
    underruns: Arc<AtomicU64>,
}

impl PlaybackHandle {
    /// Push samples to the audio device. Use only inside the thread that
    /// owns this handle (typically the audio feeder thread or, for the
    /// co-located pattern, the player thread).
    pub fn producer_mut(&mut self) -> &mut rtrb::Producer<f32> {
        &mut self.producer
    }

    pub fn channels(&self) -> usize { self.channels }
    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn flush_flag(&self) -> Arc<AtomicBool> { self.flush.clone() }
    pub fn device_error_flag(&self) -> Arc<AtomicBool> { self.device_error.clone() }
    pub fn underrun_counter(&self) -> Arc<AtomicU64> { self.underruns.clone() }

    /// Returns true when a real cpal output device is open. Returns false
    /// under `DAW_NO_AUDIO=1` (the hard-gate test mode).
    pub fn is_live(&self) -> bool { self.stream_guard.is_some() }

    /// Split the handle into a `!Send` `StreamKeeper` (must stay on the
    /// init thread or be intentionally leaked) and a `Send` `FeederBundle`
    /// (move into the audio feeder thread).
    ///
    /// Use this only when stream and producer cannot be co-located. The
    /// co-located pattern is preferred — it keeps audio device lifetime
    /// trivially correct without any !Send dance.
    #[allow(unsafe_code)]
    pub fn split(self) -> (StreamKeeper, FeederBundle) {
        // Suppress the Drop log on the source handle — the resources are
        // moving into two new guards, not actually being released.
        let mut me = std::mem::ManuallyDrop::new(self);
        let stream = me.stream_guard.take();
        // SAFETY: ManuallyDrop wrapping guarantees Drop is suppressed for `me`.
        // Each field read via ptr::read is a valid, initialized value.
        // We move every owned field exactly once; order is:
        //   producer (Arc<rtrb::Producer<f32>>), flush (Arc<AtomicBool>),
        //   device_error (Arc<AtomicBool>), underruns (Arc<AtomicU64>).
        // After these four reads + channels/sample_rate copy, `me` goes out
        // of scope without invoking Drop, so no double-drop or use-after-free.
        let producer = unsafe { std::ptr::read(&me.producer) };
        let flush = unsafe { std::ptr::read(&me.flush) };
        let device_error = unsafe { std::ptr::read(&me.device_error) };
        let underruns = unsafe { std::ptr::read(&me.underruns) };
        let channels = me.channels;
        let sample_rate = me.sample_rate;

        let keeper = StreamKeeper { stream_guard: stream };
        let bundle = FeederBundle {
            producer,
            flush,
            device_error,
            channels,
            sample_rate,
            underruns,
        };
        (keeper, bundle)
    }
}

impl Drop for PlaybackHandle {
    fn drop(&mut self) {
        if self.stream_guard.is_some() {
            eprintln!("[forge-audio] PlaybackHandle dropped — audio device closed");
        }
    }
}

/// `!Send` half of a split [`PlaybackHandle`]. Owns the `cpal::Stream`.
/// Must stay on the thread that called `start_playback_lockfree`, OR be
/// `Box::leak`'d if you need cross-thread reconnect (the Dead Drop pattern).
///
/// Drop this to stop the audio device. The stream cannot be extracted.
pub struct StreamKeeper {
    stream_guard: Option<cpal::Stream>,
}

impl Drop for StreamKeeper {
    fn drop(&mut self) {
        if self.stream_guard.is_some() {
            eprintln!("[forge-audio] StreamKeeper dropped — audio device closed");
        }
    }
}

/// `Send` half of a split [`PlaybackHandle`]. Owns the producer plus all
/// the metadata flags. Move this into the audio feeder thread.
pub struct FeederBundle {
    pub producer: rtrb::Producer<f32>,
    pub flush: Arc<AtomicBool>,
    pub device_error: Arc<AtomicBool>,
    pub channels: usize,
    pub sample_rate: u32,
    pub underruns: Arc<AtomicU64>,
}

/// Moved to `crate::device_info` 2026-08-19 (L05 one-home) so
/// `input_capture.rs` doesn't need this module's excluded, unsafe-laden path
/// for one plain data struct. Re-exported here so this module's own
/// references (and anyone re-enabling it) keep resolving under the old name.
pub use crate::device_info::AudioDeviceInfo;

/// Enumerate all available audio output devices with their supported configs.
pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let host_name = format!("{:?}", host.id());
    let devices = match host.output_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[audio] Cannot enumerate output devices: {}", e);
            return vec![];
        }
    };
    devices
        .map(|dev| {
            let name = dev.name().unwrap_or_else(|_| "Unknown".into());
            let mut sample_rates = Vec::new();
            let mut channels = 2u16;
            if let Ok(configs) = dev.supported_output_configs() {
                for cfg in configs {
                    channels = channels.max(cfg.channels());
                    let min = cfg.min_sample_rate().0;
                    let max = cfg.max_sample_rate().0;
                    for &sr in &[44100u32, 48000, 88200, 96000, 192000] {
                        if sr >= min && sr <= max && !sample_rates.contains(&sr) {
                            sample_rates.push(sr);
                        }
                    }
                }
            }
            sample_rates.sort();
            if sample_rates.is_empty() {
                sample_rates.push(48000);
            }
            AudioDeviceInfo {
                name,
                api: host_name.clone(),
                sample_rates,
                channels,
            }
        })
        .collect()
}

/// Enumerate all available audio INPUT devices with their supported configs —
/// the capture-side mirror of [`list_output_devices`].
/// DRAINED 2026-07-20 from E:/.airgap/condense-2026-06-10/forge-audio-engine-pre-merge/realtime_input.rs (S4 crown diamond).
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let host_name = format!("{:?}", host.id());
    let devices = match host.input_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[audio] Cannot enumerate input devices: {}", e);
            return vec![];
        }
    };
    devices
        .map(|dev| {
            let name = dev.name().unwrap_or_else(|_| "Unknown".into());
            let mut sample_rates = Vec::new();
            let mut channels = 1u16;
            if let Ok(configs) = dev.supported_input_configs() {
                for cfg in configs {
                    channels = channels.max(cfg.channels());
                    let min = cfg.min_sample_rate().0;
                    let max = cfg.max_sample_rate().0;
                    for &sr in &[44100u32, 48000, 88200, 96000, 192000] {
                        if sr >= min && sr <= max && !sample_rates.contains(&sr) {
                            sample_rates.push(sr);
                        }
                    }
                }
            }
            sample_rates.sort();
            if sample_rates.is_empty() {
                sample_rates.push(48000);
            }
            AudioDeviceInfo {
                name,
                api: host_name.clone(),
                sample_rates,
                channels,
            }
        })
        .collect()
}

/// Pick the device's highest-quality supported output config for best fidelity.
///
/// Prefers the top standard sample rate the device actually supports (up to
/// 96 kHz) over `default_output_config()`, whose WASAPI shared-mode value can be
/// as low as 32 kHz and silently caps audio quality. Returns `None` if the
/// device reports no enumerable configs (caller falls back to the OS default).
///
/// 96 kHz is the ceiling on purpose: 192 kHz doubles DSP cost against the
/// 2.5 ms real-time budget for no audible gain. If real-time underruns appear,
/// drop the ceiling to 48 kHz.
fn best_output_config(device: &cpal::Device) -> Option<cpal::SupportedStreamConfig> {
    use cpal::traits::DeviceTrait;
    // Standard output rates, best first.
    const PREFERRED: [u32; 4] = [96_000, 88_200, 48_000, 44_100];
    let mut best: Option<(u32, cpal::SupportedStreamConfigRange)> = None;
    for range in device.supported_output_configs().ok()? {
        // Highest preferred rate this range covers (PREFERRED is high -> low).
        let cand = PREFERRED
            .iter()
            .copied()
            .find(|&r| range.min_sample_rate().0 <= r && r <= range.max_sample_rate().0);
        if let Some(r) = cand {
            let better = match &best {
                None => true,
                Some((br, brange)) => r > *br || (r == *br && range.channels() > brange.channels()),
            };
            if better {
                best = Some((r, range));
            }
        }
    }
    let (rate, range) = best?;
    Some(range.with_sample_rate(cpal::SampleRate(rate)))
}

/// Start real-time audio playback using a lock-free SPSC ring buffer.
///
/// Returns a [`PlaybackHandle`] guard. The cpal::Stream is held privately
/// inside the guard — drop the handle to stop playback. The handle prevents
/// the bug class where callers drop the stream while keeping the producer
/// alive (silent audio death, no compile error).
///
/// GUARDRAIL: This is the ONLY audio output path. The feeder thread owns
/// the Mixer and pushes samples here. Never lock a Mutex on the audio thread.
pub fn start_playback_lockfree(
    buffer_size: usize,
) -> Result<PlaybackHandle, String> {
    start_playback_lockfree_with_error_flag(buffer_size, None, None, None)
}

/// Silent fallback handle for when no audio device is available.
/// Mixer math and IPC still run; nothing reaches speakers. App stays alive.
pub fn null_playback(buffer_size: usize, sample_rate: u32) -> PlaybackHandle {
    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(buffer_size);
    std::mem::forget(consumer);
    PlaybackHandle {
        stream_guard: None,
        producer,
        flush: Arc::new(AtomicBool::new(false)),
        device_error: Arc::new(AtomicBool::new(true)),
        channels: 2,
        sample_rate,
        underruns: Arc::new(AtomicU64::new(0)),
    }
}

/// Inner implementation that optionally accepts an existing error flag for reconnect scenarios.
///
/// SILENCE IS THE FLOOR (Sean 2026-07-24): a cpal device opens ONLY when
/// `FORGE_AUDIO=1` (the launch blast). Default — no var, every test, every normal
/// boot — returns a [`PlaybackHandle`] whose `stream_guard` is `None` and whose
/// producer's consumer is leaked into the void. Mixer math + IPC + ForgeVision
/// still run; nothing reaches phones/S2/speakers, so a forgotten mute can never
/// blast the speakers. Replaces the old opt-OUT `DAW_NO_AUDIO`.
pub fn start_playback_lockfree_with_error_flag(
    buffer_size: usize,
    error_flag: Option<Arc<AtomicBool>>,
    device_name: Option<&str>,
    cpal_buffer: Option<u32>,
) -> Result<PlaybackHandle, String> {
    if !crate::audio_enabled() {
        eprintln!("[forge-audio] silent: FORGE_AUDIO!=1, no cpal device opened.");
        eprintln!("[forge-audio] mixer math + IPC still run; nothing reaches phones/S2/speakers.");
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(buffer_size);
        std::mem::forget(consumer); // keep producer push valid; samples drop into the void
        let flush = Arc::new(AtomicBool::new(false));
        let device_error = error_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
        let underruns = Arc::new(AtomicU64::new(0));
        return Ok(PlaybackHandle {
            stream_guard: None,
            producer,
            flush,
            device_error,
            channels: 2,
            sample_rate: 48000,
            underruns,
        });
    }
    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        use cpal::traits::HostTrait;
        host.output_devices()
            .map_err(|e| format!("enumerate devices: {}", e))?
            .find(|d| {
                use cpal::traits::DeviceTrait;
                d.name().is_ok_and(|n| n == name)
            })
            .unwrap_or_else(|| host.default_output_device().expect("No audio output device"))
    } else {
        host.default_output_device().ok_or("No audio output device")?
    };
    let supported = device.default_output_config().map_err(|e| format!("{}", e))?;
    // Best audio quality: prefer the device's highest supported standard rate
    // over the OS default (WASAPI shared-mode default can be as low as 32 kHz).
    let default_rate = supported.sample_rate().0;
    let supported = best_output_config(&device).unwrap_or(supported);
    let channels = supported.channels() as usize;
    let device_sample_rate = supported.sample_rate().0;
    let mut config: cpal::StreamConfig = supported.into();
    if let Some(bs) = cpal_buffer {
        config.buffer_size = cpal::BufferSize::Fixed(bs);
        eprintln!("[audio] Output device: {} channels, {} Hz (OS default {} Hz), buffer {}", channels, device_sample_rate, default_rate, bs);
    } else {
        eprintln!("[audio] Output device: {} channels, {} Hz (OS default {} Hz)", channels, device_sample_rate, default_rate);
    }

    let (mut producer, mut consumer) = rtrb::RingBuffer::new(buffer_size);
    // Prime the ring with silence BEFORE `stream.play()` below starts the callback
    // draining. Without this, the first few callback cycles pop from an empty ring
    // (the feeder thread hasn't been scheduled yet) — a startup underrun burst on
    // every device open, not a deck-specific one. Leave 512 slots free for the
    // feeder's first push (mirrors the 20ms content fade-in that hides this lead-in
    // silence). AUDIO-ONE-BUS fix-phase item 1 (07-11 PROOF LOG: 6 underruns).
    let prime = buffer_size.saturating_sub(512);
    for _ in 0..prime {
        if producer.push(0.0).is_err() {
            break;
        }
    }
    let flush = Arc::new(AtomicBool::new(false));
    let flush_cb = flush.clone();
    let device_error = error_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let device_error_cb = device_error.clone();
    let underruns = Arc::new(AtomicU64::new(0));
    let underruns_cb = underruns.clone();

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // FORGE-AUDIO-HOTPATH-CLEAN-001 (2026-05-19): cpal callback is THE
            // canonical real-time path. The body is wrapped in `rt_guard` so a
            // future binary-level `assert_no_alloc` will catch any allocation
            // that sneaks in here.
            //
            // AUDIO-TELEMETRY-F12 (2026-06-01): this IS "the audio thread" for the
            // no-alloc proof. Register it on entry (idempotent TLS write, never an
            // alloc), then time the cycle. `consumer.slots()` is sampled here,
            // before the drain, so it reflects ring fill on entry. Everything
            // outside rt_guard is atomics + Instant only — no heap on the deadline.
            // crate::alloc_tracer::register_audio_thread(); // EXCLUDED - alloc_tracer not wired (needs unsafe impl GlobalAlloc)
            let cycle_start = std::time::Instant::now();
            let fill_slots = consumer.slots();
            // AUDIO-TELEMETRY-F12 (2026-06-01): publish post-cycle metrics — all
            // atomics + one Instant read, no allocation. Fetched early so the
            // AUDIO-ONE-BUS underrun-cycle probe below can use it too.
            let tel = crate::telemetry::telemetry();
            let this_cycle = tel.tick_cycle();
            crate::rt_safety::rt_guard(|| {
                // If flush requested, drain the ring buffer and output silence
                if flush_cb.swap(false, Ordering::Relaxed) {
                    while consumer.pop().is_ok() {}
                    for sample in data.iter_mut() {
                        *sample = 0.0;
                    }
                    return;
                }
                let mut had_underrun = false;
                for sample in data.iter_mut() {
                    match consumer.pop() {
                        Ok(s) => *sample = s,
                        Err(_) => { *sample = 0.0; had_underrun = true; }
                    }
                }
                if had_underrun {
                    underruns_cb.fetch_add(1, Ordering::Relaxed);
                    // AUDIO-ONE-BUS PROOF LOG probe: which cycle #, is it periodic.
                    tel.log_underrun_cycle(this_cycle);
                }
            });
            tel.record_cycle(cycle_start.elapsed().as_micros() as u64);
            tel.set_underruns(underruns_cb.load(Ordering::Relaxed));
            if buffer_size > 0 {
                tel.ring_buffer_fill_pct
                    .store((fill_slots as u64 * 100) / buffer_size as u64, Ordering::Relaxed);
            }
            // Master bus level = the final output buffer (post-mix/limiter). One
            // extra O(n) scalar pass, no heap. SFX/Music buses don't exist in this
            // engine, so only master is published; the panel shows the rest at floor.
            let mut peak_l = 0.0f32;
            let mut peak_r = 0.0f32;
            let mut sum_sq = 0.0f32;
            let mut sum_l_sq = 0.0f32;
            let mut sum_r_sq = 0.0f32;
            let mut sum_lr = 0.0f32;
            for frame in data.chunks(channels.max(1)) {
                let l = frame.first().copied().unwrap_or(0.0);
                let r = if channels > 1 { frame.get(1).copied().unwrap_or(0.0) } else { l };
                let al = l.abs();
                let ar = r.abs();
                if al > peak_l { peak_l = al; }
                if ar > peak_r { peak_r = ar; }
                sum_sq += l * l + r * r;
                sum_l_sq += l * l;
                sum_r_sq += r * r;
                sum_lr += l * r;
            }
            let rms = (sum_sq / data.len().max(1) as f32).sqrt();
            let to_db = |x: f32| if x > 1e-6 { 20.0 * x.log10() } else { -120.0 };
            tel.set_master_levels(to_db(rms), to_db(peak_l.max(peak_r)));
            let denom = (sum_l_sq * sum_r_sq).sqrt();
            let phase = if denom > 1e-6 { (sum_lr / denom).clamp(-1.0, 1.0) } else { 1.0 };
            tel.set_meter_bands(to_db(peak_l), to_db(peak_r), (phase * 10_000.0).round() as i32);
        },
        move |err| {
            eprintln!("[audio] Output error (device disconnect?): {}", err);
            device_error_cb.store(true, Ordering::Relaxed);
        },
        None,
    ).map_err(|e| format!("Stream: {}", e))?;

    stream.play().map_err(|e| format!("Play: {}", e))?;
    Ok(PlaybackHandle {
        stream_guard: Some(stream),
        producer,
        flush,
        device_error,
        channels,
        sample_rate: device_sample_rate,
        underruns,
    })
}

/// Start headphone/booth output on a second audio device.
/// Returns None if no second device is available (graceful fallback).
/// The feeder thread pushes headphone_mix samples to the returned producer.
pub fn start_headphone_output(
    buffer_size: usize,
) -> Option<(cpal::Stream, rtrb::Producer<f32>, usize)> {
    if !crate::audio_enabled() {
        eprintln!("[forge-audio] silent: headphone output skipped (FORGE_AUDIO!=1)");
        return None;
    }
    let host = cpal::default_host();
    let default = host.default_output_device();
    let default_name = default.as_ref().and_then(|d| d.name().ok()).unwrap_or_default();

    // Find first non-default output device
    let mut devices = match host.output_devices() {
        Ok(d) => d,
        Err(_) => { eprintln!("[audio] Cannot enumerate output devices"); return None; }
    };

    let alt_device = devices
        .find(|d| {
            let name = d.name().unwrap_or_default();
            name != default_name
                && !name.to_lowercase().contains("nvidia")
                && !name.to_lowercase().contains("hdmi")
                && !name.to_lowercase().contains("displayport")
                && !name.to_lowercase().contains("ultragear")
        });

    let device = match alt_device {
        Some(d) => {
            eprintln!("[audio] Headphone output: {}", d.name().unwrap_or_default());
            d
        }
        None => {
            eprintln!("[audio] No second output device — headphone mix disabled");
            return None;
        }
    };

    let config = match best_output_config(&device) {
        Some(c) => c,
        None => match device.default_output_config() {
            Ok(c) => c,
            Err(e) => { eprintln!("[audio] Headphone device config error: {}", e); return None; }
        },
    };
    let channels = config.channels() as usize;
    let (producer, mut consumer) = rtrb::RingBuffer::new(buffer_size);

    let stream = match device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for sample in data.iter_mut() {
                *sample = consumer.pop().unwrap_or(0.0);
            }
        },
        |err| eprintln!("[audio] Headphone output error: {}", err),
        None,
    ) {
        Ok(s) => s,
        Err(e) => { eprintln!("[audio] Headphone stream error: {}", e); return None; }
    };

    if let Err(e) = stream.play() {
        eprintln!("[audio] Headphone play error: {}", e);
        return None;
    }

    Some((stream, producer, channels))
}

// ════════════════════════════════════════════════════════════════════════════
// Hard-gate guard tests — verify the typed PlaybackHandle bug class is closed.
// ════════════════════════════════════════════════════════════════════════════
//
// All tests run under DAW_NO_AUDIO=1 — no real cpal device is opened. The
// guard's `is_live()` returns false, but every other field is populated and
// the producer is push-able. Tests assert the guard's structural invariants
// (private stream, Drop fires, split yields a Send bundle).

#[cfg(test)]
mod hard_gate_tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn enter_hard_gate() {
        // Set DAW_NO_AUDIO once per test process. Safe across parallel tests
        // because every assertion in this module assumes the gate is on.
        INIT.call_once(|| {
            std::env::set_var("DAW_NO_AUDIO", "1");
        });
    }

    #[test]
    fn handle_under_hard_gate_is_not_live() {
        enter_hard_gate();
        let h = start_playback_lockfree(1024).expect("guard mode must succeed");
        assert!(!h.is_live(), "DAW_NO_AUDIO must produce non-live handle");
        assert_eq!(h.sample_rate(), 48000);
        assert_eq!(h.channels(), 2);
    }

    #[test]
    fn producer_is_pushable_under_hard_gate() {
        enter_hard_gate();
        let mut h = start_playback_lockfree(1024).expect("guard mode must succeed");
        // The consumer is leaked into mem::forget — push must succeed until
        // ring is full, then return Err. Either is fine for this assertion;
        // we only verify the producer is reachable via the guard's accessor.
        let _ = h.producer_mut().push(0.5);
    }

    #[test]
    fn flags_are_clonable_arcs() {
        enter_hard_gate();
        let h = start_playback_lockfree(1024).expect("guard mode must succeed");
        let flush_a = h.flush_flag();
        let flush_b = h.flush_flag();
        flush_a.store(true, Ordering::Relaxed);
        assert!(flush_b.load(Ordering::Relaxed), "flags must be Arc-clones, not copies");
    }

    #[test]
    fn split_produces_send_bundle() {
        enter_hard_gate();
        let h = start_playback_lockfree(1024).expect("guard mode must succeed");
        let (keeper, mut bundle) = h.split();
        // The bundle MUST be Send — if it isn't, this thread::spawn won't compile.
        let jh = std::thread::spawn(move || {
            assert_eq!(bundle.sample_rate, 48000);
            assert_eq!(bundle.channels, 2);
            let _ = bundle.producer.push(0.25);
            bundle.sample_rate
        });
        let sr = jh.join().expect("feeder thread joined");
        assert_eq!(sr, 48000);
        // Keeper stays on this thread (it is !Send because cpal::Stream is !Send,
        // even when None, because the Option<cpal::Stream> field type is !Send).
        // Drop happens at end of scope.
        drop(keeper);
    }

    #[test]
    fn drop_under_hard_gate_does_not_panic() {
        enter_hard_gate();
        let h = start_playback_lockfree(1024).expect("guard mode must succeed");
        // The Drop log fires only when stream_guard is Some — under DAW_NO_AUDIO
        // it is None and Drop is silent. Just verify no panic on scope exit.
        drop(h);
    }

    #[test]
    fn with_error_flag_passthrough_preserves_arc_identity() {
        enter_hard_gate();
        let shared = Arc::new(AtomicBool::new(false));
        let h = start_playback_lockfree_with_error_flag(1024, Some(shared.clone()), None, None)
            .expect("guard mode must succeed");
        // The handle's device_error flag must be the same Arc we passed in,
        // so reconnect callers (Dead Drop main.rs:454) can mutate one source.
        h.device_error_flag().store(true, Ordering::Relaxed);
        assert!(shared.load(Ordering::Relaxed),
            "with_error_flag must reuse the caller's Arc, not clone its contents");
    }
}
</content>
