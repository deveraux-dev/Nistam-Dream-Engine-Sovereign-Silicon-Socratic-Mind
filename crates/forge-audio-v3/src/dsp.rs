//! DSP primitives: load, effects, write.

use std::collections::VecDeque;
use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType, WindowFunction};

/// Mono or stereo audio buffer at a known sample rate.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<Vec<f32>>,
    pub sample_rate: u32,
}

impl AudioBuffer {
    pub fn channels(&self) -> usize { self.samples.len() }
    pub fn len(&self) -> usize { self.samples.first().map(|c| c.len()).unwrap_or(0) }
    pub fn is_empty(&self) -> bool { self.samples.first().is_none_or(|c| c.is_empty()) }
    pub fn duration_secs(&self) -> f32 { self.len() as f32 / self.sample_rate as f32 }
    pub fn to_mono(&self) -> Vec<f32> {
        if self.channels() == 1 { return self.samples[0].clone(); }
        let n = self.len();
        let mut mono = vec![0.0; n];
        let scale = 1.0 / self.channels() as f32;
        for ch in &self.samples { for (i, &s) in ch.iter().enumerate() { mono[i] += s * scale; } }
        mono
    }
}

/// Permyriad vibe snapshot (0–10 000 = 0.0–1.0). Mirrors forge_core::brush::VibeMod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VibeMod {
    pub rms: i32,
    pub low: i32,
    pub mid: i32,
    pub hi: i32,
}

/// Compute a [`VibeMod`] from an offline [`AudioBuffer`].
pub fn audio_snapshot_from_buffer(buf: &AudioBuffer) -> VibeMod {
    let total: usize = buf.samples.iter().map(|ch| ch.len()).sum();
    if total == 0 {
        return VibeMod::default();
    }
    let sum_sq: f64 = buf.samples.iter()
        .flat_map(|ch| ch.iter())
        .map(|&s| (s as f64) * (s as f64))
        .sum();
    let rms_f = (sum_sq / total as f64).sqrt() as f32;
    let rms = (rms_f.clamp(0.0, 1.0) * 10_000.0) as i32;
    VibeMod { rms, low: rms, mid: rms, hi: rms }
}

/// Minimal xorshift64 RNG — replaces forge_core::seed::ForgeRng in offline DSP paths.
struct ForgeRng(u64);
impl ForgeRng {
    fn new(seed: u64) -> Self { Self(seed | 1) }
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    fn next_f32(&mut self) -> f32 { (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32 }
}

pub fn load_wav(path: &str) -> Result<AudioBuffer, String> {
    let reader = hound::WavReader::open(path).map_err(|e| format!("WAV read: {}", e))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let raw: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.into_samples::<f32>().map(|s| s.unwrap_or(0.0)).collect(),
        hound::SampleFormat::Int => {
            let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
            reader.into_samples::<i32>().map(|s| s.unwrap_or(0) as f32 / max).collect()
        }
    };
    let mut samples = vec![Vec::new(); channels];
    for (i, &s) in raw.iter().enumerate() { samples[i % channels].push(s); }
    Ok(AudioBuffer { samples, sample_rate: spec.sample_rate })
}

/// 32-bit float WAV with a PLAIN 16-byte `fmt ` chunk (tag 3 = IEEE float).
///
/// Not hound's writer: for f32 hound emits WAVE_FORMAT_EXTENSIBLE with
/// `dwChannelMask = 0x1` (SPEAKER_FRONT_LEFT) even for one channel. ffmpeg reads
/// that as layout "FL", the AAC encoder rejects a front-left-only mono stream
/// (`-22 Invalid argument`) and writes a 0-byte mp4 — exactly what the 1000-drop
/// soundtrack hit (2026-08-01). A 16-byte header names no layout, so every tool
/// falls back to the channel COUNT and mono stays mono. Load-time export only.
pub fn write_wav(path: &str, buf: &AudioBuffer) -> Result<(), String> {
    let channels = buf.channels().max(1) as u16;
    let frames = buf.len();
    let data_bytes = (frames * channels as usize * 4) as u32;
    let byte_rate = buf.sample_rate * channels as u32 * 4;

    let mut out = Vec::with_capacity(44 + data_bytes as usize); // @forge:allow_alloc load-time export, one buffer
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt size — plain, not extensible
    out.extend_from_slice(&3u16.to_le_bytes()); // WAVE_FORMAT_IEEE_FLOAT
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&buf.sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&(channels * 4).to_le_bytes()); // block align
    out.extend_from_slice(&32u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_bytes.to_le_bytes());
    for i in 0..frames {
        for ch in &buf.samples {
            out.extend_from_slice(&ch.get(i).copied().unwrap_or(0.0).to_le_bytes());
        }
    }
    std::fs::write(path, &out).map_err(|e| format!("WAV write: {}", e)) // @forge:allow_alloc load-time export, error path only
}

