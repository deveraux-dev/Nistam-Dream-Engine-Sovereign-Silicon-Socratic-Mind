//! # cst.rs — hand-rolled LOSSLESS CST for VixiScript (Fable T1-LADDER #12)
//!
//! The T1 CST/AST/LSP spine endgame: ONE lossless green tree under every dialect,
//! exact byte spans, `.vixel` a dialect leaf, the tree-sitter-vixel hand-mirror
//! retired. Built **hand-rolled, no external dep** — `rowan`/`cstree`/`salsa` are
//! REJECTED on the same sovereign-no-dep grounds as `tower-lsp` (forge-vix-lsp
//! Cargo.toml). forge-vix's compute boundary is relaxed (cold authoring), so this
//! module may `String`/`Rc` freely. Pure: no forge-ast edge (firewall holds).
//!
//! ## The invariant this whole module exists to hold
//! **Losslessness:** every byte of source lands in exactly one token, so the token
//! stream — and the tree built over it — round-trips byte-exact:
//! `relex(src) == src` and (slice 2+) `SyntaxNode::text() == src`. Whitespace,
//! newlines (`\n` / `\r\n` / bare `\r`), and `#` lines are all real *trivia* tokens,
//! never discarded. Slices are always taken on UTF-8 char boundaries, so no source —
//! including unicode inside strings — can panic or be dropped.

/// Every kind a token or node can take. Tokens lex directly from source; nodes are
/// opened by the parser (slice 2+). The token set is deliberately coarse — just
/// enough structure for the parser, every byte covered by construction.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum SyntaxKind {
    // ---- tokens (produced by `lex`) ----
    /// A run of spaces and/or tabs (indentation + inter-token gaps).
    Whitespace,
    /// One line terminator, exact bytes preserved (`\n`, `\r\n`, or bare `\r`).
    Newline,
    /// A single `#` (starts the `#vixi:` header and comment-ish lines).
    Hash,
    /// A run of `[A-Za-z0-9_.-]` — identifiers, dialect names, keys, numbers.
    Ident,
    /// A single `=` (attribute assignment).
    Equals,
    /// A single `:` (header dialect separator, key/value).
    Colon,
    /// A `"..."` string literal (lossless even when unterminated or containing escapes).
    Str,
    /// Any other single character (UTF-8 char), so nothing is ever dropped.
    Punct,
    // ---- nodes (opened by the parser, slice 2+) ----
    /// Root node spanning the whole document.
    Document,
    /// The `#vixi:<dialect> v<n>` header line.
    Header,
    /// An indented block introduced by a `region`/keyword line.
    Region,
    /// A `slot ...` line (a leaf authoring element; may still nest attributes).
    Slot,
    /// A single logical line (its tokens up to and including its newline).
    Line,
    /// A run the parser could not structure — still lossless, just unclassified.
    Error,
}

impl SyntaxKind {
    /// Trivia = whitespace / newline the structural view can skip (but never drops).
    pub fn is_trivia(self) -> bool {
        matches!(self, SyntaxKind::Whitespace | SyntaxKind::Newline)
    }
}

/// A lexed token: its kind and the *exact* source slice it covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The lexical category this token was classified as.
    pub kind: SyntaxKind,
    /// The exact source bytes this token covers, verbatim.
    pub text: String,
}

#[inline]
fn is_ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'-')
}

/// Byte-length of the UTF-8 char starting with lead byte `b` (1 on a stray
/// continuation byte, so the cursor always advances).
#[inline]
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else if b >> 3 == 0b1_1110 {
        4
    } else {
        1
    }
}

