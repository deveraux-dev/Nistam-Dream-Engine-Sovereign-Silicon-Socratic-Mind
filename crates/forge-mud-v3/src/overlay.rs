//! The overlay spine: authoring never edits the seed's deal — it appends an
//! integer OverlayEntry to a persisted ledger; readers resolve overlay-first,
//! seed-second. Skeleton pre-created by the conductor; Weld A fills it.
//!
//! Donor: ironroot `OverlayEntry` schema (fully integer), welded onto this
//! crate's bijection-tested codec style — see `crate::operator` for the
//! house pattern this copies: magic + version + fields, little-nistam byte
//! order via `forge_core_v3::sprite_blob`'s own helpers.
//!
//! Codec: magic `OVL1` + version `u8` (=1) + count `u32` + entries,
//! little-nistam. ANY malformation refuses the WHOLE ledger (L10) — decode
//! returns `None`/`Err`, never a partial ledger.

use std::path::{Path, PathBuf};

use forge_core_v3::sprite_blob::{
    u16_from_nistam, u16_to_nistam, u32_from_nistam, u32_to_nistam, u64_from_nistam,
    u64_to_nistam,
};

/// Ledger file magic — distinct from every other `.forge` reader.
pub const LEDGER_MAGIC: [u8; 4] = *b"OVL1";
/// Ledger schema version.
pub const LEDGER_VERSION: u8 = 1;
/// A `ReplaceStr` payload longer than this many bytes is refused (L10).
pub const MAX_STR_LEN: usize = 256;

/// The fixed per-entry header size: domain(1) + key(2) + mod-tag(1) +
/// priority(2) + scope-tag(1). Variable payloads (a `ReplaceStr`'s bytes, a
/// `Node` scope's seed) ride after this.
pub const ENTRY_HEADER_BYTES: usize = 1 + 2 + 1 + 2 + 1;
const _: () = assert!(ENTRY_HEADER_BYTES == 7, "overlay entry header layout drifted");

/// The overlay domains — packet-stable numbering. Widen later by appending;
/// NEVER renumber a live tag.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    /// Faction standings.
    Faction = 0,
    /// Town state.
    Town = 1,
    /// Biome state.
    Biome = 2,
    /// Law/watch state.
    Law = 3,
    /// NPC state.
    Npc = 4,
    /// Quest state.
    Quest = 5,
    /// Item state.
    Item = 6,
    /// Fishing state.
    Fish = 7,
    /// Brewing state.
    Brew = 8,
    /// Pet state.
    Pet = 9,
    /// Talent tree state.
    Talent = 10,
    /// Boss state.
    Boss = 11,
    /// Sky/celestial state.
    Sky = 12,
    /// Ambient vibe state.
    Vibe = 13,
    /// Weather state.
    Weather = 14,
    /// Zone state.
    Zone = 15,
    /// Action-bar state (Weld H: `crate::actions`).
    Action = 16,
    /// CYOA archetype choice pressures (Weld: archetype_ledger).
    Archetype = 17,
}

impl std::convert::TryFrom<u8> for Domain {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(Domain::Faction),
            1 => Ok(Domain::Town),
            2 => Ok(Domain::Biome),
            3 => Ok(Domain::Law),
            4 => Ok(Domain::Npc),
            5 => Ok(Domain::Quest),
            6 => Ok(Domain::Item),
            7 => Ok(Domain::Fish),
            8 => Ok(Domain::Brew),
            9 => Ok(Domain::Pet),
            10 => Ok(Domain::Talent),
            11 => Ok(Domain::Boss),
            12 => Ok(Domain::Sky),
            13 => Ok(Domain::Vibe),
            14 => Ok(Domain::Weather),
            15 => Ok(Domain::Zone),
            16 => Ok(Domain::Action),
            17 => Ok(Domain::Archetype),
            18..=255 => Err(()),
        }
    }
}

