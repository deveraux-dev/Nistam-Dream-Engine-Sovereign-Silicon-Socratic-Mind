//! The streaming degeneration tripwire (MIGRATION §COCKPIT, conductor-router).
//!
//! Measured ground, 2026-08-10: the raw INFER lane collapsed into a greedy
//! token rut on a 5.5 KB brief — ~4,096 tokens of digit repetition
//! (`for32323233…333…`), three byte-identical attempts, each discovered only
//! after the full 198-second generation had been paid for and the FILE-contract
//! red-carded it. The wire is one frame per reply, so the earliest the foreman
//! can see the rut is on receipt — this module makes that receipt-time check
//! typed, integer-only, and loud.
//!
//! Doctrine: detect → journal → red — **never mask, never reroute silently**
//! (ADR-0001 signal law; the conductor ruling). The tripwire refuses a
//! degenerate reply as a typed error so the run loop's existing evidence /
//! retry / queue ladder carries it; it decides nothing on its own.
//!
//! Detection is trailing periodicity: a greedy rut ends the reply with a short
//! unit repeated verbatim (period 1 for `3333…`, period 2 for `3232…`, longer
//! for phrase loops). Real code never ends with hundreds of bytes of exact
//! short-period repetition — a run of closing braces is an order of magnitude
//! below the span floor. Integer byte compares only; no floats, no alloc.

/// A detected degeneration: the repeating unit's byte length and how many
/// bytes of the reply's tail it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Degeneracy {
    /// Byte length of the repeating unit (1 = single-character flood).
    pub period: usize,
    /// Total bytes of the tail covered by exact repeats of the unit.
    pub span: usize,
}

/// Longest repeating unit the tripwire looks for. Ruts measured so far are
/// period 1–8; 64 leaves headroom for phrase-loops without scanning prose.
const MAX_PERIOD: usize = 64;

/// A tail must carry at least this many bytes of pure repetition to trip.
/// The measured healthy ceiling (closing-brace runs, padded tables) is tens of
/// bytes; the measured rut is thousands. 240 sits an order of magnitude from
/// both shores.
const MIN_SPAN: usize = 240;

/// And at least this many whole repeats — so a long unit repeated twice
/// (a legitimately duplicated code block at EOF) is not a rut.
const MIN_REPEATS: usize = 4;

/// Examines a reply's tail for short-period exact repetition. Returns the
/// smallest period that trips, so a digit flood reports period 1 even though
/// larger periods also divide it. Trailing whitespace is ignored — ruts are
/// content, padding is not.
pub fn degeneracy(reply: &str) -> Option<Degeneracy> {
    let b = reply.trim_end().as_bytes();
    let n = b.len();
    for period in 1..=MAX_PERIOD {
        if n < period * MIN_REPEATS {
            break;
        }
        let unit = &b[n - period..];
        let mut span = period;
        while span + period <= n && &b[n - span - period..n - span] == unit {
            span += period;
        }
        if span >= MIN_SPAN && span / period >= MIN_REPEATS {
            return Some(Degeneracy { period, span });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The measured 08-10 failure shape: a digit flood trips at period 1.
    #[test]
    fn a_digit_flood_trips_at_period_one() {
        let reply = format!("fn lowered() {{ for{}", "3".repeat(4000));
        let d = degeneracy(&reply).expect("the measured rut must trip");
        assert_eq!(d.period, 1);
        assert!(d.span >= 4000);
    }

    /// The alternating variant (`3232…`) trips at its true period.
    #[test]
    fn an_alternating_rut_trips_at_period_two() {
        let reply = format!("// FILE: src/lib.rs\nfor{}", "32".repeat(2000));
        let d = degeneracy(&reply).expect("alternation is still a rut");
        assert_eq!(d.period, 2);
    }

    /// A phrase-loop (whole clause repeated) trips on the longer unit.
    #[test]
    fn a_phrase_loop_trips_within_the_period_ceiling() {
        let reply = format!("start {}", "return value(); ".repeat(40));
        let d = degeneracy(&reply).expect("phrase loops are ruts too");
        assert_eq!(d.period, "return value(); ".len());
    }

    /// Healthy code — including a run of closing braces and trailing
    /// newlines — never trips: the span floor is an order of magnitude above
    /// anything real files end with.
    #[test]
    fn healthy_code_with_brace_runs_never_trips() {
        let reply = "//! Doc.\npub fn a() -> u8 { 1 }\nmod b { mod c { mod d { fn e() {} } } }\n}\n}\n}\n}\n}\n\n\n";
        assert_eq!(degeneracy(reply), None);
        assert_eq!(degeneracy(""), None);
        assert_eq!(degeneracy("short"), None);
    }

    /// A long varied unit repeated only a few times (legitimate duplication,
    /// e.g. a copied block at EOF) is not a rut — three repeats of a 50-byte
    /// unit sit under both the repeat floor and the span floor.
    #[test]
    fn few_repeats_of_a_long_block_do_not_trip() {
        let block = "let alpha = beta + gamma; // varied unit 0123456789\n";
        let reply = format!("{block}{block}{block}");
        assert_eq!(degeneracy(&reply), None);
    }
}
