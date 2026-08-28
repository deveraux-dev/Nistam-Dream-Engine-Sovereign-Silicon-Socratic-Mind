//! Driving tasks from your own event ids, one frame at a time:
//! `cargo run --example custom_events`.
//!
//! This is the shape you want inside a game loop: `Runtime::step` is called
//! once per frame, your simulation decides which event ids fired that frame,
//! and the tasks waiting on them resume inside the same frame.

use vixitic::{await_event, sleep_ticks, Runtime};

const PLAYER_LANDED: u64 = 1;
const BOSS_STAGGERED: u64 = 2;

fn main() {
    let rt = Runtime::new();

    rt.spawn_on(async {
        let t = await_event(PLAYER_LANDED).await;
        println!("[tick {t}] player landed -> play dust puff");
        let t = sleep_ticks(2).await;
        println!("[tick {t}] dust puff faded");
    });

    rt.spawn_on(async {
        let t = await_event(BOSS_STAGGERED).await;
        println!("[tick {t}] boss staggered -> open damage window");
    });

    // Your frame loop IS the clock. Six frames, scripted events.
    for _ in 0..6 {
        let tick = rt.step(|tick| match tick {
            2 => vec![PLAYER_LANDED],
            3 => vec![BOSS_STAGGERED],
            _ => vec![],
        });
        println!("--- frame {tick} done ---");
    }
}
