//! Destruction System — Battlefield-style environmental destruction
//!
//! Server-authoritative destruction with fragment-based damage model.
//! Supports structural integrity (cascading collapse), material resistances,
//! semantic loot drops, and delta-based client synchronization.
//!
//! ## Architecture
//! ```text
//! Client hits object → DestructionDamageRequest (JSON/HTTP)
//!       ↓
//! Server validates → apply_destruction_damage system
//!       ↓
//! Fragment HP reduced → structural_integrity_check
//!       ↓
//! DestructionDelta → broadcast to clients (fragment_mask bitmask)
//!       ↓
//! UE5 Chaos Destruction → visual fracture + physics debris
//! ```

use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// ============================================================================
// Components
// ============================================================================

/// Material type affects damage resistance and fracture behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestructionMaterial {
    Wood,       // Low HP, splinter fracture, weak to fire
    Stone,      // Medium HP, chunk fracture, weak to explosive
    Metal,      // High HP, deformation, weak to lightning
    Crystal,    // Medium HP, Voronoi shatter, weak to kinetic
    Ice,        // Low HP, shatter + melt VFX, weak to fire
    Organic,    // Trees/vines — snap + ragdoll, weak to fire
}

impl DestructionMaterial {
    /// Base HP multiplier for this material
    pub fn hp_multiplier(&self) -> f32 {
        match self {
            Self::Wood => 1.0,
            Self::Stone => 2.5,
            Self::Metal => 4.0,
            Self::Crystal => 1.8,
            Self::Ice => 0.8,
            Self::Organic => 1.2,
        }
    }

    /// Damage resistance per damage type (0.0 = immune, 1.0 = normal, 2.0 = double)
    pub fn damage_modifier(&self, damage_type: DestructionDamageType) -> f32 {
        match (self, damage_type) {
            // Wood: burns easily, resistant to lightning
            (Self::Wood, DestructionDamageType::ElementalFire) => 2.0,
            (Self::Wood, DestructionDamageType::Explosive) => 1.5,
            (Self::Wood, DestructionDamageType::ElementalLightning) => 0.5,

            // Stone: explosives are king, resistant to fire
            (Self::Stone, DestructionDamageType::Explosive) => 2.0,
            (Self::Stone, DestructionDamageType::Kinetic) => 0.6,
            (Self::Stone, DestructionDamageType::ElementalFire) => 0.3,

            // Metal: conducts lightning, resistant to kinetic
            (Self::Metal, DestructionDamageType::ElementalLightning) => 2.0,
            (Self::Metal, DestructionDamageType::Kinetic) => 0.5,
            (Self::Metal, DestructionDamageType::Explosive) => 1.3,

            // Crystal: shatters on impact, resistant to elemental
            (Self::Crystal, DestructionDamageType::Kinetic) => 2.0,
            (Self::Crystal, DestructionDamageType::ElementalFire) => 0.5,
            (Self::Crystal, DestructionDamageType::ElementalIce) => 0.5,

            // Ice: melts in fire, shatters on kinetic
            (Self::Ice, DestructionDamageType::ElementalFire) => 3.0,
            (Self::Ice, DestructionDamageType::Kinetic) => 1.5,
            (Self::Ice, DestructionDamageType::ElementalIce) => 0.0,

            // Organic: burns, resistant to ice
            (Self::Organic, DestructionDamageType::ElementalFire) => 2.5,
            (Self::Organic, DestructionDamageType::ElementalIce) => 0.5,

            // Default: normal damage
            _ => 1.0,
        }
    }
}

/// Damage type for destruction interactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestructionDamageType {
    Kinetic,           // Physical impact
    Explosive,         // Radius damage
    ElementalFire,
    ElementalIce,
    ElementalLightning,
    Semantic,          // Tower corruption damage
}

/// Single fragment within a destructible object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    pub cluster_id: u8,
    pub hp: f32,
    pub max_hp: f32,
    pub destroyed: bool,
    pub position_offset: Vec3,
    /// Structural dependency: if this cluster is destroyed, dependents collapse
    pub supports: Vec<u8>,
}

