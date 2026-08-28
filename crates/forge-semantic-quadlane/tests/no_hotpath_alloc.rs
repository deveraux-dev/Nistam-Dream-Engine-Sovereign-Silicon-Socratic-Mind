//! no_hotpath_alloc gate (COMPILER-GROUPS.md §4 gap 1).
//!
//! The workspace denies `unsafe`, so a counting `GlobalAlloc` is not available
//! to observe allocations at runtime. The gate is therefore structural: the
//! per-tick sources must not name a heap type outside their `#[cfg(test)]`
//! region. A line may carry an explicit `COLD-PATH` marker to declare a
//! serialization-only surface; everything else that mentions a heap type on
//! the hot path turns this test red.

const HOT_SOURCES: [(&str, &str); 3] = [
    ("schedule.rs", include_str!("../src/schedule.rs")),
    ("dispatch.rs", include_str!("../src/dispatch.rs")),
    ("quad_lane.rs", include_str!("../src/quad_lane.rs")),
];

const FORBIDDEN: [&str; 10] = [
    "Vec<",
    "vec!",
    "String",
    "Box<",
    "format!",
    "to_vec",
    "to_string",
    "with_capacity",
    "HashMap",
    "BTreeMap",
];

/// Non-test region of a source file: everything before the first
/// `#[cfg(test)]`. Unit-test modules may allocate freely.
fn non_test_region(src: &str) -> &str {
    match src.find("#[cfg(test)]") {
        Some(idx) => &src[..idx],
        None => src,
    }
}

#[test]
fn hot_path_sources_name_no_heap_type() {
    let mut violations = Vec::new();
    for (name, src) in HOT_SOURCES {
        for (lineno, line) in non_test_region(src).lines().enumerate() {
            if line.contains("COLD-PATH") {
                continue;
            }
            for token in FORBIDDEN {
                if line.contains(token) {
                    violations.push(format!("{name}:{} `{token}` -> {}", lineno + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "heap type named on a hot path (add COLD-PATH only for genuine serialization surfaces):\n{}",
        violations.join("\n")
    );
}

#[test]
fn cold_path_markers_are_enumerated() {
    // The allow-list must stay small and visible. A new COLD-PATH marker is a
    // deliberate act: bump this count in the same change that adds the marker.
    let count: usize = HOT_SOURCES
        .iter()
        .map(|(_, src)| non_test_region(src).lines().filter(|l| l.contains("COLD-PATH")).count())
        .sum();
    assert_eq!(count, 2, "COLD-PATH markers drifted: expected exactly SieveSnapshot's two serialization lines");
}

#[test]
fn full_capacity_tick_cycle_runs_on_fixed_storage() {
    use forge_semantic_quadlane::quad_lane::{Conductor, ExecLane, LaneFanout};
    use forge_semantic_quadlane::schedule::{ScheduleError, SCHEDULE_CAP};

    let mut c = Conductor::new();
    // Fill the schedule to its fixed brim, then confirm the loud refusal.
    for i in 0..SCHEDULE_CAP as u64 {
        c.arm_phrase(i % 3, 1).expect("within capacity");
    }
    assert_eq!(c.arm_phrase(0, 1), Err(ScheduleError::CapacityExceeded));

    // Drain everything in one worst-case tick; all events fan on inline arrays.
    let mut fan = LaneFanout::new();
    c.tick(u64::MAX, &mut fan);
    assert!(fan.total() >= SCHEDULE_CAP, "every due event must fan to at least one lane");

    // Steady state after the burst: cleared fanout, empty schedule, still serviceable.
    fan.clear();
    c.tick(u64::MAX, &mut fan);
    assert_eq!(fan.total(), 0);
    assert_eq!(fan.lane(ExecLane::L0Audio).len(), 0);
}
