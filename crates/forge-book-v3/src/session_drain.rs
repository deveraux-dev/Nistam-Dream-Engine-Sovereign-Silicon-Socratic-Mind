//! Session drain 2026-07-23 — Gemini-welder capability merge. 28 crates green
//! (Opus merge-gate, each lane `cargo test -p <crate>`; +poll5d/forge-vision 07-24).
//! Gate `cargo check -p forge-studio` EXIT 0; each row carries its test-count receipt.
//!
//! # FROZEN — READ-ONLY, NO NEW ROWS (Sean 2026-08-04)
//!
//! "session_drain.rs is a fucking cop out, make it read only. We have a ROI triage, we
//! have TECH_DEBT.json, and a board, and seams.rs."
//!
//! It had grown to a `session_*` fn per session — narrative accumulating in a file whose
//! only consumer is [`receipted_trash_blocks`]. Writing a paragraph here FEELS like
//! filing a receipt and proves nothing: no gate reads it, no test resolves its claims,
//! and it drifts the moment the code moves. The existing rows stay (they are the
//! historical record, and `agent-code` condenses, never removes). Nothing new lands.
//!
//! Receipts route by KIND to ledgers that already gate themselves:
//!
//! | Receipt | Home | Why it beats a paragraph |
//! |---|---|---|
//! | debt opened/cleared | `.forge/recovery/TECH-DEBT.json` (`qa debt`) | exit 1 on unproven clear |
//! | board row / truth flip | `.forge/board_tasks.json` (`board --harvest`) | a row is green or it is not |
//! | cross-domain seam | [`crate::seams::SEAMS`] | anchors resolved against disk every run |
//! | priority / triage | [`crate::oracle1_governor::board_row_priority`] | an integer, not an adjective |
//!
//! Enforced by `the_drain_takes_no_new_rows`, not remembered.

use crate::atlas::{AtlasSection, CapabilityEntry, CapabilityStatus};
use crate::chapter::Chapter;

/// Every crate merged green this session, receipt = crate + test count (self-verified
/// by the orchestrator's own `cargo test -p <crate>`, never the welder's self-claim).
pub fn session_2026_07_23_drain() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven("door fold — MCP door dissolved INTO 13forge-studio: stdio NDJSON + std TcpListener HTTP MCP over one cached forge_daemon::Brain, off both render clocks; airgap WasiSandbox real", S::Capabilities, "forge-studio/src/door/mod.rs · cargo check -p forge-studio EXIT 0"),
        CapabilityEntry::proven("ghostmoon-5d spatial index — 5D Point/Bounds (int-quant 16b/dim), Morton uint64 12b×5, K-d single-axis split, insert/range/knn, temporal prune", S::Capabilities, "forge-game-systems/src/spatial5d.rs · -p forge-game-systems 887 green"),
        CapabilityEntry::proven("poll5d live poller (2026-07-24) — self-contained low-cost 5D-indexed window poll: PrintWindow capture -> tile-delta 5D contacts -> bounded Morton present-ring + k-NN, exact bounded distinct-colour + EWMA trend, AIMD self-pacing (still scene ≈ free); ternary/octal substrate: 0..7 pack (21/u64), 3-bit counting-Bloom, quantized Count-Min, balanced trits (5/u8), Tri predicate, luminance-tier pHash dedup", S::Capabilities, "forge-vision/src/poll5d/* · -p forge-vision poll5d 35 green"),
        CapabilityEntry::proven("UMP / harmonic theory — MIDI2.0 13-type encode/decode round-trip + scale/chord/interval tables, integer-deterministic", S::Learning, "forge-harmonics · 412 green"),
        CapabilityEntry::proven("UCAS calligraphy codebook — syllabic classification + stroke primitives + canonical serialization round-trips", S::Capabilities, "forge-calligraphy · 71 green"),
        CapabilityEntry::proven("codex extensions + vix_kit roundtrip fix — remove/find chapter/lore/page; escape/unescape embedded newlines (surface: bug); arch_tablets live-read RETIRED -> include_str! encoded (Sean: _plans archive-only)", S::Capabilities, "forge-book/src/{book_extensions,chapter_extensions,vixi_kit,arch_tablets}.rs · 519 lib + roundtrip green"),
        CapabilityEntry::proven("steganography — encode/decode payload-in-carrier, capacity + tamper detection", S::Capabilities, "forge-stego · 41 green"),
        CapabilityEntry::proven("graph algorithms — traversal/topo/shortest-path/cycle/components, deterministic", S::Learning, "forge-graph · 13 green"),
        CapabilityEntry::proven("revenue math — integer-money pricing tiers / tax / split / license terms (no float money)", S::Capabilities, "forge-revenue · 36 green"),
        CapabilityEntry::proven("item forge — ForgeConfig::builtin() code-embedded anvil (rarity tiers), assemble_sword config-driven rarity roll; ForgeParams split", S::Capabilities, "forge-items/src/{forge,config}.rs · 65 green (5 callers swept: sf-wasm/studio×2/game-systems/export)"),
        CapabilityEntry::proven("animation — easing/interp curves, keyframe sampling, blend/layer, deterministic timeline", S::Capabilities, "forge-anim · 147 green"),
        CapabilityEntry::proven("cart brain — combat resolution + deterministic loot rolls (per-kill advancing id) + input chord + hermetic laws", S::Learning, "forge-cart-brain · 256 green"),
        CapabilityEntry::proven("materials — L1 64-material table ops, blend/mix, tier/rarity mapping, deterministic decode (bake feature green)", S::Capabilities, "forge-materials · 28 green + --features bake"),
        CapabilityEntry::proven("flash capability build-out", S::Capabilities, "forge-flash · 14 green"),
        CapabilityEntry::proven("MCP tool router — profile-aware upstream routing, collision resolve", S::Runbook, "forge-router · 5 green"),
        CapabilityEntry::proven("orchestrator — profile-aware MCP tool dispatch / classification wall", S::Runbook, "forge-orchestrator · 9 green"),
        CapabilityEntry::proven("context router — session-tier keyword classify / provider config / cost estimate", S::Runbook, "forge-context-router · 32 green"),
        CapabilityEntry::proven("viewport host — layout/viewport math", S::Capabilities, "forge-viewport-host · 33 green"),
        CapabilityEntry::proven("marketplace — listing mint (BLAKE3) / bridge-to-asset / price validation", S::Capabilities, "forge-marketplace · 38 green"),
        CapabilityEntry::proven("tile-crawler — action-platformer tile/collision/mechanic-rail, integer-deterministic (cad feature green)", S::Capabilities, "forge-tile-crawler · 16 green + --features cad"),
        CapabilityEntry::proven("semantic classification / tagging, deterministic", S::Learning, "forge-semantic · 61 green"),
        CapabilityEntry::proven("broski dream pipeline / orchestration driver capability", S::Runbook, "forge-broski · 5 green + ghost-speak"),
        CapabilityEntry::proven("probe capability", S::Capabilities, "forge-probe · 1 green"),
        CapabilityEntry::proven("meaning budget — token/meaning accounting + estimation (cli feature green)", S::Learning, "forge-meaning-budget · 11 green + --features cli"),
        CapabilityEntry::proven("shader build — WGSL/naga validation + transform (SPIR-V/DXIL path)", S::Shaders, "forge-shader-build · 13 green"),
        CapabilityEntry::proven("consequence WCE — sim state -> 16B queries -> curve/MoE ruling -> PhysicsEffects, deterministic", S::Learning, "forge-consequence · 67 green"),
        CapabilityEntry::proven("zones — biome/heightmap/zone-graph + quest generator + integrity validation (worldgen feature green)", S::Capabilities, "forge-zones · 174 green + --features worldgen"),
        CapabilityEntry::proven("semantic firewall — scan/verdict/block, weighted threat rules; jailbreak weight 100 (a lone prompt-injection blocks on its own)", S::Capabilities, "forge-firewall/src/semantic.rs · 118 green"),
    ]
}

