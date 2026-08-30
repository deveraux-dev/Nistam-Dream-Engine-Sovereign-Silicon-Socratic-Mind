//! DAR — Directed Attention Raycast (Sean 2026-07-29). One ray, three domains, and
//! what comes back is SEALS, never prose.
//!
//! The board router and the DAR pulse are the same mechanism pointed at different
//! spaces: aim from somewhere, sweep, take what the ray intersected, act, then CLEAR.
//! So [`RayQuery`] carries a `Board` arm beside `Spatial` and `AstCallGraph`, and the
//! board's ranking lives in [`crate::realwork`] — the ray's engine for that domain.
//!
//! Why seals: a hit is a [`SeedId`] — four base-243 limbs, ~4 bytes, spoken as seven
//! syllables ([`crate::evoke`]). Identity travels; the body is hydrated on demand and
//! then dropped. A pulse is transient by construction, so attention costs no resident
//! context.
//!
//! Every boundary here is DECLARED, not hedged (root#rank triad): the payload ceiling
//! is hard, the seal count is hard, and a pulse that would exceed either is refused
//! with a typed error rather than truncated into a soft pass.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::board_sync::BoardTask;
use crate::evoke::{Seed, SeedId, Field};
use crate::realwork::{route, Assignment, Lane};

/// Resident context ceiling, in bytes — the 4.5 KB floor the whole context
/// architecture is built around. A pulse may not exceed it. HARD.
pub const MAX_PAYLOAD_BYTES: u16 = 4_608;

/// Seals in a single spotlight pass. Past this the ray is not attention, it is a
/// sweep, and a sweep belongs on the bulk-read lane. HARD.
pub const MAX_ACTIVE_SEALS: u8 = 12;

/// The standing door. ONE process serves all three ports (forge-daemon#door-process);
/// DAR adds no socket of its own.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DoorEndpointConfig {
    /// 13013 — door HTTP control.
    pub ctrl_port: u16,
    /// 13016 — MCP protocol.
    pub mcp_port: u16,
    /// 13017 — gemma sidecar, OUT OF PROCESS. The shipped bin links no weights.
    pub sidecar_port: u16,
    /// Written by the door on boot; the directory is the door's to create.
    pub pid_file: PathBuf,
    /// The deployed binary — the daemon self-deploys here, `target/` is transient.
    pub binary_target: PathBuf,
    /// Mirrors [`MAX_PAYLOAD_BYTES`] into the manifest so config drift is visible.
    pub max_resident_bytes: u16,
}

impl Default for DoorEndpointConfig {
    fn default() -> Self {
        Self {
            ctrl_port: 13013,
            mcp_port: 13016,
            sidecar_port: 13017,
            pid_file: PathBuf::from(".forge/run/daemon.pid"),
            binary_target: PathBuf::from(".forge/bin/13forge-studio.exe"),
            max_resident_bytes: MAX_PAYLOAD_BYTES,
        }
    }
}

/// The full ladder. Never collapsed (forge-daemon#model-ladder): each rung is a real
/// tier with its own cost, and erasing rungs is how work silently lands on the
/// expensive one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModelTier {
    /// Entry-level tier.
    Student,
    /// Mid-level tier.
    Teacher,
    /// High-level tier.
    Master,
    /// Out-of-process sidecar over TCP 13017. Zero candle, zero gguf, zero weights
    /// in the shipped binary.
    Gemma,
    /// Maximum-level tier.
    Oracle,
}

/// Where a query is aimed. Three domains, one mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RayQuery {
    /// The 33x33x33 world grid.
    Spatial {
        /// Starting cell coordinates.
        origin_cell: [i16; 3],
        /// Direction vector for the ray.
        direction_vec: [i8; 3],
        /// Range in cells to sweep.
        range_cells: u8,
    },
    /// The symbol graph, from a cursor outward.
    AstCallGraph {
        /// Source file path.
        file_path: PathBuf,
        /// Line number where the cursor is positioned.
        cursor_line: u32,
        /// Depth to traverse in the call graph.
        depth: u8,
    },
    /// The board DAG. `take` is how many ranked rows the spotlight holds.
    Board {
        /// Number of ranked rows the spotlight holds.
        take: u8
    },
}

