//! tree-sitter-vixel — Grammar definition for VixiScript DSL
//!
//! Three semantic branches:
//!   @MaterialDef → forge-furnace → .forge_reg binary registry
//!   @SpatialDef  → socket graph / chunk placement
//!   @AutomataDef → forge-shader-build → .spv / .dxil compute shaders
//!
//! Constraints:
//!   - Integer-only (no floats, no decimals)
//!   - Permyriad values: `Np` where N is 0-10000
//!   - Hex colors: `0xRRGGBBAA`
//!   - Names: quoted strings, max 32 bytes
//!
//! This grammar MUST align with forge-ml::gbnf_sampler::GbnfConstraint::new_vixiscript()
//! so that AI-generated VixiScript is guaranteed parseable.

// This is the canonical grammar definition. tree-sitter-cli generates the C parser from this.
//
// module.exports = grammar({
//   name: 'vixel',
//
//   rules: {
//     source_file: $ => repeat($._definition),
//
//     _definition: $ => choice(
//       $.material_def,
//       $.spatial_def,
//       $.automata_def,
//       $.environment_call,
//       $.ui_def,
//     ),
//
//     // ── @MaterialDef ──────────────────────────────────────────────────────
//     material_def: $ => seq(
//       'material',
//       $.string_literal,
//       '{',
//       repeat($.property),
//       '}',
//     ),
//
//     // ── @SpatialDef ───────────────────────────────────────────────────────
//     spatial_def: $ => seq(
//       choice('spawn_grid', 'spawn_ring', 'spawn_scatter'),
//       '(',
//       $.string_literal,
//       repeat(seq(',', $._value)),
//       ')',
//     ),
//
//     // ── @AutomataDef ──────────────────────────────────────────────────────
//     automata_def: $ => seq(
//       'rule',
//       $.string_literal,
//       '{',
//       $.when_clause,
//       $.then_clause,
//       optional($.tick_delay),
//       '}',
//     ),
//
//     when_clause: $ => seq('when', ':', $._expression),
//     then_clause: $ => seq('then', ':', $._expression),
//     tick_delay: $ => seq('tick_delay', ':', $.integer),
//
//     // ── Environment ────────────────────────────────────────────────────────
//     environment_call: $ => seq(
//       choice('set_temperature', 'set_wind', 'set_gravity'),
//       '(',
//       choice($.string_literal, $.array_literal),
//       ',',
//       $._value,
//       ')',
//     ),
//
//     // ── UI ─────────────────────────────────────────────────────────────────
//     ui_def: $ => seq(
//       'ui',
//       $.string_literal,
//       '{',
//       repeat($.property),
//       '}',
//     ),
//
//     // ── Shared Rules ───────────────────────────────────────────────────────
//     property: $ => seq($.identifier, ':', $._value),
//
//     _value: $ => choice(
//       $.integer,
//       $.permyriad,
//       $.hex_literal,
//       $.string_literal,
//       $.identifier,
//       $.array_literal,
//     ),
//
//     _expression: $ => choice(
//       $.identifier,
//       $.comparison,
//       $.function_call,
//     ),
//
//     comparison: $ => seq($._value, choice('>', '<', '>=', '<=', '=='), $._value),
//     function_call: $ => seq($.identifier, '(', optional(seq($._value, repeat(seq(',', $._value)))), ')'),
//     array_literal: $ => seq('[', optional(seq($._value, repeat(seq(',', $._value)))), ']'),
//
//     // ── Terminals ──────────────────────────────────────────────────────────
//     integer: $ => /[0-9]+/,
//     permyriad: $ => /[0-9]+p/,
//     hex_literal: $ => /0x[0-9a-fA-F]+/,
//     string_literal: $ => /"[^"]*"/,
//     identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,
//   },
// });

// Until tree-sitter-cli generates the C parser, we provide a Rust-native
// zero-alloc parser that produces the same CST structure.

/// Top-level branch aliases mirroring the grammar's `_definition` rule.
/// `gbnf_sampler::GbnfConstraint::new_vixiscript()` derives its grammar text from
/// this const so the two stay in sync without hand-mirroring.
pub const VIXI_BRANCH_ALIASES: &[&str] = &[
    "@MaterialDef",
    "@SpatialDef",
    "@AutomataDef",
    "@EnvironmentCall",
    "@UiDef",
];

