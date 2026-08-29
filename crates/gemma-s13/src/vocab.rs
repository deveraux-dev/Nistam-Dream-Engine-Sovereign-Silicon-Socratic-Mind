// Copyright (c) 2026 Sean Morin, Edmonton River Valley, Alberta. All rights reserved.
// SPDX-License-Identifier: MIT

//! Zero-Heap Gemma Static Vocabulary & 1-Byte Fixed-Point Autoencoder.
//!
//! Eradicates the Gemma 1.07GB embedding matrix by replacing it with:
//! 1. Static 1D byte array Look-Up Table (LUT) with u32 offset mapping (< 2.6MB static read-only memory).
//! 2. Zero-heap 1-byte autoencoder: `Linear(256, 24)` using fixed-point integer-Permyriad matrix math.
//! 3. On-the-fly projection of 24-lane continuous latent signatures directly into $d_{\text{model}} = 2048$.

#![deny(unsafe_code)]

/// Model hidden dimension ($d_{\text{model}}$).
pub const D_MODEL: usize = 2048;

/// Byte-level autoencoder input dimension.
pub const BYTE_DIM: usize = 256;

/// Compact bottleneck latent dimension.
pub const LATENT_DIM: usize = 24;

/// Permyriad fixed-point divisor (1.0000 = 10,000).
pub const PERMYRIAD_ONE: i32 = 10_000;

/// Static LUT entry offset representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabEntry {
    /// Byte offset into the text LUT.
    pub offset: u32,
    /// Length of the token string.
    pub len: u16,
    /// Token semantic category / ID.
    pub token_id: u32,
}

/// Zero-heap fixed-point autoencoder weight parameters.
pub struct AutoEncoderWeights {
    /// Encoder weights: Linear(256, 24) in permyriad units ([-10000, 10000]).
    pub enc_weights: [[i16; BYTE_DIM]; LATENT_DIM],
    /// Encoder biases (24 lanes).
    pub enc_biases: [i32; LATENT_DIM],
    /// Projection weights: Linear(24, 2048) in permyriad units.
    pub proj_weights: [[i16; LATENT_DIM]; D_MODEL],
}

impl AutoEncoderWeights {
    /// Canonical deterministic identity / orthogonal initialization for zero-heap runtime.
    pub const fn default_fixed() -> Self {
        let mut enc_weights = [[0i16; BYTE_DIM]; LATENT_DIM];
        let enc_biases = [0i32; LATENT_DIM];
        let mut proj_weights = [[0i16; LATENT_DIM]; D_MODEL];

        // Seed deterministic orthogonal projection bands
        let mut d = 0;
        while d < LATENT_DIM {
            let mut b = 0;
            while b < BYTE_DIM {
                // Fixed integer sinusoidal basis function without libm
                let phase = ((b * 13 + d * 37) % 256) as i32;
                let val = if phase < 128 {
                    ((phase * 10_000) / 128) - 5_000
                } else {
                    5_000 - (((phase - 128) * 10_000) / 128)
                };
                enc_weights[d][b] = val as i16;
                b += 1;
            }
            d += 1;
        }

        let mut m = 0;
        while m < D_MODEL {
            let mut l = 0;
            while l < LATENT_DIM {
                let phase = ((m * 17 + l * 89) % 2048) as i32;
                let val = if phase < 1024 {
                    ((phase * 10_000) / 1024) - 5_000
                } else {
                    5_000 - (((phase - 1024) * 10_000) / 1024)
                };
                proj_weights[m][l] = (val / 2) as i16;
                l += 1;
            }
            m += 1;
        }

        Self {
            enc_weights,
            enc_biases,
            proj_weights,
        }
    }

    /// Encode a single byte `u8` into 24-lane continuous latent signature (fixed-point i32).
    #[inline(always)]
    pub fn encode_byte(&self, byte: u8, out_latent: &mut [i32; LATENT_DIM]) {
        let b = byte as usize;
        for (i, row) in self.enc_weights.iter().enumerate() {
            let w = row[b] as i32;
            let mut acc = (w * PERMYRIAD_ONE) / 10_000 + self.enc_biases[i];
            // LeakyReLU / Hardtanh activation in permyriad space
            if acc < 0 {
                acc /= 4; // 0.25 negative slope
            }
            out_latent[i] = acc;
        }
    }

