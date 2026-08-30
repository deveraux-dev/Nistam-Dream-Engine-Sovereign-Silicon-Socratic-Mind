//! ironroot-web — WASM web preview of IRONROOT for deveraux.dev MUD client.
//!
//! V1 contract:
//! - Deterministic: identical Mulberry32 outputs as the canonical Steam build for any seed.
//! - Engine-free: zero dependencies on `forge-game-systems`, `wgpu`, `winit`, etc.
//!   The game crate (`ironroot`) pulls those for desktop; this crate ships a minimal
//!   parallel core that respects the same determinism contract (frozen RNG algorithm,
//!   permyriad integer stats, no combat-critical floats).
//! - Direct 2D canvas painting via web-sys. V2 may swap for `forge-canvas-web` once
//!   the VixiScript-driven HUD pattern is needed.
//!
//! Aesthetic: diffiedahlia preset (violet-black, cyan/magenta/amber, system mono).
//! Zero border-radius per frontend-forge v2.
//!
//! PORT RECEIPT (2026-08-15): ported from `F:\NewRepo\crates\ironroot-web\src\
//! lib.rs`. Logic, names, and test bodies are verbatim. The ONLY delta is doc
//! comments added to public items that had none — v3's workspace lints set
//! `missing_docs = "deny"`, which the v2 crate did not, so a byte-identical
//! copy does not compile here (C06: port, and add only what the lint forces).

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

// ─── Mulberry32 (canonical, frozen) ────────────────────────────────────────
// Mirror of forge-core::seed::Mulberry32. Same constants, same output.
// Verify by running both desktop and WASM through the determinism test.

#[derive(Debug, Clone, Copy)]
struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u64) -> Self {
        Self { state: seed as u32 }
    }
    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut z = self.state;
        z = (z ^ (z >> 15)).wrapping_mul(z | 1);
        z ^= z.wrapping_add((z ^ (z >> 7)).wrapping_mul(z | 61));
        z ^ (z >> 14)
    }
    fn range(&mut self, max: u32) -> u32 {
        if max == 0 { 0 } else { self.next_u32() % max }
    }
}

fn hash3(seed: u64, a: i32, b: i32) -> u64 {
    let mut h = seed;
    h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(a as i64 as u64);
    h = h.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(b as i64 as u64);
    h ^= h >> 33;
    h
}

// ─── Input bits (mirror of forge_game_systems::input::InputBits) ───────────

const INPUT_MOVE_NORTH: u32 = 1 << 4; // MOVE_UP
const INPUT_MOVE_SOUTH: u32 = 1 << 5; // MOVE_DOWN
const INPUT_MOVE_EAST:  u32 = 1 << 1; // MOVE_RIGHT
const INPUT_MOVE_WEST:  u32 = 1 << 0; // MOVE_LEFT
const INPUT_ATTACK:     u32 = 1 << 3;
const INPUT_INTERACT:   u32 = 1 << 9;

// ─── State ─────────────────────────────────────────────────────────────────

/// Minimal MUD state. Permyriad (0-10000) integers for all stats — no floats in logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebMudState {
    /// The run's deterministic master seed.
    pub master_seed: u64,
    /// Ticks advanced since `new`.
    pub tick_count: u64,
    /// Player world-space X (grid cells).
    pub player_x: i32,
    /// Player world-space Y (grid cells).
    pub player_y: i32,
    /// Current HP, permyriad (0-10000).
    pub hp_permyriad: i32,
    /// Current stamina, permyriad (0-10000).
    pub stamina_permyriad: i32,
    /// Input bitmask consumed by the most recent tick.
    pub last_input: u32,
    /// Rolling terminal-style log, capped at 64 lines.
    pub scrollback: Vec<String>,
    /// Whether the Name-Shear signature event has fired this run.
    pub name_shear_fired: bool,
}

impl WebMudState {
    fn new(seed: u64) -> Self {
        let mut s = Self {
            master_seed: seed,
            tick_count: 0,
            player_x: 0,
            player_y: 0,
            hp_permyriad: 10_000,
            stamina_permyriad: 10_000,
            last_input: 0,
            scrollback: Vec::with_capacity(32),
        name_shear_fired: false,
        };
        s.push_line(format!("IRONROOT v0.1 — terminal client engaged. seed={}", seed));
        s.push_line(String::from("Type N S E W to move. ATTACK to strike. LOOK to read."));
        s.push_line(String::new());
        s.push_line(describe_room(seed, 0, 0));
        s
    }

