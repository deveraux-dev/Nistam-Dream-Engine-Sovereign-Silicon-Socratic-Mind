//! Resonance scoring engine: weighted aspect-based synthesis.

/// Calculate resonance score between two entities based on aspects and weights.
///
/// Returns a 0–100 score based on the average aspect multiplier scaled by
/// the average of all entity weights.
pub fn calculate_resonance(aspects: &[&str], weights: &[f32]) -> f32 {
    if aspects.is_empty() || weights.is_empty() {
        return 0.0;
    }

    let multipliers = [
        ("CONJUNCT", 0.90),
        ("TRINE", 0.85),
        ("SEXTILE", 0.80),
        ("OPPOSE", 0.75),
        ("SQUARE", 0.70),
    ];

    let avg_aspect_mult = aspects
        .iter()
        .map(|a| {
            multipliers
                .iter()
                .find(|(name, _)| name == a)
                .map(|(_, mult)| mult)
                .copied()
                .unwrap_or(0.75)
        })
        .sum::<f32>()
        / (aspects.len() as f32);

    let base_score = (weights.iter().sum::<f32>() / weights.len() as f32) * 100.0;
    let final_score = base_score * avg_aspect_mult;

    final_score.max(0.0).min(100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonance_conjunct_high() {
        let aspects = vec!["CONJUNCT"];
        let weights = vec![1.0];
        let score = calculate_resonance(&aspects, &weights);
        assert!((score - 90.0).abs() < 0.1);
    }

    #[test]
    fn resonance_square_low() {
        let aspects = vec!["SQUARE"];
        let weights = vec![1.0];
        let score = calculate_resonance(&aspects, &weights);
        assert!((score - 70.0).abs() < 0.1);
    }

    #[test]
    fn resonance_mixed_aspects() {
        let aspects = vec!["CONJUNCT", "SQUARE"];
        let weights = vec![0.5, 0.5];
        let score = calculate_resonance(&aspects, &weights);
        let expected = 0.5 * 100.0 * ((0.90 + 0.70) / 2.0);
        assert!((score - expected).abs() < 0.1);
    }

    #[test]
    fn resonance_empty_clamps_zero() {
        assert_eq!(calculate_resonance(&[], &[]), 0.0);
        assert_eq!(calculate_resonance(&["CONJUNCT"], &[]), 0.0);
    }

    #[test]
    fn resonance_unknown_aspect_defaults() {
        let aspects = vec!["UNKNOWN_ASPECT"];
        let weights = vec![1.0];
        let score = calculate_resonance(&aspects, &weights);
        let expected = 1.0 * 100.0 * 0.75;
        assert!((score - expected).abs() < 0.1);
    }
}
