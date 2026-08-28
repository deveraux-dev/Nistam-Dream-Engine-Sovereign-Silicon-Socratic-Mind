//! The subprocess-driver half of the witness harness — launches
//! `studio-shell.exe`, injects scripted input, and captures frames, all by
//! shelling out to `forgewright.exe` (`forge-wright/src/main.rs`, ported
//! verbatim from v2 `forge-vision::window_driver::inject`). Nothing here
//! touches Win32 directly — that FFI stays in `forge-wright`, its own
//! excluded workspace, one home (L05). This module owns only: which binary
//! to launch, what to inject, when to capture, and what a passing scenario
//! means — pure process orchestration plus calls into [`crate::diff_bmp`]/
//! [`crate::diff_bmp_region`] for the actual pixel verdict.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use crate::{diff_bmp, diff_bmp_region, WitnessError};

/// Owns a launched `studio-shell.exe` child and kills it on EVERY exit path
/// — `Drop`, not a `let _ = child.kill()` at the bottom of the happy path.
/// A scenario that error-returns early (any `?`) would otherwise leave the
/// child alive holding its stdio handles, which hangs any caller piping this
/// process's own output while it waits for an EOF that never comes (the
/// exact hang this crate's first live run hit, 2026-08-15).
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl ChildGuard {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.0.try_wait()
    }
}

/// The window title substring every scenario looks for — `shell/src/
/// main.rs`'s `.with_title("13FORGE-STUDIO V3 :: SOVEREIGN WINDOW")`
/// verbatim prefix, matched case-insensitively by `forgewright::find_hwnd`.
const WINDOW_TITLE: &str = "13FORGE-STUDIO";

/// Win32 virtual-key code for `W` (the ASCII value — the same table
/// `windows-sys`'s `Win32_UI_Input_KeyboardAndMouse` and `forge-wright`'s
/// own `postkey` argument both use).
const VK_W: &str = "0x57";

/// Locates the two binaries a witness run drives, and builds the one it's
/// actually allowed to build.
pub struct WitnessKit {
    root: PathBuf,
    wright: PathBuf,
    shell_exe: PathBuf,
}

