//! Gesture-to-asset brush — Laban / BESS effort-driven deterministic
//! deformation. Ported from `F:\NewRepo\crates\forge-core\src\gesture_brush.rs`
//! (v2 Crate Zero), TRIMMED to the self-contained BESS-classification half
//! (`GestureStroke`/`BessEffort`/`BrushOp`/`select_operator`) — forge-audio-v3's
//! only real need is `BrushOp` (`recipe/ce_audio.rs`). The excluded half
//! (`REGION`/`fox_tail_region`/`apply_gesture`) needs `mesh_hub`/
//! `surfaceledger`/`surfaceledger_hash`/`colour_ir`, none of which exist
//! anywhere in `F:\v3` — a separate, real, not-yet-scoped port, named here
//! rather than silently dropped.

use serde::{Deserialize, Serialize};

// ── 1. GestureStroke ────────────────────────────────────────────────────────

/// A raw gesture stroke. `points` is the integer pixel/atlas path; the four
/// `_q` fields are quantised Laban effort factors in the range `0..=255`
/// (`_q` = quantised). Quantisation is what makes classification deterministic
/// — no floating point enters the brush at any stage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GestureStroke {
    pub points: Vec<[i32; 2]>,
    pub pressure_q: u8,
    pub speed_q: u8,
    pub directness_q: u8,
    pub continuity_q: u8,
}

impl GestureStroke {
    pub fn from_efforts(pressure_q: u8, speed_q: u8, directness_q: u8, continuity_q: u8) -> Self {
        GestureStroke { points: Vec::new(), pressure_q, speed_q, directness_q, continuity_q }
    }

    pub fn rms_modulated(&self, rms_q: u8) -> GestureStroke {
        let mut s = self.clone();
        s.pressure_q = s.pressure_q.saturating_add(rms_q / 4);
        s
    }
}

// ── 2. BessEffort ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weight { Light, Strong }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Time { Quick, Sustained }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Space { Direct, Flexible }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Flow { Bound, Free }

/// A fully-classified Laban / BESS effort — the four binary factors. 16 total
/// combinations, every one of which resolves to an operator (see
/// [`select_operator`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BessEffort {
    pub weight: Weight,
    pub time: Time,
    pub space: Space,
    pub flow: Flow,
}

/// Quantised-factor threshold. A `_q` value `>= 128` is the "high" pole of its
/// factor — the exact midpoint of `u8`, so the split is total and symmetric.
pub const EFFORT_THRESHOLD: u8 = 128;

pub fn classify_effort(stroke: &GestureStroke) -> BessEffort {
    BessEffort {
        weight: if stroke.pressure_q >= EFFORT_THRESHOLD { Weight::Strong } else { Weight::Light },
        time: if stroke.speed_q >= EFFORT_THRESHOLD { Time::Quick } else { Time::Sustained },
        space: if stroke.directness_q >= EFFORT_THRESHOLD { Space::Direct } else { Space::Flexible },
        flow: if stroke.continuity_q >= EFFORT_THRESHOLD { Flow::Bound } else { Flow::Free },
    }
}

// ── 3. The three operators ──────────────────────────────────────────────────

/// Laban's eight basic effort actions — the complete `Weight × Time × Space`
/// octet, one operator per corner of the cube.
///
/// WAS three (Press/Flick/Wring), with a note reading "no more will be added
/// under this ticket — the brush is deliberately not generalised". That note
/// was TICKET-SCOPED and this is the later ticket (`laban-octet-complete`),
/// which asks for the octet by name. The three original operators keep their
/// exact signatures and their exact sonic profiles; the five that were always
/// implied by the cube are now named rather than folded onto their nearest
/// neighbour by [`select_operator`]'s scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BrushOp {
    /// `Strong + Direct + Sustained` → deepen / extrude.
    Press,
    /// `Light + Flexible + Quick` → edge jitter / fur / feather detail.
    Flick,
    /// `Strong + Flexible + Sustained` → twist / spiral.
    Wring,
    /// `Strong + Direct + Quick` → drive straight in and stop.
    Punch,
    /// `Strong + Flexible + Quick` → tear across.
    Slash,
    /// `Light + Direct + Quick` → touch and leave.
    Dab,
    /// `Light + Direct + Sustained` → draw a long even line.
    Glide,
    /// `Light + Flexible + Sustained` → drift, going nowhere in particular.
    Float,
}

impl BrushOp {
    /// The canonical `(Weight, Time, Space)` signature each operator was
    /// specified for. `Flow` is intentionally NOT part of the signature.
    fn signature(self) -> (Weight, Time, Space) {
        match self {
            BrushOp::Press => (Weight::Strong, Time::Sustained, Space::Direct),
            BrushOp::Flick => (Weight::Light, Time::Quick, Space::Flexible),
            BrushOp::Wring => (Weight::Strong, Time::Sustained, Space::Flexible),
            BrushOp::Punch => (Weight::Strong, Time::Quick, Space::Direct),
            BrushOp::Slash => (Weight::Strong, Time::Quick, Space::Flexible),
            BrushOp::Dab => (Weight::Light, Time::Quick, Space::Direct),
            BrushOp::Glide => (Weight::Light, Time::Sustained, Space::Direct),
            BrushOp::Float => (Weight::Light, Time::Sustained, Space::Flexible),
        }
    }
}

