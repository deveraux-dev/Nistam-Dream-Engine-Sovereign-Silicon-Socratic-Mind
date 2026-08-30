//! Export a Book to a standalone folding-grimoire HTML page — the Opus lineage.
//! Pure deterministic string build (no clock, no deps); author -> HTML/site fast.

use crate::block::{Block, Emphasis};
use crate::book::Book;
use crate::chapter::{Chapter, Visibility};
use crate::ink::{Ink, InkId};
use forge_vix_v3::emit_html::{esc, page};

/// Full standalone HTML document for `book` — cover, chapters, brag, fold JS.
pub fn export_book(book: &Book) -> String {
    export_book_themed(book, None)
}

/// [`export_book`] plus an optional `:root{--name:hex;...}` CSS custom-property
/// block from `(name, hex)` tokens (e.g. a resolved `.sheet.vixi` poet-word
/// sheet) — `None` emits byte-identical output to [`export_book`]. The page
/// CSS reads the token via `var(--wall, <literal fallback>)`, so an absent
/// token silently resolves to the original look.
pub fn export_book_themed(book: &Book, tokens: Option<&[(&str, &str)]>) -> String {
    // HTML5 SHELL FOLD (Sean 2026-08-02): skeleton + esc live in the
    // compiler-final, forge_vix::emit_html — this face builds css + body only.
    let mut css = String::with_capacity(8 * 1024);
    css.push_str(CSS);
    if let Some(pairs) = tokens {
        css.push_str("\n:root{");
        for (name, hex) in pairs {
            css.push_str(&format!("--{}:{};", esc(name), esc(hex)));
        }
        css.push_str("}\n");
    }
    let mut s = String::with_capacity(16 * 1024);

    s.push_str(&cover_html(book));
    s.push_str("<main class=\"book-body\" id=\"book-body\">\n");
    s.push_str(&toc_html(book));
    for (i, ch) in book.spine.chapters.iter().enumerate() {
        s.push_str(&chapter_html(book, ch, i));
    }
    s.push_str(&capabilities_html(book));
    s.push_str("</main>\n");
    s.push_str("<nav class=\"turn\" id=\"turn\">");
    s.push_str("<button id=\"prev\">&lsaquo; prev</button>");
    s.push_str("<span id=\"folio\"></span>");
    s.push_str("<button id=\"next\">next &rsaquo;</button></nav>\n");

    s.push_str("<script>\n");
    s.push_str(JS);
    s.push_str("\n</script>\n");
    page(&format!("{} — {}", book.title, book.author), &css, &s)
}

/// The ink's `#rrggbb`, for inline stroke colour.
fn ink_hex(ink: InkId) -> String {
    Ink::of(ink).hex()
}

/// CSS emphasis class for a verse block.
fn emphasis_class(e: Emphasis) -> &'static str {
    match e {
        Emphasis::Plain => "plain",
        Emphasis::Whisper => "whisper",
        Emphasis::Shout => "shout",
        Emphasis::Chant => "chant",
    }
}

/// A `[BADGE] rest` line renders as a `.cap` badge-chip row — reuses the SAME
/// CSS the capabilities index already ships (state_board.rs, no new classes).
/// Any other text falls through to the normal verse paragraph.
fn cap_row_html(text: &str) -> Option<String> {
    let (badge, cls) = [("[PROVEN]", "proven"), ("[WIRED]", "wired"), ("[PLANNED]", "planned"), ("[STUDY]", "study")]
        .into_iter()
        .find(|(b, _)| text.starts_with(b))?;
    let rest = text[badge.len()..].trim();
    Some(format!(
        "<div class=\"cap {cls}\"><span class=\"badge\">{badge}</span> <strong>{}</strong></div>\n",
        esc(rest)
    ))
}

/// One content block rendered to HTML.
fn block_html(b: &Block, book: &Book) -> String {
    match b {
        Block::Text(t) => cap_row_html(&t.text).unwrap_or_else(|| format!(
            "<p class=\"verse {}\" style=\"color:{}\">{}</p>\n",
            emphasis_class(t.emphasis),
            ink_hex(t.ink),
            esc(&t.text)
        )),
        Block::Asset(p) => match book.assets.get(p.asset_id) {
            Some(a) => format!(
                "<figure class=\"asset\" style=\"width:{}%\"><img src=\"{}\" alt=\"dropped asset\" loading=\"lazy\"></figure>\n",
                (p.w_pmy / 100).clamp(5, 100),
                esc(&a.source_path)
            ),
            None => format!("<figure class=\"asset missing\">[missing asset {:016x}]</figure>\n", p.asset_id),
        },
        Block::Divider => "<hr class=\"rule\">\n".to_string(),
        Block::Seal(sm) => format!("<div class=\"seal\">&#9670; sealed {:016x}</div>\n", sm.hash),
        Block::Embed(e) => format!("<div class=\"embed\">&#8618; {}</div>\n", esc(&e.target)),
    }
}