impl WitnessKit {
    /// Build `studio-shell` (debug, fast — matches `xtask::photon`'s own
    /// build invocation verbatim) and confirm `forgewright.exe` is already
    /// built. This crate does not build forge-wright itself: that's an
    /// excluded workspace behind its own firewall, and a missing build is a
    /// named blocker here, not a silently-triggered second build reaching
    /// across it.
    pub fn open(root: &Path) -> Result<Self, String> {
        let build = Command::new("cargo")
            .args(["build", "--manifest-path", "shell/Cargo.toml"])
            .current_dir(root)
            .status()
            .map_err(|e| format!("cargo build (shell): {e}"))?;
        if !build.success() {
            return Err("shell build failed — no witness of a red build".into());
        }
        let target_dir = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("shell/target"));
        let shell_exe = target_dir.join("debug/studio-shell.exe");
        if !shell_exe.is_file() {
            return Err(format!("built but missing: {}", shell_exe.display()));
        }
        // Same CARGO_TARGET_DIR override shell_exe above already respects —
        // forge-wright is its OWN excluded workspace with its own default
        // target dir (`forge-wright/target`), but an ambient CARGO_TARGET_DIR
        // redirects every cargo invocation in this shell to the shared
        // `<root>/target`, including forge-wright's (2026-08-15 receipt: the
        // crate-local path held a stale binary while every real build landed
        // here instead, and the mismatch cost a "why won't my fix take"
        // detour). Prefer whichever candidate actually exists.
        let wright_shared = std::env::var_os("CARGO_TARGET_DIR")
            .map(|d| PathBuf::from(d).join("release/forgewright.exe"));
        let wright_local = root.join("forge-wright/target/release/forgewright.exe");
        let wright = match wright_shared {
            Some(p) if p.is_file() => p,
            _ if wright_local.is_file() => wright_local,
            _ => {
                return Err(format!(
                    "forgewright.exe not found at {} or {} — build it: cargo build --release \
                     --manifest-path forge-wright/Cargo.toml --target-dir forge-wright/target \
                     (the explicit --target-dir matters: cargo's config discovery follows the \
                     CALLER's CWD, not --manifest-path, so running this from the repo root without \
                     it lands the binary in the shared F:/v3/target instead, where the housekeeping \
                     hook's periodic `cargo clean --release` collateral-deletes it)",
                    wright_local.display(),
                    root.join("target/release/forgewright.exe").display()
                ));
            }
        };
        Ok(Self { root: root.to_path_buf(), wright, shell_exe })
    }

    /// Launch studio-shell with the given extra environment variables, its
    /// stdio redirected to null (never inherited — an inherited handle held
    /// open by this child is exactly what hangs a caller piping this
    /// process's own output). The returned [`ChildGuard`] kills the process
    /// on drop; this never blocks waiting for paint (callers sleep the
    /// amount their scenario needs).
    fn launch(&self, env: &[(&str, &str)]) -> Result<ChildGuard, String> {
        let mut cmd = Command::new(&self.shell_exe);
        cmd.current_dir(&self.root);
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        for (k, v) in env {
            cmd.env(k, v);
        }
        cmd.spawn().map(ChildGuard).map_err(|e| format!("spawn {}: {e}", self.shell_exe.display()))
    }

    /// Capture the shell window's client area to `out` (BMP) via
    /// `forgewright capture`, and return the raw bytes for diffing.
    fn capture(&self, out: &Path) -> Result<Vec<u8>, String> {
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        let status = Command::new(&self.wright)
            .args(["capture", WINDOW_TITLE, &out.display().to_string()])
            .status()
            .map_err(|e| format!("forgewright capture: {e}"))?;
        if !status.success() {
            return Err(format!("forgewright capture exited {status} — is the window open?"));
        }
        std::fs::read(out).map_err(|e| format!("read {}: {e}", out.display()))
    }

    /// Tap one key (down+up, ~15ms apart) via `forgewright postkey` —
    /// `PostMessageW`, reaches native `WindowEvent::KeyboardInput` (the
    /// WASD bit-tracking path `shell/src/main.rs` reads).
    fn postkey(&self, vk_hex: &str) -> Result<(), String> {
        let status = Command::new(&self.wright)
            .args(["postkey", WINDOW_TITLE, vk_hex])
            .status()
            .map_err(|e| format!("forgewright postkey: {e}"))?;
        if !status.success() {
            return Err(format!("forgewright postkey exited {status} — is the window open?"));
        }
        Ok(())
    }
}

/// A named, runnable scenario. Each launches its OWN fresh shell instance —
/// scenarios never share a window, so one scenario's state can never leak
/// into the next.
pub trait Scenario {
    /// The name `foreman witness <name>` and the baseline file both use.
    fn name(&self) -> &'static str;
    /// Run against a freshly-launched shell; return the final captured
    /// frame's raw BMP bytes (for the baseline regression check) plus a
    /// human receipt line.
    fn run(&self, kit: &WitnessKit) -> Result<(Vec<u8>, String), String>;
}

/// WASD scenario: tap W, prove the player marker's pixels actually moved.
/// Whole-frame diff, deliberately not region-targeted — this crate does not
/// assume a fixed window size the way the sprite region below safely can.
pub struct WasdTranslation;

impl Scenario for WasdTranslation {
    fn name(&self) -> &'static str {
        "wasd-translation"
    }

    fn run(&self, kit: &WitnessKit) -> Result<(Vec<u8>, String), String> {
        let mut child = kit.launch(&[])?;
        std::thread::sleep(Duration::from_millis(2500));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("studio-shell exited before capture: {status}"));
        }
        let before = kit.capture(&kit.root.join(".forge/witness/.tmp/wasd-before.bmp"))?;
        kit.postkey(VK_W)?;
        std::thread::sleep(Duration::from_millis(200));
        let after = kit.capture(&kit.root.join(".forge/witness/.tmp/wasd-after.bmp"))?;
        // `child` drops here (guard kills it) whether we return Ok or the
        // `?`s above already bailed — no explicit kill needed either way.

        let report = diff_bmp(&before, &after, crate::DEFAULT_TOLERANCE).map_err(|e| e.to_string())?;
        // A floor, not a fraction: real translation moves a solid-colour
        // 36x36px marker (`gpu.rs::PLAYER_DRAW`) well past 20 pixels even at
        // one tick's worth of motion; antialias/GPU-driver jitter alone is
        // single-digit, so this floor is unreachable by noise.
        const MOVED_FLOOR: u64 = 20;
        if report.differing_pixels < MOVED_FLOOR {
            return Err(format!(
                "wasd-translation: only {} pixels changed (need >={MOVED_FLOOR}) — W did not move the player marker",
                report.differing_pixels
            ));
        }
        Ok((
            after,
            format!(
                "wasd-translation: W moved {} of {} pixels — camera/player translated",
                report.differing_pixels, report.total_pixels
            ),
        ))
    }
}

