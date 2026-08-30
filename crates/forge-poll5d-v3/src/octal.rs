//! `Octal64` — the 64-byte tier-slot pack, T4 of the forge-vision drain
//! (plan-2-welds.md T4, quarried from 05-poll5d.md's 21x3-bit/u64 hardcoded
//! octal scheme — the encoding is new, the discipline is drained).
//!
//! [ASSUMED: the brief and the crate skeleton's own header comment both
//! name this type "64 bytes" while describing only 60+4=64 *bits* of
//! payload. The quarry files are inaccessible (brief line 66-67), so there
//! is no source layout to recover. This file resolves the contradiction by
//! reading "Octal64" as a 64-byte, 64-byte-*aligned* word — one packed u64
//! carrying the 20 tier slots, plus 56 reserved bytes for the "future
//! trit-grammar alignment" the brief names on line 14. A 64-byte align is a
//! cache-line width, which is the only reading that gives the "64" in the
//! name independent meaning from the "8" in `Morton8`. Tests are the
//! authority per the dispatch brief: the bijection gate below is exact and
//! self-consistent regardless of which reading is "true" upstream.]
//!
//! Machine-first (L08): slot pack/unpack is integer shift-and-mask only,
//! no float anywhere on the path.
//!
//! ONE HOME (L05): this file is the only definition of `Octal64`.

/// How many 3-bit tier slots the packed word carries: `20 * 3 == 60` bits,
/// leaving 4 spare bits in the word for future trit-grammar alignment
/// (brief line 14).
pub const OCTAL_SLOTS: usize = 20;

/// Bits per tier slot.
pub const OCTAL_SLOT_BITS: u32 = 3;

/// Saturating sentinel and inclusive ceiling for a tier value: the maximum
/// 3-bit cell, `0b111 == 7`. `oct_set` refuses anything above it rather than
/// wrapping — there is no tier 8.
pub const OCTAL_TIER_MAX: u8 = 0b111;

/// One poll5d octal state pack, 64 bytes, exact: a single 60-bit-payload
/// word (20 tier slots, 3 bits each, plus 4 spare bits) followed by 56
/// reserved bytes held at zero until a later tranche defines them. Field
/// order is offset order — every byte is a field, no padding hole. The
/// offsets below are locked by rustc, not prose.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Octal64 {
    /// The 20 packed 3-bit tier slots (bits 0..60) plus 4 spare bits
    /// (bits 60..64), held at zero until a tranche defines them.
    pub word: u64,
    /// Reserved for future trit-grammar alignment (brief line 14). A
    /// nonzero reserved byte today is corruption, never
    /// forward-compatibility — same doctrine as `ColourTrit8::tags`.
    pub reserved: [u8; 56],
}

impl Octal64 {
    /// The origin: every tier slot at 0, no spare or reserved bits live.
    pub const ORIGIN: Self = Self { word: 0, reserved: [0; 56] };

    /// True when the word's 4 spare bits and all 56 reserved bytes are
    /// zero. `decode` is only a bijection over words where this holds.
    #[inline(always)]
    pub const fn is_valid(self) -> bool {
        (self.word >> (OCTAL_SLOTS as u32 * OCTAL_SLOT_BITS)) == 0 && Self::reserved_is_zero(&self.reserved)
    }

