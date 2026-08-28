//! Effect dispatcher: binds phrase kinds to effect masks and routes to lanes.

/// Query tag: grave bell marker.
pub const QUERY_TAG_GRAVE_BELL: u8 = 0x01;
/// Query tag: none / untagged.
pub const QUERY_TAG_NONE: u8 = 0x00;

/// Phrase kind: minor third descent.
pub const PHRASE_KIND_MINOR_THIRD_DESCENT: u8 = 1;
/// Phrase kind: silent hold.
pub const PHRASE_KIND_SILENT_HOLD: u8 = 2;
/// Phrase kind: refusal rest.
pub const PHRASE_KIND_REFUSAL_REST: u8 = 3;

/// Glyph index: grave bell.
pub const GLYPH_IDX_GRAVE_BELL: u8 = 0;
/// Glyph index: phrase echo.
pub const GLYPH_IDX_PHRASE_ECHO: u8 = 1;

/// Maximum binding entries in the dispatcher table.
pub const MAX_BINDINGS: usize = 8;
/// Maximum pending actions queued.
pub const MAX_PENDING: usize = 16;

/// Effect bit: interaction query (physics lane).
pub const EFFECT_INTERACTION_QUERY: u8 = 0x01;
/// Effect bit: mixer delta (audio lane).
pub const EFFECT_MIXER_DELTA: u8 = 0x02;
/// Effect bit: scene audio (audio lane).
pub const EFFECT_SCENE_AUDIO: u8 = 0x04;
/// Effect bit: glyph render (render lane).
pub const EFFECT_GLYPH_RENDER: u8 = 0x08;
/// Effect bit: explicit NDE inference / semantic resolution (inference lane).
pub const EFFECT_SEMANTIC_RESOLVE: u8 = 0x10;
/// Effect bit: camera Strike toward the MAX pole (render lane; `shell::camera_lens`).
pub const EFFECT_CAMERA_MAX: u8 = 0x20;
/// Effect bit: camera Strike toward the ATOM pole (render lane; `shell::camera_lens`).
pub const EFFECT_CAMERA_ATOM: u8 = 0x40;

/// Default budget threshold in microseconds.
pub const DEFAULT_BUDGET_THRESHOLD_US: u32 = 1_000;
/// Sentinel value meaning budget_remaining is not set.
pub const BUDGET_REMAINING_UNSET: u32 = u32::MAX;
/// Type identifier for effect dispatcher sieve.
pub const SIEVE_TYPE_EFFECT_DISPATCHER: u32 = 1;

/// An action produced by the dispatcher for a lane to consume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SieveAction {
    /// Emit an interaction query with 16 bytes of query data.
    EmitInteractionQuery {
        /// Query bytes: `[0]` is tag, `[1..9]` is hash.
        query_bytes: [u8; 16],
        /// Phrase kind that produced this action.
        phrase_kind: u8,
    },
    /// Apply mixer delta with 26 bytes of delta data.
    ApplyMixerDelta {
        /// Delta payload.
        delta_bytes: [u8; 26],
        /// Phrase kind that produced this action.
        phrase_kind: u8,
    },
    /// Fire a scene audio event with intensity.
    FireSceneAudioEvent {
        /// Event identifier.
        event_id: u8,
        /// Intensity quantized to u16.
        intensity_q: u16,
        /// Phrase kind that produced this action.
        phrase_kind: u8,
    },
    /// Render a glyph with parameters.
    GlyphRender {
        /// Glyph depth scale (0..=255).
        depth_scale: u8,
        /// Glyph origin chunk.
        origin_chunk: u32,
        /// Glyph grid hash seed.
        grid_hash: u64,
        /// Phrase kind that produced this action.
        phrase_kind: u8,
    },
}

/// Event observed by the dispatcher.
#[derive(Debug, Clone, Copy)]
pub enum SieveEvent {
    /// Semantic binding fired with payload.
    SemanticBindingFired {
        /// Kind of phrase that fired.
        phrase_kind: u8,
        /// Window duration in microseconds.
        window_us: u32,
        /// Payload hash for lookup/xor.
        payload_hash: u64,
    },
    /// Immutable ASP-solved 5D keyframe intent (ported from v2 forge-sieve's
    /// `SieveEvent::Mutate5D`, MMX3-CLOCKWORK-001 — an external solver pushes
    /// solved Mutate(X,Y,Z,T,S) onto the bus; the engine blindly renders).
    /// `*_mu` fields are `MilliUnit` (i64), `t_tick` is a 120 Hz `SimTick`,
    /// `s` is a state ordinal. Integer-only. First real consumer:
    /// `forge-arena-v3::mechanic_rail::MechanicRail::observe`.
    Mutate5D {
        /// Entity this keyframe applies to.
        ent: u64,
        /// X position, MilliUnit.
        x_mu: i64,
        /// Y position, MilliUnit.
        y_mu: i64,
        /// Z position, MilliUnit.
        z_mu: i64,
        /// Solved tick, 120 Hz SimTick.
        t_tick: u64,
        /// State ordinal.
        s: u32,
    },
}

