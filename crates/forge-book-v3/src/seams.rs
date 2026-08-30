//! Cross-domain SEAMS — id in a code comment resolves to its anchors.
//! Drained from staged-integration/seams.md 2026-07-31; DRAIN CLOSED 2026-08-04 —
//! the prose ledger is the husk `_attic/seams-md-drained-2026-08-04/seams.md`, and
//! the `#[ignore]`d prose parser that read it is gone. This file is the only seam truth.
//!
//! [`resolve_parts`] is also the board's green primitive (`board_sync::state_of`):
//! ONE anchor resolver, two consumers.

/// `Proven`/`DesignTarget` anchors MUST be on disk. `Owed` is declared-but-unwired:
/// no anchor to protect yet, so the resolution gate skips it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeamStatus {
    /// Seam is proven with anchors verified against disk.
    Proven,
    /// Seam is designed but not yet fully wired or proven.
    DesignTarget,
    /// Seam is declared but has no anchors to protect.
    Owed,
}

/// A symbol a grep can find, and the file it lives in.
#[derive(Clone, Copy, Debug)]
pub struct Anchor {
    /// The symbol name that a grep search can locate.
    pub symbol: &'static str,
    /// The file path containing the symbol.
    pub file: &'static str,
}

/// A wiring seam connecting two or more sides of the system.
#[derive(Clone, Copy, Debug)]
pub struct Seam {
    /// Unique identifier for this seam (e.g., "G-LAW-01").
    pub id: &'static str,
    /// Human-readable description of what the seam joins.
    pub name: &'static str,
    /// Anchors on each side of the seam, which must resolve on disk if wired.
    pub sides: &'static [Anchor],
    /// Current status of the seam (Proven, DesignTarget, or Owed).
    pub status: SeamStatus,
}

