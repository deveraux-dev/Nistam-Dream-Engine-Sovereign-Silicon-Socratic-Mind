//! Icecast source client — streams live audio as OGG/Vorbis to an Icecast server.

use std::io::{Write, BufWriter};
use std::net::TcpStream;
use std::time::Instant;

use base64::Engine;

/// Configuration for connecting to an Icecast server.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct BroadcastConfig {
    pub host: String,
    pub port: u16,
    pub mount: String,
    pub password: String,
    pub stream_name: String,
    pub bitrate: u32,
}

impl Default for BroadcastConfig {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            port: 8000,
            mount: "/stream".into(),
            password: std::env::var("ICECAST_SOURCE_PASSWORD").unwrap_or_else(|_| "hackme".into()),
            stream_name: "Deveraux.FM".into(),
            bitrate: 192,
        }
    }
}

/// Broadcast status for the UI.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BroadcastStatus {
    pub is_live: bool,
    pub uptime_secs: u64,
    pub bytes_sent: u64,
}

/// Icecast source client. Connects via HTTP PUT, streams OGG/Vorbis frames.
pub struct IcecastSource {
    config: BroadcastConfig,
    writer: Option<BufWriter<TcpStream>>,
    encoder: Option<VorbisEncoder>,
    is_live: bool,
    start_time: Option<Instant>,
    bytes_sent: u64,
}

/// Wraps vorbis_encoder for streaming OGG output.
struct VorbisEncoder {
    encoder: vorbis_encoder::Encoder,
}

impl VorbisEncoder {
    fn new(channels: u32, sample_rate: u32, _bitrate: u32) -> Result<Self, String> {
        // vorbis_encoder::Encoder::new(channels, rate, quality)
        // quality: -0.1 to 1.0 (0.4 ≈ ~128kbps, 0.6 ≈ ~192kbps)
        let quality = 0.6;
        let encoder = vorbis_encoder::Encoder::new(
            channels,
            sample_rate as u64,
            quality,
        ).map_err(|e| format!("Vorbis encoder init failed: error code {}", e))?;
        Ok(Self { encoder })
    }

    /// Encode interleaved f32 samples, return OGG bytes.
    fn encode(&mut self, samples: &[f32]) -> Result<Vec<u8>, String> {
        // vorbis_encoder expects &Vec<i16> interleaved
        let pcm_i16: Vec<i16> = samples.iter()
            .map(|&s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();

        self.encoder.encode(&pcm_i16)
            .map_err(|e| format!("Vorbis encode: error code {}", e))
    }

    /// Flush remaining data and return final OGG bytes.
    fn flush(&mut self) -> Result<Vec<u8>, String> {
        self.encoder.flush()
            .map_err(|e| format!("Vorbis flush: error code {}", e))
    }
}

// SAFETY: IcecastSource owns its TcpStream + VorbisEncoder exclusively.
// The raw pointer inside vorbis_encoder::Encoder is a heap-allocated C struct
// that is only accessed through &mut self methods — safe to move across threads.
unsafe impl Send for IcecastSource {}

impl IcecastSource {
    pub fn new(config: BroadcastConfig) -> Self {
        Self {
            config,
            writer: None,
            encoder: None,
            is_live: false,
            start_time: None,
            bytes_sent: 0,
        }
    }

    /// Connect to the Icecast server and send the HTTP source header.
    pub fn connect(&mut self, sample_rate: u32, channels: u32) -> Result<(), String> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let stream = TcpStream::connect(&addr)
            .map_err(|e| format!("Connect to {}: {}", addr, e))?;

        // Set a write timeout to avoid blocking the audio thread indefinitely
        stream.set_write_timeout(Some(std::time::Duration::from_millis(500)))
            .map_err(|e| format!("Set timeout: {}", e))?;
        stream.set_nodelay(true)
            .map_err(|e| format!("Set nodelay: {}", e))?;

        let mut writer = BufWriter::new(stream);

        // Build HTTP PUT request (Icecast source protocol)
        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("source:{}", self.config.password));

        let header = format!(
            "PUT {} HTTP/1.1\r\n\
             Host: {}:{}\r\n\
             Authorization: Basic {}\r\n\
             Content-Type: application/ogg\r\n\
             Transfer-Encoding: chunked\r\n\
             Ice-Name: {}\r\n\
             Ice-Description: Broadcasting from the prairies\r\n\
             Ice-Genre: Electronic\r\n\
             Ice-Public: 0\r\n\
             \r\n",
            self.config.mount,
            self.config.host,
            self.config.port,
            auth,
            self.config.stream_name,
        );

        writer.write_all(header.as_bytes())
            .map_err(|e| format!("Send header: {}", e))?;
        writer.flush()
            .map_err(|e| format!("Flush header: {}", e))?;

        // Initialize Vorbis encoder
        let encoder = VorbisEncoder::new(channels, sample_rate, self.config.bitrate)?;

        self.writer = Some(writer);
        self.encoder = Some(encoder);
        self.is_live = true;
        self.start_time = Some(Instant::now());
        self.bytes_sent = 0;

        eprintln!("[Broadcast] Connected to {} — {} is LIVE", addr, self.config.stream_name);
        Ok(())
    }

    /// Disconnect from the Icecast server.
    pub fn disconnect(&mut self) {
        if let Some(ref mut encoder) = self.encoder {
            if let Ok(final_bytes) = encoder.flush() {
                if let Some(ref mut writer) = self.writer {
                    let _ = writer.write_all(&final_bytes);
                    let _ = writer.flush();
                }
            }
        }
        self.writer = None;
        self.encoder = None;
        self.is_live = false;
        let uptime = self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);
        eprintln!("[Broadcast] Disconnected. Uptime: {}s, Sent: {} bytes", uptime, self.bytes_sent);
        self.start_time = None;
    }

    /// Send PCM samples to the Icecast server.
    /// `samples` should be interleaved f32 (matching channel count from connect()).
    /// Call this from the mixer output bus after each block.
    pub fn send_samples(&mut self, samples: &[f32]) -> Result<(), String> {
        if !self.is_live {
            return Err("Not connected".into());
        }

        let encoder = self.encoder.as_mut().ok_or("No encoder")?;
        let ogg_bytes = encoder.encode(samples)?;

        if !ogg_bytes.is_empty() {
            let writer = self.writer.as_mut().ok_or("No connection")?;
            match writer.write_all(&ogg_bytes) {
                Ok(_) => {
                    self.bytes_sent += ogg_bytes.len() as u64;
                    // Flush periodically (not every call — BufWriter handles batching)
                    if self.bytes_sent % 8192 < ogg_bytes.len() as u64 {
                        let _ = writer.flush();
                    }
                }
                Err(e) => {
                    eprintln!("[Broadcast] Write error: {} — disconnecting", e);
                    self.is_live = false;
                    return Err(format!("Write failed: {}", e));
                }
            }
        }

        Ok(())
    }

    pub fn is_live(&self) -> bool {
        self.is_live
    }

    pub fn status(&self) -> BroadcastStatus {
        BroadcastStatus {
            is_live: self.is_live,
            uptime_secs: self.start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0),
            bytes_sent: self.bytes_sent,
        }
    }
}
</content>
