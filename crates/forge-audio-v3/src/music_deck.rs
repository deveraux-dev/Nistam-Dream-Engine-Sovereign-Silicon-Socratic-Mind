//! MusicDeck — the front door's music player. A WIRE, not an engine.
//!
//! Every part here already existed and had no live caller on the launcher:
//! [`crate::radio_db::RadioDb`] is the library (tracks table, `search_paged`,
//! `track_count`), [`crate::scanner::LibraryScanner`] in `fast` mode is the
//! indexer (tags + header, no BPM/key/waveform analysis), and
//! [`crate::mixer::Deck`] is the transport (tempo, loop, waveform cache).
//! This module holds them together and hands the host ONE method it can call
//! from its existing block loop: [`MusicDeck::mix_into`].
//!
//! The level lane is NOT re-tapped. The studio already publishes
//! `TELEMETRY.master_rms_db` from its device push (`main.rs:4362`), so mixing
//! into that same block means the meter, the orb swarm and the launcher stars
//! all read one real signal.
//!
//! Allocation: construction, load and scan are LOAD-TIME (forge-audio#domain:
//! zero-alloc binds the realtime callback only). `mix_into` allocates nothing —
//! `Deck::read_block` recycles its own scratch buffers.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::mixer::Deck;
use crate::radio_db::{LibraryTrack, RadioDb, SortCol};

/// How many rows the front-door strip keeps in hand. The library may hold
/// thousands; a discreet strip shows a page, and paging is a DB query away.
pub const PAGE: usize = 64;

/// A library + one deck, sized for a front door rather than a DJ console.
pub struct MusicDeck {
    db: Option<Arc<Mutex<RadioDb>>>,
    deck: Deck,
    /// The current page of the library, for the query last passed to `refresh`.
    page: Vec<LibraryTrack>,
    /// Index into [`Self::page`] of the row that is loaded on the deck.
    cursor: usize,
    /// Rows the background scan has indexed so far. Shared with the scan
    /// thread so the strip can say "1204 tracks" while the walk is still
    /// running — a library that appears progressively beats one that appears
    /// after a minute of nothing.
    scanned: Arc<AtomicUsize>,
    /// True while a scan thread is live.
    scanning: Arc<AtomicBool>,
    /// Last successfully read library size. See [`Self::track_count`] for why
    /// this is cached rather than queried on demand.
    last_count: Arc<AtomicUsize>,
    /// Playback gain in PERMYRIAD (10000 = unity). A DEFINED field with a
    /// control on it, not an implicit 1.0 buried in a call site (Sean
    /// 2026-08-03, "if we can define it or make a slider or button").
    gain_q: i64,
    /// Mute, held SEPARATELY from gain so unmuting restores the level the
    /// slider was at rather than an arbitrary default.
    muted: bool,
    /// The column the page is ordered by, and its direction.
    ///
    /// Sorting happens in SQL, not on `page`: the page is 64 of a library that
    /// holds thousands, so an in-memory sort would only reorder the window and
    /// call it sorted. `search_paged` already took `order_by`/`order_dir` —
    /// [`Self::refresh`] pinned them to `"artist"`/default and this is the
    /// state that unpins them.
    sort_col: SortCol,
    sort_asc: bool,
    /// Library filters, likewise already accepted by `search_paged` and
    /// likewise passed as `None` until now (Sean 2026-08-03, "none = 0").
    /// `None` means "no filter", which is a different thing from a filter
    /// that matches nothing.
    genre_filter: Option<String>,
    bpm_filter: Option<(f64, f64)>,
    key_filter: Option<String>,
    /// The query [`Self::refresh`] last ran, so a sort or filter change can
    /// re-run the SAME search without the caller re-supplying it.
    query: String,
}

/// Gain slider travel: one detent per press. Discrete, like the star slider —
/// the front door has buttons, not drags.
pub const GAIN_STEP_Q: i64 = 1_000;
/// Unity gain. The boot value, named so it is a decision on the record.
pub const GAIN_UNITY_Q: i64 = 10_000;
/// Ceiling. Above unity the mix into the host block starts clipping the
/// device push, which clamps at ±1.0 — so the slider stops where the sound
/// stops improving.
pub const GAIN_MAX_Q: i64 = 12_000;

/// The ONE deck, shared by the three scopes that need it: the audio block loop
/// (mixes it), the press handler (turns it) and the door render (reads its
/// words). They are separate scopes in the host, so a borrow cannot join them —
/// the same reason `TELEMETRY` is a static rather than a passed handle.
///
/// Locked with `try_lock` at every site: a contended frame SKIPS rather than
/// blocks, because the audio block loop must never wait on a UI thread.
static SHARED: std::sync::OnceLock<Mutex<MusicDeck>> = std::sync::OnceLock::new();

/// The shared deck, born headless on first touch. Headless so a host that never
/// calls [`install`] still gets a deck that answers every question with a
/// defined value instead of a panic.
pub fn shared() -> &'static Mutex<MusicDeck> {
    SHARED.get_or_init(|| Mutex::new(MusicDeck::headless())) // @forge:allow_alloc one-time init, never a frame
}

/// Install the real deck, REPLACING whatever is there.
///
/// Not `OnceLock::set`: the host's door renders before its audio stack is
/// built, so the render's first `shared()` read wins the `get_or_init` race
/// and installs a headless deck. `set` then fails, the real deck is dropped on
/// the floor, and the front door reports an empty library forever with no
/// error anywhere — which is exactly what the first capture showed.
///
/// Ordering is not something a host should have to get right for the library
/// to appear, so this does not depend on it.
pub fn install(deck: MusicDeck) {
    match shared().lock() {
        Ok(mut slot) => *slot = deck,
        // A poisoned deck means a previous holder panicked mid-mix. Take it
        // anyway — the replacement is a fresh deck, which is the repair.
        Err(p) => *p.into_inner() = deck,
    }
}

