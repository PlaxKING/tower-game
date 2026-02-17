//! Wave Function Collapse Floor Generator
//!
//! Generates room-based floor layouts deterministically from seed + floor_id.
//! Adapted from procedural-core's WFC algorithm for standalone use in bevy-server.
//!
//! ## Pipeline
//! ```text
//! (seed, floor_id) → FloorTier → grid size + room count
//!       → generate rooms (non-overlapping, typed)
//!       → carve rooms into tile grid
//!       → connect rooms with L-shaped corridors
//!       → place special tiles (stairs, chests, spawners, traps, shrines)
//!       → FloorLayout (2D tile grid + room metadata)
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Tile Types
// ============================================================================

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
    Shrine,
    WindColumn,
    VoidPit,
}

impl TileType {
    /// Numeric ID for protobuf/network serialization
    pub fn to_id(self) -> u32 {
        match self {
            TileType::Empty => 0,
            TileType::Floor => 1,
            TileType::Wall => 2,
            TileType::Door => 3,
            TileType::StairsUp => 4,
            TileType::StairsDown => 5,
            TileType::Chest => 6,
            TileType::Trap => 7,
            TileType::Spawner => 8,
            TileType::Shrine => 9,
            TileType::WindColumn => 10,
            TileType::VoidPit => 11,
        }
    }

    /// Whether players can walk on this tile
    pub fn is_walkable(self) -> bool {
        !matches!(self, TileType::Wall | TileType::Empty | TileType::VoidPit)
    }

    /// Whether this tile has physics collision
    pub fn has_collision(self) -> bool {
        matches!(self, TileType::Wall)
    }
}

// ============================================================================
// Floor Layout
// ============================================================================

/// Generated floor layout — deterministic from seed + floor_id
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorLayout {
    pub width: usize,
    pub height: usize,
    /// 2D tile grid [y][x]
    pub tiles: Vec<Vec<TileType>>,
    pub rooms: Vec<Room>,
    pub spawn_points: Vec<(usize, usize)>,
    pub exit_point: (usize, usize),
    pub floor_id: u32,
    pub seed: u64,
}

/// A room within the floor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub room_type: RoomType,
    pub semantic_tags: HashMap<String, f32>,
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

// ============================================================================
// Floor Tier (determines size and complexity)
// ============================================================================

#[derive(Debug, Clone, Copy)]
pub enum FloorTier {
    /// Floors 1-100: Small (16x16)
    Echelon1,
    /// Floors 101-300: Medium (24x24)
    Echelon2,
    /// Floors 301-600: Large (32x32)
    Echelon3,
    /// Floors 601+: Massive (48x48)
    Echelon4,
}

impl FloorTier {
    pub fn from_floor_id(floor_id: u32) -> Self {
        match floor_id {
            0..=100 => FloorTier::Echelon1,
            101..=300 => FloorTier::Echelon2,
            301..=600 => FloorTier::Echelon3,
            _ => FloorTier::Echelon4,
        }
    }

    fn grid_size(self) -> (usize, usize) {
        match self {
            FloorTier::Echelon1 => (16, 16),
            FloorTier::Echelon2 => (24, 24),
            FloorTier::Echelon3 => (32, 32),
            FloorTier::Echelon4 => (48, 48),
        }
    }

    fn room_count_range(self) -> (usize, usize) {
        match self {
            FloorTier::Echelon1 => (3, 6),
            FloorTier::Echelon2 => (5, 10),
            FloorTier::Echelon3 => (8, 15),
            FloorTier::Echelon4 => (12, 20),
        }
    }
}

// ============================================================================
// Deterministic RNG (xorshift64)
// ============================================================================

struct Rng {
    state: u64,
}

impl Rng {
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

// ============================================================================
// Main Entry Point
// ============================================================================

/// Generate a floor layout deterministically from seed and floor_id.
pub fn generate_layout(seed: u64, floor_id: u32) -> FloorLayout {
    let combined_seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(floor_id as u64);
    let mut rng = Rng::new(combined_seed);

    let tier = FloorTier::from_floor_id(floor_id);
    let (width, height) = tier.grid_size();

    // Biome tags for this floor (affects room semantics and special tiles)
    let biome_tags = determine_biome_tags(floor_id);

    // Phase 1: Generate rooms
    let (min_rooms, max_rooms) = tier.room_count_range();
    let room_count = rng.next_range(min_rooms, max_rooms + 1);
    let rooms = generate_rooms(width, height, room_count, &biome_tags, &mut rng);

    // Phase 2: Fill tile grid
    let mut tiles = vec![vec![TileType::Wall; width]; height];
    carve_rooms(&mut tiles, &rooms);
    connect_rooms(&mut tiles, &rooms, &mut rng);

    // Phase 3: Place special tiles
    place_special_tiles(&mut tiles, &rooms, &biome_tags, &mut rng);

    // Phase 4: Find spawn/exit points
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
        floor_id,
        seed,
    }
}