/// A chapter's lore slots + canvas pages, honouring visibility.
fn chapter_html(book: &Book, ch: &Chapter, idx: usize) -> String {
    let mut out = String::new();
    let sec = ch.section.title();
    out.push_str(&format!(
        "<section class=\"chapter\" id=\"ch-{}\" data-section=\"{}\">\n",
        idx,
        esc(&ch.section.slug())
    ));
    out.push_str(&format!(
        "<header class=\"chap-head\"><span class=\"sec-tag\">{}</span><h2>{}</h2></header>\n",
        esc(&sec),
        esc(ch.title())
    ));

    let locked = !matches!(ch.visibility, Visibility::Open);
    if locked {
        // Do NOT leak sealed content — render a lock placeholder only.
        out.push_str("<div class=\"locked\">&#128274; This chapter is sealed. Advance to unlock.</div>\n");
        out.push_str("</section>\n");
        return out;
    }

    // Lore slots (forge-lore text).
    for slot in &ch.codex.slots {
        out.push_str(&format!("<p class=\"lore\">{}</p>\n", esc(&slot.text)));
    }
    // Canvas pages.
    for page in &ch.pages {
        out.push_str(&format!("<article class=\"page\" data-folio=\"{}\">\n", page.number));
        for b in &page.blocks {
            out.push_str(&block_html(b, book));
        }
        out.push_str("</article>\n");
    }
    out.push_str("</section>\n");
    out
}

/// Table of contents — visible chapters, in reading order.
fn toc_html(book: &Book) -> String {
    let mut out = String::from("<nav class=\"toc\"><h3>Atlas</h3><ol>\n");
    for (i, ch) in book.spine.chapters.iter().enumerate() {
        let open = matches!(ch.visibility, Visibility::Open);
        let lock = if open { "" } else { " &#128274;" };
        out.push_str(&format!(
            "<li><a href=\"#ch-{}\">{}{}</a> <em>{}</em></li>\n",
            i,
            esc(ch.title()),
            lock,
            esc(&ch.section.title())
        ));
    }
    out.push_str("<li><a href=\"#ch-capabilities\">Capabilities</a> <em>the brag</em></li>\n");
    out.push_str("</ol></nav>\n");
    out
}

/// The capabilities index — "this is what I can do", with proof badges.
fn capabilities_html(book: &Book) -> String {
    let mut out = String::from(
        "<section class=\"chapter caps\" id=\"ch-capabilities\"><header class=\"chap-head\"><span class=\"sec-tag\">Capabilities</span><h2>This is what I can do</h2></header>\n<ul class=\"cap-index\">\n",
    );
    for cap in &book.capabilities {
        let cls = match cap.status {
            crate::atlas::CapabilityStatus::Proven => "proven",
            crate::atlas::CapabilityStatus::Wired => "wired",
            crate::atlas::CapabilityStatus::Planned => "planned",
            crate::atlas::CapabilityStatus::Study => "study",
        };
        out.push_str(&format!(
            "<li class=\"cap {}\"><span class=\"badge\">{}</span> <strong>{}</strong> <span class=\"receipt\">{}</span></li>\n",
            cls,
            cap.status.badge(),
            esc(&cap.name),
            esc(&cap.receipt)
        ));
    }
    out.push_str("</ul></section>\n");
    out
}

/// The cover leaf — title, author, seal, "touch to open".
fn cover_html(book: &Book) -> String {
    format!(
        "<div class=\"cover\" id=\"cover\">\n<div class=\"cover-leather\"></div>\n<div class=\"cover-title\"><h1>{}</h1><h2>{}</h2></div>\n<div class=\"cover-seal\">SEAL<br>OF<br>CRAFT</div>\n<div class=\"cover-hint\">Touch to Open</div>\n</div>\n",
        esc(&book.title),
        esc(&book.author)
    )
}

