//! Achievement System
//!
//! Semantic and historical achievements. Achievements are not just
//! "kill 100 monsters" — they're based on semantic patterns:
//! - Discover 5 fire-water synergy interactions
//! - Survive a Corruption Surge on floor 30+
//! - Complete a floor using only semantic attacks
//!
//! From opensourcestack.txt Category 16:
//! "Semantic and historical achievements"

use serde::{Deserialize, Serialize};

/// Achievement categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementCategory {
    Combat,      // fighting accomplishments
    Exploration, // floor/tower milestones
    Semantic,    // tag-based interactions
    Social,      // faction/multiplayer
    Crafting,    // crafting milestones
    Survival,    // death/echo related
    Mastery,     // weapon/skill mastery
    Tower,       // tower-specific (breath, events)
}

/// Achievement tier (determines reward quality)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Mythic,
}

/// Condition type for tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AchievementCondition {
    /// Reach a numeric threshold
    Counter { current: u64, target: u64 },
    /// Achieve something in a single run
    SingleRun { achieved: bool },
    /// Complete multiple sub-conditions
    Composite {
        completed: Vec<bool>,
        names: Vec<String>,
    },
    /// Floor-gated: must be on floor X+
    FloorGated { min_floor: u32, met: bool },
    /// Semantic: requires specific tag interactions
    SemanticPattern {
        required_tags: Vec<String>,
        matched: bool,
    },
    /// Time-limited challenge
    TimedChallenge {
        time_limit_secs: f32,
        elapsed: f32,
        completed: bool,
    },
}

impl AchievementCondition {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::Counter { current, target } => current >= target,
            Self::SingleRun { achieved } => *achieved,
            Self::Composite { completed, .. } => completed.iter().all(|c| *c),
            Self::FloorGated { met, .. } => *met,
            Self::SemanticPattern { matched, .. } => *matched,
            Self::TimedChallenge { completed, .. } => *completed,
        }
    }

    pub fn progress_percent(&self) -> f32 {
        match self {
            Self::Counter { current, target } => {
                if *target == 0 {
                    1.0
                } else {
                    (*current as f32 / *target as f32).min(1.0)
                }
            }
            Self::SingleRun { achieved } => {
                if *achieved {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Composite { completed, .. } => {
                let done = completed.iter().filter(|c| **c).count() as f32;
                done / completed.len().max(1) as f32
            }
            Self::FloorGated { met, .. } => {
                if *met {
                    1.0
                } else {
                    0.0
                }
            }
            Self::SemanticPattern { matched, .. } => {
                if *matched {
                    1.0
                } else {
                    0.0
                }
            }
            Self::TimedChallenge {
                completed,
                elapsed,
                time_limit_secs,
                ..
            } => {
                if *completed {
                    1.0
                } else {
                    (elapsed / time_limit_secs).min(0.99)
                }
            }
        }
    }
}

/// A single achievement definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub hint: String,
    pub category: AchievementCategory,
    pub tier: AchievementTier,
    pub condition: AchievementCondition,
    pub hidden: bool, // don't show until discovered
    pub reward_shards: u64,
    pub unlocked: bool,
    pub unlock_timestamp: Option<u64>,
}

impl Achievement {
    pub fn check_and_unlock(&mut self, timestamp: u64) -> bool {
        if self.unlocked {
            return false;
        }
        if self.condition.is_complete() {
            self.unlocked = true;
            self.unlock_timestamp = Some(timestamp);
            return true;
        }
        false
    }
}

/// The player's achievement tracker
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AchievementTracker {
    pub achievements: Vec<Achievement>,
    pub total_unlocked: u32,
    pub total_shards_earned: u64,
}

impl AchievementTracker {
    /// Initialize with all game achievements
    pub fn new() -> Self {
        Self {
            achievements: all_achievements(),
            ..Default::default()
        }
    }

    /// Increment a counter-based achievement
    pub fn increment_counter(&mut self, achievement_id: &str, amount: u64) {
        for ach in &mut self.achievements {
            if ach.id == achievement_id {
                if let AchievementCondition::Counter { current, .. } = &mut ach.condition {
                    *current += amount;
                }
            }
        }
    }

    /// Set a single-run achievement as achieved
    pub fn mark_achieved(&mut self, achievement_id: &str) {
        for ach in &mut self.achievements {
            if ach.id == achievement_id {
                if let AchievementCondition::SingleRun { achieved } = &mut ach.condition {
                    *achieved = true;
                }
            }
        }
    }