/// Snapshot of dispatcher state for serialization.
#[derive(Debug, Clone)]
pub struct SieveSnapshot {
    /// Type identifier.
    pub sieve_type: u32,
    /// Serialized data.
    pub data: Vec<u8>, // COLD-PATH: serialization only, never per-tick
}

/// One row of the effect binding table. Defines what effects fire for a phrase.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BindingEntry {
    /// Phrase kind this binding matches.
    pub phrase_kind: u8,
    /// Bit mask of effects to fire (EFFECT_* bits).
    pub effect_mask: u8,
    /// Index into mixer delta table.
    pub mixer_delta_idx: u8,
    /// Index into glyph table.
    pub glyph_idx: u8,
    /// Scene event ID to fire.
    pub scene_event_id: u8,
    /// Padding to 8 bytes.
    pub _pad: [u8; 3],
}

impl BindingEntry {
    /// Empty binding entry (all zeros).
    pub const fn empty() -> Self {
        Self {
            phrase_kind: 0,
            effect_mask: 0,
            mixer_delta_idx: 0,
            glyph_idx: 0,
            scene_event_id: 0,
            _pad: [0; 3],
        }
    }
}

/// Glyph binding definition.
#[derive(Clone, Copy, Debug, Default)]
pub struct GlyphBinding {
    /// Depth scale for rendering.
    pub depth_scale: u8,
    /// Origin chunk identifier.
    pub origin_chunk: u32,
    /// Grid hash seed.
    pub grid_hash_seed: u64,
}

impl GlyphBinding {
    /// Empty glyph binding.
    pub const fn empty() -> Self {
        Self { depth_scale: 0, origin_chunk: 0, grid_hash_seed: 0 }
    }
}

/// Lookup a glyph by index. Returns None for out-of-range or undefined glyphs.
pub fn lookup_glyph(idx: u8) -> Option<GlyphBinding> {
    match idx {
        GLYPH_IDX_GRAVE_BELL => Some(GlyphBinding {
            depth_scale: 100,
            origin_chunk: 0x12345678,
            grid_hash_seed: 0xABCDEF0123456789,
        }),
        GLYPH_IDX_PHRASE_ECHO => Some(GlyphBinding {
            depth_scale: 50,
            origin_chunk: 0x87654321,
            grid_hash_seed: 0x123456789ABCDEF0,
        }),
        _ => None,
    }
}

/// Effect dispatcher: binds phrase kinds to effect masks, stages actions, and
/// evaluates them on demand. No allocation during dispatch; all data fits in fixed arrays.
#[derive(Clone, Debug)]
pub struct EffectDispatcher {
    /// Binding entries indexed by slot (not by phrase_kind).
    pub bindings: [BindingEntry; MAX_BINDINGS],
    /// Pending actions awaiting evaluation.
    pub pending: [Option<SieveAction>; MAX_PENDING],
    /// Next free slot in pending array.
    pub pending_head: u8,
    /// Lifetime count of actions promoted.
    pub dispatched_count: u32,
    /// Threshold below which the budget is considered tight.
    pub budget_threshold_us: u32,
    /// Cached remaining budget in microseconds.
    pub cached_remaining_us: u32,
    /// Count of times dispatch was skipped due to tight budget.
    pub budget_skipped_count: u32,
    /// Sum of `budget_deficit_us()` at every skip — real severity, not just
    /// frequency. A dispatcher that skips twice at a 5us deficit is a
    /// different (much better) situation than one that skips twice at a
    /// 5000us deficit; `budget_skipped_count` alone cannot distinguish them.
    pub budget_deficit_total_us: u64,
}

