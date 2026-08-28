//! GhostMoon codebook-lookup primitive. PIN `_plans/pins/ghost-moon/PIN.md`,
//! Sean net-new sign-off 2026-07-08 ("wire it", then "rung 2"). Ported verbatim
//! from `F:\NewRepo\crates\forge-ml\src\nearest_neighbor.rs` 2026-08-14 (Sean
//! "we need all 2000 lines, recon it properly then decide").
//!
//! Home: `forge-ml-bqrouter`, not a new crate — v2's `nearest_neighbor.rs` lived
//! beside `bq_router.rs` in the same `forge-ml` crate and depends on it
//! (`crate::bq_router::{BQ_BYTES, BqRouter}` → `crate::{BQ_BYTES, BqRouter}` here,
//! both already landed at this crate's root). "forge-ml-bqrouter = forge-ml"
//! (Sean 2026-08-14): this crate IS the v3 landing for v2's forge-ml surface,
//! not a narrower sibling.
//!
//! DEVIATIONS from verbatim (all receipted, not silent):
//! - `forge_hal::triple_buffer::{ClockPlane, TripleBuffer}` → `forge_hal_clockspine::
//!   {ClockPlane, TripleBuffer}` (confirmed matching publish/try_take/try_generation API).
//! - `pp_math::fixed_point::isqrt_i64` → `forge_core_v3::fixed_point::isqrt_i64`
//!   (exact name + signature match, confirmed 2026-08-14).
//! - The `cree_frame_seam_tests` module (SPCC seam proof against
//!   `outland_index::soulword::{collide, compress_frames_silent, Collision}`) is
//!   EXCLUDED. That target is superseded, not missing: v3's live SPCC lives at
//!   `forge_core_v3::spcc` (same "Soliton-Phase Context Collapse" concept, same
//!   name, redesigned onto the ternary substrate — `ContextRow`/`Field5D`/
//!   `TritCell5D`/permyriad weight/tritwise Hamming distance — not `Frame5D`
//!   (`[i32;5]`)/millidegree-theta compatible). Porting a second `collide`/
//!   `Collision` implementation here would be an L05 (one-home) violation, not
//!   a completion.
//! - `rank5_no_lane_is_constant_on_real_disk_data` and
//!   `river_idx_round_trips_on_real_disk_data` are EXCLUDED: both `.expect()` a
//!   real `.forge/river.idx` at `<CARGO_MANIFEST_DIR>/../../.forge/river.idx` —
//!   checked, `F:\v3\.forge\river.idx` does not exist (that file is a product of
//!   v2's own tooling, never generated in this workspace). `absent` is a valid
//!   costless answer (T1 zero_hallucination); fabricating a fixture to fake "real
//!   disk data" would defeat the point of the test. `r3_projection_preserves_
//!   hamming_locality` is KEPT — it already falls back to a fixed base when its
//!   `meta_router.bqr` (also absent here) fails to load, so it is real coverage,
//!   not disk-dependent.
//!
//! Rung 1: squared-Euclidean nearest-of-N over a low-D integer embedding,
//! same shape as `bq_router`'s hamming-nearest scan generalized past its
//! fixed 7 centroids.
//!
//! Rung 2a: couples `nearest()` to the REAL codebook C the PIN names —
//! `.forge/river.idx` — via `embed_river_line`, a deterministic FNV1a
//! feature-lane embedding (v2 — rung 2f). This is SYNTACTIC (stable per exact
//! line text), NOT SEMANTIC — it does not know what a line MEANS, only that
//! it reliably lands the same line at the same point twice.
//!
//! Rung 2c (this addition): a genuinely SEMANTIC embedding, sourced from
//! `_plans/CREE.md`'s own structure — HORIZON/scratch per that doc's own
//! header, carried here as an embedding SOURCE, not a claim that grid is
//! shipped canon. CREE.md's 25 doctrine families x 4 rotations (`▽△◁▷`)
//! are a literal rotation encoding — Cree syllabic orientation = vowel is
//! quantized-angle meaning, the exact shape `THETA_LANE` exists for. Two
//! judgment calls made here, flagged not hidden: (1) degree assignment —
//! CREE.md gives directions not degrees, so `▷/PROOF`=0°, `△/ACT`=90°,
//! `◁/LAW`=180°, `▽/STATE`=270° (standard unit-circle, counterclockwise);
//! (2) `CREE_FAMILY_SCALE_MDEG` — without it, `z`(0..24) is numerically
//! swamped by `theta`(0..360000) in squared-Euclidean and family stops
//! mattering at all (caught by `family_axis_is_not_swamped_by_theta`).
//! Rung 2f (embed-rank, 2026-07-10): both codebooks now exercise ALL 5 lanes.
//! `embed_river_line` v2 derives each lane from an INDEPENDENT syntactic
//! feature (tag / payload / shape / token-order / token-set) instead of five
//! rotations of ONE hash (the rank-1 curve the embed-rank-1 flag gauged);
//! `embed_cree_cell` fills x/y/w with small identity lanes (family hash /
//! glyph / Λ_z layer) sized to NEVER swamp the z/theta semantics. Production
//! callers live: daemon raycast+query (`repo_query.rs`), studio probe,
//! `timeline_recorder`'s GhostMoonBridge host.

use std::path::Path;

/// GhostMoon box dimensionality: `[x, y, z, theta, w]`.
pub const EMBED_DIM: usize = 5;

/// Index of the theta (angular) lane — wraps at 360°, unlike the other 4
/// linear lanes. Matches forge-audio's live `Point5D::theta_mdeg` convention
/// (`dimensional_collapse.rs:32,96-98`, `rem_euclid(360_000)`).
pub const THETA_LANE: usize = 3;
/// Millidegrees per full turn — same wrap modulus as `Point5D::theta_mdeg`.
pub const THETA_WRAP_MDEG: i32 = 360_000;

/// The world's own 5D cell, for reference: `forge_zones::normalized_zone` is a 33×33×33
/// wire cube whose point carries `[x, y, z, t, s]` — the SAME five lanes as this box, and
/// it already refuses `sqrt` by comparing squared radii (`normalized_zone.rs:82-86`). The
/// semantic box and the world lattice are the same shape at different scales, which is why
/// `ANGULAR_CELLS` is 66 = 2×33.
pub const WORLD_LATTICE_CELLS: i64 = 33;

/// One codebook entry: a caller-defined id + its embedding coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeEntry {
    /// Caller-defined identity for this codebook cell.
    pub id: u32,
    /// The cell's coordinates in the 5D box.
    pub coords: [i32; EMBED_DIM],
}

/// Per-lane difference. `THETA_LANE` wraps at `THETA_WRAP_MDEG` (1° and 359°
/// are 2° apart, not 358°); the other 4 lanes are plain linear difference.
/// For values already within `(-THETA_WRAP_MDEG, THETA_WRAP_MDEG)` this is
/// numerically identical to `a - b` — no behavior change for non-angular data.
#[inline]
fn lane_diff(lane: usize, a: i32, b: i32) -> i64 {
    if lane == THETA_LANE {
        let raw = (a - b).rem_euclid(THETA_WRAP_MDEG) as i64;
        raw.min(THETA_WRAP_MDEG as i64 - raw)
    } else {
        (a - b) as i64
    }
}

/// Squared Euclidean distance, integer-exact (no float leakage). Wrap-aware
/// on the theta lane (see `lane_diff`) — a flat sum-of-squares would be wrong
/// for an angle by design, not just imprecise.
#[inline]
pub fn squared_distance(a: &[i32; EMBED_DIM], b: &[i32; EMBED_DIM]) -> i64 {
    let mut d: i64 = 0;
    for i in 0..EMBED_DIM {
        let diff = lane_diff(i, a[i], b[i]);
        d += diff * diff;
    }
    d
}

/// Nearest-of-N: brute-force scan, deterministic first-minimum tie-break.
/// Returns `None` on an empty codebook (mirrors `BqRouter::route`'s
/// None-on-no-active-centroids contract — caller must fall back).
pub fn nearest(query: &[i32; EMBED_DIM], codebook: &[CodeEntry]) -> Option<(u32, i64)> {
    let mut best: Option<(u32, i64)> = None;
    for e in codebook {
        let d = squared_distance(query, &e.coords);
        match best {
            Some((_, bd)) if d >= bd => {}
            _ => best = Some((e.id, d)),
        }
    }
    best
}

/// Nearest-**k**-of-N into a caller-owned slice; returns how many slots were
/// filled (`min(k, codebook.len())`). Zero-alloc: `out.len()` IS `k`, so a hot
/// caller hands in a fixed array and never touches the heap.
///
/// Results land ascending by distance. Ties keep the earlier codebook entry
/// first — the same deterministic first-minimum rule [`nearest`] uses, so
/// `k_nearest_into(q, cb, &mut [slot; 1])[0] == nearest(q, cb)` always.
///
/// Bounded insertion, not a sort: an entry worse than the current k-th is
/// rejected on one comparison, so the scan stays O(N·k) with k small and never
/// materializes the full N-length distance list.
pub fn k_nearest_into(
    query: &[i32; EMBED_DIM],
    codebook: &[CodeEntry],
    out: &mut [(u32, i64)],
) -> usize {
    let k = out.len();
    if k == 0 {
        return 0;
    }
    let mut len = 0usize;
    for e in codebook {
        let d = squared_distance(query, &e.coords);
        if len == k && d >= out[k - 1].1 {
            continue;
        }
        // Free slot while filling; otherwise overwrite the k-th, already beaten.
        let mut i = len.min(k - 1);
        while i > 0 && out[i - 1].1 > d {
            out[i] = out[i - 1];
            i -= 1;
        }
        out[i] = (e.id, d);
        if len < k {
            len += 1;
        }
    }
    len
}

/// Allocating wrapper over [`k_nearest_into`] for cold callers (edge-graph
/// build, tests, tooling). Hot paths take the `_into` form.
pub fn k_nearest(
    query: &[i32; EMBED_DIM],
    codebook: &[CodeEntry],
    k: usize,
) -> Vec<(u32, i64)> {
    let mut out = vec![(0u32, 0i64); k]; // @forge:allow_alloc — cold-path wrapper; hot lane is `k_nearest_into`
    let n = k_nearest_into(query, codebook, &mut out);
    out.truncate(n);
    out
}

// ── Rung 3: raycast (closest-point-to-ray) — 5D-native, no forge-render dep ──

/// Signed, shortest wrap-aware delta FROM `from` TO `to` on one lane. Unlike
/// `lane_diff` (magnitude only, for squared distance), a vector needs a
/// SIGN — the direction of "shortest way there" on the circle. On
/// `THETA_LANE`, 350°→0° is a signed `+10°`, never `-350°`. Sean's directive:
/// wrap-awareness stays load-bearing in the vector itself, not just the
/// final distance.
#[inline]
fn lane_delta_signed(lane: usize, from: i32, to: i32) -> i64 {
    if lane == THETA_LANE {
        let raw = (to as i64 - from as i64).rem_euclid(THETA_WRAP_MDEG as i64);
        if raw > (THETA_WRAP_MDEG as i64) / 2 { raw - THETA_WRAP_MDEG as i64 } else { raw }
    } else {
        to as i64 - from as i64
    }
}

/// Wrap-aware vector FROM `from` TO `to`, one signed delta per lane.
fn vector_to(from: &[i32; EMBED_DIM], to: &[i32; EMBED_DIM]) -> [i64; EMBED_DIM] {
    std::array::from_fn(|i| lane_delta_signed(i, from[i], to[i]))
}

#[inline]
fn dot(a: &[i64; EMBED_DIM], b: &[i64; EMBED_DIM]) -> i128 {
    let mut s = 0i128;
    for i in 0..EMBED_DIM {
        s += a[i] as i128 * b[i] as i128;
    }
    s
}

/// A ray in the 5D box: origin + direction. Direction need not be unit —
/// the projection below normalizes by `dot(dir, dir)` internally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ray5D {
    /// Where the ray starts.
    pub origin: [i32; EMBED_DIM],
    /// The ray's (not necessarily unit) direction.
    pub dir: [i32; EMBED_DIM],
}

/// Wrap-aware ray FROM `from` THROUGH `to` — the constructor `Ray5D` lacked,
/// and the reason nothing outside this crate could build one. Direction comes
/// from `vector_to`/`lane_delta_signed`, so `THETA_LANE` takes the SHORT way
/// round: a caller writing the obvious `to[i] - from[i]` gets lane 3 wrong.
/// Lane magnitudes saturate into `i32` rather than wrap.
pub fn ray_between(from: &[i32; EMBED_DIM], to: &[i32; EMBED_DIM]) -> Ray5D {
    let v = vector_to(from, to);
    Ray5D {
        origin: *from,
        dir: std::array::from_fn(|i| v[i].clamp(i32::MIN as i64, i32::MAX as i64) as i32),
    }
}

