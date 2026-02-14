//! Equipment Special Effects System
//!
//! From dopopensource.txt:
//! "снаряжение не должно давать много характеристик, а упор делается на специальные эффекты"
//!
//! Equipment provides EFFECTS not raw stats:
//! - On-hit: fire damage, lifesteal, chance to stagger
//! - Aura: nearby ally buff, enemy slow, semantic resonance
//! - Conditional: below 30% HP → damage boost, on parry → counter damage
//! - Set bonuses: wearing 2/3/4 pieces from same set
//!
//! Stats from equipment are intentionally SMALL — the effects are the draw.

use serde::{Deserialize, Serialize};

/// Equipment effect trigger conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectTrigger {
    /// Always active while equipped
    Passive,
    /// On successful hit
    OnHit { chance: f32 },
    /// On taking damage
    OnHurt { chance: f32 },
    /// On successful parry
    OnParry,
    /// On successful dodge
    OnDodge,
    /// On combo finisher (last hit of chain)
    OnComboFinisher,
    /// When HP below threshold (0.0-1.0)
    BelowHpThreshold(f32),
    /// When HP above threshold
    AboveHpThreshold(f32),
    /// During specific Breath phase
    DuringBreathPhase(String),
    /// On kill
    OnKill,
    /// On floor transition
    OnFloorEnter,
    /// Every N seconds
    Periodic { interval: f32 },
}

/// What the effect actually does
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectAction {
    /// Deal extra elemental damage
    ElementalDamage { element: String, amount: f32 },
    /// Heal self
    Lifesteal { percent: f32 },
    /// Apply status to target
    ApplyStatus { status: String, duration: f32 },
    /// Buff self temporarily
    SelfBuff {
        stat: String,
        amount: f32,
        duration: f32,
    },
    /// Area damage around player
    AoeDamage {
        radius: f32,
        damage: f32,
        element: String,
    },
    /// Shield / damage absorption
    Shield { amount: f32, duration: f32 },
    /// Resource regeneration
    ResourceRegen { resource: String, amount: f32 },
    /// Movement speed modification
    SpeedModifier { multiplier: f32, duration: f32 },
    /// Chance to not consume resources on ability
    FreeAbility { chance: f32 },
    /// Reduce incoming damage %
    DamageReduction { percent: f32 },
    /// Bonus semantic tag strength
    SemanticBoost { tag: String, amount: f32 },
    /// Cooldown reduction on abilities
    CooldownReduction { percent: f32 },
    /// Summon echo ally
    SummonEcho { echo_type: String, duration: f32 },
    /// Extra loot from kills
    BonusLoot { chance: f32 },
    /// Reflect damage back
    DamageReflect { percent: f32 },
}

/// A single effect on an equipment piece
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentEffect {
    pub name: String,
    pub trigger: EffectTrigger,
    pub action: EffectAction,
}

/// Equipment set definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquipmentSet {
    pub id: String,
    pub name: String,
    pub piece_ids: Vec<String>,
    pub bonuses: Vec<SetBonus>,
}

/// Bonus activated at N pieces equipped
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetBonus {
    pub pieces_required: u32,
    pub description: String,
    pub effects: Vec<EquipmentEffect>,
}

/// Equipment piece with effects (extends existing EquipmentItem)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GearPiece {
    pub id: String,
    pub name: String,
    pub slot: String,
    pub rarity: String,
    // Small stat bonuses (intentionally low)
    pub stat_bonuses: StatBonuses,
    // The main draw — special effects
    pub effects: Vec<EquipmentEffect>,
    pub set_id: Option<String>,
    pub semantic_tags: Vec<(String, f32)>,
    pub durability: f32,
    pub max_durability: f32,
}

/// Intentionally small stat bonuses from equipment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatBonuses {
    pub strength: f32, // typically 1-3 per piece
    pub agility: f32,
    pub vitality: f32,
    pub mind: f32,
    pub spirit: f32,
    pub defense: f32,
}

impl StatBonuses {
    pub fn total(&self) -> f32 {
        self.strength + self.agility + self.vitality + self.mind + self.spirit
    }
}

