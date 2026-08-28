//! Playtest panel bridge — maps IDE `InputState` to `GameInputs` and emits
//! `DrawCmd::Image` for the game viewport. The seam between the IDE shell and a
//! cartridge, holding no cartridge types so the dependency firewall stands.

use crate::draw::{DrawCmd, DrawList};
use crate::geom::UiRect;
use crate::input::InputState;

// ── XInput button bitmask constants (from XINPUT_GAMEPAD) ──────────────
const XINPUT_GAMEPAD_A: u16 = 0x1000;
const XINPUT_GAMEPAD_B: u16 = 0x2000;
const XINPUT_GAMEPAD_X: u16 = 0x4000;

// Raw arena input bits, mirroring the cartridge-side input bitmask plus
// Ironroot's surge trigger, without importing cartridge crates.
const RAW_MOVE_LEFT: u16 = 1 << 0;
const RAW_MOVE_RIGHT: u16 = 1 << 1;
const RAW_ATTACK: u16 = 1 << 3;
const RAW_MOVE_UP: u16 = 1 << 4;
const RAW_MOVE_DOWN: u16 = 1 << 5;
const RAW_PARRY: u16 = 1 << 7;
const RAW_SURGE: u16 = 0x0400;

/// Dead-zone: a stick axis under this magnitude produces no direction bit.
const STICK_THRESHOLD: f32 = 0.25;

/// Permyriad scale — 10_000 pmy = 1.0, the tree's integer fraction unit.
const PMY: f32 = 10_000.0;

/// Abstracted game inputs produced by the bridge.
///
/// `movement` stays `[f32; 2]` because that is exactly the shape the host
/// already hands us: [`crate::input::RawInputState::gamepad_stick_left`] is
/// `[f32; 2]` by design ("may include floating point per-axis", input.rs:58).
/// This is the hardware boundary, not core IR — the float stops here. Integer
/// consumers take [`GameInputs::movement_pmy`] and never see an `f32`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameInputs {
    /// Movement vector from left stick, `[-1.0, 1.0]` per axis.
    pub movement: [f32; 2],
    /// Primary action (A button).
    pub primary: bool,
    /// Secondary action (B button).
    pub secondary: bool,
    /// Surge trigger (X button).
    pub surge: bool,
    /// Raw input bitmask for host-side cartridge compatibility.
    pub raw_input: u16,
    /// Optional fixed timestep override. The launcher fills this before tick.
    pub dt_override: Option<f32>,
}

impl GameInputs {
    /// Movement as permyriad integers, clamped to `[-10_000, 10_000]`.
    ///
    /// The integer face of [`GameInputs::movement`]: downstream simulation is
    /// integer-deterministic, so it reads this and never the raw axis. Values
    /// outside the unit range (a miscalibrated pad) clamp rather than wrap, and
    /// a non-finite axis reads as zero rather than poisoning the sim.
    pub fn movement_pmy(&self) -> [i32; 2] {
        let axis = |v: f32| -> i32 {
            if !v.is_finite() {
                return 0;
            }
            (v * PMY).round().clamp(-PMY, PMY) as i32
        };
        [axis(self.movement[0]), axis(self.movement[1])]
    }
}

/// Bridge between the IDE UI and the game cartridge.
///
/// Holds the off-screen render target texture id and the panel rect where the
/// game viewport is composited into the editor.
pub struct PlaytestBridge {
    /// GPU texture id of the off-screen render target.
    pub texture_id: u32,
    /// Panel rect in MilliUnit coordinates where the viewport is drawn.
    pub panel_rect: UiRect,
}

