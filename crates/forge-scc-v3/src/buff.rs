//! Domain 3: Buff Compiler — additive synthesis over assimilator gap queues.
//!
//! Reads `native_widget_queue.json` (produced by an FFI-UI assimilator pass) and
//! emits native boilerplate for every unbuilt gap:
//!   - `native_missing_widget` → `.vixi` kit skeleton
//!   - `native_missing_token`  → Rust struct with a `Permyriad` boundary
//!   - `engine_overlay`        → overlay `.vixi` kit skeleton
//!   - `render_harvest`        → WGSL `@vertex`/`@fragment` stub
//!
//! Output is candidate scaffolding annotated with `# BUFF:` / `// BUFF:`
//! provenance headers — not auto-wired. HITL verification is required before
//! any generated file is committed as live code.
//!
//! Generic: the same driver works for any corpus (astro, forge_app_slapp,
//! analog_console, …) by pointing at the relevant `native_widget_queue.json`.
//!
//! Ported from `F:\NewRepo\crates\scc\src\buff.rs` (2026-08-15). Test fixtures
//! copied in under `corpora/` (was `../../ffi-ui-assimilator-001/corpora/` in
//! v2, a separate crate this workspace doesn't have); `include_str!` paths
//! adjusted accordingly. Logic unchanged.

use crate::contract::{GapReport, Verdict};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ── Input JSON types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WidgetQueue {
    items: Vec<QueueItem>,
}

#[derive(Debug, Deserialize)]
struct QueueItem {
    classification: String,
    id: String,
    #[serde(default)]
    recommendation: String,
    #[serde(default)]
    source: String,
}

#[derive(Debug, Deserialize)]
struct GapReportJson {
    #[serde(default)]
    gaps: Vec<GapDetail>,
}

#[derive(Debug, Deserialize)]
struct GapDetail {
    id: String,
    #[serde(default)]
    next_action: String,
    #[serde(default)]
    reason: String,
}

// ── Output types ─────────────────────────────────────────────────────────────

/// What kind of scaffold a gap resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactKind {
    /// A `.vixi` kit skeleton for a missing widget.
    VixiKit,
    /// A Rust struct stub with a `Permyriad` boundary for a missing token.
    RustToken,
    /// A WGSL `@vertex`/`@fragment` stub for a render-harvest gap.
    WgslStub,
    /// An overlay `.vixi` kit skeleton for an engine overlay.
    OverlayKit,
}

impl ArtifactKind {
    /// Stable snake_case token for this kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            ArtifactKind::VixiKit    => "vixi_kit",
            ArtifactKind::RustToken  => "rust_token",
            ArtifactKind::WgslStub   => "wgsl_stub",
            ArtifactKind::OverlayKit => "overlay_kit",
        }
    }
}

/// One generated scaffold, not yet written to disk.
#[derive(Debug)]
pub struct EmittedArtifact {
    /// Where this artifact would land if flushed.
    pub path: PathBuf,
    /// What kind of scaffold this is.
    pub kind: ArtifactKind,
    /// The generated file content.
    pub content: String,
}

/// Everything one `BuffCompiler` run produced.
#[derive(Debug)]
pub struct BuffResult {
    /// Every scaffold generated this run.
    pub emitted: Vec<EmittedArtifact>,
    /// The classification of every queue item.
    pub gap_report: GapReport,
}

/// Failure modes for a `BuffCompiler` run.
#[derive(Debug)]
pub enum BuffError {
    /// A file couldn't be read or written.
    Io(std::io::Error),
    /// Input JSON didn't parse.
    Json(serde_json::Error),
}

impl std::fmt::Display for BuffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuffError::Io(e)   => write!(f, "buff compiler IO error: {e}"),
            BuffError::Json(e) => write!(f, "buff compiler JSON error: {e}"),
        }
    }
}

impl From<std::io::Error>   for BuffError { fn from(e: std::io::Error)   -> Self { BuffError::Io(e) } }
impl From<serde_json::Error> for BuffError { fn from(e: serde_json::Error) -> Self { BuffError::Json(e) } }

// ── Core compiler ─────────────────────────────────────────────────────────────

/// Reads a widget queue (+ optional gap report) and generates scaffolding.
pub struct BuffCompiler {
    queue_path: PathBuf,
    gap_report_path: Option<PathBuf>,
    out_dir: PathBuf,
}

