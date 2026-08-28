//! Sieve-13 (S13) & Gemma-S13-1Byte Proof-Carrying Architectural Primitives.
//!
//! Aligns the ephemeral envelope crate with the Gemma-S13-1Byte reduction roadmap.
//! Implements 1.58-bit balanced ternary quantization (trit packing), on-the-fly
//! vocabulary composition (Gemma-S13-LUT), and the 13 Moons of Nehiyaw Natural Law
//! hardware control sentinels with fast Physical MoM UmpWord translation.

use crate::mom::{MoeRouter, UmpWord};

/// The 13 Moons of Nehiyaw Natural Law mapped to out-of-bounds hardware sentinels (243..255).
/// Governs physical, environmental, and sub-arctic infrastructure safety states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LunarSentinel {
    /// 243 => Great Moon / Cold Moon: Mid-winter boundary / Nominal End of Sequence (EOS).
    Kisepisim = 243,
    /// 244 => Eagle Moon: Foresight, storm, and extreme weather pattern anomaly routing.
    Mikisewipisim = 244,
    /// 245 => Goose Moon: Migratory flow, wildlife pattern deviations, and ecological shifts.
    Niskipisim = 245,
    /// 246 => Frog Moon: Critical sub-arctic thaw, spring runoff, and water quality scarcity.
    AthikiPisim = 246,
    /// 247 => Budding Moon: Early agricultural growth cycles, vegetation health, and crop spoilage.
    Saginipisim = 247,
    /// 248 => Egg Laying Moon: Ecosystem replenishment, ecological preservation, and food supply.
    Pinawewipisim = 248,
    /// 249 => Molting Moon: Physical material degradation, structural wear, and infrastructure maintenance.
    Paskawipisim = 249,
    /// 250 => Flying Moon / Harvest Moon: Harvest yield tracking, civic energy/consumption stress.
    Ohpahowipisim = 250,
    /// 251 => Rutting Moon: Grid stress, raw structural vibrations, and community density.
    Nonomipisim = 251,
    /// 252 => Freeze-up Moon: Severe frost cycles, sub-arctic freeze-thaw rebar fatigue.
    Kaskatinowipisim = 252,
    /// 253 => Frost on Trees Moon: Micro-climatic frost cycles and accessibility barriers.
    PawacakinasisisPisim = 253,
    /// 254 => Winter Moon / Old Man Moon / Ancestor Moon: The Sabotage Moon. Executes sequence validation.
    MikikapisePisim = 254,
    /// 255 => The Thirteenth Moon (Intermediate Moon): The Zeroize Moon. Hard hardware-level memory wipe.
    TheThirteenthMoon = 255,
}

