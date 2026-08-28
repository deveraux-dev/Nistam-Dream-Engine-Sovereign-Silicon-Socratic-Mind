//! phonology — structured featural decoder for the UCAS syllabary.
//!
//! ## Why a decoder, not more string heuristics
//! `syllabic_to_event` classifies vowels/onsets with ad-hoc `ends_with`/`contains`
//! chains that special-case each series by hand. That works for a handful of glyphs
//! but leaks (see its `!upper.ends_with("NASKAPI")` guards). Every UCAS codepoint,
//! though, carries its **own** ground truth: the canonical Unicode name. "CANADIAN
//! SYLLABICS KWAA" *is* the statement "consonant K, labial medial W, long vowel AA".
//!
//! This module parses that name into a [`Phoneme`] once, and the rest of the crate
//! (pitch mapping, romanization, the book pipe) reads structured fields instead of
//! re-deriving them from substrings. The ~600-entry table becomes a self-verifying
//! oracle: a parse is correct iff it round-trips against the name it came from.
//!
//! ## The CV grammar
//! After stripping `CANADIAN SYLLABICS ` and any dialect qualifier, the trailing
//! whitespace token is the phonetic cluster: `[onset consonant] [W medial] <vowel>`.
//! Cree/Ojibwe abugida structure: the glyph *shape* is the consonant, its *rotation*
//! is the vowel. We recover both from the name.
//!
//! Firewall Law: pure `std`, no engine dep — same as the rest of forge-calligraphy.

use crate::cree_syllabics::SyllabicEntry;

// ── Vowel quality ───────────────────────────────────────────────────────────────

/// The vowel quality carried by a syllabic's orientation. Length is tracked
/// separately in [`Phoneme::long`] because the Cree short/long pair (a/â, i/î)
/// shares one quality but differs in duration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vowel {
    /// ê — front, unrounded, mid.
    E,
    /// î — front, unrounded, high.
    I,
    /// ô — back, rounded, high/mid.
    O,
    /// â — open, central.
    A,
    /// ai — the front-closing diphthong (AI / AAI).
    Ai,
    /// oy — the back-closing diphthong (Extended block, OY).
    Oy,
    /// ay — the open-closing diphthong (Extended block, AY / AAY).
    Ay,
}

impl Vowel {
    /// Standard Roman Orthography short form, no length mark.
    pub fn sro_short(self) -> &'static str {
        match self {
            Vowel::E => "e",
            Vowel::I => "i",
            Vowel::O => "o",
            Vowel::A => "a",
            Vowel::Ai => "ai",
            Vowel::Oy => "oy",
            Vowel::Ay => "ay",
        }
    }

    /// Standard Roman Orthography with the circumflex length mark applied when
    /// `long` is set (â/î/ô/ê). The diphthongs lengthen their leading nucleus.
    pub fn sro(self, long: bool) -> &'static str {
        match (self, long) {
            (Vowel::E, false) => "e",
            (Vowel::E, true) => "ê",
            (Vowel::I, false) => "i",
            (Vowel::I, true) => "î",
            (Vowel::O, false) => "o",
            (Vowel::O, true) => "ô",
            (Vowel::A, false) => "a",
            (Vowel::A, true) => "â",
            (Vowel::Ai, false) => "ai",
            (Vowel::Ai, true) => "âi",
            (Vowel::Oy, false) => "oy",
            (Vowel::Oy, true) => "ôy",
            (Vowel::Ay, false) => "ay",
            (Vowel::Ay, true) => "ây",
        }
    }
}

// ── Consonant onset ─────────────────────────────────────────────────────────────

/// The onset (initial) consonant of a syllable. `None` is a bare-vowel glyph
/// (the ᐁ ᐃ ᐅ ᐊ series). `Other` is an onset outside the core Cree/Ojibwe grid
/// (Carrier, Dene, Blackfoot exotics) that slice-3 dialect handling refines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consonant {
    /// Bare vowel — no onset.
    None,
    /// p — voiceless bilabial stop.
    P,
    /// t — voiceless alveolar stop.
    T,
    /// k — voiceless velar stop.
    K,
    /// c — the affricate (ts / ch).
    C,
    /// m — bilabial nasal.
    M,
    /// n — alveolar nasal.
    N,
    /// l — lateral approximant.
    L,
    /// s — voiceless alveolar fricative.
    S,
    /// š (sh) — voiceless postalveolar fricative (Ojibwe).
    Sh,
    /// y — palatal approximant.
    Y,
    /// r — rhotic (Michif / loan).
    R,
    /// w — labiovelar approximant as a full onset (the WE/WI/WO/WA series).
    W,
    /// h — glottal fricative.
    H,
    /// th — the Woods-Cree interdental (ð).
    Th,
    /// An onset outside the core grid (Carrier/Dene/Blackfoot); dialect-specific.
    Other,
}

impl Consonant {
    /// Standard Roman Orthography onset spelling (`""` for a bare vowel).
    pub fn sro(self) -> &'static str {
        match self {
            Consonant::None => "",
            Consonant::P => "p",
            Consonant::T => "t",
            Consonant::K => "k",
            Consonant::C => "c",
            Consonant::M => "m",
            Consonant::N => "n",
            Consonant::L => "l",
            Consonant::S => "s",
            Consonant::Sh => "sh",
            Consonant::Y => "y",
            Consonant::R => "r",
            Consonant::W => "w",
            Consonant::H => "h",
            Consonant::Th => "th",
            Consonant::Other => "?",
        }
    }
}

