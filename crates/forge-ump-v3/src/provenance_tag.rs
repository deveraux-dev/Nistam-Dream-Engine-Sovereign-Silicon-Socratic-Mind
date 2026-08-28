//! Provenance tagging — bind a [`SourceKind`] to a UMP run and SEAL it into the
//! canonical stamp-chain hash (scc), so a run's ORIGIN cannot be stripped or
//! forged without changing its content identity.
//!
//! Also binds the epoch `moon` (via [`seal_with_moon`]) — completing the
//! `(tick_id, moon, code_hash)` spine coordinate: tick_id rides the Stamped tick,
//! moon folds in here, code_hash IS the scc BrutalHash.
//!
//! Why this lives in forge-ump: the crate already owns the scc seal
//! ([`crate::stamp_chain::hash_canonical`]) AND sees [`forge_core_v3::spine::SourceKind`].
//! The tag is therefore born on the UMP spine ("tie it to ump"); the
//! `forge_semantic::AuthorityLedger` is the downstream ENFORCER, not the origin.
//!
//! NDE-ladder rule (Signal Law: an untagged climb is a SILENT fault — make it loud):
//!   - [`Tier::Local`]        (NDE / Gemma student-draft)  -> [`SourceKind::LLMCandidate`]
//!   - [`Tier::Cloud`]        (Opus/Sonnet Master pass)    -> [`SourceKind::LLMCandidate`] (still a HEDGE)
//!   - [`Tier::HumanVerified`](Sean HITL verdict)          -> [`SourceKind::HumanAuthored`]
//!     (the only kind the spine may ever mark Permanent — Authority Rule 2)

use forge_core_v3::spine::{BrutalHash, SourceKind};

use crate::packet::{Stamped, Ump};
use crate::stamp_chain::hash_canonical;

/// Low-24-bit marker in the tag atom's `word0` — distinguishes a provenance atom
/// from a real MIDI 2.0 message. (`0x5F_47` == `_G`, the tail of "taG".)
const TAG_MAGIC: u32 = 0x00_5F_47;

/// One rung of the NDE ladder that carries a distinct provenance authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// Local zero-cost tier (NDE MoE router / Gemma wall). Breadth, never authority.
    Local,
    /// Cloud Master synthesis (Sonnet T1 / Opus T2). Higher stakes, still machine output.
    Cloud,
    /// Sean's human-in-the-loop verdict. The ONLY tier that may author truth.
    HumanVerified,
}

/// The provenance tag every run at `tier` MUST carry. Encoded ONCE, here — every
/// {source, sink, tier} preset references this binding instead of re-declaring it.
pub const fn required_source_kind(tier: Tier) -> SourceKind {
    match tier {
        Tier::Local => SourceKind::LLMCandidate,
        Tier::Cloud => SourceKind::LLMCandidate,
        Tier::HumanVerified => SourceKind::HumanAuthored,
    }
}

/// The leading provenance atom binding `kind` (origin) AND `moon` (the epoch/context
/// leg of the `(tick_id, moon, code_hash)` spine coordinate) as ONE UMP event.
/// word0 = kind + TAG_MAGIC; word1 = moon. `moon == 0` reproduces the byte-exact
/// tier-only atom, so the moon-less seals stay value-identical (back-compat).
fn tag_atom_with_moon(kind: SourceKind, moon: u8) -> Stamped<Ump> {
    Stamped {
        universal_tick_us: i64::MIN, // sentinel tick: precedes any real event
        payload: Ump::new([((kind.as_u8() as u32) << 24) | TAG_MAGIC, moon as u32, 0, 0]),
    }
}

/// Seal a run for `tier`: resolve the required [`SourceKind`] then fold it into scc.
pub fn seal_with_tier(tier: Tier, events: &[Stamped<Ump>], jr_quantize_us: i64) -> BrutalHash {
    seal_with_kind(required_source_kind(tier), events, jr_quantize_us)
}

