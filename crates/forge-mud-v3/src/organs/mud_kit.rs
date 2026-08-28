//! mud_kit.rs — strangler for the three IRONROOT overlays (`mud_console`,
//! `mud_stats`, `mud_taming`). Binds [`MudView`] onto the authored slots.
//!
//! All three were registered with `source=` binds and NO host: they lowered,
//! passed the name census, and painted nothing. Same shape `dungeon_master`
//! wore. READ-ONLY faces — the buttons carry `on_click=edict:` and are the
//! existing dispatch's business, not this file's.

use forge_canvas_v3::draw::{DrawCmd, DrawList};
use forge_canvas_v3::geom::UiRect;
use forge_canvas_v3::text::FontAtlas;
use forge_canvas_v3::tokens::TokenId;
use forge_canvas_v3::widgets;
use forge_vix_v3::ir::{IrRect, LoweredUi};
use forge_vix_v3::loader::{load_kit, studio_panel, LoadedPanel};

use crate::binds::MudView;

/// Every `source=` the three kits author, keyed by panel. The census proves the
/// NAMES agree with [`MudView`]'s schema; the render counts prove the pixels do.
pub const CONSOLE_SLOTS: [&str; 7] = [
    "root.came.title",
    "root.stream.header",
    "root.stream.desc",
    "root.stream.output",
    "root.input.field",
    "root.status.indicator",
    "root.status.help",
];
pub const STATS_SLOTS: [&str; 5] = [
    "root.came.title",
    "root.become.arch.name",
    "root.become.arch.creed",
    "root.become.arch.birth",
    "root.become.arch.prestige",
];
/// The SENSES meter's widget slots (`name=gauge` ×5, `name=glow_dot` ×4) —
/// index-aligned with `MudView::sense_grants_q` / `sense_suppressors_q`.
pub const STATS_GRANT_SLOTS: [&str; 5] = [
    "root.become.senses.grants.logic",
    "root.become.senses.grants.wisdom",
    "root.become.senses.grants.upbringing",
    "root.become.senses.grants.sightline",
    "root.become.senses.grants.attunement",
];
pub const STATS_TAKEN_SLOTS: [&str; 4] = [
    "root.become.senses.taken.blocked",
    "root.become.senses.taken.shadowed",
    "root.become.senses.taken.muted",
    "root.become.senses.taken.dulled",
];
pub const TAMING_SLOTS: [&str; 2] = ["root.came.title", "root.roster.header"];

/// Lower one of the three MUD overlays by its registered panel name.
pub fn lower_mud_kit(panel: &str, viewport: IrRect) -> LoadedPanel {
    let src = studio_panel(panel)
        .unwrap_or_else(|| panic!("'{panel}' is registered in STUDIO_PANELS"));
    load_kit(src, &forge_vix_v3::live::live_ctx(), viewport, 1)
        .unwrap_or_else(|e| panic!("{panel}.kit.vixi lowers clean: {e:?}"))
}

fn box_ui(ui: &LoweredUi, key: &str) -> Option<UiRect> {
    ui.layout.iter().find(|b| b.stable_key.as_str() == key).map(|b| {
        UiRect::new(b.rect.min_x, b.rect.min_y, b.rect.max_x - b.rect.min_x, b.rect.max_y - b.rect.min_y)
    })
}