/// Main component for destructible entities
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Destructible {
    pub entity_id: u64,
    pub template_id: String,
    pub material: DestructionMaterial,
    pub fragments: Vec<Fragment>,
    pub collapsed: bool,
    pub supports_rebuild: bool,
    pub floor_id: u32,
    pub semantic_tags: Vec<(String, f32)>,
    pub rebuild_progress: f32,
}

impl Destructible {
    /// Create a new destructible from a template
    pub fn new(
        entity_id: u64,
        template_id: &str,
        material: DestructionMaterial,
        fragment_count: u8,
        base_hp_per_fragment: f32,
    ) -> Self {
        let hp = base_hp_per_fragment * material.hp_multiplier();
        let fragments = (0..fragment_count)
            .map(|i| Fragment {
                cluster_id: i,
                hp,
                max_hp: hp,
                destroyed: false,
                position_offset: Vec3::ZERO,
                supports: if i == 0 {
                    // Foundation supports everything above it
                    (1..fragment_count).collect()
                } else {
                    vec![]
                },
            })
            .collect();

        Self {
            entity_id,
            template_id: template_id.to_string(),
            material,
            fragments,
            collapsed: false,
            supports_rebuild: true,
            floor_id: 0,
            semantic_tags: vec![],
            rebuild_progress: 0.0,
        }
    }

    /// Total current HP across all fragments
    pub fn total_hp(&self) -> f32 {
        self.fragments.iter().map(|f| f.hp).sum()
    }

    /// Total maximum HP
    pub fn max_total_hp(&self) -> f32 {
        self.fragments.iter().map(|f| f.max_hp).sum()
    }

    /// Generate bitmask of destroyed fragments (1 bit per fragment)
    pub fn fragment_mask(&self) -> Vec<u8> {
        let byte_count = (self.fragments.len() + 7) / 8;
        let mut mask = vec![0u8; byte_count];
        for (i, frag) in self.fragments.iter().enumerate() {
            if frag.destroyed {
                mask[i / 8] |= 1 << (i % 8);
            }
        }
        mask
    }

    /// Count destroyed fragments
    pub fn destroyed_count(&self) -> usize {
        self.fragments.iter().filter(|f| f.destroyed).count()
    }

    /// Apply damage at a specific point with radius
    pub fn apply_damage(
        &mut self,
        impact_point: Vec3,
        entity_position: Vec3,
        damage: f32,
        radius: f32,
        damage_type: DestructionDamageType,
    ) -> DestructionResult {
        if self.collapsed {
            return DestructionResult::default();
        }

        let material_modifier = self.material.damage_modifier(damage_type);
        let effective_damage = damage * material_modifier;

        let mut result = DestructionResult::default();
        result.damage_dealt = effective_damage;

        // Apply damage to fragments within radius
        for fragment in &mut self.fragments {
            if fragment.destroyed {
                continue;
            }

            let frag_world_pos = entity_position + fragment.position_offset;
            let distance = (frag_world_pos - impact_point).length();

            // Point damage (radius=0) hits nearest fragment
            // AoE damage hits all within radius with falloff
            let damage_factor = if radius <= 0.0 {
                // Point damage: full damage to closest non-destroyed fragment
                if distance < 2.0 { 1.0 } else { 0.0 }
            } else {
                // AoE: linear falloff within radius
                if distance <= radius {
                    1.0 - (distance / radius) * 0.5 // 50% falloff at edge
                } else {
                    0.0
                }
            };

            if damage_factor > 0.0 {
                let frag_damage = effective_damage * damage_factor;
                fragment.hp -= frag_damage;

                if fragment.hp <= 0.0 {
                    fragment.hp = 0.0;
                    fragment.destroyed = true;
                    result.newly_destroyed_clusters.push(fragment.cluster_id);
                } else {
                    result.damaged_clusters.push(fragment.cluster_id);
                }
            }
        }

        // Check structural integrity
        let collapse_ids = self.check_structural_integrity();
        for id in &collapse_ids {
            if !result.newly_destroyed_clusters.contains(id) {
                result.newly_destroyed_clusters.push(*id);
            }
        }

        // Check for total collapse (all fragments destroyed or foundation gone)
        let all_destroyed = self.fragments.iter().all(|f| f.destroyed);
        let foundation_destroyed = self.fragments.first().map(|f| f.destroyed).unwrap_or(true);

        if all_destroyed || foundation_destroyed {
            self.collapsed = true;
            // Mark all remaining as destroyed on collapse
            for frag in &mut self.fragments {
                if !frag.destroyed {
                    frag.destroyed = true;
                    result.newly_destroyed_clusters.push(frag.cluster_id);
                }
            }
            result.structural_collapse = true;
        }

        result.fragment_mask = self.fragment_mask();
        result
    }