// ── The decoded syllable ────────────────────────────────────────────────────────

/// A fully decoded CV syllable: onset consonant, optional labial medial, vowel
/// quality, and length. This is the structured truth that replaces the scattered
/// `contains`/`ends_with` heuristics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Phoneme {
    /// Onset consonant.
    pub consonant: Consonant,
    /// True if a labial medial `w` sits between onset and vowel (PWA, KWE, SWOO).
    pub medial_w: bool,
    /// The vowel quality.
    pub vowel: Vowel,
    /// True if the vowel is long (â/î/ô/ê-doubled: AA, II, OO, AAI …).
    pub long: bool,
}

impl Phoneme {
    /// True for a bare-vowel glyph (no onset consonant, no medial).
    pub fn is_bare_vowel(&self) -> bool {
        matches!(self.consonant, Consonant::None) && !self.medial_w
    }
}

// ── Token grammar ───────────────────────────────────────────────────────────────

/// The longest vowel suffixes, tried longest-first so `AAI` wins over `AI` over
/// `A`. Each maps to (quality, long). `Y` never appears alone — only in a
/// closing diphthong — so it is not a standalone suffix.
const VOWEL_SUFFIXES: &[(&str, Vowel, bool)] = &[
    ("AAI", Vowel::Ai, true),
    ("AAY", Vowel::Ay, true),
    ("AI", Vowel::Ai, false),
    ("AY", Vowel::Ay, false),
    ("OY", Vowel::Oy, false),
    ("AA", Vowel::A, true),
    ("II", Vowel::I, true),
    ("OO", Vowel::O, true),
    ("EE", Vowel::E, true),
    ("A", Vowel::A, false),
    ("E", Vowel::E, false),
    ("I", Vowel::I, false),
    ("O", Vowel::O, false),
];

/// Map a leading onset cluster (the letters before the vowel, W already removed)
/// to a [`Consonant`]. Empty → `None`. Unknown → `Other`.
fn onset_from(cluster: &str) -> Consonant {
    match cluster {
        "" => Consonant::None,
        "P" => Consonant::P,
        "T" => Consonant::T,
        "K" => Consonant::K,
        "C" => Consonant::C,
        "M" => Consonant::M,
        "N" => Consonant::N,
        "L" => Consonant::L,
        "S" => Consonant::S,
        "SH" => Consonant::Sh,
        "Y" => Consonant::Y,
        "R" => Consonant::R,
        "H" => Consonant::H,
        "TH" => Consonant::Th,
        _ => Consonant::Other,
    }
}

/// Parse a bare phonetic token (e.g. `"PWAA"`, `"KWE"`, `"OO"`, `"WA"`) into a
/// [`Phoneme`]. Returns `None` if no vowel suffix matches (a consonant-only final
/// like `"P"`, or an exotic cluster like `"KEH"`).
pub fn parse_token(token: &str) -> Option<Phoneme> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    // 1. Peel the longest vowel suffix off the tail.
    let (vowel, long, head) = VOWEL_SUFFIXES.iter().find_map(|&(suf, v, l)| {
        t.strip_suffix(suf).map(|head| (v, l, head))
    })?;

    // 2. What remains is [onset][W]. A trailing W with something before it is a
    //    labial medial; a lone W is the approximant onset itself.
    let (consonant, medial_w) = if let Some(before) = head.strip_suffix('W') {
        if before.is_empty() {
            (Consonant::W, false) // WE / WI / WO / WA — W is the onset
        } else {
            (onset_from(before), true) // PWE / KWA — W is the medial
        }
    } else {
        (onset_from(head), false)
    };

    Some(Phoneme { consonant, medial_w, vowel, long })
}

/// The dialect/qualifier words that may precede the phonetic token in a UCAS name.
/// We take the last whitespace token as the cluster, so this list documents the
/// space we skip past rather than driving the parse — but a name that is *only* a
/// qualifier + non-vowel token (e.g. a bare final) will fail the vowel peel and
/// return `None`, which is the honest answer for slice 1.
pub const QUALIFIERS: &[&str] = &[
    "WEST-CREE", "Y-CREE", "MOOSE-CREE", "R-CREE", "NASKAPI", "SOUTH-SLAVEY",
    "SAYISI", "ATHAPASCAN", "BLACKFOOT", "OJIBWAY", "EASTERN", "WESTERN",
    "CARRIER", "SANIEGO", "BEAVER", "DENE",
];

/// Parse a full UCAS canonical name into a [`Phoneme`].
///
/// Returns `None` for names that are not a plain CV/V syllable: finals, the
/// hyphen, punctuation, bare consonants, and exotic clusters. Those are handled by
/// [`crate::syllabic_to_event`] (finals → drums) and later dialect slices.
pub fn parse_name(name: &str) -> Option<Phoneme> {
    let rest = name.strip_prefix("CANADIAN SYLLABICS ")?;
    // Finals and punctuation carry no CV structure.
    if rest.contains("FINAL") || rest.contains("HYPHEN") || rest.contains("FULL STOP") {
        return None;
    }
    // The phonetic cluster is the last whitespace token (qualifiers precede it).
    let token = rest.rsplit(' ').next().unwrap_or(rest);
    parse_token(token)
}

