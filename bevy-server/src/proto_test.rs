//! Test module for Protobuf serialization/deserialization

#[cfg(test)]
mod tests {
    use crate::proto::tower::game::*;
    use prost::Message;

    #[test]
    fn test_vec3_serialization() {
        let pos = Vec3 {
            x: 10.0,
            y: 2.0,
            z: 5.0,
        };

        // Serialize
        let mut buf = Vec::new();
        pos.encode(&mut buf).expect("Failed to encode Vec3");

        // Deserialize
        let decoded = Vec3::decode(&buf[..]).expect("Failed to decode Vec3");

        assert_eq!(decoded.x, 10.0);
        assert_eq!(decoded.y, 2.0);
        assert_eq!(decoded.z, 5.0);
    }

    #[test]
    fn test_player_data_creation() {
        let player = PlayerData {
            id: 12345,
            position: Some(Vec3 {
                x: 0.0,
                y: 2.0,
                z: 0.0,
            }),
            velocity: Some(Velocity {
                x: 0.0,
                y: 0.0,
                z: 1.5,
            }),
            rotation: Some(Rotation {
                pitch: 0.0,
                yaw: 0.0,
                roll: 0.0,
            }),
            health: 100.0,
            max_health: 100.0,
            current_floor: 1,
            player_name: "TestPlayer".to_string(),
            level: 1,
            in_combat: false,
            target_entity: 0,
            is_grounded: true,
            is_flying: false,
            is_dashing: false,
        };

        assert_eq!(player.id, 12345);
        assert_eq!(player.health, 100.0);
        assert_eq!(player.player_name, "TestPlayer");
    }

    #[test]
    fn test_world_snapshot_serialization() {
        let snapshot = WorldSnapshot {
            tick: 1000,
            timestamp: 1234567890,
            server_time_ms: 5000,
            players: vec![EntitySnapshot {
                entity_id: 1,
                changed_fields: 0xFF,
                position: Some(Vec3 {
                    x: 10.0,
                    y: 0.0,
                    z: 5.0,
                }),
                velocity: None,
                rotation: None,
                health: Some(85.5),
                state: None,
            }],
            monsters: vec![],
        };

        // Serialize
        let mut buf = Vec::new();
        snapshot
            .encode(&mut buf)
            .expect("Failed to encode WorldSnapshot");

        // Deserialize
        let decoded = WorldSnapshot::decode(&buf[..]).expect("Failed to decode WorldSnapshot");

        assert_eq!(decoded.tick, 1000);
        assert_eq!(decoded.players.len(), 1);
        assert_eq!(decoded.players[0].entity_id, 1);
        assert_eq!(decoded.players[0].health, Some(85.5));
    }

    #[test]
    fn test_chunk_data_with_tiles() {
        let chunk = ChunkData {
            seed: 0x1234567890ABCDEF,
            floor_id: 5,
            tiles: vec![
                FloorTileData {
                    tile_type: 1,
                    grid_x: 0,
                    grid_y: 0,
                    biome_id: 10,
                    is_walkable: true,
                    has_collision: false,
                },
                FloorTileData {
                    tile_type: 2,
                    grid_x: 1,
                    grid_y: 0,
                    biome_id: 10,
                    is_walkable: true,
                    has_collision: true,
                },
            ],
            validation_hash: vec![0xAB, 0xCD, 0xEF, 0x12],
            biome_id: 10,
            width: 50,
            height: 50,
            world_offset: Some(Vec3 {
                x: 0.0,
                y: 100.0,
                z: 0.0,
            }),
            semantic_tags: None,
        };

        // Serialize
        let mut buf = Vec::new();
        chunk.encode(&mut buf).expect("Failed to encode ChunkData");

        println!("ChunkData size: {} bytes", buf.len());

        // Deserialize
        let decoded = ChunkData::decode(&buf[..]).expect("Failed to decode ChunkData");

        assert_eq!(decoded.seed, 0x1234567890ABCDEF);
        assert_eq!(decoded.floor_id, 5);
        assert_eq!(decoded.tiles.len(), 2);
        assert_eq!(decoded.width, 50);
        assert_eq!(decoded.height, 50);
    }

    #[test]
    fn test_procedural_bandwidth_savings() {
        // Simulate full mesh data (500 KB)
        let full_mesh_size = 500_000;

        // Procedural data: seed + tiles + hash
        let chunk = ChunkData {
            seed: 0x1234567890ABCDEF,
            floor_id: 1,
            tiles: vec![FloorTileData::default(); 2500], // 50x50 floor
            validation_hash: vec![0; 32],                // SHA-256
            biome_id: 1,
            width: 50,
            height: 50,
            world_offset: Some(Vec3::default()),
            semantic_tags: None,
        };

        let mut buf = Vec::new();
        chunk.encode(&mut buf).unwrap();
        let procedural_size = buf.len();

        let savings_ratio = full_mesh_size / procedural_size;

        println!("Full mesh size: {} bytes", full_mesh_size);
        println!("Procedural size: {} bytes", procedural_size);
        println!("Savings ratio: {}x", savings_ratio);

        // Should save at least 90x bandwidth (procedural data transfer wins!)
        assert!(
            savings_ratio >= 90,
            "Expected at least 90x savings, got {}x",
            savings_ratio
        );
    }
}
