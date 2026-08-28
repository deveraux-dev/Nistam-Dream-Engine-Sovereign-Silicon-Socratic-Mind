#![forbid(unsafe_code)]
//! `forge-cart-brain-v3` — the RunDevRun cart brain (engine owns EXECUTION).
//!
//! Integer-deterministic and edge-portable: it depends ONLY on the pure
//! `forge-cart-sink-v3` seam, so it compiles unchanged to `wasm32-unknown-unknown`
//! (browser) and `wasm32-wasip1` (edge). The live engine is injected via
//! `CartSinks`; this crate never takes a cargo edge on it.
//!
//! PORT RECEIPT (2026-08-16): ported verbatim from `F:\NewRepo\crates\
//! forge-cart-brain` onto the already-landed `forge-cart-sink-v3` seam.
//! Landing in stages — modules are added to this list as each port lands;
//! an unlisted `.rs` file under `src/` is drained work, not a missing file.

pub mod ai;
pub mod audio;
pub mod authority_enforcement;
pub mod building;
pub mod cartridge;
pub mod combat;
pub mod dissonance_sieve;
pub mod faction_mind;
pub mod hermite;
pub mod ledger_drift;
pub mod loot;
pub mod movement;
pub mod resonance_constants;
pub mod run_dev_run;
pub mod skill_book;
pub mod state;
pub mod tempo_run;
pub mod terrain;
pub mod terrain_sieve;
pub mod terrain_waveform;
pub mod tick_loop;
pub mod worms;
pub mod zone_topology;

use forge_cart_sink_v3::{
    CartColor, CartInput, CartRect, CartSession, CartSinks, HarmonicEvent, MotionParams, RenderSink,
};
use loot::{LootItem, LootTable};
use state::{ArenaState, TickFrame, TickRing};

/// Placeholder per-entity render side (mm) until the UI domain (Phase 2).
const ENTITY_SIDE_MM: i64 = 1_000;

/// Mob fill — a distinct danger-red so a mob never reads as the player's tier colour.
const MOB_RGBA: u32 = 0xCC33_22FF;

/// The cart brain. **State domain:** a seed-driven [`ArenaState`] advanced one
/// 120Hz integer tick per [`CartSession::tick`], every frame recorded into the
/// deterministic [`TickRing`] with a `BrutalHash` `state_hash` (via the sink).
/// [`CartSession::render`] samples the latest snapshot — `&self`, never advances
/// (Clock Isolation).
///
/// **Movement domain:** [`movement::MoveState`] holds the current tier (0-10),
/// kill streak, potion counter, and idle-decay timer. Tier controls speed, color,
/// and how much of the haunt drag actually lands. See `movement.rs`.
pub struct ArenaCart {
    state: ArenaState,
    ring: TickRing,
    scars: combat::ScarLedger,
    respawn: combat::RespawnTimer,
    beat: audio::BeatClock,
    beats: u64,
    last_motion: MotionParams,
    move_state: movement::MoveState,
    loot_table: LootTable,
    dropped_loot: Vec<LootItem>,
}

impl ArenaCart {
    /// A fresh arena cart: seeded state, empty ring, spawn-anchored respawn timer.
    pub fn new(seed: u64, player_count: u8) -> Self {
        // Player spawns at arena centre (0,0); half-side offset so top-left
        // corner is at the origin when rendered.
        let half = ENTITY_SIDE_MM / 2;
        Self {
            state: ArenaState::new(seed, player_count),
            ring: TickRing::new(),
            scars: combat::ScarLedger::new(),
            respawn: combat::RespawnTimer::new(-half, -half),
            beat: audio::BeatClock::new(audio::REFERENCE_BPM as u16),
            beats: 0,
            last_motion: MotionParams::default(),
            move_state: movement::MoveState::new(),
            loot_table: LootTable::new(vec![(1, 10), (2, 5), (3, 1)]), // Example loot table
            dropped_loot: Vec::new(),
        }
    }

    /// Current movement tier (0 = Walk … 9 = Redline, 10 = Obliterate while potioned).
    pub fn move_tier(&self) -> u8 {
        self.move_state.tier
    }

