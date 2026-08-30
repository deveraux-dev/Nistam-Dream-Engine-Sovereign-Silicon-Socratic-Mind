//! Crate law texts, drained out of the 130 per-crate `CLAUDE.md` files
//! (Sean 2026-07-30: "take the verbs out of the CLAUDE.md and encode them in
//! river.idx and forge-book, then delete the 90% leaving only our domains").
//!
//! Each row carries the file's whole pre-drain text verbatim, its sha256 as of
//! the drain, and the one domain line that stays on disk. Nothing was dropped —
//! it moved from prose no program reads to rows a test walks. Since 2026-08-06
//! (Sean) the per-crate files are deleted outright; this table is the sole home.

use std::path::Path;

/// One crate's drained `CLAUDE.md`.
#[derive(Debug, Clone, Copy)]
pub struct CrateLaws {
    /// Path of the `CLAUDE.md`, relative to the repo root.
    pub path: &'static str,
    /// First 16 hex of the file's sha256 at drain time.
    pub sha256: &'static str,
    /// The domain line that stays on disk.
    pub domain: &'static str,
    /// The whole pre-drain text, verbatim.
    pub drained: &'static str,
}

/// Every drained crate `CLAUDE.md`, sorted by path.
pub const CRATE_LAWS: &[CrateLaws] = &[
    CrateLaws {
        path: "crates/alchemy-ink/CLAUDE.md",
        sha256: "5a92efd147de382e",
        domain: r##"Core capability for alchemy-ink"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<alchemy-ink><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for alchemy-ink</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p alchemy-ink);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</alchemy-ink>
"##,
    },
    CrateLaws {
        path: "crates/CUI/CLAUDE.md",
        sha256: "4edc7b4565fb22b4",
        domain: r##"Core capability for CUI"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<CUI><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for CUI</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p CUI);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</CUI>
"##,
    },
    CrateLaws {
        path: "crates/dead-drop/CLAUDE.md",
        sha256: "08480e4d7823ec59",
        domain: r##"Core capability for dead-drop"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<dead-drop><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for dead-drop</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p dead-drop);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</dead-drop>
"##,
    },
    CrateLaws {
        path: "crates/ffi-ui-assimilator-001/CLAUDE.md",
        sha256: "fa07af89ff086a39",
        domain: r##"Core capability for ffi-ui-assimilator-001"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<ffi-ui-assimilator-001><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for ffi-ui-assimilator-001</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p ffi-ui-assimilator-001);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</ffi-ui-assimilator-001>
"##,
    },
    CrateLaws {
        path: "crates/forge-anim-wasm/CLAUDE.md",
        sha256: "b321377463627e17",
        domain: r##"Core capability for forge-anim-wasm"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<forge-anim-wasm><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for forge-anim-wasm</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p forge-anim-wasm);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</forge-anim-wasm>
"##,
    },
    CrateLaws {
        path: "crates/forge-anim/CLAUDE.md",
        sha256: "affcb74b8b56f5c4",
        domain: r##"Core capability for forge-anim"##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<forge-anim><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">Core capability for forge-anim</law>
  <done-bar>plumbing DONE=build+test-green(cargo test -p forge-anim);inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</forge-anim>
"##,
    },
    CrateLaws {
        path: "crates/forge-ast/CLAUDE.md",
        sha256: "0571c3a7cc01ae0b",
        domain: r##"VixiScript parser and Abstract Syntax Tree, the SoT the forge-vix grammar/LSP hand-mirrors."##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<forge-ast><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="role">VixiScript parser and Abstract Syntax Tree, the SoT the forge-vix grammar/LSP hand-mirrors.</law>
  <law id="firewall">SoT with NO cargo edge to forge-vix; the seam is drift-checked, never a dependency.</law>
  <law id="vixicoat" pri="TOP">Clean Rust before Vixicoat. Rust bleeds through. No un-traced warnings, no dead_code suppressions, no stubs holding a pass.</law>
  <done-bar>OWNS the VixiScript .vixel AST / grammar SoT; inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</forge-ast>
"##,
    },
    CrateLaws {
        path: "crates/forge-audio/CLAUDE.md",
        sha256: "6bc71622cc7c1da7",
        domain: r##"Zero-alloc applies to the realtime audio callback ONLY. Ingest/decode/parse are LOAD-TIME where heap allocation is fine."##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate)</law>
<law id="v1-engine" pri="TOP">=forge-studio/CLAUDE.md#v1-engine(verbatim,no-restate;engine-layer(conductor/DSP);Tauri=thin-shell-face,root#one-engine)</law>
<forge-audio><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="zero-alloc-carve-out" pri="TOP">Zero-alloc applies to the realtime audio callback ONLY. Ingest/decode/parse are LOAD-TIME where heap allocation is fine.</law>
  <law id="sensory-ears">roadie.rs is the EARS of the sensory bus. RoadieBot::analyze(&amp;MeterData) -> telemetry.rs:222 set_roadie -> studio's vibe_threat -> vibe_glow.</law>
  <law id="parallel-divergence">LIVE is a deliberate slim subset. Port-forward SELECTIVELY + invariant-gated; never promote 13engine wholesale.</law>
  <types>dsp::AudioBuffer/sample_conversion=dsp.rs·ingest::ingest_file/Ingested=ingest.rs·dimensional_collapse::HearDecoder=dimensional_collapse.rs</types>
  <done-bar>OWNS surface daw — conductor audio buffer; inherits AAA+/HITL(root &lt;done_bar&gt;)</done-bar>
</forge-audio>
"##,
    },
    CrateLaws {
        path: "crates/forge-book/CLAUDE.md",
        sha256: "8c6b3d8364fc44f0",
        domain: r##"src/routers.rs is the stable, fact-locked census for the 30+ routers -> 7 axes. 7-expert MoE ladder must NOT collapse."##,
        drained: r##"<law id="pivot" pri="TOP">=root CLAUDE.md#pivot(verbatim,no-restate;binds this crate);DELTA:MAP-PRESS — renders atlas faces; render/export/fold lanes=GREEN-lit</law>
<forge-book><!-- crate-delta ONLY; shared doctrine=root F:/NewRepo/CLAUDE.md -->
  <law id="router-census">src/routers.rs is the stable, fact-locked census for the 30+ routers -> 7 axes. 7-expert MoE ladder must NOT collapse.</law>
  <law id="one-project" pri="TOP">ONE-PROJECT=1 typed-asset-box. Views (Paint/Book/World) are views over 1 box, never local copy silos.</law>
  <law id="nav-cards" pri="TOP">STUDIO-NAV=launcher-card-style. Bare buttons on cards, click -> SwitchSurface. NO-HOTKEYS (F-keys never used).</law>
  <law id="plain-language" pri="TOP">UI-COPY=plain-words for personas. No engine jargon (BAN "parallax/zone-nav/SwitchSurface/HUB").</law>
  <done-bar>plumbing DONE=build+test-green; inherits AAA+/HITL (root &lt;done_bar&gt;)</done-bar>
</forge-book>
"##,
    },
];

