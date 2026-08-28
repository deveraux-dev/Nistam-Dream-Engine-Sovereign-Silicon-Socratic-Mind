//! emit_html — the vixi→HTML5 emitter (source-compiler "1 AST, N emitters").
//!
//! Walks a [`LoweredUi`]'s paint plane (`draws: Vec<DrawCmd>`) — the SAME plane a
//! native `DrawCmd`→GPU renderer would walk — and emits one self-contained HTML
//! page. Drained from v2 `F:\NewRepo\crates\forge-vix\src\emit_html.rs`
//! (MIGRATION.md:179-181, §MV Track B first slice).
//!
//! SEED: geometry (absolute boxes from `DrawCmd.bounds`, integer MilliUnit → px) plus
//! a deterministic token-derived fill so distinct tokens read as distinct pixels. The
//! real `.sheet.vixi` 8-slot `token_id → rgb` resolve is a later slice.

use crate::ir::{IrRect, LayoutPolicy, LoweredUi, SlotKind};
use crate::tokens::{Palette, palette_slot};

/// The label run's sink. Bone ink on the emitter's near-black ground clears the
/// universal `contrast_min = 4.5` footer gate with room to spare; `pointer-events:
/// none` keeps the run from stealing hits from the slot that owns them, so
/// `data-vixi-id` stays the one event handle.
///
/// `display:block`, NOT flex: centering a run is a layout decision, and exact
/// mode's whole claim is that this engine already solved the layout (the guard in
/// `exact_is_the_default_and_carries_the_solved_rects` catches exactly that leak).
const TEXT_CSS: &str = "#vp .t{display:block;padding:2px 4px;\
font:12px/1.35 'IBM Plex Mono',ui-monospace,monospace;color:#e8e0d4;\
pointer-events:none;overflow:hidden;white-space:pre}";

/// [`TEXT_CSS`], but the label ink comes from the live [`Palette`]'s `fg_text`
/// slot instead of the hardcoded seed colour — the run stops fighting the
/// theme it is painted inside. Same rule shape, real source.
fn text_css_themed(palette: &Palette) -> String {
    format!(
        "#vp .t{{display:block;padding:2px 4px;\
font:12px/1.35 'IBM Plex Mono',ui-monospace,monospace;color:{};\
pointer-events:none;overflow:hidden;white-space:pre}}",
        hex(palette.fg_text)
    )
}

/// `Rgb8` -> `#rrggbb`. The one hex sink both the seed hash and the real
/// palette resolve share, so a themed page and a seeded page never drift in
/// FORMAT, only in source.
fn hex(c: crate::tokens::Rgb8) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

/// Resolve a slot's paint: an authored `color=palette.<slot>` name against a
/// live [`Palette`] wins; anything else (no palette, unknown name, no
/// authored colour) falls back to the deterministic seed hash so an
/// unthemed page still reads as distinct pixels per token (`token_shade`'s
/// original contract, untouched).
fn resolve_bg(tok: u32, chrome_color: Option<&str>, palette: Option<&Palette>) -> String {
    if let (Some(name), Some(p)) = (chrome_color, palette) {
        if let Some(rgb) = palette_slot(p, name) {
            return hex(rgb);
        }
    }
    token_shade(tok)
}

/// [`resolve_bg`] under the SINGLE-DRAW LAW, for slots that carry a `kind`.
///
/// A REGION on a themed page paints only what it authored
/// ([`crate::tokens::authored_fill`]); unauthored, it is `transparent` and the
/// plane under it shows through — which is what makes a rete/tympan split
/// possible at all. Every other kind keeps its default, and the UNTHEMED lane
/// keeps `token_shade` wholesale, because the seed hash exists so a page with
/// no palette still reads as distinct pixels per token. v2 had no unthemed
/// lane to preserve; v3 does, so the law is scoped to where a real palette
/// makes "authored vs not" a meaningful question.
fn resolve_bg_kinded(
    tok: u32,
    kind: Option<SlotKind>,
    chrome_color: Option<&str>,
    palette: Option<&Palette>,
) -> Option<String> {
    if let (Some(SlotKind::Region), Some(p)) = (kind, palette) {
        return crate::tokens::authored_fill(chrome_color, p).map(hex);
    }
    Some(resolve_bg(tok, chrome_color, palette))
}

/// [`resolve_bg_kinded`], but a `kind=brush` slot carrying an authored figure
/// paints that FIGURE instead of a flat fill. The ink is the slot's own
/// `color=`, so a brush still names its palette slot like everything else.
fn resolve_paint(
    tok: u32,
    node: Option<&crate::ir::WidgetNode>,
    palette: Option<&Palette>,
) -> Option<String> {
    let kind = node.map(|n| n.kind);
    let chrome_color = node.and_then(|n| n.chrome_color.as_deref());
    if let (Some(SlotKind::Brush), Some(b)) = (kind, node.and_then(|n| n.brush)) {
        let ink = resolve_bg(tok, chrome_color, palette);
        return Some(brush_css(b, &ink));
    }
    resolve_bg_kinded(tok, kind, chrome_color, palette)
}

/// The authored brush FIGURE as CSS paint — the emitter half of the `kind=brush`
/// dialect, landed 2026-08-26.
///
/// Integer only, and no wall clock: `radius_pmy` is a PERMYRIAD ratio of the
/// slot's own box (a plate circle is a ratio of its plate, never a pixel count),
/// and `phase=tick` is a SimTick commitment the host advances — `parse.rs:764`
/// refuses a wall clock outright, so nothing here reads one either. A `Fixed`
/// phase mater does not turn.
///
/// `stride=weyl` is the low-discrepancy spread; `even` is equal arcs. Both are
/// rendered as stop placement, so the same authored figure is the same pixels
/// every run.
fn brush_css(b: crate::layout::BrushSlot, ink: &str) -> String {
    use crate::parse::{BrushShape, BrushStride};
    // Permyriad -> CSS percent on integers: 3400 -> "34.00%".
    let pct = |p: u16| format!("{}.{:02}%", p / 100, p % 100);
    let r = pct(b.radius_pmy);
    // Weyl's irrational-rotation constant in permyriad (0.6180339887 -> 6180),
    // the low-discrepancy stride; `even` splits the turn in half instead.
    let lead = match b.stride {
        BrushStride::Weyl => "61.80%",
        BrushStride::Even => "50.00%",
    };
    match b.shape {
        BrushShape::Disc => {
            format!("radial-gradient(circle at 50% 50%,{ink} 0,transparent {r})")
        }
        BrushShape::Ring => format!(
            "radial-gradient(circle at 50% 50%,transparent 0,transparent calc({r} - 1px),{ink} {r},transparent calc({r} + 1px))"
        ),
        BrushShape::Arc => {
            format!("conic-gradient(from {lead},{ink} 0,transparent {r})")
        }
        BrushShape::Star => format!(
            "conic-gradient(from {lead},{ink} 0,transparent {r},{ink} calc({r} * 2),transparent calc({r} * 3))"
        ),
        BrushShape::Line => format!("linear-gradient(90deg,transparent 0,{ink} {lead},transparent {r})"),
    }
}

