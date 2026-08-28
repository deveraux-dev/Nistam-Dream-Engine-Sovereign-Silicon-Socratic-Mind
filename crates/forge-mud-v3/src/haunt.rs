//! The Haunt Shadow Machine — deterministic haunting through scar-memory accrual.
//!
//! The shadow is not an AI: it observes RECORDINGS and reaches conclusions deterministically.
//! Feed it a death-scar hash and it remembers; the same hash twice raises its awareness.
//! A fresh run with an empty memory starts at Replay; accumulated scars climb the ladder.
//!
//! Integer-only, no floats. No wall-clock in logic. The shadow speaks WORDS, never numbers.
//! Persistence: magic + version + fields, refuse-whole codec (L10), atomic write via temp+rename (L07).
//!
//! TRIM (OBSERVED): Drained from forge-game-systems::sim_harness.rs:
//! - ShadowAwareness ladder (Replay → Pattern → Counterpart → Witness → Harbinger → ConfusedByVowless)
//! - repetition_permyriad pressure metric (Permyriad = i32, 0..=10_000)
//! - scar-memory accrual keyed on death_scar_hash (accumulate count per hash)
//! - two-clock bridge: pressure_q() and aggression_level() (integer, no floats)

use forge_core_v3::sprite_blob::{
    u32_from_nistam, u32_to_nistam, u64_from_nistam, u64_to_nistam,
};

/// Awareness tiers — the shadow's knowledge of the player, ascending order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ShadowAwareness {
    /// Nothing has been witnessed — the shadow sleeps.
    Replay = 0,
    /// The shadow has seen a pattern — repetition caught once.
    Pattern = 1,
    /// The shadow understands the pattern — it moves like you.
    Counterpart = 2,
    /// The shadow has learned your shape — it takes notes.
    Witness = 3,
    /// The shadow knows the future — it reaches it first.
    Harbinger = 4,
    /// The shadow was confused by deception — awareness inverted.
    ConfusedByVowless = 5,
}

impl ShadowAwareness {
    /// The spoken word the shadow uses to greet you at this tier.
    /// Never numbers. AUTHORED.
    pub fn remembrance_line(self) -> &'static str {
        match self {
            ShadowAwareness::Replay => "something below repeats your shape",
            ShadowAwareness::Pattern => "it knows where you turn",
            ShadowAwareness::Counterpart => "it moves when you move",
            ShadowAwareness::Witness => "it is taking notes",
            ShadowAwareness::Harbinger => "it was waiting — it knows the way you move",
            ShadowAwareness::ConfusedByVowless => "it forgets, and learns again",
        }
    }
}

/// The shadow's memory of the player — purely integer fields from sim_harness donors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowMemory {
    /// Permyriad of repeated inputs (0..=10_000, from last run).
    pub repeated_input_q: i32,
    /// Permyriad of route repetitions (placeholder for now, unused in mud).
    pub route_repetition_q: i32,
    /// Count of times this player has been encountered / run through.
    pub execution_count: u32,
    /// Count of times the player refused (placeholder, always 0 in mud).
    pub refused_execution_count: u32,
    /// Hash of the most recent death scar — what shape you died in.
    pub death_scar_hash: u64,
    /// Drift index (placeholder, always 0 in mud).
    pub dominant_drift_index: u32,
    /// Confusion Permyriad (0..=10_000). `classify_awareness` reads this now
    /// (this session's brick), but no caller in `forge-mud-v3` sets it above 0
    /// yet — reachable via direct construction (see the `vowless` test), not
    /// yet triggered by real play. The wire from an actual deception/mislead
    /// event to this field is a separate, un-landed brick.
    pub vowless_confusion_q: i32,
    /// Scar-memory accrual map: hash -> count (serialized as entries).
    /// Up to 256 unique scar hashes tracked.
    scar_counts: [(u64, u8); 256],
    /// Number of entries in scar_counts that are non-zero.
    scar_count_len: usize,
}

impl Default for ShadowMemory {
    fn default() -> Self {
        Self {
            repeated_input_q: 0,
            route_repetition_q: 0,
            execution_count: 0,
            refused_execution_count: 0,
            death_scar_hash: 0,
            dominant_drift_index: 0,
            vowless_confusion_q: 0,
            scar_counts: [(0u64, 0u8); 256],
            scar_count_len: 0,
        }
    }
}

