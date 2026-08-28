//! `#vixi:geom v1` / `.ptex` PaTeX geometry dialect — parse + AOT bake to a
//! Pexil lattice + annotation table (ORACLE-C spec sec-2, gates sec-11).
//! Sibling of [`crate::timeline`]; bake-time only — runtime never parses.

use forge_core_v3::atom::{CellOrdinal, Pexil, TritCell5D, ValidityMask};

/// The B-locked PaTeX pane bound: a geom surface is at most 71 columns.
pub const GEOM_MAX_COLS: u16 = 71;
/// Sentinel byte for a cell no front row authored.
pub const HELD_BLANK: u8 = 255;
/// Reserved sentinel — never emitted by authored content (spec sec-2).
pub const ANIKWACAS: u8 = 254;

/// One legend binding: a source glyph and its five balanced trits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomLegendRow {
    /// The ASCII source glyph.
    pub ch: char,
    /// The five perceptual axes (A1 Ground..A5 Witness), each -1|0|+1.
    pub axes: [i8; 5],
}

/// One under-face override: the same cell re-stated with different axes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomUnderRow {
    /// Cell column.
    pub col: u16,
    /// Cell row.
    pub row: u16,
    /// The under-face axes for this cell.
    pub axes: [i8; 5],
}

/// What an annotation is anchored to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeomAnnoTarget {
    /// A single cell `col.row`.
    Cell {
        /// Cell column.
        col: u16,
        /// Cell row.
        row: u16,
    },
    /// A half-open region `[c0,c1) x [r0,r1)`.
    Region {
        /// Start column (inclusive).
        c0: u16,
        /// Start row (inclusive).
        r0: u16,
        /// End column (exclusive).
        c1: u16,
        /// End row (exclusive).
        r1: u16,
    },
}

/// What an annotation carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeomAnnoKind {
    /// A cart-key binding (words resolve at load, never here).
    Bind(
        /// The cart key path.
        String,
    ),
    /// An out-of-band sentinel mark.
    Sentinel {
        /// The sentinel byte (243..=253, 255 — 254 is unauthorable).
        byte: u8,
        /// Optional `out="..."` destination word.
        out: Option<String>,
    },
}

/// One annotation table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomAnno {
    /// Anchor.
    pub target: GeomAnnoTarget,
    /// Payload.
    pub kind: GeomAnnoKind,
}

/// The five sec-11 gates, integerized (no floats in the IR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeomGates {
    /// Minimum contrast ratio in thousandths (4.5 -> 4500).
    pub contrast_min_m3: u32,
    /// Minimum hit target in MilliUnit-derived `mu(...)` units.
    pub hit_target_min_mu: u32,
    /// `runtime_parse = forbidden` was stated.
    pub runtime_parse_forbidden: bool,
    /// `float_in_ir = forbidden` was stated.
    pub float_in_ir_forbidden: bool,
    /// `seed_deterministic = required` was stated.
    pub seed_deterministic: bool,
}

/// A fully-parsed `.geom.vixi` document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomDoc {
    /// Header version from `#vixi:geom vN`.
    pub dialect_version: u32,
    /// Surface name.
    pub surface: String,
    /// Cart word.
    pub cart: String,
    /// Band this surface embeds in.
    pub band: String,
    /// Declared columns (<= [`GEOM_MAX_COLS`]).
    pub width: u16,
    /// Declared rows.
    pub height: u16,
    /// Legend bindings.
    pub legend: Vec<GeomLegendRow>,
    /// Front-face rows, each exactly `width` chars.
    pub front: Vec<String>,
    /// Under-face overrides.
    pub under: Vec<GeomUnderRow>,
    /// Annotation rows.
    pub annos: Vec<GeomAnno>,
    /// The five gates.
    pub gates: GeomGates,
}

