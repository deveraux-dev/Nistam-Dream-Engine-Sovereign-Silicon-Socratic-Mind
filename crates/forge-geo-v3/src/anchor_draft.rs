//! Editable anchor draft — the user-correction gate between auto-layout and the
//! geodesic bake.
//!
//! `anchor_layout::layout_anchors` produces a *draft*, never a finished rig. The
//! draft stays mutable — move a joint, reset a mistake, add limbs the fixed
//! template can't express (a four-armed figure) — until the user explicitly
//! `commit`s it to the next pipeline stage (`rigging_pipeline` → geodesic weights
//! → GLB).
//!
//! This is the "in case of error" safety net: the auto-layout is morphometric,
//! not omniscient (a robe hides the legs; Goro has four arms), so nothing is
//! hardcoded into the rig until the human has had the wireframe in front of them
//! and advanced it. `commit` IS "moving to the next step"; after it the draft is
//! frozen and edits are refused.
//!
//! Cold authoring path (not the no-alloc hot path) — `Vec`/`String` are fine.
//! Edits clamp to the image FRAME, not the silhouette: when the user is fixing an
//! error the silhouette itself may be what's wrong, so they get the whole frame.

use crate::anchor_layout::layout_anchors;
use crate::rigging_pipeline::{BoneEndpoint, BoneId, SpatialAnchor};

/// A user-placed anchor BEYOND the fixed 20-bone armature — e.g. the 3rd/4th arm
/// on a Goro-style figure. Free-form (own id + label), carried alongside the
/// standard set so multi-limb topology never corrupts the canonical 20 bones.
/// (The fixed armature consumes the standard anchors today; binding extras into
/// an extended armature is the follow-on — they are preserved through `commit`.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraAnchor {
    pub id: u32,
    pub label: String,
    pub pixel_x: u32,
    pub pixel_y: u32,
}

/// Why an edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftError {
    /// The draft is committed (frozen). Adjust before committing.
    Committed,
    /// No standard anchor with that (bone, endpoint).
    NoSuchAnchor,
    /// No extra anchor with that id.
    NoSuchExtra,
}

/// The frozen result of `commit` — what flows to the geodesic rig. `anchors` is
/// the standard 20-bone set for `rigging_pipeline::resolve_anchors`; `extra`
/// carries any multi-limb additions for the (follow-on) extended armature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedRig {
    pub anchors: Vec<SpatialAnchor>,
    pub extra: Vec<ExtraAnchor>,
}

/// An editable wireframe: auto-layout baseline + live working set + extra limbs,
/// frozen only on `commit`.
#[derive(Debug, Clone)]
pub struct AnchorDraft {
    /// Immutable auto-layout result — the reset baseline.
    base: Vec<SpatialAnchor>,
    /// Working set the edits mutate (starts == base). What the wireframe renders.
    work: Vec<SpatialAnchor>,
    /// Free anchors beyond the standard armature (multi-limb overflow).
    extra: Vec<ExtraAnchor>,
    /// Image bounds edits clamp into.
    width: u32,
    height: u32,
    /// Next extra id (monotonic).
    next_extra: u32,
    /// Once committed the draft is frozen — edits return `DraftError::Committed`.
    committed: bool,
}

impl AnchorDraft {
    /// Build a draft from a silhouette mask (runs the morphometric auto-layout).
    pub fn from_mask(mask: &[bool], width: u32, height: u32) -> Self {
        Self::from_anchors(layout_anchors(mask, width, height), width, height)
    }

    /// Build a draft from anchors already laid out. `width`/`height` are the image
    /// bounds edits clamp into.
    pub fn from_anchors(anchors: Vec<SpatialAnchor>, width: u32, height: u32) -> Self {
        Self {
            base: anchors.clone(),
            work: anchors,
            extra: Vec::new(),
            width,
            height,
            next_extra: 0,
            committed: false,
        }
    }

    /// Current standard anchors — render THIS as the wireframe.
    pub fn anchors(&self) -> &[SpatialAnchor] {
        &self.work
    }

    /// Current extra (multi-limb) anchors.
    pub fn extras(&self) -> &[ExtraAnchor] {
        &self.extra
    }

    /// Frame bounds edits clamp into.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// True once `commit` has frozen the draft.
    pub fn is_committed(&self) -> bool {
        self.committed
    }

    fn clamp_xy(&self, x: i64, y: i64) -> (u32, u32) {
        let hx = (self.width as i64 - 1).max(0);
        let hy = (self.height as i64 - 1).max(0);
        (x.clamp(0, hx) as u32, y.clamp(0, hy) as u32)
    }

    fn ensure_open(&self) -> Result<(), DraftError> {
        if self.committed {
            Err(DraftError::Committed)
        } else {
            Ok(())
        }
    }