    /// Set a composite sub-condition
    pub fn complete_sub(&mut self, achievement_id: &str, sub_index: usize) {
        for ach in &mut self.achievements {
            if ach.id == achievement_id {
                if let AchievementCondition::Composite { completed, .. } = &mut ach.condition {
                    if sub_index < completed.len() {
                        completed[sub_index] = true;
                    }
                }
            }
        }
    }

    /// Mark floor-gated achievement
    pub fn check_floor_gate(&mut self, achievement_id: &str, current_floor: u32) {
        for ach in &mut self.achievements {
            if ach.id == achievement_id {
                if let AchievementCondition::FloorGated { min_floor, met } = &mut ach.condition {
                    if current_floor >= *min_floor {
                        *met = true;
                    }
                }
            }
        }
    }

    /// Check all achievements and return newly unlocked ones
    pub fn check_all(&mut self, timestamp: u64) -> Vec<Achievement> {
        let mut newly_unlocked = Vec::new();
        for ach in &mut self.achievements {
            if ach.check_and_unlock(timestamp) {
                newly_unlocked.push(ach.clone());
            }
        }
        for ach in &newly_unlocked {
            self.total_unlocked += 1;
            self.total_shards_earned += ach.reward_shards;
        }
        newly_unlocked
    }

    /// Get achievements by category
    pub fn by_category(&self, category: AchievementCategory) -> Vec<&Achievement> {
        self.achievements
            .iter()
            .filter(|a| a.category == category)
            .collect()
    }

    /// Overall completion percentage
    pub fn completion_percent(&self) -> f32 {
        if self.achievements.is_empty() {
            return 0.0;
        }
        let unlocked = self.achievements.iter().filter(|a| a.unlocked).count() as f32;
        unlocked / self.achievements.len() as f32
    }

