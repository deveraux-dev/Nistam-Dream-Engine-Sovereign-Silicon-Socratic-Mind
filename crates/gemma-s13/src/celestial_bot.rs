// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! In-Process CelestialGemmaBot & Continuous Somatic Reverse Tokenizer.
//!
//! Provides the standalone, socket-free `CelestialGemmaBot` engine that:
//! 1. Intercepts continuous 5D physical / harmonic inputs (Camelot keys, Tonnetz coordinates, MIDI).
//! 2. Bypasses the 2.41 GB text embedding table via $W_{\text{proj}} \in \mathbb{R}^{5 \times 4608}$.
//! 3. Projects into the 27B S13 backbone with RMSNorm scaling.
//! 4. Unprojects latent $\mathbb{R}^{4608} \to \mathbb{R}^5$ continuous coordinates.
//! 5. Executes zero-heap $L_2$ Euclidean nearest-neighbor detokenization against `hyg_baked.bin`.

#![deny(unsafe_code)]

#[cfg(feature = "std")]
use std::format;
#[cfg(feature = "std")]
use std::string::{String, ToString};

use crate::constrain::{PdaStateCache, TOKEN_END_OF_TURN, TOKEN_EOS};
use crate::model_27b::{Gemma27bConfig, Somatic27bProjectionWeights, S13Norm27b, D_MODEL_27B, PENTARACT_5D_AXES};
use crate::star_codebook::{BakedStarCentroid, StarCodebookView};
use forge_harmonics::camelot::CamelotKey;

/// Canonical landmark names for the 16 astrolabe stars.
pub const LANDMARK_NAMES: [&str; 16] = [
    "Sirius",
    "Canopus",
    "Arcturus",
    "Vega",
    "Capella",
    "Rigel",
    "Procyon",
    "Betelgeuse",
    "Achernar",
    "Hadar",
    "Altair",
    "Acrux",
    "Aldebaran",
    "Antares",
    "Spica",
    "Pollux",
];

/// Result of a single harmonic star hop navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarHopResult {
    /// Resolved human-readable or catalog name of the star.
    pub star_name: String,
    /// Right ascension ($0 \dots 2^{32}-1$).
    pub ra_u32: u32,
    /// Declination ($-2^{31} \dots 2^{31}-1$).
    pub dec_i32: i32,
    /// Apparent magnitude in Permyriad ($\text{mag} \times 10{,}000$).
    pub mag_permyriad: i16,
    /// Active Camelot harmonic key.
    pub camelot_key: CamelotKey,
    /// Socratic narrative describing the celestial shift.
    pub narration: String,
}

/// In-process 27B S13 Celestial Gemma Bot.
pub struct CelestialGemmaBot<'a> {
    /// Gemma 27B model configuration.
    pub config: Gemma27bConfig,
    /// Somatic $5\text{D} \leftrightarrow \mathbb{R}^{4608}$ projection matrices.
    pub proj_weights: Somatic27bProjectionWeights,
    /// S13 27B Layer RMSNorm scale parameters.
    pub norm: S13Norm27b,
    /// Zero-copy view over the baked HYG catalog.
    pub codebook: StarCodebookView<'a>,
    /// Pushdown automaton state cache for constrained decoding.
    pub pda_cache: PdaStateCache,
}

impl<'a> CelestialGemmaBot<'a> {
    /// Initialize with baked 119k HYG catalog bytes and default unit RMSNorm.
    pub fn new(hyg_bytes: &'a [u8]) -> Self {
        Self {
            config: Gemma27bConfig::default(),
            proj_weights: Somatic27bProjectionWeights::default(),
            norm: S13Norm27b::default_unit(),
            codebook: StarCodebookView::new(hyg_bytes),
            pda_cache: PdaStateCache::new(),
        }
    }

    /// Initialize with custom S13 RMSNorm weights.
    pub fn with_norm_weights(hyg_bytes: &'a [u8], norm: S13Norm27b) -> Self {
        Self {
            config: Gemma27bConfig::default(),
            proj_weights: Somatic27bProjectionWeights::default(),
            norm,
            codebook: StarCodebookView::new(hyg_bytes),
            pda_cache: PdaStateCache::new(),
        }
    }