/// Closest point on `ray` to `point`, wrap-aware throughout. Returns
/// `(t, perp_dist_sq)`: `t` is the clamped ray parameter (`t >= 0` — a TRUE
/// ray, never looking behind its own origin, not an infinite line);
/// `perp_dist_sq` is the squared perpendicular distance from `point` to that
/// on-ray point. `t` is truncating integer division — SMALL per this rung's
/// own discipline, sub-integer precision is a flagged, not hidden, gap.
///
/// Degenerate `dir == [0;5]`: the "ray" is just its origin — falls back to
/// `squared_distance` from the origin, `t = 0`, never a div-by-zero.
pub fn closest_point_on_ray(ray: &Ray5D, point: &[i32; EMBED_DIM]) -> (i64, i64) {
    let v = vector_to(&ray.origin, point); // wrap-aware origin -> point
    let d: [i64; EMBED_DIM] = std::array::from_fn(|i| ray.dir[i] as i64);
    let dd = dot(&d, &d);
    if dd == 0 {
        return (0, squared_distance(&ray.origin, point));
    }
    let vd = dot(&v, &d);
    let t: i64 = (vd / dd).max(0).try_into().unwrap_or(i64::MAX); // bounded by EMBED_DIM=5 coordinate ranges

    let mut on_ray = [0i32; EMBED_DIM];
    for i in 0..EMBED_DIM {
        let raw = ray.origin[i] as i64 + t * ray.dir[i] as i64;
        on_ray[i] = if i == THETA_LANE {
            raw.rem_euclid(THETA_WRAP_MDEG as i64) as i32
        } else {
            raw as i32
        };
    }
    (t, squared_distance(&on_ray, point))
}

/// Raycast nearest-of-N: pick the codebook entry with the smallest
/// perpendicular distance to `ray`'s line (a true ray, `t >= 0`).
/// Deterministic first-minimum tie-break, same contract as `nearest()`.
pub fn nearest_along_ray(ray: &Ray5D, codebook: &[CodeEntry]) -> Option<(u32, i64)> {
    let mut best: Option<(u32, i64)> = None;
    for e in codebook {
        let (_, perp) = closest_point_on_ray(ray, &e.coords);
        match best {
            Some((_, bd)) if perp >= bd => {}
            _ => best = Some((e.id, perp)),
        }
    }
    best
}

/// Additive sibling of [`nearest_along_ray`] (PULL-BOARD NEXT #6 gate 3): on a
/// genuinely EQUAL `perp_sq` the caller-supplied key decides (smaller wins);
/// unequal `perp_sq` behaves byte-for-byte like `nearest_along_ray`, and an
/// equal key falls back to its first-minimum order. Separate opt-in fn so the
/// existing call-sites (concept_lexicon, acoustic_index, studio, door) keep
/// their proven semantics untouched. CEILING (board-ruled): the key is a
/// TIE-BREAK only — it must never become the primary sort.
pub fn nearest_along_ray_tiebreak(
    ray: &Ray5D,
    codebook: &[CodeEntry],
    tie_key: impl Fn(u32) -> u32,
) -> Option<(u32, i64)> {
    let mut best: Option<(u32, i64, u32)> = None; // (id, perp, key)
    for e in codebook {
        let (_, perp) = closest_point_on_ray(ray, &e.coords);
        let replace = match best {
            None => true,
            Some((_, bd, _)) if perp < bd => true,
            Some((_, bd, bk)) if perp == bd => tie_key(e.id) < bk,
            _ => false,
        };
        if replace {
            best = Some((e.id, perp, tie_key(e.id)));
        }
    }
    best.map(|(id, perp, _)| (id, perp))
}

// ── Rung 2a: river.idx codebook coupling ─────────────────────────────────────

/// FNV-1a 64-bit. Deterministic, no external crate — same reproducibility
/// bar as `bq_router`'s hand-rolled XOR+POPCNT hamming.
#[inline]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// Fold a 64-bit hash into the ±0x8000 lane window (low 16 bits, centered) —
/// the same window `STRAIN_LANE_STEP`/`STRAIN_LANE_MAX` were sized against.
#[inline]
fn fold16(h: u64) -> i32 {
    (h & 0xFFFF) as i32 - 0x8000
}

/// Syntactic φ(c) v2 — effective rank 5 (rung 2f). One river.idx line -> a
/// stable `[i32; 5]` point, each lane an INDEPENDENT feature of the text
/// (v1 rotated ONE hash into all 5 lanes — a rank-1 curve, so no lane could
/// ever agree while another differed):
///   x     = tag cluster — FNV1a of the text before the first tab (the row
///           kind: MAP/SPILL/FLAG…); same-kind rows share x EXACTLY.
///   y     = payload discriminator — FNV1a of the text after the first tab;
///           full avalanche, the "distinct lines stay distinct" lane.
///   z     = shape locality — capped byte length ×32 + tab count, centered;
///           similarly-shaped rows land NEAR — the one metric (non-hash) lane.
///   theta = token order — position-salted fold of whitespace tokens;
///           reordering tokens moves this lane (and y) only.
///   w     = token set — order-insensitive XOR fold of token hashes
///           (multiset parity: duplicate pairs cancel); reordered rows KEEP w.
/// Identical text -> identical point, always. NOT a meaning embedding.
pub fn embed_river_line(line: &str) -> [i32; EMBED_DIM] {
    let (tag, payload) = match line.split_once('\t') {
        Some((t, p)) => (t, p),
        None => (line, ""),
    };
    let len = line.len().min(1023) as i32;
    let tabs = line.bytes().filter(|&b| b == b'\t').count().min(31) as i32;

    let mut order: u64 = 0xcbf29ce484222325;
    let mut set: u64 = 0;
    for (i, tok) in line.split_whitespace().enumerate() {
        let th = fnv1a64(tok.as_bytes());
        order ^= th.rotate_left((i as u32).wrapping_mul(7));
        order = order.wrapping_mul(0x100000001b3);
        set ^= th;
    }

    [
        fold16(fnv1a64(tag.as_bytes())),
        fold16(fnv1a64(payload.as_bytes())),
        (len << 5) + tabs - 0x4000,
        fold16(order),
        fold16(set),
    ]
}

/// Parse `.forge/river.idx` raw text into a codebook: one `CodeEntry` per
/// non-empty line, `id` = line index, paired with the original line text so
/// a hit is traceable back to disk (never a bare coordinate).
pub fn load_river_codebook(idx_text: &str) -> Vec<(CodeEntry, String)> {
    idx_text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (CodeEntry { id: i as u32, coords: embed_river_line(l) }, l.to_string()))
        .collect()
}

/// Load + decode `.forge/river.idx` from disk (UTF-8 raw bytes — the idx
/// files are binary-packed per project convention, never `Read`/`Path::read_to_string`
/// blind; this mirrors the decode law: `[u8]` -> `String::from_utf8_lossy`).
pub fn load_river_codebook_from_disk(path: &Path) -> std::io::Result<Vec<(CodeEntry, String)>> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(load_river_codebook(&text))
}

// ── 7-7-7 orient: a FAN of rays, cross-referenced to a consensus master ──────
// The lateral wire (2026-07-18, Sean): forge_pkm::distill is the 7-7-7 dual-school
// cascade — students -> teachers (cross-reference) -> masters -> master-to-master
// finds what one lens can't. A single orient ray is ONE student and a trajectory
// across CREE families discriminates poorly (family is the ray direction, not the
// perp). Fire the fan from many origins (lenses/schools), tally which row each ray
// ranks top-k (the teacher cross-reference), and the most-voted row is the master
// the lateral connection no single ray confirmed. Pure application of ray_between
// + closest_point_on_ray; no new geometry, no tool change, no door rebuild.

/// 7-7-7 orient: cast a ray from every `origin` (student/lens) toward one
/// `subject`, tally top-`topk` hits (teacher cross-reference), return rows by
/// (votes desc, tightest perp). `out[0]` is the consensus master. Empty origins
/// or codebook -> empty.
pub fn orient_777(
    subject: &[i32; EMBED_DIM],
    origins: &[[i32; EMBED_DIM]],
    codebook: &[CodeEntry],
    topk: usize,
) -> Vec<(u32, u32, i64)> {
    let mut votes: std::collections::HashMap<u32, (u32, i64)> = std::collections::HashMap::new();
    for origin in origins {
        let ray = ray_between(origin, subject);
        let mut ranked: Vec<(i64, u32)> = codebook
            .iter()
            .map(|e| (closest_point_on_ray(&ray, &e.coords).1, e.id))
            .collect();
        ranked.sort_by_key(|&(perp, id)| (perp, id));
        for &(perp, id) in ranked.iter().take(topk.max(1)) {
            let slot = votes.entry(id).or_insert((0, i64::MAX));
            slot.0 += 1;
            slot.1 = slot.1.min(perp);
        }
    }
    let mut out: Vec<(u32, u32, i64)> =
        votes.into_iter().map(|(id, (v, p))| (id, v, p)).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(a.0.cmp(&b.0)));
    out
}

// ── Rung 2c: CREE.md family+rotation codebook (semantic embedding) ──────────

/// `_plans/CREE.md`'s 25 doctrine domains (its `F1`..`F25`), 0-indexed here.
pub const CREE_FAMILIES: [&str; 25] = [
    "CLOCK", "GPU", "BRIDGE", "BUFFER", "GATE", "SOUND", "VISION", "LOCK-FREE",
    "PASS", "SIGNAL", "PROOF", "APERTURE", "DRUM", "FIREWALL", "SoT", "DAEMON",
    "SHADOW", "CONTEXT", "QUARRY", "WIRE", "HASH", "SEAN", "DESK", "FRED", "BUILD",
];

/// The 4 rotation roles in CREE.md's own column order (`▽ STATE, △ ACT,
/// ◁ LAW, ▷ PROOF`) with this rung's degree assignment (judgment call, see
/// module doc): standard unit-circle, counterclockwise from `▷`=0°.
pub const CREE_ROLES: [(&str, i32); 4] =
    [("STATE", 270_000), ("ACT", 90_000), ("LAW", 180_000), ("PROOF", 0)];

/// Family-index scale so `z` and `theta` land on comparable magnitudes in
/// squared-Euclidean (judgment call, see module doc) — `360_000 / 25`.
pub const CREE_FAMILY_SCALE_MDEG: i32 = 14_400;

/// CREE.md's glyph per (family, role), column order `▽△◁▷` matching
/// `CREE_ROLES`, transcribed verbatim from the doc's F1..F25 table rows.
pub const CREE_GLYPHS: [[char; 4]; 25] = [
    ['ᐁ', 'ᐂ', 'ᐃ', 'ᐄ'], ['ᐅ', 'ᐆ', 'ᐇ', 'ᐈ'], ['ᐉ', 'ᐊ', 'ᐋ', 'ᐌ'],
    ['ᐍ', 'ᐎ', 'ᐏ', 'ᐐ'], ['ᐑ', 'ᐒ', 'ᐓ', 'ᐔ'], ['ᐕ', 'ᐖ', 'ᐗ', 'ᐘ'],
    ['ᐙ', 'ᐚ', 'ᐛ', 'ᐜ'], ['ᐝ', 'ᐞ', 'ᐟ', 'ᐠ'], ['ᐡ', 'ᐢ', 'ᐣ', 'ᐤ'],
    ['ᐥ', 'ᐦ', 'ᐧ', 'ᐨ'], ['ᐩ', 'ᐪ', 'ᐫ', 'ᐬ'], ['ᐭ', 'ᐮ', 'ᐯ', 'ᐰ'],
    ['ᐱ', 'ᐲ', 'ᐳ', 'ᐴ'], ['ᐵ', 'ᐶ', 'ᐷ', 'ᐸ'], ['ᐹ', 'ᐺ', 'ᐻ', 'ᐼ'],
    ['ᐽ', 'ᐾ', 'ᐿ', 'ᑀ'], ['ᑁ', 'ᑂ', 'ᑃ', 'ᑄ'], ['ᑅ', 'ᑆ', 'ᑇ', 'ᑈ'],
    ['ᑉ', 'ᑊ', 'ᑋ', 'ᑌ'], ['ᑍ', 'ᑎ', 'ᑏ', 'ᑐ'], ['ᑑ', 'ᑒ', 'ᑓ', 'ᑔ'],
    ['ᑕ', 'ᑖ', 'ᑗ', 'ᑘ'], ['ᑙ', 'ᑚ', 'ᑛ', 'ᑜ'], ['ᑝ', 'ᑞ', 'ᑟ', 'ᑠ'],
    ['ᑡ', 'ᑢ', 'ᑣ', 'ᑤ'],
];

