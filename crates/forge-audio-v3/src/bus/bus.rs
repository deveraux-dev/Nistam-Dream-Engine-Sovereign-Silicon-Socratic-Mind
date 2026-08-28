//! AudioBusHandle and AudioBus — the shared audio bus and its handle.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::fmt;
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, Sender, TryRecvError};
use forge_ump::provenance_tag::Tier;
use forge_ump::Recorder;

use super::command::MixerCommand;
use super::mixer::MixerCommandHub;
use super::snapshot::LiveMixerState;
use super::ump_codec;

/// Target publish rate for the feeder loop (~120 Hz).
const TARGET_INTERVAL: Duration = Duration::from_micros(8333);

/// Jitter-reduction quantization granularity for the master-bus command tape (1 ms).
pub(crate) const HUB_TAPE_JR_US: i64 = 1000;

/// Wall-clock universal tick in microseconds — the `Stamped` timestamp every UMP
/// command atom rides onto the flight-recorder tape.
#[inline]
pub(crate) fn tick_us() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

/// A lock-free snapshot of the master bus's UMP flight-recorder — published every
/// time a command moment is sealed onto the tape, so a scrubber / HUD can gauge
/// liveness and seek without touching the recorder thread.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HubTapeStat {
    /// Committed moments on the tape (each = one activity tick with ≥1 command).
    pub moments: u64,
    /// Total commands encoded to UMP and observed onto the tape.
    pub events: u64,
    /// `content_seal` of the most recently sealed moment (0 before the first).
    pub last_seal: u64,
    /// `tick_id` (master-bus frame) of the most recent seal.
    pub last_tick: u64,
}

/// Process-global mirror of the master-bus flight-recorder stat — the same shape as
/// [`HubTapeStat`], stored in atomics like `forge_audio::telemetry::TELEMETRY`. The
/// feeder writes it every sealed moment; UI chrome (forge-gui `render_lowered_with_ctx`)
/// reads it to draw the REC/scrub bar on EVERY panel with no per-panel or per-call-site
/// wiring. A global is the honest single-seam here: the bar is one universal chrome
/// element, not a per-panel data binding.
pub struct HubTapeGlobal {
    moments: AtomicU64,
    events: AtomicU64,
    last_seal: AtomicU64,
    last_tick: AtomicU64,
}

impl HubTapeGlobal {
    const fn new() -> Self {
        Self {
            moments: AtomicU64::new(0),
            events: AtomicU64::new(0),
            last_seal: AtomicU64::new(0),
            last_tick: AtomicU64::new(0),
        }
    }

    /// Publish the latest tape stat (Relaxed — a status readout, not a sync point).
    pub fn store(&self, s: HubTapeStat) {
        self.moments.store(s.moments, Ordering::Relaxed);
        self.events.store(s.events, Ordering::Relaxed);
        self.last_seal.store(s.last_seal, Ordering::Relaxed);
        self.last_tick.store(s.last_tick, Ordering::Relaxed);
    }

    /// Read the latest tape stat for a chrome readout.
    pub fn load(&self) -> HubTapeStat {
        HubTapeStat {
            moments: self.moments.load(Ordering::Relaxed),
            events: self.events.load(Ordering::Relaxed),
            last_seal: self.last_seal.load(Ordering::Relaxed),
            last_tick: self.last_tick.load(Ordering::Relaxed),
        }
    }
}

/// The one master-bus flight-recorder mirror every panel's chrome reads.
pub static HUB_TAPE: HubTapeGlobal = HubTapeGlobal::new();

/// Handle cloned into every panel that needs audio access.
/// Cloning is cheap: Arc + crossbeam Sender are both Clone.
#[derive(Clone)]
pub struct AudioBusHandle {
    /// Send commands to the audio feeder thread.
    /// Bounded(256) — backpressure if panels spam commands.
    pub cmd_tx: Sender<MixerCommand>,
    /// Lock-free read of latest mixer state. Never blocks.
    pub snapshot: Arc<ArcSwap<LiveMixerState>>,
    /// Lock-free read of the master-bus UMP flight-recorder stats (ZD-003 scrubber
    /// substrate). Republished by the feeder thread on every sealed command moment.
    pub hub_tape: Arc<ArcSwap<HubTapeStat>>,
}

impl AudioBusHandle {
    // SendError / TrySendError carry the unsent `MixerCommand` payload (~200+ bytes
    // depending on variant). The Err size is set by crossbeam_channel's API and
    // cannot be reduced without dropping that payload; Boxing it would force a
    // heap allocation in the cold-path error case, which is exactly what we want
    // to avoid on real-time audio paths. Localised allow is honest here.
    #[allow(clippy::result_large_err)]
    pub fn send_command(&self, cmd: MixerCommand) -> Result<(), crossbeam_channel::SendError<MixerCommand>> {
        self.cmd_tx.send(cmd)
    }