    /// True at tier >= 7 (ROCKET): roadrunner direction-change physics.
    pub fn is_spalt(&self) -> bool {
        self.move_state.is_spalt()
    }

    /// Human-readable tier name for the HUD / overlay.
    pub fn tier_name(&self) -> &'static str {
        self.move_state.tier_name()
    }

    /// Register a mob kill — escalates the movement tier, resets idle decay, and rolls for loot.
    /// Call this when a mob is destroyed by the player.
    pub fn mob_killed(&mut self, sinks: &CartSinks) {
        self.move_state.on_kill();
        if let Some(loot) = self.loot_table.roll(sinks.rng) {
            self.dropped_loot.push(loot);
        }
    }

    /// Takes the currently dropped loot, clearing it from the cart.
    pub fn take_dropped_loot(&mut self) -> Vec<LootItem> {
        std::mem::take(&mut self.dropped_loot)
    }

    /// Set the musical tempo (BPM) — speed locks to this (the moat).
    pub fn set_tempo(&mut self, bpm: u16) {
        self.beat = audio::BeatClock::new(bpm);
    }

    /// Beats emitted so far (each fires a 120Hz harmonic event + a motion request).
    pub fn beats_emitted(&self) -> u64 {
        self.beats
    }

    /// The latest tempo-scaled motion params requested on a beat (phrase_motion).
    pub fn last_motion(&self) -> MotionParams {
        self.last_motion
    }

    /// Apply a hazard to an entity. On a death TRANSITION this is the #1 loop:
    /// forge a deterministic scar, seal its provenance, ledger it as Prior
    /// Authority. Returns the scar iff a death occurred this call.
    pub fn apply_hazard(
        &mut self,
        entity: usize,
        damage: i32,
        cause: combat::DeathCause,
        sinks: &CartSinks,
    ) -> Option<combat::DeathScar> {
        if entity >= self.state.count as usize {
            return None;
        }
        let died = combat::apply_damage(&mut self.state.entities[entity].hp, damage);
        if !died {
            return None;
        }
        // Player death (entity 0): drop 2 tiers + cancel potion + start respawn.
        // The scar that forms simultaneously adds haunt drag — compounding punishment.
        if entity == 0 {
            self.move_state.on_death();
            self.respawn.die();
        }
        let e = self.state.entities[entity];
        sinks.vfx.emit_impact(e.x_mm, e.y_mm, 200);
        let scar = combat::forge_scar(
            self.state.seed,
            self.state.tick,
            entity as u64,
            [e.x_mm, e.y_mm],
            cause,
            sinks.rng,
            sinks.evidence,
        );
        self.scars.record(scar);
        Some(scar)
    }

    /// Number of death scars on the ledger (Prior-Authority records).
    pub fn scar_count(&self) -> usize {
        self.scars.count()
    }

    /// Total bounded Prior-Authority pressure all live scars exert this tick.
    pub fn prior_authority_pressure(&self) -> i64 {
        self.scars.total_pressure_at(self.state.tick)
    }

    // ── Respawn ───────────────────────────────────────────────────────────────

    /// True when the player (entity 0) is alive and may move/take damage.
    pub fn is_player_alive(&self) -> bool {
        self.respawn.is_alive()
    }

    /// Ticks until the player respawns (0 when alive).
    /// Host can show a countdown: `ticks / 120` = seconds remaining.
    pub fn respawn_ticks(&self) -> u32 {
        self.respawn.ticks_remaining()
    }

    /// Total player deaths this run (drives the penalty ratchet + scar count).
    pub fn player_deaths(&self) -> u16 {
        self.respawn.deaths()
    }

    /// Restore an entity's hp and alive state — the scar it left behind remains.
    ///
    /// For entity 0 (the player) this also clears the respawn timer so the
    /// player is immediately allowed to move. The brain auto-revives after the
    /// natural timer expires; this override is for tests and emergency host calls.
    pub fn revive(&mut self, entity: usize, hp: i32) {
        if entity < self.state.count as usize {
            self.state.entities[entity].hp = hp;
            if entity == 0 {
                self.respawn.state = combat::RespawnState::Alive;
            }
        }
    }

    /// An entity's world-x (mm) — for tests / host sampling.
    pub fn player_x(&self, entity: usize) -> i64 {
        self.state.entities.get(entity).map(|e| e.x_mm).unwrap_or(0)
    }

    /// An entity's world-y (mm) — for tests / host sampling.
    pub fn player_y(&self, entity: usize) -> i64 {
        self.state.entities.get(entity).map(|e| e.y_mm).unwrap_or(0)
    }

    /// Spawn a generic pursuing mob (AI domain, `kind` 0 — flat contact damage).
    pub fn spawn_mob(&mut self, x_mm: i64, y_mm: i64, hp: i32) {
        self.state.spawn_mob(x_mm, y_mm, hp);
    }

    /// Spawn a hazard-kind mob (`ENT_*` from [`run_dev_run`], e.g.
    /// `run_dev_run::ENT_WOLF`) — its contact damage in [`Self::step_ai`] is
    /// scaled by [`run_dev_run::collide`]'s consequence table instead of the
    /// flat `contact_damage` a generic mob (`kind` 0) uses. This is the fold:
    /// `run_dev_run`'s track-authored hazards (wolves included) become
    /// spawnable mob types in `ArenaCart`'s free-2D arena, not a second brain.
    pub fn spawn_hazard(&mut self, kind: u8, x_mm: i64, y_mm: i64, hp: i32) {
        self.state.spawn_mob_kind(kind, x_mm, y_mm, hp);
    }

    /// Advance mob AI one step: every mob pursues the player; the first mob in
    /// contact deals damage. A generic mob (`kind` 0) deals flat
    /// `contact_damage`; a hazard-kind mob (spawned via [`Self::spawn_hazard`])
    /// scales it through [`run_dev_run::collide`]'s chain-probability table —
    /// the same consequence math `RunDevRun`'s lane-runner used, folded here
    /// instead of kept as a parallel system. A lethal strike fires the #1
    /// death loop ORGANICALLY — forging + ledgering a scar (no manual hazard).
    /// Returns the scar iff the player died this step.
    pub fn step_ai(
        &mut self,
        mob_speed_mm: i64,
        contact_radius_mm: i64,
        contact_damage: i32,
        sinks: &CartSinks,
    ) -> Option<combat::DeathScar> {
        if self.state.count == 0 {
            return None;
        }
        let (px, py) = (self.state.entities[0].x_mm, self.state.entities[0].y_mm);
        for i in 0..self.state.mob_count as usize {
            ai::pursue(&mut self.state.mobs[i], px, py, mob_speed_mm);
        }
        let player = self.state.entities[0];
        let contact_idx = (0..self.state.mob_count as usize)
            .find(|&i| ai::in_contact(&self.state.mobs[i], &player, contact_radius_mm));
        let Some(idx) = contact_idx else {
            return None;
        };
        let mob_kind = self.state.mobs[idx].kind;
        let damage = if mob_kind != 0 {
            let speed_byte = mob_speed_mm.clamp(0, 255) as u8;
            let consequence = run_dev_run::collide(mob_kind, speed_byte);
            contact_damage.saturating_mul(consequence.chain_prob as i32) / 100
        } else {
            contact_damage
        };
        let died = combat::apply_damage(&mut self.state.entities[0].hp, damage);
        if !died {
            return None;
        }
        // Mob-contact kill: drop 2 tiers + cancel potion + start respawn timer.
        self.move_state.on_death();
        self.respawn.die();
        let e = self.state.entities[0];
        sinks.vfx.emit_impact(e.x_mm, e.y_mm, 200);
        let scar = combat::forge_scar(
            self.state.seed,
            self.state.tick,
            0,
            [e.x_mm, e.y_mm],
            combat::DeathCause::Combat,
            sinks.rng,
            sinks.evidence,
        );
        self.scars.record(scar);
        Some(scar)
    }

    /// Deterministic state hash of the latest recorded tick (desync detector).
    pub fn latest_state_hash(&self) -> Option<u64> {
        self.ring.latest().map(|f| f.state_hash)
    }

    /// The replay ring (rollback / replay / desync inspection).
    pub fn ring(&self) -> &TickRing {
        &self.ring
    }

    /// Build a cart from a loaded cartridge config — "cartridge owns meaning."
    pub fn from_cartridge(cfg: &cartridge::CartridgeConfig) -> Self {
        Self::new(cfg.master_seed, cfg.player_count)
    }
}

