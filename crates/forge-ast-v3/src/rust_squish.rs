//! Rust symbol extraction over tree-sitter-rust: `squish` (declarations, bodies
//! dropped) and `signatures` (per-symbol path/shape/calls/tokens).
//! Ported from F:\NewRepo\crates\forge-ast\src\rust_squish.rs 2026-08-25.

use std::collections::BTreeSet;
use tree_sitter::{Node, Parser};

/// Parse Rust source and extract the recipe (signatures + types + docs).
pub fn squish(source: &str) -> String {
    let mut parser = Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("failed to load tree-sitter-rust grammar");

    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return source.to_string(),
    };

    let root = tree.root_node();
    let src = source.as_bytes();
    let mut output = String::with_capacity(source.len() / 2);

    extract_node(root, src, &mut output, 0);

    let mut collapsed = String::with_capacity(output.len());
    let mut blank_count = 0;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                collapsed.push('\n');
            }
        } else {
            blank_count = 0;
            collapsed.push_str(line);
            collapsed.push('\n');
        }
    }

    collapsed.trim().to_string()
}

/// Recursively walk AST nodes, extracting recipe-relevant content.
#[allow(clippy::only_used_in_recursion)]
fn extract_node(node: Node, src: &[u8], out: &mut String, depth: usize) {
    let kind = node.kind();

    if kind == "mod_item" && is_cfg_test(&node, src) {
        return;
    }

    if kind == "attribute_item" || kind == "inner_attribute_item" {
        let text = node_text(&node, src);
        if is_low_value_attr(&text) {
            return;
        }
    }

    if kind == "block_comment" {
        return;
    }

    if kind == "line_comment" {
        let text = node_text(&node, src).trim_start().to_string();
        if text.starts_with("///") || text.starts_with("//!") {
            out.push_str(&node_text(&node, src));
            out.push('\n');
        }
        return;
    }

    match kind {
        "use_declaration" | "struct_item" | "enum_item" | "union_item" | "trait_item"
        | "type_item" | "const_item" | "static_item" => {
            out.push_str(&node_text(&node, src));
            out.push_str("\n\n");
        }

        "mod_item" => {
            let name = child_by_field(&node, "name", src).unwrap_or_default();
            if let Some(body) = node.child_by_field_name("body") {
                out.push_str(&format!("mod {} {{\n", name));
                for i in 0..body.named_child_count() {
                    if let Some(child) = body.named_child(i) {
                        extract_node(child, src, out, depth + 1);
                    }
                }
                out.push_str("}\n\n");
            } else {
                out.push_str(&node_text(&node, src));
                out.push('\n');
            }
        }

        "impl_item" => {
            extract_impl(node, src, out);
        }

        "function_item" => {
            extract_fn_signature(node, src, out);
        }

        "macro_definition" => {
            let name = child_by_field(&node, "name", src).unwrap_or_default();
            out.push_str(&format!("macro_rules! {} {{ /* ... */ }}\n\n", name));
        }

        "source_file" | "declaration_list" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    extract_node(child, src, out, depth + 1);
                }
            }
        }

        _ => {}
    }
}

/// Extract an impl block: header + method signatures.
fn extract_impl(node: Node, src: &[u8], out: &mut String) {
    let mut header = String::new();
    for i in 0..node.child_count() {
        let child = node.child(i).unwrap();
        if child.kind() == "declaration_list" {
            break;
        }
        header.push_str(node_text(&child, src).trim());
        header.push(' ');
    }
    out.push_str(&format!("{}{{\n", header.trim()));

    if let Some(body) = node.child_by_field_name("body") {
        for i in 0..body.named_child_count() {
            if let Some(child) = body.named_child(i) {
                match child.kind() {
                    "function_item" => {
                        out.push_str("    ");
                        extract_fn_signature(child, src, out);
                    }
                    "line_comment" => {
                        let text = node_text(&child, src).trim_start().to_string();
                        if text.starts_with("///") || text.starts_with("//!") {
                            out.push_str("    ");
                            out.push_str(&node_text(&child, src));
                            out.push('\n');
                        }
                    }
                    "const_item" | "type_item" => {
                        out.push_str("    ");
                        out.push_str(&node_text(&child, src));
                        out.push('\n');
                    }
                    _ => {}
                }
            }
        }
    }

    out.push_str("}\n\n");
}

/// Extract a function signature, dropping the body.
fn extract_fn_signature(node: Node, src: &[u8], out: &mut String) {
    let full = node_text(&node, src);

    if let Some(body) = node.child_by_field_name("body") {
        let body_start = body.start_byte() - node.start_byte();
        let sig = &full[..body_start].trim_end();
        out.push_str(sig);
        out.push_str(";\n");
    } else {
        out.push_str(&full);
        out.push('\n');
    }
}