/// What an overlay entry does to the seed's deal at (domain, key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mod {
    /// Replace a string field outright (UTF-8, capped at [`MAX_STR_LEN`]).
    ReplaceStr(String),
    /// Add a signed delta to an integer field. The highest-priority `Add`
    /// WINS; siblings at the same key are overridden, not summed. This is the
    /// authoring primitive: "at this key, the answer is base + v."
    Add(i64),
    /// Add a signed delta that SUMS with every other visible `Accumulate` at
    /// the same key, regardless of priority — a running tally rather than an
    /// override. This is the event-log primitive: "one more thing happened."
    ///
    /// Chosen when the entry is appended, so a lane that tallies never has to
    /// remember to read-then-append (the `mint_item_id` pattern), and a second
    /// event can never silently replace the first.
    Accumulate(i64),
    /// Multiply by a permyriad (parts-per-10,000) factor: `base * m / 10_000`.
    MulPmy(u32),
    /// Erase the field — masks every lower-priority entry at this key.
    Remove,
}

/// The reseed policy an entry lives under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    /// Dies when the named node seed reseeds away.
    Node(u64),
    /// Follows the player across reseeds.
    Operator,
    /// Survives everything.
    Global,
}

/// One append-only overlay fact: "at (domain, key), under this scope, apply
/// this modification, at this priority."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayEntry {
    /// Which content family this overlay touches.
    pub domain: Domain,
    /// The field key within that domain.
    pub key: u16,
    /// The modification this entry applies.
    pub modification: Mod,
    /// Higher wins; ties break to the later-appended entry.
    pub priority: u16,
    /// The reseed policy this entry lives under.
    pub scope: Scope,
}

/// The append-only overlay ledger: authoring appends; nothing ever mutates
/// or removes an entry in place (a `Remove` entry is itself an append).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ledger {
    /// Append-only overlay facts, oldest first.
    pub entries: Vec<OverlayEntry>,
}

fn scope_visible(scope: Scope, seed: u64) -> bool {
    match scope {
        Scope::Node(s) => s == seed,
        Scope::Operator | Scope::Global => true,
    }
}

/// `true` when the candidate (priority, index) beats the current best:
/// higher priority wins; a tie is broken by the later (greater-index) entry.
fn beats(cand_priority: u16, cand_idx: usize, cur_priority: u16, cur_idx: usize) -> bool {
    cand_priority > cur_priority || (cand_priority == cur_priority && cand_idx > cur_idx)
}

impl Ledger {
    /// Append one entry. Never edits or removes an existing one (L10 wire:
    /// the deal is pure_function(seed); this only ever grows).
    pub fn append(&mut self, entry: OverlayEntry) {
        self.entries.push(entry);
    }