/// Parse the [`Phoneme`] of a table entry (`(codepoint, char, name)`).
pub fn phoneme_of(entry: &SyllabicEntry) -> Option<Phoneme> {
    parse_name(entry.2)
}

// ── Romanization (Standard Roman Orthography, forward) ──────────────────────────

impl Phoneme {
    /// Render this syllable in Standard Roman Orthography: onset + optional labial
    /// medial `w` + vowel (with the circumflex length mark). `pê`, `kwâ`, `sô`,
    /// `wa`. The one place syllabic phonology becomes readable Latin text.
    pub fn romanize(&self) -> String {
        let mut s = String::with_capacity(4);
        s.push_str(self.consonant.sro());
        if self.medial_w {
            s.push('w');
        }
        s.push_str(self.vowel.sro(self.long));
        s
    }
}

/// Romanize a single UCAS canonical name, if it decodes to a CV/V syllable.
pub fn romanize_name(name: &str) -> Option<String> {
    parse_name(name).map(|p| p.romanize())
}

/// Transliterate a run of syllabics into Standard Roman Orthography.
///
/// CV/V glyphs romanize to their syllable; standalone-consonant glyphs (bare codas
/// like ᓐ n, ᒃ k) romanize to their coda letter; non-syllabic characters (spaces,
/// Latin, punctuation) pass through unchanged so word boundaries survive. Shape-only
/// finals (the "FINAL …" glyphs) are dialect-specific and dropped rather than
/// guessed — the honest floor.
pub fn romanize_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if let Some(entry) = crate::cree_syllabics::by_char(ch) {
            if let Some(p) = phoneme_of(entry) {
                out.push_str(&p.romanize());
            } else if let Some(coda) = standalone_coda(entry.2) {
                out.push_str(coda);
            }
            // else: shape-only final / punctuation → drop (honest floor)
        } else {
            out.push(ch); // spaces / Latin / punctuation pass through
        }
    }
    out
}

// ── SRO → syllabics transliteration (slice 7) ───────────────────────────────────

/// Roman coda → syllabic glyph. The inverse of [`standalone_coda`]: scans for the
/// bare-consonant glyph whose coda spelling matches, preferring [`Dialect::Common`].
pub fn coda_glyph(roman: &str) -> Option<char> {
    let mut fallback = None;
    for entry in crate::cree_syllabics::UCAS_MAIN
        .iter()
        .chain(crate::cree_syllabics::UCAS_EXTENDED.iter())
    {
        if standalone_coda(entry.2) == Some(roman) {
            if dialect_of(entry.2) == Dialect::Common {
                return Some(entry.1);
            }
            fallback.get_or_insert(entry.1);
        }
    }
    fallback
}

/// Match an onset consonant at `chars[i..]`, returning `(consonant, chars consumed)`.
/// A vowel or unknown character yields `(None, 0)`.
fn match_onset(chars: &[char], i: usize) -> (Consonant, usize) {
    // Two-letter onsets first so `sh`/`th` beat `s`/`t`.
    if i + 1 < chars.len() {
        match (chars[i], chars[i + 1]) {
            ('s', 'h') => return (Consonant::Sh, 2),
            ('t', 'h') => return (Consonant::Th, 2),
            _ => {}
        }
    }
    let c = match chars[i] {
        'p' => Consonant::P,
        't' => Consonant::T,
        'k' => Consonant::K,
        'c' => Consonant::C,
        'm' => Consonant::M,
        'n' => Consonant::N,
        'l' => Consonant::L,
        's' => Consonant::S,
        'y' => Consonant::Y,
        'r' => Consonant::R,
        'h' => Consonant::H,
        'w' => Consonant::W,
        _ => return (Consonant::None, 0),
    };
    (c, 1)
}

/// Match a vowel (with length) at `chars[j..]`, returning `(vowel, long, consumed)`.
/// Two-glyph diphthongs and circumflex length marks are matched longest-first.
fn match_vowel(chars: &[char], j: usize) -> Option<(Vowel, bool, usize)> {
    if j >= chars.len() {
        return None;
    }
    // Diphthongs: nucleus (short or circumflex-long) + closing glide.
    if j + 1 < chars.len() {
        match (chars[j], chars[j + 1]) {
            ('â', 'i') => return Some((Vowel::Ai, true, 2)),
            ('a', 'i') => return Some((Vowel::Ai, false, 2)),
            ('ô', 'y') => return Some((Vowel::Oy, true, 2)),
            ('o', 'y') => return Some((Vowel::Oy, false, 2)),
            ('â', 'y') => return Some((Vowel::Ay, true, 2)),
            ('a', 'y') => return Some((Vowel::Ay, false, 2)),
            _ => {}
        }
    }
    let (v, long) = match chars[j] {
        'ê' => (Vowel::E, true),
        'e' => (Vowel::E, false),
        'î' => (Vowel::I, true),
        'i' => (Vowel::I, false),
        'ô' => (Vowel::O, true),
        'o' => (Vowel::O, false),
        'â' => (Vowel::A, true),
        'a' => (Vowel::A, false),
        _ => return None,
    };
    Some((v, long, 1))
}

