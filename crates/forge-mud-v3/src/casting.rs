//! Casting system — spell channels with cast bars that spell themselves.
//!
//! EverQuest-inspired: cast bars are the glyph word itself, speaking one more
//! letter each turn. The word completes → effect lands with a worded line.
//! Interruption (damage/movement) cuts the word mid-glyph and fires nothing.
//!
//! Pure state machine: no game imports, deterministic, L07-bijective.

/// The seven castable glyph words, one per [`crate::hermetics::SEVENFOLD`] row.
/// Word length is the channel duration in turns.
pub const GLYPH_WORDS: [(&str, u8); 7] = [
    ("CLASH", 0),        // Vigor (Mars, Polarity) — flurry (rapid strikes)
    ("SHADOW", 1),       // ShadowWeight (Saturn, Correspondence) — riposte (swift counter)
    ("THOUGHT", 2),      // LogicDepth (Mercury, Mentalism) — haste (speed boost)
    ("CYCLE", 3),        // Momentum (Luna, Rhythm) — regen (healing warmth)
    ("BALANCE", 4),      // Tarnish (Venus, Gender) — clarity (mental focus)
    ("RESONANCE", 5),    // Resonance (Sol, Vibration) — ward (protection shield)
    ("CONSEQUENCE", 6),  // Guilt (Jupiter, CauseEffect) — ruin (decay curse)
];

/// The effect lines — worded outcomes, one per effect index (no numbers).
/// AUTHORED: unique to this game, never EQ text copied.
pub const EFFECT_LINES: [&str; 7] = [
    "the strikes come like a flock of starlings — unstoppable.",
    "your blade moves before the mind can follow — a perfect answer.",
    "the world slows; your limbs obey light itself.",
    "warmth returns to the torn places; you stand whole again.",
    "the noise falls away; the mind pools clear.",
    "a shimmering veil settles between you and the world — the weight lifts.",
    "the ground cries out; decay spreads from where you stand.",
];

/// A channel in flight: the word being cast, glyphs spoken so far, effect index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Channel {
    /// The word being cast (index into [`GLYPH_WORDS`]), or 255 = no channel.
    word_index: u8,
    /// Glyphs spoken so far (0..word_length).
    glyphs_spoken: u8,
}

impl Channel {
    /// No channel active.
    pub const NONE: Self = Self { word_index: 255, glyphs_spoken: 0 };

    /// Start a new channel for the given word index.
    pub fn new(word_index: u8) -> Option<Self> {
        if word_index >= GLYPH_WORDS.len() as u8 {
            return None;
        }
        Some(Self { word_index, glyphs_spoken: 0 })
    }

    /// Is this channel active?
    #[inline]
    pub fn is_active(&self) -> bool {
        self.word_index < 255
    }

