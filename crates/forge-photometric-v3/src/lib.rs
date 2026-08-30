//! forge-photometric-v3 — T2 of the forge-vision drain. The photometric
//! word (`NormalAlbedo8`, 8 bytes, exact): an integer-only octahedral
//! normal plus albedo/roughness permyriad channels. Shading transforms
//! (true unit-sphere normalization, PBR lighting) arrive in later tranches
//! as functions over this type; nothing here is a second home for it.

mod normal;
pub mod solver;

pub use normal::{decode_octahedral, encode_octahedral, NormalAlbedo8, OCT_SCALE, PMY_MAX};
pub use solver::GlyphRelief;