/// Check if a mod_item has `#[cfg(test)]` attribute.
fn is_cfg_test(node: &Node, src: &[u8]) -> bool {
    let mut prev = node.prev_named_sibling();
    while let Some(sib) = prev {
        if sib.kind() == "attribute_item" {
            let text = node_text(&sib, src);
            if text.contains("cfg(test)") {
                return true;
            }
        } else {
            break;
        }
        prev = sib.prev_named_sibling();
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "attribute_item" {
                let text = node_text(&child, src);
                if text.contains("cfg(test)") {
                    return true;
                }
            }
        }
    }

    false
}

/// Check if an attribute is low-value for training.
fn is_low_value_attr(text: &str) -> bool {
    let low = ["#[allow", "#[cfg(", "#[test", "#[ignore", "#[bench", "#[derive(Debug", "#[inline"];
    low.iter().any(|p| text.contains(p))
}

/// Get the text content of a node.
fn node_text(node: &Node, src: &[u8]) -> String {
    let start = node.start_byte();
    let end = node.end_byte().min(src.len());
    String::from_utf8_lossy(&src[start..end]).to_string()
}

/// Get text of a named field child.
fn child_by_field(node: &Node, field: &str, src: &[u8]) -> Option<String> {
    node.child_by_field_name(field).map(|n| node_text(&n, src))
}

/// Per-symbol discrimination record: param/return shape, call-set, body token-set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymSig {
    /// Scope-joined symbol path, e.g. `router::Frame::tile`.
    pub path: String,
    /// Parameter list and return type, whitespace-collapsed.
    pub shape: String,
    /// Leaf-most callee names invoked in the body, sorted.
    pub calls: Vec<String>,
    /// Identifier tokens appearing in the body, sorted.
    pub tokens: Vec<String>,
}

impl SymSig {
    /// One whitespace-separated line, so the distributional embedder rides it unchanged.
    pub fn line(&self) -> String {
        let mut s = String::with_capacity(self.shape.len() + 64);
        s.push_str(&self.path);
        s.push(' ');
        s.push_str(&self.shape);
        for c in &self.calls {
            s.push_str(" call:");
            s.push_str(c);
        }
        for t in &self.tokens {
            s.push(' ');
            s.push_str(t);
        }
        s
    }
}

/// Extract one [`SymSig`] per function/method in `source`. `#[cfg(test)]` mods
/// are skipped, matching [`squish`].
pub fn signatures(source: &str) -> Vec<SymSig> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("failed to load tree-sitter-rust grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    walk_sigs(tree.root_node(), source.as_bytes(), "", &mut out);
    out
}

fn walk_sigs(node: Node, src: &[u8], scope: &str, out: &mut Vec<SymSig>) {
    match node.kind() {
        "mod_item" if is_cfg_test(&node, src) => {}
        "function_item" => out.push(signature_of(node, src, scope)),
        "impl_item" | "mod_item" => {
            let field = if node.kind() == "impl_item" { "type" } else { "name" };
            let inner = join_scope(scope, &child_by_field(&node, field, src).unwrap_or_default());
            if let Some(body) = node.child_by_field_name("body") {
                recurse_named(body, src, &inner, out);
            }
        }
        "source_file" | "declaration_list" | "trait_item" => recurse_named(node, src, scope, out),
        _ => {}
    }
}

fn recurse_named(node: Node, src: &[u8], scope: &str, out: &mut Vec<SymSig>) {
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            walk_sigs(child, src, scope, out);
        }
    }
}

fn signature_of(node: Node, src: &[u8], scope: &str) -> SymSig {
    let name = child_by_field(&node, "name", src).unwrap_or_default();
    let params = child_by_field(&node, "parameters", src).unwrap_or_default();
    let ret = child_by_field(&node, "return_type", src).unwrap_or_default();
    let shape =
        collapse_ws(&if ret.is_empty() { params } else { format!("{} -> {}", params, ret) });

    let (mut calls, mut tokens) = (BTreeSet::new(), BTreeSet::new());
    if let Some(body) = node.child_by_field_name("body") {
        scan_body(body, src, &mut calls, &mut tokens);
    }
    SymSig {
        path: join_scope(scope, &name),
        shape,
        calls: calls.into_iter().collect(),
        tokens: tokens.into_iter().collect(),
    }
}

