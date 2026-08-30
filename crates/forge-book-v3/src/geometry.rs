//! Geometry — the rig section: a bone hierarchy (the 20-bone Mobometric rig,
//! harvested from forge-geo). Integer bone ids; parent links form the tree.

use crate::atlas::AtlasSection;
use crate::chapter::Chapter;
use serde::{Deserialize, Serialize};

/// One bone: an id, a name, and an optional parent id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bone {
    /// Unique identifier for this bone (its index in the rig).
    pub id: u8,
    /// Name of the bone.
    pub name: String,
    /// Index of the parent bone, or None if this is the root.
    pub parent: Option<u8>,
}

/// A skeletal rig.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rig {
    /// All bones in the rig, indexed by id.
    pub bones: Vec<Bone>,
}

impl Rig {
    /// Create an empty rig.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add a bone; returns its id (its index).
    pub fn add(&mut self, name: impl Into<String>, parent: Option<u8>) -> u8 {
        let id = self.bones.len() as u8;
        self.bones.push(Bone { id, name: name.into(), parent });
        id
    }
    /// Return the number of bones in the rig.
    pub fn len(&self) -> usize {
        self.bones.len()
    }
    /// Return true if the rig has no bones.
    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }
    /// Return the root bone (the one with no parent), if it exists.
    pub fn root(&self) -> Option<&Bone> {
        self.bones.iter().find(|b| b.parent.is_none())
    }
    /// Return an iterator over all bones whose parent is the given id.
    pub fn children(&self, id: u8) -> impl Iterator<Item = &Bone> {
        self.bones.iter().filter(move |b| b.parent == Some(id))
    }
    /// Depth of a bone from the root (root = 0).
    pub fn depth(&self, id: u8) -> u32 {
        let mut d = 0;
        let mut cur = self.bones.get(id as usize).and_then(|b| b.parent);
        while let Some(p) = cur {
            d += 1;
            cur = self.bones.get(p as usize).and_then(|b| b.parent);
            if d > 255 {
                break; // cycle guard
            }
        }
        d
    }
    /// Convert the rig to a Chapter lore page listing each bone and its depth.
    pub fn to_chapter(&self, title: impl Into<String>) -> Chapter {
        let mut ch = Chapter::new(title, AtlasSection::Custom("Geometry".into()));
        for b in &self.bones {
            ch.add_lore(format!("{}: {} (depth {})", b.id, b.name, self.depth(b.id)));
        }
        ch
    }
}

/// A compact Mobometric-style rig (root + spine + limbs), 20 bones.
pub fn mobometric_rig() -> Rig {
    let mut r = Rig::new();
    let root = r.add("root", None);
    let pelvis = r.add("pelvis", Some(root));
    let spine = r.add("spine", Some(pelvis));
    let chest = r.add("chest", Some(spine));
    let neck = r.add("neck", Some(chest));
    r.add("head", Some(neck));
    for side in ["l", "r"] {
        let shoulder = r.add(format!("{side}_shoulder"), Some(chest));
        let elbow = r.add(format!("{side}_elbow"), Some(shoulder));
        r.add(format!("{side}_hand"), Some(elbow));
        let hip = r.add(format!("{side}_hip"), Some(pelvis));
        let knee = r.add(format!("{side}_knee"), Some(hip));
        r.add(format!("{side}_foot"), Some(knee));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rig_has_a_single_root() {
        let r = mobometric_rig();
        assert_eq!(r.root().unwrap().name, "root");
        assert_eq!(r.bones.iter().filter(|b| b.parent.is_none()).count(), 1);
    }

    #[test]
    fn depth_counts_from_root() {
        let r = mobometric_rig();
        assert_eq!(r.depth(0), 0); // root
        // head is root->pelvis->spine->chest->neck->head = depth 5
        let head = r.bones.iter().find(|b| b.name == "head").unwrap();
        assert_eq!(r.depth(head.id), 5);
    }

    #[test]
    fn children_link_correctly() {
        let r = mobometric_rig();
        let chest = r.bones.iter().find(|b| b.name == "chest").unwrap();
        // neck + two shoulders
        assert_eq!(r.children(chest.id).count(), 3);
    }
}
