//! Gauge — a labelled permyriad progress gauge for the desk HUD; renders a text
//! bar. The done-bar made visible.

use serde::{Deserialize, Serialize};

/// A labelled 0..=10000 gauge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Gauge {
    /// Display label for the gauge.
    pub label: String,
    /// Current value in permyriad (parts per 10,000).
    pub value_pmy: u32,
}

impl Gauge {
    /// Create a new gauge with the given label and value 0.
    pub fn new(label: impl Into<String>) -> Self {
        Self { label: label.into(), value_pmy: 0 }
    }
    /// Set gauge value, clamped to 0..=10000.
    pub fn set(&mut self, value_pmy: u32) {
        self.value_pmy = value_pmy.min(10_000);
    }
    /// A `width`-cell text bar: `label [####----] 50%`.
    pub fn bar(&self, width: usize) -> String {
        let filled = (self.value_pmy as usize * width / 10_000).min(width);
        let bar = "#".repeat(filled) + &"-".repeat(width - filled);
        format!("{} [{}] {}%", self.label, bar, self.value_pmy / 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_fills_proportionally() {
        let mut g = Gauge::new("merge");
        g.set(5000);
        assert_eq!(g.bar(10), "merge [#####-----] 50%");
        g.set(10_000);
        assert_eq!(g.bar(4), "merge [####] 100%");
    }

    #[test]
    fn value_clamps() {
        let mut g = Gauge::new("x");
        g.set(99_999);
        assert_eq!(g.value_pmy, 10_000);
    }
}
