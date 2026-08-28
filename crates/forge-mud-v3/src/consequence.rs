//! The consequence engine — ironroot's character-consequence spine, drained
//! whole from the IRONROOT Design Packet and the ironroot-edict justice code
//! (2026-08-11 ironroot recon fleet; receipts:
//! `ironroot_character_consequence_engine.v1.json` ProgressionQuery 16-byte /
//! GrowthDescriptor 8-byte contracts, Central-Third band law, streak decay
//! ×1/×0.7/×0.3/×0.05; `faction_system.gd:8-156` integer rep tiers + kill
//! cascade; `crime_system.gd:68-82` guard-response ladder;
//! `guard_npc.gd:60-70` era scaling). Two sides, one coin (ARCH000
//! 2026-08-11): the engine here is generic MUD substrate; the faction roster
//! at the bottom is the Ironroot lore layer riding on it.
//!
//! Integer-only (add / sub / shift / compare / permyriad). No RNG inside —
//! callers feed the chaos byte, exactly like [`crate::hermetics::law`].
//! Every packed struct carries a compile-time size gate (L18-provable: break
//! the layout, get `error[E0080]`).

// ── The 16-byte query ────────────────────────────────────────────────────────

/// What a verb did, packed for the engine: 16 unsigned bytes, the packet's
/// exact field order. `repr(C)` so the layout IS the wire format (L08).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C, align(16))]
pub struct ProgressionQuery {
    /// What kind of act (see [`ActionTag`]).
    pub action_tag: u8,
    /// How well it was performed (verb-supplied, 0..=255).
    pub action_intensity: u8,
    /// How hard the target was (biome, law, era — the world's answer).
    pub target_difficulty: u8,
    /// The acting skill's current value.
    pub current_skill: u8,
    /// A supporting skill, or 0.
    pub secondary_skill: u8,
    /// Tool quality (rod, still, tongue), or 0 bare-handed.
    pub tool_quality: u8,
    /// Does this square favour the act (0 = indifferent ground).
    pub zone_affinity: u8,
    /// The celestial window — the operator's moon day here.
    pub celestial: u8,
    /// Local volatility — the sky's intensity bucketed to a byte.
    pub root_flux: u8,
    /// Same-act repetitions since the current broke.
    pub streak: u8,
    /// Accrued tiredness.
    pub fatigue: u8,
    /// Company: 0 alone .. 7 ritual (the packet's eight rooms).
    pub social_context: u8,
    /// Standing with the watching faction, rebased to u8 (128 = even).
    pub reputation: u8,
    /// 0 first-time, 1 known, 2 mastered, 3 exhausted.
    pub discovery_state: u8,
    /// The operator's relation to the land, 0..=255.
    pub root_harmony: u8,
    /// The rare-moment phase byte — dealt by the caller, never rolled here.
    pub chaos_phase: u8,
}

/// The packet's size contract, held by the compiler (L01: the law is a gate).
const _: () = assert!(std::mem::size_of::<ProgressionQuery>() == 16);

impl ProgressionQuery {
    /// The 16 wire bytes, field order exactly as declared.
    pub fn encode(&self) -> [u8; 16] {
        [
            self.action_tag, self.action_intensity, self.target_difficulty,
            self.current_skill, self.secondary_skill, self.tool_quality,
            self.zone_affinity, self.celestial, self.root_flux, self.streak,
            self.fatigue, self.social_context, self.reputation,
            self.discovery_state, self.root_harmony, self.chaos_phase,
        ]
    }

    /// Rebuild from wire bytes — total: every 16-byte array is a query (L07).
    pub fn decode(b: [u8; 16]) -> Self {
        Self {
            action_tag: b[0], action_intensity: b[1], target_difficulty: b[2],
            current_skill: b[3], secondary_skill: b[4], tool_quality: b[5],
            zone_affinity: b[6], celestial: b[7], root_flux: b[8],
            streak: b[9], fatigue: b[10], social_context: b[11],
            reputation: b[12], discovery_state: b[13], root_harmony: b[14],
            chaos_phase: b[15],
        }
    }
}

/// The act families the mud fires today — a subset of the packet's fifteen,
/// values kept packet-stable so a future drain widens without renumbering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTag {
    /// Casting a line (gather current).
    Fish = 0,
    /// Brewing at the still (craft current).
    Craft = 4,
    /// Speaking with the node's people (voice current).
    Speak = 6,
    /// A deed done for pay (force current).
    Quest = 11,
    /// Taking what is not given (force current, watched).
    Steal = 12,
}