impl PlaytestBridge {
    /// Map IDE `InputState` to abstracted [`GameInputs`].
    ///
    /// A disconnected gamepad returns default-zeroed inputs, so the cartridge
    /// receives no movement and no button presses rather than stale ones.
    pub fn map_inputs(input: &InputState) -> GameInputs {
        if !input.raw.gamepad_connected {
            return GameInputs::default();
        }

        GameInputs {
            movement: input.raw.gamepad_stick_left,
            primary: (input.raw.gamepad_buttons & XINPUT_GAMEPAD_A) != 0,
            secondary: (input.raw.gamepad_buttons & XINPUT_GAMEPAD_B) != 0,
            surge: (input.raw.gamepad_buttons & XINPUT_GAMEPAD_X) != 0,
            raw_input: raw_input_from_gamepad(
                input.raw.gamepad_stick_left,
                input.raw.gamepad_buttons,
            ),
            dt_override: None,
        }
    }

    /// Emit a single `DrawCmd::Image` into the draw list for the game viewport.
    pub fn emit_viewport(&self, draw_list: &mut DrawList) {
        self.emit_viewport_in(draw_list, self.panel_rect);
    }

    /// Emit the viewport into an explicit shell-owned panel rect.
    ///
    /// Full UV range, neutral tint. The tint is [`crate::widgets::COLOR_CANVAS_WHITE`]
    /// and NOT `0xFFFFFFFF`: pure white is a skip-sentinel in the quad compositor
    /// (widgets.rs:115) and the donor hardcoded it, so a verbatim port would have
    /// made this viewport render nothing on the GPU path.
    pub fn emit_viewport_in(&self, draw_list: &mut DrawList, panel_rect: UiRect) {
        draw_list.push(DrawCmd::Image {
            rect: panel_rect,
            texture_id: self.texture_id,
            uv: [0.0, 0.0, 1.0, 1.0],
            tint: crate::widgets::COLOR_CANVAS_WHITE,
        });
    }
}

