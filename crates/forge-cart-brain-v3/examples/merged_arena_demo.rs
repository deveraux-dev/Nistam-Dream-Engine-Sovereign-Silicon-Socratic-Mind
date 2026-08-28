//! MERGED ARENA demo — the single, unified runtime witness that replaces the
//! two prior demos (`arena_cart_demo`, `wolf_run_demo`). `ArenaCart`'s free-2D
//! shape stays; `run_dev_run`'s wolf (`ENT_WOLF`) rides in as a hazard-kind
//! mob via [`forge_cart_brain_v3::ArenaCart::spawn_hazard`], its contact
//! damage scaled by [`forge_cart_brain_v3::run_dev_run::collide`]'s
//! consequence table instead of a flat number. The whole thing is driven by
//! [`forge_cart_brain_v3::tick_loop::TickLoop`] — the actual live caller that
//! unblocks the F02 gap (previously: `ArenaCart` compiled and tested, but
//! nothing outside its own unit tests ever drove it in a loop).
//!
//!   cargo run -p forge-cart-brain-v3 --example merged_arena_demo
//!
//! One brain, one demo, one real caller — this is the fold Sean asked for.

use forge_cart_brain_v3::run_dev_run::ENT_WOLF;
use forge_cart_brain_v3::tick_loop::TickLoop;
use forge_cart_brain_v3::ArenaCart;
use forge_cart_sink_v3::{
    CartInput, CartSession, CartSinks, DeterminismSink, NullEvidence, NullHarmonics, NullMotion, NullVfx,
};

/// Deterministic RNG mirroring the crate's own test/demo pattern.
struct DemoRng(std::cell::Cell<u32>);
impl DemoRng {
    fn new(seed: u32) -> Self {
        Self(std::cell::Cell::new(seed))
    }
}
impl DeterminismSink for DemoRng {
    fn next_u32(&self) -> u32 {
        let mut x = self.0.get().wrapping_add(0x9E37_79B9);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0.set(x);
        x
    }
    fn hash_state(&self, bytes: &[u8]) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325_u64;
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01B3);
        }
        h
    }
}

const SEED: u64 = 0x1EAD_BEEF;
const MAX_TICKS: u64 = 400;

fn run_once(label: &str) -> (u64, usize, Option<u64>) {
    let rng = DemoRng::new(SEED as u32);
    let motion = NullMotion;
    let harmonics = NullHarmonics::default();
    let evidence = NullEvidence;
    let vfx = NullVfx::default();
    let sinks = CartSinks { rng: &rng, motion: &motion, harmonics: &harmonics, evidence: &evidence, vfx: &vfx };

    let mut cart = ArenaCart::new(SEED, 1);
    cart.spawn_hazard(ENT_WOLF, 8_000, 0, 40); // a WOLF hazard, not a generic mob

    println!("== {label} ==  seed=0x{SEED:X}  ENT_WOLF hazard spawned at (8000,0) hp=40");

    let mut loop_ = TickLoop::new(cart);
    for t in 1..=MAX_TICKS {
        loop_.advance(&CartInput { tick: t, buttons: 0, x_vel: 0, y_vel: 0 }, &sinks);
        if let Some(scar) = loop_.cart.step_ai(150, 600, 999, &sinks) {
            println!(
                "  t={t:4}  DEATH -> scar_hash=0x{:016X}  cause={:?}  pressure={}  (damage was collide()-scaled, not flat)",
                scar.scar_hash, scar.cause, loop_.cart.prior_authority_pressure(),
            );
            break;
        }
        if t % 50 == 0 {
            println!(
                "  t={t:4}  loop_ticks={}  player=({:6},{:6})  alive={}",
                loop_.tick_count, loop_.cart.player_x(0), loop_.cart.player_y(0), loop_.cart.is_player_alive(),
            );
        }
    }

    let state_hash = loop_.cart.latest_state_hash();
    println!(
        "  final: loop_ticks={} cart_tick={} scars={} state_hash={:?}",
        loop_.tick_count, loop_.cart.current_tick(), loop_.cart.scar_count(), state_hash,
    );
    (loop_.cart.current_tick(), loop_.cart.scar_count(), state_hash)
}

fn main() {
    let a = run_once("RUN A");
    let b = run_once("RUN B (same seed, replay check)");

    println!("\n== VERDICT ==");
    if a == b {
        println!(
            "  DETERMINISTIC MATCH: tick={} scars={} state_hash={:?}  (TickLoop drove it, wolf is a real hazard, not a second brain)",
            a.0, a.1, a.2,
        );
    } else {
        println!("  !!! MISMATCH: run A {a:?} != run B {b:?} — determinism BROKEN");
        std::process::exit(1);
    }
}