// ============================================================================
// Biome Tags
// ============================================================================

/// Generate biome semantic tags based on floor depth
fn determine_biome_tags(floor_id: u32) -> HashMap<String, f32> {
    let mut tags = HashMap::new();
    match floor_id {
        0..=20 => {
            tags.insert("nature".into(), 0.8);
            tags.insert("exploration".into(), 0.7);
        }
        21..=50 => {
            tags.insert("forest".into(), 0.8);
            tags.insert("nature".into(), 0.5);
            tags.insert("exploration".into(), 0.6);
        }
        51..=100 => {
            tags.insert("dungeon".into(), 0.8);
            tags.insert("danger".into(), 0.5);
        }
        101..=200 => {
            tags.insert("fire".into(), 0.7);
            tags.insert("danger".into(), 0.6);
            tags.insert("corruption".into(), 0.3);
        }
        201..=400 => {
            tags.insert("ice".into(), 0.7);
            tags.insert("water".into(), 0.5);
            tags.insert("danger".into(), 0.7);
        }
        401..=600 => {
            tags.insert("volcano".into(), 0.8);
            tags.insert("fire".into(), 0.9);
            tags.insert("corruption".into(), 0.6);
        }
        _ => {
            tags.insert("void".into(), 0.9);
            tags.insert("corruption".into(), 0.8);
            tags.insert("danger".into(), 0.9);
        }
    }
    tags
}

// ============================================================================
// Room Generation
// ============================================================================

fn generate_rooms(
    grid_w: usize,
    grid_h: usize,
    count: usize,
    biome_tags: &HashMap<String, f32>,
    rng: &mut Rng,
) -> Vec<Room> {
    let mut rooms = Vec::new();
    let mut attempts = 0;

    while rooms.len() < count && attempts < count * 20 {
        attempts += 1;

        let w = rng.next_range(3, 8);
        let h = rng.next_range(3, 8);
        let x = rng.next_range(1, grid_w.saturating_sub(w + 1).max(2));
        let y = rng.next_range(1, grid_h.saturating_sub(h + 1).max(2));

        // Check overlap (with 1-tile buffer)
        let overlaps = rooms.iter().any(|r: &Room| {
            x < r.x + r.width + 1 && x + w + 1 > r.x && y < r.y + r.height + 1 && y + h + 1 > r.y
        });
        if overlaps {
            continue;
        }

        // Determine room type
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

        // Room semantic tags (inherit from biome + room-specific modifiers)
        let semantic_tags = room_semantic_tags(room_type, biome_tags);

        rooms.push(Room {
            x,
            y,
            width: w,
            height: h,
            room_type,
            semantic_tags,
        });
    }

    // Guarantee exit room exists
    let has_exit = rooms.iter().any(|r| r.room_type == RoomType::Exit);
    if !has_exit && rooms.len() >= 2 {
        let last = rooms.len() - 1;
        rooms[last].room_type = RoomType::Exit;
    }

    rooms
}

fn room_semantic_tags(
    room_type: RoomType,
    biome_tags: &HashMap<String, f32>,
) -> HashMap<String, f32> {
    let mut tags = HashMap::new();

    // Inherit biome tags at 50% weight
    for (key, val) in biome_tags {
        tags.insert(key.clone(), val * 0.5);
    }

    // Room-specific tags
    match room_type {
        RoomType::Combat => {
            tags.insert("danger".into(), 0.8);
            if let Some(fire) = biome_tags.get("fire") {
                *tags.entry("fire".into()).or_insert(0.0) += fire * 0.3;
            }
        }
        RoomType::Treasure => {
            tags.insert("exploration".into(), 0.9);
            tags.insert("reward".into(), 0.7);
        }
        RoomType::Puzzle => {
            tags.insert("semantic".into(), 0.8);
            tags.insert("exploration".into(), 0.6);
        }
        RoomType::Rest => {
            tags.insert("healing".into(), 0.7);
            if let Some(water) = biome_tags.get("water") {
                *tags.entry("water".into()).or_insert(0.0) += water * 0.3;
            }
        }
        RoomType::Boss => {
            tags.insert("danger".into(), 1.0);
            if let Some(corruption) = biome_tags.get("corruption") {
                *tags.entry("corruption".into()).or_insert(0.0) += corruption * 0.5;
            }
        }
        RoomType::Entrance | RoomType::Exit => {
            tags.insert("neutral".into(), 0.5);
        }
    }

    tags
}

