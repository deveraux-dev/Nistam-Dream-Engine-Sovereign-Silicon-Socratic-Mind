//! Water FX — 2D shallow water sim driving audio FX parameters.
//! Wave equation: d²h/dt² = c²∇²h - damping·dh/dt

pub struct WaterGrid {
    height: Vec<f64>,
    prev_height: Vec<f64>,
    velocity: Vec<f64>,
    width: usize,
    height_dim: usize,
    wave_speed: f64,
    damping: f64,
    dt: f64,
}

impl WaterGrid {
    pub fn new(width: usize, height: usize) -> Self {
        let n = width * height;
        Self {
            height: vec![0.0; n], prev_height: vec![0.0; n], velocity: vec![0.0; n],
            width, height_dim: height, wave_speed: 0.5, damping: 0.98, dt: 1.0,
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize { y * self.width + x }

    pub fn ripple(&mut self, x_norm: f64, y_norm: f64, amplitude: f64, radius: f64) {
        let cx = (x_norm * self.width as f64) as i32;
        let cy = (y_norm * self.height_dim as f64) as i32;
        let r = (radius * self.width as f64).max(1.0) as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                let gx = cx + dx;
                let gy = cy + dy;
                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height_dim as i32 {
                    let dist = ((dx * dx + dy * dy) as f64).sqrt() / r as f64;
                    if dist <= 1.0 {
                        let falloff = 1.0 - dist * dist;
                        let i = self.idx(gx as usize, gy as usize);
                        self.height[i] += amplitude * falloff;
                    }
                }
            }
        }
    }

