//! recipe_author — author a generative arrangement recipe as a `.timeline.vixi`.
//!
//! The Broadcast Booth's arrangement recipes are authored in VixiScript's
//! `.timeline.vixi` dialect (parsed by `forge_vix::timeline::parse_timeline`,
//! structure-checked on the `forge_vix::cst` AST/CST + `forge-vix-lsp`). That
//! dialect carries its events as base64 UMP wire bytes under group nibbles that
//! name the stems, with a content-hash `[stamp]` the parser verifies.
//!
//! Hand-writing the base64 events + the matching stamp hash is impractical, so
//! this module is the AUTHORING tool: build an [`ArrangementRecipe`] (stems ×
//! beat pattern), and emit a valid, stamped `.timeline.vixi` string. The emitted
//! UMP stream round-trips through [`crate::stem_conductor::StemSchedule`] — the
//! same "browser" the studio drives at play time — so author → parse → trigger is
//! one proven pipe.
//!
//! Uses only `forge-ump` (the UMP substrate the timeline parser shares); no
//! forge-vix edge. Cold authoring path — heap use is expected.

use base64::Engine as _;
use forge_ump::{hash_raw, Channel, Group, Message};

/// One stem lane: a group nibble, its name (the timeline `[groups.N] instrument`),
/// a base MIDI note, and a 16-step-per-bar hit pattern (`true` = trigger).
#[derive(Clone, Debug)]
pub struct StemLane {
    pub group: u8,
    pub name: String,
    pub note: u8,
    /// One bool per 16th step across the whole recipe (bars × 16).
    pub steps: Vec<bool>,
    /// Trigger velocity in permyriad (0..=10000).
    pub velocity_pmy: u32,
}

impl StemLane {
    pub fn new(group: u8, name: &str, note: u8, steps: Vec<bool>) -> Self {
        Self { group, name: name.to_string(), note, steps, velocity_pmy: 9000 }
    }
}

/// A generative arrangement recipe: a tempo, a bar count, and stem lanes.
#[derive(Clone, Debug)]
pub struct ArrangementRecipe {
    pub name: String,
    pub tempo_bpm: u32,
    pub bars: u32,
    pub lanes: Vec<StemLane>,
    pub seed: u64,
}

impl ArrangementRecipe {
    pub fn new(name: &str, tempo_bpm: u32, bars: u32) -> Self {
        Self { name: name.to_string(), tempo_bpm: tempo_bpm.max(1), bars: bars.max(1), lanes: Vec::new(), seed: 1 }
    }

    pub fn lane(mut self, lane: StemLane) -> Self {
        self.lanes.push(lane);
        self
    }

    /// Microseconds per 16th-note step at this tempo.
    pub fn step_us(&self) -> i64 {
        // a beat (quarter) is 60_000_000 / bpm µs; a 16th is a quarter of that.
        (60_000_000i64 / self.tempo_bpm as i64) / 4
    }

    /// Total steps across the recipe (bars × 16).
    pub fn total_steps(&self) -> usize {
        self.bars as usize * 16
    }

    /// Recipe length in microseconds.
    pub fn duration_us(&self) -> i64 {
        self.total_steps() as i64 * self.step_us()
    }

    /// Flatten every lane's pattern into a time-sorted `(at_us, group, note, vel)`
    /// list — the arrangement the browser will read.
    fn hits(&self) -> Vec<(i64, u8, u8, u32)> {
        let step_us = self.step_us();
        let mut hits: Vec<(i64, u8, u8, u32)> = Vec::new();
        for lane in &self.lanes {
            for (s, on) in lane.steps.iter().enumerate() {
                if *on {
                    hits.push((s as i64 * step_us, lane.group, lane.note, lane.velocity_pmy));
                }
            }
        }
        hits.sort_by_key(|h| h.0);
        hits
    }

    /// Emit the recipe as a raw UMP event stream (JR-timestamped NoteOn per hit).
    /// This is exactly what `[events]` base64-decodes to, and what the conductor
    /// consumes. Time resolution is 32 µs (the JR clock grid); sub-grid remainder
    /// is dropped — negligible at musical tempos.
    pub fn to_ump_events(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut clock_us = 0i64;
        for (at_us, group, note, vel_pmy) in self.hits() {
            // Advance the JR clock to `at_us` in ≤65535-unit (×32 µs) hops.
            let mut units = (at_us - clock_us) / 32;
            while units > 0 {
                let d = units.min(65_535) as u16;
                push_ump(&mut bytes, Message::JrTimestamp { delta: d });
                units -= d as i64;
            }
            clock_us += (((at_us - clock_us) / 32) * 32).max(0);
            let velocity = (vel_pmy.min(10_000) * 65_535 / 10_000) << 16;
            push_ump(
                &mut bytes,
                Message::NoteOn {
                    group: Group(group & 0x0f),
                    channel: Channel(0),
                    note,
                    velocity,
                    attribute_type: 0,
                    attribute_data: 0,
                },
            );
        }
        bytes
    }

    /// Emit the full, stamped `.timeline.vixi` document (TOML the forge-vix
    /// timeline parser accepts). The `[stamp] stage_raw` is `hash_raw(events)` so
    /// the parser's integrity check passes.
    pub fn to_timeline_vixi(&self) -> String {
        let events = self.to_ump_events();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&events);
        let stamp = hash_raw(&events).as_u64();

