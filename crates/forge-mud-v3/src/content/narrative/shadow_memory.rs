//! Shadow Memory — habit accumulator that feeds the adaptive nemesis.
//!
//! The Shadow does not only copy how the player fights.
//! It learns what the player keeps becoming.

use super::state::ShadowTier;

// ── Shadow Memory ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Classification of the Shadow based on accumulated threat.
pub enum ShadowForm {
    /// Basic form; early stage.
    Stalker,
    /// Evolved form; mid-game threat.
    Blighted,
    /// Ultimate form; apocalyptic threat.
    Harbinger,
}

#[derive(Debug, Clone, Copy, Default)]
/// Accumulated behavior patterns the Shadow learns from the player.
pub struct ShadowMemory {
    /// Attack direction counts: up/down/left/right.
    pub repeated_attack_dir: [u16; 4],
    /// Ability slot usage counts (8 slots).
    pub repeated_ability_use: [u16; 8],
    /// How many times the player parried.
    pub parry_count: u16,
    /// How many times the player dodged.
    pub dodge_count: u16,
    /// How many executions the player performed.
    pub execution_count: u16,
    /// How many times the player refused to execute.
    pub refused_execution_count: u16,
    /// How many times the player died.
    pub death_count: u16,
    /// How many merciful actions the player took.
    pub mercy_count: u16,
    /// How many erasures the player performed.
    pub erasure_count: u16,
    /// Primary resonance frequency the player exhibits.
    pub dominant_resonance_hz: i16,
    /// Hash of the primary route the player takes.
    pub route_hash: u64,
    /// Total number of inputs recorded.
    pub total_inputs: u32,
}

impl ShadowMemory {
    /// Record an attack in a given direction.
    pub fn record_attack(&mut self, direction: u8) {
        if (direction as usize) < 4 {
            self.repeated_attack_dir[direction as usize] += 1;
        }
        self.total_inputs += 1;
    }

    /// Record ability usage from a slot.
    pub fn record_ability(&mut self, slot: u8) {
        if (slot as usize) < 8 {
            self.repeated_ability_use[slot as usize] += 1;
        }
        self.total_inputs += 1;
    }

    /// Record a parry action.
    pub fn record_parry(&mut self) { self.parry_count += 1; self.total_inputs += 1; }
    /// Record a dodge action.
    pub fn record_dodge(&mut self) { self.dodge_count += 1; self.total_inputs += 1; }
    /// Record an execution action.
    pub fn record_execution(&mut self) { self.execution_count += 1; }
    /// Record a refused execution (mercy).
    pub fn record_refused_execution(&mut self) { self.refused_execution_count += 1; }
    /// Record a player death.
    pub fn record_death(&mut self) { self.death_count += 1; }
    /// Record a merciful action.
    pub fn record_mercy(&mut self) { self.mercy_count += 1; }
    /// Record an erasure action.
    pub fn record_erasure(&mut self) { self.erasure_count += 1; }

    /// Return the most frequently used attack direction.
    pub fn dominant_attack_dir(&self) -> u8 {
        self.repeated_attack_dir.iter()
            .enumerate()
            .max_by_key(|&(_, v)| *v)
            .map(|(i, _)| i as u8)
            .unwrap_or(0)
    }

    /// Return the most frequently used ability slot.
    pub fn dominant_ability(&self) -> u8 {
        self.repeated_ability_use.iter()
            .enumerate()
            .max_by_key(|&(_, v)| *v)
            .map(|(i, _)| i as u8)
            .unwrap_or(0)
    }

    /// Classify the Shadow's current form based on behavior intensity.
    pub fn classify(&self) -> ShadowForm {
        if self.execution_count > 30 || self.erasure_count > 10 {
            ShadowForm::Harbinger
        } else if self.total_inputs > 500 || self.repeated_ability_use.iter().max().copied().unwrap_or(0) > 80 {
            ShadowForm::Blighted
        } else {
            ShadowForm::Stalker
        }
    }

    /// Convert classified form to a world shadow tier.
    pub fn shadow_tier(&self) -> ShadowTier {
        match self.classify() {
            ShadowForm::Stalker => ShadowTier::Stalker,
            ShadowForm::Blighted => ShadowTier::Blighted,
            ShadowForm::Harbinger => ShadowTier::Harbinger,
        }
    }
}