const CSS: &str = r#"
*,*::before,*::after{box-sizing:border-box;margin:0;padding:0}
:root{--bg:#0c0a08;--leather:#1a1410;--parchment:#1e1a12;--gold:#c8a040;--gold-dim:#8a7030;--blood:#e54b00;--spectral:#00a08e;--text:#e0d4ba;--text-dim:#706458;--spine:#121010}
body{background:var(--wall, var(--bg));color:var(--words, var(--text));font-family:'Courier New',monospace;min-height:100vh}
.cover{position:fixed;inset:0;z-index:20;display:flex;flex-direction:column;align-items:center;justify-content:center;cursor:pointer;border:2px solid var(--gold-dim);transition:transform .8s cubic-bezier(.22,1,.36,1),opacity .5s ease .3s;transform-origin:left center}
body.open .cover{transform:rotateY(-170deg);opacity:0;pointer-events:none}
.cover-leather{position:absolute;inset:0;background:linear-gradient(135deg,var(--leather),#2a2218 30%,var(--leather) 60%,#151210);opacity:.9}
.cover-title{position:relative;z-index:2;text-align:center}
.cover-title h1{font-size:42px;letter-spacing:12px;text-transform:uppercase;color:var(--gold);font-weight:300}
.cover-title h2{font-size:14px;letter-spacing:6px;color:var(--gold-dim);margin-top:8px;text-transform:uppercase}
.cover-seal{position:relative;z-index:2;margin-top:40px;width:80px;height:80px;border-radius:50%;border:2px solid var(--gold-dim);display:flex;align-items:center;justify-content:center;font-size:9px;color:var(--gold-dim);letter-spacing:2px;text-align:center}
.cover-hint{position:relative;z-index:2;margin-top:30px;font-size:11px;color:var(--text-dim);letter-spacing:4px;text-transform:uppercase;animation:pulse 2s ease-in-out infinite}
@keyframes pulse{0%,100%{opacity:.4}50%{opacity:.9}}
.book-body{max-width:760px;margin:0 auto;padding:64px 24px 120px;opacity:0;transition:opacity .5s ease .4s}
body.open .book-body{opacity:1}
.toc{border:1px solid rgba(200,160,64,.2);border-radius:4px;padding:16px 20px;margin-bottom:48px;background:var(--parchment)}
.toc h3{color:var(--gold);letter-spacing:4px;text-transform:uppercase;font-size:13px;margin-bottom:10px}
.toc ol{list-style:none;counter-reset:toc}
.toc li{counter-increment:toc;padding:3px 0;font-size:13px}
.toc li::before{content:counter(toc,decimal-leading-zero) ' ';color:var(--gold-dim)}
.toc a{color:var(--text);text-decoration:none}
.toc a:hover{color:var(--gold)}
.toc em{color:var(--text-dim);font-size:11px}
.chapter{margin:0 0 64px;scroll-margin-top:24px}
.chap-head{border-bottom:1px solid rgba(200,160,64,.2);padding-bottom:8px;margin-bottom:20px}
.sec-tag{display:inline-block;font-size:10px;letter-spacing:3px;text-transform:uppercase;color:var(--spectral)}
.chap-head h2{font-size:26px;color:var(--gold);font-weight:400;letter-spacing:2px}
.lore{margin:12px 0;line-height:1.7;color:var(--text)}
.page{border-left:2px solid var(--spine);padding-left:18px;margin:18px 0}
.verse{margin:10px 0;line-height:1.6;white-space:pre-wrap}
.verse.whisper{opacity:.6;font-style:italic}
.verse.shout{font-weight:700;letter-spacing:1px}
.verse.chant{letter-spacing:3px;text-transform:uppercase;font-size:14px}
.asset{margin:16px 0}
.asset img{width:100%;height:auto;border:1px solid rgba(200,160,64,.2);border-radius:3px;display:block}
.asset.missing{color:var(--text-dim);font-size:12px}
.rule{border:none;border-top:1px solid var(--gold-dim);margin:24px 0;opacity:.5}
.seal{color:var(--blood);letter-spacing:2px;font-size:12px;margin:12px 0}
.embed{color:var(--spectral);font-size:13px;margin:8px 0}
.locked{color:var(--text-dim);font-style:italic;padding:16px;border:1px dashed var(--gold-dim);border-radius:4px}
.caps .cap-index{list-style:none}
.cap{padding:6px 0;border-bottom:1px solid rgba(200,160,64,.08);font-size:13px}
.cap .badge{font-size:10px;letter-spacing:1px}
.cap.proven .badge{color:var(--spectral)}
.cap.wired .badge{color:var(--gold)}
.cap.planned .badge{color:var(--gold-dim)}
.cap.study .badge{color:var(--text-dim)}
.cap .receipt{color:var(--text-dim);font-size:11px}
.turn{position:fixed;bottom:0;left:0;right:0;display:flex;gap:12px;align-items:center;justify-content:center;padding:10px;background:linear-gradient(180deg,transparent,var(--bg) 60%);opacity:0;transition:opacity .4s}
body.open .turn{opacity:1}
.turn button{background:rgba(26,22,16,.9);border:1px solid var(--gold-dim);color:var(--gold);font-family:inherit;font-size:11px;letter-spacing:2px;padding:6px 14px;cursor:pointer;border-radius:3px;text-transform:uppercase}
.turn button:hover{border-color:var(--gold);color:var(--text)}
#folio{color:var(--text-dim);font-size:11px;min-width:120px;text-align:center}
"#;

const JS: &str = r#"
(function(){
  var cover=document.getElementById('cover');
  var body=document.body;
  function open(){body.classList.add('open');}
  if(cover)cover.addEventListener('click',open);
  document.addEventListener('keydown',function(e){
    if(e.key==='Escape')body.classList.remove('open');
  });
  var chapters=Array.prototype.slice.call(document.querySelectorAll('.chapter'));
  var idx=0;
  var folio=document.getElementById('folio');
  function show(i){
    if(!chapters.length)return;
    idx=Math.max(0,Math.min(chapters.length-1,i));
    chapters[idx].scrollIntoView({behavior:'smooth',block:'start'});
    if(folio)folio.textContent=(idx+1)+' / '+chapters.length;
  }
  var prev=document.getElementById('prev'),next=document.getElementById('next');
  if(prev)prev.addEventListener('click',function(){show(idx-1);});
  if(next)next.addEventListener('click',function(){show(idx+1);});
  document.addEventListener('keydown',function(e){
    if(!body.classList.contains('open'))return;
    if(e.key==='ArrowRight')show(idx+1);
    if(e.key==='ArrowLeft')show(idx-1);
  });
  if(folio&&chapters.length)folio.textContent='1 / '+chapters.length;
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::AtlasSection;

    #[test]
    fn escapes_markup() {
        assert_eq!(esc("a<b>&\"'"), "a&lt;b&gt;&amp;&quot;&#39;");
    }

    #[test]
    fn document_is_well_formed_shell() {
        let b = Book::new("T", "A");
        let html = export_book(&b);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.trim_end().ends_with("</html>"));
        assert!(html.contains("<style>") && html.contains("<script>"));
    }

    #[test]
    fn ink_hex_matches_palette() {
        assert_eq!(ink_hex(InkId::Blood), "#e54b00");
    }

    #[test]
    fn section_title_appears() {
        let mut b = Book::new("T", "A");
        b.open_chapter(AtlasSection::Shaders, "Kernels");
        let html = export_book(&b);
        assert!(html.contains("Kernels"));
        assert!(html.contains("Shaders"));
    }

    #[test]
    fn themed_none_is_byte_identical_to_untethemed() {
        let b = Book::new("T", "A");
        assert_eq!(export_book(&b), export_book_themed(&b, None));
    }

    #[test]
    fn badge_prefixed_text_renders_as_a_cap_chip_row() {
        assert_eq!(
            cap_row_html("[PROVEN] lane-B \u{2014} landed"),
            Some("<div class=\"cap proven\"><span class=\"badge\">[PROVEN]</span> <strong>lane-B \u{2014} landed</strong></div>\n".to_string())
        );
        assert!(cap_row_html("plain prose").is_none(), "non-badge text is not a cap row");
    }

    #[test]
    fn themed_some_emits_root_token_block() {
        let b = Book::new("T", "A");
        let tokens = [("wall", "#0B0B11FF"), ("words", "#EADFC8FF")];
        let html = export_book_themed(&b, Some(&tokens));
        assert!(html.contains(":root{--wall:#0B0B11FF;--words:#EADFC8FF;}"));
        assert!(html.contains("var(--wall, var(--bg))"));
    }
}