    fn rng(&self) -> Mulberry32 {
        Mulberry32::new(self.master_seed.wrapping_add(self.tick_count))
    }

    fn push_line(&mut self, line: String) {
        self.scrollback.push(line);
        if self.scrollback.len() > 64 {
            let drain_to = self.scrollback.len() - 64;
            self.scrollback.drain(..drain_to);
        }
    }

    fn tick(&mut self, input_bits: u32) {
        self.last_input = input_bits;
        self.tick_count = self.tick_count.saturating_add(1);

        // Movement
        let (mut dx, mut dy) = (0_i32, 0_i32);
        if input_bits & INPUT_MOVE_NORTH != 0 { dy -= 1; }
        if input_bits & INPUT_MOVE_SOUTH != 0 { dy += 1; }
        if input_bits & INPUT_MOVE_EAST  != 0 { dx += 1; }
        if input_bits & INPUT_MOVE_WEST  != 0 { dx -= 1; }
        if dx != 0 || dy != 0 {
            self.player_x = self.player_x.saturating_add(dx);
            self.player_y = self.player_y.saturating_add(dy);
            self.push_line(format!("> move {}", dir_label(dx, dy)));
            self.push_line(describe_room(self.master_seed, self.player_x, self.player_y));
            self.stamina_permyriad = (self.stamina_permyriad - 200).max(0);
        }

        // Attack
        if input_bits & INPUT_ATTACK != 0 {
            let mut rng = self.rng();
            let roll = rng.range(10_000);
            let hit = roll < 6_500; // ~65% hit
            if hit {
                let dmg = 1_200 + rng.range(800) as i32;
                self.push_line(format!("> attack — hit for {} (permyriad).", dmg));
            } else {
                self.push_line(String::from("> attack — miss."));
            }
            self.stamina_permyriad = (self.stamina_permyriad - 500).max(0);
        }

        // Stamina regen
        if dx == 0 && dy == 0 && input_bits & INPUT_ATTACK == 0 {
            self.stamina_permyriad = (self.stamina_permyriad + 50).min(10_000);
        }

        // Name-Shear: signature IRONROOT event. Schedule it deterministically
        // from the master seed so different seeds fire at different ticks.
        let shear_tick = 240 + (hash3(self.master_seed, 0, 0) % 240) as u64;
        if !self.name_shear_fired && self.tick_count >= shear_tick {
            self.name_shear_fired = true;
            self.push_line(String::new());
            self.push_line(String::from(">>> [NAME-SHEAR — SEVERITY: NameRemoved] <<<"));
            self.push_line(String::from("    A relation was removed from record. Faction pressure +1."));
            self.push_line(String::new());
        }
    }
}

fn dir_label(dx: i32, dy: i32) -> &'static str {
    match (dx.signum(), dy.signum()) {
        (0, -1) => "north",
        (0,  1) => "south",
        (1,  0) => "east",
        (-1, 0) => "west",
        _ => "?",
    }
}

// ─── Procedural room descriptions (deterministic on seed + position) ───────

const ATMOSPHERES: &[&str] = &[
    "Ironroot vines crawl across the threshold",
    "A bone-light glow leaks from the walls",
    "Cinders hover in defiance of the floor",
    "Ledger script burns faintly on stone",
    "Frostmetal sings under your boots",
    "The dust here remembers your name",
    "A spouse's grave is somewhere west",
    "Names rise from the floor as steam",
    "Debt-marks scab the doorframe",
    "Shadow-relations linger between the bricks",
];

const FEATURES: &[&str] = &[
    "A ledger-altar squats in the corner",
    "An iron staff lies on the threshold",
    "A grave-stone here has been re-named",
    "A shadow-mirror reflects nothing",
    "A faction sigil scorches the wall",
    "An erased witness still gestures from a chair",
    "A vow scroll lies torn at the centre",
    "A ceremony bell hangs from a single root",
];

fn describe_room(master_seed: u64, x: i32, y: i32) -> String {
    let h = hash3(master_seed, x, y);
    let mut rng = Mulberry32::new(h);
    let atm = ATMOSPHERES[(rng.range(ATMOSPHERES.len() as u32)) as usize];
    let feat = FEATURES[(rng.range(FEATURES.len() as u32)) as usize];
    format!("[ROOM {:+},{:+}] {}. {}.", x, y, atm, feat)
}

// ─── Rendering — diffiedahlia palette, zero border-radius ──────────────────

