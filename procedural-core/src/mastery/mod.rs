//! Skill Mastery System (replaces traditional XP/levels)
//!
//! From dopopensource.txt Category 2:
//! "система прогресса — повышение мастерства навыков, боевых, ремесленных и других"
//! "характеристики распределяются ТОЛЬКО при создании персонажа"
//! "доп. характеристики только от снаряжения"
//!
//! Progression is through USE:
//! - Swing a sword → Sword Mastery XP
//! - Parry attacks → Defense Mastery XP
//! - Craft items → Crafting Mastery XP
//! - Each mastery has its own skill tree with unlockable nodes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mastery domain — top-level skill categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MasteryDomain {
    // Combat masteries (per weapon)
    SwordMastery,
    GreatswordMastery,
    DaggerMastery,
    SpearMastery,
    GauntletMastery,
    StaffMastery,
    // Combat technique masteries
    ParryMastery,
    DodgeMastery,
    BlockMastery,
    AerialMastery,
    // Crafting masteries
    Blacksmithing,
    Alchemy,
    Enchanting,
    Tailoring,
    Cooking,
    // Gathering
    Mining,
    Herbalism,
    Salvaging,
    // Social / Other
    Trading,
    Exploration,
    SemanticAttunement,
}

impl MasteryDomain {
    pub fn display_name(&self) -> &str {
        match self {
            Self::SwordMastery => "Sword Mastery",
            Self::GreatswordMastery => "Greatsword Mastery",
            Self::DaggerMastery => "Dagger Mastery",
            Self::SpearMastery => "Spear Mastery",
            Self::GauntletMastery => "Gauntlet Mastery",
            Self::StaffMastery => "Staff Mastery",
            Self::ParryMastery => "Parry Mastery",
            Self::DodgeMastery => "Dodge Mastery",
            Self::BlockMastery => "Block Mastery",
            Self::AerialMastery => "Aerial Mastery",
            Self::Blacksmithing => "Blacksmithing",
            Self::Alchemy => "Alchemy",
            Self::Enchanting => "Enchanting",
            Self::Tailoring => "Tailoring",
            Self::Cooking => "Cooking",
            Self::Mining => "Mining",
            Self::Herbalism => "Herbalism",
            Self::Salvaging => "Salvaging",
            Self::Trading => "Trading",
            Self::Exploration => "Exploration",
            Self::SemanticAttunement => "Semantic Attunement",
        }
    }

    pub fn category(&self) -> MasteryCategory {
        match self {
            Self::SwordMastery
            | Self::GreatswordMastery
            | Self::DaggerMastery
            | Self::SpearMastery
            | Self::GauntletMastery
            | Self::StaffMastery => MasteryCategory::Weapon,
            Self::ParryMastery | Self::DodgeMastery | Self::BlockMastery | Self::AerialMastery => {
                MasteryCategory::CombatTechnique
            }
            Self::Blacksmithing
            | Self::Alchemy
            | Self::Enchanting
            | Self::Tailoring
            | Self::Cooking => MasteryCategory::Crafting,
            Self::Mining | Self::Herbalism | Self::Salvaging => MasteryCategory::Gathering,
            Self::Trading | Self::Exploration | Self::SemanticAttunement => MasteryCategory::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MasteryCategory {
    Weapon,
    CombatTechnique,
    Crafting,
    Gathering,
    Other,
}

/// Mastery tier (determines available skill tree depth)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MasteryTier {
    Novice,      // 0-99
    Apprentice,  // 100-499
    Journeyman,  // 500-1499
    Expert,      // 1500-3999
    Master,      // 4000-7999
    Grandmaster, // 8000+
}

impl MasteryTier {
    pub fn from_xp(xp: u64) -> Self {
        match xp {
            0..=99 => Self::Novice,
            100..=499 => Self::Apprentice,
            500..=1499 => Self::Journeyman,
            1500..=3999 => Self::Expert,
            4000..=7999 => Self::Master,
            _ => Self::Grandmaster,
        }
    }

    pub fn xp_threshold(&self) -> u64 {
        match self {
            Self::Novice => 0,
            Self::Apprentice => 100,
            Self::Journeyman => 500,
            Self::Expert => 1500,
            Self::Master => 4000,
            Self::Grandmaster => 8000,
        }
    }

    pub fn next_tier(&self) -> Option<Self> {
        match self {
            Self::Novice => Some(Self::Apprentice),
            Self::Apprentice => Some(Self::Journeyman),
            Self::Journeyman => Some(Self::Expert),
            Self::Expert => Some(Self::Master),
            Self::Master => Some(Self::Grandmaster),
            Self::Grandmaster => None,
        }
    }
}

/// Single mastery progress tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryProgress {
    pub domain: MasteryDomain,
    pub xp: u64,
    pub tier: MasteryTier,
    pub unlocked_nodes: Vec<String>,
}

impl MasteryProgress {
    pub fn new(domain: MasteryDomain) -> Self {
        Self {
            domain,
            xp: 0,
            tier: MasteryTier::Novice,
            unlocked_nodes: Vec::new(),
        }
    }