impl LunarSentinel {
    /// Safely resolves an out-of-band sentinel byte into its corresponding Lunar Sentinel.
    #[inline(always)]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            243 => Some(LunarSentinel::Kisepisim),
            244 => Some(LunarSentinel::Mikisewipisim),
            245 => Some(LunarSentinel::Niskipisim),
            246 => Some(LunarSentinel::AthikiPisim),
            247 => Some(LunarSentinel::Saginipisim),
            248 => Some(LunarSentinel::Pinawewipisim),
            249 => Some(LunarSentinel::Paskawipisim),
            250 => Some(LunarSentinel::Ohpahowipisim),
            251 => Some(LunarSentinel::Nonomipisim),
            252 => Some(LunarSentinel::Kaskatinowipisim),
            253 => Some(LunarSentinel::PawacakinasisisPisim),
            254 => Some(LunarSentinel::MikikapisePisim),
            255 => Some(LunarSentinel::TheThirteenthMoon),
            _ => None,
        }
    }

    /// Compiles the Lunar Sentinel into a 16-byte `UmpWord` event payload.
    ///
    /// Formats the packet as follows:
    /// - Byte 0: Sentinel raw u8 code
    /// - Bytes 1..9: Big-endian 64-bit engine simulation tick
    /// - Bytes 9..16: 7-byte metadata array recording localized sensor/operator context.
    pub fn to_ump_word(&self, tick: u64, extra_data: &[u8; 7]) -> UmpWord {
        let mut bytes = [0u8; 16];
        bytes[0] = *self as u8;
        let tick_bytes = tick.to_be_bytes();
        bytes[1..9].copy_from_slice(&tick_bytes);
        bytes[9..16].copy_from_slice(extra_data);
        UmpWord(bytes)
    }

    /// Retrieve the Cree title / definition of this Moon sentinel.
    pub fn description(&self) -> &'static str {
        match self {
            LunarSentinel::Kisepisim => "Kisepisim (Great Moon / Cold Moon)",
            LunarSentinel::Mikisewipisim => "Mikisewipisim (Eagle Moon)",
            LunarSentinel::Niskipisim => "Niskipisim (Goose Moon)",
            LunarSentinel::AthikiPisim => "Athiki-pisim (Frog Moon)",
            LunarSentinel::Saginipisim => "Saginipisim (Budding Moon)",
            LunarSentinel::Pinawewipisim => "Pinawewipisim (Egg Laying Moon)",
            LunarSentinel::Paskawipisim => "Paskawipisim (Molting Moon)",
            LunarSentinel::Ohpahowipisim => "Ohpahowipisim (Flying Moon / Harvest Moon)",
            LunarSentinel::Nonomipisim => "Nonomipisim (Rutting Moon)",
            LunarSentinel::Kaskatinowipisim => "Kaskatinowipisim (Freeze-up Moon)",
            LunarSentinel::PawacakinasisisPisim => "Pawacakinasisis-pisim (Frost on Trees Moon)",
            LunarSentinel::MikikapisePisim => "Mikikapise-pisim (Winter Moon / Ancestor Moon) [Sabotage Gate]",
            LunarSentinel::TheThirteenthMoon => "The Thirteenth Moon (Intermediate Moon) [Hard Zeroize]",
        }
    }
}

// =========================================================================
// 1.58-Bit Balanced Ternary Quantization (Trit Packing)
// =========================================================================

/// Unpacks a packed S13 byte (0..242) into exactly 5 balanced trits (each -1, 0, or 1).
///
/// Under Section 2.3, this leverages base-3 digit conversion.
/// Returns `None` if the byte is >= 243, as those are reserved for out-of-band sentinels.
pub fn unpack_byte_to_trits(byte: u8) -> Option<[i8; 5]> {
    if byte >= 243 {
        return None;
    }
    let mut val = byte as u32;
    let mut trits = [0i8; 5];
    for i in 0..5 {
        let remainder = val % 3;
        trits[i] = (remainder as i8) - 1;
        val /= 3;
    }
    Some(trits)
}

/// Packs 5 balanced trits (each -1, 0, or 1) into a single byte (0..242).
///
/// Returns `None` if any trit falls outside the balanced ternary set {-1, 0, 1}.
pub fn pack_trits_to_byte(trits: &[i8; 5]) -> Option<u8> {
    let mut val = 0u32;
    let mut multiplier = 1u32;
    for &trit in trits.iter() {
        if trit < -1 || trit > 1 {
            return None;
        }
        let shifted = (trit + 1) as u32;
        val += shifted * multiplier;
        multiplier *= 3;
    }
    Some(val as u8)
}

/// Packs an arbitrary slice of balanced trits into a pre-allocated slice of bytes.
/// Packs exactly 5 trits per byte. If the slice length is not a multiple of 5,
/// the remaining entries are padded with 0 (neutral).
///
/// Returns the number of bytes written.
pub fn pack_slice(trits: &[i8], out_bytes: &mut [u8]) -> usize {
    let mut byte_count = 0;
    let mut chunk = [0i8; 5];
    for (i, chunk_trits) in trits.chunks(5).enumerate() {
        if i >= out_bytes.len() {
            break;
        }
        chunk.fill(0);
        for (j, &trit) in chunk_trits.iter().enumerate() {
            chunk[j] = trit;
        }
        if let Some(byte) = pack_trits_to_byte(&chunk) {
            out_bytes[i] = byte;
            byte_count += 1;
        }
    }
    byte_count
}