// ── The 8-byte descriptor ────────────────────────────────────────────────────

/// Flag: the acting skill rose.
pub const FLAG_SKILL_UP: u8 = 1;
/// Flag: a secret surfaced (discovery_id names it).
pub const FLAG_SECRET_FOUND: u8 = 4;
/// Flag: the streak broke this act.
pub const FLAG_STREAK_BROKEN: u8 = 16;
/// Flag: three currents met — a mastery proc.
pub const FLAG_MASTERY: u8 = 32;

/// What the engine answers: 8 bytes, the packet's exact field order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(C, align(8))]
pub struct GrowthDescriptor {
    /// XP for the acting skill.
    pub primary_xp: u8,
    /// XP for the supporting skill.
    pub secondary_xp: u8,
    /// Which discovery, when [`FLAG_SECRET_FOUND`] is up.
    pub discovery_id: u8,
    /// Standing change with the watching faction (signed).
    pub reputation_delta: i8,
    /// Tiredness applied.
    pub fatigue_cost: u8,
    /// Rare-event code, 0 for none.
    pub rare_event: u8,
    /// The land leans (signed) — fed to the sky, never spoken.
    pub root_shift: i8,
    /// Flag bits (see the FLAG_ consts).
    pub flags: u8,
}

/// The packet's size contract, held by the compiler.
const _: () = assert!(std::mem::size_of::<GrowthDescriptor>() == 8);

impl GrowthDescriptor {
    /// The 8 wire bytes; signed fields ride as two's-complement.
    pub fn encode(&self) -> [u8; 8] {
        [
            self.primary_xp, self.secondary_xp, self.discovery_id,
            self.reputation_delta as u8, self.fatigue_cost, self.rare_event,
            self.root_shift as u8, self.flags,
        ]
    }

    /// Rebuild from wire bytes — total (L07).
    pub fn decode(b: [u8; 8]) -> Self {
        Self {
            primary_xp: b[0], secondary_xp: b[1], discovery_id: b[2],
            reputation_delta: b[3] as i8, fatigue_cost: b[4], rare_event: b[5],
            root_shift: b[6] as i8, flags: b[7],
        }
    }
}

// ── The Central-Third band law ───────────────────────────────────────────────

/// Half-width of the sweet band: a third of the u8 window, halved (255/6).
const BAND_HALF: u8 = 42;

/// Where an act fell against the actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// Beneath the hand — nothing to learn.
    TooEasy,
    /// Inside the central third — the learning band.
    SweetSpot,
    /// Beyond the hand — failure, unless it teaches.
    TooHard,
}

/// The packet's Central-Third rule: growth only where difficulty sits within
/// `BAND_HALF` of the current skill.
pub fn band(difficulty: u8, skill: u8) -> Band {
    if difficulty.saturating_add(BAND_HALF) < skill {
        Band::TooEasy
    } else if difficulty > skill.saturating_add(BAND_HALF) {
        Band::TooHard
    } else {
        Band::SweetSpot
    }
}

/// Streak decay in permyriad — the packet's ladder: 0..10 reps full, 10..30
/// ×0.7, 30..60 ×0.3, 60+ ×0.05.
pub fn streak_pmy(streak: u8) -> u32 {
    match streak {
        0..=9 => 10_000,
        10..=29 => 7_000,
        30..=59 => 3_000,
        _ => 500,
    }
}

