//! Integer-only input bitfield decoder with diagonal normalization.
//!
//! Decodes a raw `u16` bitfield into normalized Permyriad movement. Zero floating-point;
//! the diagonal factor is a precomputed Permyriad constant, so there is no runtime `sqrt`.
//!
//! Bit layout (`raw_input: u16`):
//! ```text
//!   0: UP    1: DOWN   2: LEFT   3: RIGHT
//!   4: PRIMARY (A)     5: SECONDARY (B)
//!   6: SURGE (Y)       7-15: reserved
//! ```
//!
//! Ported 2026-08-15 from `F:\v3\TODO\pulled-reference\input_decoder-variants\input_decoder.rs`
//! (md5 `AC984CA7`, the richer of the two variants; the live v2 copy at
//! `F:\NewRepo\crates\forge-input\src\input_decoder.rs` is md5 `BB5B139A`, ~1KB leaner —
//! same core arithmetic, fewer comments/tests. The E:\ copy of the variant is byte-identical
//! tape, not a third source).
//!
//! **Named, not silently dropped (C09 aperture):** v2's `apply_digital_to_game_inputs` is NOT
//! ported. It is the only function here that touched `crate::GameInputs`, a type this crate
//! does not have — and this crate ships zero runtime dependencies by law (`Cargo.toml:9-11`).
//! The caller that owns both sides can write that three-line fallback itself; importing a
//! game-state type to host it would break the firewall for a convenience.
//!
//! ## Why this is also a ray direction
//!
//! `decode_raw_input` cancels opposites, so each axis lands in **{−1, 0, +1}** — a balanced
//! trit, exactly the `(m,k)=(1,1)` orbit type `PARARITY.md` §3 Corollary 2 proves is the only
//! one that can carry a neutral value. The zero is not "no input"; it is the fixed point of
//! the left↔right mirror (pinned by `opposites_cancel` below), which is precisely the state a
//! directed march needs for "this lane does not participate". See [`DecodedInput::trits`].

/// Bit 0 — up.
pub const BIT_UP: u16 = 1 << 0;
/// Bit 1 — down.
pub const BIT_DOWN: u16 = 1 << 1;
/// Bit 2 — left.
pub const BIT_LEFT: u16 = 1 << 2;
/// Bit 3 — right.
pub const BIT_RIGHT: u16 = 1 << 3;
/// Bit 4 — primary action (A).
pub const BIT_PRIMARY: u16 = 1 << 4;
/// Bit 5 — secondary action (B).
pub const BIT_SECONDARY: u16 = 1 << 5;
/// Bit 6 — surge (Y).
pub const BIT_SURGE: u16 = 1 << 6;

/// Cardinal normalization: 10000 Permyriad = 100%.
const NORM_CARDINAL: i64 = 10_000;
/// Diagonal normalization: 7071 Permyriad ≈ √2/2, keeping magnitude ≤ 10000 without a `sqrt`.
const NORM_DIAGONAL: i64 = 7_071;

/// Decoded input state — all integers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DecodedInput {
    /// Normalized X movement in Permyriad, `-10000..=10000`.
    pub move_x: i32,
    /// Normalized Y movement in Permyriad, `-10000..=10000`.
    pub move_y: i32,
    /// Primary action held.
    pub primary: bool,
    /// Secondary action held.
    pub secondary: bool,
    /// Surge held.
    pub surge: bool,
}

impl DecodedInput {
    /// The movement as a pair of **balanced trits** `{-1, 0, +1}`, one per axis.
    ///
    /// This is the same information as [`Self::move_x`]/[`Self::move_y`] with the magnitude
    /// discarded — the *direction* alone. It is the form a lattice march wants: a per-lane
    /// step of back / hold / forward, where `0` means the axis is not traversed at all.
    ///
    /// Magnitude and direction are deliberately separate: the Permyriad fields carry the
    /// diagonal normalization (7071) that a renderer needs, while the trits carry the
    /// lattice step that a discrete walk needs. Rounding one from the other at the call site
    /// is how a diagonal becomes a 1.41-cell step by accident.
    #[inline]
    pub const fn trits(self) -> [i8; 2] {
        [self.move_x.signum() as i8, self.move_y.signum() as i8]
    }

    /// True when neither axis participates — the fixed point of the mirror involution.
    #[inline]
    pub const fn is_still(self) -> bool {
        self.move_x == 0 && self.move_y == 0
    }
}

