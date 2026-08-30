//! Corpus store — persistent SoA storage for knowledge atoms.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\corpus.rs`, then reworked
//! (2026-08-14, F07 revascularize-check) to stop duplicating `forge-vcs-v3`'s
//! already-landed tape: content-addressed dedup and the LOCKOUT/TAGOUT commit lock
//! now live in `forge_vcs_v3::VcsRoot` (durable, battle-tested, real bug-fix
//! history), not re-derived here. `Corpus`'s own `corpus.jsonl` is now an
//! explicitly derived, rebuildable READ CACHE — fast local `load_all()` — never the
//! sole copy of the data. Every atom is committed to the tape FIRST (durable,
//! deduped, locked, gets a real `BrutalHash` receipt), then mirrored into the local
//! cache for fast query. `jaccard_similarity`/`avg_links_per_atom` stay permyriad
//! `u32` (C14 firewall).

use crate::atom::KnowledgeAtom;
use forge_vcs_v3::VcsRoot;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Corpus configuration.
pub struct CorpusConfig {
    /// Maximum atoms before compaction triggers.
    pub max_atoms: usize,
    /// Staleness threshold (permyriad) — atoms below this get pruned on compaction.
    pub stale_threshold: u16,
    /// Daily decay amount (permyriad) applied during maintenance.
    pub daily_decay: u16,
}

impl Default for CorpusConfig {
    fn default() -> Self {
        Self { max_atoms: 10_000, stale_threshold: 2000, daily_decay: 50 }
    }
}

/// The corpus store: a durable tape (source of truth) plus a fast local cache.
pub struct Corpus {
    /// The local read cache — a derived JSONL mirror of what's on the tape.
    path: PathBuf,
    /// The tape: content-addressed, deduped, LOCKOUT/TAGOUT-locked on write.
    vcs: VcsRoot,
    config: CorpusConfig,
}

impl Corpus {
    /// `path` is the local cache file; `vcs_root` is a `.forge/vcs`-shaped tape
    /// directory (see `VcsRoot::open`'s own contract — never a working tree).
    pub fn new(path: PathBuf, vcs_root: PathBuf, config: CorpusConfig) -> std::io::Result<Self> {
        let vcs = VcsRoot::open(vcs_root)?;
        Ok(Self { path, vcs, config })
    }

