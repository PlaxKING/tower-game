//! Floor visualization system.
//!
//! Renders generated WFC floor layouts as 3D meshes in Bevy.
//! Each tile type gets a distinct color and height.

use bevy::prelude::*;

use crate::generation::wfc::{FloorLayout, TileType};

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<RenderFloorEvent>()
            .add_systems(Update, render_floor_layout);
    }
}

/// Event to trigger floor rendering
#[derive(Event, Debug)]
pub struct RenderFloorEvent {
    pub layout: FloorLayout,
    pub origin: Vec3,
    pub tile_size: f32,
}

/// Marker for spawned floor tiles (for cleanup)
#[derive(Component)]
pub struct FloorTileMarker {
    pub floor_id: u32,
}

const WALL_HEIGHT: f32 = 3.0;

fn tile_color(tile_type: &TileType) -> Color {
    match tile_type {
        TileType::Empty => Color::BLACK,
        TileType::Floor => Color::srgb(0.6, 0.6, 0.6),
        TileType::Wall => Color::srgb(0.3, 0.3, 0.35),
        TileType::Door => Color::srgb(0.6, 0.4, 0.2),
        TileType::StairsUp => Color::srgb(0.2, 0.8, 0.2),
        TileType::StairsDown => Color::srgb(0.2, 0.2, 0.8),
        TileType::Chest => Color::srgb(0.9, 0.8, 0.1),
        TileType::Trap => Color::srgb(0.8, 0.1, 0.1),
        TileType::Spawner => Color::srgb(0.8, 0.0, 0.5),
        TileType::Shrine => Color::srgb(0.4, 0.8, 0.9),
        TileType::WindColumn => Color::srgb(0.7, 0.9, 1.0),
        TileType::VoidPit => Color::srgb(0.1, 0.0, 0.15),
    }
}

fn tile_height(tile_type: &TileType) -> f32 {
    match tile_type {
        TileType::Wall => WALL_HEIGHT,
        TileType::Chest => 0.5,
        TileType::Spawner => 0.3,
        TileType::Shrine => 1.5,
        TileType::WindColumn => 4.0,
        TileType::VoidPit => -1.0,
        TileType::StairsUp => 0.8,
        TileType::StairsDown => -0.3,
        _ => 0.1, // floor level
    }
}

fn render_floor_layout(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut events: EventReader<RenderFloorEvent>,
) {
    for event in events.read() {
        let layout = &event.layout;
        let origin = event.origin;
        let size = event.tile_size;

        for (y, row) in layout.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                if *tile == TileType::Empty {
                    continue;
                }

                let height = tile_height(tile);
                let color = tile_color(tile);
                let pos = origin + Vec3::new(x as f32 * size, height / 2.0, y as f32 * size);

                let cube_height = if height > 0.0 { height } else { 0.1 };

                commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(size * 0.95, cube_height, size * 0.95))),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: color,
                        perceptual_roughness: 0.8,
                        ..default()
                    })),
                    Transform::from_translation(pos),
                    FloorTileMarker { floor_id: 0 },
                ));
            }
        }

        info!(
            "Rendered floor: {}x{}, {} rooms",
            layout.width,
            layout.height,
            layout.rooms.len()
        );
    }
}

/// Despawn all tiles for a given floor
pub fn cleanup_floor(
    commands: &mut Commands,
    query: &Query<(Entity, &FloorTileMarker)>,
    floor_id: u32,
) {
    for (entity, marker) in query.iter() {
        if marker.floor_id == floor_id {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_colors_unique() {
        let floor_color = tile_color(&TileType::Floor);
        let wall_color = tile_color(&TileType::Wall);
        let trap_color = tile_color(&TileType::Trap);

        // Colors should be different for gameplay-critical tiles
        assert_ne!(floor_color, wall_color);
        assert_ne!(floor_color, trap_color);
        assert_ne!(wall_color, trap_color);
    }

    #[test]
    fn test_wall_is_tallest_common_tile() {
        assert!(tile_height(&TileType::Wall) > tile_height(&TileType::Floor));
        assert!(tile_height(&TileType::Wall) > tile_height(&TileType::Chest));
        assert!(tile_height(&TileType::Wall) > tile_height(&TileType::Spawner));
    }

    #[test]
    fn test_void_pit_is_below_ground() {
        assert!(tile_height(&TileType::VoidPit) < 0.0);
    }
}
