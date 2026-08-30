//! Mesh — a triangle mesh: vertex count + triangle faces, with an Euler
//! characteristic check for closedness (harvested from forge-geo).

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A triangle mesh by vertex count and face index-triples.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mesh {
    /// Vertex count in this mesh.
    pub verts: usize,
    /// Triangle faces as [a, b, c] vertex index triples.
    pub tris: Vec<[usize; 3]>,
}

impl Mesh {
    /// Creates a new mesh with a given vertex count.
    pub fn new(verts: usize) -> Self {
        Self { verts, tris: Vec::new() }
    }

    /// Add a triangle by vertex indices.
    pub fn tri(&mut self, a: usize, b: usize, c: usize) -> &mut Self {
        self.tris.push([a, b, c]);
        self
    }

    /// Unique undirected edge count.
    pub fn edges(&self) -> usize {
        let mut set = BTreeSet::new();
        for t in &self.tris {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                set.insert((a.min(b), a.max(b)));
            }
        }
        set.len()
    }

    /// Euler characteristic V - E + F.
    pub fn euler(&self) -> i64 {
        self.verts as i64 - self.edges() as i64 + self.tris.len() as i64
    }

    /// A closed sphere-topology mesh has Euler characteristic 2.
    pub fn is_closed(&self) -> bool {
        self.euler() == 2
    }

    /// Returns the number of triangle faces in this mesh.
    pub fn face_count(&self) -> usize {
        self.tris.len()
    }
}

/// A tetrahedron — the smallest closed mesh (4 verts, 4 faces).
pub fn tetrahedron() -> Mesh {
    let mut m = Mesh::new(4);
    m.tri(0, 1, 2).tri(0, 1, 3).tri(0, 2, 3).tri(1, 2, 3);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tetrahedron_is_closed() {
        let m = tetrahedron();
        assert_eq!(m.face_count(), 4);
        assert_eq!(m.edges(), 6);
        assert_eq!(m.euler(), 2);
        assert!(m.is_closed());
    }

    #[test]
    fn open_strip_is_not_closed() {
        let mut m = Mesh::new(4);
        m.tri(0, 1, 2).tri(1, 2, 3);
        assert!(!m.is_closed());
    }
}
