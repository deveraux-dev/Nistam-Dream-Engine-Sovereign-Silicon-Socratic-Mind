//! HMAC-SHA256 seed derivation layer.
//! Transforms raw prime sieve outputs into per-system isolated seeds.
//!
//! Architecture: master_prime + system_id + context
//!   → HMAC-SHA256(key=prime_bytes, msg=system_id||context)
//!   → truncate to 64-bit unsigned integer → DerivedSeed

use sha2::{Digest, Sha256};
use super::types::*;
use forge_core_v3::atom::TritCell5D;

/// Derive a single per-system seed from a master prime.
pub fn derive_seed(master_prime: u64, master_index: usize, system: SystemID, context: &str) -> DerivedSeed {
    let hash = hmac_sha256(master_prime, system.as_str(), context);
    // Truncate to 64-bit: first 16 hex chars = 8 bytes
    let seed_value = u64::from_be_bytes([
        hash[0], hash[1], hash[2], hash[3],
        hash[4], hash[5], hash[6], hash[7],
    ]);
    let derivation_hash = hex_encode(&hash);

    DerivedSeed { system, context: context.to_string(), master_prime, master_index, seed_value, derivation_hash }
}

/// Derive seeds for ALL registered systems from one master prime.
pub fn derive_for_all_systems(sieve: &SieveResult, context: &str, master_index: usize) -> Vec<DerivedSeed> {
    let prime = sieve.primes[master_index];
    SystemID::ALL.iter().map(|&sys| derive_seed(prime, master_index, sys, context)).collect()
}

/// Derive a batch of seeds for one system across multiple contexts.
pub fn derive_batch(sieve: &SieveResult, system: SystemID, contexts: &[String], prime_start_index: usize) -> SeedPack {
    let seeds: Vec<DerivedSeed> = contexts.iter().enumerate().map(|(i, ctx)| {
        let idx = prime_start_index + i;
        derive_seed(sieve.primes[idx], idx, system, ctx)
    }).collect();
    SeedPack { system, master_upper_bound: sieve.upper_bound, seeds, version: 1 }
}

/// Map a derived seed to a rarity tier index [0, num_tiers).
pub fn seed_to_rarity(seed: &DerivedSeed, num_tiers: u8) -> u8 {
    (seed.seed_value % num_tiers as u64) as u8
}

/// Bridge a derived seed to a real card address (aspire.rs
/// `soulword-card-address-bridge`): `seed_value % 256` lands on the SAME
/// packed byte `forge_core_v3::atom::TritCell5D` already defines. The 243
/// interior values are real cards — each one IS a soulword
/// (`TritCell5D::trits`/`from_trits`, `PARARITY.md`'s 5-lane balanced-ternary
/// law) — and the 13 sentinel values (`243..=255`,
/// `TritCell5D::is_sentinel`) become non-craftable control/mythic cards for
/// free, not a special case this function has to write. The HMAC-SHA256
/// layer upstream (`derive_seed`) stays exactly as-is — this is one new
/// reduction step, not a new seed source.
pub fn soulword_address(seed: &DerivedSeed) -> TritCell5D {
    TritCell5D((seed.seed_value % 256) as u8)
}

/// A drawn card is craftable player content iff its address is interior
/// (`0..=242`) — the 13 sentinel addresses are the free control/mythic band
/// [`soulword_address`] describes, never a drawable card.
pub fn is_craftable_card(addr: TritCell5D) -> bool {
    !addr.is_sentinel()
}

/// Weighted rarity with progression pity (matches Python _weighted_rarity).
pub fn weighted_rarity(seed: &DerivedSeed, floor_num: u32, num_floors: u32) -> RarityTier {
    const THRESHOLDS: [u64; 5] = [500, 750, 900, 980, 1000];
    let base_roll = seed.seed_value % 1000;
    let quarter = ((floor_num * 4) / num_floors.max(1)).min(3);
    let bonus = quarter as u64 * 30;
    let mut adjusted = (base_roll + bonus).min(999);
    if floor_num == 1 { adjusted = adjusted.min(THRESHOLDS[2] - 1); }
    for (tier, &threshold) in THRESHOLDS.iter().enumerate() {
        if adjusted < threshold { return RarityTier::from_index(tier as u8); }
    }
    RarityTier::Mythic
}

// ── HMAC-SHA256 (RFC 2104, no external crate) ───────────────────────────────