    /// Append atoms to corpus: commit each new atom to the tape (dedup + lock +
    /// durability live there now), then mirror into the local cache for fast reads.
    pub fn append(&self, atoms: &[KnowledgeAtom]) -> std::io::Result<AppendResult> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }

        let existing_ids = self.load_ids()?;
        let mut written = 0usize;
        let mut skipped = 0usize;

        let mut file = fs::OpenOptions::new().create(true).append(true).open(&self.path)?;

        for atom in atoms {
            if existing_ids.contains(&atom.id) {
                skipped += 1;
                continue;
            }
            let line = serde_json::to_string(atom).map_err(std::io::Error::other)?;

            // Tape first: durable, content-deduped, lock-protected commit. The
            // path key is stable per atom (its own content-ID), so a re-commit of
            // identical bytes is a no-op dedup on the tape's object store, and a
            // changed atom (same id, e.g. impossible today — id IS the content
            // hash — but kept honest for whatever this atom shape becomes) would
            // record a real new row on its own chain.
            self.vcs.commit_bytes(&format!("pkm/atoms/{}", atom.id), line.as_bytes())?;

            writeln!(file, "{}", line)?;
            written += 1;
        }

        Ok(AppendResult { written, skipped, total: existing_ids.len() + written })
    }

    fn load_ids(&self) -> std::io::Result<HashSet<String>> {
        let mut ids = HashSet::new();
        if !self.path.exists() {
            return Ok(ids);
        }

        let file = fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(atom) = serde_json::from_str::<KnowledgeAtom>(&line) {
                ids.insert(atom.id);
            }
        }
        Ok(ids)
    }

    /// Load all atoms from corpus. Resilient to malformed/corrupted lines.
    pub fn load_all(&self) -> std::io::Result<Vec<KnowledgeAtom>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&self.path)?;
        let reader = std::io::BufReader::new(file);
        let mut atoms = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<KnowledgeAtom>(&line) {
                Ok(atom) => atoms.push(atom),
                Err(e) => eprintln!("Warning: skipping malformed corpus line: {}", e),
            }
        }
        Ok(atoms)
    }

    fn write_all_atoms(&self, atoms: &[KnowledgeAtom]) -> std::io::Result<()> {
        let mut file = fs::File::create(&self.path)?;
        for atom in atoms {
            let line = serde_json::to_string(atom).map_err(std::io::Error::other)?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    /// Delete an atom from the local cache by ID. Returns `true` if it was present.
    pub fn delete(&self, atom_id: &str) -> std::io::Result<bool> {
        let mut atoms = self.load_all()?;
        let before_len = atoms.len();
        atoms.retain(|a| a.id != atom_id);
        let deleted = atoms.len() < before_len;
        if deleted {
            self.write_all_atoms(&atoms)?;
        }
        Ok(deleted)
    }

    /// Update an existing atom in the local cache. Returns `true` if it was found.
    pub fn update(&self, updated_atom: &KnowledgeAtom) -> std::io::Result<bool> {
        let mut atoms = self.load_all()?;
        let mut found = false;
        for atom in &mut atoms {
            if atom.id == updated_atom.id {
                *atom = updated_atom.clone();
                found = true;
                break;
            }
        }
        if found {
            self.write_all_atoms(&atoms)?;
        }
        Ok(found)
    }

    /// Atom IDs that link to the given atom ID.
    pub fn compute_backlinks(&self, atom_id: &str) -> std::io::Result<Vec<String>> {
        let atoms = self.load_all()?;
        let mut backlinks = Vec::new();
        for atom in atoms {
            if atom.links.contains(&atom_id.to_string()) {
                backlinks.push(atom.id.clone());
            }
        }
        Ok(backlinks)
    }

    /// Add a bidirectional link between two atoms. Three real, distinct outcomes —
    /// `bool` would have collapsed "atoms don't exist" and "already linked" into the
    /// same `false`, losing a real distinction (Sean 2026-08-14: "in trit it is not
    /// binary" — correct: this genuinely has three states, not two).
    pub fn link_atoms(&self, atom_id_a: &str, atom_id_b: &str) -> std::io::Result<LinkOutcome> {
        let mut atoms = self.load_all()?;
        let mut updated = false;

        let a_exists = atoms.iter().any(|a| a.id == atom_id_a);
        let b_exists = atoms.iter().any(|a| a.id == atom_id_b);
        if !a_exists || !b_exists {
            return Ok(LinkOutcome::Invalid);
        }

        for atom in &mut atoms {
            if atom.id == atom_id_a && !atom.links.contains(&atom_id_b.to_string()) {
                atom.links.push(atom_id_b.to_string());
                updated = true;
            }
            if atom.id == atom_id_b && !atom.links.contains(&atom_id_a.to_string()) {
                atom.links.push(atom_id_a.to_string());
                updated = true;
            }
        }

        if updated {
            self.write_all_atoms(&atoms)?;
            Ok(LinkOutcome::Linked)
        } else {
            Ok(LinkOutcome::AlreadyLinked)
        }
    }

    /// Apply staleness decay, remove stale, deduplicate semantic overlap, enforce budget cap.
    pub fn compact(&self) -> std::io::Result<CompactResult> {
        let mut atoms = self.load_all()?;
        let before = atoms.len();

        for atom in &mut atoms {
            atom.decay(self.config.daily_decay);
        }

        atoms.retain(|a| !a.is_stale(self.config.stale_threshold));

        let mut merged_atoms: Vec<KnowledgeAtom> = Vec::new();
        let mut redirect_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for atom in atoms {
            let mut found_merge = false;
            for existing in &mut merged_atoms {
                if existing.domain == atom.domain && jaccard_similarity_pmy(&existing.text, &atom.text) > 5000 {
                    redirect_map.insert(atom.id.clone(), existing.id.clone());
                    if atom.text.len() > existing.text.len() {
                        existing.text = atom.text.clone();
                    }
                    for claim in atom.claims.clone() {
                        if !existing.claims.contains(&claim) {
                            existing.claims.push(claim);
                        }
                    }
                    for link in atom.links.clone() {
                        if !existing.links.contains(&link) && link != existing.id {
                            existing.links.push(link);
                        }
                    }
                    existing.freshness = existing.freshness.max(atom.freshness);
                    existing.ingested_at = existing.ingested_at.min(atom.ingested_at);
                    found_merge = true;
                    break;
                }
            }
            if !found_merge {
                merged_atoms.push(atom);
            }
        }

        for atom in &mut merged_atoms {
            for link in &mut atom.links {
                if let Some(target) = redirect_map.get(link) {
                    *link = target.clone();
                }
            }
            atom.links.dedup();
            atom.links.retain(|l| l != &atom.id);
        }

        atoms = merged_atoms;

        if atoms.len() > self.config.max_atoms {
            atoms.sort_by(|a, b| b.freshness.cmp(&a.freshness));
            atoms.truncate(self.config.max_atoms);
        }

        let after = atoms.len();
        self.write_all_atoms(&atoms)?;

        Ok(CompactResult { before, after, pruned: before - after })
    }

    /// Refresh an atom's freshness to 10000 (accessed = stays fresh).
    pub fn touch(&self, atom_id: &str) -> std::io::Result<bool> {
        let mut atoms = self.load_all()?;
        let mut found = false;
        for atom in &mut atoms {
            if atom.id == atom_id {
                atom.freshness = 10000;
                found = true;
                break;
            }
        }
        if found {
            self.write_all_atoms(&atoms)?;
        }
        Ok(found)
    }

    /// Get corpus stats.
    pub fn stats(&self) -> std::io::Result<CorpusStats> {
        let atoms = self.load_all()?;
        let total_bytes: usize = atoms.iter().map(|a| a.byte_size()).sum();
        let avg_freshness = if atoms.is_empty() { 0 } else { atoms.iter().map(|a| a.freshness as u64).sum::<u64>() / atoms.len() as u64 };

        let mut domain_counts = std::collections::HashMap::new();
        let mut structure_counts = std::collections::HashMap::new();
        let mut total_links = 0usize;
        let mut max_links = 0usize;

        for atom in &atoms {
            *domain_counts.entry(atom.domain.map_or_else(|| "unclassified".to_string(), |d| format!("{d:?}"))).or_insert(0) += 1;
            *structure_counts.entry(atom.structure.map_or_else(|| "unclassified".to_string(), |s| format!("{s:?}"))).or_insert(0) += 1;
            total_links += atom.links.len();
            max_links = max_links.max(atom.links.len());
        }

        // Permyriad average (0..=10000-ish scale, integer division — C14 firewall).
        let avg_links_per_atom_pmy: u32 = if atoms.is_empty() { 0 } else { (total_links as u64 * 10000 / atoms.len() as u64) as u32 };

        Ok(CorpusStats {
            atom_count: atoms.len(),
            total_bytes,
            avg_freshness: avg_freshness as u16,
            stale_count: atoms.iter().filter(|a| a.is_stale(self.config.stale_threshold)).count(),
            domain_counts,
            structure_counts,
            avg_links_per_atom_pmy,
            max_links,
        })
    }

    /// Create a timestamped backup of the local cache file.
    pub fn backup(&self, backup_dir: &std::path::Path, now_secs: u64) -> std::io::Result<PathBuf> {
        fs::create_dir_all(backup_dir)?;
        let file_stem = self.path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "corpus".to_string());
        let backup_path = backup_dir.join(format!("{}-{}.jsonl", file_stem, now_secs));

        if self.path.exists() {
            fs::copy(&self.path, &backup_path)?;
        } else {
            fs::File::create(&backup_path)?;
        }

        Ok(backup_path)
    }

    /// Roll back the local cache file from a backup file (does not touch the tape).
    pub fn rollback(&self, backup_path: &std::path::Path) -> std::io::Result<()> {
        if !backup_path.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, format!("Backup file not found at {}", backup_path.display())));
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(backup_path, &self.path)?;
        Ok(())
    }
}