impl ShadowMemory {
    /// Create a fresh shadow memory.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a death-scar hash. Increments its count if seen before, or adds it.
    /// Max 256 unique scars; oldest entry is overwritten on overflow (FIFO eviction).
    pub fn record_scar(&mut self, scar_hash: u64) {
        self.death_scar_hash = scar_hash;
        self.execution_count += 1;

        // Look for existing entry
        for entry in &mut self.scar_counts[..self.scar_count_len] {
            if entry.0 == scar_hash {
                // Clamp count at 255
                entry.1 = entry.1.saturating_add(1);
                return;
            }
        }

        // Not found — add new entry if space
        if self.scar_count_len < 256 {
            self.scar_counts[self.scar_count_len] = (scar_hash, 1);
            self.scar_count_len += 1;
        } else {
            // Overflow: FIFO eviction (oldest first)
            for i in 0..255 {
                self.scar_counts[i] = self.scar_counts[i + 1];
            }
            self.scar_counts[255] = (scar_hash, 1);
        }
    }

    /// Classify the shadow's awareness tier based on accumulated state.
    /// Thresholds: scar count (how many times same hash died) drives the main ladder.
    /// Repetition pressure accelerates the rise.
    /// OBSERVED: drained from forge-game-systems::sim_harness.rs:100-116.
    pub fn classify_awareness(&self) -> ShadowAwareness {
        // Confusion overrides the normal ladder — a misled observer isn't reading
        // you clearly, it's reading a lie. Checked first, same as the donor.
        // PORTED (C06 revascularize, this session): threshold 7_500 from
        // F:\NewRepo\crates\forge-game-systems\src\lore\determinism\shadow.rs:143
        // (`classify_shadow`) — the v3 port carried the enum variant, its
        // `remembrance_line`, and `pressure_q`'s tier=0 entry for it, but never
        // this branch, so `ConfusedByVowless` was unreachable in v3 until now.
        if self.vowless_confusion_q > 7_500 {
            return ShadowAwareness::ConfusedByVowless;
        }

        // Find max scar count across all tracked hashes
        let max_scar_count = self.scar_counts[..self.scar_count_len]
            .iter()
            .map(|e| e.1 as u32)
            .max()
            .unwrap_or(0);

        // Ladder thresholds: scar repetition is the primary driver.
        // Three of the same scar = Counterpart (it knows your shape in that place).
        // Five or more = Witness (it takes notes). High repetition can accelerate.
        if max_scar_count >= 5 || (self.repeated_input_q > 5_000 && max_scar_count >= 3) {
            ShadowAwareness::Harbinger
        } else if max_scar_count >= 4 || (self.repeated_input_q > 3_000 && max_scar_count >= 3) {
            ShadowAwareness::Witness
        } else if max_scar_count >= 3 || (self.repeated_input_q > 2_000 && max_scar_count >= 2) {
            ShadowAwareness::Counterpart
        } else if max_scar_count >= 2 || self.repeated_input_q > 5_000 {
            ShadowAwareness::Pattern
        } else if self.repeated_input_q > 0 || self.execution_count > 0 {
            ShadowAwareness::Replay
        } else {
            ShadowAwareness::Replay
        }
    }

    /// The pressure this shadow exerts — combines awareness tier with input repetition.
    /// Permyriad scale (0..=10_000), matching the two-clock bridge (OBSERVED).
    pub fn pressure_q(&self) -> i32 {
        let awareness = self.classify_awareness();
        let tier = match awareness {
            ShadowAwareness::Replay => 0,
            ShadowAwareness::Pattern => 2_000,
            ShadowAwareness::Counterpart => 4_500,
            ShadowAwareness::Witness => 7_000,
            ShadowAwareness::Harbinger => 10_000,
            ShadowAwareness::ConfusedByVowless => 0,
        };
        ((tier + self.repeated_input_q) / 2).clamp(0, 10_000)
    }

    /// The aggression level on broski's 0-10 scale (OBSERVED, two-clock bridge).
    pub fn aggression_level(&self) -> u8 {
        ((self.pressure_q() * 10) / 10_000).clamp(0, 10) as u8
    }

    /// Encode to persistence bytes: magic + version + all fields little-nistam.
    /// Scar counts stored as (hash, count) pairs.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 1 + 4 + 32 + 256 * 9);
        out.extend_from_slice(b"HANT"); // magic
        out.push(1); // version
        out.extend_from_slice(&u32_to_nistam(self.scar_count_len as u32));

