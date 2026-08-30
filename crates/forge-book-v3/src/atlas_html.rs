//! Atlas-html — render the capabilities index as a standalone brag page. "This
//! is what I can do", with proof badges, as one self-contained HTML file.

use crate::atlas::{AtlasSection, CapabilityStatus};
use crate::book::Book;

/// How one reader wants the Atlas to read.
///
/// The index is the same set of proven rows for everyone; the DOCUMENT is not.
/// A skin picks which sections appear and in what order (omission = drop), what
/// each section is CALLED (the word swap — "Runbook" is someone else's
/// "Operations"), whether receipts are shown, which proof tiers are admitted,
/// and the palette. Nothing here can invent a row or change a receipt: a skin
/// is a view, and the proof stays the book's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtlasSkin {
    /// Page title and `<h1>`. `None` keeps the book's own title + house tagline.
    pub title: Option<String>,
    /// Sections to render, in order. Empty = one flat list, the original shape.
    /// A section left out of a non-empty list is DROPPED from the document.
    pub sections: Vec<AtlasSection>,
    /// Section renames, applied at render: `(section, what to call it)`.
    pub rename: Vec<(AtlasSection, String)>,
    /// Show the receipt column. Off makes a reader's page, on makes an auditor's.
    pub show_receipts: bool,
    /// Proof tiers admitted. Empty = all of them.
    pub statuses: Vec<CapabilityStatus>,
    /// `(background, ink, accent)` as CSS colours.
    pub palette: (String, String, String),
}

impl Default for AtlasSkin {
    /// The house look: flat list, receipts on, every tier, forge palette — byte
    /// -identical to what [`brag_page`] emitted before skins existed.
    fn default() -> Self {
        Self {
            title: None,
            sections: Vec::new(),
            rename: Vec::new(),
            show_receipts: true,
            statuses: Vec::new(),
            palette: ("#0c0a08".into(), "#e0d4ba".into(), "#c8a040".into()),
        }
    }
}

impl AtlasSkin {
    /// What this skin calls `section` — the rename if one was given, else the
    /// section's own title.
    pub fn label(&self, section: &AtlasSection) -> String {
        self.rename
            .iter()
            .find(|(s, _)| s == section)
            .map(|(_, word)| word.clone())
            .unwrap_or_else(|| section.title())
    }

    /// True when a row of this status is admitted (empty filter admits all).
    pub fn admits(&self, status: CapabilityStatus) -> bool {
        self.statuses.is_empty() || self.statuses.contains(&status)
    }
}

/// A standalone HTML brag page for `book` — the house skin.
pub fn brag_page(book: &Book) -> String {
    skinned_page(book, &AtlasSkin::default())
}