/// Cut pass (2026-07-28): removes dead code with proof of absence — receipted removals of unused dispatch integration and VCS diagnostics.
pub fn session_2026_07_28_cut() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "spool::dispatch_integration_plan CUT — repo-wide grep returned exactly ONE line, its own declaration; zero callers, dead_code warned on every build of the tree",
            S::Runbook,
            "forge-daemon/src/spool.rs (32 lines + the use) · restore = call forge_one_shot per Action::Modify, which is all it did",
        ),
        CapabilityEntry::proven(
            "mod integration_plan DRAINED — its only consumer was that fn, so the whole chain was dead; serde types IntegrationPlan/Action/Review/ReviewStatus, 30 lines, no tests",
            S::Runbook,
            "forge-daemon/src/lib.rs:11 dropped · husk _vault/_trash/2026-07-28-prim-fold/integration_plan.rs · restore = move back + re-add the mod line above `pub mod service_bridge;`",
        ),
        CapabilityEntry::proven(
            "vixel_pass VixelAtom docstring CUT — the WS0 note asserted forge-shaders was an orphan with no Cargo.toml; it has one, and the same file binds `type Kernel = forge_shaders::gpu_types::VixelAtom`. Enforced misinformation, 8 lines, replaced by the host/device-pair truth",
            S::Shaders,
            "forge-gpu/src/vixel_pass.rs:33 (bind at :741) · faces mapped in forge_book::unified_stack::PRIM_FACES",
        ),
    ]
}

/// Asset fold (2026-07-31): consolidates duplicate payload homes and font sprawl — 35 files to 15 distinct hashes, all in assets/fonts.
pub fn session_2026_07_31_asset_fold() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    let mut rows = vec![
        CapabilityEntry::proven(
            "crates/output FOLDED — three render payloads (diffscan .pgm, photomesh + voxelize .glb) sat in a second output home under crates/; unique content, no size-twin anywhere in output/, zero code refs",
            S::Runbook,
            "moved to output/ · crates/output now empty · grep crates/output over rs|toml|vixi = 0 hits",
        ),
        CapabilityEntry::proven(
            "crates/assets/IBMPlexMono-Regular.ttf ORPHANED — byte-identical (sha256 7430751F8621B402..) to both assets/fonts/ and crates/assets/fonts/ copies; every include_bytes! site in the tree ends in `assets/fonts/`, so the bare-dir copy had no reader",
            S::Runbook,
            "husk _vault/_trash/2026-07-31-asset-fold/ · grep 'assets/IBMPlexMono-Regular' over crates/ = 0 matches · restore = move back to crates/assets/",
        ),
        CapabilityEntry::proven(
            "ROOT assets/fonts IS THE SUPERSET — 7 font homes, 35 .ttf, 15 distinct sha256; all 15 live in assets/fonts/, so the other 20 copies are pure redundancy (~2.8MB) and the canonical home is already complete",
            S::Capabilities,
            "assets/fonts vs crates/{assets,forge-gui,forge-overlay,forge-render,forge-tui,ironroot}/assets/fonts · SHA256 census this session",
        ),
    ];
    // A mislabelled face is worse than a duplicate: the caller believes it loaded a
    // font it did not. Not Proven, because the CURE is Sean's — ship the real
    // CommitMono, or drop the const and let the fallback own the lane.
    rows.push(CapabilityEntry {
        status: CapabilityStatus::Planned,
        ..CapabilityEntry::proven(
            "CommitMono-400-Regular.ttf IS NOT COMMITMONO — assets/fonts/CommitMono-400-Regular.ttf hashes 7430751F8621B402.., identical to IBMPlexMono-Regular.ttf; forge_gui::font_stamp loads it as a distinct face and gets Plex twice",
            S::Capabilities,
            "crates/forge-gui/src/font_stamp.rs:62 · two names, one file, one shape on screen",
        )
    });
    rows
}

/// The asset fold as a chapter — the live caller for [`session_2026_07_31_asset_fold`].
pub fn asset_fold_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Fold 2026-07-31 — the asset drain", AtlasSection::Runbook);
    ch.add_lore(
        "Two payload homes collapsed to one each, and the font sprawl gauged: 35 files, \
         15 distinct contents, root assets/fonts already holding every one of them.",
    );
    for cap in session_2026_07_31_asset_fold() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// The cut pass as a chapter — the live caller for [`session_2026_07_28_cut`].
pub fn cut_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Cut 2026-07-28 — the prim-fold drain", AtlasSection::Runbook);
    ch.add_lore(
        "Three removals, each gated on a proof of absence rather than a judgement call. \
         The primitive itself did NOT fold: four faces, zero folds — see unified_stack::PRIM_FOLD_VERDICT.",
    );
    for cap in session_2026_07_28_cut() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Ironroot fold (2026-07-28): unifies one game-logic system — 174-file twin folded into forge-game-systems, 156 byte-identical + 18 reconciled.
pub fn session_2026_07_28_ironroot_fold() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "165 twin files FOLDED — lore/** (122), narrative/** (13), overlay/**, player/**, and 24 top-level singles left ironroot; its lib.rs now re-exports them from the one home in three waves, zero duplicate declarations remaining",
            S::Capabilities,
            "ironroot/src/lib.rs (pub use forge_game_systems::{..}) · husks _vault/_trash/2026-07-28-ironroot-fold/ · census .forge/twin-{identical,drift}.txt",
        ),
        CapabilityEntry::proven(
            "Celestial combat pipeline RECONCILED UP — 10 self-contained submodules (audio_dispatch, combo_heat, edict_surge, evaluate, input_chord, parry, shadow_grab, sieve, strike) plus ChordAction/CombatState/PatternMap/AudioCommand/VfxEvent/CombatResult and the BIT_* chord constants moved into the one home; every import was `super::`-only, so nothing else came across",
            S::Capabilities,
            "forge-game-systems/src/combat/mod.rs:24 · crossbeam-channel promoted dev->runtime (Cargo.toml:44) · cargo test -p forge-game-systems 1911 passed 0 failed",
        ),
        CapabilityEntry::proven(
            "PackedInput + OverlayValue::Float ported UP — the arena wire format (5b X / 5b Y / 6b buttons, two's-complement) now sits beside InputBits rather than duplicating it, and the overlay merger gained the float lane it lacked, scaled through the i64 pipeline so the merge stays bit-reproducible",
            S::Capabilities,
            "forge-game-systems/src/input.rs:136 · overlay/mod.rs:34 + overlay/merge.rs:54 · zone newtypes surfaced at zone_runtime.rs:8",
        ),
        CapabilityEntry::proven(
            "HOST/ENGINE line drawn by the gate, not by taste — player/rig.rs (forge_render + glam) and combat/integration_tests.rs (cartridge_arena + visual::combat_uniforms) were pushed up, failed cargo check, and went back host-side; cartridge/session/persist/visual_state proved to be NAME COLLISIONS, not twins (GameCartridge+wgpu, IronrootSession, ironroot_dir saves, GPU uniforms)",
            S::Runbook,
            "forge-game-systems/src/player/mod.rs:7 · ironroot/src/combat/mod.rs (12-line host shim) · residual debt IRONROOT-HOST-TIER-TWINS in .forge/recovery/TECH-DEBT.json",
        ),
    ]
}

/// The fold pass as a chapter — the live caller for [`session_2026_07_28_ironroot_fold`].
pub fn ironroot_fold_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Fold 2026-07-28 — one game-logic system", AtlasSection::Capabilities);
    ch.add_lore(
        "A second home for the same logic is not a backup, it is a fork that no one is reading. \
         ironroot held one and could not even compile it. Identical twins folded free; drifted \
         ones were reconciled UP into forge-game-systems before their husks moved out.",
    );
    for cap in session_2026_07_28_ironroot_fold() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Ironroot bin fold (2026-07-31): consolidates two binaries to one, dropping winit dependency — the legacy host folds to the sovereign Win32 host.
