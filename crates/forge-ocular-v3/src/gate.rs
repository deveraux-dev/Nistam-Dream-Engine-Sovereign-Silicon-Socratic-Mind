//! `RenderGate64` — the 64-byte render-gate verdict word, T3 of the
//! forge-vision drain (plan-2-welds.md T3, quarried from render_gate.rs's
//! 5-gate roll-up — the discipline is drained, the encoding is new).
//!
//! Machine-first (L08): every field is an exact integer or the embedded
//! `ColourTrit8` word (forge-colour-v3, T1); no perceptual transform lives
//! on this path.
//!
//! ONE HOME (L05): this file is the only definition of `RenderGate64` and
//! `GateVerdict`.

use forge_colour_v3::ColourTrit8;
use forge_core_v3::atom::{CellOrdinal, Pexil, TritCell5D, ValidityMask};

/// The 5-gate roll-up verdict (render_gate.rs:107): pass iff all 5 gates
/// (hotpath, pressure, canvas, fps, lens) pass; fail iff any gate fails;
/// skipped iff measurement was skipped.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GateVerdict {
    /// All 5 gates passed.
    Pass = 0,
    /// At least one gate failed.
    Fail = 1,
    /// Measurement was skipped.
    Skipped = 2,
}

impl GateVerdict {
    /// The raw byte this state occupies in `RenderGate64::verdict`.
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// `None` for anything outside the 3 named states — a fourth verdict
    /// byte is corruption, never a silent fourth state.
    #[inline(always)]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Pass),
            1 => Some(Self::Fail),
            2 => Some(Self::Skipped),
            _ => None,
        }
    }
}

/// One render-gate verdict word, 64 bytes, exact: hotpath/pressure timing
/// budgets, the embedded colour-truthfulness delta, observed fps, the
/// verdict byte, and a regression-baseline perceptual hash. Field order is
/// offset order — every byte is a field, no padding hole. The offsets below
/// are locked by rustc, not prose.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderGate64 {
    /// Hotpath measured, microseconds. Budget 2000 µs (render_gate.rs:107).
    pub hotpath_us: u32,
    /// Pressure measured, microseconds. Budget 2500 µs (render_gate.rs:107).
    pub pressure_us: u32,
    /// Colour truthfulness delta — the embedded T1 word.
    pub colour_delta: ColourTrit8,
    /// Perceptual hash from `confirm_pixels()` (render_gate.rs:77-86).
    /// Regression baseline only — no sentinel domain.
    pub phash: u64,
    /// Frames per second observed. Floor 60 @ 1440p (render_gate.rs:107).
    pub fps: u16,
    /// Raw `GateVerdict` byte.
    pub verdict: u8,
    /// Padding field — zero at origin, live once a second byte is needed.
    pub _reserved_1: u8,
    /// Future lane fields, zero-initialized.
    pub _reserved_2: [u8; 36],
}

impl RenderGate64 {
    /// The origin: zero timings, achromatic white colour delta, fps pinned
    /// to the 1440p floor, verdict Pass, zero phash, reserved zeroed.
    pub const ORIGIN: Self = Self {
        hotpath_us: 0,
        pressure_us: 0,
        colour_delta: ColourTrit8::WHITE,
        phash: 0,
        fps: 60,
        verdict: GateVerdict::Pass as u8,
        _reserved_1: 0,
        _reserved_2: [0; 36],
    };

    /// True when the embedded colour delta is itself valid, the verdict
    /// byte names one of the 3 states, and both reserved lanes are zero (or _reserved_2
    /// carries a valid, non-corrupt RenderGate5D payload).
    /// `hotpath_us`/`pressure_us`/`fps`/`phash` carry no domain restriction
    /// beyond their integer width.
    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        let is_5d_valid = {
            let active_cell = self._reserved_2[0];
            let active_validity = self._reserved_2[1];
            let has_5d_magic = self._reserved_2[26] == 0x5D;
            let pexil_validity = self._reserved_2[28];
            has_5d_magic
                && active_cell < 243
                && active_validity <= 242
                && pexil_validity <= 242
                && self._reserved_2[35] == 0
        };