impl BuffCompiler {
    /// Point the compiler at its disk inputs and output directory.
    pub fn new(
        queue_path: impl AsRef<Path>,
        gap_report_path: Option<impl AsRef<Path>>,
        out_dir: impl AsRef<Path>,
    ) -> Self {
        Self {
            queue_path: queue_path.as_ref().to_path_buf(),
            gap_report_path: gap_report_path.map(|p| p.as_ref().to_path_buf()),
            out_dir: out_dir.as_ref().to_path_buf(),
        }
    }

    /// Run in-memory: no disk reads or writes. The primary entry point for tests
    /// and for callers that already hold the JSON strings.
    pub fn run_in_memory(
        queue_json: &str,
        gap_report_json: Option<&str>,
        out_dir: impl AsRef<Path>,
    ) -> Result<BuffResult, BuffError> {
        let queue: WidgetQueue = serde_json::from_str(queue_json)?;
        let gap_index: HashMap<String, GapDetail> = if let Some(src) = gap_report_json {
            let parsed: GapReportJson = serde_json::from_str(src)?;
            parsed.gaps.into_iter().map(|g| (g.id.clone(), g)).collect()
        } else {
            HashMap::new()
        };

        let mut emitted = Vec::new();
        let mut report = GapReport::new("buff-compiler");

        for item in &queue.items {
            let gap = gap_index.get(&item.id);
            match item.classification.as_str() {
                "native_missing_widget" => {
                    let a = gen_vixi_kit(item, gap, out_dir.as_ref());
                    report.classify(
                        &item.id,
                        Verdict::Overlay,
                        format!("vixi kit skeleton scaffolded → {}", a.path.display()),
                        &item.source,
                    );
                    emitted.push(a);
                }
                "native_missing_token" => {
                    let a = gen_rust_token(item, gap, out_dir.as_ref());
                    report.classify(
                        &item.id,
                        Verdict::Overlay,
                        format!("Rust token stub scaffolded → {}", a.path.display()),
                        &item.source,
                    );
                    emitted.push(a);
                }
                "engine_overlay" => {
                    let a = gen_overlay_kit(item, gap, out_dir.as_ref());
                    report.classify(
                        &item.id,
                        Verdict::Overlay,
                        format!("overlay kit skeleton scaffolded → {}", a.path.display()),
                        &item.source,
                    );
                    emitted.push(a);
                }
                "render_harvest" => {
                    let a = gen_wgsl_stub(item, gap, out_dir.as_ref());
                    report.classify(
                        &item.id,
                        Verdict::Spike,
                        format!("WGSL stub scaffolded → {} (hand-tune required)", a.path.display()),
                        &item.source,
                    );
                    emitted.push(a);
                }
                "spike_only" => {
                    report.classify(
                        &item.id,
                        Verdict::Spike,
                        "spike_only — no scaffold emitted; spike it manually before committing",
                        &item.source,
                    );
                }
                other => {
                    report.classify(
                        &item.id,
                        Verdict::Reserve,
                        format!("classification `{other}` is not a buff target; reserved"),
                        &item.source,
                    );
                }
            }
        }

        Ok(BuffResult { emitted, gap_report: report })
    }

    /// Run, reading inputs from disk and returning results in-memory (no output
    /// files written — caller decides whether to flush with [`flush`]).
    pub fn run(self) -> Result<BuffResult, BuffError> {
        let queue_json = std::fs::read_to_string(&self.queue_path)?;
        let gap_json = self.gap_report_path
            .as_ref()
            .map(|p| std::fs::read_to_string(p))
            .transpose()?;
        Self::run_in_memory(&queue_json, gap_json.as_deref(), &self.out_dir)
    }
}

/// Write every emitted artifact to disk, creating directories as needed.
/// Returns the list of written paths.
pub fn flush(result: &BuffResult) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut written = Vec::new();
    for a in &result.emitted {
        if let Some(parent) = a.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&a.path, &a.content)?;
        written.push(a.path.clone());
    }
    Ok(written)
}

// ── Generators ────────────────────────────────────────────────────────────────