    /// Add XP from using this skill. Returns true if tier changed.
    pub fn add_xp(&mut self, amount: u64) -> bool {
        let old_tier = self.tier;
        self.xp += amount;
        self.tier = MasteryTier::from_xp(self.xp);
        self.tier != old_tier
    }

    /// Progress toward next tier (0.0 - 1.0)
    pub fn tier_progress(&self) -> f32 {
        if let Some(next) = self.tier.next_tier() {
            let current_base = self.tier.xp_threshold();
            let next_base = next.xp_threshold();
            let range = next_base - current_base;
            let progress = self.xp - current_base;
            (progress as f32 / range as f32).min(1.0)
        } else {
            1.0 // Grandmaster — capped
        }
    }

    /// Check if a skill tree node can be unlocked
    pub fn can_unlock(&self, node: &SkillTreeNode) -> bool {
        if self.tier < node.required_tier {
            return false;
        }
        // Check prerequisites
        for prereq in &node.prerequisites {
            if !self.unlocked_nodes.contains(prereq) {
                return false;
            }
        }
        true
    }

    /// Unlock a skill tree node
    pub fn unlock_node(&mut self, node: &SkillTreeNode) -> bool {
        if !self.can_unlock(node) {
            return false;
        }
        if self.unlocked_nodes.contains(&node.id) {
            return false; // already unlocked
        }
        self.unlocked_nodes.push(node.id.clone());
        true
    }

    pub fn has_node(&self, node_id: &str) -> bool {
        self.unlocked_nodes.iter().any(|n| n == node_id)
    }
}

/// A node in the mastery skill tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTreeNode {
    pub id: String,
    pub name: String,
    pub description: String,
    pub domain: MasteryDomain,
    pub required_tier: MasteryTier,
    pub prerequisites: Vec<String>,
    pub effects: Vec<SkillEffect>,
}

/// Effects granted by unlocking a skill tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SkillEffect {
    /// Increase damage with this weapon type by %
    DamageBonus(f32),
    /// Reduce resource cost by %
    ResourceCostReduction(f32),
    /// Increase combo length by N
    ComboExtension(u32),
    /// Add special attack move
    UnlockAbility(String),
    /// Faster attack speed by %
    AttackSpeedBonus(f32),
    /// Crafting quality bonus %
    CraftingQualityBonus(f32),
    /// Gathering yield bonus %
    GatheringYieldBonus(f32),
    /// Unlock new crafting recipes
    UnlockRecipe(String),
    /// Better parry window (ms)
    ParryWindowExtension(f32),
    /// Dodge i-frame extension (ms)
    DodgeIFrameExtension(f32),
    /// Aerial stamina efficiency %
    AerialEfficiency(f32),
    /// Semantic tag affinity boost
    SemanticAffinity { tag: String, bonus: f32 },
    /// Trading discount %
    TradeDiscount(f32),
    /// Exploration detection radius bonus
    ExplorationRadius(f32),
}

/// Player's complete mastery profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasteryProfile {
    pub masteries: HashMap<MasteryDomain, MasteryProgress>,
}

impl Default for MasteryProfile {
    fn default() -> Self {
        Self::new()
    }
}