    /// Check if any unsupported fragments should collapse
    fn check_structural_integrity(&mut self) -> Vec<u8> {
        let mut collapsed_ids = Vec::new();

        // Find destroyed fragments that support others
        let support_map: Vec<(u8, Vec<u8>)> = self.fragments.iter()
            .filter(|f| f.destroyed)
            .map(|f| (f.cluster_id, f.supports.clone()))
            .collect();

        for (_destroyed_id, dependents) in &support_map {
            for &dep_id in dependents {
                if let Some(frag) = self.fragments.iter_mut().find(|f| f.cluster_id == dep_id) {
                    if !frag.destroyed {
                        frag.destroyed = true;
                        frag.hp = 0.0;
                        collapsed_ids.push(dep_id);
                    }
                }
            }
        }

        collapsed_ids
    }

    /// Apply repair progress (returns true if fully repaired)
    pub fn repair(&mut self, repair_amount: f32) -> bool {
        if !self.supports_rebuild || !self.collapsed {
            return false;
        }

        self.rebuild_progress = (self.rebuild_progress + repair_amount).min(1.0);

        if self.rebuild_progress >= 1.0 {
            // Full repair — restore all fragments
            for frag in &mut self.fragments {
                frag.hp = frag.max_hp;
                frag.destroyed = false;
            }
            self.collapsed = false;
            self.rebuild_progress = 0.0;
            return true;
        }

        false
    }
}

/// Result of applying destruction damage
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DestructionResult {
    pub damage_dealt: f32,
    pub newly_destroyed_clusters: Vec<u8>,
    pub damaged_clusters: Vec<u8>,
    pub structural_collapse: bool,
    pub fragment_mask: Vec<u8>,
}

// ============================================================================
// Destructible Templates (predefined object configurations)
// ============================================================================

/// Template for creating destructible objects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructibleTemplate {
    pub id: String,
    pub display_name: String,
    pub material: DestructionMaterial,
    pub fragment_count: u8,
    pub base_hp_per_fragment: f32,
    pub supports_rebuild: bool,
    pub respawn_time_secs: Option<f32>,
    pub loot_table_id: Option<String>,
    pub category: DestructibleCategory,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Category of destructible for generation and LOD decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructibleCategory {
    /// Wall segments — pre-fractured, medium fragment count
    Wall,
    /// Pillars/columns — structural, cascading collapse
    Pillar,
    /// Trees — trunk + branches + foliage
    Tree,
    /// Crates/barrels — simple, 2-5 fragments
    Container,
    /// Crystal formations — Voronoi shatter
    Crystal,
    /// Bridges — sectional destruction
    Bridge,
    /// Tower-specific: corrupted formations
    Corruption,
}

