//! Unified asset-type taxonomy for the 13Forge marketplace and engine artifacts.
//!
//! Single source of truth — replaces `forge_marketplace::AssetType` and
//! `forge_evidence::provenance::ArtifactType` (kept as deprecated aliases there).
//!
//! Marked `#[non_exhaustive]` so adding a variant in a later canonical-bytes
//! version is not a breaking change for `match` arms in downstream code.

use serde::{Deserialize, Serialize};

/// Asset-type tag carried by every Nistam receipt and marketplace listing.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    /// Texture verdict/state.
    Texture,
    /// Pixel verdict/state.
    Pixel,
    /// Sprite sheet verdict/state.
    SpriteSheet,
    /// Model verdict/state.
    Model,
    /// Glb verdict/state.
    Glb,
    /// Audio verdict/state.
    Audio,
    /// Audio pack verdict/state.
    AudioPack,
    /// Script verdict/state.
    Script,
    /// Dialogue verdict/state.
    Dialogue,
    /// Vixi verdict/state.
    Vixi,
    /// Scene verdict/state.
    Scene,
    /// Zone verdict/state.
    Zone,

    // Engine-internal registries
    /// Forge reg verdict/state.
    ForgeReg,

    // Marketplace-canonical item (forge-items::schema::Item)
    /// Item verdict/state.
    Item,

    // QAQC stop-path classes (RULED brief 2026-07-05 §1; step-0 gap fill —
    // the taxonomy's three classes the enum didn't yet name).
    /// A painted canvas / .forge13 project (layer stack + stroke buffers).
    Forge13Canvas,
    /// A socketed primitive-based item (socket graph over registered primitives).
    SocketedItem,
    /// A game cart package — the aggregate Ed25519 chain root over its assets.
    Cart,
}

impl AssetType {
    /// Stable string tag — matches the serde `rename_all = "snake_case"` form
    /// so callers can use this for non-serde contexts (hash inputs, log lines)
    /// without re-serializing.
    pub fn as_str(self) -> &'static str {
        match self {
            AssetType::Texture     => "texture",
            AssetType::Pixel       => "pixel",
            AssetType::SpriteSheet => "sprite_sheet",
            AssetType::Model       => "model",
            AssetType::Glb         => "glb",
            AssetType::Audio       => "audio",
            AssetType::AudioPack   => "audio_pack",
            AssetType::Script      => "script",
            AssetType::Dialogue    => "dialogue",
            AssetType::Vixi        => "vixi",
            AssetType::Scene       => "scene",
            AssetType::Zone        => "zone",
            AssetType::ForgeReg    => "forge_reg",
            AssetType::Item        => "item",
            AssetType::Forge13Canvas => "forge13_canvas",
            AssetType::SocketedItem  => "socketed_item",
            AssetType::Cart          => "cart",
        }
    }
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_matches_serde_tag() {
        // The `as_str` form MUST match the serde rename_all="snake_case" output,
        // or hash inputs and serialized receipts will disagree.
        for t in [
            AssetType::Texture, AssetType::Pixel, AssetType::SpriteSheet,
            AssetType::Model, AssetType::Glb,
            AssetType::Audio, AssetType::AudioPack,
            AssetType::Script, AssetType::Dialogue, AssetType::Vixi,
            AssetType::Scene, AssetType::Zone, AssetType::ForgeReg,
            AssetType::Item,
            AssetType::Forge13Canvas, AssetType::SocketedItem, AssetType::Cart,
        ] {
            let serde_tag = serde_json::to_string(&t).unwrap();
            // serde_json wraps strings in quotes, strip them for compare
            let serde_tag_unquoted = serde_tag.trim_matches('"');
            assert_eq!(t.as_str(), serde_tag_unquoted, "mismatch for {t:?}");
        }
    }

    #[test]
    fn roundtrip_json() {
        let t = AssetType::Pixel;
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#""pixel""#);
        let back: AssetType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AssetType::Pixel);
    }

    #[test]
    fn rejects_pascal_case_with_helpful_variant_listing() {
        // First-time operators (and CI fixtures) routinely guess PascalCase
        // because the Rust variant identifiers are PascalCase. This test pins
        // the friendly-error contract: serde MUST reject the PascalCase form
        // AND surface the lowercase variants in the error message, so the
        // operator can copy-paste the right tag without reading source.
        //
        // Triggered ticket: scanner-wire-fixture-schema-002 (2026-05-20).
        for bad_pascal in [r#""Texture""#, r#""Pixel""#, r#""SpriteSheet""#, r#""ForgeReg""#] {
            let result: Result<AssetType, _> = serde_json::from_str(bad_pascal);
            let err = result.expect_err(&format!("expected {bad_pascal} to be rejected"));
            let msg = err.to_string();
            assert!(
                msg.contains("unknown variant"),
                "{bad_pascal}: error should mention 'unknown variant', got: {msg}"
            );
            // Spot-check that the listing surfaces at least the canonical
            // lowercase tags an operator would need next.
            for expected_tag in ["texture", "pixel", "sprite_sheet"] {
                assert!(
                    msg.contains(expected_tag),
                    "{bad_pascal}: error should list lowercase tag `{expected_tag}`, got: {msg}"
                );
            }
        }
    }

    #[test]
    fn accepts_canonical_snake_case() {
        // Pair-test for the rejection above: confirm every documented snake_case
        // tag actually parses. If a variant rename ever drifts from the README
        // / forgedaemon usage block, this catches it.
        let pairs: &[(&str, AssetType)] = &[
            (r#""texture""#,      AssetType::Texture),
            (r#""pixel""#,        AssetType::Pixel),
            (r#""sprite_sheet""#, AssetType::SpriteSheet),
            (r#""model""#,        AssetType::Model),
            (r#""glb""#,          AssetType::Glb),
            (r#""audio""#,        AssetType::Audio),
            (r#""audio_pack""#,   AssetType::AudioPack),
            (r#""script""#,       AssetType::Script),
            (r#""dialogue""#,     AssetType::Dialogue),
            (r#""vixi""#,         AssetType::Vixi),
            (r#""scene""#,        AssetType::Scene),
            (r#""zone""#,         AssetType::Zone),
            (r#""forge_reg""#,    AssetType::ForgeReg),
            (r#""item""#,         AssetType::Item),
        ];
        for (json, expected) in pairs {
            let parsed: AssetType = serde_json::from_str(json)
                .unwrap_or_else(|e| panic!("snake_case {json} should parse, got: {e}"));
            assert_eq!(parsed, *expected, "{json} parsed to wrong variant");
        }
    }
}