/// Render `book`'s Atlas through `skin`.
///
/// Sectioned when the skin names sections, flat when it does not. Section
/// headings carry the section slug as an anchor id so the page is linkable.
pub fn skinned_page(book: &Book, skin: &AtlasSkin) -> String {
    let (bg, ink, accent) = (&skin.palette.0, &skin.palette.1, &skin.palette.2);
    // The tab title and the on-page heading are different strings by house
    // default — "What I can do" over "<book> — this is what I can do" — and a
    // skin's own title replaces both.
    let tab = skin.title.clone().unwrap_or_else(|| "What I can do".into()); // @forge:allow_alloc cold render path
    let h1 = match &skin.title {
        Some(t) => esc(t),
        None => format!("{} &mdash; this is what I can do", esc(&book.title)), // @forge:allow_alloc cold render path
    };

    // HTML5 SHELL FOLD (Sean 2026-08-02): skeleton + esc live in
    // forge_vix::emit_html::page — this face builds css + body only.
    let mut css = String::with_capacity(1024); // @forge:allow_alloc cold render path, one page
    css.push_str(&format!(
        "body{{background:{bg};color:{ink};font-family:'Courier New',monospace;padding:40px;max-width:760px;margin:0 auto}}\nh1{{color:{accent};letter-spacing:2px;font-weight:300}}\n"
    ));
    // The section-heading rule only exists on a sectioned page, so a flat render
    // stays byte-identical to the page that shipped before skins.
    if !skin.sections.is_empty() {
        css.push_str(&format!(
            "h2{{color:{accent};letter-spacing:1px;font-weight:300;font-size:15px;margin-top:28px;opacity:.85}}\n"
        ));
    }
    css.push_str(&format!(
        "ul{{list-style:none}}\nli{{padding:6px 0;border-bottom:1px solid rgba(200,160,64,.08)}}\n.badge{{font-size:11px;letter-spacing:1px}}\n.proven .badge{{color:#00a08e}}\n.wired .badge{{color:{accent}}}\n.planned .badge{{color:#8a7030}}\n.study .badge{{color:#706458}}\n.receipt{{color:#706458;font-size:12px}}\n"
    ));
    let mut s = String::with_capacity(2048); // @forge:allow_alloc cold render path, one page
    s.push_str(&format!("<h1>{h1}</h1>\n"));

    if skin.sections.is_empty() {
        s.push_str("<ul>\n");
        for cap in book.capabilities.iter().filter(|c| skin.admits(c.status)) {
            push_row(&mut s, cap, skin);
        }
        s.push_str("</ul>\n");
    } else {
        for section in &skin.sections {
            let rows: Vec<_> = book // @forge:allow_alloc cold render path
                .capabilities
                .iter()
                .filter(|c| &c.section == section && skin.admits(c.status))
                .collect();
            if rows.is_empty() {
                continue;
            }
            s.push_str(&format!(
                "<h2 id=\"{}\">{}</h2>\n<ul>\n",
                esc(&section.slug()),
                esc(&skin.label(section))
            ));
            for cap in rows {
                push_row(&mut s, cap, skin);
            }
            s.push_str("</ul>\n");
        }
    }

    forge_vix_v3::emit_html::page(&tab, &css, &s)
}

/// One capability row, badge first, receipt only if the skin asked for it.
fn push_row(s: &mut String, cap: &crate::atlas::CapabilityEntry, skin: &AtlasSkin) {
    let cls = match cap.status {
        CapabilityStatus::Proven => "proven",
        CapabilityStatus::Wired => "wired",
        CapabilityStatus::Planned => "planned",
        CapabilityStatus::Study => "study",
    };
    s.push_str(&format!(
        "<li class=\"{}\"><span class=\"badge\">{}</span> <strong>{}</strong>",
        cls,
        cap.status.badge(),
        esc(&cap.name)
    ));
    if skin.show_receipts {
        s.push_str(&format!(" <span class=\"receipt\">{}</span>", esc(&cap.receipt)));
    }
    s.push_str("</li>\n");
}

/// The pre-skin renderer, kept as the parity oracle for [`AtlasSkin::default`].
/// Rides the same [`forge_vix_v3::emit_html::page`] shell — the oracle's subject is
/// skin NEUTRALITY (default skin adds nothing), never the shell's shape.
#[cfg(test)]
fn brag_page_legacy(book: &Book) -> String {
    let css = "body{background:#0c0a08;color:#e0d4ba;font-family:'Courier New',monospace;padding:40px;max-width:760px;margin:0 auto}\nh1{color:#c8a040;letter-spacing:2px;font-weight:300}\nul{list-style:none}\nli{padding:6px 0;border-bottom:1px solid rgba(200,160,64,.08)}\n.badge{font-size:11px;letter-spacing:1px}\n.proven .badge{color:#00a08e}\n.wired .badge{color:#c8a040}\n.planned .badge{color:#8a7030}\n.study .badge{color:#706458}\n.receipt{color:#706458;font-size:12px}\n";
    let mut s = String::new();
    s.push_str(&format!("<h1>{} &mdash; this is what I can do</h1>\n<ul>\n", esc(&book.title)));
    for cap in &book.capabilities {
        let cls = match cap.status {
            CapabilityStatus::Proven => "proven",
            CapabilityStatus::Wired => "wired",
            CapabilityStatus::Planned => "planned",
            CapabilityStatus::Study => "study",
        };
        s.push_str(&format!(
            "<li class=\"{}\"><span class=\"badge\">{}</span> <strong>{}</strong> <span class=\"receipt\">{}</span></li>\n",
            cls,
            cap.status.badge(),
            esc(&cap.name),
            esc(&cap.receipt)
        ));
    }
    s.push_str("</ul>\n");
    forge_vix_v3::emit_html::page("What I can do", css, &s)
}