fn hmac_sha256(key_prime: u64, system: &str, context: &str) -> [u8; 32] {
    let key = key_prime.to_be_bytes();
    let msg = format!("{}|{}", system, context);

    // Pad key to block size (64 bytes)
    let mut k_pad = [0u8; 64];
    if key.len() <= 64 {
        k_pad[..key.len()].copy_from_slice(&key);
    } else {
        let h = Sha256::digest(key);
        k_pad[..32].copy_from_slice(&h);
    }

    // ipad = key XOR 0x36, opad = key XOR 0x5c
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k_pad[i];
        opad[i] ^= k_pad[i];
    }

    // inner = SHA256(ipad || msg)
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(msg.as_bytes());
    let inner_hash = inner.finalize();

    // outer = SHA256(opad || inner)
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    let result = outer.finalize();

    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::sieve::sieve_of_eratosthenes;

    #[test]
    fn derive_deterministic() {
        let a = derive_seed(7, 3, SystemID::Loot, "floor_1");
        let b = derive_seed(7, 3, SystemID::Loot, "floor_1");
        assert_eq!(a.seed_value, b.seed_value);
        assert_eq!(a.derivation_hash, b.derivation_hash);
    }

    #[test]
    fn different_systems_different_seeds() {
        let a = derive_seed(7, 3, SystemID::Loot, "floor_1");
        let b = derive_seed(7, 3, SystemID::Pvp, "floor_1");
        assert_ne!(a.seed_value, b.seed_value);
    }

    #[test]
    fn different_primes_different_seeds() {
        let a = derive_seed(7, 3, SystemID::Loot, "floor_1");
        let b = derive_seed(11, 4, SystemID::Loot, "floor_1");
        assert_ne!(a.seed_value, b.seed_value);
    }

    #[test]
    fn derive_all_systems() {
        let sieve = sieve_of_eratosthenes(100);
        let seeds = derive_for_all_systems(&sieve, "match_1", 0);
        assert_eq!(seeds.len(), SystemID::ALL.len());
        // All seeds should be unique
        let values: Vec<u64> = seeds.iter().map(|s| s.seed_value).collect();
        let mut deduped = values.clone();
        deduped.sort();
        deduped.dedup();
        assert_eq!(values.len(), deduped.len());
    }

    #[test]
    fn batch_derivation() {
        let sieve = sieve_of_eratosthenes(100);
        let contexts: Vec<String> = (1..=5).map(|i| format!("floor_{}", i)).collect();
        let pack = derive_batch(&sieve, SystemID::Loot, &contexts, 0);
        assert_eq!(pack.seeds.len(), 5);
        assert_eq!(pack.seeds[0].master_prime, 2);
        assert_eq!(pack.seeds[1].master_prime, 3);
    }

    #[test]
    fn soulword_address_is_deterministic_and_seed_stable() {
        let a = derive_seed(7, 3, SystemID::Loot, "season_1");
        let b = derive_seed(7, 3, SystemID::Loot, "season_1");
        assert_eq!(soulword_address(&a), soulword_address(&b), "same seed must give the same card address");
        let c = derive_seed(7, 3, SystemID::Loot, "season_2");
        // Not asserted unequal unconditionally (a real hash CAN collide on 256 buckets),
        // but a different context must go through a different HMAC input.
        assert_ne!(a.seed_value, c.seed_value, "different context must change the seed itself");
        let _ = soulword_address(&c); // exercised, not required to differ
    }

    #[test]
    fn soulword_address_partitions_interior_cards_from_sentinels() {
        let sieve = sieve_of_eratosthenes(4000); // pi(4000) = 550, comfortably >= 500 draws
        let seeds = derive_batch(
            &sieve,
            SystemID::Loot,
            &(0..500).map(|i| format!("draw_{i}")).collect::<Vec<_>>(),
            0,
        );
        let mut interior = 0u32;
        let mut sentinel = 0u32;
        for s in &seeds.seeds {
            let addr = soulword_address(s);
            if is_craftable_card(addr) {
                interior += 1;
                // Every interior address must decode to real trits -- the
                // whole point of landing on TritCell5D instead of a bare index.
                assert!(addr.trits().is_some(), "craftable card must have real trit coordinates");
            } else {
                sentinel += 1;
                assert!(addr.trits().is_none(), "sentinel address must not decode to trits");
            }
        }
        assert!(interior > 0, "500 draws over 256 buckets must land some interior cards");
        assert!(sentinel > 0, "500 draws over 256 buckets must land some sentinel cards (13/256 band)");
        // 13/256 ~= 5.1% -- with 500 draws, some sentinels landing is expected,
        // not a fluke; a total absence would indicate the reduction is broken.
    }

    #[test]
    fn rarity_weighted_floor1_no_epic() {
        let seed = derive_seed(2, 0, SystemID::Loot, "floor_1");
        let tier = weighted_rarity(&seed, 1, 100);
        assert!(tier < RarityTier::Epic, "Floor 1 should not produce epic+");
    }

    #[test]
    fn rarity_progression_bonus() {
        // Late floors should trend higher rarity on average
        let sieve = sieve_of_eratosthenes(1000);
        let mut early_sum = 0u64;
        let mut late_sum = 0u64;
        for i in 0..50 {
            let s = derive_seed(sieve.primes[i], i, SystemID::Loot, &format!("floor_{}", i + 1));
            early_sum += weighted_rarity(&s, (i + 1) as u32, 100) as u64;
        }
        for i in 50..100 {
            let s = derive_seed(sieve.primes[i], i, SystemID::Loot, &format!("floor_{}", i + 1));
            late_sum += weighted_rarity(&s, (i + 1) as u32, 100) as u64;
        }
        assert!(late_sum >= early_sum, "Late floors should have higher average rarity");
    }
}