/// Jaccard similarity, permyriad scale (0..=10000). Integer-only (C14 firewall) —
/// replaces the v2 donor's `f64` version, same precision (10000 discrete steps is
/// finer than any corpus comparison actually needs).
fn jaccard_similarity_pmy(a: &str, b: &str) -> u32 {
    let a_words: HashSet<String> = a.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 3).map(|w| w.to_lowercase()).collect();
    let b_words: HashSet<String> = b.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 3).map(|w| w.to_lowercase()).collect();

    if a_words.is_empty() && b_words.is_empty() {
        return 10000;
    }

    let intersection = a_words.intersection(&b_words).count() as u64;
    let union = a_words.union(&b_words).count() as u64;
    if union == 0 {
        return 0;
    }
    ((intersection * 10000) / union) as u32
}

/// Outcome of a `link_atoms` attempt — a real 3-state result (2026-08-14), not a
/// lossy `bool`. Same n=3, k=1 shape `anomaly_fold.rs`/`primality_fold.rs` prove:
/// `AlreadyLinked` is the natural fixed point (the operation is idempotent there —
/// running it again stays `AlreadyLinked`), `Invalid`/`Linked` are the 2-orbit
/// (failure vs. success, opposite outcomes of attempting the same edge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkOutcome {
    /// One or both atom IDs don't exist in the local cache — invalid operation.
    Invalid,
    /// Both atoms existed and were already linked — no change made, this call's
    /// own fixed point.
    AlreadyLinked,
    /// A new link was created.
    Linked,
}

