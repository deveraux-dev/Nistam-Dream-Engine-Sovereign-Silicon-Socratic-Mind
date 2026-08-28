//! `WacomQuantizer` — deterministic tablet (stylus/pen) input quantization.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-input\src\wacom.rs` — the
//! wacom lane this crate's own module doc originally named as NOT ported.
//! Same 240Hz resample lattice as [`crate::gamepad::PadQuantizer`] (one
//! `INPUT_TICK_RATE_HZ` owner across both lanes, so a recorded pad tape and
//! a recorded pen tape share one clock with no conversion step).
//!
//! `QuantizedTabletSample::pressure` (`u16` Permyriad, `0..=10000`) lands
//! directly on `forge-brush-v3::engine::BrushEngine::effective_size`/
//! `effective_opacity`'s `pressure_permyriad: u16` parameter — no adapter
//! needed between this crate and the brush engine, confirmed by reading both
//! call sites (same Permyriad convention this workspace already commits to).
//!
//! # Architecture
//! - Raw HID collection is a host concern (v2's collection site was
//!   `forge-gpu/sovereign_window.rs`, Windows Raw Input API) — v3 has no
//!   such collector yet; this module is data-only, same firewall as
//!   [`crate::gamepad`].
//! - This module defines the quantized output type and the resampling/
//!   recording logic only.

/// Raw tablet sample from the HID layer (pre-quantization).
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct RawTabletSample {
    /// Timestamp in microseconds (monotonic, host's high-resolution clock).
    pub timestamp_us: u64,
    /// X position in device units (`0..max_x`).
    pub x: f32,
    /// Y position in device units (`0..max_y`).
    pub y: f32,
    /// Pressure, `0.0..1.0`.
    pub pressure: f32,
    /// Tilt X in degrees (`-90..90`).
    pub tilt_x: f32,
    /// Tilt Y in degrees (`-90..90`).
    pub tilt_y: f32,
    /// Pen is in proximity (hovering or touching).
    pub in_proximity: bool,
    /// Pen is touching the surface.
    pub in_contact: bool,
    /// Button bitmask (barrel button, eraser, etc.).
    pub buttons: u8,
}

/// Quantized tablet sample — deterministic integer types, ready to cross an
/// SPSC ring boundary or feed `BrushEngine` directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct QuantizedTabletSample {
    /// Tick index at 240Hz (monotonic).
    pub tick: u64,
    /// X position in MilliUnit (1000 = 1 pixel).
    pub x: i64,
    /// Y position in MilliUnit (1000 = 1 pixel).
    pub y: i64,
    /// Pressure as Permyriad (`0..=10000`) — the exact type
    /// `BrushEngine::effective_size`/`effective_opacity` expect.
    pub pressure: u16,
    /// Tilt X as centi-degrees (`-9000..=9000`, 100 per degree).
    pub tilt_x: i16,
    /// Tilt Y as centi-degrees (`-9000..=9000`).
    pub tilt_y: i16,
    /// Pen state flags.
    pub flags: u8,
}

impl QuantizedTabletSample {
    /// Pen is in proximity (hovering or touching) — see `flags`.
    pub const FLAG_PROXIMITY: u8 = 0x01;
    /// Pen is touching the surface — see `flags`.
    pub const FLAG_CONTACT: u8 = 0x02;
    /// Barrel button held — see `flags`.
    pub const FLAG_BARREL: u8 = 0x04;
    /// Eraser end in use — see `flags`.
    pub const FLAG_ERASER: u8 = 0x08;
}

/// The INPUT-lane resample clock (240Hz) — shared with
/// [`crate::gamepad::PadQuantizer`]; both quantizers in this crate resample
/// onto the same lattice so a tablet tape and a pad tape never need a
/// conversion step to compare ticks.
pub const INPUT_TICK_RATE_HZ: u64 = 240;
/// Microseconds per input tick at [`INPUT_TICK_RATE_HZ`] (4166us).
pub const INPUT_TICK_PERIOD_US: u64 = 1_000_000 / INPUT_TICK_RATE_HZ;

