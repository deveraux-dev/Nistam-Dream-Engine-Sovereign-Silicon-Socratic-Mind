//! Game-feel juice: declared hit-stop/screenshake presets and an input-to-photon
//! latency probe. No RNG state, no std::time — deterministic, testable, IO-free.
//! forbid-first: all decay/latency math is integer permyriad, no floats.

/// Shake amplitude ceiling (micro-units) — decoration must never outshout the
/// one preattentive alert accent (root CLAUDE.md cognitive-load law).
pub const SHAKE_AMPLITUDE_CEILING_MU: u16 = 4000;

/// Declared hit-stop + screenshake preset. No per-call-site customization.
///
/// `hitstop_frames`: number of frames to freeze input/logic.
/// `shake_amp_mu`: amplitude in MilliUnits.
/// `shake_decay_pmy`: exponential decay factor per frame, in permyriad (0..=10000 = 0.0..1.0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Juice {
    /// Frames to hold input freeze.
    pub hitstop_frames: u8,
    /// Peak shake amplitude in micro-units.
    pub shake_amp_mu: u16,
    /// Per-frame decay multiplier for shake amplitude, in permyriad.
    pub shake_decay_pmy: u16,
}

/// Integer permyriad exponentiation: `(decay_pmy / 10000) ^ t`, result in permyriad.
/// Deterministic, no float — used to decay shake amplitude frame-by-frame.
fn decay_pow_pmy(decay_pmy: u16, t: u8) -> u32 {
    let mut result: u64 = 10_000;
    for _ in 0..t {
        result = result * decay_pmy as u64 / 10_000;
        if result == 0 {
            break;
        }
    }
    result as u32
}

/// Sampled shake offset for a given frame — deterministic, no stored RNG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShakeOffset {
    /// X offset in micro-units.
    pub x_mu: i32,
    /// Y offset in micro-units.
    pub y_mu: i32,
}

impl Juice {
    /// Small taps — barely-there feedback.
    pub const TAP: Juice = Juice { hitstop_frames: 2, shake_amp_mu: 200, shake_decay_pmy: 6000 };
    /// Standard hit — clear but not distracting.
    pub const HIT: Juice = Juice { hitstop_frames: 5, shake_amp_mu: 800, shake_decay_pmy: 5000 };
    /// Critical hit — strong but ceilinged below alert salience.
    pub const CRIT: Juice = Juice { hitstop_frames: 9, shake_amp_mu: 2500, shake_decay_pmy: 4000 };
    /// Death/major event — max juice, still under the ceiling.
    pub const DEATH: Juice = Juice { hitstop_frames: 14, shake_amp_mu: 3800, shake_decay_pmy: 3500 };

    /// Sample deterministic decaying shake offset at frame `t_frames` since trigger.
    /// `seed` selects a fixed direction pattern — no RNG state kept across frames.
    /// Returns a ShakeOffset that decays toward zero as frames advance.
    pub fn sample(&self, t_frames: u8, seed: u32) -> ShakeOffset {
        let amp = self.shake_amp_mu.min(SHAKE_AMPLITUDE_CEILING_MU) as i64;
        let decay_pmy = decay_pow_pmy(self.shake_decay_pmy, t_frames) as i64;
        let dir = ((seed.wrapping_add(t_frames as u32)).wrapping_mul(2654435761)) as i32;
        // frac_pmy: byte 0..255 mapped to -10000..10000 permyriad (i.e. -1.0..1.0).
        let frac_x_pmy = ((((dir >> 16) & 0xFF) as i64) * 2 - 255) * 10_000 / 255;
        let frac_y_pmy = ((((dir >> 8) & 0xFF) as i64) * 2 - 255) * 10_000 / 255;
        let x_mu = frac_x_pmy * amp * decay_pmy / (10_000 * 10_000);
        let y_mu = frac_y_pmy * amp * decay_pmy / (10_000 * 10_000);
        ShakeOffset { x_mu: x_mu as i32, y_mu: y_mu as i32 }
    }
}

/// Fixed-size ring of input->present latency marks. No allocation per frame.
/// Stores up to 256 (id, nanosecond-timestamp) pairs for inputs and presents.
#[derive(Clone, Copy)]
pub struct PhotonProbe {
    /// Input event records: (id, timestamp_ns).
    inputs: [(u64, u128); 256],
    /// Present records: (id, timestamp_ns).
    presents: [(u64, u128); 256],
    /// Total inputs recorded (used modulo 256 for ring index).
    in_len: usize,
    /// Total presents recorded (used modulo 256 for ring index).
    pr_len: usize,
}

/// Latency stats over matched input/present pairs, in whole microseconds
/// (integer — the donor's fractional-millisecond floats are exact multiples
/// of 1us here, no precision lost, no float in core logic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhotonStats {
    /// 50th percentile latency (microseconds).
    pub p50_us: u64,
    /// 99th percentile latency (microseconds).
    pub p99_us: u64,
    /// Maximum latency observed (microseconds).
    pub worst_us: u64,
    /// Number of matched input/present pairs.
    pub n: usize,
}

impl PhotonProbe {
    /// New empty probe.
    pub fn new() -> Self {
        Self { inputs: [(0, 0); 256], presents: [(0, 0); 256], in_len: 0, pr_len: 0 }
    }

    /// Record an input event's timestamp under `id`.
    pub fn mark_input(&mut self, id: u64, t_ns: u128) {
        let i = self.in_len % self.inputs.len();
        self.inputs[i] = (id, t_ns);
        self.in_len += 1;
    }

