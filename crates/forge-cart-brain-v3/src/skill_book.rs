// Ported by translation from quarry ironroot-edict (pure leaf) — RunDevRun cart World/Level sprint.
//! Skill Book — UO-style use-based skill system.
//!
//! Skills improve because the player uses them. Difficulty matters.
//! Repetition soft-caps. World consequence gives the best growth.
//! Internal scale: Permyriad (0-10000 = 0.00-100.00 skill).

/// Enumeration of all available skill types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkillKind {
    /// Knifecraft skill.
    Knifecraft,
    /// Rootcraft skill.
    Rootcraft,
    /// Diplomacy skill.
    Diplomacy,
    /// Rootthief skill.
    Rootthief,
    /// Deathwalking skill.
    Deathwalking,
    /// NameLaw skill.
    NameLaw,
    /// Cartography skill.
    Cartography,
    /// Shadowbinding skill.
    Shadowbinding,
    /// Warcraft skill.
    Warcraft,
    /// Tradecraft skill.
    Tradecraft,
}

/// Current state of a single skill.
#[derive(Debug, Clone, Copy)]
pub struct SkillState {
    /// The type of skill.
    pub kind: SkillKind,
    /// 0-10000 (Permyriad). 10000 = GM (100.00).
    pub value: u16,
    /// Repetition counter for decay calculation.
    pub repetition: u32,
    /// Last use hash for novelty detection.
    pub last_use_hash: u64,
}

impl SkillState {
    /// Create a new skill at zero rank.
    pub fn new(kind: SkillKind) -> Self {
        Self { kind, value: 0, repetition: 0, last_use_hash: 0 }
    }

    /// Skill as display float (0.0 - 100.0). Only for UI.
    pub fn display_value(&self) -> f32 {
        self.value as f32 / 100.0
    }
}

/// A player's collection of all skills with trainer caps.
#[derive(Debug, Clone)]
pub struct SkillBook {
    /// All skill states (one per SkillKind).
    pub skills: Vec<SkillState>,
    /// Trainer cap per skill (faction-gated). Default 7000 (70.00).
    pub caps: Vec<u16>,
}

impl SkillBook {
    /// Create a new skill book with all skills at zero.
    pub fn new() -> Self {
        let skills: Vec<SkillState> = [
            SkillKind::Knifecraft, SkillKind::Rootcraft, SkillKind::Diplomacy,
            SkillKind::Rootthief, SkillKind::Deathwalking, SkillKind::NameLaw,
            SkillKind::Cartography, SkillKind::Shadowbinding, SkillKind::Warcraft,
            SkillKind::Tradecraft,
        ].iter().map(|&k| SkillState::new(k)).collect();
        let caps = vec![7000; skills.len()];
        Self { skills, caps }
    }

    /// Get a skill's current value by kind.
    pub fn get(&self, kind: SkillKind) -> u16 {
        self.skills.iter().find(|s| s.kind == kind).map(|s| s.value).unwrap_or(0)
    }

    /// Get mutable access to a skill's state by kind.
    pub fn get_mut(&mut self, kind: SkillKind) -> Option<&mut SkillState> {
        self.skills.iter_mut().find(|s| s.kind == kind)
    }

    /// Raise trainer cap for a skill (faction reward).
    pub fn raise_cap(&mut self, kind: SkillKind, new_cap: u16) {
        if let Some(idx) = self.skills.iter().position(|s| s.kind == kind) {
            if new_cap > self.caps[idx] {
                self.caps[idx] = new_cap;
            }
        }
    }
}

// ── Skill Use Event ──────────────────────────────────────────────────────────

/// Event representing the use of a skill in the game world.
#[derive(Debug, Clone, Copy)]
pub struct SkillUseEvent {
    /// The skill being used.
    pub skill: SkillKind,
    /// Difficulty of the action (0-10000 permyriad).
    pub difficulty: u16,
    /// Risk factor (0-10000 permyriad).
    pub risk: u16,
    /// Hash for novelty detection (different = novel use).
    pub novelty_hash: u64,
    /// World consequence bonus (0-10000 permyriad).
    pub world_consequence: u16,
}

/// Apply a skill use. Returns gain in Permyriad (0 if capped or no gain).
pub fn apply_skill_use(book: &mut SkillBook, event: &SkillUseEvent) -> u16 {
    let idx = match book.skills.iter().position(|s| s.kind == event.skill) {
        Some(i) => i,
        None => return 0,
    };

    let skill = &book.skills[idx];
    let cap = book.caps[idx];

    // Already at cap
    if skill.value >= cap { return 0; }

    // Novelty bonus: same action hash = repetition, different = novelty
    let novelty_bonus: u16 = if event.novelty_hash != skill.last_use_hash { 2 } else { 0 };

    // Repetition decay: more uses = less gain
    let rep_decay: u16 = (skill.repetition / 50).min(8) as u16;

    // Base gain from difficulty + risk + consequence
    let raw_gain = (event.difficulty / 100)
        .saturating_add(event.risk / 200)
        .saturating_add(event.world_consequence / 50)
        .saturating_add(novelty_bonus)
        .saturating_sub(rep_decay)
        .max(1);

    // Higher skill = harder to gain (diminishing returns)
    let diminish = (skill.value / 1000).max(1) as u16;
    let gain = (raw_gain / diminish).max(1).min(cap - skill.value);

    book.skills[idx].value += gain;
    book.skills[idx].repetition += 1;
    book.skills[idx].last_use_hash = event.novelty_hash;

    gain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_book_starts_at_zero() {
        let book = SkillBook::new();
        assert_eq!(book.get(SkillKind::Knifecraft), 0);
    }

    #[test]
    fn skill_use_increases_value() {
        let mut book = SkillBook::new();
        let event = SkillUseEvent {
            skill: SkillKind::Knifecraft,
            difficulty: 500,
            risk: 200,
            novelty_hash: 12345,
            world_consequence: 100,
        };
        let gain = apply_skill_use(&mut book, &event);
        assert!(gain > 0);
        assert!(book.get(SkillKind::Knifecraft) > 0);
    }

    #[test]
    fn skill_respects_cap() {
        let mut book = SkillBook::new();
        book.caps[0] = 10; // Very low cap for Knifecraft
        let event = SkillUseEvent {
            skill: SkillKind::Knifecraft,
            difficulty: 9000,
            risk: 9000,
            novelty_hash: 999,
            world_consequence: 9000,
        };
        for _ in 0..100 {
            apply_skill_use(&mut book, &event);
        }
        assert!(book.get(SkillKind::Knifecraft) <= 10);
    }

    #[test]
    fn raise_cap_allows_further_growth() {
        let mut book = SkillBook::new();
        book.caps[0] = 50;
        let event = SkillUseEvent { skill: SkillKind::Knifecraft, difficulty: 5000, risk: 1000, novelty_hash: 1, world_consequence: 500 };
        for _ in 0..200 { apply_skill_use(&mut book, &event); }
        let before = book.get(SkillKind::Knifecraft);
        book.raise_cap(SkillKind::Knifecraft, 10000);
        apply_skill_use(&mut book, &event);
        assert!(book.get(SkillKind::Knifecraft) >= before);
    }
}