fn gen_vixi_kit(item: &QueueItem, gap: Option<&GapDetail>, out_dir: &Path) -> EmittedArtifact {
    let surface = item.id
        .strip_prefix("surface.")
        .unwrap_or(&item.id)
        .replace(['.', '[', ']', ':', ':'], "_");
    let (next_action, reason) = enrich(gap, &item.recommendation);

    let content = format!(
        "# BUFF: generated vixi kit skeleton\n\
         # source:      {source}\n\
         # reason:      {reason}\n\
         {next_action_line}\
         #vixi:kit v1\n\
         surface: {surface}\n\
         profile: forge_primeflow\n\
         classification: surface_widget\n\
         \n\
         tick_hz: 120\n\
         slot root kind=region layout=stack_v\n\
         slot root.content kind=widget role={surface}\n\
         \n\
         gate integer_boundary = required\n\
         gate no_float_leak = required\n",
        source = item.source,
        reason = reason,
        next_action_line = fmt_next_action("#", next_action),
        surface = surface,
    );

    EmittedArtifact {
        path: out_dir.join(format!("{surface}.kit.vixi")),
        kind: ArtifactKind::VixiKit,
        content,
    }
}

fn gen_rust_token(item: &QueueItem, gap: Option<&GapDetail>, out_dir: &Path) -> EmittedArtifact {
    let struct_name = normalise_token_id(&item.id);
    let file_stem   = camel_to_snake(&struct_name);
    let (next_action, reason) = enrich(gap, &item.recommendation);

    let content = format!(
        "// BUFF: generated Rust token stub\n\
         // source:      {source}\n\
         // reason:      {reason}\n\
         {next_action_line}\
         // HITL: wire into the appropriate crate; verify boundary conversion.\n\
         \n\
         use pp_math::Permyriad;\n\
         \n\
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub struct {struct_name} {{\n\
             pub value: Permyriad,\n\
         }}\n\
         \n\
         impl {struct_name} {{\n\
             pub fn from_permyriad(v: Permyriad) -> Self {{ Self {{ value: v }} }}\n\
         }}\n",
        source = item.source,
        reason = reason,
        next_action_line = fmt_next_action("//", next_action),
        struct_name = struct_name,
    );

    EmittedArtifact {
        path: out_dir.join(format!("{file_stem}.rs")),
        kind: ArtifactKind::RustToken,
        content,
    }
}

fn gen_overlay_kit(item: &QueueItem, gap: Option<&GapDetail>, out_dir: &Path) -> EmittedArtifact {
    let surface = item.id.replace(['.', '[', ']', ':'], "_");
    let (next_action, reason) = enrich(gap, &item.recommendation);

    let content = format!(
        "# BUFF: generated engine overlay kit skeleton\n\
         # source:      {source}\n\
         # reason:      {reason}\n\
         {next_action_line}\
         #vixi:kit v1\n\
         surface: {surface}\n\
         profile: forge_primeflow\n\
         classification: engine_overlay\n\
         \n\
         tick_hz: 120\n\
         slot root kind=region layout=stack_v\n\
         slot root.overlay kind=widget role=engine_overlay_{surface}\n\
         \n\
         gate integer_boundary = required\n\
         gate overlay_reads_sim_buffer = required\n",
        source = item.source,
        reason = reason,
        next_action_line = fmt_next_action("#", next_action),
        surface = surface,
    );

    EmittedArtifact {
        path: out_dir.join(format!("{surface}.overlay.kit.vixi")),
        kind: ArtifactKind::OverlayKit,
        content,
    }
}

