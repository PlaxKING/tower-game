//! Wave Function Collapse floor layout generator.
//!
//! Generates room layouts from a floor hash using constrained tile placement.
//! Each tile has adjacency rules derived from semantic tags.

use serde::{Deserialize, Serialize};

use super::{FloorSpec, FloorTier};
use crate::semantic::SemanticTags;

/// Grid dimensions for WFC solver (used when full WFC is enabled)
pub const GRID_WIDTH: usize = 16;
pub const GRID_HEIGHT: usize = 16;

/// Tile types that can appear in a floor layout
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TileType {
    Empty,
    Floor,
    Wall,
    Door,
    StairsUp,
    StairsDown,
    Chest,
    Trap,
    Spawner,
    Shrine,     // faction shrine
    WindColumn, // aerial updraft
    VoidPit,    // void fall area
}

impl TileType {
    /// All placeable tile types (excluding Empty)
    pub fn all_placeable() -> &'static [TileType] {
        &[
            TileType::Floor,
            TileType::Wall,
            TileType::Door,
            TileType::StairsUp,
            TileType::StairsDown,
            TileType::Chest,
            TileType::Trap,
            TileType::Spawner,
            TileType::Shrine,
            TileType::WindColumn,
            TileType::VoidPit,
        ]
    }

    /// Weight of this tile appearing (higher = more common)
    pub fn base_weight(&self) -> f32 {
        match self {
            Self::Empty => 0.0,
            Self::Floor => 50.0,
            Self::Wall => 30.0,
            Self::Door => 5.0,
            Self::StairsUp => 1.0,
            Self::StairsDown => 1.0,
            Self::Chest => 3.0,
            Self::Trap => 4.0,
            Self::Spawner => 6.0,
            Self::Shrine => 2.0,
            Self::WindColumn => 3.0,
            Self::VoidPit => 2.0,
        }
    }

    /// Can this tile be adjacent to another?
    pub fn can_be_adjacent(&self, other: &TileType) -> bool {
        match (self, other) {
            // Void pits cannot be adjacent to stairs
            (Self::VoidPit, Self::StairsUp | Self::StairsDown) => false,
            (Self::StairsUp | Self::StairsDown, Self::VoidPit) => false,
            // Doors must be between walls or floors
            (Self::Door, Self::Door) => false,
            // Spawners need floor space around them
            (Self::Spawner, Self::Wall | Self::VoidPit) => false,
            // Everything else is allowed
            _ => true,
        }
    }
}

/// A cell in the WFC grid - starts with all possibilities, collapses to one
#[derive(Debug, Clone)]
pub struct WfcCell {
    pub possible: Vec<TileType>,
    pub collapsed: Option<TileType>,
}

impl Default for WfcCell {
    fn default() -> Self {
        Self::new()
    }
}

impl WfcCell {
    pub fn new() -> Self {
        Self {
            possible: TileType::all_placeable().to_vec(),
            collapsed: None,
        }
    }

    pub fn entropy(&self) -> usize {
        if self.collapsed.is_some() {
            0
        } else {
            self.possible.len()
        }
    }

    pub fn is_collapsed(&self) -> bool {
        self.collapsed.is_some()
    }
}

/// Generated floor layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLayout {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Vec<TileType>>,
    pub rooms: Vec<Room>,
    pub spawn_points: Vec<(usize, usize)>,
    pub exit_point: (usize, usize),
}

/// A room within the floor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub room_type: RoomType,
    pub semantic_tags: SemanticTags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomType {
    Combat,
    Treasure,
    Puzzle,
    Rest,
    Boss,
    Entrance,
    Exit,
}

/// Simple deterministic RNG from a seed (xorshift64)
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn next_f32(&mut self) -> f32 {
        (self.next() % 10000) as f32 / 10000.0
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        min + (self.next() as usize % (max - min))
    }
}

