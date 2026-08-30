//! Markdown import — turn a `.md` document into chapters/blocks. Author fast:
//! `#` opens a chapter, `##` subheads (shout), `>` quotes (whisper), `---` rules.

use crate::atlas::AtlasSection;
use crate::block::{Block, Emphasis, TextBlock};
use crate::chapter::Chapter;
use crate::page::Page;

/// Parse markdown into chapters. Each `# Heading` starts a chapter; content
/// before the first heading lands in an "Untitled" chapter. One page per chapter.
pub fn import_md(md: &str, section: AtlasSection) -> Vec<Chapter> {
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut cur: Option<Chapter> = None;
    let mut page = Page::new(1u32);
    let mut para = String::new();

    let flush_para = |page: &mut Page, para: &mut String| {
        let t = para.trim();
        if !t.is_empty() {
            page.add(Block::text(t.to_string()));
        }
        para.clear();
    };

    for line in md.lines() {
        let l = line.trim_end();
        if let Some(title) = l.strip_prefix("# ") {
            flush_para(&mut page, &mut para);
            let finished = std::mem::replace(&mut page, Page::new(1u32));
            match cur.take() {
                Some(mut ch) => {
                    if !finished.is_empty() {
                        ch.add_page(finished);
                    }
                    chapters.push(ch);
                }
                None => {
                    if !finished.is_empty() {
                        let mut ch = Chapter::new("Untitled", section.clone());
                        ch.add_page(finished);
                        chapters.push(ch);
                    }
                }
            }
            cur = Some(Chapter::new(title.trim(), section.clone()));
        } else if let Some(sub) = l.strip_prefix("## ") {
            flush_para(&mut page, &mut para);
            page.add(Block::Text(TextBlock::new(sub.trim()).emphasize(Emphasis::Shout)));
        } else if let Some(q) = l.strip_prefix("> ") {
            flush_para(&mut page, &mut para);
            page.add(Block::Text(TextBlock::new(q.trim()).emphasize(Emphasis::Whisper)));
        } else if l.trim() == "---" {
            flush_para(&mut page, &mut para);
            page.add(Block::Divider);
        } else if l.trim().is_empty() {
            flush_para(&mut page, &mut para);
        } else {
            if !para.is_empty() {
                para.push(' ');
            }
            para.push_str(l.trim());
        }
    }

    flush_para(&mut page, &mut para);
    match cur.take() {
        Some(mut ch) => {
            if !page.is_empty() {
                ch.add_page(page);
            }
            chapters.push(ch);
        }
        None => {
            if !page.is_empty() {
                let mut ch = Chapter::new("Untitled", section);
                ch.add_page(page);
                chapters.push(ch);
            }
        }
    }
    chapters
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "\
# The Belt
The 6-in-1: one body, six ground edges.

## Faces
> Prep beats product.

Set nails and scrape.
---
Open cans and spread compound.

# Skies
The sky remembers each age.";

    #[test]
    fn splits_on_h1() {
        let chs = import_md(DOC, AtlasSection::Items);
        assert_eq!(chs.len(), 2);
        assert_eq!(chs[0].title(), "The Belt");
        assert_eq!(chs[1].title(), "Skies");
    }

    #[test]
    fn blocks_carry_structure() {
        let chs = import_md(DOC, AtlasSection::Items);
        let page = &chs[0].pages[0];
        // has a divider and multiple text blocks
        assert!(page.blocks.iter().any(|b| matches!(b, Block::Divider)));
        assert!(page.len() >= 4);
    }

    #[test]
    fn preamble_becomes_untitled() {
        let chs = import_md("loose intro line\n\n# Real\nbody", AtlasSection::Learning);
        assert_eq!(chs.len(), 2);
        assert_eq!(chs[0].title(), "Untitled");
        assert_eq!(chs[1].title(), "Real");
    }

    #[test]
    fn empty_input_no_chapters() {
        assert!(import_md("", AtlasSection::Appendix).is_empty());
    }
}