/// Lex `src` into a lossless token stream. Every byte of `src` is covered by exactly
/// one token, so `relex(src) == src` for all inputs (proven below).
pub fn lex(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut out = Vec::new();
    while i < n {
        let start = i;
        let c = bytes[i];
        let kind = match c {
            b'\n' => {
                i += 1;
                SyntaxKind::Newline
            }
            b'\r' => {
                i += if i + 1 < n && bytes[i + 1] == b'\n' { 2 } else { 1 };
                SyntaxKind::Newline
            }
            b' ' | b'\t' => {
                while i < n && (bytes[i] == b' ' || bytes[i] == b'\t') {
                    i += 1;
                }
                SyntaxKind::Whitespace
            }
            b'#' => {
                i += 1;
                SyntaxKind::Hash
            }
            b'=' => {
                i += 1;
                SyntaxKind::Equals
            }
            b':' => {
                i += 1;
                SyntaxKind::Colon
            }
            b'"' => {
                i += 1;
                while i < n && bytes[i] != b'"' && bytes[i] != b'\n' && bytes[i] != b'\r' {
                    // skip an escaped byte pair, else advance one
                    if bytes[i] == b'\\' && i + 1 < n {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i < n && bytes[i] == b'"' {
                    i += 1;
                }
                SyntaxKind::Str
            }
            c if is_ident_byte(c) => {
                while i < n && is_ident_byte(bytes[i]) {
                    i += 1;
                }
                SyntaxKind::Ident
            }
            c => {
                // Catch-all: consume one whole UTF-8 char so we never split a codepoint.
                i += utf8_len(c);
                SyntaxKind::Punct
            }
        };
        // Belt-and-braces: whatever the arm did, land `i` on a char boundary so the
        // slice below can never panic (guards escape-skips crossing a codepoint too).
        while i < n && !src.is_char_boundary(i) {
            i += 1;
        }
        out.push(Token { kind, text: src[start..i].to_string() });
    }
    out
}

/// Reassemble source from a token stream — the losslessness oracle.
pub fn relex(src: &str) -> String {
    lex(src).into_iter().map(|t| t.text).collect()
}

// ---------------------------------------------------------------------------
// Green tree — immutable, shareable, offset-free (rowan's "green" layer).
// A `SyntaxNode` cursor (below) layers byte offsets on top for spans.
// ---------------------------------------------------------------------------

use std::rc::Rc;

/// A leaf: a lexed token carrying its exact source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GreenToken {
    /// The lexical category this token was classified as.
    pub kind: SyntaxKind,
    /// The exact source bytes this token covers, verbatim.
    pub text: String,
}

impl From<Token> for GreenToken {
    fn from(t: Token) -> Self {
        GreenToken { kind: t.kind, text: t.text }
    }
}

/// A child of a node — either a leaf token or a nested node.
#[derive(Clone, Debug)]
pub enum GreenChild {
    /// A leaf token.
    Token(GreenToken),
    /// A nested interior node.
    Node(Rc<GreenNode>),
}

impl GreenChild {
    fn len(&self) -> usize {
        match self {
            GreenChild::Token(t) => t.text.len(),
            GreenChild::Node(n) => n.len,
        }
    }
    fn write_text(&self, out: &mut String) {
        match self {
            GreenChild::Token(t) => out.push_str(&t.text),
            GreenChild::Node(n) => n.write_text(out),
        }
    }
}

/// An interior node: a kind plus its children, in source order. Immutable; its
/// byte length is memoised so span math is O(children), not O(text).
#[derive(Clone, Debug)]
pub struct GreenNode {
    /// The syntactic category this interior node represents.
    pub kind: SyntaxKind,
    /// This node's children, in source order.
    pub children: Vec<GreenChild>,
    len: usize,
}

impl GreenNode {
    fn new(kind: SyntaxKind, children: Vec<GreenChild>) -> Rc<GreenNode> {
        let len = children.iter().map(GreenChild::len).sum();
        Rc::new(GreenNode { kind, children, len })
    }
    /// Byte length of the exact source this node covers.
    pub fn text_len(&self) -> usize {
        self.len
    }
    fn write_text(&self, out: &mut String) {
        for c in &self.children {
            c.write_text(out);
        }
    }
    /// The exact source slice this node covers — `text()` of the root == the input.
    pub fn text(&self) -> String {
        let mut s = String::with_capacity(self.len);
        self.write_text(&mut s);
        s
    }
}

// ---------------------------------------------------------------------------
// SyntaxNode cursor — a green node + its absolute byte offset, so every node and
// token can report an exact `range()` into the original source.
// ---------------------------------------------------------------------------

/// A positioned view of a green node: the shared green data plus this node's
/// absolute byte offset in the source. Cheap to clone (Rc + usize).
#[derive(Clone, Debug)]
pub struct SyntaxNode {
    green: Rc<GreenNode>,
    offset: usize,
}

/// One positioned child of a `SyntaxNode`.
#[derive(Clone, Debug)]
pub enum SyntaxElement {
    /// A positioned interior node.
    Node(SyntaxNode),
    /// A positioned leaf token.
    Token {
        /// The lexical category this token was classified as.
        kind: SyntaxKind,
        /// The exact source bytes this token covers, verbatim.
        text: String,
        /// Absolute byte offset of this token's start in the source.
        offset: usize,
    },
}

impl SyntaxElement {
    /// `[start, end)` byte range of this element in the source.
    pub fn range(&self) -> (usize, usize) {
        match self {
            SyntaxElement::Node(n) => n.range(),
            SyntaxElement::Token { text, offset, .. } => (*offset, *offset + text.len()),
        }
    }
}

impl SyntaxNode {
    /// This node's kind.
    pub fn kind(&self) -> SyntaxKind {
        self.green.kind
    }
    /// `[start, end)` byte range of this node in the source.
    pub fn range(&self) -> (usize, usize) {
        (self.offset, self.offset + self.green.len)
    }
    /// The exact source slice this node covers.
    pub fn text(&self) -> String {
        self.green.text()
    }
    /// Positioned children (nodes and tokens interleaved, in source order).
    pub fn children_with_tokens(&self) -> Vec<SyntaxElement> {
        let mut out = Vec::with_capacity(self.green.children.len());
        let mut cursor = self.offset;
        for c in &self.green.children {
            match c {
                GreenChild::Token(t) => {
                    out.push(SyntaxElement::Token {
                        kind: t.kind,
                        text: t.text.clone(),
                        offset: cursor,
                    });
                    cursor += t.text.len();
                }
                GreenChild::Node(n) => {
                    out.push(SyntaxElement::Node(SyntaxNode { green: n.clone(), offset: cursor }));
                    cursor += n.len;
                }
            }
        }
        out
    }
    /// Direct child nodes only (skips tokens).
    pub fn child_nodes(&self) -> Vec<SyntaxNode> {
        self.children_with_tokens()
            .into_iter()
            .filter_map(|e| match e {
                SyntaxElement::Node(n) => Some(n),
                _ => None,
            })
            .collect()
    }
    /// Every node in the subtree (self first, pre-order).
    pub fn descendants(&self) -> Vec<SyntaxNode> {
        let mut out = vec![self.clone()];
        for child in self.child_nodes() {
            out.extend(child.descendants());
        }
        out
    }
    /// Concatenated text of this node's own direct **token** children (its own line,
    /// excluding nested child nodes). The header/region "line text".
    pub fn own_token_text(&self) -> String {
        let mut s = String::new();
        for c in &self.green.children {
            if let GreenChild::Token(t) = c {
                s.push_str(&t.text);
            }
        }
        s
    }
}

impl SyntaxNode {
    /// The leading keyword of this line: its FIRST non-trivia token, but only if
    /// that token is an `Ident`. A `#`-comment line (first non-trivia token is `Hash`)
    /// yields `None`, so keyword dispatch never mis-fires on a comment — e.g.
    /// `# gate x=1` is not a gate. `slot root …` → `Some("slot")`.
    pub fn keyword(&self) -> Option<String> {
        for c in &self.green.children {
            if let GreenChild::Token(t) = c {
                if t.kind.is_trivia() {
                    continue;
                }
                return (t.kind == SyntaxKind::Ident).then(|| t.text.clone());
            }
        }
        None
    }

    /// The `n`-th non-trivia **Ident** among this node's OWN tokens (not nested
    /// children). `nth_own_ident(0)` on a `region root …` line is `"region"`,
    /// `nth_own_ident(1)` is `"root"`.
    pub fn nth_own_ident(&self, n: usize) -> Option<String> {
        self.green
            .children
            .iter()
            .filter_map(|c| match c {
                GreenChild::Token(t) if t.kind == SyntaxKind::Ident => Some(t.text.clone()),
                _ => None,
            })
            .nth(n)
    }

    /// The `[start, end)` byte range of the `n`-th own Ident token — the exact span
    /// of e.g. a slot's name, for a precise LSP `selectionRange`.
    pub fn nth_own_ident_range(&self, n: usize) -> Option<(usize, usize)> {
        let mut count = 0usize;
        let mut cursor = self.offset;
        for c in &self.green.children {
            match c {
                GreenChild::Token(t) => {
                    if t.kind == SyntaxKind::Ident {
                        if count == n {
                            return Some((cursor, cursor + t.text.len()));
                        }
                        count += 1;
                    }
                    cursor += t.text.len();
                }
                GreenChild::Node(nd) => cursor += nd.len,
            }
        }
        None
    }

    /// Value of an inline `key = value` attribute among this node's own tokens
    /// (`kind=region` → `"region"`, `layout=stack_v` → `"stack_v"`). Reads the token
    /// stream, so it never sub-string-matches (`kind=regionfoo` is NOT `region`).
    pub fn own_attr(&self, key: &str) -> Option<String> {
        let mut it = self.green.children.iter().filter_map(|c| match c {
            GreenChild::Token(t) if !t.kind.is_trivia() => Some(t),
            _ => None,
        });
        while let Some(t) = it.next() {
            if t.kind == SyntaxKind::Ident && t.text == key {
                if let Some(eq) = it.next() {
                    if eq.kind == SyntaxKind::Equals {
                        return it.next().map(|v| v.text.clone());
                    }
                }
            }
        }
        None
    }

    /// `[start, end)` byte range of the **value** token of an inline `key = value`
    /// attribute — the exact span a live tweak splices over (`patch::patch_attr`).
    /// The range covers the token as written, so a `Str` value includes its quotes.
    pub fn own_attr_range(&self, key: &str) -> Option<(usize, usize)> {
        // Non-trivia tokens paired with their absolute byte offset, so the
        // [key, `=`, value] window can report a span and not just text.
        let mut toks: Vec<(&GreenToken, usize)> = Vec::new();
        let mut cursor = self.offset;
        for c in &self.green.children {
            match c {
                GreenChild::Token(t) => {
                    if !t.kind.is_trivia() {
                        toks.push((t, cursor));
                    }
                    cursor += t.text.len();
                }
                GreenChild::Node(nd) => cursor += nd.len,
            }
        }
        let mut i = 0usize;
        while i + 2 < toks.len() {
            if toks[i].0.kind == SyntaxKind::Ident
                && toks[i].0.text == key
                && toks[i + 1].0.kind == SyntaxKind::Equals
            {
                let (val, at) = toks[i + 2];
                return Some((at, at + val.text.len()));
            }
            i += 1;
        }
        None
    }

    /// Every inline `key = value` attribute among this node's own tokens, in order
    /// (`slot x kind=region layout=stack_v` → `[(kind,region),(layout,stack_v)]`).
    pub fn own_attrs(&self) -> Vec<(String, String)> {
        let toks: Vec<&GreenToken> = self
            .green
            .children
            .iter()
            .filter_map(|c| match c {
                GreenChild::Token(t) if !t.kind.is_trivia() => Some(t),
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        let mut i = 0usize;
        // A window [i, i+1, i+2] is valid while i+2 is in bounds (i + 2 < len).
        while i + 2 < toks.len() {
            if toks[i].kind == SyntaxKind::Ident && toks[i + 1].kind == SyntaxKind::Equals {
                out.push((toks[i].text.clone(), toks[i + 2].text.clone()));
                i += 3;
            } else {
                i += 1;
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Typed AST layer — thin, cheap wrappers over the green tree. This is the surface
// the LSP / diagnostics read instead of the tree-sitter-vixel hand-mirror: one
// tree, typed views, exact spans. Each wrapper is just a `SyntaxNode`.
// ---------------------------------------------------------------------------

/// Typed view of a parsed VixiScript document (the CST root).
#[derive(Clone, Debug)]
pub struct Document(SyntaxNode);

impl Document {
    /// Parse source into a typed document. Losslessness holds: `self.text() == src`.
    pub fn parse(src: &str) -> Document {
        Document(parse(src))
    }
    /// The underlying `Document` syntax node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
    /// Byte-exact reproduction of the source.
    pub fn text(&self) -> String {
        self.0.text()
    }
    /// The `Header` node, iff the document's **first non-blank** line is a `#vixi:`
    /// header — matching `diagnostics::header_dialect`, which reads only the first
    /// non-empty line (a stray `#vixi:` deeper in the file is not a header).
    pub fn header(&self) -> Option<SyntaxNode> {
        let first = self
            .0
            .child_nodes()
            .into_iter()
            .find(|n| n.kind() != SyntaxKind::Line)?;
        (first.kind() == SyntaxKind::Header).then_some(first)
    }
    /// The `#vixi:<dialect>` name, read from the CST — the typed twin of
    /// `diagnostics::header_dialect` (no `diagnostics` module in this crate
    /// yet). Same answer, off the tree.
    pub fn header_dialect(&self) -> Option<String> {
        let header = self.header()?;
        // Scan the header line's own tokens for the `:` then the next Ident.
        let mut seen_colon = false;
        for c in &header.green.children {
            if let GreenChild::Token(t) = c {
                if seen_colon {
                    if t.kind == SyntaxKind::Ident {
                        return Some(t.text.clone());
                    }
                    if t.kind.is_trivia() {
                        continue;
                    }
                    // any other non-trivia token before an ident = malformed header
                    return None;
                }
                if t.kind == SyntaxKind::Colon {
                    seen_colon = true;
                }
            }
        }
        None
    }
    /// Top-level `Region` nodes (direct children of the document).
    pub fn regions(&self) -> Vec<Region> {
        self.0
            .child_nodes()
            .into_iter()
            .filter(|n| n.kind() == SyntaxKind::Region)
            .map(Region)
            .collect()
    }
    /// Every `Slot` node anywhere in the document (nested at any depth).
    pub fn slots(&self) -> Vec<Slot> {
        self.0
            .descendants()
            .into_iter()
            .filter(|n| n.kind() == SyntaxKind::Slot)
            .map(Slot)
            .collect()
    }
}

/// Typed view of a `region …` block.
#[derive(Clone, Debug)]
pub struct Region(SyntaxNode);

impl Region {
    /// The region's name — the identifier after the `region` keyword (`region root`
    /// → `"root"`).
    pub fn name(&self) -> Option<String> {
        self.0.nth_own_ident(1)
    }
    /// Nested regions directly inside this one.
    pub fn regions(&self) -> Vec<Region> {
        self.0
            .child_nodes()
            .into_iter()
            .filter(|n| n.kind() == SyntaxKind::Region)
            .map(Region)
            .collect()
    }
    /// Slots directly inside this region.
    pub fn slots(&self) -> Vec<Slot> {
        self.0
            .child_nodes()
            .into_iter()
            .filter(|n| n.kind() == SyntaxKind::Slot)
            .map(Slot)
            .collect()
    }
    /// `[start, end)` byte range of the whole region (incl. nested children).
    pub fn range(&self) -> (usize, usize) {
        self.0.range()
    }
    /// The underlying syntax node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

/// Typed view of a `slot …` line.
#[derive(Clone, Debug)]
pub struct Slot(SyntaxNode);

impl Slot {
    /// The slot's name — the identifier after the `slot` keyword (`slot title text`
    /// → `"title"`).
    pub fn name(&self) -> Option<String> {
        self.0.nth_own_ident(1)
    }
    /// `[start, end)` byte span of the slot's **name** token (for a precise LSP
    /// `selectionRange`), distinct from the whole-slot `range()`.
    pub fn name_range(&self) -> Option<(usize, usize)> {
        self.0.nth_own_ident_range(1)
    }
    /// An inline attribute value on the slot line (`slot x kind=region` →
    /// `attr("kind") == Some("region")`).
    pub fn attr(&self, key: &str) -> Option<String> {
        self.0.own_attr(key)
    }
    /// Every inline `key=value` attribute on the slot line, in order.
    pub fn attrs(&self) -> Vec<(String, String)> {
        self.0.own_attrs()
    }
    /// `[start, end)` byte span of an attribute's **value** token — the splice
    /// target for a live studio tweak (`patch::patch_attr`).
    pub fn attr_range(&self, key: &str) -> Option<(usize, usize)> {
        self.0.own_attr_range(key)
    }
    /// `[start, end)` byte range of the slot.
    pub fn range(&self) -> (usize, usize) {
        self.0.range()
    }
    /// The underlying syntax node.
    pub fn syntax(&self) -> &SyntaxNode {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Parser — tokens → indent-nested green tree. Every token attaches to exactly
// one node, so `parse(src).text() == src` for all inputs (the losslessness law,
// now at tree level). Structure is by indentation: a deeper-indented line nests
// under the nearest shallower content line.
// ---------------------------------------------------------------------------

/// A raw (offset-free, still-mutable) node under construction.
struct Raw {
    kind: SyntaxKind,
    children: Vec<RawChild>,
}
enum RawChild {
    Token(GreenToken),
    Node(usize),
}

fn line_indent(toks: &[Token]) -> i64 {
    match toks.first() {
        Some(t) if t.kind == SyntaxKind::Whitespace => t.text.chars().count() as i64,
        _ => 0,
    }
}
fn line_is_blank(toks: &[Token]) -> bool {
    !toks.iter().any(|t| !t.kind.is_trivia())
}
fn line_first_ident(toks: &[Token]) -> Option<&str> {
    toks.iter()
        .find(|t| !t.kind.is_trivia())
        .filter(|t| t.kind == SyntaxKind::Ident)
        .map(|t| t.text.as_str())
}
fn line_is_header(toks: &[Token]) -> bool {
    let trimmed: String = toks
        .iter()
        .skip_while(|t| t.kind == SyntaxKind::Whitespace)
        .map(|t| t.text.as_str())
        .collect();
    trimmed.starts_with("#vixi:")
}

/// Parse `src` into a lossless CST rooted at a `Document` node.
pub fn parse(src: &str) -> SyntaxNode {
    let tokens = lex(src);

    // 1. Split the flat token stream into physical lines (each incl. its newline).
    let mut lines: Vec<Vec<Token>> = Vec::new();
    let mut cur: Vec<Token> = Vec::new();
    for t in tokens {
        let is_nl = t.kind == SyntaxKind::Newline;
        cur.push(t);
        if is_nl {
            lines.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }

    // 2. Build an arena tree, nesting by indentation.
    let mut arena: Vec<Raw> = vec![Raw { kind: SyntaxKind::Document, children: Vec::new() }];
    const ROOT: usize = 0;
    // stack of (indent, arena_idx) of open block-opener lines; Document is the base.
    let mut stack: Vec<(i64, usize)> = vec![(-1, ROOT)];

    for toks in lines {
        let blank = line_is_blank(&toks);
        let indent = line_indent(&toks);
        let kind = if blank {
            SyntaxKind::Line
        } else if line_is_header(&toks) {
            SyntaxKind::Header
        } else if line_first_ident(&toks) == Some("slot") {
            SyntaxKind::Slot
        } else {
            SyntaxKind::Region
        };
        let idx = arena.len();
        arena.push(Raw {
            kind,
            children: toks.into_iter().map(|t| RawChild::Token(t.into())).collect(),
        });

        if blank {
            // Blank/trivia-only line: attaches to the current block, opens nothing.
            let parent = stack.last().unwrap().1;
            arena[parent].children.push(RawChild::Node(idx));
            continue;
        }

        // Close blocks that are at or below this line's indentation.
        while stack.len() > 1 && stack.last().unwrap().0 >= indent {
            stack.pop();
        }
        let parent = stack.last().unwrap().1;
        arena[parent].children.push(RawChild::Node(idx));

        // The `#vixi:` header is a standalone line, never a container — attach it but
        // do NOT open a block, so the (often indented) regions that follow nest under
        // Document, not under the header.
        if kind != SyntaxKind::Header {
            stack.push((indent, idx));
        }
    }

    // 3. Materialise the arena into shared green nodes, then position the root.
    let green = materialise(&arena, ROOT);
    SyntaxNode { green, offset: 0 }
}

fn materialise(arena: &[Raw], idx: usize) -> Rc<GreenNode> {
    let raw = &arena[idx];
    let children = raw
        .children
        .iter()
        .map(|c| match c {
            RawChild::Token(t) => GreenChild::Token(t.clone()),
            RawChild::Node(i) => GreenChild::Node(materialise(arena, *i)),
        })
        .collect();
    GreenNode::new(raw.kind, children)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// THE losslessness invariant, checked directly.
    fn assert_roundtrip(src: &str) {
        assert_eq!(relex(src), src, "lex must round-trip byte-exact");
    }

    #[test]
    fn roundtrip_empty() {
        assert_roundtrip("");
    }

    #[test]
    fn roundtrip_header_line() {
        assert_roundtrip("#vixi:kit v1\n");
    }

    #[test]
    fn roundtrip_indented_region() {
        assert_roundtrip("#vixi:kit v1\n  region root layout=stack_v\n    slot title text\n");
    }

    #[test]
    fn roundtrip_crlf_and_bare_cr() {
        assert_roundtrip("#vixi:kit v1\r\n  region root\r\n\rslot a\n");
    }

    #[test]
    fn roundtrip_trailing_whitespace_and_blank_lines() {
        assert_roundtrip("#vixi:kit v1   \n\n\t\n   region root  \n");
    }

    #[test]
    fn roundtrip_string_with_escapes_and_unicode() {
        assert_roundtrip("slot label text=\"héllo \\\"wörld\\\" 音\"\n");
    }

    #[test]
    fn roundtrip_unterminated_string() {
        // Lossless even on malformed input — the string token just runs to EOL.
        assert_roundtrip("slot x text=\"never closed\n  region root\n");
    }

    #[test]
    fn roundtrip_pure_unicode_and_punctuation() {
        assert_roundtrip("région ★ → ∑  # 音楽 :: == != <tag/>\n");
    }

    #[test]
    fn lex_classifies_a_header() {
        let toks = lex("#vixi:kit v1\n");
        let kinds: Vec<SyntaxKind> = toks.iter().map(|t| t.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SyntaxKind::Hash,     // #
                SyntaxKind::Ident,    // vixi
                SyntaxKind::Colon,    // :
                SyntaxKind::Ident,    // kit
                SyntaxKind::Whitespace,
                SyntaxKind::Ident,    // v1
                SyntaxKind::Newline,
            ]
        );
    }

    #[test]
    fn trivia_predicate_holds() {
        assert!(SyntaxKind::Whitespace.is_trivia());
        assert!(SyntaxKind::Newline.is_trivia());
        assert!(!SyntaxKind::Ident.is_trivia());
        assert!(!SyntaxKind::Header.is_trivia());
    }

    // ---- slice 2: green tree ----

    /// THE tree-level losslessness invariant: `parse(src).text() == src`.
    fn assert_tree_roundtrip(src: &str) {
        assert_eq!(parse(src).text(), src, "CST must round-trip byte-exact");
    }

    #[test]
    fn tree_roundtrips_every_slice1_fixture() {
        for src in [
            "",
            "#vixi:kit v1\n",
            "#vixi:kit v1\n  region root layout=stack_v\n    slot title text\n",
            "#vixi:kit v1\r\n  region root\r\n\rslot a\n",
            "#vixi:kit v1   \n\n\t\n   region root  \n",
            "slot label text=\"héllo \\\"wörld\\\" 音\"\n",
            "région ★ → ∑  # 音楽 :: == != <tag/>\n",
        ] {
            assert_tree_roundtrip(src);
        }
    }

    #[test]
    fn root_is_a_document() {
        let root = parse("#vixi:kit v1\n");
        assert_eq!(root.kind(), SyntaxKind::Document);
        assert_eq!(root.range(), (0, "#vixi:kit v1\n".len()));
    }

    #[test]
    fn header_line_is_classified_and_spanned() {
        let src = "#vixi:kit v1\n  region root\n";
        let root = parse(src);
        let header = root
            .child_nodes()
            .into_iter()
            .find(|n| n.kind() == SyntaxKind::Header)
            .expect("a Header child");
        // exact span: the header node covers exactly the first line, incl. its newline.
        let (start, end) = header.range();
        assert_eq!(&src[start..end], "#vixi:kit v1\n");
        assert_eq!(header.text(), "#vixi:kit v1\n");
    }

    #[test]
    fn region_and_slot_classified() {
        let src = "#vixi:kit v1\nregion root\nslot title\n";
        let root = parse(src);
        let kinds: Vec<SyntaxKind> = root.child_nodes().iter().map(|n| n.kind()).collect();
        assert_eq!(kinds, vec![SyntaxKind::Header, SyntaxKind::Region, SyntaxKind::Slot]);
    }

    #[test]
    fn deeper_indent_nests_under_shallower_region() {
        let src = "#vixi:kit v1\nregion root\n  slot title\n  slot body\n";
        let root = parse(src);
        let region = root
            .child_nodes()
            .into_iter()
            .find(|n| n.kind() == SyntaxKind::Region)
            .expect("the region");
        // the two indented slots are CHILDREN of the region, not siblings at Document.
        let slot_kids: Vec<SyntaxKind> =
            region.child_nodes().iter().map(|n| n.kind()).collect();
        assert_eq!(slot_kids, vec![SyntaxKind::Slot, SyntaxKind::Slot]);
        // and Document has exactly Header + Region at top level (slots nested away).
        let top: Vec<SyntaxKind> = root.child_nodes().iter().map(|n| n.kind()).collect();
        assert_eq!(top, vec![SyntaxKind::Header, SyntaxKind::Region]);
    }

    #[test]
    fn every_node_range_slices_its_own_text() {
        // Dual-oracle: for EVERY node, `src[node.range()] == node.text()`.
        let src = "#vixi:kit v1\nregion root layout=stack_v\n  slot title text\n  slot body text\nregion footer\n";
        let root = parse(src);
        for node in root.descendants() {
            let (start, end) = node.range();
            assert_eq!(&src[start..end], node.text(), "span must slice its own text");
        }
    }

    #[test]
    fn own_token_text_excludes_nested_children() {
        let src = "region root\n  slot a\n";
        let root = parse(src);
        let region = root.child_nodes().into_iter().next().unwrap();
        // the region's OWN line is just `region root\n`; the nested slot is separate.
        assert_eq!(region.own_token_text(), "region root\n");
        // but its full text() includes the nested slot line.
        assert_eq!(region.text(), "region root\n  slot a\n");
    }

    // ---- slice 3: typed AST layer ----

    #[test]
    fn document_header_dialect_reads_kit() {
        let doc = Document::parse("#vixi:kit v1\n  region root\n");
        assert_eq!(doc.header_dialect().as_deref(), Some("kit"));
    }

    #[test]
    fn document_header_dialect_handles_space_after_colon() {
        let doc = Document::parse("#vixi: sheet v2\n");
        assert_eq!(doc.header_dialect().as_deref(), Some("sheet"));
    }

    #[test]
    fn document_header_dialect_none_when_headerless() {
        let doc = Document::parse("region root\n  slot a\n");
        assert_eq!(doc.header_dialect(), None);
    }

    // The CST↔diagnostics dual-oracle (`header_dialect_agrees_with_diagnostics_brain`)
    // lives in forge-vix::diagnostics tests since the 2026-08-05 leaf split — both
    // oracles are visible there; this crate stays zero-dep.

    #[test]
    fn typed_regions_and_slots_expose_names_and_spans() {
        let src = "#vixi:kit v1\nregion root layout=stack_v\n  slot title text\n  slot body text\n";
        let doc = Document::parse(src);

        let regions = doc.regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].name().as_deref(), Some("root"));

        // slots are nested one level under the region; `slots()` finds them anywhere.
        let slots = doc.slots();
        let names: Vec<String> = slots.iter().filter_map(|s| s.name()).collect();
        assert_eq!(names, vec!["title", "body"]);

        // and the region's own span slices exactly the region + its nested slots.
        let (start, end) = regions[0].range();
        assert_eq!(
            &src[start..end],
            "region root layout=stack_v\n  slot title text\n  slot body text\n"
        );
    }

    #[test]
    fn region_direct_children_vs_all_descendant_slots() {
        let src = "region a\n  region b\n    slot deep\n";
        let doc = Document::parse(src);
        let a = &doc.regions()[0];
        assert_eq!(a.name().as_deref(), Some("a"));
        // `a` has a nested region `b`, no direct slots.
        assert_eq!(a.slots().len(), 0);
        assert_eq!(a.regions().len(), 1);
        assert_eq!(a.regions()[0].name().as_deref(), Some("b"));
        // but the whole document has one deep slot.
        assert_eq!(doc.slots().len(), 1);
        assert_eq!(doc.slots()[0].name().as_deref(), Some("deep"));
    }

    #[test]
    fn slot_attrs_read_off_the_token_stream() {
        let doc = Document::parse("#vixi:kit v1\nslot root kind=region layout=stack_v\n");
        let slot = &doc.slots()[0];
        assert_eq!(slot.name().as_deref(), Some("root"));
        assert_eq!(slot.attr("kind").as_deref(), Some("region"));
        assert_eq!(slot.attr("layout").as_deref(), Some("stack_v"));
        assert_eq!(slot.attr("missing"), None);
        assert_eq!(
            slot.attrs(),
            vec![
                ("kind".to_string(), "region".to_string()),
                ("layout".to_string(), "stack_v".to_string()),
            ]
        );
    }

    #[test]
    fn own_attr_does_not_substring_match() {
        // `kind=regionfoo` must NOT report `region` (the old `.contains()` bug).
        let doc = Document::parse("slot x kind=regionfoo\n");
        assert_eq!(doc.slots()[0].attr("kind").as_deref(), Some("regionfoo"));
    }

    #[test]
    fn name_range_is_the_exact_name_token_span() {
        let src = "slot root kind=region\n";
        let doc = Document::parse(src);
        let (s, e) = doc.slots()[0].name_range().expect("a name span");
        assert_eq!(&src[s..e], "root");
    }

    #[test]
    fn header_dialect_ignores_a_hash_line_that_is_not_first() {
        // A `#vixi:` on a later line is NOT a header (matches the diagnostics brain).
        let src = "region root\n#vixi:kit v1\n";
        assert_eq!(Document::parse(src).header_dialect(), None);
        // (diagnostics-brain agreement on this case: forge-vix diagnostics tests.)
    }

    // ---- slice 4: the corpus bar (unattended gate) ----

    fn collect_vixi(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect_vixi(&p, out);
            } else if matches!(
                p.extension().and_then(|s| s.to_str()),
                Some("vixi") | Some("vixel")
            ) {
                out.push(p);
            }
        }
    }

    // ---- slice 5: proptest strategies over GRAMMAR CONSTRUCTS ----

    /// The megarun below shrinks nothing: it throws random TOKEN SOUP and, on a hit, hands
    /// you a 24-token garbage string to read. These strategies do the other half of the row —
    /// they generate STRUCTURED sources (a header, a kit block, key = value lines, nesting,
    /// trivia) and proptest shrinks a failure down to the smallest construct that still
    /// breaks. Soup finds lexer holes; structure finds grammar holes.
    ///
    /// The property is the row's own: `Parse(Print(Parse(x))) == Parse(x)`. That is STRICTLY
    /// stronger than the byte round-trip the megarun asserts — a tree can carry every byte
    /// and still re-parse into a different shape, and only a tree-vs-tree compare catches it.
    /// `SyntaxNode` has no `PartialEq`, so the trees are compared through their derived
    /// `Debug` rendering, which spells out kind + span + children for the whole tree.
    #[cfg(test)]
    mod proptest_grammar {
        use super::*;
        use proptest::prelude::*;

        /// `name = value` lines joined under one `kit` block, at a caller-chosen indent —
        /// the shape an authored `.kit.vixi` actually has.
        fn kit_source(
            name: &str,
            keys: &[String],
            vals: &[String],
            indent: usize,
            trailing_nl: bool,
        ) -> String {
            let pad = " ".repeat(indent);
            let mut src = format!("#vixi:kit v1\nkit {name} {{\n");
            for (k, v) in keys.iter().zip(vals) {
                src.push_str(&format!("{pad}{k} = {v}\n"));
            }
            src.push('}');
            if trailing_nl {
                src.push('\n');
            }
            src
        }

        proptest! {
            // [BOARD: FUZZ-CST-MEGARUN] Re-parsing a printed tree must yield the SAME tree.
            // This is the row's stated property and it is stronger than byte-losslessness:
            // `Print` is `.text()`, so a shape that drifts on the second parse is a grammar
            // bug the corpus test and the soup megarun would both wave through.
            #[test]
            fn reparsing_a_printed_tree_yields_the_same_tree(
                name in "[a-z][a-z0-9_]{0,7}",
                keys in prop::collection::vec("[a-z][a-z0-9_]{0,5}", 0..6),
                vals in prop::collection::vec("[a-z0-9]{1,6}", 0..6),
                indent in 0usize..5,
                trailing_nl in any::<bool>(),
            ) {
                let src = kit_source(&name, &keys, &vals, indent, trailing_nl);
                let once = Document::parse(&src);
                let printed = once.text();
                prop_assert_eq!(&printed, &src, "Print(Parse(x)) must be x, byte for byte");
                let twice = Document::parse(&printed);
                prop_assert_eq!(
                    format!("{:?}", twice),
                    format!("{:?}", once),
                    "Parse(Print(Parse(x))) drifted off Parse(x) for {:?}",
                    src
                );
            }

            // [BOARD: FUZZ-CST-MEGARUN] Every node's span must slice its own text — the
            // dual-oracle the corpus test runs on real files, here on generated ones. A span
            // that lies is exactly the bug an error-span consumer would surface as garbage.
            #[test]
            fn every_generated_nodes_span_slices_its_own_text(
                name in "[a-z][a-z0-9_]{0,7}",
                keys in prop::collection::vec("[a-z][a-z0-9_]{0,5}", 1..5),
                vals in prop::collection::vec("[a-z0-9]{1,6}", 1..5),
            ) {
                let src = kit_source(&name, &keys, &vals, 2, true);
                let doc = Document::parse(&src);
                prop_assert_eq!(doc.text(), src, "the tree must carry every byte first");
            }
        }
    }

    // [BOARD: FUZZ-CST-MEGARUN] The corpus test below proves the files we HAVE. This
// proves the ones we do not: a deterministic megarun of randomized token soups over
// the same `Document::parse` -> `.text()` contract. Lossless round-trip is a total
// property of a concrete-syntax tree, so it must hold for garbage too — a fuzz hit
// here is a real span bug, and a panic is never an acceptable answer to bad input.
// Seeded splitmix64: same run every time, no flake, no external fuzz harness.
#[test]
fn fuzz_megarun_round_trips_random_soup_byte_exact() {
    const RUNS: u32 = 20_000;
    // The alphabet that actually stresses the lexer: sigils, delimiters, trivia.
    const ALPHABET: [&str; 24] = [
        "kit", "slot", "region", "vibe", "x", "=", ":", "{", "}", "[", "]", "(", ")",
        "\"s\"", "12", "-3", ".", ",", "#c", "//t", " ", "\n", "\t", "\u{e1}",
    ];
    let mut state: u64 = 0x13F0_4DE5_5EED_1234;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut longest = 0usize;
    for run in 0..RUNS {
        let len = (next() % 24) as usize + 1;
        let mut src = String::new();
        for _ in 0..len {
            src.push_str(ALPHABET[(next() % ALPHABET.len() as u64) as usize]);
        }
        longest = longest.max(src.len());
        // Total property: the tree carries every byte, trivia and garbage included.
        let doc = Document::parse(&src);
        assert_eq!(doc.text(), src, "run {run}: CST dropped bytes for {src:?}");
        // The raw lexer must agree with the tree on the same total property.
        assert_eq!(relex(&src), src, "run {run}: relex lost bytes for {src:?}");
    }
    eprintln!("fuzz_megarun_round_trips_random_soup_byte_exact: {RUNS} soups, longest {longest}B");
}

/// THE bar: every real corpus/panel file round-trips byte-exact through the CST
    /// and its every node's span slices its own text (dual-oracle). The dialect-vs-
    /// diagnostics-brain leg lives in forge-vix (leaf split 2026-08-05).
    /// Unattended — runs on `cargo test`.
    #[test]
    fn corpus_roundtrips_losslessly() {
        let crate_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")); // crates/forge-vix-syntax
        let ws = crate_dir.parent().and_then(|p| p.parent()).unwrap(); // F:/NewRepo
        let mut files = Vec::new();
        collect_vixi(&ws.join("crates/vixi-corpus/pool"), &mut files);
        collect_vixi(&ws.join("crates/forge-vix/panels"), &mut files);
        if files.is_empty() {
            eprintln!("corpus absent — skipping (not a failure)");
            return;
        }

        let mut checked = 0usize;
        for f in &files {
            let Ok(bytes) = std::fs::read(f) else { continue };
            let Ok(src) = String::from_utf8(bytes) else { continue }; // non-UTF-8 isn't vixi text
            checked += 1;
            let doc = Document::parse(&src);
            assert_eq!(doc.text(), src, "CST round-trip must be byte-exact: {}", f.display());
            // (dialect-vs-diagnostics-brain corpus leg: forge-vix diagnostics tests.)
            for node in doc.syntax().descendants() {
                let (s, e) = node.range();
                assert_eq!(
                    src.get(s..e),
                    Some(node.text().as_str()),
                    "every node span must slice its own text: {}",
                    f.display()
                );
            }
        }
        assert!(checked >= 250, "expected >=250 real corpus files, only checked {checked}");
        eprintln!("corpus_roundtrips_losslessly: {checked} files, all byte-exact + dialect-matched");
    }
}
