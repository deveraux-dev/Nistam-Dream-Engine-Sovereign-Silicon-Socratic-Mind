//! Building System — 30 piece types × 5 material tiers.
//!
//! Ghost preview + 2m grid snap, zone-gated (homestead only). Placed buildings
//! live in a fixed-size array — **no runtime alloc on the hot path** — so the
//! placer is safe to drive from the 120Hz tick. Integer mm throughout.
//!
//! Ported by TRANSLATION from the quarry `ironroot-edict` (pure module, no engine
//! edge) — the placement primitive the brain owns.

/// 2m grid snap in mm.
pub const GRID_SNAP_MM: i64 = 2000;

/// Maximum placed buildings per zone.
pub const MAX_BUILDINGS: usize = 256;

/// 30 building piece types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuildingPiece {
    /// Structural foundation base.
    Foundation = 0,
    /// Solid vertical wall.
    Wall,
    /// Wall with window opening.
    WallWindow,
    /// Wall with door opening.
    WallDoor,
    /// Half-height wall.
    WallHalf,
    /// Horizontal floor or ceiling panel.
    Floor,
    /// Interior overhead cover.
    Ceiling,
    /// Sloped roof panel.
    Roof,
    /// Corner roof piece.
    RoofCorner,
    /// Peaked roof summit.
    RoofPeak,
    /// Straight staircase.
    Stairs,
    /// Spiral staircase.
    StairsSpiral,
    /// Sloped ramp.
    Ramp,
    /// Vertical support column.
    Pillar,
    /// Horizontal structural beam.
    Beam,
    /// Fencing panel.
    Fence,
    /// Hinged fence gate.
    FenceGate,
    /// Safety rail or barrier.
    Railing,
    /// Elevated platform.
    Platform,
    /// Spanning bridge structure.
    Bridge,
    /// Curved arch element.
    Arch,
    /// Tall chimney stack.
    Chimney,
    /// Protruding balcony.
    Balcony,
    /// Roof overhang or marquee.
    Awning,
    /// Vertical climbing ladder.
    Ladder,
    /// Hinged ceiling hatch.
    Trapdoor,
    /// Wall-mounted storage shelf.
    Shelf,
    /// Crafting workbench.
    Workbench,
    /// Indoor fireplace hearth.
    Hearth,
    /// Water delivery well.
    Well,
}

/// 5 material tiers (visual + durability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MaterialTier {
    /// Fragile woven plant material.
    Thatch = 0,
    /// Sturdy timber construction.
    Wood = 1,
    /// Durable quarried rock.
    Stone = 2,
    /// Forged metal framework.
    Iron = 3,
    /// Obsidian volcanic hardness.
    Obsidian = 4,
}

/// Placer state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacerState {
    /// No building selected.
    Idle,
    /// Ghost preview visible, following cursor with grid snap.
    Preview,
    /// Placement confirmed (transitions back to Preview for chain-building).
    Confirmed,
}

/// A placed building in the world.
#[derive(Debug, Clone, Copy)]
pub struct PlacedBuilding {
    /// Building piece type.
    pub piece: BuildingPiece,
    /// Material tier.
    pub tier: MaterialTier,
    /// World position snapped to 2m grid (mm).
    pub x_mm: i64,
    /// World Y coordinate in mm.
    pub y_mm: i64,
    /// World Z coordinate in mm.
    pub z_mm: i64,
    /// Rotation in 90° increments (0, 1, 2, 3).
    pub rotation: u8,
    /// Frame placed (for undo ordering).
    pub placed_frame: u64,
}

/// Building placement system.
pub struct BuildingPlacer {
    /// Current placement state machine.
    pub state: PlacerState,
    /// Currently selected piece type.
    pub selected_piece: BuildingPiece,
    /// Currently selected material tier.
    pub selected_tier: MaterialTier,
    /// Ghost preview position (grid-snapped).
    pub ghost_x_mm: i64,
    /// Ghost preview Y coordinate in mm.
    pub ghost_y_mm: i64,
    /// Ghost preview Z coordinate in mm.
    pub ghost_z_mm: i64,
    /// Ghost preview rotation (0-3).
    pub ghost_rotation: u8,
    /// Placed buildings (fixed array, no alloc).
    pub buildings: [Option<PlacedBuilding>; MAX_BUILDINGS],
    /// Count of currently placed buildings.
    pub building_count: usize,
    /// Whether current zone allows building.
    pub zone_allows_building: bool,
}

impl Default for BuildingPlacer {
    fn default() -> Self { Self::new() }
}

impl BuildingPlacer {
    /// Create a new building placer in idle state.
    pub fn new() -> Self {
        Self {
            state: PlacerState::Idle,
            selected_piece: BuildingPiece::Foundation,
            selected_tier: MaterialTier::Wood,
            ghost_x_mm: 0,
            ghost_y_mm: 0,
            ghost_z_mm: 0,
            ghost_rotation: 0,
            buildings: [None; MAX_BUILDINGS],
            building_count: 0,
            zone_allows_building: false,
        }
    }

    /// Enter build mode with a selected piece.
    pub fn select(&mut self, piece: BuildingPiece, tier: MaterialTier) {
        if !self.zone_allows_building {
            return;
        }
        self.selected_piece = piece;
        self.selected_tier = tier;
        self.state = PlacerState::Preview;
    }

    /// Cancel build mode.
    pub fn cancel(&mut self) {
        self.state = PlacerState::Idle;
    }

