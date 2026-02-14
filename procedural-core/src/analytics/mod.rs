//! Analytics & Telemetry Module
//!
//! Collects gameplay metrics for balancing and analysis:
//! - Combat events (damage, kills, deaths, parries)
//! - Floor progression (clears, time, completion %)
//! - Equipment usage (weapon types, sockets, abilities)
//! - Economic activity (gold, crafting, trading)
//! - Player behavior (APM, skill usage patterns)
//!
//! Use cases:
//! - Monte-Carlo balance simulations
//! - Meta build detection
//! - Exploit identification
//! - Player progression analysis

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub struct AnalyticsPlugin;

impl Plugin for AnalyticsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AnalyticsCollector::default())
            .add_event::<AnalyticsEvent>()
            .add_systems(Update, process_analytics_events);
    }
}

/// Global analytics collector resource
#[derive(Resource, Default)]
pub struct AnalyticsCollector {
    pub combat_stats: CombatStats,
    pub progression_stats: ProgressionStats,
    pub equipment_stats: EquipmentStats,
    pub economy_stats: EconomyStats,
    pub behavior_stats: BehaviorStats,
    pub session_start_time: f64,
}

/// Combat statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CombatStats {
    pub total_damage_dealt: u64,
    pub total_damage_taken: u64,
    pub kills_by_weapon: HashMap<String, u32>,
    pub deaths_by_floor_tier: HashMap<u8, u32>,
    pub successful_parries: u32,
    pub failed_parries: u32,
    pub dodges: u32,
    pub ability_uses: HashMap<String, u32>,
}

/// Progression statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressionStats {
    pub floors_cleared: u32,
    pub highest_floor: u32,
    pub total_playtime_secs: f64,
    pub average_floor_clear_time: f64,
    pub floors_by_tier: HashMap<u8, u32>,
    pub total_rooms_explored: u32,
    pub secrets_found: u32,
}

/// Equipment usage statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EquipmentStats {
    pub weapon_type_usage: HashMap<String, f64>, // % time used
    pub socket_gems_used: HashMap<String, u32>,
    pub set_bonuses_active: HashMap<String, u32>,
    pub equipment_slots_filled: u8,
}

/// Economic statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EconomyStats {
    pub gold_earned: u64,
    pub gold_spent: u64,
    pub items_crafted: u32,
    pub items_sold: u32,
    pub items_bought: u32,
    pub tax_paid: u64,
}

/// Behavior statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorStats {
    pub actions_per_minute: f32,
    pub total_actions: u32,
    pub skill_rotation_diversity: f32, // Shannon entropy
    pub average_combat_duration: f64,
    pub rest_time_percent: f32,
}

/// Analytics event types
#[derive(Event, Debug, Clone)]
pub enum AnalyticsEvent {
    CombatDamageDealt {
        weapon: String,
        amount: u32,
    },
    CombatDamageTaken {
        amount: u32,
    },
    CombatKill {
        weapon: String,
        floor_tier: u8,
    },
    CombatDeath {
        floor_tier: u8,
    },
    CombatParry {
        success: bool,
    },
    CombatDodge,
    CombatAbilityUsed {
        ability: String,
    },
    FloorCleared {
        floor_id: u32,
        tier: u8,
        time_secs: f64,
    },
    RoomExplored,
    SecretFound,
    WeaponSwitched {
        weapon_type: String,
    },
    GoldEarned {
        amount: u64,
    },
    GoldSpent {
        amount: u64,
    },
    ItemCrafted,
    ItemTraded {
        bought: bool,
    },
    Action,
}