/// The baked artifact: lattice bytes + annotation table + determinism receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomBake {
    /// Surface name.
    pub surface: String,
    /// Cart word.
    pub cart: String,
    /// Band word.
    pub band: String,
    /// Columns.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// `width*height` packed lattice bytes (front face); 255 = held-blank.
    pub lattice: Vec<u8>,
    /// Under-face cell overrides `(col, row, byte)`.
    pub under_overrides: Vec<(u16, u16, u8)>,
    /// Annotation table, carried through verbatim.
    pub annos: Vec<GeomAnno>,
    /// Legend as `(glyph, byte)` — the reverse ASCII projection.
    pub legend: Vec<(char, u8)>,
    /// blake3-u64 of the source text — the seed-determinism receipt.
    pub source_hash: u64,
}

/// Refusal with the source line that caused it (line 0 = document-level).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeomRefusal {
    /// 1-based source line, 0 for document-level refusals.
    pub line: u32,
    /// The worded refusal.
    pub what: String,
}

impl GeomRefusal {
    fn at(line: u32, what: impl Into<String>) -> Self {
        Self { line, what: what.into() }
    }
}

impl core::fmt::Display for GeomRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "geom line {}: {}", self.line, self.what)
    }
}

/// Sentinel word -> byte (spec sec-2 assignments). 254 has no word on purpose.
pub fn sentinel_byte(word: &str) -> Option<u8> {
    Some(match word {
        "era-turn" => 243,
        "bell-toll" => 244,
        "door" => 245,
        "sleep" | "wake" => 246,
        "gift" => 247,
        "summon-start" => 248,
        "summon-refuse" => 249,
        "eclipse" => 250,
        "account-event" => 251,
        "presence-enter" => 252,
        "presence-exit" => 253,
        "held-blank" => 255,
        _ => return None,
    })
}

fn parse_axes(s: &str, line: u32) -> Result<[i8; 5], GeomRefusal> {
    let inner = s
        .trim()
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .ok_or_else(|| GeomRefusal::at(line, format!("axes must be [t,t,t,t,t], got {s:?}")))?;
    let mut out = [0i8; 5];
    let mut n = 0usize;
    for part in inner.split(',') {
        if n >= 5 {
            return Err(GeomRefusal::at(line, "axes carry more than 5 trits"));
        }
        let v: i8 = part
            .trim()
            .parse()
            .map_err(|_| GeomRefusal::at(line, format!("bad trit {:?}", part.trim())))?;
        if !(-1..=1).contains(&v) {
            return Err(GeomRefusal::at(line, format!("trit {v} outside -1..=1")));
        }
        out[n] = v;
        n += 1;
    }
    if n != 5 {
        return Err(GeomRefusal::at(line, format!("axes carry {n} trits, want 5")));
    }
    Ok(out)
}

fn parse_cell(s: &str, line: u32) -> Result<(u16, u16), GeomRefusal> {
    let (c, r) = s
        .split_once('.')
        .ok_or_else(|| GeomRefusal::at(line, format!("cell must be col.row, got {s:?}")))?;
    let col = c.parse().map_err(|_| GeomRefusal::at(line, format!("bad cell col {c:?}")))?;
    let row = r.parse().map_err(|_| GeomRefusal::at(line, format!("bad cell row {r:?}")))?;
    Ok((col, row))
}

/// Parse a dotted decimal into thousandths with pure integer math: "4.5" -> 4500.
fn parse_m3(s: &str, line: u32) -> Result<u32, GeomRefusal> {
    let bad = || GeomRefusal::at(line, format!("bad decimal {s:?}"));
    match s.split_once('.') {
        None => s.parse::<u32>().map(|v| v * 1000).map_err(|_| bad()),
        Some((int, frac)) => {
            if frac.is_empty() || frac.len() > 3 || frac.contains('.') {
                return Err(bad());
            }
            let i: u32 = int.parse().map_err(|_| bad())?;
            let f: u32 = frac.parse().map_err(|_| bad())?;
            let scale = 10u32.pow(3 - frac.len() as u32);
            Ok(i * 1000 + f * scale)
        }
    }
}

