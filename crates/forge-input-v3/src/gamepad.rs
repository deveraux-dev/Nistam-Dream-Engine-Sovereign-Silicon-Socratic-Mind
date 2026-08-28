//! `PadQuantizer` — deterministic gamepad quantization + byte-replayable tape.
//!
//! Ported verbatim (minus the `forge_core::lockstep` hash-owner cross-reference)
//! from `F:\NewRepo\crates\forge-input\src\gamepad.rs`.
//!
//! That cross-reference now HAS a v3 home — `forge_core_v3::lockstep`, landed
//! 2026-08-15 (see `.forge/repo-map.tsv`, gen-4 of 4). It is still deliberately
//! not taken here: this crate ships zero runtime dependencies, and a pad tape is
//! already replayable on its own — a tape frame's tick IS its index. The seam
//! belongs to the caller that owns both sides, which digests a frame into a
//! lockstep input word rather than making this crate depend on the barrier.
//! Resamples irregular XInput polls onto a uniform 240Hz integer lattice —
//! same tick rate `bone_timeline.rs`'s authoring grid and this workspace's other
//! 240Hz-class quantizers already assume, so a recorded pad tape and a recorded
//! tick-driven scene share one clock without a conversion step.
//!
//! This crate does not poll hardware — `RawPadSample` is the boundary a host
//! (an `XInputGetState` call site, not present in this crate) is expected to
//! fill in and feed to `PadQuantizer::feed`.

use crate::deadzone::apply_radial_deadzone;

/// Fixed input-tick rate, matching the 240Hz lattice this workspace's other
/// input quantizers already resample onto.
pub const INPUT_TICK_RATE_HZ: u64 = 240;
/// Microseconds per input tick at [`INPUT_TICK_RATE_HZ`] (4166us).
pub const INPUT_TICK_PERIOD_US: u64 = 1_000_000 / INPUT_TICK_RATE_HZ;

/// Raw pad sample as a host's `XInputGetState`-equivalent poll site produces
/// it. Sticks/triggers are normalized `f32`; they die at the quantization
/// boundary (`PadQuantizer::feed`) and never reach [`QuantizedPadFrame`].
#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct RawPadSample {
    /// Monotonic microseconds (the host's high-resolution poll clock).
    pub timestamp_us: u64,
    /// Left stick X, `-1.0..1.0`.
    pub lx: f32,
    /// Left stick Y, `-1.0..1.0`.
    pub ly: f32,
    /// Right stick X, `-1.0..1.0`.
    pub rx: f32,
    /// Right stick Y, `-1.0..1.0`.
    pub ry: f32,
    /// Left trigger, `0.0..1.0`.
    pub lt: f32,
    /// Right trigger, `0.0..1.0`.
    pub rt: f32,
    /// XInput `wButtons` bitmask, passed through untouched.
    pub buttons: u16,
}

/// One deterministic 240Hz frame — all integer, ready to cross a thread
/// boundary or land in a replay tape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct QuantizedPadFrame {
    /// Tick index at 240Hz (monotonic; equals its index in a recorded tape).
    pub tick: u64,
    /// Left stick X in Permyriad, `-10000..10000`, radial deadzone applied.
    pub lx: i16,
    /// Left stick Y in Permyriad, `-10000..10000`, radial deadzone applied.
    pub ly: i16,
    /// Right stick X in Permyriad, `-10000..10000`, radial deadzone applied.
    pub rx: i16,
    /// Right stick Y in Permyriad, `-10000..10000`, radial deadzone applied.
    pub ry: i16,
    /// Left trigger in Permyriad, `0..10000`.
    pub lt: u16,
    /// Right trigger in Permyriad, `0..10000`.
    pub rt: u16,
    /// XInput `wButtons` of the latest raw sample (booleans never interpolate).
    pub buttons: u16,
}

/// Bytes per tape frame: 4x`i16` sticks + 2x`u16` triggers + `u16` buttons.
/// The tick is NOT stored — a tape frame's tick IS its index, so a tape
/// cannot lie about time.
pub const PAD_FRAME_BYTES: usize = 14;

impl QuantizedPadFrame {
    /// Little-endian, fixed layout — the replay/anti-cheat wire format.
    pub fn to_bytes(&self) -> [u8; PAD_FRAME_BYTES] {
        let mut b = [0u8; PAD_FRAME_BYTES];
        b[0..2].copy_from_slice(&self.lx.to_le_bytes());
        b[2..4].copy_from_slice(&self.ly.to_le_bytes());
        b[4..6].copy_from_slice(&self.rx.to_le_bytes());
        b[6..8].copy_from_slice(&self.ry.to_le_bytes());
        b[8..10].copy_from_slice(&self.lt.to_le_bytes());
        b[10..12].copy_from_slice(&self.rt.to_le_bytes());
        b[12..14].copy_from_slice(&self.buttons.to_le_bytes());
        b
    }