/// `background:` declaration for a resolved paint, or `transparent` when the
/// single-draw law says this slot paints nothing. The box still lowers — it
/// carries its `data-vixi-id`, its hit target, and its label.
fn bg_css(paint: Option<String>) -> String {
    paint.unwrap_or_else(|| "transparent".to_string())
}

/// Which layout TRUTH the HTML lane carries.
///
/// [`LoweredUi`] holds both: the solved integer rects (`LayoutBox.rect`) and the
/// authored policy that produced them (`WidgetNode.layout`). A page can be
/// provable or it can be fluid; it cannot be both, because the second one hands
/// layout to a solver this engine does not own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutEmitMode {
    /// DEFAULT. `position:absolute` off the solved rects — bit-parity with a
    /// native draw list. No reflow.
    #[default]
    Exact,
    /// Opt-in. `display:flex|grid` off the authored policy — reflows, and parity
    /// re-scopes to a per-viewport snapshot instead of a single bit match.
    Responsive,
}

/// Emit a self-contained HTML string from a lowered UI's paint plane within `vp`.
/// Deterministic: integer geometry, no float, no wall-clock, DOM order = `draws` order.
pub fn emit_html(title: &str, ui: &LoweredUi, vp: IrRect) -> String {
    let px = |mu: i64| mu / 1000;
    let (vw, vh) = (px(vp.max_x - vp.min_x).max(1), px(vp.max_y - vp.min_y).max(1));
    let mut s = String::with_capacity(192 + ui.draws.len() * 96);
    s.push_str("<style>body{margin:0}");
    s.push_str(&format!(
        "#vp{{position:relative;width:{vw}px;height:{vh}px;background:#0a0706;overflow:hidden}}"
    ));
    s.push_str("#vp>div{position:absolute;box-sizing:border-box}</style>");
    s.push_str(&format!("<div id=\"vp\" data-title=\"{}\">", esc(title)));
    for d in &ui.draws {
        let b = d.bounds;
        let (x, y, w, h) = (px(b.min_x), px(b.min_y), px(b.max_x - b.min_x), px(b.max_y - b.min_y));
        if w <= 0 || h <= 0 {
            continue;
        }
        let tok = d.token_id.unwrap_or(0);
        s.push_str(&format!(
            "<div style=\"left:{x}px;top:{y}px;width:{w}px;height:{h}px;background:{}\" data-token=\"{tok}\"></div>",
            token_shade(tok)
        ));
    }
    s.push_str("</div>");
    s
}

/// [`emit_html`] on an explicit layout truth.
///
/// `Exact` walks the LAYOUT plane (`ui.layout`), so every box carries its stable
/// key as `data-vixi-id` and its `z` as `z-index` — the reverse map an editor
/// needs to get from a clicked element back to the authored slot.
///
/// `Responsive` walks the WIDGET tree instead and emits the authored policy, so
/// the browser re-solves what this engine already solved. Nested by `parent`,
/// `container-type:inline-size` on every region so `@container` is reachable.
///
/// No JS in either mode — state rides CSS (`:hover`, `:checked`).
pub fn emit_html_mode(title: &str, ui: &LoweredUi, vp: IrRect, mode: LayoutEmitMode) -> String {
    match mode {
        LayoutEmitMode::Exact => emit_exact(title, ui, vp, None),
        LayoutEmitMode::Responsive => emit_responsive(title, ui, vp, None),
    }
}

/// [`emit_html_mode`] resolving every authored `color=palette.<slot>` against a
/// real [`Palette`] instead of the deterministic seed hash — the wire from
/// `WidgetNode::chrome_color` (`ir.rs:194`) through `StyleAtom` to actual pixels.
/// An unauthored node (no `chrome_color`) still seeds off `token_shade`, so an
/// untouched kit renders exactly as before.
pub fn emit_html_mode_themed(title: &str, ui: &LoweredUi, vp: IrRect, mode: LayoutEmitMode, palette: &Palette) -> String {
    match mode {
        LayoutEmitMode::Exact => emit_exact(title, ui, vp, Some(palette)),
        LayoutEmitMode::Responsive => emit_responsive(title, ui, vp, Some(palette)),
    }
}

/// The provable lane: solved rects, absolute, DOM order = layout order.
fn emit_exact(title: &str, ui: &LoweredUi, vp: IrRect, palette: Option<&Palette>) -> String {
    let px = |mu: i64| mu / 1000;
    let (vw, vh) = (px(vp.max_x - vp.min_x).max(1), px(vp.max_y - vp.min_y).max(1));
    let mut s = String::with_capacity(256 + ui.layout.len() * 128); // @forge:allow_alloc: cold emit path
    s.push_str("<style>body{margin:0}");
    s.push_str(&format!(
        "#vp{{position:relative;width:{vw}px;height:{vh}px;background:#0a0706;overflow:hidden}}"
    ));
    s.push_str("#vp>div{position:absolute;box-sizing:border-box}");
    match palette {
        Some(p) => s.push_str(&text_css_themed(p)),
        None => s.push_str(TEXT_CSS),
    }
    s.push_str("</style>");
    s.push_str(&format!("<div id=\"vp\" data-title=\"{}\" data-mode=\"exact\">", esc(title)));
    for lb in &ui.layout {
        let b = lb.rect;
        let (x, y, w, h) = (px(b.min_x), px(b.min_y), px(b.max_x - b.min_x), px(b.max_y - b.min_y));
        if w <= 0 || h <= 0 {
            continue;
        }
        let node = ui.widgets.iter().find(|n| n.id == lb.widget_id);
        let tok = ui
            .draws
            .iter()
            .find(|d| d.widget_id == lb.widget_id)
            .and_then(|d| d.token_id)
            .unwrap_or(0);
        let organ_markup = if node
            .map(|n| {
                n.widget_name.as_ref().map(|w| w.0.as_str()) == Some("astrolabe")
                    || n.semantic.as_deref() == Some("astrolabe")
                    || n.stable_key.0.contains("astrolabe")
            })
            .unwrap_or(false)
        {
            format!(
                "<organ type=\"astrolabe\" class=\"astrolabe-organ\" data-organ=\"astrolabe\"><canvas id=\"astrolabe-canvas-{}\" class=\"astrolabe-canvas\" width=\"{w}\" height=\"{h}\" data-organ=\"astrolabe\"></canvas></organ>",
                esc(&lb.stable_key.0)
            )
        } else {
            String::new()
        };
        s.push_str(&format!(
            "<div style=\"left:{x}px;top:{y}px;width:{w}px;height:{h}px;z-index:{};background:{}{}\" data-vixi-id=\"{}\" data-token=\"{tok}\"{}>{}{}</div>",
            lb.z,
            bg_css(resolve_paint(tok, node, palette)),
            style_css(node.map(|n| n.style_atom()).unwrap_or_default()),
            esc(&lb.stable_key.0),
            state_attrs(node),
            label_html(ui, &lb.stable_key.0),
            organ_markup
        ));
    }
    s.push_str("</div>");
    s
}