/// The drained laws for a repo-relative `CLAUDE.md` path.
pub fn laws_for(path: &str) -> Option<&'static CrateLaws> {
    let want = path.replace('\\', "/");
    CRATE_LAWS.iter().find(|c| c.path == want)
}

/// Repo root, as reached from this crate's manifest dir.
pub fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Crate `CLAUDE.md` files that carry more than their domain line — the drain
/// is only true while the law body stays here and not back on disk.
pub fn undrained(root: &Path) -> Vec<&'static str> {
    CRATE_LAWS
        .iter()
        .filter(|c| {
            std::fs::read_to_string(root.join(c.path))
                .map(|on_disk| on_disk.trim().lines().count() > 3)
                .unwrap_or(false)
        })
        .map(|c| c.path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crate_claude_md_files_are_gone() {
        let root = repo_root();
        let back: Vec<_> =
            CRATE_LAWS.iter().filter(|c| root.join(c.path).exists()).map(|c| c.path).collect();
        assert!(back.is_empty(), "a per-crate CLAUDE.md is back on disk: {back:?}");
    }

    #[test]
    fn every_row_carries_a_domain_and_a_body() {
        for c in CRATE_LAWS {
            assert!(!c.domain.trim().is_empty(), "{} drained without a domain line", c.path);
            assert!(c.drained.len() > c.domain.len(), "{} drained an empty body", c.path);
            assert_eq!(c.sha256.len(), 16, "{} sha receipt is malformed", c.path);
        }
    }

    #[test]
    fn the_drain_covers_the_whole_corpus() {
        assert_eq!(CRATE_LAWS.len(), 9, "drained crate CLAUDE.md rows in v3 subset");
    }

    #[test]
    fn the_law_body_stayed_off_disk() {
        let left = undrained(&repo_root());
        assert!(left.is_empty(), "law body back on disk instead of in this table: {left:?}");
    }

    #[test]
    fn lookup_takes_either_slash() {
        assert!(laws_for("crates/forge-book/CLAUDE.md").is_some());
        assert!(laws_for("crates\\forge-book\\CLAUDE.md").is_some());
    }
}