impl Default for EffectDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectDispatcher {
    /// Create a new dispatcher with canonical Phase-28 bindings.
    pub fn new() -> Self {
        let mut bindings = [BindingEntry::empty(); MAX_BINDINGS];
        bindings[0] = BindingEntry {
            phrase_kind: PHRASE_KIND_MINOR_THIRD_DESCENT,
            effect_mask: EFFECT_INTERACTION_QUERY | EFFECT_MIXER_DELTA | EFFECT_SCENE_AUDIO | EFFECT_GLYPH_RENDER,
            mixer_delta_idx: 1,
            glyph_idx: GLYPH_IDX_GRAVE_BELL,
            scene_event_id: 1,
            _pad: [0; 3],
        };
        bindings[1] = BindingEntry {
            phrase_kind: PHRASE_KIND_SILENT_HOLD,
            effect_mask: EFFECT_MIXER_DELTA | EFFECT_SCENE_AUDIO,
            mixer_delta_idx: 2,
            glyph_idx: 0,
            scene_event_id: 2,
            _pad: [0; 3],
        };
        bindings[2] = BindingEntry {
            phrase_kind: PHRASE_KIND_REFUSAL_REST,
            effect_mask: EFFECT_SCENE_AUDIO | EFFECT_GLYPH_RENDER,
            mixer_delta_idx: 0,
            glyph_idx: GLYPH_IDX_PHRASE_ECHO,
            scene_event_id: 3,
            _pad: [0; 3],
        };
        Self {
            bindings,
            pending: std::array::from_fn(|_| None),
            pending_head: 0,
            dispatched_count: 0,
            budget_threshold_us: DEFAULT_BUDGET_THRESHOLD_US,
            cached_remaining_us: BUDGET_REMAINING_UNSET,
            budget_skipped_count: 0,
            budget_deficit_total_us: 0,
        }
    }

    /// Update the cached remaining budget in microseconds.
    pub fn set_budget_remaining(&mut self, remaining_us: u32) {
        self.cached_remaining_us = remaining_us;
    }

    /// True if the budget is tight (remaining < threshold).
    pub fn budget_tight(&self) -> bool {
        self.cached_remaining_us < self.budget_threshold_us
    }

    /// How far UNDER the threshold the remaining budget sits, in microseconds —
    /// 0 while budget is fine, growing the deeper the overrun. Wired 2026-08-14
    /// onto `forge_core_v3::resolvent::macaulay_pow` (`⟨threshold − remaining⟩¹`,
    /// the discrete ramp): `budget_tight()` alone only ever answered yes/no and
    /// threw the deficit MAGNITUDE away — this is that same threshold crossing,
    /// graded instead of binary, real velocity of the overrun instead of a count.
    pub fn budget_deficit_us(&self) -> u32 {
        forge_core_v3::resolvent::macaulay_pow(
            self.budget_threshold_us as i64,
            self.cached_remaining_us as i64,
            1,
        ) as u32
    }

    /// Get the effect mask for a phrase kind, or 0 if unbound.
    pub fn effect_mask_for(&self, phrase_kind: u8) -> u8 {
        self.lookup(phrase_kind).map(|b| b.effect_mask).unwrap_or(0)
    }

    fn lookup(&self, phrase_kind: u8) -> Option<BindingEntry> {
        for b in &self.bindings {
            if b.phrase_kind == phrase_kind && b.effect_mask != 0 {
                return Some(*b);
            }
        }
        None
    }

    fn stage(&mut self, action: SieveAction) {
        let idx = self.pending_head as usize;
        if idx < MAX_PENDING {
            self.pending[idx] = Some(action);
            self.pending_head += 1;
        }
    }