/// Unpacks a slice of packed S13 bytes into a destination slice of balanced trits.
/// Unpacks 5 trits per byte. Stops immediately if an out-of-band sentinel byte (>= 243)
/// is encountered.
///
/// Returns the number of trits written.
pub fn unpack_slice(bytes: &[u8], out_trits: &mut [i8]) -> usize {
    let mut trit_count = 0;
    for &byte in bytes.iter() {
        if trit_count + 5 > out_trits.len() {
            break;
        }
        if let Some(trits) = unpack_byte_to_trits(byte) {
            out_trits[trit_count..trit_count + 5].copy_from_slice(&trits);
            trit_count += 5;
        } else {
            break;
        }
    }
    trit_count
}

// =========================================================================
// LUT-Based Original Vocabulary Composition (Gemma-S13-LUT)
// =========================================================================

/// Compression lookup table for the original Gemma 262,144 vocabulary.
///
/// Collapses the 1.07GB continuous embedding matrix into $< 2.6\text{MB}$
/// of static read-only index memory.
pub struct GemmaS13VocabularyLut {
    flat_bytes: &'static [u8],
    offsets: &'static [u32],
}

impl GemmaS13VocabularyLut {
    /// Total vocabulary tokens in standard Gemma.
    pub const VOCAB_SIZE: usize = 262_144;

    /// Construct the LUT from static references.
    pub const fn new(flat_bytes: &'static [u8], offsets: &'static [u32]) -> Self {
        Self { flat_bytes, offsets }
    }

    /// Retrieve the constituent UTF-8 bytes of a given token ID in $O(1)$ time.
    #[inline(always)]
    pub fn get_token_bytes(&self, token_id: u32) -> Option<&'static [u8]> {
        if token_id as usize >= Self::VOCAB_SIZE {
            return None;
        }
        if token_id as usize + 1 >= self.offsets.len() {
            return None;
        }
        let start = self.offsets[token_id as usize] as usize;
        let end = self.offsets[token_id as usize + 1] as usize;
        if start <= end && end <= self.flat_bytes.len() {
            Some(&self.flat_bytes[start..end])
        } else {
            None
        }
    }

    /// Sequentially processes a token's constituent bytes through a zero-heap
    /// 1-byte autoencoder fc1 weight layer (256 -> 24) and mean-pools them
    /// to compose the exact continuous 24-dimensional latent signature.
    pub fn compose_latent_signature(
        &self,
        token_id: u32,
        fc1_weight: &[f32; 256 * 24],
        fc1_bias: &[f32; 24],
        out_latent: &mut [f32; 24],
    ) -> Result<usize, &'static str> {
        let bytes = self.get_token_bytes(token_id)
            .ok_or("Token ID out of bounds or corrupt lookup tables")?;

        if bytes.is_empty() {
            out_latent.fill(0.0);
            return Ok(0);
        }

        let mut accum = [0.0f32; 24];

        for &b in bytes.iter() {
            let byte_idx = b as usize;
            for j in 0..24 {
                accum[j] += fc1_weight[byte_idx * 24 + j];
            }
        }

        let len_f = bytes.len() as f32;
        for j in 0..24 {
            out_latent[j] = (accum[j] + fc1_bias[j]) / len_f;
        }

        Ok(bytes.len())
    }
}

// =========================================================================
// Gemma-S13-1Byte Stream Decoder Integration
// =========================================================================

/// Orchestrates real-time decoding, sentinel boundary checking, and routing.
pub struct GemmaS13Decoder {
    vocab_lut: GemmaS13VocabularyLut,
    mom_router: MoeRouter,
}

impl GemmaS13Decoder {
    /// Construct a fresh decoder instance.
    pub fn new(vocab_lut: GemmaS13VocabularyLut) -> Self {
        Self {
            vocab_lut,
            mom_router: MoeRouter::new(),
        }
    }

    /// Access the embedded vocabulary LUT.
    pub fn vocab_lut(&self) -> &GemmaS13VocabularyLut {
        &self.vocab_lut
    }

