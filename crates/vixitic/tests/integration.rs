//! Determinism oracles for the public surface: identical schedules must produce
//! identical resume logs, and every resume must land on the exact tick its
//! condition fired.

use std::sync::{Arc, Mutex};

use vixitic::{await_event, sleep_ticks, spawn, Runtime};

/// Run an identical task graph and return the (task id, resume tick) log.
fn run() -> Vec<(u32, u64)> {
    let log = Arc::new(Mutex::new(Vec::<(u32, u64)>::new()));
    let rt = Runtime::new();
    let root_log = log.clone();
    rt.block_on(
        async move {
            // Three tasks await events; two share event 7.
            for (id, ev) in [(1u32, 7u64), (2, 3), (3, 7)] {
                let l = root_log.clone();
                spawn(async move {
                    let woke = await_event(ev).await;
                    l.lock().unwrap().push((id, woke));
                });
            }
            // Root sleeps 10 ticks — keeps the runtime alive past all children.
            let woke = sleep_ticks(10).await;
            root_log.lock().unwrap().push((0, woke));
        },
        // Engine: event 3 fires at tick 2, event 7 at tick 5.
        |tick| match tick {
            2 => vec![3],
            5 => vec![7],
            _ => vec![],
        },
        100,
    );
    Arc::try_unwrap(log).unwrap().into_inner().unwrap()
}

#[test]
fn deterministic_resume_ticks() {
    let a = run();
    let b = run();
    assert_eq!(a, b, "resume ticks must be identical across runs");
    // task 2 (event 3) -> tick 2; tasks 1 & 3 (event 7) -> tick 5; root -> tick 10.
    assert_eq!(a, vec![(2, 2), (1, 5), (3, 5), (0, 10)]);
}

#[test]
fn same_tick_events_fire_in_registration_order() {
    let log = Arc::new(Mutex::new(Vec::<u32>::new()));
    let rt = Runtime::new();
    let rl = log.clone();
    rt.block_on(
        async move {
            for (id, ev) in [(10u32, 100u64), (20, 200), (30, 300)] {
                let l = rl.clone();
                spawn(async move {
                    await_event(ev).await;
                    l.lock().unwrap().push(id);
                });
            }
            let _ = sleep_ticks(6).await;
        },
        // All three events fire on the same tick — order must be registration
        // order, never queue or thread order.
        |tick| {
            if tick == 4 {
                vec![300, 100, 200]
            } else {
                vec![]
            }
        },
        50,
    );
    assert_eq!(*log.lock().unwrap(), vec![10, 20, 30]);
}

#[test]
fn step_parks_across_calls() {
    let rt = Runtime::new();
    let fired = Arc::new(Mutex::new(Vec::<u64>::new()));
    let f = fired.clone();
    rt.spawn_on(async move {
        loop {
            let woke = sleep_ticks(2).await;
            f.lock().unwrap().push(woke);
        }
    });
    let mut pattern = Vec::new();
    for _ in 0..6 {
        let before = fired.lock().unwrap().len();
        rt.step(|_| Vec::new());
        pattern.push(fired.lock().unwrap().len() > before);
    }
    // Cadence 2 fires on even ticks only: the false EXISTS, so the gate is
    // falsifiable rather than always-true.
    assert_eq!(pattern, vec![false, true, false, true, false, true]);
    assert_eq!(*fired.lock().unwrap(), vec![2, 4, 6]);
}

#[test]
fn step_clock_persists_across_calls() {
    let rt = Runtime::new();
    assert_eq!(rt.step(|_| Vec::new()), 1);
    assert_eq!(rt.step(|_| Vec::new()), 2);
    assert_eq!(rt.step(|_| Vec::new()), 3);
    assert_eq!(rt.tick(), 3);
}

#[test]
#[should_panic(expected = "stalled past max_tick")]
fn loud_on_unsatisfiable_condition() {
    let rt = Runtime::new();
    rt.block_on(
        async {
            // Event 999 never fires — must be LOUD, never a silent hang.
            await_event(999).await;
        },
        |_| vec![],
        8,
    );
}