/// Resolve one act. Pure and total: same query, same answer, forever.
pub fn resolve(q: &ProgressionQuery) -> GrowthDescriptor {
    let mut d = GrowthDescriptor::default();
    d.fatigue_cost = q.action_intensity / 16 + q.streak / 8;
    match band(q.target_difficulty, q.current_skill) {
        Band::TooEasy => {
            // 0-1 XP: only real effort earns the single point.
            d.primary_xp = (q.action_intensity >= 128) as u8;
        }
        Band::SweetSpot => {
            let base = 1 + (q.action_intensity as u32 >> 5) + (q.target_difficulty as u32 >> 6);
            d.primary_xp = ((base * streak_pmy(q.streak)) / 10_000).min(255) as u8;
            if d.primary_xp > 0 {
                d.flags |= FLAG_SKILL_UP;
            }
            if q.secondary_skill > 0 {
                d.secondary_xp = d.primary_xp >> 1;
            }
            // Speech and trade in company carry standing (packet: social
            // progression rides witnessed acts).
            if q.action_tag == ActionTag::Speak as u8 && q.social_context >= 1 {
                d.reputation_delta = 1;
            }
            // The land leans with the harmonious, against the discordant
            // (packet root-harmony bands 51..=100 / 201..=255).
            d.root_shift = match q.root_harmony {
                51..=100 => 1,
                201..=255 => -1,
                _ => 0,
            };
            // Three currents meeting is a mastery proc (Tas-de-Charge).
            if q.secondary_skill > 0 && q.tool_quality >= 192 && q.streak == 0 {
                d.flags |= FLAG_MASTERY;
            }
        }
        Band::TooHard => {
            // No XP — but a near-boundary failure can surface a secret
            // (chaos XOR difficulty < 16, the vibration-law shape).
            if q.chaos_phase ^ q.target_difficulty < 16 {
                d.flags |= FLAG_SECRET_FOUND;
                d.discovery_id = q.target_difficulty;
            }
        }
    }
    if q.streak >= 10 && d.primary_xp > 0 {
        d.flags |= FLAG_STREAK_BROKEN & 0; // streak endures until the act changes
    }
    d
}

// ── Standing: the nine-tier ladder and the kill cascade ─────────────────────

/// The nine standing tiers, coldest first — integer thresholds drained from
/// `faction_system.gd:8-12` (KOS at -800, hunter at -1500 held verbatim).
pub const REP_TIERS: [(i16, &str); 9] = [
    (-2000, "blood-sworn foe"),
    (-1500, "hunted"),
    (-800, "kill-on-sight"),
    (-200, "distrusted"),
    (200, "unremarked"),
    (600, "known"),
    (1200, "welcome"),
    (1800, "honoured"),
    (i16::MAX, "kin"),
];

/// The tier a standing value sits in (index into [`REP_TIERS`]).
pub fn standing_tier(rep: i16) -> usize {
    REP_TIERS.iter().position(|&(ceil, _)| rep < ceil).unwrap_or(REP_TIERS.len() - 1)
}

/// The word for a standing value.
pub fn standing_word(rep: i16) -> &'static str {
    REP_TIERS[standing_tier(rep)].1
}

