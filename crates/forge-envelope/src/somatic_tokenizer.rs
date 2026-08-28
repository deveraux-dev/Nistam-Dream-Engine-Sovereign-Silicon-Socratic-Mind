//! Somatic Tokenizer — Offline Real-Time Tactile, Photometric & 5D Celestial Astrolabe Encoding.
//!
//! Maps high-frequency XInput bitfields, photometric surface gradients, and 5D celestial
//! coordinates [ra, dec, mag, spectral, hz] directly into continuous coordinate spaces and
//! model embedding manifolds in `#![no_std]` Rust without dynamic heap allocations.

/// 5D Somatic Celestial Astrolabe coordinate tuple in physical units.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CelestialCoordinates5D {
    /// Right Ascension in centidegrees (0..36,000 = 0.00° .. 360.00°).
    pub ra_cdeg: u32,
    /// Declination in centidegrees (-9,000..+9,000 = -90.00° .. +90.00°).
    pub dec_cdeg: i32,
    /// Apparent magnitude in Permyriads (-2,000..+15,000 = -2.00 .. +15.00 mag).
    pub mag_pmy: i32,
    /// Harvard Spectral Class (0=O, 1=B, 2=A, 3=F, 4=G, 5=K, 6=M, 7=Unknown/Other).
    pub spectral_class: u8,
    /// Resonant stellar pulsation / Schumann frequency in millihertz (0..100,000 mHz).
    pub milli_hz: u32,
}

impl CelestialCoordinates5D {
    /// Maximum Right Ascension in centidegrees (360° = 36,000 cdeg).
    pub const MAX_RA_CDEG: f32 = 36_000.0;
    /// Maximum absolute Declination in centidegrees (90° = 9,000 cdeg).
    pub const MAX_DEC_CDEG: f32 = 9_000.0;
    /// Reference magnitude offset in Permyriads (-2,000 pmy).
    pub const MIN_MAG_PMY: f32 = -2_000.0;
    /// Reference magnitude range in Permyriads (17,000 pmy).
    pub const RANGE_MAG_PMY: f32 = 17_000.0;
    /// Maximum spectral class index (7 = Other).
    pub const MAX_SPECTRAL_CLASS: f32 = 7.0;
    /// Maximum resonant frequency in millihertz (100,000 mHz = 100 Hz).
    pub const MAX_MILLI_HZ: f32 = 100_000.0;

    /// Normalize the 5D celestial tuple into a continuous `[f32; 5]` bounded space:
    /// - `ra`: [0.0, 1.0]
    /// - `dec`: [-1.0, 1.0]
    /// - `mag`: [0.0, 1.0]
    /// - `spectral`: [0.0, 1.0]
    /// - `milli_hz`: [0.0, 1.0]
    #[inline(always)]
    pub fn normalize_zero_heap(&self) -> [f32; 5] {
        let ra_norm = ((self.ra_cdeg as f32) % Self::MAX_RA_CDEG) / Self::MAX_RA_CDEG;
        let dec_norm = (self.dec_cdeg as f32).clamp(-Self::MAX_DEC_CDEG, Self::MAX_DEC_CDEG) / Self::MAX_DEC_CDEG;
        let mag_norm = ((self.mag_pmy as f32) - Self::MIN_MAG_PMY).clamp(0.0, Self::RANGE_MAG_PMY) / Self::RANGE_MAG_PMY;
        let spectral_norm = (self.spectral_class as f32).clamp(0.0, Self::MAX_SPECTRAL_CLASS) / Self::MAX_SPECTRAL_CLASS;
        let hz_norm = (self.milli_hz as f32).clamp(0.0, Self::MAX_MILLI_HZ) / Self::MAX_MILLI_HZ;
        [ra_norm, dec_norm, mag_norm, spectral_norm, hz_norm]
    }