    /// Record the present timestamp under `id` — matched to `mark_input` by id.
    pub fn mark_present(&mut self, id: u64, t_ns: u128) {
        let i = self.pr_len % self.presents.len();
        self.presents[i] = (id, t_ns);
        self.pr_len += 1;
    }

    /// Compute latency stats over all matched (input, present) id pairs.
    /// Returns percentiles and worst-case latency in microseconds.
    pub fn budget(&self) -> PhotonStats {
        let in_cap = self.in_len.min(self.inputs.len());
        let pr_cap = self.pr_len.min(self.presents.len());
        let mut deltas: [u64; 256] = [0; 256];
        let mut n = 0usize;
        for a in 0..in_cap {
            let (aid, at) = self.inputs[a];
            for b in 0..pr_cap {
                let (bid, bt) = self.presents[b];
                if bid == aid && bt >= at {
                    deltas[n] = ((bt - at) / 1_000) as u64;
                    n += 1;
                    break;
                }
            }
        }
        if n == 0 {
            return PhotonStats { p50_us: 0, p99_us: 0, worst_us: 0, n: 0 };
        }
        let mut sorted = deltas[..n].to_vec();
        sorted.sort_unstable();
        let p50 = sorted[(n - 1) * 50 / 100];
        let p99 = sorted[(n - 1) * 99 / 100];
        let worst = sorted[n - 1];
        PhotonStats { p50_us: p50, p99_us: p99, worst_us: worst, n }
    }
}

impl Default for PhotonProbe {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── L07-style determinism: same seed/time yields same shake ──────────────

    #[test]
    fn shake_sample_is_deterministic() {
        let j = Juice::HIT;
        let off1 = j.sample(5, 42);
        let off2 = j.sample(5, 42);
        assert_eq!(off1, off2, "same seed and frame must yield identical shake");
    }

    #[test]
    fn shake_decay_reaches_zero() {
        let off = Juice::HIT.sample(60, 1);
        assert_eq!(off.x_mu, 0);
        assert_eq!(off.y_mu, 0, "decay should approach zero at high frame counts");
    }

    #[test]
    fn preset_amplitudes_ordered() {
        assert!(Juice::TAP.shake_amp_mu < Juice::HIT.shake_amp_mu);
        assert!(Juice::HIT.shake_amp_mu < Juice::CRIT.shake_amp_mu);
        assert!(Juice::CRIT.shake_amp_mu < Juice::DEATH.shake_amp_mu);
    }

    #[test]
    fn ceiling_holds() {
        assert!(Juice::DEATH.shake_amp_mu <= SHAKE_AMPLITUDE_CEILING_MU);
        let off = Juice::DEATH.sample(0, 7);
        let amp = SHAKE_AMPLITUDE_CEILING_MU as i32;
        assert!(off.x_mu.abs() <= amp && off.y_mu.abs() <= amp);
    }

    #[test]
    fn hitstop_values_increase_with_intensity() {
        assert!(Juice::TAP.hitstop_frames < Juice::HIT.hitstop_frames);
        assert!(Juice::HIT.hitstop_frames < Juice::CRIT.hitstop_frames);
        assert!(Juice::CRIT.hitstop_frames < Juice::DEATH.hitstop_frames);
    }

    // ── L18-style sabotage: flip seed dependency ────────────────────────────
    // If the shake direction did not depend on seed, changing seed would yield
    // the same offset. We verify that different seeds produce different offsets
    // (at the same frame), proving seed affects direction.

    #[test]
    fn shake_changes_with_seed() {
        // t_frames=0: no decay applied yet, so the seed-dependent direction
        // survives integer truncation (at t=10 HIT's decay has crushed the
        // amplitude to under 1 micro-unit for most seeds, which both float
        // and integer paths alike truncate to zero — not a seed-independence
        // bug, just decay doing its job; sampling before decay isolates the
        // seed-dependence this test is actually about).
        let j = Juice::HIT;
        let off_a = j.sample(0, 42);
        let off_b = j.sample(0, 43);
        assert_ne!(off_a, off_b, "different seeds must produce different shake directions");
    }

    #[test]
    fn p50_over_known_marks() {
        let mut p = PhotonProbe::new();
        p.mark_input(1, 0);
        p.mark_present(1, 10_000_000);
        p.mark_input(2, 0);
        p.mark_present(2, 20_000_000);
        let stats = p.budget();
        assert_eq!(stats.n, 2);
        assert!(stats.p50_us > 0);
    }

    #[test]
    fn unmatched_input_never_counts() {
        let mut p = PhotonProbe::new();
        p.mark_input(1, 0);
        p.mark_present(2, 5_000_000);
        let stats = p.budget();
        assert_eq!(stats.n, 0);
    }

    #[test]
    fn probe_ring_wraps() {
        let mut p = PhotonProbe::new();
        // Mark 300 inputs (wraps the 256-slot ring).
        for i in 0..300 {
            p.mark_input(i, i as u128 * 1000);
        }
        // Only the last 256 should be in the ring.
        assert_eq!(p.in_len, 300);
        // Mark presents for the last few.
        p.mark_present(299, 299000 + 5_000_000);
        p.mark_present(298, 298000 + 5_000_000);
        let stats = p.budget();
        assert!(stats.n > 0);
    }

    #[test]
    fn photon_stats_is_deterministic() {
        let mut p1 = PhotonProbe::new();
        p1.mark_input(1, 100);
        p1.mark_present(1, 200);
        let s1 = p1.budget();

        let mut p2 = PhotonProbe::new();
        p2.mark_input(1, 100);
        p2.mark_present(1, 200);
        let s2 = p2.budget();

        assert_eq!(s1, s2, "identical probe states must yield identical stats");
    }
}