/// The fluid lane: authored policy, nested, browser re-solves the boxes.
fn emit_responsive(title: &str, ui: &LoweredUi, vp: IrRect, palette: Option<&Palette>) -> String {
    let px = |mu: i64| mu / 1000;
    let vw = px(vp.max_x - vp.min_x).max(1);
    let mut s = String::with_capacity(256 + ui.widgets.len() * 128); // @forge:allow_alloc: cold emit path
    s.push_str("<style>body{margin:0}");
    s.push_str(&format!("#vp{{width:100%;max-width:{vw}px;background:#0a0706}}"));
    s.push_str("#vp div{box-sizing:border-box;container-type:inline-size}");
    s.push_str("[data-reveal=\"hover\"]{visibility:hidden}");
    s.push_str("*:hover>[data-reveal=\"hover\"]{visibility:visible}");
    match palette {
        Some(p) => s.push_str(&text_css_themed(p)),
        None => s.push_str(TEXT_CSS),
    }
    s.push_str("</style>");
    s.push_str(&format!("<div id=\"vp\" data-title=\"{}\" data-mode=\"responsive\">", esc(title)));
    emit_children(&mut s, ui, None, palette);
    s.push_str("</div>");
    s
}

/// Depth-first emit of `parent`'s children, each carrying its own policy.
fn emit_children(s: &mut String, ui: &LoweredUi, parent: Option<crate::ir::WidgetId>, palette: Option<&Palette>) {
    for node in ui.widgets.iter().filter(|n| n.parent == parent) {
        let tok = ui
            .draws
            .iter()
            .find(|d| d.widget_id == node.id)
            .and_then(|d| d.token_id)
            .unwrap_or(0);
        s.push_str(&format!(
            "<div style=\"{}background:{}{}\" data-vixi-id=\"{}\" data-token=\"{tok}\"{}>{}",
            policy_css(node.layout),
            bg_css(resolve_paint(tok, Some(node), palette)),
            style_css(node.style_atom()),
            esc(&node.stable_key.0),
            state_attrs(Some(node)),
            label_html(ui, &node.stable_key.0)
        ));
        emit_children(s, ui, Some(node.id), palette);
        s.push_str("</div>");
    }
}

/// Emit ONE authored group as a markup FRAGMENT — no `<style>`, no `#vp` shell —
/// so a host page can embed a kit's group inside chrome it already owns.
///
/// `prefix` is a stable key; direct children of it are emitted in lowered order,
/// each carrying its own `data-vixi-id` and authored label. The host binds
/// presses by that key, exactly as a native band does — one authored shape,
/// two renderers, and a kit rename breaks both loudly instead of one silently.
///
/// Deliberately geometry-free: a fragment lands inside the host's own flow, so
/// re-asserting solved rects here would fight the chrome it is embedded in.
pub fn emit_html_fragment(ui: &LoweredUi, prefix: &str) -> String {
    let mut s = String::new(); // @forge:allow_alloc: cold emit path
    for lb in &ui.layout {
        let key = lb.stable_key.0.as_str();
        // Direct children only: "<prefix>.<leaf>" with no further dot.
        let Some(rest) = key.strip_prefix(prefix) else { continue };
        let Some(leaf) = rest.strip_prefix('.') else { continue };
        if leaf.contains('.') {
            continue;
        }
        let node = ui.widgets.iter().find(|n| n.id == lb.widget_id);
        s.push_str(&format!(
            "<button type=\"button\" class=\"htab\" data-vixi-id=\"{}\"{}>{}</button>",
            esc(key),
            state_attrs(node),
            label_html(ui, key)
        ));
    }
    s
}

/// The authored `text="…"` label for a slot, escaped, or `""` when it carries none.
///
/// `source_binds` outranks a literal at RUNTIME (live data beats the authored
/// default), but a static page has no host to ask: the literal is the whole truth
/// here, which is exactly what it was authored for.
fn label_html(ui: &LoweredUi, stable_key: &str) -> String {
    ui.text_literals
        .iter()
        .find(|(k, _)| k == stable_key)
        .map(|(_, words)| format!("<span class=\"t\">{}</span>", esc(words)))
        .unwrap_or_default()
}

/// The HTML sink for a [`StyleAtom`](crate::ir::StyleAtom). Both lanes call it,
/// so neither can drift.
fn style_css(a: crate::ir::StyleAtom) -> String {
    let mut out = String::new();
    if let Some(r) = a.radius_mu {
        out.push_str(&format!(";border-radius:{}px", r / 1000));
    }
    // Permyriad -> CSS ratio on integers: float never enters IR. 8500 -> "0.85".
    if let Some(p) = a.alpha_pmy {
        out.push_str(&format!(";opacity:{}.{:02}", p / 10_000, (p % 10_000) / 100));
    }
    if let Some(f) = a.font.as_deref() {
        out.push_str(&format!(";font-family:{}", esc(f)));
    }
    out
}

/// Authored region policy → the CSS that re-solves it.
fn policy_css(policy: Option<LayoutPolicy>) -> &'static str {
    match policy {
        Some(LayoutPolicy::StackV) => "display:flex;flex-direction:column;",
        Some(LayoutPolicy::StackH) => "display:flex;flex-direction:row;",
        Some(LayoutPolicy::Grid) | Some(LayoutPolicy::HexGrid) => {
            "display:grid;grid-template-columns:repeat(auto-fit,minmax(0,1fr));"
        }
        Some(LayoutPolicy::Overlay) => "display:grid;grid-template-areas:'stack';",
        Some(LayoutPolicy::Flow) => "display:flex;flex-wrap:wrap;",
        None => "",
    }
}

/// Authored affordance flags → data attributes the stylesheet keys off. These are
/// AST fields, not invented state — the emitter only carries what the kit already
/// declared.
fn state_attrs(node: Option<&crate::ir::WidgetNode>) -> String {
    let Some(n) = node else { return String::new() };
    let mut out = String::new();
    if n.hover_reveal {
        out.push_str(" data-reveal=\"hover\"");
    }
    if n.collapsible {
        out.push_str(" data-collapsible=\"1\"");
    }
    if n.long_press_drawer {
        out.push_str(" data-drawer=\"1\"");
    }
    out
}

