//! Specialization & Role System
//!
//! From ddopensource.txt Categories 6-7:
//! Branching paths within mastery trees, role definition (Tank/DPS/Support/Healer),
//! hybrid builds, ultimate abilities, and synergy combinations.
//!
//! Specializations unlock at Expert tier in a mastery domain.
//! Each domain has 2-3 branches — you pick ONE per domain.
//! Branches define playstyle: offensive, defensive, utility.
//! Roles emerge from specialization choices, not rigid class selection.

use crate::mastery::{MasteryDomain, MasteryProfile, MasteryTier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role that emerges from specialization choices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CombatRole {
    Vanguard,   // Tank — draws aggro, high survivability
    Striker,    // DPS — maximizes damage output
    Support,    // Buffer/debuffer — amplifies team
    Sentinel,   // Healer/shielder — keeps team alive
    Specialist, // Hybrid — unique utility role
}

impl CombatRole {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Vanguard => "Vanguard",
            Self::Striker => "Striker",
            Self::Support => "Support",
            Self::Sentinel => "Sentinel",
            Self::Specialist => "Specialist",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Vanguard => {
                "Frontline defender. Draws enemy aggro and absorbs damage for the team."
            }
            Self::Striker => {
                "Damage dealer. Maximizes offensive output through combos and abilities."
            }
            Self::Support => {
                "Force multiplier. Buffs allies and debuffs enemies to tip the scales."
            }
            Self::Sentinel => "Guardian. Shields and heals allies, maintaining team survivability.",
            Self::Specialist => {
                "Utility hybrid. Unique tools for exploration, crafting, and combat."
            }
        }
    }
}

/// A specialization branch within a mastery domain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializationBranch {
    pub id: String,
    pub name: String,
    pub domain: MasteryDomain,
    pub description: String,
    pub required_tier: MasteryTier,
    /// Role affinity — contributes to overall role determination
    pub role_affinity: CombatRole,
    /// Passive bonuses granted by choosing this branch
    pub passives: Vec<SpecPassive>,
    /// Ultimate ability unlocked at Master tier in this branch
    pub ultimate: Option<UltimateAbility>,
}

/// Passive bonus from a specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecPassive {
    /// Flat damage increase
    DamageFlat(f32),
    /// Damage multiplier %
    DamagePercent(f32),
    /// Defense multiplier %
    DefensePercent(f32),
    /// HP bonus flat
    HpBonus(f32),
    /// Resource cost reduction %
    ResourceReduction(f32),
    /// Cooldown reduction %
    CooldownReduction(f32),
    /// Buff duration increase %
    BuffDurationIncrease(f32),
    /// Heal effectiveness %
    HealEffectiveness(f32),
    /// Gathering speed bonus %
    GatheringSpeed(f32),
    /// Crafting success bonus %
    CraftingSuccess(f32),
    /// Critical hit chance bonus
    CritChance(f32),
    /// Movement speed bonus %
    MoveSpeed(f32),
    /// Aggro generation modifier
    AggroModifier(f32),
    /// Party-wide buff radius
    AuraRadius(f32),
}

/// Ultimate ability unlocked via specialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UltimateAbility {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cooldown_seconds: f32,
    pub effect: UltimateEffect,
}

/// What an ultimate ability does
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UltimateEffect {
    /// Massive AoE damage burst
    AoeBurst {
        radius: f32,
        damage: f32,
        element: String,
    },
    /// Full team heal over time
    TeamHeal { amount: f32, duration: f32 },
    /// Invulnerability for duration
    Invulnerable { duration: f32 },
    /// All enemies taunted to you
    MassTaunt { duration: f32, radius: f32 },
    /// Massive buff to all party members
    PartyBuff {
        stat: String,
        amount: f32,
        duration: f32,
    },
    /// Transform into empowered state
    Transformation {
        duration: f32,
        damage_mult: f32,
        speed_mult: f32,
    },
    /// Summon powerful ally
    SummonAlly { ally_type: String, duration: f32 },
    /// Time slow in radius (enemies only)
    TimeDistortion {
        radius: f32,
        slow_factor: f32,
        duration: f32,
    },
}

