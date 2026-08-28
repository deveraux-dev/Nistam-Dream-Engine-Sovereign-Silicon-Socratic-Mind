//! 5D semantic/lexical classification, ported from `outland-index`
//! (`F:\NewRepo\crates\outland\src\lib.rs:18-190`) — data-only, zero `unsafe`,
//! zero new dependencies.
//!
//! Ported: [`embed5`] (the family/z lane + 3 MinHash lanes + exact-fold lane),
//! [`family_of`]/the generic `FAMILIES` keyword table, and [`dist_sq_family_dominant`]
//! (the `z*z` dominant term from the real `ray_hits` scorer).
//!
//! NOT ported, on purpose: `outland`'s full `ray_hits` two-anchor scorer also adds a
//! stem-overlap tier/precision term (`tier * TIER_SQ + prec * PREC_SQ`,
//! `outland/src/lib.rs:434-446`) computed against a stem *set*, not the embed5 lanes
//! alone. [`dist_sq_family_dominant`] is the family-dominant subset only — call it a
//! coarse ranking, not a claim of parity with the original ray scorer.
//!
//! Also not ported: the `forge_core::concept` repo-lexicon layer that the v2
//! `family_of` consults before its generic table (`outland/src/lib.rs:121,129`) — v3
//! has no equivalent module yet, so every family here comes from the generic
//! `FAMILIES` table and z cells start at `0`, not offset by a `CONCEPT_SPAN`. Also
//! not ported: `vixio` NDJSON tick-delivery serving and the `windows-sys` MFT FFI
//! path — see `walker` for the zero-`unsafe` substitute for the latter.

pub mod idx;
pub mod walker;
#[cfg(feature = "unsafe-fast-scan")]
pub mod mft;

const LANE_SEEDS: [u64; 4] = [0xcbf29ce484222325, 0x9e3779b97f4a7c15, 0xff51afd7ed558ccd, 0xc4ceb9fe1a85ec53];
const IDENTITY_LANES: [usize; 3] = [0, 1, 3];
const FAMILY_LANE: usize = 2;
const EXACT_LANE: usize = 4;
const IDENTITY_MOD: u64 = 4093;
const IDENTITY_HALF: i64 = (IDENTITY_MOD / 2) as i64;
const TOKEN_MOD: u64 = 1021;
const TOKEN_HALF: i64 = (TOKEN_MOD / 2) as i64;
const EXACT_MOD: u64 = 509;
const EXACT_HALF: i64 = (EXACT_MOD / 2) as i64;
const MINHASH_LANES: usize = 3;
/// Weight step baked into the z lane at embed time (`z = (family_id+1) * FAMILY_STEP`),
/// so a squared z-delta between two different families dwarfs any lexical-lane delta —
/// `outland/src/lib.rs:3`'s own receipt: `FAMILY_STEP^2 (268M) > 3*TOKEN_HALF^2 +
/// EXACT_HALF^2 (~3.4M)`.
pub const FAMILY_STEP: i64 = 0x4000;

fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn fold_identity(h: u64) -> i64 {
    (h % IDENTITY_MOD) as i64 - IDENTITY_HALF
}

fn fold_token(h: u64) -> i64 {
    (h % TOKEN_MOD) as i64 - TOKEN_HALF
}

fn fold_exact(h: u64) -> i64 {
    (h % EXACT_MOD) as i64 - EXACT_HALF
}

/// The generic keyword-family table, verbatim from `outland/src/lib.rs:58-79`.
const FAMILIES: &[(&str, &[&str])] = &[
    ("memory", &["alloc", "malloc", "free", "heap", "gc", "pool", "arena", "dealloc", "page", "slab", "swap", "mmap", "vm"]),
    ("concurrency", &["thread", "lock", "mutex", "atomic", "async", "await", "channel", "sync", "spawn", "semaphore", "rcu"]),
    ("io", &["read", "write", "file", "stream", "buffer", "flush", "fread", "fwrite"]),
    ("network", &["tcp", "udp", "http", "socket", "packet", "dns", "network", "conn", "skb"]),
    ("parser", &["parse", "lexer", "token", "ast", "grammar", "syntax", "scan"]),
    ("scheduler", &["sched", "task", "queue", "dispatch", "worker", "runqueue", "preempt", "cpu"]),
    ("driver", &["driver", "device", "firmware", "interrupt", "irq", "dma", "probe", "pci"]),
    ("test", &["test", "mock", "assert", "fixture", "spec", "bench", "selftest"]),
    ("build", &["build", "compile", "link", "makefile", "cargo", "cmake", "configure", "kconfig"]),
    ("security", &["auth", "crypt", "encrypt", "permission", "sandbox", "cve", "secure", "hash", "key"]),
    ("ui", &["button", "widget", "render", "view", "window", "dialog", "gui"]),
    ("data", &["struct", "schema", "table", "record", "database", "index", "row", "inode", "dentry", "filesystem"]),
    ("error", &["error", "exception", "panic", "fail", "recover", "retry", "err", "oops"]),
    ("config", &["config", "setting", "option", "flag", "env", "param", "cfg"]),
    ("api", &["api", "endpoint", "route", "handler", "request", "response", "rpc", "syscall", "ioctl"]),
    ("graphics", &["shader", "pixel", "texture", "gpu", "draw", "vertex", "raster", "drm", "framebuffer"]),
    ("audio", &["audio", "sound", "sample", "codec", "mixer", "pcm", "alsa"]),
    ("math", &["matrix", "vector", "algorithm", "compute", "numeric"]),
    ("cache", &["cache", "lru", "evict"]),
    ("log", &["log", "trace", "telemetry", "logger", "printk"]),
];

