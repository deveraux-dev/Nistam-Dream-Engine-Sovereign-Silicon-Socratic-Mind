//! RestGate -- the Android edge's autonomous coherence governor.
//!
//! Fixed-point Permyriad scale (0..=10_000, 10_000 = 100%). Evaluates the
//! phase alignment of each incoming `UmpPacket64`'s local delta tick against
//! this gate's own loop phase; below `MIN_ALIGN_Q` the packet is REST (zero
//! registers written, zero AAudio dispatch) -- at or above it, AWAKE.
//!
//! Twin of `forge-harmonics::rest_gate::RestGate` (same `MIN_ALIGN_Q = 6000`
//! floor, same `hard_sync_at_anchor` re-seat law: `phase_offset = (len -
//! anchor_tick % len) % len`). Reimplemented standalone here -- not depended
//! on -- so this cdylib's `aarch64-linux-android` build doesn't inherit
//! forge-harmonics' dep chain, none of which is verified to cross-compile
//! for Android. If that changes, drain this module into a thin wrapper over
//! the real one instead.

use crate::router::UmpPacket64;

/// Engine-wide integer fixed-point: 10_000 = 1.0 (100%).
pub type Permyriad = u32;

/// Coherence floor: at or above this, a packet is AWAKE. Below, REST.
pub const MIN_ALIGN_Q: Permyriad = 6_000;

/// Per-tick decision the dispatch loop honors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestVerdict {
    /// Coupled (or a ritual anchor) -- dispatch the packet.
    Awake,
    /// Below the coherence floor and off-anchor -- zero execution, drop.
    Rest,
}

/// One coherence-gated rest bus for the Android link edge.
#[derive(Debug, Clone)]
pub struct RestGate {
    loop_len: u64,
    phase_offset: u64,
    threshold_q: Permyriad,
    woke: u64,
    total: u64,
}

impl RestGate {
    /// `loop_len` sets the phase-alignment period in ticks; clamped to at
    /// least 1 -- a gate always has a heartbeat. Coherence floor defaults to
    /// [`MIN_ALIGN_Q`].
    pub const fn new(loop_len: u64) -> Self {
        Self {
            loop_len: if loop_len == 0 { 1 } else { loop_len },
            phase_offset: 0,
            threshold_q: MIN_ALIGN_Q,
            woke: 0,
            total: 0,
        }
    }

    /// Override the coherence floor (default [`MIN_ALIGN_Q`]).
    pub fn with_threshold(mut self, threshold_q: Permyriad) -> Self {
        self.threshold_q = threshold_q;
        self
    }

    /// `(tick + phase_offset) % loop_len` -- 0 exactly at the tick last
    /// passed to [`Self::hard_sync_at_anchor`] (mod `loop_len`), the shared
    /// downbeat both sides of the link converge on.
    fn loop_phase(&self, tick: u64) -> u64 {
        (tick + self.phase_offset) % self.loop_len
    }

    /// Phase match, Permyriad: 10_000 exactly on the anchor downbeat, linear
    /// falloff to 0 at half a loop away (wrapping either direction).
    /// Integer-only, no alloc.
    pub fn phase_align_q(&self, tick: u64) -> Permyriad {
        let len = self.loop_len;
        let phase = self.loop_phase(tick);
        let dist = phase.min(len - phase);
        let half = (len / 2).max(1);
        10_000u64.saturating_sub(dist * 10_000 / half) as Permyriad
    }

    /// Computes phase match for one packet's local delta tick. `true`
    /// (AWAKE) at or above the coherence floor; `false` (REST -- zero
    /// execution, zero registers written) otherwise. Accrues the
    /// awake/total counters [`Self::awake_q`]/[`Self::rest_q`] read back.
    pub fn evaluate_coherence(&mut self, packet: &UmpPacket64) -> bool {
        self.total += 1;
        let align = self.phase_align_q(packet.timestamp as u64);
        let awake = align >= self.threshold_q;
        if awake {
            self.woke += 1;
        }
        awake
    }

    /// [`Self::evaluate_coherence`] as a named [`RestVerdict`] instead of a bool.
    pub fn verdict(&mut self, packet: &UmpPacket64) -> RestVerdict {
        if self.evaluate_coherence(packet) {
            RestVerdict::Awake
        } else {
            RestVerdict::Rest
        }
    }

    /// Re-seat `phase_offset` so [`Self::phase_align_q`] reads 10_000 at
    /// `anchor_tick` -- the shared ritual anchor downbeat both sides of the
    /// link converge at. Threads sharing an anchor realign there, drift
    /// apart between beats; idempotent -- re-syncing to the same tick twice
    /// leaves `phase_offset` unchanged.
    pub fn hard_sync_at_anchor(&mut self, anchor_tick: u64) {
        let len = self.loop_len;
        self.phase_offset = (len - anchor_tick % len) % len;
    }

    /// Awake fraction over all ticks so far, Permyriad.
    pub fn awake_q(&self) -> Permyriad {
        if self.total == 0 {
            return 0;
        }
        (self.woke * 10_000 / self.total) as Permyriad
    }

    /// Rest fraction -- the sparsity a caller can drop cores against.
    pub fn rest_q(&self) -> Permyriad {
        10_000 - self.awake_q()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkt_at(tick: u16) -> UmpPacket64 {
        UmpPacket64::new(0, tick, 0)
    }

    #[test]
    fn hard_sync_at_anchor_is_idempotent() {
        let mut g = RestGate::new(97);
        g.hard_sync_at_anchor(4200);
        let offset_after_first = g.phase_offset;
        g.hard_sync_at_anchor(4200);
        assert_eq!(
            g.phase_offset, offset_after_first,
            "re-syncing to the same anchor tick must not drift"
        );
        assert_eq!(
            g.phase_align_q(4200),
            10_000,
            "the anchor tick itself is always fully aligned"
        );
    }

    #[test]
    fn hard_sync_moves_to_a_new_anchor_deterministically() {
        let mut g = RestGate::new(97);
        g.hard_sync_at_anchor(10);
        let offset_a = g.phase_offset;
        g.hard_sync_at_anchor(50);
        let offset_b = g.phase_offset;
        assert_ne!(offset_a, offset_b);
        assert_eq!(g.phase_align_q(50), 10_000);
    }

    #[test]
    fn sub_threshold_off_anchor_packets_are_skipped() {
        let mut g = RestGate::new(1_000);
        g.hard_sync_at_anchor(0);
        // Half a loop from the anchor downbeat -> align_q == 0, well under MIN_ALIGN_Q.
        assert!(!g.evaluate_coherence(&pkt_at(500)));
    }

    #[test]
    fn at_or_above_threshold_wakes() {
        let mut g = RestGate::new(1_000);
        g.hard_sync_at_anchor(0);
        assert!(g.evaluate_coherence(&pkt_at(0)));
    }

    #[test]
    fn verdict_matches_evaluate_coherence() {
        let mut g = RestGate::new(1_000).with_threshold(9_000);
        assert_eq!(g.verdict(&pkt_at(500)), RestVerdict::Rest);
        assert_eq!(g.verdict(&pkt_at(0)), RestVerdict::Awake);
    }

    #[test]
    fn deterministic_awake_fraction() {
        let run = || {
            let mut g = RestGate::new(43);
            for beat in 0u16..200 {
                g.evaluate_coherence(&pkt_at(beat));
            }
            (g.awake_q(), g.rest_q())
        };
        assert_eq!(run(), run());
    }
}
