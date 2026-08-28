//! The vixi overlay spine: authoring never edits a [`crate::ir::WidgetNode`]'s compiled
//! paint attrs directly — it appends an integer `OverlayEntry` to a
//! persisted ledger; [`Ledger::apply`] resolves overlay-first, authored
//! second, same as `source_binds` already outranks `text_literals` in
//! [`crate::ir::LoweredUi`].
//!
//! Donor: `forge-mud-v3::overlay`'s `OVL1` ledger (magic + version + count +
//! entries, little-nistam, priority-resolved, bijection-tested). PORTED, not
//! depended-on: `forge-mud-v3/Cargo.toml` declares it depends on Crate Zero
//! only and is "never a second home" for UI primitives, so this crate keeps
//! its own copy re-keyed to vixi's own domain — `(WidgetId, StyleField)`
//! instead of the MUD's `(Domain, key)` — and drops the MUD's `Scope`
//! (`Node`/`Operator`/`Global` reseed policy): vixi's `LoweredUi` has no
//! reseed concept to scope against, so carrying that field would be
//! unearned state (C10).
//!
//! Codec: magic `VOV1` + version `u8` (=1) + count `u32` + entries,
//! little-nistam. ANY malformation refuses the WHOLE ledger (L10) — decode
//! returns `None`, never a partial ledger.

use forge_core_v3::sprite_blob::{u16_from_nistam, u16_to_nistam, u32_from_nistam, u32_to_nistam};

use crate::ir::{StyleAtom, WidgetId};

/// Ledger file magic — distinct from `forge-mud-v3::overlay`'s `OVL1`.
pub const LEDGER_MAGIC: [u8; 4] = *b"VOV1";
/// Ledger schema version.
pub const LEDGER_VERSION: u8 = 1;
/// A `ReplaceStr` payload longer than this many bytes is refused (L10).
pub const MAX_STR_LEN: usize = 256;

/// Which [`StyleAtom`] field an entry overrides — vixi's domain axis.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StyleField {
    /// Overrides [`StyleAtom::radius_mu`].
    RadiusMu = 0,
    /// Overrides [`StyleAtom::alpha_pmy`].
    AlphaPmy = 1,
    /// Overrides [`StyleAtom::chrome_color`].
    ChromeColor = 2,
    /// Overrides [`StyleAtom::font`].
    Font = 3,
    /// Overrides [`StyleAtom::semantic`].
    Semantic = 4,
}

impl std::convert::TryFrom<u8> for StyleField {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(StyleField::RadiusMu),
            1 => Ok(StyleField::AlphaPmy),
            2 => Ok(StyleField::ChromeColor),
            3 => Ok(StyleField::Font),
            4 => Ok(StyleField::Semantic),
            5..=255 => Err(()),
        }
    }
}

/// What an overlay entry does to a [`StyleAtom`] field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mod {
    /// Replace a string field (`ChromeColor`/`Font`/`Semantic`), UTF-8, capped at [`MAX_STR_LEN`].
    ReplaceStr(String),
    /// Replace [`StyleField::RadiusMu`]'s `i64` MilliUnit value outright.
    SetRadiusMu(i64),
    /// Replace [`StyleField::AlphaPmy`]'s permyriad value outright.
    SetAlphaPmy(u16),
    /// Clear the field — masks every lower-priority entry at this key.
    Remove,
}

/// One append-only overlay fact: "at (widget, field), apply this
/// modification, at this priority."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayEntry {
    /// Which widget this overlay touches.
    pub widget: WidgetId,
    /// Which paint field within that widget.
    pub field: StyleField,
    /// The modification this entry applies.
    pub modification: Mod,
    /// Higher wins; ties break to the later-appended entry.
    pub priority: u16,
}

/// The append-only overlay ledger: authoring appends; nothing ever mutates
/// or removes an entry in place (a `Remove` entry is itself an append).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    /// Append-only overlay facts, oldest first.
    pub entries: Vec<OverlayEntry>,
}