/// Number of generic families in [`family_of`]'s table — the z-lane's ceiling
/// (`(FAMILY_COUNT as i64) * FAMILY_STEP` is the highest z value it can produce).
pub const FAMILY_COUNT: usize = FAMILIES.len();

fn contains_word(hay: &str, needle: &str) -> bool {
    let (hb, nb) = (hay.as_bytes(), needle.as_bytes());
    if nb.is_empty() || nb.len() > hb.len() {
        return false;
    }
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if &hb[i..i + nb.len()] == nb {
            let before = i == 0 || !hb[i - 1].is_ascii_alphanumeric();
            let after = i + nb.len() == hb.len() || !hb[i + nb.len()].is_ascii_alphanumeric();
            if before && after {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn contains_stem(hay: &str, needle: &str) -> bool {
    let (hb, nb) = (hay.as_bytes(), needle.as_bytes());
    if nb.is_empty() || nb.len() > hb.len() {
        return false;
    }
    let mut i = 0;
    while i + nb.len() <= hb.len() {
        if &hb[i..i + nb.len()] == nb && (i == 0 || !hb[i - 1].is_ascii_alphanumeric()) {
            return true;
        }
        i += 1;
    }
    false
}

fn key_hits(low: &str, k: &str) -> bool {
    if k.len() < 4 { contains_word(low, k) } else { contains_stem(low, k) }
}

/// Classify a lowercased string into a generic family index, or `None` for
/// honestly-unclassified input (never forced to a default family).
pub fn family_of(low: &str) -> Option<u8> {
    FAMILIES
        .iter()
        .position(|(name, keys)| key_hits(low, name) || keys.iter().any(|k| key_hits(low, k)))
        .map(|i| i as u8)
}

fn stem(t: &str) -> &str {
    let mut best: Option<&str> = None;
    for (_, keys) in FAMILIES {
        for k in *keys {
            if k.len() >= 4 && t.starts_with(k) && best.map_or(true, |b| k.len() < b.len()) {
                best = Some(k);
            }
        }
    }
    best.unwrap_or(t)
}

/// Project a string into the 5-lane coordinate `[x, y, z, theta, w]`.
///
/// `z` (index 2) is the family-dominant lane; `x`/`y`/`theta` (0/1/3) are MinHash
/// over the token-stem set (Jaccard approximation); `w` (4) is a whole-string exact
/// fold for tie-breaking. Verbatim port of `outland/src/lib.rs:153-175`.
pub fn embed5(s: &str) -> [i64; 5] {
    let low = s.to_ascii_lowercase();
    let mut out = [0i64; 5];
    out[FAMILY_LANE] = family_of(&low).map_or(0, |f| (f as i64 + 1) * FAMILY_STEP);
    let mut mh = [i64::MAX; MINHASH_LANES];
    let mut n = 0usize;
    for tok in low.split(|c: char| !c.is_ascii_alphanumeric()).filter(|t| t.len() >= 2) {
        let t = stem(tok);
        for slot in 0..MINHASH_LANES {
            mh[slot] = mh[slot].min(fold_token(fnv1a(LANE_SEEDS[slot], t.as_bytes())));
        }
        n += 1;
    }
    for slot in 0..MINHASH_LANES {
        out[IDENTITY_LANES[slot]] = if n == 0 {
            fold_identity(fnv1a(LANE_SEEDS[slot], low.as_bytes()))
        } else {
            mh[slot]
        };
    }
    out[EXACT_LANE] = fold_exact(fnv1a(LANE_SEEDS[3], low.as_bytes()));
    out
}

/// Family-dominant coarse distance between two [`embed5`] vectors: the squared
/// z-lane delta alone. NOT the full `outland::ray_hits` scorer (see module docs) —
/// this ranks "same family or not" correctly and nothing finer.
pub fn dist_sq_family_dominant(a: [i64; 5], b: [i64; 5]) -> i128 {
    let dz = a[FAMILY_LANE] as i128 - b[FAMILY_LANE] as i128;
    dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_count_matches_the_table() {
        assert_eq!(FAMILY_COUNT, 20);
    }

    #[test]
    fn a_known_keyword_classifies_and_an_unrelated_word_does_not() {
        assert_eq!(family_of("mutex_lock"), Some(1), "concurrency is table index 1");
        assert!(family_of("mutex_lock").is_some());
        assert!(family_of("xyzzy_plugh_no_match_here").is_none());
    }

    #[test]
    fn embed5_z_lane_is_zero_for_unclassified_and_stepped_for_classified() {
        let unclassified = embed5("xyzzy_plugh_no_match_here");
        assert_eq!(unclassified[FAMILY_LANE], 0);
        let classified = embed5("mutex_lock_guard");
        assert_eq!(classified[FAMILY_LANE], (1i64 + 1) * FAMILY_STEP);
    }

    #[test]
    fn same_string_embeds_identically_deterministic() {
        assert_eq!(embed5("forge_core_v3"), embed5("forge_core_v3"));
    }

    #[test]
    fn family_dominant_distance_outweighs_lexical_noise() {
        // Same family (concurrency), different words -> small z term (0).
        let a = embed5("mutex_lock");
        let b = embed5("thread_spawn");
        assert_eq!(dist_sq_family_dominant(a, b), 0);
        // Different families -> nonzero, and per the receipt in outland's own
        // header comment, large enough to dominate lexical-lane noise.
        let c = embed5("shader_pixel_draw"); // graphics
        assert!(dist_sq_family_dominant(a, c) > 0);
    }
}