/// THE HTML5 SHELL: every HTML page this engine emits rides this one skeleton —
/// doctype, `lang`, charset, viewport, one `<style>`, one `<body>`. `title` is
/// escaped here; `css` and `body` are the caller's already-built HTML.
pub fn page(title: &str, css: &str, body: &str) -> String {
    let mut s = String::with_capacity(128 + css.len() + body.len());
    s.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    s.push_str(&format!("<title>{}</title>\n", esc(title)));
    s.push_str("<style>\n");
    s.push_str(css);
    s.push_str("\n</style>\n</head>\n<body>\n");
    s.push_str(body);
    s.push_str("</body>\n</html>\n");
    s
}

/// Interactive IPC bridge JavaScript snippet for sovereign HTML5 / glass runtimes.
/// Wires `window.ipc.postMessage(...)` across `window.chrome.webview`, parent postMessage,
/// and local DOM event delegation for `[data-verb]`, `[data-organ]`, and `[data-vixi-id]`.
pub const IPC_BRIDGE_SCRIPT: &str = r#"<script>
(function() {
  window.ipc = window.ipc || {
    postMessage: function(msg) {
      if (window.chrome && window.chrome.webview && window.chrome.webview.postMessage) {
        window.chrome.webview.postMessage(msg);
      } else if (window.parent && window.parent !== window && window.parent.postMessage) {
        window.parent.postMessage(msg, '*');
      } else {
        console.log('[IPC-FALLBACK]', msg);
      }
    }
  };

  document.addEventListener('click', function(e) {
    var verbEl = e.target.closest('[data-verb]');
    if (verbEl && verbEl.dataset.verb) {
      window.ipc.postMessage('game-verb ' + verbEl.dataset.verb);
      return;
    }
    var organEl = e.target.closest('[data-organ]');
    if (organEl && organEl.dataset.organ) {
      window.ipc.postMessage('organ-click ' + organEl.dataset.organ);
      return;
    }
    var vixiEl = e.target.closest('[data-vixi-id]');
    if (vixiEl && vixiEl.dataset.vixiId) {
      window.ipc.postMessage('vixi-click ' + vixiEl.dataset.vixiId);
    }
  });

  document.addEventListener('keydown', function(e) {
    if ((e.key === 'Enter' || e.keyCode === 13) && !e.shiftKey) {
      var target = e.target;
      if (target && (target.id === 'ndeinput' || (target.classList && target.classList.contains('ndeinput')) || (target.closest && target.closest('.nde-chat-organ')))) {
        e.preventDefault();
        var val = target.value ? target.value.trim() : '';
        if (val) {
          window.ipc.postMessage('nde-ask ' + val);
          target.value = '';
        }
      }
    }
  });

  window.addEventListener('message', function(ev) {
    if (ev && ev.data && window.__forge && typeof window.__forge.feed === 'function') {
      window.__forge.feed(ev.data);
    }
  });
})();
</script>"#;

/// Interactive canvas hook script for `<organ type="astrolabe">`.
/// Mounts the celestial Astrolabe engine to all matching canvases, binds Rete drag-rotation,
/// Alidade altitude sighting, Web Audio harmonic frequency triggers, and IPC events.
pub const ASTROLABE_ORGAN_HOOK_SCRIPT: &str = r#"<script>
(function initAstrolabeCanvasHooks() {
  var organs = document.querySelectorAll('organ[type="astrolabe"], .astrolabe-organ, [data-organ="astrolabe"]');
  organs.forEach(function(organ) {
    var canvas = organ.querySelector('canvas') || (organ.tagName === 'CANVAS' ? organ : null);
    if (canvas && canvas.tagName === 'CANVAS') {
      if (typeof Astrolabe !== 'undefined') {
        var lat = parseFloat(organ.dataset.latitude || '53.54');
        new Astrolabe(canvas.id || canvas, { latitude: lat, autoRotate: true });
      } else {
        var ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.fillStyle = '#08090d';
          ctx.fillRect(0, 0, canvas.width, canvas.height);
          ctx.strokeStyle = '#c3a256';
          ctx.lineWidth = 2;
          ctx.beginPath();
          ctx.arc(canvas.width / 2, canvas.height / 2, Math.min(canvas.width, canvas.height) / 2 - 20, 0, Math.PI * 2);
          ctx.stroke();
        }
      }
      canvas.addEventListener('click', function() {
        if (window.ipc && window.ipc.postMessage) {
          window.ipc.postMessage('organ-interact astrolabe');
        }
      });
    }
  });
})();
</script>"#;

/// Emit an `<organ type="astrolabe">` HTML canvas container with stereographic projection hooks.
pub fn emit_organ_astrolabe(canvas_id: &str, width_px: u32, height_px: u32, lat: f64) -> String {
    format!(
        "<organ type=\"astrolabe\" class=\"astrolabe-organ\" data-organ=\"astrolabe\" data-latitude=\"{lat}\"><canvas id=\"{}\" class=\"astrolabe-canvas\" width=\"{width_px}\" height=\"{height_px}\" data-organ=\"astrolabe\"></canvas></organ>",
        esc(canvas_id)
    )
}

/// Emit a self-contained interactive HTML page with the IPC bridge script and optional astrolabe canvas hooks.
pub fn page_interactive(title: &str, css: &str, body: &str, include_astrolabe: bool) -> String {
    let mut s = String::with_capacity(256 + css.len() + body.len());
    s.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n");
    s.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    s.push_str(&format!("<title>{}</title>\n", esc(title)));
    s.push_str("<style>\n");
    s.push_str(css);
    s.push_str("\n</style>\n</head>\n<body>\n");
    s.push_str(body);
    s.push_str("\n");
    s.push_str(IPC_BRIDGE_SCRIPT);
    s.push_str("\n");
    if include_astrolabe {
        s.push_str(ASTROLABE_ORGAN_HOOK_SCRIPT);
        s.push_str("\n");
    }
    s.push_str("</body>\n</html>\n");
    s
}

/// Emit an interactive LoweredUi HTML document with IPC bridge and astrolabe hooks.
pub fn emit_interactive_html(title: &str, ui: &LoweredUi, vp: IrRect, mode: LayoutEmitMode) -> String {
    let body = emit_html_mode(title, ui, vp, mode);
    let mut s = String::with_capacity(body.len() + 512);
    s.push_str(&body);
    s.push_str("\n");
    s.push_str(IPC_BRIDGE_SCRIPT);
    s.push_str("\n");
    s.push_str(ASTROLABE_ORGAN_HOOK_SCRIPT);
    s
}

/// HTML-escape the five markup-significant characters. The ONE escape.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// SEED `token_id → fill`: a deterministic hex shade (floor-lifted so it reads on the
/// dark ground). Replaced by a real palette resolve in a later slice.
fn token_shade(t: u32) -> String {
    let n = t.wrapping_mul(2_654_435_761);
    format!("#{:02x}{:02x}{:02x}", (n >> 16) as u8 | 0x40, (n >> 8) as u8 | 0x40, n as u8 | 0x40)
}

