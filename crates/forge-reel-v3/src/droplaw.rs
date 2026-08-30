//! The Drop Law compiler, v3. Ported from v2's `youtube-forge` bridge
//! (`F:\NewRepo\tools\youtube-forge\tools\youtube\drop_law_compiler.py`,
//! spec: `F:\NewRepo\drop_law_system_guide.pdf`) -- a deterministic pacing
//! analyzer for short-form video scripts: parses a plain-text shot script
//! into timed frames, then checks it against six empirically-sourced
//! pacing constraints (dwell floors, Attentional Blink spacing, McCloud
//! transition ratios, Kishotenketsu structure, caption reading speed,
//! Saga Litotes). No render, no I/O -- pure text in, report text out.
//!
//! `no unsafe · no f32/f64 · no regex` (CLAUDE.md forbidden_ops): every
//! duration is carried in tenths-of-a-millisecond (`u32`) to keep the
//! source compiler's one fractional constant (dialogue dwell = 834.4ms)
//! exact without a float, every percentage is rendered from an integer
//! numerator/denominator pair, and the script grammar (`[key]`, `{trans}`,
//! `(Role: P)`) is walked with `.find`/byte-slicing rather than a regex
//! engine. `[ASSUMED]` word-boundary = ASCII alphanumeric/underscore runs,
//! matching Python's `\w+` closely enough for the English-language shot
//! scripts this format targets; a script with non-ASCII dialogue words
//! would undercount, unverified against v2 since no such fixture exists.

/// One parsed shot. `duration_x10_ms` and `stakes_x10` are fixed-point
/// (tenths) so the module carries zero floats end to end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// 1-indexed source line the frame was parsed from.
    pub line_num: u32,
    /// The most recent `# header` line above this frame, or `"Unknown"`.
    pub section: String,
    /// The `[type]` tag -- drives the dwell floor.
    pub frame_type: FrameType,
    /// The Cohn grammar role (`(Role: X)`, or deduced from `frame_type`).
    pub role: CohnRole,
    /// The `{transition}` tag, or `ActionToAction` if absent.
    pub transition: Transition,
    /// Visual description with all tags stripped.
    pub description: String,
    /// `(Dialogue: "...")` contents, empty if absent.
    pub dialogue: String,
    /// `(Text: "...")` on-screen caption contents, empty if absent.
    pub text: String,
    /// Dwell floor in tenths-of-a-millisecond, from `frame_type`.
    pub duration_x10_ms: u32,
    /// `duration_x10_ms` at the compiler's `fps`, rounded.
    pub frames: u32,
    /// `(Stakes: N)` in tenths, defaulting to `10` (1.0).
    pub stakes_x10: u32,
}

/// The `[type]` shot tag -- selects the dwell floor a frame must hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// 2000ms establishing shot -- grounds geography.
    Establish,
    /// 2000ms empty transition shot -- no people, no plot.
    Pillow,
    /// 834.4ms dialogue shot (memory floor + handle time).
    Dialogue,
    /// 100ms throwaway/action transition frame.
    Motion,
    /// 501ms core narrative plot frame -- also the fallback for any
    /// unrecognized `[type]` tag.
    Key,
}

impl FrameType {
    fn parse(s: &str) -> Self {
        match s {
            "establish" => Self::Establish,
            "pillow" => Self::Pillow,
            "dialogue" => Self::Dialogue,
            "motion" => Self::Motion,
            _ => Self::Key, // unrecognized types default to `key` (v2 parity)
        }
    }

    /// Dwell floor in tenths-of-ms. `T_subliminal=13 T_motion=100
    /// T_memory=501` from the verified-constraints header; dialogue adds
    /// the 333.4ms handle constant on top of the memory floor.
    pub const fn dwell_x10_ms(self) -> u32 {
        match self {
            Self::Establish | Self::Pillow => 20_000,
            Self::Dialogue => 8_344, // 501 + 333.4
            Self::Motion => 1_000,
            Self::Key => 5_010,
        }
    }
}

/// Cohn's visual narrative grammar role -- an arc must contain a `Peak`,
/// never open on one, and never close on an `Initial`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CohnRole {
    /// `E` -- sets the initial space, world, or character.
    Establisher,
    /// `I` -- launches the action or sets physical trajectory.
    Initial,
    /// `P` -- the core event, impact, or visceral reveal. Mandatory per arc.
    Peak,
    /// `R` -- visual relief, consequence, or secondary reaction; closes an arc.
    Release,
}

