//! automaton.rs — `#vixi:kit` automaton lowering: signal bindings, effort axes,
//! and `state {}` blocks → integer [`KitAutomaton`] (float_in_ir stays forbidden).
//! Authoring donor: crates/forge-envelope/surfaceledger/astrological_starmap.kit.vixi.

use crate::parse::ParseError;

/// The lowered automaton carried by a [`crate::parse::KitDoc`]: source bindings
/// (`schaeffer.mass = signal(audio.sub_bass)`), two-pole effort axes
/// (`laban.space = direct | indirect`), and per-state pole/drive tables.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KitAutomaton {
    /// `<morph> = signal(<token>)` rows, e.g. `("schaeffer.mass", "audio.sub_bass")`.
    pub bindings: Vec<(String, String)>,
    /// `<axis> = <a> | <b>` rows, e.g. `("laban.space", ["direct", "indirect"])`.
    pub axes: Vec<(String, [String; 2])>,
    /// The `state <name> { … }` blocks in authored order.
    pub states: Vec<AutomatonState>,
}

/// One `state <name> { … }` block: axis poles and permyriad drives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutomatonState {
    /// The block name (`listening` / `previewing` / …).
    pub name: String,
    /// `<axis> <- <pole>` rows; every pole is one of its axis's two declared poles.
    pub poles: Vec<(String, String)>,
    /// `<target> <- …` rows (`vibe_glow` / `vibe_shake`).
    pub drives: Vec<Drive>,
}

/// One drive row: `<target> <- <source> * <N>p` or the constant `<target> <- <N>p`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drive {
    /// The driven channel name (`vibe_glow` / `vibe_shake`).
    pub target: String,
    /// The bound morphology source (`schaeffer.mass`); `None` = constant drive.
    pub source: Option<String>,
    /// Integer permyriad gain (source drive) or level (constant drive).
    pub gain_pmy: u32,
}

impl KitAutomaton {
    /// The state block named `name`, if authored.
    pub fn state(&self, name: &str) -> Option<&AutomatonState> {
        self.states.iter().find(|s| s.name == name)
    }

    /// The signal token a morphology key is bound to (`schaeffer.mass` → `audio.sub_bass`).
    pub fn binding(&self, morph: &str) -> Option<&str> {
        self.bindings.iter().find(|(m, _)| m == morph).map(|(_, s)| s.as_str())
    }
}

impl AutomatonState {
    /// This state's pole on `axis`, if assigned.
    pub fn pole(&self, axis: &str) -> Option<&str> {
        self.poles.iter().find(|(a, _)| a == axis).map(|(_, p)| p.as_str())
    }

    /// This state's drive row for `target`, if assigned.
    pub fn drive(&self, target: &str) -> Option<&Drive> {
        self.drives.iter().find(|d| d.target == target)
    }
}

/// Accumulates automaton rows as `build_doc` walks the source; every malformed
/// or dangling row refuses LOUD with its line number (forbid.silent_parser_drops).
pub(crate) struct AutomatonBuilder {
    auto: KitAutomaton,
    open: Option<AutomatonState>,
}

/// `<N>p` → integer permyriad. Anything else ⇒ `None` (caller raises the error).
fn parse_pmy(tok: &str) -> Option<u32> {
    tok.strip_suffix('p')?.parse::<u32>().ok()
}

impl AutomatonBuilder {
    pub(crate) fn new() -> Self {
        Self { auto: KitAutomaton::default(), open: None }
    }

    /// Whether a `state {` block is currently open (its lines route here).
    pub(crate) fn in_state(&self) -> bool {
        self.open.is_some()
    }

