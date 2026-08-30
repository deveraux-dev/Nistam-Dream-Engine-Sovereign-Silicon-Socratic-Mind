//! Ported verbatim from F:\NewRepo\crates\forge-game-systems\src\asp_constraints.rs (2026-08-17 truth-hunt lineage port).
//! ASP constraint validation — pure Rust port of astrakey_sieve Python/Clingo.
//!
//! Each `.lp` constraint program becomes a Rust function that validates
//! generated content against design rules. No clingo dependency.
//! Uses the same generate-and-test pattern: derive → validate → retry.

use crate::astrakey_sieve::derivation::derive_seed;
use crate::astrakey_sieve::sieve::{prime_gap, is_prime_index};
use crate::astrakey_sieve::types::{DerivedSeed, SieveResult, SystemID};
use serde::{Deserialize, Serialize};

// ── Result types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveStatus { Sat, Unsat, Exhausted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintResult {
    pub status: SolveStatus,
    pub domain: String,
    pub violations: Vec<String>,
    pub prime_index_used: usize,
    pub retries: u32,
}

// ── Weighted rarity (matches Python _weighted_rarity) ────────────────────────

const RARITY_THRESHOLDS: [u64; 5] = [500, 750, 900, 980, 1000];

fn weighted_rarity(seed: &DerivedSeed, floor_num: u32, num_floors: u32) -> u8 {
    let base_roll = seed.seed_value % 1000;
    let quarter = ((floor_num * 4) / num_floors.max(1)).min(3);
    let bonus = quarter as u64 * 30;
    let mut adjusted = (base_roll + bonus).min(999);
    if floor_num == 1 { adjusted = adjusted.min(RARITY_THRESHOLDS[2] - 1); }
    for (tier, &threshold) in RARITY_THRESHOLDS.iter().enumerate() {
        if adjusted < threshold { return tier as u8; }
    }
    4
}

// ── Loot constraints (port of loot_constraints.lp) ───────────────────────────

pub fn generate_loot_drops(sieve: &SieveResult, num_floors: u32, start: usize) -> Vec<(u32, u8)> {
    let mut drops: Vec<(u32, u8)> = (1..=num_floors).map(|floor| {
        let idx = start + floor as usize - 1;
        let seed = derive_seed(sieve.primes[idx], idx, SystemID::Loot, &format!("floor_{}", floor));
        (floor, weighted_rarity(&seed, floor, num_floors))
    }).collect();

    // Anti-streak: break 3+ consecutive same-tier
    for i in 2..drops.len() {
        if drops[i].1 == drops[i-1].1 && drops[i].1 == drops[i-2].1 {
            let tier = drops[i].1;
            drops[i].1 = if tier >= 3 { tier - 1 } else { (tier + 1).min(4) };
        }
    }
    // Mythic cap: max 1 per 10-floor window
    for i in 0..drops.len() {
        if drops[i].1 == 4 {
            let floor = drops[i].0;
            let start_w = floor.saturating_sub(9);
            let prior = drops[..i].iter().filter(|(f, t)| *t == 4 && *f >= start_w).count();
            if prior >= 1 { drops[i].1 = 3; }
        }
    }
    drops
}

pub fn validate_loot(sieve: &SieveResult, num_floors: u32, start: usize) -> ConstraintResult {
    let drops = generate_loot_drops(sieve, num_floors, start);
    let mut violations = Vec::new();

    // C1: No 3+ consecutive same-tier (should be fixed by anti-streak, but verify)
    for i in 2..drops.len() {
        if drops[i].1 == drops[i-1].1 && drops[i].1 == drops[i-2].1 {
            violations.push(format!("3+ consecutive tier {} at floor {}", drops[i].1, drops[i].0));
        }
    }
    // C2: Mythic max 1 per 10-floor window
    for w in 0..drops.len().saturating_sub(9) {
        let mythics = drops[w..w+10].iter().filter(|(_, t)| *t == 4).count();
        if mythics > 1 { violations.push(format!("{}+ mythics in window starting floor {}", mythics, drops[w].0)); }
    }
    // C3: No epic/mythic on floor 1
    if let Some((_, t)) = drops.first() {
        if *t >= 3 { violations.push(format!("tier {} on floor 1", t)); }
    }
    // C4: Common >= 25% (for 10+ floors)
    if num_floors >= 10 {
        let common = drops.iter().filter(|(_, t)| *t == 0).count();
        if common * 100 < drops.len() * 25 {
            violations.push(format!("common {}% < 25%", common * 100 / drops.len().max(1)));
        }
    }
    // C5: At least 2 distinct tiers in any 5-floor window
    for w in 0..drops.len().saturating_sub(4) {
        let distinct: std::collections::HashSet<u8> = drops[w..w+5].iter().map(|(_, t)| *t).collect();
        if distinct.len() < 2 { violations.push(format!("< 2 tiers in 5-floor window at {}", drops[w].0)); }
    }

    let status = if violations.is_empty() { SolveStatus::Sat } else { SolveStatus::Unsat };
    ConstraintResult { status, domain: "loot".into(), violations, prime_index_used: start, retries: 0 }
}