/// Predefined templates for common destructible objects
pub fn default_templates() -> Vec<DestructibleTemplate> {
    vec![
        DestructibleTemplate {
            id: "wall_stone_3m".into(),
            display_name: "Stone Wall (3m)".into(),
            material: DestructionMaterial::Stone,
            fragment_count: 8,
            base_hp_per_fragment: 500.0,
            supports_rebuild: true,
            respawn_time_secs: None, // Player-repaired only
            loot_table_id: Some("loot_stone_debris".into()),
            category: DestructibleCategory::Wall,
            semantic_tags: vec![("structure".into(), 0.9), ("stone".into(), 0.8)],
        },
        DestructibleTemplate {
            id: "wall_wood_3m".into(),
            display_name: "Wooden Wall (3m)".into(),
            material: DestructionMaterial::Wood,
            fragment_count: 6,
            base_hp_per_fragment: 200.0,
            supports_rebuild: true,
            respawn_time_secs: None,
            loot_table_id: Some("loot_wood_debris".into()),
            category: DestructibleCategory::Wall,
            semantic_tags: vec![("structure".into(), 0.7), ("wood".into(), 0.9)],
        },
        DestructibleTemplate {
            id: "pillar_stone".into(),
            display_name: "Stone Pillar".into(),
            material: DestructionMaterial::Stone,
            fragment_count: 4,
            base_hp_per_fragment: 1000.0,
            supports_rebuild: true,
            respawn_time_secs: None,
            loot_table_id: Some("loot_stone_debris".into()),
            category: DestructibleCategory::Pillar,
            semantic_tags: vec![("structure".into(), 1.0), ("stone".into(), 0.8), ("load_bearing".into(), 1.0)],
        },
        DestructibleTemplate {
            id: "tree_forest".into(),
            display_name: "Forest Tree".into(),
            material: DestructionMaterial::Organic,
            fragment_count: 5, // trunk, 3 branch clusters, foliage
            base_hp_per_fragment: 150.0,
            supports_rebuild: false,
            respawn_time_secs: Some(300.0), // 5 min respawn
            loot_table_id: Some("loot_wood_harvest".into()),
            category: DestructibleCategory::Tree,
            semantic_tags: vec![("nature".into(), 0.9), ("wood".into(), 0.7), ("organic".into(), 0.8)],
        },
        DestructibleTemplate {
            id: "tree_corrupted".into(),
            display_name: "Corrupted Tree".into(),
            material: DestructionMaterial::Organic,
            fragment_count: 5,
            base_hp_per_fragment: 250.0,
            supports_rebuild: false,
            respawn_time_secs: Some(600.0), // 10 min
            loot_table_id: Some("loot_corruption_harvest".into()),
            category: DestructibleCategory::Tree,
            semantic_tags: vec![("corruption".into(), 0.9), ("organic".into(), 0.6), ("dark_energy".into(), 0.7)],
        },
        DestructibleTemplate {
            id: "crate_wooden".into(),
            display_name: "Wooden Crate".into(),
            material: DestructionMaterial::Wood,
            fragment_count: 3,
            base_hp_per_fragment: 50.0,
            supports_rebuild: false,
            respawn_time_secs: Some(120.0), // 2 min
            loot_table_id: Some("loot_crate_common".into()),
            category: DestructibleCategory::Container,
            semantic_tags: vec![("container".into(), 0.8), ("wood".into(), 0.5)],
        },
        DestructibleTemplate {
            id: "barrel_metal".into(),
            display_name: "Metal Barrel".into(),
            material: DestructionMaterial::Metal,
            fragment_count: 2,
            base_hp_per_fragment: 100.0,
            supports_rebuild: false,
            respawn_time_secs: Some(120.0),
            loot_table_id: Some("loot_barrel_common".into()),
            category: DestructibleCategory::Container,
            semantic_tags: vec![("container".into(), 0.7), ("metal".into(), 0.8)],
        },
        DestructibleTemplate {
            id: "crystal_cluster".into(),
            display_name: "Crystal Cluster".into(),
            material: DestructionMaterial::Crystal,
            fragment_count: 12,
            base_hp_per_fragment: 100.0,
            supports_rebuild: false,
            respawn_time_secs: None, // Permanent destruction
            loot_table_id: Some("loot_crystal_shards".into()),
            category: DestructibleCategory::Crystal,
            semantic_tags: vec![("crystal".into(), 1.0), ("elemental".into(), 0.7), ("rare_resource".into(), 0.5)],
        },
        DestructibleTemplate {
            id: "bridge_wood_section".into(),
            display_name: "Wooden Bridge Section".into(),
            material: DestructionMaterial::Wood,
            fragment_count: 4,
            base_hp_per_fragment: 300.0,
            supports_rebuild: true,
            respawn_time_secs: None,
            loot_table_id: Some("loot_wood_debris".into()),
            category: DestructibleCategory::Bridge,
            semantic_tags: vec![("structure".into(), 0.8), ("wood".into(), 0.9), ("traversal".into(), 1.0)],
        },
        DestructibleTemplate {
            id: "corruption_node".into(),
            display_name: "Corruption Node".into(),
            material: DestructionMaterial::Crystal,
            fragment_count: 8,
            base_hp_per_fragment: 400.0,
            supports_rebuild: false,
            respawn_time_secs: Some(900.0), // 15 min
            loot_table_id: Some("loot_corruption_essence".into()),
            category: DestructibleCategory::Corruption,
            semantic_tags: vec![("corruption".into(), 1.0), ("dark_energy".into(), 0.9), ("tower_anomaly".into(), 0.8)],
        },
    ]
}