/// Seal a run for `tier` AND bind the epoch `moon` — completes the `(tick_id, moon,
/// code_hash)` spine coordinate: `tick_id` rides `Stamped::universal_tick_us`, `moon`
/// folds in here, `code_hash` IS the returned `BrutalHash`. `moon == 0` ⇒ identical
/// to [`seal_with_tier`].
pub fn seal_with_moon(
    moon: u8,
    tier: Tier,
    events: &[Stamped<Ump>],
    jr_quantize_us: i64,
) -> BrutalHash {
    seal_with_kind_moon(required_source_kind(tier), moon, events, jr_quantize_us)
}

/// Fold `kind` INTO the canonical hash so origin is inseparable from content
/// identity (scc). Stripping or swapping the tag changes the returned hash.
pub fn seal_with_kind(
    kind: SourceKind,
    events: &[Stamped<Ump>],
    jr_quantize_us: i64,
) -> BrutalHash {
    seal_with_kind_moon(kind, 0, events, jr_quantize_us)
}

/// Fold BOTH `kind` (origin) and `moon` (epoch/context) into the canonical hash so
/// neither can be stripped or swapped without changing content identity (scc).
pub fn seal_with_kind_moon(
    kind: SourceKind,
    moon: u8,
    events: &[Stamped<Ump>],
    jr_quantize_us: i64,
) -> BrutalHash {
    // Fold the provenance atom in FRONT of the payload, then hash the whole
    // stream: the tag rides the SAME canonical seal as the events, so it cannot
    // be stripped or swapped without changing the BrutalHash.
    // @forge:allow_alloc -- cold path; sealing runs once per ledger commit.
    let mut tagged: Vec<Stamped<Ump>> = Vec::with_capacity(events.len() + 1);
    tagged.push(tag_atom_with_moon(kind, moon));
    tagged.extend_from_slice(events);
    hash_canonical(&tagged, jr_quantize_us)
}

/// The scc seal for `tier` as a raw `u64` — the form carried on the wire
/// (length-prefixed MsgFrame, Inv#6). `BrutalHash` is blake3-truncated-64, so the
/// conversion is loss-free. `.as_u64()` is called on the returned hash WITHOUT
/// naming forge-core downstream — consumers stay firewall-clean.
pub fn seal_u64_for_tier(tier: Tier, events: &[Stamped<Ump>], jr_quantize_us: i64) -> u64 {
    seal_with_tier(tier, events, jr_quantize_us).as_u64()
}