/// All registered seams in the system, their anchors, and their wiring status.
pub const SEAMS: &[Seam] = &[
    Seam {
        id: "G-LAW-01",
        name: "Skill prose joins compiled law — the massloop fold",
        sides: &[
            Anchor { symbol: "MASSLOOP_EXECUTION_LAW", file: "crates/forge-book-v3/src/oracle1_governor.rs" },
            Anchor { symbol: "MASSLOOP_MILESTONES", file: "crates/forge-book-v3/src/oracle1_governor.rs" },
            Anchor { symbol: "VERIFIED_TABLES", file: "crates/forge-book-v3/src/claims.rs" },
            Anchor { symbol: "ZERO_STRUCT_IS_DEFAULT", file: "crates/forge-vix-v3/src/layout.rs" },
            Anchor { symbol: "MASSLOOP_ROW_LIES", file: "crates/forge-core-v3/src/organs/massread.rs" },
        ],
        // 2026-08-04 (Sean: "session_drain.rs is a fucking cop out, make it read only").
        // This began as a narrative paragraph in `session_drain` and is a SEAM instead,
        // because a seam's anchors are proven against disk by `every_wired_seam_anchor_is
        // _on_disk` and a paragraph is proven by nobody. Two sides: the compiled tables in
        // `oracle1_governor`, and the consumers that used to cite the skill file by name.
        // The husk is `_attic/massloop-skill-folded-2026-08-04/SKILL.md`, sha256
        // 4D06833A7307B74337DFAED949FB812FC96C0987FA654E39CAB7E745EA2F7627, 20,952B.
        //
        // NAMED, NOT BURIED: the fold inverted `DELETE_FIRST` — the replacement was
        // compiled before the skill was attic'd. The law had no teeth here (the legacy
        // path was prose the compiler cannot see), but the inversion was real, and the
        // DELETE CLAMP later refused the `session_drain` removal until this row existed,
        // which is the same law arriving with teeth from the other direction.
        status: SeamStatus::Proven,
    },
    Seam {
        id: "G-PLAT-01",
        name: "Two Clocks",
        sides: &[
            // The grid.
            Anchor { symbol: "TICK_HZ", file: "crates/forge-hal-clockspine/src/metronome.rs" },
            // The host that steps it.
            Anchor { symbol: "TickAccumulator", file: "shell/src/main.rs" },
        ],
        // 2026-08-26: the OWED half closed. v2's `clock_step` never reached
        // forge-gpu-v3; the v3 step landed inline in shell and was extracted
        // to `TickAccumulator`, beside the clock whose grid it divides.
        status: SeamStatus::Proven,
    },
    Seam {
        id: "G-PAINT-04",
        name: "Control joins see and hear — sound_hover with spring_in",
        sides: &[
            Anchor { symbol: "sound_hover", file: "crates/forge-ast-v3/src/vixel/grammar_bridge.rs" },
            Anchor { symbol: "spring_in", file: "crates/forge-ast-v3/src/vixel/grammar_bridge.rs" },
            Anchor { symbol: "step_door_lift", file: "crates/forge-studio-v3/src/dual_loop.rs" },
        ],
        // 2026-08-04: MOTION half wired. Hover was never the missing piece — it is
        // live in `forge_canvas::input::interact`. The dead side was
        // `IntegerSpring::from_spring_def_parts`, written for the authored
        // `spring_in { stiffness damping }` handoff and called by nothing, while the
        // door's "resonance spring" was a ±1200 linear ramp. `step_door_lift` is its
        // first live caller. STILL DESIGN_TARGET for the SOUND half in v2: `sound_hover`
        // parses and no lane reads it — nothing turns a hover into a note.
        // 2026-08-19: forge-ast-v3 landed (verbatim port of v2 forge-ast/src/vixel,
        // sound_hover:938 + spring_in:954 in grammar_bridge.rs, both live on disk).
        // forge-studio-v3/dual_loop.rs::step_door_lift is still absent — SeamStatus
        // has no partial state, so this stays Owed until that side lands too.
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-PLAT-04",
        name: "Single writer: only the DET thread may advance MetronomeClock",
        sides: &[Anchor { symbol: "advance", file: "crates/forge-hal-v3/src/metronome.rs" }],
        status: SeamStatus::Owed,
    },
    Seam { id: "G-PLAT-02", name: "WDA_NONE — capture must not false-green", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-PLAT-03", name: "resize with get_current_texture Outdated retry", sides: &[], status: SeamStatus::Owed },
    // 2026-08-04: "one Permyriad" was two. `rms_to_permyriad` had test-only callers
    // while the studio's star field open-coded its own dB→permyriad map at a different
    // floor. `db_to_permyriad` is now the single arithmetic, the floor stays the
    // surface's authored choice, and the live caller is `audio_lifted_intensity_q`.
    Seam { id: "G-AUDIO-02", name: "One Permyriad through three domains", sides: &[
        Anchor { symbol: "db_to_permyriad", file: "crates/forge-audio-v3/src/metering.rs" },
        // constellation_kit.rs verified absent in v3 (forge-studio-v3 crate does not exist) 2026-08-17.
        Anchor { symbol: "audio_lifted_intensity_q", file: "crates/forge-studio-v3/src/constellation_kit.rs" },
    ], status: SeamStatus::Owed },
    // 2026-08-04 reconcile: the hub side got BUILT — SetPan/ToggleMute/ToggleSolo
    // land in apply_command and kit_bridge resolves strip bindings (both doc-tagged
    // "G-AUDIO-03 wire" in prod), and the strip kit landed in forge-vix-v3/panels/.
    // apply_kit_binding still has zero callers outside forge-audio-v3 — half-wired:
    // anchors now guard the built sides until a live lane drives them.
    Seam { id: "G-AUDIO-03", name: "MixerCommandHub with mixer_channel_strip kit", sides: &[
        Anchor { symbol: "apply_kit_binding", file: "crates/forge-audio-v3/src/bus/kit_bridge.rs" },
        // mixer_channel_strip.kit.vixi verified absent in v3 (forge-vix-v3/panels/ has no such kit) 2026-08-17.
        Anchor { symbol: "mixer_channel_strip", file: "crates/forge-vix-v3/panels/mixer_channel_strip.kit.vixi" },
    ], status: SeamStatus::Owed },
    Seam { id: "G-AUDIO-04", name: "heal_voice with EDL timeline with mp4 egress", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-AUDIO-05", name: "termithesia glyph grid with UMP (x=time, y=pitch)", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-GAME-02", name: "quad_lane semantic trigger with GPU particle spawn", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-PAINT-01", name: "brush_engine with material audio feedback", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-PAINT-02", name: "stamp_acrylic with MusicSieve::on_diff with export_png", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-PAINT-05", name: "DockPanel::side with panel.toggle (space-time-space)", sides: &[], status: SeamStatus::Owed },
    // 2026-08-04 reconcile pass: rows the 07-31 drain left behind in staged-integration/
    // (verified-accurate wiring specs with no registry row), so the owed work lives
    // here instead of a dead folder. Anchors arrive when a side gets built.
    Seam { id: "G-PAINT-03", name: "3D mesh paint: viewport-host ray math with stamp_acrylic UV bridge", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-GAME-01", name: "CartridgeDef inspector: level_editor_panel edits what the tick reads", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-GAME-03", name: "import_glb_to_mesh with supermaxatom camera auto-frame", sides: &[], status: SeamStatus::Owed },
    Seam { id: "G-GAME-04", name: "Lorekeeper retrieval with cross-link graph browser (forge-lore unverified)", sides: &[], status: SeamStatus::Owed },
    Seam { id: "C-02", name: "One draw vocabulary: forge-anim DrawCommand folds into DrawList (ARCH-002)", sides: &[], status: SeamStatus::Owed },
    // 2026-08-04: S22's backend never vanished — the crate folded into
    // forge_audio::sovereign_comms on 07-10 (loopback-only defaults, crypto behind
    // the sovereign-broadcast feature). Panel and module are both on disk; no lane
    // connects lobby bindings to NostrClient yet.
    Seam { id: "S22", name: "multiplayer lobby with forge_audio::sovereign_comms (Nostr+WebRTC)", sides: &[
        Anchor { symbol: "sovereign_comms", file: "crates/forge-audio-v3/src/lib.rs" },
        // multiplayer_lobby_panel.kit.vixi verified absent in v3 2026-08-17.
        Anchor { symbol: "multiplayer_lobby_panel", file: "crates/forge-vix-v3/panels/multiplayer_lobby_panel.kit.vixi" },
    ], status: SeamStatus::Owed },
    Seam {
        id: "G-VIS-01",
        name: "Double gate: colour AND structure",
        sides: &[
            // render_gate.rs verified absent in v3 (forge-vision-v3/src has only lib.rs,
            // contour.rs, edges.rs, mod.rs) 2026-08-17.
            Anchor { symbol: "colour_check", file: "crates/forge-vision-v3/src/render_gate.rs" },
            Anchor { symbol: "confirm_pixels", file: "crates/forge-vision-v3/src/render_gate.rs" },
        ],
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-SND-01",
        name: "The painting IS the music",
        sides: &[
            Anchor { symbol: "on_diff", file: "crates/forge-core-v3/src/music_sieve.rs" },
            Anchor { symbol: "AcousticProfile", file: "crates/forge-core-v3/src/music_sieve.rs" },
        ],
        status: SeamStatus::Proven,
    },
    Seam {
        id: "G-COL-01",
        name: "One transfer function",
        sides: &[
            // Neither side verified: correspondence.rs absent from forge-core-v3/src, and
            // forge-gpu-v3 does not exist in v3 at all 2026-08-17.
            Anchor { symbol: "srgb_to_linear", file: "crates/forge-core-v3/src/correspondence.rs" },
            Anchor { symbol: "srgb_to_linear_ch", file: "crates/forge-gpu-v3/src/canvas_quad.wgsl" },
        ],
        status: SeamStatus::Owed,
    },
    // ── 2026-08-04 DRAIN CLOSE: the last six seams.md rows with no registry row. ──
    // The four PROVEN ones below carry anchors verified against disk this pass; the two
    // HYPOTHESIS rows carry none, which is what Owed means — declared, nothing to protect.
    Seam {
        id: "G-GAME-05",
        name: "The seed IS the geometry — WorldGen determinism under a paint brush",
        sides: &[
            // forge-tile-crawler-v3 verified absent in v3 2026-08-17.
            Anchor { symbol: "WorldGen", file: "crates/forge-tile-crawler-v3/src/lib.rs" },
            Anchor { symbol: "tile_biome", file: "crates/forge-tile-crawler-v3/src/lib.rs" },
        ],
        // seams.md called this PROVEN on the worldgen property tests, and the generator
        // half is: same seed, same world. The brush half (a painted override on the
        // generated grid) has no lane, so the row is a design target in v2 — and OWED
        // outright in v3, since forge-tile-crawler-v3 has not landed yet either.
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-COL-02",
        name: "One TokenSheet cascade — every rendered colour resolves through one sheet",
        sides: &[
            Anchor { symbol: "TokenSheet", file: "crates/forge-canvas-v3/src/tokens.rs" },
            Anchor { symbol: "pub fn resolve", file: "crates/forge-canvas-v3/src/tokens.rs" },
        ],
        // The cascade resolves (tokens.rs:335, base + overlays). The `hot_reload` side
        // seams.md names does not exist in forge-canvas-v3 — grep-absent 08-04 — so a global
        // colour swap is still a design target and the row says so instead of claiming it.
        status: SeamStatus::DesignTarget,
    },
    Seam {
        id: "G-GAME-06",
        name: "Quest time IS DAG space — advance_objective moves the highlighted node",
        sides: &[
            // forge-game-systems-v3 verified absent in v3 2026-08-17.
            Anchor { symbol: "advance_objective", file: "crates/forge-game-systems-v3/src/quest/manager.rs" },
            Anchor { symbol: "QuestManager", file: "crates/forge-game-systems-v3/src/quest/manager.rs" },
        ],
        // Temporal half proven by QuestManager's tests in v2. No visual DAG traversal
        // reads it, and forge-game-systems-v3 has no v3 port yet either — Owed.
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-VIS-02",
        name: "Measurement IS the CI signal — run_gate under daemon dispatch",
        sides: &[
            // Neither side verified: forge-render-gate-v3 does not exist, and
            // forge-daemon-door/src has no gate.rs, 2026-08-17.
            Anchor { symbol: "run_gate", file: "crates/forge-render-gate-v3/src/lib.rs" },
            Anchor { symbol: "run_render_gate", file: "crates/forge-daemon-door/src/gate.rs" },
        ],
        // Both sides live and wired in v2: the daemon's visual-claim strike calls the
        // gate. Neither has landed in v3 yet — Owed.
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-LAW-02",
        name: "Green joins the seam law — the board's stored-green fold",
        sides: &[
            Anchor { symbol: "pub fn state_of_task", file: "crates/forge-book-v3/src/board_sync.rs" },
            Anchor { symbol: "pub fn resolve_parts", file: "crates/forge-book-v3/src/seams.rs" },
            Anchor { symbol: "pub fn anchor_census", file: "crates/forge-book-v3/src/board_sync.rs" },
        ],
        // 2026-08-04 (Sean: "green becomes computed, never stored"). Two sides of ONE law
        // that had drifted into two answers: this registry has refused an unanchored claim
        // since 07-31 (`a_wired_seam_carries_at_least_one_anchor`) while the board six files
        // away accepted 287 of them, because a `true` in `.forge/board_status.json` WAS the
        // verdict. `state_of_task` now derives it — passing tagged test AND >=1 anchor
        // resolving through `resolve_parts`, the SAME disk probe a seam rides. No anchor is
        // LEGACY, a drained anchor is RED. `board_status.json` is demoted to measurement.
        //
        // The `board flip` verb died with it: a hand verdict laid over the harvest was a
        // SECOND truth about one board, and the file it wrote is the husk
        // `_attic/board-flips-killed-2026-08-04/board_flips.tsv`, sha256
        // BFB7077AD630C928B86E7E7D17A0A7FC6B3892FCBC85F7424CD52B9792189506, 296B, ONE row.
        status: SeamStatus::Proven,
    },
    Seam {
        id: "G-LAW-03",
        name: "WHO joins the OS — attribution from the process token",
        sides: &[
            Anchor { symbol: "pub fn whoami", file: "crates/forge-book-v3/src/actor.rs" },
            Anchor { symbol: "OpenProcessToken", file: "crates/forge-book-v3/src/actor.rs" },
            // gate.rs verified absent from forge-daemon-door/src in v3 2026-08-17.
            Anchor { symbol: "countersign_on", file: "crates/forge-daemon-door/src/gate.rs" },
        ],
        // 2026-08-04, forced by the one row `board flip` ever wrote: `CDK-TRIAD GREEN seanm`,
        // authored by an agent. `who` came from `std::env::var("USERNAME")` — an environment
        // variable, settable by any process, read inside Sean's own logon session. The audit
        // trail could not distinguish Sean from the agent, so it named the wrong one.
        //
        // `whoami` reads the user SID out of the process token instead, which no env var
        // reaches; the ForgeAgent/LOTO account split is then what makes the SID discriminate.
        // And a token proves which ACCOUNT ran, never whose DECISION it was — `Attribution::of`
        // needs a LIVE dated `[SEAN-OK YYYY-MM-DD]`, the same rule `gate::countersign_on`
        // enforces at the hook door in v2. T2 signature or it wasn't you. The daemon
        // door half has not landed in v3 yet — Owed, not Proven, until it does.
        status: SeamStatus::Owed,
    },
    Seam {
        id: "G-PAINT-06",
        name: "One sparkline pipeline: paint-stroke with MusicSieve with level_glyph VU",
        sides: &[],
        status: SeamStatus::Owed,
    },
    Seam {
        id: "C-03",
        name: "One purge algorithm: project-asset GC with repo diamond-election drain",
        sides: &[],
        status: SeamStatus::Owed,
    },
];

/// Look up a seam by its unique identifier.
pub fn by_id(id: &str) -> Option<&'static Seam> {
    SEAMS.iter().find(|s| s.id == id)
}

/// Seams touching a file — the query a drain pass owes before it cuts.
pub fn touching(file: &str) -> Vec<&'static Seam> {
    SEAMS.iter().filter(|s| s.sides.iter().any(|a| a.file.ends_with(file) || file.ends_with(a.file))).collect()
}

