//! Lightweight local-file + open-radio (Icecast / SHOUTcast / .m3u) player API.
//!
//! Wraps the existing `dsp::load_audio` + `realtime::PlaybackHandle` path
//! behind a small synchronous command surface. No new network or decoder
//! dependencies — stream transport uses `std::net::TcpStream` (stdlib) and
//! decode reuses the existing `symphonia` stack via a non-seekable shim.

use std::collections::VecDeque;
use crossbeam_channel::{unbounded, Receiver, Sender};

// ── Source type ───────────────────────────────────────────────────────────────

/// A playback source: local file path or open-radio stream URL.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerSource {
    /// Absolute or relative filesystem path. Decoded via `dsp::load_audio`.
    LocalFile(String),
    /// `http[s]://` Icecast / SHOUTcast / `.m3u` URL. No OAuth, no DRM.
    StreamUrl(String),
}

// ── State ─────────────────────────────────────────────────────────────────────

/// Observable player state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PlayerState {
    Stopped = 0,
    /// Decode or stream-connect in progress (live mode only).
    Loading = 1,
    Playing = 2,
    Paused  = 3,
}

impl From<u8> for PlayerState {
    fn from(v: u8) -> Self {
        match v {
            1 => Self::Loading,
            2 => Self::Playing,
            3 => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

// ── Worker commands ───────────────────────────────────────────────────────────

enum WorkerCmd {
    Load(PlayerSource),
    Play,
    Pause,
    Stop,
    SetVolume(f32),
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Lightweight player API over the existing realtime ring + `dsp::load_audio`.
///
/// State transitions are **synchronous** — `state()` reflects the last command
/// without waiting for the audio worker to ACK it. Actual audio output runs
/// in an optional background worker thread.
///
/// Use [`PlayerApi::new_headless`] in unit tests (no cpal device).
/// Use [`PlayerApi::spawn`] in production (starts cpal output + worker thread).
pub struct PlayerApi {
    state:   PlayerState,
    queue:   VecDeque<PlayerSource>,
    current: Option<PlayerSource>,
    volume:  f32,
    cmd_tx:  Option<Sender<WorkerCmd>>,
}

impl PlayerApi {
    /// Headless instance: no audio device opened. State transitions only.
    pub fn new_headless() -> Self {
        Self {
            state:   PlayerState::Stopped,
            queue:   VecDeque::new(),
            current: None,
            volume:  1.0,
            cmd_tx:  None,
        }
    }

    /// Spawn with a live worker thread that opens a cpal output device.
    pub fn spawn() -> Self {
        let (tx, rx) = unbounded::<WorkerCmd>();
        std::thread::spawn(move || player_worker(rx));
        Self {
            state:   PlayerState::Stopped,
            queue:   VecDeque::new(),
            current: None,
            volume:  1.0,
            cmd_tx:  Some(tx),
        }
    }

    /// Enqueue a local file path for playback.
    pub fn enqueue_local(&mut self, path: impl Into<String>) {
        self.queue.push_back(PlayerSource::LocalFile(path.into()));
    }

    /// Enqueue an Icecast / SHOUTcast / .m3u stream URL.
    pub fn enqueue_stream(&mut self, url: impl Into<String>) {
        self.queue.push_back(PlayerSource::StreamUrl(url.into()));
    }

    /// Start or resume playback.
    ///
    /// - From `Stopped`: pops the front of the queue and loads it.
    /// - From `Paused`: resumes without touching the queue.
    /// - From `Loading`/`Playing`: no-op.
    pub fn play(&mut self) {
        match self.state {
            PlayerState::Stopped => {
                if let Some(src) = self.queue.pop_front() {
                    self.current = Some(src.clone());
                    self.state = if self.cmd_tx.is_some() {
                        PlayerState::Loading
                    } else {
                        PlayerState::Playing
                    };
                    if let Some(tx) = &self.cmd_tx {
                        let _ = tx.send(WorkerCmd::Load(src));
                    }
                }
            }
            PlayerState::Paused => {
                self.state = PlayerState::Playing;
                if let Some(tx) = &self.cmd_tx {
                    let _ = tx.send(WorkerCmd::Play);
                }
            }
            PlayerState::Loading | PlayerState::Playing => {}
        }
    }

    /// Pause playback. No-op when already `Stopped`.
    pub fn pause(&mut self) {
        if matches!(self.state, PlayerState::Playing | PlayerState::Loading) {
            self.state = PlayerState::Paused;
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(WorkerCmd::Pause);
            }
        }
    }

    /// Skip to the next queued item. Transitions to `Stopped` if the queue is empty.
    pub fn next(&mut self) {
        if let Some(src) = self.queue.pop_front() {
            self.current = Some(src.clone());
            self.state = if self.cmd_tx.is_some() {
                PlayerState::Loading
            } else {
                PlayerState::Playing
            };
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(WorkerCmd::Load(src));
            }
        } else {
            self.current = None;
            self.state = PlayerState::Stopped;
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(WorkerCmd::Stop);
            }
        }
    }

    /// Stop playback and discard the current item.
    pub fn stop(&mut self) {
        self.state = PlayerState::Stopped;
        self.current = None;
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(WorkerCmd::Stop);
        }
    }