impl CartSession for ArenaCart {
    fn tick(&mut self, input: &CartInput, sinks: &CartSinks) {
        // Fixed hotpath order (backtick.yaml): potion → decay → authority → physics → record.

        // Respawn timer: tick first so the revive lands BEFORE movement this frame.
        if self.respawn.tick() {
            // Exactly one respawn frame — restore player HP and teleport to spawn.
            let sx = self.respawn.spawn_x_mm;
            let sy = self.respawn.spawn_y_mm;
            if self.state.count > 0 {
                self.state.entities[0].hp = 100;
                self.state.entities[0].x_mm = sx;
                self.state.entities[0].y_mm = sy;
            }
            self.move_state.vel_x_sub = 0;
            self.move_state.vel_y_sub = 0;
        }

        // 808 slam: potion button fires Obliterate (tier 10) for POTION_TICKS.
        if input.buttons & movement::BUTTON_POTION != 0 {
            self.move_state.use_potion();
        }
        // Advance tier decay + potion countdown.
        self.move_state.step();

        // Dead players have zero displacement — the ghost can't move until revived.
        let (dx, dy) = if self.respawn.is_alive() {
            // Resolve Prior Authority: live scar pressure → bounded haunt drag.
            let haunt_drag = self.scars.total_pressure_at(self.state.tick).min(5_000);
            // BDO 2D displacement: diagonal-normalised, momentum-lerped, haunt-dragged.
            self.move_state.displacement_2d(input.x_vel, input.y_vel, haunt_drag)
        } else {
            (0, 0)
        };
        // Advance the deterministic sim one 120Hz integer step.
        self.state.step_raw(input.buttons, dx, dy);

        // Hash + record into the replay ring.
        let state_hash = self.state.state_hash(sinks.rng);
        self.ring.record(TickFrame {
            tick: input.tick,
            snapshot: self.state.snapshot(),
            input_bits: input.buttons,
            state_hash,
        });

        // Speed locked to music: on a beat boundary emit a harmonic event +
        // request tempo-scaled motion (phrase_motion) through the sinks.
        if self.beat.is_beat(self.state.tick) {
            sinks.harmonics.emit(HarmonicEvent::KernelTick);
            self.last_motion = sinks.motion.phrase_motion(audio::tempo_q_from_bpm(self.beat.bpm));
            self.beats += 1;
        }
    }

