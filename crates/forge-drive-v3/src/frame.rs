//! `InputFrame64` — the 64-byte input-drive word, T6 of the forge-vision
//! drain (plan-2-welds.md T6 estimate, overridden by the D2 ruling recorded
//! in brief-queue/T6-inputframe64-BRIEF.md: 64 bytes, not 8, deliberately
//! breaking the quarry's 32-byte `AgentInputFrame` wire [observed
//! spooler.rs:199] per ARCH000).
//!
//! Machine-first (L08): the brief's six f32 axes are NOT stored as float —
//! every axis is a permyriad/offset-encoded `u16`, the `SLOPE_NEUTRAL`
//! precedent [observed forge-soundwave-v3/src/ecology.rs:20]. Sticks are
//! `0..=PMY_MAX` with `STICK_NEUTRAL` (5_000) at the domain's `0.0`, mapping
//! the `[-1.0, 1.0]` interval; triggers are `0..=PMY_MAX` with `0` at the
//! domain's `0.0`, mapping the `[0.0, 1.0]` interval. Any f32 form is a
//! consumer's derived bridge, never stored a second time.
//!
//! ONE HOME (L05): this file is the only definition of `InputFrame64`.

/// Full scale for the permyriad axis channels: `0..=10_000`. Matches
/// `ColourTrit8::PMY_MAX` [observed forge-colour-v3/src/trit.rs:23] and
/// `EcologyPCM8::PMY_MAX` [observed forge-soundwave-v3/src/ecology.rs:13].
pub const PMY_MAX: u16 = 10_000;

/// A stick axis's neutral point — permyriad `5_000` encodes `0.0` (centred),
/// `0` encodes `-1.0`, `PMY_MAX` encodes `+1.0`. The `SLOPE_NEUTRAL`
/// precedent [observed forge-soundwave-v3/src/ecology.rs:20], applied to the
/// four stick axes. `[ASSUMED]` this crate's amendment to the brief (D2
/// ruling override note, L08 machine-first) — no quarried source pins this
/// constant, it is chosen for symmetry with the permyriad domain.
pub const STICK_NEUTRAL: u16 = 5_000;

/// Modifier bitmask's only valid bits: Alt=1, Ctrl=2, Meta=4, Shift=8
/// [inferred cdp.rs:32, per the brief's layout table row 2].
pub const MODIFIER_MASK_MAX: u8 = 0x0F;

/// One input-drive frame, 64 bytes, exact. Field order is offset order —
/// every byte is a field or a named reserved run, no padding hole. The
/// offsets below are locked by rustc, not prose.
///
/// Layout (brief-queue/T6-inputframe64-BRIEF.md "InputFrame64 Layout",
/// axis columns amended from f32 to permyriad u16 per this crate's D2
/// amendment note):
///
/// | Offset | Field | Type | Size |
/// |---|---|---|---|
/// | 0 | `buttons` | `u16` | 2B |
/// | 2 | `modifier_mask` | `u8` | 1B |
/// | 3 | `_pad0` | `u8` | 1B |
/// | 4 | `left_stick_x_pmy` | `u16` | 2B |
/// | 6 | `left_stick_y_pmy` | `u16` | 2B |
/// | 8 | `right_stick_x_pmy` | `u16` | 2B |
/// | 10 | `right_stick_y_pmy` | `u16` | 2B |
/// | 12 | `left_trigger_pmy` | `u16` | 2B |
/// | 14 | `right_trigger_pmy` | `u16` | 2B |
/// | 16 | `timestamp_ms` | `u32` | 4B |
/// | 20 | `frame_index` | `u32` | 4B |
/// | 24 | `macro_id` | `u32` | 4B |
/// | 28 | `reserved` | `[u8; 36]` | 36B |
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputFrame64 {
    /// Bitmask of 16 game buttons [observed spooler.rs:35]; sentinel edges
    /// tested `0x0000`/`0xFFFF`. Opaque — every bit pattern is valid.
    pub buttons: u16,
    /// Modifier bitmask: Alt=1, Ctrl=2, Meta=4, Shift=8 [inferred cdp.rs:32].
    /// Valid range `0..=MODIFIER_MASK_MAX`; higher bits are corruption.
    pub modifier_mask: u8,
    /// Alignment filler [ASSUMED, brief layout row 3]. Must be zero — a
    /// nonzero pad byte is corruption, never forward-compatibility.
    pub _pad0: u8,
    /// Left stick X, permyriad offset-encoded around `STICK_NEUTRAL` for the
    /// `[-1.0, 1.0]` domain [observed spooler.rs:37, encoding amended per
    /// this crate's D2 note].
    pub left_stick_x_pmy: u16,
    /// Left stick Y, same encoding as `left_stick_x_pmy` [observed
    /// spooler.rs:38].
    pub left_stick_y_pmy: u16,
    /// Right stick X, same encoding as `left_stick_x_pmy` [observed
    /// spooler.rs:39].
    pub right_stick_x_pmy: u16,
    /// Right stick Y, same encoding as `left_stick_x_pmy` [observed
    /// spooler.rs:40].
    pub right_stick_y_pmy: u16,
    /// Left trigger, permyriad `0..=PMY_MAX` mapping `[0.0, 1.0]`, `0` at
    /// rest [observed spooler.rs:41].
    pub left_trigger_pmy: u16,
    /// Right trigger, same encoding as `left_trigger_pmy` [observed
    /// spooler.rs:42].
    pub right_trigger_pmy: u16,
    /// Tick/wall-clock time in milliseconds [`[ASSUMED]` from
    /// 03-live-loop.md:42].
    pub timestamp_ms: u32,
    /// Macro frame ordinal (0-based), for correlation with ActuationReport
    /// `[ASSUMED]`.
    pub frame_index: u32,
    /// Current macro_id from SpoolerHeader [observed spooler.rs:25].
    pub macro_id: u32,
    /// Reserved. Must be all zero — a live reserved byte is corruption,
    /// never forward-compatibility [ASSUMED, brief layout row 12 headroom
    /// note].
    pub reserved: [u8; 36],
}

