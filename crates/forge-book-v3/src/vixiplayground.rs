//! VixiPlayground — the layout HTML page maker + exporter, ASP-driven.
//!
//! "playground is the layout HTML page maker and exporter with WCE and ASP Clingo"
//! (Sean, 2026-07-12). This module is the ASP-Clingo half: an answer-set program
//! ([`crate::asp`]) decides WHICH authoring sections a page carries for a given era,
//! then the page is assembled from forge-book's own section chapters (the exact
//! builders `seed::vixiplayground_atlas` uses) and exported to one standalone HTML
//! string via [`crate::export_html::export_book`]. The studio Command Hub's
//! "Make the Magic" edict calls into [`vixiplayground_page`].
//!
//! Why ASP and not an `if` ladder: era layout is a CONSTRAINT problem — sections are
//! offered, rules derive what to include, and integrity constraints forbid illegal
//! combinations. The solver returns the one consistent layout (or none), so a page is
//! never silently half-built; an impossible layout exports an honest empty page.

use crate::asp::{Atom, Program, Rule};
use crate::book::Book;
use crate::fsm::QuestState;
use crate::midi::PhraseExt;
use crate::quest::Quest;

/// The five VixiPlayground authoring sections, in canonical page order. Each maps to a
/// real forge-book chapter builder (the same content `seed::vixiplayground_atlas`
/// assembles). ASP decides which of them a given era's page carries.
pub const SECTIONS: [&str; 5] = ["brushes", "fonts", "keys", "colour", "sound"];

/// Derive an era's offered sections from its NAME — a documented, deterministic
/// heuristic so each of the 20 vibe-rail eras drives a genuinely different page,
/// without a hand-maintained per-era table. The two core visual sections
/// (brushes, colour) are offered for EVERY era; letterpress/print eras add fonts;
/// music eras add keys; a music era that is NOT a dark twin also adds sound (a dark
/// twin's shadow register is a quiet one). Output stays in canonical [`SECTIONS`] order.
pub fn era_offering(era: &str) -> Vec<&'static str> {
    let e = era.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| e.contains(n));
    let print_era = has(&["1700s", "1800s", "1900s", "broadside", "penny", "deco"]);
    let music_era = has(&["1970s", "1980s", "1990s", "groove", "neon", "geocities", "gloss"]);
    let dark = e.contains("dark");
    SECTIONS
        .iter()
        .copied()
        .filter(|s| match *s {
            "brushes" | "colour" => true,
            "fonts" => print_era,
            "keys" => music_era,
            "sound" => music_era && !dark,
            _ => false,
        })
        .collect()
}

/// Build the ASP layout program for `era`: one `available(section)` fact per section
/// the era offers, the inclusion rule `include(S) :- available(S).`, and any era
/// conflict constraint. Solving it yields the `include(...)` atoms = the page's
/// sections. Unknown eras get the full set (nothing withheld).
pub fn layout_program(era: &str) -> Program {
    let offered: Vec<&str> = match era {
        "minimal" => vec!["brushes", "colour"],
        "sound_off" => vec!["brushes", "fonts", "keys", "colour"],
        "clash" | "full" => SECTIONS.to_vec(),
        other => era_offering(other), // real vibe-rail eras → their own section set
    };
    let mut p = Program::new();
    for s in offered {
        p.push(Rule::fact(Atom::new("available", vec![s])));
    }
    // include(S) :- available(S). — one recursive-free rule the grounder instantiates
    // over the Herbrand universe (the offered section names).
    p.push(Rule::when(Atom::new("include", vec!["S"]), vec![Atom::new("available", vec!["S"])]));
    // Era conflict: a "clash" page may not carry BOTH keys and sound — a real integrity
    // constraint that prunes an otherwise-valid layout to UNSAT (proving the solver
    // rejects illegal combinations, not just accumulates facts).
    if era == "clash" {
        p.push(Rule::constraint(vec![
            Atom::new("include", vec!["keys"]),
            Atom::new("include", vec!["sound"]),
        ]));
    }
    p
}

