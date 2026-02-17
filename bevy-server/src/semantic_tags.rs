//! Semantic Tag System
//!
//! The core of the game's procedural content interconnection.
//! Every entity (floor, monster, item, ability) has a semantic tag vector.
//!
//! ## Philosophy
//! "Procedural Semantic Fabric" - all game systems are connected through
//! semantic relationships rather than hardcoded rules.
//!
//! ## Example
//! ```rust,ignore
//! use tower_bevy_server::semantic_tags::{SemanticTags, MasteryDomain};
//!
//! let fire_floor = SemanticTags::from_pairs(vec![
//!     ("fire", 0.9), ("heat", 0.8), ("danger", 0.7),
//! ]);
//! let water_ability = SemanticTags::from_pairs(vec![
//!     ("water", 0.9), ("cold", 0.6),
//! ]);
//! let similarity = fire_floor.similarity(&water_ability);
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic tag with normalized weight (0.0 - 1.0)
///
/// Tags represent abstract concepts: "fire", "exploration", "corruption", etc.
/// Weights indicate intensity: 0.0 = absent, 1.0 = dominant
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SemanticTags {
    /// Tag name -> weight mapping
    /// Stored as Vec for deterministic iteration and Protobuf compatibility
    pub tags: Vec<(String, f32)>,
}

impl SemanticTags {
    /// Create empty tag set
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    /// Create from tag pairs
    ///
    /// # Arguments
    /// * `pairs` - Vec of (tag_name, weight) pairs
    ///
    /// # Example
    /// ```rust,ignore
    /// let tags = SemanticTags::from_pairs(vec![
    ///     ("fire", 0.7),
    ///     ("exploration", 0.9),
    /// ]);
    /// ```
    pub fn from_pairs<S: Into<String>>(pairs: Vec<(S, f32)>) -> Self {
        let tags = pairs
            .into_iter()
            .map(|(name, weight)| (name.into(), weight.clamp(0.0, 1.0)))
            .collect();
        Self { tags }
    }

    /// Add or update a tag
    ///
    /// # Arguments
    /// * `tag` - Tag name
    /// * `weight` - Tag weight (will be clamped to 0.0-1.0)
    pub fn add<S: Into<String>>(&mut self, tag: S, weight: f32) {
        let tag_name = tag.into();
        let clamped_weight = weight.clamp(0.0, 1.0);

        // Update existing or add new
        if let Some(existing) = self.tags.iter_mut().find(|(name, _)| name == &tag_name) {
            existing.1 = clamped_weight;
        } else {
            self.tags.push((tag_name, clamped_weight));
        }
    }

    /// Get tag weight (returns 0.0 if tag not present)
    pub fn get(&self, tag: &str) -> f32 {
        self.tags
            .iter()
            .find(|(name, _)| name == tag)
            .map(|(_, weight)| *weight)
            .unwrap_or(0.0)
    }

    /// Remove a tag
    pub fn remove(&mut self, tag: &str) {
        self.tags.retain(|(name, _)| name != tag);
    }

    /// Compute cosine similarity with another tag set
    ///
    /// Returns value in range [-1.0, 1.0]:
    /// - 1.0: Identical/aligned (fire floor + fire ability)
    /// - 0.0: Orthogonal/neutral (fire floor + exploration ability)
    /// - -1.0: Opposite/conflicting (fire floor + water ability)
    ///
    /// # Algorithm
    /// ```text
    /// similarity = dot_product / (magnitude_a * magnitude_b)
    ///
    /// where:
    ///   dot_product = Σ(a[i] * b[i]) for all shared tags
    ///   magnitude = sqrt(Σ(weight²)) for all tags
    /// ```
    ///
    /// # Example
    /// ```rust,ignore
    /// let fire = SemanticTags::from_pairs(vec![("fire", 0.9)]);
    /// let water = SemanticTags::from_pairs(vec![("water", 0.9)]);
    /// assert!(fire.similarity(&water) < 0.1);  // Orthogonal elements
    /// ```
    pub fn similarity(&self, other: &SemanticTags) -> f32 {
        if self.tags.is_empty() || other.tags.is_empty() {
            return 0.0;
        }

        // Convert to hashmaps for efficient lookup
        let self_map: HashMap<&str, f32> = self.tags.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        let other_map: HashMap<&str, f32> = other.tags.iter().map(|(k, v)| (k.as_str(), *v)).collect();

        // Compute dot product (only for shared tags)
        let mut dot_product = 0.0;
        for (tag, weight_a) in &self_map {
            if let Some(weight_b) = other_map.get(tag) {
                dot_product += weight_a * weight_b;
            }
        }

        // Compute magnitudes
        let magnitude_a: f32 = self.tags.iter().map(|(_, w)| w * w).sum::<f32>().sqrt();
        let magnitude_b: f32 = other.tags.iter().map(|(_, w)| w * w).sum::<f32>().sqrt();

        if magnitude_a == 0.0 || magnitude_b == 0.0 {
            return 0.0;
        }

        dot_product / (magnitude_a * magnitude_b)
    }

