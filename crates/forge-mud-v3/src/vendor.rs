//! Vendor system: buy/sell/repair prices scale with CHA.
//! Fence vendors buy stolen goods at 50%.

/// Price for buying from player. CHA modifier reduces cost.
pub fn buy_price(item_value: i32, buyer_cha: i32) -> i32 {
    let cha_mod = ability_mod(buyer_cha) as f64 * 0.02;
    (item_value as f64 * (1.0 - cha_mod)).max(1.0) as i32
}

/// Price for selling to vendor. Returns None if unsellable (soulbound/no_destroy).
/// Fence vendors (is_fence=true) buy stolen goods at 50% value.
/// Normal vendors reject stolen goods (Some(0)).
pub fn sell_price(
    item_value: i32,
    seller_cha: i32,
    is_fence: bool,
    is_stolen: bool,
    is_soulbound: bool,
    is_no_destroy: bool,
) -> Option<i32> {
    if is_soulbound || is_no_destroy {
        return None;
    }
    if !is_fence && is_stolen {
        return Some(0);
    }
    let cha_mod = ability_mod(seller_cha) as f64 * 0.02;
    let mult = if is_fence && is_stolen {
        0.5
    } else {
        0.25 * (1.0 + cha_mod)
    };
    Some((item_value as f64 * mult).max(1.0) as i32)
}

/// Repair cost scales with damage (1.0 - durability).
pub fn repair_cost(item_value: i32, durability: f64) -> i32 {
    let missing = 1.0 - durability;
    (item_value as f64 * 0.1 * missing).max(1.0) as i32
}

fn ability_mod(ability_score: i32) -> i32 {
    (ability_score - 10) / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buy_price_exact_cha10() {
        assert_eq!(buy_price(100, 10), 100);
    }

    #[test]
    fn buy_price_cha14_discount() {
        assert_eq!(buy_price(100, 14), 96);
    }

    #[test]
    fn buy_price_cha20_larger_discount() {
        assert_eq!(buy_price(100, 20), 90);
    }

    #[test]
    fn sell_price_normal_vendor_cha10() {
        assert_eq!(sell_price(100, 10, false, false, false, false), Some(25));
    }

    #[test]
    fn sell_price_soulbound_none() {
        assert_eq!(sell_price(100, 10, false, false, true, false), None);
    }

    #[test]
    fn sell_price_no_destroy_none() {
        assert_eq!(sell_price(100, 10, false, false, false, true), None);
    }

    #[test]
    fn sell_price_stolen_normal_vendor_zero() {
        assert_eq!(sell_price(100, 10, false, true, false, false), Some(0));
    }

    #[test]
    fn sell_price_stolen_fence_50percent() {
        assert_eq!(sell_price(100, 10, true, true, false, false), Some(50));
    }

    #[test]
    fn sell_price_unstolen_fence_cha10() {
        assert_eq!(sell_price(100, 10, true, false, false, false), Some(25));
    }

    #[test]
    fn repair_cost_full_durability_zero() {
        assert_eq!(repair_cost(100, 1.0), 1);
    }

    #[test]
    fn repair_cost_half_damaged() {
        assert_eq!(repair_cost(100, 0.5), 5);
    }

    #[test]
    fn repair_cost_nearly_destroyed() {
        assert_eq!(repair_cost(100, 0.1), 9);
    }
}
