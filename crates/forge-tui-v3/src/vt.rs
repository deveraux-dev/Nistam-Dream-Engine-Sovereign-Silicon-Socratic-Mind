//! VT/ANSI screen — drives this crate's [`GridBuffer`] from a terminal byte
//! stream. One home (L05): moved from shell/src/vt.rs 2026-08-26 (originally
//! ported from NewRepo forge-tui/src/vt.rs v2, 2026-08-10) so studio-tauri's
//! terminal pane can drive the same VT500 machine the native shell renders.
//!
//! `vte` (Alacritty's VT500 state machine) tokenizes the bytes; [`VtScreen`]
//! implements `vte::Perform` to mutate the grid: glyph placement + autowrap,
//! SGR colour (16 / 256 / truecolour), cursor motion, erase, scroll region,
//! and the alternate screen (`?1049h`/`l`) that full-screen TUIs like Claude
//! Code rely on. [`Terminal`] bundles the parser + screen so callers never
//! touch `vte` directly.

use std::collections::VecDeque;
use std::sync::OnceLock;

use crate::buffer::GridBuffer;
use crate::cell::GridCell;
use forge_core_v3::colour::OklchColor;
use forge_core_v3::colour_hub::oklch_to_rgb8;
use vte::{Params, Perform};

#[inline]
const fn rgb(r: u32, g: u32, b: u32) -> u32 {
    (r << 24) | (g << 16) | (b << 8) | 0xFF
}

/// The 16 standard ANSI colours (xterm), packed 0xRRGGBBAA.
// Brighter, lower-strain terminal palette (Sean 2026-06-16: "letters brighter,
// easy on the eyes"). Normals lifted off the dim 128-level; harsh pure-channel
// colours softened with a floor; dark blue (the classic dark-bg readability trap)
// lifted hardest. Black stays 0; whites stay near-max.
const PALETTE16: [u32; 16] = [
    rgb(0, 0, 0),
    rgb(200, 110, 110),
    rgb(96, 210, 116),
    rgb(200, 190, 120),
    rgb(112, 146, 228),
    rgb(210, 112, 210),
    rgb(96, 210, 210),
    rgb(228, 228, 228),
    rgb(150, 154, 164),
    rgb(240, 140, 140),
    rgb(96, 255, 120),
    rgb(245, 230, 150),
    rgb(108, 150, 255),
    rgb(255, 110, 255),
    rgb(110, 255, 255),
    rgb(255, 255, 255),
];

/// One OKLCH swatch spec in the repo's own units: lightness/chroma in
/// permyriad (chroma permyriad is OF THE 0.4 CEILING, matching
/// `forge_core_v3::colour::CHROMA_CEILING_PERMYRIAD`, not of 1.0), hue in
/// degrees. Converted to `OklchColor`'s u16 channels once, at palette build
/// time — never per-frame.
struct Swatch {
    l_pmy: u32,
    c_pmy_of_ceiling: u32,
    hue_deg: u32,
}

impl Swatch {
    const fn achromatic(l_pmy: u32) -> Self {
        Self { l_pmy, c_pmy_of_ceiling: 0, hue_deg: 0 }
    }

    fn to_oklch(&self) -> OklchColor {
        let l = ((self.l_pmy as u64 * 65_535) / 10_000) as u16;
        let c = ((self.c_pmy_of_ceiling as u64 * 65_535) / 10_000) as u16;
        let h = (((self.hue_deg as u64 % 360) * 65_536) / 360) as u16;
        OklchColor { l, c, h, a: u16::MAX }
    }
}

/// `FORGE_PALETTE=accessible` — a high-contrast, colorblind-legible-hue
/// palette built through the same integer OKLCH pipeline
/// `forge_core_v3::colour`/`colour_hub` already ships (`to_oklch`,
/// `oklch_to_rgb8`), not hand-tuned RGB. Lightness for every non-black,
/// non-achromatic slot sits at or above `OKLCH_L_FLOOR_PMY` (7_000 —
/// "Oklch lightness floor for text readability," `colour.rs:16`), same
/// accessibility floor this repo already names for text. Hues are spaced to
/// stay distinguishable under protanopia/deuteranopia (the two most common
/// CVD forms) by leaning on lightness/chroma separation, not hue alone —
/// `forge_core_v3::colour::ColorBlindMode`'s six-profile daltonisation
/// TRANSFORM (the actual per-profile correction math) is enum-ported into
/// v3 but its transform body is still v2-only (`colour_ir.rs:47-63`,
/// unported) — this palette does not attempt to reproduce that; it is a
/// single, always-on high-contrast set, not a per-profile corrected one.
///
/// Chroma held at 3_000/10_000 of the ceiling (≈0.12 raw OKLCH chroma, off a
/// 0.4 ceiling) across every chromatic slot — Sean 2026-08-19: capping
/// chroma this low is what actually kills "neon glare" sensory strain;
/// hue alone doesn't. Lightness fixed at two flat steps (0.72 normal / 0.88
/// bright, Sean's exact numbers) so every slot carries the same contrast
/// delta against a dark background regardless of hue — the non-uniform-
/// lightness problem plain sRGB/HSL has (yellow at 50% reads far brighter
/// than blue at 50%) doesn't reach this palette at all, by construction.
/// Sean reviews/adjusts the exact swatches; this is a first real proposal,
/// not a claimed-final answer.
const ACCESSIBLE_CHROMA: u32 = 3_000;
const ACCESSIBLE_L_NORMAL: u32 = 7_200;
const ACCESSIBLE_L_BRIGHT: u32 = 8_800;
const RED_CHROMA: u32 = 2_200;
const YELLOW_CHROMA: u32 = 2_200;