// ============================================================================
// Floor Destruction State (server-side tracking)
// ============================================================================

/// Resource tracking all destructible objects per floor
#[derive(Resource, Default)]
pub struct FloorDestructionManager {
    /// All destructibles indexed by floor_id -> entity_id -> Destructible
    pub floors: HashMap<u32, HashMap<u64, Destructible>>,
    /// Next entity ID for new destructibles
    next_entity_id: u64,
    /// Templates indexed by template_id
    pub templates: HashMap<String, DestructibleTemplate>,
}

impl FloorDestructionManager {
    pub fn new() -> Self {
        let mut mgr = Self::default();
        // Load default templates
        for tmpl in default_templates() {
            mgr.templates.insert(tmpl.id.clone(), tmpl);
        }
        mgr.next_entity_id = 1;
        mgr
    }

    /// Spawn a destructible from template
    pub fn spawn(
        &mut self,
        template_id: &str,
        floor_id: u32,
        _position: Vec3,
    ) -> Option<u64> {
        let template = self.templates.get(template_id)?;

        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;

        let mut destructible = Destructible::new(
            entity_id,
            template_id,
            template.material,
            template.fragment_count,
            template.base_hp_per_fragment,
        );
        destructible.floor_id = floor_id;
        destructible.supports_rebuild = template.supports_rebuild;
        destructible.semantic_tags = template.semantic_tags.clone();

        // Distribute fragment positions based on category
        distribute_fragment_positions(&mut destructible, &template.category);

        self.floors
            .entry(floor_id)
            .or_default()
            .insert(entity_id, destructible);

        Some(entity_id)
    }

    /// Apply damage to a destructible entity
    pub fn apply_damage(
        &mut self,
        entity_id: u64,
        floor_id: u32,
        impact_point: Vec3,
        entity_position: Vec3,
        damage: f32,
        radius: f32,
        damage_type: DestructionDamageType,
    ) -> Option<DestructionResult> {
        let floor = self.floors.get_mut(&floor_id)?;
        let destructible = floor.get_mut(&entity_id)?;
        Some(destructible.apply_damage(impact_point, entity_position, damage, radius, damage_type))
    }

    /// Get destruction stats for a floor
    pub fn floor_stats(&self, floor_id: u32) -> (u32, u32, f32) {
        let floor = match self.floors.get(&floor_id) {
            Some(f) => f,
            None => return (0, 0, 0.0),
        };

        let total = floor.len() as u32;
        let destroyed = floor.values().filter(|d| d.collapsed).count() as u32;
        let percentage = if total > 0 { destroyed as f32 / total as f32 } else { 0.0 };

        (total, destroyed, percentage)
    }
}

