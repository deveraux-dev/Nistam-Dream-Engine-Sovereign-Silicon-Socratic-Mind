//! Real-time audio INPUT capture via cpal. Lock-free SPSC ring buffer path only.
//!
//! Mirrors `realtime.rs` output path but inverted: the cpal input callback is
//! the producer; application code on another thread is the consumer. Stream
//! held privately inside `InputCaptureHandle` so drop-then-read bugs (the
//! inverse of the 2026-04-09 playback incident) are structurally impossible.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use crate::device_info::AudioDeviceInfo;

/// Owns a real-time audio capture resource.
///
/// `cpal::Stream` is held privately. Drop the handle to stop capture. The
/// `!Send` stream is kept alive on the construction thread; the consumer is
/// the only public path to read samples.
pub struct InputCaptureHandle {
    stream_guard: Option<cpal::Stream>,
    /// `None` once [`InputCaptureHandle::take_consumer`] has handed the read
    /// end to another thread (the analyzer). Capture keeps running — the
    /// stream lives with this handle, not with the consumer.
    consumer: Option<rtrb::Consumer<f32>>,
    device_error: Arc<AtomicBool>,
    channels: usize,
    sample_rate: u32,
    overruns: Arc<AtomicU64>,
}

impl InputCaptureHandle {
    /// Pull samples from the capture device. Use only inside the thread that
    /// owns this handle (typically the mixer feeder thread).
    ///
    /// `None` after [`take_consumer`](Self::take_consumer) — the read end
    /// belongs to someone else now. Returning an `Option` rather than
    /// unwrapping keeps a taken handle from panicking a live audio thread.
    pub fn consumer_mut(&mut self) -> Option<&mut rtrb::Consumer<f32>> {
        self.consumer.as_mut()
    }

    /// Hand the ring's read end to another thread — `analyzer::spawn_analyzer`
    /// wants an owned `rtrb::Consumer<f32>`, and this is the only way to give
    /// it one without also giving away the `cpal::Stream` that feeds it.
    ///
    /// Returns `None` on a second call: a SPSC ring has exactly one reader.
    /// Dropping this handle still stops capture, so the caller must keep it
    /// alive for as long as the analyzer should hear anything.
    pub fn take_consumer(&mut self) -> Option<rtrb::Consumer<f32>> {
        self.consumer.take()
    }

    /// True while this handle still holds the ring's read end.
    pub fn has_consumer(&self) -> bool {
        self.consumer.is_some()
    }

    pub fn channels(&self) -> usize { self.channels }
    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn device_error_flag(&self) -> Arc<AtomicBool> { self.device_error.clone() }
    pub fn overrun_counter(&self) -> Arc<AtomicU64> { self.overruns.clone() }

    /// Returns true when a real cpal input device is open. Returns false
    /// under `DAW_NO_AUDIO=1` (hard-gate test mode).
    pub fn is_live(&self) -> bool { self.stream_guard.is_some() }
}

impl Drop for InputCaptureHandle {
    fn drop(&mut self) {
        if let Some(s) = self.stream_guard.take() {
            let _ = s.pause();
            drop(s);
        }
    }
}

/// Enumerate all available audio input devices with their supported configs.
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
    let host = cpal::default_host();
    let host_name = format!("{:?}", host.id());
    let devices = match host.input_devices() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[audio-input] Cannot enumerate input devices: {}", e);
            return vec![];
        }
    };
    devices
        .filter_map(|dev| {
            let name = dev.name().unwrap_or_else(|_| "Unknown".into());
            let mut sample_rates = Vec::new();
            let mut channels = 1u16;
            if let Ok(configs) = dev.supported_input_configs() {
                for cfg in configs {
                    channels = channels.max(cfg.channels());
                    let min = cfg.min_sample_rate().0;
                    let max = cfg.max_sample_rate().0;
                    for &sr in &[44100u32, 48000, 88200, 96000, 192000] {
                        if sr >= min && sr <= max && !sample_rates.contains(&sr) {
                            sample_rates.push(sr);
                        }
                    }
                }
            }
            sample_rates.sort();
            if sample_rates.is_empty() {
                sample_rates.push(48000);
            }
            Some(AudioDeviceInfo {
                name,
                api: host_name.clone(),
                sample_rates,
                channels,
            })
        })
        .collect()
}

/// Start real-time audio input capture on the default input device using a
/// lock-free SPSC ring buffer.
///
/// Returns an [`InputCaptureHandle`] guard. The cpal::Stream is held privately
/// inside the guard. Drop the handle to stop capture. Samples are pushed to
/// the internal ring buffer on the audio thread; read via `consumer_mut()` on
/// the mixer feeder thread.
pub fn start_input_capture(buffer_size: usize) -> Result<InputCaptureHandle, String> {
    start_input_capture_with_options(buffer_size, None, None, None)
}