/// One resolved cell of the grid, traceable back to the doc (never a bare coordinate).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreeCell {
    /// 0..24 — feeds [`lambda_z_family_to_layer`] directly.
    pub family_index: u8,
    /// The family name, e.g. `"FIREWALL"`.
    pub family: &'static str,
    /// The rotation role, e.g. `"PROOF"`.
    pub role: &'static str,
    /// The transcribed CREE.md glyph for this (family, role) cell.
    pub glyph: char,
}

/// Ceiling for the cree x/y/w identity lanes (rung 2f) — sized so the whole
/// open-lane cost (x²+y²+w² ≤ ~10M) NEVER outvotes one family step
/// (`CREE_FAMILY_SCALE_MDEG`² ≈ 207M) or 10° of theta (100M): family/role
/// resolution stays z/theta-driven, the open lanes are exercised tie-breakers
/// (rank law: lane const-or-zero = UNEXERCISED).
pub const CREE_OPEN_LANE_WINDOW: i32 = 0x800;

/// φ(c) for a grid cell — all 5 lanes exercised (rung 2f):
///   x = family-name FNV1a folded small (the same syntactic-hash family as
///       `embed_river_line`'s tag lane) — constant within a family.
///   y = glyph identity, centered on the syllabics block — unique per cell.
///   z = scaled family index (unchanged).
///   theta = role's degree assignment (unchanged, wrap-aware).
///   w = Λ_z compositor layer, symmetric-centered — the proven 25->8 seam
///       echoed into the box, constant within a layer group.
pub fn embed_cree_cell(family_index: u8, role_index: usize) -> [i32; EMBED_DIM] {
    let family = CREE_FAMILIES[family_index as usize];
    let glyph = CREE_GLYPHS[family_index as usize][role_index];
    let theta = CREE_ROLES[role_index].1;
    let z = (family_index as i32) * CREE_FAMILY_SCALE_MDEG;
    let x = fold16(fnv1a64(family.as_bytes())) / 16; // ±0x800
    let y = (glyph as i32 - 0x1432) * 32; // syllabics block, ±0x640
    let w = (lambda_z_family_to_layer(family_index) as i32 * 2
        - (COMPOSITOR_LAYER_COUNT as i32 - 1))
        * 0x100; // ±0x700
    [x, y, z, theta, w]
}

/// Build the full 25x4 = 100-cell codebook.
pub fn load_cree_codebook() -> Vec<(CodeEntry, CreeCell)> {
    let mut out = Vec::with_capacity(CREE_FAMILIES.len() * CREE_ROLES.len());
    let mut id = 0u32;
    for (f_idx, &family) in CREE_FAMILIES.iter().enumerate() {
        for (role_idx, &(role, _)) in CREE_ROLES.iter().enumerate() {
            let coords = embed_cree_cell(f_idx as u8, role_idx);
            let glyph = CREE_GLYPHS[f_idx][role_idx];
            out.push((CodeEntry { id, coords }, CreeCell { family_index: f_idx as u8, family, role, glyph }));
            id += 1;
        }
    }
    out
}

/// SPCC bridge (Sean 2026-07-27): re-lane a cree cell's coords into a
/// `Frame5D`-shaped frame — semantic z/theta/w kept, x/y overlaid with
/// stream position. Returns a bare `[i32; 5]`: structurally the bridge, zero
/// crate coupling. On the 100-cell grid the SPCC predicate is total: z gates
/// family exactly, role thetas are exact quarter-turns, so every pair
/// resolves with zero near-miss ambiguity (anti-phase = PROOF<->LAW /
/// ACT<->STATE in-family). NOTE: v2's seam test proved this against
/// `outland_index::soulword::collide` (`Frame5D = [i32;5]`, millidegree
/// theta); that target is superseded in v3 by `forge_core_v3::spcc`'s
/// redesigned ternary-substrate collision kernel, which does not share this
/// shape, so the seam proof itself is not ported (see module doc). This
/// bridge function is kept — it is a real, load-bearing, zero-dependency
/// re-lane, not test scaffolding.
#[inline(always)]
pub fn cree_frame(coords: [i32; EMBED_DIM], line: i32, ordinal: i32) -> [i32; EMBED_DIM] {
    [line, ordinal, coords[2], coords[3], coords[4]]
}

// ── Rung 2g: SEMANTIC river-line embedding — all 3 rungs (Sean /goal 2026-07-11)
// `embed_river_line` above is SYNTACTIC (avalanche identity, exact-match only).
// These place a river line on the SAME kind of axis `embed_cree_cell` proved:
// the METRIC lanes carry MEANING (z = doctrine family, dominant), the hash lanes
// carry IDENTITY demoted into a small tie-break band, and `closest_point_on_ray`
// sums all five so ONE ray blends both. Family must OUT-VOTE every tie-break by
// construction: RIVER_FAMILY_STEP² (≈2.68e8) >> the whole band (x²+y²+w²+role²
// ≤ ~1.7e7). All three rungs are offline / deterministic / integer.
//   R1 lexical        — classify line -> CREE family/role by keyword lexicon.
//   R2 distributional — TF(line) × IDF(corpus) signed random-projection -> z/θ.
//   R3 sovereign-model — a learned BQ/steering code -> the same z/θ, falling
//      back to R2 when no model is loaded (the Student->Oracle ladder shape).

// ── FOUNDATION: the canonical GhostMoon 5-lane map ────────────────────────────
// Every semantic embedding below is BUILT ON this one contract, so a ray reads
// the box the same way whatever filled it. Lanes [x, y, z, theta, w] = 0..4:
//   lane 2 (FAMILY_LANE)     = coarse MEANING — linear, dominant magnitude.
//   lane 3 (ROLE_LANE=theta) = rotational MEANING (role/angle) — wrap-aware.
//   lanes 0,1,4 (IDENTITY)   = syntactic identity — bounded tie-break.
// Invariant (proven, `foundation_meaning_out_votes_identity`): one FAMILY step
// squared strictly exceeds the WHOLE identity+role band, so meaning is never a
// hash-collision artifact and identity can only ever break a within-family tie.

/// Lane carrying coarse meaning (doctrine family). Linear, dominant.
pub const FAMILY_LANE: usize = 2;
/// Lane carrying rotational meaning (role). THE wrap lane, by design.
pub const ROLE_LANE: usize = THETA_LANE;
/// Lanes carrying syntactic identity — bounded tie-break, never out-vote meaning.
pub const IDENTITY_LANES: [usize; 3] = [0, 1, 4];

// ── THE FIVE NAMED LANES (Sean 07-29) ─────────────────────────────────────────
// Family · Azimuth · Elevation · Role · Domain — every lane a true semantic or spatial
// coordinate, no lane a hash. Azimuth rides THETA because it is the wrap-aware axis the
// metric already handles (`lane_diff`); the other four are linear.

/// Direction, horizontal. THE wrap lane — 360_000 mdeg, short way round.
pub const AZIMUTH_LANE: usize = THETA_LANE;
/// Direction, vertical. Signed HALF turn, never wraps: up and down stay distinct.
pub const ELEVATION_LANE: usize = 0;
/// Role/facet within a family — what the row DOES, held apart from where it points.
pub const ROLE_LANE_V2: usize = 1;
/// Domain/organ the row belongs to — the coarse partition above family.
pub const DOMAIN_LANE: usize = 4;

/// The five lanes with their names, in lane order. A lane absent from this table is an
/// unowned axis, which is exactly how three hash lanes hid inside a semantic index.
pub const LANE_NAMES: [(usize, &str); EMBED_DIM] = [
    (ELEVATION_LANE, "elevation"),
    (ROLE_LANE_V2, "role"),
    (FAMILY_LANE, "family"),
    (AZIMUTH_LANE, "azimuth"),
    (DOMAIN_LANE, "domain"),
];

/// Elevation bound in millidegrees — a half turn, not a full one.
pub const ELEVATION_MAX_MDEG: i32 = 90_000;

/// Cells per angular axis — 66 = 2×33, the world lattice doubled (Sean 07-29). 66×66 =
/// 4_356 directions, which is why azimuth and elevation take SEPARATE lanes: one
/// 360_000-mdeg circle cannot separate 4_356 cells above the identity noise floor.
pub const ANGULAR_CELLS: i64 = 66;

/// Fixed-point scale for the octant ratio `r = min/max ∈ [0,1]`. A power of two so the
/// divides are shifts on every target.
pub const ATAN_FP: i64 = 1 << 10;

/// Working magnitude a direction vector is lifted to before its integer square root, so
/// small inputs keep their heading. `2^20` leaves `x²+y²` far inside `i64` while giving
/// `isqrt` six spare digits.
pub const SPHERE_WORK_MAG: i64 = 1 << 20;

/// Millidegrees per radian, `180_000/π`, rounded. The conversion Sean's 07-29 draft
/// missed: it scaled a radian as 45_000 mdeg, so `1_440_000/(32+9)` read 35_122 for a
/// true 45° — every direction 22% low, with no test in the repo to catch it.
pub const MDEG_PER_RAD: i64 = 57_296;

/// Rajan's arctangent coefficients, scaled to millidegrees:
/// `atan(r) ≈ (π/4)r − r(r−1)(0.2447 + 0.0663r)`.
/// The single-term `r/(1+0.28r²)` form measured 282 mdeg worst error — over 5% of a
/// 5_455-mdeg cell, so quantisation could jitter across a cell wall. This two-term form
/// holds ~86 mdeg instead. Bound asserted by `atan2_error_stays_inside_a_twentieth_of_a_cell`,
/// never by this comment.
/// Rajan's `atan` coefficient c1, scaled to millidegrees (`0.2447 × MDEG_PER_RAD`).
pub const ATAN_C1_MDEG: i64 = 14_020;
/// Rajan's `atan` coefficient c2, scaled to millidegrees (`0.0663 × MDEG_PER_RAD`).
pub const ATAN_C2_MDEG: i64 = 3_799;

/// Integer four-quadrant arctangent in MILLIDEGREES, `0..360_000` — zero float, so the
/// same heading lands bit-identical on CPU and in SPIR-V (`forge-gpu#float-boundary`;
/// only `OpIMul`/`OpSDiv`/`OpSMod`/`SAbs`/`SMin`/`SMax` are emitted, all bit-exact per
/// Vulkan 1.0 core). Octant symmetry reduces to `r ∈ [0,1]`, then Rajan's two terms.
pub fn atan2_mdeg(y: i64, x: i64) -> i32 {
    if x == 0 && y == 0 {
        return 0;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let (num, den) = if ax >= ay { (ay, ax) } else { (ax, ay) };
    // r in ATAN_FP fixed point. den >= num > 0 here, so r lands in 0..=ATAN_FP.
    let r = (num.saturating_mul(ATAN_FP)) / den.max(1);
    // (π/4)r — a quarter turn is 45_000 mdeg exactly.
    let linear = (45_000 * r) / ATAN_FP;
    // r(r−1)(c1 + c2·r), every division exact in the fixed-point scale.
    let bend = (r * (r - ATAN_FP) / ATAN_FP) * (ATAN_C1_MDEG + (ATAN_C2_MDEG * r) / ATAN_FP)
        / ATAN_FP;
    let base = (linear - bend).clamp(0, 45_000);
    let oct = if ax >= ay { base } else { 90_000 - base };
    let full = match (x >= 0, y >= 0) {
        (true, true) => oct,
        (false, true) => 180_000 - oct,
        (false, false) => 180_000 + oct,
        (true, false) => 360_000 - oct,
    };
    full.rem_euclid(THETA_WRAP_MDEG as i64) as i32
}

/// A direction as millidegrees plus its 66×66 lattice cell. The purge primitive: a
/// projection has a direction, a hash does not, so this is what replaces `fold_identity`
/// on the reclaimed lanes. Cells are floor-quantised, so a cell index IS the lattice row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SphericalLattice {
    /// `0..360_000`
    pub azimuth_mdeg: i32,
    /// `-90_000..=90_000`
    pub elevation_mdeg: i32,
    /// `0..=65`
    pub cell_azimuth: u8,
    /// `0..=65`
    pub cell_elevation: u8,
}

