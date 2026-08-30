//! Economy — buy/sell pricing by charisma (harvested from deveraux_mud). Integer;
//! higher charisma buys cheaper and sells dearer.

/// Charisma (0..=100) -> a permyriad adjustment up to 20%.
fn cha_adjust_pmy(cha: u32) -> u32 {
    (cha.min(100) * 20).min(2000)
}

/// Buy price of a `base`-value item at charisma `cha` (discounted).
pub fn buy_price(base: u32, cha: u32) -> u32 {
    let disc = cha_adjust_pmy(cha);
    base - (base * disc / 10_000)
}

/// Sell price of a `base`-value item at charisma `cha` (~50% + charisma markup),
/// never above base.
pub fn sell_price(base: u32, cha: u32) -> u32 {
    let markup = cha_adjust_pmy(cha);
    (base as u64 * (5000 + markup) as u64 / 10_000).min(base as u64) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charisma_discounts_buying() {
        assert_eq!(buy_price(1000, 0), 1000);
        assert!(buy_price(1000, 100) < 1000); // 20% off
        assert_eq!(buy_price(1000, 100), 800);
    }

    #[test]
    fn charisma_raises_selling() {
        assert_eq!(sell_price(1000, 0), 500); // 50% base
        assert!(sell_price(1000, 100) > sell_price(1000, 0));
        assert!(sell_price(1000, 100) <= 1000);
    }

    #[test]
    fn buy_beats_sell() {
        assert!(buy_price(1000, 50) > sell_price(1000, 50)); // the house always wins
    }
}