pub fn session_2026_07_31_ironroot_one_bin() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "ironroot TWO bins -> ONE — the surviving target keeps the shipped NAME `ironroot` while pointing at the sovereign host, so nothing downstream re-learns a binary name; `sovereign` joined default features so a bare `cargo build -p ironroot` builds THE host instead of nothing",
            S::Capabilities,
            "ironroot/Cargo.toml [[bin]] name=ironroot path=src/bin/ironroot-sovereign.rs · default=[hud,lore,sovereign]",
        ),
        CapabilityEntry::proven(
            "winit DRAINED from the crate — the dep and the `legacy-bin` feature that gated it are gone; the window is forge-gpu SovereignWindow and the input is forge-input RawInputState, so the game crate now takes no cross-platform window dependency at all",
            S::Capabilities,
            "ironroot/Cargo.toml:47 (dep removed) · husk _vault/_trash/2026-07-31-ironroot-fold/main.rs (41,641B, authored 2026-05-15)",
        ),
        CapabilityEntry::proven(
            "NOT-PORTED named rather than dropped quietly — GamePhase (loading intro / title menu / character select, main.rs:55), IronrootApp (:85, impl :125, ~780 lines) and RenderState (:73) did not come across; only `impl ApplicationHandler` (:908) is dead on arrival, the rest is portable and owed",
            S::Runbook,
            "debt row IRONROOT-PHASES-UNPORTED in .forge/recovery/TECH-DEBT.json · husk is the source if it is ported back",
        ),
    ]
}

/// The one-bin fold as a chapter — the live caller for [`session_2026_07_31_ironroot_one_bin`].
pub fn ironroot_one_bin_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Fold 2026-07-31 — ironroot, two bins to one", AtlasSection::Capabilities);
    ch.add_lore(
        "A second binary is a second thing to keep true. This one had been false since May: \
         a winit host nobody built, holding a window dependency the engine had already \
         replaced. The fold cost one manifest edit and named what it did not carry across.",
    );
    for cap in session_2026_07_31_ironroot_one_bin() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Band and bin drain (2026-07-31): terminal band now receives keyboard input and technothesia bin folds to single studio entry point.
pub fn session_2026_07_31_band_and_one_bin() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "launcher terminal band HEARS — the boot shell reads the same `fold_rects` split it is drawn from, so a press inside the band sets `term_kb_focus`; PROVEN on glass, not asserted: typed=11 written=11 presses=1 kb_focus=true, and the PTY grid grew 80 -> 90 lit atoms on the echo",
            S::Capabilities,
            "forge-studio/src/main.rs:3856 (band arm, folded into the dock's own click-sticky block) · before: typed=3 written=0 on_term=false · after: typed=11 written=11 on_term=true",
        ),
        CapabilityEntry::proven(
            "TERM-INPUT-SWALLOW closed at the cause — bare [ ] and Ctrl+B/D/E/G/N/L, F5, F9 and F12 are PSReadLine keys that fired studio actions AND reached the shell; every one now carries the `!on_term` guard the audio and numpad arms already had. Ctrl+` is the deliberate exception (it must still close a focused terminal), and the dock's own Ctrl+T/W/Tab/Y stay ungated because they ARE the terminal's keys",
            S::Capabilities,
            "forge-studio/src/main.rs:4356,4363,4366,4777,4800,6324,6377,6486,6565,7130 · consumer side: on_create + Ledger arm gated at :3917",
        ),
        CapabilityEntry::proven(
            "technothesia [[bin]] STRUCK (root#one-engine) — all four remaining modes folded onto `13forge-studio technothesia <mode>` and twin-certified BYTE-IDENTICAL while both binaries were still on disk: score 953B, sing-report 462B, sing-midi 312B .mid, present-demo 1,536,054B .bmp, sha256 compared per artifact",
            S::Capabilities,
            "forge-studio/src/main.rs:1234 (present-demo|score|sing-midi|sing-report) · technothesia/Cargo.toml [[bin]] removed · husk _vault/_trash/2026-07-31-one-engine-technothesia/main.rs sha256 ECB2631A2255A4D7DB1F0C10060219E228AF381D7D124089D59B00F0CDB06DFF · restore = move back + re-add [[bin]] (re-opens the conflict, needs Sean)",
        ),
        CapabilityEntry::proven(
            "pack_munsell is a const fn — the band's own grain colour could not compile (`kit_face.rs:143` calls it in a `const`), and the body is pure integer bit math, so the fix is the keyword rather than a hex at the call site",
            S::Capabilities,
            "forge-core/src/colour_id.rs:28",
        ),
        CapabilityEntry::proven(
            "NAMED, not absorbed: the band's emissive lane is DARK on this GPU — VibeUberPass panics inside naga's HLSL writer (`No bind target was defined for the push constants block`, writer.rs:894), caught at main.rs:7593, so the 90 atoms never reach pixels. The terminal now hears and echoes; on this device nothing of it is visible",
            S::Runbook,
            ".forge/bin/logs/panic.log (two entries 2026-07-31) · forgewright capture snap_00000003167e7c00.png: band region y454-681 fully black",
        ),
    ]
}

/// The band-and-bin drain as a chapter — the live caller for [`session_2026_07_31_band_and_one_bin`].
pub fn band_and_one_bin_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Drain 2026-07-31 — the band hears, the second bin goes", AtlasSection::Capabilities);
    ch.add_lore(
        "A terminal you can see but cannot type into is chrome. This one was worse than \
         chrome: a real shell, drained every frame, deaf because the only thing that could \
         focus it did not exist on the screen it was drawn on. The fix was one variable \
         gaining a third source, and the same variable then closing the swallow it caused.",
    );
    for cap in session_2026_07_31_band_and_one_bin() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Ghost SKU pull (2026-07-29): removes product from shelves — pages, downloads, and zero-caller references moved to trash.
pub fn session_2026_07_29_ghost_pull() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "ghost-thanks husk MOVED — the post-checkout page linked /downloads/ghost-desktop.exe (absent) and published its SHA-256, so any held link hit a dead download for a pulled binary",
            S::Runbook,
            "sites/13forge-site/{public,dist}/ghost-thanks.html · husk _vault/_trash/2026-07-29-ghost-sku-pull/{public,dist}/ · restore = move back",
        ),
        CapabilityEntry::proven(
            "ghost-desktop README husk MOVED — download-side doc for the same pulled binary, staged in both public/ and dist/",
            S::Runbook,
            "sites/13forge-site/{public,dist}/downloads/ghost-desktop-README.txt · husk _vault/_trash/2026-07-29-ghost-sku-pull/{public,dist}/downloads/",
        ),
        CapabilityEntry::proven(
            "sky-drop husk MOVED (page + module) — orphan by proof of absence: a repo-wide grep over *.html found references only inside the file itself, so nothing on the live site linked it",
            S::Runbook,
            "sites/13forge-site/{public,dist}/sky-drop.{html,mjs} · husk _vault/_trash/2026-07-29-web-attic/",
        ),
        CapabilityEntry::proven(
            "drone-thanks husk MOVED — post-checkout page for a SKU with no shelf row and no inbound link, same dead-end shape as ghost-thanks",
            S::Runbook,
            "sites/13forge-site/{public,dist}/drone-thanks.html · husk _vault/_trash/2026-07-29-web-attic/",
        ),
        CapabilityEntry::proven(
            "magic canvas KEPT and named as the reason — the only offline-creation surface on the site and the one Sean asked for by name; index links both faces, so it is live, not attic",
            S::Capabilities,
            "sites/13forge-site/dist/index.html:105-106 → magic-canvas.html + magic-canvas-offline.html (download)",
        ),
        CapabilityEntry::proven(
            "storefront proven clean of the pulled SKU — case-insensitive grep for 'ghost' over the deployed index returned 0 matches, so no live page advertises it",
            S::Runbook,
            "sites/13forge-site/dist/index.html · 0 occurrences (rg -i ghost)",
        ),
        CapabilityEntry::new(
            "SHELF NOW HOLDS ZERO LIVE SKUs — site CLAUDE.md #rollout still declares Ghost Desktop $9 as THE one live SKU with its Stripe link; CLAUDE.md is agent-read-only at every layer (root#agent-code) so the drift is gauged here, not edited. The payment link is live server-side; deactivating it is a Stripe dashboard action, outside the repo",
            S::Runbook,
            CapabilityStatus::Planned,
            "sites/13forge-site/CLAUDE.md #rollout vs disk · Sean-gated: canon edit + dashboard",
        ),
    ]
}

