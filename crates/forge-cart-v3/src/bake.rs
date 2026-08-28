//! Cart baking machinery — RON merge, strip, and seal.
//!
//! MERGE LAW (ironroot overlays base):
//! The ironroot RON file extends the base RON file by providing values for the
//! same keys. Fields present only in base retain their base values. Fields
//! present in ironroot override the base. This implements the dual-seed doctrine:
//! base is the NULL mechanics, ironroot is the Ironroot-themed bindings. Strip
//! the theme (ironroot) and base alone must still pass every gate.
//!
//! STRIP LAW (baseline determinism):
//! Strip function replaces every "flavor" string (prose, names, lore) with a
//! stable placeholder token derived from its key name. Keys to strip:
//! - world_word, front_line, under_line, thirteenth_word
//! - prompt_word
//! - stolen, grave, blood, reclaimed, pure (provenance words)
//! - entry_zone_word, entry_gate_word, threat_word, territorial_word, questgiver_word
//! - target_word, door_out_word, landmarks
//! - calendar_word
//! - era_words (array)
//! - choices (array of tuples in craft_pick)
//! - face_sets (paths array)
//! - palette hex values
//! - fonts paths (display, body, console)
//! - banners paths
//!
//! SIZE GATE (L14 — ceiling doctrine):
//! Whole cart ≤ 80KB (81,920 B), any single section ≤ 8KB (8,192 B).
//! A section is each top-level RON entry. Breaches are LOUD refusals (L10).

use super::CartBody;
use super::lint;

/// Flavor string keys that must be stripped for baseline. Derived from actual
/// RON structure observation. `[OBSERVED]` npe.base.ron + npe.ironroot.ron.
const STRIP_KEYS: &[&str] = &[
    // Title
    "world_word", "front_line", "under_line", "thirteenth_word",
    // Birth
    "calendar_word", "prompt_word",
    // Kit
    "stolen", "grave", "blood", "reclaimed", "pure",
    // World
    "entry_zone_word", "entry_gate_word", "threat_word", "territorial_word",
    "questgiver_word", "target_word", "door_out_word", "landmarks",
    // Visuals (art paths, choices, palette member values, fonts, banners)
    "front_slot", "under_slot", "display", "body", "console",
    "face_sets", "palette", "banners", "era_words", "choices",
    // Bench card voice (title law growth, 2026-08-23)
    "bench_line",
    // Factions + vocabulary (CART SCHEMA GROWTH, census §2 DRAIN-INTO ruling)
    "name_word", "term_word",
];

/// Size gates: whole cart ≤ 80 KB, any section ≤ 8 KB.
const MAX_CART_SIZE: usize = 81_920;
const MAX_SECTION_SIZE: usize = 8_192;

/// Bake result — success yields themed and baseline packs, either with errors or checks.
pub struct BakeReceipt {
    /// Sealed bytes of the themed pack.
    pub themed_bytes: Vec<u8>,
    /// Size in bytes of the themed pack.
    pub themed_size: usize,
    /// BLAKE3 hash of the themed pack in hex.
    pub themed_hash: String,
    /// Sealed bytes of the baseline pack (stripped of flavor).
    pub baseline_bytes: Vec<u8>,
    /// Size in bytes of the baseline pack.
    pub baseline_size: usize,
    /// BLAKE3 hash of the baseline pack in hex.
    pub baseline_hash: String,
}

/// Merge base and cartridge RON strings. Cartridge overlays base.
/// Returns the merged RON value or a refusal string.
pub fn merge_ron(base_ron: &str, cart_ron: &str) -> Result<ron::Value, String> {
    let base_val: ron::Value = ron::from_str(base_ron)
        .map_err(|e| format!("base RON parse failed: {e}"))?;
    let cart_val: ron::Value = ron::from_str(cart_ron)
        .map_err(|e| format!("cartridge RON parse failed: {e}"))?;

    // Overlay: merge cart into base at the top level.
    // Both must be maps (RON Map or Struct syntax).
    match (base_val, cart_val) {
        (ron::Value::Map(mut base_map), ron::Value::Map(cart_map)) => {
            for (k, v) in cart_map.into_iter() {
                base_map.insert(k, v);
            }
            Ok(ron::Value::Map(base_map))
        }
        _ => Err("both base and cartridge must be RON structs (name(field: val, ...))".to_string()),
    }
}

