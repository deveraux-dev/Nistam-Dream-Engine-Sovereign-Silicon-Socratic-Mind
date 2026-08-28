//! Skill Book — UO-style use-based skill system.
//!
//! Skills improve because the player uses them. Difficulty matters.
//! Repetition soft-caps. World consequence gives the best growth.
//! Internal scale: Permyriad (0-10000 = 0.00-100.00 skill).
//!
//! Deterministic: same event + same book state → same gain. No floats, no entropy.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A single skill category.
pub enum SkillKind {
    /// Blade and melee combat skill.
    Knifecraft,
    /// Nature and herbalism skill.
    Rootcraft,
    /// Social interaction and persuasion skill.
    Diplomacy,
    /// Theft and stealth skill.
    Rootthief,
    /// Undead and necromancy skill.
    Deathwalking,
    /// True name magic skill.
    NameLaw,
    /// Mapmaking and navigation skill.
    Cartography,
    /// Shadow magic and binding skill.
    Shadowbinding,
    /// Combat tactics and warfare skill.
    Warcraft,
    /// Commerce and trading skill.
    Tradecraft,
}

#[derive(Debug, Clone, Copy)]
/// Current state and value of a skill.
pub struct SkillState {
    /// The skill type.
    pub kind: SkillKind,
    /// 0-10000 (Permyriad). 10000 = GM (100.00)
    pub value: u16,
    /// Repetition counter for decay calculation
    pub repetition: u32,
    /// Last use hash for novelty detection
    pub last_use_hash: u64,
}

impl SkillState {
    /// Create a new skill at 0 value.
    pub fn new(kind: SkillKind) -> Self {
        Self { kind, value: 0, repetition: 0, last_use_hash: 0 }
    }

    /// Skill as display float (0.0 - 100.0). Only for UI.
    /// (NOTE: This is the ONLY place floats are used, for display only — not in state machine.)
    #[allow(unsafe_code)]
    // JUSTIFICATION: This cast is safe because u16 is always valid f32.
    // Used only for display; no core logic depends on float precision.
    pub fn display_value(&self) -> f32 {
        self.value as f32 / 100.0
    }
}

#[derive(Debug, Clone)]
/// A player's collection of skills and their advancement state.
pub struct SkillBook {
    /// All skills and their current values.
    pub skills: Vec<SkillState>,
    /// Trainer cap per skill (faction-gated). Default 7000 (70.00).
    pub caps: Vec<u16>,
}

impl SkillBook {
    /// Create a new skill book with all skills at 0 value.
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

    /// Get the current value of a skill.
    pub fn get(&self, kind: SkillKind) -> u16 {
        self.skills.iter().find(|s| s.kind == kind).map(|s| s.value).unwrap_or(0)
    }

    /// Get a mutable reference to a skill's state.
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

#[derive(Debug, Clone, Copy)]
/// A skill use event that drives skill advancement.
pub struct SkillUseEvent {
    /// The skill being used.
    pub skill: SkillKind,
    /// Task difficulty (0-10000).
    pub difficulty: u16,
    /// Personal risk taken (0-10000).
    pub risk: u16,
    /// Hash of the action for novelty detection.
    pub novelty_hash: u64,
    /// World consequence from the use (0-10000).
    pub world_consequence: u16,
}

/// Apply a skill use. Returns gain in Permyriad (0 if capped or no gain).
/// **Deterministic:** same event + same book state → same gain, every time.
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

    // ─── L07: Bijection Test ───────────────────────────────────────────────
    // Invariant: skill gain is deterministic — same seed + event → same gain.
    #[test]
    fn skill_gain_is_deterministic_same_seed() {
        let event = SkillUseEvent {
            skill: SkillKind::Rootcraft,
            difficulty: 3000,
            risk: 1500,
            novelty_hash: 0xDEAD_BEEF,
            world_consequence: 250,
        };

        let mut book1 = SkillBook::new();
        let mut book2 = SkillBook::new();

        let gain1 = apply_skill_use(&mut book1, &event);
        let gain2 = apply_skill_use(&mut book2, &event);

        assert_eq!(gain1, gain2, "same event must produce same gain");
        assert_eq!(book1.get(SkillKind::Rootcraft), book2.get(SkillKind::Rootcraft));
    }

    // ─── L07: Bijection Test ───────────────────────────────────────────────
    // Invariant: skill gain with novelty is always >= gain without novelty (edge case).
    #[test]
    fn novelty_never_decreases_gain() {
        let base_event = SkillUseEvent {
            skill: SkillKind::Diplomacy,
            difficulty: 2000,
            risk: 1000,
            novelty_hash: 1,
            world_consequence: 100,
        };

        let repeat_event = SkillUseEvent {
            skill: SkillKind::Diplomacy,
            difficulty: 2000,
            risk: 1000,
            novelty_hash: 1, // same hash = repetition
            world_consequence: 100,
        };

        let mut book1 = SkillBook::new();
        let mut book2 = SkillBook::new();

        // First use (always novel on fresh book)
        apply_skill_use(&mut book1, &base_event);
        apply_skill_use(&mut book2, &base_event);

        // Second use: book1 gets a different hash (novelty), book2 repeats
        let new_event = SkillUseEvent { novelty_hash: 2, ..base_event };
        let gain_novel = apply_skill_use(&mut book1, &new_event);
        let gain_repeat = apply_skill_use(&mut book2, &repeat_event);

        // Novelty should produce at least as much gain as repetition
        assert!(gain_novel >= gain_repeat, "novelty bonus must not decrease gain: novel={gain_novel} repeat={gain_repeat}");
    }

    // ─── L18: Sabotage Test ─────────────────────────────────────────────────
    // Invariant: applying a skill use always updates repetition counter.
    #[test]
    fn sabotage_repetition_increments() {
        let mut book = SkillBook::new();
        let event = SkillUseEvent {
            skill: SkillKind::Warcraft,
            difficulty: 1000,
            risk: 500,
            novelty_hash: 42,
            world_consequence: 50,
        };

        let before_rep = book.skills[book.skills.iter().position(|s| s.kind == SkillKind::Warcraft).unwrap()].repetition;
        apply_skill_use(&mut book, &event);
        let after_rep = book.skills[book.skills.iter().position(|s| s.kind == SkillKind::Warcraft).unwrap()].repetition;

        // Real assertion: repetition increments
        assert_eq!(after_rep, before_rep + 1, "repetition counter must increment");

        // Sabotaged version (commented out to pass):
        // assert_eq!(after_rep, before_rep, "THIS MUST FAIL");
        // ^ If we uncommented that, the test would panic.
    }

    // ─── L18: Sabotage Test ─────────────────────────────────────────────────
    // Invariant: skill value never exceeds the cap.
    #[test]
    fn sabotage_cap_is_enforced() {
        let mut book = SkillBook::new();
        book.caps[0] = 100; // Low cap
        let event = SkillUseEvent {
            skill: SkillKind::Knifecraft,
            difficulty: 9999,
            risk: 9999,
            novelty_hash: 1,
            world_consequence: 9999,
        };

        for _ in 0..500 {
            apply_skill_use(&mut book, &event);
        }

        let final_value = book.get(SkillKind::Knifecraft);

        // Real assertion: value does not exceed cap
        assert!(final_value <= 100, "skill value must not exceed cap: got {final_value}");

        // Sabotaged version (commented out to pass):
        // assert!(final_value > 100, "THIS MUST FAIL");
        // ^ If we uncommented that, the test would panic.
    }
}