    /// Blend two tag sets with weighted average
    ///
    /// # Arguments
    /// * `other` - Other tag set
    /// * `ratio` - Blend ratio (0.0 = all self, 1.0 = all other)
    ///
    /// # Example
    /// ```rust,ignore
    /// let fire = SemanticTags::from_pairs(vec![("fire", 1.0)]);
    /// let ice = SemanticTags::from_pairs(vec![("ice", 1.0)]);
    /// let steam = fire.blend(&ice, 0.5);  // 50% fire + 50% ice
    /// assert_eq!(steam.get("fire"), 0.5);
    /// assert_eq!(steam.get("ice"), 0.5);
    /// ```
    pub fn blend(&self, other: &SemanticTags, ratio: f32) -> SemanticTags {
        let ratio = ratio.clamp(0.0, 1.0);
        let mut result = HashMap::new();

        // Add self tags with (1 - ratio) weight
        for (tag, weight) in &self.tags {
            result.insert(tag.clone(), weight * (1.0 - ratio));
        }

        // Add other tags with ratio weight
        for (tag, weight) in &other.tags {
            *result.entry(tag.clone()).or_insert(0.0) += weight * ratio;
        }

        // Convert back to Vec and clamp
        let tags = result
            .into_iter()
            .map(|(name, weight)| (name, weight.clamp(0.0, 1.0)))
            .collect();

        SemanticTags { tags }
    }

    /// Normalize tag weights to sum to 1.0 (probability distribution)
    pub fn normalize(&mut self) {
        let sum: f32 = self.tags.iter().map(|(_, w)| w).sum();
        if sum > 0.0 {
            for (_, weight) in &mut self.tags {
                *weight /= sum;
            }
        }
    }

    /// Get magnitude (Euclidean norm)
    pub fn magnitude(&self) -> f32 {
        self.tags.iter().map(|(_, w)| w * w).sum::<f32>().sqrt()
    }

    /// Check if tag set is empty
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty()
    }

    /// Get number of tags
    pub fn len(&self) -> usize {
        self.tags.len()
    }

    /// Create tags for a specific mastery domain
    pub fn from_domain(domain: MasteryDomain) -> Self {
        domain.to_tags()
    }
}

impl Default for SemanticTags {
    fn default() -> Self {
        Self::new()
    }
}

/// Mastery Domain - Categories for skill progression
///
/// Each domain represents a playstyle aspect.
/// Players gain XP in domains by performing related actions.
///
/// ## 21 Domains (CLAUDE.md)
/// - Weapon Mastery (7): Sword, Axe, Spear, Bow, Staff, Fist, Dual
/// - Combat Techniques (5): Parry, Dodge, Counter, Combo, Positioning
/// - Crafting (3): Smithing, Alchemy, Cooking
/// - Gathering (3): Mining, Herbalism, Logging
/// - Other (3): Exploration, Corruption Resistance, Social
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MasteryDomain {
    // Weapon Mastery (7 domains)
    SwordMastery,
    AxeMastery,
    SpearMastery,
    BowMastery,
    StaffMastery,
    FistMastery,
    DualWieldMastery,

    // Combat Techniques (5 domains)
    ParryMastery,
    DodgeMastery,
    CounterMastery,
    ComboMastery,
    PositioningMastery,

    // Crafting (3 domains)
    SmithingMastery,
    AlchemyMastery,
    CookingMastery,

    // Gathering (3 domains)
    MiningMastery,
    HerbalismMastery,
    LoggingMastery,

    // Other (3 domains)
    ExplorationMastery,
    CorruptionResistance,
    SocialMastery,
}

impl MasteryDomain {
    /// Get all 21 mastery domains
    pub fn all() -> Vec<MasteryDomain> {
        vec![
            // Weapons
            MasteryDomain::SwordMastery,
            MasteryDomain::AxeMastery,
            MasteryDomain::SpearMastery,
            MasteryDomain::BowMastery,
            MasteryDomain::StaffMastery,
            MasteryDomain::FistMastery,
            MasteryDomain::DualWieldMastery,
            // Combat
            MasteryDomain::ParryMastery,
            MasteryDomain::DodgeMastery,
            MasteryDomain::CounterMastery,
            MasteryDomain::ComboMastery,
            MasteryDomain::PositioningMastery,
            // Crafting
            MasteryDomain::SmithingMastery,
            MasteryDomain::AlchemyMastery,
            MasteryDomain::CookingMastery,
            // Gathering
            MasteryDomain::MiningMastery,
            MasteryDomain::HerbalismMastery,
            MasteryDomain::LoggingMastery,
            // Other
            MasteryDomain::ExplorationMastery,
            MasteryDomain::CorruptionResistance,
            MasteryDomain::SocialMastery,
        ]
    }

