//! Reactive-edge `.vixi` DSL lowering — `vibematrix.src -> visual.tgt bounded=Z`
//! lines → `[ReactiveBind]` (the runtime form the `look_composite_fs` fragment
//! reads from its set-2 storage buffer).
//!
//! Cold path — invoked when a `.vixi` look profile is loaded / edited, NEVER on
//! the 120Hz tick or the render-submit path. All `String`/`Vec` usage is expected
//! and marked `// @forge:allow_alloc: cold path`.
//!
//! Follows the **Sieve DSL parse pattern** (`forge-gui::sieve_dsl`): a sovereign,
//! hand-written, line-oriented parser that lives BESIDE the type it produces
//! (`ReactiveBind`, this crate) rather than inside `forge-vix` — exactly as
//! `sieve_dsl` sits beside its `DslRuleSet` consumer, not in the grammar crate.
//! It is grammar-GATED (every `src`/`tgt` name must resolve to a known channel —
//! an unknown token is a hard error, never a silent drop) and round-trips through
//! [`pretty_print_reactive_dsl`](crate::reactive::pretty_print_reactive_dsl).
//!
//! Grammar (FROZEN v1, no improvements):
//! ```text
//!   profile = edge*
//!   edge    = "vibematrix." src "->" "visual." tgt "bounded=" permyriad
//!   src     = "combo_heat" | "artifact_glow" | "chromatic" | "distortion"
//!   tgt     = "emissive" | "opacity" | "bloom" | "warp"
//!   permyriad = 0..=10000   (clamped; the authored influence ceiling)
//! ```
//! Blank lines and `#` / `//` comment lines are skipped.
//!
//! ## Drain provenance
//!
//! Ported from v2 source: `F:\NewRepo\crates\forge-gpu\src\reactive_dsl.rs` (~369 lines).
//! The v2 `ReactiveBind` held `src`/`tgt` as raw u32 constants (`SRC_*`, `TGT_*` from
//! `look_composite_pass`); in v3 the enums
//! [`EdgeSource`](crate::reactive::EdgeSource) and
//! [`EdgeTarget`](crate::reactive::EdgeTarget) REPLACE those
//! constants, preserving the frozen grammar and all aliases (`chromatic_aberration`,
//! `distortion_level` for sources; `void`, `pinch` for warp).

use std::error::Error;
use std::fmt;

/// VibeMatrix source channels available to reactive edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeSource {
    /// Combo heat signal (index 0).
    ComboHeat,
    /// Artifact glow signal (index 1).
    ArtifactGlow,
    /// Chromatic aberration signal (index 2). Aliased as `chromatic_aberration` in authored text.
    Chromatic,
    /// Distortion signal (index 3). Aliased as `distortion_level` in authored text.
    Distortion,
}

impl EdgeSource {
    /// Canonical name for pretty-printing.
    fn canonical(self) -> &'static str {
        match self {
            EdgeSource::ComboHeat => "combo_heat",
            EdgeSource::ArtifactGlow => "artifact_glow",
            EdgeSource::Chromatic => "chromatic",
            EdgeSource::Distortion => "distortion",
        }
    }
}

/// Visual target channels a reactive edge can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeTarget {
    /// Emissive channel (index 0).
    Emissive,
    /// Opacity channel (index 1).
    Opacity,
    /// Bloom channel (index 2).
    Bloom,
    /// Warp channel (index 3). Aliased as `void` and `pinch` in authored text.
    Warp,
}

impl EdgeTarget {
    /// Canonical name for pretty-printing.
    fn canonical(self) -> &'static str {
        match self {
            EdgeTarget::Emissive => "emissive",
            EdgeTarget::Opacity => "opacity",
            EdgeTarget::Bloom => "bloom",
            EdgeTarget::Warp => "warp",
        }
    }
}

/// The maximum number of reactive edges bound at once (fixed-capacity storage buffer).
pub const MAX_BINDS: usize = 64;

/// One lowered reactive edge: a vibematrix source driving a visual target with a bounded influence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReactiveBind {
    /// Source vibematrix channel.
    pub src: EdgeSource,
    /// Target visual channel.
    pub tgt: EdgeTarget,
    /// Bounded influence (permyriad, 0..=10000).
    pub bounded_q: u32,
}

/// Parse error with 1-based line location (mirrors `sieve_dsl::DslParseError`).
#[derive(Debug, Clone)]
pub struct ReactiveDslError {
    /// The 1-based line number where the error occurred.
    pub line: usize,
    /// The error message.
    pub message: String, // @forge:allow_alloc: cold path (error reporting)
}