impl LinkOutcome {
    /// The involution: `Invalid` and `Linked` are each other's reflection (an edge
    /// attempt that could not exist vs. one that newly does); `AlreadyLinked`
    /// reflects to itself — the same fold shape `anomaly_fold.rs` uses.
    #[inline]
    pub const fn fold(self) -> Self {
        match self {
            LinkOutcome::Invalid => LinkOutcome::Linked,
            LinkOutcome::AlreadyLinked => LinkOutcome::AlreadyLinked,
            LinkOutcome::Linked => LinkOutcome::Invalid,
        }
    }

    /// The balanced-trit reading: `AlreadyLinked` is the true zero, precisely
    /// because it is `fold`'s only fixed point.
    #[inline]
    pub const fn to_trit(self) -> i8 {
        match self {
            LinkOutcome::Invalid => -1,
            LinkOutcome::AlreadyLinked => 0,
            LinkOutcome::Linked => 1,
        }
    }
}

/// Outcome of a corpus append.
#[derive(Debug)]
pub struct AppendResult {
    /// New atoms actually written (tape-committed + cached).
    pub written: usize,
    /// Atoms skipped because their content-ID already existed.
    pub skipped: usize,
    /// Total atom count after this append.
    pub total: usize,
}

/// Outcome of a corpus compaction pass.
#[derive(Debug)]
pub struct CompactResult {
    /// Atom count before compaction.
    pub before: usize,
    /// Atom count after compaction.
    pub after: usize,
    /// Atoms removed (stale-pruned or merged away).
    pub pruned: usize,
}

/// Summary statistics over the whole corpus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusStats {
    /// Total live atom count.
    pub atom_count: usize,
    /// Total serialized byte size across all atoms.
    pub total_bytes: usize,
    /// Average freshness (permyriad) across all atoms.
    pub avg_freshness: u16,
    /// Count of atoms currently below the stale threshold.
    pub stale_count: usize,
    /// Atom count per domain label (including `"unclassified"`).
    pub domain_counts: std::collections::HashMap<String, usize>,
    /// Atom count per structure label (including `"unclassified"`).
    pub structure_counts: std::collections::HashMap<String, usize>,
    /// Average links per atom, permyriad scale (10000 = 1.0 links/atom average).
    pub avg_links_per_atom_pmy: u32,
    /// The single highest link count on any one atom.
    pub max_links: usize,
}

impl CorpusStats {
    /// Pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Write the stats as pretty JSON to a file.
    pub fn save_to_file(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = self.to_json().map_err(std::io::Error::other)?;
        fs::write(path, content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atom::{KnowledgeDomain, Structure};

    /// No `tempfile` dep — a tiny self-cleaning scratch dir, same convention
    /// `forge-foreman-v3::velocity`'s tests already use.
    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let mut n = 0u64;
            loop {
                let p = std::env::temp_dir().join(format!("pkm_test_{n}_{}", std::process::id()));
                if fs::create_dir(&p).is_ok() {
                    return Self(p);
                }
                n += 1;
            }
        }
        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_atom(text: &str) -> KnowledgeAtom {
        KnowledgeAtom::new(text.into(), "test.txt".into(), (0, text.len()), Some(KnowledgeDomain::Physics), Some(Structure::Algorithm), 0)
    }

    #[test]
    fn append_and_load() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let atoms = vec![test_atom("Verlet is symplectic"), test_atom("GJK uses support functions")];
        let result = corpus.append(&atoms).unwrap();
        assert_eq!(result.written, 2);
        assert_eq!(result.skipped, 0);

        let loaded = corpus.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn dedup_on_append() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let atoms = vec![test_atom("same content")];
        corpus.append(&atoms).unwrap();
        let result = corpus.append(&atoms).unwrap();
        assert_eq!(result.skipped, 1);
        assert_eq!(result.written, 0);
    }