fn text_slot(
    ui: &LoweredUi,
    key: &str,
    text: &str,
    color: u32,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> usize {
    let Some(b) = box_ui(ui, key) else { return 0 };
    let inner = UiRect::new(b.x.0 + 6_000, b.y.0 + 2_000, b.w.0 - 12_000, b.h.0 - 4_000);
    widgets::label(draw, inner, text, color, atlas);
    1
}

/// Ground + frame, shared by all three overlays (one `came` band, one body).
fn ground(ui: &LoweredUi, draw: &mut DrawList) {
    let Some(root) = box_ui(ui, "root") else { return };
    let bg = draw.token(TokenId::BgVoid);
    draw.push(DrawCmd::Rect { rect: root, color: bg, radius: 4 });
    let frame = draw.token(TokenId::TextDisabled);
    draw.push(DrawCmd::RectOutline { rect: root, color: frame, thickness: 1 });
}

/// `mud_console` — the text interface. Returns the count of bound `source=`
/// slots (7); a renamed kit slot drops the count.
///
/// The ROOM TITLE is the single preattentive focal (primary ink); transcript,
/// input and status stay muted, so the eye lands on where the player IS before
/// it reads what was typed (root#a000).
pub fn render_mud_console(
    ui: &LoweredUi,
    view: &MudView,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> usize {
    ground(ui, draw);
    let ink = draw.token(TokenId::TextPrimary);
    let muted = draw.token(TokenId::TextMuted);
    let mut bound = text_slot(ui, "root.came.title", &view.title, muted, draw, atlas);
    bound += text_slot(ui, "root.stream.header", &view.room_title, ink, draw, atlas);
    for (key, val) in [
        ("root.stream.desc", &view.room_desc),
        ("root.stream.output", &view.console_history),
        ("root.input.field", &view.current_input),
        ("root.status.indicator", &view.connection_status),
        ("root.status.help", &view.help_hint),
    ] {
        bound += text_slot(ui, key, val, muted, draw, atlas);
    }
    bound
}

/// `mud_stats` — the alchemical block. The title and the ARCHETYPE well ride
/// `source=`; the eight stats ride `bind=` and are the lowering's own business.
/// The SENSES meter is painted here: five grant gauges + four suppressor pips,
/// index-aligned with the view's arrays (read, never chosen).
pub fn render_mud_stats(
    ui: &LoweredUi,
    view: &MudView,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> usize {
    ground(ui, draw);
    let ink = draw.token(TokenId::TextPrimary);
    let muted = draw.token(TokenId::TextMuted);
    let mut bound = text_slot(ui, "root.came.title", &view.title, muted, draw, atlas);
    bound += text_slot(ui, "root.become.arch.name", &view.archetype_name, ink, draw, atlas);
    for (key, val) in [
        ("root.become.arch.creed", &view.archetype_creed),
        ("root.become.arch.birth", &view.birth_line),
        ("root.become.arch.prestige", &view.prestige),
    ] {
        bound += text_slot(ui, key, val, muted, draw, atlas);
    }
    for (key, q) in STATS_GRANT_SLOTS.iter().zip(view.sense_grants_q) {
        if let Some(b) = box_ui(ui, key) {
            widgets::gauge(draw, b, q.max(0) as u32, TokenId::AccentCreation);
        }
    }
    for (key, q) in STATS_TAKEN_SLOTS.iter().zip(view.sense_suppressors_q) {
        if let Some(b) = box_ui(ui, key) {
            widgets::glow_dot(draw, b, q.max(0) as u32);
        }
    }
    bound
}

/// The `terminal_mud` face — the SAME terminal glass (`terminal.kit.vixi`) over
/// the offline world. No PTY child: the transcript is the stream, the pending
/// line is the prompt echo, and every word comes off [`MudView`] (the bind,
/// never the engine). Returns the count of slots this face fills.
pub fn render_terminal_mud(
    ui: &LoweredUi,
    view: &MudView,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> usize {
    ground(ui, draw);
    let ink = draw.token(TokenId::TextPrimary);
    let muted = draw.token(TokenId::TextMuted);
    // The mode strip names this face on the active-pill box the kit authors.
    if let Some(b) = box_ui(ui, "root.tabs.shell") {
        forge_gui::terminal_kit::render_mode_pill(
            b,
            forge_gui::terminal_kit::TermMode::Mud,
            true,
            draw,
            atlas,
        );
    }
    let mut bound = text_slot(ui, "root.tabs.path", &view.title, muted, draw, atlas);
    // Transcript + the live prompt echo at the foot — the world owns its caret.
    let stream = format!("{}\n> {}", view.console_history, view.current_input);
    bound += text_slot(ui, "root.body.code", &stream, ink, draw, atlas);
    for (key, val) in [
        ("root.status.pos", &view.roster_capacity),
        ("root.status.gate", &view.connection_status),
        ("root.status.dialect", &view.help_hint),
    ] {
        bound += text_slot(ui, key, val, muted, draw, atlas);
    }
    // THE TKNO DIAL: perception against the authored lane (both permyriad, so
    // the fill IS the ratio). Rides the gutter's top square — the mud face has
    // no line numbers, and the world's reach into you is what that rail says.
    if let Some(g) = box_ui(ui, "root.body.gutter") {
        let side = g.w.0.min(g.h.0);
        let dial = UiRect::new(g.x.0, g.y.0, side, side);
        widgets::gauge(draw, dial, view.perception_q.max(0) as u32, TokenId::AccentCreation);
    }
    bound
}

/// `mud_taming` — the companion roster. Title + the occupancy header; the three
/// pet rows ride `bind=`.
pub fn render_mud_taming(
    ui: &LoweredUi,
    view: &MudView,
    draw: &mut DrawList,
    atlas: &mut FontAtlas,
) -> usize {
    ground(ui, draw);
    let ink = draw.token(TokenId::TextPrimary);
    let muted = draw.token(TokenId::TextMuted);
    let mut bound = text_slot(ui, "root.came.title", &view.title, muted, draw, atlas);
    bound += text_slot(ui, "root.roster.header", &view.roster_capacity, ink, draw, atlas);
    bound
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_loop::FONT;

    fn vp() -> IrRect {
        IrRect::from_xywh(0, 0, 1_280_000, 720_000)
    }

    fn view() -> MudView {
        crate::binds::mud_view(&crate::edicts::DispatchState::default())
    }

    #[test]
    fn every_mud_kit_materializes_the_slots_the_census_names() {
        for (panel, slots) in [
            ("mud_console", &CONSOLE_SLOTS[..]),
            ("mud_stats", &STATS_SLOTS[..]),
            ("mud_taming", &TAMING_SLOTS[..]),
        ] {
            let p = lower_mud_kit(panel, vp());
            for key in slots {
                assert!(
                    p.ui.widgets.iter().any(|w| w.stable_key.as_str() == *key),
                    "{panel}.kit.vixi must materialize {key}"
                );
            }
        }
        // The SENSES meter's nine widget sockets (5 gauges + 4 pips) are census too.
        let p = lower_mud_kit("mud_stats", vp());
        for key in STATS_GRANT_SLOTS.iter().chain(&STATS_TAKEN_SLOTS) {
            assert!(
                p.ui.widgets.iter().any(|w| w.stable_key.as_str() == *key),
                "mud_stats.kit.vixi must materialize {key}"
            );
        }
    }

    // [BOARD: MUD-FACE] `binds::source_bind_census` proved the three overlays'
    // `source=` names agree with MudView; nothing proved a host fed them, and
    // none did — all three lowered blank. This is the pixel half of that pair.
    #[test]
    fn every_sourced_mud_slot_binds_at_paint_time() {
        let v = view();
        let mut atlas = FontAtlas::init(FONT, 16.0);
        for (panel, want, render) in [
            (
                "mud_console",
                CONSOLE_SLOTS.len(),
                render_mud_console as fn(&LoweredUi, &MudView, &mut DrawList, &mut FontAtlas) -> usize,
            ),
            ("mud_stats", STATS_SLOTS.len(), render_mud_stats),
            ("mud_taming", TAMING_SLOTS.len(), render_mud_taming),
        ] {
            let p = lower_mud_kit(panel, vp());
            let mut draw = DrawList::new_boxed();
            let bound = render(&p.ui, &v, &mut draw, &mut atlas);
            assert_eq!(bound, want, "{panel}: bound {bound} of {want} sourced slots");
            assert!(!draw.commands().is_empty(), "{panel} emitted no draw commands");
        }
    }

    /// The terminal_mud face fills the terminal glass off the SAME view — the
    /// transcript is the stream, the prompt echo rides its foot, and the
    /// perception dial sits on the gutter square.
    #[test]
    fn the_terminal_mud_face_binds_the_terminal_glass() {
        let v = view();
        let mut atlas = FontAtlas::init(FONT, 16.0);
        let p = lower_mud_kit("terminal", vp());
        assert!(
            p.ui.layout.iter().any(|b| b.stable_key.as_str() == "root.body.gutter"),
            "the dial's gutter square must lower"
        );
        let mut draw = DrawList::new_boxed();
        let bound = render_terminal_mud(&p.ui, &v, &mut draw, &mut atlas);
        assert_eq!(bound, 5, "terminal_mud binds title + stream + three status slots");
        assert!(!draw.commands().is_empty(), "the face emitted no draw commands");
    }

    // The overlays read the LIVE engine, never a blank row — a face that paints
    // empty strings is the same silent gauge as one with no host at all.
    #[test]
    fn the_view_reads_the_live_world_never_a_blank_row() {
        let v = view();
        for (name, field) in [
            ("title", &v.title),
            ("room_title", &v.room_title),
            ("room_desc", &v.room_desc),
            ("connection_status", &v.connection_status),
            ("help_hint", &v.help_hint),
            ("roster_capacity", &v.roster_capacity),
            ("archetype_name", &v.archetype_name),
            ("archetype_creed", &v.archetype_creed),
            ("birth_line", &v.birth_line),
            ("prestige", &v.prestige),
        ] {
            assert!(!field.trim().is_empty(), "{name} painted blank");
        }
        assert!(v.roster_capacity.contains("companions"), "{}", v.roster_capacity);
    }
}
