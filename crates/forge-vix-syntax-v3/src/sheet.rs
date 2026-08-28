//! .sheet.vixi parser — lifted verbatim from forge-canvas/build.rs (DEBT-SHEET-AOT)
//! to centralize hex/mu/ms/permyriad value parsing. Consumed by build.rs in forge-canvas, technothesia.

use std::path::Path;

/// A parsed `.sheet.vixi` palette — layer/name header plus its ordered token table.
pub struct Sheet {
    /// The `layer:` header value.
    pub layer: String,
    /// The `name:` header value.
    pub name: String,
    /// Ordered `(key, value)` pairs, values already resolved to packed u32.
    pub tokens: Vec<(String, u32)>,
}

/// Parse a `.sheet.vixi`. `#` is a colour prefix ONLY on the RHS of `=`; a `#` at
/// line-start is the magic header or a comment (colours never start a line).
pub fn parse_sheet(src: &str, path: &Path) -> Sheet {
    let mut layer = String::new();
    let mut name = String::new();
    let mut tokens = Vec::new();

    for (i, raw) in src.lines().enumerate() {
        let ln = i + 1;
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("#vixi:sheet") {
            if !t.contains("v1") {
                panic!("{}:{ln}: unsupported sheet version (expected v1): {t}", path.display());
            }
            continue;
        }
        if t.starts_with('#') {
            continue; // full-line comment
        }
        if let Some(rest) = t.strip_prefix("layer:") {
            layer = rest.trim().to_string();
            continue;
        }
        if let Some(rest) = t.strip_prefix("name:") {
            name = rest.trim().to_string();
            continue;
        }
        // token line: `key = value [# inline comment]`
        let (key, rhs) = t
            .split_once('=')
            .unwrap_or_else(|| panic!("{}:{ln}: expected 'key = value', got: {t}", path.display()));
        let key = key.trim();
        let val_tok = rhs
            .split_whitespace()
            .next()
            .unwrap_or_else(|| panic!("{}:{ln}: missing value for '{key}'", path.display()));
        let value = parse_value(val_tok, path, ln);
        tokens.push((key.to_string(), value));
    }

    if name.is_empty() {
        panic!("{}: missing 'name:' header", path.display());
    }
    if layer.is_empty() {
        panic!("{}: missing 'layer:' header", path.display());
    }
    Sheet { layer, name, tokens }
}

/// `#RRGGBBAA` / `#RRGGBB` → packed RGBA; `mu(N)`→N*1000; `ms(N)`/`permyriad(N)`→N.
pub fn parse_value(tok: &str, path: &Path, ln: usize) -> u32 {
    if let Some(hex) = tok.strip_prefix('#') {
        match hex.len() {
            8 => u32::from_str_radix(hex, 16)
                .unwrap_or_else(|_| panic!("{}:{ln}: bad hex colour #{hex}", path.display())),
            6 => {
                let rgb = u32::from_str_radix(hex, 16)
                    .unwrap_or_else(|_| panic!("{}:{ln}: bad hex colour #{hex}", path.display()));
                (rgb << 8) | 0xFF
            }
            _ => panic!("{}:{ln}: colour must be #RRGGBB or #RRGGBBAA, got #{hex}", path.display()),
        }
    } else if let Some(inner) = wrapped(tok, "mu") {
        parse_int(inner, path, ln) * 1000
    } else if let Some(inner) = wrapped(tok, "ms") {
        parse_int(inner, path, ln)
    } else if let Some(inner) = wrapped(tok, "permyriad") {
        parse_int(inner, path, ln)
    } else {
        panic!(
            "{}:{ln}: bad value '{tok}' (expected #hex, mu(N), ms(N), permyriad(N))",
            path.display()
        )
    }
}

/// `name(inner)` → `Some(inner)`.
pub fn wrapped<'a>(tok: &'a str, name: &str) -> Option<&'a str> {
    tok.strip_prefix(name)
        .and_then(|s| s.strip_prefix('('))
        .and_then(|s| s.strip_suffix(')'))
}

/// Parse a bare decimal integer, panicking with `path:ln` context on failure.
pub fn parse_int(s: &str, path: &Path, ln: usize) -> u32 {
    s.trim()
        .parse::<u32>()
        .unwrap_or_else(|_| panic!("{}:{ln}: expected integer, got '{s}'", path.display()))
}

/// `tab_bg_active` → `TabBgActive`. The TokenId variant naming is a pure
/// snake→Pascal transform of the `from_name` keys (verified: no exceptions).
pub fn snake_to_pascal(s: &str) -> String {
    s.split('_')
        .map(|seg| {
            let mut c = seg.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