impl fmt::Display for ReactiveDslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "reactive DSL error at line {}: {}", self.line, self.message)
    }
}

impl Error for ReactiveDslError {}

/// Resolve a `vibematrix.<name>` source channel to its canonical `EdgeSource`.
/// Accepts the short aliases used in authored profiles (`chromatic`,
/// `distortion`) alongside the full field names. Unknown ⇒ `Err` (no silent drop).
fn resolve_src(name: &str) -> Result<EdgeSource, ()> {
    match name {
        "combo_heat" => Ok(EdgeSource::ComboHeat),
        "artifact_glow" => Ok(EdgeSource::ArtifactGlow),
        "chromatic" | "chromatic_aberration" => Ok(EdgeSource::Chromatic),
        "distortion" | "distortion_level" => Ok(EdgeSource::Distortion),
        _ => Err(()),
    }
}

/// Resolve a `visual.<name>` target channel to its canonical `EdgeTarget`.
fn resolve_tgt(name: &str) -> Result<EdgeTarget, ()> {
    match name {
        "emissive" => Ok(EdgeTarget::Emissive),
        "opacity" => Ok(EdgeTarget::Opacity),
        "bloom" => Ok(EdgeTarget::Bloom),
        "warp" | "void" | "pinch" => Ok(EdgeTarget::Warp),
        _ => Err(()),
    }
}

/// Strip a required `prefix.` and return the suffix, or `Err` if absent.
fn after_dot<'a>(tok: &'a str, prefix: &str) -> Result<&'a str, ()> {
    tok.strip_prefix(prefix).ok_or(())
}

/// Parse a reactive-edge `.vixi` profile into the runtime `[ReactiveBind]`.
///
/// Each non-blank, non-comment line MUST be a complete edge; a malformed line is
/// a hard `Err` (the gate — an authored typo never silently produces a dead
/// look). At most [`MAX_BINDS`] edges are accepted; the `MAX_BINDS + 1`th is an
/// error rather than a silent truncation.
///
/// @forge:allow_alloc: entire function is cold path (`.vixi` load/edit).
pub fn parse_reactive_dsl(input: &str) -> Result<Vec<ReactiveBind>, ReactiveDslError> {
    let mut binds: Vec<ReactiveBind> = Vec::new(); // @forge:allow_alloc: cold path

    for (idx, line) in input.lines().enumerate() {
        let line_num = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect(); // @forge:allow_alloc: cold path
        if tokens.len() != 4 {
            return Err(ReactiveDslError {
                line: line_num,
                message: format!(
                    "expected 'vibematrix.<src> -> visual.<tgt> bounded=<Z>', found {} token(s)",
                    tokens.len()
                ), // @forge:allow_alloc: cold path
            });
        }

        // tok[0]: vibematrix.<src>
        let src_name = after_dot(tokens[0], "vibematrix.").map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("edge must start with 'vibematrix.', found '{}'", tokens[0]), // @forge:allow_alloc: cold path
        })?;
        let src = resolve_src(src_name).map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("unknown vibematrix source '{src_name}'"), // @forge:allow_alloc: cold path
        })?;

        // tok[1]: ->
        if tokens[1] != "->" {
            return Err(ReactiveDslError {
                line: line_num,
                message: format!("expected '->', found '{}'", tokens[1]), // @forge:allow_alloc: cold path
            });
        }

        // tok[2]: visual.<tgt>
        let tgt_name = after_dot(tokens[2], "visual.").map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("edge target must start with 'visual.', found '{}'", tokens[2]), // @forge:allow_alloc: cold path
        })?;
        let tgt = resolve_tgt(tgt_name).map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("unknown visual target '{tgt_name}'"), // @forge:allow_alloc: cold path
        })?;

        // tok[3]: bounded=<Z>
        let z_str = after_dot(tokens[3], "bounded=").map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("expected 'bounded=<Z>', found '{}'", tokens[3]), // @forge:allow_alloc: cold path
        })?;
        let z: i64 = z_str.parse().map_err(|_| ReactiveDslError {
            line: line_num,
            message: format!("invalid bounded permyriad '{z_str}'"), // @forge:allow_alloc: cold path
        })?;
        // Clamp to the permyriad range in INTEGER space (exact ceiling, never a
        // float-drifted approximation — same discipline as `look_composite`).
        let bounded_q = z.clamp(0, 10_000) as u32;

        if binds.len() >= MAX_BINDS {
            return Err(ReactiveDslError {
                line: line_num,
                message: format!("too many reactive edges (max {MAX_BINDS})"), // @forge:allow_alloc: cold path
            });
        }
        binds.push(ReactiveBind { src, tgt, bounded_q });
    }

    Ok(binds)
}