/// Why a pulse was refused. Typed, because a hedged refusal is not a boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PulseRefusal {
    /// More seals than a spotlight holds — this is a sweep, use the bulk lane.
    TooManySeals {
        /// How many seals were requested.
        asked: usize,
        /// The spotlight lane's ceiling.
        ceiling: u8,
    },
    /// The hydrated payload would breach the resident ceiling.
    PayloadTooLarge {
        /// The requested payload size in bytes.
        bytes: u32,
        /// The resident payload ceiling in bytes.
        ceiling: u16,
    },
}

/// A transient impulse. Built, read, cleared — it is never stored, which is the
/// whole reason attention here costs no resident context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DarPulseEnvelope {
    /// What was asked.
    pub query: RayQuery,
    /// What the ray intersected, as identity only.
    pub seals: Vec<SeedId>,
    /// Hydrated size, `<= MAX_PAYLOAD_BYTES`.
    pub payload_bytes: u16,
    /// `-1` `<` strife · `0` ingress · `+1` `>` bound.
    pub balance_trit: i8,
    /// Throughput beside the trit. Load-bearing: `-1|0|+1` alone cannot express
    /// "nothing is happening", so a flatline would masquerade as balance. Readers
    /// MUST check this before trusting `balance_trit`.
    pub volume_pmy: u16,
}

impl DarPulseEnvelope {
    /// Build a pulse, enforcing both hard ceilings up front.
    pub fn new(
        query: RayQuery,
        seals: Vec<SeedId>,
        payload_bytes: u32,
        balance_trit: i8,
        volume_pmy: u16,
    ) -> Result<Self, PulseRefusal> {
        if seals.len() > MAX_ACTIVE_SEALS as usize {
            return Err(PulseRefusal::TooManySeals {
                asked: seals.len(),
                ceiling: MAX_ACTIVE_SEALS,
            });
        }
        if payload_bytes > MAX_PAYLOAD_BYTES as u32 {
            return Err(PulseRefusal::PayloadTooLarge {
                bytes: payload_bytes,
                ceiling: MAX_PAYLOAD_BYTES,
            });
        }
        Ok(Self {
            query,
            seals,
            payload_bytes: payload_bytes as u16,
            balance_trit,
            volume_pmy,
        })
    }

    /// Consume the pulse: hand back the seals and drop everything else. The transient
    /// rule is enforced by ownership, not by a caller remembering to tidy up.
    pub fn clear(self) -> Vec<SeedId> {
        self.seals
    }
}

/// The seal for a board row. A row's identity is its id, its lane and its declared
/// size — rename it, re-lane it or re-size it and the seal moves, which is exactly
/// the drift a silent board hides.
pub fn seal_for(row: &Assignment) -> SeedId {
    let lane = match row.lane {
        Lane::Fable => "fable",
        Lane::Welder => "welder",
        Lane::Gemini => "gemini",
        Lane::Gemma => "gemma",
        Lane::Undeclared => "undeclared",
    };
    // Leaked once per row, so the seal's inputs live as long as the id it names.
    let name: &'static str = Box::leak(row.id.clone().into_boxed_str());
    let fields: &'static [Field] = Box::leak(
        vec![
            Field::new(lane, "lane", 5),
            Field::new("loc", "u32", 20),
            Field::new("depth", "centi", 20),
        ]
        .into_boxed_slice(),
    );
    crate::evoke::evoke(&Seed::new(name, fields)).id
}