    /// Inverse of [`Self::to_bytes`]; `tick` comes from the caller (tape index).
    pub fn from_bytes(tick: u64, b: &[u8; PAD_FRAME_BYTES]) -> Self {
        let i16le = |i: usize| i16::from_le_bytes([b[i], b[i + 1]]);
        let u16le = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
        Self {
            tick,
            lx: i16le(0),
            ly: i16le(2),
            rx: i16le(4),
            ry: i16le(6),
            lt: u16le(8),
            rt: u16le(10),
            buttons: u16le(12),
        }
    }
}

/// Serialize a recorded tape to its wire bytes (frame index = tick).
pub fn tape_to_bytes(tape: &[QuantizedPadFrame]) -> Vec<u8> {
    let mut out = Vec::with_capacity(tape.len() * PAD_FRAME_BYTES);
    for f in tape {
        out.extend_from_slice(&f.to_bytes());
    }
    out
}

/// Rebuild a tape from wire bytes. `None` if the length is not a whole
/// number of frames — a truncated tape is refused, never zero-padded.
pub fn tape_from_bytes(bytes: &[u8]) -> Option<Vec<QuantizedPadFrame>> {
    if !bytes.len().is_multiple_of(PAD_FRAME_BYTES) {
        return None;
    }
    Some(
        bytes
            .chunks_exact(PAD_FRAME_BYTES)
            .enumerate()
            .map(|(i, c)| {
                let mut b = [0u8; PAD_FRAME_BYTES];
                b.copy_from_slice(c);
                QuantizedPadFrame::from_bytes(i as u64, &b)
            })
            .collect(),
    )
}

/// Fixed-rate resampler: irregular XInput polls to uniform 240Hz frames
/// (lerp between bracketing samples, buttons from the latest, optional
/// recording tape).
pub struct PadQuantizer {
    /// Radial deadzone threshold (`0.0..1.0`) applied before quantization.
    pub deadzone: f32,
    tick_period_us: u64,
    next_tick_us: u64,
    current_tick: u64,
    prev: RawPadSample,
    initialized: bool,
    tape: Option<Vec<QuantizedPadFrame>>,
}

impl PadQuantizer {
    /// Construct a quantizer at the default deadzone, un-started tick clock.
    pub fn new() -> Self {
        Self {
            deadzone: crate::deadzone::DEFAULT_DEADZONE,
            tick_period_us: INPUT_TICK_PERIOD_US,
            next_tick_us: 0,
            current_tick: 0,
            prev: RawPadSample::default(),
            initialized: false,
            tape: None,
        }
    }

    /// Start recording frames to a replay tape (~1 min pre-sized).
    pub fn start_recording(&mut self) {
        self.tape = Some(Vec::with_capacity(240 * 60));
    }

    /// Stop recording and hand the tape over.
    pub fn stop_recording(&mut self) -> Option<Vec<QuantizedPadFrame>> {
        self.tape.take()
    }

    /// Feed one raw poll; emits zero or more uniform 240Hz frames into `out`.
    pub fn feed(&mut self, s: &RawPadSample, out: &mut Vec<QuantizedPadFrame>) {
        if !self.initialized {
            self.initialized = true;
            self.next_tick_us = s.timestamp_us + self.tick_period_us;
            self.prev = s.clone();
            return;
        }
        while self.next_tick_us <= s.timestamp_us {
            let t = if s.timestamp_us == self.prev.timestamp_us {
                1.0
            } else {
                (self.next_tick_us - self.prev.timestamp_us) as f32
                    / (s.timestamp_us - self.prev.timestamp_us) as f32
            };
            let frame = self.lerp_quantize(&self.prev.clone(), s, t);
            if let Some(ref mut tape) = self.tape {
                tape.push(frame);
            }
            out.push(frame);
            self.next_tick_us += self.tick_period_us;
            self.current_tick += 1;
        }
        self.prev = s.clone();
    }