/// HUD LZ-field scenario: launch with `FORGE_DEV_HUD=1`, prove the HUD row's
/// screen band is not a flat wash — i.e. `hud.rs` actually painted glyphs,
/// including the `LZ <n>` field (`HudLine::layer_z`, `hud.rs:188`).
pub struct HudLzField;

impl Scenario for HudLzField {
    fn name(&self) -> &'static str {
        "hud-lz-field"
    }

    fn run(&self, kit: &WitnessKit) -> Result<(Vec<u8>, String), String> {
        let mut child = kit.launch(&[("FORGE_DEV_HUD", "1")])?;
        std::thread::sleep(Duration::from_millis(2500));
        // Poll-until-painted (2026-08-17): a single early capture raced the
        // shell's warm-up — the dev-profile binary this kit builds takes well
        // past 2.5s to publish its first HUD-bearing sky plane (measured live:
        // the row paints steadily once warm; the old one-shot capture at 2.5s
        // was reading a frame from before the first publish). A scenario must
        // not fail a working face for being slow to wake, and must not wait
        // forever for a broken one: up to 10 attempts, 2s apart.
        const ATTEMPTS: u32 = 10;
        let mut last_err = String::new();
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(2000));
            }
            if let Ok(Some(status)) = child.try_wait() {
                return Err(format!("studio-shell exited before capture: {status}"));
            }
            let frame_bytes = kit.capture(&kit.root.join(".forge/witness/.tmp/hud-lz.bmp"))?;

            let (w, h) = bmp_dimensions(&frame_bytes)?;
            if h < 120 {
                return Err(format!("hud-lz-field: captured frame too short ({h}) to hold a HUD band"));
            }
            // The HUD row seats at `pane_h - HUD_ROW_H(12) - 18`, composition
            // scale 2, against the window's own bottom edge (`main.rs`: "the
            // HUD row seats at the pane's bottom — which is the window's
            // bottom"). That's a ~(18+12)*2=60px gap then a 24px row; a 100px
            // band from the bottom covers it with margin for window-chrome this
            // crate does not assume away.
            const BAND_H: u32 = 100;
            let band_h = BAND_H.min(h);
            let rect = (0, h - band_h, w, band_h);
            // Diff the band against itself with the top-left pixel's colour as
            // baseline — cheap, dependency-free "is this a flat wash" probe: no
            // fixed golden colour exists (the sky is dynamic), so self-diff
            // against one corner is the honest structural check available.
            let corner = solid_bmp_from_corner(&frame_bytes, rect).map_err(|e| e.to_string())?;
            let diff = diff_bmp_region(&corner, &frame_bytes, crate::DEFAULT_TOLERANCE, rect).map_err(|e| e.to_string())?;
            // 0.5% of a 100px-tall band is a low bar deliberately — glyph
            // coverage inside a thin text row is naturally sparse.
            let floor = (rect.2 as u64 * rect.3 as u64) / 200;
            if diff.differing_pixels >= floor {
                return Ok((
                    frame_bytes,
                    format!(
                        "hud-lz-field: HUD band has {} non-background pixels — the row is painted (attempt {})",
                        diff.differing_pixels,
                        attempt + 1
                    ),
                ));
            }
            last_err = format!(
                "hud-lz-field: HUD band has only {} non-background pixels of {} (need >={floor}) after {} attempts over ~{}s — looks like a flat wash, not a painted row",
                diff.differing_pixels,
                diff.total_pixels,
                attempt + 1,
                2 + (attempt + 1) * 2
            );
        }
        Err(last_err)
    }
}