fn scan_body(node: Node, src: &[u8], calls: &mut BTreeSet<String>, tokens: &mut BTreeSet<String>) {
    match node.kind() {
        "call_expression" => {
            if let Some(f) = node.child_by_field_name("function") {
                calls.insert(callee_name(&f, src));
            }
        }
        "macro_invocation" => {
            if let Some(m) = node.child_by_field_name("macro") {
                calls.insert(format!("{}!", node_text(&m, src)));
            }
        }
        "identifier" | "field_identifier" | "type_identifier" => {
            tokens.insert(node_text(&node, src));
        }
        _ => {}
    }
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            scan_body(child, src, calls, tokens);
        }
    }
}

/// Leaf-most callee name: `self.foo()` -> `foo`, `a::b::c()` -> `c`, `f()` -> `f`.
fn callee_name(node: &Node, src: &[u8]) -> String {
    let mut cur = *node;
    loop {
        if matches!(cur.kind(), "identifier" | "field_identifier") {
            return node_text(&cur, src);
        }
        let mut next = None;
        for i in (0..cur.child_count()).rev() {
            let child = cur.child(i).unwrap();
            if matches!(
                child.kind(),
                "identifier"
                    | "field_identifier"
                    | "scoped_identifier"
                    | "field_expression"
                    | "generic_function"
            ) {
                next = Some(child);
                break;
            }
        }
        match next {
            Some(n) => cur = n,
            None => return "<expr>".to_string(),
        }
    }
}

fn join_scope(scope: &str, name: &str) -> String {
    let name = collapse_ws(name);
    if scope.is_empty() {
        name
    } else {
        format!("{}::{}", scope, name)
    }
}

fn collapse_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A declared Rust item: what it is, not merely that its name appears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymItem {
    /// Scope-joined path, e.g. `router::Frame::tile`.
    pub path: String,
    /// Declaration keyword: `fn`, `struct`, `enum`, `union`, `trait`, `type`,
    /// `const`, `static`, `mod`, `macro`.
    pub kind: &'static str,
    /// 1-based line of the declaration.
    pub line: usize,
    /// Whether the declaration carries a `pub` visibility modifier.
    pub is_pub: bool,
}

/// Extract every declared item in `source`. `#[cfg(test)]` mods are skipped,
/// matching [`squish`]. A name inside a comment or string literal is not an item.
pub fn items(source: &str) -> Vec<SymItem> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .expect("failed to load tree-sitter-rust grammar");
    let tree = match parser.parse(source, None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    walk_items(tree.root_node(), source.as_bytes(), "", &mut out);
    out
}

fn kind_of(node_kind: &str) -> Option<&'static str> {
    Some(match node_kind {
        "function_item" | "function_signature_item" => "fn",
        "struct_item" => "struct",
        "enum_item" => "enum",
        "union_item" => "union",
        "trait_item" => "trait",
        "type_item" => "type",
        "const_item" => "const",
        "static_item" => "static",
        "mod_item" => "mod",
        "macro_definition" => "macro",
        _ => return None,
    })
}

fn walk_items(node: Node, src: &[u8], scope: &str, out: &mut Vec<SymItem>) {
    let nk = node.kind();

    if nk == "mod_item" && is_cfg_test(&node, src) {
        return;
    }

    if let Some(kind) = kind_of(nk) {
        let name = child_by_field(&node, "name", src).unwrap_or_default();
        if !name.is_empty() {
            out.push(SymItem {
                path: join_scope(scope, &name),
                kind,
                line: node.start_position().row + 1,
                is_pub: has_pub(&node, src),
            });
        }
    }

    match nk {
        "impl_item" | "mod_item" | "trait_item" => {
            let field = if nk == "impl_item" { "type" } else { "name" };
            let inner = join_scope(scope, &child_by_field(&node, field, src).unwrap_or_default());
            if let Some(body) = node.child_by_field_name("body") {
                for i in 0..body.named_child_count() {
                    if let Some(child) = body.named_child(i) {
                        walk_items(child, src, &inner, out);
                    }
                }
            }
        }
        "source_file" | "declaration_list" => {
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    walk_items(child, src, scope, out);
                }
            }
        }
        _ => {}
    }
}

