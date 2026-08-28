//! WAV recorder — captures mixer master output to file.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Records interleaved stereo f32 PCM to a WAV file.
pub struct WavRecorder {
    writer: Option<hound::WavWriter<BufWriter<File>>>,
    pub frames_written: u64,
    start_time: Option<Instant>,
    pub dropped_blocks: u64,
    output_path: Option<PathBuf>,
}

impl WavRecorder {
    pub fn new() -> Self {
        Self {
            writer: None,
            frames_written: 0,
            start_time: None,
            dropped_blocks: 0,
            output_path: None,
        }
    }

    /// Start recording to a new WAV file.
    pub fn start(&mut self, output_dir: &Path, sample_rate: u32) -> Result<PathBuf, String> {
        let now = chrono::Local::now();
        let filename = format!("set_{}.wav", now.format("%Y-%m-%d_%H-%M-%S"));
        let path = output_dir.join(&filename);

        std::fs::create_dir_all(output_dir)
            .map_err(|e| format!("Failed to create output dir: {e}"))?;

        let spec = hound::WavSpec {
            channels: 2,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let writer = hound::WavWriter::create(&path, spec)
            .map_err(|e| format!("Failed to create WAV file: {e}"))?;

        self.writer = Some(writer);
        self.frames_written = 0;
        self.dropped_blocks = 0;
        self.start_time = Some(Instant::now());
        self.output_path = Some(path.clone());

        Ok(path)
    }

    /// Write a block of interleaved stereo samples.
    pub fn write_block(&mut self, interleaved: &[f32]) {
        if let Some(ref mut writer) = self.writer {
            for &sample in interleaved {
                if writer.write_sample(sample).is_err() {
                    self.dropped_blocks += 1;
                    return;
                }
            }
            self.frames_written += (interleaved.len() / 2) as u64;
        }
    }

    /// Stop recording, finalize the WAV file.
    pub fn stop(&mut self) -> Option<PathBuf> {
        if let Some(writer) = self.writer.take() {
            let _ = writer.finalize();
        }
        self.start_time = None;
        self.output_path.take()
    }

    pub fn is_recording(&self) -> bool {
        self.writer.is_some()
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.start_time.map_or(0.0, |t| t.elapsed().as_secs_f64())
    }

    /// Validate the recorded WAV file after stopping.
    /// Returns Ok(()) if valid, Err with description if invalid.
    pub fn validate(path: &Path) -> Result<(), String> {
        let reader = hound::WavReader::open(path)
            .map_err(|e| format!("Failed to open WAV for validation: {e}"))?;
        let spec = reader.spec();

        if spec.channels != 2 {
            return Err(format!("Expected 2 channels, got {}", spec.channels));
        }
        if spec.sample_format != hound::SampleFormat::Float {
            return Err("Expected 32-bit float format".to_string());
        }
        if spec.bits_per_sample != 32 {
            return Err(format!("Expected 32 bits per sample, got {}", spec.bits_per_sample));
        }
        Ok(())
    }
}

impl Default for WavRecorder {
    fn default() -> Self {
        Self::new()
    }
}