    /// The word this channel is casting.
    #[inline]
    pub fn word(&self) -> Option<&'static str> {
        if self.is_active() {
            Some(GLYPH_WORDS[self.word_index as usize].0)
        } else {
            None
        }
    }

    /// The effect index (0-6) that will fire on completion.
    #[inline]
    pub fn effect_index(&self) -> Option<u8> {
        if self.is_active() {
            Some(GLYPH_WORDS[self.word_index as usize].1)
        } else {
            None
        }
    }

    /// How many glyphs have been spoken?
    #[inline]
    pub fn spoken(&self) -> usize {
        self.glyphs_spoken as usize
    }

    /// How many glyphs remain (including the current one)?
    #[inline]
    pub fn remaining(&self) -> usize {
        self.word()
            .map(|w| w.len().saturating_sub(self.glyphs_spoken as usize))
            .unwrap_or(0)
    }

    /// Advance by one glyph. Returns the glyph spoken (as a char).
    /// Returns `None` if channel is complete or not active.
    pub fn advance(&mut self) -> Option<char> {
        let w = self.word()?;
        if self.glyphs_spoken >= w.len() as u8 {
            return None; // Already complete
        }
        let glyph = w.chars().nth(self.glyphs_spoken as usize)?;
        self.glyphs_spoken += 1;
        Some(glyph)
    }

    /// Is this channel complete (all glyphs spoken)?
    #[inline]
    pub fn is_complete(&self) -> bool {
        self.word()
            .map(|w| self.glyphs_spoken as usize >= w.len())
            .unwrap_or(false)
    }

    /// Interrupt the channel and return the spoken prefix (e.g. "CLA—" if cut).
    /// Returns `None` if no channel is active.
    pub fn interrupt(&mut self) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        let w = self.word()?;
        let prefix = w.chars().take(self.glyphs_spoken as usize).collect::<String>();
        self.word_index = 255;
        self.glyphs_spoken = 0;
        if prefix.is_empty() {
            Some(String::from("—"))
        } else {
            Some(format!("{}—", prefix))
        }
    }

    /// Encode as 2 bytes (word_index, glyphs_spoken) for L07 bijection test.
    pub fn encode(&self) -> [u8; 2] {
        [self.word_index, self.glyphs_spoken]
    }

    /// Decode from 2 bytes, with validation (L07 bijection).
    pub fn decode(bytes: [u8; 2]) -> Option<Self> {
        let [word_index, glyphs_spoken] = bytes;
        let ch = Self { word_index, glyphs_spoken };
        // Validate: if word_index is valid (< 7), glyphs_spoken must not exceed word length.
        if word_index < 255 {
            if word_index as usize >= GLYPH_WORDS.len() {
                return None;
            }
            let word_len = GLYPH_WORDS[word_index as usize].0.len() as u8;
            if glyphs_spoken > word_len {
                return None;
            }
        } else if word_index == 255 {
            // NONE state: glyphs_spoken must be 0
            if glyphs_spoken != 0 {
                return None;
            }
        } else {
            return None; // Invalid word_index
        }
        Some(ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_words_are_seven_and_have_effect_indices() {
        assert_eq!(GLYPH_WORDS.len(), 7);
        for (i, (word, effect)) in GLYPH_WORDS.iter().enumerate() {
            assert!(!word.is_empty(), "word {} is empty", i);
            assert!(!word.chars().any(|c| c.is_ascii_digit()), "word {} has digits", i);
            assert!(*effect < 7, "effect {} is out of range", effect);
        }
        assert_eq!(EFFECT_LINES.len(), 7);
    }

    #[test]
    fn channel_new_and_active() {
        let ch = Channel::new(0).unwrap();
        assert!(ch.is_active());
        assert_eq!(ch.word(), Some("CLASH"));
        assert_eq!(ch.effect_index(), Some(0));
        assert_eq!(ch.spoken(), 0);
        assert_eq!(ch.remaining(), 5);
        assert!(!ch.is_complete());

        let none = Channel::NONE;
        assert!(!none.is_active());
        assert_eq!(none.word(), None);
    }

    #[test]
    fn channel_advance_one_glyph_per_turn() {
        let mut ch = Channel::new(0).unwrap(); // "CLASH"
        assert_eq!(ch.advance(), Some('C'));
        assert_eq!(ch.spoken(), 1);
        assert_eq!(ch.remaining(), 4);
        assert!(!ch.is_complete());

        assert_eq!(ch.advance(), Some('L'));
        assert_eq!(ch.advance(), Some('A'));
        assert_eq!(ch.advance(), Some('S'));
        assert_eq!(ch.advance(), Some('H'));
        assert_eq!(ch.spoken(), 5);
        assert_eq!(ch.remaining(), 0);
        assert!(ch.is_complete());

        // Once complete, advance returns None
        assert_eq!(ch.advance(), None);
    }

    #[test]
    fn channel_interrupt_mid_word() {
        let mut ch = Channel::new(0).unwrap(); // "CLASH"
        ch.advance(); // C
        ch.advance(); // L
        ch.advance(); // A

        let broken = ch.interrupt().unwrap();
        assert_eq!(broken, "CLA—");
        assert!(!ch.is_active());
        assert_eq!(ch.word(), None);
    }

    #[test]
    fn channel_interrupt_at_start() {
        let mut ch = Channel::new(0).unwrap();
        let broken = ch.interrupt().unwrap();
        assert_eq!(broken, "—");
    }

    #[test]
    fn channel_interrupt_no_active_channel() {
        let mut ch = Channel::NONE;
        assert_eq!(ch.interrupt(), None);
    }

    #[test]
    fn l07_encode_decode_bijection() {
        for word_idx in 0..7u8 {
            for glyphs in 0..=GLYPH_WORDS[word_idx as usize].0.len() as u8 {
                let ch = Channel {
                    word_index: word_idx,
                    glyphs_spoken: glyphs,
                };
                let encoded = ch.encode();
                let decoded = Channel::decode(encoded).expect("decode failed");
                assert_eq!(ch, decoded, "bijection failed for word {} glyphs {}", word_idx, glyphs);
            }
        }
    }

    #[test]
    fn l07_encode_decode_none_state() {
        let ch = Channel::NONE;
        let encoded = ch.encode();
        let decoded = Channel::decode(encoded).expect("decode failed");
        assert_eq!(ch, decoded);
    }

    #[test]
    fn l07_decode_invalid_word_index() {
        let invalid = Channel::decode([8, 0]);
        assert!(invalid.is_none(), "word_index 8 should be invalid");
    }

    #[test]
    fn l07_decode_glyphs_exceed_word_length() {
        let clash_len = GLYPH_WORDS[0].0.len() as u8;
        let invalid = Channel::decode([0, clash_len + 1]);
        assert!(invalid.is_none(), "glyphs_spoken exceeding word length should be invalid");
    }

    #[test]
    fn l07_decode_none_with_nonzero_glyphs() {
        let invalid = Channel::decode([255, 1]);
        assert!(invalid.is_none(), "NONE with glyphs_spoken != 0 should be invalid");
    }

    #[test]
    fn all_seven_words_cast_to_completion() {
        for (idx, (word, effect_idx)) in GLYPH_WORDS.iter().enumerate() {
            let mut ch = Channel::new(idx as u8).unwrap();
            let mut glyphs = String::new();
            while let Some(g) = ch.advance() {
                glyphs.push(g);
            }
            assert_eq!(glyphs, *word, "word {} mismatch", idx);
            assert!(ch.is_complete());
            assert_eq!(ch.effect_index(), Some(*effect_idx));
        }
    }

    #[test]
    fn effect_lines_correspond_to_effect_indices() {
        assert_eq!(EFFECT_LINES.len(), 7);
        for (idx, line) in EFFECT_LINES.iter().enumerate() {
            assert!(!line.is_empty(), "effect line {} is empty", idx);
            assert!(!line.chars().any(|c| c.is_ascii_digit()), "effect line {} has digits", idx);
        }
    }

    #[test]
    fn casting_clash_word_takes_five_turns() {
        let mut ch = Channel::new(0).unwrap(); // CLASH
        assert_eq!(ch.word(), Some("CLASH"));
        assert_eq!(ch.effect_index(), Some(0)); // flurry

        for turn in 1..=5 {
            assert!(!ch.is_complete());
            let glyph = ch.advance().expect(&format!("turn {} failed", turn));
            let expected_glyph = "CLASH".chars().nth(turn - 1).unwrap();
            assert_eq!(glyph, expected_glyph);
            if turn == 5 {
                assert!(ch.is_complete(), "should be complete after 5 glyphs");
            }
        }

        // Once complete, advance returns None
        assert_eq!(ch.advance(), None);
    }

    #[test]
    fn casting_resonance_word_takes_nine_turns() {
        let mut ch = Channel::new(5).unwrap(); // RESONANCE (9 letters)
        assert_eq!(ch.word(), Some("RESONANCE"));
        assert_eq!(ch.effect_index(), Some(5)); // ward

        for turn in 1..=9 {
            assert!(!ch.is_complete());
            ch.advance().expect(&format!("turn {} failed", turn));
            if turn == 9 {
                assert!(ch.is_complete(), "should be complete after 9 glyphs");
            }
        }
    }
}