        self.colour_delta.is_valid()
            && self.verdict <= GateVerdict::Skipped as u8
            && self._reserved_1 == 0
            && (reserved_is_zero(&self._reserved_2) || is_5d_valid)
    }

    /// Pack into 64 little-endian bytes. Byte layout is the struct layout:
    /// hotpath, pressure, colour_delta, phash, fps, verdict, _reserved_1,
    /// _reserved_2.
    pub const fn encode(self) -> [u8; 64] {
        let mut buf = [0u8; 64];
        let hp = self.hotpath_us.to_le_bytes();
        let pr = self.pressure_us.to_le_bytes();
        let cd = self.colour_delta.encode().to_le_bytes();
        let ph = self.phash.to_le_bytes();
        let fp = self.fps.to_le_bytes();

        let mut i = 0;
        while i < 4 {
            buf[i] = hp[i];
            i += 1;
        }
        let mut i = 0;
        while i < 4 {
            buf[4 + i] = pr[i];
            i += 1;
        }
        let mut i = 0;
        while i < 8 {
            buf[8 + i] = cd[i];
            i += 1;
        }
        let mut i = 0;
        while i < 8 {
            buf[16 + i] = ph[i];
            i += 1;
        }
        buf[24] = fp[0];
        buf[25] = fp[1];
        buf[26] = self.verdict;
        buf[27] = self._reserved_1;
        let mut i = 0;
        while i < 36 {
            buf[28 + i] = self._reserved_2[i];
            i += 1;
        }
        buf
    }

    /// Unpack 64 bytes. `None` for anything outside the valid domain — an
    /// invalid embedded colour word, an unnamed verdict byte, or a live
    /// reserved lane is corruption refused at the boundary, not clamped
    /// past it.
    pub const fn decode(bytes: [u8; 64]) -> Option<Self> {
        let hotpath_us = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let pressure_us = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let cd_word = u64::from_le_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        let colour_delta = match ColourTrit8::decode(cd_word) {
            Some(c) => c,
            None => return None,
        };
        let phash = u64::from_le_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        let fps = u16::from_le_bytes([bytes[24], bytes[25]]);
        let verdict = bytes[26];
        let _reserved_1 = bytes[27];
        let mut _reserved_2 = [0u8; 36];
        let mut i = 0;
        while i < 36 {
            _reserved_2[i] = bytes[28 + i];
            i += 1;
        }

        let c = Self { hotpath_us, pressure_us, colour_delta, phash, fps, verdict, _reserved_1, _reserved_2 };
        if c.is_valid() { Some(c) } else { None }
    }

    /// Get the 5D spatial and SPCC awareness packet from the reserved lane.
    #[inline(always)]
    pub const fn get_5d(&self) -> RenderGate5D {
        RenderGate5D::decode(self._reserved_2)
    }

    /// Set the 5D spatial and SPCC awareness packet in the reserved lane.
    #[inline(always)]
    pub fn set_5d(&mut self, packet: &RenderGate5D) {
        self._reserved_2 = packet.encode();
    }
}

/// 5D Spatial & SPCC (Soliton-Phase Context Collapse) awareness packet.
/// Fits exactly within the 36-byte reserved field of `RenderGate64`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RenderGate5D {
    /// Target lattice coordinate (5-lane balanced-ternary address).
    pub active_cell: TritCell5D,
    /// Kleene three-valued validity mask of the cell.
    pub active_validity: ValidityMask,
    /// Intra-cell identity index.
    pub active_ordinal: CellOrdinal,
    /// Landauer amnesia margin in permyriad (tells how close to amnesia we are).
    pub landauer_margin_pmy: i32,
    /// Total committed drive mass evicted (erased) in permyriad.
    pub erased_drive_pmy: i32,
    /// Total SPCC drive mass in permyriad.
    pub mass_in_pmy: i32,
    /// Total SPCC settled mass of surviving rows.
    pub mass_out_pmy: i32,
    /// Net interference gain in permyriad.
    pub interference_gain_pmy: i32,
    /// Whether the render hitbox intersected the active 5D Ghostmoon interval.
    pub ghostmoon_intersects: bool,
    /// SPCC desync synchronisation status byte.
    pub spcc_sync_status: u8,
    /// PaTeX perceptual state vector: the 5 trit lanes of `pexil.lattice` are
    /// A1 Ground, A2 Light, A3 Depth, A4 Drift, A5 Witness; `pexil.payload[0..2]`
    /// is glaze intensity in permyriad, LE. Bytes 27..=34 of the reserved lane.
    pub pexil: Pexil,
}

impl RenderGate5D {
    /// Pack into 36 bytes.
    pub const fn encode(&self) -> [u8; 36] {
        let mut buf = [0u8; 36];
        buf[0] = self.active_cell.0;
        buf[1] = self.active_validity.0;
        let ord = self.active_ordinal.0.to_le_bytes();
        buf[2] = ord[0];
        buf[3] = ord[1];

        let lm = self.landauer_margin_pmy.to_le_bytes();
        let ed = self.erased_drive_pmy.to_le_bytes();
        let mi = self.mass_in_pmy.to_le_bytes();
        let mo = self.mass_out_pmy.to_le_bytes();
        let ig = self.interference_gain_pmy.to_le_bytes();

        let mut i = 0;
        while i < 4 {
            buf[4 + i] = lm[i];
            buf[8 + i] = ed[i];
            buf[12 + i] = mi[i];
            buf[16 + i] = mo[i];
            buf[20 + i] = ig[i];
            i += 1;
        }
        buf[24] = self.ghostmoon_intersects as u8;
        buf[25] = self.spcc_sync_status;
        buf[26] = 0x5D;
        buf[27] = self.pexil.lattice.0;
        buf[28] = self.pexil.validity.0;
        let po = self.pexil.ordinal.0.to_le_bytes();
        buf[29] = po[0];
        buf[30] = po[1];
        let mut i = 0;
        while i < 4 {
            buf[31 + i] = self.pexil.payload[i];
            i += 1;
        }
        buf
    }