/// Fixed operator priority order. Used both to iterate all operators and to
/// break ties in [`select_operator`] deterministically (earlier wins).
///
/// The original three lead so that any caller which iterated `ALL_OPS` and
/// took the first match keeps the answer it had before the octet landed.
pub const ALL_OPS: [BrushOp; 8] = [
    BrushOp::Press,
    BrushOp::Flick,
    BrushOp::Wring,
    BrushOp::Punch,
    BrushOp::Slash,
    BrushOp::Dab,
    BrushOp::Glide,
    BrushOp::Float,
];

/// Select the operator for an effort. **Total by construction** — every one
/// of the 16 efforts resolves to exactly one `BrushOp`, never `None`.
///
/// Now EXACT as well as total: the eight operators tile the whole
/// `Weight × Time × Space` cube, so the winning score is always 3 and the
/// tie-break never decides anything. Before the octet, five of the eight
/// signatures had no operator of their own and were folded onto whichever of
/// the three scored 2 — a punch was silently rendered as a press.
pub fn select_operator(effort: &BessEffort) -> BrushOp {
    let mut best = ALL_OPS[0];
    let mut best_score = -1_i32;
    for op in ALL_OPS {
        let (w, t, s) = op.signature();
        let score = (w == effort.weight) as i32
            + (t == effort.time) as i32
            + (s == effort.space) as i32;
        if score > best_score {
            best_score = score;
            best = op;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── The octet (laban-octet-complete, 2026-08-26) ────────────────────

    /// The eight operators tile the cube exactly: one per corner, no corner
    /// twice, no corner missing. That is what makes the octet complete rather
    /// than merely larger.
    #[test]
    fn the_eight_operators_tile_the_whole_cube() {
        let mut seen = std::collections::HashSet::new();
        for op in ALL_OPS {
            assert!(seen.insert(op.signature()), "{op:?} repeats a corner");
        }
        assert_eq!(seen.len(), 8, "2 x 2 x 2 = 8 corners");

        for w in [Weight::Strong, Weight::Light] {
            for t in [Time::Sustained, Time::Quick] {
                for s in [Space::Direct, Space::Flexible] {
                    assert!(seen.contains(&(w, t, s)), "no operator for {w:?}/{t:?}/{s:?}");
                }
            }
        }
    }

    /// Every effort now resolves EXACTLY — the winning score is 3, so the
    /// tie-break decides nothing. Before the octet, five of the eight
    /// signatures scored at most 2 and were folded onto a neighbour.
    #[test]
    fn every_effort_resolves_to_its_own_corner_not_a_neighbour() {
        for w in [Weight::Strong, Weight::Light] {
            for t in [Time::Sustained, Time::Quick] {
                for s in [Space::Direct, Space::Flexible] {
                    for flow in [Flow::Bound, Flow::Free] {
                        let effort = BessEffort { weight: w, time: t, space: s, flow };
                        let op = select_operator(&effort);
                        assert_eq!(
                            op.signature(),
                            (w, t, s),
                            "{w:?}/{t:?}/{s:?} resolved to {op:?}, a different corner"
                        );
                    }
                }
            }
        }
    }

    /// Flow is deliberately outside the signature, so the two flows of one
    /// effort must land on the same operator.
    #[test]
    fn flow_does_not_change_which_operator_is_chosen() {
        for w in [Weight::Strong, Weight::Light] {
            for t in [Time::Sustained, Time::Quick] {
                for s in [Space::Direct, Space::Flexible] {
                    let bound = BessEffort { weight: w, time: t, space: s, flow: Flow::Bound };
                    let free = BessEffort { weight: w, time: t, space: s, flow: Flow::Free };
                    assert_eq!(select_operator(&bound), select_operator(&free));
                }
            }
        }
    }

    /// The three originals keep their exact signatures — the octet extends,
    /// it does not re-cut what was already decided.
    #[test]
    fn the_original_three_are_unmoved() {
        assert_eq!(BrushOp::Press.signature(), (Weight::Strong, Time::Sustained, Space::Direct));
        assert_eq!(BrushOp::Flick.signature(), (Weight::Light, Time::Quick, Space::Flexible));
        assert_eq!(BrushOp::Wring.signature(), (Weight::Strong, Time::Sustained, Space::Flexible));
        assert_eq!(ALL_OPS[0], BrushOp::Press, "and still lead the priority order");
        assert_eq!(ALL_OPS[1], BrushOp::Flick);
        assert_eq!(ALL_OPS[2], BrushOp::Wring);
    }

    #[test]
    fn press_is_strong_direct_sustained() {
        let effort = BessEffort { weight: Weight::Strong, time: Time::Sustained, space: Space::Direct, flow: Flow::Bound };
        assert_eq!(select_operator(&effort), BrushOp::Press);
    }

    #[test]
    fn flick_is_light_quick_flexible() {
        let effort = BessEffort { weight: Weight::Light, time: Time::Quick, space: Space::Flexible, flow: Flow::Free };
        assert_eq!(select_operator(&effort), BrushOp::Flick);
    }

    #[test]
    fn wring_is_strong_sustained_flexible() {
        let effort = BessEffort { weight: Weight::Strong, time: Time::Sustained, space: Space::Flexible, flow: Flow::Bound };
        assert_eq!(select_operator(&effort), BrushOp::Wring);
    }

    #[test]
    fn every_quantised_stroke_resolves_to_an_operator() {
        for p in [0u8, 127, 128, 255] {
            for s in [0u8, 127, 128, 255] {
                for d in [0u8, 127, 128, 255] {
                    for c in [0u8, 127, 128, 255] {
                        let stroke = GestureStroke::from_efforts(p, s, d, c);
                        let effort = classify_effort(&stroke);
                        let _ = select_operator(&effort); // must not panic; total by construction
                    }
                }
            }
        }
    }
}
