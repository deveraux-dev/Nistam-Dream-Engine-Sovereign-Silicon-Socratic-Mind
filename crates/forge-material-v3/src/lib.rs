//! forge-material-v3 — T8 of the forge-vision drain. The material word
//! (`MaterialTrit8`, 8 bytes, exact): the forgeatom `material_id` line, a
//! fill permyriad channel, and a 25-trit ternary craft plane packed 5
//! trits/byte in base-3. Crafting semantics and the atlas/registry tranche
//! arrive later as functions over this type; nothing here is a second home
//! for it.

mod coeffs;
mod material;
mod seam;
mod shade;
mod stack;

pub use material::{
    pack5, unpack5, MaterialAxis, MaterialTrit8, Sentinel, CRAFT_BYTE_MAX, FILL_MAX, MATERIAL_ASH,
    MATERIAL_BONE, MATERIAL_COUNT, MATERIAL_GAS, MATERIAL_GOLD, MATERIAL_ID_MAX, MATERIAL_IRON,
    MATERIAL_PHYSICAL_COUNT, MATERIAL_PHYSICAL_MAX, MATERIAL_STONE, SENTINEL_CHAOS, SENTINEL_EMISSIVE,
    SENTINEL_ENTROPY, SENTINEL_FORCE_FIELD, SENTINEL_HALT_RECYCLE, SENTINEL_IPC_SYNC, SENTINEL_MIRROR,
    SENTINEL_PASSTHROUGH, SENTINEL_SHADOW, SENTINEL_SUPERPOSITION, SENTINEL_UNTRACED, SENTINEL_USER_RESERVED,
    SENTINEL_VOID,
};

// TRANCHE A (2026-08-11): the material tier ladder — 8 / 16 / 32 / 64, each
// tier decoding down to the one below exactly over its retained channels.
// `Coeffs16` is per-MATERIAL registry data, deliberately NOT a field of the
// 64-byte per-texel word (the HOTPATH ISOLATION LAW, forge-core-v3
// soul.rs:206-231 — see the block comment atop `stack.rs`).
pub use coeffs::{Coeffs16, COEFFS_PMY_MAX};
pub use seam::{from_pair, to_pair, ReactionSeam16, REACT_RESIDUAL_MAX, SEAM_PMY_MAX};
pub use shade::MaterialShade16;
pub use stack::{MaterialStack32, MaterialStack64};