const BG:        &str = "#0a0512"; // violet-black
const TEXT:      &str = "#d4c7e8"; // off-white violet tint
const DIM:       &str = "#7a6e94"; // dim purple
const CYAN:      &str = "#4dd0e1"; // accent
const MAGENTA:   &str = "#e0399e"; // accent
const AMBER:     &str = "#ffb74d"; // accent (HUD highlights)
const SHEAR_RED: &str = "#ff3030"; // Name-Shear signal
const HP_FILL:   &str = "#c92e58";
const STAM_FILL: &str = "#6ee0b4";

fn paint_state(state: &WebMudState, canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let ctx = canvas
        .get_context("2d")?
        .ok_or_else(|| JsValue::from_str("no 2d context"))?
        .dyn_into::<CanvasRenderingContext2d>()?;

    let w = canvas.width() as f64;
    let h = canvas.height() as f64;

    // Background
    ctx.set_fill_style_str(BG);
    ctx.fill_rect(0.0, 0.0, w, h);

    // Title bar
    ctx.set_fill_style_str(MAGENTA);
    ctx.fill_rect(0.0, 0.0, w, 22.0);
    ctx.set_fill_style_str(BG);
    ctx.set_font("bold 12px 'IBM Plex Mono', monospace");
    ctx.fill_text("IRONROOT — terminal client v0.1", 8.0, 16.0).ok();

    // HUD strip (HP / stamina bars + position)
    let hud_y = 30.0;
    let bar_w = 200.0;
    let bar_h = 10.0;
    // HP
    ctx.set_fill_style_str(DIM);
    ctx.fill_rect(8.0, hud_y, bar_w, bar_h);
    ctx.set_fill_style_str(HP_FILL);
    let hp_frac = (state.hp_permyriad.max(0) as f64) / 10_000.0;
    ctx.fill_rect(8.0, hud_y, bar_w * hp_frac, bar_h);
    // Stamina
    ctx.set_fill_style_str(DIM);
    ctx.fill_rect(8.0, hud_y + bar_h + 4.0, bar_w, bar_h);
    ctx.set_fill_style_str(STAM_FILL);
    let sp_frac = (state.stamina_permyriad.max(0) as f64) / 10_000.0;
    ctx.fill_rect(8.0, hud_y + bar_h + 4.0, bar_w * sp_frac, bar_h);

    // Status text (right side)
    ctx.set_fill_style_str(AMBER);
    ctx.set_font("11px 'IBM Plex Mono', monospace");
    let status = format!(
        "TICK {:>6}  POS {:+},{:+}  HP {:>5}  SP {:>5}",
        state.tick_count, state.player_x, state.player_y,
        state.hp_permyriad, state.stamina_permyriad
    );
    ctx.fill_text(&status, 220.0, hud_y + bar_h).ok();

    // Scrollback area
    ctx.set_font("13px 'IBM Plex Mono', monospace");
    let scroll_top = 64.0;
    let line_h = 16.0;
    let max_lines = ((h - scroll_top - 24.0) / line_h) as usize;
    let start = state.scrollback.len().saturating_sub(max_lines);
    for (i, line) in state.scrollback[start..].iter().enumerate() {
        let color = if line.contains("NAME-SHEAR") {
            SHEAR_RED
        } else if line.starts_with("[ROOM") {
            CYAN
        } else if line.starts_with(">") {
            AMBER
        } else if line.starts_with("    ") {
            DIM
        } else {
            TEXT
        };
        ctx.set_fill_style_str(color);
        let y = scroll_top + (i as f64) * line_h;
        ctx.fill_text(line, 8.0, y).ok();
    }

    // Prompt line (bottom)
    ctx.set_fill_style_str(DIM);
    ctx.fill_rect(0.0, h - 22.0, w, 22.0);
    ctx.set_fill_style_str(AMBER);
    ctx.fill_text(
        "[N S E W = move]   [A = attack]   [save: state persists across reloads]",
        8.0, h - 7.0,
    ).ok();

    Ok(())
}

// ─── JS-facing API ─────────────────────────────────────────────────────────

/// The WASM-exported client handle a JS host constructs and drives.
#[wasm_bindgen]
pub struct MudClient {
    state: WebMudState,
}

#[wasm_bindgen]
impl MudClient {
    /// Start a new run from `seed`.
    #[wasm_bindgen(constructor)]
    pub fn new(seed: u64) -> Self {
        Self { state: WebMudState::new(seed) }
    }

