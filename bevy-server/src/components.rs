//! Shared ECS components used by both the Bevy game loop and the API bridge.
//!
//! These are the canonical game entity types — replicated to clients via bevy_replicon,
//! queried by ECS systems, and snapshotted by the API bridge.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Player entity component (replicated to clients)
#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct Player {
    pub id: u64,
    pub position: Vec3,
    pub health: f32,
    pub current_floor: u32,
}

/// Monster entity component (replicated to clients)
#[derive(Component, Serialize, Deserialize, Debug, Clone)]
pub struct Monster {
    pub monster_type: String,
    pub position: Vec3,
    pub health: f32,
    pub max_health: f32,
}

/// Floor tile component (replicated to clients)
#[derive(Component, Clone, Serialize, Deserialize, Debug)]
pub struct FloorTile {
    pub tile_type: u8,
    pub grid_x: i32,
    pub grid_y: i32,
}
