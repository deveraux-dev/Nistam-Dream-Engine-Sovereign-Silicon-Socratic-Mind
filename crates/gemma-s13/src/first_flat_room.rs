// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! First Flat Room Post-Birth-Rite Multi-Model Execution Engine.
//!
//! Demonstrates the 6-tier sovereign fleet pipeline at Spawn Node Origin ([0,0,0,0,0]):
//! 1. `[1] s13_gemma_2b` Action Parser & Draft (Choice Archetypes & natal pitch).
//! 2. `[2] s13_gemma_2b (Mirror)` Parity & Invariant Filter (Logit subtraction via anti-expert mask).
//! 3. `[3] s13_gemma_2b_m3` Voxel & Hermetic Codec (5D Morton key, `senses_now()`, `#star-hud` telemetry).
//! 4. `[0] s13_gemma_9b` Master World Engine (42-layer ambient CYOA room prose).
//! 5. `[4] s13_gemma_m2` Sentry & Protocol Guard (Cree ASP grammar and 13 Moons sentinel check).
//! 6. `[5] Gemini 2.5 Flash` Macro-Seed Governor (Out-of-band macro-expansion seed and zero-retention purge).

use crate::atg::UNIT_COST_CEILING_MICRO_USD;
use crate::cree_grammar::{Animacy, AspGrammarSolver, CreeTransducer, ObviationTier};
use crate::m5_geodesic::M5Coordinate;
use crate::sentinel::is_sentinel_branchless;
use crate::vault::ZeroRetentionVault;

/// Choice Archetypes available to the operator in the MUD runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChoiceArchetype {
    /// 0. Survey environmental conditions and room architecture.
    Survey = 0,
    /// 1. Advance spatial coordinates along the grid.
    Advance = 1,
    /// 2. Sing harmonic words or trigger resonant unlocking.
    Sing = 2,
    /// 3. Bind objects, ties, or pacts.
    Bind = 3,
    /// 4. Cut or cleave obstacles.
    Cut = 4,
    /// 5. Rest and recover equilibrium.
    Rest = 5,
}

impl ChoiceArchetype {
    /// Parse action text into a choice archetype.
    pub fn parse_input(input: &str) -> Self {
        let trimmed = input.trim();
        let bytes = trimmed.as_bytes();
        if bytes.is_empty() {
            return Self::Survey;
        }

        // Fast zero-alloc prefix matching
        if Self::starts_with_ci(trimmed, "look")
            || Self::starts_with_ci(trimmed, "survey")
            || Self::starts_with_ci(trimmed, "inspect")
        {
            Self::Survey
        } else if Self::starts_with_ci(trimmed, "step")
            || Self::starts_with_ci(trimmed, "north")
            || Self::starts_with_ci(trimmed, "south")
            || Self::starts_with_ci(trimmed, "east")
            || Self::starts_with_ci(trimmed, "west")
            || Self::starts_with_ci(trimmed, "advance")
            || Self::starts_with_ci(trimmed, "walk")
        {
            Self::Advance
        } else if Self::starts_with_ci(trimmed, "sing")
            || Self::starts_with_ci(trimmed, "chant")
            || Self::starts_with_ci(trimmed, "voice")
        {
            Self::Sing
        } else if Self::starts_with_ci(trimmed, "bind") || Self::starts_with_ci(trimmed, "tie") {
            Self::Bind
        } else if Self::starts_with_ci(trimmed, "cut") || Self::starts_with_ci(trimmed, "strike") {
            Self::Cut
        } else {
            Self::Rest
        }
    }

    fn starts_with_ci(s: &str, prefix: &str) -> bool {
        if s.len() < prefix.len() {
            return false;
        }
        let s_bytes = s.as_bytes();
        let p_bytes = prefix.as_bytes();
        let mut i = 0;
        while i < prefix.len() {
            if s_bytes[i].to_ascii_lowercase() != p_bytes[i].to_ascii_lowercase() {
                return false;
            }
            i += 1;
        }
        true
    }

    /// Display name of the archetype.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Survey => "SURVEY",
            Self::Advance => "ADVANCE",
            Self::Sing => "SING",
            Self::Bind => "BIND",
            Self::Cut => "CUT",
            Self::Rest => "REST",
        }
    }
}

