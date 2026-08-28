//! Deterministic relative sleeps: `cargo run --example basic_ticks`.
//!
//! No wall clock is consulted anywhere. The runtime advances its own integer
//! clock, so this prints the same lines in the same order on every machine.

use vixitic::{sleep_ticks, spawn, Runtime};

fn main() {
    let rt = Runtime::new();

    let done_at = rt.block_on(
        async {
            println!("root: spawning a child");
            spawn(async {
                println!("  child: sleeping 3 ticks");
                let t = sleep_ticks(3).await;
                println!("  child: resumed on tick {t}");
            });

            println!("root: sleeping 5 ticks");
            sleep_ticks(5).await
        },
        // The engine fires no events this run; only the clock moves.
        |_tick| Vec::new(),
        10, // hang-guard
    );

    println!("root: resumed on tick {done_at}");
    assert_eq!(done_at, 5);
}