/// Transliterate a Standard-Roman-Orthography Cree string into syllabics — the
/// inverse of [`romanize_text`]. Parses each `[onset][w][vowel]` syllable and emits
/// its glyph; a consonant with no following vowel becomes its coda glyph. Characters
/// that are not SRO letters (spaces, punctuation) pass through. Roman that has no
/// glyph (an unmodelled cluster) is emitted verbatim so no input is silently lost.
///
/// Note the inherent SRO ambiguity: bare `ai`/`oy`/`ay` are read as the UCAS
/// diphthong glyphs (matching what [`romanize_text`] emits), not as two vowels.
pub fn transliterate_sro(sro: &str) -> String {
    let chars: Vec<char> = sro.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let (onset, adv) = match_onset(&chars, i);
        let mut j = i + adv;

        // A labial medial `w` only after a real (non-w) onset consonant.
        let medial = onset != Consonant::None
            && onset != Consonant::W
            && j < chars.len()
            && chars[j] == 'w';
        if medial {
            j += 1;
        }

        if let Some((vowel, long, vadv)) = match_vowel(&chars, j) {
            j += vadv;
            match compose(onset, medial, vowel, long) {
                Some(ch) => out.push(ch),
                None => out.extend(&chars[i..j]), // no glyph → keep the roman
            }
            i = j;
        } else if onset != Consonant::None {
            // A consonant with no vowel: emit its coda glyph if one exists.
            match coda_glyph(onset.sro()) {
                Some(ch) => {
                    out.push(ch);
                    i += adv;
                }
                None => {
                    out.push(chars[i]);
                    i += 1;
                }
            }
        } else {
            out.push(chars[i]); // space / punctuation / unmodelled
            i += 1;
        }
    }
    out
}

// ── Dialect + role classification (slice 3) ─────────────────────────────────────

/// The dialect/variant marker carried in a UCAS name. Derived strictly from the
/// literal qualifier word so it is verifiable against the chart — `Common` means no
/// qualifier (the shared Cree core).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    /// No qualifier — the shared Plains/core grid.
    Common,
    /// `WEST-CREE`.
    WestCree,
    /// `Y-CREE` (the y-dialect).
    YCree,
    /// `R-CREE`.
    RCree,
    /// `MOOSE-CREE`.
    MooseCree,
    /// `NASKAPI`.
    Naskapi,
    /// `OJIBWAY`.
    Ojibway,
    /// `SOUTH-SLAVEY`.
    SouthSlavey,
    /// `SAYISI`.
    Sayisi,
    /// `ATHAPASCAN`.
    Athapascan,
    /// `BLACKFOOT`.
    Blackfoot,
    /// `CARRIER`.
    Carrier,
    /// `SANIEGO`.
    Saniego,
    /// `BEAVER` / Beaver Dene.
    Beaver,
    /// `DENE` (generic Dene).
    Dene,
    /// `EASTERN` / `WESTERN` positional variant (the W finals).
    Positional,
}

/// Classify the dialect marker of a UCAS name. First matching qualifier wins;
/// checked most-specific first so `WEST-CREE` is not shadowed by `WESTERN`.
pub fn dialect_of(name: &str) -> Dialect {
    // Ordered: compound markers before bare positional words.
    for (needle, d) in [
        ("WEST-CREE", Dialect::WestCree),
        ("Y-CREE", Dialect::YCree),
        ("R-CREE", Dialect::RCree),
        ("MOOSE-CREE", Dialect::MooseCree),
        ("NASKAPI", Dialect::Naskapi),
        ("OJIBWAY", Dialect::Ojibway),
        ("SOUTH-SLAVEY", Dialect::SouthSlavey),
        ("SAYISI", Dialect::Sayisi),
        ("ATHAPASCAN", Dialect::Athapascan),
        ("BLACKFOOT", Dialect::Blackfoot),
        ("CARRIER", Dialect::Carrier),
        ("SANIEGO", Dialect::Saniego),
        ("BEAVER", Dialect::Beaver),
        ("DENE", Dialect::Dene),
        ("EASTERN", Dialect::Positional),
        ("WESTERN", Dialect::Positional),
    ] {
        if name.contains(needle) {
            return d;
        }
    }
    Dialect::Common
}

/// What kind of unit a glyph is, structurally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A CV or bare-vowel syllable ([`parse_name`] succeeds).
    Syllable,
    /// A shape-named coda (`FINAL …`) — dialect-specific articulation.
    Final,
    /// The hyphen or full stop — no phonemic content.
    Punctuation,
    /// A bare standalone consonant glyph (ᓐ n, ᒃ k, ᒄ kw) with a roman coda.
    Standalone,
    /// A glyph whose structure this decoder does not yet model.
    Unknown,
}

