//! Weapon moveset and combo system.
//!
//! Each weapon type has a moveset of combo chains.
//! Attacks are directional and resource-gated.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{AttackPhase, CombatResources, CombatState};

/// Weapon types available in the tower
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeaponType {
    Sword,       // balanced, fast combos
    Greatsword,  // slow, high damage, wide arcs
    DualDaggers, // very fast, low damage per hit, long combos
    Spear,       // long reach, thrust attacks
    Gauntlets,   // close range, aerial combos
    Staff,       // semantic-charged attacks, ranged
}

/// A single attack in a combo chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboAttack {
    pub name: String,
    pub windup: f32,   // seconds
    pub active: f32,   // seconds
    pub recovery: f32, // seconds
    pub damage_mult: f32,
    pub knockback: f32,
    pub hitbox_size: [f32; 3],   // width, height, depth
    pub hitbox_offset: [f32; 3], // forward, up, right
    pub resource_cost: ResourceCost,
}

/// Resource cost for an attack
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceCost {
    pub kinetic: f32,
    pub thermal: f32,
    pub semantic: f32,
}

/// Full weapon definition with combo chains
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Weapon {
    pub weapon_type: WeaponType,
    pub name: String,
    pub base_damage: f32,
    pub combo_chain: Vec<ComboAttack>,
    pub aerial_chain: Vec<ComboAttack>,
}

impl Weapon {
    /// Get the current combo attack based on step
    pub fn current_attack(&self, combo_step: u32, is_aerial: bool) -> Option<&ComboAttack> {
        let chain = if is_aerial {
            &self.aerial_chain
        } else {
            &self.combo_chain
        };
        chain.get(combo_step as usize)
    }

    pub fn max_combo(&self) -> u32 {
        self.combo_chain.len() as u32
    }

    pub fn max_aerial_combo(&self) -> u32 {
        self.aerial_chain.len() as u32
    }
}

/// Currently equipped weapon slot
#[derive(Component, Debug)]
pub struct EquippedWeapon {
    pub weapon: Weapon,
    pub is_aerial: bool,
}

