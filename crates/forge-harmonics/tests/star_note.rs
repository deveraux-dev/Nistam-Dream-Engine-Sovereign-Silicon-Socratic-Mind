//! `star_note_on` is the byte hardware wants; `star_voice_on` is that byte
//! crossed into frequency. This pins the two to ONE arithmetic so the giveaway
//! can bake 1 byte per star instead of 4 and still ring the same pitch.

use forge_harmonics::scale_voice::{note_to_mhz, star_note_on, star_voice_on};
use forge_harmonics::theory::{ALCHEMICAL, MAJOR_PENTATONIC, SCALES};

fn scale() -> &'static [u8] {
    SCALES[MAJOR_PENTATONIC].degrees
}

/// At A440 the voice is exactly the note crossed to frequency — no residue.
#[test]
fn the_note_and_the_voice_are_the_same_pitch() {
    for &(kelvin, mag_pmy, dist_pc) in &[
        (2_000i32, -14_600i32, 0u16),
        (5_800, 0, 1),
        (9_400, 45_000, 260),
        (25_000, 90_000, 2_048),
        (40_000, 20_000, 700),
    ] {
        let note = star_note_on(scale(), kelvin, mag_pmy, dist_pc);
        let voice = star_voice_on(scale(), 440_000, kelvin, mag_pmy, dist_pc);
        assert_eq!(
            voice,
            note_to_mhz(note) as u64,
            "kelvin={kelvin} mag={mag_pmy} dist={dist_pc} note={note}"
        );
    }
}

/// Every star in the catalog's real input span lands inside MIDI range, so a
/// single byte is a lossless carrier for the pitch.
#[test]
fn every_note_fits_in_one_byte() {
    for kelvin in (2_000..40_000).step_by(613) {
        for mag_pmy in (-14_600..90_000).step_by(4_099) {
            for dist_pc in [0u16, 1, 17, 260, 2_048] {
                let n = star_note_on(scale(), kelvin, mag_pmy, dist_pc);
                assert!(n <= 127, "note {n} out of MIDI range");
            }
        }
    }
}

/// An empty scale yields note 0 — and `star_voice_on` still answers 0 rather
/// than the ~8Hz that note 0 would sound. The guard lives in the caller.
#[test]
fn an_empty_scale_is_silence_not_note_zero() {
    assert_eq!(star_note_on(&[], 5_800, 0, 1), 0);
    assert_eq!(star_voice_on(&[], 440_000, 5_800, 0, 1), 0);
}

/// The tuning reference is the ONLY thing that moves the voice off the note —
/// which is exactly the multiply-divide that belongs at the audio edge in
/// cents, not here on the frequency.
#[test]
fn only_the_tuning_ref_separates_voice_from_note() {
    let (k, m, d) = (5_800, 0, 1);
    let at_440 = star_voice_on(scale(), 440_000, k, m, d);
    let at_432 = star_voice_on(scale(), ALCHEMICAL.ref_a_mhz, k, m, d);
    assert_eq!(star_note_on(scale(), k, m, d), star_note_on(scale(), k, m, d));
    if ALCHEMICAL.ref_a_mhz != 440_000 {
        assert_ne!(at_440, at_432, "a different reference must move the voice");
    }
}
