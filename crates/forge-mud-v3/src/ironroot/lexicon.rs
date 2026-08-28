//! The prompt corpus and the dictionary derived from it. Three registers, one
//! per trit, named for Dante's three cries: `ah!` the gasp, `ahaha!` the turn,
//! `ahhh!` the release. Feeds [`super::trit_grammar::TritWordBanks`].

use super::dialogue::WordClass;
use super::trit_grammar::TritWordBanks;
use std::fmt;

/// Longest a single bank word may run, in bytes. A bank holds words, not
/// sentences; anything longer belongs in the template.
const MAX_WORD_BYTES: usize = 48;

/// Typed refusal from the corpus or dictionary readers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexRefusal {
    /// A header lacks a column the reader requires.
    ColumnMissing {
        /// Required column name.
        column: String,
    },
    /// A row has a different field count than its header.
    RowRagged {
        /// 1-based line number.
        line: usize,
        /// Field count the header declares.
        want: usize,
        /// Field count this row carries.
        got: usize,
    },
    /// A word class field names no known class.
    UnknownClass {
        /// 1-based line number.
        line: usize,
        /// The unrecognized text.
        found: String,
    },
    /// A register field names no known register.
    UnknownCry {
        /// 1-based line number.
        line: usize,
        /// The unrecognized text.
        found: String,
    },
    /// A bank word is empty, over-long, or carries a template slot token.
    UnusableWord {
        /// 1-based line number.
        line: usize,
        /// The offending word.
        word: String,
        /// What is wrong with it.
        why: &'static str,
    },
    /// The same word was entered twice for one class and register.
    DuplicateWord {
        /// 1-based line number of the repeat.
        line: usize,
        /// The repeated word.
        word: String,
    },
    /// A corpus entry names a digest already claimed by another entry.
    DuplicateDigest {
        /// 1-based line number of the repeat.
        line: usize,
        /// The repeated digest.
        sha256: String,
    },
}

impl fmt::Display for LexRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexRefusal::ColumnMissing { column } => {
                write!(f, "header is missing required column '{column}'")
            }
            LexRefusal::RowRagged { line, want, got } => {
                write!(f, "line {line} has {got} fields, header declares {want}")
            }
            LexRefusal::UnknownClass { line, found } => {
                write!(f, "line {line}: '{found}' is not adj, noun, or verb")
            }
            LexRefusal::UnknownCry { line, found } => {
                write!(f, "line {line}: '{found}' is not ah, ahaha, or ahhh")
            }
            LexRefusal::UnusableWord { line, word, why } => {
                write!(f, "line {line}: '{word}' {why}")
            }
            LexRefusal::DuplicateWord { line, word } => {
                write!(f, "line {line}: '{word}' already banked for this class and register")
            }
            LexRefusal::DuplicateDigest { line, sha256 } => {
                write!(f, "line {line}: digest {sha256} appears twice — the corpus is not deduplicated")
            }
        }
    }
}

/// Dante's three cries, one per trit. The register a word belongs to is the
/// register it lands the reader in, not the mood of the thing described.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cry {
    /// `ah!` — the gasp. Raw, sensory, close to the skin.
    Evocative,
    /// `ahaha!` — the turn. The thing becomes another thing.
    Transformative,
    /// `ahhh!` — the release. The thing is understood.
    Insightful,
}

impl Cry {
    /// Every register, in trit order.
    pub const ALL: [Cry; 3] = [Cry::Evocative, Cry::Transformative, Cry::Insightful];

    /// The trit this register grades to.
    pub fn trit(self) -> i8 {
        match self {
            Cry::Evocative => -1,
            Cry::Transformative => 0,
            Cry::Insightful => 1,
        }
    }

