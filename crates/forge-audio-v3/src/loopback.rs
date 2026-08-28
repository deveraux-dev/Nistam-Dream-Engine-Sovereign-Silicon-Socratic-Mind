//! WASAPI loopback meter — hears what the machine is ALREADY playing.
//!
//! Sean 2026-08-17: "I always have music, even if you just pipe it through…
//! I have a virtual cable too but I'd rather not unless it was seamless."
//! This is the seamless pipe: cpal 0.15's WASAPI backend supports loopback
//! capture by opening an INPUT stream on the default OUTPUT device — the
//! callback receives exactly the samples the speakers are rendering (any
//! player: Spotify, browser, winamp, foobar). No virtual cable, no reroute,
//! nothing to configure; the user's own playback path is untouched.
//!
//! Discipline mirrors `device.rs`: the `cpal::Stream` is `!Send`, so it is
//! opened and held inside a dedicated thread that then parks forever. The
//! only thing that crosses out is a set of relaxed atomics (permyriad
//! levels) — no lock, no allocation after startup. An optional `rtrb`
//! producer (best-effort push, never blocks) feeds `formant_meter`'s
//! off-RT-thread HPSS/LPC worker; that is the one ring in this path.
//!
//! Band split is v1-honest: three one-pole filters (≈200 Hz and ≈2 kHz
//! corners), peak-follower envelopes, published as permyriad. It is a METER,
//! not a spectrum — `fft_buf::AudioFftBuf` is the named upgrade path when a
//! consumer needs real bins. Float math is confined to the audio callback
//! (the same leaf `dsp`/`device` already are); everything published is
//! integer permyriad.
//!
//! Boot-silent law (`device.rs`, Sean 2026-07-24) carried over: a machine
//! with no output device, or a backend refusing loopback, logs ONE line and
//! leaves the meter at zero/not-live — a missing meter is never faked.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Snapshot of the system mix, all permyriad (10000 = full scale).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoopbackLevels {
    /// Overall loudness (peak-follower over |sample|).
    pub rms_pmy: u32,
    /// Bass band (≈ below 200 Hz).
    pub low_pmy: u32,
    /// Mid band (≈ 200 Hz – 2 kHz).
    pub mid_pmy: u32,
    /// Treble band (≈ above 2 kHz).
    pub high_pmy: u32,
    /// True once a real loopback stream is delivering callbacks.
    pub live: bool,
}

/// The shared meter — one writer (the audio callback), any readers.
pub struct LoopbackMeter {
    rms_pmy: AtomicU32,
    low_pmy: AtomicU32,
    mid_pmy: AtomicU32,
    high_pmy: AtomicU32,
    live: AtomicBool,
}

impl LoopbackMeter {
    /// A zeroed, not-live meter, ready to hand to [`spawn`].
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            rms_pmy: AtomicU32::new(0),
            low_pmy: AtomicU32::new(0),
            mid_pmy: AtomicU32::new(0),
            high_pmy: AtomicU32::new(0),
            live: AtomicBool::new(false),
        })
    }

    /// The current levels — relaxed loads, safe from any thread.
    pub fn levels(&self) -> LoopbackLevels {
        LoopbackLevels {
            rms_pmy: self.rms_pmy.load(Ordering::Relaxed),
            low_pmy: self.low_pmy.load(Ordering::Relaxed),
            mid_pmy: self.mid_pmy.load(Ordering::Relaxed),
            high_pmy: self.high_pmy.load(Ordering::Relaxed),
            live: self.live.load(Ordering::Relaxed),
        }
    }
}

/// Envelope gain: music RMS sits well under full scale, so raw envelopes are
/// scaled up before the permyriad clamp to give the visual consumers a
/// usable range. Tunable data, not law.
const LEVEL_GAIN: f32 = 24_000.0;
/// Per-sample envelope decay (peak follower). At 48 kHz this is roughly a
/// 150 ms release — punchy but not strobing.
const ENV_DECAY: f32 = 0.9997;

