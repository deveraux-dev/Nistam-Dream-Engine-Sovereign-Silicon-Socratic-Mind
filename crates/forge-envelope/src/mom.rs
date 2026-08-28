//! Mixture of Musicians (MoM) — Real-Time Audio Event Router and Mix Bus.
//!
//! Wires low-latency, lock-free, zero-heap routing of Universal MIDI Packet-sized
//! signals (`UmpWord`) across a pool of physical and cognitive "musicians."

/// Represents a 16-byte Universal MIDI Packet (UMP) word payload.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UmpWord(pub [u8; 16]);

impl UmpWord {
    /// Creates a 16-byte UmpWord from normalized audio envelope metrics and packed trits.
    /// Byte 0: Message Type (0x4 = MIDI 2.0 Channel Voice / Somatic Audio)
    /// Byte 1: Status / Channel
    /// Byte 2..3: RMS energy scaled to 16-bit integer
    /// Byte 4..15: 12 bytes of packed balanced-ternary trits
    #[inline(always)]
    pub fn from_audio_envelope(rms: f32, trits: &[i8]) -> Self {
        let mut bytes = [0u8; 16];
        bytes[0] = 0x40; // UMP Somatic Audio packet header
        let rms_scaled = ((rms.clamp(0.0, 1.0)) * 65535.0) as u16;
        let rms_bytes = rms_scaled.to_be_bytes();
        bytes[2] = rms_bytes[0];
        bytes[3] = rms_bytes[1];

        // Pack trits into bytes 4..15 (up to 60 trits at 5 trits/byte)
        for byte_idx in 0..12 {
            let trit_offset = byte_idx * 5;
            let mut packed_val: u8 = 0;
            let mut mul: u8 = 1;
            for k in 0..5 {
                let trit = if trit_offset + k < trits.len() {
                    trits[trit_offset + k]
                } else {
                    0
                };
                let unsigned_trit = match trit {
                    -1 => 0,
                    0 => 1,
                    1 => 2,
                    _ => 1,
                };
                packed_val += unsigned_trit * mul;
                mul = mul.saturating_mul(3);
            }
            bytes[4 + byte_idx] = packed_val;
        }

        Self(bytes)
    }
}

/// Routing tag mapping to an exact physical or cognitive asset quadrant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RoutingTag {
    /// Munsell or packed colour ID.
    pub colour_id: u8,
    /// Material category identifier.
    pub material_id: u8,
    /// Core semantic essence category ID.
    pub essence_id: u8,
}

/// A 49-slot Mixture of Experts (MoE) router using XOR + POPCNT bit-parallel distance lookups.
pub struct MoeRouter {
    centroid_masks: [[u8; 16]; 49],
}

impl MoeRouter {
    /// Construct the static router.
    pub fn new() -> Self {
        let mut router = Self { centroid_masks: [[0u8; 16]; 49] };
        // Initialize static routing centroids deterministically
        for i in 0..49 {
            router.centroid_masks[i] = [
                i as u8, (i * 2) as u8, (i * 3) as u8, 0xAA,
                0x55, i as u8, 0x00, 0xFF,
                0x11, 0x22, 0x33, 0x44,
                0x55, 0x66, 0x77, (i ^ 0xFF) as u8
            ];
        }
        router
    }

    /// Route a UmpWord to the nearest expert slot index (0..48) based on Hamming distance.
    #[inline(always)]
    pub fn route(&self, word: UmpWord) -> usize {
        let mut min_distance = u32::MAX;
        let mut best_slot = 0;

        for (slot, centroid) in self.centroid_masks.iter().enumerate() {
            let mut distance = 0u32;
            for i in 0..16 {
                let xor_val = word.0[i] ^ centroid[i];
                distance += xor_val.count_ones();
            }
            if distance < min_distance {
                min_distance = distance;
                best_slot = slot;
            }
        }
        best_slot
    }
}

/// Represent an independent, real-time audio generator or modifier.
pub trait Musician {
    /// Retrieve the unique structural ID of the musician.
    fn identity(&self) -> RoutingTag;
    /// Calculate current group delay for Phase Delay Compensation (PDC).
    fn latency_samples(&self) -> usize;
    /// Process a single audio frame frame-by-frame.
    fn process_ump(&mut self, word: UmpWord, sample: f64) -> f64;
}

