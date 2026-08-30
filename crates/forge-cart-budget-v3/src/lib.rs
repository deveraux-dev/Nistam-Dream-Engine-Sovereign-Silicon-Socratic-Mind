#![forbid(unsafe_code)]
//! `forge-cart-budget` — the RunDevRun funnel's COVERAGE LEDGER (Phase 8).
//!
//! A slim, pure port of `forge-meaning-budget`'s CORE: per-category content gaps
//! + an authoring HOUR BUDGET. **Read-only** — it REPORTS coverage, it never
//! authors (stub-emission is a later opt-in per the dropin's integration notes).
//! The full filesystem-scanning auditor is the larger `forge-meaning-budget`
//! tool; this is the budget-computation core that the cart's `.kit.vixi` content
//! inventory feeds. Zero deps -> edge-portable, like the rest of the cart.
//!
//! PORT RECEIPT (2026-08-15): logic, names, and test bodies verbatim from v2.
//! The ONLY delta is doc comments added to public items that had none — v3's
//! workspace lints set `missing_docs = "deny"`, which the v2 crate did not.

/// The production content categories `forge-meaning-budget` tracks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Non-player characters.
    Npc,
    /// Traversable routes/paths.
    Route,
    /// Playable maps/zones.
    Map,
    /// Materials/textures.
    Material,
    /// Dialogue lines.
    Dialogue,
    /// Visual effects.
    Vfx,
    /// Sound effects.
    Sfx,
    /// Music tracks.
    Music,
}

impl Category {
    /// All categories in stable order; the array index IS the lane.
    pub const ALL: [Category; 8] = [
        Category::Npc,
        Category::Route,
        Category::Map,
        Category::Material,
        Category::Dialogue,
        Category::Vfx,
        Category::Sfx,
        Category::Music,
    ];

    /// This category's stable lowercase name.
    pub fn name(self) -> &'static str {
        match self {
            Category::Npc => "npc",
            Category::Route => "route",
            Category::Map => "map",
            Category::Material => "material",
            Category::Dialogue => "dialogue",
            Category::Vfx => "vfx",
            Category::Sfx => "sfx",
            Category::Music => "music",
        }
    }

    /// Rough authoring cost per item, in hours — the budget multiplier.
    pub fn hours_per_item(self) -> u32 {
        match self {
            Category::Npc => 8,
            Category::Route => 2,
            Category::Map => 12,
            Category::Material => 3,
            Category::Dialogue => 1,
            Category::Vfx => 4,
            Category::Sfx => 1,
            Category::Music => 6,
        }
    }

    fn lane(self) -> usize {
        match self {
            Category::Npc => 0,
            Category::Route => 1,
            Category::Map => 2,
            Category::Material => 3,
            Category::Dialogue => 4,
            Category::Vfx => 5,
            Category::Sfx => 6,
            Category::Music => 7,
        }
    }
}

/// Counts per category — `Inventory` = what is authored, `Target` = what a
/// complete cart needs. Same fixed 8-lane shape for both roles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    lanes: [u32; 8],
}

impl Counts {
    /// An all-zero count set.
    pub fn new() -> Self {
        Self { lanes: [0; 8] }
    }

    /// Set `category`'s count to `n`.
    pub fn set(&mut self, category: Category, n: u32) -> &mut Self {
        self.lanes[category.lane()] = n;
        self
    }

    /// Read `category`'s count.
    pub fn get(&self, category: Category) -> u32 {
        self.lanes[category.lane()]
    }
}

/// One category's coverage row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapRow {
    /// The category this row reports.
    pub category: Category,
    /// Authored count.
    pub have: u32,
    /// Target count.
    pub need: u32,
    /// `max(0, need - have)`.
    pub gap: u32,
    /// `gap * category.hours_per_item()`.
    pub hours: u32,
}

/// The read-only coverage report.
#[derive(Clone, Copy, Debug)]
pub struct BudgetReport {
    /// One row per [`Category`], in [`Category::ALL`] order.
    pub rows: [GapRow; 8],
    /// Sum of all per-category gaps (`production_gaps`).
    pub production_gaps: u32,
    /// Sum of all per-category authoring hours (`hour_budget`).
    pub hour_budget: u32,
}

