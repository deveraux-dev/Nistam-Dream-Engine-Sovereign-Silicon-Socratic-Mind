//! Three-word compound namer, coded directly from Sean's prose theory
//! (session notes, 2026-08-17) rather than left as prose. Two axes, kept
//! separate on purpose because the theory only specified one of them per
//! word:
//!
//! **Slot theory** (verbatim, not paraphrased into something else):
//! - Word 1: unambiguous — reads only one way.
//! - Word 2: ambiguous — can be taken both ways (the pivot).
//! - Word 3: literal — pins the phrase down. Example given: "dead".
//!
//! **Hermetic polarity** (masc/fem, tied to `.claude/skills-attic/2026-08-16/
//! master-game-dev-skillv1.SKILL.md`'s hermetics section): Sean supplied one
//! full pole in prose this session — "Chaos Feminine" (reproduced in
//! `FEMININE_SOURCE` below, verbatim, so the word bank's provenance is
//! checkable against it) — and none of the masculine pole yet.
//!
//! `MASCULINE` stays an empty slice on purpose. A namer that filled it with
//! invented masculine-pole words would be a plausible-guess-as-receipt
//! (CLAUDE.md T1 `zero_hallucination`) — every word in `FEMININE` traces to
//! a line in `FEMININE_SOURCE`; there is no equivalent source yet for the
//! other pole. Callers get `Polarity::Masculine` compiling and type-checking
//! today; it just has nothing to draw from until that prose exists.
//!
//! Composition is caller-driven (`compose` takes explicit indices), not
//! RNG-seeded — this codebase already carries three independent hand-rolled
//! LCGs (`sidecar/src/ml/train_s13.rs`, `forge-envelope/src/bin/
//! chaos_monkey.rs`, `forge-envelope/tests/scale_test.rs`, flagged L05
//! one-home candidates in the 2026-08-17 truth-hunt); this module does not
//! add a fourth just to pick words.

/// Hermetic polarity a word (or a whole composed phrase) is drawn from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Polarity {
    /// "Chaos Feminine, Mother Unleashed" — see `FEMININE_SOURCE`.
    Feminine,
    /// No source prose exists yet (see module doc) — `MASCULINE` is empty.
    Masculine,
    /// No polarity claimed either way.
    Neutral,
}

/// The feminine-pole poem this word bank is drawn from, verbatim (Sean,
/// 2026-08-17 session prose). Every entry in [`FEMININE`] below traces to a
/// word or phrase actually present here.
pub const FEMININE_SOURCE: &str = "Chaos Feminine\n\
--------------\n\n\
How graceful the river runs, far and wide.\n\
Calm, until dawn.\n\n\
The river curves, and radiates pure beauty.\n\
Until it doesn't.\n\n\
The river brings life, and chaos.\n\n\
The river brings love, and hate.\n\n\
The river is always running one way or the other on the line.\n\n\
She can take you and make you feel whole, or take your life away.\n\n\
This is Chaos Feminine, Mother Unleashed.";

/// Word 1 pool — unambiguous, drawn from `FEMININE_SOURCE`. Each reads one
/// way in the poem's own context (a place, a time, a role — no pivot).
pub const FEMININE_UNAMBIGUOUS: &[&str] = &["River", "Dawn", "Mother"];

/// Word 2 pool — ambiguous, drawn from `FEMININE_SOURCE`. Each carries the
/// poem's own stated double reading:
/// - `Grace`/`Graceful` — elegance, or mercy/forgiveness.
/// - `Chaos` — destructive, or generative (the poem states both explicitly).
/// - `Whole` — complete, or the homophone "holy".
/// - `Curve` — the river's shape, or a deception/seduction.
/// - `Running` — fleeing, or flowing ("running one way or the other").
pub const FEMININE_AMBIGUOUS: &[&str] = &["Graceful", "Chaos", "Whole", "Curve", "Running"];

/// Word 3 pool — literal, grounding. `Dead` is Sean's own word3 example,
/// given directly in prose, not present in `FEMININE_SOURCE` itself — kept
/// first/index-0 as the anchor since it's the one word3 he actually named.
/// `Life`, `Hate`, `Unleashed` ARE in `FEMININE_SOURCE` — the poem's other
/// unqualified, un-hedged nouns/verb, no double reading offered for any of
/// them in the source text.
pub const FEMININE_LITERAL: &[&str] = &["Dead", "Life", "Hate", "Unleashed"];

/// Masculine pole — intentionally empty. See module doc: no source prose
/// exists yet. Filling this without one would be an invented claim about
/// Sean's own system, not a coded version of it.
pub const MASCULINE: &[&str] = &[];