/// Fixed-rate resampler: irregular HID samples to uniform 240Hz ticks (lerp
/// between bracketing samples, flags from the latest, optional recording).
pub struct WacomQuantizer {
    /// Device resolution for X normalization.
    pub max_x: f32,
    /// Device resolution for Y normalization.
    pub max_y: f32,
    /// Screen width in pixels (for MilliUnit conversion).
    pub screen_w: i64,
    /// Screen height in pixels (for MilliUnit conversion).
    pub screen_h: i64,
    tick_period_us: u64,
    next_tick_us: u64,
    current_tick: u64,
    prev_sample: RawTabletSample,
    initialized: bool,
    replay_buffer: Option<Vec<QuantizedTabletSample>>,
}

impl WacomQuantizer {
    /// Construct a quantizer for a device with the given resolution
    /// (`max_x`/`max_y`) targeting a screen of `screen_w`x`screen_h` pixels.
    pub fn new(max_x: f32, max_y: f32, screen_w: i64, screen_h: i64) -> Self {
        Self {
            max_x,
            max_y,
            screen_w,
            screen_h,
            tick_period_us: INPUT_TICK_PERIOD_US,
            next_tick_us: 0,
            current_tick: 0,
            prev_sample: RawTabletSample::default(),
            initialized: false,
            replay_buffer: None,
        }
    }

    /// Start recording the quantized stream to a replay buffer.
    pub fn start_recording(&mut self) {
        self.replay_buffer = Some(Vec::with_capacity(240 * 60)); // 1 minute at 240Hz
    }

    /// Stop recording and hand the buffer over.
    pub fn stop_recording(&mut self) -> Option<Vec<QuantizedTabletSample>> {
        self.replay_buffer.take()
    }

    /// Feed a raw HID sample. Emits zero or more quantized 240Hz ticks into
    /// `output` — more than one if the time gap spans multiple tick periods.
    pub fn feed(&mut self, sample: &RawTabletSample, output: &mut Vec<QuantizedTabletSample>) {
        if !self.initialized {
            self.initialized = true;
            self.next_tick_us = sample.timestamp_us + self.tick_period_us;
            self.prev_sample = sample.clone();
            return;
        }

        while self.next_tick_us <= sample.timestamp_us {
            let t = if sample.timestamp_us == self.prev_sample.timestamp_us {
                1.0
            } else {
                (self.next_tick_us - self.prev_sample.timestamp_us) as f32
                    / (sample.timestamp_us - self.prev_sample.timestamp_us) as f32
            };

            let quantized = self.interpolate_and_quantize(&self.prev_sample.clone(), sample, t);
            if let Some(ref mut buf) = self.replay_buffer {
                buf.push(quantized);
            }
            output.push(quantized);

            self.next_tick_us += self.tick_period_us;
            self.current_tick += 1;
        }

        self.prev_sample = sample.clone();
    }

    fn interpolate_and_quantize(
        &self,
        a: &RawTabletSample,
        b: &RawTabletSample,
        t: f32,
    ) -> QuantizedTabletSample {
        let lerp = |v0: f32, v1: f32| v0 + (v1 - v0) * t;

        let nx = lerp(a.x, b.x) / self.max_x;
        let ny = lerp(a.y, b.y) / self.max_y;

        // Normalized device space -> MilliUnit (1000 = 1 pixel).
        let x = (nx * self.screen_w as f32 * 1000.0) as i64;
        let y = (ny * self.screen_h as f32 * 1000.0) as i64;

        // Pressure -> Permyriad (0..=10000).
        let pressure = (lerp(a.pressure, b.pressure).clamp(0.0, 1.0) * 10000.0) as u16;

        // Tilt -> centi-degrees, +/-9000 (100 per degree).
        let tilt_x = (lerp(a.tilt_x, b.tilt_x).clamp(-90.0, 90.0) * 100.0) as i16;
        let tilt_y = (lerp(a.tilt_y, b.tilt_y).clamp(-90.0, 90.0) * 100.0) as i16;

        // Flags from the latest sample — booleans never interpolate.
        let flags = if b.in_proximity { QuantizedTabletSample::FLAG_PROXIMITY } else { 0 }
            | if b.in_contact { QuantizedTabletSample::FLAG_CONTACT } else { 0 }
            | if b.buttons & 0x01 != 0 { QuantizedTabletSample::FLAG_BARREL } else { 0 }
            | if b.buttons & 0x02 != 0 { QuantizedTabletSample::FLAG_ERASER } else { 0 };

        QuantizedTabletSample { tick: self.current_tick, x, y, pressure, tilt_x, tilt_y, flags }
    }