#[cfg(test)]
mod wav_header_tests {
    use super::*;

    /// The header ffmpeg can actually mux. Guards the 1000-drop regression:
    /// extensible + mask 0x1 made AAC refuse the stream and the reel came out 0 bytes.
    #[test]
    fn write_wav_emits_a_plain_float_header_no_channel_mask() {
        let path = std::env::temp_dir().join(format!("forge_wav_hdr_{}.wav", std::process::id())); // @forge:allow_alloc test temp path
        let buf = AudioBuffer { samples: vec![vec![0.0, 0.5, -0.5, 0.25]], sample_rate: 44_100 }; // @forge:allow_alloc test fixture
        write_wav(path.to_str().unwrap(), &buf).expect("write");

        let b = std::fs::read(&path).expect("read back"); // @forge:allow_alloc test read
        assert_eq!(&b[0..4], b"RIFF");
        assert_eq!(&b[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(b[16..20].try_into().unwrap()), 16, "fmt chunk must be plain");
        assert_eq!(u16::from_le_bytes(b[20..22].try_into().unwrap()), 3, "IEEE float tag");
        assert_eq!(u16::from_le_bytes(b[22..24].try_into().unwrap()), 1, "mono");
        assert_eq!(u16::from_le_bytes(b[34..36].try_into().unwrap()), 32, "32-bit samples");

        let loaded = load_wav(path.to_str().unwrap()).expect("round trip");
        assert_eq!(loaded.sample_rate, 44_100);
        assert_eq!(loaded.samples[0], buf.samples[0]);
        let _ = std::fs::remove_file(&path);
    }
}

/// Write game-ready audio: 16-bit WAV.
/// For OGG Vorbis export, use external tools (ffmpeg) or wait for a pure-Rust Vorbis encoder.
// TODO: Add OGG Vorbis export when a pure-Rust encoder crate becomes available.
pub fn write_game_audio(path: &str, buf: &AudioBuffer) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: buf.channels() as u16,
        sample_rate: buf.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut w = hound::WavWriter::create(path, spec).map_err(|e| format!("WAV write: {}", e))?;
    // TPDF dither before the 16-bit quantization — decorrelates the truncation
    // error into a benign noise floor (measured: -16.9 dB quantization harmonics
    // on a low-level tone) instead of the DC-biased `as i16` truncate. Fixed seed
    // → the same buffer exports identically.
    let mut rng = 0x9E37_79B9_7F4A_7C15u64;
    for i in 0..buf.len() {
        for ch in &buf.samples {
            let sample = (quantize_i16_dithered(ch[i], &mut rng) * 32767.0).round() as i16;
            w.write_sample(sample).map_err(|e| format!("{}", e))?;
        }
    }
    w.finalize().map_err(|e| format!("{}", e))
}

/// Lossless FLAC encode — the RECORDED FLAC-floor of the unified audio-format law.
///
/// Quantizes f32 → 16-bit PCM and FLAC-compresses it losslessly, so the 16-bit PCM
/// round-trips bit-exact through [`load_audio`]. This is the "compress" step of the
/// track roundtrip. Load-time only — never the realtime callback.
pub fn encode_flac(buf: &AudioBuffer, path: &str) -> Result<(), String> {
    use flacenc::bitsink::ByteSink;
    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    let channels = buf.channels().max(1);
    let frames = buf.len();
    // Interleave + quantize to 16-bit signed PCM (FLAC stores integer PCM).
    let mut interleaved = Vec::<i32>::with_capacity(frames * channels);
    for i in 0..frames {
        for ch in 0..channels {
            let s = buf.samples[ch].get(i).copied().unwrap_or(0.0);
            interleaved.push((s.clamp(-1.0, 1.0) * 32767.0).round() as i32);
        }
    }

    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|_| "flac: invalid encoder config".to_string())?;
    let source =
        flacenc::source::MemSource::from_samples(&interleaved, channels, 16, buf.sample_rate as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, 4096)
        .map_err(|e| format!("flac encode: {e:?}"))?;
    let mut sink = ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| format!("flac serialize: {e:?}"))?;
    std::fs::write(path, sink.as_slice()).map_err(|e| format!("flac save {path}: {e}"))
}

/// Universal audio loader using symphonia. Supports MP3, FLAC, OGG, WAV, and AIFF.
pub fn load_audio(path: &str) -> Result<AudioBuffer, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {}", path, e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("probe {}: {}", path, e))?;

    let mut format_reader = probed.format;

    let track = format_reader
        .default_track()
        .ok_or_else(|| "no default track".to_string())?;
    let sample_rate = track.codec_params.sample_rate.ok_or("no sample rate")?;
    let n_channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(1);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("decoder: {}", e))?;

    let mut interleaved = Vec::<f32>::new();
    loop {
        let packet = match format_reader.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        let duration = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        interleaved.extend_from_slice(sample_buf.samples());
    }

    let mut samples = vec![Vec::new(); n_channels];
    for (i, &s) in interleaved.iter().enumerate() {
        samples[i % n_channels].push(s);
    }

    Ok(AudioBuffer {
        samples,
        sample_rate,
    })
}

