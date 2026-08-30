//! Sprite — an atlas region (u/v box in permyriad) and a frame clip sampled on
//! an integer tick clock (harvested from forge-pixel).

use serde::{Deserialize, Serialize};

/// A permyriad u/v box into an atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Region {
    /// Horizontal offset in permyriad (0..=10000).
    pub u_pmy: u32,
    /// Vertical offset in permyriad (0..=10000).
    pub v_pmy: u32,
    /// Width in permyriad (0..=10000).
    pub w_pmy: u32,
    /// Height in permyriad (0..=10000).
    pub h_pmy: u32,
}

impl Region {
    /// Construct a region with coordinates clamped to permyriad range [0, 10000].
    pub fn new(u: u32, v: u32, w: u32, h: u32) -> Self {
        Self { u_pmy: u.min(10_000), v_pmy: v.min(10_000), w_pmy: w.min(10_000), h_pmy: h.min(10_000) }
    }
}

/// An animated sprite — an ordered set of frame regions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sprite {
    /// User-assigned name for the sprite.
    pub name: String,
    /// Ordered sequence of animation frame regions.
    pub frames: Vec<Region>,
}

impl Sprite {
    /// Construct a sprite with the given name and empty frame list.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), frames: Vec::new() }
    }
    /// Append a frame region and return self for method chaining.
    pub fn frame(&mut self, region: Region) -> &mut Self {
        self.frames.push(region);
        self
    }
    /// Count of frames in this animation.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
    /// The frame shown at `tick`, advancing one frame every `ticks_per_frame`
    /// ticks and looping. None if there are no frames.
    pub fn frame_at(&self, tick: u32, ticks_per_frame: u32) -> Option<&Region> {
        if self.frames.is_empty() {
            return None;
        }
        let tpf = ticks_per_frame.max(1);
        let idx = (tick / tpf) as usize % self.frames.len();
        self.frames.get(idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite() -> Sprite {
        let mut s = Sprite::new("flame");
        s.frame(Region::new(0, 0, 2500, 2500))
            .frame(Region::new(2500, 0, 2500, 2500))
            .frame(Region::new(5000, 0, 2500, 2500));
        s
    }

    #[test]
    fn frames_advance_and_loop() {
        let s = sprite();
        assert_eq!(s.frame_count(), 3);
        assert_eq!(s.frame_at(0, 4).unwrap().u_pmy, 0);
        assert_eq!(s.frame_at(4, 4).unwrap().u_pmy, 2500); // second frame
        assert_eq!(s.frame_at(12, 4).unwrap().u_pmy, 0); // looped back
    }

    #[test]
    fn empty_sprite_has_no_frame() {
        assert!(Sprite::new("x").frame_at(0, 1).is_none());
    }
}