impl AnalyticsCollector {
    pub fn record_event(&mut self, event: &AnalyticsEvent) {
        match event {
            AnalyticsEvent::CombatDamageDealt { weapon, amount } => {
                self.combat_stats.total_damage_dealt += *amount as u64;
                *self
                    .combat_stats
                    .kills_by_weapon
                    .entry(weapon.clone())
                    .or_insert(0) += 0;
            }
            AnalyticsEvent::CombatDamageTaken { amount } => {
                self.combat_stats.total_damage_taken += *amount as u64;
            }
            AnalyticsEvent::CombatKill { weapon, floor_tier } => {
                *self
                    .combat_stats
                    .kills_by_weapon
                    .entry(weapon.clone())
                    .or_insert(0) += 1;
                *self
                    .combat_stats
                    .deaths_by_floor_tier
                    .entry(*floor_tier)
                    .or_insert(0) += 0;
            }
            AnalyticsEvent::CombatDeath { floor_tier } => {
                *self
                    .combat_stats
                    .deaths_by_floor_tier
                    .entry(*floor_tier)
                    .or_insert(0) += 1;
            }
            AnalyticsEvent::CombatParry { success } => {
                if *success {
                    self.combat_stats.successful_parries += 1;
                } else {
                    self.combat_stats.failed_parries += 1;
                }
            }
            AnalyticsEvent::CombatDodge => {
                self.combat_stats.dodges += 1;
            }
            AnalyticsEvent::CombatAbilityUsed { ability } => {
                *self
                    .combat_stats
                    .ability_uses
                    .entry(ability.clone())
                    .or_insert(0) += 1;
            }
            AnalyticsEvent::FloorCleared {
                floor_id,
                tier,
                time_secs,
            } => {
                self.progression_stats.floors_cleared += 1;
                if *floor_id > self.progression_stats.highest_floor {
                    self.progression_stats.highest_floor = *floor_id;
                }
                *self
                    .progression_stats
                    .floors_by_tier
                    .entry(*tier)
                    .or_insert(0) += 1;

                // Update average clear time
                let total = self.progression_stats.average_floor_clear_time
                    * (self.progression_stats.floors_cleared - 1) as f64
                    + time_secs;
                self.progression_stats.average_floor_clear_time =
                    total / self.progression_stats.floors_cleared as f64;
            }
            AnalyticsEvent::RoomExplored => {
                self.progression_stats.total_rooms_explored += 1;
            }
            AnalyticsEvent::SecretFound => {
                self.progression_stats.secrets_found += 1;
            }
            AnalyticsEvent::WeaponSwitched { weapon_type } => {
                *self
                    .equipment_stats
                    .weapon_type_usage
                    .entry(weapon_type.clone())
                    .or_insert(0.0) += 1.0;
            }
            AnalyticsEvent::GoldEarned { amount } => {
                self.economy_stats.gold_earned += amount;
            }
            AnalyticsEvent::GoldSpent { amount } => {
                self.economy_stats.gold_spent += amount;
            }
            AnalyticsEvent::ItemCrafted => {
                self.economy_stats.items_crafted += 1;
            }
            AnalyticsEvent::ItemTraded { bought } => {
                if *bought {
                    self.economy_stats.items_bought += 1;
                } else {
                    self.economy_stats.items_sold += 1;
                }
            }
            AnalyticsEvent::Action => {
                self.behavior_stats.total_actions += 1;
            }
        }
    }