/// The pull pass as a chapter — the live caller for [`session_2026_07_29_ghost_pull`].
pub fn ghost_pull_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Pull 2026-07-29 — the ghost comes off the shelf", AtlasSection::Runbook);
    ch.add_lore(
        "Three husk moves proven by absence, plus one UNPROVEN row that stays loud: the \
         shelf is empty while the law still says it is stocked. An empty shelf is cleaner \
         than a wrong one, but only if the gauge says so out loud.",
    );
    for cap in session_2026_07_29_ghost_pull() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Goldminer husk fold (2026-07-30): removes six skeleton Python files with no callers — name collisions with live recovery tools.
pub fn session_2026_07_30_goldminer_fold() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "goldminer husk MOVED — six python skeletons, 292B-1279B, whose bodies were print+pass with no output sink; the pipeline's last stage printed to stdout and wrote no artifact",
            S::Runbook,
            "tools/goldminer/{diamond,lexicon,golderminer,syntax_labeller,recovery_pipeline,triage_scanner}.py · husk _vault/_trash/2026-07-30-goldminer-husk/ · restore = move back",
        ),
        CapabilityEntry::proven(
            "orphan proven by absence — repo-wide search outside _vault/ found ZERO importers of these modules; every live hit was the unrelated Rust SKU crates goldminer-core/goldminer-app, which never touch them",
            S::Runbook,
            "Cargo.toml:47-48 (goldminer-core, goldminer-app) · 0 python importers",
        ),
        CapabilityEntry::proven(
            "name collision named — the husks duplicated the names of richer live implementations, 2 homes 1 name: diamond.py 292B vs find_diamonds.py 11354B, lexicon.py 260B vs concept_lexicon.py 7258B, golderminer.py 710B vs goldmine.py 23390B",
            S::Runbook,
            "crates/forge-recovery/scripts/{find_diamonds,concept_lexicon,goldmine}.py · live, untouched",
        ),
        CapabilityEntry::new(
            "SYNTACTIC LEG ABSENT — the husk's one unique intent: syntax_labeller.py declared SYNTACTIC=DAG(forge-dag,AST) labeling held separate from semantic. The live recovery chain is ENTIRELY semantic (concept families via concepts_of); no AST leg exists, so root#cognitive-alignment's separation is asserted by test but unexercised in the recovery lane",
            S::Runbook,
            CapabilityStatus::Planned,
            "crates/forge-ml/tests/semantic_syntactic_separation.rs (SEMANTIC_LANES vs IDENTITY_LANES) vs crates/forge-recovery/scripts/concept_lexicon.py · Sean-gated: build the leg or drop the intent",
        ),
        CapabilityEntry::proven(
            "5D judgement anchor is LIVE — diamond.py named TritTree5D in a comment and nowhere else; the name resolves to a real balanced-trinary index over PackedPoint105, so the husk pointed at working machinery it never called",
            S::Capabilities,
            "crates/outland/src/trit_tree.rs:43 (struct TritTree5D)",
        ),
        CapabilityEntry::proven(
            "mass-read/mass-weld verbs allow-listed — both were ABSENT from the harness allow set, so every weld prompted; matching is glob over the whole command string, and harness_config observes hooks only, never permissions",
            S::Runbook,
            ".claude/settings.json permissions.allow += PowerShell(*13forge-studio massread*), PowerShell(*13forge-studio massweld*)",
        ),
    ]
}

/// The fold pass as a chapter — the live caller for [`session_2026_07_30_goldminer_fold`].
pub fn goldminer_fold_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Fold 2026-07-30 — the goldminer husk", AtlasSection::Runbook);
    ch.add_lore(
        "Six skeletons carrying the names of live tools. Five rows are moves proven by \
         absence; the sixth is the only thing in them worth keeping — a syntactic leg the \
         recovery chain never grew. A name that points at real machinery is not a caller.",
    );
    for cap in session_2026_07_30_goldminer_fold() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// The drain as a book chapter — one lore row per merged crate, receipts inline.
pub fn drain_chapter() -> Chapter {
    let mut ch = Chapter::new("Session Drain 2026-07-23 — the Gemini-welder merge", AtlasSection::Capabilities);
    ch.add_lore("28 crates driven to green by a flash/pro welder swarm; Opus merge-gate, every lane self-verified by cargo test. Whole-tree gate cargo check -p forge-studio EXIT 0.");
    for cap in session_2026_07_23_drain() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Cosmic Dissonance Kernel (2026-07-29): 5D spatial kernel mapping faction triads and scalar fields — one cell in, whole scene out.
pub fn session_2026_07_29_cdk() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "CDK BORN — Triad{love,strife,entropy} permyriad integers; triad() faction axes + continuous proximity + depth, triad_from_fields() collapses N forge-zones scalar fields onto 3 lanes, room_frame() resolves FactionAction + Stance + sky byte + prime seed",
            S::Capabilities,
            "crates/forge-game-systems/src/cdk.rs · cargo test -p forge-game-systems --lib cdk = 12 passed 0 failed",
        ),
        CapabilityEntry::proven(
            "FIRST LIVE CALLER for the 8-axis faction cognition — choose_faction_action had ZERO callers repo-wide before this (forge-cart-brain reaches it only from its own tests); discharges the aspire NEXT row cdk-faction-stimulus",
            S::Capabilities,
            "forge_book::aspire:150 → crates/forge-game-systems/src/cdk.rs room_frame",
        ),
        CapabilityEntry::proven(
            "RANK DEFECT CAUGHT BY ITS OWN PICTURE — triad() shipped a CONSTANT lane (in_sphere() as a bool), the root#rank const-or-zero shape; examples/cdk_wireframe.rs walked z and printed four identical rows, which is how it surfaced. Now continuous radius pull + depth",
            S::Capabilities,
            "crates/forge-game-systems/examples/cdk_wireframe.rs · guard test walking_the_z_lane_changes_the_triad",
        ),
        CapabilityEntry::proven(
            "BIND RANGE IS A SEAM, NOT A CAST — to_channels() saturates all three lanes into 0..=1000; raw love spans about +-3700 and raw strife goes negative, so any caller binding raw fields to vibematrix or vibe_post_process_at feeds a shader garbage",
            S::Capabilities,
            "crates/forge-game-systems/src/cdk.rs to_channels · test channels_stay_in_bind_range (3 factions x 2 cells x 4 haunts)",
        ),
        CapabilityEntry::proven(
            "BOTH CLOCKS JOINED — TriadHold authors on the sim-beat clock and presents on the 120Hz wall clock by integer permyriad lerp; a late beat SATURATES on the target and never extrapolates past it. Transport is forge_hal::TripleBuffer via impl ClockPlane for Triad (1 producer, 2 consumers)",
            S::Capabilities,
            "crates/forge-game-systems/src/cdk.rs TriadHold + ClockPlane · tests the_hold_crosses_from_beat_clock_to_wall_clock, the_triad_crosses_the_clock_bridge",
        ),
        CapabilityEntry::proven(
            "THE HAUNT WAS ALREADY NAMED ON THE FIELD SIDE — FieldType::DissonanceStress documents itself as 'compounds rather than settling, which is why the governor quarantines it instead of waiting it out'. triad_from_fields folds it with Corruption into entropy; an unauthored field reads as ABSENCE, never neutral",
            S::Capabilities,
            "crates/forge-zones/src/scalar_field.rs FieldType · tests empty_fields_read_as_absence_not_neutral, fields_drive_the_three_lanes",
        ),
        CapabilityEntry::proven(
            "ASCII OFF THE REAL API, TWICE — cdk_wireframe prints the singing-terminal frame and dungeon_map prints a 33x33 floor across three z slices; both follow forge-zones/examples/water_wireframe.rs so the picture and the tests cannot disagree, and both caught a defect on first render",
            S::Runbook,
            "crates/forge-game-systems/examples/{cdk_wireframe,dungeon_map}.rs",
        ),
        CapabilityEntry::proven(
            "ONLY LOVE VARIES LATERALLY — dungeon_map's first cut inked (strife + entropy) and drew a perfectly FLAT floor: strife carries the faction axes plus DEPTH and entropy carries the haunt, so neither responds to x or y. All lateral variation in the kernel lives in love's proximity pull alone, which is why the map inks disposition",
            S::Capabilities,
            "crates/forge-game-systems/examples/dungeon_map.rs · z=-10 prints uniform because the strife term swamps the radius pull at depth",
        ),
        CapabilityEntry::proven(
            "GameBroski FILLED with the IRONROOT vocabulary, not an invented one — State = NarrativeState's lowering, Action = the seven authored ResolutionModes (Kill/Spare/Expose/Bind/Erase/Inherit/Abandon) scored by each mode's own resolution_effects delta, deterministic tie-break on declaration order. Player side re-exports ActionVerb (18) and HiddenAccount (13)",
            S::Capabilities,
            "crates/forge-broski/src/lib.rs · ironroot game logic drained to forge-game-systems 07-28 (Cargo.toml:66-77) · cargo test -p forge-broski --lib = 291 passed",
        ),
        CapabilityEntry::proven(
            "TWO LISTENER HUSKS CUT — SunnyBroski and SlappBroski were empty stubs (State=(), Action=(), listen -> vec![]) with zero consumers; SLAPP died 07-21 and HeySunny has no crate in this tree. A stub naming a product that does not exist reads as planned work forever",
            S::Runbook,
            "crates/forge-broski/src/lib.rs · Cargo.toml:194 · husk _quarry/slapp-fold-2026-07-21/",
        ),
        CapabilityEntry::proven(
            "THE LANE BRINGS ITSELF UP — board --harvest --lane gemma now starts the sidecar instead of printing instructions, bounded by LANE_WARM_CEILING_S. Terminal SidecarBlocks (NotBuilt, NoWeightDir) fail FAST: sidecar::run returns 0 on NotBuilt by design, so the exit code could never be the signal and the first live run burned the whole 240s ceiling before saying so",
            S::Runbook,
            "crates/forge-studio/src/board_harvest.rs ensure_lane_live · crates/nde_chat built (the sidecar binary simply did not exist)",
        ),
        CapabilityEntry::proven(
            "BOARD 179G -> 200G, 27U -> 16U — 7 CDK rows + 3 orphan tags (GATE-IMMUTABLE, MASSWELD-VERIFY, CLI-NO-GHOST-GUI) declared, and 6 harvests stranded in the lane queue drained once the sidecar could answer",
            S::Runbook,
            "seal 5220830c10eb -> a1d724fd38b0 · board_sync::worldmerge_tasks",
        ),
        CapabilityEntry::new(
            "BLUEPRINT IS 3D AND THE REST OF THE STACK IS 5D — the Blueprint Manifold is the authoritative spatial logic for a level/room/zone, but its types stop at Vec3M and Bounds3D. It cannot express T (tick) or S (scale), so a blueprint node cannot say WHEN or at what LOD it exists, while normalized_zone, trit_tree, raycast and the CDK all ride [x,y,z,t,s]. Only blueprint_3d.rs and clockwork.rs reach forge_zones at all",
            S::Capabilities,
            CapabilityStatus::Planned,
            "crates/forge-tile-crawler/src/architecture/blueprint.rs Vec3M:35 Bounds3D:44 · vs forge-zones/src/normalized_zone.rs Cell{x,y,z,t,s}",
        ),
    ]
}

