//! P-Core topology enumeration and soft-pinning via Windows API.
//!
//! Boot-time only — enumerate P-Cores once, then use `set_ideal_processor`
//! at tick boundaries for thermal evacuation.
//!
//! Requires `windows-sys` for the actual FFI calls (behind cfg(windows)).

/// Mapped P-Core topology. Boot-time allocation only.
pub struct PCoreTopology {
    /// Logical processor IDs identified as Performance Cores.
    p_cores: Vec<u32>, // alloc-ok: boot-time only, never touched on hot path
    /// Current round-robin index for evacuation rotation.
    current_index: usize,
}

impl PCoreTopology {
    /// Enumerate P-Cores at boot.
    ///
    /// On Windows: calls `GetLogicalProcessorInformationEx(RelationProcessorCore)`
    /// and filters by `EfficiencyClass` (higher = P-Core).
    ///
    /// Fallback: returns all logical cores if topology detection fails.
    pub fn enumerate() -> Self {
        let p_cores = enumerate_p_cores_platform();
        PCoreTopology {
            p_cores,
            current_index: 0,
        }
    }

    /// Get the first P-Core for initial soft-pinning.
    pub fn initial_core(&self) -> Option<u32> {
        self.p_cores.first().copied()
    }

    /// Round-robin to the next P-Core for thermal evacuation.
    pub fn next_core(&mut self) -> Option<u32> {
        if self.p_cores.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.p_cores.len();
        Some(self.p_cores[self.current_index])
    }

    /// Number of detected P-Cores.
    pub fn count(&self) -> usize {
        self.p_cores.len()
    }

    /// Build from an explicit list (for testing or manual override).
    pub fn from_cores(cores: Vec<u32>) -> Self { // alloc-ok: boot-time / test
        PCoreTopology {
            p_cores: cores,
            current_index: 0,
        }
    }
}

/// Soft-pin the current thread to a logical processor.
///
/// Uses `SetThreadIdealProcessorEx` — a hint, not a hard affinity mask.
/// The OS can still schedule elsewhere under pressure.
///
/// Priority elevation is deliberately NOT included here.
/// Gate TIME_CRITICAL behind a config flag if needed.
pub fn set_ideal_processor(logical_core: u32) {
    set_ideal_processor_platform(logical_core);
}

#[cfg(target_os = "windows")]
fn enumerate_p_cores_platform() -> Vec<u32> { // alloc-ok: boot-time only
    // TODO: Implement via GetLogicalProcessorInformationEx(RelationProcessorCore)
    // Filter cores where EfficiencyClass > 0 (P-Cores on hybrid architectures).
    // For now, return a reasonable default — all cores are treated as P-Cores
    // on non-hybrid CPUs.
    let count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    (0..count).collect() // alloc-ok: boot-time
}

#[cfg(not(target_os = "windows"))]
fn enumerate_p_cores_platform() -> Vec<u32> { // alloc-ok: boot-time only
    let count = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4);
    (0..count).collect() // alloc-ok: boot-time
}

#[cfg(target_os = "windows")]
fn set_ideal_processor_platform(logical_core: u32) {
    // TODO: Call SetThreadIdealProcessorEx(GetCurrentThread(), &PROCESSOR_NUMBER, null)
    // where PROCESSOR_NUMBER.Group = 0, .Number = logical_core as u8.
    //
    // Requires windows-sys dependency. Stubbed for now — the call is a no-op
    // until windows-sys is wired in.
    let _ = logical_core;
}

#[cfg(not(target_os = "windows"))]
fn set_ideal_processor_platform(_logical_core: u32) {
    // No-op on non-Windows platforms.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_returns_nonzero() {
        let topo = PCoreTopology::enumerate();
        assert!(topo.count() > 0);
    }

    #[test]
    fn round_robin_cycles() {
        let mut topo = PCoreTopology::from_cores(vec![5, 6, 7]); // alloc-ok: test
        assert_eq!(topo.next_core(), Some(6));
        assert_eq!(topo.next_core(), Some(7));
        assert_eq!(topo.next_core(), Some(5)); // wraps
    }

    #[test]
    fn initial_core_returns_first() {
        let topo = PCoreTopology::from_cores(vec![5, 6, 7]); // alloc-ok: test
        assert_eq!(topo.initial_core(), Some(5));
    }

    #[test]
    fn empty_topology_returns_none() {
        let mut topo = PCoreTopology::from_cores(vec![]); // alloc-ok: test
        assert_eq!(topo.initial_core(), None);
        assert_eq!(topo.next_core(), None);
    }

    #[test]
    fn set_ideal_processor_does_not_panic() {
        set_ideal_processor(0);
    }
}
