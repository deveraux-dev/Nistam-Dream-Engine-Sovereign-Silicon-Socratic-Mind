//! Renders [`crate::karaoke::KaraokeWord`]s as a single quiet page: one
//! continuous paragraph, typewriter-revealed in exact sync with the real
//! `<audio>` -- only words already spoken exist in the DOM, future words
//! aren't there yet. "High note" words (from
//! [`crate::karaoke::emphasis_flags`]) land bigger and brighter as they
//! land, giving the eye a moving anchor without cutting to discrete
//! scenes. Section accent tints the reading rail via [`thirds`], the
//! same three-color Ki-Sho/Ten/Ketsu law [`crate::render_html`] uses --
//! computed directly from word timestamps, not routed through
//! [`crate::beats`]' paragraph merge (that merge approximates text
//! content, which this module doesn't need; it only needs three time
//! boundaries).
//!
//! The reveal container gets generous bottom clearance (`padding-bottom`
//! well past the fixed player bar's height) and every append re-scrolls
//! to the bottom of the page, not just into view of one span -- the bug
//! that clipped the final words under the audio bar was exactly this:
//! spans existed but the page never scrolled past where the bar covered
//! them.

use crate::karaoke::{emphasis_flags, KaraokeWord};

/// A time-bounded accent zone -- Ki-Sho/Ten/Ketsu by cumulative word
/// duration, 60/30/10, same split [`crate::beats::compile_beats`] uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    /// Zone start, milliseconds.
    pub start_ms: u32,
    /// Zone end, milliseconds.
    pub end_ms: u32,
    /// Accent hex color for this zone.
    pub accent: &'static str,
}

/// Splits `words`' total span into three Ki-Sho/Ten/Ketsu zones (60/30/10)
/// directly from timestamps -- no text, no merge, just the clock.
pub fn thirds(words: &[KaraokeWord]) -> Vec<Section> {
    let (Some(first), Some(last)) = (words.first(), words.last()) else {
        return Vec::new();
    };
    let start = first.start_ms;
    let end = last.end_ms;
    let total = end - start;
    let ki_sho_end = start + (total as u64 * 60 / 100) as u32;
    let ten_end = start + (total as u64 * 90 / 100) as u32;
    vec![
        Section { start_ms: start, end_ms: ki_sho_end, accent: "#1AE0FF" },
        Section { start_ms: ki_sho_end, end_ms: ten_end, accent: "#FF3B6E" },
        Section { start_ms: ten_end, end_ms: end, accent: "#4DFFB0" },
    ]
}

/// `audio_src` is a file-relative or `file://` URI to the source
/// recording -- this module never copies or embeds audio bytes.
pub fn render(words: &[KaraokeWord], sections: &[Section], audio_src: &str, title: &str) -> String {
    render_forced(words, sections, audio_src, title, &[], None, &[], None, None, &[], &[])
}