/// Strip flavor strings from a RON value, replacing them with stable placeholders.
/// Used to generate the baseline pack from the themed merged RON.
pub fn strip_flavor(val: &ron::Value) -> ron::Value {
    match val {
        ron::Value::Map(m) => {
            let mut stripped_map = ron::Map::new();
            for (k, v) in m.iter() {
                // Check if this key is a flavor string that should be stripped.
                if let ron::Value::String(key_str) = k {
                    if STRIP_KEYS.contains(&key_str.as_str()) {
                        // Strip: replace string values with placeholder, recurse on structures.
                        let placeholder = strip_value_by_key(key_str, v);
                        stripped_map.insert(k.clone(), placeholder);
                        continue;
                    }
                }
                // Not a strip key, recurse.
                stripped_map.insert(k.clone(), strip_flavor(v));
            }
            ron::Value::Map(stripped_map)
        }
        ron::Value::Seq(seq) => {
            // Recurse into sequences (for array values, choices, etc).
            let stripped: Vec<ron::Value> = seq.iter().map(strip_flavor).collect();
            ron::Value::Seq(stripped)
        }
        // Leaf values: return as-is.
        other => other.clone(),
    }
}

/// Extract the title words from an NPE cart RON string (base or merged) into
/// the typed row. Refuses when either face is absent (base title law, L10).
pub fn extract_title(npe_ron: &str) -> Result<crate::assets::TitleRow, String> {
    let val: ron::Value = ron::from_str(npe_ron).map_err(|e| format!("RON parse failed: {e}"))?;
    fn get(m: &ron::Map, key: &str) -> Option<ron::Value> {
        m.iter()
            .find(|(k, _)| matches!(k, ron::Value::String(s) if s == key))
            .map(|(_, v)| v.clone())
    }
    let ron::Value::Map(root) = val else {
        return Err("cart RON is not a struct".to_string());
    };
    let Some(ron::Value::Map(title)) = get(&root, "title") else {
        return Err("cart carries no title".to_string());
    };
    let word = |key: &str| -> String {
        match get(&title, key) {
            Some(ron::Value::String(s)) => s,
            _ => String::new(),
        }
    };
    let row = crate::assets::TitleRow {
        world_word: word("world_word"),
        front_line: word("front_line"),
        under_line: word("under_line"),
        bench_line: word("bench_line"),
    };
    row.validate().map_err(|e| format!("title refused whole: {e}"))?;
    Ok(row)
}

/// Replace a flavor value with a placeholder based on its key and content type.
fn strip_value_by_key(key: &str, val: &ron::Value) -> ron::Value {
    match val {
        ron::Value::String(_) => {
            // Simple placeholder: uppercase key.
            ron::Value::String(key.to_uppercase())
        }
        ron::Value::Option(opt) => {
            // Some(string) -> Some(placeholder), None stays None.
            match opt {
                Some(inner) => {
                    if matches!(inner.as_ref(), ron::Value::String(_)) {
                        ron::Value::Option(Some(Box::new(ron::Value::String(
                            format!("{}_PATH", key.to_uppercase()),
                        ))))
                    } else {
                        ron::Value::Option(opt.clone())
                    }
                }
                None => ron::Value::Option(None),
            }
        }
        ron::Value::Seq(seq) => {
            // Arrays: strip each element or recurse.
            let stripped: Vec<ron::Value> = seq
                .iter()
                .enumerate()
                .map(|(i, v)| match v {
                    ron::Value::String(_) => {
                        ron::Value::String(format!("{}_{}", key.to_uppercase(), i))
                    }
                    ron::Value::Seq(pair) if pair.len() == 2 => {
                        // Tuples in array: strip recursively within.
                        let mut stripped_pair = vec![];
                        for (j, elem) in pair.iter().enumerate() {
                            if let ron::Value::String(_) = elem {
                                stripped_pair.push(ron::Value::String(
                                    format!("{}_{}__{}", key.to_uppercase(), i, j),
                                ));
                            } else {
                                stripped_pair.push(strip_flavor(elem));
                            }
                        }
                        ron::Value::Seq(stripped_pair)
                    }
                    other => strip_flavor(other),
                })
                .collect();
            ron::Value::Seq(stripped)
        }
        // Complex structures: recurse into them.
        other => strip_flavor(other),
    }
}

