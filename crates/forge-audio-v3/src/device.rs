// @forge:allow_float — audio sample values are inherently f32.
//! One-lane real-time audio output.
//!
//! [`spawn_audio_lane`] opens the default output device inside the feeder
//! thread (so `cpal::Stream !Send` never crosses a boundary), renders
//! [`AudioLane`] at 120 Hz into an rtrb SPSC ring, and drives the cpal
//! callback from the consumer side.
//!
//! Set `DAW_NO_AUDIO=1` to run silently — device open is skipped, the ring
//! consumer is leaked (silent discard). Useful for headless CI.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Sender, TryRecvError};
use std::sync::{Arc, atomic::{AtomicU32, Ordering}};
use std::time::{Duration, Instant};

use crate::scheduled_event::ScheduledEvent;

use crate::conductor_audio::AudioLane;

// ── Device open ───────────────────────────────────────────────────────────────

struct DeviceHandle {
    // None in DAW_NO_AUDIO mode. Held alive until the feeder thread exits.
    _stream: Option<cpal::Stream>,
}

fn best_output_config(device: &cpal::Device) -> Option<cpal::SupportedStreamConfig> {
    // Prefer the highest standard rate the device supports up to 96 kHz.
    // 192 kHz doubles DSP cost for no audible gain in our use-case.
    const PREFERRED: [u32; 4] = [96_000, 88_200, 48_000, 44_100];
    let mut best: Option<(u32, cpal::SupportedStreamConfigRange)> = None;
    for range in device.supported_output_configs().ok()? {
        let cand = PREFERRED.iter().copied().find(|&r| {
            range.min_sample_rate().0 <= r && r <= range.max_sample_rate().0
        });
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

/// Must be called from inside the feeder thread — `cpal::Stream` is `!Send`.
fn open_output() -> Result<(DeviceHandle, rtrb::Producer<f32>, u32, usize), String> {
    // SILENCE IS THE FLOOR (Sean 2026-07-24, boot-silent law): a real cpal device
    // opens ONLY when audio is explicitly enabled via `FORGE_AUDIO=1` — the launch
    // blast. Absent that (every test, every normal boot) NO device is opened, so a
    // forgotten mute can never blast the speakers. This RETIRES the old opt-OUT
    // `DAW_NO_AUDIO`: silence now needs no flag; audio needs the positive gate.
    if !crate::audio_enabled() {
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(4096);
        std::mem::forget(consumer); // silent discard
        return Ok((DeviceHandle { _stream: None }, producer, 48_000, 2));
    }

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| "no audio output device".to_string())?;
    let config = best_output_config(&device)
        .or_else(|| device.default_output_config().ok().map(Into::into))
        .ok_or_else(|| "no supported output config".to_string())?;

    let sr = config.sample_rate().0;
    let ch = config.channels() as usize;
    // Ring sized to 8 stereo render-blocks at the actual sample rate.
    let block = (sr / 120) as usize;
    let ring_cap = block * 8 * ch;

    let (producer, mut consumer) = rtrb::RingBuffer::<f32>::new(ring_cap);

    let stream = device
        .build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                for s in data.iter_mut() {
                    *s = consumer.pop().unwrap_or(0.0);
                }
            },
            |e| eprintln!("[forge-audio] cpal error: {e}"),
            None,
        )
        .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;

    Ok((DeviceHandle { _stream: Some(stream) }, producer, sr, ch))
}

// ── Lane commands ─────────────────────────────────────────────────────────────

pub enum LaneCmd {
    Apply(Vec<ScheduledEvent>),
    /// Raw `(note, velocity, duration_ms)` triggers, bypassing `BardPhraseKind` —
    /// the melody-caller lane (`shell::singer`'s Technothesia port).
    TriggerNotes(Vec<(u8, u8, u32)>),
    Shutdown,
}

// ── Public handle ─────────────────────────────────────────────────────────────