    /// Observe a semantic binding fired event and stage corresponding actions.
    /// Any other `SieveEvent` variant (e.g. `Mutate5D`, consumed elsewhere by
    /// `forge-arena-v3::mechanic_rail::MechanicRail`) passes through untouched
    /// — the mirror of that consumer's own "foreign event: ignored" rule.
    pub fn observe(&mut self, event: &SieveEvent) {
        let SieveEvent::SemanticBindingFired { phrase_kind, window_us: _, payload_hash } = event else {
            return;
        };
        let binding = match self.lookup(*phrase_kind) {
            Some(b) => b,
            None => return,
        };

        if binding.effect_mask & EFFECT_INTERACTION_QUERY != 0 {
            self.stage(SieveAction::EmitInteractionQuery {
                query_bytes: build_interaction_query(*phrase_kind, *payload_hash),
                phrase_kind: *phrase_kind,
            });
        }

        let budget_tight = self.budget_tight();
        let deficit_us = self.budget_deficit_us();
        if binding.effect_mask & EFFECT_MIXER_DELTA != 0 {
            if budget_tight {
                self.budget_skipped_count = self.budget_skipped_count.saturating_add(1);
                self.budget_deficit_total_us = self.budget_deficit_total_us.saturating_add(deficit_us as u64);
            } else {
                self.stage(SieveAction::ApplyMixerDelta {
                    delta_bytes: build_mixer_delta(binding.mixer_delta_idx),
                    phrase_kind: *phrase_kind,
                });
            }
        }
        if binding.effect_mask & EFFECT_SCENE_AUDIO != 0 {
            if budget_tight {
                self.budget_skipped_count = self.budget_skipped_count.saturating_add(1);
                self.budget_deficit_total_us = self.budget_deficit_total_us.saturating_add(deficit_us as u64);
            } else {
                self.stage(SieveAction::FireSceneAudioEvent {
                    event_id: binding.scene_event_id,
                    intensity_q: 10_000,
                    phrase_kind: *phrase_kind,
                });
            }
        }
        if binding.effect_mask & EFFECT_GLYPH_RENDER != 0 {
            if budget_tight {
                self.budget_skipped_count = self.budget_skipped_count.saturating_add(1);
                self.budget_deficit_total_us = self.budget_deficit_total_us.saturating_add(deficit_us as u64);
            } else {
                let g = lookup_glyph(binding.glyph_idx).unwrap_or_else(GlyphBinding::empty);
                self.stage(SieveAction::GlyphRender {
                    depth_scale: g.depth_scale,
                    origin_chunk: g.origin_chunk,
                    grid_hash: g.grid_hash_seed ^ *payload_hash,
                    phrase_kind: *phrase_kind,
                });
            }
        }
    }

    /// Evaluate: copy all pending actions into the caller-owned fixed array.
    /// Returns the count written; `out[..count]` is live. `out` is
    /// [`MAX_PENDING`]-sized, so evaluation can never overflow it.
    pub fn evaluate(&self, out: &mut [Option<SieveAction>; MAX_PENDING]) -> usize {
        let mut count = 0;
        for slot in &self.pending[..self.pending_head as usize] {
            if let Some(action) = slot {
                out[count] = Some(*action);
                count += 1;
            }
        }
        count
    }

    /// Promote: clear pending queue and increment dispatched count.
    pub fn promote(&mut self) {
        for slot in &mut self.pending[..self.pending_head as usize] {
            *slot = None;
        }
        self.dispatched_count = self.dispatched_count.saturating_add(self.pending_head as u32);
        self.pending_head = 0;
    }

    /// Snapshot the dispatcher state (stub; data is empty in this minimal port).
    pub fn snapshot(&self) -> SieveSnapshot {
        SieveSnapshot { sieve_type: SIEVE_TYPE_EFFECT_DISPATCHER, data: Vec::new() } // COLD-PATH: serialization only
    }

    /// Tick interval: how often to evaluate (stub).
    pub fn tick_interval(&self) -> u32 {
        1
    }
}

fn build_interaction_query(phrase_kind: u8, payload_hash: u64) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0] = semantic_query_tag(phrase_kind);
    bytes[1..9].copy_from_slice(&payload_hash.to_be_bytes());
    bytes
}

fn semantic_query_tag(phrase_kind: u8) -> u8 {
    match phrase_kind {
        PHRASE_KIND_MINOR_THIRD_DESCENT => QUERY_TAG_GRAVE_BELL,
        _ => QUERY_TAG_NONE,
    }
}

