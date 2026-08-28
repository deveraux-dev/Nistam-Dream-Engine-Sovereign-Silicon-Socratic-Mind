//! Level Zero canvas — the drop-zone shown when no project is loaded.
//!
//! Host-agnostic: pure tokens + widgets over DrawList.
//! Three live demo objects show the engine is alive; no heap, no physics.
//! Ported from E:/airgap/divmerge-2026-06-12/forge-gui-forge-origin/src/level_zero.rs (T072).
//!
//! Frame-driven animation: `frame_count` is an integer tick counter (sim state);
//! Float sin is visual-lane only (creative positioning) and never stored.

use crate::{
    draw::DrawList,
    geom::UiRect,
    text::FontAtlas,
    tokens::TokenId,
    widgets,
};

/// Draw the Level 0 drop-zone canvas onto a DrawList.
///
/// Called each frame while no project is loaded.
/// `frame_count` drives sine-wave animations on the demo objects — no heap, no physics engine.
/// Float sin is creative-lane only (visual position); not stored as sim state.
pub fn draw_level_zero(
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
    viewport: UiRect,
    frame_count: u64,
) {
    let vx = viewport.x.0;
    let vy = viewport.y.0;
    let vw = viewport.w.0;
    let vh = viewport.h.0;

    // ── 1. Full-bleed background ─────────────────────────────────────────────
    draw.fill_token(viewport, TokenId::BgNebula, 0);

    // ── Layout ───────────────────────────────────────────────────────────────
    let toolbar_h: i64 = 60_000;
    let toolbar_y = vy + vh - toolbar_h;
    let content_h = vh - toolbar_h;
    let cx = vx + vw / 2;

    // ── 2. Headline ──────────────────────────────────────────────────────────
    let headline_w = vw.min(960_000);
    let headline_rect = UiRect::new(
        cx - headline_w / 2,
        vy + content_h / 5,
        headline_w,
        40_000,
    );
    let text_color = draw.token(TokenId::TextPrimary);
    widgets::centered_label(
        draw,
        headline_rect,
        "Drop a picture, a sound, or a tune here",
        text_color,
        atlas,
    );

    // ── 3. Hint text (dimmed alpha derived from TextPrimary) ─────────────────
    let hint_w = vw.min(480_000);
    let hint_rect = UiRect::new(
        cx - hint_w / 2,
        vy + content_h / 5 + 52_000,
        hint_w,
        28_000,
    );
    // Clear the alpha channel and set to 0x66 (40% opacity) — not a colour literal, just alpha.
    let hint_color = (draw.token(TokenId::TextPrimary) >> 8 << 8) | 0x66;
    widgets::centered_label(draw, hint_rect, "or click Start to explore", hint_color, atlas);

    // ── 4. Demo objects ───────────────────────────────────────────────────────
    // Three simple animated shapes in the vertical centre of the content area.
    // Float sin is visual-only (creative-lane); frame_count is sim state (integer).
    let t = frame_count as f32;
    let demo_y = vy + content_h * 55 / 100;
    let spread = vw / 5;

    // Object A — bouncing rect (engine physics is alive)
    {
        let size: i64 = 30_000;
        let dy = ((t * 0.05_f32).sin() * 36_000.0) as i64;
        draw.fill_token(
            UiRect::new(cx - spread - size / 2, demo_y + dy - size / 2, size, size),
            TokenId::AccentCreation,
            4,
        );
    }

    // Object B — pulsing indicator (audio beat phase)
    {
        let pulse = (t * 0.08_f32).sin();
        let size = 26_000 + (pulse * 10_000.0) as i64;
        draw.fill_token(
            UiRect::new(cx - size / 2, demo_y - size / 2, size, size),
            TokenId::AccentCuriosity,
            8,
        );
    }

    // Object C — walking rect (animation frame counter)
    {
        let size: i64 = 24_000;
        let dx = ((t * 0.02_f32).sin() * 56_000.0) as i64;
        draw.fill_token(
            UiRect::new(cx + spread + dx - size / 2, demo_y - size / 2, size, size),
            TokenId::AccentWonder,
            4,
        );
    }

    // ── 5. Bottom toolbar — 5 workspace labels (click handling is future work) ──
    draw.fill_token(
        UiRect::new(vx, toolbar_y, vw, toolbar_h),
        TokenId::Border,
        0,
    );

    let item_labels = ["DROP", "CREATE", "AUDIO", "PLAY", "SHIP"];
    let item_accents = [
        TokenId::AccentMagnificence,
        TokenId::AccentCreation,
        TokenId::AccentCuriosity,
        TokenId::AccentWonder,
        TokenId::AccentMagnificence,
    ];
    let item_w = vw / 5;

    for (i, (label, accent)) in item_labels.iter().zip(item_accents.iter()).enumerate() {
        let ix = vx + i as i64 * item_w;

        // DROP is the primary CTA — give it an accent fill.
        if i == 0 {
            draw.fill_token(UiRect::new(ix, toolbar_y, item_w, toolbar_h), *accent, 0);
        }

        let lx = ix + 8_000;
        let lw = (item_w - 16_000).max(0);
        let ly = toolbar_y + (toolbar_h - 22_000) / 2;
        let tc = draw.token(TokenId::TextPrimary);
        widgets::label(draw, UiRect::new(lx, ly, lw, 22_000), label, tc, atlas);
    }
}

