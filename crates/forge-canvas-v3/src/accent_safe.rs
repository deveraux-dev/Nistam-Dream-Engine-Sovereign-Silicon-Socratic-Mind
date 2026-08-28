//! Preattentive accent arbitration + platform safe-area tokens.
//!
//! Per Treisman: two accents == zero accents. This module enforces the rule:
//! one claim per frame wins the preattentive slot. The safe-area logic defines
//! insets for HUD elements based on TV broadcast standards (SMPTE ST 2046).
//!
//! forbid-first: `safe_rect()` computes insets in integer permyriad — no floats.

/// Priority tier for an accent claim — higher wins ties, alerts beat decoration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum AccentPri {
    /// Decoration/polish — lowest priority.
    Decoration = 0,
    /// Emphasis/guidance — medium priority.
    Emphasis = 1,
    /// Alert/critical warning — highest priority.
    Alert = 2,
}

/// A held accent claim — who holds the frame's one preattentive slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccentClaim {
    /// Static string identifier: the holder's name.
    pub who: &'static str,
    /// Priority tier.
    pub pri: AccentPri,
}

/// Per-frame claim slot for the ONE preattentive accent.
/// First claimant at the highest priority holds it; a second claimant is refused.
/// Per Treisman's research, multiple simultaneous preattentive alerts are not
/// perceived as distinct, so enforce one-per-frame strictly.
#[derive(Default)]
pub struct AccentArbiter {
    /// The current holder, if any.
    claimed: Option<AccentClaim>,
}

impl AccentArbiter {
    /// New, unclaimed arbiter.
    pub fn new() -> Self {
        Self { claimed: None }
    }

    /// Attempt to claim the frame's accent slot. Returns true if `who` now holds it.
    /// A higher-priority claimant preempts a lower one; equal-or-lower priority is refused.
    pub fn claim(&mut self, who: &'static str, pri: AccentPri) -> bool {
        match self.claimed {
            None => {
                self.claimed = Some(AccentClaim { who, pri });
                true
            }
            Some(held) if pri > held.pri => {
                self.claimed = Some(AccentClaim { who, pri });
                true
            }
            _ => false,
        }
    }

    /// Clear the slot for the next frame.
    pub fn reset(&mut self) {
        self.claimed = None;
    }

    /// The current holder, if any.
    pub fn holder(&self) -> Option<AccentClaim> {
        self.claimed
    }
}

/// Safe-area region a HUD element anchors inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafeArea {
    /// No inset — full viewport.
    None,
    /// Title-safe: the TIGHTER inner region, 80% of viewport — words live here.
    /// Per SMPTE ST 2046 and BBC guidelines.
    Title,
    /// Action-safe: the looser region, 90% of viewport — art may reach here.
    /// Per SMPTE ST 2046 and BBC guidelines.
    Action,
}

/// Title-safe inset — 80% of viewport, in permyriad (BBC + SMPTE ST 2046). Text only inside this.
pub const TITLE_SAFE_PMY: i64 = 8_000;
/// Action-safe inset — 90% of viewport, in permyriad (BBC + SMPTE ST 2046). Looser than title-safe.
pub const ACTION_SAFE_PMY: i64 = 9_000;

/// Resolve the inset rect (x, y, w, h) a HUD anchors inside for the given safe area.
/// The rect is centered within the viewport, scaled by the safe-area percentage.
/// All math is integer permyriad — forbid-first, no floats.
///
/// Returns: (x, y, width, height) all in the viewport's coordinate system (pixels/units).
pub fn safe_rect(viewport: (i64, i64), area: SafeArea) -> (i64, i64, i64, i64) {
    let (vw, vh) = viewport;
    let pct_pmy = match area {
        SafeArea::None => 10_000,
        SafeArea::Title => TITLE_SAFE_PMY,
        SafeArea::Action => ACTION_SAFE_PMY,
    };
    let w = vw * pct_pmy / 10_000;
    let h = vh * pct_pmy / 10_000;
    let x = (vw - w) / 2;
    let y = (vh - h) / 2;
    (x, y, w, h)
}

#[cfg(test)]
mod accent_tests {
    use super::*;

    // ── L07-style determinism: claim outcome is stable ──────────────────────

    #[test]
    fn single_claim_is_deterministic() {
        let mut a = AccentArbiter::new();
        let r1 = a.claim("first", AccentPri::Decoration);
        a.reset();
        let mut a = AccentArbiter::new();
        let r2 = a.claim("first", AccentPri::Decoration);
        assert_eq!(r1, r2, "identical claims must succeed identically");
    }

    #[test]
    fn holder_is_deterministic() {
        let mut a = AccentArbiter::new();
        a.claim("a", AccentPri::Decoration);
        let h1 = a.holder();
        let h2 = a.holder();
        assert_eq!(h1, h2, "holder() must be deterministic");
    }

    #[test]
    fn single_claim_wins() {
        let mut a = AccentArbiter::new();
        assert!(a.claim("a", AccentPri::Decoration));
        assert_eq!(a.holder().unwrap().who, "a");
    }

    #[test]
    fn second_claim_refused() {
        let mut a = AccentArbiter::new();
        assert!(a.claim("a", AccentPri::Decoration));
        assert!(!a.claim("b", AccentPri::Decoration));
        assert_eq!(a.holder().unwrap().who, "a");
    }