/// The kernel pass as a chapter — the live caller for [`session_2026_07_29_cdk`].
pub fn cdk_chapter() -> Chapter {
    let mut ch = Chapter::new(
        "Session Kernel 2026-07-29 — the Cosmic Dissonance Kernel",
        AtlasSection::Capabilities,
    );
    ch.add_lore(
        "One 5D cell in, a whole scene out. Twelve proven rows and one that stays loud: the \
         blueprint manifold is still 3D while everything it feeds is 5D. Two ASCII examples \
         earned their keep by breaking on their own first render.",
    );
    for cap in session_2026_07_29_cdk() {
        ch.add_lore(&format!("{} — {}", cap.name, cap.receipt));
    }
    ch
}

/// Session proofs (2026-08-02): GPL tree removal, reel plates, dual-lane rendering parity, and layout emission modes — thirteen seams with parity paid.
pub fn session_2026_08_02() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "GPL-2.0 Linux kernel tree removed from the product repo — ~94k files / 1.5 GB \
             of GPL-2.0 WITH Linux-syscall-note archived inside a repo that ships; nothing \
             linked it and it never entered the river, but an archive is not a licence \
             boundary. Its Sony-named files were Sony Mobile's own upstream mainline \
             contributions — nothing taken. TWIN = kernel.org (git clone torvalds/linux), \
             which is what made a delete of this size safe. The EDGE.md edges it anchored \
             measured path-index THROUGHPUT over a foreign C tree the concept lexicon cannot \
             classify — speed, never signal (Sean: 'the wrong metric and not a useful one'), \
             so losing the corpus drops a flattering proof, not a real one",
            S::Capabilities,
            "crates/outland/EDGE.md — the surviving on-disk consequence: every edge there \
             still cites the linux-master corpus (EDGE.md:1, :16) that no longer exists here. \
             Deleted tree was _vault/output/outland/demo-repos · T2, Sean-gated 2026-08-02",
        ),
        CapabilityEntry::proven(
            "reel plate — a recorded image seated as the truth rail's ground, anchor attached: decode once, integer-nearest sample to the rail box (no float filter, so two seats are the same pixels), correspondence colour off a 16x16 thumbnail. The anchor hashes the ORIGINAL FILE, not the seated pixels, so SourceKind decides residue and the extension never does",
            S::Capabilities,
            "crates/forge-gui/src/reel/{plate.rs,dual_rail.rs} · the 8 painted ironroot backgrounds seat and anchor free (HumanAuthored, R=0); one PNG through both SourceKinds proves residue == the original file's length · cargo test -p forge-gui --lib reel:: = 192 passed",
        ),
        CapabilityEntry::proven(
            "reel FrameParts — one frame build, two render lanes. compose_parts() returns {ground: full-frame RGBA8, chrome: DrawList}; the offline lane rasterizes on CPU, the live lane crosses the ground as a forge_gpu LayerPlane (identical layout, no repack) and the chrome as ComposeFrame::draw, under forge_hal::TickBudget + TripleBuffer",
            S::Capabilities,
            "crates/forge-gui/src/reel/dual_rail.rs (compose_parts/FrameParts::rasterize_cpu) · the_two_lanes_render_the_same_frame asserts parts->CPU is byte-identical to the pre-split one-shot · frame_composer.rs:51/:128 are the GPU-side shapes",
        ),
        CapabilityEntry::proven(
            "source-compiler SPLIT, ledgered late — the five-gate compiler lived whole in forge-sieve; the front end (analyse/rulify/condense, pure) stayed and the back end (schedule/render, the timed beat track and frame reel) moved to forge-studio. The pre-split original went to trash on 2026-08-02 with NO compiled row, so the gauge below — added in this same session — went red against its own session's husk. The row is the fix; the gauge worked",
            S::Runbook,
            "husk _vault/_trash/2026-08-02-source-compiler-fold/source_compiler.rs 23,400B sha256 1E2757B6E1EBFD9D0188CC4088DD85D9123F5F4C2EF6FA2CB80F42937698A29C (header: five gates + `use crate::kg_rule`, the forge-sieve original) · live halves crates/forge-sieve/src/source_compiler.rs:1 (\"front end — three pure gates\") + crates/forge-studio/src/source_compiler.rs · restore = move back and drop one half, which re-forks the ladder",
        ),
        CapabilityEntry::proven(
            "trash ledger GAUGE — the delete law had three recording places (_trash dir, RESTORE.md, compiled row) and ZERO verbs checking they agree. every_trash_block_has_a_compiled_receipt walks the real _vault/_trash on every test run and fails on any husk with no row; a RESTORE.md beside the husk does not satisfy it, by design",
            S::Runbook,
            "crates/forge-book/src/session_drain.rs (receipted_trash_blocks + the gauge) · it caught 3 orphan blocks a human eye had missed (fold-labhub, 2026-07-31-hooks, umwelt-prompt) · 9 green -p forge-book session_drain",
        ),
        CapabilityEntry::proven(
            "forge-vfx fold RESOLVED — the world.idx row named a crate folded into forge-core::vfx on 2026-07-10 (Cargo.toml:118) and nothing had repointed it. Donor diffed line-by-line against the live fold: import rewrites ONLY, zero logic. Donor pulled into SoT so the proof stops living off-tree; 4 world.idx rows repointed at live homes",
            S::Runbook,
            ".forge/domains/world.idx (forge-core::vfx · forge-book::lore · forge-anim::cue::presence · forge-dialogue/forge-chimera; 1458B -> 2318B) · donor _quarry/folded-donors-2026-07-11/forge-vfx (sha-verified) · 6 empty shells + 3 certified T1 twins deleted, 523,760B reclaimed",
        ),
        CapabilityEntry::proven(
            "umwelt narrator RESTORED — the husk's note claimed it was superseded by crates/sf-wasm/src/weaver.rs, a file that does not exist. Orphaned work, not dead work: closed payload + UMWELT_GOVERNOR + violation() checker, inference stays out-of-process (root#nde-ladder)",
            S::Capabilities,
            "crates/sf-wasm/src/umwelt_prompt.rs (sha E9446F60988AE5F7) + Cargo.toml serde_json (the missing dep that kept it from ever compiling) · cargo test -p sf-wasm --lib umwelt_prompt = 4 passed",
        ),
        CapabilityEntry::proven(
            "the ARBITER's second face — umwelt_prompt::violation judges PROSE against a body; weaver_arbiter::violation judges JSON against the crafting laws, same signature (Option<&'static str>, the rule broken). rejection() emits the CRITICAL REJECTION block the Weaver's own modelfile was written to obey and had never received. The ceiling is WIRED, not restated: pub use forge_items::powercurve::POWER_MAX",
            S::Capabilities,
            "crates/sf-wasm/src/weaver_arbiter.rs · work/dream_diamonds/docs/weaver{,-forge}.modelfile is the generator half (qwen2.5-coder, out-of-process) · the_ceiling_is_the_powercurves_own mints through the real powercurve() and asserts the judge passes it · 10 green",
        ),
        CapabilityEntry::proven(
            "Atlas SKIN — the index is the same proven rows for everyone, the DOCUMENT is not. AtlasSkin picks section order (omission = drop), renames sections (the word swap, anchors keep their slug), hides receipts, filters proof tiers, sets palette. A skin is a VIEW: it can hide a receipt, never rewrite one, never conjure a row",
            S::Capabilities,
            "crates/forge-book/src/atlas_html.rs (AtlasSkin + skinned_page) · the_house_skin_is_the_page_that_shipped_before_skins is byte-parity against the pre-skin renderer · a_skin_can_hide_a_receipt_but_never_invent_a_row asserts row count == book.capabilities.len() · 7 green",
        ),
        CapabilityEntry::proven(
            "PARARITY MEASURED — 'author once in .kit.vixi, emit every target' (emit_html.rs:5) was a claim with no receipt in either direction. One authored kit down both lanes: the shared box is BIT-IDENTICAL (24,62,592,40). vixi -> HTML5 -> html_lower cascade+layout -> DrawList reproduces the native rect exactly. The assumption inverted: the THIN lane is the native one (render_lowered_ui emits Rects for widget leaves only, so coloured region grounds never reach the DrawList)",
            S::Capabilities,
            "crates/forge-overlay/tests/emit_pararity.rs · 3 green · native [(24,62,592,40)] vs html 7 boxes, the shared one identical",
        ),
        CapabilityEntry::proven(
            "LayoutEmitMode — the two layout TRUTHS, split on Sean's ruling 08-02. Exact (DEFAULT) walks the solved plane: position:absolute off LayoutBox.rect, z-index off LayoutBox.z, data-vixi-id off stable_key (the editor's reverse map), asserted to contain no display:flex. Responsive (opt-in) walks the widget tree: LayoutPolicy -> flex/grid, container-type:inline-size so @container is reachable, asserted to freeze no rect. Authored affordance flags (hover_reveal/collapsible/long_press_drawer) ride to CSS as data attributes in BOTH modes with no script tag",
            S::Capabilities,
            "crates/forge-vix/src/emit_html.rs (LayoutEmitMode::{Exact,Responsive} + emit_html_mode) · 7 green -p forge-vix emit_html · the_two_truths_are_not_the_same_document",
        ),
        CapabilityEntry::proven(
            "THE TS ORACLE PARTIALLY DRAINED — `tools/vixi-compiler-final` was not a dead tool, it was a LIVE \
             authority sitting outside the bin: diagnostics.rs cited it as 'the authoritative parser' \
             for shaderbind + renderpass and grammar.rs DIALECTS listed it as their OWNER, which is \
             root#binary-verb inverted (a gate that is not a compiled verb). Its block grammar \
             (`material foo { material_id: 12; }`) is NOT folded — the sovereign parser never spoke it, \
             and one example had already been hand-translated to v1 on 07-28. What folded is the one \
             thing it alone had: the CROSS-FILE view. `vixi bundle` emits vixi.bundle.v1 over the live \
             dialects and judges what no single-file lint can — a renderpass painting a surface no \
             shaderbind declares. LEVELLED, not invented: the first draft's rule (channel prefix must \
             equal the declared surface) would have flagged the shipping audio_vis binding, because \
             shaderbind_dsl.rs:426-432 holds identity and target surface as independent fields on \
             purpose. Surface names are dotted (`aura.bard`), so the route parse cuts at `.channel[`. \
             WHAT DID NOT FOLD, named rather than implied: validator.ts still holds seven determinism laws \
             with no Rust home — MATERIAL_COLOUR_IDENTITY, REQUIRED_NUMBER_MATERIAL_ID, \
             VIXEL_COORD_OR_GENERATION, VIXEL_GENERATION_SEED, RESONANCE_Q_RANGE (integer permyriad), \
             RESONANCE_COLOUR_ONLY, socket owner/kind/compatibility. They have no home because \
             diagnostics::check has no `material` or `socket` arm at all; porting the laws needs those \
             dialect arms first, and the block grammar under them is not worth porting",
            S::Capabilities,
            "crates/forge-vix/src/bundle.rs (SCHEMA vixi.bundle.v1 · bundle-surface-undeclared ERROR · \
             bundle-target-unpainted WARN) · verb crates/forge-studio/src/main.rs `vixi bundle <f>... [--json]` \
             · ownership flipped grammar.rs:227 DIALECTS + diagnostics.rs:209/:283 (golden re-blessed) \
             · 9 green -p forge-vix --lib bundle:: , 393 lib green · TS tree left on disk unread-safe: \
             T2 (user-authored, sole copy), Sean-gated",
        ),
        CapabilityEntry::new(
            "THE 8-SLOT PINCH (Sean 08-02: 'hard coded 8 colours, I hate that') — the authored .sheet.vixi is UNBOUNDED (ladder ink/paper/signal, arbitrary depth + role bindings) and forge-colour generates 64 (palette64 -> [OklchPmy; 64], OKLCH, harmony schemes, scotopic + contrast gates). Between two open systems sits one closed struct: forge-vix tokens.rs:81 Palette, 8 named fields. parse.rs has NO ladder/role keyword at all, so the authored ladders are silently dropped on the way in",
            S::Learning,
            CapabilityStatus::Study,
            "forge-ast/corpus/native/sheet/forge_brand.sheet.vixi:14-24 (open) · forge-vix/src/tokens.rs:81 (closed 8) · forge-colour/src/lib.rs:190 palette64 + :65 contrast_ok (open 64) · blast radius = 21 files across 12 crates naming the 8 slots · emit_html.rs:48 token_shade() is still a hash-to-fake-hex, not a sheet resolve",
        ),
    ]
}