// ── Boss constraints (port of boss_constraints.lp) ───────────────────────────

pub struct BossAssignment {
    pub floor: u32,
    pub boss_id: u64,
    pub difficulty: u32,
    pub affixes: Vec<u64>,
}

pub fn generate_bosses(sieve: &SieveResult, boss_floors: &[u32], pool_size: u64, start: usize) -> Vec<BossAssignment> {
    let mut assignments = Vec::new();
    let total = boss_floors.len();

    for (i, &floor) in boss_floors.iter().enumerate() {
        let idx = start + i;
        let seed = derive_seed(sieve.primes[idx], idx, SystemID::Bosses, &format!("boss_floor_{}", floor));
        let mut boss_id = seed.seed_value % pool_size;

        // Avoid repeat within last 3 assignments
        let recent: Vec<u64> = assignments.iter().rev().take(3).map(|a: &BossAssignment| a.boss_id).collect();
        let mut attempts = 0;
        while recent.contains(&boss_id) && attempts < pool_size {
            boss_id = (boss_id + 1) % pool_size;
            attempts += 1;
        }

        let gap = if idx + 1 < sieve.primes.len() { prime_gap(sieve, idx) } else { 2 };
        let base_diff = ((i / 2) + 1).min(10) as u32;
        let spike = if gap >= 6 { 1 } else { 0 };
        let difficulty = (base_diff + spike).min(10);

        let affix_seed = derive_seed(sieve.primes[idx], idx, SystemID::Bosses, &format!("affix_{}", floor));
        let third = total / 3;
        let num_affixes = if i < third { 0 } else if i < 2 * third { 1 } else { 1 + (affix_seed.seed_value % 2) as usize };
        let mut affixes = Vec::new();
        for a in 0..num_affixes.min(2) {
            let aseed = derive_seed(sieve.primes[idx], idx, SystemID::Bosses, &format!("affix_{}_{}", floor, a));
            affixes.push(aseed.seed_value % 8);
        }

        assignments.push(BossAssignment { floor, boss_id, difficulty, affixes });
    }
    assignments
}

pub fn validate_bosses(sieve: &SieveResult, boss_floors: &[u32], pool_size: u64, start: usize) -> ConstraintResult {
    let bosses = generate_bosses(sieve, boss_floors, pool_size, start);
    let mut violations = Vec::new();

    // C1: No boss repeat within 3 floors
    for i in 0..bosses.len() {
        for j in (i+1)..bosses.len() {
            if bosses[j].floor - bosses[i].floor < 4 && bosses[i].boss_id == bosses[j].boss_id {
                violations.push(format!("boss {} repeats at floors {} and {}", bosses[i].boss_id, bosses[i].floor, bosses[j].floor));
            }
        }
    }
    // C2: Difficulty non-decreasing (allow 1 dip per 10)
    for w in 0..bosses.len().saturating_sub(9) {
        let dips = (w..w+9).filter(|&k| k + 1 < bosses.len() && bosses[k+1].difficulty < bosses[k].difficulty).count();
        if dips > 1 { violations.push(format!("{}+ difficulty dips in 10-boss window at {}", dips, bosses[w].floor)); }
    }
    // C3: Max 2 affixes per encounter
    for b in &bosses {
        if b.affixes.len() > 2 { violations.push(format!("{}+ affixes on floor {}", b.affixes.len(), b.floor)); }
    }
    // C4: >= 3 unique bosses in any 5-boss span
    for w in 0..bosses.len().saturating_sub(4) {
        let unique: std::collections::HashSet<u64> = bosses[w..w+5].iter().map(|b| b.boss_id).collect();
        if unique.len() < 3 { violations.push(format!("< 3 unique bosses in 5-span at {}", bosses[w].floor)); }
    }

    let status = if violations.is_empty() { SolveStatus::Sat } else { SolveStatus::Unsat };
    ConstraintResult { status, domain: "bosses".into(), violations, prime_index_used: start, retries: 0 }
}

// ── Level constraints (port of level_constraints.lp) ─────────────────────────