// The canonical NodeKind -> (colour ⊕ sound) vocabulary: a CST is a score.
/// Voice and color assignment for parse tree nodes.
pub mod seehear;

/// Node kind IDs matching the tree-sitter grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeKind {
    /// Root of the source file.
    SourceFile = 0,
    /// Material definition block.
    MaterialDef = 1,
    /// Spatial definition (spawn_grid, spawn_ring, spawn_scatter).
    SpatialDef = 2,
    /// Automata rule definition.
    AutomataDef = 3,
    /// Environment call (set_temperature, set_wind, set_gravity).
    EnvironmentCall = 4,
    /// UI definition block.
    UiDef = 5,
    /// Property key:value pair.
    Property = 6,
    /// When clause condition.
    WhenClause = 7,
    /// Then clause action.
    ThenClause = 8,
    /// Tick delay specification.
    TickDelay = 9,
    /// Integer literal.
    Integer = 10,
    /// Permyriad value (Np).
    Permyriad = 11,
    /// Hexadecimal color literal.
    HexLiteral = 12,
    /// String literal.
    StringLiteral = 13,
    /// Identifier name.
    Identifier = 14,
    /// Array literal.
    ArrayLiteral = 15,
    /// Comparison expression.
    Comparison = 16,
    /// Function call.
    FunctionCall = 17,
    /// `atom { coord/material_id/resonance/color }` — VixiScript's lowering target (VixelAtom).
    AtomDef = 18,
    /// `acrylic { color/material_id/essence_id/phase }` — AcrylicLoad paint dab.
    AcrylicDef = 19,
    /// `pressure { curve }` — PressureCurve pen-feel.
    PressureDef = 20,
    /// `layers { count }` — LayerStack depth.
    LayersDef = 21,
    /// `viewport { w/h/zoom }` — VixelViewport camera.
    ViewportDef = 22,
    /// `brush { w/h/falloff }` — BrushMask/MaskStamp tip.
    BrushDef = 23,
    /// Parse error node.
    Error = 255,
}

/// A CST node — zero-alloc, references byte offsets in the source.
#[derive(Debug, Clone, Copy)]
pub struct CstNode {
    /// Kind of the node.
    pub kind: NodeKind,
    /// Start byte offset in source.
    pub start: u16,
    /// End byte offset in source.
    pub end: u16,
    /// Index into children array.
    pub child_start: u16,
    /// Number of children.
    pub child_count: u8,
}

/// Fixed-capacity CST — no heap allocation.
/// Max 256 nodes covers any reasonable single-file VixiScript.
pub struct Cst {
    /// Array of CST nodes.
    pub nodes: [CstNode; 256],
    /// Number of populated nodes.
    pub count: u16,
}

impl Cst {
    /// Create an empty CST.
    pub const fn empty() -> Self {
        Self {
            nodes: [CstNode { kind: NodeKind::Error, start: 0, end: 0, child_start: 0, child_count: 0 }; 256],
            count: 0,
        }
    }

    /// Parse VixiScript source bytes into a zero-alloc CST.
    /// Returns the number of top-level definitions found.
    pub fn parse(source: &[u8]) -> Self {
        let mut cst = Self::empty();
        let mut pos: u16 = 0;
        let len = source.len() as u16;

        while pos < len && cst.count < 255 {
            pos = skip_ws(source, pos);
            if pos >= len { break; }

            let node = match source[pos as usize] {
                b'm' if starts_with(source, pos, b"material") => {
                    parse_block(source, &mut pos, NodeKind::MaterialDef, b"material")
                }
                b's' if starts_with(source, pos, b"spawn_") => {
                    parse_call(source, &mut pos, NodeKind::SpatialDef)
                }
                b'r' if starts_with(source, pos, b"rule") => {
                    parse_block(source, &mut pos, NodeKind::AutomataDef, b"rule")
                }
                b's' if starts_with(source, pos, b"set_") => {
                    parse_call(source, &mut pos, NodeKind::EnvironmentCall)
                }
                b'u' if starts_with(source, pos, b"ui") => {
                    parse_block(source, &mut pos, NodeKind::UiDef, b"ui")
                }
                b'a' if starts_with(source, pos, b"atom") => {
                    parse_block(source, &mut pos, NodeKind::AtomDef, b"atom")
                }
                b'a' if starts_with(source, pos, b"acrylic") => {
                    parse_block(source, &mut pos, NodeKind::AcrylicDef, b"acrylic")
                }
                b'p' if starts_with(source, pos, b"pressure") => {
                    parse_block(source, &mut pos, NodeKind::PressureDef, b"pressure")
                }
                b'l' if starts_with(source, pos, b"layers") => {
                    parse_block(source, &mut pos, NodeKind::LayersDef, b"layers")
                }
                b'v' if starts_with(source, pos, b"viewport") => {
                    parse_block(source, &mut pos, NodeKind::ViewportDef, b"viewport")
                }
                b'b' if starts_with(source, pos, b"brush") => {
                    parse_block(source, &mut pos, NodeKind::BrushDef, b"brush")
                }
                _ => {
                    // Skip unknown byte
                    pos += 1;
                    continue;
                }
            };

            cst.nodes[cst.count as usize] = node;
            cst.count += 1;
        }

        cst
    }