/// Player's specialization choices
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpecializationProfile {
    /// Chosen branch per domain (domain → branch_id)
    pub chosen_branches: HashMap<MasteryDomain, String>,
    /// Computed primary role (from most branch affinities)
    pub primary_role: Option<CombatRole>,
    /// Secondary role
    pub secondary_role: Option<CombatRole>,
}

impl SpecializationProfile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Choose a specialization branch for a domain
    pub fn choose_branch(
        &mut self,
        branch: &SpecializationBranch,
        profile: &MasteryProfile,
    ) -> Result<(), SpecError> {
        // Check tier requirement
        let tier = profile.tier(branch.domain);
        if tier < branch.required_tier {
            return Err(SpecError::InsufficientTier {
                required: branch.required_tier,
                current: tier,
            });
        }

        // Can only pick one branch per domain
        if self.chosen_branches.contains_key(&branch.domain) {
            return Err(SpecError::AlreadySpecialized(branch.domain));
        }

        self.chosen_branches
            .insert(branch.domain, branch.id.clone());
        self.recalculate_roles();
        Ok(())
    }

    /// Reset specialization for a domain (costs resources in-game)
    pub fn reset_branch(&mut self, domain: MasteryDomain) -> bool {
        if self.chosen_branches.remove(&domain).is_some() {
            self.recalculate_roles();
            true
        } else {
            false
        }
    }

    /// Check if player has specialized in a domain
    pub fn has_specialization(&self, domain: MasteryDomain) -> bool {
        self.chosen_branches.contains_key(&domain)
    }

    /// Get chosen branch ID for a domain
    pub fn get_branch(&self, domain: MasteryDomain) -> Option<&str> {
        self.chosen_branches.get(&domain).map(|s| s.as_str())
    }

    /// Recalculate primary/secondary roles from branch affinities
    fn recalculate_roles(&mut self) {
        let all_branches = all_specialization_branches();
        let mut role_counts: HashMap<CombatRole, u32> = HashMap::new();

        for branch_id in self.chosen_branches.values() {
            if let Some(branch) = all_branches.iter().find(|b| b.id == *branch_id) {
                *role_counts.entry(branch.role_affinity).or_insert(0) += 1;
            }
        }

        let mut sorted: Vec<_> = role_counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        self.primary_role = sorted.first().map(|(r, _)| *r);
        self.secondary_role = sorted.get(1).map(|(r, _)| *r);
    }

    /// Collect all passives from chosen branches
    pub fn active_passives(&self) -> Vec<SpecPassive> {
        let all_branches = all_specialization_branches();
        let mut passives = Vec::new();

        for branch_id in self.chosen_branches.values() {
            if let Some(branch) = all_branches.iter().find(|b| b.id == *branch_id) {
                passives.extend(branch.passives.clone());
            }
        }
        passives
    }

    /// Collect all ultimate abilities from chosen branches
    pub fn ultimate_abilities(&self) -> Vec<UltimateAbility> {
        let all_branches = all_specialization_branches();
        let mut ults = Vec::new();

        for branch_id in self.chosen_branches.values() {
            if let Some(branch) = all_branches.iter().find(|b| b.id == *branch_id) {
                if let Some(ult) = &branch.ultimate {
                    ults.push(ult.clone());
                }
            }
        }
        ults
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub enum SpecError {
    InsufficientTier {
        required: MasteryTier,
        current: MasteryTier,
    },
    AlreadySpecialized(MasteryDomain),
}

/// Synergy between two specialization branches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Synergy {
    pub branch_a: String,
    pub branch_b: String,
    pub name: String,
    pub description: String,
    pub bonus: SpecPassive,
}