/// `true` when the candidate (priority, index) beats the current best:
/// higher priority wins; a tie is broken by the later (greater-index) entry.
fn beats(cand_priority: u16, cand_idx: usize, cur_priority: u16, cur_idx: usize) -> bool {
    cand_priority > cur_priority || (cand_priority == cur_priority && cand_idx > cur_idx)
}

impl Ledger {
    /// Append one entry. Never edits or removes an existing one.
    pub fn append(&mut self, entry: OverlayEntry) {
        self.entries.push(entry);
    }

    fn resolve(&self, widget: WidgetId, field: StyleField) -> Option<&Mod> {
        let mut best: Option<(usize, &OverlayEntry)> = None;
        for (i, e) in self.entries.iter().enumerate() {
            if e.widget != widget || e.field != field {
                continue;
            }
            let take = match best {
                None => true,
                Some((bi, be)) => beats(e.priority, i, be.priority, bi),
            };
            if take {
                best = Some((i, e));
            }
        }
        best.map(|(_, e)| &e.modification)
    }

    /// Resolve `base` (a widget's authored [`StyleAtom`]) against this
    /// ledger: the highest-priority visible entry per field wins, a
    /// `Remove` masks the field to `None`, and a field with no applicable
    /// entry keeps its authored value. Never allocates when the ledger has
    /// no entry for `widget`.
    pub fn apply(&self, widget: WidgetId, base: StyleAtom) -> StyleAtom {
        if !self.entries.iter().any(|e| e.widget == widget) {
            return base;
        }
        StyleAtom {
            radius_mu: match self.resolve(widget, StyleField::RadiusMu) {
                Some(Mod::SetRadiusMu(v)) => Some(*v),
                Some(Mod::Remove) => None,
                _ => base.radius_mu,
            },
            alpha_pmy: match self.resolve(widget, StyleField::AlphaPmy) {
                Some(Mod::SetAlphaPmy(v)) => Some(*v),
                Some(Mod::Remove) => None,
                _ => base.alpha_pmy,
            },
            chrome_color: match self.resolve(widget, StyleField::ChromeColor) {
                Some(Mod::ReplaceStr(s)) => Some(s.clone()),
                Some(Mod::Remove) => None,
                _ => base.chrome_color,
            },
            font: match self.resolve(widget, StyleField::Font) {
                Some(Mod::ReplaceStr(s)) => Some(s.clone()),
                Some(Mod::Remove) => None,
                _ => base.font,
            },
            semantic: match self.resolve(widget, StyleField::Semantic) {
                Some(Mod::ReplaceStr(s)) => Some(s.clone()),
                Some(Mod::Remove) => None,
                _ => base.semantic,
            },
        }
    }

    /// Encode the whole ledger: magic + version + count + entries, all
    /// little-nistam.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&LEDGER_MAGIC);
        out.push(LEDGER_VERSION);
        out.extend_from_slice(&u32_to_nistam(self.entries.len() as u32));
        for e in &self.entries {
            encode_entry(e, &mut out);
        }
        out
    }

    /// Decode ledger bytes. ANY malformation — bad magic, bad version,
    /// truncation, a bad field/mod tag, an over-long string, or trailing
    /// bytes — refuses the WHOLE ledger: `None`, never a partial one (L10).
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
            let s = bytes.get(*at..*at + n)?;
            *at += n;
            Some(s)
        };
        if take(&mut at, 4)? != LEDGER_MAGIC {
            return None;
        }
        if take(&mut at, 1)?[0] != LEDGER_VERSION {
            return None;
        }
        let count = u32_from_nistam(take(&mut at, 4)?, 0) as usize;
        let mut entries = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            entries.push(decode_entry(bytes, &mut at)?);
        }
        if at != bytes.len() {
            return None;
        }
        Some(Ledger { entries })
    }
}

fn encode_entry(e: &OverlayEntry, out: &mut Vec<u8>) {
    out.push(e.field as u8);
    out.extend_from_slice(&u32_to_nistam(e.widget.0));
    match &e.modification {
        Mod::ReplaceStr(s) => {
            out.push(0);
            let bytes = s.as_bytes();
            out.extend_from_slice(&u16_to_nistam(bytes.len() as u16));
            out.extend_from_slice(bytes);
        }
        Mod::SetRadiusMu(v) => {
            out.push(1);
            out.extend_from_slice(&(*v as u64).to_le_bytes());
        }
        Mod::SetAlphaPmy(v) => {
            out.push(2);
            out.extend_from_slice(&u16_to_nistam(*v));
        }
        Mod::Remove => {
            out.push(3);
        }
    }
    out.extend_from_slice(&u16_to_nistam(e.priority));
}

