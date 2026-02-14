//! Floor transition and progression system.
//!
//! Manages loading/unloading floors as the player moves through the tower.
//! Handles staircase interactions, floor generation queueing, and seed tracking.

use bevy::prelude::*;

use super::wfc::{generate_layout, FloorLayout, TileType};
use super::{FloorSpec, TowerSeed};

/// Current active floor state
#[derive(Resource, Debug)]
pub struct ActiveFloor {
    pub floor_number: u32,
    pub seed: TowerSeed,
    pub spec: FloorSpec,
    pub layout: FloorLayout,
    pub explored: bool,
}

/// Event: player requests floor transition
#[derive(Event, Debug)]
pub struct FloorTransitionEvent {
    pub direction: FloorDirection,
}

/// Direction of floor transition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloorDirection {
    Up,
    Down,
}

/// Event: new floor is ready
#[derive(Event, Debug)]
pub struct FloorReadyEvent {
    pub floor_number: u32,
    pub layout: FloorLayout,
}

/// Tower progression tracker
#[derive(Resource, Debug)]
pub struct TowerProgress {
    pub tower_seed: TowerSeed,
    pub current_floor: u32,
    pub highest_reached: u32,
    pub floors_cleared: Vec<u32>,
    pub total_deaths: u32,
}

impl TowerProgress {
    pub fn new(seed: u64) -> Self {
        Self {
            tower_seed: TowerSeed { seed },
            current_floor: 1,
            highest_reached: 1,
            floors_cleared: Vec::new(),
            total_deaths: 0,
        }
    }

    pub fn is_cleared(&self, floor: u32) -> bool {
        self.floors_cleared.contains(&floor)
    }

    pub fn clear_floor(&mut self, floor: u32) {
        if !self.floors_cleared.contains(&floor) {
            self.floors_cleared.push(floor);
        }
    }
}

/// System: generate floor on transition
pub fn handle_floor_transitions(
    mut transition_events: EventReader<FloorTransitionEvent>,
    mut floor_ready_events: EventWriter<FloorReadyEvent>,
    mut progress: ResMut<TowerProgress>,
    mut active_floor: Option<ResMut<ActiveFloor>>,
    mut commands: Commands,
) {
    for event in transition_events.read() {
        let next_floor = match event.direction {
            FloorDirection::Up => progress.current_floor + 1,
            FloorDirection::Down => {
                if progress.current_floor > 1 {
                    progress.current_floor - 1
                } else {
                    continue; // Can't go below floor 1
                }
            }
        };

        // Generate the new floor
        let spec = FloorSpec::generate(&progress.tower_seed, next_floor);
        let layout = generate_layout(&spec);

        // Update progression
        progress.current_floor = next_floor;
        if next_floor > progress.highest_reached {
            progress.highest_reached = next_floor;
        }

        // Update or insert active floor resource
        let new_floor = ActiveFloor {
            floor_number: next_floor,
            seed: progress.tower_seed.clone(),
            spec: spec.clone(),
            layout: layout.clone(),
            explored: progress.is_cleared(next_floor),
        };

        if let Some(ref mut floor) = active_floor {
            floor.floor_number = new_floor.floor_number;
            floor.seed = new_floor.seed;
            floor.spec = new_floor.spec;
            floor.layout = new_floor.layout;
            floor.explored = new_floor.explored;
        } else {
            commands.insert_resource(new_floor);
        }

        floor_ready_events.send(FloorReadyEvent {
            floor_number: next_floor,
            layout,
        });

        info!("Floor transition: moved to floor {}", next_floor);
    }
}

/// System: detect player standing on stairs
pub fn detect_stair_interaction(
    active_floor: Option<Res<ActiveFloor>>,
    players: Query<&Transform, With<crate::player::Player>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut transition_events: EventWriter<FloorTransitionEvent>,
) {
    let floor = match active_floor {
        Some(f) => f,
        None => return,
    };

    if !keys.just_pressed(KeyCode::KeyE) {
        return; // Interact key
    }

    for player_tf in &players {
        // Convert player position to tile coordinates
        let tile_x = (player_tf.translation.x / 2.0).round() as usize;
        let tile_z = (player_tf.translation.z / 2.0).round() as usize;

        if tile_z >= floor.layout.height || tile_x >= floor.layout.width {
            continue;
        }

        match floor.layout.tiles[tile_z][tile_x] {
            TileType::StairsUp => {
                transition_events.send(FloorTransitionEvent {
                    direction: FloorDirection::Up,
                });
            }
            TileType::StairsDown if floor.floor_number > 1 => {
                transition_events.send(FloorTransitionEvent {
                    direction: FloorDirection::Down,
                });
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tower_progress_new() {
        let progress = TowerProgress::new(42);
        assert_eq!(progress.current_floor, 1);
        assert_eq!(progress.highest_reached, 1);
        assert!(progress.floors_cleared.is_empty());
    }

    #[test]
    fn test_tower_progress_clear_floor() {
        let mut progress = TowerProgress::new(42);
        progress.clear_floor(1);
        progress.clear_floor(1); // duplicate
        assert!(progress.is_cleared(1));
        assert!(!progress.is_cleared(2));
        assert_eq!(progress.floors_cleared.len(), 1);
    }

    #[test]
    fn test_floor_generation_consistency() {
        let seed = TowerSeed { seed: 42 };
        let spec = FloorSpec::generate(&seed, 5);
        let layout1 = generate_layout(&spec);
        let layout2 = generate_layout(&spec);
        assert_eq!(
            layout1.tiles, layout2.tiles,
            "Same floor should generate identically"
        );
    }
}
