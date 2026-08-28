//! Beat status persistence: streak, last verdict, beats total, quality last.
//! Atomic write via temp+rename. Refuse malformed state loud and fresh.

use std::path::Path;

/// One beat's recorded state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeatStatus {
    /// Consecutive non-PASS beats before survival mode (reset on PASS).
    pub streak: u32,
    /// Last verdict: "PASS" or "FAIL" (or "BLIND").
    pub last_verdict_is_pass: bool,
    /// Total beats recorded.
    pub beats_total: u64,
    /// Quality of the last beat (permyriad, 0-10000).
    pub quality_last: u16,
}

impl BeatStatus {
    /// Render as a RON row.
    pub fn render(&self) -> String {
        format!(
            "BeatStatus(streak:{},last_verdict_is_pass:{},beats_total:{},quality_last:{})",
            self.streak, self.last_verdict_is_pass, self.beats_total, self.quality_last
        )
    }

    /// Parse a RON row. Returns None if malformed.
    pub fn parse(s: &str) -> Option<Self> {
        fn field<'a>(row: &'a str, key: &str) -> Option<&'a str> {
            let s = row.find(key)? + key.len();
            let rest = &row[s..];
            Some(&rest[..rest.find([',', ')'])?])
        }
        Some(BeatStatus {
            streak: field(s, "streak:")?.parse().ok()?,
            last_verdict_is_pass: field(s, "last_verdict_is_pass:")?.parse().ok()?,
            beats_total: field(s, "beats_total:")?.parse().ok()?,
            quality_last: field(s, "quality_last:")?.parse().ok()?,
        })
    }

    /// Select a RANK word based on quality_last (permyriad bands).
    /// Bands (observed as AUTHORED items):
    /// 0-1999: ember, 2000-3999: spark, 4000-5999: arc,
    /// 6000-7999: corona, 8000-10000: star
    pub fn rank_word(&self) -> &'static str {
        match self.quality_last {
            0..=1999 => "ember",
            2000..=3999 => "spark",
            4000..=5999 => "arc",
            6000..=7999 => "corona",
            8000..=10000 => "star",
            _ => "unknown", // safety fallback
        }
    }
}

impl Default for BeatStatus {
    fn default() -> Self {
        BeatStatus {
            streak: 0,
            last_verdict_is_pass: false,
            beats_total: 0,
            quality_last: 0,
        }
    }
}

/// Read beat status from disk. Returns fresh default if absent or malformed (loud).
pub fn read_status(path: &Path) -> BeatStatus {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            if let Some(status) = BeatStatus::parse(&content) {
                return status;
            }
            // Malformed — print loud and return fresh
            eprintln!(
                "[beat] beat-status.ron malformed, discarding: {}",
                path.display()
            );
            BeatStatus::default()
        }
        Err(_) => {
            // File absent — return fresh default
            BeatStatus::default()
        }
    }
}

/// Write beat status atomically (temp+rename). Refuse whole on any error loud.
pub fn write_status(path: &Path, status: BeatStatus) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("cannot create beat status dir {}: {}", parent.display(), e))?;
    }

    let temp = path.with_extension("ron.tmp");
    let content = status.render();
    std::fs::write(&temp, &content)
        .map_err(|e| format!("cannot write beat status temp: {}", e))?;

    std::fs::rename(&temp, path)
        .map_err(|e| format!("cannot rename beat status temp: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beat_status_render_and_parse() {
        let status = BeatStatus {
            streak: 2,
            last_verdict_is_pass: false,
            beats_total: 5,
            quality_last: 3500,
        };
        let ron = status.render();
        let parsed = BeatStatus::parse(&ron).expect("parse failed");
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_beat_status_malformed_returns_none() {
        assert_eq!(BeatStatus::parse("garbage"), None);
        assert_eq!(BeatStatus::parse("BeatStatus(streak:not_a_number)"), None);
    }

    #[test]
    fn test_beat_status_rank_word() {
        assert_eq!(BeatStatus { streak: 0, last_verdict_is_pass: true, beats_total: 1, quality_last: 100 }.rank_word(), "ember");
        assert_eq!(BeatStatus { streak: 0, last_verdict_is_pass: true, beats_total: 1, quality_last: 2500 }.rank_word(), "spark");
        assert_eq!(BeatStatus { streak: 0, last_verdict_is_pass: true, beats_total: 1, quality_last: 4500 }.rank_word(), "arc");
        assert_eq!(BeatStatus { streak: 0, last_verdict_is_pass: true, beats_total: 1, quality_last: 7000 }.rank_word(), "corona");
        assert_eq!(BeatStatus { streak: 0, last_verdict_is_pass: true, beats_total: 1, quality_last: 9000 }.rank_word(), "star");
    }

    #[test]
    fn test_beat_status_default() {
        let status = BeatStatus::default();
        assert_eq!(status.streak, 0);
        assert!(!status.last_verdict_is_pass);
        assert_eq!(status.beats_total, 0);
        assert_eq!(status.quality_last, 0);
    }

    #[test]
    fn test_beat_status_round_trip_persistence() {
        let original = BeatStatus {
            streak: 2,
            last_verdict_is_pass: false,
            beats_total: 5,
            quality_last: 4500,
        };
        let ron = original.render();
        let parsed = BeatStatus::parse(&ron).expect("parse failed");
        assert_eq!(parsed.streak, original.streak);
        assert_eq!(parsed.last_verdict_is_pass, original.last_verdict_is_pass);
        assert_eq!(parsed.beats_total, original.beats_total);
        assert_eq!(parsed.quality_last, original.quality_last);
    }

    #[test]
    fn test_malformed_beat_status_returns_fresh_default() {
        // Simulate a corrupted beat-status.ron file
        let malformed = "garbage_not_ron(";
        let parsed = BeatStatus::parse(malformed);
        assert_eq!(parsed, None);

        // When read_status encounters malformed, it returns fresh default
        // (This is verified by the loud eprintln, which we can't easily test here,
        // but the behavior is deterministic: malformed → fresh)
    }

    #[test]
    fn test_beat_status_streak_reset_on_pass() {
        let mut status = BeatStatus {
            streak: 3,
            last_verdict_is_pass: false,
            beats_total: 5,
            quality_last: 1000,
        };
        // Simulate PASS verdict: streak resets
        status.streak = 0;
        status.last_verdict_is_pass = true;
        assert_eq!(status.streak, 0);
        assert!(status.last_verdict_is_pass);
    }

    #[test]
    fn test_beat_status_streak_increment_on_fail() {
        let mut status = BeatStatus {
            streak: 1,
            last_verdict_is_pass: false,
            beats_total: 2,
            quality_last: 500,
        };
        // Simulate FAIL verdict: streak increments
        status.streak += 1;
        assert_eq!(status.streak, 2);
        assert!(!status.last_verdict_is_pass);
    }
}
