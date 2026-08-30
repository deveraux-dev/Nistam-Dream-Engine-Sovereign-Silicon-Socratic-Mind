//! Knowledge Atom — the fundamental unit of distilled knowledge.
//!
//! Ported from `F:\NewRepo\crates\forge-pkm\src\atom.rs`. `Domain` is confirmed
//! (2026-08-14) as the literal source of the 9 domain lanes `.forge/criticality.tsv`/
//! `domains.tsv` already use. Trit-native here (2026-08-14, requested): both `Domain`
//! (9 real states) and `Structure` (9 real states) are exactly the n=9=3^2 shape —
//! two balanced-trit lanes, same law `PARARITY.md` Corollary 2 proves for one lane,
//! composed twice (Proposition 2). `Unknown` is NOT a 10th packed state — absence
//! lives outside the trit lane (`Option<Domain>::None`), same R1 reachability
//! discipline `soul.rs::all_bytes_are_interior_trits` already uses: a packed byte's
//! `None` case is never a zeroed coordinate pretending to be data.
//!
//! Content-addressed ID reuses `forge_core_v3::soul::content_hash_fnv1a` (already
//! landed, deterministic, integer-only) instead of adding `sha2` fresh (L19 dep-grab).

use serde::{Deserialize, Serialize};

/// School A lens — domain expert classification. 9 real states, arity-3^2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeDomain {
    /// Verlet/RK4/collision/rollback/netcode — deterministic physics.
    Physics,
    /// Faust/DSP/spectral/harmonic/MIDI — audio synthesis and analysis.
    Audio,
    /// Shader/wgpu/GPU/pixel/VFX/camera — the render pipeline.
    Rendering,
    /// Creature/quest/NPC/dialogue/loot/zone/inventory — gameplay systems.
    GameSystems,
    /// Sieve/ECC/steganography/prime/sovereign-messaging.
    Sieve,
    /// Invention/SRED/session/memory/handoff/decision/roadmap — process knowledge.
    Lorekeeper,
    /// UI/HUD/panel/widget/accessibility/frontend.
    HumanInterface,
    /// Terrain/world/biome/ecological/weather.
    World,
    /// Cargo/crate/pipeline/tool/SDK/compiler/AST — build and language tooling.
    Techkeeper,
}

/// School B lens — structural knowledge-pattern classification. 9 real states, arity-3^2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Structure {
    /// A step-by-step procedure: calculate/iterate/formula/compute/evaluate.
    Algorithm,
    /// A layer/pipeline/crate/module/system-level design description.
    Architecture,
    /// A must/never/forbidden/mandate/enforce/reject rule.
    Constraint,
    /// A however/drawback/advantage/versus comparison of two approaches.
    TradeOff,
    /// An always/guarantee/deterministic/reproducible property.
    Invariant,
    /// A step 1/step 2/phase/stage sequence.
    Protocol,
    /// A design/approach/strategy/technique/method description.
    Pattern,
    /// An "is a"/"defined as"/"refers to"/"means" definition.
    Definition,
    /// A "compared to"/"unlike"/"in contrast"/"whereas" comparison.
    Comparison,
}

/// All 9 `KnowledgeDomain` variants, index-ordered — the same order `to_trits`/`from_trits` use.
const DOMAIN_ORDER: [KnowledgeDomain; 9] = [
    KnowledgeDomain::Physics, KnowledgeDomain::Audio, KnowledgeDomain::Rendering, KnowledgeDomain::GameSystems, KnowledgeDomain::Sieve,
    KnowledgeDomain::Lorekeeper, KnowledgeDomain::HumanInterface, KnowledgeDomain::World, KnowledgeDomain::Techkeeper,
];

/// All 9 `Structure` variants, index-ordered.
const STRUCTURE_ORDER: [Structure; 9] = [
    Structure::Algorithm, Structure::Architecture, Structure::Constraint, Structure::TradeOff,
    Structure::Invariant, Structure::Protocol, Structure::Pattern, Structure::Definition,
    Structure::Comparison,
];

/// Encode an index `0..9` as two balanced trits (`-1,0,1` each), base-3, matching
/// `atom::TritCell5D::from_trits`'s own Horner convention (least-significant digit first).
const fn idx_to_trits2(idx: u8) -> [i8; 2] {
    let lo = (idx % 3) as i8 - 1;
    let hi = (idx / 3) as i8 - 1;
    [lo, hi]
}

/// Decode two balanced trits back to an index `0..9`, or `None` if either digit is
/// outside `-1..=1` (corruption, not a tenth state).
const fn trits2_to_idx(t: [i8; 2]) -> Option<u8> {
    if t[0] < -1 || t[0] > 1 || t[1] < -1 || t[1] > 1 {
        return None;
    }
    Some(((t[1] + 1) as u8) * 3 + (t[0] + 1) as u8)
}

impl KnowledgeDomain {
    /// This variant's two balanced trits.
    pub const fn to_trits(self) -> [i8; 2] {
        idx_to_trits2(self as u8)
    }