#[derive(PartialEq)]
enum Mode {
    Meta,
    Legend,
    Front,
    Under,
    Anno,
}

/// Parse `.geom.vixi` source text. Refuses whole on any violation.
pub fn parse_geom(src: &str) -> Result<GeomDoc, GeomRefusal> {
    let mut dialect_version: Option<u32> = None;
    let mut surface = String::new();
    let mut cart = String::new();
    let mut band = String::new();
    let mut width: u16 = 0;
    let mut height: u16 = 0;
    let mut legend: Vec<GeomLegendRow> = Vec::new();
    let mut front: Vec<String> = Vec::new();
    let mut under: Vec<GeomUnderRow> = Vec::new();
    let mut annos: Vec<GeomAnno> = Vec::new();
    let mut contrast_min_m3: Option<u32> = None;
    let mut hit_target_min_mu: Option<u32> = None;
    let mut runtime_parse_forbidden = false;
    let mut float_in_ir_forbidden = false;
    let mut seed_deterministic = false;
    let mut mode = Mode::Meta;

    for (idx, raw) in src.lines().enumerate() {
        let line_no = idx as u32 + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("#vixi:") {
            let mut it = rest.split_whitespace();
            let dialect = it.next().unwrap_or("");
            if dialect != "geom" {
                return Err(GeomRefusal::at(line_no, format!("expected '#vixi:geom', got dialect {dialect:?}")));
            }
            let v = it
                .next()
                .and_then(|t| t.strip_prefix('v'))
                .and_then(|n| n.parse::<u32>().ok())
                .ok_or_else(|| GeomRefusal::at(line_no, "header wants '#vixi:geom vN'"))?;
            dialect_version = Some(v);
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if dialect_version.is_none() {
            return Err(GeomRefusal::at(line_no, "content before the '#vixi:geom vN' header"));
        }
        match line {
            "legend:" => {
                mode = Mode::Legend;
                continue;
            }
            "front:" => {
                mode = Mode::Front;
                continue;
            }
            "under:" => {
                mode = Mode::Under;
                continue;
            }
            "anno:" => {
                mode = Mode::Anno;
                continue;
            }
            _ => {}
        }
        if let Some(rest) = line.strip_prefix("gate ") {
            let body = rest.split('#').next().unwrap_or("").trim();
            let (name, value) = body
                .split_once('=')
                .map(|(n, v)| (n.trim(), v.trim()))
                .ok_or_else(|| GeomRefusal::at(line_no, format!("gate wants 'gate name = value', got {body:?}")))?;
            match name {
                "contrast_min" => contrast_min_m3 = Some(parse_m3(value, line_no)?),
                "hit_target_min" => {
                    let inner = value
                        .strip_prefix("mu(")
                        .and_then(|r| r.strip_suffix(')'))
                        .ok_or_else(|| GeomRefusal::at(line_no, format!("hit_target_min wants mu(N), got {value:?}")))?;
                    hit_target_min_mu =
                        Some(inner.parse().map_err(|_| GeomRefusal::at(line_no, format!("bad mu value {inner:?}")))?);
                }
                "runtime_parse" => {
                    if value != "forbidden" {
                        return Err(GeomRefusal::at(line_no, "runtime_parse must be forbidden"));
                    }
                    runtime_parse_forbidden = true;
                }
                "float_in_ir" => {
                    if value != "forbidden" {
                        return Err(GeomRefusal::at(line_no, "float_in_ir must be forbidden"));
                    }
                    float_in_ir_forbidden = true;
                }
                "seed_deterministic" => {
                    if value != "required" {
                        return Err(GeomRefusal::at(line_no, "seed_deterministic must be required"));
                    }
                    seed_deterministic = true;
                }
                other => return Err(GeomRefusal::at(line_no, format!("unknown gate {other:?}"))),
            }
            continue;
        }
        match mode {
            Mode::Meta => {
                let (key, value) = line
                    .split_once(':')
                    .map(|(k, v)| (k.trim(), v.split('#').next().unwrap_or("").trim()))
                    .ok_or_else(|| GeomRefusal::at(line_no, format!("expected 'key: value', got {line:?}")))?;
                match key {
                    "surface" => surface = value.to_owned(),
                    "cart" => cart = value.to_owned(),
                    "band" => band = value.to_owned(),
                    "size" => {
                        let (w, h) = value
                            .split_once('x')
                            .ok_or_else(|| GeomRefusal::at(line_no, format!("size wants WxH, got {value:?}")))?;
                        width = w.trim().parse().map_err(|_| GeomRefusal::at(line_no, format!("bad width {w:?}")))?;
                        height = h.trim().parse().map_err(|_| GeomRefusal::at(line_no, format!("bad height {h:?}")))?;
                        if width == 0 || width > GEOM_MAX_COLS {
                            return Err(GeomRefusal::at(
                                line_no,
                                format!("width {width} outside 1..={GEOM_MAX_COLS} (the PaTeX bound)"),
                            ));
                        }
                        if height == 0 {
                            return Err(GeomRefusal::at(line_no, "height must be nonzero"));
                        }
                    }
                    other => return Err(GeomRefusal::at(line_no, format!("unknown key {other:?}"))),
                }
            }
            Mode::Legend => {
                let ch = line
                    .strip_prefix('\'')
                    .and_then(|r| {
                        let mut cs = r.chars();
                        let c = cs.next()?;
                        cs.next().filter(|q| *q == '\'').map(|_| c)
                    })
                    .ok_or_else(|| GeomRefusal::at(line_no, format!("legend row wants '<ch>' = [..], got {line:?}")))?;
                if !ch.is_ascii() || ch.is_ascii_control() {
                    return Err(GeomRefusal::at(line_no, format!("legend glyph {ch:?} is not printable ASCII")));
                }
                if legend.iter().any(|l| l.ch == ch) {
                    return Err(GeomRefusal::at(line_no, format!("duplicate legend glyph {ch:?}")));
                }
                let open = line
                    .find('[')
                    .ok_or_else(|| GeomRefusal::at(line_no, "legend row missing [axes]"))?;
                let close = line[open..]
                    .find(']')
                    .map(|o| open + o)
                    .ok_or_else(|| GeomRefusal::at(line_no, "legend row missing closing ]"))?;
                let axes = parse_axes(&line[open..=close], line_no)?;
                legend.push(GeomLegendRow { ch, axes });
            }
            Mode::Front => {
                let first = line
                    .find('|')
                    .ok_or_else(|| GeomRefusal::at(line_no, format!("front row must sit between pipes, got {line:?}")))?;
                let last = line.rfind('|').unwrap_or(first);
                if last <= first {
                    return Err(GeomRefusal::at(line_no, "front row wants |content|"));
                }
                let interior = &line[first + 1..last];
                if interior.chars().count() != width as usize {
                    return Err(GeomRefusal::at(
                        line_no,
                        format!("front row is {} chars, size declares {width}", interior.chars().count()),
                    ));
                }
                if front.len() as u16 >= height {
                    return Err(GeomRefusal::at(line_no, format!("more front rows than the declared height {height}")));
                }
                front.push(interior.to_owned());
            }
            Mode::Under => {
                let rest = line
                    .strip_prefix("@ ")
                    .ok_or_else(|| GeomRefusal::at(line_no, format!("under row wants '@ cell=C.R axes=[..]', got {line:?}")))?;
                let mut col_row: Option<(u16, u16)> = None;
                let mut axes: Option<[i8; 5]> = None;
                for tok in rest.split_whitespace() {
                    if let Some(v) = tok.strip_prefix("cell=") {
                        col_row = Some(parse_cell(v, line_no)?);
                    } else if let Some(v) = tok.strip_prefix("axes=") {
                        axes = Some(parse_axes(v, line_no)?);
                    } else if tok.starts_with('#') {
                        break;
                    }
                }
                let (col, row) =
                    col_row.ok_or_else(|| GeomRefusal::at(line_no, "under row missing cell="))?;
                let axes = axes.ok_or_else(|| GeomRefusal::at(line_no, "under row missing axes="))?;
                under.push(GeomUnderRow { col, row, axes });
            }
            Mode::Anno => {
                let rest = line
                    .strip_prefix("@ ")
                    .ok_or_else(|| GeomRefusal::at(line_no, format!("anno row wants '@ ...', got {line:?}")))?;
                let mut target: Option<GeomAnnoTarget> = None;
                let mut kind: Option<GeomAnnoKind> = None;
                let mut out_word: Option<String> = None;
                for tok in rest.split_whitespace() {
                    if let Some(v) = tok.strip_prefix("cell=") {
                        let (col, row) = parse_cell(v, line_no)?;
                        target = Some(GeomAnnoTarget::Cell { col, row });
                    } else if let Some(v) = tok.strip_prefix("region=") {
                        let (a, b) = v
                            .split_once("..")
                            .ok_or_else(|| GeomRefusal::at(line_no, format!("region wants C.R..C.R, got {v:?}")))?;
                        let (c0, r0) = parse_cell(a, line_no)?;
                        let (c1, r1) = parse_cell(b, line_no)?;
                        target = Some(GeomAnnoTarget::Region { c0, r0, c1, r1 });
                    } else if let Some(v) = tok.strip_prefix("bind=") {
                        kind = Some(GeomAnnoKind::Bind(v.to_owned()));
                    } else if let Some(v) = tok.strip_prefix("sentinel=") {
                        let byte = sentinel_byte(v)
                            .ok_or_else(|| GeomRefusal::at(line_no, format!("unknown sentinel word {v:?}")))?;
                        kind = Some(GeomAnnoKind::Sentinel { byte, out: None });
                    } else if let Some(v) = tok.strip_prefix("out=") {
                        out_word = Some(v.trim_matches('"').to_owned());
                    } else if tok.starts_with('#') {
                        break;
                    }
                }
                let target = target.ok_or_else(|| GeomRefusal::at(line_no, "anno missing cell=/region="))?;
                let mut kind = kind.ok_or_else(|| GeomRefusal::at(line_no, "anno missing bind=/sentinel="))?;
                if let GeomAnnoKind::Sentinel { out, .. } = &mut kind {
                    *out = out_word;
                }
                annos.push(GeomAnno { target, kind });
            }
        }
    }

    let dialect_version =
        dialect_version.ok_or_else(|| GeomRefusal::at(0, "missing '#vixi:geom vN' header"))?;
    if surface.is_empty() {
        return Err(GeomRefusal::at(0, "missing 'surface:'"));
    }
    if cart.is_empty() {
        return Err(GeomRefusal::at(0, "missing 'cart:'"));
    }
    if band.is_empty() {
        return Err(GeomRefusal::at(0, "missing 'band:'"));
    }
    if width == 0 || height == 0 {
        return Err(GeomRefusal::at(0, "missing 'size: WxH'"));
    }
    if legend.is_empty() {
        return Err(GeomRefusal::at(0, "missing 'legend:' rows"));
    }
    if front.is_empty() {
        return Err(GeomRefusal::at(0, "missing 'front:' rows"));
    }
    let gates = GeomGates {
        contrast_min_m3: contrast_min_m3.ok_or_else(|| GeomRefusal::at(0, "missing gate contrast_min"))?,
        hit_target_min_mu: hit_target_min_mu.ok_or_else(|| GeomRefusal::at(0, "missing gate hit_target_min"))?,
        runtime_parse_forbidden,
        float_in_ir_forbidden,
        seed_deterministic,
    };
    if !gates.runtime_parse_forbidden {
        return Err(GeomRefusal::at(0, "missing gate runtime_parse = forbidden"));
    }
    if !gates.float_in_ir_forbidden {
        return Err(GeomRefusal::at(0, "missing gate float_in_ir = forbidden"));
    }
    if !gates.seed_deterministic {
        return Err(GeomRefusal::at(0, "missing gate seed_deterministic = required"));
    }
    if gates.contrast_min_m3 < 4500 {
        return Err(GeomRefusal::at(0, format!("contrast_min {} below the 4.5 floor", gates.contrast_min_m3)));
    }
    if gates.hit_target_min_mu < 44 {
        return Err(GeomRefusal::at(0, format!("hit_target_min mu({}) below mu(44)", gates.hit_target_min_mu)));
    }

    Ok(GeomDoc {
        dialect_version,
        surface,
        cart,
        band,
        width,
        height,
        legend,
        front,
        under,
        annos,
        gates,
    })
}

/// Bake a parsed doc into the lattice + annotation artifact.
/// `source_hash` is the caller's blake3-u64 of the exact source text.
pub fn bake_geom(doc: &GeomDoc, source_hash: u64) -> Result<GeomBake, GeomRefusal> {
    let w = doc.width as usize;
    let h = doc.height as usize;
    let legend: Vec<(char, u8)> = doc
        .legend
        .iter()
        .map(|l| (l.ch, TritCell5D::from_trits(l.axes).0))
        .collect();
    let mut lattice = vec![HELD_BLANK; w * h];
    for (row, text) in doc.front.iter().enumerate() {
        for (col, ch) in text.chars().enumerate() {
            let byte = legend
                .iter()
                .find(|(c, _)| *c == ch)
                .map(|(_, b)| *b)
                .ok_or_else(|| GeomRefusal::at(0, format!("front glyph {ch:?} at {col}.{row} has no legend row")))?;
            lattice[row * w + col] = byte;
        }
    }
    let mut under_overrides = Vec::with_capacity(doc.under.len());
    for u in &doc.under {
        if u.col >= doc.width || u.row >= doc.height {
            return Err(GeomRefusal::at(0, format!("under cell {}.{} outside {}x{}", u.col, u.row, doc.width, doc.height)));
        }
        under_overrides.push((u.col, u.row, TritCell5D::from_trits(u.axes).0));
    }
    for a in &doc.annos {
        let ok = match a.target {
            GeomAnnoTarget::Cell { col, row } => col < doc.width && row < doc.height,
            GeomAnnoTarget::Region { c0, r0, c1, r1 } => {
                c0 <= c1 && r0 <= r1 && c1 <= doc.width && r1 <= doc.height
            }
        };
        if !ok {
            return Err(GeomRefusal::at(0, format!("anno target {:?} outside {}x{}", a.target, doc.width, doc.height)));
        }
        if let GeomAnnoKind::Sentinel { byte, .. } = a.kind {
            if byte == ANIKWACAS {
                return Err(GeomRefusal::at(0, "sentinel 254 is never emitted by authored content"));
            }
        }
    }
    Ok(GeomBake {
        surface: doc.surface.clone(),
        cart: doc.cart.clone(),
        band: doc.band.clone(),
        width: doc.width,
        height: doc.height,
        lattice,
        under_overrides,
        annos: doc.annos.clone(),
        legend,
        source_hash,
    })
}

/// Parse + bake in one deterministic pass — the only bake-time entry point.
pub fn compile_geom(src: &str) -> Result<GeomBake, GeomRefusal> {
    let doc = parse_geom(src)?;
    bake_geom(&doc, crate::timeline::hash_raw(src.as_bytes()))
}

impl GeomBake {
    /// The under face: front lattice with the under overrides applied.
    pub fn under_lattice(&self) -> Vec<u8> {
        let mut out = self.lattice.clone();
        for &(col, row, byte) in &self.under_overrides {
            out[row as usize * self.width as usize + col as usize] = byte;
        }
        out
    }

    /// Expand the packed bytes to full 8-byte Pexils (ordinal = cell index).
    pub fn pexil_field(&self) -> Vec<Pexil> {
        self.lattice
            .iter()
            .enumerate()
            .map(|(i, &b)| Pexil {
                lattice: TritCell5D(b),
                validity: ValidityMask(0),
                ordinal: CellOrdinal(i as u16),
                payload: [0; 4],
            })
            .collect()
    }

    /// The ASCII projection of a lattice byte: its legend glyph, space for
    /// held-blank, `?` for a byte no legend row names.
    pub fn glyph_for(&self, byte: u8) -> char {
        if byte == HELD_BLANK {
            return ' ';
        }
        self.legend.iter().find(|(_, b)| *b == byte).map(|(c, _)| *c).unwrap_or('?')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLL_GATE: &str = include_str!("../fixtures/toll_gate.geom.vixi");

    #[test]
    fn toll_gate_fixture_compiles() {
        let bake = compile_geom(TOLL_GATE).expect("the canonical fixture must compile");
        assert_eq!(bake.surface, "toll_gate");
        assert_eq!(bake.cart, "IRONROOT");
        assert_eq!(bake.band, "mud");
        assert_eq!((bake.width, bake.height), (71, 22));
        assert_eq!(bake.lattice.len(), 71 * 22);
        assert_eq!(bake.legend.len(), 5);
        let at = |c: usize, r: usize| bake.lattice[r * 71 + c];
        assert_eq!(at(34, 2), TritCell5D::from_trits([1, 1, 0, 0, 1]).0, "the gate cell is '+'");
        assert_eq!(at(27, 1), TritCell5D::from_trits([1, -1, 0, 0, 1]).0, "the wall cell is '#'");
        assert_eq!(at(0, 3), TritCell5D::from_trits([1, 0, -1, 0, 1]).0, "the rail cell is '='");
        assert_eq!(at(0, 2), TritCell5D::from_trits([-1, 0, 0, -1, 0]).0, "the loose cell is '~'");
        assert_eq!(at(5, 0), TritCell5D::from_trits([0, 0, 1, 0, 0]).0, "the ground cell is '.'");
        assert_eq!(at(0, 7), HELD_BLANK, "unauthored rows are held-blank");
        assert_eq!(bake.under_overrides, vec![(34, 2, TritCell5D::from_trits([1, 1, 0, 0, -1]).0)]);
        assert_eq!(bake.annos.len(), 4);
        assert!(bake.annos.iter().any(|a| matches!(
            &a.kind,
            GeomAnnoKind::Sentinel { byte: 245, out: Some(w) } if w == "bell_pit"
        )), "the door anno carries sentinel 245 out bell_pit");
        let doc = parse_geom(TOLL_GATE).expect("parses");
        assert_eq!((doc.gates.contrast_min_m3, doc.gates.hit_target_min_mu), (4500, 44));
    }

    #[test]
    fn bake_is_deterministic() {
        let a = compile_geom(TOLL_GATE).expect("compiles");
        let b = compile_geom(TOLL_GATE).expect("compiles");
        assert_eq!(a, b, "same source must bake to identical bytes");
        assert_ne!(a.source_hash, 0, "the determinism receipt is a real hash");
    }

    #[test]
    fn under_lattice_applies_the_override() {
        let bake = compile_geom(TOLL_GATE).expect("compiles");
        let under = bake.under_lattice();
        let i = 2 * 71 + 34;
        assert_eq!(under[i], TritCell5D::from_trits([1, 1, 0, 0, -1]).0);
        assert_ne!(under[i], bake.lattice[i], "the record disagrees with the claim");
        let j = 71 + 27;
        assert_eq!(under[j], bake.lattice[j], "unoverridden cells match the front");
    }

    #[test]
    fn pexil_field_expands_to_full_atoms() {
        let bake = compile_geom(TOLL_GATE).expect("compiles");
        let field = bake.pexil_field();
        assert_eq!(field.len(), 71 * 22);
        let i = 2 * 71 + 34;
        assert_eq!(field[i].lattice, TritCell5D::from_trits([1, 1, 0, 0, 1]));
        assert_eq!(field[i].ordinal, CellOrdinal(i as u16));
        assert!(field[0].lattice.is_sentinel() || field[0].lattice.trits().is_some());
    }

    #[test]
    fn glyph_projection_round_trips_authored_cells() {
        let bake = compile_geom(TOLL_GATE).expect("compiles");
        for (ch, byte) in &bake.legend {
            assert_eq!(bake.glyph_for(*byte), *ch);
        }
        assert_eq!(bake.glyph_for(HELD_BLANK), ' ');
    }

    #[test]
    fn wrong_dialect_refuses() {
        let err = compile_geom("#vixi:kit v1\nsurface: x\n").expect_err("kit is not geom");
        assert!(err.what.contains("dialect"), "{err}");
    }

    #[test]
    fn width_past_the_patex_bound_refuses() {
        let src = TOLL_GATE.replace("size: 71x22", "size: 72x22");
        let err = compile_geom(&src).expect_err("72 cols breaks the B-locked bound");
        assert!(err.what.contains("PaTeX"), "{err}");
    }

    #[test]
    fn row_width_mismatch_refuses() {
        let src = TOLL_GATE.replace("size: 71x22", "size: 70x22");
        let err = compile_geom(&src).expect_err("a 71-char row against size 70 must refuse");
        assert!(err.what.contains("71 chars"), "{err}");
    }

    #[test]
    fn unknown_front_glyph_refuses() {
        let src = TOLL_GATE.replace("######++######", "######%+######");
        let err = compile_geom(&src).expect_err("% has no legend row");
        assert!(err.what.contains("legend"), "{err}");
    }

    #[test]
    fn trit_outside_balanced_range_refuses() {
        let src = TOLL_GATE.replace("'+' = [ 1, 1, 0, 0, 1]", "'+' = [ 2, 1, 0, 0, 1]");
        let err = compile_geom(&src).expect_err("trit 2 is not balanced");
        assert!(err.what.contains("outside -1..=1"), "{err}");
    }

    #[test]
    fn missing_gate_refuses_whole() {
        let src = TOLL_GATE.replace("gate seed_deterministic = required\n", "");
        let err = compile_geom(&src).expect_err("a geom missing ANY gate is refused whole");
        assert!(err.what.contains("seed_deterministic"), "{err}");
    }

    #[test]
    fn contrast_below_floor_refuses() {
        let src = TOLL_GATE.replace("contrast_min = 4.5", "contrast_min = 4.4");
        let err = compile_geom(&src).expect_err("4.4 is below the 4.5 floor");
        assert!(err.what.contains("4.5 floor"), "{err}");
    }

    #[test]
    fn anikwacas_is_unauthorable() {
        let src = TOLL_GATE.replace("sentinel=door", "sentinel=anikwacas");
        let err = compile_geom(&src).expect_err("254 never comes from authored content");
        assert!(err.what.contains("unknown sentinel"), "{err}");
        assert_eq!(sentinel_byte("anikwacas"), None);
    }

    #[test]
    fn duplicate_legend_glyph_refuses() {
        let src = TOLL_GATE.replace("'~' = [-1, 0, 0,-1, 0]", "'.' = [-1, 0, 0,-1, 0]");
        let err = compile_geom(&src).expect_err("two legend rows for '.' must refuse");
        assert!(err.what.contains("duplicate"), "{err}");
    }

    #[test]
    fn sentinel_words_cover_the_thirteen_minus_anikwacas() {
        for (w, b) in [
            ("era-turn", 243u8),
            ("bell-toll", 244),
            ("door", 245),
            ("sleep", 246),
            ("gift", 247),
            ("summon-start", 248),
            ("summon-refuse", 249),
            ("eclipse", 250),
            ("account-event", 251),
            ("presence-enter", 252),
            ("presence-exit", 253),
            ("held-blank", 255),
        ] {
            assert_eq!(sentinel_byte(w), Some(b), "{w}");
        }
    }
}
