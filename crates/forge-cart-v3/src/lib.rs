//! Cartridge packing and sealing — RON-based cart domain.
//!
//! A sealed cart is a binary file with structure:
//!   - Header (41 bytes): magic (4) + version (1) + body_len (4) + hash (32)
//!   - Body: RON-serialized payload
//!
//! All invariants are verified on load; any mismatch (magic, version, length,
//! hash, parse) refuses the whole cart with a worded error.

use std::fmt;

pub mod assets;
pub mod bake;
pub mod dialogue_tape;
pub mod lint;
pub mod npe;
pub mod texpack;
pub mod weaver_arbiter;
pub mod zone_bake;
/// Derive-rule falsification against real pixels. Offline bake only — behind
/// the `texbake` feature so no runtime crate links an image decoder.
#[cfg(feature = "texbake")]
pub mod texprove;

use assets::AssetCache;

/// Magic bytes "CART" (0x43415254 in LE).
const CART_MAGIC: u32 = 0x5443_4152; // "CART" in little-endian
const CART_FORMAT_VERSION: u8 = 1;
const CART_HEADER_SIZE: usize = 4 + 1 + 4 + 32; // magic + version + len + hash

/// Serde-compatible cart body. The concrete payload lives here.
/// Extended to handle NpeCart structure (L05 one-home law) and asset rows (L07 bijection).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CartBody {
    /// Backward compat: legacy simple items field.
    #[serde(default)]
    pub items: Vec<String>,
    /// NPE cartridge structure - when populated, items is ignored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub npe_cart: Option<ron::Value>,
    /// Asset rows: sprite atlases. Default empty to preserve backward compat (L07 bijection).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sprites: Vec<assets::SpriteAtlasRow>,
    /// Asset rows: generation prompts. Default empty to preserve backward compat (L07 bijection).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompts: Vec<assets::PromptRow>,
    /// Asset rows: ledger entries. Default empty to preserve backward compat (L07 bijection).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ledger: Vec<assets::AssetLedgerRow>,
    /// Asset cache: sprite pixel data indexed by sprite ID.
    /// Populated during bake and loaded at game startup.
    #[serde(default, skip_serializing_if = "AssetCache::is_empty")]
    pub asset_cache: assets::AssetCache,
    /// Baked geom surfaces: packed Pexil lattices + annotation tables
    /// (ORACLE-C spec sec-2). Default empty to preserve backward compat (L07).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geom_fields: Vec<assets::GeomFieldRow>,
    /// Baked scene timelines: raw UMP event bytes + group bindings
    /// (scene-convergence lock). Default empty to preserve backward compat (L07).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timelines: Vec<assets::TimelineRow>,
    /// The title's two faces + bench voice (base title law). Default None
    /// to preserve backward compat (L07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<assets::TitleRow>,
    /// One playthrough's dialogue tape (authored + speculated rows). Default
    /// None to preserve backward compat (L07).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dialogue: Option<dialogue_tape::DialogueTape>,
}

impl Default for CartBody {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            asset_cache: assets::AssetCache::new(),
            geom_fields: Vec::new(),
            timelines: Vec::new(),
            title: None,
            dialogue: None,
        }
    }
}

impl CartBody {
    /// Accessor: deserialize npe_cart as a typed NpeCart struct.
    /// Returns None if npe_cart is None. Returns an error if deserialization fails.
    pub fn npe(&self) -> Option<Result<npe::NpeCart, ron::error::Error>> {
        self.npe_cart.as_ref().map(|v| v.clone().into_rust())
    }
}

/// Refusal reason — all-or-nothing, never partial.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CartRefusal {
    /// Magic word is wrong.
    MagicMismatch {
        /// Expected magic value.
        expected: u32,
        /// Found magic value.
        found: u32,
    },
    /// Format version is unsupported.
    VersionMismatch {
        /// Expected version.
        expected: u8,
        /// Found version.
        found: u8,
    },
    /// Declared body length doesn't match.
    LengthMismatch {
        /// Expected body length.
        expected: usize,
        /// Actual body length.
        found: usize,
    },
    /// BLAKE3 hash of body doesn't match sealed hash.
    HashMismatch {
        /// Expected hash.
        expected: String,
        /// Computed hash.
        found: String,
    },
    /// RON parse failed.
    ParseError(String),
    /// File too short for header.
    FileTooShort {
        /// Minimum required size.
        min_size: usize,
        /// Actual file size.
        found: usize,
    },
}

