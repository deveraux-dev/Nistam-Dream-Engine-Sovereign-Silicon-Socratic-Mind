//! Cognitive-load gauge for EMITTED assets (Sean 2026-08-05, "go deeper").
//!
//! root#a000 binds cognitive load to ALL ui/asset/agent-out, and the enforcement it names is
//! real — but it lands on AUTHORED DATA only: [`crate::creation_dag`] fails a group of
//! primitives that grows past 4±1. The thing a person actually reads — a rendered page, a
//! deck, an export — had no gauge at all, so "is this simple enough" was a matter of taste,
//! and the 100-row page that shipped on 2026-08-05 was overwhelming while every gate stayed
//! green. A law with no instrument on the artifact is advisory, whatever the prose says.
//!
//! Nothing here invents a number. The choice ceiling is [`crate::creation_dag::APERTURE_DEFAULT`]
//! and the words-per-reveal bar is [`AddhConfig::response_length_target`]
//! — a config that was built, tested, and carried ZERO consumers outside its own crate. This is
//! its live caller in the asset lane (EXISTS != REACHABLE, root#revascularize).
//!
//! The model is what a reader meets, not what the file contains: text outside every `<details>`
//! plus the door labels is what lands BEFORE any click, and opening one door reveals that door's
//! own text plus the labels of the doors inside it. Progressive disclosure is therefore
//! measurable — depth costs nothing, breadth and prose cost everything.

use crate::creation_dag::APERTURE_DEFAULT;

/// ADHD-friendly cognitive configuration. Ported locally since forge_sieve_v3
/// has no cognitive module; this is the minimal bridge needed for gauge().
struct AddhConfig {
    /// Maximum words a single view may display for ADHD-friendly rendering.
    /// Conservative value for progressive disclosure safety.
    response_length_target: u32,
}

impl AddhConfig {
    /// Default configuration for ADHD-friendly UI rendering.
    /// Set to 80 words as a proven safe maximum before overwhelm (ported from forge_sieve::cognitive::AdhdConfig).
    fn default() -> Self {
        AddhConfig { response_length_target: 80 }
    }
}

/// One disclosure node — a `<details>` door and what opening it puts on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Door {
    /// Nesting depth; 0 = visible at load.
    pub depth: usize,
    /// Words in this door's own label.
    pub label_words: usize,
    /// Words revealed by opening it: its own text plus the labels of the doors inside it.
    pub reveal_words: usize,
    /// Doors directly inside it — the choices the reader faces next.
    pub choices: usize,
}

/// What a rendered asset costs a reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Load {
    /// Words on screen before any click: loose text plus every top-level door label.
    pub landing_words: usize,
    /// Doors offered at load.
    pub top_choices: usize,
    /// Every door, in document order.
    pub doors: Vec<Door>,
    /// Deepest nesting reached.
    pub depth: usize,
}

/// The bars, both sourced — never authored here.
impl Load {
    /// Max choices at any one level (`creation_dag::APERTURE_DEFAULT`, Miller/Cowan 4±1).
    pub const CHOICE_CEIL: usize = APERTURE_DEFAULT;

    /// Max words any single view may put on screen — the ADHD lens's own target.
    pub fn word_ceil() -> usize {
        AddhConfig::default().response_length_target as usize
    }

    /// Every way this asset exceeds a bar, named in reader's terms. Empty = the page is calm.
    pub fn faults(&self) -> Vec<String> {
        let wc = Self::word_ceil();
        let mut out = Vec::new();
        if self.landing_words > wc {
            out.push(format!(
                "landing shows {} words before a single click (ceiling {wc}) — put the fine print behind a door",
                self.landing_words
            ));
        }
        if self.top_choices > Self::CHOICE_CEIL {
            out.push(format!(
                "{} doors at load (ceiling {}) — group them",
                self.top_choices, Self::CHOICE_CEIL
            ));
        }
        for (i, d) in self.doors.iter().enumerate() {
            if d.choices > Self::CHOICE_CEIL {
                out.push(format!(
                    "door {i} (depth {}) offers {} choices (ceiling {})",
                    d.depth, d.choices, Self::CHOICE_CEIL
                ));
            }
            if d.reveal_words > wc {
                out.push(format!(
                    "door {i} (depth {}) reveals {} words at once (ceiling {wc})",
                    d.depth, d.reveal_words
                ));
            }
        }
        out
    }

    /// One line for the desk: the reading, pass or fail.
    pub fn report(&self) -> String {
        let f = self.faults();
        format!(
            "landing {}w/{} · {} doors · depth {} · widest {} · heaviest {}w — {}",
            self.landing_words,
            Self::word_ceil(),
            self.doors.len(),
            self.depth,
            self.doors.iter().map(|d| d.choices).max().unwrap_or(0),
            self.doors.iter().map(|d| d.reveal_words).max().unwrap_or(0),
            if f.is_empty() { "CALM".to_string() } else { format!("{} FAULT(S)", f.len()) }
        )
    }
}