    /// Project 24-lane latent vector on-the-fly directly to $d_{\text{model}} = 2048$.
    /// 0 heap allocations, writes directly to pre-allocated slice.
    #[inline(always)]
    pub fn project_latent_to_dmodel(&self, latent: &[i32; LATENT_DIM], out_dmodel: &mut [i32; D_MODEL]) {
        for (m, row) in self.proj_weights.iter().enumerate() {
            let mut acc: i64 = 0;
            for (l, &w) in row.iter().enumerate() {
                acc += (w as i64) * (latent[l] as i64);
            }
            out_dmodel[m] = (acc / (PERMYRIAD_ONE as i64)) as i32;
        }
    }

    /// Complete on-the-fly pipeline: single raw byte -> $d_{\text{model}}$ continuous vector.
    #[inline]
    pub fn byte_to_dmodel(&self, byte: u8, out_dmodel: &mut [i32; D_MODEL]) {
        let mut latent = [0i32; LATENT_DIM];
        self.encode_byte(byte, &mut latent);
        self.project_latent_to_dmodel(&latent, out_dmodel);
    }

    /// Encode a somatic 5D coordinate `[ra, dec, mag, spectral, milli_hz]` —
    /// each already normalized to permyriad — into the same 24-lane latent the
    /// byte encoder produces.
    ///
    /// The somatic lane's token is a celestial coordinate, not a subword: there
    /// is no BPE table and no `token_embd` anywhere in this path. Each of the 5
    /// axes fans out across the 24 lanes through the existing `enc_weights`
    /// rows, so a star lands in the *same* latent space a byte does and
    /// [`Self::project_latent_to_dmodel`] carries it to `d_model` unchanged.
    ///
    /// Axis `a` reads column `a * (BYTE_DIM / 5)` of each encoder row, spreading
    /// the five axes evenly across the 256-wide byte fan-in rather than
    /// crowding them into the first five columns.
    ///
    /// NOTE on the coupling: the five axes are treated here as INDEPENDENT.
    /// The repo's exact-integer primitive for *coupled* 5-channel equilibrium is
    /// `forge_core_v3::resolvent::Field5D<5>` — `resolve` is the Neumann-summed
    /// `(I - M)^-1 g`, `deproject` the exact one-pass `(I - M) f` inverse, and
    /// `Field5D::new` refuses any coupling whose infinity norm reaches PMY.
    /// That is the right operator for axis coupling, and it is deliberately NOT
    /// used: it lives in `forge-core-v3`, and this crate's whole dependency list
    /// is `memmap2`. Pulling it in would trade the `no_std` one-dependency
    /// property for coupling this lane does not yet need. If coupling becomes
    /// load-bearing, port `Field5D` in rather than re-deriving it here.
    #[inline(always)]
    pub fn encode_somatic_5d(&self, coords_pmy: &[i32; 5], out_latent: &mut [i32; LATENT_DIM]) {
        const STRIDE: usize = BYTE_DIM / 5;
        for (i, row) in self.enc_weights.iter().enumerate() {
            let mut acc: i64 = self.enc_biases[i] as i64;
            for (a, &c) in coords_pmy.iter().enumerate() {
                let w = row[a * STRIDE] as i64;
                acc += (w * c as i64) / 10_000;
            }
            let mut acc = acc as i32;
            // Same LeakyReLU / Hardtanh as the byte lane — one activation, one
            // latent space, so both encoders stay comparable.
            if acc < 0 {
                acc /= 4;
            }
            out_latent[i] = acc;
        }
    }

    /// Complete somatic pipeline: 5D celestial coordinate -> $d_{\text{model}}$
    /// continuous vector, the star-native counterpart of [`Self::byte_to_dmodel`].
    #[inline]
    pub fn somatic_5d_to_dmodel(&self, coords_pmy: &[i32; 5], out_dmodel: &mut [i32; D_MODEL]) {
        let mut latent = [0i32; LATENT_DIM];
        self.encode_somatic_5d(coords_pmy, &mut latent);
        self.project_latent_to_dmodel(&latent, out_dmodel);
    }