impl fmt::Display for CartRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CartRefusal::MagicMismatch { expected, found } => {
                write!(
                    f,
                    "the magic word is wrong (expected 0x{:08x}, found 0x{:08x})",
                    expected, found
                )
            }
            CartRefusal::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "the format version is wrong (expected {}, found {})",
                    expected, found
                )
            }
            CartRefusal::LengthMismatch { expected, found } => {
                write!(
                    f,
                    "the seal does not hold (declared body length {}, but file has {})",
                    expected, found
                )
            }
            CartRefusal::HashMismatch { expected, found } => {
                write!(
                    f,
                    "the seal does not hold (hash mismatch: expected {}, found {})",
                    expected, found
                )
            }
            CartRefusal::ParseError(msg) => {
                write!(f, "the cart body is corrupted (parse error: {})", msg)
            }
            CartRefusal::FileTooShort { min_size, found } => {
                write!(
                    f,
                    "the cart file is truncated (need at least {} bytes, found {})",
                    min_size, found
                )
            }
        }
    }
}

/// Seal a cart body: validate it parses, then return header + body bytes.
/// Any validation failure returns a refusal (not a panic).
pub fn seal(body: &CartBody) -> Result<Vec<u8>, CartRefusal> {
    // Serialize body to RON.
    let ron_str = ron::to_string(body).map_err(|e| CartRefusal::ParseError(e.to_string()))?;
    let body_bytes = ron_str.into_bytes();
    let body_len = body_bytes.len() as u32;

    // Hash the body.
    let body_hash = blake3::hash(&body_bytes);

    // Construct header: magic + version + body_len + hash.
    let mut header = Vec::with_capacity(CART_HEADER_SIZE);
    header.extend_from_slice(&CART_MAGIC.to_le_bytes());
    header.push(CART_FORMAT_VERSION);
    header.extend_from_slice(&body_len.to_le_bytes());
    header.extend_from_slice(body_hash.as_bytes());

    // Return header + body.
    let mut result = header;
    result.extend_from_slice(&body_bytes);
    Ok(result)
}

