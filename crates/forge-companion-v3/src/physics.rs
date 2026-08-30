//! Procedural secondary motion — damped harmonic oscillators for 5 bone systems.
//!
//! All physics computed on CPU, final transforms sent to GPU per frame.
//! Uses f32 math (not glam) to keep zero dependencies.
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-companion\src\physics.rs`.

use crate::types::BehaviorState;

/// Spring parameters for a single bone oscillator.
#[derive(Clone)]
struct Spring {
    /// Stiffness (N/m).
    k: f32,
    /// Damping coefficient (N*s/m).
    c: f32,
    /// Mass (kg).
    m: f32,
    /// Current angular displacement (radians).
    angle: f32,
    /// Current angular velocity (rad/s).
    velocity: f32,
}

impl Spring {
    fn new(k: f32, c: f32, m: f32) -> Self {
        Self {
            k,
            c,
            m,
            angle: 0.0,
            velocity: 0.0,
        }
    }

    /// Step the oscillator forward by dt seconds with an external impulse force.
    fn step(&mut self, dt: f32, impulse: f32) {
        let restoring = -self.k * self.angle;
        let damping = -self.c * self.velocity;
        let acceleration = (restoring + damping + impulse) / self.m;
        self.velocity += acceleration * dt;
        self.angle += self.velocity * dt;
        // Clamp to prevent wraparound.
        self.angle = self.angle.clamp(-1.2, 1.2);
    }
}

/// Simple PRNG state for idle variant timers (no rand crate).
struct SimpleRng {
    state: u32,
}

impl SimpleRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// Next random f32 in [0, 1).
    fn next_f32(&mut self) -> f32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.state >> 16) & 0x7fff) as f32 / 32768.0
    }
}

/// All 5 procedural bone systems from the design bible.
pub struct SecondaryMotion {
    /// Paintbrush pendulum (k=50, c=5, m=0.5).
    paintbrush: Spring,
    /// Left moustache (k=80, c=8, m=0.1).
    moustache_l: Spring,
    /// Right moustache (k=80, c=8, m=0.1).
    moustache_r: Spring,
    /// Hard hat brim (k=100, c=10, m=0.3).
    hat_brim: Spring,
    /// Pneumatic hose base (k=30, c=3, m=0.2).
    hose: Spring,

    /// Previous hip position for impulse detection.
    prev_hip_x: f32,
    /// Previous head angle for hat/moustache impulse.
    prev_head_angle: f32,

    /// Accumulated time for sine-driven hose oscillation.
    time: f32,
    /// Time since last idle variant trigger.
    idle_timer: f32,
    /// Which idle variant to play next (0=look, 1=scratch, 2=icy_hot).
    idle_variant: u8,

    /// Glasses emission intensity (0.0-1.0).
    pub glasses_emission: f32,
    /// Time until next random glasses glint.
    glint_timer: f32,
    /// PRNG state for random events.
    rng: SimpleRng,
}

impl SecondaryMotion {
    /// Create a new secondary motion system.
    pub fn new() -> Self {
        Self {
            paintbrush: Spring::new(50.0, 5.0, 0.5),
            moustache_l: Spring::new(80.0, 8.0, 0.1),
            moustache_r: Spring::new(80.0, 8.0, 0.1),
            hat_brim: Spring::new(100.0, 10.0, 0.3),
            hose: Spring::new(30.0, 3.0, 0.2),
            prev_hip_x: 0.0,
            prev_head_angle: 0.0,
            time: 0.0,
            idle_timer: 0.0,
            idle_variant: 0,
            glasses_emission: 0.0,
            glint_timer: 4.0,
            rng: SimpleRng::new(0xDEAD_BEEF),
        }
    }

    /// Update all secondary motion systems.
    /// `hip_x` and `head_angle` come from the current baked animation pose.
    /// `state` drives amplitude modifiers per the design bible.
    pub fn update(&mut self, dt: f32, hip_x: f32, head_angle: f32, state: &BehaviorState) {
        self.time += dt;
        self.idle_timer += dt;

        // Compute impulses from parent bone motion.
        let hip_velocity = if dt > 0.0 {
            (hip_x - self.prev_hip_x) / dt
        } else {
            0.0
        };
        let head_velocity = if dt > 0.0 {
            (head_angle - self.prev_head_angle) / dt
        } else {
            0.0
        };
        self.prev_hip_x = hip_x;
        self.prev_head_angle = head_angle;

        // State-driven amplitude modifiers.
        let (brush_impulse_scale, moustache_scale, hose_freq, hose_amp) = match state {
            BehaviorState::Idle => (1.0, 0.3, 0.3, 0.15),
            BehaviorState::Sleep => (0.0, 0.1, 0.0, 0.0),
            BehaviorState::Listening => (0.3, 0.05, 1.2, 0.4),
            BehaviorState::Previewing => (1.0, 0.5, 0.6, 0.2),
            BehaviorState::Executing => (1.0, 0.3, 0.8, 0.3),
        };

        // 1. PAINTBRUSH — reacts to hip motion.
        let brush_impulse = -hip_velocity * 2.0 * brush_impulse_scale;
        self.paintbrush.step(dt, brush_impulse);

        // 2. MOUSTACHE L/R — reacts to head turn.
        let moustache_impulse = head_velocity * moustache_scale;
        self.moustache_l.step(dt, moustache_impulse * 0.8);
        self.moustache_r.step(dt, -moustache_impulse * 0.8);

        // 3. HAT BRIM — reacts to head motion, high stiffness.
        let hat_impulse = -head_velocity * 0.5;
        self.hat_brim.step(dt, hat_impulse);

        // 4. HOSE — sine wave base + spring for transients.
        let sine_drive = (self.time * std::f32::consts::TAU * hose_freq).sin() * hose_amp;
        self.hose.step(dt, sine_drive * self.hose.k * 0.1);

        // 5. GLASSES EMISSION — random glint.
        self.glint_timer -= dt;
        if self.glint_timer <= 0.0 {
            self.glasses_emission = 0.3;
            self.glint_timer = 3.0 + self.rng.next_f32() * 5.0;
        } else {
            self.glasses_emission *= (1.0 - dt * 3.0).max(0.0); // Fade out.
        }
    }

    /// Apply a sharp impulse to specific bones (for animation transitions).
    pub fn impulse_abort(&mut self) {
        self.hat_brim.velocity += 3.0;
        self.moustache_l.velocity += 2.0;
        self.moustache_r.velocity -= 2.0;
        self.hose.velocity += 5.0; // Air burst.
        self.glasses_emission = 0.0;
    }

    /// Apply error impulse.
    pub fn impulse_error(&mut self) {
        self.hat_brim.velocity += 5.0;
        self.moustache_l.velocity += 4.0;
        self.moustache_r.velocity -= 4.0;
        self.hose.velocity = 0.0;
        self.hose.angle = 0.8; // Rigid vertical.
        self.glasses_emission = 0.0;
    }

    /// Apply success impulse.
    pub fn impulse_success(&mut self) {
        self.hat_brim.velocity += 2.0;
        self.paintbrush.velocity += 4.0;
        self.glasses_emission = 1.0;
    }

    /// Apply listen impulse (glasses push-up).
    pub fn impulse_listen(&mut self) {
        self.glasses_emission = 1.0; // Dramatic glasses push-up.
        self.hat_brim.velocity -= 1.0; // Tilts back.
    }

    /// Get current procedural bone angles.
    pub fn paintbrush_angle(&self) -> f32 {
        self.paintbrush.angle
    }

    /// Get left moustache angle.
    pub fn moustache_l_angle(&self) -> f32 {
        self.moustache_l.angle
    }

    /// Get right moustache angle.
    pub fn moustache_r_angle(&self) -> f32 {
        self.moustache_r.angle
    }

    /// Get hat brim angle.
    pub fn hat_brim_angle(&self) -> f32 {
        self.hat_brim.angle
    }

    /// Get hose angle.
    pub fn hose_angle(&self) -> f32 {
        self.hose.angle
    }

    /// Idle variant timer — returns which variant to trigger, or None.
    pub fn check_idle_variant(&mut self) -> Option<IdleVariant> {
        if self.idle_timer < 8.0 {
            return None;
        }

        let threshold = match self.idle_variant {
            0 => 8.0 + self.rng.next_f32() * 7.0, // look_around: 8-15s
            1 => 20.0 + self.rng.next_f32() * 10.0, // scratch: 20-30s
            2 => 45.0 + self.rng.next_f32() * 15.0, // icy_hot: 45-60s
            _ => 10.0,
        };

        if self.idle_timer >= threshold {
            self.idle_timer = 0.0;
            let variant = match self.idle_variant {
                0 => IdleVariant::LookAround,
                1 => IdleVariant::Scratch,
                _ => IdleVariant::IcyHot,
            };
            self.idle_variant = (self.idle_variant + 1) % 3;
            Some(variant)
        } else {
            None
        }
    }
}

impl Default for SecondaryMotion {
    fn default() -> Self {
        Self::new()
    }
}

/// Idle animation variants.
#[derive(Debug, Clone, PartialEq)]
pub enum IdleVariant {
    /// Look around animation.
    LookAround,
    /// Scratch head animation.
    Scratch,
    /// Icy/hot reaction animation.
    IcyHot,
}
