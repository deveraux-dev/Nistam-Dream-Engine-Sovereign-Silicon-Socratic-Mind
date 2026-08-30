//! `EngineTick8` — the 8-byte engine tick word (ENGINE-SPINE-BRIEF.md
//! "EngineTick8"): a 120Hz carrier frame count plus a pulse clock derived
//! from it, `phase = frame % 30` (`LCM(1,2,3,5) == 30`, the sky doctrine's
//! pulse clock), CRT residues over `{2,3,5}`, a 5-trit base-3 mode byte, and
//! a wrap flag.
//!
//! Machine-first (L08): `phase`/`residues`/`flags` are all derived from
//! `frame` by integer `%` and bit ops in `encode`; nothing here is a float.
//!
//! ONE HOME (L05): this file is the only definition of `EngineTick8`.

/// The pulse clock's period: `LCM(1, 2, 3, 5) == 30`. `phase` is always
/// `frame % PULSE_PERIOD`, so `phase` never reaches this value.
pub const PULSE_PERIOD: u8 = 30;

/// Highest valid mode byte: 5 trits base-3 top out at `2 + 2*3 + 2*9 + 2*27 +
/// 2*81 == 242` (the `forge-material-v3` `CRAFT_BYTE_MAX` grammar, copied
/// verbatim — same 5-trit base-3 byte, same ceiling).
pub const MODE_BYTE_MAX: u8 = 242;

/// Run-state trit0 values: `encode`'s `run_state` parameter domain.
pub const RUN_STATE_HALT: u8 = 0;
/// Run-state trit0: running.
pub const RUN_STATE_RUN: u8 = 1;
/// Run-state trit0: replay.
pub const RUN_STATE_REPLAY: u8 = 2;

/// Register trit1: dark mature fantasy (ARCH000 2026-08-11 seat ruling,
/// ENGINE-SPINE-LEDGER.md "REGISTER IS ONE TRIT": Inferno=-1,
/// Purgatorio=0, Paradiso=+1 — the narrative canon and the balanced-trit
/// grammar are the same base-3 word).
pub const REGISTER_INFERNO: i8 = -1;
/// Register trit1: the middle.
pub const REGISTER_PURGATORIO: i8 = 0;
/// Register trit1: bright high fantasy.
pub const REGISTER_PARADISO: i8 = 1;

/// How many trits one mode byte carries.
const TRITS_PER_BYTE: usize = 5;

/// Pack 5 balanced trits (`-1, 0, +1`, least-significant first) into one
/// base-3 byte, `0..=MODE_BYTE_MAX`. Copied from `forge-material-v3`'s
/// `pack5` — same codec, same domain.
#[inline(always)]
const fn pack5(trits: [i8; TRITS_PER_BYTE]) -> u8 {
    let mut value: u16 = 0;
    let mut place: u16 = 1;
    let mut i = 0;
    while i < TRITS_PER_BYTE {
        let digit = (trits[i] + 1) as u16;
        value += digit * place;
        place *= 3;
        i += 1;
    }
    value as u8
}

/// Unpack a base-3 byte into 5 balanced trits. `None` for `243..=255` — a
/// byte with no balanced-trit reading is corruption refused at the
/// boundary, not clamped past it.
#[inline(always)]
const fn unpack5(byte: u8) -> Option<[i8; TRITS_PER_BYTE]> {
    if byte > MODE_BYTE_MAX {
        return None;
    }
    let mut v = byte as u16;
    let mut trits = [0i8; TRITS_PER_BYTE];
    let mut i = 0;
    while i < TRITS_PER_BYTE {
        let digit = (v % 3) as i8;
        trits[i] = digit - 1;
        v /= 3;
        i += 1;
    }
    Some(trits)
}

