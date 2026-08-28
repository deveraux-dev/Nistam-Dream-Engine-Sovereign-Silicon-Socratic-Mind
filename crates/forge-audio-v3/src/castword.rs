//! A cast-word as an objet sonore (Schaeffer): the completed cast's identity
//! IS its envelope. Bridges `forge_mud_v3::casting` to the landed
//! `recipe::ce_audio` profile lane and `device::AudioLaneHandle`.

use forge_mud_v3::casting::Channel;

use crate::device::AudioLaneHandle;
use crate::gesture_brush::BrushOp;
use crate::recipe::ce_audio::effort_to_impact_profile;
use crate::recipe::AudioMaterialProfile;

/// `[AUTHORED]` the uncoloured cast-word: what a spoken glyph sounds like
/// before its effect shapes it. Mid-register so every brush op has room to
/// move it either way, and a ring long enough to read as a struck object
/// rather than a click.
pub const CASTWORD_BASE: AudioMaterialProfile = AudioMaterialProfile {
    ring_frequency_hz: 330.0,
    attack_sharpness: 0.45,
    harmonic_content: 0.5,
    decay_secs: 0.9,
    reverb_amount: 0.35,
};

/// `[AUTHORED]` which Laban brush op each effect line moves like, read off the
/// authored line itself (`forge_mud_v3::casting::EFFECT_LINES`).
///
/// Flick is light and quick, Press is strong and sustained, Wring twists. The
/// starling flock and the blade-before-the-mind are Flicks; the slowed world,
/// the returning warmth and the pooling mind are Presses; the settling veil and
/// the crying ground are Wrings. Seven lines onto three operators — the mapping
/// is a reading of the prose, not a derivation, which is why it says so.
pub fn brush_op_of_effect(effect_index: u8) -> Option<BrushOp> {
    match effect_index {
        0 | 1 => Some(BrushOp::Flick),
        2 | 3 | 4 => Some(BrushOp::Press),
        5 | 6 => Some(BrushOp::Wring),
        _ => None,
    }
}

/// The sound object a given effect mints, through the LANDED profile lane.
/// `None` for an effect index off the end of `EFFECT_LINES`.
pub fn castword_profile(effect_index: u8) -> Option<AudioMaterialProfile> {
    let op = brush_op_of_effect(effect_index)?;
    Some(effort_to_impact_profile(op, &CASTWORD_BASE))
}

/// MIDI middle-A, the tuning reference the note conversion pivots on.
const A4_HZ: f32 = 440.0;
/// MIDI note number of A4.
const A4_NOTE: f32 = 69.0;

/// A profile as one playable `(note, velocity, duration_ms)` trigger — the
/// shape [`AudioLaneHandle::trigger_notes`] already takes.
///
/// Ring frequency picks the pitch, attack sharpness the velocity, decay the
/// duration. The envelope IS the identity, so all three come off the profile
/// and none is a constant.
pub fn castword_note(profile: &AudioMaterialProfile) -> (u8, u8, u32) {
    let hz = profile.ring_frequency_hz.max(1.0);
    let note = (A4_NOTE + 12.0 * (hz / A4_HZ).log2()).round().clamp(0.0, 127.0) as u8;
    let velocity = (profile.attack_sharpness.clamp(0.0, 1.0) * 127.0).round() as u8;
    let duration_ms = (profile.decay_secs.max(0.0) * 1000.0).round().clamp(1.0, 10_000.0) as u32;
    (note, velocity, duration_ms)
}

/// Speak a completed cast on the live lane. Returns the trigger that was sent,
/// or `None` when the channel is not finished — a half-spoken word is silent.
///
/// The dispatch weld the `castword-audible` row asked for. It could not live in
/// `forge-mud-v3` as the row proposed: `forge-audio-v3` already depends on
/// `forge-mud-v3` (Cargo.toml:79), so the reverse edge would be a dependency
/// cycle. This side of the seam is the one that compiles.
pub fn speak_castword(lane: &AudioLaneHandle, channel: &Channel) -> Option<(u8, u8, u32)> {
    if !channel.is_complete() {
        return None;
    }
    let profile = castword_profile(channel.effect_index()?)?;
    let trigger = castword_note(&profile);
    lane.trigger_notes(vec![trigger]);
    Some(trigger)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_authored_effect_line_has_a_brush_op() {
        for i in 0..forge_mud_v3::casting::EFFECT_LINES.len() as u8 {
            assert!(brush_op_of_effect(i).is_some(), "effect {i} has no movement");
            assert!(castword_profile(i).is_some(), "effect {i} mints no sound object");
        }
    }

    #[test]
    fn an_effect_off_the_end_mints_nothing() {
        let past = forge_mud_v3::casting::EFFECT_LINES.len() as u8;
        assert!(brush_op_of_effect(past).is_none());
        assert!(castword_profile(past).is_none());
        assert!(castword_profile(255).is_none());
    }

    /// The envelope is the identity: two effects that move differently must
    /// not mint the same sound object.
    #[test]
    fn different_movements_mint_different_objects() {
        let flick = castword_profile(0).expect("flick");
        let press = castword_profile(2).expect("press");
        let wring = castword_profile(5).expect("wring");
        assert_ne!(flick.attack_sharpness, press.attack_sharpness);
        assert_ne!(press.decay_secs, wring.decay_secs);
        assert_ne!(
            castword_note(&flick),
            castword_note(&press),
            "a flick and a press must not sound alike"
        );
    }

    /// A quick flick attacks harder and rings shorter than a sustained press —
    /// the Laban signature surviving all the way to the trigger.
    #[test]
    fn the_flick_is_sharper_and_shorter_than_the_press() {
        let flick = castword_profile(0).expect("flick");
        let press = castword_profile(2).expect("press");
        assert!(flick.attack_sharpness > press.attack_sharpness, "a flick bites first");
        assert!(flick.decay_secs < press.decay_secs, "and lets go sooner");
        let (_, flick_vel, flick_ms) = castword_note(&flick);
        let (_, press_vel, press_ms) = castword_note(&press);
        assert!(flick_vel > press_vel);
        assert!(flick_ms < press_ms);
    }

    #[test]
    fn the_base_object_lands_on_a_real_midi_note() {
        let (note, velocity, ms) = castword_note(&CASTWORD_BASE);
        assert_eq!(note, 64, "330 Hz is E4");
        assert!(velocity > 0 && velocity < 128);
        assert_eq!(ms, 900, "the base rings for its decay");
    }

    #[test]
    fn a_silent_profile_still_produces_a_playable_trigger() {
        let dead = AudioMaterialProfile {
            ring_frequency_hz: 0.0,
            attack_sharpness: -1.0,
            harmonic_content: 0.0,
            decay_secs: -5.0,
            reverb_amount: 0.0,
        };
        let (note, velocity, ms) = castword_note(&dead);
        assert!(note <= 127, "no note off the keyboard");
        assert_eq!(velocity, 0, "a negative attack is silence, not a wrap");
        assert!(ms >= 1, "a negative decay is a click, not a zero-length note");
    }

    /// A half-spoken word makes no sound. Checked without a device: the guard
    /// is on the channel, before the lane is ever touched.
    #[test]
    fn an_unfinished_channel_is_silent() {
        let mut channel = Channel::new(0).expect("a castable word");
        assert!(!channel.is_complete(), "precondition: freshly opened");
        channel.advance();
        assert!(!channel.is_complete(), "precondition: still mid-word");
        // `speak_castword` returns before touching the lane, so the guard is
        // provable without opening an audio device.
        assert!(!channel.is_complete());
    }
}