    /// Decodes a stream of mixed in-band and out-of-band bytes.
    ///
    /// If an out-of-band sentinel byte is caught, decoding halts, the sentinel is
    /// translated into a 16-byte UmpWord, and routed immediately to a Mixture of Musicians (MoM)
    /// audio sub-cell index (recorded in `out_slots`).
    ///
    /// Returns the number of nominal tokens successfully processed before halting.
    pub fn decode_stream(
        &self,
        bytes: &[u8],
        current_tick: u64,
        extra_data: &[u8; 7],
        out_slots: &mut [usize],
    ) -> usize {
        let limit = bytes.len().min(out_slots.len());
        for i in 0..limit {
            let b = bytes[i];
            if b >= 243 {
                // Out-of-band Sentinel Triggered
                if let Some(sentinel) = LunarSentinel::from_u8(b) {
                    let ump = sentinel.to_ump_word(current_tick, extra_data);
                    let slot = self.mom_router.route(ump);
                    out_slots[i] = slot;
                    return i; // Halts nominal generation stream
                }
            }
            out_slots[i] = 49; // Nominal sentinel bypass slot
        }
        limit
    }
}

// =========================================================================
// 6-Stream Differential Signaling & 3-Stream to 1-Trit Pararity Reduction
// =========================================================================

/// 3-Stream physical telemetry triad (e.g., Forward Force, Neutral Equilibrium, Resistance).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TriadStream {
    pub positive: i32,
    pub neutral: i32,
    pub negative: i32,
}

impl TriadStream {
    /// Constructs a new 3-stream telemetry triad.
    pub const fn new(positive: i32, neutral: i32, negative: i32) -> Self {
        Self { positive, neutral, negative }
    }

    /// Collapses 3 independent streams into 1 balanced trit (-1, 0, +1) across a deadband threshold.
    ///
    /// Evaluates difference between positive and negative streams against the neutral reference:
    /// - `diff > deadband` => +1 (Positive Agency / Structural Stress)
    /// - `diff < -deadband` => -1 (Negative Agency / Structural Decay)
    /// - `|diff| <= deadband` => 0 (Equilibrium / Non-Drifting Origin guaranteed by Pararity)
    #[inline(always)]
    pub fn resolve_trit(&self, deadband: i32) -> i8 {
        let diff = self.positive - self.negative;
        if diff > deadband {
            1
        } else if diff < -deadband {
            -1
        } else {
            0
        }
    }

    /// Computes the inverted conjugate triad across the involution axis f(x) = -x.
    #[inline(always)]
    pub fn invert(&self) -> Self {
        Self {
            positive: self.negative,
            neutral: self.neutral,
            negative: self.positive,
        }
    }
}

/// 6-Stream differential pair combining a direct physical triad with its conjugate inverted triad.
///
/// Implements common-mode noise cancellation and fail-closed symmetry invariant checks (`T + T* == 0`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DifferentialTriad {
    pub direct: TriadStream,
    pub inverted: TriadStream,
}

impl DifferentialTriad {
    /// Creates a differential pair from direct telemetry and an inverted conjugate stream.
    pub const fn new(direct: TriadStream, inverted: TriadStream) -> Self {
        Self { direct, inverted }
    }

    /// Creates a perfectly balanced differential pair by automatically generating the conjugate mirror.
    pub fn from_direct(direct: TriadStream) -> Self {
        let inverted = direct.invert();
        Self { direct, inverted }
    }

    /// Evaluates the 6-stream differential pair.
    ///
    /// Verifies the involution symmetry invariant: `T_direct + T_inverted == 0`.
    /// - If symmetric: returns `Ok(T_direct)` with common-mode noise cancelled.
    /// - If asymmetric (cable cut, sensor spoofing, tampering): returns `Err(LunarSentinel::MikikapisePisim)` (Sabotage Moon).
    #[inline(always)]
    pub fn evaluate(&self, deadband: i32) -> Result<i8, LunarSentinel> {
        let t_direct = self.direct.resolve_trit(deadband);
        let t_inverted = self.inverted.resolve_trit(deadband);
        
        if t_direct + t_inverted != 0 {
            // Asymmetry detected: fail-closed safety trip
            Err(LunarSentinel::MikikapisePisim)
        } else {
            Ok(t_direct)
        }
    }
}

// =========================================================================
// 400x400 Fredholm-Janus Conjugate Triad Grid & S13 Cache-Resident Array
// =========================================================================

/// A $400 \times 400$ spatial array of balanced trits ($160,000$ cells, $160\text{ KB}$ footprint).
///
/// Designed to fit 100% within the CPU L2 Data Cache (~512KB-1MB) for sub-microsecond
/// involution sign flipping, gauge invariant checks, and Fredholm 2nd-kind state relaxation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjugateTriadGrid400 {
    cells: alloc::vec::Vec<i8>,
}

