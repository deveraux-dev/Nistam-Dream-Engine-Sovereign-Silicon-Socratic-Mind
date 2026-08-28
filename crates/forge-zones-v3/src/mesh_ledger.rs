//! TERRAFORMA MESH P0 — the world is its ledger. Whoever opens the Mesh holds
//! the pen; every later face is a projection of the same append-only intent
//! list. Replaying a prefix IS scrubbing: state is a fold, never a snapshot.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::svg::svg_markup;
use crate::zone_state::{Marker, Volume, ZoneState};

/// One mutation intent. The first intent of a ledger must be `Open` — a Mesh
/// with no opening has no pen-holder and no extent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MeshIntent {
    /// Open the Mesh: name and extent. The genesis row.
    Open {
        /// Zone display name.
        name: String,
        /// Extent along X.
        width: f64,
        /// Extent along Z.
        length: f64,
        /// Lowest legal Y.
        y_min: f64,
        /// Highest legal Y.
        y_max: f64,
        /// Authoring origin/atlas id.
        origin: String,
    },
    /// Place one volume.
    PlaceVolume(Box<Volume>),
    /// Place one logic marker.
    PlaceMarker(Box<Marker>),
    /// Erase every volume and marker carrying this name.
    Erase {
        /// The name to erase.
        name: String,
    },
}

/// An append-only intent list. Flat JSON-lines: one intent per line, parsed by
/// exact deserialization — never by pattern matching over the text.
#[derive(Debug, Clone, Default)]
pub struct MeshLedger {
    intents: Vec<MeshIntent>,
}

/// What a replay produced, and how far down the ledger it read.
#[derive(Debug, Clone)]
pub struct Replay {
    /// The world as of the replayed prefix.
    pub zone: ZoneState,
    /// How many intents were folded in.
    pub depth: usize,
    /// MeshIntents the fold refused (out of order, or erasing nothing).
    pub refused: Vec<usize>,
}

impl MeshLedger {
    /// An unopened Mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one intent. The ledger never rewrites, only grows.
    pub fn append(&mut self, intent: MeshIntent) {
        self.intents.push(intent);
    }

    /// How many intents stand.
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// True when nothing has been written yet.
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// Every intent, in the order they were written.
    pub fn intents(&self) -> &[MeshIntent] {
        &self.intents
    }

    /// Fold the whole ledger into a world.
    pub fn replay(&self) -> Replay {
        self.replay_to(self.intents.len())
    }

    /// Fold only the first `depth` intents — the scrub primitive. A depth past
    /// the end reads the whole ledger rather than erroring: there is no state
    /// beyond the last intent to be wrong about.
    pub fn replay_to(&self, depth: usize) -> Replay {
        let depth = depth.min(self.intents.len());
        let mut zone = ZoneState::new("", 0.0, 0.0, 0.0, 0.0, "");
        let mut refused = Vec::new();

        for (i, intent) in self.intents[..depth].iter().enumerate() {
            match intent {
                MeshIntent::Open { name, width, length, y_min, y_max, origin } => {
                    if i == 0 {
                        zone = ZoneState::new(
                            name.clone(),
                            *width,
                            *length,
                            *y_min,
                            *y_max,
                            origin.clone(),
                        );
                    } else {
                        refused.push(i);
                    }
                }
                MeshIntent::PlaceVolume(v) => {
                    if zone.name.is_empty() {
                        refused.push(i);
                    } else if zone.add_volume((**v).clone()).is_err() {
                        refused.push(i);
                    }
                }
                MeshIntent::PlaceMarker(m) => {
                    if zone.name.is_empty() {
                        refused.push(i);
                    } else {
                        zone.add_marker((**m).clone());
                    }
                }
                MeshIntent::Erase { name } => {
                    let before = zone.volumes.len() + zone.markers.len();
                    zone.volumes.retain(|v| &v.name != name);
                    zone.markers.retain(|m| &m.name != name);
                    if zone.volumes.len() + zone.markers.len() == before {
                        refused.push(i);
                    }
                }
            }
        }

        Replay { zone, depth, refused }
    }

    /// The world one intent back — the scrub the P0 receipt walks.
    pub fn scrubbed_back(&self, steps: usize) -> Replay {
        self.replay_to(self.intents.len().saturating_sub(steps))
    }