pub struct LevelLayout {
    pub floor: u32,
    pub room_count: usize,
    pub connections: Vec<(usize, usize)>,
    pub entrance: usize,
    pub exit: usize,
    pub difficulty: u32,
}

pub fn validate_level(level: &LevelLayout) -> Vec<String> {
    let mut violations = Vec::new();

    if level.room_count < 3 { violations.push(format!("floor {} has {} rooms < 3", level.floor, level.room_count)); }
    if level.entrance == level.exit { violations.push(format!("floor {} entrance == exit", level.floor)); }

    // Reachability via BFS
    let mut reachable = vec![false; level.room_count];
    let mut queue = vec![level.entrance];
    reachable[level.entrance] = true;
    while let Some(r) = queue.pop() {
        for &(a, b) in &level.connections {
            let neighbor = if a == r { Some(b) } else if b == r { Some(a) } else { None };
            if let Some(n) = neighbor {
                if n < level.room_count && !reachable[n] {
                    reachable[n] = true;
                    queue.push(n);
                }
            }
        }
    }
    if !reachable[level.exit] { violations.push(format!("floor {} exit unreachable", level.floor)); }
    let isolated: Vec<usize> = (0..level.room_count).filter(|&r| !reachable[r]).collect();
    if !isolated.is_empty() { violations.push(format!("floor {} isolated rooms: {:?}", level.floor, isolated)); }

    violations
}

// ── Secrets constraints (port of secrets_constraints.lp) ─────────────────────

pub fn generate_secrets(sieve: &SieveResult, num_floors: u32, start: usize) -> Vec<(u32, u8, u8)> {
    let mut secrets = Vec::new();
    for floor in 1..=num_floors {
        let idx = start + floor as usize - 1;
        if idx + 1 >= sieve.primes.len() { break; }
        let prime = sieve.primes[idx];
        let gap = prime_gap(sieve, idx);
        if is_prime_index(sieve, prime) || gap >= 6 {
            let seed = derive_seed(prime, idx, SystemID::Secrets, &format!("secret_{}", floor));
            let stype = (seed.seed_value % 4) as u8;
            let rarity = weighted_rarity(&seed, floor, num_floors);
            secrets.push((floor, stype, rarity));
        }
    }
    // Anti-adjacent same type
    for i in 1..secrets.len() {
        if secrets[i].0.abs_diff(secrets[i-1].0) == 1 && secrets[i].1 == secrets[i-1].1 {
            secrets[i].1 = (secrets[i].1 + 1) % 4;
        }
    }
    // No 15-floor empty window
    if num_floors >= 15 {
        let mut secret_floors: std::collections::HashSet<u32> = secrets.iter().map(|s| s.0).collect();
        for w in 1..=num_floors.saturating_sub(14) {
            if !(w..w+15).any(|f| secret_floors.contains(&f)) {
                let mid = w + 7;
                let idx = start + mid as usize - 1;
                if idx < sieve.primes.len() {
                    secrets.push((mid, 0, 0));
                    secret_floors.insert(mid);
                }
            }
        }
        secrets.sort_by_key(|s| s.0);
    }
    // At least 1 rare+ per 20-floor window
    if num_floors >= 20 {
        for w in 1..=num_floors.saturating_sub(19) {
            let has_rare = secrets.iter().any(|(f, _, r)| *f >= w && *f < w + 20 && *r >= 2);
            if !has_rare {
                if let Some(s) = secrets.iter_mut().find(|(f, _, r)| *f >= w && *f < w + 20 && *r < 2) {
                    s.2 = 2;
                }
            }
        }
    }
    secrets
}

// ── Generate-and-test loop ───────────────────────────────────────────────────