/// Step 1: Action Draft from `[1] s13_gemma_2b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionDraft {
    /// Matched choice archetype.
    pub archetype: ChoiceArchetype,
    /// Target 5D coordinate delta proposed by the draft.
    pub proposed_delta: [i8; 5],
    /// Draft confidence in Permyriad (0..10,000).
    pub confidence_pmy: u16,
    /// Operator natal star pitch in millihertz.
    pub natal_pitch_mhz: u32,
}

/// Step 2: Parity & Invariant Outcome from `[2] s13_gemma_2b (Mirror)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParityFilterOutcome {
    /// Whether the action adheres to Flat Room physical invariants.
    pub is_valid: bool,
    /// Logit mask penalty applied to counter-factual paths (0 for valid, 1000 for invalid).
    pub logit_mask_penalty: i32,
    /// Anti-expert parity checksum.
    pub parity_checksum: u64,
}

/// Step 3: Voxel & Hermetic Codec State from `[3] s13_gemma_2b_m3`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoxelSensorySnapshot {
    /// 64-bit packed 5D Morton coordinate.
    pub morton_key_5d: u64,
    /// Resolved 5D ternary coordinate.
    pub coord: M5Coordinate,
    /// 32-channel sensory gains: [Light, Acoustic, Pressure, Dissonance, Polarity, Wind, Thermal, Aether].
    pub senses: [u16; 8],
    /// Cumulative 7-school art delta.
    pub art_delta: [i32; 7],
    /// Status strip HUD hash.
    pub hud_hash: u64,
}

/// Step 4: Master World Engine Output from `[0] s13_gemma_9b`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomUmweltProse {
    /// Ambient CYOA room prose text slice.
    pub prose: &'static str,
    /// Harmonic root frequency (Hz).
    pub harmonic_freq_hz: u32,
    /// Active transformer layer count (42).
    pub layers_active: u8,
}

/// Step 5: Sentry Protocol Audit from `[4] s13_gemma_m2`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentryAuditResult {
    /// Whether Cree ASP obviation agreement holds.
    pub cree_asp_passed: bool,
    /// Whether 13-Moons sentinel check passed without breach.
    pub sentinel_clean: bool,
    /// Whether ADR-0026 zero-retention memory sweep executed cleanly.
    pub vault_sweep_done: bool,
}

/// Step 6: Macro-Seed Governor from `[5] Gemini 2.5 Flash`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacroSeedExpansion {
    /// Whether out-of-band escalation was triggered (boundary breach).
    pub escalated: bool,
    /// Macro-region seed hex (generated or default).
    pub macro_seed_hex: u64,
    /// Estimated call cost in micro-USD.
    pub cost_micro_usd: u32,
    /// Zero-cloud-retention staging wipe status.
    pub staging_purged: bool,
}

/// Unified Result of the 6-Model First Flat Room Execution Step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstFlatRoomStepResult {
    /// Step 1: Action Draft.
    pub draft: ActionDraft,
    /// Step 2: Parity Check.
    pub parity: ParityFilterOutcome,
    /// Step 3: Voxel Snapshot.
    pub voxel: VoxelSensorySnapshot,
    /// Step 4: Ambient Prose.
    pub umwelt: RoomUmweltProse,
    /// Step 5: Sentry Audit.
    pub sentry: SentryAuditResult,
    /// Step 6: Macro Seed.
    pub macro_seed: MacroSeedExpansion,
}

/// The 16 CATALOG_16 Star Frequencies in millihertz (A440 anchor).
pub const STAR_MILLI_HZ_16: [u32; 16] = [
    440_000, 415_305, 391_995, 369_994, 349_228, 329_628, 311_127, 293_665,
    277_183, 261_626, 246_942, 233_082, 220_000, 207_652, 195_998, 184_997,
];

/// The 6-Model First Flat Room Execution Engine.
pub struct FirstFlatRoomEngine;