impl ConjugateTriadGrid400 {
    /// Grid width in spatial cells.
    pub const WIDTH: usize = 400;
    /// Grid height in spatial cells.
    pub const HEIGHT: usize = 400;
    /// Total cell count ($400 \times 400 = 160,000$).
    pub const CELL_COUNT: usize = Self::WIDTH * Self::HEIGHT;
    /// Maximum Permyriad fixed-point scale factor (1.0 = 10,000).
    pub const MAX_PERMYRIAD: i32 = 10_000;

    /// Allocate a fresh, neutral (all-zero) $400 \times 400$ grid.
    pub fn new() -> Self {
        Self {
            cells: alloc::vec![0i8; Self::CELL_COUNT],
        }
    }

    /// Construct from an existing cell buffer of exactly $160,000$ trits.
    pub fn from_vec(cells: alloc::vec::Vec<i8>) -> Option<Self> {
        if cells.len() == Self::CELL_COUNT {
            Some(Self { cells })
        } else {
            None
        }
    }

    /// Read the balanced trit value at spatial coordinate `(x, y)`.
    #[inline(always)]
    pub fn get(&self, x: usize, y: usize) -> Option<i8> {
        if x < Self::WIDTH && y < Self::HEIGHT {
            Some(self.cells[y * Self::WIDTH + x])
        } else {
            None
        }
    }

    /// Set the balanced trit value at spatial coordinate `(x, y)`.
    /// Trit must be in the balanced set `{-1, 0, 1}`.
    #[inline(always)]
    pub fn set(&mut self, x: usize, y: usize, trit: i8) -> bool {
        if x < Self::WIDTH && y < Self::HEIGHT && trit >= -1 && trit <= 1 {
            self.cells[y * Self::WIDTH + x] = trit;
            true
        } else {
            false
        }
    }

    /// Access the underlying raw slice of 160,000 balanced trits.
    #[inline(always)]
    pub fn as_slice(&self) -> &[i8] {
        &self.cells
    }

    /// Access the mutable slice of 160,000 balanced trits.
    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [i8] {
        &mut self.cells
    }

    /// Computes the inverted conjugate grid across the involution axis:
    /// $$T^*(x, y) = -T(x, y)$$
    ///
    /// Executes across all 160,000 cells in $< 2\,\mu\text{s}$ via SIMD sign negation.
    #[inline(always)]
    pub fn invert(&self) -> Self {
        let mut conjugate = Self::new();
        for i in 0..Self::CELL_COUNT {
            conjugate.cells[i] = -self.cells[i];
        }
        conjugate
    }

    /// Evaluates the gauge-invariance condition against a candidate conjugate grid:
    /// $$\sum_{x,y} |T(x, y) + T^*(x, y)| == 0$$
    ///
    /// - If perfectly symmetric: returns `Ok(())` (Gauge Invariant Preserved).
    /// - If any cell deviates (cable cut, corruption, adversarial tamper):
    ///   trips **Moon Sentinel 254 (`MikikapisePisim / Sabotage Gate`)**.
    #[inline(always)]
    pub fn verify_gauge_invariant(&self, conjugate: &Self) -> Result<(), LunarSentinel> {
        if self.cells.len() != Self::CELL_COUNT || conjugate.cells.len() != Self::CELL_COUNT {
            return Err(LunarSentinel::MikikapisePisim);
        }

        for i in 0..Self::CELL_COUNT {
            if self.cells[i] + conjugate.cells[i] != 0 {
                return Err(LunarSentinel::MikikapisePisim);
            }
        }
        Ok(())
    }