impl CohnRole {
    fn parse(c: char) -> Option<Self> {
        match c {
            'E' => Some(Self::Establisher),
            'I' => Some(Self::Initial),
            'P' => Some(Self::Peak),
            'R' => Some(Self::Release),
            _ => None,
        }
    }

    const fn letter(self) -> char {
        match self {
            Self::Establisher => 'E',
            Self::Initial => 'I',
            Self::Peak => 'P',
            Self::Release => 'R',
        }
    }
}

/// The `{transition}` shot tag -- McCloud's panel-transition taxonomy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    /// McCloud action-to-action -- target ~65% of all transitions.
    ActionToAction,
    /// McCloud subject-to-subject -- target ~20%.
    SubjectToSubject,
    /// McCloud scene-to-scene -- target ~15%.
    SceneToScene,
    /// Any other named transition (`moment_to_moment`, `aspect_to_aspect`,
    /// `non_sequitur`, ...), carried verbatim.
    Other(String),
}

impl Transition {
    fn parse(s: &str) -> Self {
        match s {
            "action_to_action" => Self::ActionToAction,
            "subject_to_subject" => Self::SubjectToSubject,
            "scene_to_scene" => Self::SceneToScene,
            other => Self::Other(other.to_string()),
        }
    }
}

/// fps a script compiles at. v2 default is 24; kept explicit rather than
/// a hardcoded constant since a caller may target a different reel rate.
pub struct DropLawCompiler {
    fps: u32,
    /// Frames parsed so far, in script order.
    pub frames: Vec<Frame>,
}

/// The compiled analysis: report text plus the two severities that gate
/// the downstream pipeline (droplaw.py halts on any `criticals`).
pub struct Analysis {
    /// The full markdown report, matching v2's `droplaw-report.txt` shape.
    pub report: String,
    /// Hard failures -- a caller should halt the render pipeline on any.
    pub criticals: Vec<String>,
    /// Soft drift from a target ratio; does not block rendering.
    pub warnings: Vec<String>,
    /// Total script duration, tenths-of-a-millisecond.
    pub total_x10_ms: u32,
    /// Total frame count at the compiler's `fps`.
    pub total_frames: u32,
}

/// Balanced-ternary attention verdict: `-1`/`0`/`+1`, matching this
/// repo's own TritCell5D substrate (CLAUDE.md T1: v3 is "5D-TritCell/
/// balanced-ternary") rather than inventing a new scale. Reduces
/// [`Analysis`]'s own already-measured `criticals`/`warnings` -- real
/// signal from this script's actual dwell floors, Attentional Blink
/// spacing, and WPM/ratio drift -- into three states, no fabricated
/// engagement or network data involved:
/// - `-1` Subcritical: no hazard, no drift. Clean, but also nothing held
///   long enough to register as a landed moment -- safe and inert.
/// - `0` Critical: no hard hazard, but a soft ratio/WPM drift exists.
///   The only state still doing something -- balanced, watchful.
/// - `+1` Supercritical: a real hazard fired (Attentional Blink Hazard,
///   over-350-WPM captions, or broken Cohn grammar) -- the pacing
///   genuinely outran what the dwell floors say a viewer can retain.
pub fn attention_trit(analysis: &Analysis) -> i8 {
    if !analysis.criticals.is_empty() {
        1
    } else if !analysis.warnings.is_empty() {
        0
    } else {
        -1
    }
}

impl DropLawCompiler {
    /// A compiler targeting `fps` frames per second.
    pub const fn new(fps: u32) -> Self {
        Self { fps, frames: Vec::new() }
    }

    /// Build a compiler from pre-constructed frames (used by adapters).
    pub fn from_frames(fps: u32, frames: Vec<Frame>) -> Self {
        Self { fps, frames }
    }