/// The kill cascade, permyriad-exact from `faction_system.gd:126-156`:
/// the wronged lose `amount`; their enemies gain half; their allies lose
/// three-tenths; the unaligned warm a tenth. Integer end to end.
pub fn cascade(standings: &mut [i16; FACTION_COUNT], wronged: usize, amount: i16) {
    for (i, s) in standings.iter_mut().enumerate() {
        let delta: i32 = if i == wronged {
            -(amount as i32)
        } else if i == FACTIONS[wronged].enemy {
            (amount as i32 * 5_000) / 10_000
        } else if i == FACTIONS[wronged].ally {
            -(amount as i32 * 3_000) / 10_000
        } else {
            (amount as i32 * 1_000) / 10_000
        };
        *s = (*s as i32 + delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

// ── Justice: town law and the guard ladder ──────────────────────────────────

/// How the watch answers a flagged act — the ladder drained from
/// `crime_system.gd:68-82`, era-tempered per `guard_npc.gd:60-70`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardResponse {
    /// No law here, or nothing owed.
    Unanswered,
    /// The watch marks the act and hunts — survivable, costly.
    Hunted,
    /// High law answers a nameless thief with the blade.
    Struck,
}

/// The node's law level, 0..=100, dealt once from the seed like every other
/// face of the node.
pub fn law_level(node_seed: u64) -> u8 {
    (crate::operator::seed_hash(&[&node_seed.to_le_bytes(), b"law"]) % 101) as u8
}

/// The guard ladder: lawless ground never answers; the Ancient era has no
/// watch to answer; high law strikes, lower law hunts; the Decay era's watch
/// is too thin to strike (it hunts instead); the Void era's watch glitches —
/// it answers by the chaos byte, blade or nothing.
pub fn guard_response(law: u8, era: crate::weather::Era, heat: u16, chaos: u8) -> GuardResponse {
    use crate::weather::Era;
    if heat == 0 || law < 10 || era == Era::Ancient {
        return GuardResponse::Unanswered;
    }
    let strikes = law >= 80;
    match era {
        Era::Ancient => GuardResponse::Unanswered,
        Era::Golden => if strikes { GuardResponse::Struck } else { GuardResponse::Hunted },
        Era::Decay => GuardResponse::Hunted,
        Era::Void => {
            if chaos & 1 == 0 {
                GuardResponse::Struck
            } else {
                GuardResponse::Unanswered
            }
        }
    }
}

// ── The Ironroot lore layer (the other side of the coin) ────────────────────

/// How many factions watch a node.
pub const FACTION_COUNT: usize = 5;

/// One faction: its name, line, and its braid (ally / enemy indices).
#[derive(Debug, Clone, Copy)]
pub struct Faction {
    /// The faction's name.
    pub name: &'static str,
    /// One line of its temper.
    pub line: &'static str,
    /// Index of its natural ally.
    pub ally: usize,
    /// Index of its natural enemy.
    pub enemy: usize,
}

/// The five Ironroot factions (names drained from `faction_system.gd:21-97`;
/// the ally/enemy braid is a v3 ruling, Authored — the donor pairs were not
/// cited by the recon lane).
pub const FACTIONS: [Faction; FACTION_COUNT] = [
    Faction { name: "the Thornguard", line: "law carried as a drawn line", ally: 1, enemy: 4 },
    Faction { name: "the Verdant Pact", line: "the land's own patience", ally: 0, enemy: 2 },
    Faction { name: "the Ashborn Legion", line: "order burned into ground", ally: 3, enemy: 1 },
    Faction { name: "the Pallid Court", line: "memory kept cold and long", ally: 2, enemy: 0 },
    Faction { name: "the Null Communion", line: "the quiet between names", ally: 4, enemy: 0 },
];

/// The faction holding a node's town, dealt from the seed.
pub fn town_faction(node_seed: u64) -> usize {
    (crate::operator::seed_hash(&[&node_seed.to_le_bytes(), b"faction"]) % FACTION_COUNT as u64)
        as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weather::Era;

    /// L07 over both wire codecs: interior, zero and edge values survive
    /// encode→decode byte-exact.
    #[test]
    fn the_query_and_descriptor_codecs_are_bijections() {
        let q = ProgressionQuery {
            action_tag: 12, action_intensity: 255, target_difficulty: 0,
            current_skill: 77, secondary_skill: 1, tool_quality: 200,
            zone_affinity: 3, celestial: 27, root_flux: 128, streak: 61,
            fatigue: 9, social_context: 7, reputation: 128,
            discovery_state: 2, root_harmony: 201, chaos_phase: 0xAB,
        };
        assert_eq!(ProgressionQuery::decode(q.encode()), q);
        assert_eq!(ProgressionQuery::decode([0; 16]), ProgressionQuery::default());
        let d = GrowthDescriptor {
            primary_xp: 255, secondary_xp: 4, discovery_id: 9,
            reputation_delta: -128, fatigue_cost: 1, rare_event: 0,
            root_shift: -1, flags: 0xFF,
        };
        assert_eq!(GrowthDescriptor::decode(d.encode()), d);
        // The negative fields ride two's-complement and come home signed.
        assert_eq!(GrowthDescriptor::decode(d.encode()).reputation_delta, -128);
    }

    /// The Central-Third law: growth only in the band; the easy earn at most
    /// one point; the hard earn nothing but may learn a secret.
    #[test]
    fn the_central_third_band_governs_growth() {
        assert_eq!(band(10, 200), Band::TooEasy);
        assert_eq!(band(200, 10), Band::TooHard);
        assert_eq!(band(100, 100), Band::SweetSpot);
        assert_eq!(band(100u8.saturating_add(BAND_HALF), 100), Band::SweetSpot);
        let easy = resolve(&ProgressionQuery {
            target_difficulty: 10, current_skill: 200, action_intensity: 255,
            ..Default::default()
        });
        assert!(easy.primary_xp <= 1, "the easy road paid {}", easy.primary_xp);
        let sweet = resolve(&ProgressionQuery {
            target_difficulty: 100, current_skill: 100, action_intensity: 128,
            ..Default::default()
        });
        assert!(sweet.primary_xp >= 1 && sweet.flags & FLAG_SKILL_UP != 0);
        let hard = resolve(&ProgressionQuery {
            target_difficulty: 250, current_skill: 10, chaos_phase: 250,
            ..Default::default()
        });
        assert_eq!(hard.primary_xp, 0);
        assert!(hard.flags & FLAG_SECRET_FOUND != 0, "a near miss must teach");
    }

    /// The streak ladder decays exactly on the packet's steps.
    #[test]
    fn the_streak_ladder_holds_the_packets_steps() {
        assert_eq!(streak_pmy(0), 10_000);
        assert_eq!(streak_pmy(9), 10_000);
        assert_eq!(streak_pmy(10), 7_000);
        assert_eq!(streak_pmy(29), 7_000);
        assert_eq!(streak_pmy(30), 3_000);
        assert_eq!(streak_pmy(59), 3_000);
        assert_eq!(streak_pmy(60), 500);
        assert_eq!(streak_pmy(255), 500);
    }

    /// The engine is a pure function: same query, same answer.
    #[test]
    fn the_engine_is_deterministic() {
        let q = ProgressionQuery {
            action_tag: 0, action_intensity: 99, target_difficulty: 80,
            current_skill: 90, streak: 12, root_harmony: 60,
            ..Default::default()
        };
        assert_eq!(resolve(&q), resolve(&q));
    }

    /// The kill cascade: the wronged fall by the whole amount, their enemy
    /// warms by half, their ally cools by three-tenths, the rest warm a
    /// tenth — integers, no drift.
    #[test]
    fn the_cascade_is_permyriad_exact() {
        let mut s = [0i16; FACTION_COUNT];
        cascade(&mut s, 0, 100); // wrong the Thornguard
        assert_eq!(s[0], -100);
        assert_eq!(s[FACTIONS[0].enemy], 50);
        assert_eq!(s[FACTIONS[0].ally], -30);
        let unaligned =
            (0..FACTION_COUNT).find(|&i| i != 0 && i != FACTIONS[0].enemy && i != FACTIONS[0].ally);
        if let Some(i) = unaligned {
            assert_eq!(s[i], 10);
        }
    }

    /// The standing ladder: thresholds hold at the drained KOS/hunter marks.
    #[test]
    fn the_standing_ladder_holds_the_drained_marks() {
        assert_eq!(standing_word(-2001), "blood-sworn foe");
        assert_eq!(standing_word(-1600), "hunted");
        assert_eq!(standing_word(-900), "kill-on-sight");
        assert_eq!(standing_word(0), "unremarked");
        assert_eq!(standing_word(5000), "kin");
    }

    /// The guard ladder end to end: lawless and Ancient ground never answer;
    /// Golden high law strikes; Decay only hunts; Void answers by the chaos
    /// byte; no heat, no answer.
    #[test]
    fn the_guard_ladder_is_the_drained_law() {
        assert_eq!(guard_response(90, Era::Golden, 0, 0), GuardResponse::Unanswered);
        assert_eq!(guard_response(5, Era::Golden, 9, 0), GuardResponse::Unanswered);
        assert_eq!(guard_response(90, Era::Ancient, 9, 0), GuardResponse::Unanswered);
        assert_eq!(guard_response(90, Era::Golden, 9, 0), GuardResponse::Struck);
        assert_eq!(guard_response(40, Era::Golden, 9, 0), GuardResponse::Hunted);
        assert_eq!(guard_response(90, Era::Decay, 9, 0), GuardResponse::Hunted);
        assert_eq!(guard_response(90, Era::Void, 9, 2), GuardResponse::Struck);
        assert_eq!(guard_response(90, Era::Void, 9, 3), GuardResponse::Unanswered);
    }

    /// Law and the town's faction are the seed: dealt once, dealt forever.
    #[test]
    fn law_and_faction_are_the_seed() {
        for seed in [1u64, 0xDEAD_BEEF, u64::MAX] {
            assert_eq!(law_level(seed), law_level(seed));
            assert!(law_level(seed) <= 100);
            assert_eq!(town_faction(seed), town_faction(seed));
            assert!(town_faction(seed) < FACTION_COUNT);
        }
        // Different seeds move the law across many nodes (no dead dial).
        let moved = (0..64u64)
            .map(|s| law_level(crate::operator::seed_hash(&[&s.to_le_bytes()])))
            .collect::<std::collections::HashSet<_>>();
        assert!(moved.len() > 8, "the law dial is stuck: {} values", moved.len());
    }
}