impl SphericalLattice {
    /// Quantise a signed 3-vector (a TF×IDF projection, a Gerzon energy vector, any real
    /// direction) onto the lattice. Integer throughout.
    pub fn from_cartesian(x: i32, y: i32, z: i32) -> Self {
        let (x, y, z) = (x as i64, y as i64, z as i64);
        // PRE-SCALE so `isqrt_i64` has precision to spend. Direction is scale-free, but
        // integer isqrt is not: for (-7,2,-1) the horizontal radius truncates to 7 where
        // the true value is 7.28, and the elevation ratio -1/7 differs from -1000/7280 by
        // a whole lattice cell. Lifting the vector to a fixed working magnitude first makes
        // the answer identical at 1× and 1000×, which
        // `scaling_a_vector_never_moves_its_direction` asserts.
        let mag = x.abs().max(y.abs()).max(z.abs());
        let (x, y, z) = if mag == 0 {
            (0, 0, 0)
        } else {
            let k = SPHERE_WORK_MAG / mag;
            if k > 1 { (x * k, y * k, z * k) } else { (x, y, z) }
        };
        let r_xy = isqrt_i64(x * x + y * y);
        let azimuth_mdeg = atan2_mdeg(y, x);
        // Elevation is signed and never wraps: measure from the horizontal plane, and at
        // the poles (r_xy == 0) answer straight up or straight down exactly.
        let elevation_mdeg = if r_xy == 0 {
            match z.cmp(&0) {
                std::cmp::Ordering::Greater => ELEVATION_MAX_MDEG,
                std::cmp::Ordering::Less => -ELEVATION_MAX_MDEG,
                std::cmp::Ordering::Equal => 0,
            }
        } else {
            let a = atan2_mdeg(z.abs(), r_xy).min(ELEVATION_MAX_MDEG);
            if z < 0 { -a } else { a }
        };
        let cell_azimuth =
            ((azimuth_mdeg as i64 * ANGULAR_CELLS) / THETA_WRAP_MDEG as i64).clamp(0, 65) as u8;
        let cell_elevation = (((elevation_mdeg as i64 + ELEVATION_MAX_MDEG as i64)
            * ANGULAR_CELLS)
            / (2 * ELEVATION_MAX_MDEG as i64))
            .clamp(0, 65) as u8;
        Self { azimuth_mdeg, elevation_mdeg, cell_azimuth, cell_elevation }
    }
}

/// Integer square root, via `forge_core_v3::fixed_point::isqrt_i64` (v2's own
/// comment: "DRAINED 07-29 to `pp_math::fixed_point::isqrt_i64`, the canonical
/// home... a 21st [isqrt] would be one more place for CPU/SPIR-V parity to
/// drift" — v3's home is `forge-core-v3::fixed_point`, exact name+signature
/// match). Negative inputs cannot reach it — every caller passes a sum of squares.
#[inline]
fn isqrt_i64(n: i64) -> i64 {
    forge_core_v3::fixed_point::isqrt_i64(n.max(0))
}

/// One family step on the meaning axis — sized to dominate the identity+role band.
pub const FAMILY_STEP: i32 = 0x4000;
/// One role step on the wrap-aware rotational axis — a facet, not a driver.
pub const ROLE_STEP: i32 = 0x200;
/// Half-width of one identity lane's tie-break band.
pub const IDENTITY_BAND: i32 = 0x800;

/// Fold a syntactic hash into the ±`IDENTITY_BAND` tie-break window (12-bit,
/// centered): breaks ties, can never out-vote one family step.
#[inline]
pub fn fold_identity(h: u64) -> i32 {
    ((h & 0xFFF) as i32) - IDENTITY_BAND
}

/// Is a name-absent orphan actually a RENAMED twin of live code, or genuinely lost?
///
/// The goldmine's ported FNV1a copy answered this with `dist == 0`, which oracle A
/// makes unreachable (a name-absent orphan can never embed onto a live cell), so it
/// reported shadow=0 over 22,817 orphans — arithmetic, not evidence (Sean 2026-07-20).
/// Under R1 the bands ARE the answer, no tuned constant: identity lives inside
/// ±[`IDENTITY_BAND`] on 3 lanes, one role is [`ROLE_STEP`], one family is
/// [`FAMILY_STEP`] — so distance alone says which bank you are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TwinVerdict {
    /// Same family AND role: only identity lanes differ — a renamed twin. Wire, don't re-port.
    ShadowStrong,
    /// Same family, different role: same subsystem meaning, different rotation. Inspect.
    ShadowWeak,
    /// Another family entirely — no live twin on this axis. Genuinely lost.
    Lost,
}

/// Widest squared distance two points can share when family AND role agree:
/// 3 identity lanes each spanning ±[`IDENTITY_BAND`].
pub const SAME_ROLE_MAX_SQ: i64 = 3 * (2 * IDENTITY_BAND as i64) * (2 * IDENTITY_BAND as i64);
/// One family step, squared — the bank between meanings.
pub const FAMILY_STEP_SQ: i64 = (FAMILY_STEP as i64) * (FAMILY_STEP as i64);

/// Read a nearest-cell squared distance as a twin verdict. Bands, not thresholds.
pub fn twin_verdict(dist_sq: i64) -> TwinVerdict {
    if dist_sq <= SAME_ROLE_MAX_SQ {
        TwinVerdict::ShadowStrong
    } else if dist_sq < FAMILY_STEP_SQ {
        TwinVerdict::ShadowWeak
    } else {
        TwinVerdict::Lost
    }
}

/// One role step, squared — the band between call-arity kinds on the wrap axis.
pub const ROLE_STEP_SQ: i64 = (ROLE_STEP as i64) * (ROLE_STEP as i64);

/// Signature-grain (R2) verdict. `refine` is clamped inside ±[`IDENTITY_BAND`]
/// by construction, so under content grain EVERY within-family distance sits
/// below [`SAME_ROLE_MAX_SQ`] and [`twin_verdict`] collapses to one band
/// (2026-07-20: 12512/12512 ShadowStrong). Content reads the SAME constants at
/// the grain below: the projection distance IS the content axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentVerdict {
    /// Projection-identical to a live cell — redundant content, wire don't re-port.
    Redundant,
    /// Under one role step: a rename/small-edit twin of live content. Inspect.
    NearTwin,
    /// Same family, genuinely different content — the diamond grain.
    Diamond,
    /// Another family entirely — no live neighbor on the meaning axis.
    Lost,
}

/// Read a nearest-cell squared distance at SIGNATURE grain. Bands, not thresholds.
pub fn content_verdict(dist_sq: i64) -> ContentVerdict {
    if dist_sq == 0 {
        ContentVerdict::Redundant
    } else if dist_sq < ROLE_STEP_SQ {
        ContentVerdict::NearTwin
    } else if dist_sq < FAMILY_STEP_SQ {
        ContentVerdict::Diamond
    } else {
        ContentVerdict::Lost
    }
}

/// Place a point with an explicit REFINEMENT on the identity lanes — the water
/// under the banks. The discrete `family`/`role` LEAD (they cut the channel);
/// `refine` is a continuous signal (a projection) clamped into ±`IDENTITY_BAND`
/// so it can order WITHIN a family/role cell but never breach a bank. This is the
/// collapse target: R2 (distributional) and R3 (model) feed their projection here.
pub fn map5_refine(family: Option<u8>, role: usize, refine: [i32; 3]) -> [i32; EMBED_DIM] {
    let mut p = [0i32; EMBED_DIM];
    p[FAMILY_LANE] = family.map_or(0, |f| f as i32 * FAMILY_STEP);
    p[ROLE_LANE] = role as i32 * ROLE_STEP;
    for (slot, &lane) in IDENTITY_LANES.iter().enumerate() {
        p[lane] = refine[slot].clamp(-IDENTITY_BAND, IDENTITY_BAND);
    }
    p
}

/// Place a point with identity from three hashes — R1's syntactic tie-break as
/// the refinement. Thin wrapper over [`map5_refine`]; the coarse `family`/`role`
/// still lead, the hash only breaks within-cell ties.
pub fn map5(family: Option<u8>, role: usize, identity: [u64; 3]) -> [i32; EMBED_DIM] {
    map5_refine(
        family,
        role,
        [fold_identity(identity[0]), fold_identity(identity[1]), fold_identity(identity[2])],
    )
}

/// Deprecated alias of [`FAMILY_STEP`] (pre-foundation name, kept for callers).
pub const RIVER_FAMILY_STEP: i32 = FAMILY_STEP;
/// Deprecated alias of [`IDENTITY_BAND`] (pre-foundation name, kept for callers).
pub const RIVER_TIEBREAK: i32 = IDENTITY_BAND;

/// Boundary-aware substring: `needle` present in `hay` flanked by string edges
/// or non-alphanumeric bytes — so "GPU" matches "GPU dispatch" but not "GPURKIT".
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

/// Task-vocabulary SYNONYMS -> CREE family: words that carry no family NAME but
/// belong to one. The MoE-DSP-GPU quadratic-quantized-LLM dispatch router vocab
/// (Sean's "router") -> WIRE (routing IS wiring); GPU stays its own family, so a
/// "gpu ..." query still homes to GPU. Checked AFTER the direct name scan so an
/// explicit family word wins. Extend when a live orient ray can't home a subject
/// (2026-07-18: the router census was family-None -> unreachable on tape AND
/// spine; Sean "fix the tool, never re-derive"). One row per vocabulary word.
const FAMILY_SYNONYMS: &[(&str, &str)] = &[
    ("ROUTERS", "WIRE"),
    ("ROUTER", "WIRE"),
    ("ROUTING", "WIRE"),
    ("ROUTE", "WIRE"),
    ("ROUTEEXPERT", "WIRE"),
    ("METAROUTER", "WIRE"),
    ("DISPATCH", "WIRE"),
    ("MOE", "WIRE"),
    ("QUADRATIC", "WIRE"),
    ("QUANTIZED", "WIRE"),
    ("BQ", "WIRE"),
];

/// R1 lexical classifier — first CREE family (doc order) whose NAME appears as a
/// whole word, else a `FAMILY_SYNONYMS` (private) task-word's family, else `None` (z=0).
pub fn river_family_of(line: &str) -> Option<u8> {
    let up = line.to_ascii_uppercase();
    if let Some(i) = CREE_FAMILIES
        .iter()
        .position(|fam| contains_word(&up, &fam.to_ascii_uppercase()))
    {
        return Some(i as u8);
    }
    FAMILY_SYNONYMS
        .iter()
        .find(|(alias, _)| contains_word(&up, alias))
        .and_then(|(_, fam)| CREE_FAMILIES.iter().position(|f| f == fam).map(|i| i as u8))
}

/// R1 role classifier — map the line to one of CREE's 4 rotations by keyword.
/// Returns a `CREE_ROLES` index (0=STATE, 1=ACT, 2=LAW, 3=PROOF). Default ACT.
pub fn river_role_of(line: &str) -> usize {
    let up = line.to_ascii_uppercase();
    const PROOF: [&str; 5] = ["COVERAGE", "PROOF", "VERIFIED", "PROVEN", "ORACLE"];
    const LAW: [&str; 3] = ["BAN", "LAW", "QUARANTINE"];
    const STATE: [&str; 5] = ["HEAD", "APERTURE", "STATE", "MAP", "ATTIC"];
    if PROOF.iter().any(|k| contains_word(&up, k)) {
        return 3;
    }
    if LAW.iter().any(|k| contains_word(&up, k)) {
        return 2;
    }
    if STATE.iter().any(|k| contains_word(&up, k)) {
        return 0;
    }
    1
}

/// RUNG 1 — lexical semantic embedding. Family on z (dominant, metric), role a
/// small metric offset on theta, syntactic identity (tag / payload / token-set
/// hash) demoted to the ±`RIVER_TIEBREAK` band. Same text -> same point; two
/// lines of the SAME family land NEAR each other regardless of wording.
pub fn embed_river_semantic_lexical(line: &str) -> [i32; EMBED_DIM] {
    let (tag, payload) = line.split_once('\t').unwrap_or((line, ""));
    let mut set: u64 = 0;
    for tok in line.split_whitespace() {
        set ^= fnv1a64(tok.as_bytes());
    }
    // R1 is now a thin caller of the foundation: family + role + 3 identity hashes.
    map5(
        river_family_of(line),
        river_role_of(line),
        [fnv1a64(tag.as_bytes()), fnv1a64(payload.as_bytes()), set],
    )
}

/// Build an R1 semantic codebook from raw river.idx text — the semantic twin of
/// `load_river_codebook`, same `(entry, line-text)` pairing so a hit traces back.
pub fn load_river_codebook_semantic(idx_text: &str) -> Vec<(CodeEntry, String)> {
    idx_text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| {
            (
                CodeEntry { id: i as u32, coords: embed_river_semantic_lexical(l) },
                l.to_string(),
            )
        })
        .collect()
}

/// Load + decode `.forge/river.idx` from disk into an R1 SEMANTIC codebook — the
/// semantic twin of `load_river_codebook_from_disk`, same decode law (binary-
/// packed bytes -> `from_utf8_lossy`, never a blind `read_to_string`).
pub fn load_river_codebook_semantic_from_disk(path: &Path) -> std::io::Result<Vec<(CodeEntry, String)>> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(load_river_codebook_semantic(&text))
}

// ── RUNG 2 — distributional (TF×IDF signed random projection) ─────────────────

/// `floor(log2(x))` for `x>=1`, integer-exact (no float `ln`). `x=0` -> 0.
#[inline]
fn ilog2_u32(x: u32) -> u32 {
    31 - x.max(1).leading_zeros()
}