impl MasteryProfile {
    pub fn new() -> Self {
        let mut masteries = HashMap::new();
        // Initialize all domains at Novice
        let all_domains = [
            MasteryDomain::SwordMastery,
            MasteryDomain::GreatswordMastery,
            MasteryDomain::DaggerMastery,
            MasteryDomain::SpearMastery,
            MasteryDomain::GauntletMastery,
            MasteryDomain::StaffMastery,
            MasteryDomain::ParryMastery,
            MasteryDomain::DodgeMastery,
            MasteryDomain::BlockMastery,
            MasteryDomain::AerialMastery,
            MasteryDomain::Blacksmithing,
            MasteryDomain::Alchemy,
            MasteryDomain::Enchanting,
            MasteryDomain::Tailoring,
            MasteryDomain::Cooking,
            MasteryDomain::Mining,
            MasteryDomain::Herbalism,
            MasteryDomain::Salvaging,
            MasteryDomain::Trading,
            MasteryDomain::Exploration,
            MasteryDomain::SemanticAttunement,
        ];
        for domain in all_domains {
            masteries.insert(domain, MasteryProgress::new(domain));
        }
        Self { masteries }
    }

    /// Add XP to a specific domain. Returns true if tier changed.
    pub fn gain_xp(&mut self, domain: MasteryDomain, amount: u64) -> bool {
        if let Some(progress) = self.masteries.get_mut(&domain) {
            progress.add_xp(amount)
        } else {
            false
        }
    }

    /// Get mastery for a domain
    pub fn get(&self, domain: MasteryDomain) -> Option<&MasteryProgress> {
        self.masteries.get(&domain)
    }

    /// Get tier for a domain
    pub fn tier(&self, domain: MasteryDomain) -> MasteryTier {
        self.masteries
            .get(&domain)
            .map(|m| m.tier)
            .unwrap_or(MasteryTier::Novice)
    }

    /// Get total mastery XP across all domains
    pub fn total_xp(&self) -> u64 {
        self.masteries.values().map(|m| m.xp).sum()
    }

    /// Count how many domains are at a given tier or above
    pub fn domains_at_tier(&self, tier: MasteryTier) -> usize {
        self.masteries.values().filter(|m| m.tier >= tier).count()
    }

    /// Collect all active skill effects from unlocked nodes
    pub fn active_effects(&self, tree: &SkillTree) -> Vec<SkillEffect> {
        let mut effects = Vec::new();
        for progress in self.masteries.values() {
            for node_id in &progress.unlocked_nodes {
                if let Some(node) = tree.get_node(node_id) {
                    effects.extend(node.effects.clone());
                }
            }
        }
        effects
    }

    /// Serialize to JSON for Nakama storage
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Complete skill tree definition (all domains)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTree {
    pub nodes: Vec<SkillTreeNode>,
}

impl Default for SkillTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillTree {
    #[allow(clippy::vec_init_then_push)]
    pub fn new() -> Self {
        let mut nodes = Vec::new();

        // === Sword Mastery Tree ===
        nodes.push(SkillTreeNode {
            id: "sword_keen_edge".into(),
            name: "Keen Edge".into(),
            description: "Sword attacks deal 10% more damage.".into(),
            domain: MasteryDomain::SwordMastery,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::DamageBonus(0.10)],
        });
        nodes.push(SkillTreeNode {
            id: "sword_swift_combo".into(),
            name: "Swift Combo".into(),
            description: "Sword combo attacks are 15% faster.".into(),
            domain: MasteryDomain::SwordMastery,
            required_tier: MasteryTier::Journeyman,
            prerequisites: vec!["sword_keen_edge".into()],
            effects: vec![SkillEffect::AttackSpeedBonus(0.15)],
        });
        nodes.push(SkillTreeNode {
            id: "sword_extended_chain".into(),
            name: "Extended Chain".into(),
            description: "Add a 4th hit to sword combos.".into(),
            domain: MasteryDomain::SwordMastery,
            required_tier: MasteryTier::Expert,
            prerequisites: vec!["sword_swift_combo".into()],
            effects: vec![SkillEffect::ComboExtension(1)],
        });
        nodes.push(SkillTreeNode {
            id: "sword_rising_slash".into(),
            name: "Rising Slash".into(),
            description: "Unlock aerial launcher attack.".into(),
            domain: MasteryDomain::SwordMastery,
            required_tier: MasteryTier::Master,
            prerequisites: vec!["sword_extended_chain".into()],
            effects: vec![SkillEffect::UnlockAbility("Rising Slash".into())],
        });