// ============================================================================
// Tile Grid Operations
// ============================================================================

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

fn connect_rooms(tiles: &mut [Vec<TileType>], rooms: &[Room], rng: &mut Rng) {
    for i in 0..rooms.len().saturating_sub(1) {
        let (cx1, cy1) = room_center(&rooms[i]);
        let (cx2, cy2) = room_center(&rooms[i + 1]);

        // L-shaped corridor (random orientation)
        if rng.next_f32() < 0.5 {
            carve_h_corridor(tiles, cx1, cx2, cy1);
            carve_v_corridor(tiles, cy1, cy2, cx2);
        } else {
            carve_v_corridor(tiles, cy1, cy2, cx1);
            carve_h_corridor(tiles, cx1, cx2, cy2);
        }

        // Place door at corridor junction
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
    let (start, end) = (x1.min(x2), x1.max(x2));
    for x in start..=end {
        if y < tiles.len() && x < tiles[0].len() && tiles[y][x] == TileType::Wall {
            tiles[y][x] = TileType::Floor;
        }
    }
}

fn carve_v_corridor(tiles: &mut [Vec<TileType>], y1: usize, y2: usize, x: usize) {
    let (start, end) = (y1.min(y2), y1.max(y2));
    for y in start..=end {
        if y < tiles.len() && x < tiles[0].len() && tiles[y][x] == TileType::Wall {
            tiles[y][x] = TileType::Floor;
        }
    }
}

// ============================================================================
// Special Tile Placement
// ============================================================================

fn place_special_tiles(
    tiles: &mut [Vec<TileType>],
    rooms: &[Room],
    biome_tags: &HashMap<String, f32>,
    rng: &mut Rng,
) {
    for room in rooms {
        let (cx, cy) = room_center(room);

        match room.room_type {
            RoomType::Entrance => {
                set_tile(tiles, cx, cy, TileType::StairsDown);
            }
            RoomType::Exit => {
                set_tile(tiles, cx, cy, TileType::StairsUp);
            }
            RoomType::Treasure => {
                set_tile(tiles, cx, cy, TileType::Chest);
            }
            RoomType::Combat => {
                set_tile(tiles, cx, cy, TileType::Spawner);
                // Traps in corrupted areas
                let corruption = biome_tags.get("corruption").copied().unwrap_or(0.0);
                if corruption > 0.3 && room.width > 3 && room.height > 3 {
                    let tx = room.x + rng.next_range(1, room.width - 1);
                    let ty = room.y + rng.next_range(1, room.height - 1);
                    if get_tile(tiles, tx, ty) == Some(TileType::Floor) {
                        set_tile(tiles, tx, ty, TileType::Trap);
                    }
                }
            }
            RoomType::Rest => {
                set_tile(tiles, cx, cy, TileType::Shrine);
            }
            RoomType::Boss => {
                set_tile(tiles, cx, cy, TileType::Spawner);
                // Wind column for aerial combat
                if room.width > 4 && room.height > 4 {
                    set_tile(tiles, room.x + 1, room.y + 1, TileType::WindColumn);
                }
            }
            RoomType::Puzzle => {
                let exploration = biome_tags.get("exploration").copied().unwrap_or(0.0);
                if exploration > 0.3 && room.width > 3 {
                    let vx = room.x + room.width - 1;
                    let vy = room.y + room.height / 2;
                    if get_tile(tiles, vx, vy) == Some(TileType::Floor) {
                        set_tile(tiles, vx, vy, TileType::VoidPit);
                    }
                }
            }
        }
    }
}

fn set_tile(tiles: &mut [Vec<TileType>], x: usize, y: usize, tile: TileType) {
    if y < tiles.len() && x < tiles[0].len() {
        tiles[y][x] = tile;
    }
}

fn get_tile(tiles: &[Vec<TileType>], x: usize, y: usize) -> Option<TileType> {
    tiles.get(y).and_then(|row| row.get(x)).copied()
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let layout_a = generate_layout(42, 1);
        let layout_b = generate_layout(42, 1);
        assert_eq!(
            layout_a.tiles, layout_b.tiles,
            "Same seed must produce same layout"
        );
        assert_eq!(layout_a.rooms.len(), layout_b.rooms.len());
    }

    #[test]
    fn test_different_floors_differ() {
        let layout_1 = generate_layout(42, 1);
        let layout_2 = generate_layout(42, 2);
        assert_ne!(
            layout_1.tiles, layout_2.tiles,
            "Different floors should differ"
        );
    }

    #[test]
    fn test_different_seeds_differ() {
        let layout_a = generate_layout(100, 5);
        let layout_b = generate_layout(200, 5);
        assert_ne!(
            layout_a.tiles, layout_b.tiles,
            "Different seeds should differ"
        );
    }

    #[test]
    fn test_has_entrance_and_exit() {
        let layout = generate_layout(12345, 50);
        let stairs_up = find_tiles(&layout.tiles, TileType::StairsUp);
        let stairs_down = find_tiles(&layout.tiles, TileType::StairsDown);
        assert!(!stairs_up.is_empty(), "Must have exit stairs");
        assert!(!stairs_down.is_empty(), "Must have entrance stairs");
    }

    #[test]
    fn test_echelon_sizes() {
        let e1 = generate_layout(42, 50); // Echelon1
        let e4 = generate_layout(42, 700); // Echelon4
        assert!(e4.width > e1.width, "Higher echelon = larger floor");
        assert_eq!(e1.width, 16);
        assert_eq!(e4.width, 48);
    }

    #[test]
    fn test_rooms_exist() {
        let layout = generate_layout(99, 1);
        assert!(layout.rooms.len() >= 2, "Need at least entrance + exit");
    }

    #[test]
    fn test_room_types() {
        let layout = generate_layout(42, 1);
        let has_entrance = layout
            .rooms
            .iter()
            .any(|r| r.room_type == RoomType::Entrance);
        assert!(has_entrance, "Must have entrance room");
    }

    #[test]
    fn test_tile_walkability() {
        assert!(TileType::Floor.is_walkable());
        assert!(TileType::Door.is_walkable());
        assert!(TileType::Chest.is_walkable());
        assert!(!TileType::Wall.is_walkable());
        assert!(!TileType::VoidPit.is_walkable());
        assert!(!TileType::Empty.is_walkable());
    }

    #[test]
    fn test_tile_ids() {
        assert_eq!(TileType::Empty.to_id(), 0);
        assert_eq!(TileType::Floor.to_id(), 1);
        assert_eq!(TileType::Wall.to_id(), 2);
        assert_eq!(TileType::VoidPit.to_id(), 11);
    }

    #[test]
    fn test_floor_has_walkable_tiles() {
        let layout = generate_layout(42, 1);
        let floor_count = layout
            .tiles
            .iter()
            .flat_map(|row| row.iter())
            .filter(|t| t.is_walkable())
            .count();
        assert!(
            floor_count > 20,
            "Should have many walkable tiles, got {}",
            floor_count
        );
    }

    #[test]
    fn test_biome_tags_vary_by_depth() {
        let shallow = determine_biome_tags(10);
        let deep = determine_biome_tags(500);
        // Shallow floors should have nature tags
        assert!(shallow.contains_key("nature"));
        // Deep floors should have fire/corruption tags
        assert!(deep.contains_key("fire") || deep.contains_key("corruption"));
    }

    #[test]
    fn test_room_semantic_tags_exist() {
        let layout = generate_layout(42, 1);
        for room in &layout.rooms {
            assert!(
                !room.semantic_tags.is_empty(),
                "Room {:?} should have tags",
                room.room_type
            );
        }
    }

    #[test]
    fn test_spawner_placement() {
        // Generate many floors and check spawners exist on combat floors
        let layout = generate_layout(42, 50);
        let combat_rooms: Vec<_> = layout
            .rooms
            .iter()
            .filter(|r| r.room_type == RoomType::Combat || r.room_type == RoomType::Boss)
            .collect();

        if !combat_rooms.is_empty() {
            let spawner_tiles = find_tiles(&layout.tiles, TileType::Spawner);
            assert!(
                !spawner_tiles.is_empty(),
                "Combat rooms should have spawners"
            );
        }
    }

    #[test]
    fn test_large_floor_generation() {
        // Echelon 4 (48x48) should still work
        let layout = generate_layout(42, 999);
        assert_eq!(layout.width, 48);
        assert_eq!(layout.height, 48);
        assert!(
            layout.rooms.len() >= 8,
            "Large floor should have many rooms"
        );
    }
}