/// Derive the era's page palette — the `(--wall, --words)` CSS tokens the exported
/// page's own stylesheet reads via `var(--wall, …)` / `var(--words, …)`. A deterministic
/// register-per-era map (neon → violet ground + cyan ink, letterpress → cream + sepia,
/// dark twin → near-black, …), so a page LOOKS like its era, not just carries its
/// sections. Folds `export_html::export_book_themed`'s existing token seam — no new CSS.
pub fn era_theme_tokens(era: &str) -> Vec<(&'static str, &'static str)> {
    let e = era.to_ascii_lowercase();
    let has = |n: &str| e.contains(n);
    let (wall, words) = if has("dark") {
        ("#050505", "#c8c8c8") // dark twin — near-black ground, bleached ink
    } else if has("neon") || has("1980") || has("80s") {
        ("#0a0018", "#00ffd0") // neon — deep violet ground, cyan ink
    } else if has("1700") || has("1800") || has("broadside") || has("penny") || has("deco") {
        ("#efe6d0", "#2a2016") // letterpress — cream paper, sepia ink
    } else if has("geocities") || has("1990") || has("90s") {
        ("#000080", "#c0c0c0") // geocities — web-safe navy + silver
    } else if has("gloss") || has("2000") {
        ("#e8f0f8", "#102030") // gloss — glassy light
    } else if has("dino") || has("amber") {
        ("#1a1206", "#e0a030") // amber resin — dark ground + gold
    } else {
        ("#0c0a08", "#e0d4ba") // the base grimoire look
    };
    vec![("wall", wall), ("words", words)]
}

/// Export `b` with the era's palette folded in through the page's own `--wall`/`--words`
/// token seam ([`era_theme_tokens`] → `export_book_themed`).
fn export_themed(b: &Book, era: &str) -> String {
    crate::export_html::export_book_themed(b, Some(&era_theme_tokens(era)))
}

// ── PASS-1 PRIMER — the white-out cure (Sean 2026-07-14) ───────────────────────

/// The forced ground when the variance gate rejects a flat or near-white pair.
pub const COLD_IRON: &str = "#121519";

/// Seed-bound closed-loop colour field: integer HSL clamped to L∈[5,35] S∈[60,95] —
/// high saturation, low luminance. The wall a page stands on when no era register
/// claims it; a blank sheet cannot be constructed here, and the gate below makes it
/// impossible even under a planted fault.
pub fn primer_tokens(seed: u32) -> [(&'static str, String); 2] {
    let h = seed.wrapping_mul(2_654_435_761) % 360;
    let s = 60 + (seed >> 9) % 36; // 60..=95
    let l = 5 + (seed >> 17) % 31; // 5..=35
    let wall = hsl_hex(h, s, l);
    let words = hsl_hex((h + 40) % 360, s.min(80), 82); // light ink on the dark wall
    let (wall, words) = variance_gate(wall, words);
    [("wall", wall), ("words", words)]
}

/// Integer HSL→`#rrggbb`. h wraps at 360; s,l in percent.
fn hsl_hex(h: u32, s: u32, l: u32) -> String {
    let (h, s, l) = ((h % 360) as i64, s as i64, l as i64);
    let c = (100 - (2 * l - 100).abs()) * s / 100;
    let x = c * (60 - ((h % 120) - 60).abs()) / 60;
    let m = l - c / 2;
    let (r, g, b) = match h / 60 {
        0 => (c, x, 0),
        1 => (x, c, 0),
        2 => (0, c, x),
        3 => (0, x, c),
        4 => (x, 0, c),
        _ => (c, 0, x),
    };
    let ch = |v: i64| ((v + m).clamp(0, 100) * 255 / 100) as u8;
    format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
}

/// Rec-709 luma of a `#rrggbb`, 0..=255. A parse failure reads as white so the
/// gate fails TOWARD cold iron, never toward a blank sheet.
fn luma(hex: &str) -> u32 {
    let p = |i: usize| u32::from_str_radix(hex.get(i..i + 2).unwrap_or("ff"), 16).unwrap_or(255);
    (p(1) * 2126 + p(3) * 7152 + p(5) * 722) / 10000
}

/// The variance gate: zero contrast (wall==words) or a bright wall (luma>150 —
/// a sheet, not a ground) is rejected LOUD and forced to cold iron.
fn variance_gate(wall: String, words: String) -> (String, String) {
    if wall == words || luma(&wall) > 150 {
        eprintln!("[primer] variance gate tripped: wall={wall} words={words} -> cold iron");
        return (COLD_IRON.to_string(), "#e0d4ba".to_string());
    }
    (wall, words)
}

/// Era tokens with the primer riding underneath: a mapped era keeps its register
/// (dinosaurs stay amber); an unmapped era — which previously shared ONE fixed
/// grimoire tone — takes the seed's own primer field. Deterministic per (era, seed).
pub fn era_theme_tokens_seeded(era: &str, seed: u32) -> Vec<(&'static str, String)> {
    let base = era_theme_tokens(era);
    if base[0].1 == "#0c0a08" {
        return primer_tokens(seed).into();
    }
    base.into_iter().map(|(k, v)| (k, v.to_string())).collect()
}

/// Export through the primed palette — the exporter the seeded pipeline rides.
fn export_themed_seeded(b: &Book, era: &str, seed: u32) -> String {
    let toks = era_theme_tokens_seeded(era, seed);
    let borrowed: Vec<(&str, &str)> = toks.iter().map(|(k, v)| (*k, v.as_str())).collect();
    crate::export_html::export_book_themed(b, Some(&borrowed))
}

/// Solve the era's layout program and return its sections in canonical [`SECTIONS`]
/// order. An UNSAT program (a fired constraint) yields NO sections — the honest
/// "this layout is impossible" answer, never a silent partial page.
pub fn selected_sections(era: &str) -> Vec<&'static str> {
    let Some(model) = layout_program(era).answer_set() else {
        return Vec::new();
    };
    SECTIONS
        .iter()
        .copied()
        .filter(|s| model.contains(&format!("include({s})")))
        .collect()
}