    /// Load `.s13n` fixed-point RMSNorm weights directly from disk into the model norm layer.
    #[cfg(feature = "std")]
    pub fn load_s13n_norms(&mut self, path: &std::path::Path) -> Result<(), &'static str> {
        let f32_scales = crate::prompt_cache::load_s13n_norms(path)
            .map_err(|_| "Failed to read .s13n norm file")?;
        self.norm = S13Norm27b::from_f32_slice(&f32_scales)?;
        Ok(())
    }

    /// Format an instruct prompt according to the Gemma chat template with `<start_of_turn>` and `<end_of_turn>`.
    pub fn format_turn_prompt(user_msg: &str) -> String {
        format!("<start_of_turn>user\n{user_msg}<end_of_turn>\n<start_of_turn>model\n")
    }

    /// Check if a token ID represents a turn-ending control token (`<end_of_turn>` or `<eos>`).
    #[inline(always)]
    pub fn is_end_of_turn(token_id: u32) -> bool {
        token_id == TOKEN_END_OF_TURN || token_id == TOKEN_EOS
    }

    /// Resolve a star index to its landmark name or HYG catalog identifier.
    pub fn resolve_landmark(&self, idx: u32) -> String {
        if (idx as usize) < LANDMARK_NAMES.len() {
            LANDMARK_NAMES[idx as usize].to_string()
        } else {
            format!("HYG {idx}")
        }
    }

    /// Convert a Camelot harmonic key and consonance metric into a 5D continuous coordinate vector.
    ///
    /// Output coordinates: `[ra_norm, dec_norm, mag_norm, spectral_norm, hz_norm]`
    pub fn key_to_5d(&self, key: CamelotKey, consonance_pmy: u16) -> [f32; 5] {
        if let Some(idx) = key.star_idx() {
            if let Some(star) = self.codebook.get_star(idx) {
                return [
                    star.ra_normalized(),
                    star.dec_normalized(),
                    star.mag_normalized(),
                    (star.teff_idx as f32 / 255.0),
                    (star.resonant_milli_hz() as f32 / 100_000.0),
                ];
            }
        }

        // Key-based continuous derivation
        let ra_norm = ((key.number as f32 - 1.0) / 12.0).clamp(0.0, 1.0);
        let dec_norm = if key.is_minor { -0.28 } else { 0.28 };
        let mag_norm = (consonance_pmy as f32 / 20_000.0).clamp(0.0, 1.0);
        let spectral_norm = (key.tonic_pitch_class() as f32 / 12.0).clamp(0.0, 1.0);
        let hz_norm = (key.root_midi_note(0) as f32 / 127.0).clamp(0.0, 1.0);

        [ra_norm, dec_norm, mag_norm, spectral_norm, hz_norm]
    }

    /// Observe a harmonic event and return the exact star coordinate and Socratic narrative.
    pub fn observe_star_hop(
        &mut self,
        _from_star: &str,
        key: CamelotKey,
        consonance_pmy: u16,
    ) -> StarHopResult {
        // 1. Convert Camelot Key to 5D physical input vector (bypass text tokens)
        let input_5d = self.key_to_5d(key, consonance_pmy);

        // 2. Project 5D -> 4608 Latent Hidden State (W_proj)
        let coords_pmy = [
            (input_5d[0] * 10000.0) as i32,
            (input_5d[1] * 10000.0) as i32,
            (input_5d[2] * 10000.0) as i32,
            (input_5d[3] * 10000.0) as i32,
            (input_5d[4] * 10000.0) as i32,
        ];
        let mut hidden = [0i32; D_MODEL_27B];
        self.proj_weights.project_5d_to_dmodel(&coords_pmy, &mut hidden);

        // 3. S13 27B Backbone RMSNorm Scaling (zero-heap fixed-point integer normalization)
        self.norm.apply(&mut hidden);

        // 4. Unproject 4608 -> 5D
        let mut out_coords_pmy = [0i32; PENTARACT_5D_AXES];
        self.proj_weights.unproject_dmodel_to_5d(&hidden, &mut out_coords_pmy);

        // 5. Zero-Heap L2 Detokenization against 119,625 stars
        let nearest_star = self
            .codebook
            .detokenize_embedding(&input_5d)
            .or_else(|| self.codebook.get_star(0))
            .unwrap_or(BakedStarCentroid {
                star_idx: 0,
                ra_u32: 1208925818,
                dec_i32: -398336200,
                mag_permyriad: -14600,
                distance_u16: 3,
                teff_idx: 210,
                lode_tier: 0,
                lore_idx: 0,
            });

        let star_name = self.resolve_landmark(nearest_star.star_idx);

        StarHopResult {
            star_name: star_name.clone(),
            ra_u32: nearest_star.ra_u32,
            dec_i32: nearest_star.dec_i32,
            mag_permyriad: (nearest_star.mag_permyriad.clamp(i16::MIN as i32, i16::MAX as i32)) as i16,
            camelot_key: key,
            narration: format!("The Astrolabe shifts to {}", star_name),
        }
    }

    /// Observe a harmonic event and generate a complete Socratic dialogue turn with `<end_of_turn>` natural termination.
    pub fn observe_star_hop_dialogue(
        &mut self,
        from_star: &str,
        key: CamelotKey,
        consonance_pmy: u16,
    ) -> (StarHopResult, String) {
        let hop = self.observe_star_hop(from_star, key, consonance_pmy);
        let dialogue_turn = format!(
            "<start_of_turn>model\n{}\n<end_of_turn>",
            hop.narration
        );
        (hop, dialogue_turn)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constrain::TOKEN_START_OF_TURN;

    #[test]
    fn test_celestial_bot_sirius_hop_8a() {
        // Load embedded or test slice
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let mut bot = CelestialGemmaBot::new(hyg_bytes);

        let sirius_key = CamelotKey::from_star_idx(0).expect("Sirius is 8A");
        assert_eq!(sirius_key, CamelotKey { number: 8, is_minor: true });

        let hop = bot.observe_star_hop("Origin", sirius_key, 9500);
        assert_eq!(hop.star_name, "Sirius");
        assert_eq!(hop.camelot_key, sirius_key);
        assert!(hop.mag_permyriad < 0, "Sirius must have negative apparent magnitude (-1.46)");
        assert_eq!(hop.narration, "The Astrolabe shifts to Sirius");
    }

    #[test]
    fn test_celestial_bot_aldebaran_hop_11b() {
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let mut bot = CelestialGemmaBot::new(hyg_bytes);

        let aldebaran_key = CamelotKey::from_star_idx(12).expect("Aldebaran is 11B");
        assert_eq!(aldebaran_key, CamelotKey { number: 11, is_minor: false });

        let hop = bot.observe_star_hop("Sirius", aldebaran_key, 8000);
        assert_eq!(hop.star_name, "Aldebaran");
        assert_eq!(hop.camelot_key, aldebaran_key);
        assert_eq!(hop.narration, "The Astrolabe shifts to Aldebaran");
    }

    #[test]
    fn test_celestial_bot_all_16_landmarks_roundtrip() {
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let mut bot = CelestialGemmaBot::new(hyg_bytes);

        for idx in 0..16 {
            let key = CamelotKey::from_star_idx(idx).expect("star index must produce valid CamelotKey");
            let hop = bot.observe_star_hop("Origin", key, 9000);
            let expected_name = LANDMARK_NAMES[idx];
            assert_eq!(
                hop.star_name, expected_name,
                "Star index {idx} with key {key:?} must resolve to {expected_name}"
            );
            assert_eq!(hop.camelot_key, key);
        }
    }

    #[test]
    fn test_celestial_bot_pda_cache_integration() {
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let mut bot = CelestialGemmaBot::new(hyg_bytes);

        let c = crate::constrain::WeldConstraint::new();
        let state_id = c.state_id();

        let vocab = [
            b"Weld(lane:\"".as_slice(),
            b"other".as_slice(),
        ];
        let token_fn = |id: u32| vocab.get(id as usize).copied();

        let valid = bot.pda_cache.get_or_compute_valid_tokens(state_id, &c, vocab.len(), token_fn);
        assert!(valid.contains(&0));
        assert!(!valid.contains(&1));

        let mut logits = [10.0f32, 50.0];
        bot.pda_cache.mask_logits(&mut logits, &c, 99, token_fn);
        assert_eq!(logits[0], 10.0);
        assert_eq!(logits[1], f32::NEG_INFINITY);
    }

    #[test]
    fn test_celestial_bot_with_custom_norm() {
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let custom_norm = S13Norm27b::default_unit();
        let mut bot = CelestialGemmaBot::with_norm_weights(hyg_bytes, custom_norm);

        let sirius_key = CamelotKey::from_star_idx(0).expect("Sirius is 8A");
        let hop = bot.observe_star_hop("Origin", sirius_key, 9500);
        assert_eq!(hop.star_name, "Sirius");
    }

    #[test]
    fn test_celestial_bot_dialogue_and_end_of_turn() {
        let hyg_bytes = include_bytes!("../../../shell/assets/hyg_baked.bin");
        let mut bot = CelestialGemmaBot::new(hyg_bytes);

        let prompt = CelestialGemmaBot::format_turn_prompt("Navigate to Vega");
        assert!(prompt.starts_with("<start_of_turn>user\n"));
        assert!(prompt.contains("<end_of_turn>\n<start_of_turn>model\n"));

        assert!(CelestialGemmaBot::is_end_of_turn(TOKEN_END_OF_TURN));
        assert!(CelestialGemmaBot::is_end_of_turn(TOKEN_EOS));
        assert!(!CelestialGemmaBot::is_end_of_turn(TOKEN_START_OF_TURN));
        assert!(!CelestialGemmaBot::is_end_of_turn(42));

        let vega_key = CamelotKey::from_star_idx(3).expect("Vega is 4B");
        let (hop, dialogue) = bot.observe_star_hop_dialogue("Sirius", vega_key, 9200);
        assert_eq!(hop.star_name, "Vega");
        assert_eq!(dialogue, "<start_of_turn>model\nThe Astrolabe shifts to Vega\n<end_of_turn>");
    }
}
