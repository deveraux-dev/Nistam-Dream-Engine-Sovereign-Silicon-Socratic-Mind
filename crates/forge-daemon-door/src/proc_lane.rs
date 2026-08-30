//! Process lifecycle as integer telemetry: one `NiprPackedWord` per watched exe.
//! Publish is lock-free `AtomicU64`; readers never touch Win32 or allocate.
//! The previously published word carries the history the classifier needs.

use crate::winproc;
use forge_hal_clockspine::nipr::{NiprGateStatus, NiprPackedWord};
use std::sync::atomic::{AtomicU64, Ordering};

/// One watched process.
pub struct RosterEntry {
    /// Fixed-width label for the always-on row.
    pub label: &'static str,
    /// Exe name handed to [`winproc::enumerate_by_name`], with or without `.exe`.
    pub exe: &'static str,
    /// Working-set budget in bytes; `pmy_level` is resident bytes as permyriad of this.
    pub budget_bytes: u64,
    /// Instance count above which the lane reads `Fallback`.
    pub max_instances: u32,
}

/// Every process this tree starts.
pub const ROSTER: [RosterEntry; 5] = [
    RosterEntry { label: "DMN", exe: "forgedaemon", budget_bytes: 256 << 20, max_instances: 1 },
    RosterEntry { label: "GEM", exe: "gemma-sidecar", budget_bytes: 8 << 30, max_instances: 1 },
    RosterEntry { label: "SHL", exe: "studio-shell", budget_bytes: 4 << 30, max_instances: 2 },
    RosterEntry { label: "MCP", exe: "forgeMCP", budget_bytes: 512 << 20, max_instances: 4 },
    RosterEntry { label: "PTY", exe: "pwsh", budget_bytes: 2 << 30, max_instances: 16 },
];

/// Roster length; the slot array and every index are checked against it.
pub const ROSTER_LEN: usize = ROSTER.len();

static SLOTS: [AtomicU64; ROSTER_LEN] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

/// What a single poll measured for one lane.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaneObservation {
    /// Live instances found by name.
    pub instances: u32,
    /// Summed resident bytes across those instances.
    pub working_set: u64,
    /// Lane-specific health probe; `None` when the lane has no probe.
    pub probe_ok: Option<bool>,
    /// A pid file for this lane names a process that is not alive.
    pub pid_file_dead: bool,
}

/// Resident bytes as permyriad of `budget`, saturating at 10,000.
pub fn ws_permyriad(bytes: u64, budget: u64) -> u16 {
    if budget == 0 {
        return 0;
    }
    let pmy = bytes.saturating_mul(10_000) / budget;
    if pmy > 10_000 {
        10_000
    } else {
        pmy as u16
    }
}

/// Fold one observation against the previously published status.
///
/// `Init` is "never seen running". A lane that WAS up and is now gone reads
/// `Fault`, not `Init` — an unexpected stop is not the same as never started.
pub fn classify(prev: NiprGateStatus, obs: &LaneObservation, max_instances: u32) -> NiprGateStatus {
    if obs.pid_file_dead {
        return NiprGateStatus::Fault;
    }

    if obs.instances == 0 {
        return match prev {
            NiprGateStatus::Active | NiprGateStatus::Fallback => NiprGateStatus::Fault,
            NiprGateStatus::Fault => NiprGateStatus::Fault,
            NiprGateStatus::Init => NiprGateStatus::Init,
        };
    }

    if obs.instances > max_instances || obs.probe_ok == Some(false) {
        return NiprGateStatus::Fallback;
    }

    NiprGateStatus::Active
}

/// Last published word for `idx`, or `None` when `idx` is off the roster.
pub fn lane(idx: usize) -> Option<NiprPackedWord> {
    SLOTS
        .get(idx)
        .map(|slot| NiprPackedWord::load_atomic(slot, Ordering::Relaxed))
}

/// Every lane's last published word, in roster order.
pub fn snapshot() -> [NiprPackedWord; ROSTER_LEN] {
    let mut out = [NiprPackedWord { raw: 0 }; ROSTER_LEN];
    for (i, slot) in SLOTS.iter().enumerate() {
        out[i] = NiprPackedWord::load_atomic(slot, Ordering::Relaxed);
    }
    out
}

/// Classify `obs` against the stored history, publish, and return the new word.
pub fn publish(idx: usize, obs: &LaneObservation, tick: u16) -> Option<NiprPackedWord> {
    let entry = ROSTER.get(idx)?;
    let slot = SLOTS.get(idx)?;

    let prev = NiprPackedWord::load_atomic(slot, Ordering::Relaxed).gate_status();
    let status = classify(prev, obs, entry.max_instances);
    let pmy = ws_permyriad(obs.working_set, entry.budget_bytes);

    let word = NiprPackedWord::pack(pmy, obs.instances, status, tick);
    word.store_atomic(slot, Ordering::Relaxed);
    Some(word)
}