/// Append the real forge-book chapter for one section name to `b`. Unknown names are
/// ignored (the ASP selection only ever yields [`SECTIONS`] entries). The builders
/// are the exact ones `seed::vixiplayground_atlas` uses — one source of section truth.
fn add_section(b: &mut Book, section: &str) {
    match section {
        "brushes" => {
            b.add_chapter(crate::brushes::forge_brushes().to_chapter("Brushes"));
        }
        "fonts" => {
            b.add_chapter(crate::fonts::TypeRamp::default_ramp().to_chapter("Fonts"));
        }
        "keys" => {
            b.add_chapter(crate::music::to_chapter(&crate::music::minor_ring(), "Keys"));
        }
        "colour" => {
            b.add_chapter(crate::colour::to_chapter(crate::colour::Oklch::new(6000, 1500, 30), "Colour"));
        }
        "sound" => {
            let mut phrase = crate::midi::Phrase::new();
            phrase
                .add(crate::midi::Note::new(60, 8000, 0))
                .add(crate::midi::Note::new(64, 8000, 0))
                .add(crate::midi::Note::new(67, 8000, 0));
            b.add_chapter(phrase.to_chapter("Sound"));
        }
        _ => {}
    }
}

/// THE VixiPlayground page maker: ASP picks the era's sections, the page is built from
/// their real chapters and exported to one standalone HTML string. This is the whole
/// "layout HTML page maker + exporter" contract at the forge-book layer — the studio
/// hub's Make-the-Magic edict lowers into this. An UNSAT era exports an honest empty
/// page (title shell, no sections), never a crash and never a silent partial.
pub fn vixiplayground_page(era: &str, title: &str, author: &str) -> String {
    let mut b = Book::new(title, author);
    for section in selected_sections(era) {
        add_section(&mut b, section);
    }
    export_themed(&b, era)
}

// ── WCE (World Consequence Engine) gate ────────────────────────────────────────

/// The extra section a completed world-quest unlocks — rendered by [`add_consequence`],
/// never part of the base [`SECTIONS`] rail.
pub const CONSEQUENCE_SECTION: &str = "consequence";

/// Extend `layout_program(era)` with a WCE input: a quest whose STATE gates the
/// `consequence` section. A `Sealed` quest asserts the nullary fact `sealed` and the
/// rule `include(consequence) :- sealed.`, so the page grows a consequence coda ONLY
/// once the world-quest has run to completion — WCE feeding ASP feeding the layout.
/// Any earlier quest state leaves the base layout untouched.
pub fn layout_program_wce(era: &str, quest: &Quest) -> Program {
    let mut p = layout_program(era);
    if quest.state == QuestState::Sealed {
        p.push(Rule::fact(Atom::nullary("sealed")));
        p.push(Rule::when(
            Atom::new("include", vec![CONSEQUENCE_SECTION]),
            vec![Atom::nullary("sealed")],
        ));
    }
    p
}