    #[test]
    fn higher_priority_alert_preempts_decoration() {
        let mut a = AccentArbiter::new();
        assert!(a.claim("deco", AccentPri::Decoration));
        assert!(a.claim("alert", AccentPri::Alert));
        assert_eq!(a.holder().unwrap().who, "alert");
    }

    #[test]
    fn equal_priority_second_claim_refused() {
        let mut a = AccentArbiter::new();
        assert!(a.claim("first", AccentPri::Emphasis));
        assert!(!a.claim("second", AccentPri::Emphasis));
        assert_eq!(a.holder().unwrap().who, "first");
    }

    #[test]
    fn lower_priority_never_preempts() {
        let mut a = AccentArbiter::new();
        assert!(a.claim("alert", AccentPri::Alert));
        assert!(!a.claim("deco", AccentPri::Decoration));
        assert_eq!(a.holder().unwrap().who, "alert");
    }

    // ── L18-style sabotage: flip priority comparison ───────────────────────
    // If the priority comparison used < instead of >, a lower-priority claim
    // would preempt a higher one. We verify the correct direction.

    #[test]
    fn priority_ordering_is_correct() {
        assert!(AccentPri::Alert > AccentPri::Emphasis);
        assert!(AccentPri::Emphasis > AccentPri::Decoration);
        assert!(!(AccentPri::Decoration > AccentPri::Alert));
    }

    #[test]
    fn reset_clears_holder() {
        let mut a = AccentArbiter::new();
        a.claim("x", AccentPri::Alert);
        assert!(a.holder().is_some());
        a.reset();
        assert!(a.holder().is_none());
    }

    #[test]
    fn reset_allows_new_claim() {
        let mut a = AccentArbiter::new();
        a.claim("first", AccentPri::Decoration);
        a.reset();
        assert!(a.claim("second", AccentPri::Decoration));
        assert_eq!(a.holder().unwrap().who, "second");
    }
}

#[cfg(test)]
mod safe_area_tests {
    use super::*;

    // ── L07-style determinism: safe rect is stable ────────────────────────

    #[test]
    fn safe_rect_is_deterministic() {
        let vp = (1920, 1080);
        let r1 = safe_rect(vp, SafeArea::Title);
        let r2 = safe_rect(vp, SafeArea::Title);
        assert_eq!(r1, r2, "safe_rect() must be deterministic");
    }

    #[test]
    fn title_inset_tighter_than_action() {
        let vp = (1920, 1080);
        let title = safe_rect(vp, SafeArea::Title);
        let action = safe_rect(vp, SafeArea::Action);
        assert!(title.2 < action.2 && title.3 < action.3);
    }

    #[test]
    fn none_returns_full_viewport() {
        let vp = (1920, 1080);
        let r = safe_rect(vp, SafeArea::None);
        assert_eq!(r, (0, 0, 1920, 1080));
    }

    #[test]
    fn safe_area_is_centered() {
        let vp = (1000, 1000);
        let r = safe_rect(vp, SafeArea::Title); // 80%
        let expected_size = 1000 * 8_000 / 10_000;
        let expected_offset = (1000 - expected_size) / 2;
        assert_eq!(r, (expected_offset, expected_offset, expected_size, expected_size));
    }

    #[test]
    fn title_safe_pmy_is_eighty_percent() {
        assert_eq!(TITLE_SAFE_PMY, 8_000);
    }

    #[test]
    fn action_safe_pmy_is_ninety_percent() {
        assert_eq!(ACTION_SAFE_PMY, 9_000);
    }

    #[test]
    fn safe_area_constants_are_ordered() {
        assert!(TITLE_SAFE_PMY < ACTION_SAFE_PMY);
        assert!(ACTION_SAFE_PMY < 10_000);
    }

    // ── L18-style sabotage: flip centering logic ─────────────────────────
    // If we removed the (vw - w) / 2 centering and just set x=0, the rect
    // would be left-aligned, not centered. We verify centering happens.

    #[test]
    fn safe_rect_is_horizontally_centered() {
        let vp = (2000, 1000);
        let r = safe_rect(vp, SafeArea::Title);
        // Width should be 2000 * 80% = 1600
        // x offset should be (2000 - 1600) / 2 = 200
        assert_eq!(r.0, 200, "x offset must center the rect");
    }

    #[test]
    fn safe_rect_is_vertically_centered() {
        let vp = (1000, 2000);
        let r = safe_rect(vp, SafeArea::Title);
        // Height should be 2000 * 80% = 1600
        // y offset should be (2000 - 1600) / 2 = 200
        assert_eq!(r.1, 200, "y offset must center the rect");
    }

    #[test]
    fn safe_rect_scales_with_viewport() {
        let vp_small = (1000, 1000);
        let vp_large = (2000, 2000);
        let r_small = safe_rect(vp_small, SafeArea::Title);
        let r_large = safe_rect(vp_large, SafeArea::Title);
        // Both should be 80%, so large should be exactly double.
        assert_eq!(r_large.2, r_small.2 * 2);
        assert_eq!(r_large.3, r_small.3 * 2);
    }

    #[test]
    fn safe_rect_handles_non_square_viewports() {
        let vp = (1920, 1080); // 16:9
        let r = safe_rect(vp, SafeArea::Action); // 90%
        let expected_w = 1920 * 9_000 / 10_000;
        let expected_h = 1080 * 9_000 / 10_000;
        assert_eq!(r.2, expected_w);
        assert_eq!(r.3, expected_h);
    }
}
