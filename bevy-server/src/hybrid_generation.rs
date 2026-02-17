use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};
use tracing::{info, warn};

use tower_bevy_server::wfc;

/// Hybrid generation: Server sends seed, client generates, server validates
#[derive(Clone, Serialize, Deserialize)]
pub struct FloorGenerationPacket {
    pub floor_id: u32,
    pub seed: u64,

    // Validation hash (SHA3-256 of canonical floor representation)
    pub validation_hash: [u8; 32],

    // Minimal metadata (for client sanity checks)
    pub tile_count: u32,
    pub room_count: u8,
    pub spawner_count: u16,
}

impl FloorGenerationPacket {
    /// Create packet from WFC-generated floor layout
    pub fn from_layout(layout: &wfc::FloorLayout) -> Self {
        let hash = Self::compute_hash(layout);
        let spawner_count = layout.spawn_points.len() as u16;
        let total_tiles = (layout.width * layout.height) as u32;

        Self {
            floor_id: layout.floor_id,
            seed: layout.seed,
            validation_hash: hash,
            tile_count: total_tiles,
            room_count: layout.rooms.len() as u8,
            spawner_count,
        }
    }

    /// Compute canonical SHA3-256 hash of floor layout (deterministic)
    fn compute_hash(layout: &wfc::FloorLayout) -> [u8; 32] {
        let mut hasher = Sha3_256::new();

        // Hash inputs in canonical order
        hasher.update(layout.seed.to_le_bytes());
        hasher.update(layout.floor_id.to_le_bytes());
        hasher.update((layout.width as u32).to_le_bytes());
        hasher.update((layout.height as u32).to_le_bytes());

        // Hash tile data row by row (already in canonical order)
        for (y, row) in layout.tiles.iter().enumerate() {
            for (x, tile) in row.iter().enumerate() {
                let tile: &wfc::TileType = tile;
                hasher.update((x as i32).to_le_bytes());
                hasher.update((y as i32).to_le_bytes());
                hasher.update(tile.to_id().to_le_bytes());
            }
        }

        // Hash room count and types
        hasher.update((layout.rooms.len() as u32).to_le_bytes());
        for room in &layout.rooms {
            hasher.update((room.room_type as u8).to_le_bytes());
        }

        hasher.finalize().into()
    }

    /// Validate client-generated floor matches server hash
    pub fn validate(&self, client_layout: &wfc::FloorLayout) -> ValidationResult {
        let client_hash = Self::compute_hash(client_layout);

        if client_hash == self.validation_hash {
            ValidationResult::Valid
        } else {
            ValidationResult::Invalid {
                expected: self.validation_hash,
                received: client_hash,
                reason: "Hash mismatch - possible cheating or version incompatibility".into(),
            }
        }
    }
}

#[derive(Debug)]
pub enum ValidationResult {
    Valid,
    Invalid {
        expected: [u8; 32],
        received: [u8; 32],
        reason: String,
    },
}

/// Component marking a floor as validated
#[derive(Component)]
pub struct ValidatedFloor {
    pub floor_id: u32,
    pub validation_time: f64,
}

/// Resource holding server-side generated floor packets for validation
#[derive(Resource, Default)]
pub struct FloorValidationCache {
    /// Map of floor_id → validation packet
    pub packets: std::collections::HashMap<u32, FloorGenerationPacket>,
}

/// System: Generate floor server-side and create validation packet
pub fn generate_floor_with_validation(
    mut commands: Commands,
    requests: Query<(Entity, &FloorGenerationRequest), Added<FloorGenerationRequest>>,
    mut cache: ResMut<FloorValidationCache>,
) {
    for (entity, request) in requests.iter() {
        // Generate floor using WFC
        let layout = wfc::generate_layout(request.seed, request.floor_id);
        let packet = FloorGenerationPacket::from_layout(&layout);

        info!(
            "Generated floor {} ({}x{}, {} rooms, hash={:02x}{:02x}{:02x}{:02x}...)",
            request.floor_id,
            layout.width,
            layout.height,
            layout.rooms.len(),
            packet.validation_hash[0],
            packet.validation_hash[1],
            packet.validation_hash[2],
            packet.validation_hash[3],
        );

        // Cache the packet for future validation
        cache.packets.insert(request.floor_id, packet);

        // Remove the request component (processed)
        commands.entity(entity).remove::<FloorGenerationRequest>();
    }
}

/// System: Validate client-submitted floor layouts
pub fn validate_client_floors(
    mut commands: Commands,
    submissions: Query<(Entity, &ClientFloorSubmission)>,
    cache: Res<FloorValidationCache>,
    time: Res<Time>,
) {
    for (entity, submission) in submissions.iter() {
        let floor_id = submission.layout.floor_id;

        if let Some(server_packet) = cache.packets.get(&floor_id) {
            match server_packet.validate(&submission.layout) {
                ValidationResult::Valid => {
                    info!(
                        "Floor {} validated OK (client gen {}ms)",
                        floor_id, submission.generation_time_ms
                    );
                    commands.entity(entity).insert(ValidatedFloor {
                        floor_id,
                        validation_time: time.elapsed_secs_f64(),
                    });
                }
                ValidationResult::Invalid { reason, .. } => {
                    warn!("Floor {} validation FAILED: {}", floor_id, reason);
                    commands.entity(entity).insert(ForceServerFloor {
                        reason: reason.clone(),
                    });
                }
            }
        } else {
            // Server hasn't generated this floor yet — request it
            commands.entity(entity).insert(FloorGenerationRequest {
                seed: submission.layout.seed,
                floor_id,
            });
        }

        // Remove the submission (processed)
        commands.entity(entity).remove::<ClientFloorSubmission>();
    }
}

// ============================================================================
// Supporting types
// ============================================================================

#[derive(Component)]
pub struct FloorGenerationRequest {
    pub seed: u64,
    pub floor_id: u32,
}

#[derive(Component)]
pub struct ClientFloorSubmission {
    pub layout: wfc::FloorLayout,
    pub generation_time_ms: u32,
}

#[derive(Component)]
pub struct ForceServerFloor {
    pub reason: String,
}