/// Distribute fragment positions based on destructible category
fn distribute_fragment_positions(destructible: &mut Destructible, category: &DestructibleCategory) {
    let count = destructible.fragments.len();
    match category {
        DestructibleCategory::Wall => {
            // Grid layout: fragments in a 2D grid
            let cols = (count as f32).sqrt().ceil() as usize;
            for (i, frag) in destructible.fragments.iter_mut().enumerate() {
                let col = i % cols;
                let row = i / cols;
                frag.position_offset = Vec3::new(col as f32 * 1.5, row as f32 * 1.5, 0.0);
            }
        }
        DestructibleCategory::Pillar => {
            // Vertical stack
            for (i, frag) in destructible.fragments.iter_mut().enumerate() {
                frag.position_offset = Vec3::new(0.0, i as f32 * 2.0, 0.0);
                // Each section supports those above
                frag.supports = ((i + 1) as u8..count as u8).collect();
            }
        }
        DestructibleCategory::Tree => {
            // Trunk (0), branches (1-3), foliage (4)
            if count >= 5 {
                destructible.fragments[0].position_offset = Vec3::new(0.0, 0.0, 0.0); // trunk base
                destructible.fragments[1].position_offset = Vec3::new(-1.0, 3.0, 0.0);
                destructible.fragments[2].position_offset = Vec3::new(1.0, 4.0, 0.5);
                destructible.fragments[3].position_offset = Vec3::new(0.0, 5.0, -0.5);
                destructible.fragments[4].position_offset = Vec3::new(0.0, 6.0, 0.0);
                // Trunk supports everything
                destructible.fragments[0].supports = vec![1, 2, 3, 4];
            }
        }
        DestructibleCategory::Container => {
            // Simple radial fragments
            for (i, frag) in destructible.fragments.iter_mut().enumerate() {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                frag.position_offset = Vec3::new(angle.cos() * 0.3, 0.5, angle.sin() * 0.3);
            }
        }
        DestructibleCategory::Crystal => {
            // Voronoi-like cluster spread
            let mut rng = 12345u64; // Deterministic
            for frag in destructible.fragments.iter_mut() {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let x = ((rng >> 33) as f32 / u32::MAX as f32) * 3.0 - 1.5;
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let y = ((rng >> 33) as f32 / u32::MAX as f32) * 2.0;
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let z = ((rng >> 33) as f32 / u32::MAX as f32) * 3.0 - 1.5;
                frag.position_offset = Vec3::new(x, y, z);
            }
        }
        DestructibleCategory::Bridge => {
            // Linear sections along X axis
            for (i, frag) in destructible.fragments.iter_mut().enumerate() {
                frag.position_offset = Vec3::new(i as f32 * 3.0, 0.0, 0.0);
                // Each section only supports adjacent ones
                if i > 0 {
                    frag.supports = vec![];
                }
            }
        }
        DestructibleCategory::Corruption => {
            // Organic tendrils spreading from center
            for (i, frag) in destructible.fragments.iter_mut().enumerate() {
                let angle = (i as f32 / count as f32) * std::f32::consts::TAU;
                let dist = (i as f32 + 1.0) * 0.5;
                frag.position_offset = Vec3::new(
                    angle.cos() * dist,
                    (i as f32) * 0.3,
                    angle.sin() * dist,
                );
            }
        }
    }
}

// ============================================================================
// Bevy Systems
// ============================================================================