    #[allow(clippy::result_large_err)]
    pub fn try_send_command(&self, cmd: MixerCommand) -> Result<(), crossbeam_channel::TrySendError<MixerCommand>> {
        self.cmd_tx.try_send(cmd)
    }

    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(MixerCommand::Shutdown);
    }
}

/// Errors that can occur when spawning the audio bus.
#[derive(Debug)]
pub enum AudioBusError {
    /// No audio output device was found on the system.
    NoOutputDevice,
    /// An error occurred while setting up the audio stream.
    StreamError(String),
}

impl fmt::Display for AudioBusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AudioBusError::NoOutputDevice => {
                write!(f, "no audio output device available")
            }
            AudioBusError::StreamError(msg) => {
                write!(f, "audio stream error: {msg}")
            }
        }
    }
}

impl std::error::Error for AudioBusError {}

/// The stub audio feeder thread main loop (silent mode — no real audio I/O).
///
/// Snapshots published here carry zeroed metering fields (`rms_level`,
/// `spectral_energy`, `eq_bands`, `beat_phase`, `waveform_bands`). This is
/// intentional: the realtime callback that computes metering is absent in
/// silent mode. UI/viz consumers must treat all-zero metering as "no signal",
/// not as a live reading of silence.
pub fn stub_feeder_loop(
    cmd_rx: Receiver<MixerCommand>,
    snapshot: Arc<ArcSwap<LiveMixerState>>,
    hub_tape: Arc<ArcSwap<HubTapeStat>>,
) {
    let mut mixer = MixerCommandHub::new();
    let mut last_tick = Instant::now();

    // Master-bus flight recorder: every command applied this tick is also encoded
    // to a UMP and observed onto a sealed, scrubbable tape (Tier::Local — machine
    // origin, never authority). Commit fires only on ticks that carried commands,
    // so tape growth tracks activity, not idle frames.
    let mut recorder = Recorder::new(HUB_TAPE_JR_US, Tier::Local);
    let mut total_events: u64 = 0;
    let mut last_essence: u8 = 0;
    let mut frame: u64 = 0;

    loop {
        let iteration_start = Instant::now();
        frame += 1;

        loop {
            match cmd_rx.try_recv() {
                Ok(MixerCommand::Shutdown) => {
                    recorder.observe(ump_codec::stamp(&MixerCommand::Shutdown, tick_us()));
                    let _ = recorder.commit(frame, 0, ump_codec::essence_of(&MixerCommand::Shutdown));
                    return;
                }
                Ok(cmd) => {
                    recorder.observe(ump_codec::stamp(&cmd, tick_us()));
                    last_essence = ump_codec::essence_of(&cmd);
                    total_events += 1;
                    mixer.apply_command(cmd);
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    return;
                }
            }
        }

        let now = Instant::now();
        let dt = now.duration_since(last_tick).as_secs_f64();
        last_tick = now;
        mixer.tick(dt);

        // Seal a command moment only when this tick actually observed commands.
        if recorder.pending_len() > 0 {
            if let Ok(sealed) = recorder.commit(frame, 0, last_essence) {
                let stat = HubTapeStat {
                    moments: recorder.len() as u64,
                    events: total_events,
                    last_seal: sealed.content_seal,
                    last_tick: sealed.tick_id,
                };
                hub_tape.store(Arc::new(stat));
                // Mirror into the process-global so panel chrome (every panel's REC
                // bar) can read it without a per-panel handle.
                HUB_TAPE.store(stat);
            }
        }

        let mut snap = mixer.build_snapshot();
        snap.timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        snapshot.store(Arc::new(snap));

        let elapsed = iteration_start.elapsed();
        if let Some(remaining) = TARGET_INTERVAL.checked_sub(elapsed) {
            std::thread::sleep(remaining);
        }
    }
}

/// Spawns the audio feeder thread in stub (silent) mode and returns the handle.
pub struct AudioBus;

impl AudioBus {
    pub fn spawn_stub() -> Result<AudioBusHandle, AudioBusError> {
        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<MixerCommand>(256);
        let snapshot = Arc::new(ArcSwap::from_pointee(LiveMixerState::default()));
        let hub_tape = Arc::new(ArcSwap::from_pointee(HubTapeStat::default()));
        let snap_thread = Arc::clone(&snapshot);
        let tape_thread = Arc::clone(&hub_tape);
        std::thread::Builder::new()
            .name("audio-feeder-stub".into())
            .spawn(move || stub_feeder_loop(cmd_rx, snap_thread, tape_thread))
            .map_err(|e| AudioBusError::StreamError(e.to_string()))?;
        Ok(AudioBusHandle { cmd_tx, snapshot, hub_tape })
    }