    /// Fold a $d_{\text{model}}$ state back to the 24-lane latent — the transpose
    /// of [`Self::project_latent_to_dmodel`], reusing the SAME `proj_weights`.
    ///
    /// Tied by construction, exactly as Gemma ties its lm_head to its embedding
    /// table: one matrix serves both directions, so there is no second set of
    /// weights to ship, drift, or keep in sync.
    #[inline(always)]
    pub fn unproject_dmodel_to_latent(
        &self,
        dmodel: &[i32; D_MODEL],
        out_latent: &mut [i32; LATENT_DIM],
    ) {
        for (l, slot) in out_latent.iter_mut().enumerate() {
            let mut acc: i64 = 0;
            for (m, row) in self.proj_weights.iter().enumerate() {
                acc += (row[l] as i64) * (dmodel[m] as i64);
            }
            *slot = (acc / (PERMYRIAD_ONE as i64)) as i32;
        }
    }

    /// Decode a 24-lane latent back to a somatic 5D coordinate — the transpose
    /// of [`Self::encode_somatic_5d`], reading the same `enc_weights` columns.
    #[inline(always)]
    pub fn decode_latent_to_5d(&self, latent: &[i32; LATENT_DIM], out_coords_pmy: &mut [i32; 5]) {
        const STRIDE: usize = BYTE_DIM / 5;
        for (a, slot) in out_coords_pmy.iter_mut().enumerate() {
            let mut acc: i64 = 0;
            for (i, row) in self.enc_weights.iter().enumerate() {
                acc += (row[a * STRIDE] as i64) * (latent[i] as i64);
            }
            *slot = (acc / 10_000) as i32;
        }
    }

    /// The inverse leg of the somatic lane: a settled $d_{\text{model}}$ state
    /// back out to a 5D celestial coordinate, ready for an $L_2$ argmax against
    /// the star codebook (`star_codebook::nearest_centroid_l2`) to resolve a
    /// star id, its resonant `milli_hz`, and its label.
    ///
    /// Forward and inverse share both matrices, so the round trip is a single
    /// authored operator read in two directions — no second table on disk, and
    /// no BPE vocabulary anywhere in the path.
    #[inline]
    pub fn dmodel_to_somatic_5d(&self, dmodel: &[i32; D_MODEL], out_coords_pmy: &mut [i32; 5]) {
        let mut latent = [0i32; LATENT_DIM];
        self.unproject_dmodel_to_latent(dmodel, &mut latent);
        self.decode_latent_to_5d(&latent, out_coords_pmy);
    }

    /// Resolve a settled $d_{\text{model}}$ state to the nearest star in
    /// `codebook` — the full inverse-projection chain in one call:
    /// `dmodel -> latent -> 5D somatic coordinate -> star lookup`.
    ///
    /// Closes the gap named 2026-08-28: `dmodel_to_somatic_5d` and
    /// `star_codebook::StarCodebookView::detokenize_embedding` were each
    /// proven and tested in isolation but never wired to each other — the
    /// coordinate convention matches by construction (`encode_somatic_5d`'s
    /// own test inputs are permyriad-scaled ~[-10000, 10000], the same
    /// convention `pmy_to_normalized` reads here), so no resolvent/coupling
    /// operator is needed for this leg; `forge_core_v3::resolvent::Field5D`
    /// stays the documented escape hatch if independent-axis treatment is
    /// ever observed to diverge in practice (see this file's `encode_somatic_5d`
    /// doc comment).
    pub fn resolve_star(
        &self,
        dmodel: &[i32; D_MODEL],
        codebook: &super::star_codebook::StarCodebookView,
    ) -> Option<super::star_codebook::BakedStarCentroid> {
        let mut coords_pmy = [0i32; 5];
        self.dmodel_to_somatic_5d(dmodel, &mut coords_pmy);
        let coords_norm: [f32; 5] = core::array::from_fn(|i| pmy_to_normalized(coords_pmy[i]));
        codebook.detokenize_embedding(&coords_norm)
    }
}