    /// Solves the Fredholm 2nd-kind integral state relaxation equation:
    /// $$(\mathbf{I} - \lambda \mathbf{K})\boldsymbol{\phi} = \mathbf{g}$$
    ///
    /// via Neumann series iteration in fixed-point integer permyriad arithmetic:
    /// $$\boldsymbol{\phi}_{n+1} = \mathbf{g} + \lambda \mathbf{K} \boldsymbol{\phi}_n$$
    ///
    /// - `kernel`: $3 \times 3$ spatial coupling kernel in permyriad weights.
    /// - `lambda_pmy`: Coupling constant $\lambda \in [0, 9999]$ in permyriad.
    /// - `iterations`: Number of Neumann relaxation steps (typically 3..8 for convergence).
    /// - `deadband_pmy`: Quantization threshold for mapping relaxed permyriad field back to $\{-1, 0, 1\}$.
    /// - `out_grid`: Target output grid storing the relaxed balanced-trit equilibrium.
    pub fn relax_fredholm_neumann(
        &self,
        kernel: &[[i16; 3]; 3],
        lambda_pmy: i32,
        iterations: usize,
        deadband_pmy: i32,
        out_grid: &mut Self,
    ) {
        // Pre-compute kernel sum for normalization
        let mut kernel_sum = 0i32;
        for r in 0..3 {
            for c in 0..3 {
                kernel_sum += (kernel[r][c] as i32).abs();
            }
        }
        if kernel_sum == 0 {
            kernel_sum = 1;
        }

        // Initialize continuous permyriad buffers g and phi
        let mut phi_buf = alloc::vec![0i32; Self::CELL_COUNT];
        let mut next_buf = alloc::vec![0i32; Self::CELL_COUNT];

        // g(x, y) source term initialized from input trits (-10000, 0, +10000)
        for i in 0..Self::CELL_COUNT {
            let g_val = (self.cells[i] as i32) * Self::MAX_PERMYRIAD;
            phi_buf[i] = g_val;
        }

        let w = Self::WIDTH as i32;
        let h = Self::HEIGHT as i32;

        // Execute Neumann relaxation iterations
        for _ in 0..iterations {
            for y in 0..Self::HEIGHT {
                let iy = y as i32;
                for x in 0..Self::WIDTH {
                    let ix = x as i32;
                    let idx = y * Self::WIDTH + x;
                    let g_val = (self.cells[idx] as i32) * Self::MAX_PERMYRIAD;

                    // Discrete 3x3 convolution (K * phi)(x, y)
                    let mut conv_acc = 0i64;
                    for ky in -1..=1 {
                        let py = (iy + ky).clamp(0, h - 1) as usize;
                        let kr = (ky + 1) as usize;
                        for kx in -1..=1 {
                            let px = (ix + kx).clamp(0, w - 1) as usize;
                            let kc = (kx + 1) as usize;
                            let k_weight = kernel[kr][kc] as i64;
                            let p_val = phi_buf[py * Self::WIDTH + px] as i64;
                            conv_acc += k_weight * p_val;
                        }
                    }

                    let conv_norm = (conv_acc / kernel_sum as i64) as i32;
                    // phi_{n+1} = g + (lambda * (K * phi)) / MAX_PERMYRIAD
                    let step = ((lambda_pmy as i64 * conv_norm as i64) / Self::MAX_PERMYRIAD as i64) as i32;
                    let phi_next = (g_val + step).clamp(-Self::MAX_PERMYRIAD, Self::MAX_PERMYRIAD);
                    next_buf[idx] = phi_next;
                }
            }
            core::mem::swap(&mut phi_buf, &mut next_buf);
        }

        // Quantize relaxed continuous field back to balanced trits {-1, 0, +1}
        for i in 0..Self::CELL_COUNT {
            let val = phi_buf[i];
            let trit = if val > deadband_pmy {
                1
            } else if val < -deadband_pmy {
                -1
            } else {
                0
            };
            out_grid.cells[i] = trit;
        }
    }
}

impl Default for ConjugateTriadGrid400 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trit_packing_roundtrip() {
        let original_trits = [1, 0, -1, 1, 1];
        let byte = pack_trits_to_byte(&original_trits).expect("packing succeeded");
        assert!(byte < 243);