    /// `<morph> = signal(<token>)`.
    pub(crate) fn bind(&mut self, line_no: usize, morph: &str, rhs: &str) -> Result<(), ParseError> {
        let token = rhs
            .strip_prefix("signal(")
            .and_then(|r| r.strip_suffix(')'))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| ParseError::at(line_no, format!("binding wants `= signal(<token>)`, got '{rhs}'")))?;
        if self.auto.bindings.iter().any(|(m, _)| m == morph) {
            return Err(ParseError::at(line_no, format!("duplicate binding '{morph}'")));
        }
        self.auto.bindings.push((morph.to_string(), token.to_string()));
        Ok(())
    }

    /// `<axis> = <a> | <b>`.
    pub(crate) fn axis(&mut self, line_no: usize, axis: &str, rhs: &str) -> Result<(), ParseError> {
        let (a, b) = rhs
            .split_once('|')
            .map(|(a, b)| (a.trim(), b.trim()))
            .filter(|(a, b)| !a.is_empty() && !b.is_empty())
            .ok_or_else(|| ParseError::at(line_no, format!("axis wants `= <a> | <b>`, got '{rhs}'")))?;
        if self.auto.axes.iter().any(|(n, _)| n == axis) {
            return Err(ParseError::at(line_no, format!("duplicate axis '{axis}'")));
        }
        self.auto.axes.push((axis.to_string(), [a.to_string(), b.to_string()]));
        Ok(())
    }

    /// `state <name> {`.
    pub(crate) fn open_state(&mut self, line_no: usize, rest: &str) -> Result<(), ParseError> {
        if self.open.is_some() {
            return Err(ParseError::at(line_no, "nested `state` block"));
        }
        let name = rest
            .strip_suffix('{')
            .map(str::trim)
            .filter(|n| !n.is_empty() && n.split_whitespace().count() == 1)
            .ok_or_else(|| ParseError::at(line_no, format!("state wants `state <name> {{`, got 'state {rest}'")))?;
        if self.auto.states.iter().any(|s| s.name == name) {
            return Err(ParseError::at(line_no, format!("duplicate state '{name}'")));
        }
        self.open = Some(AutomatonState { name: name.to_string(), poles: Vec::new(), drives: Vec::new() });
        Ok(())
    }

    /// One line inside an open block: `}` close, `<axis> <- <pole>` (dotted lhs),
    /// or `<target> <- [<source> *] <N>p` drive.
    pub(crate) fn state_line(&mut self, line_no: usize, line: &str) -> Result<(), ParseError> {
        if line == "}" {
            let done = self.open.take().expect("state_line only routes while open");
            self.auto.states.push(done);
            return Ok(());
        }
        let (lhs, rhs) = line
            .split_once("<-")
            .map(|(l, r)| (l.trim(), r.trim()))
            .filter(|(l, r)| !l.is_empty() && !r.is_empty())
            .ok_or_else(|| ParseError::at(line_no, format!("state row wants `<lhs> <- <rhs>`, got '{line}'")))?;
        let open = self.open.as_mut().expect("state_line only routes while open");
        if lhs.contains('.') {
            // Axis pole assignment — the axis must be declared, the pole must be one of its two.
            let axis = self
                .auto
                .axes
                .iter()
                .find(|(n, _)| n == lhs)
                .ok_or_else(|| ParseError::at(line_no, format!("undeclared axis '{lhs}'")))?;
            if !axis.1.iter().any(|p| p == rhs) {
                return Err(ParseError::at(
                    line_no,
                    format!("pole '{rhs}' outside axis '{lhs}' ({} | {})", axis.1[0], axis.1[1]),
                ));
            }
            if open.poles.iter().any(|(a, _)| a == lhs) {
                return Err(ParseError::at(line_no, format!("duplicate pole for axis '{lhs}' in state '{}'", open.name)));
            }
            open.poles.push((lhs.to_string(), rhs.to_string()));
            return Ok(());
        }
        // Drive: `<source> * <N>p` or constant `<N>p`.
        let (source, gain_tok) = match rhs.split_once('*') {
            Some((s, g)) => (Some(s.trim()), g.trim()),
            None => (None, rhs),
        };
        if let Some(src) = source {
            if !self.auto.bindings.iter().any(|(m, _)| m == src) {
                return Err(ParseError::at(line_no, format!("undeclared drive source '{src}'")));
            }
        }
        let gain_pmy = parse_pmy(gain_tok)
            .ok_or_else(|| ParseError::at(line_no, format!("drive gain wants `<N>p`, got '{gain_tok}'")))?;
        if open.drives.iter().any(|d| d.target == lhs) {
            return Err(ParseError::at(line_no, format!("duplicate drive '{lhs}' in state '{}'", open.name)));
        }
        open.drives.push(Drive { target: lhs.to_string(), source: source.map(str::to_string), gain_pmy });
        Ok(())
    }

    /// End of source: an unclosed block refuses; no automaton rows at all ⇒ `None`.
    pub(crate) fn finish(self) -> Result<Option<KitAutomaton>, ParseError> {
        if let Some(open) = &self.open {
            return Err(ParseError::at(0, format!("unclosed state block '{}'", open.name)));
        }
        let empty = self.auto.bindings.is_empty() && self.auto.axes.is_empty() && self.auto.states.is_empty();
        Ok(if empty { None } else { Some(self.auto) })
    }
}
