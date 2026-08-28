//! Wacom CTE-430 HID driver — pen position, pressure, proximity, buttons.

pub const WACOM_VID: u16 = 0x056A;
pub const WACOM_PID: u16 = 0x0013;
pub const WACOM_MAX_X: u16 = 10206;
pub const WACOM_MAX_Y: u16 = 7422;
pub const WACOM_MAX_PRESSURE: u16 = 511;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PenState {
    pub x: u16,
    pub y: u16,
    pub pressure: u16,
    pub tip_down: bool,
    pub eraser: bool,
    pub button1: bool,
    pub button2: bool,
    pub in_proximity: bool,
}

impl PenState {
    pub fn x_norm(&self) -> f32 { self.x as f32 / WACOM_MAX_X as f32 }
    pub fn y_norm(&self) -> f32 { self.y as f32 / WACOM_MAX_Y as f32 }
    pub fn pressure_norm(&self) -> f32 { self.pressure as f32 / WACOM_MAX_PRESSURE as f32 }
    pub fn in_timeline_zone(&self) -> bool { self.y_norm() > 0.5 }
    pub fn in_fx_zone(&self) -> bool { self.y_norm() <= 0.5 }
}

pub fn parse_report(data: &[u8]) -> Option<PenState> {
    if data.len() < 7 || data[0] != 0x02 { return None; }
    let buttons = data[1];
    Some(PenState {
        tip_down: buttons & 0x01 != 0,
        button1: buttons & 0x02 != 0,
        button2: buttons & 0x04 != 0,
        eraser: buttons & 0x08 != 0,
        in_proximity: buttons & 0x20 != 0,
        x: u16::from_le_bytes([data[2], data[3]]),
        y: u16::from_le_bytes([data[4], data[5]]),
        pressure: data[6] as u16 | ((buttons as u16 & 0x10) << 4),
    })
}

#[derive(Debug, Clone, Copy)]
pub enum PenEvent {
    PenDown { x: f32, y: f32, pressure: f32 },
    PenMove { x: f32, y: f32, pressure: f32 },
    PenUp { x: f32, y: f32 },
    PenHover { x: f32, y: f32 },
    EraserDown { x: f32, y: f32, pressure: f32 },
    EraserUp { x: f32, y: f32 },
    Button1Pressed,
    Button1Released,
    Button2Pressed,
    Button2Released,
    ProximityEnter,
    ProximityLeave,
}

pub struct WacomDriver {
    prev: PenState,
}

impl Default for WacomDriver {
    fn default() -> Self {
        Self::new()
    }
}

impl WacomDriver {
    pub fn new() -> Self { Self { prev: PenState::default() } }

    pub fn process_report(&mut self, data: &[u8]) -> Vec<PenEvent> {
        let state = match parse_report(data) { Some(s) => s, None => return vec![] };
        let mut events = Vec::new();
        let p = &self.prev;

        // Proximity
        if state.in_proximity && !p.in_proximity { events.push(PenEvent::ProximityEnter); }
        if !state.in_proximity && p.in_proximity { events.push(PenEvent::ProximityLeave); }

        // Buttons
        if state.button1 && !p.button1 { events.push(PenEvent::Button1Pressed); }
        if !state.button1 && p.button1 { events.push(PenEvent::Button1Released); }
        if state.button2 && !p.button2 { events.push(PenEvent::Button2Pressed); }
        if !state.button2 && p.button2 { events.push(PenEvent::Button2Released); }

        // Tip / eraser
        let (x, y, pr) = (state.x_norm(), state.y_norm(), state.pressure_norm());
        if state.eraser {
            if state.tip_down && !p.tip_down { events.push(PenEvent::EraserDown { x, y, pressure: pr }); }
            if !state.tip_down && p.tip_down { events.push(PenEvent::EraserUp { x, y }); }
        } else {
            if state.tip_down && !p.tip_down { events.push(PenEvent::PenDown { x, y, pressure: pr }); }
            if !state.tip_down && p.tip_down { events.push(PenEvent::PenUp { x, y }); }
            if state.tip_down && p.tip_down && (state.x != p.x || state.y != p.y || state.pressure != p.pressure) {
                events.push(PenEvent::PenMove { x, y, pressure: pr });
            }
            if !state.tip_down && state.in_proximity && (state.x != p.x || state.y != p.y) {
                events.push(PenEvent::PenHover { x, y });
            }
        }

        self.prev = state;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(buttons: u8, x: u16, y: u16, pressure: u8) -> [u8; 7] {
        let xb = x.to_le_bytes();
        let yb = y.to_le_bytes();
        [0x02, buttons, xb[0], xb[1], yb[0], yb[1], pressure]
    }

    #[test]
    fn test_parse_report_pen_down() {
        let r = report(0x01, 5000, 3000, 100);
        let s = parse_report(&r).unwrap();
        assert!(s.tip_down);
        assert!(!s.eraser);
    }

    #[test]
    fn test_parse_report_eraser() {
        let r = report(0x09, 5000, 3000, 100); // tip + eraser
        let s = parse_report(&r).unwrap();
        assert!(s.eraser);
        assert!(s.tip_down);
    }

    #[test]
    fn test_parse_report_position() {
        let r = report(0x20, 5103, 3711, 0); // proximity
        let s = parse_report(&r).unwrap();
        assert_eq!(s.x, 5103);
        assert_eq!(s.y, 3711);
    }

    #[test]
    fn test_parse_report_pressure() {
        // pressure MSB in bit 4 of buttons byte
        let r = report(0x11, 1000, 1000, 255); // tip + pressure MSB set
        let s = parse_report(&r).unwrap();
        assert_eq!(s.pressure, 255 | (1 << 8)); // 511
    }

    #[test]
    fn test_parse_report_too_short() {
        assert!(parse_report(&[0x02, 0x00, 0x00]).is_none());
    }

    #[test]
    fn test_parse_report_wrong_id() {
        assert!(parse_report(&[0x01, 0, 0, 0, 0, 0, 0]).is_none());
    }

    #[test]
    fn test_norm_ranges() {
        let s = PenState { x: WACOM_MAX_X, y: WACOM_MAX_Y, pressure: WACOM_MAX_PRESSURE, ..Default::default() };
        assert!((s.x_norm() - 1.0).abs() < 0.01);
        assert!((s.y_norm() - 1.0).abs() < 0.01);
        assert!((s.pressure_norm() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_zone_detection() {
        let fx = PenState { y: 1000, ..Default::default() };
        assert!(fx.in_fx_zone());
        let tl = PenState { y: 5000, ..Default::default() };
        assert!(tl.in_timeline_zone());
    }

    #[test]
    fn test_event_generation() {
        let mut drv = WacomDriver::new();
        let down = report(0x21, 5000, 3000, 100); // tip + proximity
        let events = drv.process_report(&down);
        assert!(events.iter().any(|e| matches!(e, PenEvent::PenDown { .. })));
        let up = report(0x20, 5000, 3000, 0); // proximity only
        let events = drv.process_report(&up);
        assert!(events.iter().any(|e| matches!(e, PenEvent::PenUp { .. })));
    }

    #[test]
    fn test_event_no_duplicate() {
        let mut drv = WacomDriver::new();
        let r = report(0x21, 5000, 3000, 100);
        drv.process_report(&r);
        let events = drv.process_report(&r); // same state
        assert!(events.is_empty(), "same state should produce no events");
    }
}
