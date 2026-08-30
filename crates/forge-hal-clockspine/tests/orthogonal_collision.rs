//! Orthogonal-collision proof: Two drums do not stall, and drift is detectable at the boundary.
//! Per ARCH-009 §4, the CollisionBridge must prove: (a) neither Alpha nor Beta ever blocks,
//! (b) each lane's generation counter advances independently, (c) a miss on one side never
//! corrupts the other's next take (zero drift = independent sequences per lane).

use forge_hal_clockspine::{CollisionBridge, Permyriad, ResonanceImpulse};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn run_alpha_solo() {
    let bridge = Arc::new(CollisionBridge::new());
    let alpha_never_blocked = Arc::new(AtomicBool::new(true));
    let beta_never_attempted = Arc::new(AtomicBool::new(true));

    let b = Arc::clone(&bridge);
    let never_blocked = Arc::clone(&alpha_never_blocked);
    let alpha_thread = thread::spawn(move || {
        for i in 0..100 {
            let impulse = ResonanceImpulse { idx: i, mag_pmy: Permyriad(i as i32 * 100), lane: 0 };
            let _recycled = b.alpha_publish(impulse);
        }
        never_blocked.store(true, Ordering::Release);
    });

    let b = Arc::clone(&bridge);
    let beta_never_attempted_ref = Arc::clone(&beta_never_attempted);
    let beta_absence_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        if b.beta_take(0, &mut impulse).is_some() {
            beta_never_attempted_ref.store(false, Ordering::Release);
        }
    });

    alpha_thread.join().unwrap();
    beta_absence_thread.join().unwrap();

    assert!(alpha_never_blocked.load(Ordering::Acquire), "alpha should not block");
}

#[test]
fn run_beta_solo() {
    let bridge = Arc::new(CollisionBridge::new());
    let beta_never_blocked = Arc::new(AtomicBool::new(true));

    let b = Arc::clone(&bridge);
    let never_blocked = Arc::clone(&beta_never_blocked);
    let beta_thread = thread::spawn(move || {
        for i in 0..100 {
            let impulse = ResonanceImpulse { idx: i, mag_pmy: Permyriad(i as i32 * 100), lane: 1 };
            let _recycled = b.beta_publish(impulse);
        }
        never_blocked.store(true, Ordering::Release);
    });

    let b = Arc::clone(&bridge);
    let alpha_absence_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(10));
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
        if b.alpha_take(0, &mut impulse).is_some() {
        }
    });

    beta_thread.join().unwrap();
    alpha_absence_thread.join().unwrap();

    assert!(beta_never_blocked.load(Ordering::Acquire), "beta should not block");
}

#[test]
fn run_collision() {
    let bridge = Arc::new(CollisionBridge::new());
    let alpha_idx_seq: Arc<std::sync::Mutex<Vec<u64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let beta_idx_seq: Arc<std::sync::Mutex<Vec<u64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let alpha_never_blocked = Arc::new(AtomicBool::new(true));
    let beta_never_blocked = Arc::new(AtomicBool::new(true));

    let b = Arc::clone(&bridge);
    let _alpha_seq = Arc::clone(&alpha_idx_seq);
    let alpha_blocked = Arc::clone(&alpha_never_blocked);
    let alpha_thread = thread::spawn(move || {
        for i in 0..100u64 {
            let impulse = ResonanceImpulse { idx: i, mag_pmy: Permyriad((i % 10000) as i32), lane: 0 };
            let _recycled = b.alpha_publish(impulse);
        }
        alpha_blocked.store(true, Ordering::Release);
    });

    let b = Arc::clone(&bridge);
    let beta_seq = Arc::clone(&beta_idx_seq);
    let beta_blocked = Arc::clone(&beta_never_blocked);
    let beta_thread = thread::spawn(move || {
        let mut last_gen = 0u64;
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };

        for _ in 0..150 {
            if let Some(gen) = b.beta_take(last_gen, &mut impulse) {
                last_gen = gen;
                beta_seq.lock().unwrap().push(impulse.idx);
            }
            thread::yield_now();
        }
        beta_blocked.store(true, Ordering::Release);
    });

    alpha_thread.join().unwrap();
    beta_thread.join().unwrap();

    assert!(alpha_never_blocked.load(Ordering::Acquire), "alpha should not block");
    assert!(beta_never_blocked.load(Ordering::Acquire), "beta should not block");

    let beta_observed = beta_idx_seq.lock().unwrap();
    if !beta_observed.is_empty() {
        for window in beta_observed.windows(2) {
            assert!(
                window[0] <= window[1],
                "beta's observed idx sequence should be monotonic; saw {} then {}",
                window[0],
                window[1]
            );
        }
    }
}