    /// Denormalize a continuous `[f32; 5]` vector back into discrete `CelestialCoordinates5D`.
    #[inline(always)]
    pub fn from_normalized_5d(coords: &[f32; 5]) -> Self {
        let ra_cdeg = ((coords[0].clamp(0.0, 1.0) * Self::MAX_RA_CDEG) as u32).min(36_000);
        let dec_cdeg = (coords[1].clamp(-1.0, 1.0) * Self::MAX_DEC_CDEG) as i32;
        let mag_pmy = (coords[2].clamp(0.0, 1.0) * Self::RANGE_MAG_PMY + Self::MIN_MAG_PMY) as i32;
        let spectral_class = (coords[3].clamp(0.0, 1.0) * Self::MAX_SPECTRAL_CLASS).round() as u8;
        let milli_hz = ((coords[4].clamp(0.0, 1.0) * Self::MAX_MILLI_HZ) as u32).min(100_000);

        Self {
            ra_cdeg,
            dec_cdeg,
            mag_pmy,
            spectral_class: spectral_class.min(7),
            milli_hz,
        }
    }
}

/// Kinematic physical-state snapshot recorded at a discrete metronome tick.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SomaticKinematics {
    /// Monotonically advancing simulation clock tick.
    pub tick: u64,
    /// 2D normalized surface coordinate position.
    pub pos: [f32; 2],
    /// Velocity vector (dPos / dt).
    pub vel: [f32; 2],
    /// Acceleration vector (d^2Pos / dt^2).
    pub acc: [f32; 2],
    /// The active balanced-ternary trit state (-1 = Inferno, 0 = Purgatorio, +1 = Paradiso).
    pub trit_state: i8,
}

/// Somatic tokenizer executing offline, zero-allocation signal translation.
pub struct EmergentSomaticTokenizer {
    vocab_size: usize,
}

impl EmergentSomaticTokenizer {
    /// Invariant upper bound of physical coordinates.
    pub const MAX_PERMYRIAD: f32 = 10_000.0;
    /// The metronome frequency interval (120 Hz).
    pub const DT: f32 = 1.0 / 120.0;

    /// 2B Gemma model hidden dimension (Baby Bear / Mama Bear).
    pub const DIM_2B: usize = 2048;
    /// 9B Gemma model hidden dimension (Papa Bear).
    pub const DIM_9B: usize = 3584;

    /// Construct a new somatic tokenizer.
    pub fn new() -> Self {
        Self { vocab_size: 65536 }
    }

    /// Retrieve the vocab size of the somatic state space.
    pub fn vocab_size(&self) -> usize {
        self.vocab_size
    }

    /// Translates raw 16-bit register bitfields into normalized coordinates on a stack-provided slice.
    ///
    /// Extends raw 5-bit signed packed integers into clamped, scale-invariant Permyriad ranges
    /// and performs continuous Euclidean L2 normalization in `#![no_std]`.
    #[inline(always)]
    pub fn encode_bitfields_zero_heap(&self, raw_inputs: &[u16], out_coords: &mut [[f32; 2]]) {
        debug_assert_eq!(raw_inputs.len(), out_coords.len());
        for (i, &raw) in raw_inputs.iter().enumerate() {
            // Unpack 5-bit signed X input from bottom 5 bits
            let x_raw = (raw & 0x1F) as i32;
            let x_signed = if x_raw & 0x10 != 0 { x_raw | !0x1F } else { x_raw };

            // Unpack 5-bit signed Y input from next 5 bits
            let y_raw = ((raw >> 5) & 0x1F) as i32;
            let y_signed = if y_raw & 0x10 != 0 { y_raw | !0x1F } else { y_raw };

            // Clamping directly to prevent two's-complement overflow
            let px = (x_signed.clamp(-15, 15) as f32 / 15.0) * Self::MAX_PERMYRIAD;
            let py = (y_signed.clamp(-15, 15) as f32 / 15.0) * Self::MAX_PERMYRIAD;

            let sum_sq = px * px + py * py;
            let mag = self.no_std_sqrt(sum_sq);

            let (nx, ny) = if mag > Self::MAX_PERMYRIAD {
                (px / mag, py / mag)
            } else {
                (px / Self::MAX_PERMYRIAD, py / Self::MAX_PERMYRIAD)
            };

            out_coords[i] = [nx, ny];
        }
    }