/// Corpus inverse-document-frequency table for the distributional rung. One
/// "document" per non-blank river line; `df` = how many lines a token appears in
/// (set semantics). Common words weigh less, rare content words more. Built once
/// off the hot path (corpus preprocessing, like the index build itself).
pub struct RiverIdf {
    df: std::collections::HashMap<u64, u32>,
    n: u32,
}

/// Weight scale for one IDF step — sized so a single content-token difference
/// (>= `IDF_SCALE`*2) dwarfs the ±0x40 identity band (`fold_tiebreak_small`).
pub const IDF_SCALE: i32 = 0x400;

impl RiverIdf {
    /// Build the IDF table from raw river.idx text.
    pub fn build(idx_text: &str) -> RiverIdf {
        let mut df: std::collections::HashMap<u64, u32> = std::collections::HashMap::new();
        let mut n = 0u32;
        for line in idx_text.lines().filter(|l| !l.trim().is_empty()) {
            n += 1;
            let mut seen: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for tok in line.split_whitespace() {
                let h = fnv1a64(tok.as_bytes());
                if seen.insert(h) {
                    *df.entry(h).or_insert(0) += 1;
                }
            }
        }
        RiverIdf { df, n }
    }

    /// Integer IDF weight for a token hash: `IDF_SCALE * (1 + floor(log2(N/df+1)))`.
    /// Unknown / hapax tokens (df<=1) score highest; ubiquitous tokens lowest.
    #[inline]
    pub fn weight(&self, token_hash: u64) -> i32 {
        let df = (*self.df.get(&token_hash).unwrap_or(&0)).max(1);
        let ratio = self.n.max(1) / df + 1;
        IDF_SCALE * (1 + ilog2_u32(ratio) as i32)
    }
}

/// Stable Rademacher (±1) projection sign for token `h` on dim `d` (0|1) — the
/// random-projection direction, deterministic from the token hash.
#[inline]
fn rp_sign(h: u64, d: u32) -> i64 {
    if (h >> (13 + d * 13)) & 1 == 1 {
        1
    } else {
        -1
    }
}

/// Shift applied to a raw TF×IDF projection before it rides the identity band —
/// a judgment call (flagged, not hidden) sized so within-cell content spreads
/// across ±`IDENTITY_BAND` without pinning every line to the rails.
pub const REFINE_SHIFT: u32 = 3;

/// RUNG 2 — distributional embedding, as REFINEMENT under the banks. The discrete
/// Cree family/role LEAD (`map5_refine` sets lanes 2/3); the TF×IDF signed random
/// projection is the water — three dims onto the identity lanes, clamped to the
/// band so shared-content ordering happens WITHIN a family cell and can never
/// breach it. The language cuts the channel; the math flows inside it.
pub fn embed_river_semantic_distributional(line: &str, idf: &RiverIdf) -> [i32; EMBED_DIM] {
    map5_refine(
        river_family_of(line),
        river_role_of(line),
        refine_distributional(line, idf),
    )
}

/// The R2 water alone — the TF×IDF signed random projection, no banks. Split out
/// so a caller that owns its OWN family/role axis (a taxonomy the codebook cannot
/// infer from the line) can still ride the same refinement into `map5_refine`.
pub fn refine_distributional(line: &str, idf: &RiverIdf) -> [i32; 3] {
    let mut p = [0i64; 3];
    for tok in line.split_whitespace() {
        let h = fnv1a64(tok.as_bytes());
        let w = idf.weight(h) as i64; // repeated tokens sum -> term frequency
        for d in 0..3 {
            p[d] += w * rp_sign(h, d as u32);
        }
    }
    std::array::from_fn(|d| (p[d] >> REFINE_SHIFT).clamp(i32::MIN as i64, i32::MAX as i64) as i32)
}

// ── RUNG 3 — sovereign-model (learned BQ/steering code -> the same box) ────────

/// Scale for one bit's ±1 contribution in `project_bq_code` — keeps the 512-bit
/// projection in an integer-legible range.
pub const BQ_PROJ_SCALE: i32 = 0x40;

/// Read bit `b` (0..512) of a 64-byte BQ code, MSB-first within each byte.
#[inline]
fn bq_bit(code: &[u8; crate::BQ_BYTES], b: usize) -> bool {
    (code[b >> 3] >> (7 - (b & 7))) & 1 == 1
}

/// Project a 512-bit sovereign BQ/steering code onto the four metric lanes via a
/// signed random projection of its ±1 bit vector — HAMMING-LOCALITY preserving:
/// two learned codes differing in few bits land Euclidean-near, so the model's
/// own notion of "same meaning" (Hamming-close code) becomes 5D-near. `theta`
/// left 0 (wrap lane out of the Euclidean projection).
pub fn project_bq_code(code: &[u8; crate::BQ_BYTES]) -> [i32; EMBED_DIM] {
    let mut p = [0i64; 4];
    for b in 0..(crate::BQ_BYTES * 8) {
        let s: i64 = if bq_bit(code, b) { 1 } else { -1 };
        let dir = fnv1a64(&[b as u8, (b >> 8) as u8]);
        for d in 0..4 {
            p[d] += s * if (dir >> (7 + d * 11)) & 1 == 1 { 1 } else { -1 };
        }
    }
    let sc = BQ_PROJ_SCALE as i64;
    let c = |v: i64| (v * sc).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    [c(p[0]), c(p[1]), c(p[2]), 0, c(p[3])]
}

/// The seam the sovereign model plugs into: one LEARNED 512-bit code per line.
/// The real implementation is the student/teacher forward + `binarize_i8` (needs
/// weights on disk — `E:/nde-models`, `F:/output`). That leg stays \[UNVERIFIED\]
/// until the inference path is wired; the projection + fallback below are proven.
pub trait SovereignCoder {
    /// Return the learned 512-bit code for `line`.
    fn code(&self, line: &str) -> [u8; crate::BQ_BYTES];
}

/// Project a 512-bit learned code onto THREE identity-band refinement dims
/// (Hamming-locality preserving, same signed-projection as `project_bq_code` but
/// sized for the band) — the model's water INSIDE the language's banks.
fn bq_refine(code: &[u8; crate::BQ_BYTES]) -> [i32; 3] {
    let mut p = [0i64; 3];
    for b in 0..(crate::BQ_BYTES * 8) {
        let s: i64 = if bq_bit(code, b) { 1 } else { -1 };
        let dir = fnv1a64(&[b as u8, (b >> 8) as u8]);
        for d in 0..3 {
            p[d] += s * if (dir >> (7 + d * 11)) & 1 == 1 { 1 } else { -1 };
        }
    }
    std::array::from_fn(|d| {
        (p[d] * BQ_PROJ_SCALE as i64).clamp(i32::MIN as i64, i32::MAX as i64) as i32
    })
}

/// RUNG 3 — sovereign-model embedding, as REFINEMENT under the banks. The Cree
/// family/role STILL lead (from the line's own words — the language is the source,
/// the model is only the surveyor): with a loaded model the learned code refines
/// WITHIN the cell via `bq_refine`; without one it falls back to R2 distributional
/// (the Student->Oracle ladder). The machine never sets a bank, never a silent zero.
pub fn embed_river_semantic_model<C: SovereignCoder>(
    line: &str,
    coder: Option<&C>,
    idf: &RiverIdf,
) -> [i32; EMBED_DIM] {
    match coder {
        Some(c) => map5_refine(river_family_of(line), river_role_of(line), bq_refine(&c.code(line))),
        None => embed_river_semantic_distributional(line, idf),
    }
}

// ── Rung 2b: Λ_z + the state triple buffer ───────────────────────────────────

use forge_hal_clockspine::{ClockPlane, TripleBuffer};

/// Layer count forge-canvas's compositor actually has (`compositor.rs:11`,
/// `CompositorLayer.z: u8`, fixed `[CompositorLayer;8]`).
pub const COMPOSITOR_LAYER_COUNT: u8 = 8;

/// Λ_z: compress a CREE family index (0..24) down to a compositor-layer-safe
/// `u8` (0..=7) so GhostMoon's semantic z can ride the SAME range as
/// forge-canvas's real render z. Many-to-one BY CONSTRUCTION — 25 families
/// into 8 layers is lossy, and that lossiness is the honest cost of sharing
/// the buffer, not hidden. `family_index * 8 / 25`, floor division.
#[inline]
pub fn lambda_z_family_to_layer(family_index: u8) -> u8 {
    (family_index as u32 * COMPOSITOR_LAYER_COUNT as u32 / CREE_FAMILIES.len() as u32) as u8
}

/// GhostMoon -> compositor semantic-z bridge payload. Deliberately its OWN
/// type on its OWN `TripleBuffer`, NOT `forge_hal::collision_bridge::
/// ResonanceImpulse` — that bridge is dimensionless/concept-free BY LAW
/// (Alpha/Beta orthogonality; its own module doc has a `compile_fail` proof
/// forbidding exactly this kind of domain payload crossing it). This is a
/// different producer/consumer pair with a different contract: it DOES carry
/// a concept (which nearest-neighbor cell, which render layer), on purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostMoonImpulse {
    /// `CodeEntry.id` of the resolved nearest cell — a bare index, same
    /// "receiver decides what it indexes" contract as `ResonanceImpulse.idx`.
    pub nearest_id: u32,
    /// Λ_z-scaled, `0..=7` — safe to sit alongside `CompositorLayer.z`.
    pub layer_z: u8,
}

impl GhostMoonImpulse {
    /// The zero/unset impulse.
    pub const ZERO: GhostMoonImpulse = GhostMoonImpulse { nearest_id: 0, layer_z: 0 };
}

impl ClockPlane for GhostMoonImpulse {
    #[inline]
    fn copy_into(&self, dst: &mut Self) {
        *dst = *self;
    }
}

/// One-producer bridge for GhostMoon nearest-neighbor results. Mirrors
/// `collision_bridge::CollisionBridge`'s publish/try_take shape (the SAME
/// proven lock-free primitive) but is its own instance — a new
/// producer/consumer pair, not a repurposing of the Alpha/Beta bridge.
pub struct GhostMoonBridge {
    buf: TripleBuffer<GhostMoonImpulse>,
}

impl Default for GhostMoonBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl GhostMoonBridge {
    /// A fresh bridge, zeroed.
    pub fn new() -> Self {
        Self { buf: TripleBuffer::new(GhostMoonImpulse::ZERO) }
    }

    /// Producer side: publish a nearest-neighbor result. Returns the recycled
    /// old impulse (zero-alloc ping-pong, same contract as `TripleBuffer::publish`).
    #[inline]
    pub fn publish(&self, impulse: GhostMoonImpulse) -> GhostMoonImpulse {
        self.buf.publish(impulse)
    }

    /// Consumer side: try to copy the latest result into `dst`. Never blocks.
    #[inline]
    pub fn try_take(&self, last_gen: u64, dst: &mut GhostMoonImpulse) -> Option<u64> {
        self.buf.try_take(last_gen, dst)
    }
}

/// Λ_z⁻¹: recover the CREE family index (0..=24) from a stored z lane — the
/// exact inverse of `embed_cree_cell`'s `z = family * CREE_FAMILY_SCALE_MDEG`,
/// kept in this file so the pair cannot drift. Off-grid z (tie-break jitter,
/// non-cree rows) floors to the family below; negatives clamp to 0.
#[inline]
pub fn family_index_from_z(z: i32) -> u8 {
    (z / CREE_FAMILY_SCALE_MDEG).clamp(0, CREE_FAMILIES.len() as i32 - 1) as u8
}

/// The process-wide GhostMoon host bridge (the PIN's named orphan, drained
/// 2026-07-13). ONE producer — the daemon's raycast arm publishes each ray's
/// nearest cell; ONE consumer — the studio loop `try_take`s for the
/// mâmawapiwin deck. Lives here so both clocks of the ONE process see it.
static HOST_BRIDGE: std::sync::OnceLock<GhostMoonBridge> = std::sync::OnceLock::new();

/// Access the process-wide GhostMoon host bridge, lazily initialized.
pub fn host_bridge() -> &'static GhostMoonBridge {
    HOST_BRIDGE.get_or_init(GhostMoonBridge::new)
}

// ── Rung 2e: strain lanes — live [x,y,w] displacement on the QUERY origin ────
// (Sean 2026-07-10, strain-to-5d-lanes). The governor's measured StrainScore
// folds into the open lanes ON THE QUERY, never the codebook: a strained
// system genuinely tilts the ray while cells stay put. Since rung 2f the
// cells carry their own structured x/y/w, so the tilt moves the query
// RELATIVE to real lane geometry instead of across a zero plane.

use std::sync::atomic::{AtomicI32, Ordering};

/// One axis trip displaces its lane by this much — visible against the
/// ±0x8000 FNV window without dwarfing the syntactic lanes.
pub const STRAIN_LANE_STEP: i32 = 0x2000;
/// Lane ceiling — strain saturates inside the same window the embeddings use.
pub const STRAIN_LANE_MAX: i32 = 0x7FFF;