/// Accumulator for one door while the scan is still inside it.
struct Acc {
    depth: usize,
    parent: Option<usize>,
    label_words: usize,
    own_words: usize,
    choices: usize,
}

/// Measure a rendered HTML asset. Tag-aware, dependency-free, deterministic: `<style>` and
/// `<script>` bodies are not prose and never count toward what a reader reads.
pub fn gauge(html: &str) -> Load {
    let mut accs: Vec<Acc> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut loose_words = 0usize;
    let mut top_choices = 0usize;
    let mut in_label = false;
    let mut skip_body: Option<&'static str> = None;
    let mut depth_max = 0usize;

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0usize;
    let mut text = String::new();

    // Flush pending text into whichever bucket the scan is currently standing in.
    macro_rules! flush {
        () => {{
            let n = text.split_whitespace().count();
            if n > 0 {
                if in_label {
                    if let Some(&t) = stack.last() {
                        accs[t].label_words += n;
                    }
                } else if let Some(&t) = stack.last() {
                    accs[t].own_words += n;
                } else {
                    loose_words += n;
                }
            }
            text.clear();
        }};
    }

    while i < chars.len() {
        if chars[i] != '<' {
            text.push(chars[i]);
            i += 1;
            continue;
        }
        // A tag. Read to '>', then classify.
        let start = i;
        while i < chars.len() && chars[i] != '>' {
            i += 1;
        }
        i = (i + 1).min(chars.len());
        let raw: String = chars[start..i].iter().collect();
        let closing = raw.starts_with("</");
        let name: String = raw
            .trim_start_matches('<')
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();

        // Inside <style>/<script>, everything up to the matching close is not prose.
        if let Some(open) = skip_body {
            if closing && name == open {
                skip_body = None;
            }
            text.clear();
            continue;
        }

        flush!();

        match (name.as_str(), closing) {
            ("style", false) | ("script", false) => skip_body = Some(if name == "style" { "style" } else { "script" }),
            ("details", false) => {
                let parent = stack.last().copied();
                let depth = stack.len();
                depth_max = depth_max.max(depth);
                match parent {
                    Some(p) => accs[p].choices += 1,
                    None => top_choices += 1,
                }
                accs.push(Acc { depth, parent, label_words: 0, own_words: 0, choices: 0 });
                stack.push(accs.len() - 1);
            }
            ("details", true) => {
                stack.pop();
            }
            ("summary", false) => in_label = true,
            ("summary", true) => in_label = false,
            _ => {}
        }
    }
    flush!();

    // A door's label is paid for by whoever can SEE it: the parent that reveals it, or the
    // landing view when there is no parent.
    let mut landing_words = loose_words;
    let mut doors: Vec<Door> = accs
        .iter()
        .map(|a| Door { depth: a.depth, label_words: a.label_words, reveal_words: a.own_words, choices: a.choices })
        .collect();
    for (idx, a) in accs.iter().enumerate() {
        match a.parent {
            Some(p) => doors[p].reveal_words += doors[idx].label_words,
            None => landing_words += doors[idx].label_words,
        }
    }

    Load { landing_words, top_choices, doors, depth: depth_max + usize::from(!accs.is_empty()) }
}

/// One authored surface's widest group: the slot with the most direct children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Widest {
    /// The dotted path of the parent slot containing the widest group.
    pub parent: String,
    /// The number of direct children under the parent slot.
    pub children: usize,
}