/// Generate a floor layout from a FloorSpec
pub fn generate_layout(spec: &FloorSpec) -> FloorLayout {
    let mut rng = DeterministicRng::new(spec.hash);

    let (width, height) = grid_size_for_tier(&spec.tier);

    // Phase 1: Generate rooms
    let room_count = room_count_for_tier(&spec.tier, &mut rng);
    let rooms = generate_rooms(width, height, room_count, spec, &mut rng);

    // Phase 2: Fill tile grid from rooms
    let mut tiles = vec![vec![TileType::Wall; width]; height];
    carve_rooms(&mut tiles, &rooms);
    connect_rooms(&mut tiles, &rooms, &mut rng);

    // Phase 3: Place special tiles
    place_special_tiles(&mut tiles, &rooms, spec, &mut rng);

    // Phase 4: Find spawn/exit
    let spawn_points = find_tiles(&tiles, TileType::StairsDown);
    let exit_point = find_tiles(&tiles, TileType::StairsUp)
        .first()
        .copied()
        .unwrap_or((width / 2, height / 2));

    FloorLayout {
        width,
        height,
        tiles,
        rooms,
        spawn_points,
        exit_point,
    }
}

fn grid_size_for_tier(tier: &FloorTier) -> (usize, usize) {
    match tier {
        FloorTier::Echelon1 => (16, 16),
        FloorTier::Echelon2 => (24, 24),
        FloorTier::Echelon3 => (32, 32),
        FloorTier::Echelon4 => (48, 48),
    }
}

fn room_count_for_tier(tier: &FloorTier, rng: &mut DeterministicRng) -> usize {
    let (min, max) = match tier {
        FloorTier::Echelon1 => (3, 6),
        FloorTier::Echelon2 => (5, 10),
        FloorTier::Echelon3 => (8, 15),
        FloorTier::Echelon4 => (12, 20),
    };
    rng.next_range(min, max + 1)
}

fn generate_rooms(
    grid_w: usize,
    grid_h: usize,
    count: usize,
    spec: &FloorSpec,
    rng: &mut DeterministicRng,
) -> Vec<Room> {
    let mut rooms = Vec::new();
    let mut attempts = 0;

    while rooms.len() < count && attempts < count * 20 {
        attempts += 1;

        let w = rng.next_range(3, 8);
        let h = rng.next_range(3, 8);
        let x = rng.next_range(1, grid_w.saturating_sub(w + 1));
        let y = rng.next_range(1, grid_h.saturating_sub(h + 1));

        // Check overlap
        let overlaps = rooms.iter().any(|r: &Room| {
            x < r.x + r.width + 1 && x + w + 1 > r.x && y < r.y + r.height + 1 && y + h + 1 > r.y
        });

        if overlaps {
            continue;
        }

        let room_type = if rooms.is_empty() {
            RoomType::Entrance
        } else if rooms.len() == count - 1 {
            RoomType::Exit
        } else {
            let roll = rng.next_f32();
            if roll < 0.4 {
                RoomType::Combat
            } else if roll < 0.6 {
                RoomType::Treasure
            } else if roll < 0.75 {
                RoomType::Puzzle
            } else if roll < 0.9 {
                RoomType::Rest
            } else {
                RoomType::Boss
            }
        };

        // Room semantic tags derived from floor biome + room type
        let mut tags = vec![];
        let fire_base = spec.biome_tags.get("fire");
        let water_base = spec.biome_tags.get("water");

        match room_type {
            RoomType::Combat => {
                tags.push(("danger", 0.8));
                tags.push(("fire", fire_base * 1.2));
            }
            RoomType::Treasure => {
                tags.push(("exploration", 0.9));
                tags.push(("reward", 0.7));
            }
            RoomType::Puzzle => {
                tags.push(("semantic", 0.8));
                tags.push(("exploration", 0.6));
            }
            RoomType::Rest => {
                tags.push(("healing", 0.7));
                tags.push(("water", water_base * 1.3));
            }
            RoomType::Boss => {
                tags.push(("danger", 1.0));
                tags.push(("corruption", spec.biome_tags.get("corruption") * 1.5));
            }
            RoomType::Entrance | RoomType::Exit => {
                tags.push(("neutral", 0.5));
            }
        }

        rooms.push(Room {
            x,
            y,
            width: w,
            height: h,
            room_type,
            semantic_tags: SemanticTags::new(tags),
        });
    }

    // Guarantee Exit room exists — promote last non-Entrance room if needed
    let has_exit = rooms.iter().any(|r| matches!(r.room_type, RoomType::Exit));
    if !has_exit && rooms.len() >= 2 {
        let last = rooms.len() - 1;
        rooms[last].room_type = RoomType::Exit;
    }

    rooms
}