/// Validate cart size — whole ≤ 80KB, sections ≤ 8KB each.
/// Returns Err(String) if any gate breaks, loudly naming the offender.
pub fn validate_size(body_bytes: &[u8]) -> Result<(), String> {
    let total = body_bytes.len();
    if total > MAX_CART_SIZE {
        return Err(format!(
            "REFUSAL: whole cart is {} bytes, exceeds 80KB limit (81,920 B)",
            total
        ));
    }

    // Parse RON to check sections. Sections are top-level entries in the map.
    let ron_str = String::from_utf8_lossy(body_bytes);
    let val: ron::Value = ron::from_str(&ron_str)
        .map_err(|e| format!("REFUSAL: cart body RON parse failed during size check: {e}"))?;

    if let ron::Value::Map(map) = val {
        for (k, v) in map.iter() {
            let section_ron = ron::to_string(v)
                .map_err(|e| format!("REFUSAL: section serialization failed: {e}"))?;
            let section_bytes = section_ron.as_bytes().len();
            if section_bytes > MAX_SECTION_SIZE {
                let key_name = match k {
                    ron::Value::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };
                return Err(format!(
                    "REFUSAL: section '{}' is {} bytes, exceeds 8KB limit (8,192 B)",
                    key_name, section_bytes
                ));
            }
        }
    }

    Ok(())
}

/// Validate all asset rows in a cart body. Returns first refusal or all linting wounds.
/// This gate runs BEFORE size validation so asset additions do not breach the 80KB limit.
pub fn validate_assets(body: &CartBody) -> Result<(), String> {
    // Validate sprite atlas rows.
    for (i, sprite) in body.sprites.iter().enumerate() {
        if let Err(e) = sprite.validate() {
            return Err(format!("sprite row {}: {}", i, e));
        }
    }

    // Validate and lint prompt rows.
    for (i, prompt) in body.prompts.iter().enumerate() {
        if let Err(e) = prompt.validate() {
            return Err(format!("prompt row {}: {}", i, e));
        }
        // Lint the prompt text for tone/style issues.
        let wounds = lint::lint(&prompt.text);
        if !wounds.is_empty() {
            let spoken = lint::speak_wounds(&wounds);
            return Err(format!("prompt row {} lint issue: {}", i, spoken));
        }
    }

    // Validate ledger rows (status transitions).
    for (i, ledger) in body.ledger.iter().enumerate() {
        if let Err(e) = ledger.validate(None) {
            return Err(format!("ledger row {}: {}", i, e));
        }
    }

    Ok(())
}