/// The three EQ bands, named so a caller cannot pass the wrong index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    Low,
    Mid,
    High,
}

/// Full kill, in whole dB. `DeckEQ` documents -60 as full kill and
/// `dsp::eq_3band` treats anything under 0.5 dB as flat, so these are the
/// pre-existing ends of a pre-existing ladder, not a new scale.
pub const KILL_DB: i64 = -60;
/// The other end of the band slider.
pub const BOOST_MAX_DB: i64 = 12;
/// One detent of the band slider, whole dB.
pub const BAND_STEP_DB: i64 = 3;

impl MusicDeck {
    /// Open (or create) the library DB. A DB that will not open leaves a deck
    /// that still plays whatever is handed to it directly — the front door
    /// must not fail to paint because a file is locked.
    pub fn open(db_path: &Path, device_sample_rate: u32) -> Self {
        let db = RadioDb::open(&db_path.to_string_lossy())
            .map_err(|e| log::warn!("[music_deck] library {db_path:?}: {e}"))
            .ok()
            .map(|d| Arc::new(Mutex::new(d))); // @forge:allow_alloc cold open, once per boot
        let mut out = Self::headless();
        out.db = db;
        out.deck.device_sample_rate = device_sample_rate;
        out
    }

    /// A deck with no library — the headless/test shape and the fallback when
    /// the DB will not open.
    pub fn headless() -> Self {
        Self {
            db: None,
            deck: Deck::default(),
            page: Vec::new(),                            // @forge:allow_alloc cold init
            cursor: 0,
            scanned: Arc::new(AtomicUsize::new(0)),      // @forge:allow_alloc cold init
            scanning: Arc::new(AtomicBool::new(false)),  // @forge:allow_alloc cold init
            last_count: Arc::new(AtomicUsize::new(0)),   // @forge:allow_alloc cold init
            gain_q: GAIN_UNITY_Q,
            muted: false,
            // Artist-first, ascending — what `refresh` hardcoded, kept as the
            // boot value so this changes what is REACHABLE, not what is shown.
            sort_col: SortCol::Artist,
            sort_asc: true,
            genre_filter: None,
            bpm_filter: None,
            key_filter: None,
            query: String::new(),                        // @forge:allow_alloc cold init
        }
    }

    // ── Defined values, each with a control ──────────────────────────────────
    //
    // Nothing here returns "unknown". A track with no BPM tag has a BPM of 0,
    // and 0 is a number the strip can print and a slider can sit at; `None`
    // rendered as "undefined" is a hole in the face (Sean 2026-08-03).

    /// Playback gain, permyriad.
    pub fn gain_q(&self) -> i64 {
        self.gain_q
    }
    /// Set gain directly, clamped to `0..=GAIN_MAX_Q`.
    pub fn set_gain_q(&mut self, q: i64) {
        self.gain_q = q.clamp(0, GAIN_MAX_Q);
    }
    /// The slider: one detent up (`+1`) or down (`-1`). Returns the new value.
    /// Turning the slider off mute UNMUTES — a knob that moves while the sound
    /// stays off is a knob that lies.
    pub fn nudge_gain(&mut self, detents: i64) -> i64 {
        self.muted = false;
        self.set_gain_q(self.gain_q + detents * GAIN_STEP_Q);
        self.gain_q
    }

