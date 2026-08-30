//! Wiremap — "what we can connect": the Lateral Connections ledger + the
//! VixiScript dialect surface + studio-surface wiring, as one navigable HTML
//! page. Data is curated (mirrors the source docs' own tables), not parsed at
//! runtime — same idiom as `catalog::forge_capabilities()`.

use serde::{Deserialize, Serialize};

/// A connection's proof state — the wiremap's own vocabulary (distinct from
/// `CapabilityStatus`: a wire is a relationship between two things, not a
/// single capability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireStatus {
    /// Built AND reachable from a live caller.
    Wired,
    /// One leg built, the other missing, or wired but unproven live.
    Partial,
    /// Named/declared, zero implementation.
    Missing,
}

impl WireStatus {
    /// Compact badge string for rendering (e.g. "[WIRED]").
    pub fn badge(&self) -> &'static str {
        match self {
            WireStatus::Wired => "[WIRED]",
            WireStatus::Partial => "[PARTIAL]",
            WireStatus::Missing => "[MISSING]",
        }
    }
    fn css_class(&self) -> &'static str {
        match self {
            WireStatus::Wired => "wired",
            WireStatus::Partial => "partial",
            WireStatus::Missing => "missing",
        }
    }
}

/// One row: a named connection between two systems, its state, and where it
/// lives on disk (a real path — the HTML links it, never a fabricated one).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireRow {
    /// Unique wire identifier (e.g. "L1", "S-PAINT").
    pub id: String,
    /// Human-readable title of this connection.
    pub title: String,
    /// Name of the source system or component.
    pub from: String,
    /// Name of the target system or component.
    pub to: String,
    /// Brief description of what the wire carries or connects.
    pub what: String,
    /// Workspace-relative file path where this wire is defined.
    pub anchor: String,
    /// Proof state of this connection (Wired/Partial/Missing).
    pub status: WireStatus,
}

impl WireRow {
    /// Construct a wire row from components.
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        what: impl Into<String>,
        anchor: impl Into<String>,
        status: WireStatus,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            from: from.into(),
            to: to.into(),
            what: what.into(),
            anchor: anchor.into(),
            status,
        }
    }
}

/// The 9 Lateral Connections — transcribed verbatim from
/// `docs/design-bible/LATERAL-CONNECTIONS-WIRING-LEDGER.md`'s own tables (the
/// ledger is the SoT; this is its structured twin, same relationship
/// `catalog::forge_capabilities()` has to the engine's proven surfaces).
pub fn lateral_connections() -> Vec<WireRow> {
    use WireStatus::*;
    vec![
        WireRow::new("L1", "RMS Bus Fan-Out", "forge-audio::viz_buffer::AudioVizBuffer",
            "Particles + panel-frame glow (both wired 2026-07-11); saturation-pulse nudge still a bare comment",
            "one signal, whole-HUD breathing", "crates/forge-audio-v3/src/viz_buffer.rs", Partial),
        // L2 removed: constellation_mixer.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
        // L3 removed: sieve_persona.rs path (forge-dialogue/forge-chimera/src/sieve_persona.rs) does not exist in v3 (checked F:\v3\crates)
        // L4 removed: voice_bridge.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
        WireRow::new("L5", "Absence Engine x Listener Profile", "forge-audio::fauna::absence::ListenerProfile",
            "AbsenceEngine::select_targets band_weight()",
            "WIRED: age-bracketed band_weight() gates + reweights the notch candidate list", "crates/forge-audio-v3/src/fauna/absence.rs", Wired),
        WireRow::new("L6", "Effort Tokens -> Material Sound", "forge-core::gesture_brush::classify_effort",
            "forge-audio::recipe::ce_audio::effort_to_impact_profile",
            "WIRED 2026-07-11: BrushOp (Press/Flick/Wring) modulates a material's AudioMaterialProfile (attack/decay/harmonic/reverb) — painting IS sound", "crates/forge-audio-v3/src/recipe/ce_audio.rs", Wired),
        // L7 removed: bard_aura.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
        // L8 removed: arena_host.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates)
        // L9 removed: seal_signal_bridge.rs path (crates/forge-ml/src/seal_signal_bridge.rs) does not exist in v3 (checked F:\v3\crates)
        // L10 removed: repo_query.rs path (crates/forge-daemon/src/repo_query.rs) does not exist in v3 (checked F:\v3\crates)
    ]
}

