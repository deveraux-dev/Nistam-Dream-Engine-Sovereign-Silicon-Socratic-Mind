//! Dialogue tape — the play-once, mutate-forever dialogue artifact (RON rows).
//! Gemma proposes rows (Speculated, seed-carried), the human locks them; replay
//! is deterministic from `world_seed` + each row's node domain + carried seed.

/// Who produced a dialogue row and whether it is settled. A speculated row is
/// mutable (regenerable from its seed); locking RETAINS the seed so provenance
/// and replay both survive the promotion.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineProvenance {
    /// Hand-typed by the author/player. No generation seed exists.
    Authored,
    /// Machine-proposed, unsettled. `seed` is the generation stream key
    /// (the serving lane's prompt-hash discipline: same seed, same line).
    Speculated {
        /// Deterministic generation seed the line was drawn under.
        seed: u64,
    },
    /// Machine-proposed, then accepted by the human. Seed retained.
    LockedSpeculation {
        /// The seed the accepted line was drawn under.
        seed: u64,
    },
}

/// One line on the tape. Several rows may share a `node` — they are that
/// node's candidate pool, and a weighted draw (forge-core-v3
/// `weighted_reservoir`, offered `weight` per row) picks the spoken one.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DialogueRow {
    /// Dialogue-graph node id, e.g. `bell_warden:greet`. Never empty.
    pub node: String,
    /// Speaking voice, e.g. `BELL-WARDEN`, `PLAYER`. Never empty.
    pub speaker: String,
    /// The line itself.
    pub text: String,
    /// Who wrote it and whether it is settled.
    pub provenance: LineProvenance,
    /// Node spoken next. Empty string = the conversation ends here.
    #[serde(default)]
    pub next_node: String,
    /// Candidate weight for the node's pick (0 = never spoken). Default 1.
    #[serde(default = "default_weight")]
    pub weight: u64,
    /// Integer stat consequences of speaking this row, `(stat, delta)` —
    /// dirge `DialogueChoice::stat_shift`, Vec-not-map for deterministic order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stat_shift: Vec<(String, i32)>,
}

fn default_weight() -> u64 {
    1
}

/// The tape: one playthrough's dialogue document. RON on disk, hand-editable,
/// appended live, replayable from `world_seed`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DialogueTape {
    /// Schema word, always [`TAPE_SCHEMA`].
    pub schema: String,
    /// World RNG seed. The per-node pick stream is
    /// `Mulberry32::new(world_seed).fork(&fork_domain(node))` — the contract
    /// [`fork_domain`] pins; this crate carries the string, not the RNG.
    pub world_seed: u64,
    /// The rows, in play order. Append-only during a run; mutable after.
    pub rows: Vec<DialogueRow>,
}

/// Schema word for the dialogue-tape format.
pub const TAPE_SCHEMA: &str = "TAPE1";

/// The ONE canonical fork-domain string for a node's pick stream. Both the
/// writer (serve lane) and the replayer derive it from here, never inline.
pub fn fork_domain(node: &str) -> String {
    format!("node:{node}")
}

impl DialogueTape {
    /// A new empty tape for `world_seed`.
    pub fn new(world_seed: u64) -> Self {
        Self { schema: TAPE_SCHEMA.into(), world_seed, rows: Vec::new() }
    }

