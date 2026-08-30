//! Animation state machine — clip selection, blending, idle variants.
//!
//! Maps BehaviorState → animation clip, handles transitions with
//! blend times from the design bible.
//!
//! Ported 2026-08-17 from `F:\NewRepo\crates\link-companion\src\animation.rs`.

use crate::model::Transform;
use crate::types::{BehaviorState, Reaction};
use crate::physics::IdleVariant;

/// Active animation with playback state.
struct PlayingClip {
    clip_name: String,
    time: f32,
    duration: f32,
    looping: bool,
    speed: f32,
}

impl PlayingClip {
    fn new(name: &str, duration: f32, looping: bool) -> Self {
        Self {
            clip_name: name.to_string(),
            time: 0.0,
            duration,
            looping,
            speed: 1.0,
        }
    }

    /// Advance playback. Returns true if a non-looping clip has finished.
    fn advance(&mut self, dt: f32) -> bool {
        self.time += dt * self.speed;
        if self.time >= self.duration {
            if self.looping {
                self.time %= self.duration;
                false
            } else {
                self.time = self.duration;
                true
            }
        } else {
            false
        }
    }
}

/// The animation state machine driving the 3rd-Year Painter.
pub struct AnimStateMachine {
    /// Current primary clip.
    current: PlayingClip,
    /// Previous clip being blended out (during transitions).
    blending_from: Option<PlayingClip>,
    /// Blend progress (0.0 = fully previous, 1.0 = fully current).
    blend_factor: f32,
    /// Blend duration in seconds.
    blend_duration: f32,

    /// Current behavior state.
    state: BehaviorState,
    /// Time spent in current state (for sleep transition).
    state_time: f32,
    /// Whether we're playing an idle variant (returns to breathe after).
    in_idle_variant: bool,
    /// Whether a one-shot reactive anim is playing (returns to previous state after).
    in_reaction: bool,
    /// State to return to after reaction finishes.
    return_state: Option<BehaviorState>,
}

/// Blend times from the design bible (in seconds at 30fps).
const BLEND_SNAP: f32 = 3.0 / 30.0; // 3 frames — listen_start, react_error
const BLEND_QUICK: f32 = 2.0 / 30.0; // 2 frames — abort_flinch
const BLEND_SMOOTH: f32 = 5.0 / 30.0; // 5 frames — preview_show
const BLEND_NORMAL: f32 = 4.0 / 30.0; // 4 frames — execute_nod
const BLEND_SETTLE: f32 = 10.0 / 30.0; // 10 frames — return to idle
const BLEND_DRIFT: f32 = 30.0 / 30.0; // 30 frames — idle → sleep

/// Clip durations in seconds at 30fps.
fn clip_duration(name: &str) -> f32 {
    match name {
        "idle_breathe" => 60.0 / 30.0,
        "idle_look_around" => 90.0 / 30.0,
        "idle_scratch" => 45.0 / 30.0,
        "idle_icy_hot" => 60.0 / 30.0,
        "idle_sleep" => 120.0 / 30.0,
        "listen_start" => 12.0 / 30.0,
        "listen_hold" => 30.0 / 30.0,
        "preview_show" => 15.0 / 30.0,
        "abort_flinch" => 20.0 / 30.0,
        "execute_nod" => 18.0 / 30.0,
        "ctx_coding" | "ctx_daw" | "ctx_terminal" => 1.0 / 30.0,
        "react_error" => 30.0 / 30.0,
        "react_success" => 25.0 / 30.0,
        _ => 30.0 / 30.0,
    }
}

fn clip_loops(name: &str) -> bool {
    matches!(name, "idle_breathe" | "idle_sleep" | "listen_hold")
}

impl AnimStateMachine {
    /// Create a new state machine in idle_breathe.
    pub fn new() -> Self {
        Self {
            current: PlayingClip::new("idle_breathe", clip_duration("idle_breathe"), true),
            blending_from: None,
            blend_factor: 1.0,
            blend_duration: 0.0,
            state: BehaviorState::Idle,
            state_time: 0.0,
            in_idle_variant: false,
            in_reaction: false,
            return_state: None,
        }
    }

    /// Transition to a new clip with a given blend time.
    fn transition_to(&mut self, clip_name: &str, blend_time: f32) {
        let old = std::mem::replace(
            &mut self.current,
            PlayingClip::new(clip_name, clip_duration(clip_name), clip_loops(clip_name)),
        );
        self.blending_from = Some(old);
        self.blend_factor = 0.0;
        self.blend_duration = blend_time;
    }