/// System: Process pending destruction events
pub fn process_destruction_events(
    mut manager: ResMut<FloorDestructionManager>,
    events: Query<(Entity, &DestructionEvent), Added<DestructionEvent>>,
    positions: Query<&Transform>,
    mut commands: Commands,
) {
    for (event_entity, event) in &events {
        // Get target position from ECS or use default
        let target_pos = positions
            .get(event_entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        let _result = manager.apply_damage(
            event.target_entity_id,
            event.floor_id,
            event.impact_point,
            target_pos,
            event.damage,
            event.radius,
            event.damage_type,
        );

        // Clean up event entity
        commands.entity(event_entity).despawn();
    }
}

/// System: Handle respawning of destroyed destructibles
pub fn respawn_destructibles(
    mut manager: ResMut<FloorDestructionManager>,
    time: Res<Time>,
) {
    let dt = time.delta_secs();

    for floor in manager.floors.values_mut() {
        for destructible in floor.values_mut() {
            if !destructible.collapsed {
                continue;
            }
            // Check if this template has a respawn timer
            // For now, rebuild_progress serves as the respawn timer
            if !destructible.supports_rebuild {
                // Auto-respawn objects accumulate progress
                destructible.rebuild_progress += dt / 300.0; // 5 min default
                if destructible.rebuild_progress >= 1.0 {
                    for frag in &mut destructible.fragments {
                        frag.hp = frag.max_hp;
                        frag.destroyed = false;
                    }
                    destructible.collapsed = false;
                    destructible.rebuild_progress = 0.0;
                }
            }
        }
    }
}

/// Event component for destruction requests
#[derive(Component)]
pub struct DestructionEvent {
    pub target_entity_id: u64,
    pub floor_id: u32,
    pub impact_point: Vec3,
    pub damage: f32,
    pub radius: f32,
    pub damage_type: DestructionDamageType,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destructible_creation() {
        let d = Destructible::new(1, "wall_stone_3m", DestructionMaterial::Stone, 8, 500.0);
        assert_eq!(d.fragments.len(), 8);
        assert_eq!(d.total_hp(), 500.0 * 2.5 * 8.0); // base * stone multiplier * count
        assert!(!d.collapsed);
        assert_eq!(d.destroyed_count(), 0);
    }

    #[test]
    fn test_fragment_mask() {
        let mut d = Destructible::new(1, "test", DestructionMaterial::Wood, 8, 100.0);
        assert_eq!(d.fragment_mask(), vec![0b00000000]);

        d.fragments[0].destroyed = true;
        d.fragments[2].destroyed = true;
        assert_eq!(d.fragment_mask(), vec![0b00000101]);

        d.fragments[7].destroyed = true;
        assert_eq!(d.fragment_mask(), vec![0b10000101]);
    }

    #[test]
    fn test_material_damage_modifiers() {
        // Wood is weak to fire
        assert_eq!(DestructionMaterial::Wood.damage_modifier(DestructionDamageType::ElementalFire), 2.0);
        // Stone is resistant to kinetic
        assert_eq!(DestructionMaterial::Stone.damage_modifier(DestructionDamageType::Kinetic), 0.6);
        // Metal conducts lightning
        assert_eq!(DestructionMaterial::Metal.damage_modifier(DestructionDamageType::ElementalLightning), 2.0);
        // Ice is immune to ice
        assert_eq!(DestructionMaterial::Ice.damage_modifier(DestructionDamageType::ElementalIce), 0.0);
    }

    #[test]
    fn test_point_damage() {
        let mut d = Destructible::new(1, "crate", DestructionMaterial::Wood, 3, 50.0);
        // Position fragment 0 at origin
        d.fragments[0].position_offset = Vec3::ZERO;

        let result = d.apply_damage(
            Vec3::ZERO,       // impact at origin
            Vec3::ZERO,       // entity at origin
            100.0,            // damage
            0.0,              // point damage
            DestructionDamageType::Kinetic,
        );

        assert!(result.damage_dealt > 0.0);
        // Fragment 0 should be destroyed (50 HP < 100 damage)
        assert!(result.newly_destroyed_clusters.contains(&0));
    }

    #[test]
    fn test_aoe_damage() {
        let mut d = Destructible::new(1, "wall", DestructionMaterial::Wood, 4, 50.0);
        // Spread fragments
        d.fragments[0].position_offset = Vec3::new(0.0, 0.0, 0.0);
        d.fragments[1].position_offset = Vec3::new(2.0, 0.0, 0.0);
        d.fragments[2].position_offset = Vec3::new(4.0, 0.0, 0.0);
        d.fragments[3].position_offset = Vec3::new(6.0, 0.0, 0.0);

        let result = d.apply_damage(
            Vec3::new(1.0, 0.0, 0.0), // impact between frag 0 and 1
            Vec3::ZERO,
            200.0,
            3.0, // 3m radius
            DestructionDamageType::Explosive,
        );

        // Explosive does 1.5x to wood
        assert!(result.damage_dealt > 200.0);
        // At least fragments 0 and 1 should take damage
        let total_affected = result.newly_destroyed_clusters.len() + result.damaged_clusters.len();
        assert!(total_affected >= 2);
    }

    #[test]
    fn test_structural_collapse() {
        let mut d = Destructible::new(1, "pillar", DestructionMaterial::Stone, 4, 100.0);
        // Fragment 0 (foundation) supports 1, 2, 3
        d.fragments[0].supports = vec![1, 2, 3];
        d.fragments[0].position_offset = Vec3::ZERO;

        // Destroy foundation with massive damage
        let result = d.apply_damage(
            Vec3::ZERO,
            Vec3::ZERO,
            10000.0, // Overkill
            0.0,
            DestructionDamageType::Explosive,
        );

        assert!(result.structural_collapse);
        assert!(d.collapsed);
        // All fragments should be destroyed
        assert!(d.fragments.iter().all(|f| f.destroyed));
    }

    #[test]
    fn test_fire_vs_wood_bonus() {
        let mut wood = Destructible::new(1, "wall_wood", DestructionMaterial::Wood, 1, 100.0);
        wood.fragments[0].position_offset = Vec3::ZERO;

        let fire_result = wood.apply_damage(
            Vec3::ZERO, Vec3::ZERO,
            50.0, 0.0,
            DestructionDamageType::ElementalFire,
        );

        let mut wood2 = Destructible::new(2, "wall_wood", DestructionMaterial::Wood, 1, 100.0);
        wood2.fragments[0].position_offset = Vec3::ZERO;

        let kinetic_result = wood2.apply_damage(
            Vec3::ZERO, Vec3::ZERO,
            50.0, 0.0,
            DestructionDamageType::Kinetic,
        );

        // Fire should deal 2x damage to wood
        assert!(fire_result.damage_dealt > kinetic_result.damage_dealt);
        assert_eq!(fire_result.damage_dealt, 100.0); // 50 * 2.0
        assert_eq!(kinetic_result.damage_dealt, 50.0); // 50 * 1.0
    }

    #[test]
    fn test_repair() {
        let mut d = Destructible::new(1, "wall", DestructionMaterial::Stone, 4, 100.0);
        d.collapsed = true;
        d.supports_rebuild = true;
        for frag in &mut d.fragments {
            frag.destroyed = true;
            frag.hp = 0.0;
        }

        // Partial repair
        assert!(!d.repair(0.5));
        assert_eq!(d.rebuild_progress, 0.5);

        // Complete repair
        assert!(d.repair(0.5));
        assert!(!d.collapsed);
        assert_eq!(d.rebuild_progress, 0.0);
        assert!(d.fragments.iter().all(|f| !f.destroyed));
        assert!(d.fragments.iter().all(|f| f.hp == f.max_hp));
    }

    #[test]
    fn test_floor_destruction_manager() {
        let mut manager = FloorDestructionManager::new();

        // Spawn destructibles on floor 1
        let id1 = manager.spawn("wall_stone_3m", 1, Vec3::new(0.0, 0.0, 0.0)).unwrap();
        let id2 = manager.spawn("crate_wooden", 1, Vec3::new(5.0, 0.0, 0.0)).unwrap();
        let _id3 = manager.spawn("tree_forest", 1, Vec3::new(10.0, 0.0, 0.0)).unwrap();

        assert_ne!(id1, id2);

        let (total, destroyed, pct) = manager.floor_stats(1);
        assert_eq!(total, 3);
        assert_eq!(destroyed, 0);
        assert_eq!(pct, 0.0);

        // Destroy the crate (low HP)
        let result = manager.apply_damage(
            id2, 1,
            Vec3::new(5.0, 0.5, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            10000.0, // Overkill
            5.0,
            DestructionDamageType::Explosive,
        ).unwrap();

        assert!(result.structural_collapse);

        let (total, destroyed, pct) = manager.floor_stats(1);
        assert_eq!(total, 3);
        assert_eq!(destroyed, 1);
        assert!(pct > 0.0);
    }

    #[test]
    fn test_default_templates() {
        let templates = default_templates();
        assert!(templates.len() >= 10);

        // Verify all templates have valid data
        for t in &templates {
            assert!(!t.id.is_empty());
            assert!(t.fragment_count > 0);
            assert!(t.base_hp_per_fragment > 0.0);
            assert!(!t.semantic_tags.is_empty());
        }
    }

    #[test]
    fn test_ice_immune_to_ice() {
        let mut ice = Destructible::new(1, "ice_wall", DestructionMaterial::Ice, 1, 100.0);
        ice.fragments[0].position_offset = Vec3::ZERO;

        let result = ice.apply_damage(
            Vec3::ZERO, Vec3::ZERO,
            1000.0, 0.0,
            DestructionDamageType::ElementalIce,
        );

        // Ice is immune to ice damage (modifier = 0.0)
        assert_eq!(result.damage_dealt, 0.0);
        assert!(!ice.fragments[0].destroyed);
    }
}
