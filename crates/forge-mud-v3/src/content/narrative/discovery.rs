//! Discovery System — environmental tells replace quest markers.
//!
//! Progressive disclosure: the player should feel systems before understanding them.
//! Expertise comes from reading sound, route, faction schedules, and world scars.

// ── Discovery Tells ──────────────────────────────────────────────────────────

/// An environmental tell the player can read to discover world state without a quest marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiscoveryTell {
    /// Visual tell: claw or blade scratches on a surface (visibility 40).
    ScratchMarks,
    /// Visual tell: an account book left open, exposing debts or transactions (visibility 60).
    OpenLedger,
    /// Visual tell: a weapon still lodged where it struck (visibility 60).
    EmbeddedWeapon,
    /// Visual tell: residue from a fire or burn event (visibility 40).
    AshResidue,
    /// Visual tell: a knotted scrap of cloth left as a marker (visibility 40).
    ClothKnot,
    /// Visual tell: a wall marked with an owed debt (visibility 30).
    WallDebt,
    /// Haptic tell: a rhythmic pulse felt through roots or ground (visibility 20).
    RootPulse,
    /// Temperature tell: an unnaturally cold patch (visibility 10).
    ColdSpot,
    /// Audio tell: a bell struck in a recognizable rhythm (visibility 20).
    BellRhythm,
    /// Visual tell: a stain of spilled ink (visibility 30).
    InkStain,
    /// Visual tell: a collapsed archway or structure (visibility 80).
    CollapsedArch,
    /// Visual tell: the posture or placement of a body (visibility 80).
    BodyPosition,
}

impl DiscoveryTell {
    /// Return the sensory channel through which this tell is perceived.
    pub fn channel(self) -> SensoryChannel {
        match self {
            Self::ScratchMarks | Self::OpenLedger | Self::EmbeddedWeapon
            | Self::AshResidue | Self::ClothKnot | Self::WallDebt
            | Self::CollapsedArch | Self::BodyPosition | Self::InkStain => SensoryChannel::Visual,
            Self::RootPulse => SensoryChannel::Haptic,
            Self::ColdSpot => SensoryChannel::Temperature,
            Self::BellRhythm => SensoryChannel::Audio,
        }
    }