const ACCESSIBLE_SWATCHES: [Swatch; 16] = [
    Swatch::achromatic(0), // 0 black
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: RED_CHROMA, hue_deg: 25 }, // 1 red
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 142 }, // 2 green
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: YELLOW_CHROMA, hue_deg: 95 }, // 3 yellow
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 258 }, // 4 blue
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 328 }, // 5 magenta
    Swatch { l_pmy: ACCESSIBLE_L_NORMAL, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 195 }, // 6 cyan
    Swatch::achromatic(9_000),                                    // 7 white
    Swatch::achromatic(5_500),                                    // 8 bright black (gray)
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: RED_CHROMA, hue_deg: 25 }, // 9 bright red
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 142 }, // 10 bright green
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: YELLOW_CHROMA, hue_deg: 95 }, // 11 bright yellow
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 258 }, // 12 bright blue
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 328 }, // 13 bright magenta
    Swatch { l_pmy: ACCESSIBLE_L_BRIGHT, c_pmy_of_ceiling: ACCESSIBLE_CHROMA, hue_deg: 195 }, // 14 bright cyan
    Swatch::achromatic(9_800),                                    // 15 bright white
];

fn build_accessible_palette16() -> [u32; 16] {
    let mut out = [0u32; 16];
    for (i, swatch) in ACCESSIBLE_SWATCHES.iter().enumerate() {
        let [r, g, b] = oklch_to_rgb8(swatch.to_oklch());
        out[i] = rgb(r as u32, g as u32, b as u32);
    }
    out
}

/// Resolved once per process — `FORGE_PALETTE=accessible` selects the OKLCH
/// palette above, anything else (including unset) keeps the default
/// `PALETTE16`. Checked once, not per-glyph: this is a boot-time choice, not
/// a live-toggle (matching how `FORGE_DIAG`/`FORGE_MUD` are read elsewhere).
fn active_palette() -> &'static [u32; 16] {
    static ACTIVE: OnceLock<[u32; 16]> = OnceLock::new();
    ACTIVE.get_or_init(|| {
        if std::env::var("FORGE_PALETTE").as_deref() == Ok("accessible") {
            build_accessible_palette16()
        } else {
            PALETTE16
        }
    })
}

/// Map an xterm 256-colour index to packed RGBA.
fn ansi256(n: u16) -> u32 {
    let n = n as u32;
    if n < 16 {
        return active_palette()[n as usize];
    }
    if n <= 231 {
        let n = n - 16;
        let conv = |v: u32| -> u32 {
            if v == 0 {
                0
            } else {
                55 + v * 40
            }
        };
        return rgb(conv(n / 36), conv((n / 6) % 6), conv(n % 6));
    }
    let v = 8 + (n - 232) * 10;
    rgb(v, v, v)
}

/// Parse an extended SGR colour beginning at `p[0]` (which is 38 or 48).
/// Returns (packed colour, extra indices consumed).
fn parse_ext(p: &[u16]) -> Option<(u32, usize)> {
    match p.get(1)? {
        5 => Some((ansi256(*p.get(2)?), 2)),
        2 => {
            let r = *p.get(2)? as u32;
            let g = *p.get(3)? as u32;
            let b = *p.get(4)? as u32;
            Some((rgb(r, g, b), 4))
        }
        _ => None,
    }
}

/// Default foreground colour (off-white).
pub const DEFAULT_FG: u32 = 0xF4F2_EAFF;

/// Default background colour (near-black).
pub const DEFAULT_BG: u32 = 0x0C0A_08FF;

/// Max scrollback rows retained for the MAIN screen (alt-screen has none — a
/// full-screen TUI owns the whole grid). Bounds memory: rows × cols × 16B —
/// 50k rows × 160 cols ≈ 128 MB worst case (Sean 2026-08-26 "large buffer").
pub const SCROLLBACK_MAX: usize = 50_000;

/// A terminal screen: a grid plus the VT cursor/pen state.
pub struct VtScreen {
    /// The grid buffer backing the screen.
    pub grid: GridBuffer,
    cx: u32,
    cy: u32,
    fg: u32,
    bg: u32,
    bold: bool,
    reverse: bool,
    saved: (u32, u32),
    top: u32,
    bot: u32,
    alt: Option<(Vec<GridCell>, u32, u32)>,
    wrap_pending: bool,
    /// Rows that scrolled off the top of the MAIN screen, oldest at the front.
    /// Each row is exactly `grid.width` cells wide at capture time.
    scrollback: VecDeque<Box<[GridCell]>>,
    /// Rows the viewport is scrolled back from the live bottom. 0 = live tail;
    /// the render path reads [`visible_cell`](Self::visible_cell) so 0 is a no-op.
    view_offset: u32,
    /// Bytes the screen owes the SHELL. A terminal is a conversation, not a
    /// billboard: some sequences are questions, and a shell that asks one waits
    /// for the answer before it prints anything else. Drained by the owner of
    /// the pty.
    reply: Vec<u8>,
}