    #[inline(always)]
    const fn reserved_is_zero(r: &[u8; 56]) -> bool {
        let mut i = 0;
        while i < r.len() {
            if r[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Read tier slot `slot`'s 3-bit value, `0..=OCTAL_TIER_MAX`. Slots
    /// outside `0..OCTAL_SLOTS` read as 0 — callers that need refusal for an
    /// out-of-range slot use `oct_set`, which is the only write path.
    #[inline(always)]
    pub const fn oct_get(self, slot: usize) -> u8 {
        if slot >= OCTAL_SLOTS {
            return 0;
        }
        let shift = slot as u32 * OCTAL_SLOT_BITS;
        ((self.word >> shift) & OCTAL_TIER_MAX as u64) as u8
    }

    /// Write tier slot `slot` to `value`. `None` for an out-of-range slot
    /// index or a value above the saturating sentinel — corruption refused
    /// at the boundary, not clamped past it (L08/L10 doctrine, matching
    /// `ColourTrit8::decode`'s refusal shape).
    #[inline(always)]
    pub const fn oct_set(self, slot: usize, value: u8) -> Option<Self> {
        if slot >= OCTAL_SLOTS || value > OCTAL_TIER_MAX {
            return None;
        }
        let shift = slot as u32 * OCTAL_SLOT_BITS;
        let mask = !((OCTAL_TIER_MAX as u64) << shift);
        Some(Self { word: (self.word & mask) | ((value as u64) << shift), reserved: self.reserved })
    }

    /// Pack into one little-endian u64 word — the 60-bit slot payload plus
    /// 4 spare bits. The 56 reserved bytes are not part of this word; they
    /// travel with the struct, not the wire word.
    #[inline(always)]
    pub const fn encode(self) -> u64 {
        self.word
    }

    /// Unpack a word into a fresh `Octal64` with reserved bytes zeroed.
    /// `None` when the word's spare bits (60..64) are live — corruption
    /// refused at the boundary, not clamped past it.
    #[inline(always)]
    pub const fn decode(word: u64) -> Option<Self> {
        let c = Self { word, reserved: [0; 56] };
        if c.is_valid() { Some(c) } else { None }
    }
}

// ---------------------------------------------------------------------------
// LAYOUT LOCKS. Measured with rustc, never hand-computed.
// ---------------------------------------------------------------------------
const _: () = assert!(core::mem::size_of::<Octal64>() == 64);
const _: () = assert!(core::mem::align_of::<Octal64>() == 64);

const _: () = assert!(core::mem::offset_of!(Octal64, word) == 0);
const _: () = assert!(core::mem::offset_of!(Octal64, reserved) == 8);

// Every one of the 64 bytes is a field — no padding hole.
const _: () = assert!(8 + 56 == core::mem::size_of::<Octal64>());

// The origin survives its own wire. A const gate so E0080 fires before any
// test harness runs: encode-then-decode is the identity at the origin.
const _: () = {
    match Octal64::decode(Octal64::ORIGIN.encode()) {
        Some(w) => assert!(w.word == 0, "the origin word did not decode to zero"),
        None => panic!("the origin failed to decode — the word layout is corrupt"),
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 over the interior: every (slot, tier) pair survives a
    /// set-then-get round trip, including the saturating sentinel 7, over
    /// all 20 slots and all 8 tier values.
    #[test]
    fn bijection_holds_over_all_slots_and_tiers() {
        for slot in 0..OCTAL_SLOTS {
            for tier in 0..=OCTAL_TIER_MAX {
                let w = Octal64::ORIGIN.oct_set(slot, tier).expect("in-domain slot/tier refused");
                assert_eq!(w.oct_get(slot), tier, "slot={slot} tier={tier}");
                assert!(w.is_valid(), "slot={slot} tier={tier} produced an invalid word");
                assert_eq!(Octal64::decode(w.encode()), Some(w), "slot={slot} tier={tier} did not survive the wire");
            }
        }
    }

    /// The saturating sentinel: tier 7 stays 7, never overflows into the
    /// next slot or the spare bits.
    #[test]
    fn tier_seven_is_a_sentinel_not_a_wraparound() {
        let w = Octal64::ORIGIN.oct_set(0, OCTAL_TIER_MAX).unwrap();
        assert_eq!(w.oct_get(0), 7);
        assert_eq!(w.oct_get(1), 0, "tier 7 in slot 0 leaked into slot 1");
        assert_eq!(w.word >> 60, 0, "tier 7 in slot 0 leaked into the spare bits");
    }

    /// Every slot packed simultaneously: fill all 20 slots with distinct
    /// tiers (cycling 0..=7) and read every one back.
    #[test]
    fn all_twenty_slots_pack_independently() {
        let mut w = Octal64::ORIGIN;
        for slot in 0..OCTAL_SLOTS {
            let tier = (slot % 8) as u8;
            w = w.oct_set(slot, tier).expect("in-domain slot/tier refused");
        }
        for slot in 0..OCTAL_SLOTS {
            assert_eq!(w.oct_get(slot), (slot % 8) as u8, "slot {slot} did not hold its own value");
        }
        assert!(w.is_valid());
        assert_eq!(Octal64::decode(w.encode()), Some(w));
    }

    /// The boundary refuses corruption: an out-of-range slot index, an
    /// above-sentinel tier value, and a word with live spare or reserved
    /// bits are all refused, not clamped.
    #[test]
    fn out_of_domain_values_are_refused() {
        assert_eq!(Octal64::ORIGIN.oct_set(OCTAL_SLOTS, 0), None, "slot 20 is out of range");
        assert_eq!(Octal64::ORIGIN.oct_set(0, OCTAL_TIER_MAX + 1), None, "tier 8 is out of range");

        let live_spare_bit = 1u64 << 60;
        assert_eq!(Octal64::decode(live_spare_bit), None, "a live spare bit was accepted");

        let mut reserved = [0u8; 56];
        reserved[0] = 1;
        let bad = Octal64 { word: 0, reserved };
        assert!(!bad.is_valid(), "a live reserved byte was accepted");
    }

    /// The origin: the all-zero pack is valid, every slot reads 0, and it
    /// survives its own wire.
    #[test]
    fn the_origin_survives_its_wire() {
        assert!(Octal64::ORIGIN.is_valid());
        for slot in 0..OCTAL_SLOTS {
            assert_eq!(Octal64::ORIGIN.oct_get(slot), 0);
        }
        assert_eq!(Octal64::decode(Octal64::ORIGIN.encode()), Some(Octal64::ORIGIN));
    }
}