    /// Parses shot-script text into `self.frames`. Byte-slice/`.find`
    /// scanning only -- no regex (CLAUDE.md forbidden_ops).
    pub fn parse_script(&mut self, script_text: &str) {
        let mut current_section = String::from("Unknown");

        for (idx, raw_line) in script_text.lines().enumerate() {
            let line_num = (idx + 1) as u32;
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(rest) = line.strip_prefix('#') {
                current_section = rest.trim_start_matches('#').trim().to_string();
                continue;
            }

            let Some((frame_type_raw, bracket_start, bracket_end)) = find_delim(line, '[', ']')
            else {
                continue;
            };
            let frame_type = FrameType::parse(&frame_type_raw.to_lowercase());

            let mut desc = String::new();
            desc.push_str(&line[..bracket_start]);
            desc.push_str(&line[bracket_end + 1..]);
            let mut desc = desc.trim().to_string();

            let transition = if let Some((raw, s, e)) = find_delim(&desc, '{', '}') {
                let t = Transition::parse(&raw.to_lowercase());
                desc = remove_span(&desc, s, e);
                t
            } else {
                Transition::ActionToAction
            };

            let dialogue = extract_quoted_tag(&mut desc, "Dialogue").unwrap_or_default();
            let text = extract_quoted_tag(&mut desc, "Text").unwrap_or_default();

            let role = extract_simple_tag(&mut desc, "Role")
                .and_then(|r| r.chars().next())
                .and_then(CohnRole::parse)
                .unwrap_or(match frame_type {
                    FrameType::Establish => CohnRole::Establisher,
                    FrameType::Dialogue | FrameType::Motion => CohnRole::Initial,
                    FrameType::Pillow => CohnRole::Release,
                    FrameType::Key => CohnRole::Peak,
                });

            let stakes_x10 = extract_simple_tag(&mut desc, "Stakes")
                .and_then(|s| parse_fixed_x10(&s))
                .unwrap_or(10); // default 1.0

            let duration_x10_ms = frame_type.dwell_x10_ms();
            // round(dur_ms * fps / 1000) done in tenths: round(dur_x10*fps/10000)
            let frames_count = round_div(duration_x10_ms as u64 * self.fps as u64, 10_000);

            self.frames.push(Frame {
                line_num,
                section: current_section.clone(),
                frame_type,
                role,
                transition,
                description: desc.trim().to_string(),
                dialogue,
                text,
                duration_x10_ms,
                frames: frames_count as u32,
                stakes_x10,
            });
        }
    }