    /// Decode two balanced trits into a `KnowledgeDomain`, or `None` for out-of-range digits.
    pub fn from_trits(t: [i8; 2]) -> Option<Self> {
        trits2_to_idx(t).map(|i| DOMAIN_ORDER[i as usize])
    }
}

impl Structure {
    /// This variant's two balanced trits.
    pub const fn to_trits(self) -> [i8; 2] {
        idx_to_trits2(self as u8)
    }

    /// Decode two balanced trits into a `Structure`, or `None` for out-of-range digits.
    pub fn from_trits(t: [i8; 2]) -> Option<Self> {
        trits2_to_idx(t).map(|i| STRUCTURE_ORDER[i as usize])
    }
}

/// A distilled knowledge atom — the master output of the 7-7-7 cascade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAtom {
    /// Content-addressed ID (fnv1a hash of canonical text, hex).
    pub id: String,
    /// The distilled claim or concept (max 2048 chars).
    pub text: String,
    /// Source file this was extracted from.
    pub source_file: String,
    /// Byte range in source.
    pub source_range: (usize, usize),
    /// School A classification: domain expert. `None` = genuinely unclassified.
    pub domain: Option<KnowledgeDomain>,
    /// School B classification: structural pattern. `None` = genuinely unclassified.
    pub structure: Option<Structure>,
    /// Extracted factual claims (atomic assertions).
    pub claims: Vec<String>,
    /// Cross-references to related atom IDs.
    pub links: Vec<String>,
    /// Unix timestamp of ingestion, caller-supplied (C14 firewall).
    pub ingested_at: i64,
    /// Staleness score (0-10000 permyriad). 10000 = fully fresh.
    pub freshness: u16,
    /// Topic hint from chunker.
    pub topic: Option<String>,
}

impl KnowledgeAtom {
    /// Generate content-addressed ID from text. Deterministic, integer-only.
    pub fn content_id(text: &str) -> String {
        format!("{:016x}", forge_core_v3::soul::content_hash_fnv1a(text.as_bytes()))
    }

    /// Create a new atom from distilled text.
    pub fn new(
        text: String,
        source_file: String,
        source_range: (usize, usize),
        domain: Option<KnowledgeDomain>,
        structure: Option<Structure>,
        now_unix: i64,
    ) -> Self {
        let id = Self::content_id(&text);
        Self {
            id,
            text,
            source_file,
            source_range,
            domain,
            structure,
            claims: Vec::new(),
            links: Vec::new(),
            ingested_at: now_unix,
            freshness: 10000,
            topic: None,
        }
    }

    /// Apply staleness decay. Reduces freshness by `decay_amount` (permyriad).
    pub fn decay(&mut self, decay_amount: u16) {
        self.freshness = self.freshness.saturating_sub(decay_amount);
    }

    /// Check if atom is stale (below threshold).
    pub fn is_stale(&self, threshold: u16) -> bool {
        self.freshness < threshold
    }

    /// Byte size estimate for budget tracking.
    pub fn byte_size(&self) -> usize {
        self.text.len() + self.source_file.len() + self.claims.iter().map(|c| c.len()).sum::<usize>() + 128
    }
}

/// Domain routing — maps keywords to domains. `None` = no keyword matched (genuinely
/// unclassified, not a lossy default). Same keyword table as the v2 donor.
pub fn classify_domain(text: &str) -> Option<KnowledgeDomain> {
    let lower = text.to_lowercase();
    let scores: &[(KnowledgeDomain, &[&str])] = &[
        (KnowledgeDomain::Physics, &["verlet", "rk4", "collision", "gjk", "minkowski", "integrat", "deterministic", "rollback", "netcode", "physics"]),
        (KnowledgeDomain::Audio, &["faust", "dsp", "audio", "spectral", "harmonic", "waveform", "midi", "psychoacoustic", "mixer"]),
        (KnowledgeDomain::Rendering, &["shader", "wgpu", "render", "gpu", "pixel", "vfx", "camera", "composit", "forge-hal"]),
        (KnowledgeDomain::GameSystems, &["creature", "quest", "npc", "dialogue", "loot", "zone", "biome", "spawn", "inventory"]),
        (KnowledgeDomain::Sieve, &["sieve", "ecc", "stego", "opaque", "prime", "encrypt", "sovereign-messaging"]),
        (KnowledgeDomain::Lorekeeper, &["invention", "sred", "session", "memory", "handoff", "decision", "roadmap"]),
        (KnowledgeDomain::HumanInterface, &["ui", "hud", "panel", "widget", "accessibility", "wcag", "frontend"]),
        (KnowledgeDomain::World, &["terrain", "world", "zone", "biome", "ecological", "weather"]),
        (KnowledgeDomain::Techkeeper, &["cargo", "crate", "pipeline", "tool", "sdk", "compiler", "ast", "vixiscript"]),
    ];

    let mut best: Option<KnowledgeDomain> = None;
    let mut best_score = 0usize;
    for (domain, keywords) in scores {
        let score = keywords.iter().filter(|kw| lower.contains(*kw)).count();
        if score > best_score {
            best_score = score;
            best = Some(*domain);
        }
    }
    best
}