/// Drain 2026-08-01 — the three husk blocks that moved without a compiled row.
///
/// Found by `every_trash_block_has_a_compiled_receipt`, not by reading a
/// directory listing: two carried a `RESTORE.md` (prose beside the husk, which
/// is exactly what the rule says the receipt is NOT) and one carried nothing.
/// The gauge is the fix; these rows are the debt it named.
pub fn session_2026_08_01_trash_reconcile() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![
        CapabilityEntry::proven(
            "visual-gate baselines vaulted — lab.bmp + hub.bmp left when the LAB/HUB tabs folded to one canvas (Sean 07-29 '1 canvas')",
            S::Runbook,
            "husk _vault/_trash/2026-07-29-fold-labhub/{lab,hub}.bmp · restore = move back to crates/forge-studio/qa/visual_gate_baselines/ + re-add the panels to ALL_PANELS and PANEL_ORGANS",
        ),
        CapabilityEntry::proven(
            "dead-pointer roost config vaulted — the husk drove 13forge-studio-roost.exe, a binary that does not exist on disk, with 8 write gates; the LIVE .claude/settings.json supersedes it with 13 write gates (adds ast-protection/inert-shader/type-provenance/delete-clamp/zero-caller-proof), the plan-scope hook, and search-ladder over PowerShell, all on 13forge-studio.exe which is present and answers every one of the 13 gate verbs (exit 0 each, probed 2026-08-01). RESTORING THIS HUSK WOULD DISARM THE HARNESS.",
            S::Runbook,
            "husk _vault/_trash/2026-07-31-hooks/settings.json (12,129B, roost) · live .claude/settings.json:67 names all 13 gates, :88 search-ladder, :109 plan-scope, 0 roost refs · restore = DO NOT (dead binary); an attempt to copy it over live was refused by the harness classifier",
        ),
        CapabilityEntry::proven(
            "umwelt narrator payload RESTORED, not vaulted — the husk's own note claimed it was superseded by crates/sf-wasm/src/weaver.rs; that file DOES NOT EXIST, so nothing had replaced it and the module was orphaned work, not dead work (Sean 08-01 'it was new yesterday'). Folded back: closed payload + UMWELT_GOVERNOR + violation() checker, inference stays out-of-process per root#nde-ladder",
            S::Capabilities,
            "crates/sf-wasm/src/umwelt_prompt.rs (10,102B, sha E9446F60988AE5F7) · lib.rs `pub mod umwelt_prompt;` · Cargo.toml serde_json = \"1\" (the missing dep that kept it from ever compiling) · cargo test -p sf-wasm --lib umwelt_prompt = 4 passed · husk block _vault/_trash/2026-08-01-umwelt-prompt/ removed, twin now live",
        ),
    ]
}