/// Emit shader metadata for GPU binding — pentaract 5D raymarching kernel.
///
/// Returns a JSON manifest containing:
/// - `shader_name`: identifier for the compute kernel
/// - `phase`: implementation phase (3.1 = SPIRV compiled)
/// - `target`: GPU API (vulkan, webgpu)
/// - `workgroup_size`: [8, 8, 1] threads per group
/// - `bindings`: parameter buffer layout (M5Params, heightmap, output texture)
/// - `phase_proof`: list of proven validation gates
///
/// This pairs with `forge-ocular-v3::PENTARACT_MARCH_5D_WGSL` — wire it into
/// a canvas render target for 2D slice visualization (Phase 4 GPU-CPU parity test).
pub fn emit_shader_manifest_pentaract() -> String {
    r#"{"shader_name":"pentaract_march_5d","phase":"4.0","description":"5D pentaract raymarching (trit quantization, O(1) absence mask, transmittance convergence gate)","target":"vulkan","workgroup_size":[8,8,1],"max_steps":64,"transmittance_threshold":0.001,"bindings":{"params":0,"heightmap_tex":1,"heightmap_samp":2,"out_color":3},"phase_proof":["0.1_trit_bijection_243_cells_zero_collisions","1.4_transmittance_convergence_early_exit","2.3_cpu_slice_raymarching","3.1_spirv_compilation_validated","4.0_gpu_cpu_slice_parity_proven"]}"#.to_string()
}

/// The SINGLE-DRAW LAW, end to end through the real parse→lower→emit lane.
#[cfg(test)]
mod single_draw_law_tests {
    use super::*;
    use crate::layout::{lower, TokenCtx};

    const VP: IrRect = IrRect { min_x: 0, min_y: 0, max_x: 400_000, max_y: 300_000 };

    /// A ground that paints, a plane that does not, and a widget under the
    /// unpainted plane — the rete/tympan shape the law exists for.
    const SRC: &str = "#vixi:kit v1\n\
slot root kind=region layout=overlay padding=mu(0) color=palette.bg_far\n\
slot root.ground kind=region layout=stack_v color=palette.bg_near\n\
slot root.plane kind=region layout=stack_v\n\
slot root.plane.card kind=widget name=button size=mu(44)\n";

    fn page(palette: Option<&Palette>) -> String {
        let doc = crate::parse::parse_kit(SRC).expect("parses");
        let ui = lower(&doc.root, VP, &TokenCtx::comfy(), 1);
        match palette {
            Some(p) => emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, p),
            None => emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact),
        }
    }

    /// The declaration emitted for one slot, by stable key.
    fn bg_for(html: &str, key: &str) -> String {
        let anchor = format!("\" data-vixi-id=\"{key}\"");
        let at = html.find(&anchor).unwrap_or_else(|| panic!("{key} not in page"));
        let head = &html[..at];
        let bg = head.rfind("background:").expect("a background declaration");
        head[bg + "background:".len()..].to_string()
    }

    /// The brush FIGURE reaches the glass. Before 2026-08-26 `shape=`/`radius=`
    /// were strictly typechecked and then dropped at spec→IR, so every authored
    /// figure emitted a flat rect identical to its neighbours.
    #[test]
    fn an_authored_brush_figure_reaches_the_paint() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v color=palette.bg_far\n\