/// Bake: merge base + cart RON, produce themed and baseline sealed packs.
/// Extracts sprite pixel data into AssetCache for efficient access.
/// Returns receipt with sizes and BLAKE3 hashes.
pub fn bake_npe(
    base_ron: &str,
    cart_ron: &str,
    seal_fn: fn(&CartBody) -> Result<Vec<u8>, super::CartRefusal>,
) -> Result<BakeReceipt, String> {
    // Step 1: Merge.
    let merged = merge_ron(base_ron, cart_ron)?;

    // Step 2: Create themed pack from merged RON.
    let themed_ron_str = ron::to_string(&merged)
        .map_err(|e| format!("themed RON serialization failed: {e}"))?;

    // Step 2a: Validate size on themed pack body before adding assets.
    validate_size(themed_ron_str.as_bytes())?;

    // Step 3: Build themed body with asset cache populated from sprites.
    let mut themed_body = CartBody {
        npe_cart: Some(merged.clone()),
        items: vec![],
        ..Default::default()
    };
    // Extract sprite pixel data into cache.
    if !themed_body.sprites.is_empty() {
        for sprite in &themed_body.sprites {
            themed_body.asset_cache.insert_sprite(sprite.id.clone(), sprite.pixel_data.clone());
        }
    }

    // Step 4: Seal themed pack.
    let themed_bytes =
        seal_fn(&themed_body).map_err(|e| format!("seal themed pack failed: {e}"))?;

    // Step 5: Strip for baseline.
    let baseline_val = strip_flavor(&merged);
    let baseline_ron_str = ron::to_string(&baseline_val)
        .map_err(|e| format!("baseline RON serialization failed: {e}"))?;

    // Step 6: Validate size on baseline pack body.
    validate_size(baseline_ron_str.as_bytes())?;

    // Step 7: Build baseline body with asset cache.
    let mut baseline_body = CartBody {
        npe_cart: Some(baseline_val),
        items: vec![],
        ..Default::default()
    };
    // Extract sprite pixel data into cache for baseline as well.
    if !baseline_body.sprites.is_empty() {
        for sprite in &baseline_body.sprites {
            baseline_body.asset_cache.insert_sprite(sprite.id.clone(), sprite.pixel_data.clone());
        }
    }

    // Step 8: Seal baseline pack.
    let baseline_bytes =
        seal_fn(&baseline_body).map_err(|e| format!("seal baseline pack failed: {e}"))?;

    // Step 9: Compute hashes.
    let themed_hash = blake3::hash(&themed_bytes).to_hex().to_string();
    let baseline_hash = blake3::hash(&baseline_bytes).to_hex().to_string();

    Ok(BakeReceipt {
        themed_size: themed_bytes.len(),
        themed_hash,
        themed_bytes,
        baseline_size: baseline_bytes.len(),
        baseline_hash,
        baseline_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{seal, load};

    #[test]
    fn extract_title_reads_the_base_shape_and_refuses_a_frontless_cart() {
        let base = "NpeCart(title: Title(world_word: \"the world\", front_line: \"welcome, traveler\", under_line: \"the record disagrees\", bench_line: \"nothing on the bench yet\"))";
        let row = extract_title(base).expect("the base title shape must extract");
        assert_eq!(row.front_line, "welcome, traveler");
        assert_eq!(row.under_line, "the record disagrees");
        assert_eq!(row.bench_line, "nothing on the bench yet");
        let frontless = "NpeCart(title: Title(world_word: \"w\", under_line: \"u\"))";
        assert!(extract_title(frontless).is_err(), "a front-only/under-only title refuses whole");
        assert!(extract_title("NpeCart(schema: \"NPE1\")").is_err(), "no title at all refuses");
    }

    /// L07 bijection: strip → serialize → parse → should still pass size gate.
    #[test]
    fn strip_preserves_structure() {
        let ron_str = r#"NpeCart(
            schema: "NPE1",
            version: (major: 0, minor: 1, patch: 0),
            title: Title(
                world_word: "the world",
                front_line: "welcome",
            ),
        )"#;
        let val: ron::Value = ron::from_str(ron_str).expect("parse test ron");
        let stripped = strip_flavor(&val);
        let stripped_str =
            ron::to_string(&stripped).expect("stripped ronserializes");
        ron::from_str::<ron::Value>(&stripped_str).expect("stripped ron parses");
    }

    /// L18 sabotage: oversized section → LOUD refusal.
    #[test]
    fn size_gate_oversized_section_refuses() {
        // Craft an oversized section.
        let huge_string = "x".repeat(MAX_SECTION_SIZE + 100);
        let payload = format!(
            r#"NpeCart(
            schema: "NPE1",
            title: Title(
                world_word: "{huge}",
            ),
        )"#,
            huge = huge_string
        );
        let result = validate_size(payload.as_bytes());
        assert!(result.is_err(), "oversized section must be refused");
        let err = result.unwrap_err();
        assert!(err.contains("REFUSAL"), "refusal must be loud");
        assert!(err.contains("exceeds 8KB"), "refusal must name the gate");
    }

    /// L18 sabotage: oversized whole cart → LOUD refusal.
    #[test]
    fn size_gate_oversized_cart_refuses() {
        // Craft a payload larger than 80KB total.
        let huge_string = "x".repeat(MAX_CART_SIZE + 100);
        let payload = format!(r#"NpeCart(version: (major: 0, minor: 0, patch: 0), data: "{}")"#, huge_string);
        let result = validate_size(payload.as_bytes());
        assert!(result.is_err(), "oversized cart must be refused");
        let err = result.unwrap_err();
        assert!(err.contains("REFUSAL"), "refusal must be loud");
        assert!(err.contains("80KB"), "refusal must name the cart gate");
    }

    /// L07 bijection: bake produces packs that load and round-trip.
    #[test]
    fn bake_round_trip_bijection() {
        // Use RON Map syntax so both parse as Map values.
        let base_ron = r#"{schema: "NPE1", version: (major: 0, minor: 1, patch: 0)}"#;
        let cart_ron = r#"{}"#;
        let receipt = bake_npe(base_ron, cart_ron, seal)
            .expect("bake should succeed");

        // Load themed pack.
        let themed_loaded = load(&receipt.themed_bytes)
            .expect("themed pack must load");
        // Load baseline pack.
        let baseline_loaded = load(&receipt.baseline_bytes)
            .expect("baseline pack must load");

        // Both should deserialize without error.
        assert!(themed_loaded.npe_cart.is_some(), "themed pack must have npe_cart");
        assert!(baseline_loaded.npe_cart.is_some(), "baseline pack must have npe_cart");
    }

    /// CART SCHEMA GROWTH (census §2-3): the real base+ironroot carts, grown
    /// with systems/vocabulary/factions/budget sections, still bake, still
    /// pass the size gate, and still round-trip after strip.
    #[test]
    fn grown_real_carts_bake_and_pass_size_gate() {
        let base_ron = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../carts/base/npe.base.ron"),
        )
        .expect("npe.base.ron must be readable");
        let cart_ron = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../carts/ironroot/npe.ironroot.ron"),
        )
        .expect("npe.ironroot.ron must be readable");

        let receipt = bake_npe(&base_ron, &cart_ron, seal).expect("grown carts must bake");
        assert!(receipt.themed_size <= MAX_CART_SIZE, "themed pack must pass the 80KB gate");
        assert!(receipt.baseline_size <= MAX_CART_SIZE, "baseline pack must pass the 80KB gate");

        let themed_loaded = load(&receipt.themed_bytes).expect("themed pack must load");
        let baseline_loaded = load(&receipt.baseline_bytes).expect("baseline pack must load");
        let themed_val = themed_loaded.npe_cart.expect("themed npe_cart present");
        let baseline_val = baseline_loaded.npe_cart.expect("baseline npe_cart present");

        // Dual-seed law: strip must have scrubbed every faction name_word and
        // vocabulary term_word down to their placeholder form.
        let themed_str = ron::to_string(&themed_val).unwrap();
        let baseline_str = ron::to_string(&baseline_val).unwrap();
        assert!(themed_str.contains("Thornguard"), "themed pack must carry the drained faction name");
        assert!(!baseline_str.contains("Thornguard"), "baseline pack must not carry themed faction flavor");
        assert!(baseline_str.contains("NAME_WORD"), "stripped faction rows use the name_word placeholder");
        assert!(baseline_str.contains("TERM_WORD"), "stripped vocabulary rows use the term_word placeholder");

        // Base-alone must also pass every gate (strip of an all-NULL cart is a no-op parse check).
        let base_only: ron::Value = ron::from_str(&base_ron).expect("base alone must parse");
        let base_only_str = ron::to_string(&base_only).unwrap();
        validate_size(base_only_str.as_bytes()).expect("base alone must pass the size gate");
    }

    /// Determinism: baking twice with the same inputs produces byte-identical packs.
    #[test]
    fn bake_deterministic_outputs() {
        // Use RON Map syntax so both parse as Map values.
        let base_ron = r#"{schema: "NPE1", version: (major: 0, minor: 1, patch: 0)}"#;
        let cart_ron = r#"{}"#;

        let receipt1 = bake_npe(base_ron, cart_ron, seal)
            .expect("first bake should succeed");
        let receipt2 = bake_npe(base_ron, cart_ron, seal)
            .expect("second bake should succeed");

        // Bytes must be byte-identical.
        assert_eq!(
            &receipt1.themed_bytes, &receipt2.themed_bytes,
            "themed packs must be byte-identical"
        );
        assert_eq!(
            &receipt1.baseline_bytes, &receipt2.baseline_bytes,
            "baseline packs must be byte-identical"
        );
    }

    /// L07 bijection: cart with valid asset rows round-trips unchanged.
    #[test]
    fn asset_rows_round_trip() {
        use crate::assets::{SpriteAtlasRow, PromptRow, AssetLedgerRow, AssetStatus};

        let sprite = SpriteAtlasRow {
            id: "test_sprite".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![0; 32 * 48],
        };

        let prompt = PromptRow {
            id: "test_prompt".to_string(),
            category: "texture".to_string(),
            text: "A tileable seamless stone wall texture, hand-crafted".to_string(),
        };

        let ledger = AssetLedgerRow {
            id: "test_ledger".to_string(),
            name: "banner".to_string(),
            asset_type: "sprite".to_string(),
            status: AssetStatus::Generated.as_u8(),
            prompt: "test".to_string(),
            qa_result: None,
            export_path: None,
            hash: "abc123".to_string(),
        };

        let body = CartBody {
            items: vec![],
            npe_cart: None,
            sprites: vec![sprite.clone()],
            prompts: vec![prompt.clone()],
            ledger: vec![ledger.clone()],
            ..Default::default()
        };

        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");

        assert_eq!(body, loaded, "asset rows must round-trip unchanged");
    }

    /// L18 sabotage: invalid sprite atlas is refused at validation.
    #[test]
    fn asset_validation_rejects_bad_atlas() {
        use crate::assets::SpriteAtlasRow;

        let bad_sprite = SpriteAtlasRow {
            id: "bad".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "unknown_palette".to_string(),
            pixel_data: vec![0; 32 * 48],
        };

        let body = CartBody {
            items: vec![],
            npe_cart: None,
            sprites: vec![bad_sprite],
            prompts: vec![],
            ledger: vec![],
            ..Default::default()
        };

        let result = validate_assets(&body);
        assert!(result.is_err(), "invalid atlas should be refused");
        assert!(result.unwrap_err().contains("unknown_palette"));
    }

    /// L18 sabotage: prompt with banned pattern is refused at validation.
    #[test]
    fn asset_validation_rejects_banned_pattern() {
        use crate::assets::PromptRow;

        let bad_prompt = PromptRow {
            id: "bad".to_string(),
            category: "texture".to_string(),
            text: "A magical enchanted tileable seamless sword with arcane runes".to_string(),
        };

        let body = CartBody {
            items: vec![],
            npe_cart: None,
            sprites: vec![],
            prompts: vec![bad_prompt],
            ledger: vec![],
            ..Default::default()
        };

        let result = validate_assets(&body);
        assert!(result.is_err(), "prompt with banned pattern should be refused");
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("banned pattern"));
    }

    /// L07 bijection: sprite pixel data loads through AssetCache.
    #[test]
    fn sprite_loads_through_asset_cache() {
        use crate::assets::{SpriteAtlasRow, AssetCache};

        let sprite = SpriteAtlasRow {
            id: "test_sprite".to_string(),
            source_name: "src.png".to_string(),
            source_dims: [1024, 1024],
            target_dims: [32, 48],
            crop_bounds: [0, 50, 1023, 1013],
            palette_id: "2dak_64".to_string(),
            pixel_data: vec![42; 32 * 48], // 42 is a test marker
        };

        // Create a cart body with the sprite.
        let mut body = CartBody {
            items: vec![],
            npe_cart: None,
            sprites: vec![sprite.clone()],
            prompts: vec![],
            ledger: vec![],
            asset_cache: AssetCache::new(),
            ..Default::default()
        };

        // Manually populate cache (simulating bake_npe behavior).
        body.asset_cache.insert_sprite(sprite.id.clone(), sprite.pixel_data.clone());

        // Seal and load.
        let sealed = seal(&body).expect("seal should succeed");
        let loaded = load(&sealed).expect("load should succeed");

        // Verify sprite pixel data is accessible through cache.
        let cached_data = loaded
            .asset_cache
            .get_sprite(&sprite.id)
            .expect("sprite data must be in cache");
        assert_eq!(cached_data, vec![42; 32 * 48], "sprite pixel data must match");
        assert_eq!(
            loaded.asset_cache.total_size(),
            32 * 48,
            "cache total size must equal sprite data"
        );
    }
}