fn build_mixer_delta(mixer_delta_idx: u8) -> [u8; 26] {
    let mut bytes = [0u8; 26];
    bytes[0] = mixer_delta_idx;
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fire(phrase_kind: u8) -> SieveEvent {
        SieveEvent::SemanticBindingFired {
            phrase_kind,
            window_us: 0,
            payload_hash: 0xC0FFEE_DEAD_BEEF_u64,
        }
    }

    #[test]
    fn binding_entry_pod_size_is_8_bytes() {
        assert_eq!(std::mem::size_of::<BindingEntry>(), 8);
    }

    // budget_deficit_us is 0 whenever budget isn't tight — matches budget_tight()'s
    // own boundary exactly (both keyed off the same `remaining < threshold` line).
    #[test]
    fn deficit_is_zero_when_budget_is_fine() {
        let mut d = EffectDispatcher::new();
        d.set_budget_remaining(d.budget_threshold_us); // exactly at threshold: not tight
        assert!(!d.budget_tight());
        assert_eq!(d.budget_deficit_us(), 0);
        d.set_budget_remaining(d.budget_threshold_us + 500); // well clear
        assert_eq!(d.budget_deficit_us(), 0);
    }

    // The graded case macaulay_pow exists for: two different overruns both
    // trip budget_tight() (the old binary signal), but only budget_deficit_us
    // tells them apart. This is the actual improvement — proven, not asserted.
    #[test]
    fn deficit_grades_the_overrun_binary_tight_cannot() {
        let mut small = EffectDispatcher::new();
        small.set_budget_remaining(small.budget_threshold_us - 5);
        let mut large = EffectDispatcher::new();
        large.set_budget_remaining(large.budget_threshold_us.saturating_sub(500));

        assert!(small.budget_tight() && large.budget_tight(), "both trip the old binary signal");
        assert_eq!(small.budget_deficit_us(), 5);
        assert_eq!(large.budget_deficit_us(), 500);
        assert!(large.budget_deficit_us() > small.budget_deficit_us(), "the magnitude survives now");
    }

    // Unset budget (BUDGET_REMAINING_UNSET sentinel) must read as zero deficit,
    // same as it already reads as "not tight" — no new inconsistency introduced.
    #[test]
    fn unset_budget_has_zero_deficit() {
        let d = EffectDispatcher::new();
        assert!(!d.budget_tight());
        assert_eq!(d.budget_deficit_us(), 0);
    }

    // observe() under a real tight budget accumulates real deficit magnitude
    // alongside the existing count — both fields stay true to their own name.
    #[test]
    fn observe_accumulates_real_deficit_alongside_the_existing_count() {
        let mut d = EffectDispatcher::new();
        d.set_budget_remaining(d.budget_threshold_us.saturating_sub(200)); // 200us deficit
        d.observe(&fire(PHRASE_KIND_MINOR_THIRD_DESCENT)); // MIXER_DELTA+SCENE_AUDIO+GLYPH_RENDER all skip
        assert_eq!(d.budget_skipped_count, 3, "unchanged existing semantics: one count per skip");
        assert_eq!(d.budget_deficit_total_us, 600, "new: 3 skips x 200us real deficit each");
    }

    #[test]
    fn grave_bell_fans_out_4_actions() {
        let mut d = EffectDispatcher::new();
        d.observe(&fire(PHRASE_KIND_MINOR_THIRD_DESCENT));
        let mut out = [None; MAX_PENDING];
        let n = d.evaluate(&mut out);
        assert_eq!(n, 4);
        assert!(matches!(out[0], Some(SieveAction::EmitInteractionQuery { .. })));
        assert!(matches!(out[1], Some(SieveAction::ApplyMixerDelta { .. })));
        assert!(matches!(out[2], Some(SieveAction::FireSceneAudioEvent { .. })));
        assert!(matches!(out[3], Some(SieveAction::GlyphRender { .. })));
    }

    #[test]
    fn promote_clears_pending() {
        let mut d = EffectDispatcher::new();
        d.observe(&fire(PHRASE_KIND_REFUSAL_REST));
        let mut out = [None; MAX_PENDING];
        let n = d.evaluate(&mut out);
        assert_eq!(n, 2);
        d.promote();
        let n = d.evaluate(&mut [None; MAX_PENDING]);
        assert_eq!(n, 0);
        assert_eq!(d.dispatched_count, 2);
    }

    #[test]
    fn no_match_no_output() {
        let mut d = EffectDispatcher::new();
        d.observe(&fire(99));
        let n = d.evaluate(&mut [None; MAX_PENDING]);
        assert_eq!(n, 0);
    }

    #[test]
    fn staged_actions_carry_their_originating_phrase_kind() {
        let mut d = EffectDispatcher::new();
        d.observe(&fire(PHRASE_KIND_REFUSAL_REST));
        let mut out = [None; MAX_PENDING];
        let n = d.evaluate(&mut out);
        assert_eq!(n, 2, "PHRASE_KIND_REFUSAL_REST produces 2 actions (SCENE_AUDIO + GLYPH_RENDER)");

        // Verify the first action (FireSceneAudioEvent) carries the phrase_kind.
        if let Some(SieveAction::FireSceneAudioEvent { event_id: _, intensity_q: _, phrase_kind }) = out[0] {
            assert_eq!(phrase_kind, PHRASE_KIND_REFUSAL_REST);
        } else {
            panic!("Expected FireSceneAudioEvent at out[0]");
        }

        // Verify the second action (GlyphRender) carries the phrase_kind.
        if let Some(SieveAction::GlyphRender { depth_scale: _, origin_chunk: _, grid_hash: _, phrase_kind }) = out[1] {
            assert_eq!(phrase_kind, PHRASE_KIND_REFUSAL_REST);
        } else {
            panic!("Expected GlyphRender at out[1]");
        }
    }
}