/// A composed three-word phrase plus the polarity it was drawn from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamePhrase {
    /// Unambiguous first word.
    pub word1: &'static str,
    /// Ambiguous, dual-reading second word.
    pub word2: &'static str,
    /// Literal, grounding third word.
    pub word3: &'static str,
    /// Which pole this phrase was composed from.
    pub polarity: Polarity,
}

impl NamePhrase {
    /// Render as a single space-joined phrase, e.g. `"River Chaos Dead"`.
    pub fn phrase(&self) -> String {
        format!("{} {} {}", self.word1, self.word2, self.word3)
    }
}

/// Errors composing a [`NamePhrase`] — LOUD, not a silent fallback word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeError {
    /// The requested polarity has no word bank yet (currently: `Masculine`).
    EmptyBank(Polarity),
    /// An index was out of range for its pool. `(slot, index, pool_len)`.
    IndexOutOfRange(u8, usize, usize),
}

/// Balanced-trit synthesis state, ported from a Python `TRIT_TEMP_MAP`
/// pattern Sean shared this session (an LLM `generationConfig.temperature`
/// keyed by trit state: -1 → 0.0 "coldest/crystalline argmax", 0 → 0.2
/// "equilibrium/fixed point", +1 → 1.0 "fluid/generative synthesis"). No
/// float, no LLM call here (`CLAUDE.md forbidden_ops`, and this module is a
/// local static word-bank generator, not a network client) — the concept
/// that ports is the *shape*: trit state gates how committed a composed
/// phrase is, not the specific temperature-to-API-call plumbing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i8)]
pub enum TritState {
    /// Coldest / crystalline argmax — always resolve to the single most
    /// literal word3 (`FEMININE_LITERAL[0]`, "Dead" — Sean's own anchor
    /// example), ignoring the caller's `idx3`.
    Crystalline = -1,
    /// Equilibrium / fixed point — the plain `compose`: caller's indices,
    /// used as given.
    Fixed = 0,
    /// Fluid / generative synthesis — word3 is dropped in favor of a
    /// second ambiguous-pool word (`idx3` reused against the ambiguous
    /// bank), so the phrase stays fully open instead of ever landing.
    Fluid = 1,
}

/// Synthesis commitment, fixed-point Permyriad (`0..=10_000`; 1.0 = 10_000).
/// Mirrors the Python map's `temperature` field one-for-one at each trit
/// state — `0.0`/`0.2`/`1.0` become `0`/`2_000`/`10_000`.
pub fn trit_temp_pmy(state: TritState) -> u16 {
    match state {
        TritState::Crystalline => 0,
        TritState::Fixed => 2_000,
        TritState::Fluid => 10_000,
    }
}

/// [`compose`], but gated by [`TritState`] the way the ported Python map
/// gates an LLM call's temperature — same three inputs, but `Crystalline`
/// pins word3 to the literal anchor and `Fluid` never lands on one at all.
pub fn compose_at_trit(
    polarity: Polarity,
    state: TritState,
    idx1: usize,
    idx2: usize,
    idx3: usize,
) -> Result<NamePhrase, ComposeError> {
    match state {
        TritState::Crystalline => compose(polarity, idx1, idx2, 0),
        TritState::Fixed => compose(polarity, idx1, idx2, idx3),
        TritState::Fluid => {
            let (unambig, ambig, _literal) = match polarity {
                Polarity::Feminine => (FEMININE_UNAMBIGUOUS, FEMININE_AMBIGUOUS, FEMININE_LITERAL),
                Polarity::Masculine => return Err(ComposeError::EmptyBank(Polarity::Masculine)),
                Polarity::Neutral => return Err(ComposeError::EmptyBank(Polarity::Neutral)),
            };
            let word1 = *unambig.get(idx1).ok_or(ComposeError::IndexOutOfRange(1, idx1, unambig.len()))?;
            let word2 = *ambig.get(idx2).ok_or(ComposeError::IndexOutOfRange(2, idx2, ambig.len()))?;
            let word3 = *ambig.get(idx3).ok_or(ComposeError::IndexOutOfRange(3, idx3, ambig.len()))?;
            Ok(NamePhrase { word1, word2, word3, polarity })
        }
    }
}