pub fn time_stretch(buf: &AudioBuffer, factor: f64) -> Result<AudioBuffer, String> {
    let new_len = (buf.len() as f64 * factor) as usize;
    let samples = buf.samples.iter().map(|ch| {
        (0..new_len).map(|i| {
            let src = i as f64 / factor;
            let idx = src as usize;
            let frac = (src - idx as f64) as f32;
            let a = ch.get(idx).copied().unwrap_or(0.0);
            let b = ch.get(idx + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        }).collect()
    }).collect();
    Ok(AudioBuffer { samples, sample_rate: buf.sample_rate })
}

pub fn pitch_shift(buf: &AudioBuffer, semitones: f32) -> Result<AudioBuffer, String> {
    let ratio = 2.0_f64.powf(semitones as f64 / 12.0);
    let mut stretched = time_stretch(buf, ratio)?;
    stretched.sample_rate = buf.sample_rate;
    Ok(stretched)
}

pub fn reverb(mut buf: AudioBuffer, room_size: f32, damping: f32, mix: f32) -> AudioBuffer {
    let sr = buf.sample_rate as f32;
    let comb_lengths = [(0.0297 * room_size * sr) as usize, (0.0371 * room_size * sr) as usize,
                        (0.0411 * room_size * sr) as usize, (0.0437 * room_size * sr) as usize];
    let ap_lengths = [(0.0050 * sr) as usize, (0.0017 * sr) as usize];
    for ch in &mut buf.samples {
        let mut comb_out = vec![0.0f32; ch.len()];
        for &len in &comb_lengths {
            let mut delay = VecDeque::from(vec![0.0f32; len.max(1)]);
            let mut fs = 0.0f32;
            for i in 0..ch.len() {
                let d = delay.pop_front().unwrap_or(0.0);
                fs = d * (1.0 - damping) + fs * damping;
                delay.push_back(ch[i] + fs * 0.7);
                comb_out[i] += d;
            }
        }
        for &len in &ap_lengths {
            let mut delay = VecDeque::from(vec![0.0f32; len.max(1)]);
            for slot in comb_out.iter_mut() {
                let d = delay.pop_front().unwrap_or(0.0);
                let inp = *slot;
                *slot = d - 0.5 * inp;
                delay.push_back(inp + 0.5 * d);
            }
        }
        for i in 0..ch.len() { ch[i] = ch[i] * (1.0 - mix) + comb_out[i] * mix * 0.25; }
    }
    buf
}

pub fn delay(mut buf: AudioBuffer, time_ms: f32, feedback: f32, mix: f32) -> AudioBuffer {
    let dl = (time_ms * 0.001 * buf.sample_rate as f32) as usize;
    let dl = dl.max(1);
    for ch in &mut buf.samples {
        let mut ring = vec![0.0f32; dl];
        for (pos, sample) in ch.iter_mut().enumerate() {
            let d = ring[pos % dl];
            ring[pos % dl] = *sample + d * feedback;
            *sample = *sample * (1.0 - mix) + d * mix;
        }
    }
    buf
}

pub fn lowpass(mut buf: AudioBuffer, cutoff_hz: f32) -> AudioBuffer {
    let alpha = {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let dt = 1.0 / buf.sample_rate as f32;
        dt / (rc + dt)
    };
    for ch in &mut buf.samples { let mut p = 0.0f32; for s in ch.iter_mut() { *s = p + alpha * (*s - p); p = *s; } }
    buf
}

pub fn highpass(mut buf: AudioBuffer, cutoff_hz: f32) -> AudioBuffer {
    let alpha = {
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
        let dt = 1.0 / buf.sample_rate as f32;
        rc / (rc + dt)
    };
    for ch in &mut buf.samples {
        let (mut pi, mut po) = (0.0f32, 0.0f32);
        for s in ch.iter_mut() { let i = *s; *s = alpha * (po + i - pi); pi = i; po = *s; }
    }
    buf
}

pub fn granular(buf: &AudioBuffer, grain_ms: f32, density: f32, scatter: f32, seed: u64) -> AudioBuffer {
    let mut rng = ForgeRng::new(seed);
    let gl = (grain_ms * 0.001 * buf.sample_rate as f32) as usize;
    let n = buf.len();
    let mut out = AudioBuffer { samples: vec![vec![0.0; n]; buf.channels()], sample_rate: buf.sample_rate };
    let num = (density * n as f32 / gl.max(1) as f32) as usize;
    for _ in 0..num {
        let ss = (rng.next_f32() * n.saturating_sub(gl) as f32) as usize;
        let ds = ((rng.next_f32() * n as f32) as usize + (rng.next_f32() * scatter * n as f32) as usize) % n;
        for i in 0..gl.min(n - ds) {
            let env = hann(i, gl);
            for c in 0..buf.channels() { if ss + i < buf.samples[c].len() { out.samples[c][ds + i] += buf.samples[c][ss + i] * env; } }
        }
    }
    out
}

pub fn reverse(buf: &AudioBuffer) -> AudioBuffer {
    let mut out = buf.clone();
    for ch in &mut out.samples { ch.reverse(); }
    out
}

/// Phase vocoder time stretch (simplified overlap-add with windowed segments).
pub fn phase_vocoder(buf: &AudioBuffer, stretch_factor: f32) -> AudioBuffer {
    let win = 2048usize;
    let hop_in = win / 4;
    let hop_out = (hop_in as f32 * stretch_factor) as usize;
    let hop_out = hop_out.max(1);
    let new_len = (buf.len() as f32 * stretch_factor) as usize;
    let mut out_samples = vec![vec![0.0f32; new_len]; buf.channels()];
    for (ci, ch) in buf.samples.iter().enumerate() {
        let mut out_pos = 0usize;
        let mut in_pos = 0usize;
        while in_pos + win <= ch.len() && out_pos + win <= new_len {
            for j in 0..win {
                let env = hann(j, win);
                out_samples[ci][out_pos + j] += ch[in_pos + j] * env;
            }
            in_pos += hop_in;
            out_pos += hop_out;
        }
    }
    AudioBuffer { samples: out_samples, sample_rate: buf.sample_rate }
}

/// Notch (band-reject) filter.
pub fn notch_filter(buf: &AudioBuffer, freq_hz: f32, q: f32, _gain_db: f32) -> AudioBuffer {
    let mut out = buf.clone();
    let w0 = 2.0 * std::f32::consts::PI * freq_hz / buf.sample_rate as f32;
    let alpha = w0.sin() / (2.0 * q);
    let b0 = 1.0; let b1 = -2.0 * w0.cos(); let b2 = 1.0;
    let a0 = 1.0 + alpha; let a1 = -2.0 * w0.cos(); let a2 = 1.0 - alpha;
    for ch in &mut out.samples { biquad_stateless(ch, b0/a0, b1/a0, b2/a0, a1/a0, a2/a0); }
    out
}

/// LFO amplitude modulation.
pub fn lfo_modulate(buf: &AudioBuffer, rate_hz: f32, depth: f32) -> AudioBuffer {
    let mut out = buf.clone();
    let sr = buf.sample_rate as f32;
    for ch in &mut out.samples {
        for (i, s) in ch.iter_mut().enumerate() {
            let lfo = (2.0 * std::f32::consts::PI * rate_hz * i as f32 / sr).sin();
            *s *= 1.0 + lfo * depth;
        }
    }
    out
}

/// Reverse preswell: slice tail, reverse, prepend.
pub fn reverse_preswell(buf: &AudioBuffer, segment_ms: f32) -> AudioBuffer {
    let seg_len = (segment_ms * 0.001 * buf.sample_rate as f32) as usize;
    if seg_len == 0 || seg_len >= buf.len() { return buf.clone(); }
    let mut out = AudioBuffer { samples: Vec::new(), sample_rate: buf.sample_rate };
    for ch in &buf.samples {
        let tail_start = ch.len() - seg_len;
        let mut tail: Vec<f32> = ch[tail_start..].to_vec();
        tail.reverse();
        for (i, s) in tail.iter_mut().enumerate() { *s *= hann(i, seg_len); }
        let mut combined = tail;
        combined.extend_from_slice(ch);
        out.samples.push(combined);
    }
    out
}

/// Bitcrush: reduce bit depth and sample rate.
pub fn bitcrush(buf: &AudioBuffer, bit_depth: u32, target_rate: u32) -> AudioBuffer {
    let mut out = buf.clone();
    let levels = 2.0f32.powi(bit_depth.clamp(1, 32) as i32);
    let hold_every = (buf.sample_rate / target_rate.max(1)).max(1) as usize;
    for ch in &mut out.samples {
        let mut held = 0.0f32;
        for (i, s) in ch.iter_mut().enumerate() {
            if i % hold_every == 0 { held = (*s * levels).round() / levels; }
            *s = held;
        }
    }
    out
}

/// Bandpass filter (lowpass + highpass).
pub fn bandpass(buf: &AudioBuffer, low_hz: f32, high_hz: f32) -> AudioBuffer {
    highpass(lowpass(buf.clone(), high_hz), low_hz)
}

/// 3-band EQ using biquad shelving/peaking filters (no phase-splitting).
/// Low shelf at 200 Hz, mid peak at 1 kHz (Q=0.7), high shelf at 3 kHz.
/// Audio EQ Cookbook coefficients (Robert Bristow-Johnson).
///
/// `state` is `[low/mid/high][channel]` — persists across blocks for click-free filtering.
pub fn eq_3band(buf: &mut AudioBuffer, low_db: f32, mid_db: f32, high_db: f32, state: &mut [[BiquadState; 2]; 3]) {
    if low_db.abs() < 0.5 && mid_db.abs() < 0.5 && high_db.abs() < 0.5 {
        return;
    }

    let sr = buf.sample_rate as f32;

    if low_db.abs() >= 0.5 {
        let c = biquad_low_shelf(200.0, low_db, 0.7, sr);
        let coeffs = [c.0, c.1, c.2, 1.0, c.3, c.4];
        for (ch_idx, ch) in buf.samples.iter_mut().enumerate() {
            biquad(ch, &coeffs, &mut state[0][ch_idx.min(1)]);
        }
    }

    if mid_db.abs() >= 0.5 {
        let c = biquad_peaking(1000.0, mid_db, 0.7, sr);
        let coeffs = [c.0, c.1, c.2, 1.0, c.3, c.4];
        for (ch_idx, ch) in buf.samples.iter_mut().enumerate() {
            biquad(ch, &coeffs, &mut state[1][ch_idx.min(1)]);
        }
    }

    if high_db.abs() >= 0.5 {
        let c = biquad_high_shelf(3000.0, high_db, 0.7, sr);
        let coeffs = [c.0, c.1, c.2, 1.0, c.3, c.4];
        for (ch_idx, ch) in buf.samples.iter_mut().enumerate() {
            biquad(ch, &coeffs, &mut state[2][ch_idx.min(1)]);
        }
    }
}

/// Offline 3-band EQ (stateless — for non-realtime use like file processing).
pub fn eq_3band_offline(buf: &AudioBuffer, low_db: f32, mid_db: f32, high_db: f32) -> AudioBuffer {
    let mut out = buf.clone();
    let mut state: [[BiquadState; 2]; 3] = Default::default();
    eq_3band(&mut out, low_db, mid_db, high_db, &mut state);
    out
}

/// Persistent biquad filter state — must survive across audio blocks.
#[derive(Default, Clone)]
pub struct BiquadState {
    pub x1: f32, pub x2: f32,
    pub y1: f32, pub y2: f32,
}

fn hann(i: usize, len: usize) -> f32 {
    if len <= 1 { return 1.0; }
    0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (len - 1) as f32).cos())
}