impl BudgetReport {
    /// One-line `metrics.summary`.
    pub fn summary(&self) -> String {
        format!(
            "production_gaps={} hour_budget={}h across {} categories",
            self.production_gaps,
            self.hour_budget,
            Category::ALL.len()
        )
    }
}

/// Run the read-only audit: `have` vs `need` -> per-category gaps -> hour budget.
/// `gap = max(0, need - have)`, `hours = gap * hours_per_item`. Never authors.
pub fn audit(inventory: &Counts, target: &Counts) -> BudgetReport {
    let mut rows = [GapRow { category: Category::Npc, have: 0, need: 0, gap: 0, hours: 0 }; 8];
    let mut production_gaps = 0;
    let mut hour_budget = 0;
    for (i, &c) in Category::ALL.iter().enumerate() {
        let have = inventory.get(c);
        let need = target.get(c);
        let gap = need.saturating_sub(have);
        let hours = gap * c.hours_per_item();
        rows[i] = GapRow { category: c, have, need, gap, hours };
        production_gaps += gap;
        hour_budget += hours;
    }
    BudgetReport { rows, production_gaps, hour_budget }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_complete() -> Counts {
        let mut t = Counts::new();
        t.set(Category::Npc, 10)
            .set(Category::Route, 5)
            .set(Category::Map, 3)
            .set(Category::Material, 20)
            .set(Category::Dialogue, 40)
            .set(Category::Vfx, 8)
            .set(Category::Sfx, 16)
            .set(Category::Music, 4);
        t
    }

    #[test]
    fn audit_computes_gaps_and_hours() {
        let target = target_complete();
        let mut have = Counts::new();
        have.set(Category::Npc, 4).set(Category::Material, 20);
        let report = audit(&have, &target);
        // npc: need 10, have 4 -> gap 6, hours 6*8 = 48
        let npc = report.rows[Category::Npc.lane()];
        assert_eq!(npc.gap, 6);
        assert_eq!(npc.hours, 48);
        // material: need 20, have 20 -> gap 0
        assert_eq!(report.rows[Category::Material.lane()].gap, 0);
        assert!(report.production_gaps > 0);
        assert!(report.hour_budget > 0);
    }

    #[test]
    fn production_gaps_strictly_decrease_as_content_is_authored() {
        // The plan's invariant: authoring more content strictly shrinks the gap.
        let target = target_complete();
        let empty = Counts::new();
        let mut partial = Counts::new();
        partial.set(Category::Npc, 5).set(Category::Dialogue, 20);
        let r_empty = audit(&empty, &target);
        let r_partial = audit(&partial, &target);
        assert!(
            r_partial.production_gaps < r_empty.production_gaps,
            "authoring content must strictly decrease production_gaps ({} < {})",
            r_partial.production_gaps,
            r_empty.production_gaps
        );
        assert!(r_partial.hour_budget < r_empty.hour_budget, "and the hour budget");
    }

    #[test]
    fn a_complete_inventory_has_zero_gap_and_budget() {
        let target = target_complete();
        let report = audit(&target, &target); // have == need
        assert_eq!(report.production_gaps, 0, "fully authored -> no gaps");
        assert_eq!(report.hour_budget, 0, "fully authored -> no budget left");
    }

    #[test]
    fn overshoot_does_not_underflow() {
        // have > need clamps the gap to 0 (saturating), never wraps.
        let mut target = Counts::new();
        target.set(Category::Sfx, 4);
        let mut have = Counts::new();
        have.set(Category::Sfx, 99);
        let report = audit(&have, &target);
        assert_eq!(report.production_gaps, 0);
    }

    #[test]
    fn summary_reports_the_metrics() {
        let report = audit(&Counts::new(), &target_complete());
        assert!(report.summary().contains("production_gaps"));
        assert!(report.summary().contains("hour_budget"));
    }
}