/// A single-line summary of all seams and their half-wired status, suitable for cold-start cost.
pub fn blast_line() -> String {
    let held: Vec<&str> = SEAMS
        .iter()
        .filter(|s| s.status == SeamStatus::DesignTarget)
        .map(|s| s.id)
        .collect();
    format!("SEAMS     {} bound · half-wired (never drain): {}", SEAMS.len(), held.join(" "))
}

/// The repo root, reached from this crate's manifest dir.
///
/// Promoted out of `mod tests` 2026-08-04: `crate::claims` resolves the SAME class of
/// claim (a doctrine string naming a file or a symbol) and a second root-finder would be
/// a second truth about where the repo starts.
pub fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What an anchor lookup actually found. Sean 2026-08-02: `-> bool` collapsed three
/// distinct states into one `false`, so a drained seam and a moved file read alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnchorState {
    /// Symbol exists in the file.
    Present,
    /// File exists but the symbol was not found in it.
    SymbolAbsent,
    /// File does not exist on disk.
    FileAbsent,
}

/// Resolve one anchor against disk. Public since 08-04 — this is the primitive the
/// doctrine-claim gate rides, and it was already the only tested one in the crate.
pub fn resolve(a: &Anchor) -> AnchorState {
    resolve_parts(a.symbol, a.file)
}