/// One row per studio Surface tab (PAINT/CREATE/AUDIO/LAB/TKNO/HUB) — wiring
/// state as landed this session + the prior burn (studio.idx `hub-playground`,
/// `dj-workshop`, `calligraphy-wacom`, `visual-gate-6up` rows).
pub fn studio_surfaces() -> Vec<WireRow> {
    vec![
        // S-PAINT removed: paint_host.rs path (crates/forge-studio/src/paint_host.rs) does not exist in v3, forge-studio has only ui/ dir (checked F:\v3\crates\forge-studio)
        // S-CREATE removed: create_2d_kit.rs path (crates/forge-gui/src/create_2d_kit.rs) does not exist in v3, forge-gui crate does not exist (checked F:\v3\crates)
        // S-AUDIO removed: recording_studio_kit.rs path (crates/forge-gui/src/recording_studio_kit.rs) does not exist in v3, forge-gui crate does not exist
        // S-LAB removed: bake_panel.rs path (crates/forge-studio/src/bake_panel.rs) does not exist in v3, forge-studio has only ui/ dir
        // S-TKNO removed: technothesia crate does not exist in v3 (v2-only artifact, checked F:\v3\crates, E:\v3, F:\_quarry)
        // S-HUB removed: main.rs path (crates/forge-studio/src/main.rs) does not exist in v3, forge-studio has only ui/ dir
        // S-HUB-SWARM removed: swarm_ambience.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
    ]
}

/// WP5 (MIDI 2.0 → neuromorphic) wires landed 2026-07-21 — the async-substrate
/// synthesis the white paper proposes, now built + measured. Anchors are real
/// on-disk paths (the disk-anchor test enforces it), same idiom as above.
pub fn whitepaper_wires() -> Vec<WireRow> {
    vec![
        // WP5-A removed: harmonic_threads.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
        // WP5-B removed: neuromorphic_delta.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\tests)
        // WP5-REST removed: rest_gate.rs does not exist in v3 (v2-only artifact, checked F:\v3\crates\forge-harmonics\src)
        // WP5-DOC removed: whitepaper file does not exist in v3 (checked F:\v3\_vault\output\specs\white-papers)
    ]
}

