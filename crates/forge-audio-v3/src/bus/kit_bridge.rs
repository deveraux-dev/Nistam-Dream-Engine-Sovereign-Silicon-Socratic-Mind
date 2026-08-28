//! VixiScript kit-binding → MixerCommand translator (G-AUDIO-03 wire).
//!
//! `outbox/G-AUDIO-03/WIRING-SPEC.md`: the `mixer_channel_strip.kit.vixi` surface
//! declares bind-strings (`mixer.channel_volume`, `mixer.channel_pan`, ...) but had
//! no runtime bridge into `MixerCommand`. This is that bridge — a pure, testable
//! translator plus `MixerCommandHub::apply_kit_binding` as the one seam a future
//! VixiKit runtime dispatcher calls per widget event.

use super::command::MixerCommand;

/// Map a `.kit.vixi` bind-string + raw Permyriad widget value to a `MixerCommand`.
///
/// `value_permyriad`: unipolar `0..=10000` for volume/rms-style widgets; bipolar
/// `-10000..=10000` (center 0) for pan/crossfader — the caller (widget runtime)
/// is responsible for the unipolar→bipolar remap since that's a widget-kind
/// property (slider vs. dial), not something this translator can infer from the
/// bind-string alone.
pub fn resolve_kit_binding(binding: &str, value_permyriad: i32, deck: usize) -> Option<MixerCommand> {
    match binding {
        "mixer.channel_volume" => Some(MixerCommand::SetVolume {
            deck,
            volume: (value_permyriad as f32 / 10_000.0).clamp(0.0, 1.0),
        }),
        "mixer.master_volume" => Some(MixerCommand::SetMasterVolume {
            volume: (value_permyriad as f32 / 10_000.0).clamp(0.0, 1.0),
        }),
        "mixer.channel_pan" => Some(MixerCommand::SetPan {
            deck,
            pan: (value_permyriad as f32 / 10_000.0).clamp(-1.0, 1.0),
        }),
        "mixer.crossfader" => Some(MixerCommand::SetCrossfader {
            position: (value_permyriad as f32 / 10_000.0).clamp(-1.0, 1.0),
        }),
        "mixer.channel_mute" => Some(MixerCommand::ToggleMute { deck }),
        "mixer.channel_solo" => Some(MixerCommand::ToggleSolo { deck }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn volume_binding_maps_and_scales() {
        match resolve_kit_binding("mixer.channel_volume", 7500, 2) {
            Some(MixerCommand::SetVolume { deck, volume }) => {
                assert_eq!(deck, 2);
                assert!((volume - 0.75).abs() < 1e-6);
            }
            other => panic!("expected SetVolume, got {other:?}"),
        }
    }

    #[test]
    fn pan_binding_is_bipolar() {
        match resolve_kit_binding("mixer.channel_pan", -5000, 0) {
            Some(MixerCommand::SetPan { pan, .. }) => assert!((pan + 0.5).abs() < 1e-6),
            other => panic!("expected SetPan, got {other:?}"),
        }
    }

    #[test]
    fn mute_and_solo_bindings_ignore_value() {
        assert!(matches!(
            resolve_kit_binding("mixer.channel_mute", 0, 1),
            Some(MixerCommand::ToggleMute { deck: 1 })
        ));
        assert!(matches!(
            resolve_kit_binding("mixer.channel_solo", 0, 3),
            Some(MixerCommand::ToggleSolo { deck: 3 })
        ));
    }

    #[test]
    fn unknown_binding_is_none() {
        assert!(resolve_kit_binding("mixer.channel_rms", 5000, 0).is_none());
    }

    #[test]
    fn apply_kit_binding_drives_a_real_hub() {
        // The live-caller proof: one bind-string + Permyriad value flows all the
        // way through MixerCommandHub state, no manual MixerCommand construction.
        let mut hub = super::super::mixer::MixerCommandHub::new();
        assert!(hub.apply_kit_binding("mixer.channel_volume", 3000, 1));
        assert!((hub.build_snapshot().decks[1].volume - 0.3).abs() < 1e-6);

        assert!(hub.apply_kit_binding("mixer.channel_mute", 0, 1));
        assert!(hub.build_snapshot().decks[1].muted);

        assert!(!hub.apply_kit_binding("mixer.channel_rms", 0, 1), "read-only binding must not apply");
    }
}
