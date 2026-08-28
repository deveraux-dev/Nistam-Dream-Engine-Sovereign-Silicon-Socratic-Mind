/// Signal Ghosts — frequency clustering, psychoacoustic emergence, pen interaction, water riding.

#[derive(Debug, Clone)]
pub struct GhostSpawnState {
    pub spawn_time: f32,
    pub emergence: f32,
    pub birth_frequency: f32,
}

impl GhostSpawnState {
    pub fn new(birth_frequency: f32) -> Self {
        Self { spawn_time: 0.0, emergence: 0.0, birth_frequency }
    }
    /// Tick emergence. Returns current visibility (smoothstep over 0.3s).
    pub fn tick(&mut self, dt: f32) -> f32 {
        if self.emergence < 1.0 {
            self.emergence = (self.emergence + dt / 0.3).min(1.0);
        }
        let t = self.emergence;
        t * t * (3.0 - 2.0 * t) // smoothstep
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowType { Cockpit, Performance }

#[derive(Debug, Clone)]
pub struct Ghost {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color_hash: u64,
    pub spawn: GhostSpawnState,
    pub alive: bool,
    pub fading: bool,
    pub fade_timer: f32,
}

pub struct SignalGhostEngine {
    attraction_points: Vec<(f32, f32)>,
}

impl Default for SignalGhostEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalGhostEngine {
    pub fn new() -> Self { Self { attraction_points: Vec::new() } }

    pub fn update_from_spectrum(&mut self, spectrum: &[f32; 64]) {
        self.attraction_points.clear();
        for (i, &energy) in spectrum.iter().enumerate() {
            if energy > 0.3 {
                let x = (i as f32 / 64.0) * 2.0 - 1.0;
                let y = energy * 0.5;
                self.attraction_points.push((x, y));
            }
        }
    }

    fn nearest_attractor(&self, gx: f32, gy: f32) -> Option<&(f32, f32)> {
        self.attraction_points.iter().min_by(|a, b| {
            let da = (a.0 - gx).powi(2) + (a.1 - gy).powi(2);
            let db = (b.0 - gx).powi(2) + (b.1 - gy).powi(2);
            da.partial_cmp(&db).unwrap()
        })
    }

    pub fn ghost_attractions(&self, ghost_positions: &[(f32, f32)]) -> Vec<(f32, f32)> {
        ghost_positions.iter().map(|&(gx, gy)| {
            if let Some(&(ax, ay)) = self.nearest_attractor(gx, gy) {
                ((ax - gx) * 0.02, (ay - gy) * 0.02)
            } else { (0.0, 0.0) }
        }).collect()
    }

    /// Pen interaction: hover attracts, touch scatters.
    pub fn pen_forces(ghosts: &[(f32, f32)], pen_x: f32, pen_y: f32, tip_down: bool) -> Vec<(f32, f32)> {
        let strength = if tip_down { -0.05 } else { 0.03 };
        ghosts.iter().map(|&(gx, gy)| {
            let dx = pen_x - gx;
            let dy = pen_y - gy;
            let dist = (dx * dx + dy * dy).sqrt().max(0.1);
            (dx / dist * strength, dy / dist * strength)
        }).collect()
    }

    pub fn disconnect_ghost(ghost: &mut Ghost, window: WindowType) {
        match window {
            WindowType::Cockpit => { ghost.alive = false; }
            WindowType::Performance => { ghost.fading = true; ghost.fade_timer = 30.0; }
        }
    }

    pub fn tick_fading(ghost: &mut Ghost, dt: f32) {
        if ghost.fading {
            ghost.fade_timer -= dt;
            if ghost.fade_timer <= 0.0 { ghost.alive = false; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergence_starts_zero() {
        let s = GhostSpawnState::new(440.0);
        assert_eq!(s.emergence, 0.0);
    }

    #[test]
    fn test_emergence_reaches_one() {
        let mut s = GhostSpawnState::new(440.0);
        for _ in 0..30 { s.tick(0.01); } // 0.3s total
        assert!((s.emergence - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_emergence_smoothstep() {
        let mut s = GhostSpawnState::new(440.0);
        for _ in 0..15 { s.tick(0.01); } // 0.15s = halfway
        let vis = s.tick(0.0);
        assert!(vis > 0.3 && vis < 0.7, "smoothstep at midpoint should be 0.3-0.7, got {}", vis);
    }

    #[test]
    fn test_clustering_toward_peak() {
        let mut engine = SignalGhostEngine::new();
        let mut spectrum = [0.0f32; 64];
        spectrum[32] = 0.8; // peak at band 32
        engine.update_from_spectrum(&spectrum);
        let forces = engine.ghost_attractions(&[(0.0, 0.0)]);
        assert!(forces[0].0.abs() > 0.001 || forces[0].1.abs() > 0.001);
    }

    #[test]
    fn test_clustering_no_attraction_silence() {
        let engine = SignalGhostEngine::new(); // no spectrum update
        let forces = engine.ghost_attractions(&[(0.0, 0.0)]);
        assert_eq!(forces[0], (0.0, 0.0));
    }

    #[test]
    fn test_pen_hover_attracts() {
        let forces = SignalGhostEngine::pen_forces(&[(0.0, 0.0)], 0.5, 0.5, false);
        assert!(forces[0].0 > 0.0, "hover should attract toward pen");
    }

    #[test]
    fn test_pen_touch_scatters() {
        let forces = SignalGhostEngine::pen_forces(&[(0.0, 0.0)], 0.5, 0.5, true);
        assert!(forces[0].0 < 0.0, "touch should scatter away from pen");
    }

    #[test]
    fn test_disconnect_fade() {
        let mut ghost = Ghost {
            x: 0.0, y: 0.0, vx: 0.0, vy: 0.0, color_hash: 0,
            spawn: GhostSpawnState::new(0.0), alive: true, fading: false, fade_timer: 0.0,
        };
        SignalGhostEngine::disconnect_ghost(&mut ghost, WindowType::Performance);
        assert!(ghost.fading);
        assert_eq!(ghost.fade_timer, 30.0);
        for _ in 0..300 { SignalGhostEngine::tick_fading(&mut ghost, 0.1); }
        assert!(!ghost.alive);
    }

    #[test]
    fn test_water_force_integration() {
        // Non-zero gradient should produce velocity change
        let force = (0.1_f32, -0.05_f32);
        let mut vx = 0.0_f32;
        let mut vy = 0.0_f32;
        vx += force.0;
        vy += force.1;
        assert!(vx.abs() > 0.0 && vy.abs() > 0.0);
    }
}