    /// Move a standard anchor to `(x, y)` (clamped to the frame). The user
    /// override — survives into `commit`.
    pub fn set_anchor(&mut self, bone: BoneId, endpoint: BoneEndpoint, x: i64, y: i64) -> Result<(), DraftError> {
        self.ensure_open()?;
        let (px, py) = self.clamp_xy(x, y);
        let a = self
            .work
            .iter_mut()
            .find(|a| a.bone_id == bone && a.endpoint == endpoint)
            .ok_or(DraftError::NoSuchAnchor)?;
        a.pixel_x = px;
        a.pixel_y = py;
        Ok(())
    }

    /// Nudge a standard anchor by `(dx, dy)` pixels (clamped to the frame).
    pub fn nudge_anchor(&mut self, bone: BoneId, endpoint: BoneEndpoint, dx: i64, dy: i64) -> Result<(), DraftError> {
        self.ensure_open()?;
        let cur = self
            .work
            .iter()
            .find(|a| a.bone_id == bone && a.endpoint == endpoint)
            .ok_or(DraftError::NoSuchAnchor)?;
        let (nx, ny) = (cur.pixel_x as i64 + dx, cur.pixel_y as i64 + dy);
        self.set_anchor(bone, endpoint, nx, ny)
    }

    /// Restore one standard anchor to its auto-layout position.
    pub fn reset_anchor(&mut self, bone: BoneId, endpoint: BoneEndpoint) -> Result<(), DraftError> {
        self.ensure_open()?;
        let base = *self
            .base
            .iter()
            .find(|a| a.bone_id == bone && a.endpoint == endpoint)
            .ok_or(DraftError::NoSuchAnchor)?;
        let a = self
            .work
            .iter_mut()
            .find(|a| a.bone_id == bone && a.endpoint == endpoint)
            .ok_or(DraftError::NoSuchAnchor)?;
        a.pixel_x = base.pixel_x;
        a.pixel_y = base.pixel_y;
        Ok(())
    }

    /// Restore ALL standard anchors to auto-layout (extras are left as-is — they
    /// are deliberate additions; use `remove_extra` to drop them).
    pub fn reset_all(&mut self) -> Result<(), DraftError> {
        self.ensure_open()?;
        self.work = self.base.clone();
        Ok(())
    }

    /// Add an extra anchor (a limb beyond the standard 20). Returns its id.
    pub fn add_extra(&mut self, label: impl Into<String>, x: i64, y: i64) -> Result<u32, DraftError> {
        self.ensure_open()?;
        let (px, py) = self.clamp_xy(x, y);
        let id = self.next_extra;
        self.next_extra += 1;
        self.extra.push(ExtraAnchor { id, label: label.into(), pixel_x: px, pixel_y: py });
        Ok(id)
    }

    /// Move an extra anchor by id (clamped to the frame).
    pub fn move_extra(&mut self, id: u32, x: i64, y: i64) -> Result<(), DraftError> {
        self.ensure_open()?;
        let (px, py) = self.clamp_xy(x, y);
        let e = self.extra.iter_mut().find(|e| e.id == id).ok_or(DraftError::NoSuchExtra)?;
        e.pixel_x = px;
        e.pixel_y = py;
        Ok(())
    }

    /// Remove an extra anchor by id.
    pub fn remove_extra(&mut self, id: u32) -> Result<(), DraftError> {
        self.ensure_open()?;
        let before = self.extra.len();
        self.extra.retain(|e| e.id != id);
        if self.extra.len() == before {
            Err(DraftError::NoSuchExtra)
        } else {
            Ok(())
        }
    }

    /// Freeze the draft and hand the rig to the next pipeline stage. After this,
    /// edits return `DraftError::Committed`. Idempotent. THIS is "moving to the
    /// next step" — nothing is hardcoded into the rig before it.
    pub fn commit(&mut self) -> CommittedRig {
        self.committed = true;
        // The WORKING set (auto-layout + every user edit), never the untouched
        // baseline — the user's corrections are what bakes into the rig.
        CommittedRig { anchors: self.work.clone(), extra: self.extra.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_mask(w: usize, h: usize) -> Vec<bool> {
        vec![true; w * h]
    }

    fn find<'a>(set: &'a [SpatialAnchor], b: BoneId, e: BoneEndpoint) -> &'a SpatialAnchor {
        set.iter().find(|a| a.bone_id == b && a.endpoint == e).unwrap()
    }