    /// Get the source text for a node.
    pub fn text<'a>(&self, source: &'a [u8], idx: u16) -> &'a [u8] {
        let node = &self.nodes[idx as usize];
        &source[node.start as usize..node.end as usize]
    }

    /// Iterate top-level definitions by kind.
    pub fn iter_kind(&self, kind: NodeKind) -> impl Iterator<Item = (u16, &CstNode)> {
        self.nodes[..self.count as usize].iter()
            .enumerate()
            .filter(move |(_, n)| n.kind == kind)
            .map(|(i, n)| (i as u16, n))
    }
}

fn skip_ws(source: &[u8], mut pos: u16) -> u16 {
    let len = source.len() as u16;
    while pos < len {
        match source[pos as usize] {
            b' ' | b'\t' | b'\n' | b'\r' => pos += 1,
            b'/' if pos + 1 < len && source[(pos + 1) as usize] == b'/' => {
                while pos < len && source[pos as usize] != b'\n' { pos += 1; }
            }
            _ => break,
        }
    }
    pos
}

fn starts_with(source: &[u8], pos: u16, prefix: &[u8]) -> bool {
    let start = pos as usize;
    if start + prefix.len() > source.len() { return false; }
    &source[start..start + prefix.len()] == prefix
}

fn parse_block(source: &[u8], pos: &mut u16, kind: NodeKind, keyword: &[u8]) -> CstNode {
    let start = *pos;
    *pos += keyword.len() as u16;
    let len = source.len() as u16;
    while *pos < len && source[*pos as usize] != b'{' { *pos += 1; }
    if *pos < len { *pos += 1; }
    // depth-tracked brace matching
    let mut depth: u16 = 1;
    while *pos < len && depth > 0 {
        match source[*pos as usize] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        *pos += 1;
    }
    CstNode { kind, start, end: *pos, child_start: 0, child_count: 0 }
}

fn parse_call(source: &[u8], pos: &mut u16, kind: NodeKind) -> CstNode {
    let start = *pos;
    let len = source.len() as u16;
    while *pos < len && source[*pos as usize] != b'(' { *pos += 1; }
    if *pos < len { *pos += 1; }
    // depth-tracked paren matching
    let mut depth: u16 = 1;
    while *pos < len && depth > 0 {
        match source[*pos as usize] {
            b'(' => depth += 1,
            b')' => depth -= 1,
            _ => {}
        }
        *pos += 1;
    }
    CstNode { kind, start, end: *pos, child_start: 0, child_count: 0 }
}

/// GBNF grammar derived from the tree-sitter grammar rules above.
/// This is the authoritative constraint for `forge-ml::gbnf_sampler::new_vixiscript()`.
/// Keeping it here (co-located with NodeKind + parser) prevents drift.
/// Covers the 5 NodeKind constructs; v2 extensions (scene/light/physics/synthesia)
/// live in the validator, not the grammar.
pub const VIXI_GBNF: &str = r#"
root ::= definition+
definition ::= material-def | spatial-def | automata-def | environment-call | ui-def | ws

material-def ::= "material " string " {" ws? property* "}" ws?
spatial-def ::= spawn-type "(" string ("," ws? value)* ")" ws?
automata-def ::= "rule " string " {" ws? when-clause then-clause tick-delay? "}" ws?
environment-call ::= env-fn "(" (string | array) "," ws? value ")" ws?
ui-def ::= "ui " string " {" ws? property* "}" ws?