    /// Spawn the REAL audio feeder: opens the canonical cpal output device
    /// ([`crate::realtime`]) at its best supported rate, wires a lock-free SPSC
    /// ring to the callback, and runs [`super::engine_adapter::real_feeder_loop`]
    /// (`crate::mixer::Mixer::mix_block`). This is what actually makes sound —
    /// it replaces the silent [`Self::spawn_stub`].
    ///
    /// Returns [`AudioBusError::NoOutputDevice`] when no device is available
    /// (incl. the `DAW_NO_AUDIO=1` headless/CI hard-gate) so the caller can fall
    /// back to the stub via [`Self::spawn`].
    pub fn spawn_real() -> Result<AudioBusHandle, AudioBusError> {
        // rtrb ring capacity in f32 slots (~4096 stereo frames ≈ 85 ms @ 48 kHz).
        const RING_CAP: usize = 8192;

        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded::<MixerCommand>(256);
        let snapshot = Arc::new(ArcSwap::from_pointee(LiveMixerState::default()));
        let snap_thread = Arc::clone(&snapshot);
        let hub_tape = Arc::new(ArcSwap::from_pointee(HubTapeStat::default()));
        let tape_thread = Arc::clone(&hub_tape);

        // Open the optional headphone/booth cue device FIRST. It enumerates and
        // opens a SECOND cpal device, which is slow (tens-to-hundreds of ms). Doing
        // it before the main output's `play()` means nothing is draining yet, so
        // that latency cannot starve the main ring (the cause of the startup
        // underruns). Graceful None when there is no second device.
        let hp_producer = match crate::realtime::start_headphone_output(RING_CAP) {
            Some((stream, producer, _channels)) => {
                std::mem::forget(stream); // process-lifetime keep-alive
                Some(producer)
            }
            None => None,
        };

        // Open the canonical main output. The PlaybackHandle owns the cpal::Stream;
        // `start_playback_lockfree` calls `stream.play()` so the callback begins
        // draining from here on.
        let handle = match crate::realtime::start_playback_lockfree(RING_CAP) {
            Ok(h) => h,
            Err(e) if e.contains("No audio output device") => {
                return Err(AudioBusError::NoOutputDevice);
            }
            Err(e) => return Err(AudioBusError::StreamError(e)),
        };
        if !handle.is_live() {
            // DAW_NO_AUDIO hard-gate (headless / CI): no real device opened.
            return Err(AudioBusError::NoOutputDevice);
        }

        // Split so the !Send StreamKeeper (cpal::Stream) can be leaked for the
        // process lifetime while the Send FeederBundle moves into the feeder
        // thread. Leaking the keeper is the documented keep-alive pattern
        // (realtime.rs::StreamKeeper) — the stream closes only at process exit.
        let (keeper, bundle) = handle.split();
        std::mem::forget(keeper);

        // Ring priming now happens inside start_playback_lockfree (realtime.rs),
        // before stream.play() — the callback never sees an empty ring on startup.
        let producer = bundle.producer;
        let underruns = bundle.underruns;
        std::thread::Builder::new()
            .name("audio-feeder-real".into())
            .spawn(move || {
                super::engine_adapter::real_feeder_loop(
                    cmd_rx,
                    snap_thread,
                    producer,
                    underruns,
                    hp_producer,
                    None, // AudioStatePublisher — wired by the vibe host later
                    tape_thread,
                )
            })
            .map_err(|e| AudioBusError::StreamError(e.to_string()))?;

        Ok(AudioBusHandle { cmd_tx, snapshot, hub_tape })
    }

    /// Preferred host entry: try the REAL feeder, and on a missing output device
    /// (incl. `DAW_NO_AUDIO`/CI) fall honestly back to the silent stub so the rest
    /// of the app still runs. This is NOT suppression — the stub is a legitimate
    /// no-device fallback, and the real path is taken whenever a device exists.
    pub fn spawn() -> Result<AudioBusHandle, AudioBusError> {
        match Self::spawn_real() {
            Ok(h) => Ok(h),
            Err(AudioBusError::NoOutputDevice) => {
                eprintln!("[audio] no output device — falling back to silent stub feeder");
                Self::spawn_stub()
            }
            Err(e) => Err(e),
        }
    }
}