    fn render(&self, render: &dyn RenderSink) {
        // Sample-only: draw the latest entities; never advance the sim clock.
        // Color = tier: grey (Walk) → yellow (Dash) → magenta (Redline) → white (Obliterate).
        // The color IS the HUD — you read your tier at a glance.
        // Dead entities (hp <= 0) are NOT drawn — the absence IS the visual death signal.
        if let Some(frame) = self.ring.latest() {
            let n = frame.snapshot.count as usize;
            let color = CartColor(self.move_state.color());
            for e in frame.snapshot.entities.iter().take(n) {
                if e.hp > 0 {
                    render.rect(
                        CartRect { x_mm: e.x_mm, y_mm: e.y_mm, w_mm: ENTITY_SIDE_MM, h_mm: ENTITY_SIDE_MM },
                        color,
                    );
                }
            }
        }

        // The AI mobs live in ArenaState (not the tick-ring snapshot) — draw them
        // directly, in a distinct danger-red, so the hunt is visible. Dead mobs
        // (hp <= 0) fall out, the same death-by-absence signal the player uses.
        for m in self.state.mobs.iter().take(self.state.mob_count as usize) {
            if m.hp > 0 {
                render.rect(
                    CartRect { x_mm: m.x_mm, y_mm: m.y_mm, w_mm: ENTITY_SIDE_MM, h_mm: ENTITY_SIDE_MM },
                    CartColor(MOB_RGBA),
                );
            }
        }
    }