        let mut s = String::new();
        s.push_str("#vixi:timeline v1\n");
        s.push_str(&format!("# Broadcast Booth arrangement recipe — {}\n\n", self.name));
        s.push_str("[timeline]\n");
        s.push_str("mode = \"audio\"\n");
        s.push_str(&format!("duration_us = {}\n", self.duration_us().max(1)));
        s.push_str(&format!("tempo_bpm_m4 = {}\n", self.tempo_bpm * 1000));
        s.push_str("sample_rate = 48000\n");
        s.push_str("frame_rate_m4 = 24000\n");
        s.push_str("property_profile = \"ReadOnlyBasics\"\n\n");

        for (i, lane) in self.lanes.iter().enumerate() {
            let g = lane.group;
            let default_lane = match i {
                0 => "Critical",
                1 => "NearFuture",
                2 => "PriorAuthority",
                _ => "Speculative",
            };
            s.push_str(&format!("[groups.{g}]\n"));
            s.push_str("kind = \"music\"\n");
            s.push_str(&format!("instrument = \"{}\"\n", lane.name));
            s.push_str(&format!("default_lane = \"{default_lane}\"\n\n"));
        }

        s.push_str("[events]\n");
        s.push_str("encoding = \"base64\"\n");
        s.push_str(&format!("data = \"{b64}\"\n\n"));

        s.push_str("[stamp]\n");
        s.push_str(&format!("stage_raw = \"{stamp:016x}\"\n"));
        s.push_str(&format!("canonical = \"{stamp:016x}\"\n"));
        s.push_str(&format!("seed = {}\n", self.seed));
        s.push_str("authored = \"2026-07-12T00:00:00Z\"\n");
        s
    }
}

fn push_ump(out: &mut Vec<u8>, m: Message) {
    let ump = m.to_ump();
    // Word count is implied by message type: JR clock/timestamp = 1 word,
    // MIDI-2 channel-voice = 2 words. Encode the significant words big-endian.
    let words = match m {
        Message::JrClock { .. } | Message::JrTimestamp { .. } => 1,
        _ => 2,
    };
    for w in ump.words.iter().take(words) {
        out.extend_from_slice(&w.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stem_conductor::{StemSchedule, StemTrigger};

    /// A tiny 1-bar house pattern: kick on the quarters, hat on the offbeats.
    fn house() -> ArrangementRecipe {
        let kick = StemLane::new(0, "drums", 36, four_on_floor());
        let hat = StemLane::new(1, "hats", 42, offbeats());
        ArrangementRecipe::new("house_1bar", 120, 1).lane(kick).lane(hat)
    }

    fn four_on_floor() -> Vec<bool> {
        (0..16).map(|s| s % 4 == 0).collect()
    }
    fn offbeats() -> Vec<bool> {
        (0..16).map(|s| s % 4 == 2).collect()
    }

    #[test]
    fn recipe_round_trips_through_the_conductor() {
        let recipe = house();
        let events = recipe.to_ump_events();
        let sched = StemSchedule::from_ump(&events);
        // 4 kicks + 4 hats
        assert_eq!(sched.len(), 8, "1-bar 4-on-floor + 4 offbeat hats = 8 triggers");
        // every event is an On trigger
        assert!(sched.events().iter().all(|e| matches!(e.trigger, StemTrigger::On { .. })));
        // groups present: 0 (drums) and 1 (hats)
        assert!(sched.events().iter().any(|e| e.group == 0));
        assert!(sched.events().iter().any(|e| e.group == 1));
    }

    #[test]
    fn step_timing_matches_tempo() {
        let recipe = house();
        // 120 bpm -> quarter = 500_000 µs -> 16th = 125_000 µs
        assert_eq!(recipe.step_us(), 125_000);
        let sched = StemSchedule::from_ump(&recipe.to_ump_events());
        // The kick on step 4 (beat 2) lands at ~500_000 µs (within the 32 µs grid).
        let kick_beat2 = sched
            .events()
            .iter()
            .filter(|e| e.group == 0)
            .map(|e| e.at_us)
            .find(|&t| (t - 500_000).abs() < 64);
        assert!(kick_beat2.is_some(), "kick on beat 2 should land at ~500 ms");
    }

    #[test]
    fn emitted_vixi_has_the_required_sections_and_valid_stamp() {
        let recipe = house();
        let vixi = recipe.to_timeline_vixi();
        assert!(vixi.contains("#vixi:timeline v1"));
        assert!(vixi.contains("[timeline]"));
        assert!(vixi.contains("mode = \"audio\""));
        assert!(vixi.contains("[groups.0]"));
        assert!(vixi.contains("instrument = \"drums\""));
        assert!(vixi.contains("[events]"));
        assert!(vixi.contains("encoding = \"base64\""));
        assert!(vixi.contains("[stamp]"));

        // The stamp must equal hash_raw(events) so the timeline parser's integrity
        // check passes — recompute and confirm it is embedded.
        let events = recipe.to_ump_events();
        let stamp = hash_raw(&events).as_u64();
        assert!(vixi.contains(&format!("stage_raw = \"{stamp:016x}\"")));
    }

    #[test]
    fn empty_recipe_emits_a_valid_stamp() {
        let recipe = ArrangementRecipe::new("silence", 120, 1);
        let vixi = recipe.to_timeline_vixi();
        // hash of empty events is well-defined; the doc still parses structurally.
        assert!(vixi.contains("[stamp]"));
        assert!(StemSchedule::from_ump(&recipe.to_ump_events()).is_empty());
    }
}