    /// Indices + rows forming `node`'s candidate pool, in tape order — the
    /// offer stream for the weighted pick.
    pub fn candidates<'a>(&'a self, node: &'a str) -> impl Iterator<Item = (usize, &'a DialogueRow)> {
        self.rows.iter().enumerate().filter(move |(_, r)| r.node == node)
    }

    /// Promote a speculated row to locked (human acceptance). Returns `true`
    /// iff the row existed and was `Speculated` — locking is the ONLY
    /// provenance transition; everything else is a text edit, not a promotion.
    pub fn lock(&mut self, row_idx: usize) -> bool {
        match self.rows.get_mut(row_idx) {
            Some(row) => match row.provenance {
                LineProvenance::Speculated { seed } => {
                    row.provenance = LineProvenance::LockedSpeculation { seed };
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Validate the tape: schema word, no empty node/speaker, every non-empty
    /// `next_node` resolves to some row's node. Worded errors, all-or-nothing.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != TAPE_SCHEMA {
            return Err(format!("tape schema is '{}', expected '{TAPE_SCHEMA}'", self.schema));
        }
        for (i, r) in self.rows.iter().enumerate() {
            if r.node.is_empty() {
                return Err(format!("row {i} has an empty node id"));
            }
            if r.speaker.is_empty() {
                return Err(format!("row {i} ('{}') has an empty speaker", r.node));
            }
        }
        for (i, r) in self.rows.iter().enumerate() {
            if !r.next_node.is_empty() && !self.rows.iter().any(|x| x.node == r.next_node) {
                return Err(format!(
                    "row {i} ('{}') points at next_node '{}' which no row provides",
                    r.node, r.next_node
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DialogueTape {
        let mut t = DialogueTape::new(0xB0A7);
        t.rows.push(DialogueRow {
            node: "bell_warden:greet".into(),
            speaker: "BELL-WARDEN".into(),
            text: "Few come to the pit at this hour. State your trade.".into(),
            provenance: LineProvenance::Authored,
            next_node: "bell_warden:reply".into(),
            weight: 1,
            stat_shift: Vec::new(),
        });
        t.rows.push(DialogueRow {
            node: "bell_warden:reply".into(),
            speaker: "BELL-WARDEN".into(),
            text: "The orchard keeps its own hours.".into(),
            provenance: LineProvenance::Speculated { seed: 0xC0FFEE },
            next_node: String::new(),
            weight: 3,
            stat_shift: vec![("lore".into(), 25)],
        });
        t.rows.push(DialogueRow {
            node: "bell_warden:reply".into(),
            speaker: "BELL-WARDEN".into(),
            text: "Trade first. Questions after.".into(),
            provenance: LineProvenance::Speculated { seed: 0xC0FFEF },
            next_node: String::new(),
            weight: 1,
            stat_shift: Vec::new(),
        });
        t
    }

    /// L07 bijection: struct → RON text → struct, identical, byte-stable.
    #[test]
    fn ron_round_trip_is_identity() {
        let tape = sample();
        let text = ron::to_string(&tape).expect("serialize");
        let back: DialogueTape = ron::from_str(&text).expect("parse own output");
        assert_eq!(tape, back);
        let text2 = ron::to_string(&back).expect("re-serialize");
        assert_eq!(text, text2, "second trip must be byte-stable");
    }

    /// The live-document claim made falsifiable: a HAND-WRITTEN RON string
    /// (no serializer involved, defaults omitted) parses into the same shape.
    #[test]
    fn hand_edited_ron_parses() {
        let src = r#"(
            schema: "TAPE1",
            world_seed: 7,
            rows: [
                (
                    node: "gate:open",
                    speaker: "PLAYER",
                    text: "I'd like to trade.",
                    provenance: Authored,
                ),
                (
                    node: "gate:open",
                    speaker: "WARDEN",
                    text: "Then show your coin.",
                    provenance: Speculated(seed: 99),
                    weight: 4,
                ),
            ],
        )"#;
        let tape: DialogueTape = ron::from_str(src).expect("hand-authored RON parses");
        assert_eq!(tape.rows.len(), 2);
        assert_eq!(tape.rows[0].weight, 1, "omitted weight defaults to 1");
        assert_eq!(tape.rows[0].next_node, "", "omitted next_node defaults to end");
        assert_eq!(tape.rows[1].provenance, LineProvenance::Speculated { seed: 99 });
        tape.validate().expect("hand-authored tape validates");
    }

    /// Lock promotes Speculated → LockedSpeculation and retains the seed;
    /// Authored rows and out-of-range indices refuse.
    #[test]
    fn lock_is_the_only_promotion() {
        let mut tape = sample();
        assert!(!tape.lock(0), "Authored must not re-lock");
        assert!(tape.lock(1), "Speculated locks");
        assert_eq!(tape.rows[1].provenance, LineProvenance::LockedSpeculation { seed: 0xC0FFEE });
        assert!(!tape.lock(1), "already locked must not lock twice");
        assert!(!tape.lock(99), "out of range refuses");
    }

    /// Candidate pool: exactly the node's rows, in tape order, with weights
    /// ready to feed `WeightedReservoir::offer`.
    #[test]
    fn candidates_are_the_offer_stream() {
        let tape = sample();
        let pool: Vec<(usize, u64)> =
            tape.candidates("bell_warden:reply").map(|(i, r)| (i, r.weight)).collect();
        assert_eq!(pool, vec![(1, 3), (2, 1)]);
    }

    /// L18 sabotage: a dangling next_node, an empty speaker, and a wrong
    /// schema word each refuse with a worded error.
    #[test]
    fn validate_refuses_broken_tapes() {
        let mut dangling = sample();
        dangling.rows[0].next_node = "nowhere:at_all".into();
        assert!(dangling.validate().unwrap_err().contains("nowhere:at_all"));

        let mut mute = sample();
        mute.rows[1].speaker = String::new();
        assert!(mute.validate().unwrap_err().contains("empty speaker"));

        let mut wrong = sample();
        wrong.schema = "TAPE9".into();
        assert!(wrong.validate().unwrap_err().contains("TAPE9"));
    }

    /// The fork-domain contract is pinned: replayer and writer must agree.
    #[test]
    fn fork_domain_is_canonical() {
        assert_eq!(fork_domain("bell_warden:greet"), "node:bell_warden:greet");
    }
}