/// Compose a three-word phrase from `polarity`'s banks at the given
/// caller-supplied indices — deterministic, no RNG (see module doc).
pub fn compose(polarity: Polarity, idx1: usize, idx2: usize, idx3: usize) -> Result<NamePhrase, ComposeError> {
    let (unambig, ambig, literal) = match polarity {
        Polarity::Feminine => (FEMININE_UNAMBIGUOUS, FEMININE_AMBIGUOUS, FEMININE_LITERAL),
        Polarity::Masculine => return Err(ComposeError::EmptyBank(Polarity::Masculine)),
        Polarity::Neutral => return Err(ComposeError::EmptyBank(Polarity::Neutral)),
    };
    let word1 = *unambig.get(idx1).ok_or(ComposeError::IndexOutOfRange(1, idx1, unambig.len()))?;
    let word2 = *ambig.get(idx2).ok_or(ComposeError::IndexOutOfRange(2, idx2, ambig.len()))?;
    let word3 = *literal.get(idx3).ok_or(ComposeError::IndexOutOfRange(3, idx3, literal.len()))?;
    Ok(NamePhrase { word1, word2, word3, polarity })
}

/// Total number of distinct phrases `compose` can produce for a polarity
/// (0 for banks that are empty, e.g. `Masculine` today).
pub fn phrase_count(polarity: Polarity) -> usize {
    match polarity {
        Polarity::Feminine => FEMININE_UNAMBIGUOUS.len() * FEMININE_AMBIGUOUS.len() * FEMININE_LITERAL.len(),
        Polarity::Masculine | Polarity::Neutral => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_a_phrase_from_feminine_bank() {
        let p = compose(Polarity::Feminine, 0, 1, 0).unwrap();
        assert_eq!(p.phrase(), "River Chaos Dead");
        assert_eq!(p.polarity, Polarity::Feminine);
    }

    #[test]
    fn masculine_bank_is_empty_by_design() {
        assert!(MASCULINE.is_empty());
        assert_eq!(phrase_count(Polarity::Masculine), 0);
        assert_eq!(compose(Polarity::Masculine, 0, 0, 0), Err(ComposeError::EmptyBank(Polarity::Masculine)));
    }

    #[test]
    fn neutral_bank_is_also_empty() {
        assert_eq!(compose(Polarity::Neutral, 0, 0, 0), Err(ComposeError::EmptyBank(Polarity::Neutral)));
    }

    #[test]
    fn out_of_range_index_is_loud_not_a_silent_wrap() {
        let err = compose(Polarity::Feminine, 99, 0, 0).unwrap_err();
        assert_eq!(err, ComposeError::IndexOutOfRange(1, 99, FEMININE_UNAMBIGUOUS.len()));
    }

    #[test]
    fn phrase_count_matches_the_bank_product() {
        assert_eq!(
            phrase_count(Polarity::Feminine),
            FEMININE_UNAMBIGUOUS.len() * FEMININE_AMBIGUOUS.len() * FEMININE_LITERAL.len()
        );
    }

    #[test]
    fn trit_temp_matches_the_ported_python_map() {
        assert_eq!(trit_temp_pmy(TritState::Crystalline), 0);
        assert_eq!(trit_temp_pmy(TritState::Fixed), 2_000);
        assert_eq!(trit_temp_pmy(TritState::Fluid), 10_000);
    }

    #[test]
    fn crystalline_always_lands_on_the_anchor_word() {
        let p = compose_at_trit(Polarity::Feminine, TritState::Crystalline, 0, 1, 3).unwrap();
        assert_eq!(p.word3, FEMININE_LITERAL[0]);
        assert_eq!(p.word3, "Dead");
    }

    #[test]
    fn fixed_matches_plain_compose() {
        let a = compose_at_trit(Polarity::Feminine, TritState::Fixed, 1, 2, 1).unwrap();
        let b = compose(Polarity::Feminine, 1, 2, 1).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn fluid_never_lands_on_a_literal_word() {
        let p = compose_at_trit(Polarity::Feminine, TritState::Fluid, 0, 0, 1).unwrap();
        assert!(!FEMININE_LITERAL.contains(&p.word3));
        assert!(FEMININE_AMBIGUOUS.contains(&p.word3));
    }

    #[test]
    fn every_word_appears_in_its_own_source_poem_except_the_named_word3_example() {
        // Provenance check: every bank entry (case-insensitively) traces to
        // FEMININE_SOURCE, so the bank cannot silently drift from the poem
        // it claims to be derived from — except "Dead", which the module
        // doc and FEMININE_LITERAL's own comment both mark as Sean's
        // separately-given word3 example, not part of the poem text.
        let lower_source = FEMININE_SOURCE.to_lowercase();
        for &w in FEMININE_UNAMBIGUOUS.iter().chain(FEMININE_AMBIGUOUS).chain(FEMININE_LITERAL) {
            if w == "Dead" {
                continue;
            }
            let stem = w.to_lowercase();
            let stem = stem.strip_suffix("ful").unwrap_or(&stem); // Graceful -> grace(ful)
            assert!(
                lower_source.contains(stem) || lower_source.contains(&w.to_lowercase()),
                "{w} not traceable to FEMININE_SOURCE"
            );
        }
    }
}
