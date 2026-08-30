//! ADR-0001 — one control plane, no silent failures: WHO VERIFIES THE VERIFIER.
//!
//! The lattice is self-scanning. The board harvests its own tests and seals
//! itself, the gates are authored by the agent they gate, the index indexes
//! itself, the park is written by the session that just ran. A closed loop
//! agrees with itself perfectly and calls that green. Measured 2026-08-02:
//! 14 gate bounces, ONE named a real disk fact, and zero caught the four real
//! errors of that session — a human caught all four by asking a question.
//!
//! So proof is graded by WHO ANSWERED, not by how confident the answer was.
//! An oracle counts only if the system did not author it.

/// Where a receipt's answer came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Oracle {
    /// The machine answered: exit code, pixel readback, wall clock, the OS
    /// refusing to unlink a file a live holder still has open. Strongest —
    /// the repo cannot author it and cannot argue with it.
    Physics,
    /// A prior sealed BEFORE the question was asked: content-addressed hash,
    /// tape revision, a frozen baseline. Only independent because it predates
    /// the claim; a seal written to match a result proves nothing.
    SealedPrior,
    /// An observer outside this system — a different model family with
    /// different failure modes. Real, but CORRELATED: also confident when
    /// wrong, which is why its citations get checked against bytes.
    Foreign,
    /// The system grading its own homework. Never evidence on its own.
    SelfAuthored,
}

impl Oracle {
    /// Did something other than this system answer?
    pub fn is_independent(self) -> bool {
        !matches!(self, Oracle::SelfAuthored)
    }
}

/// The proof ladder. `Verified` is not a stronger feeling than `Proven` — it
/// is a different number of independent answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rung {
    /// Nothing outside the system has spoken.
    Unproven,
    /// One independent oracle answered. Traced, not corroborated.
    Proven,
    /// Two DISTINCT independent classes agreed. They must be able to fail
    /// differently, or the second answer is the first one wearing a hat.
    Verified,
}

/// Grade a claim by the oracles behind it. Duplicates within one class do not
/// count twice: ten exit codes are still one machine answering once.
pub fn rung(oracles: &[Oracle]) -> Rung {
    let mut kinds: Vec<Oracle> = oracles.iter().copied().filter(|o| o.is_independent()).collect();
    kinds.sort_unstable();
    kinds.dedup();
    match kinds.len() {
        0 => Rung::Unproven,
        1 => Rung::Proven,
        _ => Rung::Verified,
    }
}

/// Classify one receipt line by the shape of the evidence it carries. Pure and
/// deliberately conservative: anything it cannot place is [`Oracle::SelfAuthored`],
/// because an unrecognised receipt is the system talking about itself.
pub fn classify(receipt: &str) -> Oracle {
    let r = receipt.to_ascii_lowercase();
    const PHYSICS: &[&str] = &[
        "exit 0", "exit=0", "exit_code", "exitcode", "pixel", "readback", "phash",
        "os error", "errorkind", "wall clock", "elapsed", "vram", "fps",
    ];
    // Keys must be RECEIPT SHAPES, never prose words. "board sealed green" is the
    // system congratulating itself; `sha=` is a hash it had to compute. Caught by
    // the first test run, which classified the former as a sealed prior.
    const SEALED: &[&str] = &["sha256", "sha=", "seal=", "rev=", "content-address"];
    const FOREIGN: &[&str] = &["gemini", "massread", "peer review", "foreign oracle"];
    if PHYSICS.iter().any(|k| r.contains(k)) {
        Oracle::Physics
    } else if SEALED.iter().any(|k| r.contains(k)) {
        Oracle::SealedPrior
    } else if FOREIGN.iter().any(|k| r.contains(k)) {
        Oracle::Foreign
    } else {
        Oracle::SelfAuthored
    }
}

/// Why a claim may not be sealed green, or `None` if it may. THE LAW: a green
/// seal needs two independent classes. One is a trace; none is an opinion.
pub fn refuse_seal(receipts: &[String]) -> Option<String> {
    let oracles: Vec<Oracle> = receipts.iter().map(|r| classify(r)).collect();
    match rung(&oracles) {
        Rung::Verified => None,
        Rung::Proven => Some(
            "[PROVEN] not [VERIFIED]: one independent oracle answered. A second \
             class that can fail DIFFERENTLY is required to seal green."
                .into(),
        ),
        Rung::Unproven => Some(
            "[UNPROVEN]: every receipt is self-authored. The system graded its own \
             homework — bring physics (exit/readback/OS), a prior sealed before the \
             question, or a foreign observer."
                .into(),
        ),
    }
}

