//! Pen Instrument — gesture detection + pad zone mapping for looper.

use crate::wacom_hid::PenEvent;
use std::time::Instant;

const TAP_MAX_DURATION_MS: u64 = 100;
const TAP_MIN_PRESSURE: f32 = 0.3;
const HOLD_MIN_DURATION_MS: u64 = 300;
const STATIONARY_THRESHOLD: f32 = 0.02;
const SLOW_STROKE_MAX_SPEED: f32 = 0.3;
const FAST_SWIPE_MIN_SPEED: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenGesture {
    StepHit { x_norm: f32, pressure: f32 },
    LiveCapture { x_norm: f32, pressure: f32 },
    WaveformDraw { x_norm: f32, y_norm: f32, pressure: f32 },
    Scrub { x_norm: f32, velocity: f32 },
    Erase { x_norm: f32, y_norm: f32 },
    FxDraw { x_norm: f32, y_norm: f32, pressure: f32 },
    GhostAttract { x_norm: f32, y_norm: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LooperAction {
    TriggerHit { beat_position: f32, velocity: f32 },
    StartCapture { loop_position: f32, gain: f32 },
    StopCapture,
    DrawSample { loop_position: f32, amplitude: f32 },
    ScrubTo { loop_position: f32 },
    EraseAt { loop_position: f32 },
    WaterRipple { x: f32, y: f32, pressure: f32 },
    GhostPull { x: f32, y: f32 },
    Undo,
}

#[derive(Clone, Copy)]
struct PenSample { x: f32, y: f32, pressure: f32, timestamp: Instant }

pub struct GestureDetector {
    history: Vec<PenSample>,
    max_history: usize,
    pen_down_time: Option<Instant>,
    pen_down_pos: Option<(f32, f32)>,
}

impl Default for GestureDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureDetector {
    pub fn new() -> Self {
        Self { history: Vec::with_capacity(64), max_history: 64, pen_down_time: None, pen_down_pos: None }
    }

    fn pen_speed(&self) -> f32 {
        if self.history.len() < 2 { return 0.0; }
        let a = &self.history[self.history.len() - 2];
        let b = &self.history[self.history.len() - 1];
        let dt = b.timestamp.duration_since(a.timestamp).as_secs_f32().max(0.001);
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        (dx * dx + dy * dy).sqrt() / dt
    }

    fn is_stationary(&self, threshold: f32) -> bool {
        if let Some((px, py)) = self.pen_down_pos {
            self.history.last().is_none_or(|s| {
                let dx = s.x - px;
                let dy = s.y - py;
                (dx * dx + dy * dy).sqrt() < threshold
            })
        } else { true }
    }

    pub fn process(&mut self, event: &PenEvent) -> Option<PenGesture> {
        match event {
            PenEvent::PenDown { x, y, pressure } => {
                self.pen_down_time = Some(Instant::now());
                self.pen_down_pos = Some((*x, *y));
                self.history.clear();
                self.history.push(PenSample { x: *x, y: *y, pressure: *pressure, timestamp: Instant::now() });
                None // ambiguous — wait for more data
            }
            PenEvent::PenMove { x, y, pressure } => {
                if self.history.len() >= self.max_history { self.history.remove(0); }
                self.history.push(PenSample { x: *x, y: *y, pressure: *pressure, timestamp: Instant::now() });
                let speed = self.pen_speed();
                let in_fx = *y <= 0.5;
                if in_fx {
                    Some(PenGesture::FxDraw { x_norm: *x, y_norm: *y, pressure: *pressure })
                } else if speed > FAST_SWIPE_MIN_SPEED {
                    Some(PenGesture::Scrub { x_norm: *x, velocity: speed })
                } else if speed < SLOW_STROKE_MAX_SPEED {
                    Some(PenGesture::WaveformDraw { x_norm: *x, y_norm: *y, pressure: *pressure })
                } else { None }
            }
            PenEvent::PenUp { x, y } => {
                let dur_ms = self.pen_down_time.map_or(0, |t| t.elapsed().as_millis() as u64);
                let pressure = self.history.last().map_or(0.0, |s| s.pressure);
                let in_fx = *y <= 0.5;
                self.pen_down_time = None;
                self.pen_down_pos = None;
                if !in_fx && dur_ms < TAP_MAX_DURATION_MS && pressure >= TAP_MIN_PRESSURE && self.is_stationary(STATIONARY_THRESHOLD) {
                    Some(PenGesture::StepHit { x_norm: *x, pressure })
                } else if !in_fx && dur_ms >= HOLD_MIN_DURATION_MS && self.is_stationary(STATIONARY_THRESHOLD) {
                    Some(PenGesture::LiveCapture { x_norm: *x, pressure })
                } else { None }
            }
            PenEvent::EraserDown { x, y, .. } | PenEvent::EraserUp { x, y } => {
                Some(PenGesture::Erase { x_norm: *x, y_norm: *y })
            }
            PenEvent::PenHover { x, y } => {
                if *y <= 0.5 { Some(PenGesture::GhostAttract { x_norm: *x, y_norm: *y }) }
                else { None }
            }
            _ => None,
        }
    }
}

pub struct PenInstrument {
    detector: GestureDetector,
}

impl Default for PenInstrument {
    fn default() -> Self {
        Self::new()
    }
}

impl PenInstrument {
    pub fn new() -> Self { Self { detector: GestureDetector::new() } }

    pub fn process(&mut self, event: &PenEvent) -> Vec<LooperAction> {
        let mut actions = Vec::new();
        // Side buttons
        if let PenEvent::Button1Pressed = event { actions.push(LooperAction::Undo); return actions; }
        if let Some(gesture) = self.detector.process(event) {
            match gesture {
                PenGesture::StepHit { x_norm, pressure } =>
                    actions.push(LooperAction::TriggerHit { beat_position: x_norm, velocity: pressure }),
                PenGesture::LiveCapture { x_norm, pressure } =>
                    actions.push(LooperAction::StartCapture { loop_position: x_norm, gain: pressure }),
                PenGesture::WaveformDraw { x_norm, y_norm, pressure } =>
                    actions.push(LooperAction::DrawSample { loop_position: x_norm, amplitude: (y_norm - 0.75) * 4.0 * pressure }),
                PenGesture::Scrub { x_norm, .. } =>
                    actions.push(LooperAction::ScrubTo { loop_position: x_norm }),
                PenGesture::Erase { x_norm, .. } =>
                    actions.push(LooperAction::EraseAt { loop_position: x_norm }),
                PenGesture::FxDraw { x_norm, y_norm, pressure } =>
                    actions.push(LooperAction::WaterRipple { x: x_norm, y: y_norm, pressure }),
                PenGesture::GhostAttract { x_norm, y_norm } =>
                    actions.push(LooperAction::GhostPull { x: x_norm, y: y_norm }),
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_tap_step_hit() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.5, y: 0.7, pressure: 0.5 });
        // Simulate <100ms by immediately sending PenUp
        let g = det.process(&PenEvent::PenUp { x: 0.5, y: 0.7 });
        assert!(matches!(g, Some(PenGesture::StepHit { .. })));
    }

    #[test]
    fn test_long_press_capture() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.5, y: 0.7, pressure: 0.5 });
        // Simulate >300ms hold
        std::thread::sleep(std::time::Duration::from_millis(310));
        let g = det.process(&PenEvent::PenUp { x: 0.5, y: 0.7 });
        assert!(matches!(g, Some(PenGesture::LiveCapture { .. })));
    }

    #[test]
    fn test_slow_stroke_waveform() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.1, y: 0.7, pressure: 0.5 });
        std::thread::sleep(std::time::Duration::from_millis(50));
        let g = det.process(&PenEvent::PenMove { x: 0.11, y: 0.7, pressure: 0.5 });
        assert!(matches!(g, Some(PenGesture::WaveformDraw { .. })));
    }

    #[test]
    fn test_fast_swipe_scrub() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.1, y: 0.7, pressure: 0.5 });
        std::thread::sleep(std::time::Duration::from_millis(5));
        // Big jump in tiny time = fast
        let g = det.process(&PenEvent::PenMove { x: 0.9, y: 0.7, pressure: 0.5 });
        assert!(matches!(g, Some(PenGesture::Scrub { .. })));
    }

    #[test]
    fn test_eraser_erase() {
        let mut det = GestureDetector::new();
        let g = det.process(&PenEvent::EraserDown { x: 0.5, y: 0.7, pressure: 0.5 });
        assert!(matches!(g, Some(PenGesture::Erase { .. })));
    }

    #[test]
    fn test_top_half_fx_draw() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.5, y: 0.3, pressure: 0.5 });
        std::thread::sleep(std::time::Duration::from_millis(10));
        let g = det.process(&PenEvent::PenMove { x: 0.55, y: 0.3, pressure: 0.5 });
        assert!(matches!(g, Some(PenGesture::FxDraw { .. })));
    }

    #[test]
    fn test_hover_ghost_attract() {
        let mut det = GestureDetector::new();
        let g = det.process(&PenEvent::PenHover { x: 0.5, y: 0.3 });
        assert!(matches!(g, Some(PenGesture::GhostAttract { .. })));
    }

    #[test]
    fn test_button1_undo() {
        let mut pi = PenInstrument::new();
        let actions = pi.process(&PenEvent::Button1Pressed);
        assert!(actions.iter().any(|a| matches!(a, LooperAction::Undo)));
    }

    #[test]
    fn test_zone_boundary() {
        let mut det = GestureDetector::new();
        det.process(&PenEvent::PenDown { x: 0.5, y: 0.5, pressure: 0.5 });
        std::thread::sleep(std::time::Duration::from_millis(10));
        let g = det.process(&PenEvent::PenMove { x: 0.55, y: 0.5, pressure: 0.5 });
        assert!(matches!(g, Some(PenGesture::FxDraw { .. })), "y=0.5 should be FX zone");
    }

    #[test]
    fn test_no_gesture_while_ambiguous() {
        let mut det = GestureDetector::new();
        let g = det.process(&PenEvent::PenDown { x: 0.5, y: 0.7, pressure: 0.5 });
        assert!(g.is_none(), "pen down alone should be ambiguous");
    }
}