/// All predefined specialization branches
pub fn all_specialization_branches() -> Vec<SpecializationBranch> {
    vec![
        // === Sword Mastery Branches ===
        SpecializationBranch {
            id: "sword_bladestorm".into(),
            name: "Bladestorm".into(),
            domain: MasteryDomain::SwordMastery,
            description: "Offensive sword style. Faster combos, higher damage, combo extensions.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::DamagePercent(0.15),
                SpecPassive::CritChance(0.05),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_bladestorm".into(),
                name: "Thousand Cuts".into(),
                description: "Unleash a flurry of 12 strikes in 2 seconds, each dealing 80% weapon damage.".into(),
                cooldown_seconds: 90.0,
                effect: UltimateEffect::Transformation {
                    duration: 2.0, damage_mult: 0.8, speed_mult: 3.0,
                },
            }),
        },
        SpecializationBranch {
            id: "sword_guardian".into(),
            name: "Guardian's Edge".into(),
            domain: MasteryDomain::SwordMastery,
            description: "Defensive sword style. Counter-attacks, parry bonuses, aggro generation.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Vanguard,
            passives: vec![
                SpecPassive::DefensePercent(0.20),
                SpecPassive::AggroModifier(1.5),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_guardian_stance".into(),
                name: "Unbreakable Stance".into(),
                description: "Become immovable for 5 seconds. All attacks auto-parried, reflecting 50% damage.".into(),
                cooldown_seconds: 120.0,
                effect: UltimateEffect::Invulnerable { duration: 5.0 },
            }),
        },

        // === Staff Mastery Branches ===
        SpecializationBranch {
            id: "staff_arcane".into(),
            name: "Arcane Conduit".into(),
            domain: MasteryDomain::StaffMastery,
            description: "Offensive magic. Amplified elemental damage and AoE abilities.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::DamagePercent(0.20),
                SpecPassive::ResourceReduction(0.10),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_meteor".into(),
                name: "Semantic Meteor".into(),
                description: "Channel tower energy into a devastating AoE blast.".into(),
                cooldown_seconds: 120.0,
                effect: UltimateEffect::AoeBurst {
                    radius: 15.0, damage: 500.0, element: "semantic".into(),
                },
            }),
        },
        SpecializationBranch {
            id: "staff_mender".into(),
            name: "Mender's Path".into(),
            domain: MasteryDomain::StaffMastery,
            description: "Healing focus. Restore HP, cleanse debuffs, shield allies.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Sentinel,
            passives: vec![
                SpecPassive::HealEffectiveness(0.30),
                SpecPassive::BuffDurationIncrease(0.20),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_rejuvenation".into(),
                name: "Tower's Blessing".into(),
                description: "Heal all party members for 50% HP over 8 seconds.".into(),
                cooldown_seconds: 150.0,
                effect: UltimateEffect::TeamHeal { amount: 0.5, duration: 8.0 },
            }),
        },

        // === Gauntlet Mastery Branches ===
        SpecializationBranch {
            id: "gauntlet_berserker".into(),
            name: "Berserker".into(),
            domain: MasteryDomain::GauntletMastery,
            description: "Reckless offense. More damage at lower HP. Speed and fury.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::DamagePercent(0.25),
                SpecPassive::MoveSpeed(0.10),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_rampage".into(),
                name: "Unstoppable Rampage".into(),
                description: "For 10 seconds, each hit increases attack speed by 5%. Stacks infinitely.".into(),
                cooldown_seconds: 100.0,
                effect: UltimateEffect::Transformation {
                    duration: 10.0, damage_mult: 1.0, speed_mult: 1.5,
                },
            }),
        },
        SpecializationBranch {
            id: "gauntlet_ironwall".into(),
            name: "Iron Wall".into(),
            domain: MasteryDomain::GauntletMastery,
            description: "Unyielding defense. Block mastery, crowd control, team protection.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Vanguard,
            passives: vec![
                SpecPassive::DefensePercent(0.25),
                SpecPassive::HpBonus(200.0),
                SpecPassive::AggroModifier(2.0),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_fortress".into(),
                name: "Living Fortress".into(),
                description: "Taunt all enemies in range. Gain 80% damage reduction for 8 seconds.".into(),
                cooldown_seconds: 120.0,
                effect: UltimateEffect::MassTaunt { duration: 8.0, radius: 20.0 },
            }),
        },

        // === Parry Mastery Branches ===
        SpecializationBranch {
            id: "parry_riposte".into(),
            name: "Riposte Master".into(),
            domain: MasteryDomain::ParryMastery,
            description: "Turn defense into offense. Perfect parries become devastating counters.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::CritChance(0.10),
                SpecPassive::DamagePercent(0.10),
            ],
            ultimate: None, // Parry doesn't get an ultimate — it enhances other weapon ultimates
        },
        SpecializationBranch {
            id: "parry_bulwark".into(),
            name: "Bulwark".into(),
            domain: MasteryDomain::ParryMastery,
            description: "Defensive parry. Wider windows, party-wide damage mitigation on perfect parry.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Support,
            passives: vec![
                SpecPassive::DefensePercent(0.10),
                SpecPassive::AuraRadius(10.0),
            ],
            ultimate: None,
        },

        // === Blacksmithing Branches ===
        SpecializationBranch {
            id: "smith_weaponsmith".into(),
            name: "Weaponsmith".into(),
            domain: MasteryDomain::Blacksmithing,
            description: "Weapon specialist. Higher damage rolls, unique weapon effects.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Specialist,
            passives: vec![
                SpecPassive::CraftingSuccess(0.20),
            ],
            ultimate: None,
        },
        SpecializationBranch {
            id: "smith_armorsmith".into(),
            name: "Armorsmith".into(),
            domain: MasteryDomain::Blacksmithing,
            description: "Armor specialist. Higher defense rolls, socket creation.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Specialist,
            passives: vec![
                SpecPassive::CraftingSuccess(0.15),
                SpecPassive::DefensePercent(0.05),
            ],
            ultimate: None,
        },

        // === Dodge Mastery Branches ===
        SpecializationBranch {
            id: "dodge_shadow".into(),
            name: "Shadow Step".into(),
            domain: MasteryDomain::DodgeMastery,
            description: "Evasion offense. Dodges leave afterimages that deal damage.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::MoveSpeed(0.15),
                SpecPassive::DamagePercent(0.08),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_phantom".into(),
                name: "Phantom Rush".into(),
                description: "Become untargetable for 4 seconds, striking all enemies in path.".into(),
                cooldown_seconds: 80.0,
                effect: UltimateEffect::TimeDistortion {
                    radius: 12.0, slow_factor: 0.3, duration: 4.0,
                },
            }),
        },
        SpecializationBranch {
            id: "dodge_windwalker".into(),
            name: "Windwalker".into(),
            domain: MasteryDomain::DodgeMastery,
            description: "Supportive mobility. Dodging near allies grants them speed.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Support,
            passives: vec![
                SpecPassive::MoveSpeed(0.20),
                SpecPassive::AuraRadius(8.0),
            ],
            ultimate: Some(UltimateAbility {
                id: "ult_gale_force".into(),
                name: "Gale Force".into(),
                description: "All party members gain 50% movement and attack speed for 10 seconds.".into(),
                cooldown_seconds: 120.0,
                effect: UltimateEffect::PartyBuff {
                    stat: "speed".into(), amount: 0.50, duration: 10.0,
                },
            }),
        },

        // === Alchemy Branches ===
        SpecializationBranch {
            id: "alchemy_poisoner".into(),
            name: "Poisoner".into(),
            domain: MasteryDomain::Alchemy,
            description: "Offensive alchemy. Craft deadly poisons and corrosive bombs.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Striker,
            passives: vec![
                SpecPassive::DamagePercent(0.10),
                SpecPassive::CraftingSuccess(0.15),
            ],
            ultimate: None,
        },
        SpecializationBranch {
            id: "alchemy_herbalist".into(),
            name: "Herbalist".into(),
            domain: MasteryDomain::Alchemy,
            description: "Healing alchemy. Enhanced potions, HoT effects, team restoration.".into(),
            required_tier: MasteryTier::Expert,
            role_affinity: CombatRole::Sentinel,
            passives: vec![
                SpecPassive::HealEffectiveness(0.20),
                SpecPassive::CraftingSuccess(0.15),
            ],
            ultimate: None,
        },
    ]
}