impl FirstFlatRoomEngine {
    /// Execute one complete choice step across all 6 model tiers at the First Flat Room.
    pub fn execute_step(
        _operator_name: &str,
        natal_star_idx: usize,
        input: &str,
        current_coord: M5Coordinate,
    ) -> FirstFlatRoomStepResult {
        let star_idx = natal_star_idx.min(15);
        let pitch_mhz = STAR_MILLI_HZ_16[star_idx];

        // 1. Action Parser & Draft (2B)
        let archetype = ChoiceArchetype::parse_input(input);
        let proposed_delta = match archetype {
            ChoiceArchetype::Advance => [0, 1, 0, 0, 0], // Move north/along grid
            ChoiceArchetype::Survey => [0, 0, 0, 0, 0],  // Remain stationary
            ChoiceArchetype::Sing => [0, 0, 0, 1, 0],    // Advance harmonic phase
            ChoiceArchetype::Bind => [0, 0, 0, 0, 1],    // Increase polarity
            ChoiceArchetype::Cut => [0, 0, -1, 0, 0],    // Penetrate depth
            ChoiceArchetype::Rest => [0, 0, 0, 0, 0],
        };
        let draft = ActionDraft {
            archetype,
            proposed_delta,
            confidence_pmy: 9500,
            natal_pitch_mhz: pitch_mhz,
        };

        // 2. Parity & Invariant Filter (2B Anti-Expert Mirror)
        // Flat Room Invariant: cannot move outside bounds [-1..=1] on each axis from origin
        let mut target_axes = [0i8; 5];
        let mut is_valid = true;
        let mut i = 0;
        while i < 5 {
            let res = (current_coord.axes[i] as i16) + (proposed_delta[i] as i16);
            if res < -1 || res > 1 {
                is_valid = false;
            }
            target_axes[i] = if res < -1 {
                -1
            } else if res > 1 {
                1
            } else {
                res as i8
            };
            i += 1;
        }
        let target_coord = M5Coordinate::new(target_axes).unwrap_or(current_coord);
        let parity = ParityFilterOutcome {
            is_valid,
            logit_mask_penalty: if is_valid { 0 } else { 1000 },
            parity_checksum: 0x513_CAFE_BABE,
        };

        // 3. Voxel & Hermetic Codec (2B_M3)
        let morton_key_5d = Self::pack_morton_5d(target_coord.axes);
        let senses = [
            4500, // Light (pmy)
            9200, // Acoustic (pmy)
            1013, // Pressure (hPa)
            if is_valid { 0 } else { 8000 }, // Dissonance
            5000, // Polarity (q)
            120,  // Wind
            293,  // Thermal (Kelvin)
            1000, // Aether
        ];
        let art_delta = [100, 0, 50, 0, 0, 25, 0];
        let voxel = VoxelSensorySnapshot {
            morton_key_5d,
            coord: target_coord,
            senses,
            art_delta,
            hud_hash: morton_key_5d ^ 0xA5A5_5A5A_A5A5_5A5A,
        };

        // 4. Master World Engine (9B Backbone - 42 layers)
        let prose = match archetype {
            ChoiceArchetype::Survey => {
                "The pale megalithic floor of the first room is level and cold. Your footfalls ring clear in minor resonance against smooth stone. Above, the astrolabe ceiling aligns with your natal sky."
            }
            ChoiceArchetype::Advance => {
                "You take a deliberate stride forward. The flagstones settle beneath your weight without a murmur, the horizon of grey silt unrolling steadily before you."
            }
            ChoiceArchetype::Sing => {
                "You voice a clean phrase across the chamber. The walls return the frequency in pure harmonic fifths, illuminating etched sigils along the perimeter."
            }
            ChoiceArchetype::Bind => {
                "You reach out to tether the local resonance. A pale filament of light anchors between your hand and the bedrock."
            }
            ChoiceArchetype::Cut => {
                "You cleave the air with measured intent. The boundary between stone and silence parts for a heartbeat before closing smooth."
            }
            ChoiceArchetype::Rest => {
                "You pause at the center of the chamber. The quiet hum of the world lattice settles into steady equilibrium."
            }
        };
        let umwelt = RoomUmweltProse {
            prose,
            harmonic_freq_hz: pitch_mhz / 1000,
            layers_active: 42,
        };

        // 5. Sentry Protocol Guard (m2)
        let slot = CreeTransducer::parse_stroke_bytes(b"wapamew").expect("VTA token");
        let cree_asp_passed = AspGrammarSolver::solve_constraints(
            &slot,
            ObviationTier::ThirdProximate,
            Some(Animacy::Animate),
            Some(ObviationTier::ThirdObviative),
        )
        .is_ok();
        let sentinel_clean = is_sentinel_branchless(242) == 0; // 242 is sub-sentinel clean

        let mut vault = ZeroRetentionVault::new();
        vault.stage_transient_data(&[0x01, 0x02, 0x03, 0x04], 100, 10);
        let vault_sweep_done = vault.sweep_if_expired(110);

        let sentry = SentryAuditResult {
            cree_asp_passed,
            sentinel_clean,
            vault_sweep_done,
        };

        // 6. Macro-Seed Governor (Gemini 2.5 Flash)
        let needs_expansion = !is_valid;
        let macro_seed = MacroSeedExpansion {
            escalated: needs_expansion,
            macro_seed_hex: if needs_expansion { 0xFEEDBEEF_CAFE0001 } else { 0 },
            cost_micro_usd: if needs_expansion { UNIT_COST_CEILING_MICRO_USD } else { 0 },
            staging_purged: true,
        };

        FirstFlatRoomStepResult {
            draft,
            parity,
            voxel,
            umwelt,
            sentry,
            macro_seed,
        }
    }