    /// Unpacks raw Modbus RTU / RS-485 16-bit analog sensor registers (e.g., strain gauge, thermal probe)
    /// into clamped Permyriad coordinates [0.0..1.0] without heap allocations.
    #[inline(always)]
    pub fn encode_modbus_registers_zero_heap(&self, registers: &[u16], out_normalized: &mut [f32]) {
        debug_assert_eq!(registers.len(), out_normalized.len());
        for (i, &reg) in registers.iter().enumerate() {
            // Unpack 16-bit unsigned raw sensor value, normalize against 0..10,000 Permyriads
            let clamped = (reg as f32).clamp(0.0, Self::MAX_PERMYRIAD);
            out_normalized[i] = clamped / Self::MAX_PERMYRIAD;
        }
    }

    /// Unpacks an 8-byte CAN Bus (ISO 11898) sensor frame into 4 x 16-bit signed telemetry channels.
    #[inline(always)]
    pub fn encode_can_frame_zero_heap(&self, frame_payload: &[u8; 8], out_channels: &mut [f32; 4]) {
        for ch in 0..4 {
            let offset = ch * 2;
            let raw_i16 = i16::from_be_bytes([frame_payload[offset], frame_payload[offset + 1]]);
            out_channels[ch] = (raw_i16 as f32) / 32767.0;
        }
    }

    /// Encodes a slice of normalized audio PCM float samples into balanced-ternary trits (`-1, 0, +1`)
    /// and calculates the RMS energy envelope without dynamic heap allocation.
    ///
    /// - Positive attack peaks (`sample > deadband`) -> `+1`
    /// - Negative troughs (`sample < -deadband`) -> `-1`
    /// - Silence / floor (`|sample| <= deadband`) -> `0`
    #[inline(always)]
    pub fn encode_audio_pcm_zero_heap(
        &self,
        samples: &[f32],
        deadband: f32,
        out_trits: &mut [i8],
    ) -> f32 {
        let count = samples.len().min(out_trits.len());
        if count == 0 {
            return 0.0;
        }

        let mut sum_sq = 0.0f32;
        for i in 0..count {
            let s = samples[i].clamp(-1.0, 1.0);
            sum_sq += s * s;
            if s > deadband {
                out_trits[i] = 1;
            } else if s < -deadband {
                out_trits[i] = -1;
            } else {
                out_trits[i] = 0;
            }
        }

        let mean_sq = sum_sq / (count as f32);
        self.no_std_sqrt(mean_sq)
    }

    /// Transduces legacy celestial coordinates and harmonic frequency into a 5D continuous coordinate vector
    /// and a dominant balanced-ternary trit state (-1 = Inferno, 0 = Purgatorio, +1 = Paradiso).
    ///
    /// Returns `([norm_ra, norm_dec, norm_mag, norm_hz, energy], trit_state)`.
    #[inline(always)]
    pub fn encode_celestial_harmonic(
        &self,
        ra_cdeg: u32,
        dec_cdeg: i32,
        mag_pmy: i32,
        milli_hz: u32,
    ) -> ([f32; 5], i8) {
        let norm_ra = ((ra_cdeg % 36_000) as f32 / 36_000.0) * 2.0 - 1.0;
        let norm_dec = (dec_cdeg.clamp(-9000, 9000) as f32) / 9000.0;
        let norm_mag = (mag_pmy.clamp(-15_000, 15_000) as f32) / 15_000.0;
        let norm_hz = (milli_hz.clamp(100_000, 880_000) as f32 - 440_000.0) / 440_000.0;

        let energy_sq = norm_ra * norm_ra + norm_dec * norm_dec + norm_hz * norm_hz;
        let energy = self.no_std_sqrt(energy_sq / 3.0);

        let trit_state = if norm_hz > 0.10 {
            1 // High celestial harmonic / Paradiso
        } else if norm_hz < -0.10 {
            -1 // Deep celestial undertone / Inferno
        } else {
            0 // Equilibrium / Purgatorio (A440 anchor)
        };

        ([norm_ra, norm_dec, norm_mag, norm_hz, energy], trit_state)
    }

