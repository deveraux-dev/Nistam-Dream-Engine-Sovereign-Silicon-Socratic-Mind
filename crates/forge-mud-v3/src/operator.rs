//! The operator — the harness identity the game binds to: a chosen name
//! (Operator-class, never "Claude"), a 13-moon birthday, and the state the
//! terminal earns: XP, deaths, the node seed, and ONE 8-byte position word.
//!
//! Save codec: `MUD3` magic + version + integers in **little-nistam** byte
//! order (nistam = Cree "first": least-significant byte FIRST — the tree's
//! one wire order, spoken through Crate Zero's own helpers, never a second
//! byte-order site). Every encode has a tested decode (L07); a malformed
//! save is refused whole, never partially applied.

use forge_core_v3::astrolabe::GenesisToken;
use forge_core_v3::ramus_prime::MortonKey5D;
use forge_core_v3::sprite_blob::{
    u16_from_nistam, u16_to_nistam, u32_from_nistam, u32_to_nistam, u64_from_nistam,
    u64_to_nistam,
};

use crate::skills::Skills;

/// Save magic — distinct from every other `.forge` reader so a foreign
/// schema can never silently load here.
pub const SAVE_MAGIC: [u8; 4] = *b"MUD3";
/// Save schema version. v2 added the deeds ledger + world bias (the WCE
/// seam: play writes the world, invisibly). v3 added the ironroot weld:
/// faction standings + watch heat (2026-08-11 ironroot-edict drain). v4
/// added the seven arts (`skills`, weld G, 2026-08-11). v5 added the magic
/// wire (2026-08-24): the worn form byte + carried noise permyriad — a v4
/// save decodes with both defaulted (Mortal, silent). v6+ is refused as any
/// unknown version always has been.
pub const SAVE_VERSION: u32 = 5;
/// The last version whose save bytes still decode (with skills defaulted).
const MIN_READABLE_VERSION: u32 = 3;

/// The deed families — the WCE verb-tag lineage (v2 forge-consequence:
/// every verb group fires its own consequence curve). The PLAYER NEVER SEES
/// these (Sean 2026-08-11: "the player should never know they are making a
/// mud") — they only feel the worlds lean.
pub const DEED_FAMILIES: usize = 4;
/// Deed family: force (the strike current).
pub const DEED_FORCE: usize = 0;
/// Deed family: craft (the forge current).
pub const DEED_CRAFT: usize = 1;
/// Deed family: gather (the harvest current).
pub const DEED_GATHER: usize = 2;
/// Deed family: voice (the speech current).
pub const DEED_VOICE: usize = 3;
/// Bias value meaning "no dominant current yet".
pub const BIAS_NONE: u8 = 4;
/// Moons in the year — the birthday's first axis.
pub const MOON_COUNT: u8 = 13;
/// Days in a moon — the birthday's second axis.
pub const MOON_DAYS: u8 = 28;