    /// Runs all six Drop Law checks and renders the markdown report.
    pub fn analyze(&self) -> Analysis {
        let mut report = Vec::new();
        let mut criticals = Vec::new();
        let mut warnings = Vec::new();

        // 1. Timing metrics.
        let total_x10_ms: u32 = self.frames.iter().map(|f| f.duration_x10_ms).sum();
        let total_frames: u32 = self.frames.iter().map(|f| f.frames).sum();
        report.push("### TIMING METRICS".to_string());
        report.push(format!(
            "- **Total Duration:** {} seconds ({} ms)",
            fmt_seconds(total_x10_ms),
            fmt_ms(total_x10_ms)
        ));
        report.push(format!(
            "- **Frame Count:** {total_frames} frames (at {} fps)",
            self.fps
        ));
        report.push(format!("- **Shot Count:** {}", self.frames.len()));
        report.push(String::new());

        // 2. Cohn arc grammar.
        report.push("### COHN ARC GRAMMAR SYSTEM CHECK".to_string());
        let mut arcs: Vec<Vec<CohnRole>> = Vec::new();
        let mut current: Vec<CohnRole> = Vec::new();
        for (i, f) in self.frames.iter().enumerate() {
            current.push(f.role);
            if f.role == CohnRole::Release || i == self.frames.len() - 1 {
                arcs.push(std::mem::take(&mut current));
            }
        }
        for (a_idx, arc) in arcs.iter().enumerate() {
            let seq: String = arc
                .iter()
                .map(|r| r.letter().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            let has_peak = arc.contains(&CohnRole::Peak);
            let starts_peak = arc.first() == Some(&CohnRole::Peak);
            let ends_initial = arc.last() == Some(&CohnRole::Initial);
            let is_valid = has_peak && !starts_peak && !ends_initial;
            if is_valid {
                report.push(format!("- **Arc {}:** Sequence `{seq}` ✅ VALID", a_idx + 1));
            } else {
                let mut err = format!("Broken Cohn visual grammar sequence: `{seq}`.");
                if !has_peak {
                    err.push_str(" (Missing Peak 'P').");
                }
                if starts_peak {
                    err.push_str(" (Peak 'P' placed at start).");
                }
                if ends_initial {
                    err.push_str(" (Initial 'I' placed at end).");
                }
                report.push(format!(
                    "- **Arc {}:** Sequence `{seq}` ❌ BROKEN - {err}",
                    a_idx + 1
                ));
                criticals.push(format!(
                    "Broken Visual Grammar in Arc {}: sequence is {:?}.",
                    a_idx + 1,
                    arc.iter().map(|r| r.letter()).collect::<Vec<_>>()
                ));
            }
        }
        report.push(String::new());

        // 3. Attentional blink spacing (dwell in (0, 500ms] between two
        // "key" frame types -- establish/dialogue/key, not pillow/motion).
        report.push("### ATTENTIONAL BLINK (200-500ms) SCANNER".to_string());
        let mut blink_violation = false;
        for i in 0..self.frames.len().saturating_sub(1) {
            let f1 = &self.frames[i];
            let f2 = &self.frames[i + 1];
            let is_key = |t: FrameType| {
                matches!(t, FrameType::Key | FrameType::Dialogue | FrameType::Establish)
            };
            if is_key(f1.frame_type) && is_key(f2.frame_type) {
                let dt = f1.duration_x10_ms;
                let valid = dt == 0 || dt > 5_000;
                if !valid {
                    blink_violation = true;
                    report.push(format!(
                        "- ❌ **Blink Hazard:** Frame {} to {} is spaced at {} ms. Second frame will be dropped by visual cortex.",
                        i + 1, i + 2, fmt_ms(dt)
                    ));
                    criticals.push(format!(
                        "Attentional Blink Hazard between Frame {} and {} ({} ms dwell).",
                        i + 1, i + 2, fmt_ms(dt)
                    ));
                }
            }
        }
        if !blink_violation {
            report.push("- ✅ **All key frame transitions are outside the Attentional Blink window (200-500 ms).** Memory retention is high.".to_string());
        }
        report.push(String::new());

        // 4. McCloud transition ratios.
        report.push("### MCCLOUD TRANSITION PROBABILITY ANALYSIS".to_string());
        let total_transitions = self.frames.len().saturating_sub(1) as u32;
        let mut action = 0u32;
        let mut subject = 0u32;
        let mut scene = 0u32;
        let mut other = 0u32;
        for f in self.frames.iter().take(total_transitions as usize) {
            match &f.transition {
                Transition::ActionToAction => action += 1,
                Transition::SubjectToSubject => subject += 1,
                Transition::SceneToScene => scene += 1,
                Transition::Other(_) => other += 1,
            }
        }
        let action_x10 = pct_x10(action, total_transitions);
        let subject_x10 = pct_x10(subject, total_transitions);
        let scene_x10 = pct_x10(scene, total_transitions);
        let other_x10 = pct_x10(other, total_transitions);
        report.push(format!(
            "- **Action-to-Action:** {} (Target: ~65%)",
            fmt_pct(action_x10)
        ));
        report.push(format!(
            "- **Subject-to-Subject:** {} (Target: ~20%)",
            fmt_pct(subject_x10)
        ));
        report.push(format!(
            "- **Scene-to-Scene:** {} (Target: ~15%)",
            fmt_pct(scene_x10)
        ));
        report.push(format!(
            "- **Other Transitions:** {} (Target: ~0%)",
            fmt_pct(other_x10)
        ));
        let drift = |x10: i64, target: i64| (x10 - target * 10).abs() > 150;
        if drift(action_x10 as i64, 65) || drift(subject_x10 as i64, 20) || drift(scene_x10 as i64, 15) {
            report.push("- ⚠️ **Pacing warning:** Transition ratios drift from the standard Action Mode 65/20/15 layout. Ensure this contemplative drift is intentional.".to_string());
            warnings.push("Transition ratios drift from the 65/20/15 Action Mode benchmark.".to_string());
        } else {
            report.push("- ✅ **Transition proportions are in the optimal 65/20/15 soft band for short action video pacing.**".to_string());
        }
        report.push(String::new());

        // 5. Kishotenketsu structural split.
        report.push("### KISHŌTENKETSU FOUR-ACT STRUCTURAL SPLITS".to_string());
        let mut ki_sho = 0u32;
        let mut ten = 0u32;
        let mut ketsu = 0u32;
        {
            let mut seen_sections: Vec<&str> = Vec::new();
            for f in &self.frames {
                if seen_sections.contains(&f.section.as_str()) {
                    continue;
                }
                seen_sections.push(&f.section);
                let dur: u32 = self
                    .frames
                    .iter()
                    .filter(|g| g.section == f.section)
                    .map(|g| g.duration_x10_ms)
                    .sum();
                let s = f.section.to_lowercase();
                if s.contains("ki") || s.contains("sho") || s.contains("intro") || s.contains("develop") {
                    ki_sho += dur;
                } else if s.contains("ten") || s.contains("twist") || s.contains("turn") {
                    ten += dur;
                } else if s.contains("ketsu") || s.contains("resolution") || s.contains("close") {
                    ketsu += dur;
                } else {
                    ki_sho += dur;
                }
            }
        }
        let total_mapped = ki_sho + ten + ketsu;
        if total_mapped > 0 {
            let ki_sho_x10 = pct_x10(ki_sho, total_mapped);
            let ten_x10 = pct_x10(ten, total_mapped);
            let ketsu_x10 = pct_x10(ketsu, total_mapped);
            report.push(format!(
                "- **Ki-Shō (Act 1-2 Development):** {} (Target: ~60%)",
                fmt_pct(ki_sho_x10)
            ));
            report.push(format!(
                "- **Ten (Act 3 Unexpected Twist):** {} (Target: ~30%)",
                fmt_pct(ten_x10)
            ));
            report.push(format!(
                "- **Ketsu (Act 4 Recontextualization):** {} (Target: ~10%)",
                fmt_pct(ketsu_x10)
            ));
            if drift(ki_sho_x10 as i64, 60) || drift(ten_x10 as i64, 30) || (ketsu_x10 as i64 - 100).abs() > 100 {
                report.push("- ⚠️ **Structure warning:** Kishōtenketsu proportions deviate from optimal 60/30/10 timeline.".to_string());
                warnings.push("Kishōtenketsu timeline splits deviate significantly from 60/30/10.".to_string());
            } else {
                report.push("- ✅ **Structural proportions are highly congruent with four-act Kishōtenketsu flow.**".to_string());
            }
        } else {
            report.push("- 🔍 **Section headers do not contain 'Ki', 'Sho', 'Ten', or 'Ketsu'. Structural proportions not determined.**".to_string());
        }
        report.push(String::new());

        // 6. Caption/dialogue reading-speed scan.
        report.push("### TEXT/SPEECH VELOCITY ZONE SCAN".to_string());
        let mut word_violation = false;
        let mut any_text = false;
        for (i, f) in self.frames.iter().enumerate() {
            let t = if !f.text.is_empty() { &f.text } else { &f.dialogue };
            if t.is_empty() {
                continue;
            }
            any_text = true;
            let words = count_words(t);
            let wpm = wpm_whole(words, f.duration_x10_ms);
            if wpm > 350 {
                word_violation = true;
                report.push(format!(
                    "- ❌ **Over-speed text on Frame {}:** Speed is {wpm} WPM! Exceeds safe 350 WPM cognitive ceiling.",
                    i + 1
                ));
                criticals.push(format!(
                    "Caption speed exceeds 350 WPM on Frame {} ({wpm} WPM).",
                    i + 1
                ));
            } else if wpm > 0 && wpm < 250 {
                report.push(format!(
                    "- ⚠️ **Under-speed text on Frame {}:** Speed is {wpm} WPM (below 250 WPM standard). Slow read risk.",
                    i + 1
                ));
            } else {
                report.push(format!(
                    "- ✅ **Frame {} captions:** Speed is {wpm} WPM (within optimal 250-350 WPM zone).",
                    i + 1
                ));
            }
        }
        if !word_violation && any_text {
            report.push("- ✅ **All captions/spoken text respect the 250-350 WPM peer-reviewed safe reading zone.**".to_string());
        }
        report.push(String::new());

        // 7. Saga Litotes (stakes vs. dialogue verbosity).
        report.push("### SAGA LITOTES (UNDERSTATEMENT SCORE)".to_string());
        for (i, f) in self.frames.iter().enumerate() {
            if f.stakes_x10 <= 30 {
                continue;
            }
            let word_count = count_words(&f.dialogue);
            let stakes = fmt_1dp(f.stakes_x10);
            if word_count > 5 {
                report.push(format!(
                    "- ⚠️ **Litotes Risk on Frame {}:** High Stakes ({stakes}) but word count is high ({word_count} words). The dialogue is too talkative; emotional resonance drops. Use flat understatement instead.",
                    i + 1
                ));
                warnings.push(format!(
                    "Dialogue is too verbose for high-stakes Frame {} (fails Saga Litotes).",
                    i + 1
                ));
            } else {
                report.push(format!(
                    "- ✅ **Litotes Active on Frame {}:** High Stakes ({stakes}) paired with silent/understated line ({word_count} words). Emotional intensity optimized.",
                    i + 1
                ));
            }
        }
        report.push(String::new());

        Analysis {
            report: report.join("\n"),
            criticals,
            warnings,
            total_x10_ms,
            total_frames,
        }
    }
}

// ---- byte-slice parsing helpers (no regex, CLAUDE.md forbidden_ops) ----

/// First `open ... close` span; returns (inner, byte_start_of_open, byte_index_of_close).
fn find_delim(s: &str, open: char, close: char) -> Option<(String, usize, usize)> {
    let start = s.find(open)?;
    let end = s[start..].find(close)? + start;
    Some((s[start + open.len_utf8()..end].to_string(), start, end))
}

fn remove_span(s: &str, start: usize, end: usize) -> String {
    let mut out = String::new();
    out.push_str(s[..start].trim_end());
    out.push(' ');
    out.push_str(s[end + 1..].trim_start());
    out.trim().to_string()
}

/// Pulls `(Key: "value")` out of `desc`, mutating it to remove the tag.
fn extract_quoted_tag(desc: &mut String, key: &str) -> Option<String> {
    let needle = format!("({key}: \"");
    let start = desc.find(&needle)?;
    let content_start = start + needle.len();
    let close_rel = desc[content_start..].find("\")")?;
    let content_end = content_start + close_rel;
    let value = desc[content_start..content_end].to_string();
    let tag_end = content_end + 2; // `")`
    *desc = remove_span(desc, start, tag_end - 1);
    Some(value)
}

/// Pulls `(Key: value)` (unquoted) out of `desc`, mutating it to remove the tag.
fn extract_simple_tag(desc: &mut String, key: &str) -> Option<String> {
    let needle = format!("({key}: ");
    let start = desc.find(&needle)?;
    let content_start = start + needle.len();
    let close_rel = desc[content_start..].find(')')?;
    let content_end = content_start + close_rel;
    let value = desc[content_start..content_end].trim().to_string();
    *desc = remove_span(desc, start, content_end);
    Some(value)
}

/// Parses a decimal literal like `"4"` or `"4.5"` into tenths (`45`).
/// [ASSUMED] at most one fractional digit is ever authored in a script
/// (matches every fixture seen: `1`, `1.0`, `3.5`, `5.0`) -- a second
/// fractional digit is truncated, not rounded.
fn parse_fixed_x10(s: &str) -> Option<u32> {
    let s = s.trim();
    match s.split_once('.') {
        Some((whole, frac)) => {
            let whole: u32 = whole.parse().ok()?;
            let digit = frac.chars().next().unwrap_or('0');
            let tenth = digit.to_digit(10)?;
            Some(whole * 10 + tenth)
        }
        None => Some(s.parse::<u32>().ok()? * 10),
    }
}

fn round_div(numer: u64, denom: u64) -> u64 {
    (numer + denom / 2) / denom
}

/// count*1000/total rounded to nearest, as tenths-of-a-percent.
fn pct_x10(count: u32, total: u32) -> u32 {
    if total == 0 {
        return 0;
    }
    round_div(count as u64 * 1000, total as u64) as u32
}

fn wpm_whole(words: u32, duration_x10_ms: u32) -> u32 {
    if duration_x10_ms == 0 {
        return 0;
    }
    // wpm = words / (duration_ms/60000) = words*60000/duration_ms
    // in tenths-of-ms duration: wpm = words*600000/duration_x10_ms
    round_div(words as u64 * 600_000, duration_x10_ms as u64) as u32
}

/// ASCII alphanumeric/underscore run count -- see module doc [ASSUMED].
fn count_words(s: &str) -> u32 {
    let mut count = 0u32;
    let mut in_word = false;
    for c in s.chars() {
        let is_word_char = c.is_ascii_alphanumeric() || c == '_';
        if is_word_char && !in_word {
            count += 1;
        }
        in_word = is_word_char;
    }
    count
}

fn fmt_1dp(x10: u32) -> String {
    format!("{}.{}", x10 / 10, x10 % 10)
}

fn fmt_pct(x10: u32) -> String {
    format!("{}%", fmt_1dp(x10))
}

fn fmt_ms(x10_ms: u32) -> String {
    fmt_1dp(x10_ms)
}

/// Renders tenths-of-ms as seconds with two decimal digits, matching
/// Python's `{:.2f}` on `ms/1000.0`.
fn fmt_seconds(x10_ms: u32) -> String {
    let hundredths = round_div(x10_ms as u64, 100);
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The exact self-test fixture embedded in v2's
    // `drop_law_compiler.py.__main__`. NOT a byte match for the real
    // receipt at `...\deep-sea-script\droplaw-report.txt` -- that file's
    // totals (17175.8ms/412 frames) don't reconcile with this script's
    // own stated dwell constants (16010.2ms/384 frames, hand-verified
    // below), so the shipped report was compiled from a since-edited
    // version of the script, not this literal text. The arc-grammar and
    // McCloud-transition assertions below ARE still real receipts (those
    // depend only on Role/transition tags, unaffected by whatever the
    // duration drift was) -- the Kishotenketsu percentages are this
    // module's own hand-verified arithmetic, not the file's.
    const DEEP_SEA_SCRIPT: &str = r#"
# Ki-Sho: Launch
[establish] Wide shot of the deep sea exploration pod sinking into black water. {scene_to_scene} (Dialogue: "Going down.") (Role: E) (Stakes: 1.0)
[initial] Sonar blip shows a massive object 200m below. {action_to_action} (Role: I) (Stakes: 1.5)
[key] Close-up of pilot's widening eyes reflected in the screen glass. {subject_to_subject} (Role: I) (Stakes: 2.0)
[dialogue] Pilot murmurs into the mic. {action_to_action} (Dialogue: "Something is down here.") (Role: I) (Stakes: 3.0)
[key] The external spotlight cuts out. Pitch blackness. {action_to_action} (Role: P) (Stakes: 4.0)
[pillow] Static camera view of empty console with blinking red warning light. {scene_to_scene} (Role: R) (Stakes: 2.0)

# Ten: The Turn
[establish] Sinking pod exterior silhouette. {scene_to_scene} (Role: E) (Stakes: 3.0)
[initial] A sound waves vibrational pattern ripples across the water. {action_to_action} (Role: I) (Stakes: 4.0)
[key] The object is not mechanical; it is organic, covered in thousands of glowing yellow eyes. {subject_to_subject} (Role: P) (Stakes: 5.0)
[dialogue] Pilot remains completely motionless and whispers. {action_to_action} (Dialogue: "It is looking at me.") (Role: P) (Stakes: 5.0)
[pillow] Slow-motion view of water bubbles rising through the light beam. {scene_to_scene} (Role: R) (Stakes: 1.0)

# Ketsu: Close
[establish] Wide underwater vista: the massive shadow vanishes back into the trench. {scene_to_scene} (Role: E) (Stakes: 3.0)
[initial] Pod spotlight turns back on, illuminating an empty sandy seabed. {action_to_action} (Role: I) (Stakes: 2.0)
[key] Pilot's glove flips the radio switch to off. {action_to_action} (Role: P) (Stakes: 4.0)
[dialogue] Pilot sits in silence as water drops leak into the chamber. {action_to_action} (Dialogue: "We are alone.") (Role: R) (Stakes: 5.0)
"#;

    fn compile(script: &str) -> Analysis {
        let mut c = DropLawCompiler::new(24);
        c.parse_script(script);
        c.analyze()
    }

    #[test]
    fn deep_sea_script_hand_verified() {
        let analysis = compile(DEEP_SEA_SCRIPT);
        // Hand-summed dwell floors: 3*establish(2000) + 2*pillow(2000) +
        // 7*key(501, includes the 3 unrecognized `[initial]` tags falling
        // back to key) + 3*dialogue(834.4) = 16010.2ms.
        assert_eq!(fmt_seconds(analysis.total_x10_ms), "16.01");
        assert_eq!(fmt_ms(analysis.total_x10_ms), "16010.2");
        assert_eq!(analysis.total_frames, 384);
        // One real critical: Frame 10's dialogue ("It is looking at me.",
        // 5 words) at the 834.4ms dialogue dwell hand-computes to
        // round(5*60000/834.4) = 360 WPM, over the 350 ceiling.
        assert_eq!(analysis.criticals.len(), 1);
        assert!(analysis.criticals[0].contains("Caption speed exceeds 350 WPM on Frame 10"));
        assert!(analysis.report.contains("**Shot Count:** 15"));
        // Real receipts, unaffected by the duration drift (see const doc
        // comment above) -- both are the exact bullets in
        // `...\deep-sea-script\droplaw-report.txt`.
        assert!(analysis.report.contains("Arc 1:** Sequence `E -> I -> I -> I -> P -> R` ✅ VALID"));
        assert!(analysis.report.contains("Arc 2:** Sequence `E -> I -> P -> P -> R` ✅ VALID"));
        assert!(analysis.report.contains("Arc 3:** Sequence `E -> I -> P -> R` ✅ VALID"));
        assert!(analysis.report.contains("Action-to-Action:** 50.0% (Target: ~65%)"));
        assert!(analysis.report.contains("Subject-to-Subject:** 14.3% (Target: ~20%)"));
        assert!(analysis.report.contains("Scene-to-Scene:** 35.7% (Target: ~15%)"));
        // Hand-verified against this module's own dwell sums (not the
        // file's, per the drift noted above): ki_sho=6337.4ms/16010.2ms,
        // ten=5836.4/16010.2, ketsu=3836.4/16010.2.
        assert!(analysis.report.contains("Ki-Shō (Act 1-2 Development):** 39.6%"));
        assert!(analysis.report.contains("Ten (Act 3 Unexpected Twist):** 36.5%"));
        assert!(analysis.report.contains("Ketsu (Act 4 Recontextualization):** 24.0%"));
        assert!(analysis.report.contains("Frame 1:** Speed is 60 WPM"));
        assert!(analysis.report.contains("Frame 4 captions:** Speed is 288 WPM"));
        assert!(analysis.report.contains("Litotes Active on Frame 5:"));
        assert!(analysis.report.contains("Litotes Active on Frame 10:** High Stakes (5.0) paired with silent/understated line (5 words)"));
    }

    #[test]
    fn a_peak_opening_an_arc_is_broken_grammar() {
        let script = "[key] cold open on the reveal. (Role: P) (Stakes: 1)\n[pillow] quiet room. (Role: R)";
        let analysis = compile(script);
        assert_eq!(analysis.criticals.len(), 1);
        assert!(analysis.criticals[0].contains("Broken Visual Grammar"));
        assert!(analysis.report.contains("Peak 'P' placed at start"));
    }

    #[test]
    fn stakes_parses_whole_and_one_decimal() {
        let mut c = DropLawCompiler::new(24);
        c.parse_script("[key] x. (Stakes: 4) (Role: P)\n[key] y. (Stakes: 4.5) (Role: R)");
        assert_eq!(c.frames[0].stakes_x10, 40);
        assert_eq!(c.frames[1].stakes_x10, 45);
    }

    #[test]
    fn round_div_rounds_half_up() {
        assert_eq!(round_div(5, 2), 3);
        assert_eq!(round_div(4, 2), 2);
    }

    #[test]
    fn attention_trit_is_supercritical_when_a_hazard_fired() {
        // The deep-sea fixture has one real critical (Frame 10's dialogue
        // reads at 360 WPM, over the 350 ceiling) -- a genuine hazard.
        let analysis = compile(DEEP_SEA_SCRIPT);
        assert_eq!(attention_trit(&analysis), 1);
    }

    fn fake_analysis(criticals: Vec<&str>, warnings: Vec<&str>) -> Analysis {
        Analysis {
            report: String::new(),
            criticals: criticals.into_iter().map(String::from).collect(),
            warnings: warnings.into_iter().map(String::from).collect(),
            total_x10_ms: 0,
            total_frames: 0,
        }
    }

    #[test]
    fn attention_trit_is_critical_with_only_soft_drift() {
        let analysis = fake_analysis(vec![], vec!["ratio drift"]);
        assert_eq!(attention_trit(&analysis), 0);
    }

    #[test]
    fn attention_trit_is_subcritical_when_clean() {
        let analysis = fake_analysis(vec![], vec![]);
        assert_eq!(attention_trit(&analysis), -1);
    }

    #[test]
    fn attention_trit_favors_supercritical_when_both_present() {
        let analysis = fake_analysis(vec!["hazard"], vec!["drift"]);
        assert_eq!(attention_trit(&analysis), 1);
    }
}