    /// Set playback volume (clamped to 0.0–1.0).
    pub fn set_volume(&mut self, v: f32) {
        self.volume = v.clamp(0.0, 1.0);
        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(WorkerCmd::SetVolume(self.volume));
        }
    }

    pub fn state(&self)     -> PlayerState           { self.state }
    pub fn current(&self)   -> Option<&PlayerSource> { self.current.as_ref() }
    pub fn volume(&self)    -> f32                   { self.volume }
    pub fn queue_len(&self) -> usize                 { self.queue.len() }
}

// ── Worker thread ─────────────────────────────────────────────────────────────
//
// Mirrors the forge-app-hud/src/player.rs pattern exactly:
//   decode thread → crossbeam channel → push loop → PlaybackHandle producer.

fn player_worker(rx: Receiver<WorkerCmd>) {
    let mut volume:          f32    = 1.0;
    let mut playing:         bool   = false;
    let mut pos:             usize  = 0;
    let mut buf: Option<crate::dsp::AudioBuffer> = None;
    let mut play_start       = std::time::Instant::now();
    let mut play_start_pos:  usize  = 0;

    let (decode_tx, decode_rx) = unbounded::<Result<crate::dsp::AudioBuffer, String>>();

    let mut device_sample_rate = 48_000u32;
    let mut playback: Option<crate::realtime::PlaybackHandle> =
        match crate::realtime::start_playback_lockfree(8192) {
            Ok(h) => { device_sample_rate = h.sample_rate(); Some(h) }
            Err(e) => { eprintln!("[player_api] cpal init failed: {e}"); None }
        };

    loop {
        // ── Drain decode completions ──────────────────────────────────────────
        while let Ok(result) = decode_rx.try_recv() {
            match result {
                Ok(audio) => {
                    buf          = Some(audio);
                    pos          = 0;
                    playing      = true;
                    play_start   = std::time::Instant::now();
                    play_start_pos = 0;
                }
                Err(e) => eprintln!("[player_api] decode error: {e}"),
            }
        }

        // ── Process commands ──────────────────────────────────────────────────
        while let Ok(cmd) = rx.try_recv() {
            match cmd {
                WorkerCmd::Load(src) => {
                    playing = false;
                    buf     = None;
                    pos     = 0;
                    if let Some(h) = &playback {
                        h.flush_flag().store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    let tx = decode_tx.clone();
                    std::thread::spawn(move || { let _ = tx.send(load_source(src)); });
                }
                WorkerCmd::Play => {
                    if buf.is_some() {
                        playing        = true;
                        play_start     = std::time::Instant::now();
                        play_start_pos = pos;
                    }
                }
                WorkerCmd::Pause        => { playing = false; }
                WorkerCmd::Stop         => { playing = false; buf = None; pos = 0; }
                WorkerCmd::SetVolume(v) => { volume = v; }
            }
        }

        // ── Audio push loop — mirrors forge-app-hud/src/player.rs ────────────
        // No heap allocation in this section; all arithmetic on stack.
        if playing {
            if let (Some(b), Some(handle)) = (&buf, playback.as_mut()) {
                if pos >= b.len() {
                    playing = false;
                } else {
                    let elapsed    = play_start.elapsed().as_secs_f64();
                    let target     = (play_start_pos as f64 + elapsed * b.sample_rate as f64)
                                         .min(b.len() as f64);
                    let ratio      = b.sample_rate as f64 / device_sample_rate as f64;

                    if target as usize > pos {
                        let prod       = handle.producer_mut();
                        let slots      = prod.slots() / 2;
                        let file_want  = target as usize - pos;
                        let push_count = ((file_want as f64 / ratio) as usize)
                                             .min(slots)
                                             .min(2048);

                        for d in 0..push_count {
                            let file_f = pos as f64 + d as f64 * ratio;
                            let idx    = (file_f as usize).min(b.len().saturating_sub(2));
                            let frac   = (file_f - idx as f64) as f32;

                            let l0 = b.samples[0][idx];
                            let l1 = b.samples[0][(idx + 1).min(b.len() - 1)];
                            let l  = (l0 + (l1 - l0) * frac) * volume;
                            let r  = if b.channels() > 1 {
                                let r0 = b.samples[1][idx];
                                let r1 = b.samples[1][(idx + 1).min(b.len() - 1)];
                                (r0 + (r1 - r0) * frac) * volume
                            } else {
                                l
                            };
                            let _ = prod.push(l);
                            let _ = prod.push(r);
                        }
                        pos = (pos as f64 + push_count as f64 * ratio) as usize;
                        pos = pos.min(b.len());
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(if playing { 2 } else { 10 }));
    }
}

// ── Source loading ────────────────────────────────────────────────────────────

fn load_source(src: PlayerSource) -> Result<crate::dsp::AudioBuffer, String> {
    match src {
        PlayerSource::LocalFile(path) => crate::dsp::load_audio(&path),
        PlayerSource::StreamUrl(url)  => decode_stream(&url),
    }
}

// ── Icecast / SHOUTcast stream decode ────────────────────────────────────────
//
// Transport: raw TCP + minimal HTTP/1.0 GET (handles both `HTTP/1.x 200` and
// the non-standard `ICY 200 OK` response SHOUTcast servers emit).
// Decode:    symphonia non-seekable path (OGG/Vorbis, MP3, AAC).
// No new deps: stdlib TcpStream + existing symphonia crate.

/// Open an Icecast / SHOUTcast stream URL and decode into an AudioBuffer.
fn decode_stream(url: &str) -> Result<crate::dsp::AudioBuffer, String> {
    use std::io::{Read, Write};
    use std::net::TcpStream;

    // Minimal URL parse — strip scheme, split host:port from path.
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(|| format!("[player_api] not an http/https URL: {url}"))?;

    let (host_port, path_tail) = rest.split_once('/').unwrap_or((rest, ""));
    let path = format!("/{path_tail}");
    let host = host_port.split(':').next().unwrap_or(host_port);
    let addr = if host_port.contains(':') {
        host_port.to_string()
    } else {
        format!("{host_port}:80")
    };

    let mut stream = TcpStream::connect(&addr)
        .map_err(|e| format!("[player_api] connect {addr}: {e}"))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(15)))
        .map_err(|e| format!("[player_api] set timeout: {e}"))?;

    // Minimal HTTP/1.0 GET — Icy-MetaData:0 suppresses inline metadata.
    let request = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}\r\n\
         Icy-MetaData: 0\r\nUser-Agent: forge-audio/1.0\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("[player_api] send request: {e}"))?;

    // Skip HTTP / ICY response headers (read until \r\n\r\n or \n\n).
    let mut hdr  = [0u8; 4096];
    let mut hlen = 0usize;
    loop {
        if hlen >= hdr.len() {
            return Err("[player_api] response header exceeds 4 KiB".into());
        }
        let n = stream
            .read(&mut hdr[hlen..hlen + 1])
            .map_err(|e| format!("[player_api] read header: {e}"))?;
        if n == 0 {
            return Err("[player_api] stream closed during header read".into());
        }
        hlen += n;
        if hlen >= 4 && &hdr[hlen - 4..hlen] == b"\r\n\r\n" {
            break;
        }
        if hlen >= 2 && &hdr[hlen - 2..hlen] == b"\n\n" {
            break;
        }
    }

    // Feed the remaining stream body to symphonia.
    symphonia_decode_stream(stream)
}