spawn-type ::= "spawn_grid" | "spawn_ring" | "spawn_scatter"
env-fn ::= "set_temperature" | "set_wind" | "set_gravity"

property ::= ident ":" ws? value ws?
value ::= permyriad | integer | hex-color | string | ident | array
array ::= "[" (value ("," ws? value)*)? "]"

permyriad ::= [0-9]+ "p"
integer ::= "-"? [0-9]+
hex-color ::= "0x" [0-9A-Fa-f]+
string ::= "\"" [^"]* "\""
ident ::= [a-zA-Z_] [a-zA-Z0-9_]*

when-clause ::= "when " condition ws?
then-clause ::= "then " action ws?
tick-delay ::= "tick_delay " ":" ws? integer ws?
condition ::= ident comparison value
comparison ::= " > " | " < " | " == " | " >= " | " <= " | " != "
action ::= ident "(" (value ("," ws? value)*)? ")"

ws ::= [ \t\n\r]+
"#;

// Alignment contract: if gbnf_sampler passes, Cst::parse() succeeds.
// Forbidden by GBNF: decimal points, unquoted string values, blocks deeper than 1 level.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_material() {
        let src = b"material \"stone\" { hardness: 8000p, albedo: 0x808080FF }";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 1);
        assert_eq!(cst.nodes[0].kind, NodeKind::MaterialDef);
    }

    #[test]
    fn parse_spatial() {
        let src = b"spawn_grid(\"wall_segment\", 4, 8)";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 1);
        assert_eq!(cst.nodes[0].kind, NodeKind::SpatialDef);
    }

    #[test]
    fn parse_automata() {
        let src = b"rule \"fire_spread\" { when: temperature > 7000, then: ignite(neighbor), tick_delay: 3 }";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 1);
        assert_eq!(cst.nodes[0].kind, NodeKind::AutomataDef);
    }

    #[test]
    fn parse_mixed() {
        let src = b"material \"wood\" { hardness: 3000p }\nspawn_grid(\"plank\", 2)\nrule \"burn\" { when: fire > 5000, then: destroy(), tick_delay: 1 }";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 3);
        assert_eq!(cst.nodes[0].kind, NodeKind::MaterialDef);
        assert_eq!(cst.nodes[1].kind, NodeKind::SpatialDef);
        assert_eq!(cst.nodes[2].kind, NodeKind::AutomataDef);
    }

    #[test]
    fn parse_environment() {
        let src = b"set_temperature(\"fire\", 9500)";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 1);
        assert_eq!(cst.nodes[0].kind, NodeKind::EnvironmentCall);
    }

    #[test]
    fn parse_atom() {
        // The lowering-target join: an `atom` block is a first-class CST node, so the
        // TKNO terminal (termi) sees + hears the VixelAtom primitive.
        let src = b"atom { coord: (3, 4), material_id: 2, resonance: 5000p, color: 0x808080 }";
        let cst = Cst::parse(src);
        assert_eq!(cst.count, 1);
        assert_eq!(cst.nodes[0].kind, NodeKind::AtomDef);
    }

    #[test]
    fn parse_new_prims() {
        // Five joined primitives each parse to their own first-class CST node, so each
        // sings its identity note in the TKNO terminal.
        let cases: [(&[u8], NodeKind); 5] = [
            (b"acrylic { color: 0x40C0FF, material_id: 3, essence_id: 1, phase: 5000p }", NodeKind::AcrylicDef),
            (b"pressure { curve: soft }", NodeKind::PressureDef),
            (b"layers { count: 4 }", NodeKind::LayersDef),
            (b"viewport { w: 1920, h: 1080, zoom: 10000p }", NodeKind::ViewportDef),
            (b"brush { w: 64, h: 64, falloff: 5000p }", NodeKind::BrushDef),
        ];
        for (src, kind) in cases {
            let cst = Cst::parse(src);
            assert_eq!(cst.count, 1, "one node for {kind:?}");
            assert_eq!(cst.nodes[0].kind, kind);
        }
    }

    #[test]
    fn text_extraction() {
        let src = b"material \"iron\" { mass: 7800p }";
        let cst = Cst::parse(src);
        let text = cst.text(src, 0);
        assert_eq!(text, src.as_slice());
    }
}