/// Structure classification — what kind of knowledge pattern is this. `None` = no
/// keyword matched.
pub fn classify_structure(text: &str) -> Option<Structure> {
    let lower = text.to_lowercase();
    let scores: &[(Structure, &[&str])] = &[
        (Structure::Algorithm, &["algorithm", "step", "calculate", "iterate", "formula", "compute", "evaluate"]),
        (Structure::Architecture, &["architecture", "layer", "pipeline", "crate", "module", "system", "host"]),
        (Structure::Constraint, &["constraint", "must", "never", "forbidden", "mandate", "enforce", "reject"]),
        (Structure::TradeOff, &["trade-off", "tradeoff", "however", "drawback", "advantage", "unlike", "versus", "vs"]),
        (Structure::Invariant, &["invariant", "guarantee", "always", "deterministic", "reproducib", "mathematically"]),
        (Structure::Protocol, &["protocol", "step 1", "step 2", "phase", "stage", "pipeline", "process"]),
        (Structure::Pattern, &["pattern", "design", "approach", "strategy", "technique", "method"]),
        (Structure::Definition, &["is a", "defined as", "refers to", "represents", "means"]),
        (Structure::Comparison, &["compared to", "unlike", "in contrast", "whereas", "difference between"]),
    ];

    let mut best: Option<Structure> = None;
    let mut best_score = 0usize;
    for (structure, keywords) in scores {
        let score = keywords.iter().filter(|kw| lower.contains(*kw)).count();
        if score > best_score {
            best_score = score;
            best = Some(*structure);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_DOMAINS: [KnowledgeDomain; 9] = DOMAIN_ORDER;
    const ALL_STRUCTURES: [Structure; 9] = STRUCTURE_ORDER;

    /// Pararity proof, same shape as `anomaly_fold.rs`/`primality_fold.rs`: round-trip
    /// through the trit encoding is lossless for every real state.
    #[test]
    fn domain_trit_round_trips_for_all_9_states() {
        for d in ALL_DOMAINS {
            assert_eq!(KnowledgeDomain::from_trits(d.to_trits()), Some(d));
        }
    }

    #[test]
    fn structure_trit_round_trips_for_all_9_states() {
        for s in ALL_STRUCTURES {
            assert_eq!(Structure::from_trits(s.to_trits()), Some(s));
        }
    }

    /// Every trit digit stays in `-1..=1` — no domain ever encodes to an out-of-range
    /// component, which would collide with the corruption sentinel.
    #[test]
    fn all_trit_digits_are_balanced() {
        for d in ALL_DOMAINS {
            let [a, b] = d.to_trits();
            assert!((-1..=1).contains(&a) && (-1..=1).contains(&b));
        }
    }

    /// Out-of-range digits decode to `None` (corruption trap), not a wrong-but-valid state.
    #[test]
    fn out_of_range_trits_decode_to_none() {
        assert_eq!(KnowledgeDomain::from_trits([2, 0]), None);
        assert_eq!(KnowledgeDomain::from_trits([0, -2]), None);
    }

    #[test]
    fn content_id_deterministic() {
        assert_eq!(KnowledgeAtom::content_id("hello"), KnowledgeAtom::content_id("hello"));
        assert_ne!(KnowledgeAtom::content_id("hello"), KnowledgeAtom::content_id("world"));
    }

    #[test]
    fn domain_classification() {
        assert_eq!(classify_domain("Verlet integration is symplectic"), Some(KnowledgeDomain::Physics));
        assert_eq!(classify_domain("The shader renders pixels via wgpu"), Some(KnowledgeDomain::Rendering));
        assert_eq!(classify_domain("Faust DSP audio callback"), Some(KnowledgeDomain::Audio));
        assert_eq!(classify_domain("zzz no keywords here zzz"), None);
    }

    #[test]
    fn structure_classification() {
        assert_eq!(classify_structure("The algorithm iterates and calculates the formula"), Some(Structure::Algorithm));
        assert_eq!(
            classify_structure("Unlike RK4, Verlet has the advantage of energy conservation. However, the drawback is..."),
            Some(Structure::TradeOff)
        );
    }

    #[test]
    fn decay_works() {
        let mut atom = KnowledgeAtom::new("test".into(), "f.txt".into(), (0, 4), Some(KnowledgeDomain::Physics), Some(Structure::Definition), 0);
        assert_eq!(atom.freshness, 10000);
        atom.decay(500);
        assert_eq!(atom.freshness, 9500);
        atom.decay(10000);
        assert_eq!(atom.freshness, 0);
    }

    #[test]
    fn staleness_check() {
        let mut atom = KnowledgeAtom::new("test".into(), "f.txt".into(), (0, 4), Some(KnowledgeDomain::Physics), Some(Structure::Definition), 0);
        assert!(!atom.is_stale(5000));
        atom.freshness = 3000;
        assert!(atom.is_stale(5000));
    }
}