/// Roofline drain (2026-08-04): vaults a distill checkpoint trained on fifty rows — overfitted, not resumable.
pub fn session_2026_08_04_roofline() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![CapabilityEntry::proven(
        "d512 distill checkpoint VAULTED — 101,871,879 params (vocab 256 / d 512 / 7 experts / 3 layers) built by ten passes that each used 50 of 84,269 queue rows; the default --max-tokens 20000 fills its budget on the newest ~50 rows at train_nde.rs:44 and never reads the rest. Loss 2.9052 -> 2.5918, then flat — fifty samples, memorised",
        S::Runbook,
        "husk _vault/_trash/2026-08-04-d512-50row/student-distill-d512.safetensors (407,487,700B) · proof .forge/run/nde-train-d512-20260803.log, `used=50` on all ten passes · restore = move back to nde-models/ · superseded by a fresh-init pass over the full queue, not by resuming this",
    )]
}

/// Spill tape drain (2026-08-04): vaults shadow VCS tape at bare root — 197 rows with zero readers, decoy worse than absent.
pub fn session_2026_08_04_spill_tape() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![CapabilityEntry::proven(
        "bare-root shadow tape VAULTED — F:/NewRepo/log.tsv (197 rows) + objects/ (196 blobs, 3,796,704B) written to the working tree by a caller that passed the repo root as a vcs root; zero readers, every reader opens .forge/vcs",
        S::Runbook,
        "husk _vault/_trash/2026-08-04-spill-tape/ (log.tsv + objects/, 196 blobs, 3,796,704B) · proof forge-vcs/src/lib.rs `VcsRoot::open` now refuses a working tree, test `a_working_tree_is_refused_as_a_vcs_root` · restore = move both back to the repo root, where the gate will refuse the next writer anyway",
    )]
}

/// 2026-08-04 — eight `Cargo.lock` files that no resolver ever opened.
///
/// A workspace MEMBER resolves against the root lock, so a lock file sitting
/// Dead Cargo.lock sweep (2026-08-04): removes inert lock files from workspace members and phantom nested workspace manifests.
pub fn session_2026_08_04_dead_cargo_locks() -> Vec<CapabilityEntry> {
    use AtlasSection as S;
    vec![CapabilityEntry::proven(
        "dead Cargo.lock sweep — 8 lock files removed from ROOT WORKSPACE MEMBERS (pp-math, outland, ironroot, moe-gpu-dsp, sf-wasm, forge-gpu-ops, CUI, CUI/forge-render); a member resolves against the root lock, so every one of these was inert and readable only as a false claim of independent resolution",
        S::Runbook,
        "husk _vault/_trash/2026-08-04-dead-cargo-locks/ (8 files, names = repo-relative path with \\ flattened to __) · membership proof Cargo.toml:6,44,49,83,93,163,210 · CUI/forge-render has no manifest at all (crates/CUI/Cargo.toml:21 [lib] path) · restore = move back to <crate>/Cargo.lock, though cargo regenerates them · NOT touched: the 5 declared nested workspace roots (dead-drop, 13forge-business/forge-public, forge-shaders/spirv-builder-driver, public-tools/forge-ibl-bake, public-tools/forge-weld-inspect) whose own lock is correct",
    ), CapabilityEntry::proven(
        "nested-workspace manifest sweep — 12 Cargo.toml husks lifted out of archive trees (_attic/forgeMCP-dissolved, _vault/output rustgpu-spike + kernel + custom-gpt-bootstrap + ironroot-final-engine-patch, _vault/_quarry reposold/store pulls: 13engine, 13forge-super, E-NewRepo merge, airgap snapshots, repos-mirror); every one described a workspace that no longer exists, and a stray manifest inside the live tree reads as a phantom nested workspace to cargo and rust-analyzer",
        S::Runbook,
        "husk _vault/_trash/2026-08-04-nested-ws-manifests/ (12 files, original relative paths preserved, MANIFEST.tsv = 64-bit hash + bytes + source path per file) · restore = move back to the path MANIFEST.tsv records · row compiled 2026-08-04 by the staged-integration reconcile pass from the mover's own MANIFEST.tsv",
    )]
}

// RELOCATED 2026-08-04 -> `crate::seams::SEAMS` id `G-LAW-01`, five anchors proven on
// disk every test run. It was written here first, and that was the cop-out this file's
// freeze exists to stop: a paragraph is proven by nobody.

/// Every husk block this ledger accounts for, as it appears under
/// `_vault/_trash/<date>-<block>/`.
///
/// The receipt strings already name their husk paths, so this walks the rows
/// rather than repeating them — one source, no second list to rot.
pub fn receipted_trash_blocks() -> Vec<String> {
    let mut out: Vec<String> = Vec::new(); // @forge:allow_alloc cold ledger walk, test/gauge path
    for rows in [
        session_2026_07_23_drain(),
        session_2026_07_28_cut(),
        session_2026_07_31_asset_fold(),
        session_2026_07_28_ironroot_fold(),
        session_2026_07_31_ironroot_one_bin(),
        session_2026_07_31_band_and_one_bin(),
        session_2026_07_29_ghost_pull(),
        session_2026_07_30_goldminer_fold(),
        session_2026_07_29_cdk(),
        session_2026_08_01_trash_reconcile(),
        session_2026_08_02(),
        session_2026_08_04_roofline(),
        session_2026_08_04_spill_tape(),
        session_2026_08_04_dead_cargo_locks(),
    ] {
        for cap in rows {
            let mut rest = cap.receipt.as_str();
            while let Some(i) = rest.find(TRASH_MARK) {
                rest = &rest[i + TRASH_MARK.len()..];
                let block: String = rest.chars().take_while(|c| *c != '/' && *c != '\\').collect(); // @forge:allow_alloc cold ledger walk
                if !block.is_empty() && !out.contains(&block) {
                    out.push(block);
                }
            }
        }
    }
    out
}