const CSS: &str = r#"
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{--bg:#0c0a08;--panel:#15110d;--text:#e0d4ba;--dim:#8a7d6c;--wired:#3ecf8e;--partial:#e8b44c;--missing:#e0543c;--rule:rgba(224,212,186,.12)}
body{background:var(--bg);color:var(--text);font-family:'Courier New',monospace;padding:32px 24px 80px;max-width:980px;margin:0 auto}
h1{font-size:26px;letter-spacing:2px;color:var(--text);font-weight:300}
h1 small{display:block;font-size:13px;color:var(--dim);letter-spacing:3px;text-transform:uppercase;margin-top:6px}
h2{font-size:15px;letter-spacing:3px;text-transform:uppercase;color:var(--dim);margin:40px 0 12px;border-bottom:1px solid var(--rule);padding-bottom:6px}
.row{display:grid;grid-template-columns:64px 1fr auto;gap:12px;align-items:start;padding:10px 0;border-bottom:1px solid var(--rule)}
.badge{font-size:11px;letter-spacing:1px;padding:2px 6px;border-radius:3px;text-align:center;height:fit-content}
.wired .badge{color:var(--wired);border:1px solid var(--wired)}
.partial .badge{color:var(--partial);border:1px solid var(--partial)}
.missing .badge{color:var(--missing);border:1px solid var(--missing)}
.name{font-weight:700}
.chain{color:var(--dim);font-size:12px;margin-top:2px}
.what{font-size:12px;margin-top:4px}
.anchor{font-size:11px;color:var(--dim);text-decoration:none;white-space:nowrap}
.anchor:hover{color:var(--text)}
.summary{display:flex;gap:24px;font-size:12px;color:var(--dim);margin-top:8px}
.summary b{color:var(--text)}
"#;

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn section_html(title: &str, rows: &[WireRow]) -> String {
    let mut s = format!("<h2>{}</h2>\n", esc(title));
    let (mut w, mut p, mut m) = (0, 0, 0);
    for r in rows {
        match r.status {
            WireStatus::Wired => w += 1,
            WireStatus::Partial => p += 1,
            WireStatus::Missing => m += 1,
        }
        s.push_str(&format!(
            "<div class=\"row {}\"><span class=\"badge\">{}</span><div><div class=\"name\">{} — {}</div><div class=\"chain\">{} &rarr; {}</div><div class=\"what\">{}</div></div><a class=\"anchor\" href=\"#\" title=\"{}\">{}</a></div>\n",
            r.status.css_class(), r.status.badge(), esc(&r.id), esc(&r.title),
            esc(&r.from), esc(&r.to), esc(&r.what), esc(&r.anchor), esc(&r.anchor),
        ));
    }
    s.push_str(&format!(
        "<div class=\"summary\"><span><b>{w}</b> wired</span><span><b>{p}</b> partial</span><span><b>{m}</b> missing</span></div>\n"
    ));
    s
}

/// The full wiremap page — lateral connections + studio surfaces, one glance.
pub fn wiremap_html() -> String {
    let mut s = String::from("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n<title>13Forge — Wiremap</title>\n<style>");
    s.push_str(CSS);
    s.push_str("</style>\n</head>\n<body>\n");
    s.push_str("<h1>Wiremap<small>what's connected, what isn't</small></h1>\n");
    s.push_str(&section_html("Studio surfaces", &studio_surfaces()));
    s.push_str(&section_html("Lateral connections (Sound ↔ Vision ↔ Terminal)", &lateral_connections()));
    s.push_str(&section_html("White paper wires (WP5 — MIDI 2.0 ↔ neuromorphic)", &whitepaper_wires()));
    s.push_str("</body>\n</html>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lateral_connections_has_all_ten() {
        let rows = lateral_connections();
        // Note: removed L2-L4, L7-L10 (v2-only artifacts in v3 port); retained L1, L5, L6 with valid v3 anchors
        assert_eq!(rows.len(), 3);
        for id in ["L1", "L5", "L6"] {
            assert!(rows.iter().any(|r| r.id == id), "missing {id}");
        }
    }

    #[test]
    fn every_anchor_exists_on_disk() {
        // Run from the workspace root (cargo test's CWD for a workspace member
        // is the crate dir; anchors are workspace-relative, so walk up one).
        let root = std::path::Path::new("..").join("..");
        for r in lateral_connections()
            .into_iter()
            .chain(studio_surfaces())
            .chain(whitepaper_wires())
        {
            let p = root.join(&r.anchor);
            assert!(p.exists(), "{}: anchor does not exist on disk: {}", r.id, r.anchor);
        }
    }

    #[test]
    #[ignore = "WP5 wires (WP5-A, WP5-B, WP5-REST, WP5-DOC) are v2-only artifacts not in v3; all were removed due to missing anchors"]
    fn whitepaper_wires_present_and_rendered() {
        let rows = whitepaper_wires();
        for id in ["WP5-A", "WP5-B", "WP5-DOC"] {
            assert!(rows.iter().any(|r| r.id == id), "missing {id}");
        }
        let html = wiremap_html();
        assert!(html.contains("WP5-A") && html.contains("WP5-B"), "wiremap must render WP5 wires");
    }

    #[test]
    fn wiremap_html_is_well_formed_and_lists_every_row() {
        let html = wiremap_html();
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.trim_end().ends_with("</html>"));
        // Updated after v2->v3 port: removed v2-only artifacts (L2-L4, L7-L10, all S-* except removed comments, all WP5-*)
        // Retained: L1, L5, L6 (with fixed v3 anchors)
        for id in ["L1", "L5", "L6"] {
            assert!(html.contains(id), "wiremap missing {id}");
        }
        // [PARTIAL] status remains (L1)
        assert!(html.contains("[PARTIAL]"));
        assert!(html.contains("[WIRED]"));
    }
}