// ── Shadow Counter Rules ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// How the Shadow adapts its strategy based on player habits.
pub enum ShadowResponse {
    /// Counters the player's favorite attack direction.
    DirectionalCounter,
    /// Resists the player's most-used ability.
    AspectResistance,
    /// Baits the player into parry spam.
    FeintBait,
    /// Chases down the dodging player.
    DelayedPursuit,
    /// Attacks from the spirit layer.
    SpiritAmbush,
    /// Mimics restraint then punishes.
    MimicRestraintThenPunish,
    /// Becomes overtly lethal (stops playing).
    DirectLethality,
    /// Appears earlier than expected.
    EarlierAppearances,
}

/// Determine the Shadow's next counter strategy.
pub fn shadow_counter(memory: &ShadowMemory) -> ShadowResponse {
    let max_dir = *memory.repeated_attack_dir.iter().max().unwrap_or(&0);
    let max_ability = *memory.repeated_ability_use.iter().max().unwrap_or(&0);

    if memory.erasure_count > 5 {
        ShadowResponse::DirectLethality
    } else if memory.death_count > 10 {
        ShadowResponse::SpiritAmbush
    } else if memory.mercy_count > 8 {
        ShadowResponse::MimicRestraintThenPunish
    } else if memory.parry_count > memory.dodge_count * 2 {
        ShadowResponse::FeintBait
    } else if memory.dodge_count > memory.parry_count * 2 {
        ShadowResponse::DelayedPursuit
    } else if max_ability > 80 {
        ShadowResponse::AspectResistance
    } else if max_dir > 60 {
        ShadowResponse::DirectionalCounter
    } else {
        ShadowResponse::EarlierAppearances
    }
}

// ── Shadow File ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
/// A single recorded frame from a player's game (for Shadow replay).
pub struct ShadowFrame {
    /// Tick delta since last frame.
    pub tick_delta: u16,
    /// Bitmask of inputs during this frame.
    pub input_bits: u32,
    /// X position at this frame.
    pub position_x: i32,
    /// Y position at this frame.
    pub position_y: i32,
    /// Hash of game state at this frame.
    pub state_hash: u32,
}

/// Maximum number of frames that can be recorded in a shadow file.
pub const MAX_SHADOW_FRAMES: usize = 512;

/// A recorded Shadow entity with replay data and checksums.
pub struct ShadowFile {
    /// Format version of this file.
    pub version: u16,
    /// Hash of the player that spawned this Shadow.
    pub origin_player_hash: u64,
    /// Accumulated behavior patterns.
    pub memory: ShadowMemory,
    /// Array of recorded frames.
    pub frames: [ShadowFrame; MAX_SHADOW_FRAMES],
    /// Number of frames currently recorded.
    pub frame_count: u16,
    /// Integrity checksum.
    pub checksum: u64,
}

impl ShadowFile {
    /// Create a new shadow file for a given player.
    pub fn new(player_hash: u64) -> Self {
        Self {
            version: 1,
            origin_player_hash: player_hash,
            memory: ShadowMemory::default(),
            frames: [ShadowFrame { tick_delta: 0, input_bits: 0, position_x: 0, position_y: 0, state_hash: 0 }; MAX_SHADOW_FRAMES],
            frame_count: 0,
            checksum: 0,
        }
    }

    /// Record a frame if capacity allows.
    pub fn record_frame(&mut self, frame: ShadowFrame) {
        if (self.frame_count as usize) < MAX_SHADOW_FRAMES {
            self.frames[self.frame_count as usize] = frame;
            self.frame_count += 1;
        }
    }