    /// The sound, as authored in a dictionary file.
    pub fn sound(self) -> &'static str {
        match self {
            Cry::Evocative => "ah",
            Cry::Transformative => "ahaha",
            Cry::Insightful => "ahhh",
        }
    }

    /// Parse a register from its cry.
    pub fn from_sound(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.sound() == text)
    }

    /// Index into a trit-partitioned bank triple.
    fn index(self) -> usize {
        (self.trit() + 1) as usize
    }
}

/// Parse a word class from its dictionary spelling.
fn class_from_text(text: &str) -> Option<WordClass> {
    match text {
        "adj" => Some(WordClass::Adjective),
        "noun" => Some(WordClass::Noun),
        "verb" => Some(WordClass::Verb),
        _ => None,
    }
}

/// One row of `CORPUS.tsv` — a source file the dictionary was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusEntry {
    /// Category directory the file sits in.
    pub category: String,
    /// Bare filename.
    pub filename: String,
    /// Content digest, lowercase hex.
    pub sha256: String,
    /// File length in bytes.
    pub bytes: u64,
}

impl CorpusEntry {
    /// Path relative to the corpus root.
    pub fn rel_path(&self) -> String {
        format!("{}/{}", self.category, self.filename)
    }
}

/// Locate a column in a tab-separated header.
fn column_index(header: &[&str], column: &str) -> Result<usize, LexRefusal> {
    header
        .iter()
        .position(|h| *h == column)
        .ok_or_else(|| LexRefusal::ColumnMissing { column: column.to_string() })
}

/// Split a tab-separated body into numbered, non-empty rows.
fn rows(tsv: &str) -> (Vec<&str>, Vec<(usize, Vec<&str>)>) {
    let mut lines = tsv.lines();
    let header: Vec<&str> = lines.next().unwrap_or_default().split('\t').map(str::trim).collect();
    let body = lines
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i + 2, l.split('\t').collect::<Vec<&str>>()))
        .collect();
    (header, body)
}

/// Read `CORPUS.tsv`. Every digest must be unique — a repeat means the
/// round-up did not deduplicate and the dictionary would be weighted by
/// however many copies of a tree happened to be on disk.
pub fn read_corpus(tsv: &str) -> Result<Vec<CorpusEntry>, LexRefusal> {
    let (header, body) = rows(tsv);
    let (i_cat, i_file) = (column_index(&header, "category")?, column_index(&header, "filename")?);
    let (i_sha, i_bytes) = (column_index(&header, "sha256")?, column_index(&header, "bytes")?);

    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for (line, f) in body {
        if f.len() != header.len() {
            return Err(LexRefusal::RowRagged { line, want: header.len(), got: f.len() });
        }
        let sha256 = f[i_sha].trim().to_string();
        if seen.contains(&sha256) {
            return Err(LexRefusal::DuplicateDigest { line, sha256 });
        }
        seen.push(sha256.clone());
        out.push(CorpusEntry {
            category: f[i_cat].trim().to_string(),
            filename: f[i_file].trim().to_string(),
            sha256,
            bytes: f[i_bytes].trim().parse().unwrap_or(0),
        });
    }
    Ok(out)
}

/// Why a word cannot enter a bank, or None if it may.
fn word_fault(word: &str) -> Option<&'static str> {
    if word.is_empty() {
        return Some("is empty");
    }
    if word.len() > MAX_WORD_BYTES {
        return Some("is longer than a bank word may run");
    }
    if WordClass::ALL.iter().any(|c| word.contains(c.slot())) {
        return Some("carries a template slot token and would recurse");
    }
    if word.contains('\t') {
        return Some("carries a tab");
    }
    None
}

/// Read a dictionary into trit-partitioned banks. Columns: `class`,
/// `register`, `word`.
pub fn read_dictionary(tsv: &str) -> Result<TritWordBanks, LexRefusal> {
    let mut banks = TritWordBanks::default();
    for (offset, row) in parse_dictionary(tsv)?.into_iter().enumerate() {
        if !bank_push(&mut banks, &row) {
            return Err(LexRefusal::DuplicateWord { line: offset + 2, word: row.word });
        }
    }
    Ok(banks)
}

