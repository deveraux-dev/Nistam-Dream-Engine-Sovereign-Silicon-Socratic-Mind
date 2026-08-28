//! OPUS RAMP LAW lint (alchemy.md:36). Reading stop1→stop4 as
//! nigredo→albedo→citrinitas→rubedo: check (a) value(stop2)>value(stop1)
//! whitening, (b) hue(stop3) in yellow band with chroma peak, (c) hue(stop4)
//! trending red vs stop3 with chroma≥stop3 saturation.

use crate::trit::ColourTrit8;
use crate::{from_oklch, rgb8_to_oklch};

/// One stop in a four-stop ramp (a ColourTrit8 color without position).
/// Distinct from forge-book-v3's Stop, which carries a `t_pmy` position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RampStop {
    /// The colour at this ramp position in Munsell space.
    pub colour: ColourTrit8,
}

/// Opus-ramp alchemical tier registry — correlates to resonance in
/// alchemical.toml. nigredo_stone: 40 Hz, albedo_waterstone: 432 Hz,
/// citrinitas_inverse_root: 408 Hz, rubedo_iron_ash: 800 Hz, void_glass:
/// 963 Hz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpusAlchemicalTier {
    /// Nigredo (black). Resonance 40 Hz, material basalt_dense.
    Nigredo,
    /// Albedo (white). Resonance 432 Hz, material limestone_wet.
    Albedo,
    /// Citrinitas (yellow). Resonance 408 Hz, material rootbound_loam.
    Citrinitas,
    /// Rubedo (red). Resonance 800 Hz, material ferric_slag.
    Rubedo,
    /// Void. Resonance 963 Hz, material void_containment.
    Void,
}

impl OpusAlchemicalTier {
    /// Resonance frequency in Hz (drained from alchemical.toml).
    pub const fn resonance_hz(&self) -> u32 {
        match self {
            Self::Nigredo => 40,
            Self::Albedo => 432,
            Self::Citrinitas => 408,
            Self::Rubedo => 800,
            Self::Void => 963,
        }
    }
}

/// Violation of the OPUS RAMP LAW, typed per clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpusBreach {
    /// Clause (a): value(stop2) is not > value(stop1) — no whitening.
    NoWhitening,
    /// Clause (b): hue(stop3) not in yellow band [8,11] or no local chroma
    /// peak.
    NoCitrinitas,
    /// Clause (c): hue(stop4) not trending red vs stop3, or
    /// chroma(stop4)<chroma(stop3).
    NoRubedo,
}

/// Build a four-stop ramp from sRGB byte triples. MODELING DECISION: Slot
/// mapping to stops is fixed here (a judgement call, not a drained fact). This
/// is the ONLY place the mapping is defined; change it here to re-lint.
/// - stop1 (nigredo):  rgb[0] — dark background.
/// - stop2 (albedo):   rgb[1] — light background.
/// - stop3 (citrinitas): rgb[2] — accent/yellow stop.
/// - stop4 (rubedo):   rgb[3] — warning/red stop.
pub fn ramp_from_rgb8(rgb: &[u8; 12]) -> [RampStop; 4] {
    let mut stops = [RampStop { colour: ColourTrit8::WHITE }; 4];
    for i in 0..4 {
        let r = rgb[i * 3];
        let g = rgb[i * 3 + 1];
        let b = rgb[i * 3 + 2];
        let oklch = rgb8_to_oklch(r, g, b);
        stops[i].colour = from_oklch(oklch);
    }
    stops
}