fn gen_wgsl_stub(item: &QueueItem, gap: Option<&GapDetail>, out_dir: &Path) -> EmittedArtifact {
    let name = item.id.replace(['.', '[', ']', ':'], "_");
    let (next_action, reason) = enrich(gap, &item.recommendation);

    let content = format!(
        "// BUFF: generated WGSL render stub\n\
         // source:      {source}\n\
         // reason:      {reason}\n\
         {next_action_line}\
         // HITL: replace stubs with real vertex/fragment logic.\n\
         \n\
         struct VertexOut {{\n\
             @builtin(position) pos: vec4<f32>,\n\
         }};\n\
         \n\
         @vertex\n\
         fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOut {{\n\
             // TODO: {name} vertex stage\n\
             var out: VertexOut;\n\
             out.pos = vec4<f32>(0.0, 0.0, 0.0, 1.0);\n\
             return out;\n\
         }}\n\
         \n\
         @fragment\n\
         fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {{\n\
             // TODO: {name} fragment stage — use CID_* via rgba() not hardcoded hex\n\
             return vec4<f32>(0.0, 0.0, 0.0, 1.0);\n\
         }}\n",
        source = item.source,
        reason = reason,
        next_action_line = fmt_next_action("//", next_action),
        name = name,
    );

    EmittedArtifact {
        path: out_dir.join(format!("{name}.wgsl.stub")),
        kind: ArtifactKind::WgslStub,
        content,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn enrich<'a>(gap: Option<&'a GapDetail>, fallback: &'a str) -> (&'a str, &'a str) {
    match gap {
        Some(g) => (g.next_action.as_str(), g.reason.as_str()),
        None    => ("", fallback),
    }
}

fn fmt_next_action(comment_prefix: &str, next_action: &str) -> String {
    if next_action.is_empty() {
        String::new()
    } else {
        format!("{comment_prefix} next_action: {next_action}\n")
    }
}

/// Normalise an assimilator token id to a PascalCase Rust struct name.
///
/// - `type.ramp[acoustic_risk]`         → `RampAcousticRisk`
/// - `creature_engine::acoustic_inspection` → `AcousticInspection`
/// - `type.some_token`                  → `SomeToken`
pub fn normalise_token_id(id: &str) -> String {
    let stripped = id.strip_prefix("type.").unwrap_or(id);
    let stripped = stripped.rsplit("::").next().unwrap_or(stripped);
    let cleaned: String = stripped
        .chars()
        .map(|c| if matches!(c, '[' | ']') { ' ' } else { c })
        .collect();
    cleaned
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(capitalise)
        .collect()
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None    => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// `RampAcousticRisk` → `ramp_acoustic_risk`
fn camel_to_snake(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SLAPP_QUEUE: &str = include_str!("../corpora/forge_app_slapp/reports/native_widget_queue.json");
    const SLAPP_GAP: &str = include_str!("../corpora/forge_app_slapp/reports/ffi_ui_gap_report.json");
    const ASTRO_QUEUE: &str = include_str!("../corpora/astro/reports/ffi_ui_gap_report.json");

    #[test]
    fn test_vixi_kit_gen_from_forge_app_slapp() {
        let result = BuffCompiler::run_in_memory(SLAPP_QUEUE, Some(SLAPP_GAP), "/tmp/buff_test")
            .expect("buff compiler should not fail on slapp queue");

        let kit = result.emitted.iter().find(|a| a.kind == ArtifactKind::VixiKit)
            .expect("should emit at least one VixiKit");
        assert!(kit.content.contains("#vixi:kit v1"), "must start with vixi header");
        assert!(kit.content.contains("tick_hz: 120"), "must declare 120Hz clock");
        assert!(kit.content.contains("surface: pipe_network_map"), "surface name from gap id");
        assert!(kit.content.contains("# BUFF:"), "must carry provenance header");
        assert!(!kit.content.contains("0xFF"), "Vision Gate: no hardcoded hex");
    }

    #[test]
    fn test_rust_token_gen_name_normalisation() {
        assert_eq!(normalise_token_id("type.ramp[acoustic_risk]"), "RampAcousticRisk");
        assert_eq!(normalise_token_id("creature_engine::acoustic_inspection"), "AcousticInspection");
        assert_eq!(normalise_token_id("type.some_token"), "SomeToken");

        let result = BuffCompiler::run_in_memory(SLAPP_QUEUE, Some(SLAPP_GAP), "/tmp/buff_test")
            .expect("buff compiler should not fail");
        let tokens: Vec<_> = result.emitted.iter()
            .filter(|a| a.kind == ArtifactKind::RustToken)
            .collect();
        assert_eq!(tokens.len(), 2, "forge_app_slapp has 2 native_missing_token gaps");
        assert!(tokens.iter().any(|a| a.content.contains("struct RampAcousticRisk")));
        assert!(tokens.iter().any(|a| a.content.contains("struct AcousticInspection")));
        assert!(tokens.iter().all(|a| a.content.contains("use pp_math::Permyriad")));
        assert!(tokens.iter().all(|a| a.content.contains("pub value: Permyriad")));
    }

    #[test]
    fn test_gap_report_emitted() {
        let result = BuffCompiler::run_in_memory(SLAPP_QUEUE, Some(SLAPP_GAP), "/tmp/buff_test")
            .expect("buff compiler should not fail");
        // 3 items in slapp queue, all scaffolded as Overlay
        assert_eq!(result.gap_report.count(Verdict::Overlay), 3);
        assert!(result.gap_report.is_clean(), "all slapp gaps scaffolded → report clean");
    }

    #[test]
    fn test_generic_corpus_astro() {
        // The Astro gap report has no native_missing_widget or native_missing_token —
        // the buff compiler sees an empty queue format (astro only has a gap_report,
        // not a widget_queue — we feed a synthetic empty queue here).
        let empty_queue = r#"{"queue":"native_widget_queue","version":"0.1.0","items":[]}"#;
        let result = BuffCompiler::run_in_memory(empty_queue, Some(ASTRO_QUEUE), "/tmp/buff_test")
            .expect("empty queue should produce empty result");
        assert!(result.emitted.is_empty(), "no missing widgets/tokens in Astro → nothing emitted");
        assert!(result.gap_report.is_clean(), "no gaps → report clean");
    }

    #[test]
    fn test_all_corpora_sweep() {
        // Highest-value multi-corpus sweep: run every committed widget queue through
        // the buff compiler and assert structural invariants across all outputs.
        struct CorpusCase { name: &'static str, queue: &'static str, expected_emitted: usize }

        let cases = [
            CorpusCase {
                name: "analog_console", expected_emitted: 8,
                queue: include_str!("../corpora/analog_console/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "aseprite", expected_emitted: 0,
                queue: include_str!("../corpora/aseprite/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "forgecanvas_web", expected_emitted: 4,
                queue: include_str!("../corpora/forgecanvas_web/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "love2d", expected_emitted: 0,
                queue: include_str!("../corpora/love2d/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "naga", expected_emitted: 0,
                queue: include_str!("../corpora/naga/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "raylib", expected_emitted: 0,
                queue: include_str!("../corpora/raylib/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "react", expected_emitted: 0,
                queue: include_str!("../corpora/react/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "tiled", expected_emitted: 0,
                queue: include_str!("../corpora/tiled/reports/native_widget_queue.json"),
            },
            CorpusCase {
                name: "visual_editor_spec", expected_emitted: 20,
                queue: include_str!("../corpora/visual_editor_spec/reports/native_widget_queue.json"),
            },
        ];

        let mut total = 0;
        for case in &cases {
            let result = BuffCompiler::run_in_memory(case.queue, None, "/tmp/buff_sweep")
                .unwrap_or_else(|e| panic!("[{}] buff compiler error: {}", case.name, e));

            assert_eq!(
                result.emitted.len(), case.expected_emitted,
                "[{}] expected {} emitted artifacts", case.name, case.expected_emitted
            );

            // Vision Gate: no hardcoded colour hex (0xRRGGBB / 0xRRGGBBAA) in any generated file
            for a in &result.emitted {
                assert!(
                    !a.content.contains("0xFF") && !a.content.contains("0xff"),
                    "[{}] Vision Gate violation: C-style hex colour in {}", case.name, a.path.display()
                );
            }

            // Every vixi kit must have tick_hz: 120 and the BUFF header
            for a in result.emitted.iter().filter(|a| a.kind == ArtifactKind::VixiKit) {
                assert!(a.content.contains("tick_hz: 120"),
                    "[{}] {:?}: missing tick_hz: 120", case.name, a.path);
                assert!(a.content.contains("# BUFF:"),
                    "[{}] {:?}: missing provenance header", case.name, a.path);
            }

            // Every rust token must have Permyriad
            for a in result.emitted.iter().filter(|a| a.kind == ArtifactKind::RustToken) {
                assert!(a.content.contains("Permyriad"),
                    "[{}] {:?}: missing Permyriad boundary type", case.name, a.path);
            }

            total += result.emitted.len();
        }
        // Total across 9 corpora: 8 + 4 + 20 = 32
        assert_eq!(total, 32, "expected 32 artifacts across all non-forge_app_slapp corpora");
    }
}