/// One validated dictionary row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictRow {
    /// Which slot the word fills.
    pub class: WordClass,
    /// Which trit it grades to.
    pub cry: Cry,
    /// The word itself.
    pub word: String,
}

/// Validate every row's shape without judging repeats.
pub fn parse_dictionary(tsv: &str) -> Result<Vec<DictRow>, LexRefusal> {
    let (header, body) = rows(tsv);
    let i_class = column_index(&header, "class")?;
    let i_cry = column_index(&header, "register")?;
    let i_word = column_index(&header, "word")?;

    let mut out = Vec::new();
    for (line, f) in body {
        if f.len() != header.len() {
            return Err(LexRefusal::RowRagged { line, want: header.len(), got: f.len() });
        }
        let class = class_from_text(f[i_class].trim())
            .ok_or_else(|| LexRefusal::UnknownClass { line, found: f[i_class].trim().to_string() })?;
        let cry = Cry::from_sound(f[i_cry].trim())
            .ok_or_else(|| LexRefusal::UnknownCry { line, found: f[i_cry].trim().to_string() })?;
        let word = f[i_word].trim().to_string();
        if let Some(why) = word_fault(&word) {
            return Err(LexRefusal::UnusableWord { line, word, why });
        }
        out.push(DictRow { class, cry, word });
    }
    Ok(out)
}

/// Push a row into its bank, reporting whether the bank did not already hold it.
fn bank_push(banks: &mut TritWordBanks, row: &DictRow) -> bool {
    let slot = &mut match row.class {
        WordClass::Adjective => &mut banks.adjectives,
        WordClass::Noun => &mut banks.nouns,
        WordClass::Verb => &mut banks.verbs,
    }[row.cry.index()];
    if slot.contains(&row.word) {
        return false;
    }
    slot.push(row.word.clone());
    true
}

/// What merging several dictionaries produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergeReport {
    /// Rows read across every source.
    pub rows_in: usize,
    /// Distinct words banked.
    pub words_out: usize,
    /// Rows dropped because another source already banked that word.
    pub overlaps: usize,
}

/// Union several dictionaries. A word two sources both chose for the same class
/// and cry is banked once and counted as an overlap — independent agreement,
/// which is why this unions where [`read_dictionary`] refuses.
pub fn merge_dictionaries(sources: &[String]) -> Result<(TritWordBanks, MergeReport), LexRefusal> {
    let mut banks = TritWordBanks::default();
    let mut report = MergeReport::default();
    for tsv in sources {
        for row in parse_dictionary(tsv)? {
            report.rows_in += 1;
            if bank_push(&mut banks, &row) {
                report.words_out += 1;
            } else {
                report.overlaps += 1;
            }
        }
    }
    Ok((banks, report))
}

/// Render banks back to tab-separated form, classes then cries in stable order.
pub fn write_dictionary(banks: &TritWordBanks) -> String {
    let mut out = String::from("class\tregister\tword");
    for class in WordClass::ALL {
        let name = match class {
            WordClass::Adjective => "adj",
            WordClass::Noun => "noun",
            WordClass::Verb => "verb",
        };
        for cry in Cry::ALL {
            let mut words: Vec<&String> = banks.bank(class, cry.trit()).iter().collect();
            words.sort_unstable();
            for word in words {
                out.push('\n');
                out.push_str(name);
                out.push('\t');
                out.push_str(cry.sound());
                out.push('\t');
                out.push_str(word);
            }
        }
    }
    out
}

