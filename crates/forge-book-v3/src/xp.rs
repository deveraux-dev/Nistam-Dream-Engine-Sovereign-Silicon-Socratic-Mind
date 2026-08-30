//! XP — the leveling curve (harvested from deveraux_mud xpToNextLevel). Integer
//! quadratic; the inverse walks levels.

/// XP required to advance FROM `level` to the next (quadratic).
pub fn xp_to_next(level: u32) -> u64 {
    let l = level as u64;
    100 * l * l + 50 * l + 100
}

/// Total XP to reach `level` from level 1.
pub fn xp_to_reach(level: u32) -> u64 {
    (1..level).map(xp_to_next).sum()
}

/// The level a given total XP buys (>= 1).
pub fn level_for_xp(total: u64) -> u32 {
    let mut level = 1u32;
    let mut acc = 0u64;
    loop {
        let need = xp_to_next(level);
        if acc + need > total || level >= 10_000 {
            break;
        }
        acc += need;
        level += 1;
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_increases() {
        assert!(xp_to_next(2) > xp_to_next(1));
        assert!(xp_to_next(10) > xp_to_next(9));
    }

    #[test]
    fn level_and_xp_are_inverse() {
        for lvl in 1..30 {
            let total = xp_to_reach(lvl);
            assert_eq!(level_for_xp(total), lvl, "at level {lvl}");
        }
    }

    #[test]
    fn zero_xp_is_level_one() {
        assert_eq!(level_for_xp(0), 1);
    }
}