    /// Projects a 5D celestial coordinate vector `[ra, dec, mag, spectral, hz]` into an arbitrary
    /// model hidden dimension `out_embedding` using an orthonormal discrete cosine carrier basis.
    ///
    /// Operates `#![no_std]` and zero-heap.
    #[inline(always)]
    pub fn project_up_5d_to_dim(&self, coords_5d: &[f32; 5], out_embedding: &mut [f32]) {
        let dim = out_embedding.len();
        if dim < 5 {
            return;
        }
        let norm_factor = self.no_std_sqrt(2.0 / (dim as f32));
        const PI: f32 = 3.14159265358979323846;

        for d in 0..dim {
            let mut sum = 0.0f32;
            let d_phase = (d as f32 + 0.5) / (dim as f32);
            for k in 0..5 {
                let freq = (k + 1) as f32 * PI;
                let carrier = norm_factor * Self::fast_cos_f32(d_phase * freq);
                sum += coords_5d[k] * carrier;
            }
            out_embedding[d] = sum;
        }
    }

    /// Projects a model hidden dimension embedding back down into 5D celestial coordinates `[f32; 5]`
    /// via adjoint inner product against the orthonormal discrete cosine carrier basis.
    ///
    /// Operates `#![no_std]` and zero-heap with exact pseudo-inverse reconstruction.
    #[inline(always)]
    pub fn project_down_dim_to_5d(&self, embedding: &[f32], out_5d: &mut [f32; 5]) {
        let dim = embedding.len();
        if dim < 5 {
            *out_5d = [0.0; 5];
            return;
        }
        let norm_factor = self.no_std_sqrt(2.0 / (dim as f32));
        const PI: f32 = 3.14159265358979323846;

        for k in 0..5 {
            let freq = (k + 1) as f32 * PI;
            let mut sum = 0.0f32;
            for d in 0..dim {
                let d_phase = (d as f32 + 0.5) / (dim as f32);
                let carrier = norm_factor * Self::fast_cos_f32(d_phase * freq);
                sum += embedding[d] * carrier;
            }
            out_5d[k] = sum;
        }
    }

    /// Fixed-point up-projection adapter for 2B Gemma models (`dim = 2048`).
    #[inline(always)]
    pub fn project_up_2048(&self, coords_5d: &[f32; 5], out_embedding: &mut [f32; Self::DIM_2B]) {
        self.project_up_5d_to_dim(coords_5d, out_embedding);
    }

    /// Fixed-point down-projection adapter for 2B Gemma models (`dim = 2048`).
    #[inline(always)]
    pub fn project_down_2048(&self, embedding: &[f32; Self::DIM_2B], out_5d: &mut [f32; 5]) {
        self.project_down_dim_to_5d(embedding, out_5d);
    }

    /// Fixed-point up-projection adapter for 9B Gemma model (`dim = 3584`).
    #[inline(always)]
    pub fn project_up_3584(&self, coords_5d: &[f32; 5], out_embedding: &mut [f32; Self::DIM_9B]) {
        self.project_up_5d_to_dim(coords_5d, out_embedding);
    }

    /// Fixed-point down-projection adapter for 9B Gemma model (`dim = 3584`).
    #[inline(always)]
    pub fn project_down_3584(&self, embedding: &[f32; Self::DIM_9B], out_5d: &mut [f32; 5]) {
        self.project_down_dim_to_5d(embedding, out_5d);
    }

    /// Fast, deterministic `#![no_std]` Taylor cosine approximation with quadrant range reduction.
    #[inline(always)]
    pub fn fast_cos_f32(x: f32) -> f32 {
        const PI: f32 = 3.14159265358979323846;
        const TWO_PI: f32 = 6.28318530717958647692;
        const HALF_PI: f32 = 1.57079632679489661923;

        let mut theta = x % TWO_PI;
        if theta < 0.0 {
            theta += TWO_PI;
        }

        let (z, sign) = if theta <= HALF_PI {
            (theta, 1.0f32)
        } else if theta <= PI {
            (PI - theta, -1.0f32)
        } else if theta <= PI + HALF_PI {
            (theta - PI, -1.0f32)
        } else {
            (TWO_PI - theta, 1.0f32)
        };

        let z2 = z * z;
        let term2 = z2 * 0.5;
        let term4 = (z2 * term2) / 12.0;
        let term6 = (z2 * term4) / 30.0;
        let term8 = (z2 * term6) / 56.0;
        let term10 = (z2 * term8) / 90.0;

        sign * (1.0 - term2 + term4 - term6 + term8 - term10)
    }