/// Live [`AudioLane`] wired to the default output device.
/// Drop (or call [`shutdown`][Self::shutdown]) to stop the feeder thread.
#[must_use]
pub struct AudioLaneHandle {
    cmd_tx: Sender<LaneCmd>,
    pub sample_rate: u32,
    /// Latest block RMS stored by the feeder thread as IEEE-754 bits.
    /// Read with `lane_rms()`; Relaxed ordering — stale-by-one-tick is fine for vibe.
    rms_bits: Arc<AtomicU32>,
    beat_phase_bits: Arc<AtomicU32>,
}

impl AudioLaneHandle {
    /// Queue phrase events for the next 120 Hz tick.
    pub fn send_events(&self, events: Vec<ScheduledEvent>) {
        let _ = self.cmd_tx.try_send(LaneCmd::Apply(events));
    }

    /// Queue raw `(note, velocity, duration_ms)` triggers for the next tick,
    /// bypassing `BardPhraseKind` entirely.
    pub fn trigger_notes(&self, notes: Vec<(u8, u8, u32)>) {
        let _ = self.cmd_tx.try_send(LaneCmd::TriggerNotes(notes));
    }

    /// Stop the feeder thread gracefully.
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.try_send(LaneCmd::Shutdown);
    }

    /// Instantaneous RMS of the last rendered audio block [0.0, 1.0].
    /// Relaxed: stale by at most one 120 Hz tick — acceptable for vibemat.
    pub fn lane_rms(&self) -> f32 {
        f32::from_bits(self.rms_bits.load(Ordering::Relaxed))
    }

    /// Beat-grid phase [0.0, 1.0) of the last rendered audio block.
    /// 0.0 when the transport is stopped (no beat clock active).
    pub fn lane_beat_phase(&self) -> f32 {
        f32::from_bits(self.beat_phase_bits.load(Ordering::Relaxed))
    }
}

impl Drop for AudioLaneHandle {
    fn drop(&mut self) {
        let _ = self.cmd_tx.try_send(LaneCmd::Shutdown);
    }
}

fn compute_block_rms(buf: &[f32]) -> f32 {
    if buf.is_empty() { return 0.0; }
    let sum_sq: f32 = buf.iter().map(|s| s * s).sum();
    (sum_sq / buf.len() as f32).sqrt()
}