    /// Set the behavior state (from AnimCommand::SetState).
    pub fn set_state(&mut self, new_state: BehaviorState) {
        if self.in_reaction {
            self.return_state = Some(new_state);
            return;
        }

        let (clip, blend) = match new_state {
            BehaviorState::Idle => ("idle_breathe", BLEND_SETTLE),
            BehaviorState::Sleep => ("idle_sleep", BLEND_DRIFT),
            BehaviorState::Listening => {
                if self.state == BehaviorState::Idle || self.state == BehaviorState::Sleep {
                    ("listen_start", BLEND_SNAP)
                } else {
                    ("listen_hold", BLEND_SMOOTH)
                }
            }
            BehaviorState::Previewing => ("preview_show", BLEND_SMOOTH),
            BehaviorState::Executing => ("execute_nod", BLEND_NORMAL),
        };

        self.state = new_state;
        self.state_time = 0.0;
        self.in_idle_variant = false;
        self.transition_to(clip, blend);
    }

    /// Trigger a one-shot reaction (from AnimCommand::React).
    pub fn react(&mut self, reaction: Reaction) {
        let clip = match reaction {
            Reaction::Error => "react_error",
            Reaction::Success => "react_success",
            Reaction::Abort => "abort_flinch",
        };
        let blend = match reaction {
            Reaction::Abort => BLEND_QUICK,
            _ => BLEND_SNAP,
        };

        self.return_state = Some(self.state.clone());
        self.in_reaction = true;
        self.transition_to(clip, blend);
    }

    /// Trigger an idle variant animation.
    pub fn play_idle_variant(&mut self, variant: IdleVariant) {
        if self.state != BehaviorState::Idle {
            return;
        }

        let clip = match variant {
            IdleVariant::LookAround => "idle_look_around",
            IdleVariant::Scratch => "idle_scratch",
            IdleVariant::IcyHot => "idle_icy_hot",
        };

        self.in_idle_variant = true;
        self.transition_to(clip, BLEND_SETTLE);
    }

    /// Advance the animation by dt seconds. Returns the current clip name.
    pub fn update(&mut self, dt: f32) -> &str {
        self.state_time += dt;

        // Advance blend.
        if self.blend_duration > 0.0 && self.blend_factor < 1.0 {
            self.blend_factor += dt / self.blend_duration;
            if self.blend_factor >= 1.0 {
                self.blend_factor = 1.0;
                self.blending_from = None;
            }
        }

        // Advance previous clip if blending.
        if let Some(ref mut prev) = self.blending_from {
            prev.advance(dt);
        }

        // Advance current clip.
        let finished = self.current.advance(dt);

        // Handle non-looping clip completion.
        if finished {
            if self.in_reaction {
                self.in_reaction = false;
                if let Some(return_state) = self.return_state.take() {
                    self.set_state(return_state);
                } else {
                    self.set_state(BehaviorState::Idle);
                }
            } else if self.in_idle_variant {
                self.in_idle_variant = false;
                self.transition_to("idle_breathe", BLEND_SETTLE);
            } else if self.current.clip_name == "listen_start" {
                // listen_start → listen_hold (automatic).
                self.transition_to("listen_hold", BLEND_SMOOTH);
            } else if self.current.clip_name == "execute_nod" {
                self.set_state(BehaviorState::Idle);
            }
        }

        // Auto-sleep after 60s idle.
        if self.state == BehaviorState::Idle
            && !self.in_idle_variant
            && !self.in_reaction
            && self.state_time > 60.0
        {
            self.set_state(BehaviorState::Sleep);
        }

        &self.current.clip_name
    }

    /// Get the current clip name for sampling.
    pub fn current_clip(&self) -> &str {
        &self.current.clip_name
    }

    /// Get blend info for the renderer.
    pub fn blend_info(&self) -> Option<(&str, f32)> {
        self.blending_from.as_ref().map(|prev| (prev.clip_name.as_str(), self.blend_factor))
    }

    /// Sample the current animation state, blending if in transition.
    pub fn sample(&self, clips: &[crate::model::AnimationClip]) -> Vec<Option<Transform>> {
        let current_clip = clips.iter().find(|c| c.name == self.current.clip_name);
        let current_sample = current_clip
            .map(|c| c.sample(self.current.time))
            .unwrap_or_default();

        if let Some(ref prev) = self.blending_from {
            let prev_clip = clips.iter().find(|c| c.name == prev.clip_name);
            if let Some(prev_clip) = prev_clip {
                let prev_sample = prev_clip.sample(prev.time);
                // Blend between previous and current.
                return current_sample
                    .iter()
                    .zip(prev_sample.iter())
                    .map(|(curr, prev)| match (curr, prev) {
                        (Some(c), Some(p)) => Some(p.lerp(c, self.blend_factor)),
                        (Some(c), None) => Some(c.clone()),
                        (None, Some(p)) => Some(p.clone()),
                        (None, None) => None,
                    })
                    .collect();
            }
        }

        current_sample
    }
}

impl Default for AnimStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