/// Decode a raw bitfield into normalized integer movement.
///
/// Opposites cancel (LEFT|RIGHT is zero, not "last wins"), and a diagonal is capped so its
/// magnitude never exceeds cardinal speed.
#[inline]
pub fn decode_raw_input(raw: u16) -> DecodedInput {
    // Extract directional bits and cancel opposites. Each axis is now a balanced trit.
    let x_raw: i64 = ((raw & BIT_RIGHT) != 0) as i64 - ((raw & BIT_LEFT) != 0) as i64;
    let y_raw: i64 = ((raw & BIT_UP) != 0) as i64 - ((raw & BIT_DOWN) != 0) as i64;

    // Diagonal detection: both axes non-zero — i.e. neither lane sits on its fixed point.
    let is_diagonal = x_raw != 0 && y_raw != 0;
    let norm_factor = if is_diagonal { NORM_DIAGONAL } else { NORM_CARDINAL };

    DecodedInput {
        move_x: (x_raw * norm_factor) as i32,
        move_y: (y_raw * norm_factor) as i32,
        primary: (raw & BIT_PRIMARY) != 0,
        secondary: (raw & BIT_SECONDARY) != 0,
        surge: (raw & BIT_SURGE) != 0,
    }
}

/// MilliUnit displacement from a normalized direction, a speed, and a tick's duration.
///
/// `(norm_dir * speed_milli_per_sec * dt_micros) / 10_000_000_000` — the 10B divisor resolves
/// Permyriad × MilliUnit/s × microseconds without overflow or a float.
#[inline]
pub fn calculate_displacement(norm_dir: i32, speed_milli_per_sec: i64, dt_micros: u32) -> i64 {
    (norm_dir as i64 * speed_milli_per_sec * dt_micros as i64) / 10_000_000_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cardinal_right() {
        let d = decode_raw_input(BIT_RIGHT);
        assert_eq!(d.move_x, 10_000);
        assert_eq!(d.move_y, 0);
    }

    #[test]
    fn cardinal_up() {
        let d = decode_raw_input(BIT_UP);
        assert_eq!(d.move_x, 0);
        assert_eq!(d.move_y, 10_000);
    }

    #[test]
    fn diagonal_capped() {
        let d = decode_raw_input(BIT_RIGHT | BIT_UP);
        assert_eq!(d.move_x, 7_071);
        assert_eq!(d.move_y, 7_071);
        let mag_sq = (d.move_x as i64).pow(2) + (d.move_y as i64).pow(2);
        assert!(mag_sq <= 10_000 * 10_000 + 1, "diagonal magnitude exceeded: {mag_sq}");
    }

    /// The pararity fixed point, as a test: the left↔right mirror sends +1 to −1 and fixes 0,
    /// so pressing both must land ON the fixed point rather than picking a winner.
    #[test]
    fn opposites_cancel() {
        let d = decode_raw_input(BIT_LEFT | BIT_RIGHT);
        assert_eq!(d.move_x, 0);
        assert_eq!(d.move_y, 0);
        assert!(d.is_still());
    }

    #[test]
    fn displacement_calculation() {
        // Right at 5000 milli/s for 8333 microseconds (one 120Hz tick).
        assert_eq!(calculate_displacement(10_000, 5_000, 8_333), 41);
    }

    #[test]
    fn zero_speed_zero_displacement() {
        assert_eq!(calculate_displacement(10_000, 0, 8_333), 0);
    }

    /// Every decoded axis is a balanced trit — never a magnitude that leaked through.
    #[test]
    fn trits_are_balanced_ternary() {
        for raw in 0u16..=0x7F {
            let t = decode_raw_input(raw).trits();
            for lane in t {
                assert!((-1..=1).contains(&lane), "lane {lane} outside balanced ternary for raw={raw:#x}");
            }
        }
    }

    /// Direction survives the diagonal normalization: a diagonal is still (+1,+1) as a step,
    /// even though its Permyriad magnitude was scaled down to 7071.
    #[test]
    fn the_diagonal_keeps_its_direction_after_normalization() {
        let d = decode_raw_input(BIT_RIGHT | BIT_UP);
        assert_eq!(d.trits(), [1, 1], "normalization must not round a diagonal step away");
        assert_eq!(decode_raw_input(BIT_LEFT | BIT_DOWN).trits(), [-1, -1]);
    }

    /// The mirror involution is its own inverse on the trit lanes: flipping LEFT<->RIGHT and
    /// UP<->DOWN negates the direction, and doing it twice returns the original (L07).
    #[test]
    fn the_mirror_is_an_involution_on_the_lanes() {
        let mirror = |raw: u16| -> u16 {
            let mut m = raw & !(BIT_UP | BIT_DOWN | BIT_LEFT | BIT_RIGHT);
            if raw & BIT_LEFT != 0 { m |= BIT_RIGHT }
            if raw & BIT_RIGHT != 0 { m |= BIT_LEFT }
            if raw & BIT_UP != 0 { m |= BIT_DOWN }
            if raw & BIT_DOWN != 0 { m |= BIT_UP }
            m
        };
        for raw in 0u16..=0x0F {
            let a = decode_raw_input(raw).trits();
            let b = decode_raw_input(mirror(raw)).trits();
            assert_eq!(b, [-a[0], -a[1]], "mirror must negate the lanes for raw={raw:#x}");
            assert_eq!(decode_raw_input(mirror(mirror(raw))).trits(), a, "f(f(x)) != x");
        }
    }
}