        // === Parry Mastery Tree ===
        nodes.push(SkillTreeNode {
            id: "parry_timing".into(),
            name: "Practiced Timing".into(),
            description: "Extend parry window by 30ms.".into(),
            domain: MasteryDomain::ParryMastery,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::ParryWindowExtension(30.0)],
        });
        nodes.push(SkillTreeNode {
            id: "parry_counter".into(),
            name: "Riposte".into(),
            description: "Perfect parry unlocks counterattack.".into(),
            domain: MasteryDomain::ParryMastery,
            required_tier: MasteryTier::Expert,
            prerequisites: vec!["parry_timing".into()],
            effects: vec![SkillEffect::UnlockAbility("Riposte".into())],
        });

        // === Dodge Mastery Tree ===
        nodes.push(SkillTreeNode {
            id: "dodge_extended".into(),
            name: "Lingering Shadow".into(),
            description: "Extend dodge i-frames by 50ms.".into(),
            domain: MasteryDomain::DodgeMastery,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::DodgeIFrameExtension(50.0)],
        });
        nodes.push(SkillTreeNode {
            id: "dodge_efficient".into(),
            name: "Efficient Movement".into(),
            description: "Dodge costs 20% less kinetic energy.".into(),
            domain: MasteryDomain::DodgeMastery,
            required_tier: MasteryTier::Journeyman,
            prerequisites: vec!["dodge_extended".into()],
            effects: vec![SkillEffect::ResourceCostReduction(0.20)],
        });

        // === Blacksmithing Tree ===
        nodes.push(SkillTreeNode {
            id: "smith_quality".into(),
            name: "Quality Materials".into(),
            description: "Crafted weapons gain +15% quality.".into(),
            domain: MasteryDomain::Blacksmithing,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::CraftingQualityBonus(0.15)],
        });
        nodes.push(SkillTreeNode {
            id: "smith_temper".into(),
            name: "Tempering".into(),
            description: "Unlock weapon tempering recipes.".into(),
            domain: MasteryDomain::Blacksmithing,
            required_tier: MasteryTier::Journeyman,
            prerequisites: vec!["smith_quality".into()],
            effects: vec![SkillEffect::UnlockRecipe("Weapon Tempering".into())],
        });
        nodes.push(SkillTreeNode {
            id: "smith_masterwork".into(),
            name: "Masterwork".into(),
            description: "Small chance to create masterwork items.".into(),
            domain: MasteryDomain::Blacksmithing,
            required_tier: MasteryTier::Master,
            prerequisites: vec!["smith_temper".into()],
            effects: vec![SkillEffect::CraftingQualityBonus(0.30)],
        });

        // === Alchemy Tree ===
        nodes.push(SkillTreeNode {
            id: "alchemy_potency".into(),
            name: "Potent Brews".into(),
            description: "Potions are 20% more effective.".into(),
            domain: MasteryDomain::Alchemy,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::CraftingQualityBonus(0.20)],
        });
        nodes.push(SkillTreeNode {
            id: "alchemy_transmute".into(),
            name: "Transmutation".into(),
            description: "Unlock material conversion recipes.".into(),
            domain: MasteryDomain::Alchemy,
            required_tier: MasteryTier::Expert,
            prerequisites: vec!["alchemy_potency".into()],
            effects: vec![SkillEffect::UnlockRecipe("Transmutation".into())],
        });

        // === Mining Tree ===
        nodes.push(SkillTreeNode {
            id: "mining_yield".into(),
            name: "Rich Veins".into(),
            description: "Gather 25% more ore.".into(),
            domain: MasteryDomain::Mining,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::GatheringYieldBonus(0.25)],
        });

        // === Semantic Attunement Tree ===
        nodes.push(SkillTreeNode {
            id: "semantic_sense".into(),
            name: "Tag Sense".into(),
            description: "Detect semantic tags at longer range.".into(),
            domain: MasteryDomain::SemanticAttunement,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::ExplorationRadius(50.0)],
        });
        nodes.push(SkillTreeNode {
            id: "semantic_resonance".into(),
            name: "Deep Resonance".into(),
            description: "Fire tag affinity +0.2 — stronger fire interactions.".into(),
            domain: MasteryDomain::SemanticAttunement,
            required_tier: MasteryTier::Journeyman,
            prerequisites: vec!["semantic_sense".into()],
            effects: vec![SkillEffect::SemanticAffinity {
                tag: "fire".into(),
                bonus: 0.2,
            }],
        });

        // === Aerial Mastery Tree ===
        nodes.push(SkillTreeNode {
            id: "aerial_efficiency".into(),
            name: "Wind Rider".into(),
            description: "Flight consumes 20% less stamina.".into(),
            domain: MasteryDomain::AerialMastery,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::AerialEfficiency(0.20)],
        });

        // === Trading Tree ===
        nodes.push(SkillTreeNode {
            id: "trade_haggle".into(),
            name: "Haggle".into(),
            description: "NPC trades cost 10% less.".into(),
            domain: MasteryDomain::Trading,
            required_tier: MasteryTier::Apprentice,
            prerequisites: vec![],
            effects: vec![SkillEffect::TradeDiscount(0.10)],
        });

        Self { nodes }
    }

    pub fn get_node(&self, id: &str) -> Option<&SkillTreeNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn nodes_for_domain(&self, domain: MasteryDomain) -> Vec<&SkillTreeNode> {
        self.nodes.iter().filter(|n| n.domain == domain).collect()
    }
}