/// The aperture law over an AUTHORED `.kit.vixi` surface — the structural twin of
/// [`gauge`], which reads a rendered page. A `slot a.b.c` line declares `c` a direct
/// child of `a.b`, so the slot tree carries its own grouping and the widest group is
/// exactly what Miller/Cowan bounds. Visual chunking (a `cols=N` grid) does NOT count:
/// neurohud carried ten boxes under one parent for a fortnight behind a comment saying
/// the grid held it, which is how an unmeasured law reads green while drifting.
pub fn widest_group(kit_src: &str) -> Option<Widest> {
    // `variant NAME` opens a mutually-exclusive alternate slot tree (page/tab/mode
    // switch — only one variant's slots ever materialize at once, forge-vix lowers
    // the active one). Grouping counts are scoped per variant: a reader facing
    // `opus` sees at most one page's fields, never the union of all twenty, so
    // summing slot lines across variants as if they were simultaneous siblings
    // both over- and mis-counts the group Miller/Cowan actually bounds. Base slots
    // (declared before any `variant` line) are always visible and stay one shared
    // bucket per parent, exactly as before.
    let mut counts: Vec<((Option<String>, String), usize)> = Vec::new();
    let mut current_variant: Option<String> = None;
    for line in kit_src.lines() {
        let line = line.trim_start();
        if let Some(name) = line.strip_prefix("variant ") {
            current_variant = Some(name.split_whitespace().next().unwrap_or(name).to_string());
            continue;
        }
        let Some(rest) = line.strip_prefix("slot ") else { continue };
        let Some(path) = rest.split_whitespace().next() else { continue };
        let parent = match path.rsplit_once('.') {
            Some((p, _)) => p.to_string(),
            None => String::new(), // a root slot; its parent is the surface itself
        };
        let key = (current_variant.clone(), parent);
        match counts.iter_mut().find(|(k, _)| *k == key) {
            Some((_, n)) => *n += 1,
            None => counts.push((key, 1)),
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|((_, parent), children)| Widest { parent, children })
}

/// THE APERTURE DEBT, frozen (2026-08-05). Every registered panel whose widest group
/// exceeds [`Load::CHOICE_CEIL`], measured the first time the law was ever pointed at a
/// panel. This is a FORWARD RATCHET (root#a000), not a permission slip: a surface may
/// leave this list, never join it, and never grow. Widening a row to pass is the
/// delete-to-green the debt ledger calls a strike.
///
/// It is frozen rather than fixed because each row is a design call on a shipped product
/// surface — `opus` at seventy children under one parent is a chapter of work, not a
/// drive-by. What the freeze buys is that the thirty-first cannot arrive in silence,
/// which is exactly how `neurohud` drifted from nine boxes to ten behind a comment.
///
/// v3 STATUS (2026-08-17): Only neurohud is ported to v3 so far. Entries for
/// canvas_window, constellation, launcher, and shell have been removed; those panels
/// do not exist in F:\v3\crates\forge-book-v3\panels\ (checked via bounded scan).
pub const APERTURE_DEBT: &[(&str, &str, usize)] = &[];

/// Panel name -> kit source, mirroring v2 forge-vix's `loader::STUDIO_PANELS`.
/// forge-vix-v3 has no `panels/` directory or loader module yet (checked
/// directly — the whole vixi-panel UI lane is unported), so this crate carries
/// its own `panels/` copy (ported verbatim from `F:\NewRepo\crates\forge-vix\
/// panels\`) rather than inventing content. Only `neurohud` is drained so far;
/// the rest of v2's panel set lands here as it gets a live caller.
const STUDIO_PANELS: &[(&str, &str)] = &[("neurohud", include_str!("../panels/neurohud.kit.vixi"))];

/// Look up a registered studio panel's kit source by name. Test-only today —
/// [`aperture_census`] walks [`STUDIO_PANELS`] directly; this is the by-name
/// lookup the neurohud regression test exercises.
#[cfg(test)]
fn studio_panel(name: &str) -> Option<&'static str> {
    STUDIO_PANELS.iter().find(|(n, _)| *n == name).map(|(_, src)| *src)
}