/// Predefined equipment sets
pub fn tower_equipment_sets() -> Vec<EquipmentSet> {
    vec![
        EquipmentSet {
            id: "echo_walker".into(),
            name: "Echo Walker's Regalia".into(),
            piece_ids: vec![
                "ew_helm".into(),
                "ew_chest".into(),
                "ew_legs".into(),
                "ew_boots".into(),
            ],
            bonuses: vec![
                SetBonus {
                    pieces_required: 2,
                    description: "Echo interactions deal 15% more damage.".into(),
                    effects: vec![EquipmentEffect {
                        name: "Echo Resonance".into(),
                        trigger: EffectTrigger::Passive,
                        action: EffectAction::SemanticBoost {
                            tag: "echo".into(),
                            amount: 0.15,
                        },
                    }],
                },
                SetBonus {
                    pieces_required: 4,
                    description: "On kill: 20% chance to summon a helpful echo.".into(),
                    effects: vec![EquipmentEffect {
                        name: "Echo Manifestation".into(),
                        trigger: EffectTrigger::OnKill,
                        action: EffectAction::SummonEcho {
                            echo_type: "helpful".into(),
                            duration: 30.0,
                        },
                    }],
                },
            ],
        },
        EquipmentSet {
            id: "flame_forged".into(),
            name: "Flame-Forged Arsenal".into(),
            piece_ids: vec!["ff_helm".into(), "ff_chest".into(), "ff_gauntlets".into()],
            bonuses: vec![
                SetBonus {
                    pieces_required: 2,
                    description: "Attacks deal bonus fire damage.".into(),
                    effects: vec![EquipmentEffect {
                        name: "Ember Strike".into(),
                        trigger: EffectTrigger::OnHit { chance: 0.30 },
                        action: EffectAction::ElementalDamage {
                            element: "fire".into(),
                            amount: 15.0,
                        },
                    }],
                },
                SetBonus {
                    pieces_required: 3,
                    description: "Below 30% HP: fire aura damages nearby enemies.".into(),
                    effects: vec![EquipmentEffect {
                        name: "Inferno Aura".into(),
                        trigger: EffectTrigger::BelowHpThreshold(0.3),
                        action: EffectAction::AoeDamage {
                            radius: 5.0,
                            damage: 8.0,
                            element: "fire".into(),
                        },
                    }],
                },
            ],
        },
        EquipmentSet {
            id: "void_touched".into(),
            name: "Void-Touched Vestments".into(),
            piece_ids: vec!["vt_helm".into(), "vt_robes".into(), "vt_ring".into()],
            bonuses: vec![
                SetBonus {
                    pieces_required: 2,
                    description: "Semantic energy regenerates 25% faster.".into(),
                    effects: vec![EquipmentEffect {
                        name: "Void Flow".into(),
                        trigger: EffectTrigger::Passive,
                        action: EffectAction::ResourceRegen {
                            resource: "semantic".into(),
                            amount: 0.25,
                        },
                    }],
                },
                SetBonus {
                    pieces_required: 3,
                    description: "On dodge: become invisible for 2s (cooldown 15s).".into(),
                    effects: vec![EquipmentEffect {
                        name: "Void Step".into(),
                        trigger: EffectTrigger::OnDodge,
                        action: EffectAction::SelfBuff {
                            stat: "stealth".into(),
                            amount: 1.0,
                            duration: 2.0,
                        },
                    }],
                },
            ],
        },
    ]
}