    fn current_tick(&self) -> u64 {
        self.state.tick
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_cart_sink_v3::{
        CartInput, CartSinks, NullDeterminism, NullEvidence, NullHarmonics, NullMotion, NullRender,
        NullVfx,
    };

    #[test]
    fn render_draws_the_live_mobs() {
        use core::cell::Cell;
        use forge_cart_sink_v3::{CartColor, CartRect, ImageId, RenderSink};
        struct Counter(Cell<usize>);
        impl RenderSink for Counter {
            fn rect(&self, _r: CartRect, _c: CartColor) { self.0.set(self.0.get() + 1); }
            fn image(&self, _i: ImageId, _r: CartRect) {}
        }
        let tick1 = |cart: &mut ArenaCart| {
            let rng = NullDeterminism::new(1);
            let (motion, harmonics, evidence, vfx) =
                (NullMotion, NullHarmonics::default(), NullEvidence, NullVfx::default());
            let sinks = CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
            cart.tick(&CartInput { tick: 1, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
        };
        let count = |cart: &ArenaCart| { let c = Counter(Cell::new(0)); cart.render(&c); c.0.get() };

        let mut no_mobs = ArenaCart::new(7, 1);
        tick1(&mut no_mobs);
        let mut with_mobs = ArenaCart::new(7, 1);
        with_mobs.spawn_mob(5_000, 0, 50);
        with_mobs.spawn_mob(-5_000, 0, 50);
        tick1(&mut with_mobs);

        // Discriminator: 2 live mobs add exactly 2 rects to the rendered frame.
        // Pre-fix (player-only render) this was 0 — the mobs were invisible.
        assert_eq!(count(&with_mobs), count(&no_mobs) + 2, "render must draw both live mobs");
    }

    /// Drive a cart through an input stream (tick = 1-based index).
    fn drive(cart: &mut ArenaCart, rng: &NullDeterminism, inputs: &[(u16, i8, i8)]) {
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let sinks = CartSinks { rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
        for (i, &(buttons, x_vel, y_vel)) in inputs.iter().enumerate() {
            cart.tick(&CartInput { tick: (i + 1) as u64, buttons, x_vel, y_vel }, &sinks);
        }
    }

    /// A deterministic, varied input stream.
    fn sample_inputs(n: usize) -> Vec<(u16, i8, i8)> {
        (0..n)
            .map(|k| ((k as u16) & 0x3FF, ((k as i8) % 15) - 7, 7 - (k as i8) % 15))
            .collect()
    }

    #[test]
    fn deterministic_replay_two_carts_bit_identical() {
        // THE determinism discriminator: same seed + input stream => identical
        // state_hash sequence (backtick.yaml `deterministic_replay` invariant).
        let inputs = sample_inputs(300);
        let r1 = NullDeterminism::new(0xABCD);
        let r2 = NullDeterminism::new(0xABCD);
        let mut a = ArenaCart::new(0xABCD, 4);
        let mut b = ArenaCart::new(0xABCD, 4);
        drive(&mut a, &r1, &inputs);
        drive(&mut b, &r2, &inputs);
        assert_eq!(a.current_tick(), 300);
        assert_eq!(a.current_tick(), b.current_tick());
        for t in 181..=300u64 {
            // the retained 120-frame window
            assert_eq!(
                a.ring().state_hash(t),
                b.ring().state_hash(t),
                "state_hash diverged at tick {t}"
            );
        }
        assert_eq!(a.latest_state_hash(), b.latest_state_hash());
    }

    #[test]
    fn divergent_input_changes_state_hash() {
        // Discriminator: the hash REFLECTS state — it is not a constant.
        let r1 = NullDeterminism::new(1);
        let r2 = NullDeterminism::new(1);
        let mut a = ArenaCart::new(1, 1);
        let mut b = ArenaCart::new(1, 1);
        drive(&mut a, &r1, &[(0, 5, 0)]); // move +x
        drive(&mut b, &r2, &[(0, -5, 0)]); // move -x
        assert_ne!(
            a.latest_state_hash(),
            b.latest_state_hash(),
            "state_hash must reflect divergent state"
        );
    }

    #[test]
    fn ring_evicts_after_120_ticks() {
        let r = NullDeterminism::new(7);
        let mut cart = ArenaCart::new(7, 1);
        drive(&mut cart, &r, &sample_inputs(130));
        assert_eq!(cart.ring().valid_count(), 120);
        assert!(cart.ring().find_by_tick(5).is_none(), "tick 5 must be evicted");
        assert!(cart.ring().find_by_tick(125).is_some(), "tick 125 must be retained");
    }

    #[test]
    fn render_samples_without_advancing_tick() {
        // The two-clock discriminator at the brain level.
        let r = NullDeterminism::new(3);
        let mut cart = ArenaCart::new(3, 4);
        drive(&mut cart, &r, &sample_inputs(10));
        let before = cart.current_tick();
        let render = NullRender::default();
        for _ in 0..50 {
            cart.render(&render);
        }
        assert_eq!(cart.current_tick(), before, "render must not advance the sim");
        assert_eq!(render.draws.get(), 50 * 4, "render must draw all 4 entities each call");
    }

    #[test]
    fn cartridge_bake_load_drives_identical_determinism() {
        // ADR-0008 bake -> load -> assert: a config sealed + serialized + loaded
        // back must build a cart whose determinism matches one from the live
        // config (the cartridge-owns-meaning / engine-owns-execution round-trip).
        use cartridge::CartridgeConfig;
        let rng_seal = NullDeterminism::new(0);
        let cfg = CartridgeConfig {
            cartridge_id: 7,
            master_seed: 0x77,
            cartridge_hash: 0,
            player_count: 3,
            tick_hz: 120,
        }
        .sealed(&rng_seal);
        let bytes = cfg.to_bytes();
        let loaded = CartridgeConfig::from_bytes(&bytes).expect("valid compiled cartridge");
        assert_eq!(cfg, loaded);

        let r1 = NullDeterminism::new(0x99);
        let r2 = NullDeterminism::new(0x99);
        let mut a = ArenaCart::from_cartridge(&cfg);
        let mut b = ArenaCart::from_cartridge(&loaded);
        let inputs = sample_inputs(150);
        drive(&mut a, &r1, &inputs);
        drive(&mut b, &r2, &inputs);
        assert_eq!(
            a.latest_state_hash(),
            b.latest_state_hash(),
            "a cart from a baked+loaded cartridge must be bit-identical to one from the live config"
        );
    }

    #[test]
    fn cart_death_forges_a_replayable_prior_authority_scar() {
        // The #1 loop end-to-end: lethal hazard -> death -> scar -> sealed ->
        // ledgered -> exerts bounded future pressure (the backtick ` operator).
        let rng = NullDeterminism::new(0x5CA12);
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let sinks = CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
        let mut cart = ArenaCart::new(0x5CA12, 1);
        cart.tick(&CartInput { tick: 1, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
        assert_eq!(cart.scar_count(), 0);
        let scar = cart.apply_hazard(0, 999, combat::DeathCause::Hazard, &sinks);
        assert!(scar.is_some(), "a lethal hazard must forge a scar");
        assert_eq!(cart.scar_count(), 1);
        assert!(cart.prior_authority_pressure() > 0, "the fresh scar exerts pressure on the future");
        // G-GAME-02 wire: the same death that forges a scar also emits VFX.
        assert_eq!(vfx.count.get(), 1, "death must emit exactly one impact VFX event");

        // Replay: an identical run forges the bit-identical scar.
        let rng2 = NullDeterminism::new(0x5CA12);
        let vfx2 = NullVfx::default();
        let sinks2 = CartSinks { rng: &rng2, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx2 };
        let mut cart2 = ArenaCart::new(0x5CA12, 1);
        cart2.tick(&CartInput { tick: 1, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks2);
        let scar2 = cart2.apply_hazard(0, 999, combat::DeathCause::Hazard, &sinks2);
        assert_eq!(scar, scar2, "the death scar must be replay-identical");
    }

    #[test]
    fn a_recent_death_haunts_future_movement() {
        // The executable-memory loop CLOSED: a past death (scar) drags the
        // present — a haunted cart moves LESS far on the same input,
        // deterministically (the past affects the future).
        fn run(with_death: bool) -> i64 {
            let rng = NullDeterminism::new(42);
            let motion = NullMotion;
            let harmonics = NullHarmonics::default();
            let evidence = NullEvidence;
            let vfx = NullVfx::default();
            let sinks =
                CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
            let mut cart = ArenaCart::new(42, 1);
            cart.tick(&CartInput { tick: 1, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
            if with_death {
                cart.apply_hazard(0, 999, combat::DeathCause::Combat, &sinks);
                cart.revive(0, 100); // respawn so it can still move; the scar remains
            }
            for t in 2..=61u64 {
                cart.tick(&CartInput { tick: t, buttons: 0, x_vel: 15, y_vel: 0 }, &sinks);
            }
            cart.player_x(0)
        }
        let unhaunted = run(false);
        let haunted = run(true);
        assert!(
            haunted < unhaunted,
            "a recent death must drag movement (haunted={haunted} unhaunted={unhaunted})"
        );
        assert!(haunted > 0, "the player still moves (the drag is bounded)");
    }

    #[test]
    fn dnb_tempo_emits_more_beats_than_halftime() {
        // The moat at the cart level: at DnB tempo the cart emits beats (each a
        // 120Hz harmonic event + a tempo-scaled motion request) more often than
        // at half-time, over the same run — speed locked to music, deterministic.
        fn beats_for(bpm: u16) -> (u64, u32) {
            let rng = NullDeterminism::new(1);
            let motion = NullMotion;
            let harmonics = NullHarmonics::default();
            let evidence = NullEvidence;
            let vfx = NullVfx::default();
            let sinks =
                CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
            let mut cart = ArenaCart::new(1, 1);
            cart.set_tempo(bpm);
            for t in 1..=600u64 {
                cart.tick(&CartInput { tick: t, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
            }
            (cart.beats_emitted(), harmonics.count.get())
        }
        let (dnb, dnb_emits) = beats_for(170);
        let (half, _) = beats_for(85);
        assert_eq!(dnb, dnb_emits as u64, "every beat emits exactly one harmonic event");
        assert!(dnb > half, "DnB emits more beats than half-time (dnb={dnb} half={half})");
    }

    #[test]
    fn a_mob_pursues_and_organically_kills_the_player() {
        // The gameplay loop CLOSED: a mob chases the player, reaches it, and its
        // contact strike fires the #1 death loop with NO manual hazard.
        let rng = NullDeterminism::new(9);
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let sinks =
            CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
        let mut cart = ArenaCart::new(9, 1);
        cart.spawn_mob(10_000, 0, 50); // a mob 10 m to the player's right
        let mut scar = None;
        for t in 1..=200u64 {
            cart.tick(&CartInput { tick: t, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
            if let Some(s) = cart.step_ai(200, 500, 999, &sinks) {
                scar = Some(s);
                break;
            }
        }
        assert!(scar.is_some(), "the mob caught the player and forged a death scar organically");
        assert_eq!(cart.scar_count(), 1);
        assert!(cart.prior_authority_pressure() > 0, "the organic death haunts the future");
        // G-GAME-02 wire: an organic mob-kill death emits VFX too, no manual hazard.
        assert_eq!(vfx.count.get(), 1, "the organic kill must emit exactly one impact VFX event");
    }

    #[test]
    fn mob_kill_drops_deterministic_loot() {
        let rng = NullDeterminism::new(0);
        let motion = NullMotion;
        let harmonics = NullHarmonics::default();
        let evidence = NullEvidence;
        let vfx = NullVfx::default();
        let sinks =
            CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
        let mut cart = ArenaCart::new(0, 1);
        cart.mob_killed(&sinks);
        cart.mob_killed(&sinks);
        let loot = cart.take_dropped_loot();
        assert_eq!(loot.len(), 2);
        assert_eq!(loot[0].id, 1);
        assert_eq!(loot[1].id, 2);
        assert!(cart.take_dropped_loot().is_empty(), "taking loot should clear it");

        // A different seed should produce different loot
        let rng2 = NullDeterminism::new(15);
        let sinks2 =
            CartSinks { rng: &rng2, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };
        let mut cart2 = ArenaCart::new(0, 1);
        cart2.mob_killed(&sinks2);
        let loot2 = cart2.take_dropped_loot();
        assert_eq!(loot2[0].id, 3);
    }
}