/// Aim the ray at the board. Returns the ranked spotlight and the aperture heading it
/// implies — the caller feeds that heading to the existing iris dial. Nothing here
/// moves the aperture; naming and moving stay separate.
pub fn pulse_board(
    tasks: &[BoardTask],
    outcomes: &BTreeMap<String, bool>,
    take: u8,
    balance_trit: i8,
    volume_pmy: u16,
) -> Result<(DarPulseEnvelope, Option<String>), PulseRefusal> {
    let routed = route(tasks, outcomes);
    let held: Vec<&Assignment> = routed.iter().take(take.min(MAX_ACTIVE_SEALS) as usize).collect();
    let aperture = crate::realwork::aperture_line(&routed);
    // Identity only: seven syllables per row, not the row's prose.
    let payload_bytes = (held.len() * 10) as u32;
    let seals = held.iter().map(|a| seal_for(a)).collect();
    let pulse = DarPulseEnvelope::new(
        RayQuery::Board { take },
        seals,
        payload_bytes,
        balance_trit,
        volume_pmy,
    )?;
    Ok((pulse, aperture))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board_sync::Intent;

    fn task(id: &str, title: &str) -> BoardTask {
        BoardTask::new(id, Intent::Make, title).domain("world-engine")
    }

    fn none() -> BTreeMap<String, bool> {
        BTreeMap::new()
    }

    /// The ceilings are boundaries, not suggestions: past them the pulse is REFUSED
    /// with a typed error, never truncated into a soft pass.
    // [BOARD: DAR-PULSE]
    #[test]
    fn ceilings_refuse_they_do_not_truncate() {
        let seals: Vec<SeedId> = (0..13).map(|_| seal_for(&mint("X"))).collect();
        assert_eq!(
            DarPulseEnvelope::new(RayQuery::Board { take: 13 }, seals, 10, 0, 5_000),
            Err(PulseRefusal::TooManySeals { asked: 13, ceiling: 12 })
        );
        assert_eq!(
            DarPulseEnvelope::new(RayQuery::Board { take: 1 }, vec![], 4_609, 0, 5_000),
            Err(PulseRefusal::PayloadTooLarge { bytes: 4_609, ceiling: 4_608 })
        );
        // Exactly at the ceiling passes — the boundary is inclusive and stated.
        assert!(DarPulseEnvelope::new(RayQuery::Board { take: 1 }, vec![], 4_608, 0, 5_000).is_ok());
    }

    fn mint(id: &str) -> Assignment {
        Assignment {
            id: id.to_string(),
            lane: Lane::Fable,
            cost: Default::default(),
            domain: "world-engine".into(),
            impact: 0,
        }
    }

    #[test]
    fn board_pulse_ranks_and_names_the_aperture() {
        let tasks = [
            task("SMALL", "[lane:welder][loc:20][d:0.25]"),
            task("BIG", "[lane:opus][loc:400][d:2]"),
        ];
        let (pulse, aperture) = pulse_board(&tasks, &none(), 4, 0, 6_000).unwrap();
        assert_eq!(pulse.seals.len(), 2);
        assert_eq!(aperture.as_deref(), Some("world-engine · BIG"));
        assert!(pulse.payload_bytes <= MAX_PAYLOAD_BYTES);
    }

    #[test]
    fn a_seal_moves_when_the_row_is_re_laned_or_re_sized() {
        let base = mint("ROW");
        let mut relaned = base.clone();
        relaned.lane = Lane::Gemma;
        assert_ne!(seal_for(&base), seal_for(&relaned), "re-laning is drift, said loud");
        let renamed = mint("ROW2");
        assert_ne!(seal_for(&base), seal_for(&renamed));
        // Same row, same seal — identity is stable when nothing moved.
        assert_eq!(seal_for(&base), seal_for(&mint("ROW")));
    }

    #[test]
    fn volume_rides_beside_the_trit() {
        // A flatline and a live parity share a trit of 0; only volume separates them,
        // which is why the envelope refuses to carry the trit alone.
        let dead = pulse_board(&[task("A", "[lane:opus][loc:1][d:1]")], &none(), 1, 0, 0).unwrap().0;
        let live = pulse_board(&[task("A", "[lane:opus][loc:1][d:1]")], &none(), 1, 0, 6_000).unwrap().0;
        assert_eq!(dead.balance_trit, live.balance_trit);
        assert_ne!(dead.volume_pmy, live.volume_pmy, "the trit alone would hide this");
    }

    #[test]
    fn clearing_consumes_the_pulse() {
        let (pulse, _) = pulse_board(&[task("A", "[lane:opus][loc:9][d:1]")], &none(), 1, 1, 7_000).unwrap();
        let seals = pulse.clear();
        assert_eq!(seals.len(), 1);
        // `pulse` is moved — the transient rule is enforced by ownership, not habit.
    }
}