    pub fn compute_derived_stats(&mut self, current_time: f64) {
        // Compute playtime
        self.progression_stats.total_playtime_secs = current_time - self.session_start_time;

        // Compute APM
        if self.progression_stats.total_playtime_secs > 0.0 {
            self.behavior_stats.actions_per_minute = (self.behavior_stats.total_actions as f64
                / self.progression_stats.total_playtime_secs
                * 60.0) as f32;
        }

        // Compute skill rotation diversity (Shannon entropy)
        if !self.combat_stats.ability_uses.is_empty() {
            let total: u32 = self.combat_stats.ability_uses.values().sum();
            if total > 0 {
                let mut entropy = 0.0_f32;
                for &count in self.combat_stats.ability_uses.values() {
                    let p = count as f32 / total as f32;
                    if p > 0.0 {
                        entropy -= p * p.log2();
                    }
                }
                self.behavior_stats.skill_rotation_diversity = entropy;
            }
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// System to process analytics events
fn process_analytics_events(
    mut collector: ResMut<AnalyticsCollector>,
    mut events: EventReader<AnalyticsEvent>,
    time: Res<Time>,
) {
    for event in events.read() {
        collector.record_event(event);
    }

    // Update derived stats every frame
    collector.compute_derived_stats(time.elapsed_secs_f64());
}

/// Analytics snapshot for FFI
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsSnapshot {
    pub combat: CombatStats,
    pub progression: ProgressionStats,
    pub equipment: EquipmentStats,
    pub economy: EconomyStats,
    pub behavior: BehaviorStats,
}

impl AnalyticsSnapshot {
    pub fn capture(collector: &AnalyticsCollector) -> Self {
        Self {
            combat: collector.combat_stats.clone(),
            progression: collector.progression_stats.clone(),
            equipment: collector.equipment_stats.clone(),
            economy: collector.economy_stats.clone(),
            behavior: collector.behavior_stats.clone(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combat_damage_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatDamageDealt {
            weapon: "Sword".to_string(),
            amount: 100,
        });
        collector.record_event(&AnalyticsEvent::CombatDamageTaken { amount: 50 });

        assert_eq!(collector.combat_stats.total_damage_dealt, 100);
        assert_eq!(collector.combat_stats.total_damage_taken, 50);
    }

    #[test]
    fn test_combat_kill_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatKill {
            weapon: "Sword".to_string(),
            floor_tier: 1,
        });
        collector.record_event(&AnalyticsEvent::CombatKill {
            weapon: "Sword".to_string(),
            floor_tier: 1,
        });
        collector.record_event(&AnalyticsEvent::CombatKill {
            weapon: "Bow".to_string(),
            floor_tier: 2,
        });

        assert_eq!(
            collector.combat_stats.kills_by_weapon.get("Sword"),
            Some(&2)
        );
        assert_eq!(collector.combat_stats.kills_by_weapon.get("Bow"), Some(&1));
    }

    #[test]
    fn test_parry_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatParry { success: true });
        collector.record_event(&AnalyticsEvent::CombatParry { success: true });
        collector.record_event(&AnalyticsEvent::CombatParry { success: false });

        assert_eq!(collector.combat_stats.successful_parries, 2);
        assert_eq!(collector.combat_stats.failed_parries, 1);
    }

