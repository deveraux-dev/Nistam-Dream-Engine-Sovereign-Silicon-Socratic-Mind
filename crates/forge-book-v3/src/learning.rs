//! Learning — the tutorial/progression Atlas section. Lessons gate on growth;
//! the manual teaches itself as the reader levels up.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use crate::grow::Growth;
use serde::{Deserialize, Serialize};

/// One lesson — gated by a reader level and (optionally) an unlock tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    /// The lesson title shown to the reader.
    pub title: String,
    /// The lesson body content.
    pub body: String,
    /// Minimum reader level required to access this lesson.
    pub level_req: u32,
    /// Optional unlock tag that gates this lesson; 0 means no tag gate.
    pub unlock_tag: u64,
}

impl Lesson {
    /// Creates a new lesson with title, body, and level requirement; no tag gate by default.
    pub fn new(title: impl Into<String>, body: impl Into<String>, level_req: u32) -> Self {
        Self { title: title.into(), body: body.into(), level_req, unlock_tag: 0 }
    }
    /// Require a sieve tag as well as the level.
    pub fn gated(mut self, tag: u64) -> Self {
        self.unlock_tag = tag;
        self
    }
    /// Is this lesson available to a reader with this growth?
    pub fn available(&self, g: &Growth) -> bool {
        g.reader_level >= self.level_req && (self.unlock_tag == 0 || g.has(self.unlock_tag))
    }
}

/// An ordered track of lessons.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearningTrack {
    /// The ordered sequence of lessons in this track.
    pub lessons: Vec<Lesson>,
}

impl LearningTrack {
    /// Creates a new empty learning track.
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds a lesson to the track and returns its index.
    pub fn add(&mut self, lesson: Lesson) -> usize {
        let i = self.lessons.len();
        self.lessons.push(lesson);
        i
    }
    /// Returns the number of lessons in this track.
    pub fn len(&self) -> usize {
        self.lessons.len()
    }
    /// Returns true if this track contains no lessons.
    pub fn is_empty(&self) -> bool {
        self.lessons.is_empty()
    }

    /// The lessons currently available to `g`, in order.
    pub fn available<'a>(&'a self, g: &'a Growth) -> impl Iterator<Item = &'a Lesson> {
        self.lessons.iter().filter(move |l| l.available(g))
    }

    /// Fraction of the track unlocked, in permyriad (`0..=10000`).
    pub fn progress_pmy(&self, g: &Growth) -> u32 {
        if self.lessons.is_empty() {
            return 10_000;
        }
        let done = self.lessons.iter().filter(|l| l.available(g)).count() as u64;
        ((done * 10_000) / self.lessons.len() as u64) as u32
    }

    /// Bind the available lessons into a Learning chapter for this reader.
    pub fn to_chapter(&self, title: impl Into<String>, g: &Growth) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Learning);
        for l in self.available(g) {
            ch.add_lore(format!("{} — {}", l.title, l.body));
        }
        ch
    }
}

/// The onboarding track for the book itself — grows with the author.
pub fn onboarding() -> LearningTrack {
    let mut t = LearningTrack::new();
    t.add(Lesson::new("Open the cover", "Touch to open. Escape to close.", 0));
    t.add(Lesson::new("Drop an asset", "Drag a picture onto a page; it settles into a permyriad box.", 1));
    t.add(Lesson::new("Seal a page", "Hash-and-hide a page behind a key; only the key reveals it.", 2));
    t.add(Lesson::new("Author the atlas", "Index a capability with a receipt — the honest brag.", 3).gated(0xA71A5));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lessons_gate_on_level() {
        let t = onboarding();
        let mut g = Growth::new();
        assert_eq!(t.available(&g).count(), 1); // only level-0 lesson
        g.advance(2);
        assert_eq!(t.available(&g).count(), 3); // levels 0,1,2
    }

    #[test]
    fn tag_gate_holds_until_unlocked() {
        let t = onboarding();
        let mut g = Growth::new();
        g.advance(9);
        assert_eq!(t.available(&g).count(), 3); // level ok, but tag lesson locked
        g.unlock(0xA71A5);
        assert_eq!(t.available(&g).count(), 4);
    }

    #[test]
    fn progress_is_permyriad() {
        let t = onboarding();
        let g = Growth::new();
        assert_eq!(t.progress_pmy(&g), 2500); // 1 of 4
        assert_eq!(LearningTrack::new().progress_pmy(&g), 10_000); // empty = done
    }

    #[test]
    fn chapter_only_shows_available() {
        let t = onboarding();
        let g = Growth::new();
        let ch = t.to_chapter("First Steps", &g);
        assert_eq!(ch.section, AtlasSection::Learning);
        assert_eq!(ch.lore_count(), 1);
    }
}
