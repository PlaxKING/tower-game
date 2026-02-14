use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub mod npcs;

pub struct FactionPlugin;

impl Plugin for FactionPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FactionRegistry::default())
            .add_systems(Update, update_faction_standing);
    }
}

/// Four major tower factions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    /// Ascending Order - progress at all costs, competitive
    AscendingOrder,
    /// Deep Dwellers - explore lower floors, knowledge seekers
    DeepDwellers,
    /// Echo Keepers - preserve death echoes, spiritual
    EchoKeepers,
    /// Free Climbers - independent, trade-focused
    FreeClimbers,
}

impl Faction {
    /// Base relationship between factions (-1.0 hostile to 1.0 allied)
    pub fn base_relation(&self, other: &Faction) -> f32 {
        if self == other {
            return 1.0;
        }
        match (self, other) {
            // Ascending Order vs Deep Dwellers: natural tension
            (Faction::AscendingOrder, Faction::DeepDwellers)
            | (Faction::DeepDwellers, Faction::AscendingOrder) => -0.3,

            // Echo Keepers are neutral with most
            (Faction::EchoKeepers, _) | (_, Faction::EchoKeepers) => 0.1,

            // Free Climbers trade with everyone
            (Faction::FreeClimbers, _) | (_, Faction::FreeClimbers) => 0.2,

            _ => 0.0,
        }
    }
}

/// Player's standing with all factions
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct FactionStanding {
    pub ascending_order: f32, // -100 to 100
    pub deep_dwellers: f32,
    pub echo_keepers: f32,
    pub free_climbers: f32,
    pub primary_faction: Option<Faction>,
}

impl Default for FactionStanding {
    fn default() -> Self {
        Self {
            ascending_order: 0.0,
            deep_dwellers: 0.0,
            echo_keepers: 0.0,
            free_climbers: 0.0,
            primary_faction: None,
        }
    }
}

impl FactionStanding {
    pub fn get(&self, faction: &Faction) -> f32 {
        match faction {
            Faction::AscendingOrder => self.ascending_order,
            Faction::DeepDwellers => self.deep_dwellers,
            Faction::EchoKeepers => self.echo_keepers,
            Faction::FreeClimbers => self.free_climbers,
        }
    }

    pub fn modify(&mut self, faction: &Faction, delta: f32) {
        let value = match faction {
            Faction::AscendingOrder => &mut self.ascending_order,
            Faction::DeepDwellers => &mut self.deep_dwellers,
            Faction::EchoKeepers => &mut self.echo_keepers,
            Faction::FreeClimbers => &mut self.free_climbers,
        };
        *value = (*value + delta).clamp(-100.0, 100.0);
    }

    /// Reputation tier based on standing value
    pub fn tier(&self, faction: &Faction) -> ReputationTier {
        let value = self.get(faction);
        match value as i32 {
            -100..=-50 => ReputationTier::Hostile,
            -49..=-10 => ReputationTier::Unfriendly,
            -9..=9 => ReputationTier::Neutral,
            10..=49 => ReputationTier::Friendly,
            50..=89 => ReputationTier::Honored,
            _ => ReputationTier::Exalted,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationTier {
    Hostile,
    Unfriendly,
    Neutral,
    Friendly,
    Honored,
    Exalted,
}

/// Global faction registry with dynamic faction relationships
#[derive(Resource, Debug, Default)]
pub struct FactionRegistry {
    pub world_events_modifier: f32, // shifts all faction relations
}

fn update_faction_standing(mut query: Query<&mut FactionStanding, Changed<FactionStanding>>) {
    for mut standing in &mut query {
        // Auto-detect primary faction (highest standing)
        let factions = [
            (Faction::AscendingOrder, standing.ascending_order),
            (Faction::DeepDwellers, standing.deep_dwellers),
            (Faction::EchoKeepers, standing.echo_keepers),
            (Faction::FreeClimbers, standing.free_climbers),
        ];

        if let Some((faction, value)) = factions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            if *value >= 10.0 {
                standing.primary_faction = Some(*faction);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_faction_self_relation() {
        assert!(
            (Faction::AscendingOrder.base_relation(&Faction::AscendingOrder) - 1.0).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn test_faction_tension() {
        let rel = Faction::AscendingOrder.base_relation(&Faction::DeepDwellers);
        assert!(rel < 0.0, "AO and DD should have negative base relation");
    }

    #[test]
    fn test_standing_modify() {
        let mut standing = FactionStanding::default();
        standing.modify(&Faction::EchoKeepers, 50.0);
        assert!((standing.get(&Faction::EchoKeepers) - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_standing_clamp() {
        let mut standing = FactionStanding::default();
        standing.modify(&Faction::AscendingOrder, 200.0);
        assert!((standing.get(&Faction::AscendingOrder) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_reputation_tiers() {
        let mut standing = FactionStanding::default();
        assert_eq!(
            standing.tier(&Faction::AscendingOrder),
            ReputationTier::Neutral
        );

        standing.modify(&Faction::AscendingOrder, 50.0);
        assert_eq!(
            standing.tier(&Faction::AscendingOrder),
            ReputationTier::Honored
        );

        standing.modify(&Faction::AscendingOrder, -150.0);
        assert_eq!(
            standing.tier(&Faction::AscendingOrder),
            ReputationTier::Hostile
        );
    }
}