/// Silent fallback handle for when no input device is available.
pub fn null_input_capture(buffer_size: usize, sample_rate: u32) -> InputCaptureHandle {
    let (producer, consumer) = rtrb::RingBuffer::<f32>::new(buffer_size);
    std::mem::forget(producer);
    InputCaptureHandle {
        stream_guard: None,
        consumer: Some(consumer),
        device_error: Arc::new(AtomicBool::new(true)),
        channels: 1,
        sample_rate,
        overruns: Arc::new(AtomicU64::new(0)),
    }
}

/// Inner implementation accepting an optional reconnect error flag,
/// an optional device name, and an optional cpal buffer frame count.
///
/// HARD GATE: `DAW_NO_AUDIO=1` bypasses the cpal device entirely. The handle's
/// `stream_guard` is `None`; the producer is leaked into the void. Mixer math
/// still runs with silence on the input channel.
pub fn start_input_capture_with_options(
    buffer_size: usize,
    error_flag: Option<Arc<AtomicBool>>,
    device_name: Option<&str>,
    cpal_buffer: Option<u32>,
) -> Result<InputCaptureHandle, String> {
    if std::env::var("DAW_NO_AUDIO").is_ok() {
        eprintln!("[DAW_NO_AUDIO] *** AUDIO INPUT DISABLED *** No cpal input device opened.");
        let (producer, consumer) = rtrb::RingBuffer::<f32>::new(buffer_size);
        std::mem::forget(producer);
        let device_error = error_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
        return Ok(InputCaptureHandle {
            stream_guard: None,
            consumer: Some(consumer),
            device_error,
            channels: 1,
            sample_rate: 48000,
            overruns: Arc::new(AtomicU64::new(0)),
        });
    }

    let host = cpal::default_host();
    let device = if let Some(name) = device_name {
        let mut devs = host
            .input_devices()
            .map_err(|e| format!("enumerate input devices: {}", e))?;
        devs.find(|d| d.name().ok().as_deref() == Some(name))
            .ok_or_else(|| format!("input device not found: {}", name))?
    } else {
        host.default_input_device()
            .ok_or_else(|| "no default input device".to_string())?
    };

    let supported = device
        .default_input_config()
        .map_err(|e| format!("default input config: {}", e))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;

    let mut stream_cfg: cpal::StreamConfig = supported.clone().into();
    if let Some(frames) = cpal_buffer {
        stream_cfg.buffer_size = cpal::BufferSize::Fixed(frames);
    }

    let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(buffer_size);
    let device_error = error_flag.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    let overruns = Arc::new(AtomicU64::new(0));

    let overruns_cb = overruns.clone();
    let device_error_cb = device_error.clone();

    let err_fn = move |e: cpal::StreamError| {
        eprintln!("[audio-input] stream error: {}", e);
        device_error_cb.store(true, Ordering::Release);
    };

    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &stream_cfg,
                move |data: &[f32], _| {
                    for &s in data {
                        if producer.push(s).is_err() {
                            overruns_cb.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build f32 input stream: {}", e))?,
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &stream_cfg,
                move |data: &[i16], _| {
                    for &s in data {
                        let f = (s as f32) / (i16::MAX as f32);
                        if producer.push(f).is_err() {
                            overruns_cb.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build i16 input stream: {}", e))?,
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                &stream_cfg,
                move |data: &[u16], _| {
                    for &s in data {
                        let f = (s as f32 - i16::MAX as f32) / (i16::MAX as f32);
                        if producer.push(f).is_err() {
                            overruns_cb.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build u16 input stream: {}", e))?,
        fmt => return Err(format!("unsupported input sample format: {:?}", fmt)),
    };

    stream
        .play()
        .map_err(|e| format!("start input stream: {}", e))?;

    Ok(InputCaptureHandle {
        stream_guard: Some(stream),
        consumer: Some(consumer),
        device_error,
        channels,
        sample_rate,
        overruns,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_handle_is_not_live() {
        let h = null_input_capture(1024, 48000);
        assert!(!h.is_live());
        assert_eq!(h.sample_rate(), 48000);
        assert_eq!(h.channels(), 1);
        assert_eq!(h.overrun_counter().load(Ordering::Relaxed), 0);
    }

    #[test]
    fn list_input_devices_does_not_panic() {
        let _ = list_input_devices();
    }

    #[test]
    fn daw_no_audio_bypass_returns_handle() {
        std::env::set_var("DAW_NO_AUDIO", "1");
        let h = start_input_capture(512).expect("bypass should succeed");
        assert!(!h.is_live());
        std::env::remove_var("DAW_NO_AUDIO");
    }
}