/// Measure one roster entry through Win32.
pub fn observe(idx: usize, probe_ok: Option<bool>, pid_file_dead: bool) -> Option<LaneObservation> {
    let entry = ROSTER.get(idx)?;
    let instances = winproc::enumerate_by_name(entry.exe);

    let mut working_set = 0u64;
    for (pid, _parent) in &instances {
        if let Some(bytes) = winproc::working_set_bytes(*pid) {
            working_set = working_set.saturating_add(bytes);
        }
    }

    Some(LaneObservation {
        instances: instances.len() as u32,
        working_set,
        probe_ok,
        pid_file_dead,
    })
}

/// Observe and publish every lane once. `door_reachable` feeds the daemon lane's probe.
pub fn poll_once(tick: u16, door_reachable: bool, daemon_pid_file_dead: bool) {
    for idx in 0..ROSTER_LEN {
        let is_daemon = ROSTER[idx].exe == "forgedaemon";
        let probe = if is_daemon { Some(door_reachable) } else { None };
        let dead = is_daemon && daemon_pid_file_dead;

        if let Some(obs) = observe(idx, probe, dead) {
            publish(idx, &obs, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(instances: u32) -> LaneObservation {
        LaneObservation { instances, ..Default::default() }
    }

    #[test]
    fn the_slot_array_matches_the_roster() {
        assert_eq!(SLOTS.len(), ROSTER_LEN);
        assert_eq!(ROSTER.len(), ROSTER_LEN);
    }

    #[test]
    fn a_lane_that_never_ran_stays_init_not_fault() {
        assert_eq!(
            classify(NiprGateStatus::Init, &obs(0), 1),
            NiprGateStatus::Init
        );
    }

    #[test]
    fn a_lane_that_was_up_and_vanished_reads_fault() {
        assert_eq!(
            classify(NiprGateStatus::Active, &obs(0), 1),
            NiprGateStatus::Fault,
            "an unexpected stop is not the same as never started"
        );
        assert_eq!(
            classify(NiprGateStatus::Fallback, &obs(0), 1),
            NiprGateStatus::Fault
        );
    }

    #[test]
    fn fault_latches_until_the_lane_returns() {
        assert_eq!(classify(NiprGateStatus::Fault, &obs(0), 1), NiprGateStatus::Fault);
        assert_eq!(classify(NiprGateStatus::Fault, &obs(1), 1), NiprGateStatus::Active);
    }

    #[test]
    fn a_dead_pid_file_faults_even_while_instances_are_up() {
        let o = LaneObservation { instances: 1, pid_file_dead: true, ..Default::default() };
        assert_eq!(classify(NiprGateStatus::Active, &o, 1), NiprGateStatus::Fault);
    }

    #[test]
    fn a_failing_probe_or_an_instance_flood_reads_fallback() {
        let probe_down = LaneObservation { instances: 1, probe_ok: Some(false), ..Default::default() };
        assert_eq!(classify(NiprGateStatus::Active, &probe_down, 1), NiprGateStatus::Fallback);

        assert_eq!(classify(NiprGateStatus::Active, &obs(3), 1), NiprGateStatus::Fallback);
        assert_eq!(classify(NiprGateStatus::Active, &obs(1), 1), NiprGateStatus::Active);
    }

    #[test]
    fn working_set_scales_to_permyriad_and_saturates() {
        assert_eq!(ws_permyriad(0, 1000), 0);
        assert_eq!(ws_permyriad(500, 1000), 5_000);
        assert_eq!(ws_permyriad(1000, 1000), 10_000);
        assert_eq!(ws_permyriad(9999, 1000), 10_000, "over budget pins, never wraps");
        assert_eq!(ws_permyriad(u64::MAX, 1), 10_000, "no overflow panic");
        assert_eq!(ws_permyriad(5, 0), 0, "a zero budget reports 0, not a divide by zero");
    }

    #[test]
    fn an_off_roster_index_is_none_not_a_panic() {
        assert!(lane(ROSTER_LEN).is_none());
        assert!(publish(ROSTER_LEN, &obs(1), 0).is_none());
        assert!(observe(ROSTER_LEN, None, false).is_none());
    }

    #[test]
    fn publish_round_trips_through_the_atomic() {
        let idx = 0;
        let o = LaneObservation { instances: 1, working_set: 128 << 20, ..Default::default() };
        let word = publish(idx, &o, 77).expect("roster index 0 exists");

        assert_eq!(word.gate_status(), NiprGateStatus::Active);
        assert_eq!(word.dimension_n(), 1);
        assert_eq!(word.sequence_tick(), 77);
        assert_eq!(word.pmy_level(), ws_permyriad(128 << 20, ROSTER[idx].budget_bytes));

        let read_back = lane(idx).expect("slot 0 readable");
        assert_eq!(read_back, word, "the atomic is the one home for lane state");
    }
}