    pub fn step(&mut self) {
        let c2dt2 = (self.wave_speed * self.dt).powi(2);
        let w = self.width;
        let h = self.height_dim;
        // Swap prev
        std::mem::swap(&mut self.prev_height, &mut self.height);
        // prev_height now has the "current" values, height will be written with new values
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let left = if x > 0 { self.prev_height[i - 1] } else { self.prev_height[i] };
                let right = if x < w - 1 { self.prev_height[i + 1] } else { self.prev_height[i] };
                let up = if y > 0 { self.prev_height[i - w] } else { self.prev_height[i] };
                let down = if y < h - 1 { self.prev_height[i + w] } else { self.prev_height[i] };
                let laplacian = left + right + up + down - 4.0 * self.prev_height[i];
                self.velocity[i] = (self.velocity[i] + c2dt2 * laplacian) * self.damping;
                self.height[i] = self.prev_height[i] + self.velocity[i] * self.dt;
            }
        }
    }

    pub fn height_at(&self, x_norm: f64, y_norm: f64) -> f64 {
        let x = ((x_norm * self.width as f64) as usize).min(self.width - 1);
        let y = ((y_norm * self.height_dim as f64) as usize).min(self.height_dim - 1);
        self.height[self.idx(x, y)]
    }

    pub fn gradient_at(&self, x_norm: f64, y_norm: f64) -> (f64, f64) {
        let x = ((x_norm * self.width as f64) as usize).clamp(1, self.width - 2);
        let y = ((y_norm * self.height_dim as f64) as usize).clamp(1, self.height_dim - 2);
        let i = self.idx(x, y);
        let dx = self.height[i + 1] - self.height[i - 1];
        let dy = self.height[i + self.width] - self.height[i - self.width];
        (dx * 0.5, dy * 0.5)
    }

    pub fn height_field(&self) -> &[f64] { &self.height }

    pub fn black_hole(&mut self, x_norm: f64, y_norm: f64, radius: f64) {
        let cx = (x_norm * self.width as f64) as i32;
        let cy = (y_norm * self.height_dim as f64) as i32;
        let r = (radius * self.width as f64).max(1.0) as i32;
        for dy in -r..=r {
            for dx in -r..=r {
                let gx = cx + dx;
                let gy = cy + dy;
                if gx >= 0 && gx < self.width as i32 && gy >= 0 && gy < self.height_dim as i32 {
                    let dist = ((dx * dx + dy * dy) as f64).sqrt() / r as f64;
                    if dist <= 1.0 {
                        let i = self.idx(gx as usize, gy as usize);
                        self.height[i] *= dist; // absorb toward zero at center
                        self.velocity[i] *= dist;
                    }
                }
            }
        }
    }

    pub fn total_energy(&self) -> f64 {
        self.height.iter().map(|h| h * h).sum::<f64>() + self.velocity.iter().map(|v| v * v).sum::<f64>()
    }

    pub fn clear(&mut self) {
        self.height.fill(0.0);
        self.prev_height.fill(0.0);
        self.velocity.fill(0.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActiveFx { Filter, Reverb, Delay, Distortion }

pub struct WaterFxParams {
    pub filter_cutoff: f64,
    pub reverb_mix: f64,
    pub delay_feedback: f64,
    pub distortion: f64,
}

pub struct WaterFxMapper {
    pub grid: WaterGrid,
    active_fx: ActiveFx,
}

impl Default for WaterFxMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl WaterFxMapper {
    pub fn new() -> Self {
        Self { grid: WaterGrid::new(64, 64), active_fx: ActiveFx::Filter }
    }

    pub fn pen_ripple(&mut self, x_norm: f64, y_norm: f64, pressure: f64) {
        self.grid.ripple(x_norm, y_norm, pressure, 0.05 + pressure * 0.05);
    }

    pub fn pen_erase(&mut self, x_norm: f64, y_norm: f64) {
        self.grid.black_hole(x_norm, y_norm, 0.08);
    }

    pub fn tick(&mut self) -> WaterFxParams {
        self.grid.step();
        let h = self.grid.height_at(0.5, 0.5); // sample center
        let t = h.clamp(-1.0, 1.0) * 0.5 + 0.5; // map to 0..1
        WaterFxParams {
            filter_cutoff: 200.0 + t * 17800.0,
            reverb_mix: t * 0.8,
            delay_feedback: t * 0.7,
            distortion: 1.0 + t * 3.0,
        }
    }

    pub fn next_fx(&mut self) {
        self.active_fx = match self.active_fx {
            ActiveFx::Filter => ActiveFx::Reverb,
            ActiveFx::Reverb => ActiveFx::Delay,
            ActiveFx::Delay => ActiveFx::Distortion,
            ActiveFx::Distortion => ActiveFx::Filter,
        };
    }

    pub fn ghost_forces(&self, ghost_positions: &[(f64, f64)]) -> Vec<(f64, f64)> {
        ghost_positions.iter().map(|&(x, y)| {
            let (gx, gy) = self.grid.gradient_at(x, y);
            (-gx, -gy) // ghosts flow downhill
        }).collect()
    }

    pub fn render_data(&self) -> &[f64] { self.grid.height_field() }

    /// Render the 64×64 height field as RGBA8 into the output buffer.
    /// The buffer is resized to 64*64*4 bytes (RGBA format).
    /// Heights are clamped to [-1, 1]; positive (crest) → cyan, negative (trough) → navy.
    /// Alpha encodes the absolute height scaled permyriad-style (0..10000 → 0..255).
    pub fn height_field_rgba(&self, out: &mut Vec<u8>) {
        out.clear();
        out.resize(64 * 64 * 4, 0);
        for (i, &h) in self.grid.height_field().iter().enumerate() {
            let h_clamped = h.clamp(-1.0, 1.0);
            let alpha_pmy = (h_clamped.abs() * 10_000.0).clamp(0.0, 10_000.0) as u32;
            let alpha = ((alpha_pmy * 255) / 10_000) as u8;

            // Color mapping: h in [-1, 1]
            // h = 1.0 → crest (cyan: 0, 255, 255)
            // h = 0.0 → neutral (very dim blue: 0, 0, 64)
            // h = -1.0 → trough (navy: 0, 0, 32)
            let (r, g, b) = if h_clamped >= 0.0 {
                // Crest side: interpolate from neutral to cyan
                let t = h_clamped;
                let r = 0u8;
                let g = (t * 255.0) as u8;
                let b = (64.0 + t * (255.0 - 64.0)) as u8;
                (r, g, b)
            } else {
                // Trough side: interpolate from neutral to navy
                let t = -h_clamped;
                let r = 0u8;
                let g = 0u8;
                let b = (64.0 - t * (64.0 - 32.0)) as u8;
                (r, g, b)
            };

            let off = i * 4;
            out[off] = r;
            out[off + 1] = g;
            out[off + 2] = b;
            out[off + 3] = alpha;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_creates_wave() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.5, 0.5, 1.0, 0.1);
        assert!(g.height_at(0.5, 0.5) > 0.0);
    }

    #[test]
    fn test_wave_propagates() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.5, 0.5, 1.0, 0.05);
        for _ in 0..30 { g.step(); }
        assert!(g.height_at(0.8, 0.5).abs() > 1e-6, "wave should reach edges");
    }

    #[test]
    fn test_wave_decays() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.5, 0.5, 1.0, 0.1);
        let e0 = g.total_energy();
        for _ in 0..500 { g.step(); }
        assert!(g.total_energy() < e0 * 0.1, "energy should decay");
    }

    #[test]
    fn test_boundary_reflection() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.05, 0.5, 1.0, 0.05); // near left edge
        for _ in 0..20 { g.step(); }
        // Energy should still be present (reflected, not absorbed)
        assert!(g.total_energy() > 0.001);
    }

    #[test]
    fn test_black_hole_absorbs() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.5, 0.5, 1.0, 0.1);
        let e_before = g.total_energy();
        g.black_hole(0.5, 0.5, 0.15);
        assert!(g.total_energy() < e_before);
    }

    #[test]
    fn test_fx_mapping_neutral_at_rest() {
        let mut m = WaterFxMapper::new();
        let p = m.tick();
        // At rest, height=0, t=0.5 → midpoint values
        assert!((p.filter_cutoff - 9100.0).abs() < 200.0);
        assert!((p.reverb_mix - 0.4).abs() < 0.05);
    }

    #[test]
    fn test_fx_mapping_crest() {
        let mut m = WaterFxMapper::new();
        m.grid.ripple(0.5, 0.5, 2.0, 0.1);
        let p = m.tick();
        assert!(p.filter_cutoff > 9100.0);
    }

    #[test]
    fn test_fx_mapping_trough() {
        let mut m = WaterFxMapper::new();
        m.grid.ripple(0.5, 0.5, -2.0, 0.1);
        let p = m.tick();
        assert!(p.filter_cutoff < 9100.0);
    }

    #[test]
    fn test_ghost_forces_from_gradient() {
        let mut m = WaterFxMapper::new();
        m.grid.ripple(0.5, 0.5, 1.0, 0.1);
        let forces = m.ghost_forces(&[(0.55, 0.5)]);
        assert!(forces[0].0.abs() > 1e-6 || forces[0].1.abs() > 1e-6);
    }

    #[test]
    fn test_ghost_forces_at_rest() {
        let m = WaterFxMapper::new();
        let forces = m.ghost_forces(&[(0.5, 0.5)]);
        assert!(forces[0].0.abs() < 1e-10 && forces[0].1.abs() < 1e-10);
    }

    #[test]
    fn test_clear_resets() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.5, 0.5, 1.0, 0.1);
        g.clear();
        assert!(g.total_energy() < 1e-15);
    }

    #[test]
    fn test_multiple_ripples_interfere() {
        let mut g = WaterGrid::new(64, 64);
        g.ripple(0.3, 0.5, 1.0, 0.05);
        g.ripple(0.7, 0.5, 1.0, 0.05);
        for _ in 0..15 { g.step(); }
        // Midpoint should show interference (constructive or destructive)
        let mid = g.height_at(0.5, 0.5);
        assert!(mid.abs() > 1e-6, "interference should produce non-zero at midpoint");
    }

    #[test]
    fn height_field_rgba_flat_grid_mostly_transparent() {
        let m = WaterFxMapper::new(); // At rest, all heights near zero
        let mut out = Vec::new();
        m.height_field_rgba(&mut out);
        assert_eq!(out.len(), 64 * 64 * 4, "buffer must be exactly 64×64×4 bytes");
        // At rest, alpha should be very low (near zero height)
        let mut nonzero_alpha_count = 0;
        for i in (3..out.len()).step_by(4) {
            if out[i] > 10 {
                nonzero_alpha_count += 1;
            }
        }
        assert!(nonzero_alpha_count < 10, "resting grid should be mostly transparent");
    }

    #[test]
    fn height_field_rgba_ripple_produces_nonzero_alpha() {
        let mut m = WaterFxMapper::new();
        m.grid.ripple(0.5, 0.5, 1.0, 0.1);
        let mut out = Vec::new();
        m.height_field_rgba(&mut out);
        assert_eq!(out.len(), 64 * 64 * 4, "buffer must be exactly 64×64×4 bytes");
        // After ripple, center should have nonzero alpha
        let center_idx = (32 * 64 + 32) * 4 + 3; // center pixel, alpha channel
        assert!(out[center_idx] > 20, "ripple center should have significant alpha");
    }
}