fn raw_input_from_gamepad(stick: [f32; 2], buttons: u16) -> u16 {
    let mut raw = 0;

    if stick[0] < -STICK_THRESHOLD {
        raw |= RAW_MOVE_LEFT;
    } else if stick[0] > STICK_THRESHOLD {
        raw |= RAW_MOVE_RIGHT;
    }

    if stick[1] > STICK_THRESHOLD {
        raw |= RAW_MOVE_UP;
    } else if stick[1] < -STICK_THRESHOLD {
        raw |= RAW_MOVE_DOWN;
    }

    if (buttons & XINPUT_GAMEPAD_A) != 0 {
        raw |= RAW_ATTACK;
    }
    if (buttons & XINPUT_GAMEPAD_B) != 0 {
        raw |= RAW_PARRY;
    }
    if (buttons & XINPUT_GAMEPAD_X) != 0 {
        raw |= RAW_SURGE;
    }

    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_core_v3::fixed_point::MilliUnit;

    fn pad(stick: [f32; 2], buttons: u16, connected: bool) -> InputState {
        let mut i = InputState::default();
        i.raw.gamepad_connected = connected;
        i.raw.gamepad_stick_left = stick;
        i.raw.gamepad_buttons = buttons;
        i
    }

    fn bridge() -> PlaytestBridge {
        PlaytestBridge {
            texture_id: 7,
            panel_rect: UiRect {
                x: MilliUnit(0),
                y: MilliUnit(0),
                w: MilliUnit(640_000),
                h: MilliUnit(480_000),
            },
        }
    }

    #[test]
    fn a_disconnected_pad_yields_no_movement_and_no_buttons() {
        // Stick hard over and every button down, but unplugged: the cartridge
        // must see stillness, not the last physical state.
        let g = PlaytestBridge::map_inputs(&pad([1.0, -1.0], 0xFFFF, false));
        assert_eq!(g, GameInputs::default());
        assert_eq!(g.raw_input, 0);
        assert_eq!(g.movement_pmy(), [0, 0]);
    }

    #[test]
    fn the_dead_zone_holds_on_both_axes() {
        // Just inside the dead zone — no direction bit may light.
        let inside = PlaytestBridge::map_inputs(&pad([0.25, -0.25], 0, true));
        assert_eq!(inside.raw_input, 0, "a stick at exactly the threshold is still centred");

        let outside = PlaytestBridge::map_inputs(&pad([0.26, 0.26], 0, true));
        assert_eq!(outside.raw_input, RAW_MOVE_RIGHT | RAW_MOVE_UP);
    }

    #[test]
    fn left_and_right_are_exclusive_and_down_is_negative_y() {
        assert_eq!(PlaytestBridge::map_inputs(&pad([-1.0, 0.0], 0, true)).raw_input, RAW_MOVE_LEFT);
        assert_eq!(PlaytestBridge::map_inputs(&pad([1.0, 0.0], 0, true)).raw_input, RAW_MOVE_RIGHT);
        assert_eq!(PlaytestBridge::map_inputs(&pad([0.0, -1.0], 0, true)).raw_input, RAW_MOVE_DOWN);
        assert_eq!(PlaytestBridge::map_inputs(&pad([0.0, 1.0], 0, true)).raw_input, RAW_MOVE_UP);
    }

    #[test]
    fn each_face_button_projects_to_its_own_action_and_raw_bit() {
        let a = PlaytestBridge::map_inputs(&pad([0.0, 0.0], XINPUT_GAMEPAD_A, true));
        assert!(a.primary && !a.secondary && !a.surge);
        assert_eq!(a.raw_input, RAW_ATTACK);

        let b = PlaytestBridge::map_inputs(&pad([0.0, 0.0], XINPUT_GAMEPAD_B, true));
        assert!(b.secondary && !b.primary && !b.surge);
        assert_eq!(b.raw_input, RAW_PARRY);

        let x = PlaytestBridge::map_inputs(&pad([0.0, 0.0], XINPUT_GAMEPAD_X, true));
        assert!(x.surge && !x.primary && !x.secondary);
        assert_eq!(x.raw_input, RAW_SURGE);
    }

    #[test]
    fn movement_pmy_is_the_integer_face_and_never_escapes_unit_range() {
        let g = PlaytestBridge::map_inputs(&pad([0.5, -1.0], 0, true));
        assert_eq!(g.movement_pmy(), [5_000, -10_000]);

        // A miscalibrated pad must clamp, not wrap into a huge sim impulse.
        let hot = GameInputs { movement: [9.0, -9.0], ..Default::default() };
        assert_eq!(hot.movement_pmy(), [10_000, -10_000]);

        // A non-finite axis reads as centred rather than poisoning the sim.
        let nan = GameInputs { movement: [f32::NAN, f32::INFINITY], ..Default::default() };
        assert_eq!(nan.movement_pmy(), [0, 0]);
    }

    #[test]
    fn the_viewport_never_tints_with_the_skip_sentinel() {
        // The donor hardcoded 0xFFFFFFFF here; pure white is discarded by the
        // quad compositor, so a verbatim port would have drawn nothing.
        let mut dl = DrawList::default();
        bridge().emit_viewport(&mut dl);

        let images: Vec<(u32, u32)> = dl
            .commands()
            .iter()
            .filter_map(|c| match c {
                DrawCmd::Image { texture_id, tint, .. } => Some((*texture_id, *tint)),
                _ => None,
            })
            .collect();

        assert_eq!(images.len(), 1, "one viewport quad");
        assert_eq!(images[0].0, 7, "carries the off-screen target id");
        assert_ne!(images[0].1, 0xFFFF_FFFF, "pure white would render nothing");
        assert_eq!(images[0].1, crate::widgets::COLOR_CANVAS_WHITE);
    }

    #[test]
    fn emit_viewport_in_overrides_the_held_rect() {
        let mut dl = DrawList::default();
        let shell_rect = UiRect {
            x: MilliUnit(10_000),
            y: MilliUnit(20_000),
            w: MilliUnit(100_000),
            h: MilliUnit(50_000),
        };
        bridge().emit_viewport_in(&mut dl, shell_rect);

        match dl.commands().first().expect("one command") {
            DrawCmd::Image { rect, .. } => assert_eq!(*rect, shell_rect, "shell rect wins over the held one"),
            other => panic!("expected an Image, got {other:?}"),
        }
    }
}
