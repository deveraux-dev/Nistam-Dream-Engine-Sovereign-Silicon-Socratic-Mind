// @forge:allow_float — DDSP leaf; audio sample arithmetic is inherently f32.
//! Master transport + arrangement timeline.
//!
//! [`Transport`] — one playhead (`u64` samples) over a [`BeatGrid`], with play/stop/seek
//! and an optional [`LoopRegion`]. [`Arrangement`] — up to [`MAX_CLIPS`] [`Clip`]s across
//! [`MAX_LANES`] lanes. References PCM by index — never owns sample data; zero-heap render.

use crate::bpm::BeatGrid;

pub const MAX_CLIPS: usize = 64;
pub const MAX_LANES: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TransportState {
    Stopped,
    Playing,
}

/// A loop region on the timeline, in samples. Half-open `[start, end)`.
#[derive(Clone, Copy, Debug)]
pub struct LoopRegion {
    pub start: u64,
    pub end: u64,
}

impl LoopRegion {
    #[inline]
    fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
}

/// Master transport: a single playhead over a tempo grid.
pub struct Transport {
    pub grid: BeatGrid,
    pub playhead: u64,
    pub state: TransportState,
    pub loop_region: Option<LoopRegion>,
    pub sample_rate: u32,
}

impl Transport {
    pub fn new(bpm: f32, sample_rate: u32) -> Self {
        Self {
            grid: BeatGrid::from_bpm(bpm, 0, sample_rate, 0),
            playhead: 0,
            state: TransportState::Stopped,
            loop_region: None,
            sample_rate,
        }
    }

    pub fn play(&mut self) { self.state = TransportState::Playing; }
    pub fn stop(&mut self) { self.state = TransportState::Stopped; }
    pub fn stop_and_rewind(&mut self) {
        self.state = TransportState::Stopped;
        self.playhead = self.loop_region.map(|r| r.start).unwrap_or(0);
    }
    pub fn toggle(&mut self) {
        self.state = match self.state {
            TransportState::Playing => TransportState::Stopped,
            TransportState::Stopped => TransportState::Playing,
        };
    }
    #[inline]
    pub fn is_playing(&self) -> bool { self.state == TransportState::Playing }

    pub fn seek(&mut self, pos: u64) { self.playhead = pos; }
    pub fn seek_to_beat(&mut self, n: usize) {
        self.playhead = self.grid.beat_pos(n) as u64;
    }

    pub fn set_loop(&mut self, start: u64, end: u64) {
        if end > start {
            self.loop_region = Some(LoopRegion { start, end });
        }
    }
    pub fn clear_loop(&mut self) { self.loop_region = None; }

    /// Fractional beat position of the playhead.
    pub fn beat_position(&self) -> f32 {
        let iv = self.grid.beat_interval.max(1) as f32;
        self.playhead as f32 / iv
    }

    /// Advance the playhead by `frames` if playing, wrapping at the loop region.
    /// Returns the number of times the loop wrapped.
    pub fn advance(&mut self, frames: u64) -> u32 {
        if self.state != TransportState::Playing {
            return 0;
        }
        let mut wraps = 0;
        match self.loop_region {
            Some(r) if r.len() > 0 => {
                let len = r.len();
                let mut rel = self.playhead.saturating_sub(r.start) % len;
                rel += frames;
                while rel >= len {
                    rel -= len;
                    wraps += 1;
                }
                self.playhead = r.start + rel;
            }
            _ => {
                self.playhead = self.playhead.wrapping_add(frames);
            }
        }
        wraps
    }
}

/// One clip: a window into a caller-supplied PCM buffer, placed on a lane at a timeline position.
#[derive(Clone, Copy, Debug)]
pub struct Clip {
    pub lane: u8,
    pub buffer_id: usize,
    pub start: u64,
    pub len: u64,
    pub buf_offset: u64,
    pub gain: f32, // @forge:allow_float
    pub muted: bool,
}

impl Clip {
    fn empty() -> Self {
        Self { lane: 0, buffer_id: 0, start: 0, len: 0, buf_offset: 0, gain: 0.0, muted: true }
    }
    #[inline]
    fn covers(&self, pos: u64) -> bool {
        pos >= self.start && pos < self.start + self.len
    }
}

/// A multi-lane arrangement of clips. Fixed-capacity, zero-heap render.
pub struct Arrangement {
    clips: [Clip; MAX_CLIPS],
    count: usize,
    lane_gain: [f32; MAX_LANES],  // @forge:allow_float
    lane_muted: [bool; MAX_LANES],
}

impl Default for Arrangement {
    fn default() -> Self { Self::new() }
}

impl Arrangement {
    pub fn new() -> Self {
        Self {
            clips: [Clip::empty(); MAX_CLIPS],
            count: 0,
            lane_gain: [1.0; MAX_LANES],
            lane_muted: [false; MAX_LANES],
        }
    }

    pub fn add_clip(&mut self, clip: Clip) -> Option<usize> {
        if self.count >= MAX_CLIPS || clip.lane as usize >= MAX_LANES {
            return None;
        }
        let idx = self.count;
        self.clips[idx] = clip;
        self.count += 1;
        Some(idx)
    }

    pub fn clip_count(&self) -> usize { self.count }
    pub fn is_empty(&self) -> bool { self.count == 0 }
    pub fn clear_clips(&mut self) { self.count = 0; }

