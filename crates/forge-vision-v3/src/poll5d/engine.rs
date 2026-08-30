//! From F:\NewRepo\crates\forge-vision\src\poll5d\engine.rs (lines 1-248)
//! Poll5dEngine: frame to contacts to 5D index to sketches to AIMD pace.

use crate::poll5d::contact::ContactExtractor;
use crate::poll5d::octal::{grid_delta, lum_grid, oct_pack, oct_tier, QuantCountMin};
use crate::poll5d::pace::Pacer;
use crate::poll5d::sketch::{fnv1a64, Distinct, Ewma};
use crate::poll5d::spatial::{Index5D, P5};

/// Configuration for Poll5dEngine polling behavior.
#[derive(Debug, Clone, Copy)]
pub struct PollCfg {
    pub tile: usize,
    pub ring_cap: usize,
    pub window_ticks: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub step_ms: u64,
    pub ewma_alpha_pmy: i64,
    pub colour_stride: usize,
    pub colour_cap: usize,
    pub lum_tol: u32,
}

impl Default for PollCfg {
    fn default() -> Self {
        Self {
            tile: 16,
            ring_cap: 2048,
            window_ticks: 600,
            min_ms: 120,
            max_ms: 2000,
            step_ms: 120,
            ewma_alpha_pmy: 3000,
            colour_stride: 37,
            colour_cap: 512,
            lum_tol: 0,
        }
    }
}

/// Report from a single poll tick.
#[derive(Debug, Clone, Copy)]
pub struct PollReport {
    pub tick: u64,
    pub total: usize,
    pub recent: usize,
    pub recent_weight: u64,
    pub changed_tiles: u32,
    pub distinct_colours: usize,
    pub trend: i64,
    pub interval_ms: u64,
    pub deduped: bool,
    pub morton_span: Option<(u64, u64)>,
    pub octal_digest: u64,
    pub hot: u8,
}

impl PollReport {
    /// Format report as a single-line log string.
    pub fn line(&self) -> String {
        let span = self.morton_span.map(|(lo, hi)| (hi - lo).to_string()).unwrap_or_else(|| "-".into());
        format!(
            "tick={} {} 5d[total={} recent={} weight={} span={}] changed={} colours={} trend={} hot={} oct=0x{:015x} next={}ms",
            self.tick,
            if self.deduped { "still" } else { "live" },
            self.total, self.recent, self.recent_weight, span,
            self.changed_tiles, self.distinct_colours, self.trend, self.hot,
            self.octal_digest, self.interval_ms,
        )
    }
}

/// 5D polling engine: frame deduplication and contact tracking.
pub struct Poll5dEngine {
    cfg: PollCfg,
    ex: ContactExtractor,
    idx: Index5D,
    pacer: Pacer,
    distinct: Distinct,
    ewma: Ewma,
    cm: QuantCountMin,
    prev_lum: Vec<u8>,
    last_distinct: usize,
    z: i32,
    s: u32,
}

impl Poll5dEngine {
    /// Create a new engine with the given configuration.
    pub fn new(cfg: PollCfg) -> Self {
        Self {
            ex: ContactExtractor::new(cfg.tile),
            idx: Index5D::new(cfg.ring_cap, cfg.window_ticks),
            pacer: Pacer::new(cfg.min_ms, cfg.max_ms, cfg.step_ms),
            distinct: Distinct::new(cfg.colour_cap),
            ewma: Ewma::new(cfg.ewma_alpha_pmy),
            cm: QuantCountMin::new(4, 256),
            prev_lum: Vec::new(),
            last_distinct: 0,
            z: 0,
            s: 0,
            cfg,
        }
    }

    fn digest(&self, changed: u32, distinct: usize, trend: i64, recent: usize, hot: u8) -> u64 {
        oct_pack(&[
            oct_tier(changed as u64, 64),
            oct_tier(distinct as u64, self.cfg.colour_cap as u64),
            oct_tier(trend.max(0) as u64, 64),
            oct_tier(recent as u64, self.cfg.ring_cap as u64),
            hot & 7,
        ])
    }

    /// Set the compositor layer and LoD for subsequent contacts.
    pub fn set_plane(&mut self, z: i32, s: u32) {
        self.z = z;
        self.s = s;
    }

    /// Get current polling interval in milliseconds.
    pub fn interval_ms(&self) -> u64 {
        self.pacer.interval_ms()
    }