    #[test]
    fn test_floor_cleared_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::FloorCleared {
            floor_id: 1,
            tier: 1,
            time_secs: 120.0,
        });
        collector.record_event(&AnalyticsEvent::FloorCleared {
            floor_id: 2,
            tier: 1,
            time_secs: 180.0,
        });

        assert_eq!(collector.progression_stats.floors_cleared, 2);
        assert_eq!(collector.progression_stats.highest_floor, 2);
        assert_eq!(collector.progression_stats.average_floor_clear_time, 150.0);
    }

    #[test]
    fn test_economy_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::GoldEarned { amount: 1000 });
        collector.record_event(&AnalyticsEvent::GoldSpent { amount: 300 });
        collector.record_event(&AnalyticsEvent::ItemCrafted);

        assert_eq!(collector.economy_stats.gold_earned, 1000);
        assert_eq!(collector.economy_stats.gold_spent, 300);
        assert_eq!(collector.economy_stats.items_crafted, 1);
    }

    #[test]
    fn test_apm_calculation() {
        let mut collector = AnalyticsCollector::default();
        collector.session_start_time = 0.0;

        for _ in 0..120 {
            collector.record_event(&AnalyticsEvent::Action);
        }

        collector.compute_derived_stats(60.0); // 60 seconds elapsed

        assert_eq!(collector.behavior_stats.total_actions, 120);
        assert!((collector.behavior_stats.actions_per_minute - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_skill_diversity_entropy() {
        let mut collector = AnalyticsCollector::default();

        // Use 2 abilities equally → high entropy
        for _ in 0..10 {
            collector.record_event(&AnalyticsEvent::CombatAbilityUsed {
                ability: "Fireball".to_string(),
            });
            collector.record_event(&AnalyticsEvent::CombatAbilityUsed {
                ability: "IceBlast".to_string(),
            });
        }

        collector.compute_derived_stats(1.0);

        assert!(collector.behavior_stats.skill_rotation_diversity > 0.9);
    }

    #[test]
    fn test_analytics_snapshot_json() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatKill {
            weapon: "Sword".to_string(),
            floor_tier: 1,
        });

        let snapshot = AnalyticsSnapshot::capture(&collector);
        let json = snapshot.to_json();
        assert!(json.contains("\"combat\""));
        assert!(json.contains("\"progression\""));

        let restored = AnalyticsSnapshot::from_json(&json).unwrap();
        assert_eq!(restored.combat.kills_by_weapon.get("Sword"), Some(&1));
    }

    #[test]
    fn test_reset_analytics() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatDamageDealt {
            weapon: "Sword".to_string(),
            amount: 500,
        });

        assert_eq!(collector.combat_stats.total_damage_dealt, 500);

        collector.reset();

        assert_eq!(collector.combat_stats.total_damage_dealt, 0);
    }

    #[test]
    fn test_weapon_switch_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::WeaponSwitched {
            weapon_type: "Sword".to_string(),
        });
        collector.record_event(&AnalyticsEvent::WeaponSwitched {
            weapon_type: "Bow".to_string(),
        });
        collector.record_event(&AnalyticsEvent::WeaponSwitched {
            weapon_type: "Sword".to_string(),
        });

        assert_eq!(
            collector.equipment_stats.weapon_type_usage.get("Sword"),
            Some(&2.0)
        );
        assert_eq!(
            collector.equipment_stats.weapon_type_usage.get("Bow"),
            Some(&1.0)
        );
    }

    #[test]
    fn test_secrets_and_rooms() {
        let mut collector = AnalyticsCollector::default();
        for _ in 0..5 {
            collector.record_event(&AnalyticsEvent::RoomExplored);
        }
        for _ in 0..2 {
            collector.record_event(&AnalyticsEvent::SecretFound);
        }

        assert_eq!(collector.progression_stats.total_rooms_explored, 5);
        assert_eq!(collector.progression_stats.secrets_found, 2);
    }

    #[test]
    fn test_death_tracking_by_tier() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::CombatDeath { floor_tier: 1 });
        collector.record_event(&AnalyticsEvent::CombatDeath { floor_tier: 1 });
        collector.record_event(&AnalyticsEvent::CombatDeath { floor_tier: 3 });

        assert_eq!(
            collector.combat_stats.deaths_by_floor_tier.get(&1),
            Some(&2)
        );
        assert_eq!(
            collector.combat_stats.deaths_by_floor_tier.get(&3),
            Some(&1)
        );
    }

    #[test]
    fn test_trading_tracking() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::ItemTraded { bought: true });
        collector.record_event(&AnalyticsEvent::ItemTraded { bought: true });
        collector.record_event(&AnalyticsEvent::ItemTraded { bought: false });

        assert_eq!(collector.economy_stats.items_bought, 2);
        assert_eq!(collector.economy_stats.items_sold, 1);
    }

    #[test]
    fn test_floor_tier_distribution() {
        let mut collector = AnalyticsCollector::default();
        collector.record_event(&AnalyticsEvent::FloorCleared {
            floor_id: 1,
            tier: 1,
            time_secs: 60.0,
        });
        collector.record_event(&AnalyticsEvent::FloorCleared {
            floor_id: 5,
            tier: 2,
            time_secs: 90.0,
        });
        collector.record_event(&AnalyticsEvent::FloorCleared {
            floor_id: 10,
            tier: 3,
            time_secs: 120.0,
        });

        assert_eq!(collector.progression_stats.floors_by_tier.get(&1), Some(&1));
        assert_eq!(collector.progression_stats.floors_by_tier.get(&2), Some(&1));
        assert_eq!(collector.progression_stats.floors_by_tier.get(&3), Some(&1));
    }
}
