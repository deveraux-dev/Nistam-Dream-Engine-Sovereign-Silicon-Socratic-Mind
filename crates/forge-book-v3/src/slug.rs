//! Slug — slugify a title into an anchor id: lowercase, alphanumerics kept,
//! everything else collapsed to single hyphens.

/// Turn `s` into a URL/anchor slug.
pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_and_lowercases() {
        assert_eq!(slugify("The Belt"), "the-belt");
        assert_eq!(slugify("Skies  of --- the Void!"), "skies-of-the-void");
        assert_eq!(slugify("Field Notes 2026"), "field-notes-2026");
    }

    #[test]
    fn trims_edges() {
        assert_eq!(slugify("  —hi—  "), "hi");
        assert_eq!(slugify("!!!"), "");
    }
}
