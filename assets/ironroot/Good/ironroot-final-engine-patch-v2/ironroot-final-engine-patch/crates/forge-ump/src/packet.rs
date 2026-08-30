//! POD-safe packet and lightweight wrapper types.

/// A Universal MIDI Packet normalized to four 32-bit big-endian-decoded words.
///
/// Shorter packets keep unused trailing words at zero. The struct is exactly
/// 16 bytes so it can be shared with sieve/GPU-style lanes without heap data.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Ump {
    pub words: [u32; 4],
}

impl Ump {
    #[inline]
    pub const fn new(words: [u32; 4]) -> Self {
        Self { words }
    }

    #[inline]
    pub const fn mt(self) -> u8 {
        ((self.words[0] >> 28) & 0x0f) as u8
    }

    #[inline]
    pub const fn group(self) -> Group {
        Group(((self.words[0] >> 24) & 0x0f) as u8)
    }

    #[inline]
    pub const fn status(self) -> u8 {
        ((self.words[0] >> 20) & 0x0f) as u8
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Group(pub u8);

#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Channel(pub u8);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Stamped<T> {
    pub universal_tick_us: i64,
    pub payload: T,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ump_is_pod_safe() {
        fn assert_pod<T: bytemuck::Pod>() {}
        fn assert_zeroable<T: bytemuck::Zeroable>() {}
        assert_pod::<Ump>();
        assert_zeroable::<Ump>();
    }

    #[test]
    fn ump_total_size_is_16_bytes() {
        assert_eq!(core::mem::size_of::<Ump>(), 16);
        assert_eq!(core::mem::align_of::<Ump>(), 4);
    }
}
