# vixitic

A deterministic tick-reactor async runtime. Wakers bind to **your** integer
simulation clock, not to OS `epoll`/IOCP. Zero dependencies, `std` only,
~400 lines.

```toml
[dependencies]
vixitic = "0.1"
```

## The idea

`async` in Rust is two halves: a compiler-generated state machine, and a
`Waker` contract — *"am I done? no? call this when I should re-check."* Tokio
answers "who calls the waker" with an OS reactor. That is the right answer for
sockets and the wrong answer for a simulation, because it makes every resume
depend on wall-clock time and thread scheduling.

vixitic answers it with an integer tick you supply. A resume is a pure function
of `(condition, event schedule)`. Run the same schedule twice, get a
byte-identical resume log — that is the whole product.

## Two primitives

- `Runtime` — drains a run-queue of tasks. Drive it whole with `block_on`, or
  one tick at a time with `step`.
- `Reactor` — parks `(Cond, Waker)` pairs. `advance(tick, events)` returns the
  wakers whose condition the tick satisfied, **unwoken**, so the caller wakes
  them after dropping the lock (no lock-order inversion).

Conditions are deliberately two:

```rust
pub enum Cond {
    AtTick(u64), // sleep_ticks(n)
    Event(u64),  // await_event(id)
}
```

The event id is yours — a collision cell, an entity handle, a hashed name.
vixitic never interprets it.

## Whole loop

```rust
use vixitic::{sleep_ticks, spawn, Runtime};

let rt = Runtime::new();
let woke = rt.block_on(
    async {
        spawn(async { sleep_ticks(3).await; });
        sleep_ticks(5).await
    },
    |_tick| Vec::new(), // your readiness oracle: tick -> fired event ids
    10,                 // hang-guard
);
assert_eq!(woke, 5);
```

## Inside a frame loop

`step` persists the queue, the clock, and every parked condition across calls,
so a task parked on `sleep_ticks(4)` spans four frames and your own loop *is*
the clock:

```rust
for _ in 0..6 {
    rt.step(|tick| match tick {
        2 => vec![PLAYER_LANDED],
        _ => vec![],
    });
}
```

Run `cargo run --example custom_events` for the full version.

## Design commitments

- **No wall clock.** `Instant::now` appears nowhere in the crate.
- **Stable order.** Wakers fire in registration order, so two tasks waiting on
  the same event always resume in the same order.
- **Loud, never silent.** `block_on` takes a `max_tick` and panics if the root
  has not completed by then. A deterministic runtime that hangs quietly is
  worse than one that crashes.
- **No dependencies.** `[dependencies]` is empty and stays that way.

## Not (yet) in 0.1

- `spawn` is fire-and-forget; there is no `JoinHandle`.
- The executor is single-threaded per drive. The reactor is `Send`-safe to
  advance from another thread, but tasks poll on the driving thread.
- No `no_std` mode yet, despite the crate needing very little of `std`.

## Tests

```
cargo test          # determinism oracles in tests/integration.rs
cargo run --example basic_ticks
cargo run --example custom_events
```

## License

MIT. See [LICENSE](LICENSE). Keep the copyright notice; that is all that is owed.