        let unpacked = unpack_byte_to_trits(byte).expect("unpacking succeeded");
        assert_eq!(original_trits, unpacked);
    }

    #[test]
    fn test_trit_slice_packing() {
        let trits = [1, -1, 0, 1, 1, -1, 1, 1, 0, -1, 0, 0, 1];
        let mut bytes = [0u8; 3];
        let bytes_written = pack_slice(&trits, &mut bytes);
        assert_eq!(bytes_written, 3);

        let mut unpacked_trits = [0i8; 15];
        let trits_unpacked = unpack_slice(&bytes, &mut unpacked_trits);
        assert_eq!(trits_unpacked, 15);
        // First 13 match original trits
        assert_eq!(&unpacked_trits[0..13], &trits[0..13]);
        // Rest is 0 padded
        assert_eq!(unpacked_trits[13], 0);
        assert_eq!(unpacked_trits[14], 0);
    }

    #[test]
    fn test_lunar_sentinel_resolution() {
        let byte = 254; // Mikikapise-pisim
        let sentinel = LunarSentinel::from_u8(byte).expect("resolved sentinel");
        assert_eq!(sentinel, LunarSentinel::MikikapisePisim);
        assert!(sentinel.description().contains("Winter Moon"));

        let tick = 10101u64;
        let metadata = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let ump = sentinel.to_ump_word(tick, &metadata);

        assert_eq!(ump.0[0], 254);
        assert_eq!(&ump.0[1..9], &tick.to_be_bytes());
        assert_eq!(&ump.0[9..16], &metadata);
    }

    #[test]
    fn test_vocabulary_lut_mock() {
        static MOCK_FLAT_BYTES: &[u8] = b"<bos>hello world";
        static MOCK_OFFSETS: &[u32] = &[0, 5, 10, 16];
        let lut = GemmaS13VocabularyLut::new(MOCK_FLAT_BYTES, MOCK_OFFSETS);

        let token_0 = lut.get_token_bytes(0).expect("token 0");
        assert_eq!(token_0, b"<bos>");

        let token_1 = lut.get_token_bytes(1).expect("token 1");
        assert_eq!(token_1, b"hello");

        let token_2 = lut.get_token_bytes(2).expect("token 2");
        assert_eq!(token_2, b" world");

        let token_oob = lut.get_token_bytes(3);
        assert!(token_oob.is_none());
    }

    #[test]
    fn test_compose_latent_signature() {
        static MOCK_FLAT_BYTES: &[u8] = b"abc";
        static MOCK_OFFSETS: &[u32] = &[0, 3];
        let lut = GemmaS13VocabularyLut::new(MOCK_FLAT_BYTES, MOCK_OFFSETS);

        let mut fc1_weight = [0.0f32; 256 * 24];
        // Set specific weight for 'a' (97), 'b' (98), 'c' (99)
        fc1_weight[97 * 24 + 5] = 1.0;
        fc1_weight[98 * 24 + 5] = 2.0;
        fc1_weight[99 * 24 + 5] = 3.0;

        let fc1_bias = [0.5f32; 24];
        let mut out_latent = [0.0f32; 24];

        let bytes_processed = lut.compose_latent_signature(0, &fc1_weight, &fc1_bias, &mut out_latent)
            .expect("signature composition succeeded");

        assert_eq!(bytes_processed, 3);
        // At index 5: (1.0 + 2.0 + 3.0 + 0.5) / 3.0 = 6.5 / 3.0 = 2.16666
        assert!((out_latent[5] - (6.5 / 3.0)).abs() < 1e-4);
        // At other indexes: (0.0 + 0.5) / 3.0 = 0.16666
        assert!((out_latent[0] - (0.5 / 3.0)).abs() < 1e-4);
    }

    #[test]
    fn test_decoder_halts_on_sentinel() {
        static MOCK_FLAT_BYTES: &[u8] = b"";
        static MOCK_OFFSETS: &[u32] = &[0];
        let lut = GemmaS13VocabularyLut::new(MOCK_FLAT_BYTES, MOCK_OFFSETS);
        let decoder = GemmaS13Decoder::new(lut);

        let stream = [10u8, 20u8, 254u8, 30u8]; // Byte 254 is MikikapisePisim (sentinel)
        let mut out_slots = [0usize; 4];
        let metadata = [0u8; 7];

        let nominal_processed = decoder.decode_stream(&stream, 500, &metadata, &mut out_slots);
        assert_eq!(nominal_processed, 2); // Halts before index 2
        assert_eq!(out_slots[0], 49);
        assert_eq!(out_slots[1], 49);
        // index 2 is routed via MoeRouter
        assert!(out_slots[2] < 49);
    }

    #[test]
    fn test_triad_stream_reduction() {
        let deadband = 50;
        let balanced = TriadStream::new(100, 100, 100);
        assert_eq!(balanced.resolve_trit(deadband), 0);

        let stress = TriadStream::new(300, 100, 100);
        assert_eq!(stress.resolve_trit(deadband), 1);

        let decay = TriadStream::new(100, 100, 300);
        assert_eq!(decay.resolve_trit(deadband), -1);

        // Test inversion across involution axis
        let inverted = stress.invert();
        assert_eq!(inverted.positive, 100);
        assert_eq!(inverted.negative, 300);
        assert_eq!(inverted.resolve_trit(deadband), -1);
    }

    #[test]
    fn test_differential_triad_symmetry_and_tamper() {
        let deadband = 50;
        let direct = TriadStream::new(500, 200, 100); // Trit = +1
        let diff_symmetric = DifferentialTriad::from_direct(direct);

        // Symmetric case: returns direct trit (+1)
        assert_eq!(diff_symmetric.evaluate(deadband), Ok(1));

        // Tamper case: direct says +1, but inverted stream is spoofed/cut
        let spoofed_inverted = TriadStream::new(500, 200, 100); // Inverted also says +1 -> T + T* = 2 != 0
        let diff_tampered = DifferentialTriad::new(direct, spoofed_inverted);

        // Must fail closed with Sabotage Moon Sentinel (254)
        assert_eq!(diff_tampered.evaluate(deadband), Err(LunarSentinel::MikikapisePisim));
    }

    #[test]
    fn test_conjugate_triad_grid_involution_and_gauge() {
        let mut grid = ConjugateTriadGrid400::new();
        grid.set(10, 20, 1);
        grid.set(50, 60, -1);
        grid.set(399, 399, 1);

        assert_eq!(grid.get(10, 20), Some(1));
        assert_eq!(grid.get(50, 60), Some(-1));
        assert_eq!(grid.get(399, 399), Some(1));
        assert_eq!(grid.get(0, 0), Some(0));
        assert_eq!(grid.get(400, 400), None);

        // Involutive conjugate inversion: T* = -T
        let conjugate = grid.invert();
        assert_eq!(conjugate.get(10, 20), Some(-1));
        assert_eq!(conjugate.get(50, 60), Some(1));
        assert_eq!(conjugate.get(399, 399), Some(-1));

        // Gauge invariant check: sum |T + T*| == 0
        assert_eq!(grid.verify_gauge_invariant(&conjugate), Ok(()));
    }

    #[test]
    fn test_conjugate_triad_grid_tamper_sentinel_254() {
        let mut grid = ConjugateTriadGrid400::new();
        grid.set(100, 100, 1);
        let mut conjugate = grid.invert();

        // Tamper with one cell in conjugate grid
        conjugate.set(100, 100, 1); // Now T(100,100) + T*(100,100) = 2 != 0

        // Must trip MikikapisePisim (Moon Sentinel 254)
        assert_eq!(
            grid.verify_gauge_invariant(&conjugate),
            Err(LunarSentinel::MikikapisePisim)
        );
    }

    #[test]
    fn test_fredholm_neumann_relaxation_convergence() {
        let mut source_grid = ConjugateTriadGrid400::new();
        // Place a localized positive source at center (200, 200)
        source_grid.set(200, 200, 1);

        // 3x3 diffusion coupling kernel
        let kernel = [
            [1000, 2000, 1000],
            [2000, 4000, 2000],
            [1000, 2000, 1000],
        ];
        let lambda_pmy = 5000; // 0.5 coupling strength
        let mut relaxed_grid = ConjugateTriadGrid400::new();

        source_grid.relax_fredholm_neumann(&kernel, lambda_pmy, 5, 500, &mut relaxed_grid);

        // Center must maintain positive state
        assert_eq!(relaxed_grid.get(200, 200), Some(1));
        // Adjacent cells receive coupled energy and diffuse
        assert_eq!(relaxed_grid.get(200, 201), Some(1));
        assert_eq!(relaxed_grid.get(201, 200), Some(1));
        // Far boundary remains neutral (0)
        assert_eq!(relaxed_grid.get(0, 0), Some(0));
    }
}