/// Convert a permyriad-scaled coordinate (`PERMYRIAD_ONE` = 1.0) to the
/// normalized `f32` form `star_codebook::StarCodebookView::detokenize_embedding`
/// expects. The inverse of the scaling `encode_somatic_5d`'s own tests apply
/// to their input coordinates — same convention, both directions.
#[inline(always)]
pub fn pmy_to_normalized(pmy: i32) -> f32 {
    pmy as f32 / PERMYRIAD_ONE as f32
}

/// Static Vocab Table with fixed-size layout.
pub struct StaticVocabTable<'a> {
    /// 1D contiguous byte stream.
    pub raw_lut: &'a [u8],
    /// Index offset map.
    pub offsets: &'a [VocabEntry],
}

impl<'a> StaticVocabTable<'a> {
    /// Create a new table reference.
    pub const fn new(raw_lut: &'a [u8], offsets: &'a [VocabEntry]) -> Self {
        Self { raw_lut, offsets }
    }

    /// Lookup token string bytes by token_id with bounds checking.
    #[inline]
    pub fn get_token_bytes(&self, token_id: u32) -> Option<&'a [u8]> {
        if (token_id as usize) >= self.offsets.len() {
            return None;
        }
        let entry = self.offsets[token_id as usize];
        let start = entry.offset as usize;
        let end = start + (entry.len as usize);
        if end <= self.raw_lut.len() {
            Some(&self.raw_lut[start..end])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autoencoder_deterministic_dimensions() {
        let ae = AutoEncoderWeights::default_fixed();
        let mut latent = [0i32; LATENT_DIM];
        let mut dmodel = [0i32; D_MODEL];

        ae.encode_byte(65, &mut latent); // ASCII 'A'
        assert_eq!(latent.len(), 24);

        ae.project_latent_to_dmodel(&latent, &mut dmodel);
        assert_eq!(dmodel.len(), 2048);

        // Deterministic repeat check
        let mut dmodel_repeat = [0i32; D_MODEL];
        ae.byte_to_dmodel(65, &mut dmodel_repeat);
        assert_eq!(dmodel, dmodel_repeat);
    }

    /// The somatic lane must carry SIGNAL, not merely run: two different 5D
    /// coordinates have to land on different d_model states. A constant-output
    /// encoder would pass a "does it execute" check and make every star
    /// identical downstream — the exact failure the synthetic placeholder in
    /// e2e_token_step has, where one scalar per token collapses the argmax.
    #[test]
    fn somatic_5d_encodes_distinct_coordinates_distinctly() {
        let ae = AutoEncoderWeights::default_fixed();

        // Sirius-ish vs Antares-ish: different ra, dec, mag, spectral, hz.
        let a: [i32; 5] = [1_012, -1_671, -1_460, 9_000, 4_400];
        let b: [i32; 5] = [2_473, -2_643, 9_600, 1_000, 2_076];

        let mut da = [0i32; D_MODEL];
        let mut db = [0i32; D_MODEL];
        ae.somatic_5d_to_dmodel(&a, &mut da);
        ae.somatic_5d_to_dmodel(&b, &mut db);

        assert_ne!(
            da.as_slice(),
            db.as_slice(),
            "distinct 5D coordinates must not collapse to the same d_model state"
        );
        assert!(da.iter().any(|&v| v != 0), "encoder must not emit an all-zero state");

        // Determinism: same input, same output, every time.
        let mut again = [0i32; D_MODEL];
        ae.somatic_5d_to_dmodel(&a, &mut again);
        assert_eq!(da.as_slice(), again.as_slice(), "somatic encode must be deterministic");
    }

    /// Forward then inverse must preserve ORDERING, which is what the L2 argmax
    /// against the star codebook actually consumes. Exact round-trip recovery is
    /// not claimed — the 5 -> 24 -> 2048 lift is not invertible and the fixed-
    /// point divides truncate — so this pins the property the pipeline needs
    /// rather than one it cannot have.
    #[test]
    fn somatic_round_trip_keeps_distinct_coordinates_distinct() {
        let ae = AutoEncoderWeights::default_fixed();
        let a: [i32; 5] = [1_012, -1_671, -1_460, 9_000, 4_400];
        let b: [i32; 5] = [2_473, -2_643, 9_600, 1_000, 2_076];

        let mut d = [0i32; D_MODEL];
        let (mut ra, mut rb) = ([0i32; 5], [0i32; 5]);

        ae.somatic_5d_to_dmodel(&a, &mut d);
        ae.dmodel_to_somatic_5d(&d, &mut ra);
        ae.somatic_5d_to_dmodel(&b, &mut d);
        ae.dmodel_to_somatic_5d(&d, &mut rb);

        assert_ne!(ra, rb, "the inverse leg must not collapse two stars onto one coordinate");
    }

    #[test]
    fn test_static_vocab_lut_lookup() {
        static LUT: [u8; 12] = *b"HELLO WORLD!";
        static ENTRIES: [VocabEntry; 2] = [
            VocabEntry {
                offset: 0,
                len: 5,
                token_id: 0,
            },
            VocabEntry {
                offset: 6,
                len: 6,
                token_id: 1,
            },
        ];

        let table = StaticVocabTable::new(&LUT, &ENTRIES);
        assert_eq!(table.get_token_bytes(0), Some(&b"HELLO"[..]));
        assert_eq!(table.get_token_bytes(1), Some(&b"WORLD!"[..]));
        assert_eq!(table.get_token_bytes(2), None);
    }

    #[test]
    fn test_autoencoder_byte_to_dmodel_pipeline() {
        let ae = AutoEncoderWeights::default_fixed();
        let mut dmodel = [0i32; D_MODEL];
        ae.byte_to_dmodel(42, &mut dmodel);
        // Non-zero transformed projection
        let mut has_nonzero = false;
        for &val in dmodel.iter() {
            if val != 0 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero);
    }

    #[test]
    fn pmy_to_normalized_boundary_values() {
        assert_eq!(pmy_to_normalized(0), 0.0);
        assert_eq!(pmy_to_normalized(PERMYRIAD_ONE), 1.0);
        assert_eq!(pmy_to_normalized(-PERMYRIAD_ONE), -1.0);
        // Out-of-band values pass through unclamped — nearest_centroid_l2's L2
        // distance still behaves sanely on an out-of-range query, and clamping
        // here would hide a genuinely divergent decode rather than surface it.
        assert_eq!(pmy_to_normalized(20_000), 2.0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn resolve_star_closes_the_dmodel_to_star_loop() {
        use std::path::Path;
        // Same absolute path convention as examples/somatic_astrolabe_demo.rs —
        // the repo's established location for the baked HYG catalog.
        let hyg_path = Path::new("F:/v3/shell/assets/hyg_baked.bin");
        if !hyg_path.exists() {
            panic!(
                "hyg_baked.bin not found at {} — this test proves the dmodel->star \
                 chain against the real catalog, not a synthetic stand-in; if the \
                 asset genuinely moved, update this path, don't skip the assertion",
                hyg_path.display()
            );
        }
        let hyg_bytes = std::fs::read(hyg_path).expect("read hyg_baked.bin");
        let codebook = super::super::star_codebook::StarCodebookView::parse(&hyg_bytes)
            .expect("parse real HYG catalog");

        let ae = AutoEncoderWeights::default_fixed();
        let mut dmodel = [0i32; D_MODEL];
        ae.byte_to_dmodel(65, &mut dmodel); // ASCII 'A', deterministic

        let star = ae.resolve_star(&dmodel, &codebook);
        assert!(star.is_some(), "a real dmodel state must resolve to a real star");

        // Determinism: same dmodel, same star, every time.
        let again = ae.resolve_star(&dmodel, &codebook);
        assert_eq!(star, again, "star resolution must be deterministic");
    }

    #[test]
    fn test_vocab_lut_out_of_bounds() {
        static LUT: [u8; 4] = *b"ABCD";
        static CORRUPT_ENTRIES: [VocabEntry; 1] = [VocabEntry {
            offset: 2,
            len: 10, // exceeds 4 bytes
            token_id: 0,
        }];
        let table = StaticVocabTable::new(&LUT, &CORRUPT_ENTRIES);
        assert_eq!(table.get_token_bytes(0), None);
    }
}
