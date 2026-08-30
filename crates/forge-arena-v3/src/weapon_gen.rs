//! Weapon proc-gen harness — deterministic weapon generation from an authored corpus.
//!
//! Reads a corpus of authored `JsonItem` weapons (e.g. an Act's weapon set),
//! extracts per-tier priors (damage band, stat magnitude cap, element/material
//! pools, freq_byte band, socket/durability range), then rolls new in-distribution
//! weapons from a seed via the arena_core `ProcRng`. Integer-only, deterministic,
//! zero new deps.
//!
//! Firewall note: the engine stays corpus-agnostic. The game feeds the authored
//! `&[JsonItem]`; this module never bakes in cartridge data.

use super::item_loader::{JsonItem, JsonStats, JsonDamage, JsonDefense, JsonDurability};
use super::procgen::ProcRng;

/// Per-tier generation priors extracted from an authored corpus.
#[derive(Clone, Debug, Default)]
pub struct WeaponPriors {
    pub tier: u8,
    pub dmg_min: i32,
    pub dmg_max: i32,
    pub stat_mag_max: i32,      // largest single-stat magnitude observed at this tier
    pub elements: Vec<String>,  // observed element pool
    pub materials: Vec<String>, // observed material pool
    pub freq_min: u8,
    pub freq_max: u8,
    pub socket_max: u8,
    pub dur_max: u16,
    pub samples: u32,
}

/// Extract per-tier priors from an authored weapon corpus.
/// Returns one `WeaponPriors` per tier present, ascending by tier.
pub fn extract_priors(corpus: &[JsonItem]) -> Vec<WeaponPriors> {
    let mut tiers: Vec<WeaponPriors> = Vec::new();
    for w in corpus {
        // Find-or-create by index (avoids an iter_mut/push borrow conflict).
        let idx = match tiers.iter().position(|p| p.tier == w.tier) {
            Some(i) => i,
            None => {
                tiers.push(WeaponPriors {
                    tier: w.tier,
                    dmg_min: i32::MAX,
                    dmg_max: i32::MIN,
                    freq_min: u8::MAX,
                    freq_max: u8::MIN,
                    ..Default::default()
                });
                tiers.len() - 1
            }
        };
        let p = &mut tiers[idx];
        p.samples += 1;
        p.dmg_min = p.dmg_min.min(w.damage.base);
        p.dmg_max = p.dmg_max.max(w.damage.base);
        p.freq_min = p.freq_min.min(w.damage.freq_byte);
        p.freq_max = p.freq_max.max(w.damage.freq_byte);
        p.socket_max = p.socket_max.max(w.sockets);
        p.dur_max = p.dur_max.max(w.durability.max);
        p.stat_mag_max = p.stat_mag_max.max(stat_mag(&w.stats));
        if !w.damage.element.is_empty() && !p.elements.contains(&w.damage.element) {
            p.elements.push(w.damage.element.clone());
        }
        if !w.material.is_empty() && !p.materials.contains(&w.material) {
            p.materials.push(w.material.clone());
        }
    }
    tiers.sort_by_key(|p| p.tier);
    tiers
}

/// Largest single-stat magnitude across the 8-stat block.
fn stat_mag(s: &JsonStats) -> i32 {
    [
        s.vigor, s.momentum, s.logic_depth, s.shadow_weight,
        s.tarnish, s.resonance, s.guilt, s.clarity,
    ]
    .iter()
    .map(|v| v.abs())
    .max()
    .unwrap_or(0)
}

/// Deterministically generate a weapon for `tier` from the extracted priors.
/// Same seed + same priors + same tier => identical `JsonItem`.
///
/// Precondition: `priors` is non-empty (call `extract_priors` on a non-empty corpus).
pub fn generate(rng: &mut ProcRng, priors: &[WeaponPriors], tier: u8) -> JsonItem {
    let p = pick_tier(priors, tier);

    let base = roll_i32(rng, p.dmg_min, p.dmg_max);
    let freq_byte = roll_u8(rng, p.freq_min, p.freq_max);
    let element = pick(rng, &p.elements, "earth");
    let material = pick(rng, &p.materials, "IRON");

    let cap = p.stat_mag_max.max(1);
    let stats = JsonStats {
        vigor:         roll_i32(rng, -cap, cap),
        momentum:      roll_i32(rng, -cap, cap),
        logic_depth:   roll_i32(rng, -cap, cap),
        shadow_weight: roll_i32(rng, -cap, cap),
        tarnish:       roll_i32(rng, -cap, cap),
        resonance:     roll_i32(rng, -cap, cap),
        guilt:         roll_i32(rng, -cap, cap),
        clarity:       roll_i32(rng, -cap, cap),
    };

    let sockets = roll_u8(rng, 0, p.socket_max);
    let dur_floor = (p.dur_max / 2).max(1);
    let dur_span = p.dur_max.saturating_sub(dur_floor).min(255) as u8;
    let dur_max = dur_floor + roll_u8(rng, 0, dur_span) as u16;
    let serial = rng.next_u64() as u32;

    JsonItem {
        id: format!("wpn_gen_t{}_{:08x}", p.tier, serial),
        name: forged_name(&material, &element),
        slot: 0,
        tier: p.tier,
        level_req: p.tier,
        stats,
        damage: JsonDamage { base, element, freq_byte },
        defense: JsonDefense::default(),
        tags: Vec::new(),
        material,
        gender: "active".into(),
        durability: JsonDurability { current: dur_max, max: dur_max },
        sockets,
        description: "Forged from the pattern of older weapons.".into(),
    }
}