/// One engine tick, 8 bytes, exact. Field order is offset order — every
/// byte is a field, no padding hole. The offsets below are locked by rustc,
/// not prose.
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineTick8 {
    /// Monotonic tick count, the 120Hz carrier.
    pub frame: u32,
    /// `frame % PULSE_PERIOD` — the pulse clock phase, `0..PULSE_PERIOD`.
    pub phase: u8,
    /// CRT residues over `{2, 3, 5}`, packed: bit0 = `phase % 2`, bits2-3 =
    /// `phase % 3`, bits4-6 = `phase % 5`, bit1 and bit7 zero.
    pub residues: u8,
    /// 5-trit base-3 mode byte, `0..=MODE_BYTE_MAX`. Trit0 is the run-state
    /// (`RUN_STATE_HALT`/`RUN_STATE_RUN`/`RUN_STATE_REPLAY`); trit1 is the
    /// register (`REGISTER_INFERNO`/`REGISTER_PURGATORIO`/
    /// `REGISTER_PARADISO`, ARCH000 2026-08-11 seat ruling); trits2-4 are
    /// OPEN lanes, `encode` only ever writes them `0` (ARCH000 pending).
    pub mode: u8,
    /// bit0 = wrap marker (`frame % PULSE_PERIOD == 0`); bits1-7 must be
    /// zero.
    pub flags: u8,
}

impl EngineTick8 {
    /// The origin: frame 0, phase 0, residues all zero, mode all-halt
    /// trits at the middle register, wrap set (`frame % PULSE_PERIOD == 0`
    /// at frame 0).
    pub const ORIGIN: Self = match Self::encode(0, RUN_STATE_HALT, REGISTER_PURGATORIO) {
        Some(t) => t,
        None => panic!("the origin tick failed to encode"),
    };

    /// The CRT residue byte for a given phase: bit0 = `phase % 2`, bits2-3 =
    /// `phase % 3`, bits4-6 = `phase % 5`.
    #[inline(always)]
    const fn residues_for(phase: u8) -> u8 {
        (phase % 2) | ((phase % 3) << 2) | ((phase % 5) << 4)
    }

