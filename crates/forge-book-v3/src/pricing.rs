//! Invention pricing oracle — deterministic price table (skill fold 2026-07-21).
//! The LLM judgment half stays skill-side; this table is what the judgment reads.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// (id, name, model, price_usd, unit, y1_revenue_usd, churn_risk).
pub type PriceRow = (&'static str, &'static str, &'static str, u32, &'static str, u32, &'static str);

/// CLASS A invention licensing table (market-researched comps, skill 07-20 state).
pub const PRICING: &[PriceRow] = &[
    ("003", "Hierarchical MoE Inference", "per_dev_seat_annual", 3600, "USD/dev/year", 360_000, "low"),
    ("004", "Master Decoder (5D VQ Codec)", "per_game_royalty", 2, "% of gross", 200_000, "medium"),
    ("005", "Photogrammetry Stack", "per_dev_subscription", 1500, "USD/dev/year", 75_000, "medium"),
    ("006", "Vocal Synthesis + DSP", "perpetual_plugin", 199, "USD one-time", 299_000, "medium"),
    ("007", "Geometry Mesh Engine", "asset_store_70_30", 70, "% to us", 171_000, "medium"),
];

/// Lookup by id or case-insensitive name fragment.
pub fn price_lookup(q: &str) -> Option<&'static PriceRow> {
    let ql = q.to_lowercase();
    PRICING.iter().find(|r| r.0 == q || r.1.to_lowercase().contains(&ql))
}

/// Y1 conservative baseline across all rows ($1.105M — the combined model).
pub fn y1_total() -> u32 {
    PRICING.iter().map(|r| r.5).sum()
}

/// Bind the price table into a Capabilities chapter (the catalogue face).
pub fn pricing_chapter() -> Chapter {
    let mut ch = Chapter::new("Invention Pricing Oracle", AtlasSection::Capabilities);
    ch.add_lore(format!(
        "5 CLASS A inventions · Y1 conservative baseline ${} · 2x marketing ${} · comps market-researched (skill fold 07-21).",
        y1_total(),
        y1_total() * 2
    ));
    for &(id, name, model, price, unit, y1, churn) in PRICING {
        ch.add_lore(format!("{id} {name} → {model} {price} {unit} · y1 ${y1} · churn {churn}"));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_by_id_and_name_fragment() {
        assert_eq!(price_lookup("003").unwrap().1, "Hierarchical MoE Inference");
        assert_eq!(price_lookup("codec").unwrap().0, "004");
        assert!(price_lookup("nonexistent").is_none());
    }

    #[test]
    fn y1_baseline_is_1_105m() {
        assert_eq!(y1_total(), 1_105_000);
    }

    #[test]
    fn chapter_binds_all_rows() {
        let ch = pricing_chapter();
        assert_eq!(ch.section, AtlasSection::Capabilities);
        assert_eq!(ch.lore_count(), 6); // header + 5 rows
    }
}