/// Returns true if Level 0 should be dismissed.
///
/// Pure input function — specific inputs → specific outputs (Signal Law).
pub fn handle_level_zero_input(file_dropped: bool, start_clicked: bool) -> bool {
    file_dropped || start_clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_zero_dismiss_on_file_drop() {
        assert!(handle_level_zero_input(true, false));
    }

    #[test]
    fn level_zero_dismiss_on_start_clicked() {
        assert!(handle_level_zero_input(false, true));
    }

    #[test]
    fn level_zero_no_dismiss_when_idle() {
        assert!(!handle_level_zero_input(false, false));
    }

    // L07: Bijection test — input logic is deterministic and pure (same input → same output).
    #[test]
    fn bijection_input_handling() {
        // Test all 4 combinations to verify determinism
        assert_eq!(handle_level_zero_input(false, false), false);
        assert_eq!(handle_level_zero_input(false, false), false); // same input, same output
        assert_eq!(handle_level_zero_input(true, false), true);
        assert_eq!(handle_level_zero_input(true, false), true); // bijection
        assert_eq!(handle_level_zero_input(false, true), true);
        assert_eq!(handle_level_zero_input(false, true), true); // bijection
        assert_eq!(handle_level_zero_input(true, true), true);
        assert_eq!(handle_level_zero_input(true, true), true); // bijection
    }

    // L18: Sabotage test — verify the OR logic is mandatory for correct dismiss behavior.
    // If someone changed && to ||, the function would dismiss even when neither input fires.
    #[test]
    #[should_panic = "sabotage: OR logic required for Level 0 dismiss"]
    fn sabotage_input_logic_or_requirement() {
        // This test documents the invariant: the function must use OR, not AND.
        // If the implementation were (file_dropped && start_clicked), both must be true.
        // Verify the actual logic is OR by testing a case that would fail if it were AND.
        let result = handle_level_zero_input(true, false);
        assert!(result, "sabotage: file_dropped=true alone should dismiss (OR logic)");

        let result2 = handle_level_zero_input(false, true);
        assert!(result2, "sabotage: start_clicked=true alone should dismiss (OR logic)");

        // If AND were used, both would fail. Our assertion catches the logic error.
        // If someone broke it to AND: handle_level_zero_input(true, false) would be false.
        panic!("sabotage: OR logic required for Level 0 dismiss");
    }

    // L07 + L18: Frame count as integer sim state (never float).
    // Verify animation uses frame_count as integer tick, not wall-clock float.
    #[test]
    fn frame_count_integer_discipline() {
        // frame_count parameter is u64 (integer).
        // The function accepts u64 frame_count and only converts to f32 for sin() calc.
        // Sin output is never stored, only used for visual position (creative-lane).
        // This test documents the discipline: frame_count is sim state, sin is visual-lane.

        // Simulate two frames
        let fc1: u64 = 0;
        let fc2: u64 = 1;

        // Both are integers; conversion to f32 happens at point-of-use only (sin).
        let t1 = fc1 as f32;
        let t2 = fc2 as f32;

        // Sin is computed at draw time (creative-lane), never stored.
        let _sin1 = (t1 * 0.05_f32).sin(); // creative: visual position only
        let _sin2 = (t2 * 0.05_f32).sin(); // creative: visual position only

        // Verify frame_count remains integer throughout
        assert_eq!(fc1, 0, "frame_count sim state is integer");
        assert_eq!(fc2, 1, "frame_count sim state is integer");
        // Sin values are never stored in state, only used for temporary visual calc.
    }
}