    /// Serialize to JSON for Nakama storage
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// All game achievements
fn all_achievements() -> Vec<Achievement> {
    vec![
        // === Combat ===
        Achievement {
            id: "combat_first_kill".into(),
            name: "First Blood".into(),
            description: "Slay your first monster.".into(),
            hint: "Attack a monster until it falls.".into(),
            category: AchievementCategory::Combat,
            tier: AchievementTier::Bronze,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 1,
            },
            hidden: false,
            reward_shards: 10,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "combat_100_kills".into(),
            name: "Centurion".into(),
            description: "Slay 100 monsters.".into(),
            hint: "Keep fighting.".into(),
            category: AchievementCategory::Combat,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 100,
            },
            hidden: false,
            reward_shards: 50,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "combat_perfect_parry".into(),
            name: "Perfect Timing".into(),
            description: "Execute 10 perfect parries.".into(),
            hint: "Parry within 80-120ms of an incoming attack.".into(),
            category: AchievementCategory::Combat,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 10,
            },
            hidden: false,
            reward_shards: 100,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "combat_all_weapons".into(),
            name: "Arsenal Master".into(),
            description: "Use all 6 weapon types in combat.".into(),
            hint: "Try every weapon at least once.".into(),
            category: AchievementCategory::Mastery,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::Composite {
                completed: vec![false; 6],
                names: vec![
                    "Sword".into(),
                    "Greatsword".into(),
                    "DualDaggers".into(),
                    "Spear".into(),
                    "Gauntlets".into(),
                    "Staff".into(),
                ],
            },
            hidden: false,
            reward_shards: 75,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Exploration ===
        Achievement {
            id: "explore_floor_10".into(),
            name: "Tower Climber".into(),
            description: "Reach floor 10.".into(),
            hint: "Keep ascending.".into(),
            category: AchievementCategory::Exploration,
            tier: AchievementTier::Bronze,
            condition: AchievementCondition::FloorGated {
                min_floor: 10,
                met: false,
            },
            hidden: false,
            reward_shards: 25,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "explore_floor_50".into(),
            name: "Veteran Climber".into(),
            description: "Reach floor 50.".into(),
            hint: "The tower grows deeper.".into(),
            category: AchievementCategory::Exploration,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::FloorGated {
                min_floor: 50,
                met: false,
            },
            hidden: false,
            reward_shards: 100,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "explore_floor_100".into(),
            name: "Echelon Breaker".into(),
            description: "Break through to Echelon 2 (floor 101).".into(),
            hint: "Master the first hundred floors.".into(),
            category: AchievementCategory::Exploration,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::FloorGated {
                min_floor: 101,
                met: false,
            },
            hidden: false,
            reward_shards: 250,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Semantic ===
        Achievement {
            id: "semantic_synergy".into(),
            name: "Elemental Synergy".into(),
            description: "Trigger 5 semantic synergy bonuses in combat.".into(),
            hint: "Match your element to your attacks.".into(),
            category: AchievementCategory::Semantic,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 5,
            },
            hidden: false,
            reward_shards: 60,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "semantic_resonance_event".into(),
            name: "Resonance Catalyst".into(),
            description: "Trigger a Semantic Resonance world event.".into(),
            hint: "Your tags must align with the floor's.".into(),
            category: AchievementCategory::Semantic,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::SingleRun { achieved: false },
            hidden: true,
            reward_shards: 150,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Survival ===
        Achievement {
            id: "survival_first_death".into(),
            name: "Welcome to the Tower".into(),
            description: "Die for the first time.".into(),
            hint: "Everyone falls eventually.".into(),
            category: AchievementCategory::Survival,
            tier: AchievementTier::Bronze,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 1,
            },
            hidden: false,
            reward_shards: 5,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "survival_corruption_surge".into(),
            name: "Corruption Survivor".into(),
            description: "Survive a Corruption Surge on floor 30+.".into(),
            hint: "Endure when the tower writhes.".into(),
            category: AchievementCategory::Survival,
            tier: AchievementTier::Platinum,
            condition: AchievementCondition::FloorGated {
                min_floor: 30,
                met: false,
            },
            hidden: true,
            reward_shards: 300,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Social ===
        Achievement {
            id: "social_faction_friendly".into(),
            name: "Making Friends".into(),
            description: "Reach Friendly standing with any faction.".into(),
            hint: "Complete quests and interact with shrines.".into(),
            category: AchievementCategory::Social,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::SingleRun { achieved: false },
            hidden: false,
            reward_shards: 50,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "social_all_factions".into(),
            name: "Diplomat".into(),
            description: "Reach Neutral or better with all 4 factions.".into(),
            hint: "Balance your relationships carefully.".into(),
            category: AchievementCategory::Social,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::Composite {
                completed: vec![false; 4],
                names: vec![
                    "Seekers".into(),
                    "Wardens".into(),
                    "Breakers".into(),
                    "Weavers".into(),
                ],
            },
            hidden: false,
            reward_shards: 200,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Crafting ===
        Achievement {
            id: "craft_first".into(),
            name: "Apprentice Crafter".into(),
            description: "Craft your first item.".into(),
            hint: "Visit a crafting station.".into(),
            category: AchievementCategory::Crafting,
            tier: AchievementTier::Bronze,
            condition: AchievementCondition::Counter {
                current: 0,
                target: 1,
            },
            hidden: false,
            reward_shards: 15,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "craft_high_quality".into(),
            name: "Master Craftsman".into(),
            description: "Craft an item with 90%+ quality.".into(),
            hint: "Use materials with highly matching tags.".into(),
            category: AchievementCategory::Crafting,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::SingleRun { achieved: false },
            hidden: false,
            reward_shards: 150,
            unlocked: false,
            unlock_timestamp: None,
        },
        // === Tower ===
        Achievement {
            id: "tower_breath_all_phases".into(),
            name: "Breath Witness".into(),
            description: "Experience all 4 Breath of Tower phases.".into(),
            hint: "Stay in the tower through a full cycle.".into(),
            category: AchievementCategory::Tower,
            tier: AchievementTier::Silver,
            condition: AchievementCondition::Composite {
                completed: vec![false; 4],
                names: vec![
                    "Inhale".into(),
                    "Hold".into(),
                    "Exhale".into(),
                    "Pause".into(),
                ],
            },
            hidden: false,
            reward_shards: 40,
            unlocked: false,
            unlock_timestamp: None,
        },
        Achievement {
            id: "tower_memory_event".into(),
            name: "Remembered".into(),
            description: "Trigger a Tower Memory event.".into(),
            hint: "The tower watches your patterns...".into(),
            category: AchievementCategory::Tower,
            tier: AchievementTier::Gold,
            condition: AchievementCondition::SingleRun { achieved: false },
            hidden: true,
            reward_shards: 200,
            unlocked: false,
            unlock_timestamp: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_initialization() {
        let tracker = AchievementTracker::new();
        assert!(!tracker.achievements.is_empty());
        assert_eq!(tracker.total_unlocked, 0);
        assert!((tracker.completion_percent() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_counter_increment_and_unlock() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("combat_first_kill", 1);

        let newly = tracker.check_all(1000);
        assert_eq!(newly.len(), 1);
        assert_eq!(newly[0].id, "combat_first_kill");
        assert_eq!(tracker.total_unlocked, 1);
        assert!(tracker.total_shards_earned > 0);
    }

    #[test]
    fn test_no_double_unlock() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("combat_first_kill", 1);
        let first = tracker.check_all(1000);
        assert_eq!(first.len(), 1);

        let second = tracker.check_all(2000);
        assert_eq!(second.len(), 0, "Should not unlock twice");
    }

    #[test]
    fn test_floor_gate() {
        let mut tracker = AchievementTracker::new();
        tracker.check_floor_gate("explore_floor_10", 5);
        let none = tracker.check_all(1000);
        assert!(
            none.iter().all(|a| a.id != "explore_floor_10"),
            "Floor 5 < 10"
        );

        tracker.check_floor_gate("explore_floor_10", 15);
        let unlocked = tracker.check_all(2000);
        assert!(unlocked.iter().any(|a| a.id == "explore_floor_10"));
    }

    #[test]
    fn test_composite_achievement() {
        let mut tracker = AchievementTracker::new();

        // Complete 3 out of 6 weapons
        tracker.complete_sub("combat_all_weapons", 0);
        tracker.complete_sub("combat_all_weapons", 1);
        tracker.complete_sub("combat_all_weapons", 2);
        let none = tracker.check_all(1000);
        assert!(none.iter().all(|a| a.id != "combat_all_weapons"));

        // Complete remaining
        tracker.complete_sub("combat_all_weapons", 3);
        tracker.complete_sub("combat_all_weapons", 4);
        tracker.complete_sub("combat_all_weapons", 5);
        let unlocked = tracker.check_all(2000);
        assert!(unlocked.iter().any(|a| a.id == "combat_all_weapons"));
    }

    #[test]
    fn test_single_run_achievement() {
        let mut tracker = AchievementTracker::new();
        tracker.mark_achieved("semantic_resonance_event");
        let unlocked = tracker.check_all(1000);
        assert!(unlocked.iter().any(|a| a.id == "semantic_resonance_event"));
    }

    #[test]
    fn test_by_category() {
        let tracker = AchievementTracker::new();
        let combat = tracker.by_category(AchievementCategory::Combat);
        assert!(combat.len() >= 2, "Should have combat achievements");
        for a in &combat {
            assert_eq!(a.category, AchievementCategory::Combat);
        }
    }

    #[test]
    fn test_progress_percent() {
        assert!(
            (AchievementCondition::Counter {
                current: 50,
                target: 100
            }
            .progress_percent()
                - 0.5)
                .abs()
                < 0.01
        );
        assert!(
            (AchievementCondition::Counter {
                current: 200,
                target: 100
            }
            .progress_percent()
                - 1.0)
                .abs()
                < 0.01
        );
        assert!(
            (AchievementCondition::SingleRun { achieved: false }.progress_percent() - 0.0).abs()
                < 0.01
        );
        assert!(
            (AchievementCondition::SingleRun { achieved: true }.progress_percent() - 1.0).abs()
                < 0.01
        );
    }

    #[test]
    fn test_hidden_achievements_exist() {
        let tracker = AchievementTracker::new();
        let hidden = tracker.achievements.iter().filter(|a| a.hidden).count();
        assert!(hidden >= 2, "Should have hidden achievements");
    }

    #[test]
    fn test_all_tiers_represented() {
        let tracker = AchievementTracker::new();
        let tiers: std::collections::HashSet<_> =
            tracker.achievements.iter().map(|a| a.tier).collect();
        assert!(tiers.contains(&AchievementTier::Bronze));
        assert!(tiers.contains(&AchievementTier::Silver));
        assert!(tiers.contains(&AchievementTier::Gold));
    }

    #[test]
    fn test_serialization() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("combat_first_kill", 1);
        tracker.check_all(1000);

        let json = tracker.to_json();
        assert!(!json.is_empty());
        let restored: AchievementTracker = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.total_unlocked, 1);
    }
}