    /// Encode a tick from its three free inputs; `phase`, `residues`, and
    /// `flags` are all derived from `frame` (single source of truth). `None`
    /// when `run_state` is outside `0..=2` or `register` is outside
    /// `-1..=1` — corruption refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn encode(frame: u32, run_state: u8, register: i8) -> Option<Self> {
        if run_state > RUN_STATE_REPLAY {
            return None;
        }
        if register < REGISTER_INFERNO || register > REGISTER_PARADISO {
            return None;
        }
        let phase = (frame % PULSE_PERIOD as u32) as u8;
        let mode = pack5([run_state as i8 - 1, register, 0, 0, 0]);
        let flags = if phase == 0 { 1 } else { 0 };
        Some(Self { frame, phase, residues: Self::residues_for(phase), mode, flags })
    }

    /// True when every channel is internally consistent: `phase ==
    /// frame % PULSE_PERIOD` and `phase < PULSE_PERIOD`, `residues` matches
    /// `phase`, `mode <= MODE_BYTE_MAX` with trits2-4 all zero (trit0
    /// run-state and trit1 register are the live lanes), and `flags` has no
    /// live bit beyond bit0, matching the wrap condition.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        if self.phase >= PULSE_PERIOD {
            return false;
        }
        if self.phase != (self.frame % PULSE_PERIOD as u32) as u8 {
            return false;
        }
        if self.residues != Self::residues_for(self.phase) {
            return false;
        }
        let trits = match unpack5(self.mode) {
            Some(t) => t,
            None => return false,
        };
        if trits[2] != 0 || trits[3] != 0 || trits[4] != 0 {
            return false;
        }
        let expect_wrap = if self.phase == 0 { 1u8 } else { 0u8 };
        self.flags == expect_wrap
    }

    /// The run-state trit0, folded back to `0..=2`. `None` when `mode` has
    /// no balanced-trit reading.
    #[inline(always)]
    pub const fn run_state(self) -> Option<u8> {
        match unpack5(self.mode) {
            Some(trits) => Some((trits[0] + 1) as u8),
            None => None,
        }
    }

    /// The register trit1, balanced (`REGISTER_INFERNO..=REGISTER_PARADISO`).
    /// `None` when `mode` has no balanced-trit reading.
    #[inline(always)]
    pub const fn register(self) -> Option<i8> {
        match unpack5(self.mode) {
            Some(trits) => Some(trits[1]),
            None => None,
        }
    }

    /// Pack into one little-endian u64 word, low 8 bytes live: frame,
    /// phase, residues, mode, flags in struct/offset order.
    #[inline(always)]
    pub const fn encode_word(self) -> u64 {
        self.frame as u64
            | (self.phase as u64) << 32
            | (self.residues as u64) << 40
            | (self.mode as u64) << 48
            | (self.flags as u64) << 56
    }

    /// Unpack a word. `None` for anything outside the valid domain — see
    /// `is_valid`.
    #[inline(always)]
    pub const fn decode_word(word: u64) -> Option<Self> {
        let c = Self {
            frame: word as u32,
            phase: (word >> 32) as u8,
            residues: (word >> 40) as u8,
            mode: (word >> 48) as u8,
            flags: (word >> 56) as u8,
        };
        if c.is_valid() {
            Some(c)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<EngineTick8>() == 8);
const _: () = assert!(core::mem::align_of::<EngineTick8>() == 4);

// OFFSET LOCKS. Size alone is weak: swapping two fields keeps size 8 while
// silently reinterpreting every stored tick.
const _: () = assert!(core::mem::offset_of!(EngineTick8, frame) == 0);
const _: () = assert!(core::mem::offset_of!(EngineTick8, phase) == 4);
const _: () = assert!(core::mem::offset_of!(EngineTick8, residues) == 5);
const _: () = assert!(core::mem::offset_of!(EngineTick8, mode) == 6);
const _: () = assert!(core::mem::offset_of!(EngineTick8, flags) == 7);

// Every one of the 8 bytes is a field — no padding hole.
const _: () = assert!(4 + 1 + 1 + 1 + 1 == core::mem::size_of::<EngineTick8>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match EngineTick8::decode_word(EngineTick8::ORIGIN.encode_word()) {
        Some(w) => {
            assert!(w.frame == 0 && w.phase == 0 && w.residues == 0 && w.flags == 1);
            assert!(matches!(w.run_state(), Some(RUN_STATE_HALT)));
            assert!(matches!(w.register(), Some(REGISTER_PURGATORIO)));
        }
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(all(test, not(feature = "sky-mount")))]
mod tests {
    use super::*;

    /// L07 over the interior: every frame in one full pulse period, all
    /// three run states, all three registers, survives an encode-then-decode
    /// round trip.
    #[test]
    fn word_bijection_holds_over_the_interior() {
        for run_state in [RUN_STATE_HALT, RUN_STATE_RUN, RUN_STATE_REPLAY] {
            for register in [REGISTER_INFERNO, REGISTER_PURGATORIO, REGISTER_PARADISO] {
                for frame in 0u32..90 {
                    let t = EngineTick8::encode(frame, run_state, register)
                        .expect("in-domain encode refused");
                    assert_eq!(
                        EngineTick8::decode_word(t.encode_word()),
                        Some(t),
                        "frame={frame} run_state={run_state} register={register}"
                    );
                }
            }
        }
    }

    /// L07 over the sentinels: frame 0, 29, 30, and `u32::MAX`.
    #[test]
    fn word_bijection_holds_over_the_sentinels() {
        for frame in [0u32, 29, 30, u32::MAX] {
            let t = EngineTick8::encode(frame, RUN_STATE_RUN, REGISTER_PURGATORIO)
                .expect("sentinel encode refused");
            assert_eq!(EngineTick8::decode_word(t.encode_word()), Some(t), "frame={frame}");
        }
    }

    /// The origin: `ORIGIN` round-trips and reads back as halt, wrapped.
    #[test]
    fn the_origin_survives_its_wire() {
        let t = EngineTick8::ORIGIN;
        assert_eq!(EngineTick8::decode_word(t.encode_word()), Some(t));
        assert_eq!(t.run_state(), Some(RUN_STATE_HALT));
        assert_eq!(t.register(), Some(REGISTER_PURGATORIO));
        assert_eq!(t.flags, 1);
    }

    /// `phase` and `residues` are derived correctly across a full period.
    #[test]
    fn phase_and_residues_match_frame_modulo_the_period() {
        for frame in 0u32..120 {
            let t = EngineTick8::encode(frame, RUN_STATE_RUN, REGISTER_PURGATORIO).unwrap();
            assert_eq!(t.phase, (frame % PULSE_PERIOD as u32) as u8);
            assert_eq!(t.residues & 0b1, (t.phase % 2) & 0b1);
            assert_eq!((t.residues >> 2) & 0b11, t.phase % 3);
            assert_eq!((t.residues >> 4) & 0b111, t.phase % 5);
            let expect_wrap = if t.phase == 0 { 1 } else { 0 };
            assert_eq!(t.flags, expect_wrap);
        }
    }

    /// The boundary refuses corruption: each invalid word decodes to None,
    /// and an out-of-domain `run_state` or `register` is refused by
    /// `encode` itself.
    #[test]
    fn out_of_domain_ticks_are_refused() {
        assert_eq!(EngineTick8::encode(0, 3, REGISTER_PURGATORIO), None, "run_state 3 was accepted");
        assert_eq!(EngineTick8::encode(0, 255, REGISTER_PURGATORIO), None, "run_state 255 was accepted");
        assert_eq!(EngineTick8::encode(0, RUN_STATE_RUN, -2), None, "register -2 was accepted");
        assert_eq!(EngineTick8::encode(0, RUN_STATE_RUN, 2), None, "register 2 was accepted");
        assert_eq!(EngineTick8::encode(0, RUN_STATE_RUN, i8::MIN), None, "register i8::MIN was accepted");

        let good = EngineTick8::encode(31, RUN_STATE_RUN, REGISTER_PURGATORIO).unwrap();
        assert!(EngineTick8::decode_word(good.encode_word()).is_some(), "the baseline itself is invalid");

        let bad = [
            EngineTick8 { phase: PULSE_PERIOD, ..good },
            EngineTick8 { phase: good.phase.wrapping_add(1), ..good },
            EngineTick8 { residues: good.residues ^ 1, ..good },
            EngineTick8 { mode: 243, ..good },
            EngineTick8 { flags: good.flags | 0b10, ..good },
        ];
        for (i, b) in bad.iter().enumerate() {
            assert_eq!(EngineTick8::decode_word(b.encode_word()), None, "bad row {i} was accepted");
        }
    }

    /// `mode`'s trits2-4 are OPEN lanes: `encode` only ever writes them
    /// zero, and `decode_word` refuses any tick whose mode byte carries a
    /// nonzero open-lane trit, even though the byte itself is `<=
    /// MODE_BYTE_MAX`.
    #[test]
    fn open_mode_lanes_refuse_a_nonzero_trit() {
        let good = EngineTick8::encode(0, RUN_STATE_RUN, REGISTER_INFERNO).unwrap();
        assert_eq!(good.mode, pack5([RUN_STATE_RUN as i8 - 1, REGISTER_INFERNO, 0, 0, 0]));
        for lane in 2..TRITS_PER_BYTE {
            let mut trits = unpack5(good.mode).unwrap();
            trits[lane] = 1;
            let bad = EngineTick8 { mode: pack5(trits), ..good };
            assert_eq!(EngineTick8::decode_word(bad.encode_word()), None, "open lane {lane} accepted a nonzero trit");
        }
    }

    /// `run_state` reads trit0 back and refuses a corrupt mode byte.
    #[test]
    fn run_state_reads_trit_zero() {
        for run_state in [RUN_STATE_HALT, RUN_STATE_RUN, RUN_STATE_REPLAY] {
            let t = EngineTick8::encode(0, run_state, REGISTER_PURGATORIO).unwrap();
            assert_eq!(t.run_state(), Some(run_state));
        }
        let mut corrupt = EngineTick8::encode(0, RUN_STATE_RUN, REGISTER_PURGATORIO).unwrap();
        corrupt.mode = 243;
        assert_eq!(corrupt.run_state(), None);
    }

    /// `register` reads trit1 back — orthogonal to trit0 across all nine
    /// run-state x register pairs — and refuses a corrupt mode byte.
    #[test]
    fn register_reads_trit_one() {
        for run_state in [RUN_STATE_HALT, RUN_STATE_RUN, RUN_STATE_REPLAY] {
            for register in [REGISTER_INFERNO, REGISTER_PURGATORIO, REGISTER_PARADISO] {
                let t = EngineTick8::encode(0, run_state, register).unwrap();
                assert_eq!(t.register(), Some(register), "run_state={run_state}");
                assert_eq!(t.run_state(), Some(run_state), "register={register}");
            }
        }
        let mut corrupt = EngineTick8::encode(0, RUN_STATE_RUN, REGISTER_PARADISO).unwrap();
        corrupt.mode = 243;
        assert_eq!(corrupt.register(), None);
    }
}
