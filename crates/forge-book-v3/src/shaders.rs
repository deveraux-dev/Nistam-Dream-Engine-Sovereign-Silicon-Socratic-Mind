//! Shaders — the shaderbind index Atlas section. Harvested from
//! deveraux_radio.shaderbind: `signal -> surface.channel[N]`, integer permyriad.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// One signal->channel binding row. Range is permyriad (integer) — never float.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShaderChannel {
    /// Name of the signal being bound.
    pub signal: String,
    /// Channel index on the surface.
    pub channel: u8,
    /// Low end of the range in permyriad.
    pub lo_pmy: u32,
    /// High end of the range in permyriad.
    pub hi_pmy: u32,
}

impl ShaderChannel {
    /// A full-range `0..=10000` binding of `signal` to `channel`.
    pub fn new(signal: impl Into<String>, channel: u8) -> Self {
        Self { signal: signal.into(), channel, lo_pmy: 0, hi_pmy: 10_000 }
    }
    /// Restrict the range to `[lo, hi]` permyriad (clamped to `0..=10000`).
    pub fn ranged(mut self, lo: u32, hi: u32) -> Self {
        self.lo_pmy = lo.min(10_000);
        self.hi_pmy = hi.min(10_000);
        self
    }
}

/// One `.shaderbind.vixi` surface — a profiled set of channel bindings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShaderBindEntry {
    /// Name of the surface being bound to.
    pub surface: String,
    /// Profile name associated with this binding.
    pub profile: String,
    /// List of channel bindings for this surface.
    pub channels: Vec<ShaderChannel>,
}

impl ShaderBindEntry {
    /// Create a new shaderbind entry for the given surface and profile.
    pub fn new(surface: impl Into<String>, profile: impl Into<String>) -> Self {
        Self { surface: surface.into(), profile: profile.into(), channels: Vec::new() }
    }
    /// Bind `signal` to the next channel index.
    pub fn bind(&mut self, signal: impl Into<String>) -> &mut Self {
        let ch = self.channels.len() as u8;
        self.channels.push(ShaderChannel::new(signal, ch));
        self
    }
    /// Regenerate the `#vixi:shaderbind` source — round-trips the harvested form.
    pub fn to_vixi(&self) -> String {
        let mut s = String::from("#vixi:shaderbind v1\n");
        s.push_str(&format!("surface: {}\n", self.surface));
        s.push_str(&format!("profile: {}\n", self.profile));
        for c in &self.channels {
            s.push_str(&format!(
                "signal {:<8} source=audio.{:<16} range={}..{}\n",
                c.signal, c.signal, c.lo_pmy, c.hi_pmy
            ));
        }
        for c in &self.channels {
            s.push_str(&format!("{}.channel[{}] <- {}\n", self.surface, c.channel, c.signal));
        }
        s
    }
}

/// The deveraux_radio shaderbind, harvested from disk (rms/beat/spectral).
pub fn deveraux_radio() -> ShaderBindEntry {
    let mut e = ShaderBindEntry::new("deveraux_radio", "seehear");
    e.bind("rms").bind("beat").bind("spectral");
    e
}

/// Bind a set of shaderbinds into a Shaders chapter (one vixi block per surface).
pub fn to_chapter(entries: &[ShaderBindEntry], title: impl Into<String>) -> Chapter {
    let mut ch = Chapter::new(title, AtlasSection::Shaders);
    for e in entries {
        ch.add_lore(e.to_vixi());
    }
    ch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deveraux_binds_three_channels() {
        let e = deveraux_radio();
        assert_eq!(e.channels.len(), 3);
        assert_eq!(e.channels[0].signal, "rms");
        assert_eq!(e.channels[2].channel, 2);
    }

    #[test]
    fn vixi_round_trips_shape() {
        let v = deveraux_radio().to_vixi();
        assert!(v.starts_with("#vixi:shaderbind v1"));
        assert!(v.contains("surface: deveraux_radio"));
        assert!(v.contains("deveraux_radio.channel[0] <- rms"));
        assert!(v.contains("range=0..10000"));
    }

    #[test]
    fn ranged_channel_clamps() {
        let c = ShaderChannel::new("beat", 1).ranged(2000, 99_999);
        assert_eq!(c.lo_pmy, 2000);
        assert_eq!(c.hi_pmy, 10_000);
    }

    #[test]
    fn shaders_chapter_indexes_binds() {
        let ch = to_chapter(&[deveraux_radio()], "Kernels");
        assert_eq!(ch.section, AtlasSection::Shaders);
        assert_eq!(ch.lore_count(), 1);
    }
}