/// XP amounts for common actions
pub fn xp_for_action(action: &str) -> u64 {
    match action {
        "attack_hit" => 2,
        "combo_complete" => 5,
        "parry_success" => 8,
        "perfect_parry" => 15,
        "dodge_success" => 3,
        "block_success" => 2,
        "aerial_kill" => 10,
        "dive_attack_hit" => 8,
        "craft_item" => 10,
        "craft_rare" => 25,
        "craft_legendary" => 50,
        "gather_resource" => 3,
        "gather_rare" => 10,
        "trade_complete" => 5,
        "explore_new_room" => 5,
        "explore_secret" => 20,
        "semantic_interaction" => 4,
        "floor_clear" => 15,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mastery_tier_from_xp() {
        assert_eq!(MasteryTier::from_xp(0), MasteryTier::Novice);
        assert_eq!(MasteryTier::from_xp(99), MasteryTier::Novice);
        assert_eq!(MasteryTier::from_xp(100), MasteryTier::Apprentice);
        assert_eq!(MasteryTier::from_xp(500), MasteryTier::Journeyman);
        assert_eq!(MasteryTier::from_xp(1500), MasteryTier::Expert);
        assert_eq!(MasteryTier::from_xp(4000), MasteryTier::Master);
        assert_eq!(MasteryTier::from_xp(8000), MasteryTier::Grandmaster);
        assert_eq!(MasteryTier::from_xp(99999), MasteryTier::Grandmaster);
    }

    #[test]
    fn test_mastery_profile_new() {
        let profile = MasteryProfile::new();
        assert_eq!(profile.masteries.len(), 21); // 21 domains
        assert_eq!(profile.total_xp(), 0);
        assert_eq!(
            profile.tier(MasteryDomain::SwordMastery),
            MasteryTier::Novice
        );
    }

    #[test]
    fn test_gain_xp_tier_change() {
        let mut profile = MasteryProfile::new();
        // Add XP below threshold
        assert!(!profile.gain_xp(MasteryDomain::SwordMastery, 50));
        assert_eq!(
            profile.tier(MasteryDomain::SwordMastery),
            MasteryTier::Novice
        );

        // Cross into Apprentice
        assert!(profile.gain_xp(MasteryDomain::SwordMastery, 60));
        assert_eq!(
            profile.tier(MasteryDomain::SwordMastery),
            MasteryTier::Apprentice
        );
    }

    #[test]
    fn test_tier_progress() {
        let mut progress = MasteryProgress::new(MasteryDomain::SwordMastery);
        progress.xp = 50;
        // Novice: 0-99, so 50/100 = 0.5
        let p = progress.tier_progress();
        assert!((p - 0.5).abs() < 0.01, "Expected ~0.5, got {}", p);
    }

    #[test]
    fn test_skill_tree_nodes() {
        let tree = SkillTree::new();
        assert!(!tree.nodes.is_empty());

        let sword_nodes = tree.nodes_for_domain(MasteryDomain::SwordMastery);
        assert!(sword_nodes.len() >= 3, "Sword should have at least 3 nodes");

        // First node has no prerequisites
        assert!(sword_nodes[0].prerequisites.is_empty());
    }

    #[test]
    fn test_unlock_node() {
        let tree = SkillTree::new();
        let mut profile = MasteryProfile::new();

        // Can't unlock Keen Edge as Novice
        let node = tree.get_node("sword_keen_edge").unwrap();
        assert!(!profile
            .masteries
            .get(&MasteryDomain::SwordMastery)
            .unwrap()
            .can_unlock(node));

        // Reach Apprentice
        profile.gain_xp(MasteryDomain::SwordMastery, 100);
        let progress = profile
            .masteries
            .get_mut(&MasteryDomain::SwordMastery)
            .unwrap();
        assert!(progress.can_unlock(node));
        assert!(progress.unlock_node(node));
        assert!(!progress.unlock_node(node)); // can't double-unlock

        // Can't unlock Swift Combo without Keen Edge prerequisite — but we have it now
        let swift = tree.get_node("sword_swift_combo").unwrap();
        // Need Journeyman tier though
        assert!(!progress.can_unlock(swift)); // still Apprentice
        progress.add_xp(400); // total 500 = Journeyman
        assert!(progress.can_unlock(swift));
    }

    #[test]
    fn test_active_effects() {
        let tree = SkillTree::new();
        let mut profile = MasteryProfile::new();

        // No effects initially
        let effects = profile.active_effects(&tree);
        assert!(effects.is_empty());

        // Unlock Keen Edge
        profile.gain_xp(MasteryDomain::SwordMastery, 100);
        let node = tree.get_node("sword_keen_edge").unwrap().clone();
        profile
            .masteries
            .get_mut(&MasteryDomain::SwordMastery)
            .unwrap()
            .unlock_node(&node);

        let effects = profile.active_effects(&tree);
        assert_eq!(effects.len(), 1);
        assert!(matches!(effects[0], SkillEffect::DamageBonus(b) if (b - 0.10).abs() < 0.01));
    }

    #[test]
    fn test_domains_at_tier() {
        let mut profile = MasteryProfile::new();
        assert_eq!(profile.domains_at_tier(MasteryTier::Novice), 21);
        assert_eq!(profile.domains_at_tier(MasteryTier::Apprentice), 0);

        profile.gain_xp(MasteryDomain::SwordMastery, 100);
        profile.gain_xp(MasteryDomain::ParryMastery, 200);
        assert_eq!(profile.domains_at_tier(MasteryTier::Apprentice), 2);
    }

    #[test]
    fn test_xp_for_actions() {
        assert_eq!(xp_for_action("attack_hit"), 2);
        assert_eq!(xp_for_action("perfect_parry"), 15);
        assert_eq!(xp_for_action("craft_legendary"), 50);
        assert_eq!(xp_for_action("unknown_action"), 1);
    }

    #[test]
    fn test_mastery_domain_categories() {
        assert_eq!(
            MasteryDomain::SwordMastery.category(),
            MasteryCategory::Weapon
        );
        assert_eq!(
            MasteryDomain::ParryMastery.category(),
            MasteryCategory::CombatTechnique
        );
        assert_eq!(
            MasteryDomain::Blacksmithing.category(),
            MasteryCategory::Crafting
        );
        assert_eq!(MasteryDomain::Mining.category(), MasteryCategory::Gathering);
        assert_eq!(MasteryDomain::Trading.category(), MasteryCategory::Other);
    }

    #[test]
    fn test_mastery_json_serialization() {
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 150);
        let json = profile.to_json();
        assert!(!json.is_empty());
        assert!(json.contains("SwordMastery"));
    }

    #[test]
    fn test_grandmaster_progress() {
        let mut progress = MasteryProgress::new(MasteryDomain::SwordMastery);
        progress.xp = 10000;
        progress.tier = MasteryTier::from_xp(10000);
        assert_eq!(progress.tier, MasteryTier::Grandmaster);
        assert!((progress.tier_progress() - 1.0).abs() < 0.01); // capped at 1.0
    }
}
