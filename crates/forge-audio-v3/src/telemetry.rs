//! Lock-free audio telemetry — a bag of atomics written by the audio thread and
//! read by the F12 Audio Telemetry panel at UI frame rate.
//!
//! Every field is a relaxed atomic: there are NO locks and NO channel sends from
//! the audio thread, so a panel read is stale-by-at-most-one-frame, which is
//! fine for a display surface (the panel reads at 60 Hz; the audio path writes
//! at the callback rate). The allocation-proof counters live in
//! `crate::alloc_tracer` (currently disabled — `mod alloc_tracer;` is
//! commented out in `lib.rs`); this struct would bridge them via
//! `AudioTelemetry::audio_alloc_count` (unported) so the panel reads one place.
//!
//! The singleton is a plain `const`-initialised `static` ([`TELEMETRY`]) — no
//! `OnceLock`/`Lazy` needed, so it is always available with zero init cost.

use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, AtomicU8, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

// alloc_tracer import EXCLUDED - real unsafe GlobalAlloc impl.

/// Shared monotonic epoch so the feeder thread and the cpal callback can express
/// "now" as one comparable u64 (µs). Eager-inited at feeder start
/// ([`AudioTelemetry::record_iter_start`]'s first call) so the RT callback only
/// ever hits the lock-free fast path.
static MONO_EPOCH: OnceLock<Instant> = OnceLock::new();