impl VtScreen {
    /// Create a new screen with the given dimensions.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            grid: GridBuffer::new(cols as u32, rows as u32),
            cx: 0,
            cy: 0,
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            bold: false,
            reverse: false,
            saved: (0, 0),
            top: 0,
            bot: (rows as u32).saturating_sub(1),
            alt: None,
            wrap_pending: false,
            reply: Vec::new(),
            scrollback: VecDeque::new(),
            view_offset: 0,
        }
    }

    #[inline]
    fn w(&self) -> u32 {
        self.grid.width
    }

    #[inline]
    fn h(&self) -> u32 {
        self.grid.height
    }

    /// Effective (fg, bg) honouring reverse video.
    fn pen(&self) -> (u32, u32) {
        if self.reverse {
            (self.bg, self.fg)
        } else {
            (self.fg, self.bg)
        }
    }

    /// Create a blank cell with the current pen colours.
    fn blank(&self) -> GridCell {
        let (_f, b) = self.pen();
        GridCell {
            glyph: b' ' as u32,
            fg: self.fg,
            bg: b,
            flags: 0,
        }
    }

    /// Advance to the next line, scrolling if needed.
    fn newline(&mut self) {
        if self.cy >= self.bot {
            // A full-screen scroll (region top == 0) on the MAIN screen retires
            // the top row — capture it into scrollback BEFORE it is overwritten.
            // Alt-screen (a TUI like Claude) keeps no scrollback.
            if self.alt.is_none() && self.top == 0 {
                let w = self.w() as usize;
                let row: Box<[GridCell]> = self.grid.cells[..w].into();
                self.scrollback.push_back(row);
                if self.scrollback.len() > SCROLLBACK_MAX {
                    self.scrollback.pop_front();
                }
                // Keep a scrolled-back view stationary as new lines arrive.
                if self.view_offset > 0 {
                    self.view_offset = (self.view_offset + 1).min(self.scrollback.len() as u32);
                }
            }
            self.grid.scroll_region(self.top, self.bot + 1, 1);
        } else {
            self.cy += 1;
        }
    }

    /// Output a character at the cursor, advancing the cursor (or marking wrap pending).
    fn put(&mut self, ch: char) {
        if self.wrap_pending {
            self.cx = 0;
            self.newline();
            self.wrap_pending = false;
        }
        let (f, b) = self.pen();
        let mut flags = 0u32;
        if self.bold {
            flags |= 1;
        }
        self.grid.set(
            self.cx,
            self.cy,
            GridCell {
                glyph: ch as u32,
                fg: f,
                bg: b,
                flags,
            },
        );
        if self.cx + 1 >= self.w() {
            self.wrap_pending = true;
        } else {
            self.cx += 1;
        }
    }

    /// Resize the screen, reshaping the grid and preserving scrollback history.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let old_grid = std::mem::replace(&mut self.grid, GridBuffer::new(cols as u32, rows as u32));
        if self.alt.is_none() {
            let old_w = old_grid.width as usize;
            for y in 0..self.cy {
                let start = y as usize * old_w;
                let end = start + old_w;
                if end <= old_grid.cells.len() {
                    let row: Box<[GridCell]> = old_grid.cells[start..end].into();
                    self.scrollback.push_back(row);
                }
            }
            while self.scrollback.len() > SCROLLBACK_MAX {
                self.scrollback.pop_front();
            }
        }
        self.cx = self.cx.min(self.grid.width.saturating_sub(1));
        self.cy = 0;
        self.top = 0;
        self.bot = self.grid.height.saturating_sub(1);
        self.wrap_pending = false;
        self.alt = None;
        self.view_offset = 0;
    }

    /// Erase part or all of the current line.
    fn erase_line(&mut self, mode: u16) {
        let blank = self.blank();
        let (s, e) = match mode {
            1 => (0, self.cx + 1),
            2 => (0, self.w()),
            _ => (self.cx, self.w()),
        };
        for x in s..e {
            self.grid.set(x, self.cy, blank);
        }
    }

    /// Erase part or all of the display.
    fn erase_display(&mut self, mode: u16) {
        let blank = self.blank();
        match mode {
            1 => {
                for y in 0..self.cy {
                    for x in 0..self.w() {
                        self.grid.set(x, y, blank);
                    }
                }
                self.erase_line(1);
            }
            2 | 3 => {
                for y in 0..self.h() {
                    for x in 0..self.w() {
                        self.grid.set(x, y, blank);
                    }
                }
            }
            _ => {
                self.erase_line(0);
                for y in self.cy + 1..self.h() {
                    for x in 0..self.w() {
                        self.grid.set(x, y, blank);
                    }
                }
            }
        }
    }

    /// ICH — insert `n` blank cells at the cursor, shifting the rest of the row
    /// right; cells pushed past the right edge fall off. A line editor emits this
    /// when you type into the middle of an existing line.
    fn insert_chars(&mut self, n: u32) {
        let w = self.w();
        if self.cy >= self.h() || self.cx >= w {
            return;
        }
        let n = n.min(w - self.cx);
        let blank = self.blank();
        for x in (self.cx + n..w).rev() {
            let src = self.grid.get(x - n, self.cy);
            self.grid.set(x, self.cy, src);
        }
        for x in self.cx..self.cx + n {
            self.grid.set(x, self.cy, blank);
        }
    }

    /// DCH — delete `n` cells at the cursor, shifting the rest of the row left;
    /// blanks fill in at the right edge. The delete-mid-line counterpart of ICH.
    fn delete_chars(&mut self, n: u32) {
        let w = self.w();
        if self.cy >= self.h() || self.cx >= w {
            return;
        }
        let n = n.min(w - self.cx);
        let blank = self.blank();
        for x in self.cx..w - n {
            let src = self.grid.get(x + n, self.cy);
            self.grid.set(x, self.cy, src);
        }
        for x in w - n..w {
            self.grid.set(x, self.cy, blank);
        }
    }

    /// Scroll the viewport back into history (`delta > 0`) or forward toward the
    /// live tail (`delta < 0`), clamped to the retained scrollback depth.
    pub fn scroll_view(&mut self, delta: i32) {
        let max = self.scrollback.len() as i32;
        self.view_offset = (self.view_offset as i32 + delta).clamp(0, max) as u32;
    }

    /// Snap the viewport back to the live tail (call on user input / new output).
    pub fn reset_view(&mut self) {
        self.view_offset = 0;
    }

    /// Rows the viewport is currently scrolled back (0 = live tail).
    pub fn view_offset(&self) -> u32 {
        self.view_offset
    }

    /// Retained scrollback depth in rows.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// One retained row, oldest first. `idx` is the ABSOLUTE row index used by
    /// [`visible_cell`](Self::visible_cell): row `y` of the viewport is
    /// `scrollback_len() - view_offset() + y`, and indices past the end are live grid rows.
    pub fn scrollback_row(&self, idx: usize) -> Option<&[GridCell]> {
        self.scrollback.get(idx).map(|r| &**r)
    }

    /// Take whatever the screen owes the shell (a DSR answer, today). Empty when
    /// nothing is pending — the caller writes it straight back to the pty.
    pub fn take_reply(&mut self) -> Option<Vec<u8>> {
        (!self.reply.is_empty()).then(|| std::mem::take(&mut self.reply))
    }

    /// The cell shown at visible (`col`, `row`) honouring the scrollback view.
    /// With `view_offset == 0` this is exactly `grid.get(col, row)`; scrolled
    /// back, the top `view_offset` rows come from history and the live grid
    /// slides down out of view.
    pub fn visible_cell(&self, col: u32, row: u32) -> GridCell {
        if self.view_offset == 0 {
            return self.grid.get(col, row);
        }
        let off = self.view_offset;
        if row < off {
            let idx = self.scrollback.len() - off as usize + row as usize;
            return self
                .scrollback
                .get(idx)
                .and_then(|r| r.get(col as usize))
                .copied()
                .unwrap_or(GridCell::EMPTY);
        }
        self.grid.get(col, row - off)
    }

    /// True while a full-screen TUI owns the screen (`?1049h`/`?1047h`/`?47h`).
    ///
    /// Input routing reads this: an app on the alt screen owns the keyboard, so
    /// the host must forward bytes RAW — no line buffer, no command gate, no
    /// synthesised `\r`. Ask before you interpret a keystroke.
    pub fn alt_active(&self) -> bool {
        self.alt.is_some()
    }

    /// Enter the alternate screen.
    fn enter_alt(&mut self) {
        if self.alt.is_none() {
            self.alt = Some((self.grid.cells.clone(), self.cx, self.cy));
            self.grid.clear();
            self.cx = 0;
            self.cy = 0;
            self.wrap_pending = false;
        }
    }

    /// Leave the alternate screen and restore the main screen.
    fn leave_alt(&mut self) {
        if let Some((cells, cx, cy)) = self.alt.take() {
            if cells.len() == self.grid.cells.len() {
                self.grid.cells = cells;
            }
            self.cx = cx;
            self.cy = cy;
            self.grid.dirty = true;
            self.wrap_pending = false;
        }
    }

    /// Handle SGR (Select Graphic Rendition) parameters.
    fn sgr(&mut self, p: &[u16]) {
        if p.is_empty() {
            self.fg = DEFAULT_FG;
            self.bg = DEFAULT_BG;
            self.bold = false;
            self.reverse = false;
            return;
        }
        let mut i = 0;
        while i < p.len() {
            match p[i] {
                0 => {
                    self.fg = DEFAULT_FG;
                    self.bg = DEFAULT_BG;
                    self.bold = false;
                    self.reverse = false;
                }
                1 => self.bold = true,
                22 => self.bold = false,
                7 => self.reverse = true,
                27 => self.reverse = false,
                30..=37 => self.fg = active_palette()[(p[i] - 30) as usize],
                40..=47 => self.bg = active_palette()[(p[i] - 40) as usize],
                90..=97 => self.fg = active_palette()[(p[i] - 90 + 8) as usize],
                100..=107 => self.bg = active_palette()[(p[i] - 100 + 8) as usize],
                39 => self.fg = DEFAULT_FG,
                49 => self.bg = DEFAULT_BG,
                38 => {
                    if let Some((c, adv)) = parse_ext(&p[i..]) {
                        self.fg = c;
                        i += adv;
                    }
                }
                48 => {
                    if let Some((c, adv)) = parse_ext(&p[i..]) {
                        self.bg = c;
                        i += adv;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

/// Append `0xRRGGBBAA` as an xterm OSC colour answer body: `rgb:RRRR/GGGG/BBBB`.
/// Each 8-bit channel widens to 16 bits by `v * 257`, so `0xFF` answers `ffff`.
fn push_osc_rgb(out: &mut Vec<u8>, packed: u32) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let hex = |shift: u32, out: &mut Vec<u8>| {
        let v = ((packed >> shift) & 0xFF) as u16 * 257;
        out.push(DIGITS[((v >> 12) & 0xF) as usize]);
        out.push(DIGITS[((v >> 8) & 0xF) as usize]);
        out.push(DIGITS[((v >> 4) & 0xF) as usize]);
        out.push(DIGITS[(v & 0xF) as usize]);
    };
    out.extend_from_slice(b"rgb:");
    hex(24, out);
    out.push(b'/');
    hex(16, out);
    out.push(b'/');
    hex(8, out);
}

impl Perform for VtScreen {
    fn print(&mut self, c: char) {
        self.put(c);
    }

    /// OSC 4 / OSC 11 with a `?` argument are QUESTIONS, same class as DSR: the
    /// asking program blocks, and an unanswered query surfaces its bytes at the
    /// prompt as phantom input. Answers come from `active_palette` so
    /// `FORGE_PALETTE=accessible` reports what it actually renders. The
    /// set-colour forms carry no `?` and are ignored, not answered.
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let is_query = |p: &[u8]| p == b"?".as_slice();
        if params.len() == 2 && params[0] == b"11".as_slice() && is_query(params[1]) {
            self.reply.extend_from_slice(b"\x1b]11;");
            let bg = active_palette()[0];
            push_osc_rgb(&mut self.reply, bg);
            self.reply.extend_from_slice(b"\x1b\\");
        } else if params.len() == 3 && params[0] == b"4".as_slice() && is_query(params[2]) {
            let Ok(text) = std::str::from_utf8(params[1]) else {
                return;
            };
            let Ok(idx) = text.parse::<u16>() else {
                return;
            };
            if idx > 255 {
                return;
            }
            self.reply.extend_from_slice(b"\x1b]4;");
            self.reply.extend_from_slice(params[1]);
            self.reply.push(b';');
            push_osc_rgb(&mut self.reply, ansi256(idx));
            self.reply.extend_from_slice(b"\x1b\\");
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x0A | 0x0B | 0x0C => {
                self.wrap_pending = false;
                self.newline();
            }
            0x0D => {
                self.cx = 0;
                self.wrap_pending = false;
            }
            0x09 => {
                let next = ((self.cx / 8) + 1) * 8;
                self.cx = next.min(self.w().saturating_sub(1));
            }
            0x08 => {
                if self.wrap_pending {
                    self.wrap_pending = false;
                } else if self.cx > 0 {
                    self.cx -= 1;
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        // ★ hot-alloc → fixed [u16;32] array replaces Vec::with_capacity per CSI
        let mut p_arr = [0u16; 32];
        let mut p_len = 0usize;
        for grp in params.iter() {
            for v in grp {
                if p_len < 32 { p_arr[p_len] = *v; p_len += 1; }
            }
        }
        let p = &p_arr[..p_len];
        let private = intermediates.first() == Some(&b'?');
        let arg = |i: usize, d: u32| -> u32 {
            p.get(i)
                .copied()
                .map(|v| v as u32)
                .filter(|v| *v != 0)
                .unwrap_or(d)
        };

        match action {
            'H' | 'f' => {
                self.cy = (arg(0, 1) - 1).min(self.h().saturating_sub(1));
                self.cx = (arg(1, 1) - 1).min(self.w().saturating_sub(1));
                self.wrap_pending = false;
            }
            'A' => self.cy = self.cy.saturating_sub(arg(0, 1)),
            'B' => self.cy = (self.cy + arg(0, 1)).min(self.h().saturating_sub(1)),
            'C' => self.cx = (self.cx + arg(0, 1)).min(self.w().saturating_sub(1)),
            'D' => self.cx = self.cx.saturating_sub(arg(0, 1)),
            'G' => self.cx = (arg(0, 1) - 1).min(self.w().saturating_sub(1)),
            'd' => self.cy = (arg(0, 1) - 1).min(self.h().saturating_sub(1)),
            'J' => self.erase_display(p.first().copied().unwrap_or(0)),
            'K' => self.erase_line(p.first().copied().unwrap_or(0)),
            'm' => self.sgr(p),
            'r' => {
                self.top = arg(0, 1) - 1;
                self.bot = (arg(1, self.h()) - 1).min(self.h().saturating_sub(1));
            }
            // DSR — Device Status Report. `CSI 6 n` asks WHERE THE CURSOR IS, and the
            // shell blocks on the answer: ConPTY's very first bytes are this query, so
            // an unanswered terminal gets 4 bytes and then silence forever. That is
            // exactly what an empty grid behind a living shell looks like.
            // Appends, never clears: DA1 and DSR arrive in the SAME chunk from
            // ConPTY, and clearing here would drop the answer written first.
            // `take_reply` drains the whole buffer, so nothing accumulates.
            'n' if !private && p.first().copied() == Some(6) => {
                use std::io::Write as _;
                let _ = write!(self.reply, "\x1b[{};{}R", self.cy + 1, self.cx + 1);
            }
            // DA1 — Primary Device Attributes. `CSI c` asks WHAT THIS TERMINAL IS.
            // Unanswered, the reply never comes and the querying program's bytes
            // surface at the prompt as phantom input. `?62;22` = VT220 + ANSI colour,
            // which is what this screen implements (truecolour SGR, alt screen 1049).
            'c' if !private => {
                self.reply.extend_from_slice(b"\x1b[?62;22c");
            }
            's' => self.saved = (self.cx, self.cy),
            'u' => {
                self.cx = self.saved.0;
                self.cy = self.saved.1;
            }
            'h' if private => {
                for &v in p {
                    if matches!(v, 1049 | 1047 | 47) {
                        self.enter_alt();
                    }
                }
            }
            'l' if private => {
                for &v in p {
                    if matches!(v, 1049 | 1047 | 47) {
                        self.leave_alt();
                    }
                }
            }
            'L' => {
                if self.cy >= self.top && self.cy <= self.bot {
                    self.grid
                        .scroll_region(self.cy, self.bot + 1, -(arg(0, 1) as i32));
                }
            }
            'M' => {
                if self.cy >= self.top && self.cy <= self.bot {
                    self.grid.scroll_region(self.cy, self.bot + 1, arg(0, 1) as i32);
                }
            }
            'X' => {
                let n = arg(0, 1);
                let blank = self.blank();
                let end = (self.cx + n).min(self.w());
                for x in self.cx..end {
                    self.grid.set(x, self.cy, blank);
                }
            }
            '@' => self.insert_chars(arg(0, 1)),
            'P' => self.delete_chars(arg(0, 1)),
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            b'M' => {
                if self.cy <= self.top {
                    self.grid.scroll_region(self.top, self.bot + 1, -1);
                } else {
                    self.cy -= 1;
                }
            }
            b'7' => self.saved = (self.cx, self.cy),
            b'8' => {
                self.cx = self.saved.0;
                self.cy = self.saved.1;
            }
            b'c' => {
                self.grid.clear();
                self.cx = 0;
                self.cy = 0;
                self.fg = DEFAULT_FG;
                self.bg = DEFAULT_BG;
            }
            _ => {}
        }
    }
}

/// A terminal: the `vte` parser + the screen it drives. Feed it raw shell
/// output bytes; read the grid for rendering.
pub struct Terminal {
    /// The VT escape sequence parser.
    parser: vte::Parser,
    /// The screen state and grid.
    pub screen: VtScreen,
}

impl Terminal {
    /// Create a new terminal with the given dimensions.
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            parser: vte::Parser::new(),
            screen: VtScreen::new(cols, rows),
        }
    }

    /// Feed raw bytes from the shell into the parser -> screen.
    pub fn feed(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.parser.advance(&mut self.screen, b);
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.screen.resize(cols, rows);
    }

    /// Access the grid buffer.
    pub fn grid(&self) -> &GridBuffer {
        &self.screen.grid
    }

    /// Access the grid buffer mutably.
    pub fn grid_mut(&mut self) -> &mut GridBuffer {
        &mut self.screen.grid
    }

    /// Current cursor cell (col, row).
    pub fn cursor(&self) -> (u32, u32) {
        (self.screen.cx, self.screen.cy)
    }

    /// Scroll the viewport back (`delta > 0`) / toward live (`delta < 0`).
    pub fn scroll_view(&mut self, delta: i32) {
        self.screen.scroll_view(delta);
    }

    /// Snap the viewport back to the live tail.
    pub fn reset_view(&mut self) {
        self.screen.reset_view();
    }

    /// Rows currently scrolled back (0 = live tail).
    pub fn view_offset(&self) -> u32 {
        self.screen.view_offset()
    }

    /// Retained scrollback depth in rows.
    pub fn scrollback_len(&self) -> usize {
        self.screen.scrollback_len()
    }

    /// One retained scrollback row by absolute index; see `VtScreen::scrollback_row`.
    pub fn scrollback_row(&self, idx: usize) -> Option<&[GridCell]> {
        self.screen.scrollback_row(idx)
    }

    /// The cell shown at visible (`col`, `row`) honouring the scrollback view.
    /// Renderers read THIS instead of `grid().get(..)` to become scroll-aware.
    pub fn visible_cell(&self, col: u32, row: u32) -> GridCell {
        self.screen.visible_cell(col, row)
    }

    /// Bytes owed back to the shell after the last `feed` (a DSR answer). The
    /// pty owner writes these; see `VtScreen::take_reply`.
    pub fn take_reply(&mut self) -> Option<Vec<u8>> {
        self.screen.take_reply()
    }

    /// True while a full-screen TUI owns the screen; see `VtScreen::alt_active`.
    pub fn alt_active(&self) -> bool {
        self.screen.alt_active()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prints_plain_text() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"hello");
        assert_eq!(t.grid().get(0, 0).glyph, b'h' as u32);
        assert_eq!(t.grid().get(4, 0).glyph, b'o' as u32);
        assert_eq!(t.cursor(), (5, 0));
    }

    #[test]
    fn newline_moves_down() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"a\r\nb");
        assert_eq!(t.grid().get(0, 0).glyph, b'a' as u32);
        assert_eq!(t.grid().get(0, 1).glyph, b'b' as u32);
    }

    #[test]
    fn sgr_sets_foreground() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b[31mR");
        assert_eq!(t.grid().get(0, 0).fg, PALETTE16[1]); // red
    }

    /// The accessible palette runs through the real `oklch_to_rgb8` pipeline,
    /// not hand-picked RGB — black/white stay achromatic, every chromatic
    /// slot lands on a distinct, non-black colour, and it differs from the
    /// default palette (the whole point of the switch existing).
    #[test]
    fn accessible_palette_is_real_oklch_not_a_relabelled_default() {
        let acc = build_accessible_palette16();
        assert_eq!(acc[0], rgb(0, 0, 0), "slot 0 must stay true black");
        // Slot 15 (bright white) must be near-max on every channel.
        let [r, g, b] = [(acc[15] >> 24) & 0xFF, (acc[15] >> 16) & 0xFF, (acc[15] >> 8) & 0xFF];
        for ch in [r, g, b] {
            assert!(ch > 200, "bright white channel too dim: {ch}");
        }
        // Every chromatic slot (not 0/7/8/15) must differ from the default —
        // if the OKLCH pipeline silently collapsed to identity this would catch it.
        for i in [1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 13, 14] {
            assert_ne!(acc[i], PALETTE16[i], "slot {i} did not change under the accessible palette");
        }
        // The 6 hues must be pairwise distinct — a broken hue calc could
        // collapse them all onto one colour while still passing the above.
        let hues: Vec<u32> = [1, 2, 3, 4, 5, 6].iter().map(|&i| acc[i]).collect();
        for i in 0..hues.len() {
            for j in i + 1..hues.len() {
                assert_ne!(hues[i], hues[j], "hue slots {i} and {j} collapsed to the same colour");
            }
        }
    }

    #[test]
    fn swatch_to_oklch_channel_math_is_in_range() {
        let s = Swatch { l_pmy: 7_200, c_pmy_of_ceiling: 3_000, hue_deg: 258 };
        let c = s.to_oklch();
        assert_eq!(c.l, ((7_200u64 * 65_535) / 10_000) as u16);
        assert_eq!(c.c, ((3_000u64 * 65_535) / 10_000) as u16);
        assert_eq!(c.a, u16::MAX, "must be fully opaque");
    }

    #[test]
    fn cursor_position_csi() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b[2;5HX");
        assert_eq!(t.grid().get(4, 1).glyph, b'X' as u32); // row2 col5 -> (4,1)
    }

    /// The alt-screen flag is what tells a host to stop interpreting keystrokes
    /// and forward them raw — without it a full-screen TUI (Claude Code's Ink UI)
    /// gets its keyboard eaten by the host's own line editor.
    #[test]
    fn alt_active_reports_who_owns_the_keyboard() {
        let mut t = Terminal::new(10, 3);
        assert!(!t.alt_active(), "a bare shell prompt does not own the keyboard");
        t.feed(b"\x1b[?1049h");
        assert!(t.alt_active(), "a full-screen TUI does");
        t.feed(b"\x1b[?1049l");
        assert!(!t.alt_active(), "and gives it back on the way out");
        for enter in [b"\x1b[?1047h".as_slice(), b"\x1b[?47h"] {
            t.feed(enter);
            assert!(t.alt_active(), "the older alt-screen modes count too");
            t.feed(b"\x1b[?1049l");
        }
    }

    #[test]
    fn alt_screen_round_trip() {
        let mut t = Terminal::new(10, 3);
        t.feed(b"main");
        t.feed(b"\x1b[?1049h"); // enter alt -> cleared
        assert_eq!(t.grid().get(0, 0).glyph, b' ' as u32);
        t.feed(b"\x1b[?1049l"); // leave alt -> restored
        assert_eq!(t.grid().get(0, 0).glyph, b'm' as u32);
    }

    #[test]
    fn ich_inserts_blanks_and_shifts_right() {
        let mut t = Terminal::new(5, 1);
        t.feed(b"ABCDE");
        t.feed(b"\x1b[1;1H"); // home
        t.feed(b"\x1b[2@"); // insert 2 blanks -> "  ABC" (D,E fall off)
        assert_eq!(t.grid().get(0, 0).glyph, b' ' as u32);
        assert_eq!(t.grid().get(2, 0).glyph, b'A' as u32);
        assert_eq!(t.grid().get(4, 0).glyph, b'C' as u32);
    }

    #[test]
    fn dch_deletes_and_shifts_left() {
        let mut t = Terminal::new(5, 1);
        t.feed(b"ABCDE");
        t.feed(b"\x1b[1;1H"); // home
        t.feed(b"\x1b[2P"); // delete 2 -> "CDE  "
        assert_eq!(t.grid().get(0, 0).glyph, b'C' as u32);
        assert_eq!(t.grid().get(2, 0).glyph, b'E' as u32);
        assert_eq!(t.grid().get(3, 0).glyph, b' ' as u32);
    }

    #[test]
    fn scrollback_captures_and_view_reveals_history() {
        let mut t = Terminal::new(3, 2);
        t.feed(b"A\r\nB\r\nC"); // 'A' scrolls off the top when 'C' pushes up
        assert_eq!(t.scrollback_len(), 1);
        assert_eq!(t.grid().get(0, 0).glyph, b'B' as u32); // live top row
        t.scroll_view(1);
        assert_eq!(t.view_offset(), 1);
        assert_eq!(t.visible_cell(0, 0).glyph, b'A' as u32); // history surfaces
        assert_eq!(t.visible_cell(0, 1).glyph, b'B' as u32); // live top slides down
        t.reset_view();
        assert_eq!(t.view_offset(), 0);
        assert_eq!(t.visible_cell(0, 0).glyph, b'B' as u32); // back to live tail
    }

    /// The index contract a selecting renderer depends on: absolute row
    /// `scrollback_len() - view_offset() + y` names the same cell the viewport
    /// shows at `y`, whether that row came from history or the live grid.
    #[test]
    fn scrollback_row_shares_the_absolute_index_visible_cell_uses() {
        let mut t = Terminal::new(3, 2);
        t.feed(b"A\r\nB\r\nC\r\nD"); // A and B retire into history
        assert_eq!(t.scrollback_len(), 2);
        assert_eq!(t.scrollback_row(0).unwrap()[0].glyph, b'A' as u32);
        assert_eq!(t.scrollback_row(1).unwrap()[0].glyph, b'B' as u32);
        assert!(t.scrollback_row(2).is_none(), "index 2 is a live grid row, not history");

        for view in [0, 1, 2] {
            t.reset_view();
            t.scroll_view(view as i32);
            let base = t.scrollback_len() - t.view_offset() as usize;
            for y in 0..2u32 {
                let abs = base + y as usize;
                let want = t.visible_cell(0, y).glyph;
                let got = match t.scrollback_row(abs) {
                    Some(row) => row[0].glyph,
                    None => t.grid().get(0, (abs - t.scrollback_len()) as u32).glyph,
                };
                assert_eq!(got, want, "view={view} y={y} abs={abs}");
            }
        }
    }

    #[test]
    fn scroll_view_clamps_to_available_history() {
        let mut t = Terminal::new(4, 2);
        t.scroll_view(5); // nothing captured yet
        assert_eq!(t.view_offset(), 0);
        t.scroll_view(-5);
        assert_eq!(t.view_offset(), 0);
    }

    /// Expected `rgb:RRRR/GGGG/BBBB` body for a packed `0xRRGGBBAA` palette entry.
    fn rgb_body(packed: u32) -> String {
        let ch = |shift: u32| ((packed >> shift) & 0xFF) as u16 * 257;
        format!("rgb:{:04x}/{:04x}/{:04x}", ch(24), ch(16), ch(8))
    }

    #[test]
    fn da1_query_is_answered() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b[c");
        assert_eq!(t.take_reply().as_deref(), Some(&b"\x1b[?62;22c"[..]));
    }

    #[test]
    fn da1_private_form_is_not_answered() {
        // `CSI ? c` is not DA1; answering it would put bytes on the wire unasked.
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b[?c");
        assert!(t.take_reply().is_none());
    }

    #[test]
    fn osc11_background_query_answers_from_live_palette() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b]11;?\x07");
        let reply = t.take_reply().expect("OSC 11 query must be answered");
        let want = format!("\x1b]11;{}\x1b\\", rgb_body(active_palette()[0]));
        assert_eq!(String::from_utf8_lossy(&reply), want);
    }

    #[test]
    fn osc4_palette_query_answers_the_named_slot() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b]4;9;?\x07");
        let reply = t.take_reply().expect("OSC 4 query must be answered");
        let want = format!("\x1b]4;9;{}\x1b\\", rgb_body(ansi256(9)));
        assert_eq!(String::from_utf8_lossy(&reply), want);
    }

    #[test]
    fn osc4_set_form_is_ignored_not_answered() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b]4;9;rgb:1111/2222/3333\x07");
        assert!(t.take_reply().is_none(), "set-colour is not a question");
    }

    #[test]
    fn osc4_out_of_range_index_is_ignored() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b]4;999;?\x07");
        assert!(t.take_reply().is_none());
    }

    /// The regression the BEL strip caused: with 0x07 removed before the parser,
    /// the OSC never terminated and swallowed everything after it.
    #[test]
    fn text_after_a_bel_terminated_osc_still_prints() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b]0;title\x07hi");
        assert_eq!(t.grid().get(0, 0).glyph, b'h' as u32);
        assert_eq!(t.grid().get(1, 0).glyph, b'i' as u32);
    }

    /// DA1 and DSR arrive in one ConPTY chunk; both answers must survive.
    #[test]
    fn two_queries_in_one_chunk_both_answered() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"\x1b[c\x1b[6n");
        let reply = t.take_reply().expect("both queries answered");
        let text = String::from_utf8_lossy(&reply);
        assert!(text.contains("\x1b[?62;22c"), "DA1 answer dropped: {text:?}");
        assert!(text.contains("R"), "DSR answer dropped: {text:?}");
    }

    /// L18 sabotage: a stream carrying NO query must leave the reply empty, so
    /// the assertions above cannot pass on an unconditional write.
    #[test]
    fn plain_text_owes_the_shell_nothing() {
        let mut t = Terminal::new(20, 4);
        t.feed(b"hello world");
        assert!(t.take_reply().is_none());
    }
}