    /// Resolve a string field: highest-priority visible `ReplaceStr` wins;
    /// a `Remove` at or above that priority masks it to `None`. Ties are
    /// broken by the later-appended entry.
    pub fn resolve_str<'a>(&'a self, domain: Domain, key: u16, seed: u64) -> Option<&'a str> {
        let mut best: Option<(usize, &OverlayEntry)> = None;
        for (i, e) in self.entries.iter().enumerate() {
            if e.domain != domain || e.key != key || !scope_visible(e.scope, seed) {
                continue;
            }
            if !matches!(e.modification, Mod::ReplaceStr(_) | Mod::Remove) {
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
        match best.map(|(_, e)| &e.modification) {
            Some(Mod::ReplaceStr(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Resolve an integer field over `base`.
    ///
    /// Two modification kinds meet here, and which one a caller appends is the
    /// choice that decides whether a second event replaces the first or adds
    /// to it:
    ///
    /// - [`Mod::Add`] OVERRIDES — the single highest-priority visible `Add`
    ///   wins and is applied. Siblings are masked, never summed.
    /// - [`Mod::Accumulate`] TALLIES — every visible `Accumulate` that
    ///   survives the mask is summed, priority notwithstanding.
    ///
    /// A `Remove` masks everything at or below its own priority, both kinds;
    /// entries that beat it still count. The two compose: the winning `Add`
    /// sets the standing value, the surviving `Accumulate`s ride on top.
    pub fn resolve_i64(&self, domain: Domain, key: u16, seed: u64, base: i64) -> i64 {
        let mut best: Option<(usize, &OverlayEntry)> = None;
        let mut mask: Option<(usize, u16)> = None;
        for (i, e) in self.entries.iter().enumerate() {
            if e.domain != domain || e.key != key || !scope_visible(e.scope, seed) {
                continue;
            }
            if matches!(e.modification, Mod::Remove) {
                if mask.is_none_or(|(mi, mp)| beats(e.priority, i, mp, mi)) {
                    mask = Some((i, e.priority));
                }
            }
            if !matches!(e.modification, Mod::Add(_) | Mod::Remove) {
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
        let standing = match best.map(|(_, e)| &e.modification) {
            Some(Mod::Add(v)) => base + v,
            _ => base,
        };
        let tally: i64 = self
            .entries
            .iter()
            .enumerate()
            .filter(|(i, e)| {
                e.domain == domain
                    && e.key == key
                    && scope_visible(e.scope, seed)
                    && mask.is_none_or(|(mi, mp)| beats(e.priority, *i, mp, mi))
            })
            .filter_map(|(_, e)| match e.modification {
                Mod::Accumulate(v) => Some(v),
                _ => None,
            })
            .fold(0i64, i64::saturating_add);
        standing.saturating_add(tally)
    }

    /// Resolve a permyriad-scaled field over `base`: highest-priority
    /// visible `MulPmy` wins and is applied as `base * m / 10_000`; a
    /// `Remove` at or above that priority (or no applicable entry) resolves
    /// to `base`.
    pub fn resolve_pmy(&self, domain: Domain, key: u16, seed: u64, base: u32) -> u32 {
        let mut best: Option<(usize, &OverlayEntry)> = None;
        for (i, e) in self.entries.iter().enumerate() {
            if e.domain != domain || e.key != key || !scope_visible(e.scope, seed) {
                continue;
            }
            if !matches!(e.modification, Mod::MulPmy(_) | Mod::Remove) {
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
        match best.map(|(_, e)| &e.modification) {
            Some(Mod::MulPmy(m)) => ((base as u64 * *m as u64) / 10_000) as u32,
            _ => base,
        }
    }

    /// Encode the whole ledger: magic + version + count + entries, all
    /// little-nistam (the house wire order — see `crate::operator`).
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
    /// truncation, a bad domain/mod/scope tag, an over-long string, or
    /// trailing bytes — refuses the WHOLE ledger: `None`, never a partial
    /// one (L10).
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

    /// Load a ledger from `path`. An absent file is an empty ledger — that
    /// is not corruption, it is a fresh world. A malformed file is refused
    /// whole and reported to the caller rather than silently discarded
    /// (L10: a swallowed refusal is silent loss, not safety).
    pub fn load(path: &Path) -> Result<Ledger, LedgerError> {
        match std::fs::read(path) {
            Ok(bytes) => Ledger::decode(&bytes).ok_or(LedgerError::Malformed),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Ledger::default()),
            Err(e) => Err(LedgerError::Io(e)),
        }
    }

    /// Save atomically: write to `<path>.tmp`, then rename over `path`.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let mut tmp_name = path.as_os_str().to_os_string();
        tmp_name.push(".tmp");
        let tmp: PathBuf = tmp_name.into();
        std::fs::write(&tmp, self.encode())?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}

/// Why [`Ledger::load`] could not hand back a ledger.
#[derive(Debug)]
pub enum LedgerError {
    /// The file exists but failed to decode (bad magic/version/tag,
    /// truncation, over-long string, or trailing bytes).
    Malformed,
    /// The filesystem itself refused the read (permissions, etc.).
    Io(std::io::Error),
}

fn encode_entry(e: &OverlayEntry, out: &mut Vec<u8>) {
    out.push(e.domain as u8);
    out.extend_from_slice(&u16_to_nistam(e.key));
    match &e.modification {
        Mod::ReplaceStr(s) => {
            out.push(0);
            let bytes = s.as_bytes();
            out.extend_from_slice(&u16_to_nistam(bytes.len() as u16));
            out.extend_from_slice(bytes);
        }
        Mod::Add(v) => {
            out.push(1);
            out.extend_from_slice(&u64_to_nistam(*v as u64));
        }
        Mod::MulPmy(m) => {
            out.push(2);
            out.extend_from_slice(&u32_to_nistam(*m));
        }
        Mod::Remove => {
            out.push(3);
        }
        // Tag 4 was free; LEDGER_VERSION stays 1 on purpose. `decode` matches
        // the version EXACTLY, so a bump would refuse every existing
        // overlays.ovl WHOLE (L10) and drop live overlay state. A new tag
        // costs nothing to old files — they contain none.
        Mod::Accumulate(v) => {
            out.push(4);
            out.extend_from_slice(&u64_to_nistam(*v as u64));
        }
    }
    out.extend_from_slice(&u16_to_nistam(e.priority));
    match e.scope {
        Scope::Node(s) => {
            out.push(0);
            out.extend_from_slice(&u64_to_nistam(s));
        }
        Scope::Operator => out.push(1),
        Scope::Global => out.push(2),
    }
}

fn decode_entry(bytes: &[u8], at: &mut usize) -> Option<OverlayEntry> {
    let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
        let s = bytes.get(*at..*at + n)?;
        *at += n;
        Some(s)
    };
    let domain = std::convert::TryFrom::try_from(take(at, 1)?[0]).ok()?;
    let key = u16_from_nistam(take(at, 2)?, 0);
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
        1 => Mod::Add(u64_from_nistam(take(at, 8)?, 0) as i64),
        2 => Mod::MulPmy(u32_from_nistam(take(at, 4)?, 0)),
        3 => Mod::Remove,
        4 => Mod::Accumulate(u64_from_nistam(take(at, 8)?, 0) as i64),
        _ => return None,
    };
    let priority = u16_from_nistam(take(at, 2)?, 0);
    let scope_tag = take(at, 1)?[0];
    let scope = match scope_tag {
        0 => Scope::Node(u64_from_nistam(take(at, 8)?, 0)),
        1 => Scope::Operator,
        2 => Scope::Global,
        _ => return None,
    };
    Some(OverlayEntry { domain, key, modification, priority, scope })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    fn entry(domain: Domain, key: u16, modification: Mod, priority: u16, scope: Scope) -> OverlayEntry {
        OverlayEntry { domain, key, modification, priority, scope }
    }

    /// L07: empty ledger, one of each `Mod`, a max-len (256) string, u16::MAX
    /// key/priority, all three scopes, and a 100-entry ledger all survive
    /// encode -> decode byte-exact.
    #[test]
    fn the_codec_is_a_bijection_over_interior_and_edges() {
        let empty = Ledger::default();
        assert_eq!(Ledger::decode(&empty.encode()), Some(empty));

        let one_of_each = Ledger {
            entries: vec![
                entry(Domain::Town, 1, Mod::ReplaceStr("hello".into()), 5, Scope::Global),
                entry(Domain::Faction, 2, Mod::Add(-42), 10, Scope::Operator),
                entry(Domain::Weather, 3, Mod::MulPmy(12_345), 20, Scope::Node(777)),
                entry(Domain::Law, 4, Mod::Remove, 0, Scope::Global),
            ],
        };
        assert_eq!(Ledger::decode(&one_of_each.encode()), Some(one_of_each));

        // Tag 4 round-trips, and the version did NOT move — an existing
        // overlays.ovl written before Accumulate existed must still open.
        let with_tally = Ledger {
            entries: vec![entry(Domain::Item, 0xFFFE, Mod::Accumulate(-3_500), 0, Scope::Operator)],
        };
        assert_eq!(Ledger::decode(&with_tally.encode()), Some(with_tally));
        assert_eq!(LEDGER_VERSION, 1, "bumping this refuses every ledger on disk WHOLE (L10)");

        let max_str = Ledger {
            entries: vec![entry(
                Domain::Npc,
                u16::MAX,
                Mod::ReplaceStr("x".repeat(256)),
                u16::MAX,
                Scope::Node(u64::MAX),
            )],
        };
        assert_eq!(Ledger::decode(&max_str.encode()), Some(max_str));

        let hundred = Ledger {
            entries: (0..100u32)
                .map(|i| {
                    entry(
                        Domain::Item,
                        (i % 65536) as u16,
                        Mod::Add(i as i64),
                        (i % 65536) as u16,
                        Scope::Operator,
                    )
                })
                .collect(),
        };
        assert_eq!(Ledger::decode(&hundred.encode()), Some(hundred));
    }

    /// L10: bad magic, truncation at every field boundary of entry 0,
    /// domain=16, trailing bytes, and an over-long string len all refuse
    /// whole — None, never a partial ledger.
    #[test]
    fn malformed_ledgers_are_refused_whole() {
        let good = Ledger {
            entries: vec![entry(Domain::Town, 9, Mod::Add(1), 1, Scope::Global)],
        }
        .encode();

        let mut bad_magic = good.clone();
        bad_magic[0] ^= 0xff;
        assert!(Ledger::decode(&bad_magic).is_none(), "bad magic");

        // Truncate at every prefix length up to (but not including) the
        // full buffer: every field boundary of entry 0 is covered.
        for len in 0..good.len() {
            assert!(Ledger::decode(&good[..len]).is_none(), "truncated at {len}");
        }

        let mut trailing = good.clone();
        trailing.push(0);
        assert!(Ledger::decode(&trailing).is_none(), "trailing byte");

        let mut bad_domain = good.clone();
        bad_domain[5] = 16;
        assert!(Ledger::decode(&bad_domain).is_none(), "domain=16");

        let over_long = Ledger {
            entries: vec![entry(Domain::Town, 1, Mod::ReplaceStr("x".repeat(257)), 1, Scope::Global)],
        };
        // Hand-build past the constructor: force a len field of 257 with
        // only 257 bytes actually following (the len check must fire before
        // any read-past-end could).
        assert!(Ledger::decode(&over_long.encode()).is_none(), "over-long string len");
    }

    /// Resolver semantics: priority wins, ties break to the later entry,
    /// `Node` scope is invisible under a different seed, `Remove` masks
    /// lower-priority entries, and `Add`/`MulPmy` arithmetic is exact.
    #[test]
    fn resolver_semantics() {
        let mut l = Ledger::default();
        l.append(entry(Domain::Town, 1, Mod::ReplaceStr("low".into()), 1, Scope::Global));
        l.append(entry(Domain::Town, 1, Mod::ReplaceStr("high".into()), 5, Scope::Global));
        assert_eq!(l.resolve_str(Domain::Town, 1, 0), Some("high"), "priority wins");

        let mut tie = Ledger::default();
        tie.append(entry(Domain::Town, 2, Mod::ReplaceStr("first".into()), 3, Scope::Global));
        tie.append(entry(Domain::Town, 2, Mod::ReplaceStr("second".into()), 3, Scope::Global));
        assert_eq!(tie.resolve_str(Domain::Town, 2, 0), Some("second"), "later entry breaks tie");

        let mut node = Ledger::default();
        node.append(entry(Domain::Sky, 1, Mod::ReplaceStr("node-truth".into()), 1, Scope::Node(42)));
        assert_eq!(node.resolve_str(Domain::Sky, 1, 42), Some("node-truth"));
        assert_eq!(node.resolve_str(Domain::Sky, 1, 43), None, "different seed can't see it");

        let mut removed = Ledger::default();
        removed.append(entry(Domain::Item, 1, Mod::Add(100), 1, Scope::Global));
        removed.append(entry(Domain::Item, 1, Mod::Remove, 5, Scope::Global));
        assert_eq!(removed.resolve_i64(Domain::Item, 1, 0, 10), 10, "remove masks the lower-priority add");

        let mut under = Ledger::default();
        under.append(entry(Domain::Item, 2, Mod::Add(100), 1, Scope::Global));
        under.append(entry(Domain::Item, 2, Mod::Remove, 0, Scope::Global));
        assert_eq!(under.resolve_i64(Domain::Item, 2, 0, 10), 110, "a lower-priority remove does not mask a higher add");

        let mut add = Ledger::default();
        add.append(entry(Domain::Item, 3, Mod::Add(-7), 1, Scope::Global));
        assert_eq!(add.resolve_i64(Domain::Item, 3, 0, 20), 13);

        let mut pmy = Ledger::default();
        pmy.append(entry(Domain::Item, 4, Mod::MulPmy(15_000), 1, Scope::Global));
        assert_eq!(pmy.resolve_pmy(Domain::Item, 4, 0, 200), 300, "150% of 200 is 300");

        let absent = Ledger::default();
        assert_eq!(absent.resolve_i64(Domain::Item, 99, 0, 5), 5);

        // The choice, side by side. Same two events, same key: Add overrides,
        // Accumulate tallies. This is the whole point of the second variant.
        let mut overrides = Ledger::default();
        overrides.append(entry(Domain::Item, 50, Mod::Add(10), 0, Scope::Global));
        overrides.append(entry(Domain::Item, 50, Mod::Add(10), 0, Scope::Global));
        assert_eq!(overrides.resolve_i64(Domain::Item, 50, 0, 0), 10, "Add replaces");

        let mut tallies = Ledger::default();
        tallies.append(entry(Domain::Item, 50, Mod::Accumulate(10), 0, Scope::Global));
        tallies.append(entry(Domain::Item, 50, Mod::Accumulate(10), 0, Scope::Global));
        assert_eq!(tallies.resolve_i64(Domain::Item, 50, 0, 0), 20, "Accumulate sums");

        // Priority does NOT gate a tally — that is what distinguishes it.
        let mut mixed_priority = Ledger::default();
        mixed_priority.append(entry(Domain::Item, 51, Mod::Accumulate(3), 99, Scope::Global));
        mixed_priority.append(entry(Domain::Item, 51, Mod::Accumulate(4), 0, Scope::Global));
        assert_eq!(mixed_priority.resolve_i64(Domain::Item, 51, 0, 0), 7, "low priority still counts");

        // The two compose: the winning Add sets the standing value, surviving
        // tallies ride on top of it.
        let mut both = Ledger::default();
        both.append(entry(Domain::Item, 52, Mod::Add(100), 5, Scope::Global));
        both.append(entry(Domain::Item, 52, Mod::Add(200), 1, Scope::Global));
        both.append(entry(Domain::Item, 52, Mod::Accumulate(7), 0, Scope::Global));
        assert_eq!(both.resolve_i64(Domain::Item, 52, 0, 0), 107, "highest Add wins, tally rides");

        // Remove still masks both kinds at or below its priority, and entries
        // that beat it still count — the erase primitive is not weakened.
        let mut masked = Ledger::default();
        masked.append(entry(Domain::Item, 53, Mod::Accumulate(5), 0, Scope::Global));
        masked.append(entry(Domain::Item, 53, Mod::Remove, 3, Scope::Global));
        assert_eq!(masked.resolve_i64(Domain::Item, 53, 0, 0), 0, "Remove masks the tally");
        masked.append(entry(Domain::Item, 53, Mod::Accumulate(9), 4, Scope::Global));
        assert_eq!(masked.resolve_i64(Domain::Item, 53, 0, 0), 9, "a tally above the mask survives");

        // Scope still gates a tally.
        let mut scoped = Ledger::default();
        scoped.append(entry(Domain::Item, 54, Mod::Accumulate(6), 0, Scope::Node(1)));
        assert_eq!(scoped.resolve_i64(Domain::Item, 54, 1, 0), 6);
        assert_eq!(scoped.resolve_i64(Domain::Item, 54, 2, 0), 0, "another node cannot see it");

        // A runaway tally saturates rather than wrapping (L10: never silent).
        let mut runaway = Ledger::default();
        for _ in 0..4 {
            runaway.append(entry(Domain::Item, 55, Mod::Accumulate(i64::MAX), 0, Scope::Global));
        }
        assert_eq!(runaway.resolve_i64(Domain::Item, 55, 0, 0), i64::MAX);
        assert_eq!(absent.resolve_pmy(Domain::Item, 99, 0, 5), 5);
        assert_eq!(absent.resolve_str(Domain::Item, 99, 0), None);
    }

    /// Atomic save/load round-trip through a tempdir.
    #[test]
    fn atomic_save_load_round_trip() {
        let dir = std::env::temp_dir().join("forge-mud-v3-overlay-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("ledger.ovl");
        let _ = std::fs::remove_file(&path);

        let empty = Ledger::load(&path).expect("absent file is an empty ledger, not an error");
        assert_eq!(empty, Ledger::default());

        let mut l = Ledger::default();
        l.append(entry(Domain::Boss, 1, Mod::Add(3), 1, Scope::Global));
        l.save(&path).expect("atomic save");
        let loaded = Ledger::load(&path).expect("round-trip load");
        assert_eq!(loaded, l);

        std::fs::remove_file(&path).ok();
        std::fs::remove_dir(&dir).ok();
    }

    /// `Domain::try_from` refuses every tag past the packet-stable range.
    #[test]
    fn domain_try_from_refuses_out_of_range() {
        assert_eq!(Domain::try_from(15), Ok(Domain::Zone));
        assert_eq!(Domain::try_from(16), Ok(Domain::Action));
        assert_eq!(Domain::try_from(17), Ok(Domain::Archetype));
        assert!(Domain::try_from(18).is_err());
        assert!(Domain::try_from(255).is_err());
    }
}