/// Generate a random equipment effect based on semantic tags
pub fn generate_effect_for_tags(
    tags: &[(String, f32)],
    rarity_tier: u32,
) -> Option<EquipmentEffect> {
    // Find dominant tag
    let dominant = tags
        .iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    let dominant_tag = dominant.map(|(t, _)| t.as_str()).unwrap_or("neutral");

    let effect = match dominant_tag {
        "fire" | "thermal" => EquipmentEffect {
            name: "Ember Touch".into(),
            trigger: EffectTrigger::OnHit {
                chance: 0.15 + rarity_tier as f32 * 0.05,
            },
            action: EffectAction::ElementalDamage {
                element: "fire".into(),
                amount: 5.0 + rarity_tier as f32 * 3.0,
            },
        },
        "water" | "ice" => EquipmentEffect {
            name: "Frost Bite".into(),
            trigger: EffectTrigger::OnHit {
                chance: 0.15 + rarity_tier as f32 * 0.05,
            },
            action: EffectAction::ApplyStatus {
                status: "slow".into(),
                duration: 2.0 + rarity_tier as f32,
            },
        },
        "earth" | "stone" => EquipmentEffect {
            name: "Stoneguard".into(),
            trigger: EffectTrigger::OnHurt { chance: 0.20 },
            action: EffectAction::Shield {
                amount: 10.0 + rarity_tier as f32 * 5.0,
                duration: 5.0,
            },
        },
        "wind" | "air" => EquipmentEffect {
            name: "Gale Step".into(),
            trigger: EffectTrigger::OnDodge,
            action: EffectAction::SpeedModifier {
                multiplier: 1.3,
                duration: 3.0,
            },
        },
        "void" | "corruption" => EquipmentEffect {
            name: "Void Drain".into(),
            trigger: EffectTrigger::OnHit {
                chance: 0.10 + rarity_tier as f32 * 0.05,
            },
            action: EffectAction::Lifesteal {
                percent: 0.05 + rarity_tier as f32 * 0.03,
            },
        },
        "echo" => EquipmentEffect {
            name: "Echo Memory".into(),
            trigger: EffectTrigger::OnKill,
            action: EffectAction::SummonEcho {
                echo_type: "lingering".into(),
                duration: 15.0 + rarity_tier as f32 * 5.0,
            },
        },
        _ => EquipmentEffect {
            name: "Balanced Aura".into(),
            trigger: EffectTrigger::Passive,
            action: EffectAction::DamageReduction {
                percent: 0.03 + rarity_tier as f32 * 0.02,
            },
        },
    };

    Some(effect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_bonuses_intentionally_small() {
        let gear = GearPiece {
            id: "test_helm".into(),
            name: "Iron Helm".into(),
            slot: "Head".into(),
            rarity: "Common".into(),
            stat_bonuses: StatBonuses {
                strength: 1.0,
                agility: 0.0,
                vitality: 2.0,
                mind: 0.0,
                spirit: 0.0,
                defense: 3.0,
            },
            effects: vec![],
            set_id: None,
            semantic_tags: vec![("earth".into(), 0.5)],
            durability: 100.0,
            max_durability: 100.0,
        };
        // Equipment stats should be small — effects are the draw
        assert!(
            gear.stat_bonuses.total() <= 5.0,
            "Stats should be intentionally low"
        );
    }

    #[test]
    fn test_equipment_sets_exist() {
        let sets = tower_equipment_sets();
        assert_eq!(sets.len(), 3);
        assert_eq!(sets[0].name, "Echo Walker's Regalia");
        assert_eq!(sets[1].name, "Flame-Forged Arsenal");
        assert_eq!(sets[2].name, "Void-Touched Vestments");
    }

    #[test]
    fn test_set_bonus_tiers() {
        let sets = tower_equipment_sets();
        for set in &sets {
            assert!(!set.bonuses.is_empty());
            // First bonus should require fewer pieces
            assert!(set.bonuses[0].pieces_required <= set.bonuses.last().unwrap().pieces_required);
        }
    }

    #[test]
    fn test_generate_fire_effect() {
        let tags = vec![("fire".into(), 0.8), ("combat".into(), 0.3)];
        let effect = generate_effect_for_tags(&tags, 2).unwrap();
        assert_eq!(effect.name, "Ember Touch");
        assert!(matches!(
            effect.action,
            EffectAction::ElementalDamage { .. }
        ));
    }

    #[test]
    fn test_generate_water_effect() {
        let tags = vec![("water".into(), 0.7)];
        let effect = generate_effect_for_tags(&tags, 1).unwrap();
        assert_eq!(effect.name, "Frost Bite");
        assert!(matches!(effect.action, EffectAction::ApplyStatus { .. }));
    }

    #[test]
    fn test_generate_void_effect() {
        let tags = vec![("void".into(), 0.9)];
        let effect = generate_effect_for_tags(&tags, 3).unwrap();
        assert_eq!(effect.name, "Void Drain");
        assert!(matches!(effect.action, EffectAction::Lifesteal { .. }));
    }

    #[test]
    fn test_rarity_scales_effects() {
        let tags = vec![("fire".into(), 0.8)];
        let low = generate_effect_for_tags(&tags, 0).unwrap();
        let high = generate_effect_for_tags(&tags, 4).unwrap();
        // Higher rarity should give stronger effects
        match (&low.action, &high.action) {
            (
                EffectAction::ElementalDamage { amount: a, .. },
                EffectAction::ElementalDamage { amount: b, .. },
            ) => {
                assert!(b > a, "Higher rarity should deal more damage");
            }
            _ => panic!("Expected ElementalDamage"),
        }
    }

    #[test]
    fn test_effect_trigger_variants() {
        // Ensure all trigger types are constructible
        let triggers = vec![
            EffectTrigger::Passive,
            EffectTrigger::OnHit { chance: 0.5 },
            EffectTrigger::OnHurt { chance: 0.3 },
            EffectTrigger::OnParry,
            EffectTrigger::OnDodge,
            EffectTrigger::OnComboFinisher,
            EffectTrigger::BelowHpThreshold(0.3),
            EffectTrigger::OnKill,
            EffectTrigger::Periodic { interval: 5.0 },
        ];
        assert_eq!(triggers.len(), 9);
    }

    #[test]
    fn test_durability() {
        let gear = GearPiece {
            id: "test".into(),
            name: "Test".into(),
            slot: "Head".into(),
            rarity: "Common".into(),
            stat_bonuses: StatBonuses::default(),
            effects: vec![],
            set_id: None,
            semantic_tags: vec![],
            durability: 80.0,
            max_durability: 100.0,
        };
        assert!(gear.durability < gear.max_durability);
    }
}