/// Try successive prime indices until constraints pass. Mirrors Python validate_with_retry.
pub fn validate_with_retry<F>(
    sieve: &SieveResult,
    start: usize,
    max_retries: u32,
    domain: &str,
    validator: F,
) -> ConstraintResult
where
    F: Fn(&SieveResult, usize) -> ConstraintResult,
{
    for offset in 0..max_retries {
        let idx = start + offset as usize;
        if idx >= sieve.primes.len() { break; }
        let mut result = validator(sieve, idx);
        result.retries = offset;
        if result.status == SolveStatus::Sat { return result; }
    }
    ConstraintResult {
        status: SolveStatus::Exhausted,
        domain: domain.into(),
        violations: vec![format!("exhausted {} retries from index {}", max_retries, start)],
        prime_index_used: start + max_retries as usize - 1,
        retries: max_retries,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astrakey_sieve::sieve::sieve_of_eratosthenes;

    fn sieve_1000() -> SieveResult { sieve_of_eratosthenes(1000) }

    #[test]
    fn loot_20_floors_is_sat() {
        let s = sieve_1000();
        let r = validate_loot(&s, 20, 0);
        assert_eq!(r.status, SolveStatus::Sat, "violations: {:?}", r.violations);
    }

    #[test]
    fn loot_no_epic_floor_1() {
        let s = sieve_1000();
        let drops = generate_loot_drops(&s, 50, 0);
        assert!(drops[0].1 < 3, "floor 1 got tier {}", drops[0].1);
    }

    #[test]
    fn loot_no_triple_streak() {
        let s = sieve_1000();
        let drops = generate_loot_drops(&s, 100, 0);
        for i in 2..drops.len() {
            assert!(
                !(drops[i].1 == drops[i-1].1 && drops[i].1 == drops[i-2].1),
                "triple streak tier {} at floor {}", drops[i].1, drops[i].0
            );
        }
    }

    #[test]
    fn bosses_10_floors_is_sat() {
        let s = sieve_1000();
        let floors: Vec<u32> = (1..=10).collect();
        let r = validate_bosses(&s, &floors, 8, 0);
        assert_eq!(r.status, SolveStatus::Sat, "violations: {:?}", r.violations);
    }

    #[test]
    fn bosses_no_repeat_within_3() {
        let s = sieve_1000();
        let floors: Vec<u32> = (1..=20).collect();
        let bosses = generate_bosses(&s, &floors, 8, 0);
        for i in 0..bosses.len() {
            for j in (i+1)..bosses.len() {
                if bosses[j].floor - bosses[i].floor < 4 {
                    assert_ne!(bosses[i].boss_id, bosses[j].boss_id,
                        "repeat boss {} at floors {} and {}", bosses[i].boss_id, bosses[i].floor, bosses[j].floor);
                }
            }
        }
    }

    #[test]
    fn bosses_max_2_affixes() {
        let s = sieve_1000();
        let floors: Vec<u32> = (1..=30).collect();
        let bosses = generate_bosses(&s, &floors, 8, 0);
        for b in &bosses {
            assert!(b.affixes.len() <= 2, "floor {} has {} affixes", b.floor, b.affixes.len());
        }
    }

    #[test]
    fn level_reachability() {
        let level = LevelLayout {
            floor: 1, room_count: 5, entrance: 0, exit: 4, difficulty: 1,
            connections: vec![(0, 1), (1, 2), (2, 3), (3, 4)],
        };
        assert!(validate_level(&level).is_empty());
    }

    #[test]
    fn level_unreachable_exit() {
        let level = LevelLayout {
            floor: 1, room_count: 5, entrance: 0, exit: 4, difficulty: 1,
            connections: vec![(0, 1), (1, 2)], // 3 and 4 disconnected
        };
        let v = validate_level(&level);
        assert!(v.iter().any(|s| s.contains("unreachable")));
    }

    #[test]
    fn secrets_no_15_floor_gap() {
        let s = sieve_1000();
        let secrets = generate_secrets(&s, 100, 0);
        let floors: std::collections::HashSet<u32> = secrets.iter().map(|s| s.0).collect();
        for w in 1..=86 {
            assert!((w..w+15).any(|f| floors.contains(&f)), "empty 15-floor window at {}", w);
        }
    }

    #[test]
    fn validate_with_retry_finds_sat() {
        let s = sieve_1000();
        let r = validate_with_retry(&s, 0, 10, "loot", |sieve, idx| {
            validate_loot(sieve, 20, idx)
        });
        assert_eq!(r.status, SolveStatus::Sat);
    }

    #[test]
    fn weighted_rarity_floor1_no_epic() {
        let s = sieve_1000();
        for i in 0..50 {
            let seed = derive_seed(s.primes[i], i, SystemID::Loot, "floor_1");
            let tier = weighted_rarity(&seed, 1, 100);
            assert!(tier < 3, "floor 1 got tier {} at prime index {}", tier, i);
        }
    }

    #[test]
    fn weighted_rarity_late_floors_trend_higher() {
        let s = sieve_1000();
        let mut early_sum = 0u64;
        let mut late_sum = 0u64;
        for i in 0..50 {
            let seed = derive_seed(s.primes[i], i, SystemID::Loot, &format!("floor_{}", i + 1));
            early_sum += weighted_rarity(&seed, (i + 1) as u32, 100) as u64;
        }
        for i in 50..100 {
            let seed = derive_seed(s.primes[i], i, SystemID::Loot, &format!("floor_{}", i + 1));
            late_sum += weighted_rarity(&seed, (i + 1) as u32, 100) as u64;
        }
        assert!(late_sum >= early_sum, "late {} should >= early {}", late_sum, early_sum);
    }
}