use forge_vix_v3::emit_html::esc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::full_atlas;

    #[test]
    fn brag_lists_capabilities() {
        let b = full_atlas("The Opus", "deveraux");
        let html = brag_page(&b);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("this is what I can do"));
        assert!(html.contains("[PROVEN]"));
        assert!(html.trim_end().ends_with("</html>"));
    }

    /// PARARITY (Sean 08-02): two render lanes, one document. The skinned path
    /// under the house skin must emit what the pre-skin renderer emitted — a
    /// customizable Atlas that quietly changed the default page would be a fork,
    /// not a skin.
    #[test]
    fn the_house_skin_is_the_page_that_shipped_before_skins() {
        let b = full_atlas("The Opus", "deveraux");
        assert_eq!(
            brag_page_legacy(&b),
            skinned_page(&b, &AtlasSkin::default()),
            "the default skin drifted from the original brag page"
        );
    }

    /// The word swap: a reader may rename a section without touching a row.
    #[test]
    fn a_reader_can_rename_a_section_without_touching_a_receipt() {
        let b = full_atlas("The Opus", "deveraux");
        let skin = AtlasSkin {
            sections: vec![AtlasSection::Runbook, AtlasSection::Capabilities],
            rename: vec![(AtlasSection::Runbook, "Operations".into())],
            ..Default::default()
        };
        let html = skinned_page(&b, &skin);
        assert!(html.contains(">Operations<"), "the swapped word must reach the page");
        assert!(!html.contains(">Runbook<"), "the old word must not survive the swap");
        // The anchor stays the SLUG, so links written against the Atlas keep working.
        assert!(html.contains("id=\"runbook\""), "a rename must not break the anchor");
    }

    /// Drop: a section left out of a non-empty list does not render at all.
    #[test]
    fn a_section_left_out_is_dropped_from_the_document() {
        let b = full_atlas("The Opus", "deveraux");
        let only_runbook = skinned_page(
            &b,
            &AtlasSkin { sections: vec![AtlasSection::Runbook], ..Default::default() },
        );
        let with_caps = skinned_page(
            &b,
            &AtlasSkin {
                sections: vec![AtlasSection::Runbook, AtlasSection::Capabilities],
                ..Default::default()
            },
        );
        assert!(with_caps.len() > only_runbook.len(), "dropping a section must shrink the page");
        assert!(!only_runbook.contains("id=\"capabilities\""));
    }

    /// A skin is a VIEW: it can hide a receipt, never rewrite one, and it can
    /// never conjure a row the book does not carry.
    #[test]
    fn a_skin_can_hide_a_receipt_but_never_invent_a_row() {
        let b = full_atlas("The Opus", "deveraux");
        let bare = AtlasSkin { show_receipts: false, ..Default::default() };
        let html = skinned_page(&b, &bare);
        assert!(!html.contains("class=\"receipt\""), "receipts were asked to be hidden");
        let rows = html.matches("<li class=").count();
        assert_eq!(rows, b.capabilities.len(), "a skin must not add or drop rows silently");
    }

    /// The proof filter: a page for buyers shows only what is PROVEN.
    #[test]
    fn a_proven_only_skin_admits_no_lesser_tier() {
        let b = full_atlas("The Opus", "deveraux");
        let skin = AtlasSkin { statuses: vec![CapabilityStatus::Proven], ..Default::default() };
        let html = skinned_page(&b, &skin);
        for weaker in ["[WIRED]", "[PLANNED]", "[STUDY]"] {
            assert!(!html.contains(weaker), "a proven-only Atlas leaked {weaker}");
        }
        assert!(html.contains("[PROVEN]"));
    }

    #[test]
    fn a_custom_palette_reaches_the_stylesheet() {
        let b = full_atlas("The Opus", "deveraux");
        let skin = AtlasSkin {
            title: Some("The Ironroot Compendium".into()),
            palette: ("#101820".into(), "#f2f2f2".into(), "#7cc4ff".into()),
            ..Default::default()
        };
        let html = skinned_page(&b, &skin);
        assert!(html.contains("background:#101820"));
        assert!(html.contains("color:#7cc4ff"));
        assert!(html.contains("<title>The Ironroot Compendium</title>"));
    }
}