    /// Unpack from 36 bytes.
    pub const fn decode(bytes: [u8; 36]) -> Self {
        let active_cell = TritCell5D(bytes[0]);
        let active_validity = ValidityMask(bytes[1]);
        let active_ordinal = CellOrdinal(u16::from_le_bytes([bytes[2], bytes[3]]));

        let landauer_margin_pmy = i32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let erased_drive_pmy = i32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let mass_in_pmy = i32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let mass_out_pmy = i32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let interference_gain_pmy = i32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        let ghostmoon_intersects = bytes[24] != 0;
        let spcc_sync_status = bytes[25];
        let pexil = Pexil {
            lattice: TritCell5D(bytes[27]),
            validity: ValidityMask(bytes[28]),
            ordinal: CellOrdinal(u16::from_le_bytes([bytes[29], bytes[30]])),
            payload: [bytes[31], bytes[32], bytes[33], bytes[34]],
        };

        Self {
            active_cell,
            active_validity,
            active_ordinal,
            landauer_margin_pmy,
            erased_drive_pmy,
            mass_in_pmy,
            mass_out_pmy,
            interference_gain_pmy,
            ghostmoon_intersects,
            spcc_sync_status,
            pexil,
        }
    }
}

/// Every byte of the reserved lane must be zero at origin — a live reserved
/// byte is corruption today, never forward-compatibility.
const fn reserved_is_zero(bytes: &[u8; 36]) -> bool {
    let mut i = 0;
    while i < 36 {
        if bytes[i] != 0 {
            return false;
        }
        i += 1;
    }
    true
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<RenderGate64>() == 64);
const _: () = assert!(core::mem::align_of::<RenderGate64>() == 8);
const _: () = assert!(core::mem::size_of::<GateVerdict>() == 1);

// OFFSET LOCKS. Size alone is weak: reordering two fields keeps size 64
// while silently reinterpreting every stored verdict.
const _: () = assert!(core::mem::offset_of!(RenderGate64, hotpath_us) == 0);
const _: () = assert!(core::mem::offset_of!(RenderGate64, pressure_us) == 4);
const _: () = assert!(core::mem::offset_of!(RenderGate64, colour_delta) == 8);
const _: () = assert!(core::mem::offset_of!(RenderGate64, phash) == 16);
const _: () = assert!(core::mem::offset_of!(RenderGate64, fps) == 24);
const _: () = assert!(core::mem::offset_of!(RenderGate64, verdict) == 26);
const _: () = assert!(core::mem::offset_of!(RenderGate64, _reserved_1) == 27);
const _: () = assert!(core::mem::offset_of!(RenderGate64, _reserved_2) == 28);