    /// A fast, deterministic `#![no_std]` Babylonian square-root solver.
    /// Employs safe `to_bits` and `from_bits` to comply with strict `-D unsafe-code` rules.
    #[inline(always)]
    fn no_std_sqrt(&self, value: f32) -> f32 {
        if value <= 0.0 {
            return 0.0;
        }
        // Initial estimate via bit manipulation (safe equivalent of Quake's fast inv sqrt seed)
        let i = value.to_bits();
        let i_approx = 0x1fbd1df5 + (i >> 1); // Approximation seed
        let mut x = f32::from_bits(i_approx);

        // 3 iterations of Babylonian refinement are sufficient for sub-millimeter float precision
        x = 0.5 * (x + value / x);
        x = 0.5 * (x + value / x);
        x = 0.5 * (x + value / x);
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_somatic_tokenizer_unpack() {
        let tokenizer = EmergentSomaticTokenizer::new();
        // 0x0000 -> x=0, y=0
        // 0x000F -> x=15 (max positive), y=0
        // 0x01E0 -> x=0, y=15 (max positive)
        let inputs = [0x0000, 0x000F, 0x01E0];
        let mut out = [[0.0f32; 2]; 3];
        tokenizer.encode_bitfields_zero_heap(&inputs, &mut out);

        assert_eq!(out[0], [0.0, 0.0]);
        assert!((out[1][0] - 1.0).abs() < 1e-4);
        assert!((out[1][1] - 0.0).abs() < 1e-4);
        assert!((out[2][0] - 0.0).abs() < 1e-4);
        assert!((out[2][1] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_modbus_and_can_unpack() {
        let tokenizer = EmergentSomaticTokenizer::new();

        // Test Modbus RTU 16-bit register scaling
        let modbus_regs = [5000u16, 10000u16, 15000u16];
        let mut modbus_out = [0.0f32; 3];
        tokenizer.encode_modbus_registers_zero_heap(&modbus_regs, &mut modbus_out);

        assert!((modbus_out[0] - 0.5).abs() < 1e-4);
        assert!((modbus_out[1] - 1.0).abs() < 1e-4);
        assert!((modbus_out[2] - 1.0).abs() < 1e-4); // Clamped to 1.0

        // Test CAN Bus 8-byte frame unpacking
        let can_frame = [0x7F, 0xFF, 0x80, 0x01, 0x00, 0x00, 0x40, 0x00];
        let mut can_channels = [0.0f32; 4];
        tokenizer.encode_can_frame_zero_heap(&can_frame, &mut can_channels);

        assert!((can_channels[0] - 1.0).abs() < 1e-3); // 32767 / 32767 = 1.0
        assert!((can_channels[1] - (-1.0)).abs() < 1e-3); // -32767 / 32767 = -1.0
        assert_eq!(can_channels[2], 0.0); // 0 / 32767 = 0.0
        assert!((can_channels[3] - 0.5).abs() < 1e-3); // 16384 / 32767 ≈ 0.5
    }

    #[test]
    fn test_audio_pcm_zero_heap() {
        let tokenizer = EmergentSomaticTokenizer::new();
        let samples = [0.0f32, 0.05, 0.8, -0.9, -0.02, 0.5];
        let mut trits = [0i8; 6];
        let deadband = 0.1f32;

        let rms = tokenizer.encode_audio_pcm_zero_heap(&samples, deadband, &mut trits);

        assert_eq!(trits[0], 0);  // 0.0 <= 0.1
        assert_eq!(trits[1], 0);  // 0.05 <= 0.1
        assert_eq!(trits[2], 1);  // 0.8 > 0.1
        assert_eq!(trits[3], -1); // -0.9 < -0.1
        assert_eq!(trits[4], 0);  // -0.02 > -0.1
        assert_eq!(trits[5], 1);  // 0.5 > 0.1

        assert!(rms > 0.4 && rms < 0.6);
    }

    #[test]
    fn test_encode_celestial_harmonic() {
        let tokenizer = EmergentSomaticTokenizer::new();
        // Sirius (RA=10128, Dec=-1671, Mag=-14600, 440,000 mHz)
        let (coords, trit) = tokenizer.encode_celestial_harmonic(10128, -1671, -14600, 440_000);
        assert_eq!(trit, 0); // ~0.44 -> centered
        assert!(coords[0] >= -1.0 && coords[0] <= 1.0);
        assert!(coords[1] >= -1.0 && coords[1] <= 1.0);
        assert!(coords[2] >= -1.0 && coords[2] <= 1.0);
        assert!(coords[3] >= -1.0 && coords[3] <= 1.0);
        assert!(coords[4] >= 0.0);
    }

    #[test]
    fn test_celestial_coordinates_normalization_roundtrip() {
        let coords = CelestialCoordinates5D {
            ra_cdeg: 18_000,    // 180.00 deg -> 0.5
            dec_cdeg: -4_500,   // -45.00 deg -> -0.5
            mag_pmy: 6_500,     // 6.50 mag -> (6500 - (-2000)) / 17000 = 8500 / 17000 = 0.5
            spectral_class: 4,  // G class (Sun-like) -> 4 / 7 ≈ 0.5714
            milli_hz: 50_000,   // 50 Hz -> 0.5
        };

        let norm = coords.normalize_zero_heap();
        assert!((norm[0] - 0.5).abs() < 1e-4);
        assert!((norm[1] - (-0.5)).abs() < 1e-4);
        assert!((norm[2] - 0.5).abs() < 1e-4);
        assert!((norm[3] - 4.0 / 7.0).abs() < 1e-4);
        assert!((norm[4] - 0.5).abs() < 1e-4);

        let recovered = CelestialCoordinates5D::from_normalized_5d(&norm);
        assert_eq!(recovered.ra_cdeg, coords.ra_cdeg);
        assert_eq!(recovered.dec_cdeg, coords.dec_cdeg);
        assert_eq!(recovered.mag_pmy, coords.mag_pmy);
        assert_eq!(recovered.spectral_class, coords.spectral_class);
        assert_eq!(recovered.milli_hz, coords.milli_hz);
    }

    #[test]
    fn test_up_down_projection_orthonormality_2048() {
        let tokenizer = EmergentSomaticTokenizer::new();
        let input_5d = [0.25f32, -0.75f32, 0.5f32, 0.857f32, 0.123f32];
        let mut embedding = [0.0f32; EmergentSomaticTokenizer::DIM_2B];
        let mut recovered_5d = [0.0f32; 5];

        tokenizer.project_up_2048(&input_5d, &mut embedding);
        tokenizer.project_down_2048(&embedding, &mut recovered_5d);

        for k in 0..5 {
            assert!(
                (recovered_5d[k] - input_5d[k]).abs() < 2e-3,
                "Carrier {} failed roundtrip reconstruction: expected {}, got {}",
                k,
                input_5d[k],
                recovered_5d[k]
            );
        }
    }

    #[test]
    fn test_up_down_projection_orthonormality_3584() {
        let tokenizer = EmergentSomaticTokenizer::new();
        let input_5d = [0.8f32, 0.3f32, 0.1f32, 0.571f32, 0.95f32];
        let mut embedding = [0.0f32; EmergentSomaticTokenizer::DIM_9B];
        let mut recovered_5d = [0.0f32; 5];

        tokenizer.project_up_3584(&input_5d, &mut embedding);
        tokenizer.project_down_3584(&embedding, &mut recovered_5d);

        for k in 0..5 {
            assert!(
                (recovered_5d[k] - input_5d[k]).abs() < 2e-3,
                "Carrier {} failed roundtrip reconstruction: expected {}, got {}",
                k,
                input_5d[k],
                recovered_5d[k]
            );
        }
    }
}