    pub fn set_lane_gain(&mut self, lane: usize, gain: f32) {
        if lane < MAX_LANES { self.lane_gain[lane] = gain; }
    }
    pub fn set_lane_muted(&mut self, lane: usize, muted: bool) {
        if lane < MAX_LANES { self.lane_muted[lane] = muted; }
    }

    pub fn length_samples(&self) -> u64 {
        let mut max_end = 0;
        for c in &self.clips[..self.count] {
            max_end = max_end.max(c.start + c.len);
        }
        max_end
    }

    /// Render `out.len()` frames from the transport playhead, then advance it. Zero-heap.
    pub fn render_block(&self, transport: &mut Transport, out: &mut [f32], buffers: &[&[f32]]) {
        if !transport.is_playing() { return; }
        let start_pos = transport.playhead;
        let loop_region = transport.loop_region;

        for (i, sample) in out.iter_mut().enumerate() {
            let pos = match loop_region {
                Some(r) if r.len() > 0 => {
                    let len = r.len();
                    let rel = (start_pos.saturating_sub(r.start) + i as u64) % len;
                    r.start + rel
                }
                _ => start_pos.wrapping_add(i as u64),
            };

            let mut mix = 0.0_f32;
            for c in &self.clips[..self.count] {
                if c.muted || self.lane_muted[c.lane as usize] || !c.covers(pos) {
                    continue;
                }
                let buf = match buffers.get(c.buffer_id) {
                    Some(b) if !b.is_empty() => *b,
                    _ => continue,
                };
                let read = c.buf_offset + (pos - c.start);
                if let Some(&s) = buf.get(read as usize) {
                    mix += s * c.gain * self.lane_gain[c.lane as usize];
                }
            }
            *sample += mix;
        }

        transport.advance(out.len() as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;

    #[test]
    fn playhead_moves_only_when_playing() {
        let mut t = Transport::new(120.0, SR);
        assert_eq!(t.advance(256), 0);
        assert_eq!(t.playhead, 0, "stopped transport does not advance");
        t.play();
        t.advance(256);
        assert_eq!(t.playhead, 256);
    }

    #[test]
    fn seek_to_beat_lands_on_grid() {
        let mut t = Transport::new(120.0, SR);
        let iv = t.grid.beat_interval as u64;
        assert_eq!(iv, 24_000);
        t.seek_to_beat(4);
        assert_eq!(t.playhead, 4 * iv);
        assert!((t.beat_position() - 4.0).abs() < 1e-4);
    }

    #[test]
    fn loop_region_wraps_playhead() {
        let mut t = Transport::new(120.0, SR);
        t.set_loop(1_000, 1_100);
        t.seek(1_050);
        t.play();
        let wraps = t.advance(80);
        assert_eq!(wraps, 1);
        assert_eq!(t.playhead, 1_030);
        let wraps = t.advance(250);
        assert_eq!(wraps, 2);
        assert_eq!(t.playhead, 1_080);
    }

    #[test]
    fn render_pulls_clip_only_within_window() {
        let mut t = Transport::new(120.0, SR);
        t.play();
        let mut arr = Arrangement::new();
        let pcm = vec![1.0_f32; 8];
        let bufs: [&[f32]; 1] = [&pcm];
        arr.add_clip(Clip { lane: 0, buffer_id: 0, start: 4, len: 4, buf_offset: 0, gain: 1.0, muted: false }).unwrap();
        let mut out = vec![0.0_f32; 10];
        arr.render_block(&mut t, &mut out, &bufs);
        for (i, &s) in out.iter().enumerate() {
            let expect = if (4..8).contains(&i) { 1.0 } else { 0.0 };
            assert!((s - expect).abs() < 1e-6, "sample {i} = {s}, want {expect}");
        }
        assert_eq!(t.playhead, 10);
    }

    #[test]
    fn lane_mute_and_gain_apply() {
        let mut t = Transport::new(120.0, SR);
        t.play();
        let mut arr = Arrangement::new();
        let pcm = vec![1.0_f32; 8];
        let bufs: [&[f32]; 1] = [&pcm];
        arr.add_clip(Clip { lane: 0, buffer_id: 0, start: 0, len: 8, buf_offset: 0, gain: 0.5, muted: false }).unwrap();
        arr.add_clip(Clip { lane: 1, buffer_id: 0, start: 0, len: 8, buf_offset: 0, gain: 1.0, muted: false }).unwrap();
        arr.set_lane_muted(1, true);
        let mut out = vec![0.0_f32; 8];
        arr.render_block(&mut t, &mut out, &bufs);
        for &s in &out {
            assert!((s - 0.5).abs() < 1e-6, "muted lane must be silent, got {s}");
        }
    }

    #[test]
    fn stopped_transport_renders_nothing() {
        let mut t = Transport::new(120.0, SR);
        let mut arr = Arrangement::new();
        let pcm = vec![1.0_f32; 8];
        let bufs: [&[f32]; 1] = [&pcm];
        arr.add_clip(Clip { lane: 0, buffer_id: 0, start: 0, len: 8, buf_offset: 0, gain: 1.0, muted: false }).unwrap();
        let mut out = vec![0.0_f32; 8];
        arr.render_block(&mut t, &mut out, &bufs);
        assert!(out.iter().all(|&s| s == 0.0), "stopped → silence");
        assert_eq!(t.playhead, 0);
    }
}
