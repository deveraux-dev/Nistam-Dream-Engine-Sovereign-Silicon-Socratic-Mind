//! Single-tap wave-phase canceller — an L1 adaptive notch that tracks ONE
//! narrowband interferer (mains hum, a resonant rumble) and subtracts an
//! anti-phase estimate, with an explicit freeze so transient events survive.
//!
//! Honest scope: this is a NUMERICAL SIMULATION in f32. It models no converter,
//! no analog front end, and no power. It answers exactly one question — how much
//! a one-tap phase tracker suppresses, separately for background and for events.

/// Two-pi in f32, the phase wrap for the tracked oscillator.
const TAU: f32 = std::f32::consts::TAU;

/// One narrowband interferer tracked by amplitude and phase.
///
/// CIRCULARITY, stated plainly: the canceller READS the incoming signal. Every
/// `step` consumes one sample and adapts from the residual. There is no
/// phase oracle and no separate reference channel — the claim this supports is
/// "cheap sensing" (one sample, two tracked scalars), never "no sensing".
pub struct WavePhaseCanceller {
    /// Tracked interferer frequency in radians/sample.
    omega: f32,
    /// In-phase and quadrature weights — amplitude and phase in Cartesian form,
    /// which keeps the update linear (no atan2 in the loop).
    w_i: f32,
    w_q: f32,
    /// LMS step size. Larger = faster lock, worse steady-state residual.
    eta: f32,
    /// Oscillator phase accumulator.
    phase: f32,
    /// Running mean-square of the residual — the novelty detector's baseline.
    residual_ms: f32,
    /// Leak factor for `residual_ms`; sets the averaging window.
    leak: f32,
    /// Adaptation is inhibited while the residual exceeds this multiple of its
    /// own running RMS. THIS is the selectivity mechanism: an event does not
    /// merely outrun the loop, it actively halts it.
    freeze_sigma: f32,
    /// Samples the loop spent frozen — the receipt that the gate fired.
    pub frozen_samples: u64,
    /// Samples consumed, and the count the gate waits for before arming.
    ///
    /// The freeze gate compares the residual against its own running RMS, so it
    /// is meaningless until that baseline exists: armed from sample 2 it fires
    /// on the acquisition ramp itself (the residual is still the whole input),
    /// freezes the loop permanently, and the canceller never adapts at all.
    seen: u64,
    warmup: u64,
}

impl WavePhaseCanceller {
    /// `f_hz` = interferer frequency, `sr` = sample rate, `eta` = LMS step.
    ///
    /// `leak` is derived from the requested averaging window in samples, so the
    /// time constant is stated in real units by the caller, never implied.
    pub fn new(f_hz: f32, sr: f32, eta: f32, window_samples: usize, freeze_sigma: f32) -> Self {
        Self {
            omega: TAU * f_hz / sr,
            w_i: 0.0,
            w_q: 0.0,
            eta,
            phase: 0.0,
            residual_ms: 0.0,
            leak: 1.0 / window_samples.max(1) as f32,
            freeze_sigma,
            frozen_samples: 0,
            seen: 0,
            warmup: window_samples.max(1) as u64,
        }
    }

    /// The adaptation time constant in samples (1/leak), for the record.
    pub fn tau_samples(&self) -> f32 {
        1.0 / self.leak
    }

    /// Consume one sample, return the residual after anti-phase subtraction.
    pub fn step(&mut self, x: f32) -> f32 {
        let (s, c) = self.phase.sin_cos();
        // The anti-phase estimate: S_anti(t) = w_i·cos + w_q·sin.
        let s_anti = self.w_i * c + self.w_q * s;
        let e = x - s_anti;

        // Novelty gate BEFORE the weight update, so the offending sample never
        // pollutes the weights it triggered on.
        self.seen += 1;
        let rms = self.residual_ms.sqrt();
        let armed = self.seen > self.warmup && rms > 0.0;
        let frozen = armed && e.abs() > self.freeze_sigma * rms;
        if frozen {
            self.frozen_samples += 1;
        } else {
            self.w_i += self.eta * e * c;
            self.w_q += self.eta * e * s;
            self.residual_ms += self.leak * (e * e - self.residual_ms);
        }

        self.phase += self.omega;
        if self.phase > TAU {
            self.phase -= TAU;
        }
        e
    }
}

/// Suppression as a power ratio in dB: `10·log10(Σx² / Σe²)` over the window
/// supplied. Positive = the residual carries less energy than the input.
/// Reported over an explicit slice so background and event windows never share
/// a denominator.
pub fn suppression_db(input: &[f32], residual: &[f32]) -> f32 {
    let p_in: f64 = input.iter().map(|v| (*v as f64) * (*v as f64)).sum();
    let p_out: f64 = residual.iter().map(|v| (*v as f64) * (*v as f64)).sum();
    if p_in <= 0.0 {
        return 0.0; // no input energy — suppression is undefined, not infinite
    }
    if p_out <= 0.0 {
        return f32::INFINITY; // residual is exactly zero: total cancellation
    }
    (10.0 * (p_in / p_out).log10()) as f32
}

/// Deterministic white noise in [-1, 1] — LCG, so every run reproduces.
pub fn white(n: usize, seed: u32) -> Vec<f32> {
    let mut state = seed | 1;
    let mut out = vec![0.0f32; n]; // @forge:allow_alloc — offline analysis, cold path
    for slot in out.iter_mut() {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *slot = (state >> 8) as f32 / (1u32 << 23) as f32 - 1.0;
    }
    out
}