/// Classify a UCAS name into a structural [`Role`].
pub fn role_of(name: &str) -> Role {
    if name.contains("HYPHEN") || name.contains("FULL STOP") {
        return Role::Punctuation;
    }
    if name.contains("FINAL") {
        return Role::Final;
    }
    if parse_name(name).is_some() {
        return Role::Syllable;
    }
    if standalone_coda(name).is_some() {
        return Role::Standalone;
    }
    Role::Unknown
}

/// Roman coda for a bare standalone-consonant glyph (name ends in a lone consonant
/// cluster with no vowel: `CANADIAN SYLLABICS N`, `… KW`, `… SK`). `None` for
/// anything that is not a recognised bare consonant.
pub fn standalone_coda(name: &str) -> Option<&'static str> {
    let rest = name.strip_prefix("CANADIAN SYLLABICS ")?;
    if rest.contains("FINAL") || rest.contains("HYPHEN") || rest.contains("FULL STOP") {
        return None;
    }
    // The glottal stop (ᐞ) is a real Cree phoneme, conventionally romanized `h`.
    if rest.contains("GLOTTAL") {
        return Some("h");
    }
    let token = rest.rsplit(' ').next().unwrap_or(rest);
    Some(match token {
        "P" => "p",
        "T" => "t",
        "K" => "k",
        "C" => "c",
        "M" => "m",
        "N" => "n",
        "L" => "l",
        "S" => "s",
        "Y" => "y",
        "R" => "r",
        "W" => "w",
        "H" => "h",
        "KW" => "kw",
        // ᕽ — the Plains-Cree hk coda (x-final), restored to its true name 07-28.
        "HK" => "hk",
        "SK" => "sk",
        "SKW" => "skw",
        "SW" => "sw",
        "S-W" => "sw",
        "SH" => "sh",
        "NG" => "ng",
        "NH" => "nh",
        "MH" => "mh",
        "TH" => "th",
        // Vowel + n combined glyphs (ᐫ ᐬ ᐭ ᐮ) — a vowel nucleus with an n coda.
        "EN" => "en",
        "IN" => "in",
        "ON" => "on",
        "AN" => "an",
        _ => return None,
    })
}

// ── Reverse composition (slice 6) — phoneme → glyph ─────────────────────────────

/// Find the syllabic character for a decoded [`Phoneme`]. The inverse of
/// [`parse_name`]: scans the table and returns the glyph whose name decodes to the
/// same phoneme, preferring the canonical [`Dialect::Common`] form over dialect
/// variants (so `k + â` yields ᑲ-series KAA, not WEST-CREE KAA). `None` if the CV
/// combination has no glyph in the table.
pub fn compose(consonant: Consonant, medial_w: bool, vowel: Vowel, long: bool) -> Option<char> {
    let target = Phoneme { consonant, medial_w, vowel, long };
    let mut fallback = None;
    for entry in crate::cree_syllabics::UCAS_MAIN
        .iter()
        .chain(crate::cree_syllabics::UCAS_EXTENDED.iter())
    {
        if phoneme_of(entry) == Some(target) {
            if dialect_of(entry.2) == Dialect::Common {
                return Some(entry.1);
            }
            fallback.get_or_insert(entry.1);
        }
    }
    fallback
}

/// Compose from a [`Phoneme`] directly (convenience over [`compose`]).
pub fn compose_phoneme(p: &Phoneme) -> Option<char> {
    compose(p.consonant, p.medial_w, p.vowel, p.long)
}

// ── Vowel featural model (slice 4) ──────────────────────────────────────────────

/// Tongue height of a monophthong — the IPA vertical axis. Diphthongs report their
/// *starting* nucleus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Height {
    /// Close (i-like).
    High,
    /// Mid (e/o-like).
    Mid,
    /// Open (a-like).
    Low,
}

/// Backness — the IPA horizontal axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backness {
    /// Front (i, e).
    Front,
    /// Central (a).
    Central,
    /// Back (o).
    Back,
}

impl Vowel {
    /// Tongue height (diphthongs use their leading nucleus).
    pub fn height(self) -> Height {
        match self {
            Vowel::I => Height::High,
            Vowel::E => Height::Mid,
            Vowel::O => Height::Mid,   // Cree ô ranges o~u; classed mid here
            Vowel::A => Height::Low,
            Vowel::Ai => Height::Low,  // starts open, closes front
            Vowel::Ay => Height::Low,
            Vowel::Oy => Height::Mid,  // starts mid-back, closes front
        }
    }

    /// Backness (diphthongs use their leading nucleus).
    pub fn backness(self) -> Backness {
        match self {
            Vowel::I => Backness::Front,
            Vowel::E => Backness::Front,
            Vowel::O => Backness::Back,
            Vowel::A => Backness::Central,
            Vowel::Ai => Backness::Central,
            Vowel::Ay => Backness::Central,
            Vowel::Oy => Backness::Back,
        }
    }

    /// Lip rounding. Only the back vowel `o` (and the oy nucleus) is rounded in Cree.
    pub fn rounded(self) -> bool {
        matches!(self, Vowel::O | Vowel::Oy)
    }

    /// True for the closing diphthongs (ai/oy/ay), false for the monophthongs.
    pub fn is_diphthong(self) -> bool {
        matches!(self, Vowel::Ai | Vowel::Oy | Vowel::Ay)
    }
}