/// Sprite-breathe scenario: prove `organs::SpriteOrgan`'s face (the
/// checkerboard frame flip and/or its breathe glaze, whichever is live)
/// actually changes pixels across real elapsed time. Region-targeted at the
/// sprite's fixed placement (`gpu.rs`: `op(16, 16, 8, 6)`, `SPRITE_SIZE=8`) —
/// a source-literal constant, unlike the player marker's dynamic position.
pub struct SpriteBreathe;

impl Scenario for SpriteBreathe {
    fn name(&self) -> &'static str {
        "sprite-breathe"
    }

    fn run(&self, kit: &WitnessKit) -> Result<(Vec<u8>, String), String> {
        let mut child = kit.launch(&[])?;
        std::thread::sleep(Duration::from_millis(2500));
        if let Ok(Some(status)) = child.try_wait() {
            return Err(format!("studio-shell exited before capture: {status}"));
        }
        let frame0 = kit.capture(&kit.root.join(".forge/witness/.tmp/sprite-0.bmp"))?;
        // 600ms real time at the 120Hz metronome covers well over 30 ticks
        // even under scheduler slack — the scenario's own "frame 30" ask,
        // given generously rather than raced against exact tick timing.
        std::thread::sleep(Duration::from_millis(600));
        let frame30 = kit.capture(&kit.root.join(".forge/witness/.tmp/sprite-30.bmp"))?;

        const SPRITE_RECT: (u32, u32, u32, u32) = (16, 16, 64, 64);
        let report = diff_bmp_region(&frame0, &frame30, crate::DEFAULT_TOLERANCE, SPRITE_RECT).map_err(|e| e.to_string())?;
        const CHANGED_FLOOR: u64 = 8;
        if report.differing_pixels < CHANGED_FLOOR {
            return Err(format!(
                "sprite-breathe: only {} of {} sprite-rect pixels changed (need >={CHANGED_FLOOR}) — the face looks frozen",
                report.differing_pixels, report.total_pixels
            ));
        }
        Ok((
            frame30,
            format!(
                "sprite-breathe: {} of {} sprite-rect pixels changed between frame 0 and ~frame 30",
                report.differing_pixels, report.total_pixels
            ),
        ))
    }
}

/// Read a BMP's pixel dimensions straight from its header (offsets 18/22,
/// little-endian i32 — the standard `BITMAPINFOHEADER` layout `image`'s BMP
/// encoder writes) without a full decode, since only the size is needed here.
fn bmp_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() < 26 || &bytes[0..2] != b"BM" {
        return Err("bmp_dimensions: not a BMP (missing 'BM' magic)".into());
    }
    let w = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
    let h = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
    Ok((w.unsigned_abs(), h.unsigned_abs()))
}