fn carve_rooms(tiles: &mut [Vec<TileType>], rooms: &[Room]) {
    for room in rooms {
        for dy in 0..room.height {
            for dx in 0..room.width {
                let y = room.y + dy;
                let x = room.x + dx;
                if y < tiles.len() && x < tiles[0].len() {
                    tiles[y][x] = TileType::Floor;
                }
            }
        }
    }
}

fn connect_rooms(tiles: &mut [Vec<TileType>], rooms: &[Room], rng: &mut DeterministicRng) {
    for i in 0..rooms.len().saturating_sub(1) {
        let (cx1, cy1) = room_center(&rooms[i]);
        let (cx2, cy2) = room_center(&rooms[i + 1]);

        // L-shaped corridor
        if rng.next_f32() < 0.5 {
            carve_h_corridor(tiles, cx1, cx2, cy1);
            carve_v_corridor(tiles, cy1, cy2, cx2);
        } else {
            carve_v_corridor(tiles, cy1, cy2, cx1);
            carve_h_corridor(tiles, cx1, cx2, cy2);
        }

        // Place door at junction
        let door_x = cx2.min(tiles[0].len() - 1);
        let door_y = cy1.min(tiles.len() - 1);
        if tiles[door_y][door_x] == TileType::Floor {
            tiles[door_y][door_x] = TileType::Door;
        }
    }
}

fn room_center(room: &Room) -> (usize, usize) {
    (room.x + room.width / 2, room.y + room.height / 2)
}

fn carve_h_corridor(tiles: &mut [Vec<TileType>], x1: usize, x2: usize, y: usize) {
    let start = x1.min(x2);
    let end = x1.max(x2);
    for x in start..=end {
        if y < tiles.len() && x < tiles[0].len() && tiles[y][x] == TileType::Wall {
            tiles[y][x] = TileType::Floor;
        }
    }
}

fn carve_v_corridor(tiles: &mut [Vec<TileType>], y1: usize, y2: usize, x: usize) {
    let start = y1.min(y2);
    let end = y1.max(y2);
    for y in start..=end {
        if y < tiles.len() && x < tiles[0].len() && tiles[y][x] == TileType::Wall {
            tiles[y][x] = TileType::Floor;
        }
    }
}

fn place_special_tiles(
    tiles: &mut [Vec<TileType>],
    rooms: &[Room],
    spec: &FloorSpec,
    rng: &mut DeterministicRng,
) {
    for room in rooms {
        let (cx, cy) = room_center(room);

        match room.room_type {
            RoomType::Entrance => {
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::StairsDown;
                }
            }
            RoomType::Exit => {
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::StairsUp;
                }
            }
            RoomType::Treasure => {
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::Chest;
                }
            }
            RoomType::Combat => {
                // Place spawner in center
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::Spawner;
                }
                // Place traps around combat rooms based on corruption
                let corruption = spec.biome_tags.get("corruption");
                if corruption > 0.5 && room.width > 3 && room.height > 3 {
                    let tx = room.x + rng.next_range(1, room.width - 1);
                    let ty = room.y + rng.next_range(1, room.height - 1);
                    if ty < tiles.len() && tx < tiles[0].len() && tiles[ty][tx] == TileType::Floor {
                        tiles[ty][tx] = TileType::Trap;
                    }
                }
            }
            RoomType::Rest => {
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::Shrine;
                }
            }
            RoomType::Boss => {
                // Boss room gets spawner + wind column for aerial combat
                if cy < tiles.len() && cx < tiles[0].len() {
                    tiles[cy][cx] = TileType::Spawner;
                }
                if room.width > 4 && room.height > 4 {
                    let wx = room.x + 1;
                    let wy = room.y + 1;
                    if wy < tiles.len() && wx < tiles[0].len() {
                        tiles[wy][wx] = TileType::WindColumn;
                    }
                }
            }
            RoomType::Puzzle => {
                // Puzzles get void pits
                if spec.biome_tags.get("exploration") > 0.4 && room.width > 3 {
                    let vx = room.x + room.width - 1;
                    let vy = room.y + room.height / 2;
                    if vy < tiles.len() && vx < tiles[0].len() && tiles[vy][vx] == TileType::Floor {
                        tiles[vy][vx] = TileType::VoidPit;
                    }
                }
            }
        }
    }
}