// ── Whole-table coverage (slice 4) — the honest completeness oracle ─────────────

/// True if a name carries no dialect qualifier outside the shared Cree core
/// (Common / West-Cree / Y-Cree / R-Cree / Moose-Cree / Naskapi). The decoder
/// models this whole grid; Carrier/Dene/Blackfoot/Ojibway exotics are the named
/// boundary beyond it.
pub fn is_core_cree(name: &str) -> bool {
    matches!(
        dialect_of(name),
        Dialect::Common | Dialect::WestCree | Dialect::YCree | Dialect::RCree
            | Dialect::MooseCree | Dialect::Naskapi
    )
}

/// Count of `(core-Cree classified, core-Cree total, unknown-tail)` across the whole
/// UCAS table. `classified` == `total` for the core grid is the coverage invariant;
/// the tail is the exotic remainder the decoder deliberately does not claim.
pub fn coverage() -> (usize, usize, usize) {
    let mut core_classified = 0;
    let mut core_total = 0;
    let mut unknown_tail = 0;
    for entry in crate::cree_syllabics::UCAS_MAIN
        .iter()
        .chain(crate::cree_syllabics::UCAS_EXTENDED.iter())
    {
        let name = entry.2;
        let role = role_of(name);
        if is_core_cree(name) {
            core_total += 1;
            if role != Role::Unknown {
                core_classified += 1;
            }
        } else if role == Role::Unknown {
            unknown_tail += 1;
        }
    }
    (core_classified, core_total, unknown_tail)
}

