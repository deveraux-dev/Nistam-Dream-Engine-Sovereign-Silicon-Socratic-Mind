//! EVOKE faces — a declared shape, made readable (Sean 2026-07-28: "html? md?
//! thats it"). No ts, no proto: this engine has one binary and one language, so
//! emitting client code would emit it for nobody. The only consumer that exists
//! is a person reading, so the faces are the two a person reads.
//!
//! Both carry the same spoken seal in a comment, so a face that has gone stale
//! against its shape is caught by [`crate::evoke::read_mark`] — the page cannot
//! quietly disagree with the code it describes.

use crate::evoke::{evoke, Seed, MARK_PATH};

/// Escape the five characters that would otherwise close a tag or an entity.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A shape as markdown: mark, name, seal, field table. Reads plain in a diff.
pub fn face_md(seed: &Seed) -> String {
    let echo = evoke(seed);
    let mut s = format!("<!-- {} -->\n<!-- {MARK_PATH} -->\n\n", echo.spoken_line());
    s.push_str(&format!("# {}\n\n", seed.name));
    s.push_str(&format!(
        "**{}** — *{}* — {} trits declared\n\n",
        echo.syllabics(),
        echo.roman(),
        echo.trits,
    ));
    s.push_str("| field | kind | trits |\n| --- | --- | --- |\n");
    for f in seed.fields {
        s.push_str(&format!("| {} | {} | {} |\n", f.name, f.kind, f.trits));
    }
    s
}

/// One preattentive element (the seal), one group of fields, one footer — the
/// aperture law's 4±1, so a shape reads at a glance instead of being scanned.
const CSS: &str = r#"
:root{--bg:#0e0f13;--card:#171922;--text:#e8e9ee;--dim:#8b90a3;--seal:#d8b26a;--line:#252836}
*{box-sizing:border-box}
body{margin:0;padding:32px 20px;background:var(--bg);color:var(--text);
  font:15px/1.55 ui-sans-serif,system-ui,Segoe UI,sans-serif}
main{max-width:720px;margin:0 auto}
h1{font-size:20px;font-weight:600;margin:0 0 4px}
h1 small{display:block;font-size:12px;font-weight:400;color:var(--dim);margin-top:4px}
.shape{background:var(--card);border:1px solid var(--line);border-radius:10px;
  padding:20px;margin:20px 0}
.seal{font-size:38px;line-height:1.25;color:var(--seal);letter-spacing:.06em;
  word-break:break-word}
.roman{font-size:13px;color:var(--dim);margin-top:2px;letter-spacing:.12em}
table{width:100%;border-collapse:collapse;margin-top:16px;font-size:14px}
th{text-align:left;font-weight:500;color:var(--dim);font-size:11px;
  text-transform:uppercase;letter-spacing:.08em;padding:0 8px 6px 0}
td{padding:6px 8px 6px 0;border-top:1px solid var(--line)}
td.n{color:var(--dim);text-align:right;font-variant-numeric:tabular-nums}
.foot{margin-top:14px;font-size:12px;color:var(--dim)}
"#;

/// Shapes as one page, shaped: seal first, fields under it, width last.
pub fn face_html(seeds: &[Seed]) -> String {
    let mut s = String::from(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n\
         <title>13Forge — Shapes</title>\n<style>",
    );
    s.push_str(CSS);
    s.push_str("</style>\n</head>\n<body>\n<main>\n");
    s.push_str("<h1>Shapes<small>what is declared, and what it is called out loud</small></h1>\n");

    for seed in seeds {
        let echo = evoke(seed);
        s.push_str(&format!("<!-- {} -->\n", esc(&echo.spoken_line())));
        s.push_str("<section class=\"shape\">\n");
        s.push_str(&format!("<h2>{}</h2>\n", esc(seed.name)));
        s.push_str(&format!("<div class=\"seal\">{}</div>\n", esc(&echo.syllabics())));
        s.push_str(&format!("<div class=\"roman\">{}</div>\n", esc(&echo.roman())));
        s.push_str("<table>\n<tr><th>field</th><th>kind</th><th class=\"n\">trits</th></tr>\n");
        for f in seed.fields {
            s.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td class=\"n\">{}</td></tr>\n",
                esc(f.name),
                esc(f.kind),
                f.trits,
            ));
        }
        s.push_str("</table>\n");
        s.push_str(&format!(
            "<div class=\"foot\">{} trits declared · {MARK_PATH}</div>\n",
            echo.trits,
        ));
        s.push_str("</section>\n");
    }

    s.push_str("</main>\n</body>\n</html>\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assay::SHEET_SEED;
    use crate::evoke::{read_mark, Field};

    const ODD: [Field; 2] = [
        Field::new("width<px>", "u32 & i32", 20),
        Field::new("tail", "trit", 3),
    ];
    const ODD_SEED: Seed = Seed::new("Odd \"Shape\"", &ODD);

    #[test]
    fn the_markdown_face_carries_the_seal_back_to_the_id() {
        let md = face_md(&SHEET_SEED);
        assert_eq!(read_mark(&md), Some(evoke(&SHEET_SEED).id));
        assert!(md.contains("# AssaySheet"), "{md}");
        assert!(md.contains("| verdicts | trit | 20 |"), "{md}");
        assert!(md.contains(MARK_PATH), "{md}");
    }

    #[test]
    fn the_html_face_is_well_formed_and_reads_back() {
        let html = face_html(&[SHEET_SEED]);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.trim_end().ends_with("</html>"));
        assert_eq!(html.matches("<section").count(), html.matches("</section>").count());
        assert_eq!(read_mark(&html), Some(evoke(&SHEET_SEED).id));
        assert!(html.contains(&evoke(&SHEET_SEED).syllabics()), "seal must show");
    }

    #[test]
    fn hostile_names_cannot_break_out_of_the_page() {
        let html = face_html(&[ODD_SEED]);
        assert!(html.contains("Odd &quot;Shape&quot;"), "{html}");
        assert!(html.contains("width&lt;px&gt;"), "{html}");
        assert!(html.contains("u32 &amp; i32"), "{html}");
        assert!(!html.contains("<px>"), "raw angle bracket escaped into the page");
    }

    #[test]
    fn a_page_holds_every_shape_it_is_given() {
        let html = face_html(&[SHEET_SEED, ODD_SEED]);
        assert_eq!(html.matches("<section class=\"shape\">").count(), 2);
        assert!(html.contains("AssaySheet") && html.contains("Odd &quot;Shape&quot;"));
        assert_eq!(face_html(&[]).matches("<section").count(), 0);
    }

    #[test]
    fn a_reshaped_seed_moves_both_faces() {
        const GROWN: [Field; 1] = [Field::new("verdicts", "trit", 21)];
        let grown = Seed::new("AssaySheet", &GROWN);
        assert_ne!(read_mark(&face_md(&grown)), read_mark(&face_md(&SHEET_SEED)));
        assert_ne!(
            read_mark(&face_html(&[grown])),
            read_mark(&face_html(&[SHEET_SEED]))
        );
    }
}