    /// Convert domain to semantic tags
    ///
    /// Each domain has associated semantic tags that define its "flavor"
    pub fn to_tags(self) -> SemanticTags {
        use MasteryDomain::*;

        let pairs = match self {
            // Weapons
            SwordMastery => vec![("melee", 0.9), ("slashing", 0.8), ("versatile", 0.6)],
            AxeMastery => vec![("melee", 0.9), ("slashing", 0.7), ("heavy", 0.8)],
            SpearMastery => vec![("melee", 0.8), ("piercing", 0.9), ("reach", 0.7)],
            BowMastery => vec![("ranged", 0.9), ("piercing", 0.8), ("precision", 0.7)],
            StaffMastery => vec![("melee", 0.6), ("magic", 0.8), ("elemental", 0.7)],
            FistMastery => vec![("melee", 1.0), ("bludgeoning", 0.6), ("speed", 0.8)],
            DualWieldMastery => vec![("melee", 0.9), ("speed", 0.9), ("complexity", 0.7)],

            // Combat Techniques
            ParryMastery => vec![("defense", 0.9), ("timing", 1.0), ("skill", 0.8)],
            DodgeMastery => vec![("defense", 0.8), ("mobility", 0.9), ("positioning", 0.7)],
            CounterMastery => vec![("offense", 0.8), ("timing", 0.9), ("punish", 0.8)],
            ComboMastery => vec![("offense", 0.9), ("complexity", 0.8), ("rhythm", 0.7)],
            PositioningMastery => vec![("tactical", 0.9), ("spatial", 0.8), ("awareness", 0.7)],

            // Crafting
            SmithingMastery => vec![("crafting", 1.0), ("fire", 0.6), ("metal", 0.9)],
            AlchemyMastery => vec![("crafting", 1.0), ("magic", 0.7), ("chemical", 0.8)],
            CookingMastery => vec![("crafting", 1.0), ("fire", 0.5), ("restoration", 0.7)],

            // Gathering
            MiningMastery => vec![("gathering", 1.0), ("earth", 0.8), ("metal", 0.7)],
            HerbalismMastery => vec![("gathering", 1.0), ("nature", 0.9), ("restoration", 0.6)],
            LoggingMastery => vec![("gathering", 1.0), ("nature", 0.8), ("wood", 0.9)],

            // Other
            ExplorationMastery => vec![("exploration", 1.0), ("discovery", 0.9), ("mobility", 0.6)],
            CorruptionResistance => vec![("defense", 0.7), ("mental", 0.8), ("corruption", -0.9)],
            SocialMastery => vec![("social", 1.0), ("charisma", 0.8), ("trading", 0.6)],
        };

        SemanticTags::from_pairs(pairs)
    }

    /// Get domain name as string
    pub fn name(&self) -> &str {
        use MasteryDomain::*;
        match self {
            SwordMastery => "Sword Mastery",
            AxeMastery => "Axe Mastery",
            SpearMastery => "Spear Mastery",
            BowMastery => "Bow Mastery",
            StaffMastery => "Staff Mastery",
            FistMastery => "Fist Mastery",
            DualWieldMastery => "Dual Wield Mastery",
            ParryMastery => "Parry Mastery",
            DodgeMastery => "Dodge Mastery",
            CounterMastery => "Counter Mastery",
            ComboMastery => "Combo Mastery",
            PositioningMastery => "Positioning Mastery",
            SmithingMastery => "Smithing Mastery",
            AlchemyMastery => "Alchemy Mastery",
            CookingMastery => "Cooking Mastery",
            MiningMastery => "Mining Mastery",
            HerbalismMastery => "Herbalism Mastery",
            LoggingMastery => "Logging Mastery",
            ExplorationMastery => "Exploration Mastery",
            CorruptionResistance => "Corruption Resistance",
            SocialMastery => "Social Mastery",
        }
    }

    /// Get domain category
    pub fn category(&self) -> DomainCategory {
        use MasteryDomain::*;
        match self {
            SwordMastery | AxeMastery | SpearMastery | BowMastery | StaffMastery | FistMastery | DualWieldMastery => {
                DomainCategory::Weapon
            }
            ParryMastery | DodgeMastery | CounterMastery | ComboMastery | PositioningMastery => {
                DomainCategory::Combat
            }
            SmithingMastery | AlchemyMastery | CookingMastery => DomainCategory::Crafting,
            MiningMastery | HerbalismMastery | LoggingMastery => DomainCategory::Gathering,
            ExplorationMastery | CorruptionResistance | SocialMastery => DomainCategory::Other,
        }
    }
}