    /// Get k hottest contacts (closest to frame center).
    pub fn hotspots(&self, k: usize) -> Vec<(u32, i64)> {
        let (tx, ty) = self.ex.grid();
        self.idx.knn(&P5::new((tx / 2) as i32, (ty / 2) as i32, self.z, 0, self.s), k)
    }

    /// Process a frame and return a poll report.
    pub fn tick(&mut self, rgba: &[u8], w: u32, h: u32, tick: u64) -> PollReport {
        let lum = lum_grid(rgba, w, h, self.cfg.tile);
        let deduped = !self.prev_lum.is_empty() && grid_delta(&self.prev_lum, &lum) <= self.cfg.lum_tol;
        self.prev_lum = lum;

        if deduped {
            self.idx.prune(tick);
            let trend = self.ewma.update(0);
            self.pacer.on_idle();
            let recent = self.idx.recent(tick);
            return PollReport {
                tick,
                total: self.idx.len(),
                recent,
                recent_weight: self.idx.recent_weight(tick),
                changed_tiles: 0,
                distinct_colours: self.last_distinct,
                trend,
                interval_ms: self.pacer.interval_ms(),
                deduped: true,
                morton_span: self.idx.morton_span(),
                octal_digest: self.digest(0, self.last_distinct, trend, recent, 0),
                hot: 0,
            };
        }

        let contacts = self.ex.extract(rgba, w, h, tick, self.z, self.s);
        let changed = contacts.len() as u32;
        let mut hot = 0u8;
        for (p, wt) in &contacts {
            self.idx.insert(*p, *wt);
            let key = fnv1a64(&[p.x.to_le_bytes(), p.y.to_le_bytes()].concat());
            self.cm.add(key);
            hot = hot.max(self.cm.estimate(key));
        }
        self.cm.decay();
        self.idx.prune(tick);

        self.distinct.clear();
        for px in rgba.chunks_exact(4).step_by(self.cfg.colour_stride.max(1)) {
            self.distinct.add(px);
        }
        let distinct = self.distinct.count();
        self.last_distinct = distinct;

        let trend = self.ewma.update(changed as i64);
        self.pacer.observe(changed);
        let recent = self.idx.recent(tick);

        PollReport {
            tick,
            total: self.idx.len(),
            recent,
            recent_weight: self.idx.recent_weight(tick),
            changed_tiles: changed,
            distinct_colours: distinct,
            trend,
            interval_ms: self.pacer.interval_ms(),
            deduped: false,
            morton_span: self.idx.morton_span(),
            octal_digest: self.digest(changed, distinct, trend, recent, hot),
            hot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: usize, h: usize, c: [u8; 4]) -> Vec<u8> {
        std::iter::repeat_n(c, w * h).flatten().collect()
    }

    #[test]
    fn still_frame_dedups_and_paces_up() {
        let mut e = Poll5dEngine::new(PollCfg::default());
        let f = solid(64, 64, [10, 20, 30, 255]);
        let r1 = e.tick(&f, 64, 64, 1);
        assert!(!r1.deduped);
        let start = r1.interval_ms;
        let r2 = e.tick(&f, 64, 64, 2);
        assert!(r2.deduped);
        assert_eq!(r2.changed_tiles, 0);
        assert!(r2.interval_ms >= start);
    }

    #[test]
    fn motion_produces_contacts_and_snaps_pace_down() {
        let mut e = Poll5dEngine::new(PollCfg { min_ms: 100, max_ms: 2000, step_ms: 100, ..PollCfg::default() });
        let f0 = solid(64, 64, [0, 0, 0, 255]);
        e.tick(&f0, 64, 64, 1);
        for t in 2..8 {
            e.tick(&f0, 64, 64, t);
        }
        let grown = e.interval_ms();

        let mut f1 = f0.clone();
        for y in 0..32usize {
            for x in 0..32usize {
                let i = (y * 64 + x) * 4;
                f1[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
            }
        }
        let r = e.tick(&f1, 64, 64, 9);
        assert!(!r.deduped);
        assert!(r.changed_tiles > 0);
        assert!(r.total > 0);
        assert!(r.interval_ms < grown);
    }

    #[test]
    fn distinct_colours_track_content() {
        let mut e = Poll5dEngine::new(PollCfg::default());
        let flat = solid(64, 64, [50, 50, 50, 255]);
        let r = e.tick(&flat, 64, 64, 1);
        assert_eq!(r.distinct_colours, 1);
    }
}
