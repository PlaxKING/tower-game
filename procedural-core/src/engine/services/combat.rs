use crate::combat::AttackAngle;
use crate::constants::*;
use crate::engine::config::EngineConfig;
use crate::engine::messages::{DamageCalcResultMsg, DamageModifierMsg};
use crate::generation::TowerSeed;
use crate::semantic::SemanticTags;

/// CombatService — processes combat actions and calculates damage
pub struct CombatService {
    #[allow(dead_code)]
    tower_seed: TowerSeed,
}

impl CombatService {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            tower_seed: TowerSeed {
                seed: config.tower_seed,
            },
        }
    }

    pub fn calculate_damage(
        &self,
        base_damage: f32,
        angle_id: u32,
        combo_step: u32,
        attacker_tags: &[(String, f32)],
        defender_tags: &[(String, f32)],
    ) -> DamageCalcResultMsg {
        let angle_mult = match angle_id {
            0 => AttackAngle::Front.multiplier(),
            1 => AttackAngle::Side.multiplier(),
            2 => AttackAngle::Back.multiplier(),
            _ => 1.0,
        };

        let sem_a = SemanticTags {
            tags: attacker_tags.to_vec(),
        };
        let sem_b = SemanticTags {
            tags: defender_tags.to_vec(),
        };
        let similarity = sem_a.similarity(&sem_b);

        let semantic_mult = if similarity > SEMANTIC_HIGH_THRESHOLD {
            SEMANTIC_HIGH_MULT
        } else if similarity < SEMANTIC_LOW_THRESHOLD {
            SEMANTIC_LOW_MULT
        } else {
            1.0
        };
        let combo_mult = 1.0 + combo_step as f32 * COMBO_STEP_MULT;

        let modified = base_damage * angle_mult * combo_mult * semantic_mult;

        let mut modifiers = vec![
            DamageModifierMsg {
                source: "angle".into(),
                multiplier: angle_mult,
                description: format!(
                    "{:?} attack",
                    match angle_id {
                        0 => "Front",
                        1 => "Side",
                        2 => "Back",
                        _ => "Unknown",
                    }
                ),
            },
            DamageModifierMsg {
                source: "combo".into(),
                multiplier: combo_mult,
                description: format!("Combo step {}", combo_step),
            },
        ];

        if (semantic_mult - 1.0).abs() > f32::EPSILON {
            modifiers.push(DamageModifierMsg {
                source: "semantic".into(),
                multiplier: semantic_mult,
                description: format!("Tag similarity {:.2}", similarity),
            });
        }

        DamageCalcResultMsg {
            base_damage,
            modified_damage: modified,
            crit_chance: BASE_CRIT_CHANCE,
            crit_damage: modified * CRIT_DAMAGE_MULT,
            modifiers,
        }
    }
}