    /// True while muted. Distinct from `gain_q == 0`: mute REMEMBERS the level,
    /// so unmuting returns to where the slider was rather than to silence-plus-one-detent.
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// The MUTE button.
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }

    /// Gain as the strip prints it: whole percent of unity, or 0 while muted.
    /// A number, always — `mute` is not a hole in the readout.
    pub fn gain_pct(&self) -> i64 {
        if self.muted { 0 } else { self.gain_q * 100 / GAIN_UNITY_Q }
    }

    /// The effective gain the mix uses: the slider, gated by mute.
    fn live_gain(&self) -> f32 {
        if self.muted { 0.0 } else { self.gain_q as f32 / GAIN_UNITY_Q as f32 }
    }

    /// The loaded track's BPM, rounded. 0 when the library never detected one
    /// (the front door's fast scan does not run BPM detect — that is the DJ
    /// console's arm), so 0 here means "not measured", printed as 0.
    pub fn bpm(&self) -> i64 {
        let mbpm = self.page.get(self.cursor).and_then(|t| t.bpm_mbpm).unwrap_or(0);
        ((mbpm as i64) + 500) / 1000
    }

    /// The loaded track's Camelot key, or "" — never the word "none".
    pub fn key(&self) -> &str {
        self.page
            .get(self.cursor)
            .and_then(|t| t.key.as_deref())
            .unwrap_or("")
    }

    /// The loaded track's duration in whole seconds. 0 when unmeasured.
    pub fn duration_secs(&self) -> i64 {
        let ms = self.page.get(self.cursor).and_then(|t| t.duration_ms).unwrap_or(0).max(0);
        (ms as i64) / 1000
    }

    // ── EQ: kill the lows, mids or highs ─────────────────────────────────────
    //
    // Drained, not built. `DeckEQ` + `dsp::eq_3band` (low shelf 200Hz, mid
    // peak, high shelf) are the mixer's own 3-band, with the deck's persistent
    // `BiquadState` carried across blocks so a filter does not click at every
    // block boundary. `Mixer::mix_block:960` was its only caller; the front
    // door is the second.

    fn band_mut(&mut self, band: Band) -> &mut f32 {
        match band {
            Band::Low => &mut self.deck.params.eq.low,
            Band::Mid => &mut self.deck.params.eq.mid,
            Band::High => &mut self.deck.params.eq.high,
        }
    }

    /// This band's gain in whole dB. 0 is flat — a defined number, always.
    pub fn band_db(&self, band: Band) -> i64 {
        let v = match band {
            Band::Low => self.deck.params.eq.low,
            Band::Mid => self.deck.params.eq.mid,
            Band::High => self.deck.params.eq.high,
        };
        v.round() as i64
    }

    /// The BUTTON: kill this band outright, or restore it to flat.
    pub fn toggle_band_kill(&mut self, band: Band) {
        let killed = self.band_db(band) <= KILL_DB;
        *self.band_mut(band) = if killed { 0.0 } else { KILL_DB as f32 };
    }

    /// The SLIDER: one detent up or down, clamped to the ladder's ends.
    pub fn nudge_band(&mut self, band: Band, detents: i64) -> i64 {
        let next = (self.band_db(band) + detents * BAND_STEP_DB).clamp(KILL_DB, BOOST_MAX_DB);
        *self.band_mut(band) = next as f32;
        next
    }

    /// This band as the MIXER PANEL states it: permyriad 0..10000, centre
    /// 5000 = flat (`dead_drop_daw.kit.vixi:28`, "3-band EQ (0..10000
    /// center=5000)"). The panel's sliders are already authored against that
    /// scale, so the deck speaks it rather than asking the kit to change.
    pub fn band_q(&self, band: Band) -> i64 {
        let db = self.band_db(band);
        if db >= 0 {
            5_000 + db * 5_000 / BOOST_MAX_DB
        } else {
            5_000 - db * 5_000 / KILL_DB
        }
    }

    /// Set this band from a mixer-panel slider position (permyriad, centre 5000).
    pub fn set_band_q(&mut self, band: Band, q: i64) {
        let q = q.clamp(0, 10_000);
        let db = if q >= 5_000 {
            (q - 5_000) * BOOST_MAX_DB / 5_000
        } else {
            (5_000 - q) * KILL_DB / 5_000
        };
        *self.band_mut(band) = db.clamp(KILL_DB, BOOST_MAX_DB) as f32;
    }

    // ── Vocal kill ───────────────────────────────────────────────────────────

    /// True when the vocal (centre) cancel is armed.
    ///
    /// This reads `Deck.stems.vocal_muted` — the flag `mixer_cmd.rs:222` has
    /// been toggling since it was written with NO code anywhere reading it in
    /// a mix (root#rank: declared and toggled, never exercised).
    /// [`Self::mix_into`] is its first live consumer.
    pub fn vocal_killed(&self) -> bool {
        self.deck.stems.vocal_muted
    }

    /// The BUTTON: arm or disarm the vocal cancel.
    pub fn toggle_vocal_kill(&mut self) {
        self.deck.stems.vocal_muted = !self.deck.stems.vocal_muted;
    }

    /// Index `root` into the library on a BACKGROUND thread, tags-only.
    ///
    /// `fast = true` is the pre-existing scanner's own tags-only arm: no BPM
    /// detect, no key detect, no waveform blob. That is the difference between
    /// a 5000-track root landing in seconds and landing in an hour, and it is
    /// why the front door uses this arm and the DJ console uses the other one.
    /// Re-scanning is cheap: `process_file` returns early on a path already in
    /// the DB, so the second boot walks the tree and inserts nothing.
    pub fn scan_root(&self, root: &Path) {
        let (Some(db), false) = (self.db.clone(), self.is_scanning()) else { return }; // @forge:allow_alloc Arc bump, cold path
        let root = root.to_path_buf();          // @forge:allow_alloc cold path
        let scanned = self.scanned.clone();     // @forge:allow_alloc Arc bump, cold path
        let scanning = self.scanning.clone();   // @forge:allow_alloc Arc bump, cold path
        scanning.store(true, Ordering::Relaxed);
        std::thread::spawn(move || {
            let counter = scanned.clone();      // @forge:allow_alloc Arc bump, scan thread
            let scanner = crate::scanner::LibraryScanner::new(db);
            let res = scanner.scan_directory(&root, true, None, move |p| {
                counter.store(p.scanned, Ordering::Relaxed);
            });
            match res {
                Ok(r) => log::info!(
                    "[music_deck] indexed {} of {} in {:.1}s",
                    r.new_tracks, r.total, r.duration_secs
                ),
                Err(e) => log::warn!("[music_deck] scan {root:?}: {e}"),
            }
            scanning.store(false, Ordering::Relaxed);
        });
    }

    /// Re-query the library page. `query` empty = the whole library.
    ///
    /// Order and filters come from this deck's browse state, NOT from a wall of
    /// `None` at the call site — every argument `search_paged` accepts now has
    /// something on the deck that can set it.
    pub fn refresh(&mut self, query: &str) {
        if query != self.query {
            self.query = query.to_string(); // @forge:allow_alloc on a query change, not per frame
        }
        self.requery();
    }

    /// Re-run the LAST query with the current sort and filters. This is what a
    /// header click or a filter press calls — the search text has not changed,
    /// only how the library is being looked at.
    pub fn requery(&mut self) {
        let Some(db) = &self.db else { return };
        // A scan holds this briefly; a skipped refresh costs one frame of
        // staleness, a blocked one costs a frame of the whole window.
        let Ok(guard) = db.try_lock() else { return };
        // `bpm_filter` stays f64 (UI-facing, a slider range) — converted to
        // milli-BPM only at this DB-query boundary.
        let (bpm_min, bpm_max) = match self.bpm_filter {
            Some((lo, hi)) => (Some((lo * 1000.0).round() as i32), Some((hi * 1000.0).round() as i32)),
            None => (None, None),
        };
        let res = guard.search_paged(
            &self.query,
            self.genre_filter.as_deref(),
            bpm_min,
            bpm_max,
            self.key_filter.as_deref(),
            Some(self.sort_col.as_sql()),
            Some(if self.sort_asc { "asc" } else { "desc" }),
            PAGE,
            0,
        );
        match res {
            Ok(rows) => {
                self.page = rows;
                self.cursor = self.cursor.min(self.page.len().saturating_sub(1));
            }
            Err(e) => log::warn!("[music_deck] search: {e}"),
        }
    }

    // ── Browse state: sort + filters ─────────────────────────────────────────

    pub fn sort_col(&self) -> SortCol { self.sort_col }
    pub fn sort_asc(&self) -> bool { self.sort_asc }

    /// Click a column header. Same column flips direction; a new column starts
    /// ascending. Re-queries, because the order lives in SQL.
    pub fn toggle_sort(&mut self, col_idx: usize) {
        let col = SortCol::from_idx(col_idx);
        if self.sort_col == col { self.sort_asc = !self.sort_asc; }
        else { self.sort_col = col; self.sort_asc = true; }
        self.requery();
    }

    /// Restrict the page to one genre, or `None` for all of them.
    pub fn set_genre_filter(&mut self, genre: Option<&str>) {
        self.genre_filter = genre.map(str::to_string); // @forge:allow_alloc on a press
        self.requery();
    }

    /// Restrict the page to a BPM window, or `None` for the whole range.
    pub fn set_bpm_filter(&mut self, range: Option<(f64, f64)>) {
        self.bpm_filter = range;
        self.requery();
    }

    /// Restrict the page to one Camelot key, or `None` for every key.
    pub fn set_key_filter(&mut self, key: Option<&str>) {
        self.key_filter = key.map(str::to_string); // @forge:allow_alloc on a press
        self.requery();
    }

    /// Camelot keys that mix with the loaded track — the harmonic set a DJ
    /// reaches for. Empty when the track has no key tag. Delegates to
    /// [`crate::camelot::compatible_keys`], the wheel this repo already owns.
    pub fn compatible_keys(&self) -> Vec<String> {
        match self.key() {
            "" => Vec::new(), // @forge:allow_alloc empty vec, no allocation; cold path
            k => crate::camelot::compatible_keys(k),
        }
    }

    /// True if `key` mixes harmonically with what is loaded — the highlight
    /// test for a track row.
    pub fn is_harmonic_match(&self, key: &str) -> bool {
        !key.is_empty()
            && self.compatible_keys().iter().any(|c| c.eq_ignore_ascii_case(key))
    }

    /// Drop every filter. The search text is left alone — clearing filters is
    /// not the same press as clearing the search.
    pub fn clear_filters(&mut self) {
        self.genre_filter = None;
        self.bpm_filter = None;
        self.key_filter = None;
        self.requery();
    }

    /// Total rows in the library, or the live scan count while indexing.
    ///
    /// CACHED, because the honest answer under contention is "what it was",
    /// not "zero". A running scan keeps eight worker threads spinning on the
    /// DB mutex, so a `try_lock` from the render thread loses most of the time
    /// — and the first build of this printed a confident `0` on the front door
    /// while `radio.db` held 4000 rows. A number that reads as an empty library
    /// is worse than a number one refresh out of date.
    pub fn track_count(&self) -> usize {
        let scanned = self.scanned.load(Ordering::Relaxed);
        if scanned > 0 {
            return scanned;
        }
        self.last_count.load(Ordering::Relaxed)
    }

    /// Re-read the library size and cache it. Called on the same slow beat as
    /// [`Self::refresh`] — off the lock that caller already holds, never from
    /// the render path.
    pub fn recount(&self) {
        if let Some(n) = self
            .db
            .as_ref()
            .and_then(|d| d.try_lock().ok())
            .and_then(|g| g.track_count().ok())
        {
            self.last_count.store(n, Ordering::Relaxed);
        }
    }

    pub fn is_scanning(&self) -> bool {
        self.scanning.load(Ordering::Relaxed)
    }
    pub fn page(&self) -> &[LibraryTrack] {
        &self.page
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn is_playing(&self) -> bool {
        self.deck.params.playing
    }

    /// Decode and start row `i` of the current page. Decode is synchronous —
    /// a press on a track is a deliberate act and the file is local; a
    /// background decode would need a loading state the front door has no
    /// place to show.
    pub fn play_index(&mut self, i: usize) -> Result<(), String> {
        let track = self.page.get(i).ok_or("row out of range")?.clone(); // @forge:allow_alloc one row, on press
        let buffer = crate::dsp::load_audio(&track.path)?;
        self.deck.load(buffer);
        self.deck.title = track.title.clone().unwrap_or_default();   // @forge:allow_alloc on press
        self.deck.artist = track.artist.clone().unwrap_or_default(); // @forge:allow_alloc on press
        self.deck.params.playing = true;
        self.cursor = i;
        Ok(())
    }

    /// Play/pause. A deck with nothing loaded loads the cursor row first, so
    /// one press on a cold front door starts music.
    pub fn toggle(&mut self) {
        if self.deck.buffer.is_none() {
            let _ = self.play_index(self.cursor);
            return;
        }
        self.deck.params.playing = !self.deck.params.playing;
    }

    /// Advance to the next row, wrapping. False when there is nothing to advance to.
    pub fn next(&mut self) -> bool {
        if self.page.is_empty() {
            return false;
        }
        let i = (self.cursor + 1) % self.page.len();
        self.play_index(i).is_ok()
    }

    /// Step back one row, wrapping.
    pub fn prev(&mut self) -> bool {
        if self.page.is_empty() {
            return false;
        }
        let i = self.cursor.checked_sub(1).unwrap_or(self.page.len() - 1);
        self.play_index(i).is_ok()
    }

    /// One line for the strip: what is on the deck right now.
    pub fn now_playing(&self) -> String {
        if self.deck.buffer.is_none() {
            return String::from("—"); // @forge:allow_alloc UI string, logic thread
        }
        let mark = if self.deck.params.playing { "▶" } else { "❙❙" };
        match (self.deck.artist.is_empty(), self.deck.title.is_empty()) {
            (true, true) => format!("{mark} track {}", self.cursor + 1), // @forge:allow_alloc UI string
            (true, false) => format!("{mark} {}", self.deck.title),      // @forge:allow_alloc UI string
            _ => format!("{mark} {} — {}", self.deck.artist, self.deck.title), // @forge:allow_alloc UI string
        }
    }

    /// Elapsed / total as `m:ss / m:ss` — the transport readout the strip prints.
    ///
    /// Derived from the DECK's own sample position and buffer length, not from
    /// the library's `duration_secs` tag: the tag is what the file claims, this
    /// is where the playhead actually is. Reads `0:00 / 0:00` with nothing
    /// loaded — defined, never blank.
    pub fn position_line(&self) -> String {
        let (pos, total) = match &self.deck.buffer {
            Some(b) if b.sample_rate > 0 => (
                self.deck.playback_pos as i64 / b.sample_rate as i64,
                b.len() as i64 / b.sample_rate as i64,
            ),
            _ => (0, 0),
        };
        let mmss = |s: i64| format!("{}:{:02}", s / 60, s % 60); // @forge:allow_alloc UI string
        format!("{} / {}", mmss(pos), mmss(total)) // @forge:allow_alloc UI string
    }

    /// Playback progress in PERMYRIAD (0..10000), 0 with nothing loaded.
    pub fn progress_q(&self) -> i64 {
        match &self.deck.buffer {
            Some(b) if b.len() > 0 => {
                (self.deck.playback_pos as i64 * 10_000 / b.len() as i64).clamp(0, 10_000)
            }
            _ => 0,
        }
    }

    /// Sum this deck's next block into `out` (mono), at the deck's own
    /// [`Self::gain_q`] — the host does not pass a loose float, it turns the
    /// slider.
    ///
    /// Called from the host's existing audio block loop AFTER its synth lane
    /// renders, so the music lands in the same buffer the device push and the
    /// `TELEMETRY` meter feed already read. That is the whole reason the stars
    /// react without a second audio tap and the ONE device stays one device.
    ///
    /// Zero-alloc: `Deck::read_block` writes into its own recycled scratch.
    pub fn mix_into(&mut self, out: &mut [f32]) {
        if !self.deck.params.playing || self.deck.buffer.is_none() || self.live_gain() == 0.0 {
            return;
        }
        let (lo, mid, hi) = (
            self.deck.params.eq.low,
            self.deck.params.eq.mid,
            self.deck.params.eq.high,
        );
        let vocal_kill = self.deck.stems.vocal_muted;
        let Some(mut block) = self.deck.read_block(out.len()) else { return };
        let chans = block.samples.len();
        if chans == 0 {
            return;
        }
        // The mixer's own 3-band, with the deck's persistent biquad state so
        // the filter does not restart (and click) at every block boundary.
        // `eq_3band` returns immediately when all three bands are flat.
        crate::dsp::eq_3band(&mut block, lo, mid, hi, &mut self.deck.eq_state);

        let gain = self.live_gain();
        // Centre-cancel: what is identical in L and R is, on almost every
        // mixed record, the lead vocal — so `L - R` removes it and keeps the
        // stereo sides. Cheap, exact, and honest about its limits: it also
        // takes the kick and the bass with it, and a MONO track has no side
        // channel at all, so cancelling it would delete the whole track. That
        // case falls through to the normal mono sum.
        let cancel = vocal_kill && chans >= 2;
        for (i, o) in out.iter_mut().enumerate() {
            let sample = if cancel {
                (block.samples[0].get(i).copied().unwrap_or(0.0)
                    - block.samples[1].get(i).copied().unwrap_or(0.0))
                    * 0.5
            } else {
                let mut sum = 0.0f32;
                for ch in block.samples.iter() {
                    sum += ch.get(i).copied().unwrap_or(0.0);
                }
                sum / chans as f32
            };
            *o += sample * gain;
        }
        // A deck that ran off the end stops itself, so the strip stops lying.
        if let Some(b) = &self.deck.buffer {
            if self.deck.playback_pos >= b.len() {
                self.deck.params.playing = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// Every column gets a DEFINED value — no `None` standing in for a number
    /// (Sean 2026-08-03, "undefined is 0"). A row here looks like a row the
    /// scanner writes, so a test passing does not mean a hole went unnoticed.
    fn deck_with_rows(rows: &[(&str, &str, &str)]) -> MusicDeck {
        let conn = Connection::open_in_memory().unwrap();
        let mut db = RadioDb::from_connection(conn).unwrap();
        for (path, artist, title) in rows {
            db.insert_track_full(
                path,            // path
                0,               // size_bytes
                path,            // hash (path is unique in these fixtures)
                Some(artist),
                Some("Test Album"),
                Some(title),
                Some("Electronic"),
                Some(2026),      // year
                180_000,         // duration_ms
                44_100,
                2,
                "mp3",
                Some(0),         // bpm_mbpm — 0 = not measured, never undefined
                Some(""),        // musical_key — "" = not measured
                Some("Other"),   // detected_genre
                Some(&[][..]),   // peaks blob — empty, defined
            )
            .unwrap();
        }
        let mut d = MusicDeck::headless();
        d.db = Some(Arc::new(Mutex::new(db)));
        d
    }

    #[test]
    fn refresh_pages_the_library_and_counts_it() {
        let mut d = deck_with_rows(&[
            ("/a.mp3", "Portishead", "Roads"),
            ("/b.mp3", "Nine Inch Nails", "Hurt"),
        ]);
        d.refresh("");
        assert_eq!(d.page().len(), 2);
        assert_eq!(d.track_count(), 0, "the count is CACHED — unread until recount");
        d.recount();
        assert_eq!(d.track_count(), 2);
    }

    #[test]
    fn refresh_with_a_query_filters() {
        let mut d = deck_with_rows(&[
            ("/a.mp3", "Portishead", "Roads"),
            ("/b.mp3", "Nine Inch Nails", "Hurt"),
        ]);
        d.refresh("portis");
        assert_eq!(d.page().len(), 1, "the query is a filter, not a suggestion");
        assert_eq!(d.page()[0].title.as_deref(), Some("Roads"));
    }

    /// Rows that DIFFER on the columns sort and filter act on. The fixture
    /// above pins genre/bpm/key to one value each, which cannot tell a working
    /// filter from a no-op filter.
    fn deck_with_varied(rows: &[(&str, &str, &str, &str, f64, &str)]) -> MusicDeck {
        let conn = Connection::open_in_memory().unwrap();
        let mut db = RadioDb::from_connection(conn).unwrap();
        for (path, artist, title, genre, bpm, key) in rows {
            db.insert_track_full(
                path, 0, path, Some(artist), Some("Test Album"), Some(title),
                Some(genre), Some(2026), 180_000, 44_100, 2, "mp3",
                // `detected_genre` is None on purpose: `insert_track_full` writes
                // `detected_genre.or(genre)`, so a stand-in here would overwrite
                // the very column this fixture varies.
                Some((*bpm * 1000.0).round() as i32), Some(*key), None, Some(&[][..]),
            )
            .unwrap();
        }
        let mut d = MusicDeck::headless();
        d.db = Some(Arc::new(Mutex::new(db))); // @forge:allow_alloc test fixture
        d
    }

    /// The order is SQL's, so a header click must change the rows that come
    /// back — not just a field nobody reads.
    #[test]
    fn toggle_sort_reorders_the_page() {
        let mut d = deck_with_rows(&[
            ("/a.mp3", "Portishead", "Roads"),
            ("/b.mp3", "Nine Inch Nails", "Hurt"),
        ]);
        d.refresh("");
        assert_eq!(d.sort_col(), SortCol::Artist, "the boot order refresh used to hardcode");
        assert_eq!(d.page()[0].artist.as_deref(), Some("Nine Inch Nails"));

        d.toggle_sort(1); // same column → flip direction
        assert!(!d.sort_asc());
        assert_eq!(d.page()[0].artist.as_deref(), Some("Portishead"));

        d.toggle_sort(0); // new column → ascending
        assert_eq!(d.sort_col(), SortCol::Title);
        assert!(d.sort_asc());
        assert_eq!(d.page()[0].title.as_deref(), Some("Hurt"));
    }

    /// Each filter argument `search_paged` accepts used to be a literal `None`
    /// at the only call site (Sean 2026-08-03, "none = 0"). Each one now has a
    /// setter, and each setter has to actually narrow the page.
    #[test]
    fn every_filter_narrows_the_page() {
        let rows = &[
            ("/a.mp3", "Alpha", "One",   "Drum & Bass", 174.0, "8A"),
            ("/b.mp3", "Bravo", "Two",   "Ambient",      90.0, "3B"),
            ("/c.mp3", "Chief", "Three", "Drum & Bass", 172.0, "9A"),
        ];
        let mut d = deck_with_varied(rows);
        d.refresh("");
        assert_eq!(d.page().len(), 3);

        d.set_genre_filter(Some("Drum & Bass"));
        assert_eq!(d.page().len(), 2);

        d.set_bpm_filter(Some((173.0, 180.0)));
        assert_eq!(d.page().len(), 1, "genre AND bpm, not genre OR bpm");
        assert_eq!(d.page()[0].title.as_deref(), Some("One"));

        d.clear_filters();
        assert_eq!(d.page().len(), 3, "clearing must restore the whole library");

        d.set_key_filter(Some("3B"));
        assert_eq!(d.page().len(), 1);
        assert_eq!(d.page()[0].title.as_deref(), Some("Two"));
    }

    /// The harmonic set reads the LOADED row's key through the Camelot wheel
    /// forge-audio already owns — a 9A next to an 8A is the mix a DJ wants
    /// flagged, and a 3A is not.
    #[test]
    fn harmonic_match_follows_the_loaded_key() {
        let mut d = deck_with_varied(&[("/a.mp3", "Alpha", "One", "DnB", 174.0, "8A")]);
        d.refresh("");
        assert_eq!(d.key(), "8A");
        assert!(d.is_harmonic_match("9A"), "adjacent on the wheel");
        assert!(d.is_harmonic_match("8B"), "the relative mode switch");
        assert!(!d.is_harmonic_match("3A"), "across the wheel");
        assert!(!d.is_harmonic_match(""), "an untagged track is never a match");
    }

    /// A deck with no library must answer every browse press with a defined
    /// value rather than a panic — the same law `headless` exists for.
    #[test]
    fn browse_state_is_safe_without_a_library() {
        let mut d = MusicDeck::headless();
        d.toggle_sort(2);
        d.set_genre_filter(Some("Ambient"));
        d.set_bpm_filter(Some((90.0, 100.0)));
        d.clear_filters();
        assert_eq!(d.sort_col(), SortCol::Genre);
        assert!(d.page().is_empty());
        assert!(d.compatible_keys().is_empty());
    }

    #[test]
    fn a_missing_file_is_an_error_not_a_panic() {
        let mut d = deck_with_rows(&[("Z:/gone.mp3", "Nobody", "Nothing")]);
        d.refresh("");
        assert!(d.play_index(0).is_err(), "a dead path must surface, not abort the door");
        assert!(!d.is_playing());
    }

    #[test]
    fn mix_into_is_silent_when_nothing_is_loaded() {
        let mut d = MusicDeck::headless();
        let mut out = [0.5f32; 8];
        d.mix_into(&mut out);
        assert_eq!(out, [0.5f32; 8], "an idle deck must not touch the host's block");
    }

    #[test]
    fn mix_into_sums_a_loaded_deck_into_the_host_block() {
        let mut d = MusicDeck::headless();
        d.deck.load(crate::dsp::AudioBuffer {
            samples: vec![vec![1.0f32; 256], vec![1.0f32; 256]],
            sample_rate: 48_000,
        });
        d.deck.params.playing = true;
        d.deck.device_sample_rate = 48_000;
        d.set_gain_q(GAIN_UNITY_Q / 2);
        let mut out = [0.0f32; 64];
        d.mix_into(&mut out);
        assert!(out.iter().any(|s| *s != 0.0), "a playing deck must reach the block");
        assert!(out.iter().all(|s| s.abs() <= 1.0));
    }

    /// A stereo fixture with a CENTRE element (equal in both channels, the
    /// vocal's position) and a SIDE element (opposite sign), so centre-cancel
    /// has something real to remove and something real to leave behind.
    fn stereo_deck() -> MusicDeck {
        let mut d = MusicDeck::headless();
        let left: Vec<f32> = (0..512).map(|i| 0.5 + if i % 2 == 0 { 0.25 } else { -0.25 }).collect(); // @forge:allow_alloc test fixture
        let right: Vec<f32> = (0..512).map(|i| 0.5 - if i % 2 == 0 { 0.25 } else { -0.25 }).collect(); // @forge:allow_alloc test fixture
        d.deck.load(crate::dsp::AudioBuffer { samples: vec![left, right], sample_rate: 48_000 }); // @forge:allow_alloc test fixture
        d.deck.params.playing = true;
        d.deck.device_sample_rate = 48_000;
        d
    }

    #[test]
    fn the_gain_slider_clamps_and_steps() {
        let mut d = MusicDeck::headless();
        assert_eq!(d.gain_q(), GAIN_UNITY_Q, "boot value is unity, not an anonymous zero");
        assert_eq!(d.nudge_gain(1), GAIN_UNITY_Q + GAIN_STEP_Q);
        for _ in 0..50 {
            d.nudge_gain(1);
        }
        assert_eq!(d.gain_q(), GAIN_MAX_Q, "the slider stops at its ceiling");
        for _ in 0..50 {
            d.nudge_gain(-1);
        }
        assert_eq!(d.gain_q(), 0, "and at its floor");
    }

    #[test]
    fn mute_silences_the_mix_and_remembers_the_level() {
        let mut d = stereo_deck();
        d.set_gain_q(7_000);
        d.toggle_mute();
        assert!(d.is_muted());
        assert_eq!(d.gain_pct(), 0, "a muted strip reads 0, not the level it would be");
        let mut out = [0.25f32; 32];
        d.mix_into(&mut out);
        assert_eq!(out, [0.25f32; 32], "mute must be silence, not a duck");
        d.toggle_mute();
        assert_eq!(d.gain_q(), 7_000, "unmute returns to the slider, not to a default");
        assert_eq!(d.gain_pct(), 70);
    }

    #[test]
    fn turning_the_slider_unmutes() {
        let mut d = MusicDeck::headless();
        d.toggle_mute();
        d.nudge_gain(-1);
        assert!(!d.is_muted(), "a knob that moves while the sound stays off is a knob that lies");
    }

    #[test]
    fn gain_zero_is_silence_not_a_quiet_mix() {
        let mut d = stereo_deck();
        d.set_gain_q(0);
        let mut out = [0.25f32; 32];
        d.mix_into(&mut out);
        assert_eq!(out, [0.25f32; 32]);
    }

    #[test]
    fn unmeasured_track_facts_read_as_zero_never_as_undefined() {
        let mut d = deck_with_rows(&[("/a.mp3", "Portishead", "Roads")]);
        d.refresh("");
        assert_eq!(d.bpm(), 0, "an unmeasured BPM is 0");
        assert_eq!(d.key(), "", "an unmeasured key is empty, not the word none");
        assert_eq!(d.duration_secs(), 180);
    }

    #[test]
    fn a_contended_count_keeps_the_last_number_never_drops_to_zero() {
        let mut d = deck_with_rows(&[("/a.mp3", "A", "One"), ("/b.mp3", "B", "Two")]);
        d.refresh("");
        d.recount();
        assert_eq!(d.track_count(), 2);
        // Hold the DB the way eight scan workers do, then ask again.
        let held = d.db.as_ref().unwrap().lock().unwrap();
        d.recount(); // try_lock loses
        assert_eq!(d.track_count(), 2, "contention must not report an empty library");
        drop(held);
    }

    #[test]
    fn an_empty_deck_still_answers_every_fact() {
        let d = MusicDeck::headless();
        assert_eq!(d.bpm(), 0);
        assert_eq!(d.key(), "");
        assert_eq!(d.duration_secs(), 0);
        assert_eq!(d.track_count(), 0);
        assert_eq!(d.band_db(Band::Low), 0, "flat is 0 dB, defined");
        assert!(!d.vocal_killed());
    }

    #[test]
    fn a_killed_band_is_minus_sixty_and_toggles_back_to_flat() {
        let mut d = MusicDeck::headless();
        for band in [Band::Low, Band::Mid, Band::High] {
            assert_eq!(d.band_db(band), 0);
            d.toggle_band_kill(band);
            assert_eq!(d.band_db(band), KILL_DB, "kill is full kill, not a duck");
            d.toggle_band_kill(band);
            assert_eq!(d.band_db(band), 0, "and back to flat");
        }
    }

    #[test]
    fn the_band_reads_on_the_mixer_panels_own_permyriad_scale() {
        let mut d = MusicDeck::headless();
        assert_eq!(d.band_q(Band::Low), 5_000, "flat is the panel's centre detent");
        d.toggle_band_kill(Band::Low);
        assert_eq!(d.band_q(Band::Low), 0, "full kill is the slider's floor");
        d.set_band_q(Band::Low, 10_000);
        assert_eq!(d.band_db(Band::Low), BOOST_MAX_DB, "the slider's ceiling is max boost");
        d.set_band_q(Band::Low, 5_000);
        assert_eq!(d.band_db(Band::Low), 0, "and the centre round-trips to flat");
    }

    #[test]
    fn the_band_slider_steps_between_kill_and_boost() {
        let mut d = MusicDeck::headless();
        for _ in 0..40 {
            d.nudge_band(Band::Mid, -1);
        }
        assert_eq!(d.band_db(Band::Mid), KILL_DB);
        for _ in 0..40 {
            d.nudge_band(Band::Mid, 1);
        }
        assert_eq!(d.band_db(Band::Mid), BOOST_MAX_DB);
    }

    #[test]
    fn killing_a_band_changes_what_reaches_the_block() {
        let mut flat = stereo_deck();
        let mut killed = stereo_deck();
        killed.toggle_band_kill(Band::Low);
        let (mut a, mut b) = ([0.0f32; 256], [0.0f32; 256]);
        flat.mix_into(&mut a);
        killed.mix_into(&mut b);
        assert!(
            a.iter().zip(b.iter()).any(|(x, y)| (x - y).abs() > 1e-4),
            "an EQ kill that changes no sample is a switch wired to nothing"
        );
    }

    #[test]
    fn vocal_kill_removes_the_centre_and_keeps_the_sides() {
        let mut d = stereo_deck();
        d.toggle_vocal_kill();
        assert!(d.vocal_killed());
        let mut out = [0.0f32; 256];
        d.mix_into(&mut out);
        // The fixture's centre is a constant +0.5 in both channels; the sides
        // are ±0.25 and survive. Cancelling the centre must therefore leave a
        // signal with no DC offset left in it.
        let mean: f32 = out.iter().sum::<f32>() / out.len() as f32;
        assert!(mean.abs() < 1e-3, "centre survived the cancel: mean {mean}");
        assert!(out.iter().any(|s| s.abs() > 1e-3), "the sides must survive too");
    }

    #[test]
    fn vocal_kill_on_a_mono_track_is_a_no_op_not_a_silence() {
        let mut d = MusicDeck::headless();
        d.deck.load(crate::dsp::AudioBuffer {
            samples: vec![vec![0.5f32; 256]], // @forge:allow_alloc test fixture
            sample_rate: 48_000,
        });
        d.deck.params.playing = true;
        d.toggle_vocal_kill();
        let mut out = [0.0f32; 64];
        d.mix_into(&mut out);
        assert!(
            out.iter().any(|s| s.abs() > 1e-3),
            "mono has no side channel to keep — cancelling would delete the track"
        );
    }

    #[test]
    fn now_playing_reads_empty_before_anything_loads() {
        assert_eq!(MusicDeck::headless().now_playing(), "—");
        assert_eq!(MusicDeck::headless().progress_q(), 0);
        assert_eq!(MusicDeck::headless().position_line(), "0:00 / 0:00");
    }

    #[test]
    fn the_position_line_counts_off_the_playhead() {
        let mut d = MusicDeck::headless();
        // 90 seconds at 48k, playhead parked at 65s.
        d.deck.load(crate::dsp::AudioBuffer {
            samples: vec![vec![0.0f32; 48_000 * 90]], // @forge:allow_alloc test fixture
            sample_rate: 48_000,
        });
        d.deck.playback_pos = 48_000 * 65;
        assert_eq!(d.position_line(), "1:05 / 1:30");
        assert_eq!(d.progress_q(), 7_222, "progress is the same fact in permyriad");
    }

    #[test]
    fn headless_deck_ignores_scan_and_refresh_without_a_db() {
        let mut d = MusicDeck::headless();
        d.scan_root(Path::new("Z:/no/such/root"));
        d.refresh("");
        assert!(!d.is_scanning());
        assert!(d.page().is_empty());
    }
}
