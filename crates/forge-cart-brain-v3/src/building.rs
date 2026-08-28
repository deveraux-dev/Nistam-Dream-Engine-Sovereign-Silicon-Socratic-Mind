//! Building System — 30 piece types × 5 material tiers.
//!
//! Ghost preview + 2m grid snap, zone-gated (homestead only). Placed buildings
//! live in a fixed-size array — **no runtime alloc on the hot path** — so the
//! placer is safe to drive from the 120Hz tick. Integer mm throughout.
//!
//! Ported by TRANSLATION from the quarry `ironroot-edict` (pure module, no engine
//! edge) — the placement primitive the cart brain owns.

/// 2m grid snap in mm.
pub const GRID_SNAP_MM: i64 = 2000;

/// Maximum placed buildings per zone.
pub const MAX_BUILDINGS: usize = 256;

/// 30 building piece types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BuildingPiece {
    /// Foundation piece.
    Foundation = 0,
    /// Wall piece.
    Wall,
    /// Wall with window piece.
    WallWindow,
    /// Wall with door piece.
    WallDoor,
    /// Half-height wall piece.
    WallHalf,
    /// Floor piece.
    Floor,
    /// Ceiling piece.
    Ceiling,
    /// Roof piece.
    Roof,
    /// Roof corner piece.
    RoofCorner,
    /// Roof peak piece.
    RoofPeak,
    /// Stairs piece.
    Stairs,
    /// Spiral stairs piece.
    StairsSpiral,
    /// Ramp piece.
    Ramp,
    /// Pillar piece.
    Pillar,
    /// Beam piece.
    Beam,
    /// Fence piece.
    Fence,
    /// Fence gate piece.
    FenceGate,
    /// Railing piece.
    Railing,
    /// Platform piece.
    Platform,
    /// Bridge piece.
    Bridge,
    /// Arch piece.
    Arch,
    /// Chimney piece.
    Chimney,
    /// Balcony piece.
    Balcony,
    /// Awning piece.
    Awning,
    /// Ladder piece.
    Ladder,
    /// Trapdoor piece.
    Trapdoor,
    /// Shelf piece.
    Shelf,
    /// Workbench piece.
    Workbench,
    /// Hearth piece.
    Hearth,
    /// Well piece.
    Well,
}

/// 5 material tiers (visual + durability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MaterialTier {
    /// Thatch material tier.
    Thatch = 0,
    /// Wood material tier.
    Wood = 1,
    /// Stone material tier.
    Stone = 2,
    /// Iron material tier.
    Iron = 3,
    /// Obsidian material tier.
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
    /// The piece type of this building.
    pub piece: BuildingPiece,
    /// The material tier of this building.
    pub tier: MaterialTier,
    /// World position snapped to 2m grid (mm).
    pub x_mm: i64,
    /// World position snapped to 2m grid (mm).
    pub y_mm: i64,
    /// World position snapped to 2m grid (mm).
    pub z_mm: i64,
    /// Rotation in 90° increments (0, 1, 2, 3).
    pub rotation: u8,
    /// Frame placed (for undo ordering).
    pub placed_frame: u64,
}

/// Building placement system.
pub struct BuildingPlacer {
    /// Current state of the placer (Idle/Preview/Confirmed).
    pub state: PlacerState,
    /// The currently selected piece type.
    pub selected_piece: BuildingPiece,
    /// The currently selected material tier.
    pub selected_tier: MaterialTier,
    /// Ghost preview position X (grid-snapped, mm).
    pub ghost_x_mm: i64,
    /// Ghost preview position Y (grid-snapped, mm).
    pub ghost_y_mm: i64,
    /// Ghost preview position Z (grid-snapped, mm).
    pub ghost_z_mm: i64,
    /// Ghost preview rotation (0-3).
    pub ghost_rotation: u8,
    /// Placed buildings (fixed array, no alloc).
    pub buildings: [Option<PlacedBuilding>; MAX_BUILDINGS],
    /// Current number of placed buildings.
    pub building_count: usize,
    /// Whether current zone allows building.
    pub zone_allows_building: bool,
}

impl Default for BuildingPlacer {
    /// Create a new default building placer.
    fn default() -> Self { Self::new() }
}

impl BuildingPlacer {
    /// Create a new, empty building placer in Idle state.
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
}