/// Biquad peaking EQ coefficients (Audio EQ Cookbook).
fn biquad_peaking(freq: f32, db_gain: f32, q: f32, sr: f32) -> (f32, f32, f32, f32, f32) {
    let a = 10.0_f32.powf(db_gain / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let alpha = w0.sin() / (2.0 * q);
    let a0 = 1.0 + alpha / a;
    ( (1.0 + alpha * a) / a0,
      (-2.0 * w0.cos()) / a0,
      (1.0 - alpha * a) / a0,
      (-2.0 * w0.cos()) / a0,
      (1.0 - alpha / a) / a0 )
}

/// Biquad low shelf coefficients (Audio EQ Cookbook).
fn biquad_low_shelf(freq: f32, db_gain: f32, q: f32, sr: f32) -> (f32, f32, f32, f32, f32) {
    let a = 10.0_f32.powf(db_gain / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let alpha = w0.sin() / (2.0 * q);
    let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
    let a0 = (a + 1.0) + (a - 1.0) * w0.cos() + two_sqrt_a_alpha;
    ( (a * ((a + 1.0) - (a - 1.0) * w0.cos() + two_sqrt_a_alpha)) / a0,
      (2.0 * a * ((a - 1.0) - (a + 1.0) * w0.cos())) / a0,
      (a * ((a + 1.0) - (a - 1.0) * w0.cos() - two_sqrt_a_alpha)) / a0,
      (-2.0 * ((a - 1.0) + (a + 1.0) * w0.cos())) / a0,
      ((a + 1.0) + (a - 1.0) * w0.cos() - two_sqrt_a_alpha) / a0 )
}

/// Biquad high shelf coefficients (Audio EQ Cookbook).
fn biquad_high_shelf(freq: f32, db_gain: f32, q: f32, sr: f32) -> (f32, f32, f32, f32, f32) {
    let a = 10.0_f32.powf(db_gain / 40.0);
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let alpha = w0.sin() / (2.0 * q);
    let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
    let a0 = (a + 1.0) - (a - 1.0) * w0.cos() + two_sqrt_a_alpha;
    ( (a * ((a + 1.0) + (a - 1.0) * w0.cos() + two_sqrt_a_alpha)) / a0,
      (-2.0 * a * ((a - 1.0) + (a + 1.0) * w0.cos())) / a0,
      (a * ((a + 1.0) + (a - 1.0) * w0.cos() - two_sqrt_a_alpha)) / a0,
      (2.0 * ((a - 1.0) - (a + 1.0) * w0.cos())) / a0,
      ((a + 1.0) - (a - 1.0) * w0.cos() - two_sqrt_a_alpha) / a0 )
}

/// Biquad filter with persistent state (for real-time block-by-block processing).
pub fn biquad(ch: &mut [f32], coeffs: &[f32; 6], state: &mut BiquadState) {
    let (b0, b1, b2, a0, a1, a2) = (coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4], coeffs[5]);
    for s in ch.iter_mut() {
        let x0 = *s;
        let y0 = (b0 * x0 + b1 * state.x1 + b2 * state.x2 - a1 * state.y1 - a2 * state.y2) / a0;
        state.x2 = state.x1; state.x1 = x0;
        state.y2 = state.y1; state.y1 = y0;
        *s = y0;
    }
}

/// Shared rubato sinc-resample path for [`resample`] and [`resample_sinc`].
/// Allocation happens once at construction (resampler filter bank + the
/// returned output buffer) — load-time/offline only, never the RT callback
/// (no call site on the audio thread; verified against every caller in-tree).
fn resample_via_rubato(
    buf: &AudioBuffer,
    target_rate: u32,
    sinc_len: usize,
    oversampling_factor: usize,
    interpolation: SincInterpolationType,
) -> AudioBuffer {
    if buf.sample_rate == target_rate || buf.is_empty() {
        return buf.clone();
    }
    let channels = buf.channels();
    let frames = buf.len();
    let ratio = target_rate as f64 / buf.sample_rate as f64;

    let params = SincInterpolationParameters {
        sinc_len,
        f_cutoff: 0.95,
        oversampling_factor,
        interpolation,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = Async::<f32>::new_sinc(ratio, 1.1, &params, frames, channels, FixedAsync::Input)
        .expect("rubato sinc resampler construction");

    let out_len = resampler.process_all_needed_output_len(frames);
    let mut out_samples: Vec<Vec<f32>> = vec![vec![0.0f32; out_len]; channels];

    let input_adapter = SequentialSliceOfVecs::new(&buf.samples, channels, frames)
        .expect("input adapter dimensions");
    let mut output_adapter = SequentialSliceOfVecs::new_mut(&mut out_samples, channels, out_len)
        .expect("output adapter dimensions");

    let (_, frames_out) = resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, frames, None)
        .expect("rubato resample");

    for ch in &mut out_samples {
        ch.truncate(frames_out);
    }

    AudioBuffer {
        samples: out_samples,
        sample_rate: target_rate,
    }
}

/// Resample an AudioBuffer to the target sample rate. Fast tier: linear
/// sinc interpolation, small filter — good enough for playback normalization.
pub fn resample(buf: &AudioBuffer, target_rate: u32) -> AudioBuffer {
    resample_via_rubato(buf, target_rate, 16, 128, SincInterpolationType::Linear)
}

/// Resample with a windowed-sinc kernel (rubato `Async` sinc resampler).
/// `half_taps` sets filter sharpness (16–32 typical) — mapped to rubato's
/// `sinc_len` (rounded up to a multiple of 8 internally). Anti-aliasing on
/// downsampling is handled internally by rubato (cutoff scaled by the ratio
/// when ratio < 1.0), matching the old hand-rolled band-limit behavior.
pub fn resample_sinc(buf: &AudioBuffer, target_rate: u32, half_taps: usize) -> AudioBuffer {
    let sinc_len = half_taps.max(1) * 2;
    resample_via_rubato(buf, target_rate, sinc_len, 256, SincInterpolationType::Cubic)
}

/// `tanh` soft-clip waveshaper (analog-style saturation) for a master sum that
/// runs hot. A hard clip (`clamp(-1,1)`) on an over-unity 4-deck sum puts a
/// CORNER in the waveform → harsh high-order odd-harmonic splatter; this rounds
/// the knee at ±`threshold` so it saturates musically (far lower THD). Below the
/// threshold it is ~transparent (`tanh(x)≈x`). `drive` (≥ ~0) pre-gains the knee.
/// Memoryless / zero-state; offline or per-block.
pub fn soft_clip(buf: &mut AudioBuffer, threshold: f32, drive: f32) {
    let t = threshold.max(1e-4);
    let d = drive.max(1e-4);
    for ch in &mut buf.samples {
        for s in ch.iter_mut() {
            *s = t * (d * *s / t).tanh();
        }
    }
}

/// xorshift64 → f32 in [0, 1). Deterministic; for dither, not security.
fn xorshift_unit(state: &mut u64) -> f32 {
    let mut x = *state | 1; // never 0
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    ((x >> 40) as f32) / ((1u64 << 24) as f32) // top 24 bits → [0,1)
}

/// Quantize one sample to the 16-bit grid (round-to-nearest), returned as f32.
/// The naive f32→int export: the quantization error is CORRELATED with the
/// signal, so low-level tones gain audible harmonic distortion. Pair with
/// [`quantize_i16_dithered`] to see the difference.
pub fn quantize_i16(s: f32) -> f32 {
    (s * 32767.0).round().clamp(-32768.0, 32767.0) / 32767.0
}

/// Quantize to the 16-bit grid with **TPDF (triangular) dither** — one LSB of
/// triangular noise added before rounding decorrelates the quantization error,
/// trading the harmonic distortion of [`quantize_i16`] for a flat, benign noise
/// floor. This is the correct f32→int export (write_game_audio / FLAC encode).
/// `rng` is an xorshift state advanced per call (deterministic for tests).
pub fn quantize_i16_dithered(s: f32, rng: &mut u64) -> f32 {
    let scaled = s * 32767.0;
    // TPDF = sum of two independent uniforms → triangular in (-1, 1) LSB.
    let tpdf = xorshift_unit(rng) - xorshift_unit(rng);
    (scaled + tpdf).round().clamp(-32768.0, 32767.0) / 32767.0
}

/// Biquad filter with internal (zeroed) state — for offline/full-buffer processing.
fn biquad_stateless(ch: &mut [f32], b0: f32, b1: f32, b2: f32, a1: f32, a2: f32) {
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for s in ch.iter_mut() {
        let x0 = *s;
        let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1; x1 = x0; y2 = y1; y1 = y0;
        *s = y0;
    }
}

/// Compute 3-band waveform energy at `resolution`-sample windows.
/// Returns Vec<[low, mid, high]> with each value normalized 0.0–1.0.
/// Bands: low <200Hz, mid 200–4000Hz, high >4000Hz.
pub fn compute_waveform_bands(buf: &AudioBuffer, resolution: usize) -> Vec<[f32; 3]> {
    let mono = buf.to_mono();
    if mono.is_empty() || resolution == 0 { return vec![]; }
    let sr = buf.sample_rate as f32;
    let chunk = resolution.max(1);
    let n_windows = mono.len().div_ceil(chunk);

    let lp200 = biquad_lp_coeffs(200.0, 0.707, sr);
    let hp200 = biquad_hp_coeffs(200.0, 0.707, sr);
    let lp4000 = biquad_lp_coeffs(4000.0, 0.707, sr);
    let hp4000 = biquad_hp_coeffs(4000.0, 0.707, sr);

    let mut low_sig = mono.clone();
    let mut mid_sig = mono.clone();
    let mut high_sig = mono.clone();

    biquad_apply(&mut low_sig, &lp200);

    biquad_apply(&mut mid_sig, &hp200);
    biquad_apply(&mut mid_sig, &lp4000);

    biquad_apply(&mut high_sig, &hp4000);

    let mut result = Vec::with_capacity(n_windows);
    let mut max_e = [0.0f32; 3];
    for w in 0..n_windows {
        let start = w * chunk;
        let end = (start + chunk).min(mono.len());
        let n = (end - start) as f32;
        if n <= 0.0 { result.push([0.0; 3]); continue; }
        let lo: f32 = low_sig[start..end].iter().map(|s| s * s).sum::<f32>() / n;
        let mi: f32 = mid_sig[start..end].iter().map(|s| s * s).sum::<f32>() / n;
        let hi: f32 = high_sig[start..end].iter().map(|s| s * s).sum::<f32>() / n;
        let e = [lo.sqrt(), mi.sqrt(), hi.sqrt()];
        for b in 0..3 { if e[b] > max_e[b] { max_e[b] = e[b]; } }
        result.push(e);
    }
    for r in &mut result {
        for b in 0..3 {
            if max_e[b] > 0.0 { r[b] /= max_e[b]; }
        }
    }
    result
}

/// Simple 2nd-order lowpass biquad coefficients.
fn biquad_lp_coeffs(freq: f32, q: f32, sr: f32) -> [f32; 5] {
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let alpha = w0.sin() / (2.0 * q);
    let cos_w0 = w0.cos();
    let a0 = 1.0 + alpha;
    let b0 = ((1.0 - cos_w0) / 2.0) / a0;
    let b1 = (1.0 - cos_w0) / a0;
    let b2 = b0;
    let a1 = (-2.0 * cos_w0) / a0;
    let a2 = (1.0 - alpha) / a0;
    [b0, b1, b2, a1, a2]
}

/// Simple 2nd-order highpass biquad coefficients.
fn biquad_hp_coeffs(freq: f32, q: f32, sr: f32) -> [f32; 5] {
    let w0 = 2.0 * std::f32::consts::PI * freq / sr;
    let alpha = w0.sin() / (2.0 * q);
    let cos_w0 = w0.cos();
    let a0 = 1.0 + alpha;
    let b0 = ((1.0 + cos_w0) / 2.0) / a0;
    let b1 = (-(1.0 + cos_w0)) / a0;
    let b2 = b0;
    let a1 = (-2.0 * cos_w0) / a0;
    let a2 = (1.0 - alpha) / a0;
    [b0, b1, b2, a1, a2]
}

/// Apply biquad filter in-place (stateless, single pass).
fn biquad_apply(samples: &mut [f32], coeffs: &[f32; 5]) {
    let (b0, b1, b2, a1, a2) = (coeffs[0], coeffs[1], coeffs[2], coeffs[3], coeffs[4]);
    let (mut x1, mut x2, mut y1, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    for s in samples.iter_mut() {
        let x0 = *s;
        let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
        x2 = x1; x1 = x0;
        y2 = y1; y1 = y0;
        *s = y0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, dur: f32, sr: u32) -> AudioBuffer {
        let n = (dur * sr as f32) as usize;
        let s = (0..n).map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin()).collect();
        AudioBuffer { samples: vec![s], sample_rate: sr }
    }

    #[test] fn reverb_len() { let b = sine(440.0, 0.5, 44100); let n = b.len(); assert_eq!(reverb(b, 1.0, 0.5, 0.3).len(), n); }
    #[test] fn lp_attenuates() { let b = sine(8000.0, 0.1, 44100); let b2 = sine(8000.0, 0.1, 44100); let o = lowpass(b, 200.0);
        let ei: f32 = b2.samples[0].iter().map(|s| s*s).sum(); let eo: f32 = o.samples[0].iter().map(|s| s*s).sum();
        assert!(eo < ei * 0.5); }
    #[test] fn rev_involution() { let b = sine(440.0, 0.1, 44100); assert_eq!(b.samples[0], reverse(&reverse(&b)).samples[0]); }
    #[test] fn notch_removes_freq() { let b = sine(1000.0, 0.2, 44100); let o = notch_filter(&b, 1000.0, 10.0, 0.0);
        let ei: f32 = b.samples[0].iter().map(|s| s*s).sum(); let eo: f32 = o.samples[0].iter().map(|s| s*s).sum();
        assert!(eo < ei * 0.3, "notch should remove target freq"); }
    #[test] fn bitcrush_quantizes() { let b = sine(440.0, 0.1, 44100); let o = bitcrush(&b, 4, 8000);
        let unique: std::collections::HashSet<u32> = o.samples[0].iter().map(|s| s.to_bits()).collect();
        assert!(unique.len() < b.len() / 2, "bitcrush should reduce unique values"); }
    #[test] fn bandpass_works() { let b = sine(440.0, 0.1, 44100); let o = bandpass(&b, 200.0, 800.0);
        assert_eq!(o.len(), b.len()); }
    #[test] fn phase_vocoder_stretches() { let b = sine(440.0, 0.5, 44100); let o = phase_vocoder(&b, 2.0);
        assert!(o.len() > b.len(), "vocoder should stretch"); }
    #[test] fn preswell_prepends() { let b = sine(440.0, 0.5, 44100); let o = reverse_preswell(&b, 100.0);
        assert!(o.len() > b.len(), "preswell should add samples"); }

    // -- audio_snapshot_from_buffer ------------------------------------------

    #[test]
    fn snapshot_silent_buffer_is_zero() {
        // FALSE-POSITIVE guard: a buffer of zeros must yield rms=0.
        let buf = AudioBuffer { samples: vec![vec![0.0f32; 4800]], sample_rate: 48_000 };
        let snap = audio_snapshot_from_buffer(&buf);
        assert_eq!(snap.rms, 0, "silent buffer → rms Permyriad 0");
    }

    #[test]
    fn snapshot_sine_rms_in_range() {
        // A 1.0-amplitude sine has RMS = 1/√2 ≈ 0.7071 → Permyriad ≈ 7071.
        let buf = sine(440.0, 0.1, 48_000);
        let snap = audio_snapshot_from_buffer(&buf);
        // Allow a few counts of float rounding.
        assert!((snap.rms - 7_071).abs() < 5, "sine rms ≈ 7071 Permyriad; got {}", snap.rms);
        assert!(snap.rms > 0 && snap.rms <= 10_000, "rms within Permyriad bounds");
    }

    #[test]
    fn snapshot_empty_buffer_is_zero() {
        let buf = AudioBuffer { samples: vec![], sample_rate: 44_100 };
        let snap = audio_snapshot_from_buffer(&buf);
        assert_eq!(snap, VibeMod::default());
    }
}