/// Sections for a WCE-gated page: the base era sections (canonical order) plus
/// `consequence` appended as a coda when the quest has sealed. An UNSAT layout yields
/// no sections (same honest contract as [`selected_sections`]).
pub fn selected_sections_wce(era: &str, quest: &Quest) -> Vec<&'static str> {
    let Some(model) = layout_program_wce(era, quest).answer_set() else {
        return Vec::new();
    };
    let mut out: Vec<&'static str> = SECTIONS
        .iter()
        .copied()
        .filter(|s| model.contains(&format!("include({s})")))
        .collect();
    if model.contains(&format!("include({CONSEQUENCE_SECTION})")) {
        out.push(CONSEQUENCE_SECTION);
    }
    out
}

/// Append the WCE consequence chapter — the sealed quest's objectives + XP, the
/// "world remembers" coda — built from forge-book's own Chapter/Page/Block model.
fn add_consequence(b: &mut Book, quest: &Quest) {
    let mut ch = crate::chapter::Chapter::new(
        "Consequence",
        crate::atlas::AtlasSection::Custom("Consequence".into()),
    );
    ch.add_lore(format!(
        "Quest '{}' sealed — {} XP awarded. The world remembers what you made.",
        quest.id, quest.xp
    ));
    let mut body = String::from("Objectives fulfilled:\n");
    for o in &quest.objectives {
        body.push_str(&format!("- {}: {}/{}\n", o.target, o.current, o.required));
    }
    let mut page = crate::page::Page::new(1);
    page.add(crate::block::Block::text(body));
    ch.add_page(page);
    b.add_chapter(ch);
}

/// The WCE-driven VixiPlayground page: like [`vixiplayground_page`], but a `Sealed`
/// world-quest unlocks an extra Consequence chapter (its objectives + XP). The page
/// layout is a CONSEQUENCE of world state — WCE (quest) → ASP (gate) → HTML (page).
pub fn vixiplayground_page_wce(era: &str, quest: &Quest, title: &str, author: &str) -> String {
    let mut b = Book::new(title, author);
    for section in selected_sections_wce(era, quest) {
        if section == CONSEQUENCE_SECTION {
            add_consequence(&mut b, quest);
        } else {
            add_section(&mut b, section);
        }
    }
    export_themed(&b, era)
}

// ── GhostMoon × Randomizer × VixiPlayground — the seeded pipeline ───────────────

/// The 20 vibe-rail eras (10 eras × light/dark), verbatim from the studio's ERA_CHIPS
/// labels — the roll space a seed picks from.
pub const ERA_NAMES: [&str; 20] = [
    "dinosaurs amber", "1700s broadside", "1800s penny", "1900s deco", "1970s groove",
    "1980s neon", "1990s geocities", "2000s gloss", "2010s flat", "whatever open",
    "dino amber dark", "1700s dark", "1800s dark", "1900s dark", "1970s dark",
    "1980s dark", "1990s dark", "2000s dark", "2010s dark", "whatever dark",
];

/// The eras as an equal-weight roll table for the [`crate::randomizer::Randomizer`].
pub fn era_roll_table() -> crate::randomizer::WeightedTable {
    let mut t = crate::randomizer::WeightedTable::new();
    for era in ERA_NAMES {
        t.add(1, era);
    }
    t
}

/// Roll an era from `seed`, deterministically (mulberry32). A GhostMoon 5D probe's
/// codeword is a `u32` — feeding it here lets the ghost DECK pick a VixiPlayground era,
/// and the same seed always re-rolls the same era (share a seed, not a file).
pub fn roll_era_from_seed(seed: u32) -> String {
    let mut r = crate::randomizer::Randomizer::new(seed);
    r.roll(&era_roll_table()).unwrap_or_else(|| "whatever open".to_string())
}

