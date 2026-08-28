//! Broski State Writer — human-readable state description for MCP/debug.
//!
//! Ported 2026-08-17 from F:\NewRepo\crates\forge-broski\src\dj\state_writer.rs (57 LOC).

pub fn describe_deck(
    label: &str, track_name: &str, bpm: f64, key: &str, genre: &str,
    energy: f64, spectral_centroid: f64, energy_trend: f64,
) -> String {
    let heat = if energy > 0.7 { "hot" } else if energy > 0.4 { "warm" } else { "quiet" };
    let tone = if spectral_centroid > 3500.0 { "bright" } else if spectral_centroid > 1500.0 { "mid" } else { "dark" };
    let trend = if energy_trend > 0.05 { "climbing" } else if energy_trend < -0.05 { "falling" } else { "steady" };
    format!("Deck {}: \"{}\" {:.0}BPM {} {} | {} {} {}", label, track_name, bpm, key, genre, heat, tone, trend)
}

pub fn format_state(
    deck_lines: &[String], crossfader: f32, limiter_state: &str,
    fx_slots: &[String], recording: bool, issues: &[String], ghost_count: u32,
) -> String {
    let mut out = String::new();
    for line in deck_lines { out.push_str(line); out.push('\n'); }
    out.push_str(&format!("Crossfader: {:.2} | Limiter: {}\n", crossfader, limiter_state));
    if !fx_slots.is_empty() { out.push_str(&format!("FX: {}\n", fx_slots.join(", "))); }
    if recording { out.push_str("Recording: YES\n"); }
    out.push_str(&format!("Ghosts: {}\n", ghost_count));
    if !issues.is_empty() {
        out.push_str("Issues:\n");
        for issue in issues { out.push_str(&format!("  - {}\n", issue)); }
    }
    out
}

pub fn write_state_file(content: &str) {
    std::fs::write("F:/output/dead_drop_state.txt", content).ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_deck_hot_bright_climbing() {
        let s = describe_deck("A", "banger.mp3", 174.0, "8A", "DnB", 0.85, 4000.0, 0.1);
        assert!(s.contains("hot") && s.contains("bright") && s.contains("climbing"));
    }

    #[test]
    fn describe_deck_quiet_dark_steady() {
        let s = describe_deck("B", "ambient.mp3", 90.0, "1B", "Deep", 0.2, 800.0, 0.0);
        assert!(s.contains("quiet") && s.contains("dark") && s.contains("steady"));
    }

    #[test]
    fn format_state_complete() {
        let decks = vec![describe_deck("A", "t.mp3", 174.0, "8A", "DnB", 0.8, 3000.0, 0.0)];
        let s = format_state(&decks, 0.5, "OFF", &["echo".into()], false, &[], 47);
        assert!(s.contains("Crossfader: 0.50"));
        assert!(s.contains("FX: echo"));
        assert!(s.contains("Ghosts: 47"));
    }

    #[test]
    fn format_state_with_issues() {
        let s = format_state(&[], 0.5, "ON", &[], true, &["limiter overwork".into()], 0);
        assert!(s.contains("Recording: YES"));
        assert!(s.contains("limiter overwork"));
    }
}