fn decode_entry(bytes: &[u8], at: &mut usize) -> Option<OverlayEntry> {
    let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
        let s = bytes.get(*at..*at + n)?;
        *at += n;
        Some(s)
    };
    let field = std::convert::TryFrom::try_from(take(at, 1)?[0]).ok()?;
    let widget = WidgetId(u32_from_nistam(take(at, 4)?, 0));
    let mod_tag = take(at, 1)?[0];
    let modification = match mod_tag {
        0 => {
            let len = u16_from_nistam(take(at, 2)?, 0) as usize;
            if len > MAX_STR_LEN {
                return None;
            }
            let s = std::str::from_utf8(take(at, len)?).ok()?.to_string();
            Mod::ReplaceStr(s)
        }
        1 => {
            let raw: [u8; 8] = take(at, 8)?.try_into().ok()?;
            Mod::SetRadiusMu(u64::from_le_bytes(raw) as i64)
        }
        2 => Mod::SetAlphaPmy(u16_from_nistam(take(at, 2)?, 0)),
        3 => Mod::Remove,
        _ => return None,
    };
    let priority = u16_from_nistam(take(at, 2)?, 0);
    Some(OverlayEntry { widget, field, modification, priority })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(widget: u32, field: StyleField, modification: Mod, priority: u16) -> OverlayEntry {
        OverlayEntry { widget: WidgetId(widget), field, modification, priority }
    }

    /// L07: empty ledger, one of each `Mod`, a max-len (256) string, and a
    /// 50-entry ledger all survive encode -> decode byte-exact.
    #[test]
    fn the_codec_is_a_bijection_over_interior_and_edges() {
        let empty = Ledger::default();
        assert_eq!(Ledger::decode(&empty.encode()), Some(empty));

        let one_of_each = Ledger {
            entries: vec![
                entry(1, StyleField::ChromeColor, Mod::ReplaceStr("bronze".into()), 5),
                entry(2, StyleField::RadiusMu, Mod::SetRadiusMu(-4_000), 10),
                entry(3, StyleField::AlphaPmy, Mod::SetAlphaPmy(8_500), 20),
                entry(4, StyleField::Semantic, Mod::Remove, 0),
            ],
        };
        assert_eq!(Ledger::decode(&one_of_each.encode()), Some(one_of_each));

        let max_str = Ledger {
            entries: vec![entry(u32::MAX, StyleField::Font, Mod::ReplaceStr("x".repeat(256)), u16::MAX)],
        };
        assert_eq!(Ledger::decode(&max_str.encode()), Some(max_str));

        let fifty = Ledger {
            entries: (0..50u32)
                .map(|i| entry(i, StyleField::RadiusMu, Mod::SetRadiusMu(i as i64), (i % 65536) as u16))
                .collect(),
        };
        assert_eq!(Ledger::decode(&fifty.encode()), Some(fifty));
    }

    /// L10: bad magic, truncation at every field boundary of entry 0,
    /// field=5, trailing bytes, and an over-long string len all refuse
    /// whole — None, never a partial ledger.
    #[test]
    fn malformed_ledgers_are_refused_whole() {
        let good = Ledger { entries: vec![entry(9, StyleField::RadiusMu, Mod::SetRadiusMu(1), 1)] }.encode();

        let mut bad_magic = good.clone();
        bad_magic[0] ^= 0xff;
        assert!(Ledger::decode(&bad_magic).is_none(), "bad magic");

        for len in 0..good.len() {
            assert!(Ledger::decode(&good[..len]).is_none(), "truncated at {len}");
        }

        let mut trailing = good.clone();
        trailing.push(0);
        assert!(Ledger::decode(&trailing).is_none(), "trailing byte");

        let mut bad_field = good.clone();
        bad_field[5] = 5;
        assert!(Ledger::decode(&bad_field).is_none(), "field=5");

        let over_long = Ledger { entries: vec![entry(1, StyleField::Font, Mod::ReplaceStr("x".repeat(257)), 1)] };
        assert!(Ledger::decode(&over_long.encode()).is_none(), "over-long string len");
    }

    /// Resolver semantics: priority wins, ties break to the later entry,
    /// `Remove` masks a lower-priority set, and fields with no entry keep
    /// the authored base — mirroring `forge-mud-v3::overlay`'s resolver.
    #[test]
    fn resolver_semantics() {
        let mut l = Ledger::default();
        l.append(entry(1, StyleField::ChromeColor, Mod::ReplaceStr("low".into()), 1));
        l.append(entry(1, StyleField::ChromeColor, Mod::ReplaceStr("high".into()), 5));
        assert_eq!(l.resolve(WidgetId(1), StyleField::ChromeColor), Some(&Mod::ReplaceStr("high".into())));

        let mut tie = Ledger::default();
        tie.append(entry(2, StyleField::Font, Mod::ReplaceStr("first".into()), 3));
        tie.append(entry(2, StyleField::Font, Mod::ReplaceStr("second".into()), 3));
        assert_eq!(tie.resolve(WidgetId(2), StyleField::Font), Some(&Mod::ReplaceStr("second".into())));

        let mut removed = Ledger::default();
        removed.append(entry(3, StyleField::RadiusMu, Mod::SetRadiusMu(4_000), 1));
        removed.append(entry(3, StyleField::RadiusMu, Mod::Remove, 5));
        let atom = removed.apply(WidgetId(3), StyleAtom { radius_mu: Some(1_000), ..Default::default() });
        assert_eq!(atom.radius_mu, None, "remove masks the lower-priority set, clearing the authored base too");

        let mut under = Ledger::default();
        under.append(entry(4, StyleField::RadiusMu, Mod::SetRadiusMu(4_000), 5));
        under.append(entry(4, StyleField::RadiusMu, Mod::Remove, 1));
        let atom = under.apply(WidgetId(4), StyleAtom::default());
        assert_eq!(atom.radius_mu, Some(4_000), "a lower-priority remove does not mask a higher set");
    }

    /// `apply` is the actual sink a caller (emitter or a live-tweak tool)
    /// uses: an authored [`StyleAtom`] overlaid by the ledger, per-field.
    #[test]
    fn apply_overlays_field_by_field_leaving_unentried_fields_authored() {
        let mut l = Ledger::default();
        l.append(entry(7, StyleField::AlphaPmy, Mod::SetAlphaPmy(3_000), 1));

        let base = StyleAtom {
            radius_mu: Some(2_000),
            alpha_pmy: Some(10_000),
            chrome_color: Some("panel".into()),
            font: None,
            semantic: None,
        };
        let out = l.apply(WidgetId(7), base.clone());
        assert_eq!(out.alpha_pmy, Some(3_000), "the overlaid field changes");
        assert_eq!(out.radius_mu, base.radius_mu, "an un-entried field stays authored");
        assert_eq!(out.chrome_color, base.chrome_color, "an un-entried field stays authored");
    }

    /// A widget with no ledger entries at all returns `base` unchanged and
    /// without allocating a new `StyleAtom` field-by-field.
    #[test]
    fn a_widget_with_no_entries_passes_the_authored_atom_through() {
        let l = Ledger::default();
        let base = StyleAtom { chrome_color: Some("bronze".into()), ..Default::default() };
        assert_eq!(l.apply(WidgetId(1), base.clone()), base);
    }

    /// `Domain::try_from` twin: `StyleField::try_from` refuses every tag
    /// past the packet-stable range.
    #[test]
    fn style_field_try_from_refuses_out_of_range() {
        use std::convert::TryFrom;
        assert_eq!(StyleField::try_from(4), Ok(StyleField::Semantic));
        assert!(StyleField::try_from(5).is_err());
        assert!(StyleField::try_from(255).is_err());
    }
}