/// Spawn the loopback thread: open the default OUTPUT device in loopback,
/// meter every callback into `meter`, hold the stream alive forever. Returns
/// immediately; failure is one log line and a zero meter (boot-silent law).
///
/// `formant_meter` is optional: when given, the real device sample rate
/// (known only once the stream opens, inside this thread) spawns
/// `formant_meter`'s worker, and the callback pushes the mono-folded sample
/// into its producer (best-effort — a full ring is dropped, never blocks
/// the RT thread).
pub fn spawn(
    meter: Arc<LoopbackMeter>,
    formant_meter: Option<Arc<crate::formant_meter::FormantMeter>>,
) {
    std::thread::spawn(move || {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            eprintln!("[loopback] no default output device — meter stays zero");
            return;
        };
        let Ok(config) = device.default_output_config() else {
            eprintln!("[loopback] no default output config — meter stays zero");
            return;
        };
        let sample_rate = config.sample_rate().0.max(8_000) as f32;
        let channels = config.channels().max(1) as usize;
        let mut formant_producer =
            formant_meter.map(|fm| crate::formant_meter::spawn(fm, sample_rate as u32));
        // One-pole coefficients for the two corners (bilinear-free v1 form,
        // fine for a meter: a = 1 - e^(-2π·f/fs)).
        let a_low = 1.0 - (-2.0 * core::f32::consts::PI * 200.0 / sample_rate).exp();
        let a_mid = 1.0 - (-2.0 * core::f32::consts::PI * 2_000.0 / sample_rate).exp();
        let mut lp200 = 0.0f32;
        let mut lp2k = 0.0f32;
        let (mut env_rms, mut env_low, mut env_mid, mut env_high) = (0.0f32, 0.0, 0.0, 0.0);
        let cb_meter = Arc::clone(&meter);
        let stream = device.build_input_stream(
            &config.config(),
            move |data: &[f32], _| {
                for frame in data.chunks(channels) {
                    // Mono fold — a meter needs energy, not imaging.
                    let x = frame.iter().copied().sum::<f32>() / channels as f32;
                    if let Some(producer) = formant_producer.as_mut() {
                        let _ = producer.push(x); // best-effort; a full ring drops, never blocks RT
                    }
                    lp200 += a_low * (x - lp200);
                    lp2k += a_mid * (x - lp2k);
                    let low = lp200;
                    let mid = lp2k - lp200;
                    let high = x - lp2k;
                    env_rms = (env_rms * ENV_DECAY).max(x.abs());
                    env_low = (env_low * ENV_DECAY).max(low.abs());
                    env_mid = (env_mid * ENV_DECAY).max(mid.abs());
                    env_high = (env_high * ENV_DECAY).max(high.abs());
                }
                let pmy = |e: f32| ((e * LEVEL_GAIN) as u32).min(10_000);
                cb_meter.rms_pmy.store(pmy(env_rms), Ordering::Relaxed);
                cb_meter.low_pmy.store(pmy(env_low), Ordering::Relaxed);
                cb_meter.mid_pmy.store(pmy(env_mid), Ordering::Relaxed);
                cb_meter.high_pmy.store(pmy(env_high), Ordering::Relaxed);
                cb_meter.live.store(true, Ordering::Relaxed);
            },
            |e| eprintln!("[loopback] stream error: {e}"),
            None,
        );
        match stream {
            Ok(s) => {
                if let Err(e) = s.play() {
                    eprintln!("[loopback] play refused: {e} — meter stays zero");
                    return;
                }
                eprintln!(
                    "[loopback] live — {} Hz, {} ch (WASAPI loopback on the default output)",
                    sample_rate as u32, channels
                );
                // The !Send stream must stay on this thread; park forever.
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                eprintln!("[loopback] loopback refused by backend: {e} — meter stays zero");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The meter is honest at rest: zero and not-live before any stream.
    #[test]
    fn meter_starts_zero_and_not_live() {
        let m = LoopbackMeter::new();
        assert_eq!(m.levels(), LoopbackLevels::default());
        assert!(!m.levels().live);
    }
}