/// GhostMoon × Randomizer × VixiPlayground, one call: a `seed` (e.g. a GhostMoon
/// codeword) rolls the era, then the WCE page maker generates it — exported through
/// the PASS-1 PRIMER ([`era_theme_tokens_seeded`]), so an unmapped era paints the
/// seed's own deep-colour field instead of a shared fallback. Deterministic — the
/// same seed reproduces the same page byte-for-byte.
pub fn vixiplayground_page_seeded(seed: u32, quest: &Quest, author: &str) -> String {
    let era = roll_era_from_seed(seed);
    let mut b = Book::new(&format!("Vixi Playground — {era}"), author);
    for section in selected_sections_wce(&era, quest) {
        if section == CONSEQUENCE_SECTION {
            add_consequence(&mut b, quest);
        } else {
            add_section(&mut b, section);
        }
    }
    export_themed_seeded(&b, &era, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_era_includes_every_section() {
        assert_eq!(selected_sections("full"), SECTIONS.to_vec());
    }

    #[test]
    fn primer_never_prints_a_sheet() {
        for seed in 0..1000u32 {
            let [(_, wall), (_, words)] = primer_tokens(seed);
            assert_eq!(wall.len(), 7, "seed {seed}: malformed hex {wall}");
            assert_ne!(wall, words, "seed {seed}: zero contrast");
            assert!(luma(&wall) <= 150, "seed {seed}: wall {wall} is a sheet");
        }
        assert_eq!(primer_tokens(13), primer_tokens(13));
    }

    #[test]
    fn variance_gate_negative_control_goes_cold_iron() {
        // the planted fault MUST trip the gate — the proof-of-proof
        let (wall, _) = variance_gate("#ffffff".into(), "#ffffff".into());
        assert_eq!(wall, COLD_IRON);
        let (wall, _) = variance_gate("#f8f8f8".into(), "#101010".into());
        assert_eq!(wall, COLD_IRON);
    }

    #[test]
    fn unmapped_eras_get_seed_varied_primer_walls() {
        let a = era_theme_tokens_seeded("2010s flat", 13);
        let b = era_theme_tokens_seeded("2010s flat", 777);
        assert_ne!(a[0].1, b[0].1, "two seeds, one wall — primer not riding");
        let dino = era_theme_tokens_seeded("dinosaurs amber", 13);
        assert_eq!(dino[0].1, "#1a1206", "mapped era must keep its register");
    }

    #[test]
    fn era_offering_varies_by_era_character() {
        // musical era → keys + sound; its dark twin keeps keys but loses sound.
        assert_eq!(era_offering("1980s neon"), vec!["brushes", "keys", "colour", "sound"]);
        assert_eq!(era_offering("1980s dark"), vec!["brushes", "keys", "colour"]);
        // print era → fonts, no music.
        assert_eq!(era_offering("1700s broadside"), vec!["brushes", "fonts", "colour"]);
        // a plain era → the two core visual sections only.
        assert_eq!(era_offering("dinosaurs amber"), vec!["brushes", "colour"]);
        // EVERY era offers the core visual pair.
        for e in ["1990s geocities", "2010s flat", "whatever open", "dino amber dark"] {
            let o = era_offering(e);
            assert!(o.contains(&"brushes") && o.contains(&"colour"), "{e} missing core sections");
        }
    }

    #[test]
    fn real_era_pages_differ_by_selection() {
        // two different real vibe-rail eras → different ASP selections → different pages.
        let neon = vixiplayground_page("1980s neon", "Vixi Playground", "deveraux");
        let dino = vixiplayground_page("dinosaurs amber", "Vixi Playground", "deveraux");
        assert!(neon.contains("Sound"), "80s neon is a music era → carries the Sound section");
        assert!(
            neon.len() > dino.len(),
            "the richer 80s-neon era exports a larger page than the core-only dino era"
        );
    }

    #[test]
    fn era_theme_tokens_vary_and_reach_the_page() {
        // each era register gets its own (--wall, --words) palette.
        assert_eq!(era_theme_tokens("1980s neon"), vec![("wall", "#0a0018"), ("words", "#00ffd0")]);
        assert_eq!(era_theme_tokens("1700s broadside"), vec![("wall", "#efe6d0"), ("words", "#2a2016")]);
        assert_ne!(era_theme_tokens("1980s neon"), era_theme_tokens("dinosaurs amber"));
        // and the palette reaches the exported page's :root block.
        let neon = vixiplayground_page("1980s neon", "Vixi Playground", "deveraux");
        assert!(neon.contains("--wall:#0a0018"), "neon wall token injected into :root");
        assert!(neon.contains("--words:#00ffd0"), "neon words token injected");
        let print = vixiplayground_page("1700s broadside", "Vixi Playground", "deveraux");
        assert!(print.contains("--wall:#efe6d0"), "letterpress cream wall injected");
    }

    #[test]
    fn seeded_roll_is_deterministic_and_in_range() {
        // same seed → same era, always one of the 20 real vibe-rail eras.
        assert_eq!(roll_era_from_seed(42), roll_era_from_seed(42));
        assert!(ERA_NAMES.contains(&roll_era_from_seed(42).as_str()));
        assert!(ERA_NAMES.contains(&roll_era_from_seed(7).as_str()));
        // the roll space spreads — 200 seeds land on more than one era.
        let distinct: std::collections::BTreeSet<String> =
            (0..200u32).map(roll_era_from_seed).collect();
        assert!(distinct.len() > 1, "the seed roll is not stuck on one era");
    }

    #[test]
    fn seeded_page_reproduces_byte_for_byte() {
        use crate::fsm::Event;
        let mut q = Quest::new("first_creation", 500).objective("page", 1);
        q.advance(Event::Discover);
        q.advance(Event::Accept);
        q.record("page", 1);
        q.advance(Event::TurnIn);
        // GhostMoon codeword 0xC0DE → the SAME page every time (share a seed, not a file).
        let a = vixiplayground_page_seeded(0xC0DE, &q, "deveraux");
        let b = vixiplayground_page_seeded(0xC0DE, &q, "deveraux");
        assert_eq!(a, b, "same seed reproduces the same page byte-for-byte");
        assert!(a.contains("Vixi Playground"));
    }

    #[test]
    fn minimal_era_is_asp_pruned_to_two_sections() {
        assert_eq!(selected_sections("minimal"), vec!["brushes", "colour"]);
    }

    #[test]
    fn sound_off_era_drops_only_sound() {
        let s = selected_sections("sound_off");
        assert!(s.contains(&"brushes") && s.contains(&"colour") && s.contains(&"keys"));
        assert!(!s.contains(&"sound"), "sound_off era must not include the sound section");
    }

    #[test]
    fn clash_constraint_makes_the_layout_unsat_and_empty() {
        // keys + sound are both offered but forbidden together → answer_set None →
        // zero sections (the honest impossible-layout answer).
        assert!(layout_program("clash").answer_set().is_none());
        assert!(selected_sections("clash").is_empty());
    }

    #[test]
    fn page_export_scales_with_the_asp_selection() {
        let full = vixiplayground_page("full", "Vixi Playground", "deveraux");
        let minimal = vixiplayground_page("minimal", "Vixi Playground", "deveraux");
        assert!(full.contains("Brushes") && full.contains("Colour"));
        assert!(minimal.contains("Brushes") && minimal.contains("Colour"));
        // full carries 5 ASP-selected sections, minimal carries 2 → strictly larger.
        assert!(full.len() > minimal.len(), "more ASP-selected sections → a larger exported page");
        // a clash era is UNSAT → an empty (section-less) page, smaller than minimal.
        let clash = vixiplayground_page("clash", "Vixi Playground", "deveraux");
        assert!(clash.len() < minimal.len(), "UNSAT layout exports an empty page");
    }

    #[test]
    fn wce_sealed_quest_unlocks_the_consequence_section() {
        use crate::fsm::Event;
        let mut q = Quest::new("first_creation", 500).objective("page", 1);
        // an un-run quest (Unknown state) → no consequence coda.
        assert!(!selected_sections_wce("full", &q).contains(&CONSEQUENCE_SECTION));
        // drive the WCE state machine to Sealed → ASP unlocks the coda.
        q.advance(Event::Discover);
        q.advance(Event::Accept);
        q.record("page", 1);
        q.advance(Event::TurnIn);
        assert_eq!(q.state, QuestState::Sealed);
        let s = selected_sections_wce("full", &q);
        assert_eq!(s.last(), Some(&CONSEQUENCE_SECTION), "consequence is the coda — last");
        assert_eq!(s.len(), SECTIONS.len() + 1);
    }

    #[test]
    fn wce_page_renders_the_consequence_only_when_sealed() {
        use crate::fsm::Event;
        let mut q = Quest::new("first_creation", 777).objective("page", 1);
        let before = vixiplayground_page_wce("full", &q, "Vixi Playground", "deveraux");
        assert!(!before.contains("Consequence"), "un-sealed quest → no consequence chapter");
        q.advance(Event::Discover);
        q.advance(Event::Accept);
        q.record("page", 1);
        q.advance(Event::TurnIn);
        let after = vixiplayground_page_wce("full", &q, "Vixi Playground", "deveraux");
        assert!(after.contains("Consequence"), "sealed quest → the world-consequence chapter renders");
        assert!(after.contains("777"), "the sealed quest's XP reward reaches the page");
        assert!(after.len() > before.len(), "the consequence coda grows the page");
    }
}