fn find_tiles(tiles: &[Vec<TileType>], tile_type: TileType) -> Vec<(usize, usize)> {
    let mut result = Vec::new();
    for (y, row) in tiles.iter().enumerate() {
        for (x, tile) in row.iter().enumerate() {
            if *tile == tile_type {
                result.push((x, y));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::TowerSeed;

    #[test]
    fn test_generate_layout_deterministic() {
        let seed = TowerSeed { seed: 42 };
        let spec = FloorSpec::generate(&seed, 1);
        let layout_a = generate_layout(&spec);
        let layout_b = generate_layout(&spec);

        assert_eq!(layout_a.width, layout_b.width);
        assert_eq!(layout_a.height, layout_b.height);
        assert_eq!(
            layout_a.tiles, layout_b.tiles,
            "Same seed must produce same layout"
        );
    }

    #[test]
    fn test_different_floors_different_layouts() {
        let seed = TowerSeed { seed: 42 };
        let spec1 = FloorSpec::generate(&seed, 1);
        let spec2 = FloorSpec::generate(&seed, 2);
        let layout1 = generate_layout(&spec1);
        let layout2 = generate_layout(&spec2);

        // Very unlikely to be identical
        assert_ne!(
            layout1.tiles, layout2.tiles,
            "Different floors should differ"
        );
    }

    #[test]
    fn test_layout_has_entrance_and_exit() {
        let seed = TowerSeed { seed: 12345 };
        let spec = FloorSpec::generate(&seed, 50);
        let layout = generate_layout(&spec);

        let stairs_up = find_tiles(&layout.tiles, TileType::StairsUp);
        let stairs_down = find_tiles(&layout.tiles, TileType::StairsDown);

        assert!(!stairs_up.is_empty(), "Layout must have exit stairs");
        assert!(!stairs_down.is_empty(), "Layout must have entrance stairs");
    }

    #[test]
    fn test_echelon_sizes() {
        let seed = TowerSeed { seed: 42 };

        let e1 = FloorSpec::generate(&seed, 50); // Echelon1
        let e4 = FloorSpec::generate(&seed, 600); // Echelon4

        let l1 = generate_layout(&e1);
        let l4 = generate_layout(&e4);

        assert!(
            l4.width > l1.width,
            "Higher echelon should have larger floors"
        );
    }

    #[test]
    fn test_rooms_generated() {
        let seed = TowerSeed { seed: 99 };
        let spec = FloorSpec::generate(&seed, 1);
        let layout = generate_layout(&spec);

        assert!(
            layout.rooms.len() >= 2,
            "Must have at least entrance + exit rooms"
        );
    }

    #[test]
    fn test_tile_adjacency_rules() {
        assert!(!TileType::VoidPit.can_be_adjacent(&TileType::StairsUp));
        assert!(!TileType::Door.can_be_adjacent(&TileType::Door));
        assert!(TileType::Floor.can_be_adjacent(&TileType::Wall));
        assert!(TileType::Floor.can_be_adjacent(&TileType::Floor));
    }
}