/// Pink (1/f) noise via the Voss-McCartney octave sum — the broadband input a
/// single-tap tracker is NOT expected to handle, included precisely so the
/// failure is measured rather than assumed.
pub fn pink(n: usize, seed: u32) -> Vec<f32> {
    const OCTAVES: usize = 8;
    let src = white(n, seed);
    let mut rows = [0.0f32; OCTAVES];
    let mut out = vec![0.0f32; n]; // @forge:allow_alloc — offline analysis, cold path
    for (i, slot) in out.iter_mut().enumerate() {
        for (o, row) in rows.iter_mut().enumerate() {
            // Octave o updates every 2^o samples.
            if i % (1usize << o) == 0 {
                *row = src[(i + o * 7919) % src.len()];
            }
        }
        *slot = rows.iter().sum::<f32>() / OCTAVES as f32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 1000.0;
    const F_HUM: f32 = 60.0;

    fn hum(n: usize, amp: f32) -> Vec<f32> {
        (0..n).map(|i| amp * (TAU * F_HUM * i as f32 / SR).sin()).collect()
    }

    fn run(input: &[f32], c: &mut WavePhaseCanceller) -> Vec<f32> {
        input.iter().map(|&x| c.step(x)).collect()
    }

    /// The best case the design is entitled to: one stationary tone, exactly the
    /// frequency the tracker was told about.
    #[test]
    fn pure_tone_is_deeply_suppressed() {
        let n = 8000;
        let x = hum(n, 1.0);
        let mut c = WavePhaseCanceller::new(F_HUM, SR, 0.01, 500, 6.0);
        let e = run(&x, &mut c);
        // Measure AFTER lock — the first samples include the acquisition ramp.
        let db = suppression_db(&x[4000..], &e[4000..]);
        eprintln!("[wave-phase] pure tone: {db:.1} dB, tau={} samples", c.tau_samples());
        assert!(db > 40.0, "a stationary tone must suppress hard, got {db:.1} dB");
    }

    /// The phase-error law: subtracting a matched-amplitude sine that is Δφ off
    /// gives residual ratio `2·sin(Δφ/2)`. The consequence engineers care about
    /// is the SIGN CHANGE — beyond 60° the "canceller" adds energy instead of
    /// removing it, and at 180° it doubles the interferer. Measured here by
    /// direct summation, so the boundary is a fact in this repo, not a table.
    #[test]
    fn phase_error_law_and_the_sixty_degree_break_even() {
        let n = 4000;
        for (deg, want_db) in
            [(1.0f32, -35.1f32), (10.0, -15.2), (30.0, -5.7), (60.0, 0.0), (90.0, 3.0), (180.0, 6.0)]
        {
            let phi = deg.to_radians();
            let mut x = vec![0.0f32; n]; // @forge:allow_alloc — offline analysis, cold path
            let mut resid = vec![0.0f32; n]; // @forge:allow_alloc — offline analysis, cold path
            for i in 0..n {
                let t = TAU * F_HUM * i as f32 / SR;
                x[i] = t.sin();
                resid[i] = t.sin() - (t + phi).sin();
            }
            // Negated: suppression_db reports removal as positive, the table
            // reports residual gain, so the two differ by sign.
            let got_db = -suppression_db(&x, &resid);
            eprintln!("[wave-phase] dphi={deg:5.1} deg -> {got_db:+6.2} dB (table {want_db:+.1})");
            assert!(
                (got_db - want_db).abs() < 0.2,
                "phase-error law broke at {deg} deg: {got_db:.2} vs {want_db:.1} dB"
            );
        }
    }

    /// The decisive negative result: broadband 1/f is NOT a single tone, and one
    /// tap cannot cancel it. This test exists to keep that honest — it asserts
    /// the suppression is SMALL, so any future change that quietly claims
    /// broadband performance fails here.
    #[test]
    fn broadband_pink_noise_is_not_suppressed() {
        let n = 8000;
        let x = pink(n, 0xBEEF);
        let mut c = WavePhaseCanceller::new(F_HUM, SR, 0.01, 500, 6.0);
        let e = run(&x, &mut c);
        let db = suppression_db(&x[4000..], &e[4000..]);
        eprintln!("[wave-phase] pink 1/f: {db:.2} dB");
        assert!(
            db < 3.0,
            "one tap must NOT be claimed to cancel broadband 1/f, got {db:.2} dB"
        );
    }

    /// Selectivity, measured as two separate numbers over two separate windows:
    /// suppression on background, and suppression on an injected transient that
    /// must survive. The event is a burst at a different frequency riding the
    /// same hum.
    #[test]
    fn background_suppressed_while_event_survives() {
        let n = 12000;
        let event_at = 8000..8400;
        let mut x = hum(n, 1.0);
        for i in event_at.clone() {
            let t = (i - event_at.start) as f32 / (event_at.len() as f32);
            // Raised-cosine burst at 13 Hz — an "event": brief, off-frequency.
            let env = 0.5 - 0.5 * (TAU * t).cos();
            x[i] += 2.0 * env * (TAU * 13.0 * i as f32 / SR).sin();
        }

        let mut c = WavePhaseCanceller::new(F_HUM, SR, 0.01, 500, 6.0);
        let e = run(&x, &mut c);

        let bg = suppression_db(&x[4000..7000], &e[4000..7000]);
        let ev = suppression_db(&x[event_at.clone()], &e[event_at.clone()]);
        eprintln!(
            "[wave-phase] background {bg:.1} dB · event {ev:.2} dB · frozen {} samples",
            c.frozen_samples
        );
        assert!(bg > 30.0, "background must suppress, got {bg:.1} dB");
        assert!(ev < 6.0, "the event must largely SURVIVE, got {ev:.2} dB");
        assert!(c.frozen_samples > 0, "the freeze gate must actually fire on the event");
    }
}