/// Conductor managing real-time automation weights and slot tracking.
pub struct Conductor {
    active_weights: [f64; 49],
}

impl Conductor {
    /// Create a conductor with balanced unity gain across slots.
    pub fn new() -> Self {
        Self { active_weights: [1.0 / 49.0; 49] }
    }

    /// Update the target weights matrix (intended to be updated wait-free via a TripleBuffer).
    pub fn update_weights(&mut self, weights: [f64; 49]) {
        self.active_weights = weights;
    }

    /// Retrieve the scaling weight of an active slot.
    #[inline(always)]
    pub fn get_weight(&self, slot: usize) -> f64 {
        if slot < 49 {
            self.active_weights[slot]
        } else {
            0.0
        }
    }
}

/// Real-time summing bus with PCG-seeded TPDF dithered i24 fold.
pub struct MomBus {
    accumulator: f64,
    prng_state: u64,
}

impl MomBus {
    /// Create a fresh mix bus.
    pub fn new(seed: u64) -> Self {
        Self {
            accumulator: 0.0,
            prng_state: if seed == 0 { 0x4d32f6e5a7b8c9d0 } else { seed },
        }
    }

    /// Accumulate a single sample to the bus.
    #[inline(always)]
    pub fn accumulate(&mut self, sample: f64) {
        self.accumulator += sample;
    }

    /// Reset the bus accumulator for the next frame block.
    pub fn clear(&mut self) {
        self.accumulator = 0.0;
    }

    /// Fold the 64-bit accumulator into a high-fidelity i24 stream.
    /// Employs a PCG-seeded Triangular Probability Density Function (TPDF) dither
    /// to eliminate quantization distortion and truncation artifacts.
    #[inline(always)]
    pub fn fold_i24_dithered(&mut self) -> i32 {
        let input_scaled = self.accumulator * 8_388_608.0; // 2^23 scale for signed 24-bit

        // Generate two independent flat random noise sources [-0.5, 0.5]
        let r1 = (self.next_u32() as f64 / 4_294_967_296.0) - 0.5;
        let r2 = (self.next_u32() as f64 / 4_294_967_296.0) - 0.5;
        // TPDF dither is the difference between two independent rectangular noise sources
        let dither = r1 - r2;

        let dithered = input_scaled + dither;
        let clamped = dithered.clamp(-8_388_608.0, 8_388_607.0);
        
        clamped.round() as i32
    }

    /// Fast PCG-style random generator.
    #[inline(always)]
    fn next_u32(&mut self) -> u32 {
        let old_state = self.prng_state;
        self.prng_state = old_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let xorshifted = (((old_state >> 18) ^ old_state) >> 27) as u32;
        let rot = (old_state >> 59) as u32;
        (xorshifted >> rot) | (xorshifted << ((!rot).wrapping_add(1) & 31))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moe_routing_determinism() {
        let router = MoeRouter::new();
        let word = UmpWord([0xFF; 16]);
        let slot = router.route(word);
        assert!(slot < 49);

        // Verify routing remains identical for the exact same packet
        let slot_repeat = router.route(word);
        assert_eq!(slot, slot_repeat);
    }

    #[test]
    fn test_mombus_dither_finite() {
        let mut bus = MomBus::new(42);
        bus.accumulate(0.5);
        let i24_val = bus.fold_i24_dithered();
        assert!(i24_val >= -8_388_608 && i24_val <= 8_388_607);
    }

    #[test]
    fn test_ump_from_audio_envelope_and_route() {
        let trits = [1i8, -1, 0, 1, 1, 0, -1, 1, -1, 0];
        let rms = 0.75f32;
        let ump = UmpWord::from_audio_envelope(rms, &trits);

        assert_eq!(ump.0[0], 0x40);
        let rms_val = u16::from_be_bytes([ump.0[2], ump.0[3]]);
        assert_eq!(rms_val, (0.75 * 65535.0) as u16);

        let router = MoeRouter::new();
        let slot = router.route(ump);
        assert!(slot < 49);

        // Parity & determinism check
        let slot_repeat = router.route(ump);
        assert_eq!(slot, slot_repeat);
    }
}