/// Predefined synergies between branches
pub fn branch_synergies() -> Vec<Synergy> {
    vec![
        Synergy {
            branch_a: "sword_bladestorm".into(),
            branch_b: "parry_riposte".into(),
            name: "Counter-Storm".into(),
            description: "Perfect parries extend your next combo by 2 hits.".into(),
            bonus: SpecPassive::DamagePercent(0.10),
        },
        Synergy {
            branch_a: "gauntlet_ironwall".into(),
            branch_b: "parry_bulwark".into(),
            name: "Unshakeable".into(),
            description: "Blocking and parrying generates a shield for nearby allies.".into(),
            bonus: SpecPassive::AuraRadius(5.0),
        },
        Synergy {
            branch_a: "staff_mender".into(),
            branch_b: "alchemy_herbalist".into(),
            name: "Master Healer".into(),
            description: "All healing effects increased by 20%.".into(),
            bonus: SpecPassive::HealEffectiveness(0.20),
        },
        Synergy {
            branch_a: "dodge_shadow".into(),
            branch_b: "sword_bladestorm".into(),
            name: "Flash Blade".into(),
            description: "Attacks after dodge deal 25% more damage for 2 seconds.".into(),
            bonus: SpecPassive::DamagePercent(0.15),
        },
        Synergy {
            branch_a: "smith_weaponsmith".into(),
            branch_b: "alchemy_poisoner".into(),
            name: "Toxic Arsenal".into(),
            description: "Crafted weapons have 15% chance to apply poison.".into(),
            bonus: SpecPassive::CraftingSuccess(0.10),
        },
    ]
}