/// Classes and registers with no words, as `(class, register)` pairs. An empty
/// bank leaves its slot token visible in the rendered line
/// (`super::trit_grammar::fill_template_trit`), so a gap is loud, not silent —
/// this reports it before a player ever sees it.
pub fn empty_banks(banks: &TritWordBanks) -> Vec<(WordClass, Cry)> {
    let mut out = Vec::new();
    for class in WordClass::ALL {
        for register in Cry::ALL {
            if banks.bank(class, register.trit()).is_empty() {
                out.push((class, register));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DICT_HEADER: &str = "class\tregister\tword";

    fn dict(rows: &[&str]) -> String {
        let mut s = String::from(DICT_HEADER);
        for r in rows {
            s.push('\n');
            s.push_str(r);
        }
        s
    }

    #[test]
    fn the_three_cries_grade_to_the_three_trits() {
        assert_eq!(Cry::Evocative.trit(), -1);
        assert_eq!(Cry::Transformative.trit(), 0);
        assert_eq!(Cry::Insightful.trit(), 1);
        for r in Cry::ALL {
            assert_eq!(Cry::from_sound(r.sound()), Some(r), "every cry round-trips");
        }
        assert_eq!(Cry::from_sound("aaah"), None);
    }

    #[test]
    fn a_dictionary_lands_each_word_in_its_own_trit_bank() {
        let tsv = dict(&[
            "adj\tah\tblistered",
            "adj\tahhh\tlucent",
            "noun\tah\trot",
            "verb\tahaha\tsloughs",
        ]);
        let banks = read_dictionary(&tsv).expect("reads");
        assert_eq!(banks.bank(WordClass::Adjective, -1), ["blistered"]);
        assert_eq!(banks.bank(WordClass::Adjective, 1), ["lucent"]);
        assert_eq!(banks.bank(WordClass::Noun, -1), ["rot"]);
        assert_eq!(banks.bank(WordClass::Verb, 0), ["sloughs"]);
        assert!(banks.bank(WordClass::Adjective, 0).is_empty());
    }

    #[test]
    fn an_unknown_class_or_register_refuses_with_its_line() {
        assert!(matches!(
            read_dictionary(&dict(&["adverb\tah\twetly"])),
            Err(LexRefusal::UnknownClass { line: 2, .. })
        ));
        assert!(matches!(
            read_dictionary(&dict(&["adj\taaah\twet"])),
            Err(LexRefusal::UnknownCry { line: 2, .. })
        ));
    }

    #[test]
    fn a_word_carrying_a_slot_token_refuses_before_it_can_recurse() {
        match read_dictionary(&dict(&["adj\tah\tdamp {adj}"])) {
            Err(LexRefusal::UnusableWord { why, .. }) => {
                assert!(why.contains("recurse"), "{why}");
            }
            other => panic!("expected an unusable-word refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_over_long_or_empty_word_refuses() {
        let long = "a".repeat(MAX_WORD_BYTES + 1);
        assert!(matches!(
            read_dictionary(&dict(&[&format!("noun\tah\t{long}")])),
            Err(LexRefusal::UnusableWord { .. })
        ));
        assert!(matches!(
            read_dictionary(&dict(&["noun\tah\t"])),
            Err(LexRefusal::UnusableWord { why: "is empty", .. })
        ));
    }

    #[test]
    fn the_same_word_twice_in_one_bank_refuses_but_across_registers_stands() {
        assert!(matches!(
            read_dictionary(&dict(&["adj\tah\twet", "adj\tah\twet"])),
            Err(LexRefusal::DuplicateWord { line: 3, .. })
        ));
        let banks = read_dictionary(&dict(&["adj\tah\twet", "adj\tahhh\twet"])).expect("reads");
        assert_eq!(banks.bank(WordClass::Adjective, -1), ["wet"]);
        assert_eq!(banks.bank(WordClass::Adjective, 1), ["wet"], "a word may sit in two registers");
    }

    #[test]
    fn empty_banks_names_every_gap() {
        let banks = read_dictionary(&dict(&["adj\tah\twet"])).expect("reads");
        let gaps = empty_banks(&banks);
        assert_eq!(gaps.len(), 8, "one of nine banks is filled");
        assert!(!gaps.contains(&(WordClass::Adjective, Cry::Evocative)));
        assert!(gaps.contains(&(WordClass::Verb, Cry::Insightful)));
    }

    #[test]
    fn merging_unions_where_reading_one_file_would_refuse() {
        let a = dict(&["adj\tah\tdamp", "noun\tah\tbrine"]);
        let b = dict(&["adj\tah\tdamp", "adj\tah\tfetid"]);
        assert!(
            matches!(read_dictionary(&dict(&["adj\tah\tdamp", "adj\tah\tdamp"])), Err(LexRefusal::DuplicateWord { .. })),
            "one authored file may not repeat itself"
        );
        let (banks, report) = merge_dictionaries(&[a, b]).expect("merges");
        assert_eq!(report.rows_in, 4);
        assert_eq!(report.words_out, 3);
        assert_eq!(report.overlaps, 1, "two sources agreeing on 'damp' is agreement, not a fault");
        assert_eq!(banks.bank(WordClass::Adjective, -1).len(), 2);
    }

    #[test]
    fn a_merge_carries_every_refusal_its_sources_would_raise() {
        let good = dict(&["adj\tah\tdamp"]);
        let bad = dict(&["adj\tnope\twet"]);
        assert!(matches!(
            merge_dictionaries(&[good, bad]),
            Err(LexRefusal::UnknownCry { .. })
        ));
    }

    #[test]
    fn a_written_dictionary_reads_back_to_the_same_banks() {
        let tsv = dict(&[
            "verb\tahhh\tsettles",
            "adj\tah\tdamp",
            "noun\tahaha\trust",
            "adj\tah\tfetid",
        ]);
        let banks = read_dictionary(&tsv).expect("reads");
        let round = read_dictionary(&write_dictionary(&banks)).expect("re-reads");
        for class in WordClass::ALL {
            for cry in Cry::ALL {
                let mut before: Vec<&String> = banks.bank(class, cry.trit()).iter().collect();
                let mut after: Vec<&String> = round.bank(class, cry.trit()).iter().collect();
                before.sort_unstable();
                after.sort_unstable();
                assert_eq!(before, after, "{class:?}/{} survives the round trip", cry.sound());
            }
        }
    }

    #[test]
    fn a_corpus_with_a_repeated_digest_refuses_as_undeduplicated() {
        let tsv = "category\tfilename\tsha256\tbytes\nitems\ta.txt\tdead\t10\nitems\tb.txt\tdead\t10";
        assert!(matches!(
            read_corpus(tsv),
            Err(LexRefusal::DuplicateDigest { line: 3, .. })
        ));
    }

    // The corpus rounded up 2026-08-25 from the ironroot-edict prompt trees.
    // 3,720 files on disk, exactly 3x duplicated, 1,240 distinct.
    #[test]
    fn the_landed_corpus_reads_and_is_deduplicated() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/prompt-corpus/CORPUS.tsv");
        let Ok(tsv) = std::fs::read_to_string(path) else {
            eprintln!("corpus absent at {path} — test skipped");
            return;
        };
        let entries = read_corpus(&tsv).expect("the corpus reads and is deduplicated");
        assert_eq!(entries.len(), 1240, "distinct prompt files");

        let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/prompt-corpus"));
        let missing: Vec<String> = entries
            .iter()
            .filter(|e| !root.join(e.category.as_str()).join(e.filename.as_str()).exists())
            .map(CorpusEntry::rel_path)
            .collect();
        assert!(missing.is_empty(), "manifest names files that are not on disk: {missing:?}");

        let mut categories: Vec<&str> = entries.iter().map(|e| e.category.as_str()).collect();
        categories.sort_unstable();
        categories.dedup();
        assert_eq!(categories.len(), 29, "category directories");
        eprintln!("lexicon: {} entries across {} categories", entries.len(), categories.len());
    }
}