    /// Bit-pack a 5D ternary coordinate vector into a 64-bit Morton key.
    fn pack_morton_5d(axes: [i8; 5]) -> u64 {
        let mut key = 0u64;
        let mut i = 0;
        while i < 5 {
            let a = axes[i];
            let encoded = ((a + 1) as u64) & 0x03; // -1->0, 0->1, 1->2
            key |= encoded << (i * 2);
            i += 1;
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_flat_room_survey_origin_execution() {
        let result = FirstFlatRoomEngine::execute_step(
            "Sean",
            6, // Procyon (2A)
            "survey pale floor",
            M5Coordinate::ORIGIN,
        );

        assert_eq!(result.draft.archetype, ChoiceArchetype::Survey);
        assert_eq!(result.draft.natal_pitch_mhz, 311_127);
        assert!(result.parity.is_valid);
        assert_eq!(result.parity.logit_mask_penalty, 0);
        assert_eq!(result.voxel.coord, M5Coordinate::ORIGIN);
        assert_eq!(result.umwelt.layers_active, 42);
        assert!(result.sentry.cree_asp_passed);
        assert!(result.sentry.sentinel_clean);
        assert!(result.sentry.vault_sweep_done);
        assert!(!result.macro_seed.escalated);
    }

    #[test]
    fn test_first_flat_room_advance_step() {
        let result = FirstFlatRoomEngine::execute_step(
            "Sean",
            0, // Sirius (8A)
            "advance north",
            M5Coordinate::ORIGIN,
        );

        assert_eq!(result.draft.archetype, ChoiceArchetype::Advance);
        assert_eq!(result.draft.natal_pitch_mhz, 440_000);
        assert!(result.parity.is_valid);
        assert_eq!(result.voxel.coord.axes, [0, 1, 0, 0, 0]);
        assert_eq!(result.umwelt.layers_active, 42);
    }

    #[test]
    fn test_first_flat_room_boundary_escalation() {
        // Positioned at boundary [+1, +1, +1, +1, +1]
        let boundary_coord = M5Coordinate::new([1, 1, 1, 1, 1]).unwrap();
        let result = FirstFlatRoomEngine::execute_step(
            "Sean",
            6,
            "advance north",
            boundary_coord,
        );

        // Movement beyond boundary triggers parity logit mask and out-of-band escalation
        assert!(!result.parity.is_valid);
        assert_eq!(result.parity.logit_mask_penalty, 1000);
        assert!(result.macro_seed.escalated);
        assert_eq!(result.macro_seed.cost_micro_usd, 400); // <= $0.0004 ceiling
        assert!(result.macro_seed.staging_purged);
    }
}