/// [`resolve`] over borrowed parts, for anchors that are NOT `&'static` — the board's
/// rows carry `String` anchors loaded from `.forge/board_tasks.json`.
///
/// Split out 2026-08-04 so `board_sync::state_of` computes GREEN through the SAME disk
/// probe a seam does. A second resolver would be a second answer to "is this real".
pub fn resolve_parts(symbol: &str, file: &str) -> AnchorState {
    match std::fs::read_to_string(root().join(file)) {
        Err(_) => AnchorState::FileAbsent,
        Ok(t) if t.contains(symbol) => AnchorState::Present,
        Ok(_) => AnchorState::SymbolAbsent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_wired_seam_anchor_is_on_disk() {
        let gone: Vec<String> = SEAMS
            .iter()
            .filter(|s| s.status != SeamStatus::Owed)
            .flat_map(|s| {
                s.sides.iter().filter_map(move |a| match resolve(a) {
                    AnchorState::Present => None,
                    AnchorState::SymbolAbsent => {
                        Some(format!("{} :: {} DRAINED from {}", s.id, a.symbol, a.file))
                    }
                    AnchorState::FileAbsent => {
                        Some(format!("{} :: {} FILE GONE {}", s.id, a.symbol, a.file))
                    }
                })
            })
            .collect();
        assert!(gone.is_empty(), "seam anchors drained:\n  {}", gone.join("\n  "));
    }

    #[test]
    fn a_wired_seam_carries_at_least_one_anchor() {
        for s in SEAMS.iter().filter(|s| s.status != SeamStatus::Owed) {
            assert!(!s.sides.is_empty(), "{} claims wired with no anchor", s.id);
        }
    }

    #[test]
    fn a_drained_anchor_goes_red() {
        const DRAINED: Anchor =
            Anchor { symbol: "ticks_this_frame_REMOVED", file: "crates/forge-gpu-v3/src/sovereign_window.rs" };
        let hit = std::fs::read_to_string(root().join(DRAINED.file))
            .map(|t| t.contains(DRAINED.symbol))
            .unwrap_or(false);
        assert!(!hit, "control symbol must not exist");
    }

    #[test]
    fn the_id_in_a_code_comment_resolves() {
        assert_eq!(by_id("G-PLAT-01").map(|s| s.name), Some("Two Clocks"));
        assert!(by_id("NOPE").is_none());
    }

    /// The blast is paid for on EVERY cold start, so the seam row is capped. Ids only:
    /// a reader who wants the law calls `by_id`. Growth past this means the row started
    /// carrying prose instead of pointers.
    #[test]
    fn blast_row_stays_one_line() {
        let line = blast_line();
        eprintln!("BLAST-COST {} bytes: {line}", line.len());
        assert!(line.len() <= 160, "seam blast row is {} bytes, cap 160", line.len());
        assert!(!line.contains('\n'), "one line");
    }

    /// `touching` resolves a seam from a path it anchors. G-PLAT-01 used to be
    /// findable only by a dead placeholder path; both its anchors are live now.
    #[test]
    fn touching_finds_the_seam_before_a_drain() {
        let ids: Vec<&str> = touching("shell/src/main.rs").iter().map(|s| s.id).collect();
        assert!(ids.contains(&"G-PLAT-01"), "the host side must resolve the seam");
        let ids: Vec<&str> = touching("crates/forge-hal-clockspine/src/metronome.rs")
            .iter()
            .map(|s| s.id)
            .collect();
        assert!(ids.contains(&"G-PLAT-01"), "the grid side must resolve it too");
        assert!(
            touching("crates/forge-gpu-v3/src/sovereign_window.rs").is_empty(),
            "the dead v2 placeholder must no longer resolve to anything"
        );
    }
}