    /// Advance one tick. `input_bits` matches the InputBits layout from
    /// forge_game_systems (MOVE_LEFT=1, MOVE_RIGHT=2, ATTACK=8, MOVE_UP=16, MOVE_DOWN=32, INTERACT=512).
    pub fn tick(&mut self, input_bits: u32) {
        self.state.tick(input_bits);
    }

    /// Paint the current state to the canvas element with the given id.
    pub fn paint(&self, canvas_id: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let doc = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
        let elem = doc.get_element_by_id(canvas_id)
            .ok_or_else(|| JsValue::from_str("canvas id not found"))?;
        let canvas: HtmlCanvasElement = elem.dyn_into()
            .map_err(|_| JsValue::from_str("element is not a canvas"))?;
        paint_state(&self.state, &canvas)
    }

    /// Serialize the run to bytes (JSON). For localStorage round-trip.
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(&self.state).unwrap_or_default()
    }

    /// Construct a MudClient from previously-serialized bytes.
    pub fn deserialize(bytes: &[u8]) -> Result<MudClient, JsValue> {
        let state: WebMudState = serde_json::from_slice(bytes)
            .map_err(|e| JsValue::from_str(&format!("deserialize error: {}", e)))?;
        Ok(MudClient { state })
    }

    /// The run's master seed.
    pub fn seed(&self) -> u64 { self.state.master_seed }
    /// Ticks advanced so far.
    pub fn tick_count(&self) -> u64 { self.state.tick_count }
    /// Current HP, permyriad.
    pub fn hp(&self) -> i32 { self.state.hp_permyriad }

    /// Convenience: returns the InputBits layout constants as a JSON string
    /// so the React side does not have to hard-code them.
    #[wasm_bindgen(js_name = "inputBitsLayout")]
    pub fn input_bits_layout() -> String {
        format!(
            r#"{{"MOVE_NORTH":{},"MOVE_SOUTH":{},"MOVE_EAST":{},"MOVE_WEST":{},"ATTACK":{},"INTERACT":{}}}"#,
            INPUT_MOVE_NORTH, INPUT_MOVE_SOUTH, INPUT_MOVE_EAST,
            INPUT_MOVE_WEST, INPUT_ATTACK, INPUT_INTERACT,
        )
    }
}

// ─── Determinism tests ─────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mulberry32_matches_canonical_first_outputs() {
        // Locked outputs — if these change, the determinism contract is broken
        // and the web preview is no longer the same game as the Steam build.
        let mut rng = Mulberry32::new(42);
        let outputs: Vec<u32> = (0..4).map(|_| rng.next_u32()).collect();
        // These four values are baseline from the frozen Mulberry32 algorithm.
        // If forge-core::seed::Mulberry32 ever changes, BOTH must update together
        // and the IRONROOT save format version must bump.
        assert_eq!(outputs.len(), 4);
    }

    #[test]
    fn tick_replays_identically_for_same_seed_and_input() {
        let mut a = WebMudState::new(123);
        let mut b = WebMudState::new(123);
        let inputs: &[u32] = &[
            INPUT_MOVE_NORTH, INPUT_MOVE_EAST, INPUT_ATTACK,
            INPUT_MOVE_SOUTH, 0, INPUT_ATTACK, INPUT_MOVE_WEST,
        ];
        for &bits in inputs {
            a.tick(bits);
            b.tick(bits);
        }
        let ja = serde_json::to_string(&a).unwrap();
        let jb = serde_json::to_string(&b).unwrap();
        assert_eq!(ja, jb, "two runs with same seed + inputs must serialize identically");
    }

    #[test]
    fn serde_roundtrip_preserves_state() {
        let mut s = WebMudState::new(777);
        for _ in 0..50 { s.tick(INPUT_MOVE_NORTH); }
        let bytes = serde_json::to_vec(&s).unwrap();
        let back: WebMudState = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back.tick_count, s.tick_count);
        assert_eq!(back.player_y, s.player_y);
        assert_eq!(back.master_seed, s.master_seed);
    }

    #[test]
    fn name_shear_fires_deterministically() {
        let mut s = WebMudState::new(1);
        for _ in 0..600 { s.tick(0); }
        assert!(s.name_shear_fired, "Name-Shear should have fired by tick 600 for seed 1");
    }

    #[test]
    fn room_descriptions_are_deterministic() {
        let a = describe_room(99, 3, -2);
        let b = describe_room(99, 3, -2);
        let c = describe_room(99, 3, -1);
        assert_eq!(a, b, "same (seed, x, y) must produce same description");
        assert_ne!(a, c, "different y must produce different description (with high probability)");
    }
}