    /// Compute and store the FNV-1a checksum.
    pub fn compute_checksum(&mut self) {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.version as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.origin_player_hash;
        hash = hash.wrapping_mul(0x100000001b3);
        for i in 0..self.frame_count as usize {
            let f = &self.frames[i];
            hash ^= f.tick_delta as u64;
            hash ^= f.input_bits as u64;
            hash ^= f.state_hash as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.checksum = hash;
    }

    /// Verify the stored checksum matches the data.
    pub fn validate_checksum(&self) -> bool {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.version as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.origin_player_hash;
        hash = hash.wrapping_mul(0x100000001b3);
        for i in 0..self.frame_count as usize {
            let f = &self.frames[i];
            hash ^= f.tick_delta as u64;
            hash ^= f.input_bits as u64;
            hash ^= f.state_hash as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash == self.checksum
    }
}

// ── Counterpart Profile ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Archetypal identities the Shadow can adopt.
pub enum CounterpartContext {
    /// Weapon incarnate; mastery through practice.
    WeaponSelf,
    /// Judge incarnate; wisdom through judgment.
    JudgeSelf,
    /// Saint incarnate; virtue through mercy.
    SaintSelf,
    /// Tyrant incarnate; dominion through execution.
    TyrantSelf,
    /// Widow incarnate; sorrow through loss.
    WidowSelf,
    /// Vowless incarnate; erasure through silence.
    VowlessSelf,
    /// Forgotten incarnate; void through negation.
    ForgottenSelf,
}

#[derive(Debug, Clone, Copy)]
/// A possible alternate self derived from the player's choices.
pub struct CounterpartProfile {
    /// The player entity that spawned this counterpart.
    pub origin_entity: u64,
    /// Seed for RNG when manifesting this counterpart.
    pub branch_seed: u64,
    /// Similarity to the origin (0-1000).
    pub similarity_score: u16,
    /// Which archetype this counterpart embodies.
    pub context: CounterpartContext,
}

/// Derive a counterpart identity from accumulated behavior patterns.
pub fn derive_counterpart(memory: &ShadowMemory, player_hash: u64) -> CounterpartProfile {
    let context = if memory.execution_count > 20 {
        CounterpartContext::TyrantSelf
    } else if memory.mercy_count > 15 {
        CounterpartContext::SaintSelf
    } else if memory.refused_execution_count > 10 {
        CounterpartContext::JudgeSelf
    } else if memory.erasure_count > 8 {
        CounterpartContext::ForgottenSelf
    } else if memory.death_count > 15 {
        CounterpartContext::WidowSelf
    } else if memory.total_inputs > 1000 {
        CounterpartContext::WeaponSelf
    } else {
        CounterpartContext::VowlessSelf
    };

    let similarity = ((memory.total_inputs as u32).min(10000) / 10) as u16;

    CounterpartProfile {
        origin_entity: player_hash,
        branch_seed: player_hash.wrapping_mul(0x9E3779B97F4A7C15) ^ (memory.execution_count as u64),
        similarity_score: similarity,
        context,
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_classifies_stalker_by_default() {
        let mem = ShadowMemory::default();
        assert_eq!(mem.classify(), ShadowForm::Stalker);
    }

    #[test]
    fn shadow_classifies_harbinger_on_executions() {
        let mut mem = ShadowMemory::default();
        mem.execution_count = 31;
        assert_eq!(mem.classify(), ShadowForm::Harbinger);
    }

    #[test]
    fn shadow_classifies_blighted_on_repetition() {
        let mut mem = ShadowMemory::default();
        mem.total_inputs = 501;
        assert_eq!(mem.classify(), ShadowForm::Blighted);
    }

    #[test]
    fn counter_responds_to_parry_spam() {
        let mut mem = ShadowMemory::default();
        mem.parry_count = 50;
        mem.dodge_count = 10;
        assert_eq!(shadow_counter(&mem), ShadowResponse::FeintBait);
    }

    #[test]
    fn counter_responds_to_dodge_spam() {
        let mut mem = ShadowMemory::default();
        mem.dodge_count = 50;
        mem.parry_count = 10;
        assert_eq!(shadow_counter(&mem), ShadowResponse::DelayedPursuit);
    }

    #[test]
    fn counter_responds_to_high_erasure() {
        let mut mem = ShadowMemory::default();
        mem.erasure_count = 6;
        assert_eq!(shadow_counter(&mem), ShadowResponse::DirectLethality);
    }

    #[test]
    fn shadow_file_checksum_validates() {
        let mut file = ShadowFile::new(0xDEAD);
        file.record_frame(ShadowFrame { tick_delta: 1, input_bits: 0xFF, position_x: 100, position_y: 200, state_hash: 42 });
        file.record_frame(ShadowFrame { tick_delta: 2, input_bits: 0xAA, position_x: 110, position_y: 210, state_hash: 43 });
        file.compute_checksum();
        assert!(file.validate_checksum());
    }

    #[test]
    fn shadow_file_tampered_fails_checksum() {
        let mut file = ShadowFile::new(0xDEAD);
        file.record_frame(ShadowFrame { tick_delta: 1, input_bits: 0xFF, position_x: 100, position_y: 200, state_hash: 42 });
        file.compute_checksum();
        file.frames[0].input_bits = 0x00; // tamper
        assert!(!file.validate_checksum());
    }

    #[test]
    fn counterpart_derives_from_memory() {
        let mut mem = ShadowMemory::default();
        mem.execution_count = 25;
        let cp = derive_counterpart(&mem, 0xCAFE);
        assert_eq!(cp.context, CounterpartContext::TyrantSelf);
    }

    #[test]
    fn dominant_attack_direction() {
        let mut mem = ShadowMemory::default();
        mem.record_attack(2); // left
        mem.record_attack(2);
        mem.record_attack(2);
        mem.record_attack(0); // up
        assert_eq!(mem.dominant_attack_dir(), 2);
    }
}
