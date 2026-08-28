//! Position resolver — the ONE genuinely new piece (PLAN §1c).
//!
//! LSP `Position` is **0-based line, 0-based `character` in UTF-16 code units**.
//! Source text is UTF-8. This module converts an LSP position to a byte offset and
//! extracts the identifier under it, correctly across multi-byte (`é` = 1 UTF-16
//! unit, 2 bytes) and surrogate-pair (`😀` = 2 UTF-16 units, 4 bytes) characters.
//! A naive `column - 1` byte assumption is the off-by-one the DA pass targets (§6b).

/// Byte range `[start, end)` of the 0-based `line` within `src`, excluding the
/// line terminator. `None` if `line` is past the end of the document.
pub fn line_byte_range(src: &str, line: u32) -> Option<(usize, usize)> {
    let mut start = 0usize;
    let mut current = 0u32;
    let bytes = src.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if current == line {
            // Find end of this line (before '\n', trimming a trailing '\r').
            let mut end = i;
            while end < bytes.len() && bytes[end] != b'\n' {
                end += 1;
            }
            let mut line_end = end;
            if line_end > i && bytes[line_end - 1] == b'\r' {
                line_end -= 1;
            }
            return Some((i, line_end));
        }
        if bytes[i] == b'\n' {
            current += 1;
            start = i + 1;
        }
        i += 1;
    }
    // The final line (no trailing '\n').
    if current == line {
        return Some((start, bytes.len()));
    }
    None
}

/// Within a single line slice, convert a 0-based UTF-16 `character` index to a byte
/// index. Clamps to the line's byte length when `character` runs past the end.
pub fn utf16_char_to_byte_col(line: &str, character: u32) -> usize {
    let target = character as usize;
    let mut u16_seen = 0usize;
    for (byte_idx, ch) in line.char_indices() {
        if u16_seen >= target {
            return byte_idx;
        }
        u16_seen += ch.len_utf16();
    }
    line.len()
}

/// Extract the VixiScript identifier covering byte index `col` in `line`. An
/// identifier is a run of `[A-Za-z0-9_.]` (dotted forms like `widget.button` and
/// `type.ramp` are single tokens in the grammar). Returns `None` if `col` sits on
/// whitespace/punctuation with no adjacent identifier. ASCII-only word chars make
/// byte scanning safe across multi-byte neighbours (a UTF-8 continuation byte is
/// `>= 0x80` and never matches `is_word`, so it is a hard boundary).
pub fn word_at_byte(line: &str, col: usize) -> Option<&str> {
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'.';
    let bytes = line.as_bytes();
    let col = col.min(line.len());
    let mut start = col;
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < line.len() && is_word(bytes[end]) {
        end += 1;
    }
    if start == end {
        return None;
    }
    Some(&line[start..end])
}

/// Convenience: the identifier at an LSP `(line, character)` position in `src`.
pub fn word_at_position(src: &str, line: u32, character: u32) -> Option<&str> {
    let (lo, hi) = line_byte_range(src, line)?;
    let line_str = &src[lo..hi];
    let col = utf16_char_to_byte_col(line_str, character);
    word_at_byte(line_str, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_range_basic_and_final_line() {
        let src = "alpha\nbeta\ngamma";
        assert_eq!(line_byte_range(src, 0), Some((0, 5))); // "alpha"
        assert_eq!(line_byte_range(src, 1), Some((6, 10))); // "beta"
        assert_eq!(line_byte_range(src, 2), Some((11, 16))); // "gamma" (no trailing \n)
        assert_eq!(line_byte_range(src, 3), None);
    }

    #[test]
    fn line_range_trims_crlf() {
        let src = "a\r\nbb\r\n";
        assert_eq!(line_byte_range(src, 0), Some((0, 1))); // "a", \r trimmed
        assert_eq!(line_byte_range(src, 1), Some((3, 5))); // "bb"
    }

    #[test]
    fn utf16_ascii_is_identity() {
        assert_eq!(utf16_char_to_byte_col("slot", 0), 0);
        assert_eq!(utf16_char_to_byte_col("slot", 2), 2);
        assert_eq!(utf16_char_to_byte_col("slot", 99), 4); // clamps to end
    }

    #[test]
    fn utf16_multibyte_char_counts_one_unit() {
        // "héllo": h(1u16,1B) é(1u16,2B) l l o. char index 2 ('l') is byte 3.
        let line = "héllo";
        assert_eq!(utf16_char_to_byte_col(line, 2), 3);
    }

    #[test]
    fn utf16_surrogate_pair_counts_two_units() {
        // "😀x": 😀 is 2 UTF-16 units / 4 bytes; 'x' starts at byte 4 after 2 units.
        let line = "😀x";
        assert_eq!(utf16_char_to_byte_col(line, 2), 4);
    }

    #[test]
    fn word_at_byte_extracts_dotted_identifier() {
        let line = "slot main kind=widget name=widget.button";
        // cursor inside "widget.button"
        let col = line.find("widget.button").unwrap() + 3;
        assert_eq!(word_at_byte(line, col), Some("widget.button"));
    }

    #[test]
    fn word_at_byte_none_on_whitespace() {
        let line = "slot   main";
        assert_eq!(word_at_byte(line, 5), None); // in the gap
    }

    #[test]
    fn word_at_byte_at_end_of_word() {
        let line = "region";
        assert_eq!(word_at_byte(line, 6), Some("region")); // cursor just past last char
    }

    #[test]
    fn word_at_position_across_multibyte_prefix() {
        // The é before the token must not shift the byte extraction.
        let src = "# é note\nslot main kind=widget";
        // line 1, character 16 is inside "widget" (all ASCII on that line).
        let w = word_at_position(src, 1, 16);
        assert_eq!(w, Some("widget"));
    }
}