/// Format `[ReactiveBind]` back to canonical reactive-edge `.vixi` text. Round-
/// trips with [`parse_reactive_dsl`] (modulo comment/blank lines + clamping).
///
/// @forge:allow_alloc: entire function is cold path (returns `String`).
pub fn pretty_print_reactive_dsl(binds: &[ReactiveBind]) -> String {
    let mut out = String::new(); // @forge:allow_alloc: cold path
    for (i, b) in binds.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        // @forge:allow_alloc: cold path — format! builds DSL text
        let frag = format!(
            "vibematrix.{} -> visual.{} bounded={}",
            b.src.canonical(),
            b.tgt.canonical(),
            b.bounded_q
        );
        out.push_str(&frag);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_no_edges() {
        assert!(parse_reactive_dsl("").unwrap().is_empty());
        assert!(parse_reactive_dsl("   \n\n  \n").unwrap().is_empty());
    }

    #[test]
    fn comments_and_blanks_are_skipped() {
        let src = "# the dream-test look profile\n\
                   // combo heat fades the layer in\n\
                   vibematrix.combo_heat -> visual.opacity bounded=9000\n";
        let binds = parse_reactive_dsl(src).unwrap();
        assert_eq!(binds.len(), 1);
    }

    #[test]
    fn the_dream_test_edge_lowers_to_the_hand_built_bind() {
        // The exact edge dream-test used to hand-build:
        //   ReactiveBind { src: EdgeSource::ComboHeat, tgt: EdgeTarget::Opacity, bounded_q: 9000 }
        let binds = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity bounded=9000").unwrap();
        assert_eq!(binds.len(), 1);
        assert_eq!(binds[0].src, EdgeSource::ComboHeat);
        assert_eq!(binds[0].tgt, EdgeTarget::Opacity);
        assert_eq!(binds[0].bounded_q, 9000);
    }

    #[test]
    fn all_channels_resolve() {
        let src = "vibematrix.combo_heat -> visual.opacity bounded=1\n\
                   vibematrix.artifact_glow -> visual.bloom bounded=2\n\
                   vibematrix.chromatic -> visual.emissive bounded=3\n\
                   vibematrix.distortion -> visual.opacity bounded=4\n";
        let binds = parse_reactive_dsl(src).unwrap();
        assert_eq!(binds.len(), 4);
        assert_eq!((binds[0].src, binds[0].tgt), (EdgeSource::ComboHeat, EdgeTarget::Opacity));
        assert_eq!((binds[1].src, binds[1].tgt), (EdgeSource::ArtifactGlow, EdgeTarget::Bloom));
        assert_eq!((binds[2].src, binds[2].tgt), (EdgeSource::Chromatic, EdgeTarget::Emissive));
        assert_eq!((binds[3].src, binds[3].tgt), (EdgeSource::Distortion, EdgeTarget::Opacity));
    }

    #[test]
    fn warp_target_resolves_and_round_trips() {
        // The Sean 2026-06-16 steer: bass + bloom ride a void-compression WARP +
        // glow, NOT an opacity fade. The new authorable look profile:
        let src = "vibematrix.distortion -> visual.warp bounded=1500\n\
                   vibematrix.combo_heat -> visual.bloom bounded=6000";
        let binds = parse_reactive_dsl(src).unwrap();
        assert_eq!(binds.len(), 2);
        assert_eq!((binds[0].src, binds[0].tgt), (EdgeSource::Distortion, EdgeTarget::Warp));
        assert_eq!((binds[1].src, binds[1].tgt), (EdgeSource::ComboHeat, EdgeTarget::Bloom));
        // No edge drives opacity anymore — the fade is gone.
        assert!(binds.iter().all(|b| b.tgt != EdgeTarget::Opacity));
        // Round-trips: `warp` survives pretty-print → reparse.
        let reparsed = parse_reactive_dsl(&pretty_print_reactive_dsl(&binds)).unwrap();
        assert_eq!(reparsed[0].tgt, EdgeTarget::Warp);
    }

    #[test]
    fn warp_aliases_resolve() {
        let binds = parse_reactive_dsl(
            "vibematrix.distortion -> visual.void bounded=1\n\
             vibematrix.distortion -> visual.pinch bounded=2",
        )
        .unwrap();
        assert_eq!(binds[0].tgt, EdgeTarget::Warp);
        assert_eq!(binds[1].tgt, EdgeTarget::Warp);
    }

    #[test]
    fn full_field_aliases_resolve() {
        let binds = parse_reactive_dsl(
            "vibematrix.chromatic_aberration -> visual.opacity bounded=500\n\
             vibematrix.distortion_level -> visual.bloom bounded=500",
        )
        .unwrap();
        assert_eq!(binds[0].src, EdgeSource::Chromatic);
        assert_eq!(binds[1].src, EdgeSource::Distortion);
    }

    #[test]
    fn over_range_bounded_clamps_to_10000() {
        let binds = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity bounded=50000").unwrap();
        assert_eq!(binds[0].bounded_q, 10_000);
    }

    #[test]
    fn negative_bounded_clamps_to_zero() {
        let binds = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity bounded=-7").unwrap();
        assert_eq!(binds[0].bounded_q, 0);
    }

    #[test]
    fn unknown_source_is_a_hard_error_not_a_silent_drop() {
        let err = parse_reactive_dsl("vibematrix.bogus -> visual.opacity bounded=5000").unwrap_err();
        assert_eq!(err.line, 1);
        assert!(err.message.contains("unknown vibematrix source"));
    }

    #[test]
    fn unknown_target_is_a_hard_error() {
        let err = parse_reactive_dsl("vibematrix.combo_heat -> visual.glitter bounded=5000").unwrap_err();
        assert!(err.message.contains("unknown visual target"));
    }

    #[test]
    fn missing_arrow_is_an_error() {
        let err = parse_reactive_dsl("vibematrix.combo_heat => visual.opacity bounded=5000").unwrap_err();
        assert!(err.message.contains("expected '->'"));
    }

    #[test]
    fn wrong_prefix_is_an_error() {
        let err = parse_reactive_dsl("audio.combo_heat -> visual.opacity bounded=5000").unwrap_err();
        assert!(err.message.contains("must start with 'vibematrix.'"));
    }

    #[test]
    fn bad_bounded_keyword_is_an_error() {
        let err = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity cap=5000").unwrap_err();
        assert!(err.message.contains("expected 'bounded=<Z>'"));
    }

    #[test]
    fn non_integer_bounded_is_an_error() {
        let err = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity bounded=lots").unwrap_err();
        assert!(err.message.contains("invalid bounded permyriad"));
    }

    #[test]
    fn wrong_token_count_reports_line() {
        let err = parse_reactive_dsl("vibematrix.combo_heat -> visual.opacity").unwrap_err();
        assert_eq!(err.line, 1);
        assert!(err.message.contains("found 3 token(s)"));
    }

    #[test]
    fn error_carries_the_right_line_number() {
        let src = "vibematrix.combo_heat -> visual.opacity bounded=1\n\
                   vibematrix.combo_heat -> visual.opacity bounded=2\n\
                   vibematrix.nope -> visual.opacity bounded=3";
        let err = parse_reactive_dsl(src).unwrap_err();
        assert_eq!(err.line, 3);
    }

    #[test]
    fn exceeding_max_binds_errors_not_truncates() {
        // MAX_BINDS valid edges parse; the next one is a hard error.
        let mut src = String::new();
        for _ in 0..MAX_BINDS {
            src.push_str("vibematrix.combo_heat -> visual.opacity bounded=1\n");
        }
        assert_eq!(parse_reactive_dsl(&src).unwrap().len(), MAX_BINDS);
        src.push_str("vibematrix.combo_heat -> visual.opacity bounded=1\n");
        let err = parse_reactive_dsl(&src).unwrap_err();
        assert!(err.message.contains("too many reactive edges"));
    }

    #[test]
    fn round_trips_through_pretty_printer() {
        let src = "vibematrix.combo_heat -> visual.opacity bounded=9000\n\
                   vibematrix.artifact_glow -> visual.bloom bounded=2500";
        let parsed = parse_reactive_dsl(src).unwrap();
        let printed = pretty_print_reactive_dsl(&parsed);
        let reparsed = parse_reactive_dsl(&printed).unwrap();
        assert_eq!(parsed.len(), reparsed.len());
        for (a, b) in parsed.iter().zip(reparsed.iter()) {
            assert_eq!((a.src, a.tgt, a.bounded_q), (b.src, b.tgt, b.bounded_q));
        }
    }
}