/// Microseconds since the process-wide monotonic epoch (first caller sets it).
pub fn mono_us() -> u64 {
    MONO_EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

/// Feeder-loop phase markers (AUDIO-ONE-BUS in-flight probe, 2026-07-19): the
/// feeder stamps which section it last passed; an underrun stamp of (age, phase)
/// names the stalled section in one run. 0 = feeder never started an iteration.
pub mod iter_phase {
    pub const TOP: u64 = 1;
    pub const CMD_DRAINED: u64 = 2;
    pub const SLEEP_LOW_SLOTS: u64 = 3;
    pub const SLEEP_ZERO_FRAMES: u64 = 4;
    pub const MIXED: u64 = 5;
    pub const PUBLISHED: u64 = 6;
}

/// The 2.5 ms native DSP deadline, in microseconds — the default `deadline_us`.
pub const DSP_DEADLINE_US: u64 = 2_500;

/// Fixed capacity for the underrun-cycle diagnostic ring (AUDIO-ONE-BUS PROOF LOG,
/// 07-11: "next pass needs an underrun-timestamp probe (which cycle #, is it
/// periodic) before touching TARGET_INTERVAL or RING_CAP"). Far above the
/// observed ~6/sweep so one range-fire run never wraps mid-sweep.
pub const UNDERRUN_LOG_CAP: usize = 64;

/// A feeder-loop trip older than this (µs) at the next trip's start is logged
/// into the gap ring — well above the 500µs sleep + normal jitter, far below
/// the ~150ms stall being chased.
pub const GAP_LOG_US: u64 = 2_000;

/// Bus-level fixed-point value for "effectively silent" (-120 dB).
/// Bus levels store `dB * 100` (so `-600` == `-6.0 dB`).
pub const DB_SILENCE_FIXED: i32 = -12_000;

/// Roadie severity codes packed into [`AudioTelemetry::roadie_severity`].
/// `OK` is the absence of any issue; the rest mirror `audio_issues::IssueSeverity`.
pub mod roadie {
    pub const OK: u8 = 0;
    pub const INFO: u8 = 1;
    pub const LOW: u8 = 2;
    pub const MEDIUM: u8 = 3;
    pub const HIGH: u8 = 4;
}

/// Convert a dB float to the fixed-point form stored in the bus-level atomics
/// (`value / 100 == dB`). The cast saturates into `i32` range (Rust 1.45+).
pub fn db_to_fixed(db: f32) -> i32 {
    (db * 100.0).round() as i32
}

/// Convert a linear amplitude (0..=1) to the same fixed-point dB form. Silence
/// pins to [`DB_SILENCE_FIXED`] rather than diverging to -inf through `log10`.
pub fn amp_to_fixed(amp: f32) -> i32 {
    if amp <= 1e-6 {
        return DB_SILENCE_FIXED;
    }
    db_to_fixed(20.0 * amp.log10()).max(DB_SILENCE_FIXED)
}

/// Inverse of [`db_to_fixed`].
pub fn fixed_to_db(fixed: i32) -> f32 {
    fixed as f32 / 100.0
}

/// All telemetry fields. Written by the audio thread, read by the F12 panel.
#[derive(Debug)]
pub struct AudioTelemetry {
    // ── Timing ──────────────────────────────────────────────
    /// Last audio-callback duration, in microseconds.
    pub cycle_time_us: AtomicU64,
    /// Worst-case callback duration since the last [`Self::reset_max_cycle`] knob.
    pub max_cycle_time_us: AtomicU64,
    /// The deadline budget (default [`DSP_DEADLINE_US`]); tunable for stress tests.
    pub deadline_us: AtomicU64,
    /// Lifetime buffer underruns.
    pub underrun_count: AtomicU64,
    /// Total audio-callback invocations (monotonic; one per cpal cycle).
    pub cycle_count: AtomicU64,
    /// Callback-cycle number at each underrun (diagnostic ring, oldest slot
    /// overwritten past [`UNDERRUN_LOG_CAP`]). Read via [`Self::underrun_log`].
    pub underrun_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// Count of underrun-log writes (may exceed `UNDERRUN_LOG_CAP`; `% CAP` is
    /// the live window size, `wrapping` past it is the write cursor).
    pub underrun_log_len: AtomicU64,
    /// Last deck sample-position published by the feeder thread (`Mixer::sample_position`,
    /// ~120 Hz). Cross-thread landing pad so the cpal callback can stamp "which sample
    /// of the loudest deck was last known" at underrun time, independent of cpal's
    /// own (OS-decided, not in source) callback cadence.
    pub last_deck_sample_pos: AtomicU64,
    /// Deck sample-position at each underrun (diagnostic ring, parallel to
    /// [`Self::underrun_log`] — same index = same underrun event).
    pub underrun_pos_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// Last feeder-thread `Mixer::mix_block` duration, in microseconds
    /// (`real_feeder_loop` publishes every ~120 Hz tick, `engine_adapter.rs`
    /// `mix_us`). Cross-thread landing pad, same shape as `last_deck_sample_pos`.
    pub last_mix_us: AtomicU64,
    /// Worst-case `mix_block` duration seen (running max, mirrors
    /// `max_cycle_time_us`).
    pub max_mix_us: AtomicU64,
    /// Feeder `mix_us` at each underrun (diagnostic ring, parallel to
    /// [`Self::underrun_log`]/[`Self::underrun_pos_log`] — same index = same
    /// underrun event). AUDIO-ONE-BUS root-cause-WHY probe (2026-07-19):
    /// distinguishes compute-bound stall (mix_us spikes) from feeder-thread
    /// scheduling starvation (mix_us stays normal while the deck position
    /// freezes).
    pub underrun_mix_us_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// Last feeder-thread whole-outer-loop-iteration duration, in microseconds
    /// (`real_feeder_loop` publishes every pass around its `loop {}`, including
    /// the early-`continue` sleep paths — not just the `mix_block` span).
    /// AUDIO-ONE-BUS follow-up (2026-07-19): `mix_us` alone proved the stall
    /// isn't compute cost inside `mix_block` (22us at every underrun cycle
    /// against a 2500us deadline); this widens the probe to the whole iteration
    /// so command-drain / recorder-commit / snapshot-publish / the sleep branch
    /// can be told apart from a scheduling gap between calls.
    pub last_iter_us: AtomicU64,
    /// Worst-case whole-iteration duration seen (running max, mirrors `max_mix_us`).
    pub max_iter_us: AtomicU64,
    /// Feeder `iter_us` at each underrun (diagnostic ring, parallel to
    /// [`Self::underrun_mix_us_log`] — same index = same underrun event). A
    /// spike here with `mix_us` still small pins the stall to the rest of the
    /// loop body (or the gap before the next iteration started), not to `mix_block`.
    pub underrun_iter_us_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// [`mono_us`] stamp of the feeder's CURRENT in-flight iteration start
    /// (published at every loop top). The completed-iteration probes above can
    /// never see a stall still in progress — this one lets the callback compute
    /// how long the in-flight trip has been running (0 = feeder never started).
    pub iter_start_us: AtomicU64,
    /// Which section the in-flight iteration last passed (see [`iter_phase`]).
    pub iter_phase: AtomicU64,
    /// In-flight iteration AGE (µs) at each underrun (ring, index-aligned with
    /// [`Self::underrun_log`]). Tiny age = the trip just started (stall was the
    /// gap between trips = preemption); large/growing age = stuck inside the
    /// trip, and the phase stamp names the section.
    pub underrun_age_us_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// [`iter_phase`] code at each underrun (ring, index-aligned).
    pub underrun_phase_log: [AtomicU64; UNDERRUN_LOG_CAP],
    /// Total feeder-loop trips started (monotonic; one per [`Self::record_iter_start`]).
    pub iter_count: AtomicU64,
    /// Feeder GAP ring (2026-07-19, closes the exit-stamp blind spot): when a new
    /// trip starts and the PREVIOUS trip ran > [`GAP_LOG_US`], record (age, phase,
    /// deck-pos) — catches a monster trip regardless of which exit path it took.
    pub gap_age_log: [AtomicU64; UNDERRUN_LOG_CAP],
    pub gap_phase_log: [AtomicU64; UNDERRUN_LOG_CAP],
    pub gap_pos_log: [AtomicU64; UNDERRUN_LOG_CAP],
    pub gap_log_len: AtomicU64,

    // ── Determinism ─────────────────────────────────────────
    /// Hash of the last output window (0 == not yet sampled).
    pub determinism_hash: AtomicU64,
    /// Consecutive windows whose hash matched the previous one.
    pub determinism_match_secs: AtomicU64,
    /// Lifetime count of windows whose hash differed from the previous one.
    pub determinism_mismatch: AtomicU64,

    // ── Bus levels (fixed-point: value / 100 == dB) ─────────
    pub master_rms_db: AtomicI32,
    pub master_peak_db: AtomicI32,
    pub sfx_rms_db: AtomicI32,
    pub music_rms_db: AtomicI32,

    // ── Per-channel metering (fixed-point: value / 100 == dB) ──
    pub true_peak_l_fixed: AtomicI32,
    pub true_peak_r_fixed: AtomicI32,
    /// Phase correlation × 10000 (Permyriad: 10000 = in-phase, −10000 = inverted).
    pub phase_pmy: AtomicI32,

    // ── System-audio loopback lane (fixed-point: value / 100 == dB) ──
    /// 1 while `crate::loopback::SystemAudioCapture` (module currently disabled
    /// — `mod loopback;` is commented out in `lib.rs`) is publishing windows,
    /// 0 when the lane is stopped — the glass must never read silence as "off".
    pub system_live: AtomicU8,
    pub system_rms_db: AtomicI32,
    /// Low / high band RMS of the same window (`loopback::band_split`).
    pub system_low_db: AtomicI32,
    pub system_high_db: AtomicI32,

    // ── Queue health ────────────────────────────────────────
    pub mixer_cmd_queue_len: AtomicU64,
    pub mixer_cmd_queue_cap: AtomicU64,
    pub recipe_queue_len: AtomicU64,

    // ── Ring buffer ─────────────────────────────────────────
    /// 0..=100.
    pub ring_buffer_fill_pct: AtomicU64,

    // ── SeeHear output (fixed-point * 1000) ─────────────
    pub synth_bloom: AtomicU32,
    pub synth_trauma: AtomicU32,
    pub synth_grain: AtomicU32,
    pub synth_void_pinch: AtomicU32,

    // ── Master band energy (fixed-point * 1000: 0..=1000) ──
    /// Post-gain bass band (64-bin FFT, bins 0–8 mean). Audio-brush warm-hue driver.
    pub master_bass_milli: AtomicU32,
    /// Post-gain treble band (bins 40–64 mean). Audio-brush cool-hue driver.
    pub master_treble_milli: AtomicU32,
    /// Full 64-bin FFT spectrum (fixed-point * 1000 each) — same source as the
    /// bass/treble bands above, published whole for spectrum visualizers.
    pub master_fft_bins: [AtomicU32; 64],

    // ── Last recipe ─────────────────────────────────────────
    pub last_recipe_id: AtomicU64,
    pub last_recipe_freq_hz: AtomicU64,
    pub last_recipe_synth_us: AtomicU64,
    pub last_recipe_seed: AtomicU64,

    // ── Roadie status ───────────────────────────────────────
    /// See [`roadie`] codes (0 == OK).
    pub roadie_severity: AtomicU8,
    /// `audio_issues::IssueType as u8 + 1` (0 == none).
    pub roadie_diagnosis: AtomicU8,
}

impl AudioTelemetry {
    /// All-zero except `deadline_us` (= [`DSP_DEADLINE_US`]) and the bus levels
    /// (= [`DB_SILENCE_FIXED`]). `const` so the singleton needs no runtime init.
    pub const fn new() -> Self {
        Self {
            cycle_time_us: AtomicU64::new(0),
            max_cycle_time_us: AtomicU64::new(0),
            deadline_us: AtomicU64::new(DSP_DEADLINE_US),
            underrun_count: AtomicU64::new(0),
            cycle_count: AtomicU64::new(0),
            underrun_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            underrun_log_len: AtomicU64::new(0),
            last_deck_sample_pos: AtomicU64::new(0),
            underrun_pos_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            last_mix_us: AtomicU64::new(0),
            max_mix_us: AtomicU64::new(0),
            underrun_mix_us_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            last_iter_us: AtomicU64::new(0),
            max_iter_us: AtomicU64::new(0),
            underrun_iter_us_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            iter_start_us: AtomicU64::new(0),
            iter_phase: AtomicU64::new(0),
            underrun_age_us_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            underrun_phase_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            iter_count: AtomicU64::new(0),
            gap_age_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            gap_phase_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            gap_pos_log: [const { AtomicU64::new(0) }; UNDERRUN_LOG_CAP],
            gap_log_len: AtomicU64::new(0),
            determinism_hash: AtomicU64::new(0),
            determinism_match_secs: AtomicU64::new(0),
            determinism_mismatch: AtomicU64::new(0),
            master_rms_db: AtomicI32::new(DB_SILENCE_FIXED),
            master_peak_db: AtomicI32::new(DB_SILENCE_FIXED),
            sfx_rms_db: AtomicI32::new(DB_SILENCE_FIXED),
            music_rms_db: AtomicI32::new(DB_SILENCE_FIXED),
            true_peak_l_fixed: AtomicI32::new(DB_SILENCE_FIXED),
            true_peak_r_fixed: AtomicI32::new(DB_SILENCE_FIXED),
            phase_pmy: AtomicI32::new(10_000),
            system_live: AtomicU8::new(0),
            system_rms_db: AtomicI32::new(DB_SILENCE_FIXED),
            system_low_db: AtomicI32::new(DB_SILENCE_FIXED),
            system_high_db: AtomicI32::new(DB_SILENCE_FIXED),
            mixer_cmd_queue_len: AtomicU64::new(0),
            mixer_cmd_queue_cap: AtomicU64::new(0),
            recipe_queue_len: AtomicU64::new(0),
            ring_buffer_fill_pct: AtomicU64::new(0),
            synth_bloom: AtomicU32::new(0),
            synth_trauma: AtomicU32::new(0),
            synth_grain: AtomicU32::new(0),
            synth_void_pinch: AtomicU32::new(0),
            master_bass_milli: AtomicU32::new(0),
            master_treble_milli: AtomicU32::new(0),
            master_fft_bins: [const { AtomicU32::new(0) }; 64],
            last_recipe_id: AtomicU64::new(0),
            last_recipe_freq_hz: AtomicU64::new(0),
            last_recipe_synth_us: AtomicU64::new(0),
            last_recipe_seed: AtomicU64::new(0),
            roadie_severity: AtomicU8::new(roadie::OK),
            roadie_diagnosis: AtomicU8::new(0),
        }
    }

    /// Record one audio-callback cycle time (µs), updating the running worst case.
    /// Single-writer (the audio thread), so the max is a plain load/compare/store.
    pub fn record_cycle(&self, us: u64) {
        self.cycle_time_us.store(us, Ordering::Relaxed);
        if us > self.max_cycle_time_us.load(Ordering::Relaxed) {
            self.max_cycle_time_us.store(us, Ordering::Relaxed);
        }
    }

    /// Clear the worst-case cycle time (the panel's "Reset Peak" knob).
    pub fn reset_max_cycle(&self) {
        self.max_cycle_time_us.store(0, Ordering::Relaxed);
    }

    /// True if the last recorded cycle blew the deadline budget.
    pub fn over_deadline(&self) -> bool {
        self.cycle_time_us.load(Ordering::Relaxed) > self.deadline_us.load(Ordering::Relaxed)
    }

    /// The current deadline budget, in microseconds.
    pub fn deadline_us(&self) -> u64 {
        self.deadline_us.load(Ordering::Relaxed)
    }

    /// Override the deadline budget (the panel's "Deadline Override" knob).
    pub fn set_deadline_us(&self, us: u64) {
        self.deadline_us.store(us, Ordering::Relaxed);
    }

    /// Set the lifetime underrun count (mirrors the realtime path's counter).
    pub fn set_underruns(&self, n: u64) {
        self.underrun_count.store(n, Ordering::Relaxed);
    }

    /// Bump the per-callback cycle counter; returns the cycle number just
    /// completed (0-indexed). Call once per cpal callback, before draining.
    pub fn tick_cycle(&self) -> u64 {
        self.cycle_count.fetch_add(1, Ordering::Relaxed)
    }

    /// Record that cycle `n` underran (diagnostic ring — AUDIO-ONE-BUS probe).
    /// Zero-alloc: fixed array store + one atomic increment, safe for the
    /// realtime callback.
    pub fn log_underrun_cycle(&self, n: u64) {
        let write_at = self.underrun_log_len.fetch_add(1, Ordering::Relaxed);
        let slot = (write_at as usize) % UNDERRUN_LOG_CAP;
        self.underrun_log[slot].store(n, Ordering::Relaxed);
        self.underrun_pos_log[slot].store(self.last_deck_sample_pos.load(Ordering::Relaxed), Ordering::Relaxed);
        self.underrun_mix_us_log[slot].store(self.last_mix_us.load(Ordering::Relaxed), Ordering::Relaxed);
        self.underrun_iter_us_log[slot].store(self.last_iter_us.load(Ordering::Relaxed), Ordering::Relaxed);
        let start = self.iter_start_us.load(Ordering::Relaxed);
        let age = if start == 0 { 0 } else { mono_us().saturating_sub(start) };
        self.underrun_age_us_log[slot].store(age, Ordering::Relaxed);
        self.underrun_phase_log[slot].store(self.iter_phase.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    /// Publish the start of a new feeder-loop iteration (in-flight probe): stamp
    /// [`mono_us`] + reset the phase to [`iter_phase::TOP`]. Call at the loop top.
    /// Also the gap detector: a previous trip older than [`GAP_LOG_US`] is logged
    /// with the phase it ended in — visible no matter which exit path it took.
    pub fn record_iter_start(&self) {
        let now = mono_us();
        let prev = self.iter_start_us.load(Ordering::Relaxed);
        if prev != 0 {
            let prev_age = now.saturating_sub(prev);
            if prev_age > GAP_LOG_US {
                let write_at = self.gap_log_len.fetch_add(1, Ordering::Relaxed);
                let slot = (write_at as usize) % UNDERRUN_LOG_CAP;
                self.gap_age_log[slot].store(prev_age, Ordering::Relaxed);
                self.gap_phase_log[slot].store(self.iter_phase.load(Ordering::Relaxed), Ordering::Relaxed);
                self.gap_pos_log[slot].store(self.last_deck_sample_pos.load(Ordering::Relaxed), Ordering::Relaxed);
            }
        }
        self.iter_count.fetch_add(1, Ordering::Relaxed);
        self.iter_start_us.store(now, Ordering::Relaxed);
        self.iter_phase.store(iter_phase::TOP, Ordering::Relaxed);
    }

    /// Stamp which feeder-loop section was just passed (see [`iter_phase`]).
    /// One relaxed store — safe inside the RT region.
    pub fn set_iter_phase(&self, phase: u64) {
        self.iter_phase.store(phase, Ordering::Relaxed);
    }

    /// Publish one feeder-thread `mix_block` duration (µs), updating the
    /// running worst case. Single-writer (the feeder thread) — AUDIO-ONE-BUS
    /// root-cause-WHY probe.
    pub fn record_mix_us(&self, us: u64) {
        self.last_mix_us.store(us, Ordering::Relaxed);
        if us > self.max_mix_us.load(Ordering::Relaxed) {
            self.max_mix_us.store(us, Ordering::Relaxed);
        }
    }

    /// Publish one feeder-thread whole-outer-loop-iteration duration (µs),
    /// updating the running worst case. Single-writer (the feeder thread) —
    /// AUDIO-ONE-BUS follow-up probe (2026-07-19), call once per pass around
    /// `real_feeder_loop`'s `loop {}`, on every exit path (including the
    /// early-`continue` sleep branches), not just after `mix_block`.
    pub fn record_iter_us(&self, us: u64) {
        self.last_iter_us.store(us, Ordering::Relaxed);
        if us > self.max_iter_us.load(Ordering::Relaxed) {
            self.max_iter_us.store(us, Ordering::Relaxed);
        }
    }

    /// The recorded underrun cycle-numbers, oldest-first, in the live window
    /// (at most [`UNDERRUN_LOG_CAP`] entries even if more underruns occurred).
    pub fn underrun_cycles(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// The recorded deck sample-position at each underrun, same order/window as
    /// [`Self::underrun_cycles`] (index-aligned — pair them to name which sample of
    /// the loudest deck was sounding when cycle N underran).
    pub fn underrun_positions(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_pos_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// The recorded feeder `mix_us` at each underrun, same order/window as
    /// [`Self::underrun_cycles`] (index-aligned). AUDIO-ONE-BUS root-cause-WHY
    /// probe: a spike here (well above the typical/max-cycle scale) means the
    /// stall is compute-bound inside `mix_block`; a normal value with the deck
    /// position still frozen ([`Self::underrun_positions`]) means the feeder
    /// thread stalled *between* `mix_block` calls (scheduling, not DSP cost).
    pub fn underrun_mix_us(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_mix_us_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// The recorded feeder whole-iteration `iter_us` at each underrun, same
    /// order/window as [`Self::underrun_cycles`] (index-aligned). AUDIO-ONE-BUS
    /// follow-up (2026-07-19): pair against [`Self::underrun_mix_us`] — a big
    /// `iter_us` alongside a tiny `mix_us` means the stall is in the rest of the
    /// loop body (command drain, recorder commit, snapshot publish) or the gap
    /// before this iteration started, not inside `mix_block`.
    pub fn underrun_iter_us(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_iter_us_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// The in-flight iteration AGE (µs) at each underrun, same order/window as
    /// [`Self::underrun_cycles`] (index-aligned). AUDIO-ONE-BUS in-flight probe
    /// (2026-07-19): the completed-iteration `iter_us` stamps read 0 at every
    /// underrun because the stalling trip hadn't exited yet — this is the view
    /// that CAN see it. Tiny = preemption between trips; large/growing across a
    /// burst = stuck inside one trip (pair with [`Self::underrun_phases`]).
    pub fn underrun_iter_age_us(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_age_us_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// The logged feeder gaps, oldest-first: (trip age µs, phase it ended in,
    /// deck sample-pos at logging). Every trip that ran longer than
    /// [`GAP_LOG_US`] appears here — including one that evaded the exit-path
    /// `iter_us` stamp.
    pub fn feeder_gaps(&self) -> Vec<(u64, u64, u64)> {
        let total = self.gap_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| {
                let s = (start + i) % UNDERRUN_LOG_CAP;
                (
                    self.gap_age_log[s].load(Ordering::Relaxed),
                    self.gap_phase_log[s].load(Ordering::Relaxed),
                    self.gap_pos_log[s].load(Ordering::Relaxed),
                )
            })
            .collect()
    }

    /// The [`iter_phase`] code at each underrun, same order/window as
    /// [`Self::underrun_cycles`] (index-aligned) — names the feeder section the
    /// in-flight trip last passed when the callback starved.
    pub fn underrun_phases(&self) -> Vec<u64> {
        let total = self.underrun_log_len.load(Ordering::Relaxed);
        let count = total.min(UNDERRUN_LOG_CAP as u64) as usize;
        let start = if total as usize > UNDERRUN_LOG_CAP { total as usize % UNDERRUN_LOG_CAP } else { 0 };
        (0..count)
            .map(|i| self.underrun_phase_log[(start + i) % UNDERRUN_LOG_CAP].load(Ordering::Relaxed))
            .collect()
    }

    /// Feed one determinism-window hash. Increments `determinism_match_secs` when
    /// it matches the previous window, or `determinism_mismatch` (and resets the
    /// match streak) when it differs. The first sample (previous == 0) is a no-op.
    pub fn record_determinism(&self, hash: u64) {
        let prev = self.determinism_hash.swap(hash, Ordering::Relaxed);
        if prev != 0 && prev == hash {
            self.determinism_match_secs.fetch_add(1, Ordering::Relaxed);
        } else if prev != 0 {
            self.determinism_mismatch.fetch_add(1, Ordering::Relaxed);
            self.determinism_match_secs.store(0, Ordering::Relaxed);
        }
    }

    /// Publish per-channel true-peak (dB → fixed-point) and stereo phase correlation (Permyriad).
    pub fn set_meter_bands(&self, peak_l_db: f32, peak_r_db: f32, phase_pmy: i32) {
        self.true_peak_l_fixed.store(db_to_fixed(peak_l_db), Ordering::Relaxed);
        self.true_peak_r_fixed.store(db_to_fixed(peak_r_db), Ordering::Relaxed);
        self.phase_pmy.store(phase_pmy, Ordering::Relaxed);
    }

    /// Store master bus RMS + peak (dB → fixed-point).
    pub fn set_master_levels(&self, rms_db: f32, peak_db: f32) {
        self.master_rms_db.store(db_to_fixed(rms_db), Ordering::Relaxed);
        self.master_peak_db.store(db_to_fixed(peak_db), Ordering::Relaxed);
    }

    /// Store the SFX bus RMS (dB → fixed-point).
    pub fn set_sfx_rms(&self, rms_db: f32) {
        self.sfx_rms_db.store(db_to_fixed(rms_db), Ordering::Relaxed);
    }

    /// Store the music bus RMS (dB → fixed-point).
    pub fn set_music_rms(&self, rms_db: f32) {
        self.music_rms_db.store(db_to_fixed(rms_db), Ordering::Relaxed);
    }

    /// Publish the system-audio loopback window (linear 0..=1 amplitudes from
    /// `crate::loopback::SystemAudioCapture::meter`, module currently disabled)
    /// as dB*100 fixed, so the
    /// AUDIO glass reads the system lane on the same integer currency as the
    /// engine's own buses. `live` is the lane state, never inferred from level.
    pub fn set_system_audio(&self, live: bool, rms: f32, low: f32, high: f32) {
        self.system_live.store(u8::from(live), Ordering::Relaxed);
        self.system_rms_db.store(amp_to_fixed(rms), Ordering::Relaxed);
        self.system_low_db.store(amp_to_fixed(low), Ordering::Relaxed);
        self.system_high_db.store(amp_to_fixed(high), Ordering::Relaxed);
    }

    /// Publish the last synthesized recipe (cold-path `RecipeEngine::synthesize`).
    pub fn set_last_recipe(&self, id: u64, freq_hz: u64, synth_us: u64, seed: u64) {
        self.last_recipe_id.store(id, Ordering::Relaxed);
        self.last_recipe_freq_hz.store(freq_hz, Ordering::Relaxed);
        self.last_recipe_synth_us.store(synth_us, Ordering::Relaxed);
        self.last_recipe_seed.store(seed, Ordering::Relaxed);
    }

    /// Publish the current worst Roadie issue (0 severity == OK; see [`roadie`]).
    pub fn set_roadie(&self, severity: u8, diagnosis: u8) {
        self.roadie_severity.store(severity, Ordering::Relaxed);
        self.roadie_diagnosis.store(diagnosis, Ordering::Relaxed);
    }

    // audio_alloc_count: EXCLUDED — needs crate::alloc_tracer (excluded above).
}

impl Default for AudioTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

/// Process-wide telemetry singleton — written by the audio thread, read by F12.
/// The S-A MASTER OUTPUT GATE. Closed at boot (the boot-silent law): every lane
/// keeps rendering and every meter keeps reading, but the device is handed zeros.
///
/// Lives beside [`TELEMETRY`] and for the same reason — the audio block loop, the
/// key handler and the UI that shows a speaker icon are three different scopes,
/// and a gate only one of them can reach is a gate with no visible control
/// (Sean 2026-08-03: "terminal or env might mute audio on boot" — it does, and
/// until now the only switch was an unlabelled Ctrl+M).
pub static MASTER_GATE_OPEN: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Is the master gate open (is the device hearing anything)?
#[inline]
pub fn gate_is_open() -> bool {
    MASTER_GATE_OPEN.load(Ordering::Relaxed)
}

/// Open or close the master gate. Returns the new state.
#[inline]
pub fn set_gate_open(open: bool) -> bool {
    MASTER_GATE_OPEN.store(open, Ordering::Relaxed);
    open
}

/// Where the gate's remembered state lives, relative to the repo root.
pub const GATE_STATE_PATH: &str = ".forge/audio_gate.json";

/// Read the remembered gate state (Sean 2026-08-03: "boot silent, remember last
/// state"). `None` when there is no memory yet.
///
/// A missing, unreadable or malformed file is `None`, never a panic: a corrupt
/// settings file must not be the thing that decides your speakers.
pub fn load_gate_state(root: &std::path::Path) -> Option<bool> {
    let raw = std::fs::read_to_string(root.join(GATE_STATE_PATH)).ok()?;
    // One field, so one scan — no serde dependency for a single bool.
    let v = raw.split("\"open\"").nth(1)?;
    let v = v.split(':').nth(1)?.trim_start();
    if v.starts_with("true") {
        Some(true)
    } else if v.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

/// The gate state to boot with. Memory first; with NO memory the gate opens.
///
/// Silence is not the fallback (Sean 2026-08-03: "kill stale, 0.0 is not an
/// option"). A gate that defaults closed on every unknown — first run, wiped
/// `.forge`, unreadable file — is a studio that is silent by accident and calls
/// it a law. The remembered answer wins; absent one, sound is the default and
/// `FORGE_AUDIO=0` is how you ask for quiet.
pub fn boot_gate_state(root: &std::path::Path) -> bool {
    load_gate_state(root).unwrap_or(true)
}

/// Remember the gate's state for the next boot. Best-effort: a failed write
/// costs the next launch its memory and nothing else, so it is logged, never
/// propagated into a caller that was only trying to toggle sound.
pub fn save_gate_state(root: &std::path::Path, open: bool) {
    let path = root.join(GATE_STATE_PATH);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = format!("{{ \"open\": {open} }}\n"); // @forge:allow_alloc settings write, on a press
    if let Err(e) = std::fs::write(&path, body) {
        log::warn!("[audio-gate] could not remember state at {path:?}: {e}");
    }
}

#[cfg(test)]
mod gate_state_tests {
    use super::*;

    #[test]
    fn a_saved_gate_state_round_trips() {
        let d = tempfile::tempdir().unwrap();
        save_gate_state(d.path(), true);
        assert_eq!(load_gate_state(d.path()), Some(true));
        save_gate_state(d.path(), false);
        assert_eq!(load_gate_state(d.path()), Some(false));
    }

    #[test]
    fn no_memory_boots_audible_not_silent() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(load_gate_state(d.path()), None);
        assert!(boot_gate_state(d.path()), "silence is not the fallback for 'unknown'");
    }

    #[test]
    fn a_corrupt_file_boots_audible_too() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join(".forge")).unwrap();
        std::fs::write(d.path().join(GATE_STATE_PATH), "{ \"open\": maybe }").unwrap();
        assert_eq!(load_gate_state(d.path()), None);
        assert!(boot_gate_state(d.path()), "a bad file must not silence the studio");
    }

    #[test]
    fn a_remembered_close_is_honoured() {
        let d = tempfile::tempdir().unwrap();
        save_gate_state(d.path(), false);
        assert!(!boot_gate_state(d.path()), "you turned it off; it stays off");
    }
}

pub static TELEMETRY: AudioTelemetry = AudioTelemetry::new();

/// Borrow the process-wide telemetry singleton.
pub fn telemetry() -> &'static AudioTelemetry {
    &TELEMETRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_defaults_deadline_and_silence() {
        let t = AudioTelemetry::new();
        assert_eq!(t.deadline_us(), DSP_DEADLINE_US);
        assert_eq!(t.master_rms_db.load(Ordering::Relaxed), DB_SILENCE_FIXED);
        assert_eq!(t.roadie_severity.load(Ordering::Relaxed), roadie::OK);
    }

    #[test]
    fn record_cycle_tracks_running_max() {
        let t = AudioTelemetry::new();
        t.record_cycle(1_000);
        assert_eq!(t.cycle_time_us.load(Ordering::Relaxed), 1_000);
        assert_eq!(t.max_cycle_time_us.load(Ordering::Relaxed), 1_000);

        t.record_cycle(500); // lower → max stays sticky
        assert_eq!(t.cycle_time_us.load(Ordering::Relaxed), 500);
        assert_eq!(t.max_cycle_time_us.load(Ordering::Relaxed), 1_000);

        t.record_cycle(3_000); // higher → max rises
        assert_eq!(t.max_cycle_time_us.load(Ordering::Relaxed), 3_000);

        t.reset_max_cycle();
        assert_eq!(t.max_cycle_time_us.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn over_deadline_reflects_budget() {
        let t = AudioTelemetry::new(); // deadline 2500 µs
        t.record_cycle(1_000);
        assert!(!t.over_deadline());
        t.record_cycle(3_000);
        assert!(t.over_deadline());
        t.set_deadline_us(4_000); // raise budget → no longer over
        assert!(!t.over_deadline());
    }

    #[test]
    fn determinism_tracks_match_and_mismatch() {
        let t = AudioTelemetry::new();
        t.record_determinism(0xABCD); // first sample → no-op
        assert_eq!(t.determinism_match_secs.load(Ordering::Relaxed), 0);
        assert_eq!(t.determinism_mismatch.load(Ordering::Relaxed), 0);

        t.record_determinism(0xABCD); // match → streak++
        assert_eq!(t.determinism_match_secs.load(Ordering::Relaxed), 1);

        t.record_determinism(0x1234); // mismatch → mismatch++, streak reset
        assert_eq!(t.determinism_mismatch.load(Ordering::Relaxed), 1);
        assert_eq!(t.determinism_match_secs.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn underrun_log_records_cycle_numbers_in_order() {
        let t = AudioTelemetry::new();
        assert_eq!(t.tick_cycle(), 0);
        assert_eq!(t.tick_cycle(), 1);
        let n2 = t.tick_cycle(); // 2
        t.log_underrun_cycle(n2);
        let n3 = t.tick_cycle(); // 3
        let _ = n3;
        let n4 = t.tick_cycle(); // 4
        t.log_underrun_cycle(n4);
        assert_eq!(t.underrun_cycles(), vec![2, 4]);
    }

    #[test]
    fn underrun_log_records_mix_us_index_aligned() {
        let t = AudioTelemetry::new();
        t.record_mix_us(120);
        let n0 = t.tick_cycle();
        t.log_underrun_cycle(n0); // stamps mix_us=120
        t.record_mix_us(9_500); // feeder spike before the next underrun
        let n1 = t.tick_cycle();
        t.log_underrun_cycle(n1); // stamps mix_us=9500
        assert_eq!(t.underrun_mix_us(), vec![120, 9_500]);
        assert_eq!(t.max_mix_us.load(Ordering::Relaxed), 9_500);
    }

    #[test]
    fn underrun_log_records_iter_us_index_aligned() {
        let t = AudioTelemetry::new();
        t.record_iter_us(180);
        let n0 = t.tick_cycle();
        t.log_underrun_cycle(n0); // stamps iter_us=180
        t.record_iter_us(4_800); // whole-loop stall before the next underrun
        let n1 = t.tick_cycle();
        t.log_underrun_cycle(n1); // stamps iter_us=4800
        assert_eq!(t.underrun_iter_us(), vec![180, 4_800]);
        assert_eq!(t.max_iter_us.load(Ordering::Relaxed), 4_800);
    }

    #[test]
    fn underrun_log_records_iter_age_and_phase_index_aligned() {
        let t = AudioTelemetry::new();
        // Feeder never started: age stamps 0, phase stamps 0.
        let n0 = t.tick_cycle();
        t.log_underrun_cycle(n0);
        assert_eq!(t.underrun_iter_age_us(), vec![0]);
        assert_eq!(t.underrun_phases(), vec![0]);
        // In-flight trip: start stamped, section marked — age must be a real
        // (tiny but bounded) elapsed reading, phase the marked section.
        t.record_iter_start();
        t.set_iter_phase(iter_phase::SLEEP_LOW_SLOTS);
        let n1 = t.tick_cycle();
        t.log_underrun_cycle(n1);
        let ages = t.underrun_iter_age_us();
        assert_eq!(ages.len(), 2);
        assert!(ages[1] < 10_000_000, "age must be µs-since-start, got {}", ages[1]);
        assert_eq!(t.underrun_phases(), vec![0, iter_phase::SLEEP_LOW_SLOTS]);
        // record_iter_start resets the phase to TOP.
        t.record_iter_start();
        assert_eq!(t.iter_phase.load(Ordering::Relaxed), iter_phase::TOP);
    }

    #[test]
    fn underrun_log_wraps_past_capacity() {
        let t = AudioTelemetry::new();
        // Log CAP+3 underruns at cycles 0..CAP+3; the window must keep only the
        // newest CAP, oldest-first, never panic on the wraparound.
        let total = UNDERRUN_LOG_CAP + 3;
        for cycle in 0..total as u64 {
            t.log_underrun_cycle(cycle);
        }
        let window = t.underrun_cycles();
        assert_eq!(window.len(), UNDERRUN_LOG_CAP);
        assert_eq!(window.first().copied(), Some(3)); // oldest surviving = cycle 3
        assert_eq!(window.last().copied(), Some((total - 1) as u64));
    }

    #[test]
    fn db_fixed_point_roundtrips() {
        assert_eq!(db_to_fixed(-6.0), -600);
        assert_eq!(db_to_fixed(0.0), 0);
        assert!((fixed_to_db(-600) + 6.0).abs() < 1e-4);
        // Bus-level setter stores the fixed-point form.
        let t = AudioTelemetry::new();
        t.set_master_levels(-6.0, -0.5);
        assert_eq!(t.master_rms_db.load(Ordering::Relaxed), -600);
        assert_eq!(t.master_peak_db.load(Ordering::Relaxed), -50);
    }
}