/// Predefined weapon templates
pub fn sword() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Sword,
        name: "Iron Sword".into(),
        base_damage: 30.0,
        combo_chain: vec![
            ComboAttack {
                name: "Horizontal Slash".into(),
                windup: 0.25,
                active: 0.10,
                recovery: 0.30,
                damage_mult: 1.0,
                knockback: 2.0,
                hitbox_size: [1.5, 0.8, 1.8],
                hitbox_offset: [1.5, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 5.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Diagonal Cut".into(),
                windup: 0.20,
                active: 0.12,
                recovery: 0.28,
                damage_mult: 1.2,
                knockback: 3.0,
                hitbox_size: [1.2, 1.2, 2.0],
                hitbox_offset: [1.6, 0.3, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 8.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Rising Strike".into(),
                windup: 0.30,
                active: 0.15,
                recovery: 0.45,
                damage_mult: 1.8,
                knockback: 6.0,
                hitbox_size: [1.0, 2.0, 1.5],
                hitbox_offset: [1.5, 0.5, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 15.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![
            ComboAttack {
                name: "Air Slash".into(),
                windup: 0.15,
                active: 0.10,
                recovery: 0.20,
                damage_mult: 0.8,
                knockback: 1.0,
                hitbox_size: [1.5, 1.0, 1.5],
                hitbox_offset: [1.3, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 3.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Plunging Strike".into(),
                windup: 0.10,
                active: 0.20,
                recovery: 0.35,
                damage_mult: 2.0,
                knockback: 10.0,
                hitbox_size: [2.0, 3.0, 2.0],
                hitbox_offset: [0.0, -2.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 20.0,
                    ..Default::default()
                },
            },
        ],
    }
}

pub fn greatsword() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Greatsword,
        name: "Tower Greatsword".into(),
        base_damage: 60.0,
        combo_chain: vec![
            ComboAttack {
                name: "Heavy Sweep".into(),
                windup: 0.45,
                active: 0.15,
                recovery: 0.50,
                damage_mult: 1.0,
                knockback: 8.0,
                hitbox_size: [3.0, 1.5, 2.5],
                hitbox_offset: [2.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 15.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Overhead Smash".into(),
                windup: 0.55,
                active: 0.20,
                recovery: 0.60,
                damage_mult: 2.0,
                knockback: 12.0,
                hitbox_size: [2.0, 3.0, 2.0],
                hitbox_offset: [1.8, 0.5, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 25.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![ComboAttack {
            name: "Aerial Slam".into(),
            windup: 0.20,
            active: 0.25,
            recovery: 0.40,
            damage_mult: 2.5,
            knockback: 15.0,
            hitbox_size: [3.0, 4.0, 3.0],
            hitbox_offset: [0.0, -3.0, 0.0],
            resource_cost: ResourceCost {
                kinetic: 30.0,
                ..Default::default()
            },
        }],
    }
}

pub fn dual_daggers() -> Weapon {
    Weapon {
        weapon_type: WeaponType::DualDaggers,
        name: "Shadow Daggers".into(),
        base_damage: 15.0,
        combo_chain: vec![
            ComboAttack {
                name: "Left Jab".into(),
                windup: 0.10,
                active: 0.08,
                recovery: 0.12,
                damage_mult: 0.8,
                knockback: 1.0,
                hitbox_size: [0.8, 0.6, 1.2],
                hitbox_offset: [1.2, 0.0, -0.3],
                resource_cost: ResourceCost {
                    kinetic: 2.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Right Cross".into(),
                windup: 0.10,
                active: 0.08,
                recovery: 0.12,
                damage_mult: 0.8,
                knockback: 1.0,
                hitbox_size: [0.8, 0.6, 1.2],
                hitbox_offset: [1.2, 0.0, 0.3],
                resource_cost: ResourceCost {
                    kinetic: 2.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Double Thrust".into(),
                windup: 0.12,
                active: 0.10,
                recovery: 0.15,
                damage_mult: 1.0,
                knockback: 2.0,
                hitbox_size: [0.6, 0.6, 1.5],
                hitbox_offset: [1.5, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 3.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Spinning Slash".into(),
                windup: 0.15,
                active: 0.12,
                recovery: 0.20,
                damage_mult: 1.5,
                knockback: 4.0,
                hitbox_size: [2.0, 1.0, 2.0],
                hitbox_offset: [0.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 8.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Backstab Finisher".into(),
                windup: 0.20,
                active: 0.15,
                recovery: 0.30,
                damage_mult: 2.5,
                knockback: 6.0,
                hitbox_size: [0.5, 0.8, 1.0],
                hitbox_offset: [1.8, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 12.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![
            ComboAttack {
                name: "Air Flurry".into(),
                windup: 0.08,
                active: 0.06,
                recovery: 0.10,
                damage_mult: 0.6,
                knockback: 0.5,
                hitbox_size: [1.0, 0.8, 1.0],
                hitbox_offset: [1.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 1.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Air Flurry 2".into(),
                windup: 0.08,
                active: 0.06,
                recovery: 0.10,
                damage_mult: 0.6,
                knockback: 0.5,
                hitbox_size: [1.0, 0.8, 1.0],
                hitbox_offset: [1.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 1.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Dive Slash".into(),
                windup: 0.10,
                active: 0.15,
                recovery: 0.25,
                damage_mult: 1.5,
                knockback: 5.0,
                hitbox_size: [1.5, 2.5, 1.5],
                hitbox_offset: [0.0, -1.5, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 10.0,
                    ..Default::default()
                },
            },
        ],
    }
}

pub fn staff() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Staff,
        name: "Semantic Staff".into(),
        base_damage: 20.0,
        combo_chain: vec![
            ComboAttack {
                name: "Arcane Pulse".into(),
                windup: 0.35,
                active: 0.15,
                recovery: 0.25,
                damage_mult: 1.0,
                knockback: 3.0,
                hitbox_size: [2.0, 2.0, 4.0],
                hitbox_offset: [3.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    semantic: 10.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Semantic Burst".into(),
                windup: 0.40,
                active: 0.20,
                recovery: 0.35,
                damage_mult: 1.5,
                knockback: 5.0,
                hitbox_size: [3.0, 3.0, 3.0],
                hitbox_offset: [0.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    semantic: 20.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![ComboAttack {
            name: "Hovering Bolt".into(),
            windup: 0.20,
            active: 0.10,
            recovery: 0.20,
            damage_mult: 0.8,
            knockback: 2.0,
            hitbox_size: [1.5, 1.5, 5.0],
            hitbox_offset: [4.0, 0.0, 0.0],
            resource_cost: ResourceCost {
                semantic: 8.0,
                thermal: 5.0,
                ..Default::default()
            },
        }],
    }
}

pub fn spear() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Spear,
        name: "Tower Spear".into(),
        base_damage: 35.0,
        combo_chain: vec![
            ComboAttack {
                name: "Quick Thrust".into(),
                windup: 0.20,
                active: 0.08,
                recovery: 0.25,
                damage_mult: 0.9,
                knockback: 4.0,
                hitbox_size: [0.6, 0.6, 3.0],
                hitbox_offset: [2.5, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 6.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Sweep".into(),
                windup: 0.30,
                active: 0.12,
                recovery: 0.30,
                damage_mult: 1.1,
                knockback: 5.0,
                hitbox_size: [2.5, 0.8, 2.5],
                hitbox_offset: [1.5, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 10.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Impale".into(),
                windup: 0.35,
                active: 0.15,
                recovery: 0.40,
                damage_mult: 2.0,
                knockback: 8.0,
                hitbox_size: [0.5, 0.5, 4.0],
                hitbox_offset: [3.0, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 18.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![ComboAttack {
            name: "Falling Lance".into(),
            windup: 0.15,
            active: 0.20,
            recovery: 0.30,
            damage_mult: 2.2,
            knockback: 12.0,
            hitbox_size: [0.8, 4.0, 0.8],
            hitbox_offset: [0.0, -2.5, 0.0],
            resource_cost: ResourceCost {
                kinetic: 22.0,
                ..Default::default()
            },
        }],
    }
}

pub fn gauntlets() -> Weapon {
    Weapon {
        weapon_type: WeaponType::Gauntlets,
        name: "Iron Gauntlets".into(),
        base_damage: 22.0,
        combo_chain: vec![
            ComboAttack {
                name: "Left Hook".into(),
                windup: 0.12,
                active: 0.06,
                recovery: 0.10,
                damage_mult: 0.7,
                knockback: 2.0,
                hitbox_size: [1.0, 0.8, 1.0],
                hitbox_offset: [1.0, 0.0, -0.3],
                resource_cost: ResourceCost {
                    kinetic: 3.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Right Straight".into(),
                windup: 0.12,
                active: 0.06,
                recovery: 0.10,
                damage_mult: 0.7,
                knockback: 2.0,
                hitbox_size: [1.0, 0.8, 1.0],
                hitbox_offset: [1.0, 0.0, 0.3],
                resource_cost: ResourceCost {
                    kinetic: 3.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Body Blow".into(),
                windup: 0.15,
                active: 0.08,
                recovery: 0.15,
                damage_mult: 1.0,
                knockback: 3.0,
                hitbox_size: [0.8, 1.0, 1.2],
                hitbox_offset: [1.2, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 5.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Uppercut".into(),
                windup: 0.18,
                active: 0.10,
                recovery: 0.20,
                damage_mult: 1.5,
                knockback: 8.0,
                hitbox_size: [0.8, 2.0, 0.8],
                hitbox_offset: [1.0, 1.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 10.0,
                    thermal: 5.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Meteor Fist".into(),
                windup: 0.25,
                active: 0.12,
                recovery: 0.35,
                damage_mult: 2.5,
                knockback: 15.0,
                hitbox_size: [1.5, 1.5, 1.5],
                hitbox_offset: [1.5, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 20.0,
                    thermal: 10.0,
                    ..Default::default()
                },
            },
        ],
        aerial_chain: vec![
            ComboAttack {
                name: "Air Jab".into(),
                windup: 0.08,
                active: 0.05,
                recovery: 0.08,
                damage_mult: 0.5,
                knockback: 1.0,
                hitbox_size: [0.8, 0.6, 0.8],
                hitbox_offset: [0.8, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 2.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Air Jab 2".into(),
                windup: 0.08,
                active: 0.05,
                recovery: 0.08,
                damage_mult: 0.5,
                knockback: 1.0,
                hitbox_size: [0.8, 0.6, 0.8],
                hitbox_offset: [0.8, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 2.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Air Jab 3".into(),
                windup: 0.08,
                active: 0.05,
                recovery: 0.08,
                damage_mult: 0.6,
                knockback: 1.5,
                hitbox_size: [0.8, 0.6, 0.8],
                hitbox_offset: [0.8, 0.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 2.0,
                    ..Default::default()
                },
            },
            ComboAttack {
                name: "Gravity Slam".into(),
                windup: 0.12,
                active: 0.18,
                recovery: 0.30,
                damage_mult: 3.0,
                knockback: 20.0,
                hitbox_size: [2.5, 4.0, 2.5],
                hitbox_offset: [0.0, -3.0, 0.0],
                resource_cost: ResourceCost {
                    kinetic: 25.0,
                    thermal: 15.0,
                    ..Default::default()
                },
            },
        ],
    }
}

/// System: advance combo based on weapon timing
pub fn weapon_combo_system(
    mut query: Query<(&mut CombatState, &EquippedWeapon, &mut CombatResources)>,
) {
    for (mut state, equipped, mut resources) in &mut query {
        if state.phase != AttackPhase::Windup || state.phase_timer > 0.01 {
            continue;
        }

        // Check resource cost
        if let Some(attack) = equipped
            .weapon
            .current_attack(state.combo_step, equipped.is_aerial)
        {
            let cost = &attack.resource_cost;
            if resources.kinetic_energy >= cost.kinetic
                && resources.thermal_energy >= cost.thermal
                && resources.semantic_energy >= cost.semantic
            {
                resources.kinetic_energy -= cost.kinetic;
                resources.thermal_energy -= cost.thermal;
                resources.semantic_energy -= cost.semantic;
            } else {
                // Not enough resources — cancel attack
                state.phase = AttackPhase::Idle;
                state.combo_step = 0;
            }
        }
    }
}

/// Resource regeneration system
pub fn regenerate_resources(time: Res<Time>, mut query: Query<&mut CombatResources>) {
    let dt = time.delta_secs();
    for mut res in &mut query {
        // Kinetic: regens from movement (handled elsewhere), slow passive regen
        res.kinetic_energy = (res.kinetic_energy + 5.0 * dt).min(100.0);
        // Thermal: regens from hovering/defending, slow passive
        res.thermal_energy = (res.thermal_energy + 3.0 * dt).min(100.0);
        // Semantic: regens from analyzing (reading tags), very slow passive
        res.semantic_energy = (res.semantic_energy + 1.0 * dt).min(100.0);
        // Rage: slowly decays
        res.rage = (res.rage - 2.0 * dt).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sword_combo_chain() {
        let w = sword();
        assert_eq!(w.max_combo(), 3);
        assert_eq!(w.max_aerial_combo(), 2);
        assert!(w.current_attack(0, false).is_some());
        assert!(w.current_attack(2, false).is_some());
        assert!(w.current_attack(3, false).is_none());
    }

    #[test]
    fn test_greatsword_high_damage() {
        let gs = greatsword();
        let s = sword();
        assert!(gs.base_damage > s.base_damage);
    }

    #[test]
    fn test_daggers_long_combo() {
        let d = dual_daggers();
        assert!(d.max_combo() >= 4, "Daggers should have long combos");
    }

    #[test]
    fn test_staff_semantic_cost() {
        let s = staff();
        let attack = s.current_attack(0, false).unwrap();
        assert!(
            attack.resource_cost.semantic > 0.0,
            "Staff should cost semantic energy"
        );
    }

    #[test]
    fn test_weapon_types_unique_timing() {
        let s = sword();
        let gs = greatsword();
        let d = dual_daggers();

        let s_windup = s.combo_chain[0].windup;
        let gs_windup = gs.combo_chain[0].windup;
        let d_windup = d.combo_chain[0].windup;

        assert!(d_windup < s_windup, "Daggers should be faster than sword");
        assert!(
            s_windup < gs_windup,
            "Sword should be faster than greatsword"
        );
    }

    #[test]
    fn test_spear_long_reach() {
        let sp = spear();
        let s = sword();
        assert_eq!(sp.max_combo(), 3);
        // Spear hitbox offset should be further than sword
        assert!(
            sp.combo_chain[0].hitbox_offset[0] > s.combo_chain[0].hitbox_offset[0],
            "Spear should have longer reach"
        );
    }

    #[test]
    fn test_gauntlets_long_aerial_combo() {
        let g = gauntlets();
        assert_eq!(g.max_combo(), 5, "Gauntlets should have 5-hit ground combo");
        assert_eq!(
            g.max_aerial_combo(),
            4,
            "Gauntlets should have 4-hit aerial combo"
        );
    }

    #[test]
    fn test_gauntlets_fast_windups() {
        let g = gauntlets();
        let s = sword();
        assert!(
            g.combo_chain[0].windup < s.combo_chain[0].windup,
            "Gauntlets should be faster than sword"
        );
    }

    #[test]
    fn test_all_weapons_have_chains() {
        let weapons = vec![
            sword(),
            greatsword(),
            dual_daggers(),
            spear(),
            gauntlets(),
            staff(),
        ];
        for w in &weapons {
            assert!(
                w.max_combo() >= 1,
                "{} should have at least 1 ground attack",
                w.name
            );
            assert!(
                w.max_aerial_combo() >= 1,
                "{} should have at least 1 aerial attack",
                w.name
            );
        }
    }
}