impl InputFrame64 {
    /// The origin: no buttons, no modifiers, every stick centred, every
    /// trigger at rest, zero timing/macro fields, no reserved bits. Named in
    /// the brief's "Bijection Gate" section as the compile-time E0080 gate
    /// point (adapted here for the D2-overridden 64-byte home).
    pub const ORIGIN: Self = Self {
        buttons: 0,
        modifier_mask: 0,
        _pad0: 0,
        left_stick_x_pmy: STICK_NEUTRAL,
        left_stick_y_pmy: STICK_NEUTRAL,
        right_stick_x_pmy: STICK_NEUTRAL,
        right_stick_y_pmy: STICK_NEUTRAL,
        left_trigger_pmy: 0,
        right_trigger_pmy: 0,
        timestamp_ms: 0,
        frame_index: 0,
        macro_id: 0,
        reserved: [0; 36],
    };

    /// True when every channel is inside its domain: `modifier_mask` carries
    /// no bit outside `MODIFIER_MASK_MAX`, every axis permyriad is
    /// `<= PMY_MAX`, and both `_pad0` and `reserved` are zero. `buttons`,
    /// `timestamp_ms`, `frame_index`, and `macro_id` are opaque full-range
    /// words — no bit pattern is refused.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        self.modifier_mask <= MODIFIER_MASK_MAX
            && self._pad0 == 0
            && self.left_stick_x_pmy <= PMY_MAX
            && self.left_stick_y_pmy <= PMY_MAX
            && self.right_stick_x_pmy <= PMY_MAX
            && self.right_stick_y_pmy <= PMY_MAX
            && self.left_trigger_pmy <= PMY_MAX
            && self.right_trigger_pmy <= PMY_MAX
            && !any_reserved_byte_is_set(&self.reserved)
    }

    /// Pack into a 64-byte little-endian array. Byte layout is the struct
    /// layout, field for field, offset order.
    #[inline(always)]
    pub const fn encode(self) -> [u8; 64] {
        let mut out = [0u8; 64];
        write_u16(&mut out, 0, self.buttons);
        out[2] = self.modifier_mask;
        out[3] = self._pad0;
        write_u16(&mut out, 4, self.left_stick_x_pmy);
        write_u16(&mut out, 6, self.left_stick_y_pmy);
        write_u16(&mut out, 8, self.right_stick_x_pmy);
        write_u16(&mut out, 10, self.right_stick_y_pmy);
        write_u16(&mut out, 12, self.left_trigger_pmy);
        write_u16(&mut out, 14, self.right_trigger_pmy);
        write_u32(&mut out, 16, self.timestamp_ms);
        write_u32(&mut out, 20, self.frame_index);
        write_u32(&mut out, 24, self.macro_id);
        let mut i = 0;
        while i < 36 {
            out[28 + i] = self.reserved[i];
            i += 1;
        }
        out
    }

    /// Unpack a 64-byte array. `None` for anything outside the valid
    /// domain — an out-of-range modifier bit, an out-of-range axis
    /// permyriad, a live pad byte, or a live reserved byte is corruption
    /// refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(bytes: [u8; 64]) -> Option<Self> {
        let mut reserved = [0u8; 36];
        let mut i = 0;
        while i < 36 {
            reserved[i] = bytes[28 + i];
            i += 1;
        }
        let c = Self {
            buttons: read_u16(&bytes, 0),
            modifier_mask: bytes[2],
            _pad0: bytes[3],
            left_stick_x_pmy: read_u16(&bytes, 4),
            left_stick_y_pmy: read_u16(&bytes, 6),
            right_stick_x_pmy: read_u16(&bytes, 8),
            right_stick_y_pmy: read_u16(&bytes, 10),
            left_trigger_pmy: read_u16(&bytes, 12),
            right_trigger_pmy: read_u16(&bytes, 14),
            timestamp_ms: read_u32(&bytes, 16),
            frame_index: read_u32(&bytes, 20),
            macro_id: read_u32(&bytes, 24),
            reserved,
        };
        if c.is_valid() { Some(c) } else { None }
    }
}