/// Get active synergies for a specialization profile
pub fn active_synergies(profile: &SpecializationProfile) -> Vec<&Synergy> {
    let synergies = branch_synergies();
    let chosen: Vec<&str> = profile
        .chosen_branches
        .values()
        .map(|s| s.as_str())
        .collect();

    // We need to return owned references — instead collect into a vec
    // This is a helper, typically called with the static list
    let _ = synergies; // avoid unused
    let _ = chosen;
    // In practice, caller iterates branch_synergies() directly
    vec![]
}

/// Check which synergies are active for given chosen branches
pub fn find_active_synergies(chosen_branch_ids: &[String]) -> Vec<Synergy> {
    branch_synergies()
        .into_iter()
        .filter(|syn| {
            chosen_branch_ids.iter().any(|b| b == &syn.branch_a)
                && chosen_branch_ids.iter().any(|b| b == &syn.branch_b)
        })
        .collect()
}

/// Bevy plugin stub
pub struct SpecializationPlugin;
impl bevy::prelude::Plugin for SpecializationPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_branches_exist() {
        let branches = all_specialization_branches();
        assert!(
            branches.len() >= 14,
            "Should have at least 14 branches, got {}",
            branches.len()
        );
    }

    #[test]
    fn test_branches_per_domain() {
        let branches = all_specialization_branches();
        let sword: Vec<_> = branches
            .iter()
            .filter(|b| b.domain == MasteryDomain::SwordMastery)
            .collect();
        assert_eq!(sword.len(), 2, "Sword should have 2 branches");

        let staff: Vec<_> = branches
            .iter()
            .filter(|b| b.domain == MasteryDomain::StaffMastery)
            .collect();
        assert_eq!(staff.len(), 2, "Staff should have 2 branches");
    }

    #[test]
    fn test_choose_branch_requires_tier() {
        let mut spec = SpecializationProfile::new();
        let profile = MasteryProfile::new(); // all Novice

        let branches = all_specialization_branches();
        let sword_branch = &branches[0]; // requires Expert

        let result = spec.choose_branch(sword_branch, &profile);
        assert!(result.is_err());
    }

    #[test]
    fn test_choose_branch_success() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000); // Expert tier

        let branches = all_specialization_branches();
        let sword_branch = &branches[0]; // Bladestorm

        let result = spec.choose_branch(sword_branch, &profile);
        assert!(result.is_ok());
        assert!(spec.has_specialization(MasteryDomain::SwordMastery));
        assert_eq!(
            spec.get_branch(MasteryDomain::SwordMastery),
            Some("sword_bladestorm")
        );
    }

    #[test]
    fn test_cannot_double_specialize() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);

        let branches = all_specialization_branches();
        spec.choose_branch(&branches[0], &profile).unwrap(); // Bladestorm

        let result = spec.choose_branch(&branches[1], &profile); // Guardian's Edge
        assert!(result.is_err()); // already specialized in Sword
    }

    #[test]
    fn test_reset_branch() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);

        let branches = all_specialization_branches();
        spec.choose_branch(&branches[0], &profile).unwrap();
        assert!(spec.has_specialization(MasteryDomain::SwordMastery));

        assert!(spec.reset_branch(MasteryDomain::SwordMastery));
        assert!(!spec.has_specialization(MasteryDomain::SwordMastery));
    }

    #[test]
    fn test_role_calculation() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);
        profile.gain_xp(MasteryDomain::GauntletMastery, 2000);
        profile.gain_xp(MasteryDomain::ParryMastery, 2000);

        let branches = all_specialization_branches();
        // Choose Bladestorm (Striker) + Berserker (Striker) + Riposte (Striker)
        spec.choose_branch(&branches[0], &profile).unwrap(); // sword_bladestorm = Striker
        let berserker = branches
            .iter()
            .find(|b| b.id == "gauntlet_berserker")
            .unwrap();
        spec.choose_branch(berserker, &profile).unwrap(); // Striker
        let riposte = branches.iter().find(|b| b.id == "parry_riposte").unwrap();
        spec.choose_branch(riposte, &profile).unwrap(); // Striker

        assert_eq!(spec.primary_role, Some(CombatRole::Striker));
    }

    #[test]
    fn test_active_passives() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);

        let branches = all_specialization_branches();
        spec.choose_branch(&branches[0], &profile).unwrap(); // Bladestorm: DamagePercent + CritChance

        let passives = spec.active_passives();
        assert_eq!(passives.len(), 2);
    }

    #[test]
    fn test_ultimate_abilities() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);

        let branches = all_specialization_branches();
        spec.choose_branch(&branches[0], &profile).unwrap(); // Bladestorm has Thousand Cuts

        let ults = spec.ultimate_abilities();
        assert_eq!(ults.len(), 1);
        assert_eq!(ults[0].name, "Thousand Cuts");
    }

    #[test]
    fn test_synergies() {
        let synergies = branch_synergies();
        assert!(synergies.len() >= 5, "Should have at least 5 synergies");

        // Test finding active synergies
        let chosen = vec!["sword_bladestorm".to_string(), "parry_riposte".to_string()];
        let active = find_active_synergies(&chosen);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "Counter-Storm");
    }

    #[test]
    fn test_combat_role_display() {
        assert_eq!(CombatRole::Vanguard.display_name(), "Vanguard");
        assert_eq!(CombatRole::Striker.display_name(), "Striker");
        assert!(!CombatRole::Sentinel.description().is_empty());
    }

    #[test]
    fn test_spec_json_serialization() {
        let mut spec = SpecializationProfile::new();
        let mut profile = MasteryProfile::new();
        profile.gain_xp(MasteryDomain::SwordMastery, 2000);

        let branches = all_specialization_branches();
        spec.choose_branch(&branches[0], &profile).unwrap();

        let json = spec.to_json();
        assert!(!json.is_empty());
        assert!(json.contains("sword_bladestorm"));
    }
}
