//! Spread — pair page indices into left/right spreads (an open book shows two
//! pages at once). Index-based so it borrows nothing.

use serde::{Deserialize, Serialize};

/// One open spread: a left page and an optional right page (odd tail).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Spread {
    /// Index of the left-side page.
    pub left: usize,
    /// Index of the right-side page; `None` if this is an odd trailing spread.
    pub right: Option<usize>,
}

/// Pair `page_count` pages into spreads (2 per spread; last may be single).
pub fn spreads(page_count: usize) -> Vec<Spread> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < page_count {
        let right = if i + 1 < page_count { Some(i + 1) } else { None };
        out.push(Spread { left: i, right });
        i += 2;
    }
    out
}

/// How many spreads `page_count` pages make.
pub fn spread_count(page_count: usize) -> usize {
    page_count.div_ceil(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn even_pages_pair_up() {
        let s = spreads(4);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], Spread { left: 0, right: Some(1) });
        assert_eq!(s[1], Spread { left: 2, right: Some(3) });
    }

    #[test]
    fn odd_tail_is_single() {
        let s = spreads(3);
        assert_eq!(s.len(), 2);
        assert_eq!(s[1], Spread { left: 2, right: None });
        assert_eq!(spread_count(3), 2);
    }

    #[test]
    fn zero_pages_no_spreads() {
        assert!(spreads(0).is_empty());
        assert_eq!(spread_count(0), 0);
    }
}
