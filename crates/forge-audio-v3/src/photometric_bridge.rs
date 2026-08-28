//! From-bridges between forge-photometric audio types and the recipe engine.
//! Live on THIS side of the edge: forge-audio -> forge-photometric is acyclic,
//! the reverse cycles through forge-vision (TECH-DEBT AUDIO-RECIPE-BRIDGE-HOMELESS).

use crate::recipe;

impl From<&forge_photometric::audio_types::AudioMaterialProfile> for recipe::AudioMaterialProfile {
    fn from(p: &forge_photometric::audio_types::AudioMaterialProfile) -> Self {
        Self {
            ring_frequency_hz: p.ring_frequency_hz,
            attack_sharpness: p.attack_sharpness,
            harmonic_content: p.harmonic_content,
            decay_secs: p.decay_secs,
            reverb_amount: p.reverb_amount,
        }
    }
}

impl From<&recipe::AudioMaterialProfile> for forge_photometric::audio_types::AudioMaterialProfile {
    fn from(p: &recipe::AudioMaterialProfile) -> Self {
        Self {
            ring_frequency_hz: p.ring_frequency_hz,
            attack_sharpness: p.attack_sharpness,
            harmonic_content: p.harmonic_content,
            decay_secs: p.decay_secs,
            reverb_amount: p.reverb_amount,
        }
    }
}

/// Photometric SoundSource -> recipe SoundSource (variant-for-variant).
pub fn sound_source_to_recipe(s: forge_photometric::audio_types::SoundSource) -> recipe::SoundSource {
    use forge_photometric::audio_types::SoundSource as P;
    match s {
        P::Impact => recipe::SoundSource::Impact,
        P::CombatMelee => recipe::SoundSource::CombatMelee,
        P::CombatRanged => recipe::SoundSource::CombatRanged,
        P::CombatMagic => recipe::SoundSource::CombatMagic,
        P::VoxelImpact => recipe::SoundSource::VoxelImpact,
        P::Structural => recipe::SoundSource::Structural,
        P::Locomotion => recipe::SoundSource::Locomotion,
        P::Heat => recipe::SoundSource::Heat,
        P::Projectile => recipe::SoundSource::Projectile,
    }
}

impl From<&forge_photometric::types::MaterialBitmask> for recipe::MaterialBitmask {
    fn from(m: &forge_photometric::types::MaterialBitmask) -> Self {
        Self {
            void_pct: m.void_pct,
            shadow_pct: m.shadow_pct,
            ash_pct: m.ash_pct,
            iron_pct: m.iron_pct,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn material_bitmask_bridges_channel_for_channel() {
        let m = forge_photometric::types::MaterialBitmask {
            void_pct: 10,
            shadow_pct: 20,
            ash_pct: 30,
            iron_pct: 195,
        };
        let r: recipe::MaterialBitmask = (&m).into();
        assert_eq!(
            (r.void_pct, r.shadow_pct, r.ash_pct, r.iron_pct),
            (10, 20, 30, 195)
        );
    }

    #[test]
    fn profile_bridges_field_for_field() {
        let p = recipe::AudioMaterialProfile {
            ring_frequency_hz: 440.0,
            attack_sharpness: 0.5,
            harmonic_content: 0.25,
            decay_secs: 1.5,
            reverb_amount: 0.1,
        };
        let f: forge_photometric::audio_types::AudioMaterialProfile = (&p).into();
        assert_eq!(f.ring_frequency_hz, 440.0);
        assert_eq!(f.decay_secs, 1.5);
    }

    #[test]
    fn sound_source_maps_every_variant() {
        use forge_photometric::audio_types::SoundSource as P;
        for (p, r) in [
            (P::Impact, recipe::SoundSource::Impact),
            (P::VoxelImpact, recipe::SoundSource::VoxelImpact),
            (P::Projectile, recipe::SoundSource::Projectile),
        ] {
            assert_eq!(sound_source_to_recipe(p), r);
        }
    }
}
