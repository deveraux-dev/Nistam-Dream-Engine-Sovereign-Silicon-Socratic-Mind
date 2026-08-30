//! Renders [`crate::beats::Beat`]s into a self-contained HTML deck --
//! the "8th emit target" named in v2's `youtube-forgev1` skill doc
//! (`render.py`'s `output.html`): a reader-clocked face, DROP-LAW
//! ADVANCE, "press = step, no film clock." No render farm, no ffmpeg
//! mux, no server -- one static file, keyboard/click-navigated.
//!
//! CSS/JS structure ported verbatim from the real, already-shipped
//! `F:\NewRepo\tools\youtube-forge\youtube-projects\the-invention-machine
//! \TIM.html` (a real deck of Sean's own voice, rendered through this
//! same shape once already) -- not reinvented, per C06 revascularize.
//! Section accent colors are this module's own choice for the 3-act
//! Ki-Sho/Ten/Ketsu grammar [`crate::beats`] emits (TIM's 5-act
//! HOOK/PROBLEM/BUILD/RESULT/CTA used a 4-color palette; this reduces it
//! to 3, cyan -> pink -> green, calm -> turn -> resolution).

use crate::beats::Beat;

/// Renders `beats` into a complete HTML document titled `title`.
pub fn render(beats: &[Beat], title: &str) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"UTF-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    out.push_str(STYLE);
    out.push_str("</head>\n<body>\n");
    out.push_str(&format!(
        "<div class=\"slide-counter\" id=\"counter\">1 / {}</div>\n",
        beats.len()
    ));
    let first_pct = if beats.is_empty() { 0 } else { 100 / beats.len().max(1) };
    out.push_str(&format!(
        "<div class=\"progress\" id=\"progress\" style=\"width:{first_pct}%\"></div>\n\n"
    ));

    for (i, beat) in beats.iter().enumerate() {
        let active = if i == 0 { " active" } else { "" };
        let accent = accent_for_section(&beat.frame.section);
        let tag = format!("{} · {}", beat.frame.section.to_uppercase(), mmss(beat.start_ms));
        let duration_s = (beat.end_ms - beat.start_ms) as f64 / 1000.0;
        out.push_str(&format!(
            "  <div class=\"slide{active}\" style=\"--accent:{accent}\">\n    <div class=\"section-tag\">{}</div>\n    <div class=\"beat-text\">{}</div>\n    <div class=\"meta\">beat {} · {:.1}s</div>\n  </div>\n",
            escape_html(&tag),
            escape_html(&beat.frame.description),
            i + 1,
            duration_s
        ));
    }

    out.push_str("\n<div class=\"nav\">\n");
    out.push_str("  <button onclick=\"show(0)\">⏮</button>\n");
    out.push_str("  <button onclick=\"show(cur-1)\">←</button>\n");
    out.push_str("  <button onclick=\"show(cur+1)\">→</button>\n");
    out.push_str("  <button onclick=\"show(slides.length-1)\">⏭</button>\n");
    out.push_str("</div>\n");
    out.push_str(SCRIPT);
    out.push_str("</body>\n</html>\n");
    out
}

fn accent_for_section(section: &str) -> &'static str {
    match section {
        "Ki-Sho" => "#1AE0FF",
        "Ten" => "#FF3B6E",
        "Ketsu" => "#4DFFB0",
        _ => "#86A8D0",
    }
}

fn mmss(ms: u32) -> String {
    let total_s = ms / 1000;
    format!("{}:{:02}", total_s / 60, total_s % 60)
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

const STYLE: &str = r#"<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
  background: #050812;
  color: #E6F1FF;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  height: 100vh;
  overflow: hidden;
}
.slide {
  display: none;
  height: 100vh;
  padding: 64px 80px;
  flex-direction: column;
  justify-content: center;
  border-left: 4px solid var(--accent);
  background: linear-gradient(135deg, #050812 70%, #0B101E 100%);
  position: relative;
}
.slide.active { display: flex; }
.section-tag {
  font-size: 11px;
  letter-spacing: 4px;
  text-transform: uppercase;
  color: var(--accent);
  margin-bottom: 24px;
  opacity: 0.8;
}
.beat-text {
  font-size: clamp(22px, 3vw, 42px);
  line-height: 1.5;
  color: #E6F1FF;
  max-width: 900px;
  font-weight: 400;
}
.meta {
  position: absolute;
  bottom: 32px;
  right: 48px;
  color: #243C5E;
  font-size: 11px;
  letter-spacing: 2px;
}
.progress {
  position: fixed;
  bottom: 0;
  left: 0;
  height: 2px;
  background: var(--accent, #1AE0FF);
  transition: width 0.3s;
}
.nav {
  position: fixed;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  gap: 12px;
  opacity: 0.3;
  transition: opacity 0.2s;
}
.nav:hover { opacity: 1; }
.nav button {
  background: #0B101E;
  border: 1px solid #243C5E;
  color: #86A8D0;
  padding: 6px 16px;
  cursor: pointer;
  font-family: inherit;
  font-size: 12px;
  letter-spacing: 1px;
}
.nav button:hover { border-color: #1AE0FF; color: #1AE0FF; }
.slide-counter {
  position: fixed;
  top: 20px;
  right: 32px;
  color: #243C5E;
  font-size: 11px;
  letter-spacing: 2px;
}
</style>
"#;

const SCRIPT: &str = r#"<script>
let cur = 0;
const slides = document.querySelectorAll('.slide');
const progress = document.getElementById('progress');
const counter = document.getElementById('counter');

function show(n) {
  slides[cur].classList.remove('active');
  cur = Math.max(0, Math.min(n, slides.length - 1));
  slides[cur].classList.add('active');
  const pct = ((cur + 1) / slides.length * 100).toFixed(1);
  progress.style.width = pct + '%';
  counter.textContent = (cur + 1) + ' / ' + slides.length;
}

document.addEventListener('keydown', e => {
  if (e.key === 'ArrowRight' || e.key === ' ') show(cur + 1);
  if (e.key === 'ArrowLeft')  show(cur - 1);
  if (e.key === 'Home')       show(0);
  if (e.key === 'End')        show(slides.length - 1);
});
</script>
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::droplaw::{CohnRole, Frame, FrameType, Transition};

    fn beat(section: &str, text: &str, start_ms: u32, end_ms: u32) -> Beat {
        Beat {
            frame: Frame {
                line_num: 1,
                section: section.to_string(),
                frame_type: FrameType::Key,
                role: CohnRole::Peak,
                transition: Transition::Other("aspect_to_aspect".to_string()),
                description: text.to_string(),
                dialogue: text.to_string(),
                text: String::new(),
                duration_x10_ms: (end_ms - start_ms) * 10,
                frames: 1,
                stakes_x10: 10,
            },
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn renders_one_active_slide_with_escaped_text() {
        let beats = vec![beat("Ki-Sho", "a <script> & you", 0, 9_000)];
        let html = render(&beats, "Test");
        assert!(html.contains("<title>Test</title>"));
        assert!(html.contains("class=\"slide active\""));
        assert!(html.contains("a &lt;script&gt; &amp; you"));
        assert!(html.contains("KI-SHO · 0:00"));
        assert!(html.contains("1 / 1"));
    }

    #[test]
    fn only_the_first_slide_is_active() {
        let beats = vec![
            beat("Ki-Sho", "first", 0, 9_000),
            beat("Ten", "second", 9_000, 20_000),
        ];
        let html = render(&beats, "Test");
        assert_eq!(html.matches("class=\"slide active\"").count(), 1);
        assert!(html.contains("TEN · 0:09"));
    }
}