/// The path fragment a husk receipt must carry to count as accounted for.
const TRASH_MARK: &str = "_vault/_trash/";

#[cfg(test)]
mod tests {
    use super::*;

    /// THE FREEZE, MECHANICAL (Sean 2026-08-04 "make it read only").
    ///
    /// Counts `pub fn session_` declarations in this file's own source and pins the
    /// number. A 17th makes the build red and names the four ledgers a receipt should
    /// have gone to instead. Same idiom as `type_homes::the_drain_kept_every_declared_name`
    /// — the file is its own fixture, so the gate cannot be satisfied by editing a table.
    ///
    /// Raising this number is not a fix. It is the thing being prevented.
    #[test]
    fn the_drain_takes_no_new_rows() {
        /// The count at the moment of the freeze. A CEILING, never a target to meet:
        /// the gate is "no row is ever ADDED", so the number may fall and never rise.
        /// 17 -> 16 on 2026-08-04 when Sean's `/permit-nuke` released the massloop row to
        /// its real home (`seams::SEAMS` `G-LAW-01`, five anchors proven against disk).
        const FROZEN_AT: usize = 16;

        let src = std::fs::read_to_string(
            crate::type_homes::crates_dir().join("forge-book-v3/src/session_drain.rs"),
        )
        .expect("this file is its own fixture");
        let rows = src.matches("pub fn session_").count();

        assert!(
            rows <= FROZEN_AT,
            "session_drain.rs is FROZEN at {FROZEN_AT} rows and now has {rows}. A receipt \
             does not go here — debt -> .forge/recovery/TECH-DEBT.json (`qa debt`), board \
             row -> `board --harvest`, cross-domain seam -> crate::seams::SEAMS (its \
             anchors are proven on disk every run), priority -> board_row_priority. \
             Sean 08-04: this file is a cop-out, and raising this number is the cop-out \
             happening."
        );
    }

    /// THE GAUGE root#delete was missing: a husk that moved but never got a row
    /// is invisible, and the only thing that caught the last one was a human
    /// reading a directory listing. This walks the real `_vault/_trash/` and
    /// fails on any block the ledger does not name.
    ///
    /// A `RESTORE.md` beside the husk does NOT satisfy it — the receipt's home
    /// is compiled (that is the whole rule), so a prose file reads as a gap.
    #[test]
    fn every_trash_block_has_a_compiled_receipt() {
        let trash = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../_vault/_trash");
        let Ok(dir) = std::fs::read_dir(&trash) else {
            // No trash on this machine is a pass — nothing moved, nothing owed.
            return;
        };
        let receipted = receipted_trash_blocks();
        let mut orphans: Vec<String> = Vec::new(); // @forge:allow_alloc test path
        for entry in dir.flatten() {
            if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string(); // @forge:allow_alloc test path
            if !receipted.iter().any(|b| *b == name) {
                orphans.push(name);
            }
        }
        assert!(
            orphans.is_empty(),
            "husk block(s) moved to _vault/_trash with no compiled row in session_drain.rs: {orphans:?} \
             — add the row (receipt must contain `_vault/_trash/<block>/`), never a RESTORE.md"
        );
    }

    /// The 08-02 session ledger: ten landed seams, each with a green receipt,
    /// plus ONE Study row — the 8-slot pinch is named, not claimed as fixed.
    ///
    /// The GPL-tree delete lands HERE rather than in a `session_2026_08_02_delete()` of its
    /// own (Sean 08-02, "why is this a verb?"): the file already carries 12 date-stamped
    /// functions, each a hardcoded `vec![]`, and a 13th on a date that already had one is
    /// the convention admitting it is a database wearing a function's clothes.
    #[test]
    fn the_08_02_session_is_receipted_and_honest_about_what_is_unfixed() {
        let s = session_2026_08_02();
        // 13 <- 12 (2026-08-03: the source-compiler SPLIT husk row — the block was
        // trashed 08-02 and the gauge added the same session went red against it).
        assert_eq!(s.len(), 13, "twelve landed seams (+ TS-oracle partial drain 08-02) + the named pinch");
        assert!(s.iter().all(|c| !c.receipt.is_empty()), "a row without a receipt is [UNPROVEN]");
        let study = s.iter().filter(|c| c.status == CapabilityStatus::Study).count();
        assert_eq!(study, 1, "the 8-slot Palette pinch stays STUDY until the ladders parse");
        // Every proven row must name a file, not a feeling.
        for cap in s.iter().filter(|c| c.status == CapabilityStatus::Proven) {
            assert!(
                cap.receipt.contains("crates/") || cap.receipt.contains(".forge/"),
                "row must point at disk: {}",
                cap.receipt
            );
        }
    }

    #[test]
    fn the_ledger_finds_the_blocks_its_own_receipts_name() {
        let blocks = receipted_trash_blocks();
        assert!(blocks.len() >= 8, "the ledger already accounts for 8+ husk blocks, found {}", blocks.len());
        assert!(blocks.iter().any(|b| b == "2026-07-30-goldminer-husk"), "{blocks:?}");
        assert!(blocks.iter().all(|b| !b.is_empty()));
    }

    // [BOARD:session-drain-2026-07-23]
    #[test]
    fn drain_is_28_crates_all_proven() {
        let d = session_2026_07_23_drain();
        assert_eq!(d.len(), 28, "the session merged 28 crates green");
        assert!(d.iter().all(|c| c.status == CapabilityStatus::Proven), "every drain row is a PROVEN merge (cargo-test-verified)");
        assert!(d.iter().all(|c| !c.receipt.is_empty()), "every row carries an on-disk receipt");
    }

    #[test]
    fn ironroot_fold_rows_are_proven_and_receipted() {
        let f = session_2026_07_28_ironroot_fold();
        assert_eq!(f.len(), 4, "the fold landed four proven claims");
        assert!(f.iter().all(|c| c.status == CapabilityStatus::Proven));
        assert!(f.iter().all(|c| !c.receipt.is_empty()), "a fold row without a receipt is [UNPROVEN]");
    }

    #[test]
    fn ironroot_fold_chapter_carries_every_row() {
        let ch = ironroot_fold_chapter();
        // 1 header lore + 4 fold rows.
        assert_eq!(ch.lore_count(), 5);
    }

    #[test]
    fn ironroot_fold_names_the_one_home_not_the_husk() {
        // The fold's whole point: forge-game-systems is the single home. Any row
        // that cites only the vault path is a husk story, not a capability.
        for cap in session_2026_07_28_ironroot_fold() {
            assert!(
                cap.receipt.contains("forge-game-systems") || cap.receipt.contains("ironroot/src"),
                "row must point at a live source seam: {}",
                cap.receipt
            );
        }
    }

    #[test]
    fn ghost_pull_moves_husks_and_keeps_the_empty_shelf_loud() {
        let p = session_2026_07_29_ghost_pull();
        assert_eq!(p.len(), 7, "five husk/keep proofs + the clean-storefront proof + the shelf gauge");
        // The husk rows are proofs of absence; the shelf row is the drift, and it must
        // NOT read Proven — a law file that disagrees with disk is unproven by definition.
        let unproven = p.iter().filter(|c| c.status != CapabilityStatus::Proven).count();
        assert_eq!(unproven, 1, "the zero-SKU/law drift row stays UNPROVEN until Sean lands both halves");
        assert!(p.iter().all(|c| !c.receipt.is_empty()), "no row without a receipt");
        // Every husk row names where it went, so restore never needs a prose file.
        for cap in p.iter().filter(|c| c.status == CapabilityStatus::Proven) {
            assert!(
                cap.receipt.contains("sites/13forge-site"),
                "row must point at the live tree it left: {}",
                cap.receipt
            );
        }
    }

    #[test]
    fn ghost_pull_chapter_carries_every_row() {
        // 1 header lore + 7 rows.
        assert_eq!(ghost_pull_chapter().lore_count(), 8);
    }

    #[test]
    fn drain_chapter_carries_every_crate() {
        let ch = drain_chapter();
        // 1 header lore + 28 crate rows.
        assert_eq!(ch.lore_count(), 29);
    }
}