/// Load a cart from sealed bytes. Verifies magic, version, length, and hash.
/// Any mismatch returns a refusal (not a panic).
pub fn load(bytes: &[u8]) -> Result<CartBody, CartRefusal> {
    // Check minimum header size.
    if bytes.len() < CART_HEADER_SIZE {
        return Err(CartRefusal::FileTooShort {
            min_size: CART_HEADER_SIZE,
            found: bytes.len(),
        });
    }

    // Parse header.
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let version = bytes[4];
    let body_len = u32::from_le_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]) as usize;
    let stored_hash_bytes = &bytes[9..41];

    // Verify magic.
    if magic != CART_MAGIC {
        return Err(CartRefusal::MagicMismatch {
            expected: CART_MAGIC,
            found: magic,
        });
    }

    // Verify version.
    if version != CART_FORMAT_VERSION {
        return Err(CartRefusal::VersionMismatch {
            expected: CART_FORMAT_VERSION,
            found: version,
        });
    }

    // Extract body.
    let body_start = CART_HEADER_SIZE;
    let body_end = body_start + body_len;
    if bytes.len() < body_end {
        return Err(CartRefusal::LengthMismatch {
            expected: body_len,
            found: bytes.len() - body_start,
        });
    }

    let body_bytes = &bytes[body_start..body_end];

    // Verify hash.
    let computed_hash = blake3::hash(body_bytes);
    let stored_array: [u8; 32] = stored_hash_bytes
        .try_into()
        .map_err(|_| CartRefusal::FileTooShort {
            min_size: CART_HEADER_SIZE,
            found: bytes.len(),
        })?;
    if stored_array != *computed_hash.as_bytes() {
        return Err(CartRefusal::HashMismatch {
            expected: blake3::Hash::from_bytes(stored_array).to_hex().to_string(),
            found: computed_hash.to_hex().to_string(),
        });
    }

    // Parse RON body.
    let body: CartBody =
        ron::from_str(&String::from_utf8_lossy(body_bytes))
            .map_err(|e| CartRefusal::ParseError(e.to_string()))?;

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L07 bijection test: seal(body) then load() returns the identical body.
    #[test]
    fn bijection_normal_body() {
        let body = CartBody {
            items: vec!["item1".to_string(), "item2".to_string()],
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");
        assert_eq!(body, loaded);
    }

    /// L07 bijection test: minimal empty body.
    #[test]
    fn bijection_empty_body() {
        let body = CartBody {
            items: Vec::new(),
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");
        assert_eq!(body, loaded);
    }

    /// L07 bijection test: body with edge characters.
    #[test]
    fn bijection_edge_characters() {
        let body = CartBody {
            items: vec![
                "normal".to_string(),
                "with\"quote".to_string(),
                "with\\slash".to_string(),
                "with\nnewline".to_string(),
            ],
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");
        assert_eq!(body, loaded);
    }

    /// L07 bijection test: a cart carrying a baked geom surface round-trips.
    #[test]
    fn bijection_geom_field() {
        let body = CartBody {
            geom_fields: vec![assets::GeomFieldRow {
                surface: "toll_gate".into(),
                band: "mud".into(),
                width: 71,
                height: 22,
                lattice: vec![255u8; 71 * 22],
                under_overrides: vec![(34, 2, 120)],
                annos: vec![assets::GeomAnnoTableRow {
                    c0: 38, r0: 4, c1: 39, r1: 5,
                    bind: None, sentinel: Some(245), out: Some("bell_pit".into()),
                }],
                legend: vec![('.', 130), ('+', 229)],
                source_hash: 0x1234_5678_9ABC_DEF0,
            }],
            timelines: vec![assets::TimelineRow {
                name: "descent".into(),
                events_raw: vec![0x00, 0x10, 0x00, 0x2A, 0x43, 0x90, 0x2D, 0x00],
                groups: vec![(3, "bells".into()), (4, "sky".into())],
                source_hash: 0x0DE5_CE17,
            }],
            ..Default::default()
        };
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");
        assert_eq!(body, loaded);
        loaded.geom_fields[0].validate().expect("round-tripped geom row must still validate");
        loaded.timelines[0].validate().expect("round-tripped timeline row must still validate");
    }

    /// L07 bijection test: a cart carrying a dialogue tape round-trips and
    /// the tape still validates after the trip.
    #[test]
    fn bijection_dialogue_tape() {
        let mut tape = dialogue_tape::DialogueTape::new(0xB0A7);
        tape.rows.push(dialogue_tape::DialogueRow {
            node: "gate:open".into(),
            speaker: "WARDEN".into(),
            text: "State your trade.".into(),
            provenance: dialogue_tape::LineProvenance::Speculated { seed: 42 },
            next_node: String::new(),
            weight: 2,
            stat_shift: vec![("nerve".into(), -50)],
        });
        let body = CartBody { dialogue: Some(tape), ..Default::default() };
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");
        assert_eq!(body, loaded);
        loaded.dialogue.as_ref().unwrap().validate().expect("round-tripped tape validates");
    }

    /// L18 sabotage test: flip a magic byte → assert the magic refusal.
    #[test]
    fn sabotage_magic_byte() {
        let body = CartBody {
            items: vec!["test".to_string()],
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let mut sealed = seal(&body).expect("seal should succeed");
        // Flip the first byte of the magic.
        sealed[0] ^= 0xFF;
        match load(&sealed) {
            Err(CartRefusal::MagicMismatch { .. }) => {
                // Expected.
            }
            other => panic!("expected MagicMismatch, got {:?}", other),
        }
    }

    /// L18 sabotage test: flip one body byte after sealing → assert the hash refusal.
    #[test]
    fn sabotage_body_byte() {
        let body = CartBody {
            items: vec!["test".to_string()],
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let mut sealed = seal(&body).expect("seal should succeed");
        // Flip a byte in the body (past the header).
        if sealed.len() > CART_HEADER_SIZE {
            sealed[CART_HEADER_SIZE] ^= 0xFF;
        }
        match load(&sealed) {
            Err(CartRefusal::HashMismatch { .. }) => {
                // Expected.
            }
            other => panic!("expected HashMismatch, got {:?}", other),
        }
    }

    /// L18 sabotage test: truncate the file → assert length/file truncation refusal.
    #[test]
    fn sabotage_truncate() {
        let body = CartBody {
            items: vec!["test".to_string()],
            npe_cart: None,
            sprites: Vec::new(),
            prompts: Vec::new(),
            ledger: Vec::new(),
            ..Default::default()
        };
        let sealed = seal(&body).expect("seal should succeed");
        // Truncate to half.
        let truncated = &sealed[0..sealed.len() / 2];
        match load(truncated) {
            Err(CartRefusal::FileTooShort { .. }) | Err(CartRefusal::LengthMismatch { .. }) => {
                // Expected: either too short for header or body doesn't match declared length.
            }
            other => panic!("expected FileTooShort or LengthMismatch, got {:?}", other),
        }
    }
}
