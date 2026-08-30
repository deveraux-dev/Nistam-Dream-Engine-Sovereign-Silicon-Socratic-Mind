//! Small deterministic hash surface for authority-ticket IDs.
//!
//! This is intentionally stable, simple, and dependency-free. Replace with the
//! repo's canonical BrutalHash implementation if one already exists.

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BrutalHash(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BrutalHashInput {
    pub kind: u16,
    pub world: u64,
    pub actor: u64,
    pub subject: u64,
    pub source_tick: u64,
    pub payload_hash: u64,
    pub schema: u16,
}

#[inline]
pub const fn brutal_hash64(words: &[u64]) -> BrutalHash {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0usize;
    while i < words.len() {
        hash ^= words[i];
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    BrutalHash(hash)
}

impl BrutalHashInput {
    #[inline]
    pub const fn deterministic_hash(self) -> BrutalHash {
        brutal_hash64(&[
            self.kind as u64,
            self.world,
            self.actor,
            self.subject,
            self.source_tick,
            self.payload_hash,
            self.schema as u64,
        ])
    }
}