/// The published [x, y, w] strain lanes. ONE writer (the governor, 1s tick);
/// any query path reads. Relaxed: a gauge read, one-tick staleness is fine.
static STRAIN_LANES: [AtomicI32; 3] =
    [AtomicI32::new(0), AtomicI32::new(0), AtomicI32::new(0)];

/// φ_strain: fold the governor's measured axis counters into the 3 open lanes.
/// x = pressure-class (memory + channel), y = time-class (deadline + budget),
/// w = hygiene-class (orphans reaped + spool sweeps + sensor faults).
/// Integer-only, deterministic, saturating — no lane can leave the window.
pub fn strain_to_lanes(
    memory: u32,
    pressure: u32,
    deadline: u32,
    budget: u32,
    reaped: u32,
    spool: u32,
    faults: u32,
) -> [i32; 3] {
    let fold = |a: u32, b: u32, c: u32| -> i32 {
        let trips = a.saturating_add(b).saturating_add(c).min(16) as i32;
        trips.saturating_mul(STRAIN_LANE_STEP).min(STRAIN_LANE_MAX)
    };
    [
        fold(memory, pressure, 0),
        fold(deadline, budget, 0),
        fold(reaped, spool, faults),
    ]
}

/// Producer face — the governor publishes once per tick.
pub fn publish_strain_lanes(lanes: [i32; 3]) {
    for (slot, v) in STRAIN_LANES.iter().zip(lanes) {
        slot.store(v, Ordering::Relaxed);
    }
}

/// Consumer face — the current [x, y, w] for query paths.
pub fn strain_lanes_now() -> [i32; 3] {
    [
        STRAIN_LANES[0].load(Ordering::Relaxed),
        STRAIN_LANES[1].load(Ordering::Relaxed),
        STRAIN_LANES[2].load(Ordering::Relaxed),
    ]
}

/// Stamp a query point's open lanes with strain. z (idx 2) and theta (idx 3)
/// are untouched — theta stays wrap-aware, semantics stay the codebook's.
pub fn with_strain(mut point: [i32; EMBED_DIM], lanes: [i32; 3]) -> [i32; EMBED_DIM] {
    point[0] = point[0].saturating_add(lanes[0]);
    point[1] = point[1].saturating_add(lanes[1]);
    point[4] = point[4].saturating_add(lanes[2]);
    point
}

#[cfg(test)]
mod k_nearest_tests {
    use super::*;

    /// Entries on a straight line down lane 0, so true rank == id.
    fn line(n: u32) -> Vec<CodeEntry> {
        (0..n).map(|i| CodeEntry { id: i, coords: [i as i32 * 10, 0, 0, 0, 0] }).collect()
    }

    #[test]
    fn k1_agrees_with_nearest() {
        let cb = line(64);
        for q in [0i32, 37, 155, 640, -20] {
            let query = [q, 0, 0, 0, 0];
            let mut out = [(0u32, 0i64); 1];
            let n = k_nearest_into(&query, &cb, &mut out);
            assert_eq!(n, 1);
            assert_eq!(Some(out[0]), nearest(&query, &cb), "k=1 must BE nearest(), q={q}");
        }
    }

    #[test]
    fn results_are_ascending_and_are_the_true_k() {
        let cb = line(200);
        let got = k_nearest(&[1000, 0, 0, 0, 0], &cb, 16); // sits on id 100
        assert_eq!(got.len(), 16);
        for w in got.windows(2) {
            assert!(w[0].1 <= w[1].1, "ascending by distance");
        }
        // brute-force oracle: the true 16 nearest ids around 100
        let mut all: Vec<(u32, i64)> = cb
            .iter()
            .map(|e| (e.id, squared_distance(&[1000, 0, 0, 0, 0], &e.coords)))
            .collect();
        all.sort_by_key(|&(id, d)| (d, id));
        let want: Vec<u32> = all[..16].iter().map(|&(id, _)| id).collect();
        let mut got_ids: Vec<u32> = got.iter().map(|&(id, _)| id).collect();
        let mut want_sorted = want.clone();
        got_ids.sort_unstable();
        want_sorted.sort_unstable();
        assert_eq!(got_ids, want_sorted, "must be the TRUE k-nearest, not an approximation");
    }

    #[test]
    fn ties_keep_the_earlier_entry_like_nearest() {
        // three entries at the SAME point — first-minimum rule must hold.
        let cb: Vec<CodeEntry> =
            (0..3).map(|i| CodeEntry { id: i, coords: [5, 0, 0, 0, 0] }).collect();
        let got = k_nearest(&[5, 0, 0, 0, 0], &cb, 2);
        assert_eq!(got[0].0, 0, "earliest codebook entry wins a tie");
        assert_eq!(got[1].0, 1);
        assert_eq!(got[0].1, 0);
    }

    #[test]
    fn k_larger_than_codebook_fills_only_what_exists() {
        let cb = line(3);
        let mut out = [(0u32, 0i64); 8];
        assert_eq!(k_nearest_into(&[0, 0, 0, 0, 0], &cb, &mut out), 3);
        assert_eq!(k_nearest(&[0, 0, 0, 0, 0], &cb, 8).len(), 3);
    }

    #[test]
    fn empty_inputs_are_not_a_panic() {
        assert_eq!(k_nearest_into(&[0; EMBED_DIM], &[], &mut [(0, 0); 4]), 0);
        assert_eq!(k_nearest_into(&[0; EMBED_DIM], &line(9), &mut []), 0, "k=0 is legal, yields 0");
        assert!(k_nearest(&[0; EMBED_DIM], &[], 4).is_empty());
    }

    #[test]
    fn theta_lane_wraps_so_359_is_near_1() {
        // 1° and 359° are 2° apart, NOT 358° — the swarmray edge graph inherits
        // this from `lane_diff`, so a neighbour across the wrap must be found.
        let cb = vec![
            CodeEntry { id: 0, coords: [0, 0, 0, 359_000, 0] },
            CodeEntry { id: 1, coords: [0, 0, 0, 180_000, 0] },
            CodeEntry { id: 2, coords: [0, 0, 0, 90_000, 0] },
        ];
        let got = k_nearest(&[0, 0, 0, 1_000, 0], &cb, 1);
        assert_eq!(got[0].0, 0, "359° is the nearest neighbour of 1°");
    }