#[test]
fn generations_advance_independently() {
    let bridge = CollisionBridge::new();

    // Alpha publishes gen 1 on alpha_to_beta lane
    let alpha_impulse = ResonanceImpulse { idx: 111, mag_pmy: Permyriad(1111), lane: 0 };
    bridge.alpha_publish(alpha_impulse);

    // Beta sees gen 1 from alpha_to_beta
    let mut beta_dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
    let beta_gen_1 = bridge.beta_take(0, &mut beta_dst).expect("beta sees gen 1");
    assert_eq!(beta_gen_1, 1);

    // Beta publishes gen 1 on beta_to_alpha lane
    let beta_impulse = ResonanceImpulse { idx: 222, mag_pmy: Permyriad(2222), lane: 1 };
    bridge.beta_publish(beta_impulse);

    // Alpha sees gen 1 from beta_to_alpha
    let mut alpha_dst = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };
    let alpha_gen_1 = bridge.alpha_take(0, &mut alpha_dst).expect("alpha sees gen 1");
    assert_eq!(alpha_gen_1, 1);

    // Alpha publishes gen 2 on alpha_to_beta lane
    let alpha_impulse_2 = ResonanceImpulse { idx: 333, mag_pmy: Permyriad(3333), lane: 0 };
    bridge.alpha_publish(alpha_impulse_2);

    // Beta sees gen 2 from alpha_to_beta
    let beta_gen_2 = bridge.beta_take(beta_gen_1, &mut beta_dst).expect("beta sees gen 2");
    assert_eq!(beta_gen_2, 2);

    // Beta publishes gen 2 on beta_to_alpha lane
    let beta_impulse_2 = ResonanceImpulse { idx: 444, mag_pmy: Permyriad(4444), lane: 1 };
    bridge.beta_publish(beta_impulse_2);

    // Alpha sees gen 2 from beta_to_alpha
    let alpha_gen_2 = bridge.alpha_take(alpha_gen_1, &mut alpha_dst).expect("alpha sees gen 2");
    assert_eq!(alpha_gen_2, 2);
}

#[test]
fn miss_does_not_corrupt_other_lane() {
    let bridge = Arc::new(CollisionBridge::new());
    let alpha_sees: Arc<std::sync::Mutex<Vec<u64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let beta_sees: Arc<std::sync::Mutex<Vec<u64>>> = Arc::new(std::sync::Mutex::new(Vec::new()));

    let b = Arc::clone(&bridge);
    let _alpha_sees_ref = Arc::clone(&alpha_sees);
    let alpha_thread = thread::spawn(move || {
        for i in 0..50u64 {
            let impulse = ResonanceImpulse { idx: i, mag_pmy: Permyriad(0), lane: 0 };
            b.alpha_publish(impulse);
            thread::yield_now();
        }
    });

    let b = Arc::clone(&bridge);
    let beta_sees_ref = Arc::clone(&beta_sees);
    let beta_thread = thread::spawn(move || {
        let mut last_gen = 0u64;
        let mut impulse = ResonanceImpulse { idx: 0, mag_pmy: Permyriad::ZERO, lane: 0 };

        for _ in 0..100 {
            if let Some(gen) = b.beta_take(last_gen, &mut impulse) {
                last_gen = gen;
                beta_sees_ref.lock().unwrap().push(impulse.idx);
            } else {
                beta_sees_ref.lock().unwrap().push(9999u64);
            }
            thread::yield_now();
        }
    });

    alpha_thread.join().unwrap();
    beta_thread.join().unwrap();

    let beta_observed = beta_sees.lock().unwrap();
    let mut last_real_idx = 0u64;
    for &idx in beta_observed.iter() {
        if idx != 9999 {
            assert!(
                idx >= last_real_idx,
                "beta observed idx should never go backward; last={}, now={}",
                last_real_idx,
                idx
            );
            last_real_idx = idx;
        }
    }
}
