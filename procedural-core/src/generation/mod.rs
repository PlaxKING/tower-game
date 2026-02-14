pub mod floor_manager;
pub mod wfc;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::semantic::SemanticTags;

pub struct GenerationPlugin;

impl Plugin for GenerationPlugin {
    fn build(&self, app: &mut App) {
        let progress = floor_manager::TowerProgress::new(42);
        app.insert_resource(TowerSeed::default())
            .insert_resource(progress)
            .add_event::<floor_manager::FloorTransitionEvent>()
            .add_event::<floor_manager::FloorReadyEvent>()
            .add_systems(
                Update,
                (
                    floor_manager::detect_stair_interaction,
                    floor_manager::handle_floor_transitions,
                )
                    .chain(),
            );
    }
}

/// Global tower seed - the root of all procedural generation.
/// 1000 floors = this seed (8 bytes) + ~50KB mutations.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct TowerSeed {
    pub seed: u64,
}

impl Default for TowerSeed {
    fn default() -> Self {
        Self { seed: 42 }
    }
}

impl TowerSeed {
    /// Deterministic floor hash from tower seed and floor id
    pub fn floor_hash(&self, floor_id: u32) -> u64 {
        let mut hasher = Sha3_256::new();
        hasher.update(self.seed.to_le_bytes());
        hasher.update(floor_id.to_le_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[0..8].try_into().unwrap())
    }
}

/// Floor tier determines difficulty and mechanics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FloorTier {
    Echelon1, // 1-100: tutorial, low risk
    Echelon2, // 101-300: strategic, medium risk
    Echelon3, // 301-500: tactical depth, high risk
    Echelon4, // 501+: architects, voting rights
}

impl FloorTier {
    pub fn from_floor_id(id: u32) -> Self {
        match id {
            1..=100 => Self::Echelon1,
            101..=300 => Self::Echelon2,
            301..=500 => Self::Echelon3,
            _ => Self::Echelon4,
        }
    }
}

/// Definition of a generated floor (before spawning into ECS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorSpec {
    pub id: u32,
    pub tier: FloorTier,
    pub hash: u64,
    pub biome_tags: SemanticTags,
}

impl FloorSpec {
    pub fn generate(seed: &TowerSeed, floor_id: u32) -> Self {
        let hash = seed.floor_hash(floor_id);
        let tier = FloorTier::from_floor_id(floor_id);

        // Deterministic biome from hash bits
        let fire = (hash & 0xFF) as f32 / 255.0;
        let water = ((hash >> 8) & 0xFF) as f32 / 255.0;
        let corruption = ((hash >> 16) & 0xFF) as f32 / 255.0;
        let exploration = ((hash >> 24) & 0xFF) as f32 / 255.0;

        let biome_tags = SemanticTags::new(vec![
            ("fire", fire),
            ("water", water),
            ("corruption", corruption),
            ("exploration", exploration),
        ]);

        Self {
            id: floor_id,
            tier,
            hash,
            biome_tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_generation() {
        let seed = TowerSeed { seed: 12345 };
        let floor_a = FloorSpec::generate(&seed, 1);
        let floor_b = FloorSpec::generate(&seed, 1);
        assert_eq!(
            floor_a.hash, floor_b.hash,
            "Same seed+floor must produce same hash"
        );
    }

    #[test]
    fn test_different_floors_differ() {
        let seed = TowerSeed { seed: 12345 };
        let floor_1 = FloorSpec::generate(&seed, 1);
        let floor_2 = FloorSpec::generate(&seed, 2);
        assert_ne!(
            floor_1.hash, floor_2.hash,
            "Different floors must produce different hashes"
        );
    }

    #[test]
    fn test_floor_tiers() {
        assert_eq!(FloorTier::from_floor_id(50), FloorTier::Echelon1);
        assert_eq!(FloorTier::from_floor_id(200), FloorTier::Echelon2);
        assert_eq!(FloorTier::from_floor_id(400), FloorTier::Echelon3);
        assert_eq!(FloorTier::from_floor_id(600), FloorTier::Echelon4);
    }
}