/// Build a same-sized BMP whose whole `rect` is filled with the source
/// frame's own `rect`-top-left-corner pixel colour — the synthetic "flat
/// wash" comparator [`HudLzField`] diffs the real frame against.
fn solid_bmp_from_corner(frame_bytes: &[u8], rect: (u32, u32, u32, u32)) -> Result<Vec<u8>, WitnessError> {
    let img = image::load_from_memory_with_format(frame_bytes, image::ImageFormat::Bmp)
        .map_err(|e| WitnessError::Decode(e.to_string()))?
        .to_rgba8();
    let corner = *img.get_pixel(rect.0, rect.1);
    let mut solid = image::RgbaImage::new(img.width(), img.height());
    for p in solid.pixels_mut() {
        *p = corner;
    }
    let mut out = Vec::new();
    image::codecs::bmp::BmpEncoder::new(&mut out)
        .encode(&solid, img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .map_err(|e| WitnessError::Decode(e.to_string()))?;
    Ok(out)
}

/// All built-in scenarios, in the order `foreman witness --all` runs them.
pub fn all_scenarios() -> Vec<Box<dyn Scenario>> {
    vec![Box::new(WasdTranslation), Box::new(HudLzField), Box::new(SpriteBreathe)]
}

/// Where a scenario's regression baseline BMP lives.
pub fn baseline_path(root: &Path, scenario: &str) -> PathBuf {
    root.join(".forge/witness").join(format!("{scenario}.baseline.bmp"))
}

/// Run one named scenario end to end: the scenario's own relative/structural
/// check, THEN a baseline regression diff of its final frame against
/// `.forge/witness/<name>.baseline.bmp`. First run (no baseline yet) strikes
/// one and reports steady — this crate has no human-review UI to gate a
/// "witnessed" bless the way `xtask::phash`'s SUBORDINATION CLAUSE assumes;
/// `bless=true` re-strikes explicitly instead. Returns the receipt string on
/// success, `Err` (HALT, non-zero exit at the CLI) on either the scenario's
/// own failure or baseline drift beyond the structural ceiling (see the
/// `DRIFT_CEILING_PCT` note below — `tolerance` bounds per-channel delta, the
/// ceiling bounds how much of the frame may move).
pub fn run_named(root: &Path, name: &str, tolerance: u8, bless: bool) -> Result<String, String> {
    let scenario = all_scenarios()
        .into_iter()
        .find(|s| s.name() == name)
        .ok_or_else(|| {
            format!(
                "witness: unknown scenario '{name}' — try one of: {}",
                all_scenarios().iter().map(|s| s.name()).collect::<Vec<_>>().join(", ")
            )
        })?;
    let kit = WitnessKit::open(root)?;
    let (final_bytes, mut receipt) = scenario.run(&kit)?;

    let baseline = baseline_path(root, name);
    if bless || !baseline.is_file() {
        if let Some(parent) = baseline.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
        }
        std::fs::write(&baseline, &final_bytes).map_err(|e| format!("write {}: {e}", baseline.display()))?;
        receipt.push_str(&format!(" | baseline {}", if bless { "re-struck" } else { "struck (first run)" }));
        return Ok(receipt);
    }
    let stored = std::fs::read(&baseline).map_err(|e| format!("read {}: {e}", baseline.display()))?;
    let drift = diff_bmp(&stored, &final_bytes, tolerance).map_err(|e| e.to_string())?;
    // `differing_pixels > 0` was the bug (fixed 2026-08-15, Sean: the gate was
    // RED on an unmodified tree and auto-reverting every render edit, so no
    // render change could land at all). `tolerance` is a PER-CHANNEL bound; it
    // says nothing about how MANY pixels may move, so the old check demanded a
    // full-window screenshot of a LIVE app match a static BMP exactly. Every
    // layer this window draws is animated on a free-running 120 Hz metronome —
    // the sky rotates, the sprite breathes, the sand emerges and shatters, the
    // terminal scrolls. Byte-identical frames are not a property this face can
    // ever have, so the gate's verdict was decided by capture phase, not by the
    // diff: three consecutive runs on IDENTICAL code gave pass, 13023 drifting
    // pixels, and 627 drifting pixels.
    //
    // What a whole-frame baseline CAN honestly catch is a structural
    // regression — a layer that stopped compositing, a black window, a face
    // that lost its ground — which moves a large FRACTION of the frame, not
    // ~1%. So the check becomes a ceiling on that fraction. Measured animation
    // drift on an untouched tree spans 627..13023 of 1_280_000 px (0.05%..1.02%);
    // the ceiling sits at 5%, a ~5x margin over observed animation noise and
    // far under any real layer loss.
    //
    // This deliberately trades exactness for a gate that can be green (T3
    // break-one-rule): a gate that is always red carries no information and,
    // wired to auto-revert, actively destroys work.
    const DRIFT_CEILING_PCT: u64 = 5;
    let drift_ceiling = (drift.total_pixels * DRIFT_CEILING_PCT) / 100;
    if drift.differing_pixels > drift_ceiling {
        return Err(format!(
            "{name}: baseline drift — {}/{} pixels ({}%) exceed the {DRIFT_CEILING_PCT}% structural ceiling at {tolerance}/255 per-channel tolerance vs {} (worst delta {} at {:?}) — this is layer-scale, not animation",
            drift.differing_pixels,
            drift.total_pixels,
            (drift.differing_pixels * 100) / drift.total_pixels.max(1),
            baseline.display(),
            drift.max_channel_delta,
            drift.worst_pixel
        ));
    }
    receipt.push_str(&format!(
        " | baseline steady ({}/{} px drift, under the {DRIFT_CEILING_PCT}% ceiling, tol={tolerance})",
        drift.differing_pixels, drift.total_pixels
    ));
    Ok(receipt)
}