/// The lattice's visual grammar (Sean 2026-08-02). Three of these are the
/// closed loop's own trit states; the fourth is not a colour the machine may
/// award itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// +1 — two independent classes agreed. The only self-award permitted.
    Green,
    /// 0 — in flight: one oracle has answered, the pair is incomplete.
    Amber,
    /// -1 — the lattice measured and caught its own fault.
    Crimson,
    /// ⊥ — UN-GATED AXIOM. Sits above the trit machine: the lattice cannot
    /// grade it, tint it, or prove it, so it belongs to the operator. Painting
    /// it Green would be the closed loop congratulating itself on an unverified
    /// claim; painting it Crimson would assert an internal fault on a statement
    /// that sits above the gates. Raw reality, before the lattice swallows it.
    White,
}

impl Ink {
    /// THE SOVEREIGN INK — fixed sRGB, owned by the lattice, unreachable from any
    /// theme (Sean 2026-08-02: "themes control the canvas; the lattice controls
    /// the truth").
    ///
    /// This deliberately does NOT resolve through `forge_canvas::tokens::TokenId`.
    /// A token lookup let a cosmetic sheet alias two states onto one hex — proven
    /// on disk, not argued: `design/molten/molten.sheet.vixi:16` authors a single
    /// `warning_danger = #FFD54AFF`, which `molten.profile.sheet.vixi:17-18`
    /// carried into BOTH `warning` and `danger`. Under that theme a hard fault and
    /// an in-flight probe were the same colour, and the next theme author would
    /// have done it again. State is not a palette accent; a dark-mode skin does
    /// not get to repaint a stoplight.
    ///
    /// Coordinates are chosen for maximal hue separation at similar lightness so
    /// the three internal states stay distinguishable under the common colour
    /// deficiencies; `oklch()` below publishes where each one actually sits.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Ink::Crimson => [0xD1, 0x3A, 0x2F],
            Ink::Amber => [0xE8, 0xA3, 0x1E],
            Ink::Green => [0x35, 0xB5, 0x6B],
            // ⊥ is not a lattice colour — it is the absence of tint, the surface's
            // own ink. Near-white so an ungraded line reads as raw, unswallowed text.
            Ink::White => [0xF2, 0xF2, 0xF2],
        }
    }

    /// `0xRRGGBBAA`, the packed convention `technothesia::ansi_fg` already speaks.
    pub fn packed(self) -> u32 {
        let [r, g, b] = self.rgb();
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | 0xFF
    }

    /// Where this ink sits in OKLCH — see [`rgb_to_oklch`] for the conversion.
    /// PUBLISHED, not asserted: the separation this type claims is measurable.
    pub fn oklch(self) -> forge_core_v3::OklchColor {
        let [r, g, b] = self.rgb();
        rgb_to_oklch(r, g, b)
    }

    /// The trit this ink carries, or `None` for ⊥ — the state outside {-1,0,+1}.
    pub fn trit(self) -> Option<i8> {
        match self {
            Ink::Green => Some(1),
            Ink::Amber => Some(0),
            Ink::Crimson => Some(-1),
            Ink::White => None,
        }
    }
}