/// FNV-1a 64 over arbitrary bytes — the tree's ONE game-seed dealer (L05:
/// first home, not a second one — Crate Zero's `BrutalHash` is deliberately
/// a hashless carrier, blake3 being firewalled into forge-vcs-v3 per
/// spine.rs:14-23, and integrity hashing is NOT this: this deals lore,
/// deterministically, forever).
pub fn seed_hash(parts: &[&[u8]]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for part in parts {
        for &b in *part {
            h ^= b as u64;
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
        // A separator per part so ("ab","c") never equals ("a","bc").
        h ^= 0x1f;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // Avalanche tail (the v2 EventLedger's own finisher, drained): raw FNV low
    // bits form a closed residue machine, so `hash % small_n` dealt 1-in-8
    // seed pairs the IDENTICAL world — the world test caught it live.
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    h
}

/// The operator: identity + everything the terminal has earned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operator {
    /// The harness name the player chose at the door.
    pub name: String,
    /// Birth moon, 0..13 on the 13-moon calendar.
    pub moon: u8,
    /// Birth day, 0..28 within the moon.
    pub day: u8,
    /// Experience — terminal bytes earned through the game's own door.
    pub xp: u64,
    /// Times the operator has died (each one reseeded the world).
    pub deaths: u32,
    /// The current node's seed — the world, theme, vibe and grind.
    pub node_seed: u64,
    /// Position: ONE 8-byte word, five 12-bit axes (x, y, z, t, s).
    pub pos: MortonKey5D,
    /// The deeds ledger — verb acts per family (WCE seam). Never surfaced.
    pub deeds: [u32; DEED_FAMILIES],
    /// The world bias snapshotted at the LAST reseed: the dominant deed
    /// family the next node leans toward, or [`BIAS_NONE`]. Never surfaced.
    pub bias: u8,
    /// Standing with THIS node's five factions (ironroot ladder). A death
    /// reseeds the node and the standings with it — new people, clean slate.
    pub standings: [i16; crate::consequence::FACTION_COUNT],
    /// Watch heat: how long the node's law still hunts the operator, in
    /// command ticks. Decays one per command; a reseed clears it.
    pub heat: u16,
    /// The seven arts (weld G, v4). A v3 save decodes with this defaulted.
    pub skills: Skills,
    /// The worn body (magic wire, v5): `magic::umwelt::Form::ALL` index.
    /// `body.is.the.sensor` — this byte IS the perceptual apparatus.
    pub form: u8,
    /// Carried noise, permyriad of total muting (magic wire, v5). Singing
    /// raises it; the room settles it one step per command. HEAR face only.
    pub muted_q: u16,
}

impl Operator {
    /// Derives a canonical Nostr 3-tuple `GenesisToken` from identity fields:
    /// - `pubkey`: 32-byte BIP-340 compatible key folded from operator name
    /// - `lst_cdeg`: Local sidereal time in centidegrees [0, 36000), calibrated to moon and day
    /// - `discipline_mask`: Bitmask of active disciplines (bits 0..7)
    pub fn token_from_identity(name: &str, moon: u8, day: u8, discipline_mask: u8) -> GenesisToken {
        let moon = moon % MOON_COUNT;
        let day = day % MOON_DAYS;
        let name_bytes = name.trim().as_bytes();

        let mut pubkey = [0u8; 32];
        let mut h = seed_hash(&[name_bytes, b"pubkey_fold_x32"]);
        let mut i = 0;
        while i < 32 {
            pubkey[i] = (h & 0xFF) as u8;
            h = h.rotate_left(5) ^ (i as u64).wrapping_mul(0x0000_0100_0000_01B3);
            i += 1;
        }

        let day_idx = (moon as u32 * MOON_DAYS as u32) + day as u32;
        let lst_cdeg = ((day_idx * 36000) / (MOON_COUNT as u32 * MOON_DAYS as u32)) as u16 % 36000;

        GenesisToken {
            pubkey,
            lst_cdeg,
            discipline_mask,
        }
    }

    /// A new operator birthed directly from a Nostr 3-tuple `GenesisToken`.
    /// The node seed is dealt by `crate::genesis::node_seed(token)` (the Astrolabe fold).
    pub fn birth_with_token(name: &str, moon: u8, day: u8, token: &GenesisToken) -> Option<Self> {
        if name.trim().is_empty() {
            return None;
        }
        let moon = moon % MOON_COUNT;
        let day = day % MOON_DAYS;
        let node_seed = crate::genesis::node_seed(token);
        Some(Self {
            name: name.trim().to_string(),
            moon,
            day,
            xp: 0,
            deaths: 0,
            node_seed,
            pos: MortonKey5D::encode([0, 0, 0, 0, 0]),
            deeds: [0; DEED_FAMILIES],
            bias: BIAS_NONE,
            standings: [0; crate::consequence::FACTION_COUNT],
            heat: 0,
            skills: Skills::default(),
            form: 0,
            muted_q: 0,
        })
    }

    /// A new operator at their birth node. The birth seed is the identity
    /// itself — routed through the Sevenfold GenesisToken primitive (`genesis::node_seed`).
    pub fn birth(name: &str, moon: u8, day: u8) -> Option<Self> {
        if name.trim().is_empty() {
            return None;
        }
        let token = Self::token_from_identity(name, moon, day, 0b0111_1111);
        Self::birth_with_token(name, moon, day, &token)
    }

    /// Oath Discipline choices that can actually reach the world seed. The
    /// mask's top bit is reserved (`forge_core_v3::astrolabe`:92,99) and
    /// `genesis::sevenfold_anchors` only ever reads bits 0..7, so seven of the
    /// cart's eight authored choices are representable and the eighth is not.
    pub const DISCIPLINE_CHOICE_MAX: u8 = 7;

    /// A new operator whose ONE themed identity pick is the Oath Discipline —
    /// craft and wound, never sky (the canon revision's whole point). `choice`
    /// indexes the cart's `BirthRite::craft_pick::choices`; it becomes the one
    /// set bit of the genesis mask, so a different discipline is a different
    /// world seed and a different dungeon anchor.
    ///
    /// Refuses `choice >= DISCIPLINE_CHOICE_MAX` rather than folding it — the
    /// sevenfold row lookup wraps (`genesis::discipline_of` is `bit % 7`), and
    /// an eighth discipline silently aliased onto the first would be two
    /// identities wearing one anchor.
    pub fn birth_with_discipline(name: &str, moon: u8, day: u8, choice: u8) -> Option<Self> {
        if choice >= Self::DISCIPLINE_CHOICE_MAX {
            return None;
        }
        let token = Self::token_from_identity(name, moon, day, 1 << choice);
        Self::birth_with_token(name, moon, day, &token)
    }

    /// Recover the canonical GenesisToken for this operator.
    pub fn genesis_token(&self, discipline_mask: u8) -> GenesisToken {
        Self::token_from_identity(&self.name, self.moon, self.day, discipline_mask)
    }

    /// Calculate the sevenfold dungeon / landmark anchors on the 81x81 lattice
    /// for this operator's birth sky (at given latitude in cdeg, default 5354 for Edmonton).
    pub fn genesis_anchors(&self, latitude_cdeg: i32, discipline_mask: u8) -> [Option<(u16, u16)>; 7] {
        let token = self.genesis_token(discipline_mask);
        crate::genesis::sevenfold_anchors(&token, latitude_cdeg)
    }

    /// Primary correspondence discipline the operator was birthed under.
    pub fn birth_discipline(&self) -> crate::hermetics::Correspondence {
        let token = self.genesis_token(0b0111_1111);
        crate::genesis::discipline_of(token.natal_star_idx())
    }

    /// The Camelot key of the operator's natal star — the key everything they
    /// sing sounds in. Same mask as [`birth_discipline`] so the two read one sky.
    pub fn natal_key(&self) -> forge_harmonics::CamelotKey {
        let token = self.genesis_token(0b0111_1111);
        forge_harmonics::CamelotKey::from_star_idx(token.natal_star_idx())
            .unwrap_or(forge_harmonics::CamelotKey::DEFAULT_8B)
    }

    /// Death: the world reseeds — new node, new theme, new grind. XP is
    /// kept (the terminal earned it); the map and its vibe are gone. The
    /// WCE beat: the dominant deed family is snapshotted as the new node's
    /// bias — the world you get next is the one you played into being, and
    /// nothing ever says so.
    pub fn die(&mut self) {
        self.deaths += 1;
        self.node_seed =
            seed_hash(&[&u64_to_nistam(self.node_seed), &u32_to_nistam(self.deaths)]);
        let max = self.deeds.iter().copied().max().unwrap_or(0);
        self.bias = if max == 0 {
            BIAS_NONE
        } else {
            self.deeds.iter().position(|&d| d == max).unwrap_or(BIAS_NONE as usize) as u8
        };
        // The new node's factions have never met the operator, and its watch
        // holds no warrant — standings and heat die with the old world. The
        // noise dies with the old room too; the worn body crosses over.
        self.standings = [0; crate::consequence::FACTION_COUNT];
        self.heat = 0;
        self.muted_q = 0;
    }

    /// Fold one resolution's LOCAL share onto the walker: `shadow_pressure`
    /// raises watch heat (the law's warrant on this operator), and a
    /// `faction_pressure_shift` moves standing with the event's owner by
    /// `ledger_control`. The other eight `ResolutionDelta` fields are world
    /// state, not walker state — they leave as manifold impulses evaluated at
    /// `pos`, and are deliberately absent here so `SAVE_VERSION` holds.
    pub fn apply_resolution(
        &mut self,
        delta: &crate::dm::ResolutionDelta,
        faction_owner: Option<usize>,
    ) {
        self.heat = self.heat.saturating_add_signed(delta.shadow_pressure as i16);
        if delta.faction_pressure_shift {
            if let Some(f) = faction_owner.filter(|f| *f < crate::consequence::FACTION_COUNT) {
                self.standings[f] = self.standings[f].saturating_add(delta.ledger_control as i16);
            }
        }
    }

    /// Encode to save bytes: magic, version, then every field little-nistam.
    pub fn encode(&self) -> Vec<u8> {
        let name = self.name.as_bytes();
        let mut out = Vec::with_capacity(4 + 4 + 2 + name.len() + 1 + 1 + 8 + 4 + 8 + 8);
        out.extend_from_slice(&SAVE_MAGIC);
        out.extend_from_slice(&u32_to_nistam(SAVE_VERSION));
        out.extend_from_slice(&u16_to_nistam(name.len() as u16));
        out.extend_from_slice(name);
        out.push(self.moon);
        out.push(self.day);
        out.extend_from_slice(&u64_to_nistam(self.xp));
        out.extend_from_slice(&u32_to_nistam(self.deaths));
        out.extend_from_slice(&u64_to_nistam(self.node_seed));
        out.extend_from_slice(&u64_to_nistam(self.pos.0));
        for d in self.deeds {
            out.extend_from_slice(&u32_to_nistam(d));
        }
        out.push(self.bias);
        for s in self.standings {
            out.extend_from_slice(&u16_to_nistam(s as u16));
        }
        out.extend_from_slice(&u16_to_nistam(self.heat));
        for v in self.skills.value {
            out.extend_from_slice(&u16_to_nistam(v));
        }
        for u in self.skills.uses {
            out.extend_from_slice(&u64_to_nistam(u));
        }
        out.push(self.form);
        out.extend_from_slice(&u16_to_nistam(self.muted_q));
        out
    }

    /// Decode save bytes. Bad magic, wrong version, short buffer or a
    /// non-UTF8 name refuse WHOLE — `None`, never a partial operator.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let take = |at: &mut usize, n: usize| -> Option<&[u8]> {
            let s = bytes.get(*at..*at + n)?;
            *at += n;
            Some(s)
        };
        if take(&mut at, 4)? != SAVE_MAGIC {
            return None;
        }
        let version = u32_from_nistam(take(&mut at, 4)?, 0);
        if version < MIN_READABLE_VERSION || version > SAVE_VERSION {
            return None;
        }
        let name_len = u16_from_nistam(take(&mut at, 2)?, 0) as usize;
        let name = std::str::from_utf8(take(&mut at, name_len)?).ok()?.to_string();
        if name.trim().is_empty() {
            return None;
        }
        let moon = take(&mut at, 1)?[0];
        let day = take(&mut at, 1)?[0];
        let xp = u64_from_nistam(take(&mut at, 8)?, 0);
        let deaths = u32_from_nistam(take(&mut at, 4)?, 0);
        let node_seed = u64_from_nistam(take(&mut at, 8)?, 0);
        let pos = MortonKey5D(u64_from_nistam(take(&mut at, 8)?, 0));
        let mut deeds = [0u32; DEED_FAMILIES];
        for d in &mut deeds {
            *d = u32_from_nistam(take(&mut at, 4)?, 0);
        }
        let bias = take(&mut at, 1)?[0];
        let mut standings = [0i16; crate::consequence::FACTION_COUNT];
        for s in &mut standings {
            *s = u16_from_nistam(take(&mut at, 2)?, 0) as i16;
        }
        let heat = u16_from_nistam(take(&mut at, 2)?, 0);
        let skills = if version >= 4 {
            let mut value = [0u16; 7];
            for v in &mut value {
                *v = u16_from_nistam(take(&mut at, 2)?, 0);
            }
            let mut uses = [0u64; 7];
            for u in &mut uses {
                *u = u64_from_nistam(take(&mut at, 8)?, 0);
            }
            Skills { value, uses }
        } else {
            Skills::default()
        };
        let (form, muted_q) = if version >= 5 {
            let form = take(&mut at, 1)?[0];
            let muted_q = u16_from_nistam(take(&mut at, 2)?, 0);
            (form, muted_q)
        } else {
            (0, 0)
        };
        if at != bytes.len() || moon >= MOON_COUNT || day >= MOON_DAYS || bias > BIAS_NONE {
            return None;
        }
        if crate::magic::umwelt::Form::from_u8(form).is_none()
            || muted_q as i64 > crate::magic::umwelt::AUTHORED_Q
        {
            return None;
        }
        Some(Self {
            name, moon, day, xp, deaths, node_seed, pos, deeds, bias, standings, heat, skills,
            form, muted_q,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::dm::{resolution_effects, ResolutionMode};

    /// The local wire: a Kill's `shadow_pressure` raises watch heat.
    #[test]
    fn a_resolution_raises_the_walkers_watch_heat() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        assert_eq!(op.heat, 0, "a fresh walker holds no warrant");
        op.apply_resolution(&resolution_effects(ResolutionMode::Kill), None);
        assert_eq!(op.heat, 6, "Kill's shadow_pressure is 6");
    }

    /// `faction_pressure_shift` gates, `ledger_control` sizes: Expose moves
    /// standing with the event's owner by -8, and only that faction.
    #[test]
    fn an_exposed_event_moves_standing_with_its_owner_only() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.apply_resolution(&resolution_effects(ResolutionMode::Expose), Some(2));
        assert_eq!(op.standings[2], -8, "Expose's ledger_control is -8");
        for (i, s) in op.standings.iter().enumerate() {
            assert!(i == 2 || *s == 0, "faction {i} was not the owner");
        }
    }

    /// An unowned event shifts no standing, and a Spare (shift=false) never
    /// moves standing even when a faction owns the event.
    #[test]
    fn standing_moves_only_on_an_owned_shift() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.apply_resolution(&resolution_effects(ResolutionMode::Expose), None);
        assert!(op.standings.iter().all(|s| *s == 0), "no owner, no shift");
        op.apply_resolution(&resolution_effects(ResolutionMode::Spare), Some(1));
        assert_eq!(op.standings[1], 0, "Spare does not shift faction pressure");
    }

    /// Heat saturates instead of wrapping, and the eight world fields never
    /// reach the save: `SAVE_VERSION` bytes round-trip unchanged after a
    /// resolution.
    #[test]
    fn a_resolution_saturates_heat_and_never_widens_the_save() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.heat = u16::MAX;
        op.apply_resolution(&resolution_effects(ResolutionMode::Erase), None);
        assert_eq!(op.heat, u16::MAX, "heat saturates, never wraps");
        let bytes = op.encode();
        assert_eq!(Operator::decode(&bytes).unwrap(), op, "codec still a bijection");
    }

    /// L07 over the codec: interior, sentinel-ish and edge operators all
    /// survive encode→decode byte-exact.
    #[test]
    fn the_save_codec_is_a_bijection() {
        let cases = [
            Operator::birth("Operator", 3, 12).unwrap(),
            Operator::birth("O", 0, 0).unwrap(),
            Operator::birth("a longer operator name with spaces", 12, 27).unwrap(),
        ];
        for mut op in cases {
            op.xp = u64::MAX - 7;
            op.deaths = 3;
            op.die();
            op.pos = forge_core_v3::ramus_prime::MortonKey5D::encode([7, 3, 0, 0, 0]);
            // The ironroot weld's fields ride the same wire: negative
            // standings and both i16 edges must come home signed and exact.
            op.standings = [-800, 5, 0, i16::MIN, i16::MAX];
            op.heat = 777;
            op.skills.value = [1000, 0, 500, 150, 999, 1, 300];
            op.skills.uses = [40_350, 0, 900, 150, 12_000, 1, 3_000];
            let bytes = op.encode();
            assert_eq!(Operator::decode(&bytes).as_ref(), Some(&op));
        }
    }

    /// A v3 save (no skill bytes) still opens — skills come home
    /// `Skills::default()`, never a refusal (L07/L10 migration clause).
    #[test]
    fn v3_save_still_opens() {
        let op = Operator::birth("Operator", 3, 12).unwrap();
        let v5_bytes = op.encode();
        // A v3 image is the v5 image with version=3 and the trailing skill
        // bytes (7 x u16 + 7 x u64 = 70) plus the magic bytes (form u8 +
        // muted u16 = 3) lopped off.
        let mut v3_bytes = v5_bytes[..v5_bytes.len() - 73].to_vec();
        v3_bytes[4..8].copy_from_slice(&u32_to_nistam(3));
        let decoded = Operator::decode(&v3_bytes).expect("a v3 save must still decode");
        assert_eq!(decoded.skills, Skills::default());
        assert_eq!(decoded.name, op.name);
        assert_eq!(decoded.node_seed, op.node_seed);
    }

    /// A fresh v4 save round-trips bijectively, including `u16::MAX` and
    /// `u64::MAX` edges in the skill wire.
    #[test]
    fn v4_bijection_at_skill_edges() {
        let mut op = Operator::birth("Operator", 3, 12).unwrap();
        op.skills.value = [u16::MAX, 0, u16::MAX, 1, u16::MAX, 0, u16::MAX];
        op.skills.uses = [u64::MAX, 0, u64::MAX, 1, u64::MAX, 0, u64::MAX];
        let bytes = op.encode();
        assert_eq!(Operator::decode(&bytes).as_ref(), Some(&op));
    }

    /// A version past today's newest is refused whole, same as always.
    #[test]
    fn unknown_future_version_is_refused() {
        let op = Operator::birth("Operator", 3, 12).unwrap();
        let mut bytes = op.encode();
        bytes[4..8].copy_from_slice(&u32_to_nistam(SAVE_VERSION + 1));
        assert!(Operator::decode(&bytes).is_none());
    }

    /// Refusals are whole: bad magic, truncation, trailing garbage, and an
    /// empty name all yield None.
    #[test]
    fn malformed_saves_are_refused_whole() {
        let good = Operator::birth("Operator", 3, 12).unwrap().encode();
        assert!(Operator::decode(&good[1..]).is_none(), "shifted magic");
        assert!(Operator::decode(&good[..good.len() - 2]).is_none(), "truncated");
        let mut long = good.clone();
        long.push(0);
        assert!(Operator::decode(&long).is_none(), "trailing garbage");
        assert!(Operator::birth("   ", 1, 1).is_none(), "blank name at the door");
    }

    /// Identity is the seed: same name+birthday, same first node; a death
    /// moves the node deterministically.
    #[test]
    fn birth_is_deterministic_and_death_reseeds() {
        let a = Operator::birth("Operator", 3, 12).unwrap();
        let b = Operator::birth("Operator", 3, 12).unwrap();
        assert_eq!(a.node_seed, b.node_seed);
        let c = Operator::birth("Operator", 3, 13).unwrap();
        assert_ne!(a.node_seed, c.node_seed, "a day apart is a different sky");
        let mut d = a.clone();
        d.die();
        assert_ne!(d.node_seed, a.node_seed);
        let mut e = a.clone();
        e.die();
        assert_eq!(d.node_seed, e.node_seed, "the same death lands in the same node");
    }

    /// The cart schema's birth counts (moon_count, day_count) must always
    /// match MOON_COUNT and MOON_DAYS. This is NOT a general RON parser —
    /// it is a hand-rolled substring search over the static schema file,
    /// appropriate for a v3-authored config that is not adversarial input.
    /// A real RON parser would be over-engineered for this use case and is
    /// blocked by L19 (no dep-grab without ARCH000 nod for a ron crate).
    /// Full cart-loading infrastructure is future work.
    #[test]
    fn npe_base_cart_moon_and_day_counts_match_operator_constants() {
        use std::path::PathBuf;

        // Walk up from CARGO_MANIFEST_DIR to find the repo root (where .forge or carts exists),
        // mirroring main.rs:default_save_path's technique (main.rs:22-30).
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cart_path = loop {
            // Try to find carts/base/npe.base.ron.
            let candidate = dir.join("carts/base/npe.base.ron");
            if candidate.exists() {
                break candidate;
            }
            // Try to walk up past .forge.
            if !dir.pop() {
                panic!("could not resolve carts/base/npe.base.ron from CARGO_MANIFEST_DIR");
            }
        };

        let ron_text = std::fs::read_to_string(&cart_path)
            .unwrap_or_else(|e| panic!("failed to read cart file at {}: {}", cart_path.display(), e));

        // Hand-rolled substring search: find "moon_count: " and parse the number.
        let moon_count_str = ron_text
            .find("moon_count: ")
            .and_then(|pos| {
                let start = pos + "moon_count: ".len();
                let end = ron_text[start..].find(',').map(|i| start + i).unwrap_or(ron_text.len());
                ron_text[start..end].trim().parse::<u8>().ok()
            })
            .unwrap_or_else(|| panic!("could not parse moon_count from {}", cart_path.display()));

        // Hand-rolled substring search: find "day_count: " and parse the number.
        let day_count_str = ron_text
            .find("day_count: ")
            .and_then(|pos| {
                let start = pos + "day_count: ".len();
                let end = ron_text[start..].find(',').map(|i| start + i).unwrap_or(ron_text.len());
                ron_text[start..end].trim().parse::<u8>().ok()
            })
            .unwrap_or_else(|| panic!("could not parse day_count from {}", cart_path.display()));

        assert_eq!(moon_count_str, MOON_COUNT,
            "cart's moon_count must match operator.rs MOON_COUNT (line 48)");
        assert_eq!(day_count_str, MOON_DAYS,
            "cart's day_count must match operator.rs MOON_DAYS (line 50)");
    }

    /// The pick has to MOVE the world, or it is decoration: two operators
    /// identical but for their Oath Discipline must not share a world seed.
    #[test]
    fn a_different_discipline_is_a_different_world() {
        let a = Operator::birth_with_discipline("Morrow", 4, 12, 0).expect("valid birth");
        let b = Operator::birth_with_discipline("Morrow", 4, 12, 3).expect("valid birth");
        assert_ne!(
            a.node_seed, b.node_seed,
            "the one themed identity pick must reach the world seed"
        );
    }

    /// One discipline lights exactly its own dungeon anchor — not all seven.
    #[test]
    fn one_discipline_lights_exactly_one_anchor() {
        let op = Operator::birth_with_discipline("Morrow", 4, 12, 3).expect("valid birth");
        let anchors = op.genesis_anchors(5354, 1 << 3);
        let lit: Vec<usize> = anchors
            .iter()
            .enumerate()
            .filter_map(|(i, a)| a.map(|_| i))
            .collect();
        assert_eq!(lit, vec![3], "only the picked discipline's row may anchor");
    }

    /// The cart authors eight choices; the mask carries seven. The eighth
    /// REFUSES rather than aliasing onto the first (`discipline_of` is `% 7`).
    #[test]
    fn the_eighth_cart_discipline_refuses_instead_of_aliasing() {
        assert!(Operator::birth_with_discipline("Morrow", 4, 12, 6).is_some());
        assert!(
            Operator::birth_with_discipline("Morrow", 4, 12, 7).is_none(),
            "an eighth discipline folded onto the first is two identities on one anchor"
        );
        assert_eq!(
            format!("{:?}", crate::genesis::discipline_of(7)),
            format!("{:?}", crate::genesis::discipline_of(0)),
            "bit 7 wraps onto row 0 — the alias this refusal exists to prevent"
        );
    }

    /// `birth` keeps its standing all-rows-on constant — this weld is additive,
    /// so every existing operator's seed replays unchanged.
    #[test]
    fn plain_birth_still_deals_the_standing_full_mask() {
        let op = Operator::birth("Morrow", 4, 12).expect("valid birth");
        let full = Operator::token_from_identity("Morrow", 4, 12, 0b0111_1111);
        assert_eq!(op.node_seed, crate::genesis::node_seed(&full));
    }

    /// GenesisToken and Astrolabe integration: birth folds into valid anchors
    /// and a corresponding sevenfold discipline.
    #[test]
    fn operator_genesis_token_and_anchors_derive_cleanly() {
        let op = Operator::birth("Morrow", 4, 12).expect("valid birth");
        let token = op.genesis_token(0b0111_1111);
        assert_eq!(op.node_seed, crate::genesis::node_seed(&token));

        let anchors = op.genesis_anchors(5354, 0b0111_1111);
        for (i, a) in anchors.iter().enumerate() {
            assert!(a.is_some(), "anchor {i} must exist under full mask");
            let (x, y) = a.unwrap();
            assert!(x < crate::world::MAP_SIDE && y < crate::world::MAP_SIDE);
        }

        let disc = op.birth_discipline();
        assert!(disc.color_hex != 0);
    }
}