    /// Sample at a tick in a recorded buffer (linear scan — buffers are
    /// tick-monotonic but not index-aligned to tick, unlike the pad tape).
    pub fn replay_at(buffer: &[QuantizedTabletSample], tick: u64) -> Option<&QuantizedTabletSample> {
        buffer.iter().find(|s| s.tick == tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantize_pressure_bounds() {
        let mut q = WacomQuantizer::new(1000.0, 1000.0, 1920, 1080);
        let mut out = Vec::new();

        q.feed(&RawTabletSample { timestamp_us: 0, pressure: 0.5, ..Default::default() }, &mut out);
        assert!(out.is_empty());

        q.feed(
            &RawTabletSample { timestamp_us: INPUT_TICK_PERIOD_US, pressure: 1.0, ..Default::default() },
            &mut out,
        );
        assert!(!out.is_empty());
        assert!(out[0].pressure <= 10000);
    }

    #[test]
    fn quantize_tilt_clamps() {
        let mut q = WacomQuantizer::new(1000.0, 1000.0, 1920, 1080);
        let mut out = Vec::new();

        q.feed(&RawTabletSample { timestamp_us: 0, tilt_x: -100.0, ..Default::default() }, &mut out);
        q.feed(
            &RawTabletSample { timestamp_us: INPUT_TICK_PERIOD_US, tilt_x: 100.0, ..Default::default() },
            &mut out,
        );

        for s in &out {
            assert!(s.tilt_x >= -9000 && s.tilt_x <= 9000);
        }
    }

    #[test]
    fn resample_produces_uniform_ticks() {
        let mut q = WacomQuantizer::new(1000.0, 1000.0, 1920, 1080);
        let mut out = Vec::new();

        q.feed(&RawTabletSample { timestamp_us: 0, x: 0.0, ..Default::default() }, &mut out);
        q.feed(
            &RawTabletSample { timestamp_us: INPUT_TICK_PERIOD_US * 3, x: 300.0, ..Default::default() },
            &mut out,
        );

        assert_eq!(out.len(), 3);
        assert_eq!(out[0].tick, 0);
        assert_eq!(out[1].tick, 1);
        assert_eq!(out[2].tick, 2);
    }

    #[test]
    fn recording_captures_all_ticks() {
        let mut q = WacomQuantizer::new(1000.0, 1000.0, 1920, 1080);
        let mut out = Vec::new();

        q.start_recording();
        q.feed(&RawTabletSample { timestamp_us: 0, ..Default::default() }, &mut out);
        q.feed(
            &RawTabletSample { timestamp_us: INPUT_TICK_PERIOD_US * 5, ..Default::default() },
            &mut out,
        );

        let buf = q.stop_recording().unwrap();
        assert_eq!(buf.len(), out.len());
    }

    #[test]
    fn replay_at_finds_tick() {
        let samples = vec![
            QuantizedTabletSample { tick: 0, pressure: 5000, ..Default::default() },
            QuantizedTabletSample { tick: 1, pressure: 7000, ..Default::default() },
        ];
        assert_eq!(WacomQuantizer::replay_at(&samples, 1).unwrap().pressure, 7000);
        assert!(WacomQuantizer::replay_at(&samples, 99).is_none());
    }

    /// A quantized sample's `pressure` field is directly the `u16` type
    /// `forge_brush_v3::engine::BrushEngine::effective_size` takes as
    /// `pressure_permyriad` — this is the wire-compat guarantee, not a
    /// cross-crate test (forge-input-v3 has no dep on forge-brush-v3).
    #[test]
    fn pressure_type_matches_brush_engine_permyriad_convention() {
        let s = QuantizedTabletSample { pressure: 10000, ..Default::default() };
        let _pressure_permyriad: u16 = s.pressure;
        assert!(s.pressure <= 10000);
    }
}