fn has_pub(node: &Node, src: &[u8]) -> bool {
    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "visibility_modifier" {
                return node_text(&child, src).starts_with("pub");
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_pub_fn_signature() {
        let src = r#"
pub fn hello(name: &str) -> String {
    format!("Hello, {name}")
}
"#;
        let out = squish(src);
        assert!(out.contains("pub fn hello(name: &str) -> String;"));
        assert!(!out.contains("format!"));
    }

    #[test]
    fn strips_cfg_test_module() {
        let src = r#"
pub fn real_code() -> i32 { 42 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn it_works() {
        assert_eq!(real_code(), 42);
    }
}
"#;
        let out = squish(src);
        assert!(out.contains("pub fn real_code"));
        assert!(!out.contains("it_works"));
        assert!(!out.contains("mod tests"));
    }

    #[test]
    fn keeps_impl_with_method_signatures() {
        let src = r#"
impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height, data: vec![] }
    }
}
"#;
        let out = squish(src);
        assert!(out.contains("impl Frame"));
        assert!(out.contains("pub fn new(width: u32, height: u32) -> Self;"));
        assert!(!out.contains("vec![]"));
    }

    #[test]
    fn signature_paths_scope_through_impl_and_mod() {
        let src = r#"
mod router {
    impl Frame {
        pub fn tile(&self) -> u32 { self.width }
    }
}
"#;
        let sigs = signatures(src);
        assert_eq!(sigs.len(), 1);
        assert_eq!(sigs[0].path, "router::Frame::tile");
    }

    #[test]
    fn signature_callee_is_leafmost() {
        let src = "fn f() { self.inner.push(x); crate::a::b(); log!(y); }";
        assert_eq!(signatures(src)[0].calls, ["b", "log!", "push"]);
    }

    #[test]
    fn empty_input() {
        assert_eq!(squish(""), "");
    }

    // The grep-vs-AST line: a name in a comment or a string is a string hit, not a
    // symbol. `kind != existence` — the failure this module exists to end.
    #[test]
    fn a_mention_in_a_comment_or_string_is_not_a_symbol() {
        let src = r#"
// pub fn ghost_fn() -> u8 { 0 }
/// see also pub struct GhostStruct
pub fn real_fn() -> &'static str {
    "pub fn stringy_fn() -> u8"
}
"#;
        let paths: Vec<String> = signatures(src).into_iter().map(|s| s.path).collect();
        assert_eq!(paths, ["real_fn"], "only the declared fn is a symbol");
        assert!(!paths.iter().any(|p| p == "ghost_fn"));
        assert!(!paths.iter().any(|p| p == "stringy_fn"));
    }

    #[test]
    fn items_carry_kind_not_just_existence() {
        let src = r#"
pub struct SqFixtureA { pub w: u32 }
pub enum SqFixtureB { A, B }
pub trait SqFixtureC { fn go(&self); }
pub const SQ_FIXTURE_LIMIT: u32 = 8;
static SQ_FIXTURE_COUNT: u32 = 0;
pub type SqFixtureAlias = u32;
fn sq_fixture_helper() {}
"#;
        let found = items(src);
        let got: Vec<(&str, &str, bool)> =
            found.iter().map(|i| (i.path.as_str(), i.kind, i.is_pub)).collect();
        assert!(got.contains(&("SqFixtureA", "struct", true)));
        assert!(got.contains(&("SqFixtureB", "enum", true)));
        assert!(got.contains(&("SqFixtureC", "trait", true)));
        assert!(got.contains(&("SqFixtureC::go", "fn", false)));
        assert!(got.contains(&("SQ_FIXTURE_LIMIT", "const", true)));
        assert!(got.contains(&("SQ_FIXTURE_COUNT", "static", false)));
        assert!(got.contains(&("SqFixtureAlias", "type", true)));
        assert!(got.contains(&("sq_fixture_helper", "fn", false)));
    }

    #[test]
    fn items_report_the_declaration_line() {
        let src = "\n\npub struct SqFixtureLine;\n";
        let f = items(src).into_iter().find(|i| i.path == "SqFixtureLine").unwrap();
        assert_eq!(f.line, 3, "1-based line of the declaration");
    }

    #[test]
    fn a_commented_or_stringy_item_is_not_an_item() {
        let src = r#"
// pub struct SqGhostA { }
/// pub enum SqGhostB { }
pub fn sq_real() -> &'static str { "pub struct SqStringy;" }
"#;
        let names: Vec<String> = items(src).into_iter().map(|i| i.path).collect();
        assert_eq!(names, ["sq_real"]);
    }

    #[test]
    fn same_name_different_kind_is_discriminated() {
        let src = "pub struct SqDual;\npub fn SqDual() {}\n";
        let kinds: Vec<&str> =
            items(src).iter().filter(|i| i.path == "SqDual").map(|i| i.kind).collect();
        assert_eq!(kinds, ["struct", "fn"], "kind != existence");
    }
}