/// Same as [`render`], plus `force_emphasis` -- word indices that land the
/// "hi" treatment regardless of [`emphasis_flags`]'s duration heuristic.
/// For a deliberate authorial beat (a thesis line, a punchline) that the
/// automatic median-duration scorer can't see -- e.g. short words like
/// "one" are permanently ineligible there (`MIN_EMPHASIS_CHARS`), but a
/// caller who knows the line matters can still land it.
///
/// `bg_window` is `(start_ms, end_ms, image_src)` -- a background image
/// that fades in only while playback is inside that window, fades out
/// otherwise. Optional; `None` renders exactly as before.
///
/// `video_insets` is `&[(start_ms, end_ms, video_src, label)]` -- muted,
/// looping video clips shown in a fixed centered panel only while playback
/// is inside their window (multiple, non-overlapping windows may each carry
/// their own clip). `label` renders as a caption strip along the panel's
/// bottom edge when non-empty, else no caption. The real narration audio
/// stays the only sound; these never carry their own audio track.
///
/// `drop_bg` is `(video_src, volume_0_to_1)` -- a full-bleed background
/// video, playing the whole runtime, UNDER a CSS approximation of a
/// z-plane glitch tear (real `z_plane_bleed.wgsl` is a GPU shader this
/// plain-HTML page can't run; this is an honest CSS stand-in, not that
/// shader). Its own audio plays at `volume`, alongside the real narration
/// -- both tracks are real, simultaneous, never muted to silence.
/// `open_photo` is `(end_ms, image_src)` -- a real photo held full-screen,
/// prominent (not the ambient `bg_window` treatment), from playback start
/// through `end_ms`, then it fades out for good. For a real opening shot
/// (Sean 2026-08-20: "Walterdale bridge picture should be first").
///
/// `badges` is `&[(start_ms, end_ms, image_src, label)]` -- real credential
/// marks (union local, safety, coatings-inspection certifications) shown in
/// a row above the subtitle line, each one fading in only while playback is
/// inside its own window, one badge per spoken credential (Sean 2026-08-20:
/// "put the AMPP NACE NCSO SSPC badges as they are spoken down below in a
/// line"). Multiple windows may overlap; each badge tracks its own.
///
/// `end_cards` is a sequence of hero-size names shown one at a time after
/// the real narration ends -- `13forge.com` then `deveraux.dev` (Sean
/// 2026-08-20: "it will be 13forge.com and then my crate page
/// deveraux.dev"). Each word is textured with `onyx-color.jpg` (the real
/// Onyx crystal PBR color map) via `background-clip: text`, a CSS
/// approximation of `crystalline.wgsl`'s look -- that's a GPU refraction
/// shader this plain-HTML page can't run, so this is an honest stand-in,
/// not that shader. The last name in the list holds forever; the rest
/// hold then hand off to the next.
pub fn render_forced(
    words: &[KaraokeWord],
    sections: &[Section],
    audio_src: &str,
    title: &str,
    force_emphasis: &[usize],
    bg_window: Option<(u32, u32, &str)>,
    video_insets: &[(u32, u32, &str, &str)],
    drop_bg: Option<(&str, f32)>,
    open_photo: Option<(u32, &str)>,
    badges: &[(u32, u32, &str, &str)],
    end_cards: &[&str],
) -> String {
    let flags = emphasis_flags(words);
    let mut word_data = String::new();
    for (i, w) in words.iter().enumerate() {
        let emphasis = if flags.get(i).copied().unwrap_or(false) || force_emphasis.contains(&i) {
            1
        } else {
            0
        };
        word_data.push_str(&format!(
            "[{},{},{},\"{}\"],",
            w.start_ms,
            w.end_ms,
            emphasis,
            escape_js(&w.word)
        ));
    }

    let mut section_bounds = String::new();
    for s in sections {
        section_bounds.push_str(&format!("[{},{},\"{}\"],", s.start_ms, s.end_ms, s.accent));
    }

    // Always visible, looping the whole runtime (Sean 2026-08-20: "the gif
    // can loop the whole time") -- `bg_window` still names a window, now
    // used only to PULSE brighter during that span, not to gate on/off.
    let bg_css = if bg_window.is_some() {
        r#"
.cree-bg {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0.16;
  z-index: -1;
  transition: opacity 1.2s ease;
  pointer-events: none;
}
.cree-bg.on { opacity: 0.5; }"#
    } else {
        ""
    };
    let bg_img = match bg_window {
        Some((_, _, src)) => format!(r#"<img class="cree-bg" id="creeBg" src="{src}" alt="">"#),
        None => String::new(),
    };
    let bg_js = match bg_window {
        Some((start, end, _)) => format!(
            r#"
  const creeBg = document.getElementById('creeBg');
  if (creeBg) creeBg.classList.toggle('on', ms >= {start} && ms < {end});"#
        ),
        None => String::new(),
    };

    let open_css = if open_photo.is_some() {
        r#"
.open-photo {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  z-index: 4;
  opacity: 1;
  transition: opacity 2s ease;
}
.open-photo.gone { opacity: 0; pointer-events: none; }"#
    } else {
        ""
    };
    let open_tag = match open_photo {
        Some((_, src)) => format!(r#"<img class="open-photo" id="openPhoto" src="{src}" alt="">"#),
        None => String::new(),
    };
    let open_js = match open_photo {
        Some((end, _)) => format!(
            r#"
  const openPhoto = document.getElementById('openPhoto');
  if (openPhoto) openPhoto.classList.toggle('gone', ms >= {end});"#
        ),
        None => String::new(),
    };

    let drop_css = if drop_bg.is_some() {
        r#"
.drop-bg {
  position: fixed;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0.22;
  z-index: -3;
  mix-blend-mode: screen;
  animation: zplane-tear 6s steps(1) infinite;
}
@keyframes zplane-tear {
  0%, 100% { clip-path: inset(0 0 0 0); transform: translate(0, 0); filter: hue-rotate(0deg); }
  4% { clip-path: inset(12% 0 68% 0); transform: translate(-6px, 0); filter: hue-rotate(30deg); }
  4.5% { clip-path: inset(0 0 0 0); transform: translate(0, 0); filter: hue-rotate(0deg); }
  37% { clip-path: inset(58% 0 20% 0); transform: translate(5px, 0); filter: hue-rotate(-20deg); }
  37.6% { clip-path: inset(0 0 0 0); transform: translate(0, 0); filter: hue-rotate(0deg); }
  71% { clip-path: inset(30% 0 50% 0); transform: translate(-4px, 0); filter: hue-rotate(15deg); }
  71.4% { clip-path: inset(0 0 0 0); transform: translate(0, 0); filter: hue-rotate(0deg); }
}"#
    } else {
        ""
    };

    let video_css = if video_insets.is_empty() {
        ""
    } else {
        r#"
.proof-inset {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -46%);
  width: min(70vw, 933px);
  aspect-ratio: 16 / 9;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(5, 8, 18, 0.7);
  border-radius: 16px;
  overflow: hidden;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.6);
  opacity: 0;
  z-index: 5;
  transition: opacity 0.5s ease;
  pointer-events: none;
}
.proof-inset.on { opacity: 1; }
.proof-inset video { display: block; width: 100%; height: 100%; object-fit: contain; }
.proof-inset .proof-label {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  padding: 8px 14px;
  font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
  font-size: clamp(12px, 1.4vw, 16px);
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: #E6F1FF;
  background: linear-gradient(0deg, rgba(5, 8, 18, 0.85), transparent);
}"#
    };
    let drop_tag = match drop_bg {
        Some((src, _)) => format!(r#"<video class="drop-bg" id="dropBg" src="{src}" loop playsinline></video>"#),
        None => String::new(),
    };
    let drop_js = match drop_bg {
        Some((_, vol)) => format!(
            r#"
const dropBg = document.getElementById('dropBg');
if (dropBg) {{
  dropBg.volume = {vol};
  let narrationDone = false;
  document.addEventListener('click', () => dropBg.play().catch(() => {{}}), {{ once: true }});
  // Sean 2026-08-20: "sound doesnt stop at end" -- once the real
  // narration has genuinely ended, the drop loop is not allowed back on,
  // even if `player` fires a stray 'play' later (a scrub, a stutter).
  player.addEventListener('play', () => {{ if (!narrationDone) dropBg.play().catch(() => {{}}); }});
  // Sean 2026-08-20: "sound continues to run from drop after the slides
  // end" -- the drop loop must never outlive the real narration. Hard
  // stop: pause AND rewind, not just pause, so nothing lingers audible.
  player.addEventListener('pause', () => {{ dropBg.pause(); dropBg.currentTime = 0; }});
  player.addEventListener('ended', () => {{ narrationDone = true; dropBg.pause(); dropBg.currentTime = 0; }});
}}"#
        ),
        None => String::new(),
    };

    let mut video_tags = String::new();
    let mut video_js = String::new();
    for (i, (start, end, src, label)) in video_insets.iter().enumerate() {
        let label_html = if label.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="proof-label">{label}</div>"#)
        };
        video_tags.push_str(&format!(
            r#"<div class="proof-inset" id="proofInset{i}"><video src="{src}" muted loop playsinline></video>{label_html}</div>
"#
        ));
        video_js.push_str(&format!(
            r#"
  {{
    const el = document.getElementById('proofInset{i}');
    const v = el.querySelector('video');
    const on = ms >= {start} && ms < {end};
    el.classList.toggle('on', on);
    if (on && v.paused) v.play().catch(() => {{}});
    if (!on && !v.paused) v.pause();
  }}"#
        ));
    }

    let badge_css = if badges.is_empty() {
        ""
    } else {
        r#"
/* Off to the side, stacked, clear of the bottom subtitle line (Sean
   2026-08-20: "the logos are overtop the words... pop up slightly above
   words and then fly off to the side stacked in a vertical, 1 at a
   time"). Each badge pops near the reading line, then flies right into
   its own permanent slot in the vertical stack as its credential is
   spoken -- earned, not gated on/off. */
.badge-row {
  position: fixed;
  top: 20%;
  right: 28px;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 16px;
  z-index: 7;
  pointer-events: none;
}
/* A light backing chip behind every mark -- these are real badges cut for
   display on a light card, and one of them (NACE) is black line art with
   no fill color: straight on the page's near-black body it nearly
   disappeared. The chip fixes contrast for all of them at once and also
   gives the odd-shaped Journeyman certificate scan the same footprint as
   the round logos instead of floating as a tiny loose rectangle (Sean
   2026-08-20: "can we clean this up"). */
.badge-row .chip {
  width: 64px;
  height: 64px;
  padding: 6px;
  border-radius: 14px;
  background: #EDEFF3;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.55);
  opacity: 0;
  transform: translate(-160px, 46px) scale(0.6);
  box-sizing: border-box;
}
.badge-row .chip.cert { width: 96px; }
.badge-row .chip img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
}
.badge-row .chip.on {
  animation: badge-pop-fly 0.85s cubic-bezier(0.2, 0.9, 0.3, 1.2) forwards;
}
/* Sean 2026-08-20: "the logos can time out and fad away after making
   their point" -- once a badge's word span ends, it dissolves rather
   than sitting on screen for the rest of the reel. */
.badge-row .chip.off {
  animation: badge-fade-out 0.6s ease forwards;
}
@keyframes badge-pop-fly {
  0%   { opacity: 0; transform: translate(-160px, 46px) scale(0.6); }
  40%  { opacity: 1; transform: translate(-32px, -10px) scale(1.14); }
  100% { opacity: 1; transform: translate(0, 0) scale(1); }
}
@keyframes badge-fade-out {
  0%   { opacity: 1; transform: translate(0, 0) scale(1); }
  100% { opacity: 0; transform: translate(24px, -18px) scale(0.85); }
}"#
    };
    let mut badge_tags = String::new();
    let mut badge_js = String::new();
    for (i, (start, end, src, label)) in badges.iter().enumerate() {
        let cert_class = if src.ends_with(".jpg") { " cert" } else { "" };
        badge_tags.push_str(&format!(
            r#"<div class="chip{cert_class}" id="badge{i}"><img src="{src}" alt="{label}"></div>"#
        ));
        badge_js.push_str(&format!(
            r#"
  {{
    const b = document.getElementById('badge{i}');
    if (b && ms >= {start} && !b.classList.contains('on') && !b.classList.contains('off')) b.classList.add('on');
    if (b && ms >= {end} && !b.classList.contains('off')) b.classList.add('off');
  }}"#
        ));
    }
    let badge_row = if badges.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="badge-row">{badge_tags}</div>"#)
    };

    let end_css = if end_cards.is_empty() {
        ""
    } else {
        r#"
.end-card {
  position: fixed;
  inset: 0;
  z-index: 11;
  background: #050812;
  opacity: 0;
  pointer-events: none;
  transition: opacity 1s ease;
}
.end-card.on { opacity: 1; }
.end-card .name {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: 'Space Grotesk', 'Segoe UI', system-ui, sans-serif;
  font-weight: 700;
  font-size: clamp(40px, 9vw, 108px);
  letter-spacing: 0.02em;
  /* Sean 2026-08-20: "the colour is off on it... more blue like the
     logo" -- `color` blend mode keeps the real Onyx texture's crystal
     shading/luminance but retints its hue/saturation to the same
     oklch(78% 0.15 230) cyan-blue the "13forge" wordmark uses. Toned
     down again ("13forge.com colours dont match they are too
     cartoony") -- same void_compression.wgsl principle as the hero-glow
     rework: compress toward darkness/desaturation instead of blowing
     highlights out, so the material reads grounded, not neon. */
  background-image: linear-gradient(oklch(68% 0.10 230), oklch(68% 0.10 230)), url('onyx-color.jpg');
  background-blend-mode: color;
  background-size: 220% 220%, 220% 220%;
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  filter: brightness(1.15) contrast(1.1);
  opacity: 0;
  padding: 0 6vw;
  text-align: center;
}
.end-card .name.long { font-size: clamp(24px, 4.8vw, 48px); }
/* 13forge.com is the hero: 2/3-scale dominant, stamps in last, holds
   forever. crates.io and deveraux.dev spawn center same as the hero,
   then fly down into a small permanent bottom row -- both stay on
   screen together at the end frame (Sean 2026-08-20: "no 13forge.com
   and it should stick around at the end frame... deveraux.dev and
   crates are a bit lackluster, make them spawn and then move to bottom
   to persist with 13forge.com being 2/3 hero stamp on beeps"). */
.end-card .name.final {
  font-size: clamp(80px, 19vw, 240px);
  z-index: 1;
}
.end-card .name.show.final {
  animation: end-stamp-final 0.6s cubic-bezier(0.2, 1.7, 0.4, 1) forwards, crystal-shimmer 6s ease-in-out infinite;
}
/* Molten glowing radiant bloom (Sean 2026-08-20: "it should be in the
   middle at the end molten glowing radiant bloom"), then dark
   techno-industrial re-tone, then ("orange glow behind letters with
   blue and maybe some radiant heat") -- landed on both: the vignette
   still sinks to near-black at the edge (industrial darkness), but the
   core is now a molten orange/ember heat source instead of steel-blue,
   sitting behind the still-blue crystal text. A separate blurred
   radial-gradient layer breathing behind the hero text, not animated
   on the text's own `filter`/`transform` (those already carry the
   stamp-in and the crystal shimmer; a third animation fighting over
   the same properties would just get overridden, same bug as the
   stamp/hold clash earlier this session). */
.end-card .hero-glow {
  position: absolute;
  inset: 0;
  z-index: 0;
  opacity: 0;
  background: radial-gradient(circle at center, oklch(72% 0.19 50 / 0.75) 0%, oklch(58% 0.18 40 / 0.5) 26%, oklch(24% 0.07 30 / 0.55) 55%, transparent 78%);
  filter: blur(24px);
}
.end-card .hero-glow.on {
  opacity: 1;
  animation: heat-flicker 2.6s ease-in-out infinite;
}
/* Irregular multi-stop pulse instead of a smooth sine breathe -- reads
   as radiant heat shimmer, not a mechanical strobe. */
@keyframes heat-flicker {
  0%   { transform: scale(1);    opacity: 0.72; }
  18%  { transform: scale(1.08); opacity: 0.92; }
  34%  { transform: scale(0.96); opacity: 0.68; }
  52%  { transform: scale(1.16); opacity: 1; }
  71%  { transform: scale(1.02); opacity: 0.8; }
  100% { transform: scale(1);    opacity: 0.72; }
}
.end-card .name.dock.go {
  animation: end-dock 1.1s cubic-bezier(0.2, 1.6, 0.4, 1) forwards, crystal-shimmer 6s ease-in-out infinite;
}
/* Slams down big and bright, then settles -- a stamp hit, not a fade
   (Sean 2026-08-20: "time it to stamp big letters... on the beeps!"),
   paired 1:1 with a synthesized beep at the same instant (endBeep() in
   the timeupdate script below -- a real Web Audio oscillator tone, not
   a fabricated recording). */
@keyframes end-dock {
  0%   { opacity: 0; transform: translate(0, 0) scale(3.0); filter: brightness(1.8) contrast(1.15); }
  35%  { opacity: 1; transform: translate(0, 0) scale(0.92); filter: brightness(1.9) contrast(1.2); }
  55%  { transform: translate(0, 0) scale(1.04); filter: brightness(1.4) contrast(1.1); }
  100% { opacity: 1; transform: translate(var(--dock-x, 0), 34vh) scale(0.55); filter: brightness(1.1) contrast(1.05); }
}
@keyframes end-stamp-final {
  0%   { opacity: 0; transform: scale(3.4); filter: brightness(1.8) contrast(1.15); }
  55%  { opacity: 1; transform: scale(0.9); filter: brightness(1.9) contrast(1.2); }
  78%  { transform: scale(1.06); filter: brightness(1.4) contrast(1.1); }
  100% {
    opacity: 1;
    transform: scale(1);
    /* Back to orange (Sean 2026-08-20: "orange glow behind letters with
       blue") -- the earlier cartoony read was the *brightness blowout*
       stacked on top of the clash, not the complementary pair itself;
       kept the toned-down brightness/contrast from that fix and only
       reintroduced the warm hue, so blue letters now sit inside an
       orange heat glow instead of fighting it at full saturation. */
    filter: brightness(1.2) contrast(1.1)
      drop-shadow(0 0 20px oklch(68% 0.18 45 / 0.7))
      drop-shadow(0 0 52px oklch(58% 0.17 45 / 0.5))
      drop-shadow(0 0 110px oklch(45% 0.14 40 / 0.35));
  }
}
@keyframes crystal-shimmer {
  0%, 100% { background-position: 15% 25%, 15% 25%; }
  50% { background-position: 85% 75%, 85% 75%; }
}"#
    };
    let mut end_tags = String::new();
    let dock_count = end_cards.len().saturating_sub(1);
    for (i, name) in end_cards.iter().enumerate() {
        let long_class = if name.len() > 16 { " long" } else { "" };
        if i + 1 == end_cards.len() {
            end_tags.push_str(&format!(
                r#"<span class="name final{long_class}" id="endCard{i}">{name}</span>"#
            ));
        } else {
            let spread = (i as f64 - (dock_count as f64 - 1.0) / 2.0) * 34.0;
            end_tags.push_str(&format!(
                r#"<span class="name dock{long_class}" id="endCard{i}" style="--dock-x: {spread:.1}vw">{name}</span>"#
            ));
        }
    }
    let end_html = if end_cards.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="end-card" id="endCardWrap"><div class="hero-glow" id="heroGlow"></div>{end_tags}</div>"#)
    };
    let end_js = if end_cards.is_empty() {
        String::new()
    } else {
        let last = end_cards.len().saturating_sub(1);
        let mut chain = String::new();
        for i in 0..end_cards.len() {
            let trigger_class = if i == last { "'show', 'final'" } else { "'go'" };
            // Ascending major-third arpeggio, one beep per stamp -- 440Hz
            // (A4) stepping up 4 semitones per card (Sean 2026-08-20:
            // "on the beeps! ya!").
            let freq = 440.0_f64 * 2f64.powf(i as f64 * 4.0 / 12.0);
            let glow_hit = if i == last {
                "const heroGlow = document.getElementById('heroGlow'); if (heroGlow) heroGlow.classList.add('on'); "
            } else {
                ""
            };
            let hit = format!(
                r#"{glow_hit}document.getElementById('endCard{i}').classList.add({trigger_class}); endBeep({freq:.1});
"#
            );
            if i == 0 {
                chain.push_str(&hit);
            } else {
                // Sean 2026-08-20: "no 13forge.com" -- fast cascade, not a
                // slow reveal, so the hero lands well within a few seconds
                // of the narration ending, not a distant afterthought.
                chain.push_str(&format!("setTimeout(() => {{ {hit} }}, {});\n", i * 500));
            }
        }
        format!(
            r#"
let endCtx = null;
function endBeep(freq) {{
  if (!endCtx) endCtx = new (window.AudioContext || window.webkitAudioContext)();
  const t0 = endCtx.currentTime;
  /* Sean 2026-08-20: "a little more metal" -> "lol thats even worse" ->
     "like a cow bell or clown bell" -> "replace it with subbass" --
     dropped the pitched ping entirely for a low sub thump: a sine that
     glides down into the target frequency's low octave (classic 808-
     style pitch-envelope kick), no harmonics/noise to fight the
     narration's own low end. Still steps up per card via `freq`, just
     three octaves down, so the arpeggio shape survives as a felt pitch
     rise rather than a heard tonal one. */
  const sub = freq / 8;
  const osc = endCtx.createOscillator();
  const gain = endCtx.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(sub * 2.4, t0);
  osc.frequency.exponentialRampToValueAtTime(sub, t0 + 0.09);
  gain.gain.setValueAtTime(0, t0);
  gain.gain.linearRampToValueAtTime(0.5, t0 + 0.008);
  gain.gain.exponentialRampToValueAtTime(0.0001, t0 + 0.5);
  osc.connect(gain);
  gain.connect(endCtx.destination);
  osc.start(t0);
  osc.stop(t0 + 0.52);
}}
player.addEventListener('ended', () => {{
  const wrap = document.getElementById('endCardWrap');
  if (wrap) wrap.classList.add('on');
  {chain}}});"#
        )
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{title}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@500;700&family=Inter:wght@400;600&display=swap" rel="stylesheet">
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{
  background: #050812;
  color: #E6F1FF;
  font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
  min-height: 100vh;
  border-left: 4px solid var(--accent, #1AE0FF);
  transition: border-color 0.6s;
}}
/* The real title screen -- front and center, its OWN moment, not text
   floated over the bridge photo (Sean 2026-08-20: "the picture is blown
   up. Should not be the Title shot... It should be the title screen front
   and center"). Solid background so nothing behind it shows through
   while it holds, then dissolves for good. */
.brand {{
  position: fixed;
  inset: 0;
  z-index: 9;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.3em;
  background: #050812;
  opacity: 1;
  animation: title-card 3s ease-in forwards;
  pointer-events: none;
}}
.brand .wordmark {{
  position: relative;
  display: flex;
  align-items: baseline;
  gap: 0.4em;
  /* Sean 2026-08-20: "put a bar on opening shot logo or an overlay for
     high contrast, can be hero sized too" -- a translucent plate behind
     the mark, its own edge lit in the brand blue, so it reads clean even
     if a future background behind the title card isn't pure black. */
  padding: 0.35em 0.9em;
}}
.brand .wordmark::before {{
  content: '';
  position: absolute;
  inset: 0;
  z-index: -1;
  background: rgba(8, 13, 26, 0.72);
  border-top: 1px solid oklch(78% 0.15 230 / 0.4);
  border-bottom: 1px solid oklch(78% 0.15 230 / 0.4);
}}
.brand .name {{
  font-family: 'Space Grotesk', 'Segoe UI', system-ui, sans-serif;
  font-size: clamp(56px, 12vw, 140px);
  font-weight: 700;
  letter-spacing: 0.06em;
  color: oklch(78% 0.15 230);
  text-shadow: 0 0 32px oklch(78% 0.15 230 / 0.55);
}}
.brand .s13 {{
  font-family: 'Space Grotesk', 'Segoe UI', system-ui, sans-serif;
  font-size: clamp(24px, 3.6vw, 40px);
  font-weight: 500;
  font-style: italic;
  color: oklch(78% 0.15 50);
  text-shadow: 0 0 22px oklch(78% 0.15 50 / 0.55);
}}
.brand .sub {{
  font-family: 'Inter', 'Segoe UI', system-ui, sans-serif;
  font-size: clamp(15px, 2vw, 22px);
  font-weight: 600;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: #86A8D0;
}}
@keyframes title-card {{
  0% {{ opacity: 1; }}
  65% {{ opacity: 1; }}
  100% {{ opacity: 0; }}
}}
/* One subtitle line, fixed above the player, not a growing page --
   words appear then dissolve ethereal-style (Sean 2026-08-20: "words in
   one line in the bottom disappearing... coloured words dissappear
   ethereal style"), never accumulate. */
main {{
  position: fixed;
  left: 0;
  right: 0;
  bottom: 92px;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  align-items: baseline;
  gap: 0.45em;
  padding: 0 32px;
  font-size: clamp(28px, 4.6vw, 46px);
  text-align: center;
  z-index: 6;
  pointer-events: none;
}}
span.word {{
  opacity: 0;
  animation: rise 0.25s ease-out forwards;
}}
span.word.fade-out {{
  animation: ethereal 1.1s ease-in forwards;
}}
span.word.hi {{
  color: var(--accent, #FF2A4D);
  font-size: 1.4em;
  font-weight: 700;
  text-shadow: 0 0 18px var(--accent, #FF2A4D);
  animation: hit 0.4s cubic-bezier(0.2, 0.9, 0.3, 1.3) forwards;
}}
span.word.hi.fade-out {{
  animation: ethereal-hi 1.1s ease-in forwards;
}}
@keyframes rise {{
  from {{ opacity: 0; transform: translateY(4px); }}
  to {{ opacity: 1; transform: translateY(0); }}
}}
@keyframes hit {{
  0% {{ opacity: 0; transform: scale(1.9); }}
  55% {{ opacity: 1; transform: scale(1.08); }}
  100% {{ opacity: 1; transform: scale(1); }}
}}
@keyframes ethereal {{
  0% {{ opacity: 1; filter: blur(0); transform: translateY(0); text-shadow: none; }}
  60% {{ text-shadow: 0 0 12px var(--accent, #86A8D0); }}
  100% {{ opacity: 0; filter: blur(6px); transform: translateY(-18px); text-shadow: 0 0 22px var(--accent, #86A8D0); }}
}}
@keyframes ethereal-hi {{
  0% {{ opacity: 1; filter: blur(0); transform: translateY(0) scale(1); }}
  100% {{ opacity: 0; filter: blur(10px); transform: translateY(-24px) scale(1.15); }}
}}
.player {{
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 16px;
  background: #0B101E;
  border-top: 1px solid #243C5E;
  z-index: 10;
}}
audio {{ width: 100%; }}
{bg_css}
{drop_css}
{video_css}
{open_css}
{badge_css}
{end_css}
</style>
</head>
<body>
{drop_tag}
{bg_img}
{open_tag}
<div class="brand"><div class="wordmark"><span class="name">13forge</span><span class="s13">S13</span></div><div class="sub">Balanced. Deterministic. Real.</div></div>
{video_tags}{badge_row}<main id="text"></main>
<div class="player"><audio id="player" controls src="{audio_src}"></audio></div>
{end_html}
<script>
const player = document.getElementById('player');
const text = document.getElementById('text');
const words = [{word_data}];
const sections = [{section_bounds}];
{drop_js}
{end_js}

function accentFor(ms) {{
  for (const [start, end, color] of sections) {{
    if (ms >= start && ms < end) return color;
  }}
  return '#1AE0FF';
}}

let shown = 0;
const HOLD_MS = 1900;   // how long a word stays solid before dissolving --
                         // longer hold = bigger empty-screen gaps between
                         // words for the visuals to actually be seen
                         // (Sean 2026-08-20: "Big gaps for visuals").
const FADE_MS = 1100;   // must match the .fade-out/@keyframes ethereal* duration

function reveal(count) {{
  if (count === shown) return;
  if (count < shown) {{
    // seeked backward -- rebuild from scratch, cheap at this scale.
    text.innerHTML = '';
    shown = 0;
  }}
  for (; shown < count; shown++) {{
    const [, , emphasis, word] = words[shown];
    const span = document.createElement('span');
    span.className = 'word' + (emphasis ? ' hi' : '');
    span.textContent = word;
    text.appendChild(span);
    setTimeout(() => {{
      span.classList.add('fade-out');
      setTimeout(() => span.remove(), FADE_MS);
    }}, HOLD_MS);
  }}
}}

player.addEventListener('timeupdate', () => {{
  const ms = player.currentTime * 1000;
  document.body.style.setProperty('--accent', accentFor(ms));
  let count = 0;
  while (count < words.length && words[count][0] <= ms) count++;
  reveal(count);{bg_js}{video_js}{open_js}{badge_js}
}});
</script>
</body>
</html>
"##
    )
}

fn escape_js(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(w: &str, start_ms: u32, end_ms: u32) -> KaraokeWord {
        KaraokeWord { word: w.to_string(), start_ms, end_ms }
    }

    #[test]
    fn renders_word_data_with_emphasis_flags() {
        let words = vec![
            word("word", 0, 500),
            word("held", 500, 1_000),
            word("GRAND", 1_000, 3_000),
        ];
        let sections = vec![Section { start_ms: 0, end_ms: 3_000, accent: "#1AE0FF" }];
        let html = render(&words, &sections, "453am.wav", "453am");
        assert!(html.contains("[0,500,0,\"word\"]"));
        assert!(html.contains("[1000,3000,1,\"GRAND\"]"));
        assert!(html.contains("src=\"453am.wav\""));
        assert!(html.contains("[0,3000,\"#1AE0FF\"]"));
        // No pre-rendered WORD spans -- those are revealed by JS only.
        // The static brand header ("13FORGE"/"S13") legitimately uses
        // <span> too, so the real invariant is "word" isn't among them.
        assert!(!html.contains("<span class=\"word"));
        assert!(html.contains("id=\"text\"></main>"));
    }

    #[test]
    fn escapes_quotes_and_backslashes_in_js_string_literals() {
        let words = vec![word(r#"say "hi" \o/"#, 0, 100)];
        let html = render(&words, &[], "x.wav", "t");
        assert!(html.contains(r#"say \"hi\" \\o/"#));
    }

    #[test]
    fn thirds_splits_60_30_10_by_word_span() {
        let words = vec![word("a", 0, 1_000), word("b", 9_000, 10_000)];
        let sections = thirds(&words);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].start_ms, 0);
        assert_eq!(sections[0].end_ms, 6_000);
        assert_eq!(sections[1].end_ms, 9_000);
        assert_eq!(sections[2].end_ms, 10_000);
    }

    #[test]
    fn thirds_is_empty_for_no_words() {
        assert!(thirds(&[]).is_empty());
    }
}
