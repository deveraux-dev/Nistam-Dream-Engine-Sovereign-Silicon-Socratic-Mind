//! Curriculum — a learning path where lessons have prerequisites (a DAG). The
//! available lessons are those not done whose prereqs are all done.

use crate::dag::Dag;
use serde::{Deserialize, Serialize};

/// A course: titled lessons + prerequisite edges + completion state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Course {
    /// Lesson titles indexed by lesson ID.
    pub titles: Vec<String>,
    /// (prereq, lesson) — prereq must be done before lesson.
    edges: Vec<(usize, usize)>,
    done: Vec<bool>,
}

impl Course {
    /// Create a new empty course.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a lesson; returns its id.
    pub fn lesson(&mut self, title: impl Into<String>) -> usize {
        let id = self.titles.len();
        self.titles.push(title.into());
        self.done.push(false);
        id
    }

    /// `lesson` requires `prereq` first.
    pub fn requires(&mut self, lesson: usize, prereq: usize) -> &mut Self {
        if lesson < self.titles.len() && prereq < self.titles.len() {
            self.edges.push((prereq, lesson));
        }
        self
    }

    /// Mark a lesson as completed.
    pub fn complete(&mut self, id: usize) {
        if let Some(d) = self.done.get_mut(id) {
            *d = true;
        }
    }

    fn prereqs_done(&self, lesson: usize) -> bool {
        self.edges.iter().filter(|(_, l)| *l == lesson).all(|(p, _)| self.done[*p])
    }

    /// Lessons open right now: not done, all prereqs done.
    pub fn available(&self) -> Vec<usize> {
        (0..self.titles.len()).filter(|&i| !self.done[i] && self.prereqs_done(i)).collect()
    }

    /// A valid completion order (topo), or None if the prereqs cycle.
    pub fn order(&self) -> Option<Vec<usize>> {
        let mut d = Dag::new(self.titles.len());
        for (p, l) in &self.edges {
            d.depend(*l, *p);
        }
        d.topo()
    }

    /// Completion in permyriad.
    pub fn progress_pmy(&self) -> u32 {
        if self.titles.is_empty() {
            return 10_000;
        }
        let done = self.done.iter().filter(|d| **d).count() as u64;
        ((done * 10_000) / self.titles.len() as u64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn course() -> Course {
        let mut c = Course::new();
        let a = c.lesson("open the cover");
        let b = c.lesson("drop an asset");
        let d = c.lesson("seal a page");
        c.requires(b, a).requires(d, b);
        c
    }

    #[test]
    fn only_unblocked_lessons_are_available() {
        let mut c = course();
        assert_eq!(c.available(), vec![0]); // only the first
        c.complete(0);
        assert_eq!(c.available(), vec![1]);
        c.complete(1);
        assert_eq!(c.available(), vec![2]);
    }

    #[test]
    fn order_is_topological() {
        let order = course().order().unwrap();
        let pos = |x| order.iter().position(|&v| v == x).unwrap();
        assert!(pos(0) < pos(1) && pos(1) < pos(2));
    }

    #[test]
    fn progress_tracks_completion() {
        let mut c = course();
        assert_eq!(c.progress_pmy(), 0);
        c.complete(0);
        c.complete(1);
        c.complete(2);
        assert_eq!(c.progress_pmy(), 10_000);
    }
}