    fn lerp_quantize(&self, a: &RawPadSample, b: &RawPadSample, t: f32) -> QuantizedPadFrame {
        let lerp = |v0: f32, v1: f32| v0 + (v1 - v0) * t;
        // Deadzone in normalized space, THEN Permyriad — so a replay and a
        // live run quantize identically.
        let quant_stick = |x: f32, y: f32| -> (i16, i16) {
            let dz = apply_radial_deadzone(x, y, self.deadzone);
            ((dz[0].clamp(-1.0, 1.0) * 10000.0) as i16, (dz[1].clamp(-1.0, 1.0) * 10000.0) as i16)
        };
        let (lx, ly) = quant_stick(lerp(a.lx, b.lx), lerp(a.ly, b.ly));
        let (rx, ry) = quant_stick(lerp(a.rx, b.rx), lerp(a.ry, b.ry));
        QuantizedPadFrame {
            tick: self.current_tick,
            lx,
            ly,
            rx,
            ry,
            lt: (lerp(a.lt, b.lt).clamp(0.0, 1.0) * 10000.0) as u16,
            rt: (lerp(a.rt, b.rt).clamp(0.0, 1.0) * 10000.0) as u16,
            buttons: b.buttons,
        }
    }

    /// Frame at a tick in a recorded tape (index IS the tick).
    pub fn replay_at(tape: &[QuantizedPadFrame], tick: u64) -> Option<&QuantizedPadFrame> {
        tape.get(tick as usize)
    }
}

impl Default for PadQuantizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(us: u64, lx: f32, buttons: u16) -> RawPadSample {
        RawPadSample { timestamp_us: us, lx, buttons, ..Default::default() }
    }

    #[test]
    fn uniform_ticks_from_irregular_polls() {
        let mut q = PadQuantizer::new();
        let mut out = Vec::new();
        q.feed(&sample(0, 0.0, 0), &mut out);
        q.feed(&sample(INPUT_TICK_PERIOD_US * 3, 0.9, 0), &mut out);
        assert_eq!(out.len(), 3);
        assert_eq!((out[0].tick, out[1].tick, out[2].tick), (0, 1, 2));
    }

    #[test]
    fn sticks_stay_in_permyriad_and_deadzone_kills_drift() {
        let mut q = PadQuantizer::new();
        let mut out = Vec::new();
        q.feed(&sample(0, 0.02, 0), &mut out); // under deadzone
        q.feed(&sample(INPUT_TICK_PERIOD_US, 0.02, 0), &mut out);
        assert_eq!(out[0].lx, 0, "drift under the deadzone must quantize to zero");
        let mut q2 = PadQuantizer::new();
        let mut out2 = Vec::new();
        q2.feed(&sample(0, 1.0, 0), &mut out2);
        q2.feed(&sample(INPUT_TICK_PERIOD_US, 1.0, 0), &mut out2);
        assert!(out2[0].lx > 9000 && out2[0].lx <= 10000);
    }

    #[test]
    fn tape_roundtrips_byte_exact() {
        let mut q = PadQuantizer::new();
        let mut out = Vec::new();
        q.start_recording();
        q.feed(&sample(0, 0.5, 0b1010), &mut out);
        q.feed(&sample(INPUT_TICK_PERIOD_US * 4, -0.7, 0b0101), &mut out);
        let tape = q.stop_recording().unwrap();
        let bytes = tape_to_bytes(&tape);
        let back = tape_from_bytes(&bytes).unwrap();
        assert_eq!(tape, back);
        assert!(tape_from_bytes(&bytes[..bytes.len() - 1]).is_none(), "truncated tape refused");
    }

    #[test]
    fn same_feed_same_bytes_twice() {
        let run = || {
            let mut q = PadQuantizer::new();
            let mut out = Vec::new();
            q.start_recording();
            for i in 0..50u64 {
                q.feed(&sample(i * 3000, (i as f32 * 0.07).sin(), (i % 4) as u16), &mut out);
            }
            tape_to_bytes(&q.stop_recording().unwrap())
        };
        assert_eq!(run(), run(), "quantizer must be bit-deterministic over identical polls");
    }

    #[test]
    fn replay_at_indexes_by_tick() {
        let mut q = PadQuantizer::new();
        let mut out = Vec::new();
        q.start_recording();
        q.feed(&sample(0, 0.0, 0), &mut out);
        q.feed(&sample(INPUT_TICK_PERIOD_US * 2, 0.5, 7), &mut out);
        let tape = q.stop_recording().unwrap();
        assert_eq!(PadQuantizer::replay_at(&tape, 1).unwrap().buttons, 7);
        assert!(PadQuantizer::replay_at(&tape, 99).is_none());
    }
}
