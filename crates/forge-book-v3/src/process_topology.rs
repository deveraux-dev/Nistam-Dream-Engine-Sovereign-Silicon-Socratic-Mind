//! Process-topology doctrine (Sean 2026-07-20): verbs are free, resident
//! processes are bounded hard. The ceiling, the per-capability test, and the
//! "too many" tripwires — booked so the rule can't be lost.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;

/// One doctrine line: (kind, rule).
pub const PROCESS_DOCTRINE: &[(&str, &str)] = &[
    ("VERB", "in-process fn in an exe that already exists — zero marginal process — UNBOUNDED; fold skills here by default"),
    ("RESIDENT", "daemon/sidecar/held-socket/popup — each costs a port, resident RAM, a lifecycle, a failure mode, a boot race — BOUNDED HARD"),
    ("CEILING/brain", "one standing brain-daemon (the :13013 control + :13016 MCP wave)"),
    ("CEILING/door", "N thin doors, ephemeral — one per live session, die on stdio disconnect — cost ~0"),
    ("CEILING/sidecar", "0-2, only for own address-space/device/security-boundary the brain can't hold (GPU capture, audio); single-purpose, socket-attached, auto-dying"),
    ("CEILING/popup", "zero standing (aperture law: standing-wall=0)"),
    ("TEST", "needs its own device/boundary/lifecycle? no -> verb (the default) · yes -> sidecar, bounded"),
    ("TOO-MANY", ">1 standing brain | any sidecar that could have been a verb | any process that outlives its wave"),
];

/// Bind the process-topology doctrine into a Doctrine chapter.
pub fn process_topology_chapter() -> Chapter {
    let mut ch = Chapter::new(
        "Process Topology — Verbs Free, Residents Bounded",
        AtlasSection::Custom("Doctrine".into()),
    );
    ch.add_lore(
        "count RESIDENT processes, never verbs. a compiled verb is a function in an exe that \
         already exists; the scarce resource is standing processes (port + RAM + lifecycle + race).",
    );
    for &(kind, rule) in PROCESS_DOCTRINE {
        ch.add_lore(format!("{kind}: {rule}"));
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctrine_binds_eight_rules() {
        assert_eq!(PROCESS_DOCTRINE.len(), 8);
        let ch = process_topology_chapter();
        assert_eq!(ch.section, AtlasSection::Custom("Doctrine".into()));
        assert_eq!(ch.lore_count(), 9); // header + 8
    }
}