    #[test]
    fn user_edit_survives_commit() {
        let (w, h) = (100u32, 200u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        let auto = *find(d.anchors(), BoneId::LeftHand, BoneEndpoint::Tail);
        // the user drags the left hand to a corrected spot
        d.set_anchor(BoneId::LeftHand, BoneEndpoint::Tail, 11, 17).unwrap();
        let rig = d.commit();
        let got = find(&rig.anchors, BoneId::LeftHand, BoneEndpoint::Tail);
        assert_eq!(
            (got.pixel_x, got.pixel_y),
            (11, 17),
            "the user edit must reach the committed rig, not the auto value"
        );
        assert_ne!(
            (got.pixel_x, got.pixel_y),
            (auto.pixel_x, auto.pixel_y),
            "edit must actually differ from auto-layout"
        );
    }

    #[test]
    fn edit_rejected_after_commit() {
        let (w, h) = (80u32, 160u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        let _ = d.commit();
        assert!(d.is_committed());
        let before = *find(d.anchors(), BoneId::Head, BoneEndpoint::Tail);
        assert_eq!(d.set_anchor(BoneId::Head, BoneEndpoint::Tail, 1, 1), Err(DraftError::Committed));
        let after = *find(d.anchors(), BoneId::Head, BoneEndpoint::Tail);
        assert_eq!(
            (before.pixel_x, before.pixel_y),
            (after.pixel_x, after.pixel_y),
            "a frozen draft must not change"
        );
    }

    #[test]
    fn reset_restores_auto() {
        let (w, h) = (90u32, 180u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        let auto = *find(d.anchors(), BoneId::RightHand, BoneEndpoint::Tail);
        d.set_anchor(BoneId::RightHand, BoneEndpoint::Tail, 3, 4).unwrap();
        assert_ne!(find(d.anchors(), BoneId::RightHand, BoneEndpoint::Tail).pixel_x, auto.pixel_x);
        d.reset_anchor(BoneId::RightHand, BoneEndpoint::Tail).unwrap();
        let back = find(d.anchors(), BoneId::RightHand, BoneEndpoint::Tail);
        assert_eq!((back.pixel_x, back.pixel_y), (auto.pixel_x, auto.pixel_y));
    }

    #[test]
    fn extra_anchors_for_multilimb() {
        // Goro: four arms. The standard set stays 40; the extra arms ride alongside.
        let (w, h) = (120u32, 200u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        assert_eq!(d.anchors().len(), 40);
        let mut ids = Vec::new();
        for (i, label) in ["arm3_upper", "arm3_hand", "arm4_upper", "arm4_hand"].iter().enumerate() {
            ids.push(d.add_extra(*label, 10 + i as i64 * 5, 90).unwrap());
        }
        assert_eq!(d.extras().len(), 4);
        assert_eq!(ids, vec![0, 1, 2, 3], "extra ids are unique + monotonic");
        let rig = d.commit();
        assert_eq!(rig.anchors.len(), 40, "standard armature untouched by extras");
        assert_eq!(rig.extra.len(), 4, "extra limbs carried into the rig");
    }

    #[test]
    fn edit_clamps_into_frame() {
        let (w, h) = (50u32, 60u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        d.set_anchor(BoneId::LeftHand, BoneEndpoint::Tail, 9_999, 9_999).unwrap();
        let a = find(d.anchors(), BoneId::LeftHand, BoneEndpoint::Tail);
        assert_eq!((a.pixel_x, a.pixel_y), (w - 1, h - 1), "edit clamps to the frame");
        d.set_anchor(BoneId::LeftHand, BoneEndpoint::Tail, -50, -50).unwrap();
        let a = find(d.anchors(), BoneId::LeftHand, BoneEndpoint::Tail);
        assert_eq!((a.pixel_x, a.pixel_y), (0, 0), "negative clamps to the origin");
    }

    #[test]
    fn move_and_remove_extra() {
        let (w, h) = (100u32, 100u32);
        let mut d = AnchorDraft::from_mask(&full_mask(w as usize, h as usize), w, h);
        let id = d.add_extra("tail", 10, 10).unwrap();
        d.move_extra(id, 42, 43).unwrap();
        assert_eq!((d.extras()[0].pixel_x, d.extras()[0].pixel_y), (42, 43));
        d.remove_extra(id).unwrap();
        assert!(d.extras().is_empty());
        assert_eq!(d.move_extra(id, 1, 1), Err(DraftError::NoSuchExtra));
    }

    #[test]
    fn unknown_anchor_errs() {
        // empty mask → no standard anchors → editing one is NoSuchAnchor
        let mut d = AnchorDraft::from_mask(&vec![false; 16], 4, 4);
        assert!(d.anchors().is_empty());
        assert_eq!(d.set_anchor(BoneId::Root, BoneEndpoint::Head, 1, 1), Err(DraftError::NoSuchAnchor));
    }
}