    #[test]
    fn deterministic_across_runs() {
        let cb = line(500);
        let q = [2_468, 0, 0, 0, 0];
        assert_eq!(k_nearest(&q, &cb, 16), k_nearest(&q, &cb, 16));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_codebook_returns_none() {
        assert!(nearest(&[0; EMBED_DIM], &[]).is_none());
    }

    // the goldmine's ported FNV1a oracle called every renamed twin LOST because it
    // tested dist==0, which oracle A makes unreachable (Sean 2026-07-20 audit).
    #[test]
    fn renamed_twin_reads_shadow_where_the_syntactic_oracle_said_lost() {
        // A code-symbol caller supplies its OWN family axis (goldmine's buckets);
        // `river_family_of` is river-doctrine vocabulary and reads None here, which
        // would collapse every symbol onto one z — the mirror of the dist==0 bug.
        let orphan = "Canvas_Window\trender_audio_studio_panel\tE:/.airgap/13forge/audio_studio_panel.rs:20";
        assert_eq!(river_family_of(orphan), None, "code rows are not river vocabulary");

        let cell = |family: u8, name: &str, path: &str| {
            map5(Some(family), 1, [fnv1a64(name.as_bytes()), fnv1a64(path.as_bytes()), 0])
        };
        // same bucket, different name+path = renamed twin; different bucket = stranger
        let o = cell(13, "render_audio_studio_panel", "E:/.airgap/13forge/audio_studio_panel.rs:20");
        let t = cell(13, "audio_workshop_panel_spec", "crates/forge-vix/src/generated_panels.rs:768");
        let s = cell(4, "extrude_sprite_mesh", "crates/forge-geo/src/extrude.rs:41");

        assert_ne!(twin_verdict(squared_distance(&o, &t)), TwinVerdict::Lost, "same bucket = twin");
        assert_eq!(twin_verdict(squared_distance(&o, &s)), TwinVerdict::Lost, "other bucket = lost");

        // the syntactic oracle the miner ported cannot see it: never exactly 0
        let twin_row = "Canvas_Window\taudio_workshop_panel_spec\tcrates/forge-vix/src/generated_panels.rs:768";
        assert_ne!(
            squared_distance(&embed_river_line(orphan), &embed_river_line(twin_row)),
            0,
            "dist==0 is why goldmine reported shadow=0 on all 22,817 orphans"
        );
    }

    #[test]
    fn twin_bands_are_ordered_and_derived_from_the_lane_constants() {
        assert!(SAME_ROLE_MAX_SQ < FAMILY_STEP_SQ, "identity band must never breach a family bank");
        assert_eq!(twin_verdict(0), TwinVerdict::ShadowStrong);
        assert_eq!(twin_verdict(SAME_ROLE_MAX_SQ), TwinVerdict::ShadowStrong);
        assert_eq!(twin_verdict(SAME_ROLE_MAX_SQ + 1), TwinVerdict::ShadowWeak);
        assert_eq!(twin_verdict(FAMILY_STEP_SQ), TwinVerdict::Lost);
    }

    #[test]
    fn content_bands_read_the_grain_the_twin_bands_collapse() {
        assert!(ROLE_STEP_SQ < SAME_ROLE_MAX_SQ, "content bands live under the R1 identity band");
        assert_eq!(content_verdict(0), ContentVerdict::Redundant);
        assert_eq!(content_verdict(ROLE_STEP_SQ - 1), ContentVerdict::NearTwin);
        assert_eq!(content_verdict(ROLE_STEP_SQ), ContentVerdict::Diamond);
        assert_eq!(content_verdict(SAME_ROLE_MAX_SQ), ContentVerdict::Diamond, "the collapsed band now splits");
        assert_eq!(content_verdict(FAMILY_STEP_SQ), ContentVerdict::Lost);
    }

    #[test]
    fn strain_fold_zero_is_zero_and_trips_are_visible() {
        assert_eq!(strain_to_lanes(0, 0, 0, 0, 0, 0, 0), [0, 0, 0]);
        // one memory trip -> x lane moves exactly one step, others stay 0
        assert_eq!(
            strain_to_lanes(1, 0, 0, 0, 0, 0, 0),
            [STRAIN_LANE_STEP, 0, 0]
        );
        // class separation: time-class and hygiene-class land on their own lanes
        assert_eq!(
            strain_to_lanes(0, 0, 1, 1, 2, 1, 1),
            [0, 2 * STRAIN_LANE_STEP, STRAIN_LANE_MAX.min(4 * STRAIN_LANE_STEP)]
        );
    }

    #[test]
    fn strain_fold_saturates_inside_the_window() {
        let lanes = strain_to_lanes(u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX, u32::MAX);
        for lane in lanes {
            assert!(lane <= STRAIN_LANE_MAX, "lane {lane} escaped the window");
        }
    }

    #[test]
    fn with_strain_touches_only_open_lanes() {
        let p = [10, 20, 30, 40, 50];
        let s = with_strain(p, [1, 2, 3]);
        assert_eq!(s, [11, 22, 30, 40, 53]); // z + theta byte-identical
    }

    #[test]
    fn publish_then_read_roundtrips() {
        publish_strain_lanes([7, 8, 9]);
        assert_eq!(strain_lanes_now(), [7, 8, 9]);
        publish_strain_lanes([0, 0, 0]); // leave the gauge clean for other tests
    }

    #[test]
    fn identical_point_is_zero_distance() {
        let p = [1, 2, 3, 4, 5];
        assert_eq!(squared_distance(&p, &p), 0);
    }

    #[test]
    fn nearest_picks_closest() {
        let codebook = [
            CodeEntry { id: 0, coords: [5, 5, 5, 5, 5] },
            CodeEntry { id: 1, coords: [10, 10, 10, 10, 10] },
            CodeEntry { id: 2, coords: [1, 0, 0, 0, 0] },
        ];
        let (id, d) = nearest(&[0, 0, 0, 0, 0], &codebook).unwrap();
        assert_eq!(id, 2);
        assert_eq!(d, 1);
    }

    #[test]
    fn first_minimum_wins_tie() {
        // Two equidistant candidates — deterministic first-seen tie-break,
        // never ambiguous, never a stall.
        let codebook = [
            CodeEntry { id: 5, coords: [1, 0, 0, 0, 0] },
            CodeEntry { id: 9, coords: [-1, 0, 0, 0, 0] },
        ];
        let (id, d) = nearest(&[0, 0, 0, 0, 0], &codebook).unwrap();
        assert_eq!(id, 5);
        assert_eq!(d, 1);
    }

    /// PIN's own gate language: "prove collapse-kills-stall on one lane."
    /// Concentration-of-measure means a dense high-D codebook can produce a
    /// swarm of near-tied distances — the "paralyzed agent" stall PIN names.
    /// This proves `nearest` still resolves to exactly ONE deterministic
    /// answer under that swarm, never panics, never returns ambiguity.
    #[test]
    fn collapse_kills_stall_under_dense_near_ties() {
        let mut codebook = Vec::new();
        for i in 0..2000u32 {
            // All 2000 candidates sit on one of two mirrored shells, exactly
            // tied with each other (distance 80) but never on the origin —
            // a deliberate near-tie swarm that must not eclipse the true winner.
            let jitter: i32 = if i % 2 == 0 { 4 } else { -4 };
            codebook.push(CodeEntry {
                id: i,
                coords: [jitter, jitter, jitter, jitter, jitter],
            });
        }
        // One true winner, planted dead-center, strictly closer than the swarm.
        codebook.push(CodeEntry { id: 9999, coords: [0, 0, 0, 0, 0] });

        let (id, d) = nearest(&[0, 0, 0, 0, 0], &codebook).unwrap();
        assert_eq!(id, 9999, "the swarm must not out-vote the true nearest point");
        assert_eq!(d, 0);
    }

    #[test]
    fn embed_dim_is_five() {
        assert_eq!(EMBED_DIM, 5); // [x, y, z, theta, w] — GhostMoon 5D box
    }

    #[test]
    fn theta_wraps_at_360_degrees() {
        // 1000 mdeg (1°) and 359000 mdeg (359°) are 2000 mdeg (2°) apart on
        // a circle, NOT 358000 mdeg apart as flat linear diff would claim.
        let a = [0, 0, 0, 1_000, 0];
        let b = [0, 0, 0, 359_000, 0];
        assert_eq!(squared_distance(&a, &b), 2_000i64 * 2_000);
    }

    #[test]
    fn theta_wrap_does_not_change_small_values() {
        // Non-angular (e.g. rung-2a hash) data stays in a small range where
        // wrap-aware and flat linear diff agree exactly — no regression.
        let a = [1, 2, 3, 500, 5];
        let b = [1, 2, 3, -500, 5];
        assert_eq!(squared_distance(&a, &b), 1_000i64 * 1_000);
    }

    #[test]
    fn nearest_prefers_true_angular_neighbor_across_the_seam() {
        let codebook = [
            CodeEntry { id: 0, coords: [0, 0, 0, 358_000, 0] }, // 2° away, wrapped
            CodeEntry { id: 1, coords: [0, 0, 0, 90_000, 0] },  // 88° away, flat
        ];
        let (id, _) = nearest(&[0, 0, 0, 0, 0], &codebook).unwrap();
        assert_eq!(id, 0, "358° must read as 2° away from 0°, the true nearest angle");
    }

    #[test]
    fn embed_river_line_is_deterministic() {
        let a = embed_river_line("APERTURE\tforge-audio");
        let b = embed_river_line("APERTURE\tforge-audio");
        assert_eq!(a, b);
    }

    #[test]
    fn embed_river_line_distinguishes_distinct_lines() {
        let a = embed_river_line("APERTURE\tforge-audio");
        let b = embed_river_line("APERTURE\tforge-render");
        assert_ne!(a, b, "one changed byte must not collide (FNV1a avalanche)");
    }

    // ── Rung 2f: effective rank 5 — feature-lane witnesses. Each witness is
    // IMPOSSIBLE on the v1 rank-1 curve (one hash rotated into all lanes:
    // lanes could never agree while others differed, by construction). ──────

    #[test]
    fn rank5_same_tag_rows_share_the_cluster_lane_only() {
        let a = embed_river_line("SPILL\tsand:query\t.forge/spill/a.grain");
        let b = embed_river_line("SPILL\tsand:vixi\t.forge/spill/b.grain");
        assert_eq!(a[0], b[0], "same row kind must share the x cluster lane");
        assert_ne!(a[1], b[1], "different payload must move the y lane");
        let c = embed_river_line("MAP\tsand:query\t.forge/spill/a.grain");
        assert_ne!(a[0], c[0], "different row kind must move the x lane");
        assert_eq!(a[1], c[1], "identical payload must KEEP y while x moves");
    }

    #[test]
    fn rank5_token_reorder_moves_order_lane_keeps_set_lane() {
        let a = embed_river_line("MAP\tforge ml");
        let b = embed_river_line("MAP\tml forge");
        assert_eq!(a[0], b[0], "tag unchanged -> x holds");
        assert_eq!(a[2], b[2], "same length + tabs -> z holds");
        assert_eq!(a[4], b[4], "same token SET -> w holds");
        assert_ne!(a[3], b[3], "token ORDER changed -> theta moves");
        assert_ne!(a[1], b[1], "payload text changed -> y moves");
    }

    #[test]
    fn rank5_shape_lane_is_metric_not_hashed() {
        let a = embed_river_line("AAAA");
        let b = embed_river_line("BBBB");
        assert_eq!(a[2], b[2], "equal length + tabs -> identical z");
        let long = embed_river_line("AAAAAAAAAAAAAAAA");
        assert!(long[2] > a[2], "longer line -> strictly larger z");
    }

    #[test]
    fn cree_open_lanes_are_exercised_and_never_swamp_semantics() {
        let codebook = load_cree_codebook();
        for lane in [0usize, 1, 4] {
            let mut vals: Vec<i32> = codebook.iter().map(|(e, _)| e.coords[lane]).collect();
            vals.sort_unstable();
            vals.dedup();
            assert!(vals.len() >= 2, "cree lane {lane} still unexercised");
        }
        for (e, cell) in &codebook {
            assert_eq!(
                e.coords[0],
                embed_cree_cell(cell.family_index, 0)[0],
                "x must be family-constant (cluster contract, mirrors the river tag lane)"
            );
            assert!(
                e.coords[0].abs() <= CREE_OPEN_LANE_WINDOW
                    && e.coords[1].abs() <= CREE_OPEN_LANE_WINDOW
                    && e.coords[4].abs() <= CREE_OPEN_LANE_WINDOW,
                "open lane escaped its ceiling on {} {}", cell.family, cell.role
            );
            let open_cost = (e.coords[0] as i64).pow(2)
                + (e.coords[1] as i64).pow(2)
                + (e.coords[4] as i64).pow(2);
            assert!(
                open_cost < (CREE_FAMILY_SCALE_MDEG as i64).pow(2) / 16,
                "open-lane cost {open_cost} swamps family semantics"
            );
        }
    }

    #[test]
    fn load_river_codebook_skips_blank_lines() {
        let text = "HEAD\tfoo\n\nAPERTURE\tbar\n";
        let cb = load_river_codebook(text);
        assert_eq!(cb.len(), 2);
        assert_eq!(cb[0].1, "HEAD\tfoo");
        assert_eq!(cb[1].1, "APERTURE\tbar");
    }

    // ── Rung 2c: CREE.md family+rotation codebook ──────────────────────────

    #[test]
    fn cree_codebook_has_100_cells() {
        assert_eq!(load_cree_codebook().len(), 25 * 4);
    }

    #[test]
    fn cree_codebook_cells_are_all_distinct_points() {
        let codebook = load_cree_codebook();
        for i in 0..codebook.len() {
            for j in (i + 1)..codebook.len() {
                assert_ne!(
                    codebook[i].0.coords, codebook[j].0.coords,
                    "cell {} ({} {}) collides with cell {} ({} {})",
                    i, codebook[i].1.family, codebook[i].1.role,
                    j, codebook[j].1.family, codebook[j].1.role,
                );
            }
        }
    }

    /// This is the bug `CREE_FAMILY_SCALE_MDEG` exists to prevent: without
    /// scaling, `z` (0..24) is invisible next to `theta` (0..360000) in
    /// squared-Euclidean, so a query would resolve to "nearest rotation,
    /// ANY family" instead of the actual nearest cell. Prove family still
    /// wins when it should: same role, one family over, must beat a
    /// different role in the query's own family.
    #[test]
    fn family_axis_is_not_swamped_by_theta() {
        let codebook = load_cree_codebook();
        let entries: Vec<CodeEntry> = codebook.iter().map(|(e, _)| *e).collect();

        // Query = exact PROOF cell of family index 10 (PROOF, ironically).
        let query = embed_cree_cell(10, 3); // role_idx 3 = PROOF (0°)
        let (hit_id, dist) = nearest(&query, &entries).unwrap();
        assert_eq!(dist, 0);
        assert_eq!(codebook[hit_id as usize].1.family, "PROOF");
        assert_eq!(codebook[hit_id as usize].1.role, "PROOF");
    }

    // ── Rung 2g: RUNG 1 semantic river embedding — recall oracles ───────────

    #[test]
    fn r1_classifies_family_by_keyword() {
        assert_eq!(river_family_of("MAP\tGPU shader warden"), Some(1)); // GPU
        assert_eq!(river_family_of("HEAD\tCLOCK tick beat"), Some(0)); // CLOCK
        assert_eq!(river_family_of("COVERAGE\tSOUND voice pan"), Some(5)); // SOUND
        assert_eq!(river_family_of("just some prose no doctrine"), None);
    }

    #[test]
    fn routing_vocab_homes_to_wire_via_synonym() {
        // 2026-07-18: router census was family-None -> unreachable on the orient
        // ray. FAMILY_SYNONYMS folds routing vocab to WIRE so path rows, MAP rows,
        // and free-text queries co-locate on one family axis and home.
        let wire = CREE_FAMILIES.iter().position(|f| *f == "WIRE").unwrap() as u8;
        for line in [
            "crates/forge-book/src/routers.rs\thead rev=3 ts=1", // tape path row
            "MAP\tforge-book\trouter census 7 axes map\tLIVE\trouters.rs", // spine MAP
            "which router handles dispatch, RouteExpert MoE",    // free-text query
            "moe quadratic quantized bq metarouter",             // Sean's LLM-router vocab
        ] {
            assert_eq!(river_family_of(line), Some(wire), "routing must home to WIRE: {line}");
        }
        // an explicit family NAME still wins over a synonym: a GPU-angle query
        // (Sean's "moe-dsp-gpu ...") homes to GPU, not WIRE — both are near the
        // MoE router, so the census carries a GPU-homed pointer row too.
        let gpu = CREE_FAMILIES.iter().position(|f| *f == "GPU").unwrap() as u8;
        assert_eq!(river_family_of("GPU MoE quantized router"), Some(gpu));
        // non-routing prose stays neutral (z=0).
        assert_eq!(river_family_of("twin generator fold"), None);
    }

    #[test]
    fn orient_777_fan_masters_the_router_row_one_ray_cannot() {
        // The 7-7-7 wire: a fan of lateral rays cross-referenced homes the router
        // census row (id 0), where a lone cross-family ray ranks a neutral decoy.
        let rows = [
            "MAP\tforge-book\trouter census dispatch moe RouteExpert quantized\tLIVE\trouters.rs",
            "TOOL\ttape\tsession chain read forge-ump",
            "MAP\tforge-daemon\ttrit-ham live repo query",
            "HEAD\tforge-harmonics harmonic threads current",
            "MAP\tforge-vix\tui tickbar kit paint rail",
        ];
        let book: Vec<CodeEntry> = rows
            .iter()
            .enumerate()
            .map(|(i, l)| CodeEntry { id: i as u32, coords: embed_river_semantic_lexical(l) })
            .collect();
        let subject = embed_river_semantic_lexical("which router moe dispatch RouteExpert quantized bq");
        let origins: Vec<[i32; EMBED_DIM]> = [
            "HEAD forge-harmonics aspire aperture",
            "forge-ml inference expert routing",
            "task-graph fold twin schema",
            "metarouter bq quantized centroid",
            "RouteExpert NdeEvent seven",
            "tiers pipeline student master",
            "aspire lateral candidate spec",
        ]
        .iter()
        .map(|s| embed_river_semantic_lexical(s))
        .collect();

        let ranked = orient_777(&subject, &origins, &book, 2);
        assert_eq!(ranked[0].0, 0, "7-7-7 fan masters the router row: {ranked:?}");
        assert!(ranked[0].1 >= 2, "consensus needs >=2 votes: {ranked:?}");
    }

    /// The R1 recall oracle: on a labeled mini-corpus, the nearest NON-SELF line
    /// to any line must SHARE its doctrine family — proof that meaning (family on
    /// z) out-votes wording (syntactic hash lanes). Impossible on the syntactic
    /// `embed_river_line`, where avalanche scatters same-topic lines apart.
    #[test]
    fn r1_same_family_beats_cross_family_recall() {
        let corpus = [
            "MAP\tGPU shader warden dispatch", // 0: GPU
            "BUILD\tGPU matmul kernel tiling", // 1: GPU
            "MAP\tSOUND voice envelope pan",   // 2: SOUND
            "HEAD\tSOUND mixer voice level",   // 3: SOUND
            "HEAD\tCLOCK tick two-drum phase", // 4: CLOCK
            "MAP\tCLOCK skew seam window",     // 5: CLOCK
        ];
        let entries: Vec<CodeEntry> = corpus
            .iter()
            .enumerate()
            .map(|(i, l)| CodeEntry { id: i as u32, coords: embed_river_semantic_lexical(l) })
            .collect();
        for i in 0..corpus.len() {
            let q = embed_river_semantic_lexical(corpus[i]);
            let mut best = (usize::MAX, i64::MAX);
            for (j, e) in entries.iter().enumerate() {
                if j == i {
                    continue;
                }
                let d = squared_distance(&q, &e.coords);
                if d < best.1 {
                    best = (j, d);
                }
            }
            assert_eq!(
                river_family_of(corpus[best.0]),
                river_family_of(corpus[i]),
                "line {i} ({}) nearest non-self is line {} ({}) of a DIFFERENT family — meaning failed to out-vote wording",
                corpus[i], best.0, corpus[best.0]
            );
        }
    }

    /// The magnitude budget, proven: one family apart ALWAYS beats any
    /// within-family wording change, and within-family variation stays inside the
    /// tie-break band (family can never be a hash-collision artifact).
    #[test]
    fn r1_family_step_dominates_the_tiebreak_band() {
        let a = embed_river_semantic_lexical("MAP\tGPU one"); // family 1
        let b = embed_river_semantic_lexical("MAP\tGPU two three"); // family 1, diff wording
        let c = embed_river_semantic_lexical("MAP\tSOUND one"); // family 5
        let within = squared_distance(&a, &b);
        let across = squared_distance(&a, &c);
        assert!(within < across, "within-family {within} must be < cross-family {across}");
        let band = (RIVER_TIEBREAK as i64 * 2).pow(2) * 3;
        assert!(within <= band, "within-family variation {within} escaped the tie-break band {band}");
    }

    /// The FOUNDATION invariant: one family step out-votes the whole identity+role
    /// band unconditionally, and `map5` really places meaning on lanes 2/3 and
    /// identity on 0/1/4. Every rung built on `map5` inherits this for free.
    #[test]
    fn foundation_meaning_out_votes_identity() {
        let band = (2 * IDENTITY_BAND as i64).pow(2) * IDENTITY_LANES.len() as i64
            + (3 * ROLE_STEP as i64).pow(2);
        let one_family = (FAMILY_STEP as i64).pow(2);
        assert!(one_family > band, "family step² {one_family} must exceed identity+role band² {band}");

        let a = map5(Some(1), 1, [1, 2, 3]);
        let same_fam = map5(Some(1), 1, [999, 888, 777]); // wildly different identity
        let one_over = map5(Some(2), 1, [1, 2, 3]);
        assert!(
            squared_distance(&a, &same_fam) < squared_distance(&a, &one_over),
            "same-family (any identity) must out-rank one-family-over"
        );

        assert_eq!(a[FAMILY_LANE], FAMILY_STEP, "family must ride lane 2");
        assert_eq!(a[ROLE_LANE], ROLE_STEP, "role must ride lane 3 (theta)");
        for &lane in &IDENTITY_LANES {
            assert!(a[lane].abs() <= IDENTITY_BAND, "identity lane {lane} escaped its band");
        }
    }

    /// R1 built on the foundation must be byte-identical to its pre-foundation
    /// coords — the migration carries the proven behaviour, changes no geometry.
    #[test]
    fn r1_on_foundation_matches_hand_placement() {
        for line in ["MAP\tGPU shader warden", "COVERAGE\tSOUND voice pan", "plain prose"] {
            let (tag, payload) = line.split_once('\t').unwrap_or((line, ""));
            let mut set: u64 = 0;
            for tok in line.split_whitespace() {
                set ^= fnv1a64(tok.as_bytes());
            }
            let hand = map5(
                river_family_of(line),
                river_role_of(line),
                [fnv1a64(tag.as_bytes()), fnv1a64(payload.as_bytes()), set],
            );
            assert_eq!(embed_river_semantic_lexical(line), hand, "R1 must equal its foundation placement for {line}");
        }
    }

    // ── RUNG 2 — distributional embedding — recall oracles ──────────────────

    #[test]
    fn r2_idf_ranks_rare_above_common() {
        let corpus = "MAP\tthe worms arena\nMAP\tthe sqlite page\nMAP\tthe render frame\n";
        let idf = RiverIdf::build(corpus);
        let common = idf.weight(fnv1a64(b"the")); // in all 3 docs
        let rare = idf.weight(fnv1a64(b"worms")); // 1 doc
        assert!(rare > common, "rare token idf {rare} must exceed common {common}");
    }

    /// R2 as REFINEMENT: hold the bank (all GPU family, all MAP/STATE role) and
    /// prove the distributional projection orders lines WITHIN the cell by shared
    /// content — the water flowing inside one channel.
    #[test]
    fn r2_refines_within_a_family_by_content() {
        let corpus = [
            "MAP\tGPU render frame shade warm", // 0
            "MAP\tGPU render frame shade cool", // 1: shares render,frame,shade with 0
            "MAP\tGPU sqlite pragma page fast", // 2
            "MAP\tGPU sqlite pragma page slow", // 3: shares sqlite,pragma,page with 2
        ];
        let idf = RiverIdf::build(&corpus.join("\n"));
        for l in corpus {
            assert_eq!(river_family_of(l), Some(1), "corpus must hold the GPU bank");
            assert_eq!(river_role_of(l), 0, "corpus must hold the STATE role bank");
        }
        let entries: Vec<CodeEntry> = corpus
            .iter()
            .enumerate()
            .map(|(i, l)| CodeEntry { id: i as u32, coords: embed_river_semantic_distributional(l, &idf) })
            .collect();
        for i in 0..corpus.len() {
            let q = embed_river_semantic_distributional(corpus[i], &idf);
            let mut best = (usize::MAX, i64::MAX);
            for (j, e) in entries.iter().enumerate() {
                if j == i {
                    continue;
                }
                let d = squared_distance(&q, &e.coords);
                if d < best.1 {
                    best = (j, d);
                }
            }
            assert_eq!(
                best.0,
                i ^ 1,
                "within the GPU cell, {} must refine nearest to its content-mate",
                corpus[i]
            );
        }
    }

    /// Banks LEAD: a same-family line with DISJOINT content beats an other-family
    /// line with IDENTICAL content — the family channel is never breached, no
    /// matter how strong the distributional refinement.
    #[test]
    fn r2_family_bank_beats_shared_content() {
        let idf = RiverIdf::build("MAP\tGPU render frame\nMAP\tSOUND render frame\n");
        let gpu_a = embed_river_semantic_distributional("MAP\tGPU render frame", &idf);
        let gpu_b = embed_river_semantic_distributional("MAP\tGPU tiling kernel warp", &idf);
        let sound = embed_river_semantic_distributional("MAP\tSOUND render frame", &idf);
        assert!(
            squared_distance(&gpu_a, &gpu_b) < squared_distance(&gpu_a, &sound),
            "same-family disjoint-content must beat other-family identical-content — banks lead"
        );
    }

    // ── RUNG 3 — sovereign-model embedding — recall oracles ─────────────────

    /// The projection proof, grounded in a REAL learned artifact: take a base
    /// code from the on-disk `meta_router.bqr` centroid (a trained centroid, real
    /// model weights) and prove a 1-bit-apart code projects strictly NEARER than
    /// a 200-bit-apart one — Hamming locality survives the projection. Falls back
    /// to a fixed base if the .bqr is absent (v3: `F:\v3\meta_router.bqr` checked
    /// absent 2026-08-14 — the math is identical either way).
    #[test]
    fn r3_projection_preserves_hamming_locality() {
        let mut base = [0u8; crate::BQ_BYTES];
        let bqr = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../meta_router.bqr"));
        if let Ok(r) = crate::BqRouter::load(bqr, 512) {
            base.copy_from_slice(&r.centroid(0).bits); // real trained centroid
        } else {
            for (i, b) in base.iter_mut().enumerate() {
                *b = (i as u8).wrapping_mul(37);
            }
        }
        let near = {
            let mut c = base;
            c[0] ^= 0b0000_0001; // 1 bit
            c
        };
        let far = {
            let mut c = base;
            for byte in c.iter_mut().take(25) {
                *byte ^= 0xFF; // 200 bits
            }
            c
        };
        let (pb, pn, pf) = (project_bq_code(&base), project_bq_code(&near), project_bq_code(&far));
        let dn = squared_distance(&pb, &pn);
        let df = squared_distance(&pb, &pf);
        assert!(dn > 0, "a flipped bit must move the projection");
        assert!(dn < df, "1-bit-apart code {dn} must project nearer than 200-bit-apart {df}");
    }

    /// A fixture coder standing in for the sovereign model: it returns codes that
    /// are Hamming-CLOSE for same-topic lines (topic hash fills the high bytes,
    /// wording perturbs one low byte) — exactly the contract the real forward
    /// must satisfy. Proves R3 clusters IF the model gives topical codes.
    struct HammingTopicCoder;
    impl SovereignCoder for HammingTopicCoder {
        fn code(&self, line: &str) -> [u8; crate::BQ_BYTES] {
            let topic = line
                .split_once('\t')
                .map(|(_, p)| p)
                .unwrap_or(line)
                .split_whitespace()
                .next()
                .unwrap_or("");
            let mut code = [0u8; crate::BQ_BYTES];
            let th = fnv1a64(topic.as_bytes());
            for (i, byte) in code.iter_mut().take(56).enumerate() {
                *byte = ((th >> ((i % 8) * 8)) & 0xFF) as u8; // topic-dominant bytes
            }
            code[56] = (fnv1a64(line.as_bytes()) & 0xFF) as u8; // wording perturbation
            code
        }
    }

    #[test]
    fn r3_learned_codes_cluster_by_topic_recall() {
        let idf = RiverIdf::build(""); // unused in the model arm
        let coder = HammingTopicCoder;
        // Same bank held (all MAP/STATE, family None) so the LEARNED code alone
        // refines within the cell — topic-close codes must cluster.
        let corpus = [
            "MAP\tworms arena physics",   // 0: topic worms
            "MAP\tworms render frame",    // 1: topic worms
            "MAP\tsqlite pragma journal", // 2: topic sqlite
            "MAP\tsqlite vacuum page",    // 3: topic sqlite
        ];
        let entries: Vec<CodeEntry> = corpus
            .iter()
            .enumerate()
            .map(|(i, l)| CodeEntry { id: i as u32, coords: embed_river_semantic_model(l, Some(&coder), &idf) })
            .collect();
        for i in 0..corpus.len() {
            let q = embed_river_semantic_model(corpus[i], Some(&coder), &idf);
            let mut best = (usize::MAX, i64::MAX);
            for (j, e) in entries.iter().enumerate() {
                if j == i {
                    continue;
                }
                let d = squared_distance(&q, &e.coords);
                if d < best.1 {
                    best = (j, d);
                }
            }
            let mate = i ^ 1;
            assert_eq!(
                best.0, mate,
                "R3 line {i} ({}) nearest is {} ({}), not topic-mate {}",
                corpus[i], best.0, corpus[best.0], mate
            );
        }
    }

    /// Graceful fallback: no model -> R3 is EXACTLY R2, never a silent zero and
    /// never a regression (the Student->Oracle ladder's cheaper tier).
    #[test]
    fn r3_falls_back_to_distributional_without_a_model() {
        let idf = RiverIdf::build("MAP\tworms arena\nMAP\tsqlite page\n");
        let line = "MAP\tworms arena physics";
        let got = embed_river_semantic_model::<HammingTopicCoder>(line, None, &idf);
        let want = embed_river_semantic_distributional(line, &idf);
        assert_eq!(got, want, "no-model R3 must equal the R2 distributional tier");
    }

    /// The actual payoff: on CREE.md's REAL grid, a query near the ▷/▽ seam
    /// (0°/360°) must resolve to PROOF (0°), not STATE (270°) — flat linear
    /// diff would get this wrong (see `theta_wraps_at_360_degrees` for the
    /// synthetic version; this is the same bug on real doc data).
    #[test]
    fn cree_grid_resolves_across_the_rotation_seam() {
        let codebook = load_cree_codebook();
        let entries: Vec<CodeEntry> = codebook.iter().map(|(e, _)| *e).collect();

        // Family 6 (VISION), query 10° past the PROOF/STATE seam (350°).
        let family_z = 6i32 * CREE_FAMILY_SCALE_MDEG;
        let query = [0, 0, family_z, 350_000, 0];
        let (hit_id, _) = nearest(&query, &entries).unwrap();
        let cell = &codebook[hit_id as usize].1;
        assert_eq!(cell.family, "VISION");
        assert_eq!(cell.role, "PROOF", "350° is 10° from PROOF's 0°, not 80° from STATE's 270°");
    }
}
