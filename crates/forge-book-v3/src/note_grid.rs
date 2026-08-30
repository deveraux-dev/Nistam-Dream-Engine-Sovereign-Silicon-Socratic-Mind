//! Note-grid — a step sequencer: rows are pitches, columns are steps, cells are
//! on/off. Integer; the MIDI authoring page's grid.

use serde::{Deserialize, Serialize};

/// A pitch-by-step on/off grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteGrid {
    /// MIDI pitch values (one per row).
    pub pitches: Vec<u8>,
    /// Number of steps (columns) in the grid.
    pub steps: usize,
    cells: Vec<bool>,
}

impl NoteGrid {
    /// Create a new step sequencer grid with the given pitches and number of steps.
    pub fn new(pitches: Vec<u8>, steps: usize) -> Self {
        let n = pitches.len() * steps;
        Self { pitches, steps, cells: vec![false; n] }
    }

    fn idx(&self, row: usize, step: usize) -> Option<usize> {
        (row < self.pitches.len() && step < self.steps).then_some(row * self.steps + step)
    }

    /// Toggle the cell at the given row and step; out-of-range coordinates are ignored.
    pub fn toggle(&mut self, row: usize, step: usize) {
        if let Some(i) = self.idx(row, step) {
            self.cells[i] = !self.cells[i];
        }
    }

    /// Check if the cell at the given row and step is on; returns false for out-of-range.
    pub fn is_on(&self, row: usize, step: usize) -> bool {
        self.idx(row, step).map(|i| self.cells[i]).unwrap_or(false)
    }

    /// The pitches sounding at `step`.
    pub fn active_at(&self, step: usize) -> Vec<u8> {
        (0..self.pitches.len()).filter(|&r| self.is_on(r, step)).map(|r| self.pitches[r]).collect()
    }

    /// Return the number of cells that are currently on.
    pub fn count(&self) -> usize {
        self.cells.iter().filter(|c| **c).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggles_and_reads_columns() {
        let mut g = NoteGrid::new(vec![60, 64, 67], 8);
        g.toggle(0, 0);
        g.toggle(2, 0);
        g.toggle(1, 4);
        assert_eq!(g.active_at(0), vec![60, 67]);
        assert_eq!(g.active_at(4), vec![64]);
        assert_eq!(g.active_at(7), Vec::<u8>::new());
        assert_eq!(g.count(), 3);
    }

    #[test]
    fn out_of_range_is_noop() {
        let mut g = NoteGrid::new(vec![60], 4);
        g.toggle(9, 9);
        assert_eq!(g.count(), 0);
    }
}