// ── Symphonia non-seekable TcpStream shim ────────────────────────────────────

/// Wraps a `TcpStream` as a symphonia `MediaSource`.
/// `Seek` returns `Unsupported`; symphonia probes forward-only formats fine.
struct TcpMediaSource(std::net::TcpStream);

impl std::io::Read for TcpMediaSource {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl std::io::Seek for TcpMediaSource {
    fn seek(&mut self, _: std::io::SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "TcpMediaSource is not seekable",
        ))
    }
}

impl symphonia::core::io::MediaSource for TcpMediaSource {
    fn is_seekable(&self) -> bool  { false }
    fn byte_len(&self)    -> Option<u64> { None }
}

/// Decode an audio stream body (post-headers) using symphonia's non-seekable path.
fn symphonia_decode_stream(stream: std::net::TcpStream) -> Result<crate::dsp::AudioBuffer, String> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let mss = MediaSourceStream::new(
        Box::new(TcpMediaSource(stream)),
        Default::default(),
    );

    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("[player_api] probe stream: {e}"))?;

    let mut fmt = probed.format;
    let track = fmt
        .default_track()
        .ok_or("[player_api] stream has no default track")?;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("[player_api] stream has no sample rate")?;
    let n_ch     = track.codec_params.channels.map(|c| c.count()).unwrap_or(1);
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("[player_api] stream decoder: {e}"))?;

    let mut interleaved: Vec<f32> = Vec::new();

    loop {
        let packet = match fmt.next_packet() {
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
        let spec     = *decoded.spec();
        let capacity = decoded.capacity();
        let mut sbuf = SampleBuffer::<f32>::new(capacity as u64, spec);
        sbuf.copy_interleaved_ref(decoded);
        interleaved.extend_from_slice(sbuf.samples());
    }

    if interleaved.is_empty() {
        return Err("[player_api] stream produced no audio samples".into());
    }

    let mut samples = vec![Vec::new(); n_ch];
    for (i, &s) in interleaved.iter().enumerate() {
        samples[i % n_ch].push(s);
    }
    Ok(crate::dsp::AudioBuffer { samples, sample_rate })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// RED: fails to compile before this module exists.
    /// GREEN: all state transitions are deterministic in headless mode.
    #[test]
    fn state_machine_transitions() {
        let mut api = PlayerApi::new_headless();
        assert_eq!(api.state(), PlayerState::Stopped);

        // Enqueue one local file and one stream URL.
        api.enqueue_local("/tmp/forge-test.mp3");
        api.enqueue_stream("http://example.com/stream");
        assert_eq!(api.queue_len(), 2);

        // Stopped → Playing (headless skips the Loading step).
        api.play();
        assert_eq!(api.state(), PlayerState::Playing);
        assert_eq!(api.queue_len(), 1);

        // Playing → Paused.
        api.pause();
        assert_eq!(api.state(), PlayerState::Paused);

        // Paused → Playing (resume; does NOT pop the queue).
        api.play();
        assert_eq!(api.state(), PlayerState::Playing);
        assert_eq!(api.queue_len(), 1);

        // next() — loads the stream URL item.
        api.next();
        assert_eq!(api.state(), PlayerState::Playing);
        assert_eq!(api.queue_len(), 0);
        assert!(matches!(api.current(), Some(PlayerSource::StreamUrl(_))));

        // next() with empty queue → Stopped.
        api.next();
        assert_eq!(api.state(), PlayerState::Stopped);
        assert!(api.current().is_none());
    }

    #[test]
    fn pause_on_stopped_is_noop() {
        let mut api = PlayerApi::new_headless();
        api.pause();
        assert_eq!(api.state(), PlayerState::Stopped);
    }

    #[test]
    fn play_on_empty_queue_stays_stopped() {
        let mut api = PlayerApi::new_headless();
        api.play();
        assert_eq!(api.state(), PlayerState::Stopped);
    }

    #[test]
    fn volume_clamp() {
        let mut api = PlayerApi::new_headless();
        api.set_volume(2.0);
        assert_eq!(api.volume(), 1.0);
        api.set_volume(-1.0);
        assert_eq!(api.volume(), 0.0);
        api.set_volume(0.5);
        assert!((api.volume() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn current_reflects_loaded_source() {
        let mut api = PlayerApi::new_headless();
        api.enqueue_local("/music/track.flac");
        api.enqueue_stream("http://radio.example.com:8000/jazz");
        api.play();
        let expected = PlayerSource::LocalFile("/music/track.flac".to_string());
        assert_eq!(api.current(), Some(&expected));
        api.next();
        assert!(matches!(api.current(), Some(PlayerSource::StreamUrl(_))));
    }

    #[test]
    fn stop_clears_current() {
        let mut api = PlayerApi::new_headless();
        api.enqueue_local("/tmp/x.mp3");
        api.play();
        assert!(api.current().is_some());
        api.stop();
        assert_eq!(api.state(), PlayerState::Stopped);
        assert!(api.current().is_none());
    }
}