/// Spawn an [`AudioLane`] feeder thread wired to the default audio output.
///
/// The device is opened *inside* the feeder thread so `cpal::Stream !Send`
/// never crosses a thread boundary. Returns once the device is confirmed open
/// (or `DAW_NO_AUDIO` silent mode is confirmed active).
pub fn spawn_audio_lane(bpm: f32) -> Result<AudioLaneHandle, String> {
    let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<LaneCmd>(64);
    let (init_tx, init_rx) = crossbeam_channel::bounded::<Result<u32, String>>(1);

    let rms_bits        = Arc::new(AtomicU32::new(0));
    let beat_phase_bits = Arc::new(AtomicU32::new(0));
    let rms_write       = Arc::clone(&rms_bits);
    let bp_write        = Arc::clone(&beat_phase_bits);

    std::thread::Builder::new()
        .name("audio-lane-feeder".into())
        .spawn(move || {
            // All device + stream state created here — stays on this thread.
            let (_device, mut producer, sr, ch) = match open_output() {
                Ok(v) => {
                    let _ = init_tx.send(Ok(v.2));
                    v
                }
                Err(e) => {
                    let _ = init_tx.send(Err(e));
                    return;
                }
            };

            let block = (sr / 120) as usize;
            let tick = Duration::from_micros(1_000_000 / 120);
            let mut lane = AudioLane::new(sr, bpm);
            // Pre-allocate once; reused every tick (no hot-path alloc).
            let mut mono_buf = vec![0.0f32; block];

            loop {
                let t0 = Instant::now();

                // Drain all pending commands for this tick.
                loop {
                    match cmd_rx.try_recv() {
                        Ok(LaneCmd::Shutdown) => return,
                        Ok(LaneCmd::Apply(events)) => lane.apply(&events, 0),
                        Ok(LaneCmd::TriggerNotes(notes)) => {
                            for (note, vel, dur_ms) in notes {
                                lane.trigger_note(note, vel, dur_ms);
                            }
                        }
                        Err(TryRecvError::Empty) => break,
                        Err(TryRecvError::Disconnected) => return,
                    }
                }

                // Render mono block then interleave to device channel count.
                mono_buf.fill(0.0);
                lane.render(&mut mono_buf, &[]);
                // Soft-clip the mix — `Synth::render`/`AudioLane::render` sum
                // voices additively with no limiter of their own, so stacked
                // polyphony (e.g. `shell::singer`'s note-per-word bridge)
                // clips hard at the DAC without this. Same law v2's own
                // `technothesia/src/sing.rs` states verbatim: "the final mix
                // is tanh soft-clipped — output can NEVER exceed unity."
                for s in mono_buf.iter_mut() {
                    *s = s.tanh();
                }
                // Readback: publish RMS + beat_phase for vibe_from_audio consumers.
                // Relaxed — a vibemat reading one tick stale is visually indistinguishable.
                let rms = compute_block_rms(&mono_buf);
                rms_write.store(rms.to_bits(), Ordering::Relaxed);
                let bp = lane.transport.grid.phase_at(lane.transport.playhead as usize);
                bp_write.store(bp.to_bits(), Ordering::Relaxed);
                for &s in &mono_buf {
                    for _ in 0..ch {
                        // Drop on full — a ring underrun is preferable to a
                        // blocking push that blows the 120 Hz cadence.
                        let _ = producer.push(s);
                    }
                }

                if let Some(rem) = tick.checked_sub(t0.elapsed()) {
                    std::thread::sleep(rem);
                }
            }
        })
        .map_err(|e| e.to_string())?;

    let sr = init_rx
        .recv()
        .map_err(|_| "feeder thread died before device init".to_string())??;

    Ok(AudioLaneHandle { cmd_tx, sample_rate: sr, rms_bits, beat_phase_bits })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduled_event::ScheduledEvent;

    fn set_no_audio() {
        // audio_enabled() already defaults to silent (FORGE_AUDIO unset);
        // DAW_NO_AUDIO is a retired opt-out no longer read by open_output().
    }

    #[test]
    fn spawn_in_silent_mode() {
        set_no_audio();
        let handle = spawn_audio_lane(120.0).expect("silent spawn must succeed");
        assert_eq!(handle.sample_rate, 48_000);
        handle.shutdown();
    }

    #[test]
    fn send_events_does_not_panic_in_silent_mode() {
        set_no_audio();
        let handle = spawn_audio_lane(120.0).unwrap();
        let ev = ScheduledEvent { fire_tick: 0, tag: 1, ..Default::default() };
        handle.send_events(vec![ev]);
        // Give feeder one tick to process.
        std::thread::sleep(Duration::from_millis(20));
        handle.shutdown();
    }

    #[test]
    fn drop_shuts_down_feeder() {
        set_no_audio();
        let handle = spawn_audio_lane(120.0).unwrap();
        drop(handle); // should not hang
    }

    #[test]
    fn lane_rms_rises_after_bell_dispatch() {
        // Discriminator: glow must be non-zero after MinorThirdDescent (tag 0).
        // Fails if compute_block_rms is skipped OR if the glow fold accidentally
        // uses beat_phase (which is 0 when transport is stopped → glow=0).
        set_no_audio();
        let handle = spawn_audio_lane(120.0).expect("silent spawn");
        assert_eq!(handle.lane_rms(), 0.0, "rms must start at zero before any event");
        // tag 0 = PHRASE_KIND_MINOR_THIRD_DESCENT → strike_grave_bell → synth active
        let ev = ScheduledEvent { fire_tick: 0, tag: 0, ..Default::default() };
        handle.send_events(vec![ev]);
        std::thread::sleep(Duration::from_millis(25)); // 3+ ticks at 120 Hz
        let rms = handle.lane_rms();
        assert!(rms > 0.0, "rms must be non-zero after a bell strike, got {rms}");
        // Vibe Law: glow = rms * 0.12 (no beat_phase — one-shot strikes must glow)
        let glow = (rms * 0.12_f32).clamp(0.0, 0.12);
        assert!(glow > 0.0 && glow <= 0.12,
            "glow must be non-zero and within Vibe Law ceiling [0, 0.12], got {glow}");
        handle.shutdown();
    }
}