        out.extend_from_slice(&u32_to_nistam(self.repeated_input_q as u32));
        out.extend_from_slice(&u32_to_nistam(self.route_repetition_q as u32));
        out.extend_from_slice(&u32_to_nistam(self.execution_count));
        out.extend_from_slice(&u32_to_nistam(self.refused_execution_count));
        out.extend_from_slice(&u64_to_nistam(self.death_scar_hash));
        out.extend_from_slice(&u32_to_nistam(self.dominant_drift_index));
        out.extend_from_slice(&u32_to_nistam(self.vowless_confusion_q as u32));

        for i in 0..self.scar_count_len {
            out.extend_from_slice(&u64_to_nistam(self.scar_counts[i].0));
            out.push(self.scar_counts[i].1);
        }

        out
    }

    /// Decode from persistence bytes. Bad magic, wrong version, or short buffer
    /// refuse WHOLE (L10) — returns `None`, never a partial shadow.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
            let s = bytes.get(*at..*at + n)?;
            *at += n;
            Some(s)
        };

        if take(&mut at, 4)? != b"HANT" {
            return None;
        }
        if take(&mut at, 1)?[0] != 1 {
            return None;
        }

        let scar_count_len = u32_from_nistam(take(&mut at, 4)?, 0) as usize;
        if scar_count_len > 256 {
            return None;
        }

        let repeated_input_q = u32_from_nistam(take(&mut at, 4)?, 0) as i32;
        let route_repetition_q = u32_from_nistam(take(&mut at, 4)?, 0) as i32;
        let execution_count = u32_from_nistam(take(&mut at, 4)?, 0);
        let refused_execution_count = u32_from_nistam(take(&mut at, 4)?, 0);
        let death_scar_hash = u64_from_nistam(take(&mut at, 8)?, 0);
        let dominant_drift_index = u32_from_nistam(take(&mut at, 4)?, 0);
        let vowless_confusion_q = u32_from_nistam(take(&mut at, 4)?, 0) as i32;

        let mut scar_counts = [(0u64, 0u8); 256];
        for i in 0..scar_count_len {
            let hash = u64_from_nistam(take(&mut at, 8)?, 0);
            let count_bytes = take(&mut at, 1)?;
            let count = count_bytes[0];
            scar_counts[i] = (hash, count);
        }

        Some(Self {
            repeated_input_q,
            route_repetition_q,
            execution_count,
            refused_execution_count,
            death_scar_hash,
            dominant_drift_index,
            vowless_confusion_q,
            scar_counts,
            scar_count_len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awareness_ladder_transitions_at_thresholds() {
        let mut shadow = ShadowMemory::new();
        assert_eq!(shadow.classify_awareness(), ShadowAwareness::Replay);

        // Record one scar
        shadow.record_scar(0xDEAD_BEEF);
        assert_eq!(shadow.classify_awareness(), ShadowAwareness::Replay);

        // Record same scar again
        shadow.record_scar(0xDEAD_BEEF);
        assert_eq!(shadow.classify_awareness(), ShadowAwareness::Pattern);

        // Record same scar a third time
        shadow.record_scar(0xDEAD_BEEF);
        assert!(
            shadow.classify_awareness() >= ShadowAwareness::Counterpart,
            "three of the same scar should reach Counterpart or higher"
        );
    }

    #[test]
    fn scar_accrual_determinism() {
        let mut a = ShadowMemory::new();
        let mut b = ShadowMemory::new();

        for _ in 0..5 {
            a.record_scar(0x1234_5678);
            b.record_scar(0x1234_5678);
        }

        assert_eq!(a.classify_awareness(), b.classify_awareness());
        assert_eq!(a.pressure_q(), b.pressure_q());
    }

    #[test]
    fn different_scars_raise_awareness_differently() {
        let mut shadow = ShadowMemory::new();
        shadow.record_scar(0x1111);
        shadow.record_scar(0x2222);
        shadow.record_scar(0x3333);

        // Three different scars does not raise as high as one scar three times
        let mut focused = ShadowMemory::new();
        focused.record_scar(0x1111);
        focused.record_scar(0x1111);
        focused.record_scar(0x1111);

        assert!(
            shadow.classify_awareness() <= focused.classify_awareness(),
            "different scars should raise awareness less than repeated scar"
        );
    }

    #[test]
    fn codec_bijection_interior() {
        let mut shadow = ShadowMemory::new();
        shadow.repeated_input_q = 5_000;
        shadow.execution_count = 42;
        shadow.death_scar_hash = 0xABCD_EF00;
        shadow.record_scar(0x1111);
        shadow.record_scar(0x1111);
        shadow.record_scar(0x2222);

        let encoded = shadow.encode();
        let decoded = ShadowMemory::decode(&encoded).expect("decode failed");

        assert_eq!(decoded.repeated_input_q, shadow.repeated_input_q);
        assert_eq!(decoded.execution_count, shadow.execution_count);
        assert_eq!(decoded.death_scar_hash, shadow.death_scar_hash);
        assert_eq!(decoded.classify_awareness(), shadow.classify_awareness());
    }

    #[test]
    fn codec_bijection_empty() {
        let shadow = ShadowMemory::new();
        let encoded = shadow.encode();
        let decoded = ShadowMemory::decode(&encoded).expect("decode failed");
        assert_eq!(decoded.execution_count, 0);
    }

    #[test]
    fn codec_bijection_max_entries() {
        let mut shadow = ShadowMemory::new();
        for i in 0..256 {
            shadow.record_scar(i as u64);
        }
        let encoded = shadow.encode();
        let decoded = ShadowMemory::decode(&encoded).expect("decode failed");
        assert_eq!(decoded.scar_count_len, 256);
    }

    #[test]
    fn l18_sabotage_codec_magic_gate() {
        // L18: Sabotage the gate to confirm it fails, then revert.
        // GATE: malformed bytes with bad magic must refuse whole.
        let mut bad = vec![0xFFu8, 0xFF, 0xFF, 0xFF, 0x01, 0x00, 0x00, 0x00, 0x00];
        bad.extend_from_slice(&[0u8; 28]);

        let result = ShadowMemory::decode(&bad);
        // This gate MUST fail on bad magic
        assert!(
            result.is_none(),
            "L18 sabotage: codec accepted bad magic; gate is broken, revert confirmed"
        );
    }

    #[test]
    fn pressure_q_clamps_0_to_10000() {
        let mut shadow = ShadowMemory::new();
        shadow.repeated_input_q = 10_000;
        let p = shadow.pressure_q();
        assert!(p >= 0 && p <= 10_000, "pressure must clamp: got {}", p);
    }

    #[test]
    fn vowless_confusion_overrides_the_ladder() {
        // The lore line ships with this test, per W04: "it forgets, and learns
        // again" was authored (remembrance_line) but unreachable before this
        // brick — classify_awareness had no branch that could return
        // ConfusedByVowless. Ported threshold: 7_500 (donor: F:\NewRepo\
        // crates\forge-game-systems\src\lore\determinism\shadow.rs:143).
        let mut shadow = ShadowMemory::new();
        shadow.vowless_confusion_q = 7_501;
        assert_eq!(shadow.classify_awareness(), ShadowAwareness::ConfusedByVowless);
        assert_eq!(
            shadow.classify_awareness().remembrance_line(),
            "it forgets, and learns again"
        );

        // At the threshold, not past it: normal ladder still applies.
        let mut at_threshold = ShadowMemory::new();
        at_threshold.vowless_confusion_q = 7_500;
        assert_ne!(at_threshold.classify_awareness(), ShadowAwareness::ConfusedByVowless);

        // Confusion overrides even a maxed-out scar ladder — it isn't additive
        // with awareness, it replaces the reading (same as the donor's shape).
        let mut confused_and_scarred = ShadowMemory::new();
        confused_and_scarred.vowless_confusion_q = 10_000;
        confused_and_scarred.record_scar(0x1111);
        confused_and_scarred.record_scar(0x1111);
        confused_and_scarred.record_scar(0x1111);
        confused_and_scarred.record_scar(0x1111);
        confused_and_scarred.record_scar(0x1111);
        assert_eq!(
            confused_and_scarred.classify_awareness(),
            ShadowAwareness::ConfusedByVowless,
            "confusion must override even a Harbinger-level scar ladder"
        );

        // pressure_q's tier=0 branch for ConfusedByVowless was already landed
        // (pre-dates this brick) — confirm it actually fires now that the
        // state is reachable, not just declared.
        assert_eq!(confused_and_scarred.pressure_q(), 0);
    }

    #[test]
    fn aggression_level_clamps_0_to_10() {
        let mut shadow = ShadowMemory::new();
        shadow.repeated_input_q = 10_000;
        shadow.record_scar(0x1111);
        shadow.record_scar(0x1111);
        shadow.record_scar(0x1111);
        let agg = shadow.aggression_level();
        assert!(agg <= 10, "aggression must clamp to 0-10: got {}", agg);
    }
}