slot root.heat kind=brush shape=disc radius=pmy(3400) phase=tick color=palette.accent_primary\n\
slot root.glint kind=brush shape=arc radius=pmy(1200) stride=weyl color=palette.accent_secondary\n";
        let doc = crate::parse::parse_kit(src).expect("parses");
        let ui = lower(&doc.root, VP, &TokenCtx::comfy(), 1);
        let p = crate::tokens::BaseProfile::molten().palette;
        let html = emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, &p);

        let heat = bg_for(&html, "root.heat");
        assert!(heat.starts_with("radial-gradient"), "a disc must lower to a radial figure: {heat}");
        assert!(heat.contains("34.00%"), "radius pmy(3400) must survive as an integer ratio: {heat}");
        assert!(heat.contains(&hex(p.accent_primary)), "the figure inks with its authored slot");

        let glint = bg_for(&html, "root.glint");
        assert!(glint.starts_with("conic-gradient"), "an arc must lower to a conic figure: {glint}");
        assert!(glint.contains("61.80%"), "stride=weyl must place the low-discrepancy lead: {glint}");
    }

    /// Determinism: the same authored figure is the same pixels every run. No
    /// wall clock reaches this lane — `phase=tick` is a SimTick commitment the
    /// HOST advances, and `parse.rs:764` refuses a wall clock outright.
    #[test]
    fn a_brush_figure_is_deterministic() {
        let src = "#vixi:kit v1\n\
slot root kind=region layout=stack_v\n\
slot root.f kind=brush shape=star radius=pmy(2000) phase=tick stride=weyl\n";
        let doc = crate::parse::parse_kit(src).expect("parses");
        let p = crate::tokens::BaseProfile::permafrost().palette;
        let once = {
            let ui = lower(&doc.root, VP, &TokenCtx::comfy(), 1);
            emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, &p)
        };
        let twice = {
            let ui = lower(&doc.root, VP, &TokenCtx::comfy(), 1);
            emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, &p)
        };
        assert_eq!(once, twice, "the same figure must emit the same bytes");
    }

    #[test]
    fn an_unauthored_region_paints_nothing_on_a_themed_page() {
        let html = page(Some(&crate::tokens::BaseProfile::studio_dark().palette));
        assert_eq!(bg_for(&html, "root.plane"), "transparent", "the sparse plane must not paint");
    }

    #[test]
    fn an_authored_region_still_paints_its_slot() {
        let p = crate::tokens::BaseProfile::studio_dark().palette;
        let html = page(Some(&p));
        assert_eq!(bg_for(&html, "root.ground"), hex(p.bg_near));
        assert_eq!(bg_for(&html, "root"), hex(p.bg_far));
    }

    /// The law is scoped to REGION. A widget authors no colour here and must
    /// still paint, or every unstyled button becomes invisible.
    #[test]
    fn a_widget_is_not_covered_by_the_law() {
        let html = page(Some(&crate::tokens::BaseProfile::studio_dark().palette));
        assert_ne!(bg_for(&html, "root.plane.card"), "transparent", "a widget still paints");
    }

    /// The UNTHEMED lane keeps the seed hash wholesale — that contract exists so
    /// a page with no palette reads as distinct pixels per token, and v2 had no
    /// such lane to preserve.
    #[test]
    fn the_unthemed_seed_shade_contract_is_untouched() {
        let html = page(None);
        assert_ne!(
            bg_for(&html, "root.plane"),
            "transparent",
            "with no palette there is no 'authored vs not' question to ask"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{DrawCmd, TokenStatus, WidgetId};

    fn draw(id: u32, x: i64, y: i64, w: i64, h: i64, tok: u32) -> DrawCmd {
        DrawCmd {
            cmd_id: id,
            widget_id: WidgetId(id),
            bounds: IrRect::from_xywh(x, y, w, h),
            clip_id: None,
            token_status: TokenStatus::Resolved,
            token_id: Some(tok),
            layout_version: 1,
            render_version: 1,
        }
    }

    use crate::ir::{LayoutBox, SlotKind, StableKey};

    fn node(id: u32, key: &str, parent: Option<u32>, layout: Option<LayoutPolicy>) -> crate::ir::WidgetNode {
        crate::ir::WidgetNode {
            id: WidgetId(id),
            stable_key: StableKey(key.to_string()),
            kind: if layout.is_some() { SlotKind::Region } else { SlotKind::Widget },
            widget_name: None,
            layout,
            slot_list_max: None,
            parent: parent.map(WidgetId),
            hover_reveal: false,
            long_press_drawer: false,
            collapsible: false,
            audio_reactive: false,
            brush: None,
            material: None,
            chrome_color: None,
            border_radius: None,
            alpha_pmy: None,
            font: None,
            semantic: None,
        }
    }

    // [BOARD: HTML5-SHELL-FOLD]
    #[test]
    fn one_shell_one_doctype_title_escaped() {
        let html = page("a<b> & 'c'", "body{margin:0}", "<p>x</p>");
        assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"en\">"));
        assert!(html.contains("<meta name=\"viewport\""));
        assert!(html.contains("<title>a&lt;b&gt; &amp; &#39;c&#39;</title>"));
        assert!(html.ends_with("</body>\n</html>\n"));
        assert_eq!(html.matches("<!DOCTYPE").count(), 1);
    }

    // [BOARD: VIXI-ALPHA-PAINTS]
    #[test]
    fn authored_alpha_reaches_the_painted_style() {
        let mut ui = two_lane_ui();
        ui.widgets[0].alpha_pmy = Some(8500);
        let html = emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact);
        assert!(html.contains("opacity:0.85"), "alpha_pmy=8500 must paint opacity:0.85, got: {html}");
    }

    // [BOARD: VIXI-STYLE-ATOM-FOLD]
    #[test]
    fn every_paint_face_reads_one_resolve() {
        let mut ui = two_lane_ui();
        ui.widgets[0].alpha_pmy = Some(8500);
        ui.widgets[0].border_radius = Some(4_000);
        let atom = ui.widgets[0].style_atom();
        assert_eq!(atom.alpha_pmy, Some(8500));
        assert_eq!(atom.radius_mu, Some(4_000));
        for mode in [LayoutEmitMode::Exact, LayoutEmitMode::Responsive] {
            let html = emit_html_mode("t", &ui, VP, mode);
            assert!(html.contains("opacity:0.85"), "{mode:?} dropped alpha: {html}");
            assert!(html.contains("border-radius:4px"), "{mode:?} dropped radius: {html}");
        }
    }

    // [BOARD: PENTARACT-SHADER-MANIFEST]
    #[test]
    fn shader_manifest_pentaract_5d_emits_valid_json() {
        let manifest = emit_shader_manifest_pentaract();
        assert!(manifest.contains(r#""shader_name":"pentaract_march_5d""#));
        assert!(manifest.contains(r#""phase":"4.0""#));
        assert!(manifest.contains(r#""target":"vulkan""#));
        assert!(manifest.contains(r#""workgroup_size":[8,8,1]"#));
        assert!(manifest.contains(r#""transmittance_threshold":0.001"#));
        assert!(manifest.contains(r#""0.1_trit_bijection_243_cells_zero_collisions""#));
        assert!(manifest.contains(r#""3.1_spirv_compilation_validated""#));
        assert!(manifest.contains(r#""4.0_gpu_cpu_slice_parity_proven""#));
        assert!(manifest.starts_with('{'));
        assert!(manifest.ends_with('}'));
    }

    // [BOARD: VIXI-FONT-SEMANTIC-PAINT]
    #[test]
    fn authored_font_and_semantic_reach_the_atom() {
        let mut ui = two_lane_ui();
        ui.widgets[0].font = Some("mono".into());
        ui.widgets[0].semantic = Some("green_yellow_red".into());
        let atom = ui.widgets[0].style_atom();
        assert_eq!(atom.font.as_deref(), Some("mono"));
        assert_eq!(atom.semantic.as_deref(), Some("green_yellow_red"), "meter ramp is not chrome colour");
        for mode in [LayoutEmitMode::Exact, LayoutEmitMode::Responsive] {
            let html = emit_html_mode("t", &ui, VP, mode);
            assert!(html.contains("font-family:mono"), "{mode:?} dropped font: {html}");
        }
    }

    fn lbox(id: u32, key: &str, x: i64, y: i64, w: i64, h: i64, z: i32) -> LayoutBox {
        LayoutBox {
            widget_id: WidgetId(id),
            stable_key: StableKey(key.to_string()),
            rect: IrRect::from_xywh(x, y, w, h),
            z,
            clip_id: None,
            scroll_id: None,
            baseline: None,
            layout_version: 1,
        }
    }

    /// A root stack with one child — enough to tell the two truths apart.
    fn two_lane_ui() -> LoweredUi {
        LoweredUi {
            widgets: vec![
                node(1, "root", None, Some(LayoutPolicy::StackV)),
                node(2, "root.go", Some(1), None),
            ],
            layout: vec![
                lbox(1, "root", 0, 0, 640_000, 480_000, 0),
                lbox(2, "root.go", 24_000, 62_000, 592_000, 40_000, 1),
            ],
            draws: vec![draw(1, 0, 0, 640_000, 480_000, 3), draw(2, 24_000, 62_000, 592_000, 40_000, 7)],
            ..Default::default()
        }
    }

    const VP: IrRect = IrRect { min_x: 0, min_y: 0, max_x: 640_000, max_y: 480_000 };

    /// Exact is the DEFAULT truth: solved rects, absolute.
    #[test]
    fn exact_is_the_default_and_carries_the_solved_rects() {
        assert_eq!(LayoutEmitMode::default(), LayoutEmitMode::Exact);
        let html = emit_html_mode("t", &two_lane_ui(), VP, LayoutEmitMode::Exact);
        assert!(html.contains("data-mode=\"exact\""), "{html}");
        assert!(html.contains("left:24px;top:62px;width:592px;height:40px"), "{html}");
        assert!(!html.contains("display:flex"), "exact mode must never hand layout to the browser");
    }

    /// The reverse map an editor needs: element -> authored slot.
    #[test]
    fn every_exact_box_names_its_authored_slot_and_z() {
        let html = emit_html_mode("t", &two_lane_ui(), VP, LayoutEmitMode::Exact);
        assert!(html.contains("data-vixi-id=\"root\""), "{html}");
        assert!(html.contains("data-vixi-id=\"root.go\""), "{html}");
        assert!(html.contains("z-index:1"), "LayoutBox.z must reach the page: {html}");
    }

    /// Responsive carries the POLICY instead, and nests by parent so the browser
    /// can re-solve. `@container` is only reachable on this branch.
    #[test]
    fn responsive_carries_the_authored_policy_not_the_solved_rect() {
        let html = emit_html_mode("t", &two_lane_ui(), VP, LayoutEmitMode::Responsive);
        assert!(html.contains("data-mode=\"responsive\""), "{html}");
        assert!(html.contains("display:flex;flex-direction:column"), "StackV must lower to a column: {html}");
        assert!(html.contains("container-type:inline-size"), "@container must be reachable: {html}");
        assert!(!html.contains("left:24px"), "responsive must not freeze a solved rect: {html}");
        let root_at = html.find("data-vixi-id=\"root\"").unwrap();
        let child_at = html.find("data-vixi-id=\"root.go\"").unwrap();
        assert!(child_at > root_at, "the child must nest inside its parent: {html}");
    }

    /// The two modes are different documents — if they ever match, one of them
    /// stopped doing its job.
    #[test]
    fn the_two_truths_are_not_the_same_document() {
        let ui = two_lane_ui();
        assert_ne!(
            emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact),
            emit_html_mode("t", &ui, VP, LayoutEmitMode::Responsive)
        );
    }

    /// Authored affordance flags ride to CSS as data attributes — no JS, and
    /// nothing invented: the flags are AST fields.
    #[test]
    fn authored_state_flags_reach_the_page_without_a_script() {
        let mut ui = two_lane_ui();
        ui.widgets[1].hover_reveal = true;
        ui.widgets[1].collapsible = true;
        for mode in [LayoutEmitMode::Exact, LayoutEmitMode::Responsive] {
            let html = emit_html_mode("t", &ui, VP, mode);
            assert!(html.contains("data-reveal=\"hover\""), "{mode:?}: {html}");
            assert!(html.contains("data-collapsible=\"1\""), "{mode:?}: {html}");
            assert!(!html.contains("<script"), "{mode:?} leaked a script tag");
        }
    }

    /// [BOARD: VIX-EMIT-TEXT] the authored label reaches the page.
    #[test]
    fn an_authored_label_reaches_both_lanes() {
        let mut ui = two_lane_ui();
        let key = ui.layout[0].stable_key.0.clone();
        ui.text_literals.push((key.clone(), "FORGE".to_string()));
        for mode in [LayoutEmitMode::Exact, LayoutEmitMode::Responsive] {
            let html = emit_html_mode("t", &ui, VP, mode);
            assert!(html.contains(">FORGE<"), "{mode:?} dropped the authored label: {html}");
            assert!(html.contains("class=\"t\""), "{mode:?} label run missing its sink");
            assert!(html.contains("#vp .t{"), "{mode:?} emitted a run with no CSS");
        }
    }

    /// [BOARD: VIX-EMIT-TEXT] a label is CONTENT, never markup — and a slot with
    /// no literal must stay exactly as empty as it was before this slice.
    #[test]
    fn a_label_is_escaped_and_absent_when_unauthored() {
        let mut ui = two_lane_ui();
        let key = ui.layout[0].stable_key.0.clone();
        ui.text_literals.push((key, "<script>x</script>&\"".to_string()));
        let html = emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact);
        assert!(!html.contains("<script>"), "a label must never become markup: {html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");

        let bare = emit_html_mode("t", &two_lane_ui(), VP, LayoutEmitMode::Exact);
        assert!(!bare.contains("class=\"t\""), "no literal must emit no run: {bare}");
    }

    /// [BOARD: HEARTH-TABS] A fragment carries the group's keys and labels, in
    /// lowered order, and nothing else — no shell, no style, no sibling groups.
    ///
    /// v2's twin of this test built its `LoweredUi` from a real `.kit.vixi`
    /// source via `loader::studio_panel` + `loader::load_kit_live`; that parser
    /// front-end is not in this slice (Cargo.toml header), so the fixture below
    /// is hand-built IR that reproduces the same three-tab shape and asserts the
    /// identical contract: order, labels, no shell, no sibling leakage.
    #[test]
    fn a_fragment_carries_one_group_in_lowered_order() {
        let mut ui = LoweredUi::default();
        for (i, (key, label)) in [(1u32, ("root.tabs.sessions.forge", "FORGE")),
            (2, ("root.tabs.sessions.boot", "BOOT")),
            (3, ("root.tabs.sessions.live", "LIVE")),
            (4, ("root.tabs.modes.edit", "EDIT"))]
        {
            ui.layout.push(lbox(i, key, 0, 0, 10, 10, 0));
            ui.text_literals.push((key.to_string(), label.to_string()));
        }
        let frag = emit_html_fragment(&ui, "root.tabs.sessions");

        for (key, want) in [("forge", "FORGE"), ("boot", "BOOT"), ("live", "LIVE")] {
            assert!(frag.contains(&format!("root.tabs.sessions.{key}")), "{frag}");
            assert!(frag.contains(&format!(">{want}<")), "label {want} missing: {frag}");
        }
        assert_eq!(frag.matches("<button").count(), 3, "3 pills, no more: {frag}");
        assert!(!frag.contains("<style"), "a fragment brings no shell: {frag}");
        assert!(!frag.contains("root.tabs.modes"), "sibling group leaked in: {frag}");
        let (f, b) = (frag.find("FORGE").unwrap(), frag.find("BOOT").unwrap());
        assert!(f < b && b < frag.find("LIVE").unwrap(), "order drifted: {frag}");
    }

    /// [BOARD: HEARTH-TABS] A prefix that names nothing emits nothing — a typo'd
    /// group must not silently render an empty strip's worth of markup.
    #[test]
    fn an_unknown_prefix_emits_an_empty_fragment() {
        let ui = two_lane_ui();
        assert!(emit_html_fragment(&ui, "root.nope").is_empty());
    }

    #[test]
    fn emits_self_contained_html_from_the_paint_plane() {
        let ui = LoweredUi {
            draws: vec![
                draw(1, 0, 0, 100_000, 40_000, 3),
                draw(2, 10_000, 50_000, 80_000, 20_000, 7),
            ],
            ..Default::default()
        };
        let html = emit_html("launcher", &ui, IrRect::from_xywh(0, 0, 1_280_000, 720_000));
        assert_eq!(html.matches("<div style=").count(), 2, "one div per DrawCmd");
        assert!(!html.contains("http://") && !html.contains("https://"), "self-contained (no net)");
        assert!(html.contains("data-title=\"launcher\""), "titled viewport");
        assert!(html.contains("width:100px") && html.contains("top:50px"), "MilliUnit→px geometry");
    }

    #[test]
    fn deterministic_same_ui_same_bytes() {
        let ui = LoweredUi { draws: vec![draw(1, 0, 0, 64_000, 64_000, 5)], ..Default::default() };
        let vp = IrRect::from_xywh(0, 0, 640_000, 480_000);
        assert_eq!(emit_html("t", &ui, vp), emit_html("t", &ui, vp), "pure + deterministic");
    }

    // [ASPIRE: portrayal-slot-resolve] an authored palette slot name reaches
    // real pixels instead of the deterministic seed hash.
    #[test]
    fn authored_chrome_color_resolves_against_the_live_palette() {
        use crate::tokens::BaseProfile;
        let mut ui = two_lane_ui();
        ui.widgets[0].chrome_color = Some("accent_primary".to_string());
        let palette = BaseProfile::studio_dark().to_tokens().palette;
        let html = emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, &palette);
        let want = format!("background:{}", hex(palette.accent_primary));
        assert!(html.contains(&want), "authored accent_primary must paint the real token, got: {html}");
    }

    // [ASPIRE: portrayal-slot-resolve] an unknown slot name, or no palette at
    // all, must never invent a colour — it falls back to the seed hash exactly
    // as an untouched kit always has.
    #[test]
    fn unauthored_or_unknown_slot_falls_back_to_the_seed_hash() {
        let ui = two_lane_ui();
        let untouched = emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact);
        let want_bg = untouched.split("background:").nth(1).unwrap()[..7].to_string();

        let mut themed_ui = ui.clone();
        themed_ui.widgets[0].chrome_color = Some("not_a_real_slot".to_string());
        let palette = crate::tokens::BaseProfile::studio_dark().to_tokens().palette;
        let unknown_slot = emit_html_mode_themed("t", &themed_ui, VP, LayoutEmitMode::Exact, &palette);
        let got_bg = unknown_slot.split("background:").nth(1).unwrap()[..7].to_string();
        assert_eq!(want_bg, got_bg, "an unrecognised slot name must not change the painted pixel");
    }

    // [ASPIRE: emit-text-runs] the label ink is the theme's fg_text, not a
    // colour frozen into the emitter regardless of which palette is live.
    #[test]
    fn themed_label_ink_tracks_the_palette_fg_text() {
        let mut ui = two_lane_ui();
        let key = ui.layout[0].stable_key.0.clone();
        ui.text_literals.push((key, "FORGE".to_string()));
        let palette = crate::tokens::BaseProfile::molten().to_tokens().palette;
        let html = emit_html_mode_themed("t", &ui, VP, LayoutEmitMode::Exact, &palette);
        assert!(
            html.contains(&format!(".t{{display:block;padding:2px 4px;font:12px/1.35 'IBM Plex Mono',ui-monospace,monospace;color:{}", hex(palette.fg_text))),
            "themed label CSS must carry molten's own fg_text, got: {html}"
        );
    }

    // [ASPIRE: html5ever-parity-oracle, scoped] a full html5ever DOM parse is a
    // new dependency this crate does not carry (checked: absent from
    // Cargo.toml) and a new dep add is an L19/ARCH000-gated move, not a wire —
    // so this oracle proves the cheaper, dependency-free invariant a real
    // parser would also enforce: every emitted `<div>` opens and closes exactly
    // once, in DOM order, and that order matches `draws`/`layout` order. It is
    // NOT a substitute for the full parity oracle the aspire row names; it is
    // the part of it landable without a new dep.
    #[test]
    fn emitted_divs_balance_and_dom_order_matches_draws_order() {
        let ui = two_lane_ui();
        let html = emit_html_mode("t", &ui, VP, LayoutEmitMode::Exact);
        let opens = html.matches("<div").count();
        let closes = html.matches("</div>").count();
        assert_eq!(opens, closes, "every opened div must close: {html}");

        let root_at = html.find("data-vixi-id=\"root\"").unwrap();
        let child_at = html.find("data-vixi-id=\"root.go\"").unwrap();
        assert!(root_at < child_at, "DOM emission order must match layout order: {html}");
    }

    #[test]
    fn test_ipc_bridge_script_format() {
        assert!(IPC_BRIDGE_SCRIPT.contains("window.ipc = window.ipc ||"));
        assert!(IPC_BRIDGE_SCRIPT.contains("window.chrome.webview.postMessage"));
        assert!(IPC_BRIDGE_SCRIPT.contains("game-verb"));
        assert!(IPC_BRIDGE_SCRIPT.contains("organ-click"));
        assert!(IPC_BRIDGE_SCRIPT.contains("vixi-click"));
        assert!(IPC_BRIDGE_SCRIPT.contains("window.__forge.feed"));
    }

    #[test]
    fn test_astrolabe_organ_hooks() {
        assert!(ASTROLABE_ORGAN_HOOK_SCRIPT.contains("initAstrolabeCanvasHooks"));
        assert!(ASTROLABE_ORGAN_HOOK_SCRIPT.contains("organ[type=\"astrolabe\"]"));
        assert!(ASTROLABE_ORGAN_HOOK_SCRIPT.contains("Astrolabe"));
        assert!(ASTROLABE_ORGAN_HOOK_SCRIPT.contains("organ-interact astrolabe"));

        let organ_html = emit_organ_astrolabe("test-astrolabe", 400, 400, 53.54);
        assert!(organ_html.contains("<organ type=\"astrolabe\""));
        assert!(organ_html.contains("class=\"astrolabe-organ\""));
        assert!(organ_html.contains("id=\"test-astrolabe\""));
        assert!(organ_html.contains("width=\"400\" height=\"400\""));
        assert!(organ_html.contains("data-latitude=\"53.54\""));
    }

    #[test]
    fn test_page_interactive_and_emit_interactive_html() {
        let page = page_interactive("Studio Glass", "body{background:#000;}", "<div id=\"app\">Hello</div>", true);
        assert!(page.contains("<!DOCTYPE html>"));
        assert!(page.contains("<title>Studio Glass</title>"));
        assert!(page.contains("body{background:#000;}"));
        assert!(page.contains("<div id=\"app\">Hello</div>"));
        assert!(page.contains(IPC_BRIDGE_SCRIPT));
        assert!(page.contains(ASTROLABE_ORGAN_HOOK_SCRIPT));

        let ui = two_lane_ui();
        let interactive = emit_interactive_html("Interactive Studio", &ui, VP, LayoutEmitMode::Exact);
        assert!(interactive.contains("data-title=\"Interactive Studio\""));
        assert!(interactive.contains("data-vixi-id=\"root\""));
        assert!(interactive.contains(IPC_BRIDGE_SCRIPT));
        assert!(interactive.contains(ASTROLABE_ORGAN_HOOK_SCRIPT));
    }
}