// Every one of the 64 bytes is a field — no padding hole.
const _: () = assert!(4 + 4 + 8 + 8 + 2 + 1 + 1 + 36 == core::mem::size_of::<RenderGate64>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match RenderGate64::decode(RenderGate64::ORIGIN.encode()) {
        Some(w) => {
            assert!(w.hotpath_us == 0 && w.pressure_us == 0);
            assert!(w.phash == 0 && w.fps == 60);
            assert!(w.verdict == GateVerdict::Pass as u8);
            assert!(w._reserved_1 == 0);
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(verdict: GateVerdict, hotpath: u32, pressure: u32, fps: u16, phash: u64) -> RenderGate64 {
        RenderGate64 {
            hotpath_us: hotpath,
            pressure_us: pressure,
            colour_delta: ColourTrit8 { hue_idx: 3, alpha_flag: 1, value_pmy: 5_000, chroma_pmy: 5_000, tags: [0; 2] },
            phash,
            fps,
            verdict: verdict.as_u8(),
            _reserved_1: 0,
            _reserved_2: [0; 36],
        }
    }

    /// L07 over the interior: representative timings/fps/phash round-trip
    /// exactly for each of the 3 named verdict states.
    #[test]
    fn bijection_holds_over_the_interior_for_every_verdict_state() {
        for verdict in [GateVerdict::Pass, GateVerdict::Fail, GateVerdict::Skipped] {
            let g = sample(verdict, 1_500, 2_000, 90, 0xDEAD_BEEF_1234_5678);
            assert_eq!(RenderGate64::decode(g.encode()), Some(g), "verdict={verdict:?}");
        }
    }

    /// L07 over the sentinels: u32::MAX hotpath/pressure, 0 and u16::MAX
    /// fps, all 3 verdict states.
    #[test]
    fn bijection_holds_over_the_sentinels() {
        for verdict in [GateVerdict::Pass, GateVerdict::Fail, GateVerdict::Skipped] {
            for fps in [0u16, u16::MAX] {
                let g = sample(verdict, u32::MAX, u32::MAX, fps, u64::MAX);
                assert_eq!(RenderGate64::decode(g.encode()), Some(g), "verdict={verdict:?} fps={fps}");
            }
        }
    }

    /// The origin: `ORIGIN` round-trips and is valid.
    #[test]
    fn the_origin_survives_its_wire() {
        assert!(RenderGate64::ORIGIN.is_valid());
        assert_eq!(RenderGate64::decode(RenderGate64::ORIGIN.encode()), Some(RenderGate64::ORIGIN));
    }

    /// `GateVerdict::from_u8` is the inverse of `as_u8` over the 3 named
    /// states and refuses everything else.
    #[test]
    fn verdict_byte_bijection_holds_and_refuses_the_unnamed() {
        for v in [GateVerdict::Pass, GateVerdict::Fail, GateVerdict::Skipped] {
            assert_eq!(GateVerdict::from_u8(v.as_u8()), Some(v));
        }
        for b in 3u8..=255 {
            assert_eq!(GateVerdict::from_u8(b), None, "unnamed verdict byte {b} was accepted");
        }
    }

    /// The boundary refuses corruption: an invalid embedded colour word, an
    /// unnamed verdict byte, and a live reserved lane each decode to None.
    #[test]
    fn out_of_domain_words_are_refused() {
        let good = sample(GateVerdict::Pass, 100, 200, 60, 42);
        assert!(RenderGate64::decode(good.encode()).is_some(), "the baseline itself is invalid");

        let bad_verdict = RenderGate64 { verdict: 3, ..good };
        assert_eq!(RenderGate64::decode(bad_verdict.encode()), None, "unnamed verdict byte was accepted");

        let bad_colour = RenderGate64 {
            colour_delta: ColourTrit8 { alpha_flag: 2, ..good.colour_delta },
            ..good
        };
        assert_eq!(RenderGate64::decode(bad_colour.encode()), None, "invalid embedded colour word was accepted");

        let bad_reserved_1 = RenderGate64 { _reserved_1: 1, ..good };
        assert_eq!(RenderGate64::decode(bad_reserved_1.encode()), None, "live _reserved_1 was accepted");

        let mut reserved_2 = [0u8; 36];
        reserved_2[10] = 7;
        let bad_reserved_2 = RenderGate64 { _reserved_2: reserved_2, ..good };
        assert_eq!(RenderGate64::decode(bad_reserved_2.encode()), None, "live _reserved_2 byte was accepted");
    }

    /// Verify RenderGate5D round-trips exactly, and that the upgraded RenderGate64
    /// validates and decodes a live 5D/SPCC-aware payload perfectly, while still
    /// rejecting random corruption.
    #[test]
    fn render_gate_5d_integration_bijection() {
        let good = sample(GateVerdict::Pass, 100, 200, 60, 42);

        let packet = RenderGate5D {
            active_cell: TritCell5D(121),
            active_validity: ValidityMask(242),
            active_ordinal: CellOrdinal(42),
            landauer_margin_pmy: 8_000,
            erased_drive_pmy: 1_000,
            mass_in_pmy: 10_000,
            mass_out_pmy: 9_000,
            interference_gain_pmy: -1_000,
            ghostmoon_intersects: true,
            spcc_sync_status: 3,
            pexil: Pexil {
                lattice: TritCell5D::from_trits([1, -1, 0, 1, -1]),
                validity: ValidityMask::ALL_KNOWN,
                ordinal: CellOrdinal(7),
                payload: 8_500u32.to_le_bytes(),
            },
        };

        let mut gate_5d = good;
        gate_5d.set_5d(&packet);

        assert!(gate_5d.is_valid(), "upgraded 5D gate failed is_valid check");

        let encoded = gate_5d.encode();
        let decoded = RenderGate64::decode(encoded).expect("upgraded 5D gate failed to decode");

        assert_eq!(decoded, gate_5d, "upgraded 5D gate round-trip was not identity");

        let decoded_packet = decoded.get_5d();
        assert_eq!(decoded_packet, packet, "RenderGate5D did not round-trip identically");
    }
}