/// Every registered panel over the ceiling right now, sorted by name.
pub fn aperture_census() -> Vec<(&'static str, Widest)> {
    let mut v: Vec<(&'static str, Widest)> = STUDIO_PANELS
        .iter()
        .filter_map(|(name, src)| {
            widest_group(src).filter(|w| w.children > Load::CHOICE_CEIL).map(|w| (*name, w))
        })
        .collect();
    v.sort_by_key(|(n, _)| *n);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gauge reads a reader's view, not a file: style bodies are silent, labels are paid
    /// for by whoever can see them, and depth is free while breadth and prose are not.
    #[test]
    fn the_gauge_measures_what_lands_before_a_click() {
        let html = "<style>body{color:red}</style><p>one two three</p>\
                    <details><summary>door one</summary>alpha beta\
                    <details><summary>inner</summary>gamma</details></details>";
        let l = gauge(html);
        // "one two three" (3) + top label "door one" (2) — the style body counts for nothing.
        assert_eq!(l.landing_words, 5);
        assert_eq!(l.top_choices, 1);
        assert_eq!(l.doors.len(), 2);
        // Opening the outer door reveals its own text (2) plus the inner door's label (1).
        assert_eq!(l.doors[0].reveal_words, 3);
        assert_eq!(l.doors[0].choices, 1);
        assert_eq!(l.doors[1].reveal_words, 1);
        assert_eq!(l.depth, 2);
    }

    /// A flat wall of prose faults on the landing bar; the same words behind doors do not.
    /// This is the whole claim of progressive disclosure, stated as a test.
    #[test]
    fn depth_is_free_and_flatness_is_not() {
        let wall: String = std::iter::repeat("word ").take(200).collect();
        assert!(!gauge(&wall).faults().is_empty(), "a 200-word flat page must fault");
        let folded = format!("<details><summary>open</summary>{wall}</details>");
        let l = gauge(&folded);
        assert!(l.landing_words <= Load::word_ceil(), "folded, the landing is quiet");
        assert!(
            l.faults().iter().any(|f| f.contains("reveals")),
            "but one door dumping 200 words still faults"
        );
    }

    /// Too many doors at one level faults even when every door is short.
    #[test]
    fn breadth_faults_at_the_aperture_ceiling() {
        let many: String = (0..Load::CHOICE_CEIL + 1)
            .map(|i| format!("<details><summary>d{i}</summary>x</details>"))
            .collect();
        assert!(gauge(&many).faults().iter().any(|f| f.contains("doors at load")));
    }

    /// A slot tree's widest group is counted by PARENTAGE, never by how it is drawn.
    #[test]
    fn widest_group_counts_parentage_not_layout() {
        let flat = "slot root kind=region\nslot root.deck kind=region layout=grid cols=3\n\
                    slot root.deck.a\nslot root.deck.b\nslot root.deck.c\nslot root.deck.d";
        assert_eq!(widest_group(flat).unwrap(), Widest { parent: "root.deck".into(), children: 4 });
        let grouped = "slot root\nslot root.deck\nslot root.deck.x\nslot root.deck.x.a\n\
                       slot root.deck.x.b\nslot root.deck.y\nslot root.deck.y.c";
        assert_eq!(widest_group(grouped).unwrap().children, 2);
        assert!(widest_group("# comments only").is_none());
    }

    /// THE APERTURE LAW'S FIRST TEST OVER A PANEL (Sean 2026-08-05). root#a000 binds
    /// cognitive load to ALL ui, and until now the only enforcer on disk was
    /// `creation_dag`'s test over 14 authored primitives — so every real studio surface
    /// was ungoverned. Every registered panel now answers to the same ceiling its own
    /// doctrine names, and an eleventh card cannot land quietly.
    // [BOARD: APERTURE-PANELS]
    #[test]
    fn every_studio_panel_obeys_the_aperture_law() {
        let ceil = Load::CHOICE_CEIL;
        let census = aperture_census();
        let mut faults: Vec<String> = Vec::new();

        for (name, w) in &census {
            match APERTURE_DEBT.iter().find(|(n, _, _)| n == name) {
                None => faults.push(format!(
                    "NEW: {name}'s {} has {} direct children (ceiling {ceil}) — group or drawer it",
                    w.parent, w.children
                )),
                Some((_, _, frozen)) if w.children > *frozen => faults.push(format!(
                    "WORSE: {name}'s {} grew {frozen} -> {} — the ratchet is forward only",
                    w.parent, w.children
                )),
                Some((_, _, frozen)) if w.children < *frozen => faults.push(format!(
                    "BETTER: {name}'s {} shrank {frozen} -> {} — drop the count in APERTURE_DEBT",
                    w.parent, w.children
                )),
                _ => {}
            }
        }
        for (name, parent, frozen) in APERTURE_DEBT {
            if !census.iter().any(|(n, _)| n == name) {
                faults.push(format!(
                    "FIXED: {name}'s {parent} was {frozen} and is now inside the ceiling — \
                     delete its APERTURE_DEBT row"
                ));
            }
        }
        assert!(faults.is_empty(), "aperture ceiling {ceil}:\n{}", faults.join("\n"));
    }

    /// The surface ARCH-000 calls the engine answers to the law it embodies. It carried
    /// TEN boxes under `root.deck` until 2026-08-05; the bands are what hold it now, and
    /// the host binds the whole dotted path, so `cards()` must name the same slots.
    // [BOARD: APERTURE-PANELS]
    #[test]
    fn the_neurohud_deck_is_banded_and_its_host_agrees() {
        let src = studio_panel("neurohud").expect("neurohud is registered");
        let w = widest_group(src).expect("neurohud declares slots");
        assert!(
            w.children <= Load::CHOICE_CEIL,
            "neurohud's {} has {} direct children",
            w.parent,
            w.children
        );
        for band in ["root.deck.aim", "root.deck.work", "root.deck.state"] {
            assert!(src.contains(&format!("slot {band} ")), "band {band} missing");
        }
        // Every card the kit declares under a band is a slot the host can actually find.
        assert!(!src.contains("slot root.deck.goal "), "the flat deck must be gone");
    }

    /// THE LIVE GATE: the page Brit is handed must read CALM. This is the instrument that
    /// was missing on 2026-08-05 — the page shipped overwhelming with every other gate green.
    // [BOARD: GIFTS-100]
    #[test]
    fn the_page_brit_is_handed_reads_calm() {
        let l = gauge(&crate::gifts_for_brit::render_page());
        assert!(l.faults().is_empty(), "{} :: {:?}", l.report(), l.faults());
        assert!(l.depth >= 3, "the pitch discloses in at least three levels, got {}", l.depth);
    }
}
