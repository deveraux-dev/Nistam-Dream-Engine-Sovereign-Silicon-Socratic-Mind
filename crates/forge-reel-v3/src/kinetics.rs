//! Camera kinetics — integer easing between per-column Cam targets.
//! Whip-pan as motion, not teleport. Permyriad math, u64 intermediates, no float.

const CAM_PAN_PX: i64 = 24;
const CAM_TILT_PX: i64 = 14;

/// Camera motion target — five cardinal directions, per dual_rail.rs semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cam {
    /// Tilt down (dy=-14).
    PitchDown,
    /// Tilt back up (dy=+14).
    Recollapse,
    /// Pan left (dx=-24).
    PanLeft,
    /// Pan right (dx=+24).
    PanRight,
    /// Hold steady (dx=0, dy=0).
    Hold,
}

/// (dx, dy) target for `cam`. Mirrors dual_rail.rs::cam_offset_px exactly:
/// PitchDown=(0,-14), Recollapse=(0,14), PanLeft=(-24,0), PanRight=(24,0), Hold=(0,0).
pub fn cam_target(cam: Cam) -> (i64, i64) {
    match cam {
        Cam::PitchDown => (0, -CAM_TILT_PX),
        Cam::Recollapse => (0, CAM_TILT_PX),
        Cam::PanLeft => (-CAM_PAN_PX, 0),
        Cam::PanRight => (CAM_PAN_PX, 0),
        Cam::Hold => (0, 0),
    }
}

/// Integer ease-in-out (quadratic smoothstep), permyriad -> permyriad.
/// u64 intermediates, exact 0->0 / 10000->10000, monotonic. Same shape as
/// forge-book/src/easing.rs::Ease::InOutQuad (t*t/10000, halves at 5000).
pub fn ease_permyriad(t_pm: u32) -> u32 {
    let t = t_pm.min(10_000) as u64;
    let out = if t < 5_000 {
        2 * t * t / 10_000
    } else {
        let u = 10_000 - t;
        10_000 - 2 * u * u / 10_000
    };
    out.min(10_000) as u32
}

/// Per-column camera targets, expanded from a `Cam` sequence.
pub struct CamTrack {
    targets: Vec<(i64, i64)>,
}

impl CamTrack {
    /// Build a track from a sequence of Cam commands.
    pub fn from_cams(cams: &[Cam]) -> Self {
        CamTrack { targets: cams.iter().map(|&c| cam_target(c)).collect() }
    }

    /// Number of columns in the track.
    pub fn len(&self) -> usize {
        self.targets.len()
    }

    /// True if no columns in the track.
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    /// Eased offset at column `col`, sub-column phase `sub_pm` (0..=10000).
    /// Eases from the previous column's target (sub_pm=0) to this column's
    /// target (sub_pm=10000) across one column width. col=0 has no previous
    /// — it holds at its own target for every phase. A run of equal targets
    /// (e.g. consecutive Hold) collapses to zero motion by construction.
    pub fn offset_at(&self, col: usize, sub_pm: u32) -> (i64, i64) {
        if self.targets.is_empty() {
            return (0, 0);
        }
        let idx = col.min(self.targets.len() - 1);
        let cur = self.targets[idx];
        let prev = if idx == 0 { cur } else { self.targets[idx - 1] };
        let e = ease_permyriad(sub_pm) as i64;
        let dx = prev.0 + (cur.0 - prev.0) * e / 10_000;
        let dy = prev.1 + (cur.1 - prev.1) * e / 10_000;
        (dx, dy)
    }
}

