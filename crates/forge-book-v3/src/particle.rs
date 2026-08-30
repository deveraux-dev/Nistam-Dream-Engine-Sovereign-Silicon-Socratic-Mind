//! Particle — a tiny integer particle (MilliUnit position/velocity + a life
//! counter) and a deterministic emitter for a page's ambient motes.

use crate::mulberry::Mulberry32;
use serde::{Deserialize, Serialize};

/// One particle: MilliUnit position + velocity, and remaining life in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Particle {
    /// X position in MilliUnits.
    pub x: i64,
    /// Y position in MilliUnits.
    pub y: i64,
    /// X velocity in MilliUnits per tick.
    pub vx: i64,
    /// Y velocity in MilliUnits per tick.
    pub vy: i64,
    /// Remaining lifespan in ticks.
    pub life: u32,
}

impl Particle {
    /// Creates a new particle with the given position, velocity, and lifespan.
    pub fn new(x: i64, y: i64, vx: i64, vy: i64, life: u32) -> Self {
        Self { x, y, vx, vy, life }
    }
    /// Advance one tick; returns false once the particle has died.
    pub fn step(&mut self) -> bool {
        if self.life == 0 {
            return false;
        }
        self.x += self.vx;
        self.y += self.vy;
        self.life -= 1;
        self.life > 0
    }
    /// Returns true if the particle is still alive.
    pub fn alive(&self) -> bool {
        self.life > 0
    }
}

/// forge_core_v3::Mulberry32 deliberately carries no Default (Crate Zero, no
/// derives beyond what the algorithm itself needs) — this is the skip-field
/// fallback `serde(default = "...")` needs instead of the `Default` trait.
fn skipped_rng() -> Mulberry32 {
    Mulberry32::new(0)
}

/// A deterministic particle emitter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Emitter {
    /// The collection of active particles.
    pub particles: Vec<Particle>,
    #[serde(skip, default = "skipped_rng")]
    rng: Mulberry32,
}

impl Emitter {
    /// Creates a new emitter with the given seed.
    pub fn new(seed: u32) -> Self {
        Self { particles: Vec::new(), rng: Mulberry32::new(u64::from(seed)) }
    }

    /// Emit `n` particles from `(x, y)` with jittered velocity and life.
    pub fn emit(&mut self, x: i64, y: i64, n: usize) {
        for _ in 0..n {
            let vx = (self.rng.below(2001) as i64) - 1000; // -1000..1000 MilliUnit
            let vy = -(self.rng.below(1500) as i64); // upward drift
            let life = 30 + self.rng.below(60);
            self.particles.push(Particle::new(x, y, vx, vy, life));
        }
    }

    /// Step every particle; retire the dead.
    pub fn step(&mut self) {
        for p in &mut self.particles {
            p.step();
        }
        self.particles.retain(Particle::alive);
    }

    /// Returns the number of living particles.
    pub fn alive_count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particle_dies_after_its_life() {
        let mut p = Particle::new(0, 0, 5, -5, 3);
        assert!(p.step());
        assert!(p.step());
        assert!(!p.step()); // life hits 0
        assert!(!p.alive());
    }

    #[test]
    fn emitter_is_deterministic() {
        let mut a = Emitter::new(42);
        let mut b = Emitter::new(42);
        a.emit(0, 0, 20);
        b.emit(0, 0, 20);
        assert_eq!(a.particles, b.particles);
        for _ in 0..100 {
            a.step();
            b.step();
        }
        assert_eq!(a.alive_count(), b.alive_count());
    }

    #[test]
    fn particles_retire() {
        let mut e = Emitter::new(1);
        e.emit(0, 0, 10);
        assert_eq!(e.alive_count(), 10);
        for _ in 0..200 {
            e.step();
        }
        assert_eq!(e.alive_count(), 0); // all lived < 90 ticks
    }
}
