//! CREE-RIVERBED codebook v0 — shop-state glyph wire vocabulary.
//!
//! Ported verbatim from `F:\NewRepo\crates\forge-daemon-types\src\codebook.rs`
//! (2026-08-15). One UCAS glyph = concept + state + lane, collision-free with
//! prose. SHOP bindings only — never linguistic or cultural semantics.

/// The five v0 bindings: glyph -> the shop state it replaces.
pub const CODEBOOK_V0: [(char, &str); 5] = [
    ('ᐁ', "PROVEN/verified-live-on-disk"),
    ('ᐃ', "UNPROVEN/claim-not-traced"),
    ('ᐅ', "HEDGE/unit-green-UNRUN"),
    ('ᐊ', "BLOCKED-SEAN"),
    ('ᐍ', "thaw/round-trip-verified"),
];

/// Decode one glyph to its shop state; `None` for anything outside the codebook.
pub fn decode(glyph: char) -> Option<&'static str> {
    CODEBOOK_V0.iter().find(|(g, _)| *g == glyph).map(|(_, s)| *s)
}

/// Is `c` one of the five v0 state glyphs? The codebook is deliberately tiny
/// so a glyph is never ambiguous.
pub fn is_codebook_glyph(c: char) -> bool {
    decode(c).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_five_decode_and_outsiders_stay_out() {
        for (g, s) in CODEBOOK_V0 {
            assert_eq!(decode(g), Some(s));
            assert!(is_codebook_glyph(g));
        }
        for c in ['ᐭ', 'A', '[', '·'] {
            assert_eq!(decode(c), None, "{c} must stay outside the codebook");
        }
    }
}