    /// Serialize to JSON-lines, one intent per line, trailing newline.
    pub fn to_jsonl(&self) -> Result<String, String> {
        let mut out = String::new();
        for intent in &self.intents {
            let line = serde_json::to_string(intent).map_err(|e| e.to_string())?;
            out.push_str(&line);
            out.push('\n');
        }
        Ok(out)
    }

    /// Parse JSON-lines back. Blank lines are skipped; a malformed line names
    /// its own line number rather than being silently dropped.
    pub fn from_jsonl(text: &str) -> Result<Self, String> {
        let mut ledger = Self::new();
        for (n, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let intent: MeshIntent = serde_json::from_str(line)
                .map_err(|e| format!("mesh ledger line {}: {e}", n + 1))?;
            ledger.append(intent);
        }
        Ok(ledger)
    }

    /// Append one intent to the ledger file on disk, creating it if absent.
    /// The file is only ever opened for append — the pen does not go back.
    pub fn append_to_file(path: &Path, intent: &MeshIntent) -> Result<(), String> {
        let line = serde_json::to_string(intent).map_err(|e| e.to_string())?;
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("mesh ledger open {}: {e}", path.display()))?;
        writeln!(f, "{line}").map_err(|e| format!("mesh ledger write: {e}"))
    }

    /// Read a ledger file back. A missing file is an empty Mesh, not an error.
    pub fn load_file(path: &Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_jsonl(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(format!("mesh ledger read {}: {e}", path.display())),
        }
    }
}

