/// Keyboard → General MIDI drum map (channel 9).
///
/// Maps ASCII keycodes (lowercase) to GM drum note numbers (35–81).
/// Velocity is caller-supplied (0–127). Channel is always 9 (GM percussion).
///
/// Layout philosophy: home row = core kit, upper row = toms + cymbals,
/// number row = world percussion / effects. Designed for one-hand or
/// two-hand play on a standard QWERTY layout.

/// General MIDI drum note numbers (channel 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DrumNote {
    AcousticBassDrum = 35,
    BassDrum1        = 36,
    SideStick        = 37,
    AcousticSnare    = 38,
    HandClap         = 39,
    ElectricSnare    = 40,
    LowFloorTom      = 41,
    ClosedHiHat      = 42,
    HighFloorTom     = 43,
    PedalHiHat       = 44,
    LowTom           = 45,
    OpenHiHat        = 46,
    LowMidTom        = 47,
    HiMidTom         = 48,
    CrashCymbal1     = 49,
    HighTom          = 50,
    RideCymbal1      = 51,
    ChineseCymbal    = 52,
    RideBell         = 53,
    Tambourine       = 54,
    SplashCymbal     = 55,
    Cowbell          = 56,
    CrashCymbal2     = 57,
    Vibraslap        = 58,
    RideCymbal2      = 59,
    HiBongo          = 60,
    LowBongo         = 61,
    MuteHiConga      = 62,
    OpenHiConga      = 63,
    LowConga         = 64,
    HighTimbale      = 65,
    LowTimbale       = 66,
    HighAgogo        = 67,
    LowAgogo         = 68,
    Cabasa           = 69,
    Maracas          = 70,
}

impl DrumNote {
    #[inline]
    pub fn midi_note(self) -> u8 {
        self as u8
    }
}

/// Map an ASCII keycode (lowercase) to a drum note.
/// Returns `None` for unmapped keys.
///
/// ```
/// # use forge_audio_v3::forge_midi::keyboard_drum::key_to_drum;
/// assert_eq!(key_to_drum(b'a').map(|d| d.midi_note()), Some(36)); // kick
/// assert_eq!(key_to_drum(b's').map(|d| d.midi_note()), Some(38)); // snare
/// ```
pub fn key_to_drum(key: u8) -> Option<DrumNote> {
    Some(match key {
        // Home row — core kit
        b'a' => DrumNote::BassDrum1,
        b's' => DrumNote::AcousticSnare,
        b'd' => DrumNote::ClosedHiHat,
        b'f' => DrumNote::OpenHiHat,
        b'g' => DrumNote::PedalHiHat,
        b'h' => DrumNote::LowTom,
        b'j' => DrumNote::LowMidTom,
        b'k' => DrumNote::HiMidTom,
        b'l' => DrumNote::HighTom,
        b';' => DrumNote::CrashCymbal1,
        b'\'' => DrumNote::RideCymbal1,
        // Upper row — toms + cymbals
        b'q' => DrumNote::AcousticBassDrum,
        b'w' => DrumNote::SideStick,
        b'e' => DrumNote::HandClap,
        b'r' => DrumNote::ElectricSnare,
        b't' => DrumNote::LowFloorTom,
        b'y' => DrumNote::HighFloorTom,
        b'u' => DrumNote::HiBongo,
        b'i' => DrumNote::LowBongo,
        b'o' => DrumNote::CrashCymbal2,
        b'p' => DrumNote::RideBell,
        // Number row — world percussion + effects
        b'1' => DrumNote::Cowbell,
        b'2' => DrumNote::Tambourine,
        b'3' => DrumNote::Cabasa,
        b'4' => DrumNote::Maracas,
        b'5' => DrumNote::SplashCymbal,
        b'6' => DrumNote::ChineseCymbal,
        b'7' => DrumNote::Vibraslap,
        b'8' => DrumNote::MuteHiConga,
        b'9' => DrumNote::OpenHiConga,
        b'0' => DrumNote::LowConga,
        // Bottom row — timbales + agogos
        b'z' => DrumNote::HighTimbale,
        b'x' => DrumNote::LowTimbale,
        b'c' => DrumNote::HighAgogo,
        b'v' => DrumNote::LowAgogo,
        b'b' => DrumNote::RideCymbal2,
        _ => return None,
    })
}

/// Convert a raw keyboard event into a MIDI NoteOn event ready for `MidiSequencer`.
///
/// Returns `(channel=9, note, velocity)`. Velocity is fixed at 100 for now;
/// future: pressure-sensitive key APIs supply it dynamically.
pub fn key_to_midi_note_on(key: u8) -> Option<(u8, u8, u8)> {
    key_to_drum(key).map(|d| (9, d.midi_note(), 100))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_row_maps_core_kit() {
        assert_eq!(key_to_drum(b'a'), Some(DrumNote::BassDrum1));
        assert_eq!(key_to_drum(b's'), Some(DrumNote::AcousticSnare));
        assert_eq!(key_to_drum(b'd'), Some(DrumNote::ClosedHiHat));
        assert_eq!(key_to_drum(b'f'), Some(DrumNote::OpenHiHat));
    }

    #[test]
    fn unmapped_key_returns_none() {
        assert_eq!(key_to_drum(b'm'), None);
        assert_eq!(key_to_drum(b' '), None);
    }

    #[test]
    fn midi_note_on_channel_is_always_9() {
        let (ch, _, _) = key_to_midi_note_on(b'a').unwrap();
        assert_eq!(ch, 9);
    }

    #[test]
    fn all_mapped_keys_have_valid_gm_note() {
        let keys = b"asdfghjkl;'qwertyuiopzxcvb1234567890";
        for &k in keys {
            if let Some(d) = key_to_drum(k) {
                assert!((35..=81).contains(&d.midi_note()));
            }
        }
    }
}