/// sRGB (0..=255) -> [`forge_core_v3::OklchColor`]. Ported locally since
/// forge-core-v3's `colour.rs` deliberately carries no RGB conversion (see its
/// module doc — "two prior homes exist and neither is imported"); this is the
/// third, v3-native one. Float math (Björn Ottosson's OKLab matrices) is
/// contained entirely inside this function — the boundary the crate's
/// no-float rule protects is the stored/compared type, [`OklchColor`] itself,
/// which stays four exact `u16` channels.
fn rgb_to_oklch(r: u8, g: u8, b: u8) -> forge_core_v3::OklchColor {
    fn srgb_to_linear(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    let (r, g, b) = (srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b));

    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    let (l_, m_, s_) = (l.cbrt(), m.cbrt(), s.cbrt());

    let ok_l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
    let ok_a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
    let ok_b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

    let chroma = (ok_a * ok_a + ok_b * ok_b).sqrt();
    let hue_turn = {
        let h = ok_b.atan2(ok_a) / (2.0 * std::f64::consts::PI);
        if h < 0.0 {
            h + 1.0
        } else {
            h
        }
    };

    let quantize = |v: f64| (v.clamp(0.0, 1.0) * u16::MAX as f64).round() as u16;

    forge_core_v3::OklchColor {
        l: quantize(ok_l),
        c: quantize(chroma / 0.4),
        h: (hue_turn * forge_core_v3::TURN as f64).round() as u16,
        a: u16::MAX,
    }
}

/// Ink for a claim. `fault` is a MEASURED failure, not a doubt — only a real
/// measurement may spend Crimson.
pub fn ink(receipts: &[String], fault: bool) -> Ink {
    if fault {
        return Ink::Crimson;
    }
    match rung(&receipts.iter().map(|r| classify(r)).collect::<Vec<_>>()) {
        Rung::Verified => Ink::Green,
        Rung::Proven => Ink::Amber,
        // Nothing outside the system spoke, so there is nothing here to grade.
        Rung::Unproven => Ink::White,
    }
}