/// The priors for `tier`, or the nearest available tier as a fallback.
fn pick_tier(priors: &[WeaponPriors], tier: u8) -> &WeaponPriors {
    priors
        .iter()
        .find(|p| p.tier == tier)
        .or_else(|| priors.iter().min_by_key(|p| (p.tier as i32 - tier as i32).abs()))
        .expect("weapon_gen::generate requires non-empty priors")
}

fn roll_i32(rng: &mut ProcRng, lo: i32, hi: i32) -> i32 {
    if hi <= lo { return lo; }
    lo + rng.next_range((hi - lo + 1) as u32) as i32
}

fn roll_u8(rng: &mut ProcRng, lo: u8, hi: u8) -> u8 {
    if hi <= lo { return lo; }
    (lo as u32 + rng.next_range((hi - lo) as u32 + 1)) as u8
}

fn pick(rng: &mut ProcRng, pool: &[String], fallback: &str) -> String {
    if pool.is_empty() { return fallback.to_string(); }
    pool[rng.next_range(pool.len() as u32) as usize].clone()
}

fn forged_name(material: &str, element: &str) -> String {
    format!("{} {} Edge", material, element)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jw(tier: u8, base: i32, element: &str, freq: u8, material: &str, vigor: i32) -> JsonItem {
        JsonItem {
            id: format!("fix_{}_{}", tier, base),
            name: "Fixture".into(),
            slot: 0,
            tier,
            level_req: tier,
            stats: JsonStats { vigor, ..Default::default() },
            damage: JsonDamage { base, element: element.into(), freq_byte: freq },
            defense: JsonDefense::default(),
            tags: Vec::new(),
            material: material.into(),
            gender: "active".into(),
            durability: JsonDurability { current: 50, max: 50 },
            sockets: 0,
            description: String::new(),
        }
    }

    fn corpus() -> Vec<JsonItem> {
        vec![
            jw(0, 8, "blood", 20, "IRON", 0),
            jw(0, 18, "earth", 32, "IRON", 0),
            jw(2, 38, "fire", 64, "RUNE_METAL", 8),
            jw(2, 44, "blood", 170, "BONE", 12),
        ]
    }

    #[test]
    fn priors_capture_tier_bands() {
        let p = extract_priors(&corpus());
        let t2 = p.iter().find(|p| p.tier == 2).expect("tier 2 present");
        assert_eq!((t2.dmg_min, t2.dmg_max), (38, 44));
        assert_eq!((t2.freq_min, t2.freq_max), (64, 170));
        assert_eq!(t2.stat_mag_max, 12);
        assert!(t2.elements.contains(&"fire".to_string()));
        assert!(t2.materials.contains(&"BONE".to_string()));
    }

    #[test]
    fn generated_weapon_in_tier_band() {
        let priors = extract_priors(&corpus());
        for seed in [1u64, 7, 42, 1000, 999_999] {
            let mut rng = ProcRng::new(seed);
            let w = generate(&mut rng, &priors, 2);
            assert!(
                w.damage.base >= 38 && w.damage.base <= 44,
                "seed {seed}: damage {} outside tier-2 band [38,44]", w.damage.base
            );
            assert!(
                w.damage.freq_byte >= 64 && w.damage.freq_byte <= 170,
                "seed {seed}: freq {} outside tier-2 band [64,170]", w.damage.freq_byte
            );
            assert!(
                ["fire", "blood"].contains(&w.damage.element.as_str()),
                "seed {seed}: element {} not in tier-2 pool", w.damage.element
            );
            assert!(["RUNE_METAL", "BONE"].contains(&w.material.as_str()));
            assert_eq!(w.tier, 2);
        }
    }

    #[test]
    fn generation_is_deterministic() {
        let priors = extract_priors(&corpus());
        let mut a = ProcRng::new(12345);
        let mut b = ProcRng::new(12345);
        let wa = generate(&mut a, &priors, 2);
        let wb = generate(&mut b, &priors, 2);
        assert_eq!(wa.damage.base, wb.damage.base);
        assert_eq!(wa.damage.element, wb.damage.element);
        assert_eq!(wa.material, wb.material);
        assert_eq!(wa.id, wb.id);
        assert_eq!(wa.stats.vigor, wb.stats.vigor);
    }
}
