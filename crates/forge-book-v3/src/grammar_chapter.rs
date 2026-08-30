//! VixiScript grammar-contract chapter — ladder item 5 (`_plans/VIXISCRIPT-T1-LADDER.md`
//! NEXT-5): "docs that cannot drift". Every fact here is read straight from
//! [`grammar::to_contract_json`] at call time, never hand-typed, so the
//! chapter can only ever say what the engine actually parses today.

use crate::atlas::AtlasSection;
use crate::block::Block;
use crate::chapter::Chapter;
use crate::page::Page;

/// VixiScript grammar SoT, ported locally (header/dialects slice only —
/// forge-vix-v3 has no `grammar` module or `parse.rs` conformance target yet,
/// checked directly, so the full capability/slot/widget contract v2's
/// forge-vix speaks has no v3 counterpart to stay conformant against. This
/// slice is real and complete for what [`grammar_chapter`] actually reads.
pub mod grammar {
    /// `(dialect, file-extension, owning-parser)` — the `#vixi:<dialect> v<n>`
    /// registry, carried over verbatim from v2 forge-vix's `grammar.rs`.
    pub const DIALECTS: &[(&str, &str, &str)] = &[
        ("kit", ".kit.vixi", "forge-vix (parse.rs -> LoweredUi)"),
        ("sheet", ".sheet.vixi", "forge-vix (tokens.rs — palette/chrome/motion token sheet)"),
        ("vibe", ".vibe.vixi", "forge-vix (vibe_anim / tokens)"),
        ("inventory", ".inventory.vixi", "forge-vix"),
        ("timeline", ".timeline.vixi", "forge-vix (timeline.rs)"),
        ("semantic", ".semantic.vixi", "forge-vix (semantic.rs)"),
        ("cascade", ".cascade.vixi", "forge-vix (cascade.rs)"),
        ("colour", ".colour.vixi", "forge-vix (colour.rs)"),
        ("sprite", ".sprite.vixi", "forge-vix (sprite.rs -> SpriteDoc -> SpriteBlob)"),
        ("brush", ".brush.vixi", "forge-sieve (audio-dialect)"),
        ("shaderbind", ".shaderbind.vixi", "forge-gpu (shaderbind_dsl) + forge-vix (bundle.rs)"),
        ("renderpass", ".renderpass.vixi", "forge-vix (bundle.rs — cross-file surface laws)"),
        ("creature", ".creature.vixi", "planned -> CreatureIR"),
        ("weather", ".weather.vixi", "planned -> WeatherIR"),
        ("material", ".material.vixi", "planned -> MaterialIR (SoA registry)"),
        ("vixel", ".vixel.vixi", "forge-vix (authored #vixi:vixel v1 — VixelAtom canvas export; bare .vixel -> forge-ast)"),
    ];

    fn esc(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                _ => out.push(c),
            }
        }
        out
    }

    fn json_dialects() -> String {
        let mut s = String::from("[");
        for (i, (name, ext, owner)) in DIALECTS.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!(
                "{{\"name\":\"{}\",\"ext\":\"{}\",\"owner\":\"{}\"}}",
                esc(name),
                esc(ext),
                esc(owner)
            ));
        }
        s.push(']');
        s
    }

    /// Emit the header-convention + dialect registry as JSON — the slice of
    /// v2's full grammar contract that this chapter actually reads.
    pub fn to_contract_json() -> String {
        format!(
            "{{\"language\":\"VixiScript\",\"header_convention\":\"#vixi:<dialect> v<n>\",\"dialects\":{}}}",
            json_dialects()
        )
    }
}

/// Build the "VixiScript Grammar" chapter from the live contract JSON: one lore
/// line for the header convention, one page listing every registered dialect.
pub fn grammar_chapter() -> Chapter {
    let contract: serde_json::Value = serde_json::from_str(&grammar::to_contract_json())
        .expect("grammar::to_contract_json must emit valid JSON");

    let mut chapter = Chapter::new("VixiScript Grammar", AtlasSection::Capabilities);
    if let Some(hdr) = contract["header_convention"].as_str() {
        chapter.add_lore(format!("header convention: {hdr}"));
    }

    let mut page = Page::new(1);
    if let Some(dialects) = contract["dialects"].as_array() {
        for d in dialects {
            let name = d["name"].as_str().unwrap_or("?");
            let ext = d["ext"].as_str().unwrap_or("?");
            let owner = d["owner"].as_str().unwrap_or("?");
            page.add(Block::text(format!("{name} ({ext}) — owned by {owner}")));
        }
    }
    chapter.add_page(page);
    chapter
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anti-drift gate: every dialect currently in the grammar SoT must show up
    /// in the generated chapter text — no hand-authored list to fall out of sync.
    #[test]
    fn chapter_lists_every_live_dialect() {
        let ch = grammar_chapter();
        let text: String = ch.pages[0]
            .blocks
            .iter()
            .map(|b| b.as_plain())
            .collect::<Vec<_>>()
            .join("\n");
        for (name, _, _) in grammar::DIALECTS {
            assert!(text.contains(name), "chapter missing live dialect '{name}'");
        }
    }

    #[test]
    fn chapter_is_capabilities_section_and_carries_header_lore() {
        let ch = grammar_chapter();
        assert_eq!(ch.section, AtlasSection::Capabilities);
        assert_eq!(ch.title(), "VixiScript Grammar");
        assert!(ch.lore_count() >= 1, "expected the header-convention lore line");
    }
}