/// One human-readable line for the board / a verb's stdout.
pub fn verdict_line(receipts: &[String]) -> String {
    let mut oracles: Vec<Oracle> = receipts.iter().map(|r| classify(r)).collect();
    oracles.sort_unstable();
    oracles.dedup();
    let names: Vec<String> = oracles.iter().map(|o| format!("{o:?}")).collect();
    let r = rung(&receipts.iter().map(|x| classify(x)).collect::<Vec<_>>());
    format!("ADR-0001 {r:?} · {} receipt(s) · oracles=[{}]", receipts.len(), names.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    // [BOARD: ADR0001-ORACLE] The rule that survives V2..V5: grade by WHO answered.
    #[test]
    fn a_system_grading_its_own_homework_never_reaches_verified() {
        // The 08-02 shape: a wall of self-authored receipts.
        let self_only = vec!["board sealed green".to_string(), "all rows pass".to_string()];
        assert_eq!(rung(&[Oracle::SelfAuthored; 3]), Rung::Unproven);
        assert!(refuse_seal(&self_only).unwrap().contains("[UNPROVEN]"));

        // Ten exit codes are ONE machine answering once.
        assert_eq!(rung(&[Oracle::Physics; 10]), Rung::Proven, "duplicates do not corroborate");

        // Two classes that fail differently.
        assert_eq!(rung(&[Oracle::Physics, Oracle::SealedPrior]), Rung::Verified);
        assert_eq!(rung(&[Oracle::Physics, Oracle::Foreign]), Rung::Verified);
        assert!(refuse_seal(&["exit 0".into(), "sha256 abc".into()]).is_none(), "may seal");

        // Self-authored never counts toward the pair.
        assert_eq!(rung(&[Oracle::Physics, Oracle::SelfAuthored]), Rung::Proven);
        assert!(refuse_seal(&["exit 0".into(), "looks right".into()])
            .unwrap()
            .contains("[PROVEN] not [VERIFIED]"));
    }

    // [BOARD: ADR0001-ORACLE] An unrecognised receipt is the system talking about
    // itself — the conservative default is the whole point.
    #[test]
    fn classification_defaults_to_self_authored_and_names_the_three_real_oracles() {
        assert_eq!(classify("render-gate exit_code=1"), Oracle::Physics);
        assert_eq!(classify("pixel readback 4/4"), Oracle::Physics);
        assert_eq!(classify("bin stamped sha=6093d8fb"), Oracle::SealedPrior);
        assert_eq!(classify("gemini-3.5-flash cited blueprint.rs:36"), Oracle::Foreign);
        assert_eq!(classify("it renders correctly now"), Oracle::SelfAuthored);
        assert_eq!(classify(""), Oracle::SelfAuthored, "silence is not evidence");
        assert!(!Oracle::SelfAuthored.is_independent());
        for o in [Oracle::Physics, Oracle::SealedPrior, Oracle::Foreign] {
            assert!(o.is_independent(), "{o:?}");
        }

        let line = verdict_line(&["exit 0".into(), "gemini said so".into()]);
        assert!(line.contains("Verified") && line.contains("Physics") && line.contains("Foreign"));
    }

    // [BOARD: ADR0001-ORACLE] The visual grammar (Sean 08-02): three trit states
    // belong to the closed loop; White is the un-gated axiom above it. The machine
    // may not award itself Green, and may not tint an ungraded claim Crimson.
    #[test]
    fn white_is_the_ungated_axiom_the_lattice_may_not_tint() {
        let two = vec!["exit 0".to_string(), "sha=6093d8fb".to_string()];
        let one = vec!["exit 0".to_string()];
        let none = vec!["it renders correctly now".to_string()];

        assert_eq!(ink(&two, false), Ink::Green, "+1 two classes agreed");
        assert_eq!(ink(&one, false), Ink::Amber, "0 in flight, pair incomplete");
        assert_eq!(ink(&one, true), Ink::Crimson, "-1 measured fault");

        // The claim the lattice cannot grade is neither a pass nor a fault.
        assert_eq!(ink(&none, false), Ink::White, "un-gated axiom, operator's");
        assert_eq!(ink(&[], false), Ink::White, "silence is ungraded, not green");
        assert_ne!(ink(&none, false), Ink::Green, "no self-congratulation");
        assert_ne!(ink(&none, false), Ink::Crimson, "no fault above the gates");

        // Only a real measurement may spend Crimson — doubt is not a fault.
        assert_eq!(ink(&two, true), Ink::Crimson, "fault dominates a green pair");
    }

    // [BOARD: ADR0001-ORACLE] THE THEME-HIJACK GUARD. molten aliased danger and
    // warning onto ONE hex (design/molten/molten.sheet.vixi:16), blinding a hard
    // fault. The lattice owns its inks outright so no sheet can do that again.
    #[test]
    fn no_two_lattice_states_may_ever_share_an_ink() {
        let all = [Ink::Crimson, Ink::Amber, Ink::Green, Ink::White];
        for a in 0..all.len() {
            for b in (a + 1)..all.len() {
                assert_ne!(all[a].rgb(), all[b].rgb(), "{:?}/{:?} collided", all[a], all[b]);
            }
        }
        // The exact collision that provoked this law must be impossible here.
        assert_ne!(Ink::Crimson.rgb(), Ink::Amber.rgb(), "the molten hijack");

        for i in all {
            assert_eq!(i.packed() & 0xFF, 0xFF, "opaque 0xRRGGBBAA");
            assert_eq!(i.packed() >> 24, i.rgb()[0] as u32, "red byte survives packing");
        }

        // Hue separation is MEASURED in OKLCH, not asserted by naming. The three
        // internal states must be far apart on the hue circle so a colour
        // deficiency cannot merge two of them.
        let hue = |i: Ink| i.oklch().h as i32;
        let sep = |a: Ink, b: Ink| {
            let d = (hue(a) - hue(b)).abs();
            d.min(36_000 - d)
        };
        for (a, b) in [(Ink::Crimson, Ink::Amber), (Ink::Amber, Ink::Green), (Ink::Crimson, Ink::Green)] {
            assert!(sep(a, b) > 3_000, "{a:?}/{b:?} only {} cdeg apart", sep(a, b));
        }

        // ⊥ is the least chromatic — it is absence of tint, not a fourth accent.
        let c = |i: Ink| i.oklch().c;
        for i in [Ink::Crimson, Ink::Amber, Ink::Green] {
            assert!(c(i) > c(Ink::White), "{i:?} must out-chroma ⊥");
        }

        // Trits: the closed loop is {-1,0,+1}; ⊥ carries no trit at all.
        assert_eq!(Ink::Green.trit(), Some(1));
        assert_eq!(Ink::Amber.trit(), Some(0));
        assert_eq!(Ink::Crimson.trit(), Some(-1));
        assert_eq!(Ink::White.trit(), None, "⊥ is not a trit value");

        // Every ink paints; only the three internal states carry a trit.
        assert_eq!(all.iter().filter(|i| i.trit().is_some()).count(), 3, "3 in, 1 above");
    }
}