// ── Tests — every assertion is a real UCAS character vs its chart name ───────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cree_syllabics::by_char;

    fn ph(ch: char) -> Phoneme {
        phoneme_of(by_char(ch).unwrap()).unwrap_or_else(|| panic!("{ch} did not parse"))
    }

    #[test]
    fn bare_vowels_have_no_onset() {
        for (ch, v, long) in [
            ('ᐁ', Vowel::E, false),  // E
            ('ᐃ', Vowel::I, false),  // I
            ('ᐅ', Vowel::O, false),  // O
            ('ᐊ', Vowel::A, false),  // A
            ('ᐄ', Vowel::I, true),   // II (long)
            ('ᐆ', Vowel::O, true),   // OO (long)
            ('ᐋ', Vowel::A, true),   // AA (long)
        ] {
            let p = ph(ch);
            assert_eq!(p.consonant, Consonant::None, "{ch} is a bare vowel");
            assert_eq!(p.vowel, v, "{ch} vowel");
            assert_eq!(p.long, long, "{ch} length");
            assert!(!p.medial_w);
            assert!(p.is_bare_vowel());
        }
    }

    #[test]
    fn p_series_is_a_stop() {
        // PE PI PO PA — the four cardinal rotations of the P triangle.
        assert_eq!(ph('ᐯ'), Phoneme { consonant: Consonant::P, medial_w: false, vowel: Vowel::E, long: false });
        assert_eq!(ph('ᐱ'), Phoneme { consonant: Consonant::P, medial_w: false, vowel: Vowel::I, long: false });
        assert_eq!(ph('ᐳ'), Phoneme { consonant: Consonant::P, medial_w: false, vowel: Vowel::O, long: false });
        assert_eq!(ph('ᐸ'), Phoneme { consonant: Consonant::P, medial_w: false, vowel: Vowel::A, long: false });
    }

    #[test]
    fn t_k_c_m_n_s_series_onsets() {
        assert_eq!(ph('ᑌ').consonant, Consonant::T); // TE
        assert_eq!(ph('ᑫ').consonant, Consonant::K); // KE
        assert_eq!(ph('ᒉ').consonant, Consonant::C); // CE
        assert_eq!(ph('ᒣ').consonant, Consonant::M); // ME
        assert_eq!(ph('ᓀ').consonant, Consonant::N); // NE
        assert_eq!(ph('ᓭ').consonant, Consonant::S); // SE
        assert_eq!(ph('ᓓ').consonant, Consonant::L); // LE
    }

    #[test]
    fn w_series_onset_vs_medial() {
        // WA — the W is the onset approximant, no medial.
        let wa = ph('ᐗ');
        assert_eq!(wa.consonant, Consonant::W);
        assert!(!wa.medial_w);
        assert_eq!(wa.vowel, Vowel::A);
    }

    #[test]
    fn labial_medial_is_recovered() {
        // PWE / KWA / SWOO — a stop/fricative onset with a labial W medial.
        let pwe = ph('ᐺ'); // PWE
        assert_eq!(pwe.consonant, Consonant::P);
        assert!(pwe.medial_w, "PWE carries a labial medial");
        assert_eq!(pwe.vowel, Vowel::E);

        let kwaa = ph('ᒀ'); // KWAA
        assert_eq!(kwaa.consonant, Consonant::K);
        assert!(kwaa.medial_w);
        assert_eq!(kwaa.vowel, Vowel::A);
        assert!(kwaa.long, "KWAA is long");
    }

    #[test]
    fn long_vowels_flagged() {
        assert!(ph('ᑏ').long);  // TII
        assert!(ph('ᑑ').long);  // TOO
        assert!(ph('ᑖ').long);  // TAA
        assert!(!ph('ᑎ').long); // TI short
    }

    #[test]
    fn ai_diphthong_parses() {
        // AAI (ᐂ) — the front-closing diphthong, long.
        let aai = ph('ᐂ');
        assert_eq!(aai.vowel, Vowel::Ai);
        assert!(aai.long);
        assert_eq!(aai.consonant, Consonant::None);
    }

    #[test]
    fn finals_and_hyphen_do_not_parse() {
        assert!(phoneme_of(by_char('ᐟ').unwrap()).is_none(), "FINAL ACUTE has no CV");
        assert!(phoneme_of(by_char('᐀').unwrap()).is_none(), "HYPHEN has no CV");
        assert!(phoneme_of(by_char('ᑉ').unwrap()).is_none(), "bare P final has no vowel");
    }

    #[test]
    fn parse_token_direct() {
        assert_eq!(
            parse_token("PWAA"),
            Some(Phoneme { consonant: Consonant::P, medial_w: true, vowel: Vowel::A, long: true })
        );
        assert_eq!(parse_token("WE"), Some(Phoneme { consonant: Consonant::W, medial_w: false, vowel: Vowel::E, long: false }));
        assert_eq!(parse_token("P"), None, "consonant-only token has no vowel");
        assert_eq!(parse_token(""), None);
    }

    #[test]
    fn sro_spellings_are_stable() {
        assert_eq!(Consonant::K.sro(), "k");
        assert_eq!(Consonant::None.sro(), "");
        assert_eq!(Vowel::A.sro_short(), "a");
        assert_eq!(Vowel::Ai.sro_short(), "ai");
    }

    #[test]
    fn romanize_cardinal_syllables() {
        assert_eq!(ph('ᐯ').romanize(), "pe"); // PE
        assert_eq!(ph('ᐱ').romanize(), "pi"); // PI
        assert_eq!(ph('ᐳ').romanize(), "po"); // PO
        assert_eq!(ph('ᐸ').romanize(), "pa"); // PA
        assert_eq!(ph('ᑫ').romanize(), "ke"); // KE
        assert_eq!(ph('ᓴ').romanize(), "sa"); // SA
    }

    #[test]
    fn romanize_long_vowels_get_circumflex() {
        assert_eq!(ph('ᐹ').romanize(), "pâ"); // PAA
        assert_eq!(ph('ᑏ').romanize(), "tî"); // TII
        assert_eq!(ph('ᑑ').romanize(), "tô"); // TOO
        assert_eq!(ph('ᐋ').romanize(), "â");  // AA bare long
    }

    #[test]
    fn romanize_labial_medial_inserts_w() {
        assert_eq!(ph('ᒀ').romanize(), "kwâ"); // KWAA
        assert_eq!(ph('ᐺ').romanize(), "pwe"); // PWE
        assert_eq!(ph('ᐗ').romanize(), "wa");  // WA (onset w, no double)
    }

    #[test]
    fn romanize_name_and_text() {
        assert_eq!(romanize_name("CANADIAN SYLLABICS KWAA").as_deref(), Some("kwâ"));
        assert_eq!(romanize_name("CANADIAN SYLLABICS FINAL ACUTE"), None);
        // A pure-CV run transliterates; a trailing space passes through.
        assert_eq!(romanize_text("ᐸᑕ "), "pata ");
    }

    #[test]
    fn dialect_is_marker_derived() {
        assert_eq!(dialect_of("CANADIAN SYLLABICS PE"), Dialect::Common);
        assert_eq!(dialect_of("CANADIAN SYLLABICS WEST-CREE WE"), Dialect::WestCree);
        assert_eq!(dialect_of("CANADIAN SYLLABICS Y-CREE OO"), Dialect::YCree);
        assert_eq!(dialect_of("CANADIAN SYLLABICS NASKAPI KWAA"), Dialect::Naskapi);
        assert_eq!(dialect_of("CANADIAN SYLLABICS CARRIER HEE"), Dialect::Carrier);
        assert_eq!(dialect_of("CANADIAN SYLLABICS OJIBWAY P"), Dialect::Ojibway);
        // WEST-CREE must not be shadowed by the WESTERN positional check.
        assert_eq!(dialect_of("CANADIAN SYLLABICS WEST-CREE PWAA"), Dialect::WestCree);
    }

    #[test]
    fn role_classification() {
        assert_eq!(role_of("CANADIAN SYLLABICS PA"), Role::Syllable);
        assert_eq!(role_of("CANADIAN SYLLABICS FINAL ACUTE"), Role::Final);
        assert_eq!(role_of("CANADIAN SYLLABICS HYPHEN"), Role::Punctuation);
        assert_eq!(role_of("CANADIAN SYLLABICS N"), Role::Standalone);
        assert_eq!(role_of("CANADIAN SYLLABICS KW"), Role::Standalone);
    }

    #[test]
    fn standalone_codas_from_real_glyphs() {
        // ᓐ N, ᒃ K, ᑉ P, ᔅ S, ᒄ KW — the bare consonant glyphs.
        assert_eq!(standalone_coda(by_char('ᓐ').unwrap().2), Some("n"));
        assert_eq!(standalone_coda(by_char('ᒃ').unwrap().2), Some("k"));
        assert_eq!(standalone_coda(by_char('ᑉ').unwrap().2), Some("p"));
        assert_eq!(standalone_coda(by_char('ᔅ').unwrap().2), Some("s"));
        assert_eq!(standalone_coda(by_char('ᒄ').unwrap().2), Some("kw"));
        // A CV syllable is not a standalone coda.
        assert_eq!(standalone_coda("CANADIAN SYLLABICS PA"), None);
    }

    #[test]
    fn romanize_text_includes_standalone_codas() {
        // ᐊᑮᑉ = a-kî-p → "akîp" (vowel + long CV + bare-p coda).
        assert_eq!(romanize_text("ᐊᑮᑉ"), "akîp");
    }

    #[test]
    fn compose_is_the_inverse_of_parse() {
        // Cardinal CV glyphs round-trip: parse → compose → same char.
        for ch in ['ᐁ', 'ᐊ', 'ᐯ', 'ᑲ', 'ᓴ', 'ᒀ', 'ᐗ', 'ᐹ'] {
            let p = phoneme_of(by_char(ch).unwrap()).unwrap();
            assert_eq!(compose_phoneme(&p), Some(ch), "{ch} must round-trip");
        }
    }

    #[test]
    fn compose_prefers_common_dialect() {
        // k + long â with a labial medial → the canonical KWAA (ᒀ), not WEST-CREE.
        assert_eq!(compose(Consonant::K, true, Vowel::A, true), Some('ᒀ'));
        // A plain KA.
        assert_eq!(compose(Consonant::K, false, Vowel::A, false), Some('ᑲ'));
        // A combination with no glyph returns None.
        assert_eq!(compose(Consonant::Th, true, Vowel::Oy, true), None);
    }

    #[test]
    fn transliterate_sro_basic() {
        assert_eq!(transliterate_sro("pata"), "ᐸᑕ");
        assert_eq!(transliterate_sro("kwâ"), "ᒀ");
        assert_eq!(transliterate_sro("a"), "ᐊ");
        // A consonant with no vowel becomes its coda glyph.
        assert_eq!(transliterate_sro("pîn"), "ᐲᓐ");
        // Spaces pass through, preserving word boundaries.
        assert_eq!(transliterate_sro("pa ta"), "ᐸ ᑕ");
    }

    #[test]
    fn sro_round_trips_through_syllabics() {
        // romanize_text ∘ transliterate_sro is identity on clean SRO input.
        for word in ["pata", "kwâ", "sôma", "pîn", "kâ", "wapato"] {
            let syllabic = transliterate_sro(word);
            assert_eq!(romanize_text(&syllabic), word, "round-trip for {word}");
        }
    }

    #[test]
    fn vowel_features_are_ipa_grounded() {
        assert_eq!(Vowel::I.height(), Height::High);
        assert_eq!(Vowel::A.height(), Height::Low);
        assert_eq!(Vowel::I.backness(), Backness::Front);
        assert_eq!(Vowel::O.backness(), Backness::Back);
        assert!(Vowel::O.rounded(), "o is the rounded vowel");
        assert!(!Vowel::A.rounded());
        assert!(Vowel::Ai.is_diphthong());
        assert!(!Vowel::E.is_diphthong());
    }

    #[test]
    fn every_glyph_gets_a_role() {
        // No panic, and the disjoint roles partition the table (introspection).
        use crate::cree_syllabics::{UCAS_EXTENDED, UCAS_MAIN};
        for entry in UCAS_MAIN.iter().chain(UCAS_EXTENDED.iter()) {
            let _ = role_of(entry.2); // total function — never panics on a real name
        }
    }

    #[test]
    fn core_cree_grid_is_fully_covered() {
        // THE completeness oracle: every core-Cree glyph classifies to a real role
        // (Syllable / Standalone / Final / Punctuation) — zero Unknown in the core.
        use crate::cree_syllabics::{UCAS_EXTENDED, UCAS_MAIN};
        let mut leaks = Vec::new();
        for entry in UCAS_MAIN.iter().chain(UCAS_EXTENDED.iter()) {
            if is_core_cree(entry.2) && role_of(entry.2) == Role::Unknown {
                leaks.push(entry.2);
            }
        }
        assert!(leaks.is_empty(), "core-Cree glyphs must all classify; leaked: {leaks:?}");

        let (classified, total, tail) = coverage();
        assert_eq!(classified, total, "core coverage must be 100%");
        assert!(total >= 250, "the core grid is large (got {total})");
        // The exotic tail is a real, non-empty, NAMED boundary — not padded away.
        assert!(tail > 0, "Carrier/Dene/Blackfoot exotics are the honest boundary");
    }

    #[test]
    fn all_core_cree_syllabics_have_known_roles() {
        for entry in crate::cree_syllabics::UCAS_MAIN
            .iter()
            .chain(crate::cree_syllabics::UCAS_EXTENDED.iter())
        {
            if is_core_cree(entry.2) {
                assert_ne!(
                    role_of(entry.2),
                    Role::Unknown,
                    "Core Cree syllabic '{}' has an unknown role",
                    entry.1
                );
            }
        }
    }
}