/// Ingress integrity gate (ADR-0026 D6): recompute the run's scc seal for `tier`
/// and compare to the `claimed_seal` carried on the wire. `false` ⇒ the events
/// were tampered in transit OR the declared tier/kind disagrees with what was
/// sealed — the caller MUST refuse the commit.
///
/// Proves INTEGRITY + tier/kind CONSISTENCY, NOT authorization: *who* may claim a
/// tier is the Ed25519 signature envelope (ADR-0027), a separate layer.
pub fn verify_seal_at_ingress(
    tier: Tier,
    events: &[Stamped<Ump>],
    jr_quantize_us: i64,
    claimed_seal: u64,
) -> bool {
    // Recompute the scc seal over the RECEIVED events and compare to the wire
    // claim. Any tamper (events differ) or tier/kind swap (the folded provenance
    // atom differs) changes the hash, so equality is integrity + consistency.
    seal_u64_for_tier(tier, events, jr_quantize_us) == claimed_seal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Stamped<Ump>> {
        vec![
            Stamped { universal_tick_us: 100, payload: Ump::new([0x4090_4000, 0x7fff_0000, 0, 0]) },
            Stamped { universal_tick_us: 200, payload: Ump::new([0x4091_4000, 0x4000_0000, 0, 0]) },
        ]
    }

    // ── Tier-binding rung: pins the NDE-ladder rule (green regardless of seal) ──

    #[test]
    fn local_tier_is_llm_candidate_never_authored() {
        assert_eq!(required_source_kind(Tier::Local), SourceKind::LLMCandidate);
        assert_ne!(required_source_kind(Tier::Local), SourceKind::HumanAuthored);
    }

    #[test]
    fn cloud_tier_is_still_a_hedge_not_authored() {
        // Cloud Master output is higher-stakes but STILL machine-authored: a HEDGE.
        assert_eq!(required_source_kind(Tier::Cloud), SourceKind::LLMCandidate);
        assert_ne!(required_source_kind(Tier::Cloud), SourceKind::HumanAuthored);
    }

    #[test]
    fn only_human_verified_may_author() {
        assert_eq!(required_source_kind(Tier::HumanVerified), SourceKind::HumanAuthored);
    }

    // ── Seal rung: the SILENT-otherwise catchers (RED under the no-op stub) ──

    #[test]
    fn tag_is_sealed_into_scc_not_silent() {
        // If tagging is a silent no-op, the tagged seal == the untagged hash.
        // Signal Law: that silence MUST be a loud failure.
        let ev = sample();
        let untagged = hash_canonical(&ev, 10);
        let tagged = seal_with_tier(Tier::Local, &ev, 10);
        assert_ne!(tagged, untagged, "provenance tag did not change scc — tag is silent");
    }

    #[test]
    fn distinct_tiers_yield_distinct_seals() {
        // A forged/swapped tier must break the seal (tamper-evident scc).
        let ev = sample();
        let local = seal_with_tier(Tier::Local, &ev, 10);
        let authored = seal_with_tier(Tier::HumanVerified, &ev, 10);
        assert_ne!(local, authored, "tier swap did not change scc — tag is forgeable");
    }

    // ── Ingress seal-verify rung (ADR-0026 D6): RED under the accept-all stub ──

    #[test]
    fn ingress_accepts_honest_seal() {
        let ev = sample();
        let seal = seal_u64_for_tier(Tier::Local, &ev, 10);
        assert!(verify_seal_at_ingress(Tier::Local, &ev, 10, seal));
    }

    #[test]
    fn ingress_rejects_tampered_events() {
        let ev = sample();
        let seal = seal_u64_for_tier(Tier::Local, &ev, 10);
        // Flip one event AFTER sealing — the recomputed seal must diverge.
        let mut tampered = ev.clone();
        tampered[0].payload = Ump::new([0xDEAD_BEEF, 0, 0, 0]);
        assert!(
            !verify_seal_at_ingress(Tier::Local, &tampered, 10, seal),
            "tampered events must fail the ingress seal",
        );
    }

    #[test]
    fn ingress_rejects_tier_swap() {
        let ev = sample();
        let seal = seal_u64_for_tier(Tier::Local, &ev, 10); // sealed as Local/LLMCandidate
        // Receiver tries to admit the same bytes as HumanVerified — the kind atom
        // differs, so the recomputed seal will not match.
        assert!(
            !verify_seal_at_ingress(Tier::HumanVerified, &ev, 10, seal),
            "tier/kind swap must fail the ingress seal",
        );
    }

    // ── Moon rung: the (tick_id, moon, code_hash) context leg is bound, not silent ──

    #[test]
    fn moon_binds_into_scc_not_silent() {
        // Same events + tier, different moon ⇒ different scc. A silent no-op would
        // make the two seals match — Signal Law: that silence must be loud.
        let ev = sample();
        let moon_3 = seal_with_moon(3, Tier::Local, &ev, 10);
        let moon_7 = seal_with_moon(7, Tier::Local, &ev, 10);
        assert_ne!(moon_3, moon_7, "moon did not change scc — context leg is silent");
    }

    #[test]
    fn moon_zero_reduces_to_tier_only_seal() {
        // moon == 0 (unbound) must be byte-identical to the moon-less tier seal, so
        // existing callers keep their hashes (back-compat).
        let ev = sample();
        assert_eq!(
            seal_with_moon(0, Tier::Local, &ev, 10),
            seal_with_tier(Tier::Local, &ev, 10),
            "moon=0 must equal the tier-only seal",
        );
    }
}