/// Domain category for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    Weapon,
    Combat,
    Crafting,
    Gathering,
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_tags_creation() {
        let tags = SemanticTags::from_pairs(vec![("fire", 0.7), ("heat", 0.5)]);
        assert_eq!(tags.get("fire"), 0.7);
        assert_eq!(tags.get("heat"), 0.5);
        assert_eq!(tags.get("missing"), 0.0);
    }

    #[test]
    fn test_add_tag() {
        let mut tags = SemanticTags::new();
        tags.add("fire", 0.8);
        assert_eq!(tags.get("fire"), 0.8);

        // Update existing
        tags.add("fire", 0.9);
        assert_eq!(tags.get("fire"), 0.9);
    }

    #[test]
    fn test_weight_clamping() {
        let tags = SemanticTags::from_pairs(vec![("overflow", 1.5), ("underflow", -0.5)]);
        assert_eq!(tags.get("overflow"), 1.0);
        assert_eq!(tags.get("underflow"), 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let tags1 = SemanticTags::from_pairs(vec![("fire", 1.0)]);
        let tags2 = SemanticTags::from_pairs(vec![("fire", 1.0)]);
        assert!((tags1.similarity(&tags2) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let tags1 = SemanticTags::from_pairs(vec![("fire", 1.0)]);
        let tags2 = SemanticTags::from_pairs(vec![("water", 1.0)]);
        // No shared tags = 0 dot product = 0 similarity
        assert!((tags1.similarity(&tags2) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_partial() {
        let tags1 = SemanticTags::from_pairs(vec![("fire", 0.8), ("heat", 0.6)]);
        let tags2 = SemanticTags::from_pairs(vec![("fire", 0.6), ("danger", 0.8)]);

        let sim = tags1.similarity(&tags2);
        // Should have positive similarity due to shared "fire" tag
        assert!(sim > 0.0);
        assert!(sim < 1.0);
    }

    #[test]
    fn test_blend() {
        let fire = SemanticTags::from_pairs(vec![("fire", 1.0)]);
        let ice = SemanticTags::from_pairs(vec![("ice", 1.0)]);

        let blend = fire.blend(&ice, 0.5);
        assert_eq!(blend.get("fire"), 0.5);
        assert_eq!(blend.get("ice"), 0.5);
    }

    #[test]
    fn test_normalize() {
        let mut tags = SemanticTags::from_pairs(vec![("a", 2.0), ("b", 2.0)]);
        tags.normalize();

        let sum: f32 = tags.tags.iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_magnitude() {
        let tags = SemanticTags::from_pairs(vec![("a", 3.0), ("b", 4.0)]);
        // sqrt(3^2 + 4^2) = sqrt(25) = 5.0 (clamped: 1.0^2 + 1.0^2 = sqrt(2))
        let mag = tags.magnitude();
        assert!((mag - 1.414).abs() < 0.01); // sqrt(2)
    }

    #[test]
    fn test_mastery_domain_count() {
        assert_eq!(MasteryDomain::all().len(), 21);
    }

    #[test]
    fn test_mastery_domain_tags() {
        let sword_tags = MasteryDomain::SwordMastery.to_tags();
        assert!(sword_tags.get("melee") > 0.0);
        assert!(sword_tags.get("slashing") > 0.0);
    }

    #[test]
    fn test_domain_categories() {
        assert_eq!(MasteryDomain::SwordMastery.category(), DomainCategory::Weapon);
        assert_eq!(MasteryDomain::ParryMastery.category(), DomainCategory::Combat);
        assert_eq!(MasteryDomain::SmithingMastery.category(), DomainCategory::Crafting);
        assert_eq!(MasteryDomain::MiningMastery.category(), DomainCategory::Gathering);
        assert_eq!(MasteryDomain::ExplorationMastery.category(), DomainCategory::Other);
    }

    #[test]
    fn test_domain_similarity() {
        let sword = MasteryDomain::SwordMastery.to_tags();
        let axe = MasteryDomain::AxeMastery.to_tags();
        let bow = MasteryDomain::BowMastery.to_tags();

        // Sword and axe should be similar (both melee)
        let melee_sim = sword.similarity(&axe);
        // Sword and bow should be less similar (melee vs ranged)
        let ranged_sim = sword.similarity(&bow);

        assert!(melee_sim > ranged_sim);
    }
}