    /// How obvious this tell is (0 = expert-only, 100 = obvious).
    pub fn visibility(self) -> u8 {
        match self {
            Self::CollapsedArch | Self::BodyPosition => 80,
            Self::OpenLedger | Self::EmbeddedWeapon => 60,
            Self::ScratchMarks | Self::AshResidue | Self::ClothKnot => 40,
            Self::WallDebt | Self::InkStain => 30,
            Self::BellRhythm | Self::RootPulse => 20,
            Self::ColdSpot => 10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// How the player perceives a discovery tell.
pub enum SensoryChannel {
    /// Visual perception (sight).
    Visual,
    /// Auditory perception (sound).
    Audio,
    /// Tactile perception (touch/vibration).
    Haptic,
    /// Thermal perception (temperature).
    Temperature,
}

// ── Evidence Density ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The four acts of the narrative.
pub enum Act {
    /// Act 1: Introduction.
    Act1,
    /// Act 2: Escalation.
    Act2,
    /// Act 3: Climax.
    Act3,
    /// Act 4: Resolution.
    Act4,
}

/// Return how many discovery tells should be active per zone in each act.
pub fn evidence_density(act: Act) -> u8 {
    match act {
        Act::Act1 => 2,  // sparse — player learns to look
        Act::Act2 => 4,  // converging — patterns emerge
        Act::Act3 => 6,  // confirmatory — player reads fluently
        Act::Act4 => 8,  // resolving — everything speaks
    }
}

// ── Discovery Log (fixed-size, per-zone) ─────────────────────────────────────

/// Maximum number of tells that fit in the fixed bitset per zone.
pub const MAX_TELLS_PER_ZONE: usize = 12;

/// A tell placed beyond the fixed 12-slot bitset (allows scaling past cap).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicTell {
    /// Type of tell.
    pub tell: DiscoveryTell,
    /// Position in millimeters within the zone.
    pub position: (i64, i64),
    /// Whether the player has discovered this tell.
    pub found: bool,
}

#[derive(Debug, Clone)]
/// Discovery progress for a single zone.
pub struct ZoneDiscoveryState {
    /// Which zone this tracks.
    pub zone_id: u16,
    /// Bitset of tells present in this zone (up to 12).
    pub tells_present: u16,
    /// Bitset of tells the player has discovered.
    pub tells_found: u16,
    /// Quality of evidence gathered (0-255).
    pub evidence_quality: u8,
    /// Position of each tell in millimeters. Index = DiscoveryTell as usize.
    pub tell_positions: [(i64, i64); 12],
    /// Tells beyond the 12-slot cap (dynamic; allows repeat kinds at new positions).
    pub overflow_tells: Vec<DynamicTell>,
}

impl ZoneDiscoveryState {
    /// Create a new zone discovery tracker.
    pub fn new(zone_id: u16) -> Self {
        Self { zone_id, tells_present: 0, tells_found: 0, evidence_quality: 0, tell_positions: [(0, 0); 12], overflow_tells: Vec::new() }
    }

    /// Mark a tell as present in this zone.
    pub fn place_tell(&mut self, tell: DiscoveryTell) {
        self.tells_present |= 1 << (tell as u16);
    }

    /// Mark a tell as discovered; returns true if successful (not already found).
    pub fn discover_tell(&mut self, tell: DiscoveryTell) -> bool {
        let bit = 1 << (tell as u16);
        if self.tells_present & bit != 0 && self.tells_found & bit == 0 {
            self.tells_found |= bit;
            self.evidence_quality = self.evidence_quality.saturating_add(tell.visibility() / 4);
            true
        } else {
            false
        }
    }

    /// Add a tell to the dynamic overflow store (cold path only).
    pub fn place_overflow_tell(&mut self, tell: DiscoveryTell, position: (i64, i64)) {
        self.overflow_tells.push(DynamicTell { tell, position, found: false });
    }

    /// Mark an overflow tell as discovered; returns true if found.
    pub fn discover_overflow_tell(&mut self, tell: DiscoveryTell) -> bool {
        if let Some(dt) = self.overflow_tells.iter_mut().find(|dt| dt.tell == tell && !dt.found) {
            dt.found = true;
            self.evidence_quality = self.evidence_quality.saturating_add(tell.visibility() / 4);
            true
        } else {
            false
        }
    }

    /// Count how many tells have been discovered (fixed + overflow).
    pub fn found_count(&self) -> u32 {
        self.tells_found.count_ones() + self.overflow_tells.iter().filter(|t| t.found).count() as u32
    }
    /// Count total tells present in this zone (fixed + overflow).
    pub fn present_count(&self) -> u32 {
        self.tells_present.count_ones() + self.overflow_tells.len() as u32
    }
    /// Calculate discovery completion as a percentage (0-100).
    pub fn completion_pct(&self) -> u8 {
        let present = self.present_count();
        if present == 0 { return 0; }
        ((self.found_count() * 100) / present) as u8
    }
}

/// Seed tells into a zone deterministically from world seed and zone ID.
/// Places `count` tells; scales past MAX_TELLS_PER_ZONE into overflow store.
/// Positions are seeded within a 100m × 100m zone (0..100_000 mm per axis).
pub fn seed_zone_tells(zone: &mut ZoneDiscoveryState, world_seed: u64, count: u8) {
    let all_tells = [
        DiscoveryTell::ScratchMarks, DiscoveryTell::OpenLedger,
        DiscoveryTell::EmbeddedWeapon, DiscoveryTell::AshResidue,
        DiscoveryTell::ClothKnot, DiscoveryTell::WallDebt,
        DiscoveryTell::RootPulse, DiscoveryTell::ColdSpot,
        DiscoveryTell::BellRhythm, DiscoveryTell::InkStain,
        DiscoveryTell::CollapsedArch, DiscoveryTell::BodyPosition,
    ];
    let mut h = world_seed ^ (zone.zone_id as u64).wrapping_mul(0x9E3779B97F4A7C15);
    let capped = count.min(12);
    for _ in 0..capped {
        h = h.wrapping_mul(0xBF58476D1CE4E5B9) ^ (h >> 27);
        let idx = (h % 12) as usize;
        zone.place_tell(all_tells[idx]);
        // Seed position for this tell
        h = h.wrapping_mul(0x94D049BB133111EB) ^ (h >> 31);
        let x = (h % 100_000) as i64;
        h = h.wrapping_mul(0x6A09E667F3BCC908) ^ (h >> 29);
        let y = (h % 100_000) as i64;
        zone.tell_positions[idx] = (x, y);
    }
    // Density past MAX_TELLS_PER_ZONE: continue the same deterministic chain into the
    // dynamic overflow store. Repeat kinds at new positions are allowed here — this is
    // the only place that grows past the fixed 12-slot cap, and it runs once at seed
    // time (cold path), never per-tick.
    for _ in capped..count {
        h = h.wrapping_mul(0xBF58476D1CE4E5B9) ^ (h >> 27);
        let idx = (h % 12) as usize;
        h = h.wrapping_mul(0x94D049BB133111EB) ^ (h >> 31);
        let x = (h % 100_000) as i64;
        h = h.wrapping_mul(0x6A09E667F3BCC908) ^ (h >> 29);
        let y = (h % 100_000) as i64;
        zone.place_overflow_tell(all_tells[idx], (x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tell_channels_are_assigned() {
        assert_eq!(DiscoveryTell::BellRhythm.channel(), SensoryChannel::Audio);
        assert_eq!(DiscoveryTell::ColdSpot.channel(), SensoryChannel::Temperature);
        assert_eq!(DiscoveryTell::RootPulse.channel(), SensoryChannel::Haptic);
        assert_eq!(DiscoveryTell::ScratchMarks.channel(), SensoryChannel::Visual);
    }

    #[test]
    fn evidence_density_increases_per_act() {
        assert!(evidence_density(Act::Act1) < evidence_density(Act::Act2));
        assert!(evidence_density(Act::Act2) < evidence_density(Act::Act3));
        assert!(evidence_density(Act::Act3) < evidence_density(Act::Act4));
    }

    #[test]
    fn zone_discovery_tracks_tells() {
        let mut zone = ZoneDiscoveryState::new(1);
        zone.place_tell(DiscoveryTell::ScratchMarks);
        zone.place_tell(DiscoveryTell::BellRhythm);
        zone.place_tell(DiscoveryTell::ColdSpot);
        assert_eq!(zone.present_count(), 3);
        assert_eq!(zone.found_count(), 0);

        assert!(zone.discover_tell(DiscoveryTell::ScratchMarks));
        assert_eq!(zone.found_count(), 1);
        assert!(zone.evidence_quality > 0);

        // Can't discover same tell twice
        assert!(!zone.discover_tell(DiscoveryTell::ScratchMarks));

        // Can't discover tell not present
        assert!(!zone.discover_tell(DiscoveryTell::OpenLedger));
    }

    #[test]
    fn completion_percentage() {
        let mut zone = ZoneDiscoveryState::new(1);
        zone.place_tell(DiscoveryTell::ScratchMarks);
        zone.place_tell(DiscoveryTell::BellRhythm);
        zone.discover_tell(DiscoveryTell::ScratchMarks);
        assert_eq!(zone.completion_pct(), 50);
    }

    #[test]
    fn cold_spot_is_hardest_to_find() {
        assert_eq!(DiscoveryTell::ColdSpot.visibility(), 10);
    }

    #[test]
    fn collapsed_arch_is_most_obvious() {
        assert_eq!(DiscoveryTell::CollapsedArch.visibility(), 80);
    }

    // [BOARD: DISCOVERY-DYNAMIC]
    #[test]
    fn dynamic_overflow_scales_past_cap_and_is_deterministic() {
        let mut a = ZoneDiscoveryState::new(7);
        seed_zone_tells(&mut a, 424242, 20); // 20 > MAX_TELLS_PER_ZONE(12)

        // Overflow store holds exactly what the old fixed-12 model had to silently drop.
        assert_eq!(a.overflow_tells.len(), 20 - MAX_TELLS_PER_ZONE);
        let placed: Vec<(DiscoveryTell, (i64, i64))> =
            a.overflow_tells.iter().map(|t| (t.tell, t.position)).collect();
        assert!(a.overflow_tells.iter().all(|t| !t.found));

        // Every overflow tell registers and resolves exactly once.
        for (kind, _) in &placed {
            assert!(a.discover_overflow_tell(*kind));
        }
        assert!(a.overflow_tells.iter().all(|t| t.found));
        assert!(!a.discover_overflow_tell(placed[0].0)); // fully resolved, nothing left to find
        assert!(a.found_count() >= placed.len() as u32);

        // Determinism: identical seed+zone+count reproduces identical overflow placements.
        let mut b = ZoneDiscoveryState::new(7);
        seed_zone_tells(&mut b, 424242, 20);
        assert_eq!(a.tell_positions, b.tell_positions);
        assert_eq!(a.tells_present, b.tells_present);
        let b_placed: Vec<(DiscoveryTell, (i64, i64))> =
            b.overflow_tells.iter().map(|t| (t.tell, t.position)).collect();
        assert_eq!(placed, b_placed);
    }
}