    /// Update ghost position from player cursor (snaps to 2m grid).
    pub fn update_ghost(&mut self, cursor_x_mm: i64, cursor_y_mm: i64, cursor_z_mm: i64) {
        self.ghost_x_mm = snap_to_grid(cursor_x_mm);
        self.ghost_y_mm = snap_to_grid(cursor_y_mm);
        self.ghost_z_mm = snap_to_grid(cursor_z_mm);
    }

    /// Rotate ghost 90° clockwise.
    pub fn rotate(&mut self) {
        self.ghost_rotation = (self.ghost_rotation + 1) % 4;
    }

    /// Confirm placement. Returns true if successful.
    pub fn confirm(&mut self, frame: u64) -> bool {
        if self.state != PlacerState::Preview || !self.zone_allows_building {
            return false;
        }
        if self.building_count >= MAX_BUILDINGS {
            return false;
        }
        // Check for overlap (same grid cell)
        let occupied = self.buildings[..self.building_count].iter().any(|b| {
            if let Some(b) = b {
                b.x_mm == self.ghost_x_mm && b.y_mm == self.ghost_y_mm && b.z_mm == self.ghost_z_mm
            } else {
                false
            }
        });
        if occupied {
            return false;
        }

        self.buildings[self.building_count] = Some(PlacedBuilding {
            piece: self.selected_piece,
            tier: self.selected_tier,
            x_mm: self.ghost_x_mm,
            y_mm: self.ghost_y_mm,
            z_mm: self.ghost_z_mm,
            rotation: self.ghost_rotation,
            placed_frame: frame,
        });
        self.building_count += 1;
        self.state = PlacerState::Confirmed;
        // Auto-return to preview for chain-building
        self.state = PlacerState::Preview;
        true
    }

    /// Remove the most recently placed building (undo).
    pub fn undo(&mut self) -> bool {
        if self.building_count == 0 {
            return false;
        }
        self.building_count -= 1;
        self.buildings[self.building_count] = None;
        true
    }
}

/// Snap a world coordinate to the 2m grid (round to nearest).
#[inline]
pub fn snap_to_grid(mm: i64) -> i64 {
    if mm >= 0 {
        ((mm + GRID_SNAP_MM / 2) / GRID_SNAP_MM) * GRID_SNAP_MM
    } else {
        ((mm - GRID_SNAP_MM / 2 + 1) / GRID_SNAP_MM) * GRID_SNAP_MM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_snap() {
        assert_eq!(snap_to_grid(0), 0);
        assert_eq!(snap_to_grid(999), 0);
        assert_eq!(snap_to_grid(1000), 2000);
        assert_eq!(snap_to_grid(2999), 2000);
        assert_eq!(snap_to_grid(3000), 4000);
        assert_eq!(snap_to_grid(-1000), 0);
        assert_eq!(snap_to_grid(-1001), -2000);
    }

    #[test]
    fn place_and_undo() {
        let mut bp = BuildingPlacer::new();
        bp.zone_allows_building = true;
        bp.select(BuildingPiece::Wall, MaterialTier::Stone);
        bp.update_ghost(1500, 0, 3500);
        assert!(bp.confirm(1));
        assert_eq!(bp.building_count, 1);
        assert!(bp.undo());
        assert_eq!(bp.building_count, 0);
    }

    #[test]
    fn zone_gate_blocks_placement() {
        let mut bp = BuildingPlacer::new();
        bp.zone_allows_building = false;
        bp.select(BuildingPiece::Foundation, MaterialTier::Wood);
        assert_eq!(bp.state, PlacerState::Idle); // select refused
    }

    #[test]
    fn no_overlap() {
        let mut bp = BuildingPlacer::new();
        bp.zone_allows_building = true;
        bp.select(BuildingPiece::Floor, MaterialTier::Iron);
        bp.update_ghost(2000, 0, 2000);
        assert!(bp.confirm(1));
        // Same spot again
        bp.update_ghost(2000, 0, 2000);
        assert!(!bp.confirm(2)); // blocked
    }

    #[test]
    fn max_buildings_cap() {
        let mut bp = BuildingPlacer::new();
        bp.zone_allows_building = true;
        bp.select(BuildingPiece::Pillar, MaterialTier::Obsidian);
        for i in 0..MAX_BUILDINGS {
            bp.update_ghost(i as i64 * GRID_SNAP_MM, 0, 0);
            assert!(bp.confirm(i as u64));
        }
        // 257th fails
        bp.update_ghost(MAX_BUILDINGS as i64 * GRID_SNAP_MM, 0, 0);
        assert!(!bp.confirm(999));
    }

    // L18 sabotage test: flip the overlap check, confirm placement gate breaks.
    #[test]
    fn sabotage_overlap_invariant() {
        let mut bp = BuildingPlacer::new();
        bp.zone_allows_building = true;
        bp.select(BuildingPiece::Wall, MaterialTier::Wood);
        bp.update_ghost(4000, 0, 4000);
        assert!(bp.confirm(1), "first placement should succeed");

        // Place another building at the same snapped location.
        bp.select(BuildingPiece::Floor, MaterialTier::Stone);
        bp.update_ghost(4000, 0, 4000);
        // Overlap detection must reject this: same grid cell, same frame.
        let second = bp.confirm(2);
        assert!(!second, "sabotage gate: second building at same spot must fail");
        assert_eq!(bp.building_count, 1, "building_count must remain 1 after rejected placement");
    }
}
