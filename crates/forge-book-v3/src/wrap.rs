//! Wrap — greedy integer word-wrap a string to a column width (in chars). For
//! the canvas text layout; preserves existing line breaks.

/// Wrap `text` to `width` chars per line (0 = no wrapping).
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.lines().map(String::from).collect();
    }
    let mut out = Vec::new();
    for line in text.lines() {
        let mut cur = String::new();
        for word in line.split_whitespace() {
            let wlen = word.chars().count();
            if cur.is_empty() {
                cur = word.to_string();
            } else if cur.chars().count() + 1 + wlen <= width {
                cur.push(' ');
                cur.push_str(word);
            } else {
                out.push(std::mem::take(&mut cur));
                cur = word.to_string();
            }
        }
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_at_word_boundaries() {
        let lines = wrap("the six in one is the best tool", 12);
        assert!(lines.iter().all(|l| l.chars().count() <= 12));
        assert!(lines.len() > 1);
        // no word is split
        assert!(lines.join(" ").split_whitespace().eq("the six in one is the best tool".split_whitespace()));
    }

    #[test]
    fn preserves_explicit_breaks() {
        let lines = wrap("a\nb", 80);
        assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn zero_width_is_passthrough() {
        assert_eq!(wrap("one two", 0), vec!["one two".to_string()]);
    }
}