/// Check a four-stop ramp against the OPUS RAMP LAW (alchemy.md:36).
/// Returns Ok(()) if all clauses pass, or Err(breach) naming the first
/// violation.
///
/// The law, reading stop1→stop4 as nigredo→albedo→citrinitas→rubedo:
/// (a) value(stop2) > value(stop1) — the whitening.
/// (b) hue(stop3) in yellow band with local chroma peak — the citrinitas.
/// (c) hue(stop4) trending toward red vs stop3, chroma(stop4)≥chroma(stop3)
/// — the rubedo.
pub fn opus_ramp_verdict(stops: &[RampStop; 4]) -> Result<(), OpusBreach> {
    // Clause (a): value(stop2) > value(stop1).
    if stops[1].colour.value_pmy <= stops[0].colour.value_pmy {
        return Err(OpusBreach::NoWhitening);
    }

    // Clause (b): hue(stop3) in yellow band [8, 11] with local chroma peak.
    // Munsell yellow (Y) occupies hue indices 8-11.
    let hue3 = stops[2].colour.hue_idx;
    let chroma3 = stops[2].colour.chroma_pmy;
    let in_yellow = hue3 >= 8 && hue3 <= 11;
    let chroma_peak =
        chroma3 >= stops[0].colour.chroma_pmy && chroma3 >= stops[1].colour.chroma_pmy;
    if !in_yellow || !chroma_peak {
        return Err(OpusBreach::NoCitrinitas);
    }

    // Clause (c): hue(stop4) in red band [0-3, 36-39], trending hue4<hue3,
    // with chroma(stop4)>=chroma(stop3).
    // Munsell red (R) is 0-3; RP (red-purple, wrapping red) is 36-39.
    let hue4 = stops[3].colour.hue_idx;
    let chroma4 = stops[3].colour.chroma_pmy;
    let hue4_in_red = hue4 < 4 || hue4 >= 36;
    let hue_trend_to_red = hue4 < hue3;
    let chroma_rises = chroma4 >= chroma3;

    if !hue4_in_red || !hue_trend_to_red || !chroma_rises {
        return Err(OpusBreach::NoRubedo);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clause_a_fails_when_value_not_rising() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            }, // Not > stop1.
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 5000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 7000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            },
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoWhitening));
    }

    #[test]
    fn clause_a_passes_when_value_rises() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            }, // > stop1.
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            },
        ];
        assert_ne!(opus_ramp_verdict(&stops), Err(OpusBreach::NoWhitening));
    }

    #[test]
    fn clause_b_fails_when_hue_not_yellow() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 20,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Hue 20 (blue), not yellow [8-11].
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            },
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoCitrinitas));
    }

    #[test]
    fn clause_b_fails_when_no_chroma_peak() {
        let stops = [
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 2000,
                    chroma_pmy: 7000,
                    tags: [0; 2],
                },
            }, // High chroma at stop1.
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Lower chroma at stop3 — no peak.
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            },
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoCitrinitas));
    }

    #[test]
    fn clause_b_passes_when_yellow_and_peaked() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Yellow, peak among stops 0-2.
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            },
        ];
        assert_ne!(opus_ramp_verdict(&stops), Err(OpusBreach::NoCitrinitas));
    }

    #[test]
    fn clause_c_fails_when_hue_not_red() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 20,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            }, // Hue 20 not in red [0-3, 36-39].
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoRubedo));
    }

    #[test]
    fn clause_c_fails_when_hue_not_trending() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Hue 10 in yellow band.
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 15,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            }, // Hue 15 > hue 10 — not trending toward red.
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoRubedo));
    }

    #[test]
    fn clause_c_fails_when_chroma_not_rising() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            },
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 7000,
                    tags: [0; 2],
                },
            },
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Chroma 5000 < 7000 — not rising.
        ];
        assert_eq!(opus_ramp_verdict(&stops), Err(OpusBreach::NoRubedo));
    }

    #[test]
    fn full_ramp_passes_all_clauses() {
        let stops = [
            RampStop {
                colour: ColourTrit8::achromatic(2000),
            }, // Nigredo: dark.
            RampStop {
                colour: ColourTrit8::achromatic(7000),
            }, // Albedo: lighter (clause a).
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 10,
                    alpha_flag: 1,
                    value_pmy: 8000,
                    chroma_pmy: 5000,
                    tags: [0; 2],
                },
            }, // Citrinitas: yellow, chroma peak (clause b).
            RampStop {
                colour: ColourTrit8 {
                    hue_idx: 2,
                    alpha_flag: 1,
                    value_pmy: 9000,
                    chroma_pmy: 6000,
                    tags: [0; 2],
                },
            }, // Rubedo: red, hue<stop3, chroma≥stop3 (clause c).
        ];
        assert_eq!(opus_ramp_verdict(&stops), Ok(()));
    }

    #[test]
    fn profile_studio_dark_real_ramp() {
        // studio_dark palette: RGB [#060609, #0A0A0E, #D4A843, #D43535].
        // Mapping: bg_far→stop1, bg_near→stop2, accent_primary→stop3,
        // warning_danger→stop4. Real conversion via OklCh intermediate.
        let rgb = [
            0x06, 0x06, 0x09, // bg_far #060609
            0x0A, 0x0A, 0x0E, // bg_near #0A0A0E
            0xD4, 0xA8, 0x43, // accent_primary #D4A843
            0xD4, 0x35, 0x35, // warning_danger #D43535
        ];
        let stops = ramp_from_rgb8(&rgb);
        let result = opus_ramp_verdict(&stops);
        assert_eq!(result, Ok(()), "studio_dark verdict should pass all clauses");
    }

    #[test]
    fn profile_studio_light_real_ramp() {
        // studio_light palette: RGB [#E3D9C0, #F0E7D4, #E8843C, #E23B22].
        // Mapping: bg_far→stop1, bg_near→stop2, accent_primary→stop3,
        // warning_danger→stop4. Real conversion via OklCh intermediate.
        let rgb = [
            0xE3, 0xD9, 0xC0, // bg_far #E3D9C0
            0xF0, 0xE7, 0xD4, // bg_near #F0E7D4
            0xE8, 0x84, 0x3C, // accent_primary #E8843C (orange)
            0xE2, 0x3B, 0x22, // warning_danger #E23B22
        ];
        let stops = ramp_from_rgb8(&rgb);
        let result = opus_ramp_verdict(&stops);
        // studio_light's accent_primary (#E8843C) is orange, not pure yellow,
        // so it should breach clause (b).
        assert_eq!(result, Err(OpusBreach::NoCitrinitas), "studio_light: accent is orange (YR), not yellow");
    }

    #[test]
    fn profile_molten_real_ramp() {
        // molten palette: RGB [#0A0705, #1A0F09, #FF6A1A, #FFD54A].
        // Mapping: bg_far→stop1, bg_near→stop2, accent_primary→stop3,
        // warning_danger→stop4. Real conversion via OklCh intermediate.
        let rgb = [
            0x0A, 0x07, 0x05, // bg_far #0A0705
            0x1A, 0x0F, 0x09, // bg_near #1A0F09
            0xFF, 0x6A, 0x1A, // accent_primary #FF6A1A (orange)
            0xFF, 0xD5, 0x4A, // warning_danger #FFD54A (yellow)
        ];
        let stops = ramp_from_rgb8(&rgb);
        let result = opus_ramp_verdict(&stops);
        // molten's accent (#FF6A1A) is orange and warning (#FFD54A) is bright
        // yellow, but the law expects stop3 in yellow and stop4 in red, so both
        // should breach the law (either stop3 not yellow, or stop4 not red).
        assert!(result.is_err(), "molten: accent/warning hues don't follow law progression");
    }

    #[test]
    fn profile_permafrost_real_ramp() {
        // permafrost palette: RGB [#05090F, #0C1622, #3BC7FF, #CFF2FF].
        // Mapping: bg_far→stop1, bg_near→stop2, accent_primary→stop3,
        // warning_danger→stop4. Real conversion via OklCh intermediate.
        let rgb = [
            0x05, 0x09, 0x0F, // bg_far #05090F
            0x0C, 0x16, 0x22, // bg_near #0C1622
            0x3B, 0xC7, 0xFF, // accent_primary #3BC7FF (cyan)
            0xCF, 0xF2, 0xFF, // warning_danger #CFF2FF (light cyan/white)
        ];
        let stops = ramp_from_rgb8(&rgb);
        let result = opus_ramp_verdict(&stops);
        // permafrost's accent (#3BC7FF) is cyan (blue), not yellow, so it should
        // breach clause (b).
        assert_eq!(result, Err(OpusBreach::NoCitrinitas), "permafrost: accent is cyan, not yellow");
    }

    #[test]
    #[allow(dead_code)]
    fn diagnostic_print_palette_conversions() {
        // Debug helper: print actual Munsell values for all four palettes.
        // This test is always marked pass and is here for manual inspection.
        eprintln!("\n=== OPUS RAMP CONVERSIONS (REAL PALETTES) ===\n");

        let palettes = [
            ("studio_dark", [
                [0x06u8, 0x06, 0x09], [0x0A, 0x0A, 0x0E],
                [0xD4, 0xA8, 0x43], [0xD4, 0x35, 0x35],
            ]),
            ("studio_light", [
                [0xE3u8, 0xD9, 0xC0], [0xF0, 0xE7, 0xD4],
                [0xE8, 0x84, 0x3C], [0xE2, 0x3B, 0x22],
            ]),
            ("molten", [
                [0x0Au8, 0x07, 0x05], [0x1A, 0x0F, 0x09],
                [0xFF, 0x6A, 0x1A], [0xFF, 0xD5, 0x4A],
            ]),
            ("permafrost", [
                [0x05u8, 0x09, 0x0F], [0x0C, 0x16, 0x22],
                [0x3B, 0xC7, 0xFF], [0xCF, 0xF2, 0xFF],
            ]),
        ];

        for (name, palette) in palettes.iter() {
            let mut rgb = [0u8; 12];
            for i in 0..4 {
                rgb[i * 3] = palette[i][0];
                rgb[i * 3 + 1] = palette[i][1];
                rgb[i * 3 + 2] = palette[i][2];
            }
            let stops = ramp_from_rgb8(&rgb);
            let verdict = opus_ramp_verdict(&stops);

            eprintln!("{}: {}", name, match verdict {
                Ok(()) => "✓ PASS".to_string(),
                Err(OpusBreach::NoWhitening) => "✗ NoWhitening".to_string(),
                Err(OpusBreach::NoCitrinitas) => "✗ NoCitrinitas".to_string(),
                Err(OpusBreach::NoRubedo) => "✗ NoRubedo".to_string(),
            });

            for (i, (rgb_triple, stop)) in palette.iter().zip(stops.iter()).enumerate() {
                let label = match i {
                    0 => "stop1(nigredo)",
                    1 => "stop2(albedo)",
                    2 => "stop3(citrinitas)",
                    3 => "stop4(rubedo)",
                    _ => "?",
                };
                eprintln!(
                    "  {}: #{:02X}{:02X}{:02X} → v={} h={} c={}",
                    label, rgb_triple[0], rgb_triple[1], rgb_triple[2],
                    stop.colour.value_pmy, stop.colour.hue_idx, stop.colour.chroma_pmy
                );
            }
            eprintln!();
        }
    }
}