/// Expand `cams` to `subframes` eased integer offsets per column — the
/// 1000-drop's smooth camera ride. Column N's last subframe (sub_pm=10000,
/// value=target N) matches column N+1's first (sub_pm=0, value=target N),
/// so the ride is continuous across the cut. Length is exactly
/// `cams.len() * subframes` (0 if either is 0).
pub fn frames_between(cams: &[Cam], subframes: usize) -> Vec<(i64, i64)> {
    let track = CamTrack::from_cams(cams);
    let mut out = Vec::with_capacity(cams.len().saturating_mul(subframes));
    if subframes == 0 {
        return out;
    }
    for col in 0..cams.len() {
        for s in 0..subframes {
            let sub_pm = if subframes == 1 {
                10_000
            } else {
                (s as u64 * 10_000 / (subframes as u64 - 1)) as u32
            };
            out.push(track.offset_at(col, sub_pm));
        }
    }
    out
}

/// Largest per-frame Chebyshev jump across consecutive offsets — the
/// smoothness gauge. A teleport shows up as one huge step; an eased ride
/// spreads the same total distance across many small ones.
pub fn max_step(offsets: &[(i64, i64)]) -> i64 {
    offsets
        .windows(2)
        .map(|w| {
            let dx = (w[1].0 - w[0].0).abs();
            let dy = (w[1].1 - w[0].1).abs();
            dx.max(dy)
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ease_permyriad ───────────────────────────────────────────────────────

    #[test]
    fn ease_endpoints_are_exact() {
        assert_eq!(ease_permyriad(0), 0);
        assert_eq!(ease_permyriad(10_000), 10_000);
    }

    #[test]
    fn ease_clamps_past_10000() {
        assert_eq!(ease_permyriad(20_000), 10_000);
    }

    #[test]
    fn ease_is_monotonic() {
        let mut prev = ease_permyriad(0);
        let mut t = 0u32;
        while t <= 10_000 {
            let v = ease_permyriad(t);
            assert!(v >= prev, "ease not monotonic at t={t}: {v} < {prev}");
            prev = v;
            t += 37;
        }
    }

    #[test]
    fn ease_symmetric_midpoint() {
        assert_eq!(ease_permyriad(5_000), 5_000);
    }

    #[test]
    fn ease_slow_start_fast_middle() {
        // quadratic ease-in-out: quarter point trails linear, midpoint meets it.
        assert!(ease_permyriad(2_500) < 2_500);
        assert!(ease_permyriad(7_500) > 7_500);
    }

    // ── cam_target ────────────────────────────────────────────────────────────

    #[test]
    fn cam_target_mirrors_dual_rail_offsets() {
        assert_eq!(cam_target(Cam::PitchDown), (0, -14));
        assert_eq!(cam_target(Cam::Recollapse), (0, 14));
        assert_eq!(cam_target(Cam::PanLeft), (-24, 0));
        assert_eq!(cam_target(Cam::PanRight), (24, 0));
        assert_eq!(cam_target(Cam::Hold), (0, 0));
    }

    // ── CamTrack::offset_at ──────────────────────────────────────────────────

    #[test]
    fn offset_at_col_zero_holds_at_its_own_target() {
        let track = CamTrack::from_cams(&[Cam::PanRight, Cam::Hold]);
        for sub in [0u32, 2_500, 5_000, 7_500, 10_000] {
            assert_eq!(track.offset_at(0, sub), (24, 0));
        }
    }

    #[test]
    fn offset_at_eases_endpoints_match_targets() {
        let track = CamTrack::from_cams(&[Cam::PanLeft, Cam::PanRight]);
        assert_eq!(track.offset_at(1, 0), cam_target(Cam::PanLeft));
        assert_eq!(track.offset_at(1, 10_000), cam_target(Cam::PanRight));
    }

    #[test]
    fn offset_at_out_of_range_col_clamps_to_last() {
        let track = CamTrack::from_cams(&[Cam::Hold, Cam::PanLeft]);
        assert_eq!(track.offset_at(99, 10_000), track.offset_at(1, 10_000));
    }

    #[test]
    fn offset_at_on_empty_track_is_zero() {
        let track = CamTrack::from_cams(&[]);
        assert_eq!(track.offset_at(0, 5_000), (0, 0));
    }

    // ── Hold ──────────────────────────────────────────────────────────────────

    #[test]
    fn hold_produces_zero_motion() {
        let cams = [Cam::Hold, Cam::Hold, Cam::Hold];
        let offsets = frames_between(&cams, 6);
        assert!(offsets.iter().all(|&(x, y)| x == 0 && y == 0));
        assert_eq!(max_step(&offsets), 0);
    }

    // ── smoothness beats teleport ────────────────────────────────────────────

    #[test]
    fn pan_left_to_pan_right_cut_never_jumps_more_than_half_the_instant_cut() {
        let cams = [Cam::PanLeft, Cam::PanRight];
        let offsets = frames_between(&cams, 8);

        let (lx, ly) = cam_target(Cam::PanLeft);
        let (rx, ry) = cam_target(Cam::PanRight);
        let instant_cut_distance = (rx - lx).abs().max((ry - ly).abs());

        let step = max_step(&offsets);
        assert!(
            step <= instant_cut_distance / 2,
            "eased step {step} exceeded half the teleport distance {instant_cut_distance}"
        );
        assert!(step > 0, "a real pan-to-pan cut must still move");
    }

    // ── frames_between ────────────────────────────────────────────────────────

    #[test]
    fn frames_between_length_law() {
        let cams = [Cam::PitchDown, Cam::Recollapse, Cam::PanLeft, Cam::PanRight, Cam::Hold];
        for subframes in [0usize, 1, 2, 5, 8, 12] {
            let offsets = frames_between(&cams, subframes);
            assert_eq!(offsets.len(), cams.len() * subframes, "subframes={subframes}");
        }
    }

    #[test]
    fn frames_between_on_empty_cams_is_empty() {
        assert!(frames_between(&[], 8).is_empty());
    }

    #[test]
    fn frames_between_is_continuous_across_the_cut() {
        let cams = [Cam::PanLeft, Cam::PitchDown];
        let offsets = frames_between(&cams, 4);
        // last subframe of column 0 must equal the first subframe of column 1
        // (both land on column 0's target — the eased ride never double-jumps
        // at the boundary).
        assert_eq!(offsets[3], offsets[4]);
    }

    #[test]
    fn frames_between_is_deterministic() {
        let cams = [Cam::PanLeft, Cam::Recollapse, Cam::PanRight, Cam::Hold, Cam::PitchDown];
        let a = frames_between(&cams, 10);
        let b = frames_between(&cams, 10);
        assert_eq!(a, b);
    }

    #[test]
    fn frames_between_offsets_bounded_by_pan_magnitudes() {
        let cams = [Cam::PanLeft, Cam::PanRight, Cam::PitchDown, Cam::Recollapse, Cam::Hold];
        for &(dx, dy) in frames_between(&cams, 9).iter() {
            assert!((-24..=24).contains(&dx), "dx {dx} escaped pan bound");
            assert!((-14..=14).contains(&dy), "dy {dy} escaped tilt bound");
        }
    }

    // ── max_step ──────────────────────────────────────────────────────────────

    #[test]
    fn max_step_on_empty_and_single_is_zero() {
        assert_eq!(max_step(&[]), 0);
        assert_eq!(max_step(&[(5, 5)]), 0);
    }

    #[test]
    fn max_step_reads_the_largest_chebyshev_gap() {
        let offsets = [(0, 0), (3, 1), (3, 9), (4, 9)];
        // gaps: (3,1)=3, (0,8)=8, (1,0)=1 -> max is 8
        assert_eq!(max_step(&offsets), 8);
    }

    #[test]
    fn instant_cut_would_have_been_the_full_pan_span() {
        // Sanity receipt: the un-eased teleport this module replaces is a
        // single 48px jump (PanLeft -24 -> PanRight 24). frames_between's
        // eased max_step must always undercut that.
        let instant = [cam_target(Cam::PanLeft), cam_target(Cam::PanRight)];
        assert_eq!(max_step(&instant), 48);
    }
}
