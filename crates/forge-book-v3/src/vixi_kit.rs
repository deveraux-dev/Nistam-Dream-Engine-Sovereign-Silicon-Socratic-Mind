//! VixiScript book dialect — author a book in a line-based `#vixi:book` format,
//! parse it to a Book, and generate it back. Round-trips title/chapters/lore/gates.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::book::Book;
use crate::chapter::{Chapter, Visibility};
use crate::page::Page;

/// Map a section keyword to a section (unknown words become Custom).
fn section_of(word: &str) -> AtlasSection {
    match word {
        "Items" => AtlasSection::Items,
        "Weather" => AtlasSection::Weather,
        "Learning" => AtlasSection::Learning,
        "Appendix" => AtlasSection::Appendix,
        "Shaders" => AtlasSection::Shaders,
        "Poetry" => AtlasSection::Poetry,
        "Dialogue" => AtlasSection::Dialogue,
        "Capabilities" => AtlasSection::Capabilities,
        other => AtlasSection::Custom(other.to_string()),
    }
}

/// The section's one-word keyword (Custom keeps its name; must be single-word).
fn section_word(s: &AtlasSection) -> String {
    match s {
        AtlasSection::Custom(c) => c.clone(),
        other => other.title(),
    }
}

/// Escape a value onto one physical line (mirrors forge-vix cascade.rs's
/// `toml_escape`): a bare newline would otherwise surface as a new top-level
/// line and misparse as an "unknown keyword" (e.g. a nested `#vixi:shaderbind`
/// block). `lore`/`text` values may carry such multi-line child documents, so
/// they round-trip through this escape.
fn escape_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(c),
        }
    }
    out
}

/// Inverse of [`escape_line`].
fn unescape_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse a `#vixi:book` document into a Book.
pub fn parse_book(src: &str) -> Result<Book, String> {
    let mut title = String::from("Untitled");
    let mut author = String::from("unknown");
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut page: Option<Page> = None;

    for (i, raw) in src.lines().enumerate() {
        let n = i + 1;
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (kw, rest) = match line.split_once(char::is_whitespace) {
            Some((k, r)) => (k, r.trim()),
            None => (line, ""),
        };
        match kw {
            "title:" | "title" => title = rest.to_string(),
            "author:" | "author" => author = rest.to_string(),
            "chapter" => {
                if let (Some(p), Some(ch)) = (page.take(), chapters.last_mut()) {
                    if !p.is_empty() {
                        ch.add_page(p);
                    }
                }
                let (sec, t) = rest
                    .split_once(char::is_whitespace)
                    .ok_or_else(|| format!("line {n}: chapter needs a section and a title"))?;
                let title_txt = t.trim().trim_matches('"');
                chapters.push(Chapter::new(title_txt, section_of(sec.trim())));
                page = None;
            }
            "page" => {
                let current_chapter = chapters
                    .last_mut()
                    .ok_or_else(|| format!("line {n}: page before any chapter"))?;
                if let Some(p) = page.take() {
                    if !p.is_empty() {
                        current_chapter.add_page(p);
                    }
                }
                let page_num = current_chapter.page_count() as u32 + 1;
                page = Some(Page::new(page_num));
            }
            "lore" => {
                chapters
                    .last_mut()
                    .ok_or_else(|| format!("line {n}: lore before any chapter"))?
                    .add_lore(unescape_line(rest));
            }
            "text" => {
                let current_chapter = chapters
                    .last_mut()
                    .ok_or_else(|| format!("line {n}: text before any chapter"))?;

                if page.is_none() {
                    let page_num = current_chapter.page_count() as u32 + 1;
                    page = Some(Page::new(page_num));
                }

                page.as_mut().unwrap().add(Block::text(unescape_line(rest)));
            }
            "gate" => {
                let tag: u64 = rest.parse().map_err(|_| format!("line {n}: gate needs a number"))?;
                chapters
                    .last_mut()
                    .ok_or_else(|| format!("line {n}: gate before any chapter"))?
                    .gate_behind(tag);
            }
            other => return Err(format!("line {n}: unknown keyword '{other}'")),
        }
    }
    if let (Some(p), Some(ch)) = (page.take(), chapters.last_mut()) {
        if !p.is_empty() {
            ch.add_page(p);
        }
    }

    let mut book = Book::new(title, author);
    for ch in chapters {
        book.add_chapter(ch);
    }
    Ok(book)
}

/// Generate the `#vixi:book` source for a book.
pub fn to_vixi(book: &Book) -> String {
    let mut s = String::from("#vixi:book v1\n");
    s.push_str(&format!("title: {}\n", book.title));
    s.push_str(&format!("author: {}\n", book.author));
    for ch in &book.spine.chapters {
        s.push_str(&format!("chapter {} \"{}\"\n", section_word(&ch.section), ch.title()));
        if matches!(ch.visibility, Visibility::Hidden) {
            for tag in &ch.codex.unlock_sieve_tags {
                s.push_str(&format!("  gate {tag}\n"));
            }
        }
        for slot in &ch.codex.slots {
            s.push_str(&format!("  lore {}\n", escape_line(&slot.text)));
        }
        for p in &ch.pages {
            if p.is_empty() {
                continue;
            }
            s.push_str("  page\n");
            for b in &p.blocks {
                if let Block::Text(t) = b {
                    s.push_str(&format!("    text {}\n", escape_line(&t.text)));
                }
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "\
#vixi:book v1
title: The Opus
author: deveraux
chapter Items \"The Belt\"
  lore one body six edges
  text scrape and set
chapter Appendix \"Rites\"
  gate 777
  lore the sealed word";

    #[test]
    fn parses_metadata_and_chapters() {
        let b = parse_book(SRC).expect("parse");
        assert_eq!(b.title, "The Opus");
        assert_eq!(b.author, "deveraux");
        assert_eq!(b.chapter_count(), 2);
        assert_eq!(b.chapter(0).unwrap().title(), "The Belt");
        // the gated appendix is hidden
        assert_eq!(b.visible_chapters().len(), 1);
    }

    #[test]
    fn round_trips() {
        let b = parse_book(SRC).unwrap();
        let src2 = to_vixi(&b);
        let b2 = parse_book(&src2).unwrap();
        assert_eq!(b2.title, b.title);
        assert_eq!(b2.chapter_count(), b.chapter_count());
        assert_eq!(b2.visible_chapters().len(), b.visible_chapters().len());
    }

    #[test]
    fn unknown_keyword_errors() {
        assert!(parse_book("frobnicate whatever").is_err());
    }

    #[test]
    fn lore_with_embedded_surface_line_round_trips() {
        // Regression: a nested `#vixi:shaderbind` block (its own `surface:` line)
        // stuffed whole into one lore slot must not surface as a bare top-level
        // line the book-dialect parser rejects as "unknown keyword 'surface:'".
        let nested = "#vixi:shaderbind v1\nsurface: deveraux_radio\nprofile: seehear\n";
        let mut ch = Chapter::new("Kernels", AtlasSection::Shaders);
        ch.add_lore(nested);
        let mut b = Book::new("T", "a");
        b.add_chapter(ch);
        let src = to_vixi(&b);
        assert!(!src.lines().any(|l| l.trim() == "surface: deveraux_radio"));
        let b2 = parse_book(&src).expect("embedded surface: line must not break the parser");
        assert_eq!(b2.chapter(0).unwrap().codex.slots[0].text, nested);
    }

    #[test]
    fn lore_before_chapter_errors() {
        assert!(parse_book("lore orphaned").is_err());
    }
}