/// Render a replayed world as a standalone HTML page wrapping the landed SVG
/// lane. The depth is printed on the face so a scrub is visible, not inferred.
pub fn render_html(replay: &Replay) -> Result<String, String> {
    let svg = svg_markup(&replay.zone)?;
    Ok(format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
         <title>{name} — mesh depth {depth}</title></head>\
         <body style=\"margin:0;background:#101014;color:#d8d4e0;\
         font:13px ui-monospace,monospace\">\
         <p style=\"padding:6px 10px;margin:0\">{name} · intents folded: {depth} \
         · refused: {refused}</p>\n{svg}\n</body></html>\n",
        name = replay.zone.name,
        depth = replay.depth,
        refused = replay.refused.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zone_state::Shape;

    fn opening() -> MeshIntent {
        MeshIntent::Open {
            name: "Toll Gate".into(),
            width: 64.0,
            length: 64.0,
            y_min: 0.0,
            y_max: 16.0,
            origin: "mesh-p0".into(),
        }
    }

    fn volume(name: &str, x: f64) -> MeshIntent {
        MeshIntent::PlaceVolume(Box::new(Volume::new(name, Shape::Box, x, 1.0, 0.0)))
    }

    #[test]
    fn an_unopened_mesh_holds_nothing() {
        let ledger = MeshLedger::new();
        assert!(ledger.is_empty());
        assert_eq!(ledger.replay().zone.volumes.len(), 0);
    }

    /// Whoever opens the Mesh holds the pen: a place before an open is refused,
    /// and a second open is refused too — one opening per world.
    #[test]
    fn only_the_first_intent_may_open_the_mesh() {
        let mut early = MeshLedger::new();
        early.append(volume("wall", 0.0));
        assert_eq!(early.replay().refused, vec![0], "no pen, no placement");

        let mut twice = MeshLedger::new();
        twice.append(opening());
        twice.append(opening());
        assert_eq!(twice.replay().refused, vec![1], "the Mesh opens once");
    }

    #[test]
    fn state_is_the_fold_of_the_ledger() {
        let mut ledger = MeshLedger::new();
        ledger.append(opening());
        ledger.append(volume("wall", 0.0));
        ledger.append(volume("pillar", 8.0));

        let replay = ledger.replay();
        assert_eq!(replay.zone.name, "Toll Gate");
        assert_eq!(replay.zone.volumes.len(), 2);
        assert_eq!(replay.depth, 3);
        assert!(replay.refused.is_empty());
    }

    /// The P0 receipt itself: append one intent, re-render, scrub back one
    /// step, and land exactly where you were before the append.
    #[test]
    fn a_scrub_back_one_step_undoes_exactly_one_intent() {
        let mut ledger = MeshLedger::new();
        ledger.append(opening());
        ledger.append(volume("wall", 0.0));
        let before = ledger.replay().zone.volumes.len();

        ledger.append(volume("pillar", 8.0));
        assert_eq!(ledger.replay().zone.volumes.len(), before + 1, "the append lands");

        let scrubbed = ledger.scrubbed_back(1);
        assert_eq!(scrubbed.zone.volumes.len(), before, "the scrub undoes exactly one");
        assert_eq!(scrubbed.depth, 2);
        assert_eq!(ledger.len(), 3, "scrubbing reads the ledger, it never truncates it");
    }

    #[test]
    fn erasing_removes_by_name_and_refuses_when_it_finds_nothing() {
        let mut ledger = MeshLedger::new();
        ledger.append(opening());
        ledger.append(volume("wall", 0.0));
        ledger.append(MeshIntent::Erase { name: "wall".into() });
        let replay = ledger.replay();
        assert!(replay.zone.volumes.is_empty());
        assert!(replay.refused.is_empty());

        ledger.append(MeshIntent::Erase { name: "wall".into() });
        assert_eq!(ledger.replay().refused, vec![3], "erasing nothing is refused, not silent");
    }

    /// Round-trip through the on-the-wire form, byte for byte.
    #[test]
    fn the_ledger_survives_a_jsonl_round_trip() {
        let mut ledger = MeshLedger::new();
        ledger.append(opening());
        ledger.append(volume("wall", 0.0));
        ledger.append(MeshIntent::Erase { name: "wall".into() });

        let text = ledger.to_jsonl().expect("serializes");
        assert_eq!(text.lines().count(), 3, "one line per intent");
        let back = MeshLedger::from_jsonl(&text).expect("parses");
        assert_eq!(back.len(), ledger.len());
        assert_eq!(back.to_jsonl().unwrap(), text, "re-serializing is byte-identical");
        assert_eq!(back.replay().zone.volumes.len(), ledger.replay().zone.volumes.len());
    }

    #[test]
    fn a_malformed_line_names_its_line_number() {
        let err = MeshLedger::from_jsonl("{\"Erase\":{\"name\":\"a\"}}\nnot json\n")
            .expect_err("must refuse");
        assert!(err.contains("line 2"), "the error must point at the bad line: {err}");
    }

    /// The full P0 loop through a real file on disk: open, place, re-read,
    /// append one more intent from a second "host", re-read again, scrub.
    /// The pen only ever appends — the file grows, never rewrites.
    #[test]
    fn the_ledger_round_trips_through_a_real_file() {
        let path = std::env::temp_dir().join(format!(
            "forge-mesh-p0-{}-{}.jsonl",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_file(&path);

        MeshLedger::append_to_file(&path, &opening()).expect("opens");
        MeshLedger::append_to_file(&path, &volume("wall", 0.0)).expect("places");

        let first = MeshLedger::load_file(&path).expect("reads back");
        assert_eq!(first.len(), 2);
        assert_eq!(first.replay().zone.volumes.len(), 1);

        // A second writer appends without ever seeing the first's memory.
        MeshLedger::append_to_file(&path, &volume("pillar", 8.0)).expect("places again");

        let second = MeshLedger::load_file(&path).expect("reads back");
        assert_eq!(second.len(), 3, "the file grew by exactly one line");
        assert_eq!(second.replay().zone.volumes.len(), 2);
        assert_eq!(
            second.scrubbed_back(1).zone.volumes.len(),
            1,
            "scrubbing the file-backed ledger lands on the first writer's world"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn a_missing_ledger_file_is_an_empty_mesh_not_an_error() {
        let path = std::env::temp_dir().join("forge-mesh-p0-definitely-absent.jsonl");
        let _ = std::fs::remove_file(&path);
        assert!(MeshLedger::load_file(&path).expect("absent is fine").is_empty());
    }

    #[test]
    fn the_face_renders_the_replayed_world_and_says_how_deep_it_read() {
        let mut ledger = MeshLedger::new();
        ledger.append(opening());
        ledger.append(volume("wall", 0.0));

        let html = render_html(&ledger.replay()).expect("renders");
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("Toll Gate"));
        assert!(html.contains("intents folded: 2"), "the depth must be on the face");
        assert!(html.contains("<svg"), "the face must carry the picture, not a status line");
        assert!(html.contains("<title>wall</title>"), "the placed cell must be drawn");

        let scrubbed = render_html(&ledger.scrubbed_back(1)).expect("renders");
        assert!(scrubbed.contains("intents folded: 1"), "a scrub must be visible on the face");
    }
}
