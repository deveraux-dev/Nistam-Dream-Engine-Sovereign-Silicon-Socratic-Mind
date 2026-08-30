//! The spine face: a photon showing the rollback ring in motion.
//!
//! Runs 13 ticks, induces one rollback mid-run, prints one word-line per frame
//! showing tick count, ring depth, and rollback recovery.

use forge_engine_v3::{ChunkDiffRange, EntityPosition, EntitySnapshot, RollbackRing, MAX_ACTIVE_CHUNKS};

fn main() {
    let mut ring = RollbackRing::new();

    println!("ring starts — depth zero");

    // Record 7 ticks forward (ticks 0 through 6)
    for t in 0..7 {
        let mut snap = EntitySnapshot::default();
        snap.entity_count = 1;
        snap.positions[0] = EntityPosition {
            x: (t as i64) * 100,
            y: 0,
            z: 0,
            health: 100 - t as u16,
            status_bits: 0,
        };

        ring.record_tick(
            t,
            [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
            0,
            snap,
            0,
            t,
        );

        println!(
            "tick {}: ring holds depth {} — entity at x={} health={}",
            t,
            ring.depth(),
            snap.positions[0].x,
            snap.positions[0].health
        );
    }

    // Rollback point: find tick 3
    println!("\n--- rollback induced ---");
    let rollback_target = 3;
    if let Some(frame) = ring.find_by_tick(rollback_target) {
        println!(
            "rewound to tick {} — entity health is {} — depth still {}",
            rollback_target,
            frame.entity_snapshot.positions[0].health,
            ring.depth()
        );
    }

    // Continue forward from tick 7 through 12 (6 more ticks)
    println!("\n--- replay forward ---");
    for t in 7..13 {
        let mut snap = EntitySnapshot::default();
        snap.entity_count = 1;
        snap.positions[0] = EntityPosition {
            x: (t as i64) * 100,
            y: (t as i64) * 50,
            z: 0,
            health: 130 - t as u16,
            status_bits: 1, // Mark post-rollback
        };

        ring.record_tick(
            t,
            [ChunkDiffRange::default(); MAX_ACTIVE_CHUNKS],
            0,
            snap,
            0,
            t,
        );

        println!(
            "tick {}: ring holds depth {} — entity at x={} y={} health={}",
            t,
            ring.depth(),
            snap.positions[0].x,
            snap.positions[0].y,
            snap.positions[0].health
        );
    }

    println!("\nring complete — all ticks served and memory held");
}