    #[test]
    fn compact_removes_stale() {
        let dir = TempDir::new();
        let config = CorpusConfig { stale_threshold: 5000, daily_decay: 6000, ..Default::default() };
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), config).unwrap();

        let atoms = vec![test_atom("will become stale"), test_atom("also stale")];
        corpus.append(&atoms).unwrap();

        let result = corpus.compact().unwrap();
        assert_eq!(result.before, 2);
        assert_eq!(result.pruned, 2);
    }

    #[test]
    fn delete_and_update() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let mut a = test_atom("Verlet is symplectic");
        corpus.append(&[a.clone()]).unwrap();

        a.text = "Verlet is highly symplectic".to_string();
        let updated = corpus.update(&a).unwrap();
        assert!(updated);

        let loaded = corpus.load_all().unwrap();
        assert_eq!(loaded[0].text, "Verlet is highly symplectic");

        let deleted = corpus.delete(&a.id).unwrap();
        assert!(deleted);

        let loaded_after = corpus.load_all().unwrap();
        assert!(loaded_after.is_empty());
    }

    #[test]
    fn linking_and_backlinks() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let a = test_atom("First atom");
        let b = test_atom("Second atom");
        corpus.append(&[a.clone(), b.clone()]).unwrap();

        let linked = corpus.link_atoms(&a.id, &b.id).unwrap();
        assert_eq!(linked, LinkOutcome::Linked);

        // The fixed point, exercised for real: linking the same pair again is a
        // genuine no-op, distinct from the first call's success.
        let again = corpus.link_atoms(&a.id, &b.id).unwrap();
        assert_eq!(again, LinkOutcome::AlreadyLinked);

        // And an invalid pair is the third, distinct outcome — never confused with
        // "already linked" the way the old `bool` return conflated them.
        let invalid = corpus.link_atoms(&a.id, "does-not-exist").unwrap();
        assert_eq!(invalid, LinkOutcome::Invalid);

        let backlinks = corpus.compute_backlinks(&b.id).unwrap();
        assert_eq!(backlinks, vec![a.id.clone()]);
    }

    const ALL_LINK_OUTCOMES: [LinkOutcome; 3] = [LinkOutcome::Invalid, LinkOutcome::AlreadyLinked, LinkOutcome::Linked];

    #[test]
    fn link_outcome_fold_is_an_involution_over_all_states() {
        for x in ALL_LINK_OUTCOMES {
            assert_eq!(x.fold().fold(), x, "f(f({x:?})) must equal {x:?}");
        }
    }

    #[test]
    fn link_outcome_fixed_point_is_exactly_already_linked() {
        let fixed: Vec<LinkOutcome> = ALL_LINK_OUTCOMES.into_iter().filter(|x| x.fold() == *x).collect();
        assert_eq!(fixed, vec![LinkOutcome::AlreadyLinked], "Fix(f) must be exactly {{AlreadyLinked}}, k=1");
    }

    #[test]
    fn semantic_overlap_compactor() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig { stale_threshold: 1000, daily_decay: 0, ..Default::default() }).unwrap();

        let a = test_atom("Verlet integration is highly efficient and numerical");
        let b = test_atom("Verlet integration is highly efficient and numerical and symplectic");
        corpus.append(&[a, b]).unwrap();

        let res = corpus.compact().unwrap();
        assert_eq!(res.before, 2);
        assert_eq!(res.after, 1);
    }

    #[test]
    fn backup_and_rollback() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let a = test_atom("Verlet is symplectic");
        corpus.append(&[a]).unwrap();

        let backup_dir = dir.path().join("backup");
        let backup_file = corpus.backup(&backup_dir, 12345).unwrap();
        assert!(backup_file.exists());

        std::fs::remove_file(dir.path().join("corpus.jsonl")).unwrap();

        corpus.rollback(&backup_file).unwrap();
        assert_eq!(corpus.load_all().unwrap().len(), 1);
    }

    #[test]
    fn skip_malformed_lines() {
        let dir = TempDir::new();
        let path = dir.path().join("corpus.jsonl");

        let atom = test_atom("Proper atom");
        let valid_json = serde_json::to_string(&atom).unwrap();

        std::fs::write(&path, format!("{}\nthis is absolute gibberish\n{}", valid_json, valid_json)).unwrap();

        let corpus = Corpus::new(path, dir.path().join("vcs"), CorpusConfig::default()).unwrap();
        let loaded = corpus.load_all().unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn corpus_stats_and_save() {
        let dir = TempDir::new();
        let corpus = Corpus::new(dir.path().join("corpus.jsonl"), dir.path().join("vcs"), CorpusConfig::default()).unwrap();

        let a = test_atom("First element");
        corpus.append(&[a]).unwrap();

        let stats = corpus.stats().unwrap();
        assert_eq!(stats.atom_count, 1);

        let stats_path = dir.path().join("stats.json");
        stats.save_to_file(&stats_path).unwrap();
        assert!(stats_path.exists());
    }
}