#[inline(always)]
const fn any_reserved_byte_is_set(reserved: &[u8; 36]) -> bool {
    let mut i = 0;
    while i < 36 {
        if reserved[i] != 0 {
            return true;
        }
        i += 1;
    }
    false
}

#[inline(always)]
const fn write_u16(out: &mut [u8; 64], at: usize, v: u16) {
    let b = v.to_le_bytes();
    out[at] = b[0];
    out[at + 1] = b[1];
}

#[inline(always)]
const fn write_u32(out: &mut [u8; 64], at: usize, v: u32) {
    let b = v.to_le_bytes();
    out[at] = b[0];
    out[at + 1] = b[1];
    out[at + 2] = b[2];
    out[at + 3] = b[3];
}

#[inline(always)]
const fn read_u16(bytes: &[u8; 64], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

#[inline(always)]
const fn read_u32(bytes: &[u8; 64], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<InputFrame64>() == 64);
const _: () = assert!(core::mem::align_of::<InputFrame64>() == 8);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 64 while
// silently reinterpreting every stored frame.
const _: () = assert!(core::mem::offset_of!(InputFrame64, buttons) == 0);
const _: () = assert!(core::mem::offset_of!(InputFrame64, modifier_mask) == 2);
const _: () = assert!(core::mem::offset_of!(InputFrame64, _pad0) == 3);
const _: () = assert!(core::mem::offset_of!(InputFrame64, left_stick_x_pmy) == 4);
const _: () = assert!(core::mem::offset_of!(InputFrame64, left_stick_y_pmy) == 6);
const _: () = assert!(core::mem::offset_of!(InputFrame64, right_stick_x_pmy) == 8);
const _: () = assert!(core::mem::offset_of!(InputFrame64, right_stick_y_pmy) == 10);
const _: () = assert!(core::mem::offset_of!(InputFrame64, left_trigger_pmy) == 12);
const _: () = assert!(core::mem::offset_of!(InputFrame64, right_trigger_pmy) == 14);
const _: () = assert!(core::mem::offset_of!(InputFrame64, timestamp_ms) == 16);
const _: () = assert!(core::mem::offset_of!(InputFrame64, frame_index) == 20);
const _: () = assert!(core::mem::offset_of!(InputFrame64, macro_id) == 24);
const _: () = assert!(core::mem::offset_of!(InputFrame64, reserved) == 28);

// Every one of the 64 bytes is a field — no padding hole.
const _: () = assert!(2 + 1 + 1 + 2 * 6 + 4 * 3 + 36 == core::mem::size_of::<InputFrame64>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match InputFrame64::decode(InputFrame64::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.buttons == 0 && w.modifier_mask == 0 && w._pad0 == 0);
            assert!(w.left_stick_x_pmy == STICK_NEUTRAL && w.left_stick_y_pmy == STICK_NEUTRAL);
            assert!(w.right_stick_x_pmy == STICK_NEUTRAL && w.right_stick_y_pmy == STICK_NEUTRAL);
            assert!(w.left_trigger_pmy == 0 && w.right_trigger_pmy == 0);
            assert!(w.timestamp_ms == 0 && w.frame_index == 0 && w.macro_id == 0);
            assert!(!any_reserved_byte_is_set(&w.reserved));
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    fn base() -> InputFrame64 {
        InputFrame64 {
            buttons: 0x1234,
            modifier_mask: 0x0A,
            _pad0: 0,
            left_stick_x_pmy: 2_500,
            left_stick_y_pmy: 7_500,
            right_stick_x_pmy: 1,
            right_stick_y_pmy: 9_999,
            left_trigger_pmy: 5_000,
            right_trigger_pmy: 2_500,
            timestamp_ms: 123_456,
            frame_index: 7,
            macro_id: 42,
            reserved: [0; 36],
        }
    }

    /// L07 over the interior: every axis, buttons, modifier, and the 32-bit
    /// trio survive their own wire exactly.
    #[test]
    fn bijection_holds_over_the_interior() {
        let f = base();
        assert_eq!(InputFrame64::decode(f.encode()), Some(f));
    }

    /// L07 over the sentinels named in the brief's "Bijection Gate" section:
    /// buttons 0x0000/0xFFFF, stick edges -1/0/+1 (0/STICK_NEUTRAL/PMY_MAX),
    /// trigger edges 0/0.5/1.0 (0/5_000/PMY_MAX), modifier 0x00/0x0F,
    /// timestamp 0/u32::MAX.
    #[test]
    fn bijection_holds_over_the_sentinels() {
        for buttons in [0u16, 0xFFFF] {
            for stick in [0u16, STICK_NEUTRAL, PMY_MAX] {
                for trigger in [0u16, 5_000, PMY_MAX] {
                    for modifier_mask in [0x00u8, 0x0F] {
                        for ts in [0u32, u32::MAX] {
                            let f = InputFrame64 {
                                buttons,
                                modifier_mask,
                                _pad0: 0,
                                left_stick_x_pmy: stick,
                                left_stick_y_pmy: stick,
                                right_stick_x_pmy: stick,
                                right_stick_y_pmy: stick,
                                left_trigger_pmy: trigger,
                                right_trigger_pmy: trigger,
                                timestamp_ms: ts,
                                frame_index: ts,
                                macro_id: ts,
                                reserved: [0; 36],
                            };
                            assert_eq!(
                                InputFrame64::decode(f.encode()),
                                Some(f),
                                "buttons={buttons:#06x} stick={stick} trigger={trigger} modifier={modifier_mask:#04x} ts={ts}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The origin: every field at rest, round-trips.
    #[test]
    fn the_origin_survives_its_wire() {
        assert_eq!(InputFrame64::decode(InputFrame64::ORIGIN.encode()), Some(InputFrame64::ORIGIN));
    }

    /// frame_index and macro_id follow the u32 edge tests named in the
    /// brief's "Bijection Gate" section 6: 0, 1, 0xFFFFFFFF.
    #[test]
    fn frame_index_and_macro_id_hold_over_u32_edges() {
        for frame_index in [0u32, 1, u32::MAX] {
            for macro_id in [0u32, 1, u32::MAX] {
                let f = InputFrame64 { frame_index, macro_id, ..base() };
                assert_eq!(InputFrame64::decode(f.encode()), Some(f), "frame_index={frame_index} macro_id={macro_id}");
            }
        }
    }

    /// The boundary refuses corruption: each invalid frame decodes to None.
    #[test]
    fn out_of_domain_frames_are_refused() {
        let good = base();
        assert!(InputFrame64::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad = [
            InputFrame64 { modifier_mask: MODIFIER_MASK_MAX + 1, ..good },
            InputFrame64 { _pad0: 1, ..good },
            InputFrame64 { left_stick_x_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { left_stick_y_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { right_stick_x_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { right_stick_y_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { left_trigger_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { right_trigger_pmy: PMY_MAX + 1, ..good },
            InputFrame64 { reserved: { let mut r = [0u8; 36]; r[0] = 1; r }, ..good },
            InputFrame64 { reserved: { let mut r = [0u8; 36]; r[35] = 1; r }, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(InputFrame64::decode(b.encode()), None, "bad row {i} was accepted");
        }
    }
}
