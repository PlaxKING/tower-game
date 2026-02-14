//! Seasons, Daily/Weekly Quests, and Season Pass
//!
//! From dopopensource.txt Categories 3, 16, 19:
//! - Daily quests: reset every 24h
//! - Weekly quests: reset every 7d
//! - Season pass: 90-day seasons with reward tracks
//! - Seasonal events with unique content
//!
//! Integrated with Nakama for server-authoritative resets.

use serde::{Deserialize, Serialize};

/// Daily quest status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestResetType {
    Daily,
    Weekly,
    Seasonal,
    OneTime,
}

/// Repeatable quest objective types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DailyObjective {
    KillMonsters { count: u32, current: u32 },
    ClearFloors { count: u32, current: u32 },
    CraftItems { count: u32, current: u32 },
    GatherResources { count: u32, current: u32 },
    CompleteTrades { count: u32, current: u32 },
    ParryAttacks { count: u32, current: u32 },
    UseAbilities { count: u32, current: u32 },
    EarnMasteryXp { amount: u64, current: u64 },
    DiscoverRooms { count: u32, current: u32 },
    DefeatElite { current: u32 },
}

impl DailyObjective {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::KillMonsters { count, current } => current >= count,
            Self::ClearFloors { count, current } => current >= count,
            Self::CraftItems { count, current } => current >= count,
            Self::GatherResources { count, current } => current >= count,
            Self::CompleteTrades { count, current } => current >= count,
            Self::ParryAttacks { count, current } => current >= count,
            Self::UseAbilities { count, current } => current >= count,
            Self::EarnMasteryXp { amount, current } => current >= amount,
            Self::DiscoverRooms { count, current } => current >= count,
            Self::DefeatElite { current } => *current >= 1,
        }
    }

    pub fn progress_percent(&self) -> f32 {
        match self {
            Self::KillMonsters { count, current }
            | Self::ClearFloors { count, current }
            | Self::CraftItems { count, current }
            | Self::GatherResources { count, current }
            | Self::CompleteTrades { count, current }
            | Self::ParryAttacks { count, current }
            | Self::UseAbilities { count, current }
            | Self::DiscoverRooms { count, current } => {
                if *count == 0 {
                    return 1.0;
                }
                (*current as f32 / *count as f32).min(1.0)
            }
            Self::EarnMasteryXp { amount, current } => {
                if *amount == 0 {
                    return 1.0;
                }
                (*current as f32 / *amount as f32).min(1.0)
            }
            Self::DefeatElite { current } => {
                if *current >= 1 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::KillMonsters { count, current } => {
                format!("Kill {} monsters ({}/{})", count, current, count)
            }
            Self::ClearFloors { count, current } => {
                format!("Clear {} floors ({}/{})", count, current, count)
            }
            Self::CraftItems { count, current } => {
                format!("Craft {} items ({}/{})", count, current, count)
            }
            Self::GatherResources { count, current } => {
                format!("Gather {} resources ({}/{})", count, current, count)
            }
            Self::CompleteTrades { count, current } => {
                format!("Complete {} trades ({}/{})", count, current, count)
            }
            Self::ParryAttacks { count, current } => {
                format!("Parry {} attacks ({}/{})", count, current, count)
            }
            Self::UseAbilities { count, current } => {
                format!("Use {} abilities ({}/{})", count, current, count)
            }
            Self::EarnMasteryXp { amount, current } => {
                format!("Earn {} mastery XP ({}/{})", amount, current, amount)
            }
            Self::DiscoverRooms { count, current } => {
                format!("Discover {} rooms ({}/{})", count, current, count)
            }
            Self::DefeatElite { current } => format!("Defeat an elite monster ({}/1)", current),
        }
    }
}

/// A daily/weekly quest instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurringQuest {
    pub id: String,
    pub name: String,
    pub reset_type: QuestResetType,
    pub objective: DailyObjective,
    pub shard_reward: u64,
    pub mastery_xp_reward: u64,
    pub season_xp_reward: u64,
    pub completed: bool,
    pub claimed: bool,
}

impl RecurringQuest {
    pub fn check_complete(&mut self) {
        if self.objective.is_complete() && !self.completed {
            self.completed = true;
        }
    }

    pub fn claim_reward(&mut self) -> bool {
        if self.completed && !self.claimed {
            self.claimed = true;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        self.completed = false;
        self.claimed = false;
        // Reset objective progress
        match &mut self.objective {
            DailyObjective::KillMonsters { current, .. } => *current = 0,
            DailyObjective::ClearFloors { current, .. } => *current = 0,
            DailyObjective::CraftItems { current, .. } => *current = 0,
            DailyObjective::GatherResources { current, .. } => *current = 0,
            DailyObjective::CompleteTrades { current, .. } => *current = 0,
            DailyObjective::ParryAttacks { current, .. } => *current = 0,
            DailyObjective::UseAbilities { current, .. } => *current = 0,
            DailyObjective::EarnMasteryXp { current, .. } => *current = 0,
            DailyObjective::DiscoverRooms { current, .. } => *current = 0,
            DailyObjective::DefeatElite { current } => *current = 0,
        }
    }
}

/// Generate daily quests for a given day seed
pub fn generate_daily_quests(day_seed: u64) -> Vec<RecurringQuest> {
    // 3 daily quests, deterministic from day seed
    let mut quests = Vec::new();

    // Quest 1: Combat (always)
    let kill_count = 10 + ((day_seed % 20) as u32);
    quests.push(RecurringQuest {
        id: format!("daily_combat_{}", day_seed),
        name: "Tower Patrol".into(),
        reset_type: QuestResetType::Daily,
        objective: DailyObjective::KillMonsters {
            count: kill_count,
            current: 0,
        },
        shard_reward: 50,
        mastery_xp_reward: 10,
        season_xp_reward: 100,
        completed: false,
        claimed: false,
    });

    // Quest 2: Rotates based on seed
    let quest_type = day_seed % 5;
    let q2 = match quest_type {
        0 => RecurringQuest {
            id: format!("daily_craft_{}", day_seed),
            name: "Workshop Duty".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::CraftItems {
                count: 3,
                current: 0,
            },
            shard_reward: 40,
            mastery_xp_reward: 15,
            season_xp_reward: 80,
            completed: false,
            claimed: false,
        },
        1 => RecurringQuest {
            id: format!("daily_explore_{}", day_seed),
            name: "Cartographer's Task".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::DiscoverRooms {
                count: 5,
                current: 0,
            },
            shard_reward: 45,
            mastery_xp_reward: 12,
            season_xp_reward: 90,
            completed: false,
            claimed: false,
        },
        2 => RecurringQuest {
            id: format!("daily_parry_{}", day_seed),
            name: "Defensive Training".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::ParryAttacks {
                count: 10,
                current: 0,
            },
            shard_reward: 55,
            mastery_xp_reward: 20,
            season_xp_reward: 110,
            completed: false,
            claimed: false,
        },
        3 => RecurringQuest {
            id: format!("daily_gather_{}", day_seed),
            name: "Resource Run".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::GatherResources {
                count: 8,
                current: 0,
            },
            shard_reward: 35,
            mastery_xp_reward: 10,
            season_xp_reward: 70,
            completed: false,
            claimed: false,
        },
        _ => RecurringQuest {
            id: format!("daily_elite_{}", day_seed),
            name: "Elite Hunt".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::DefeatElite { current: 0 },
            shard_reward: 75,
            mastery_xp_reward: 25,
            season_xp_reward: 150,
            completed: false,
            claimed: false,
        },
    };
    quests.push(q2);

    // Quest 3: Floor progress
    quests.push(RecurringQuest {
        id: format!("daily_floor_{}", day_seed),
        name: "Tower Ascent".into(),
        reset_type: QuestResetType::Daily,
        objective: DailyObjective::ClearFloors {
            count: 2,
            current: 0,
        },
        shard_reward: 60,
        mastery_xp_reward: 10,
        season_xp_reward: 120,
        completed: false,
        claimed: false,
    });

    quests
}

/// Generate weekly quests
pub fn generate_weekly_quests(week_seed: u64) -> Vec<RecurringQuest> {
    vec![
        RecurringQuest {
            id: format!("weekly_slayer_{}", week_seed),
            name: "Tower Slayer".into(),
            reset_type: QuestResetType::Weekly,
            objective: DailyObjective::KillMonsters {
                count: 100,
                current: 0,
            },
            shard_reward: 300,
            mastery_xp_reward: 50,
            season_xp_reward: 500,
            completed: false,
            claimed: false,
        },
        RecurringQuest {
            id: format!("weekly_climber_{}", week_seed),
            name: "Weekly Climber".into(),
            reset_type: QuestResetType::Weekly,
            objective: DailyObjective::ClearFloors {
                count: 10,
                current: 0,
            },
            shard_reward: 400,
            mastery_xp_reward: 60,
            season_xp_reward: 600,
            completed: false,
            claimed: false,
        },
        RecurringQuest {
            id: format!("weekly_mastery_{}", week_seed),
            name: "Path of Mastery".into(),
            reset_type: QuestResetType::Weekly,
            objective: DailyObjective::EarnMasteryXp {
                amount: 500,
                current: 0,
            },
            shard_reward: 250,
            mastery_xp_reward: 0,
            season_xp_reward: 400,
            completed: false,
            claimed: false,
        },
    ]
}

// =====================
// Season Pass
// =====================

/// Season pass reward tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonReward {
    pub level: u32,
    pub name: String,
    pub reward_type: SeasonRewardType,
    pub premium_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeasonRewardType {
    Shards(u64),
    Cosmetic(String),        // transmog appearance
    Title(String),           // player title
    EquipmentEffect(String), // unique effect
    EmoteUnlock(String),
    ProfileBorder(String),
    MasteryBonus(u64), // bonus mastery XP
}

/// Player's season pass progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonPass {
    pub season_id: u32,
    pub season_name: String,
    pub xp: u64,
    pub level: u32,
    pub max_level: u32,
    pub is_premium: bool,
    pub claimed_rewards: Vec<u32>, // claimed level rewards
    pub xp_per_level: u64,
}

impl SeasonPass {
    pub fn new(season_id: u32, name: String) -> Self {
        Self {
            season_id,
            season_name: name,
            xp: 0,
            level: 0,
            max_level: 50,
            is_premium: false,
            claimed_rewards: Vec::new(),
            xp_per_level: 1000,
        }
    }

    /// Add XP and check for level ups. Returns number of levels gained.
    pub fn add_xp(&mut self, amount: u64) -> u32 {
        self.xp += amount;
        let new_level = ((self.xp / self.xp_per_level) as u32).min(self.max_level);
        let gained = new_level.saturating_sub(self.level);
        self.level = new_level;
        gained
    }

    /// Progress within current level (0.0 - 1.0)
    pub fn level_progress(&self) -> f32 {
        if self.level >= self.max_level {
            return 1.0;
        }
        let xp_in_level = self.xp % self.xp_per_level;
        xp_in_level as f32 / self.xp_per_level as f32
    }

    /// Check if a reward can be claimed
    pub fn can_claim(&self, reward: &SeasonReward) -> bool {
        if reward.level > self.level {
            return false;
        }
        if reward.premium_only && !self.is_premium {
            return false;
        }
        !self.claimed_rewards.contains(&reward.level)
    }

    pub fn claim(&mut self, level: u32) -> bool {
        if level > self.level {
            return false;
        }
        if self.claimed_rewards.contains(&level) {
            return false;
        }
        self.claimed_rewards.push(level);
        true
    }

    pub fn upgrade_to_premium(&mut self) {
        self.is_premium = true;
    }
}

/// Generate reward track for a season
pub fn generate_season_rewards(season_id: u32) -> Vec<SeasonReward> {
    let mut rewards = Vec::new();

    for level in 1..=50 {
        // Free reward every level
        let free = match level % 10 {
            0 => SeasonReward {
                level,
                name: format!("Season {} Title", season_id),
                reward_type: SeasonRewardType::Title(format!("Tower Veteran S{}", season_id)),
                premium_only: false,
            },
            5 => SeasonReward {
                level,
                name: "Mastery Boost".into(),
                reward_type: SeasonRewardType::MasteryBonus(100),
                premium_only: false,
            },
            _ => SeasonReward {
                level,
                name: "Tower Shards".into(),
                reward_type: SeasonRewardType::Shards(50 + level as u64 * 10),
                premium_only: false,
            },
        };
        rewards.push(free);

        // Premium reward every 5 levels
        if level % 5 == 0 {
            let premium = match (level / 5) % 4 {
                0 => SeasonReward {
                    level,
                    name: "Seasonal Cosmetic".into(),
                    reward_type: SeasonRewardType::Cosmetic(format!(
                        "s{}_outfit_{}",
                        season_id, level
                    )),
                    premium_only: true,
                },
                1 => SeasonReward {
                    level,
                    name: "Seasonal Emote".into(),
                    reward_type: SeasonRewardType::EmoteUnlock(format!(
                        "s{}_emote_{}",
                        season_id, level
                    )),
                    premium_only: true,
                },
                2 => SeasonReward {
                    level,
                    name: "Profile Border".into(),
                    reward_type: SeasonRewardType::ProfileBorder(format!(
                        "s{}_border_{}",
                        season_id, level
                    )),
                    premium_only: true,
                },
                _ => SeasonReward {
                    level,
                    name: "Seasonal Effect".into(),
                    reward_type: SeasonRewardType::EquipmentEffect(format!(
                        "s{}_effect_{}",
                        season_id, level
                    )),
                    premium_only: true,
                },
            };
            rewards.push(premium);
        }
    }

    rewards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_quests_generated() {
        let quests = generate_daily_quests(12345);
        assert_eq!(quests.len(), 3);
        assert!(quests.iter().all(|q| q.reset_type == QuestResetType::Daily));
        assert!(quests.iter().all(|q| !q.completed));
    }

    #[test]
    fn test_daily_quests_deterministic() {
        let a = generate_daily_quests(999);
        let b = generate_daily_quests(999);
        assert_eq!(a[0].id, b[0].id);
        assert_eq!(a[1].id, b[1].id);
    }

    #[test]
    fn test_daily_quests_vary_by_seed() {
        let a = generate_daily_quests(1);
        let b = generate_daily_quests(2);
        // At least the IDs differ
        assert_ne!(a[0].id, b[0].id);
    }

    #[test]
    fn test_weekly_quests() {
        let quests = generate_weekly_quests(1);
        assert_eq!(quests.len(), 3);
        assert!(quests
            .iter()
            .all(|q| q.reset_type == QuestResetType::Weekly));
        assert!(quests[0].shard_reward >= 200); // weekly rewards are bigger
    }

    #[test]
    fn test_objective_completion() {
        let mut obj = DailyObjective::KillMonsters {
            count: 5,
            current: 3,
        };
        assert!(!obj.is_complete());
        assert!((obj.progress_percent() - 0.6).abs() < 0.01);

        if let DailyObjective::KillMonsters { current, .. } = &mut obj {
            *current = 5;
        }
        assert!(obj.is_complete());
    }

    #[test]
    fn test_quest_claim_flow() {
        let mut quest = RecurringQuest {
            id: "test".into(),
            name: "Test".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::KillMonsters {
                count: 1,
                current: 1,
            },
            shard_reward: 50,
            mastery_xp_reward: 10,
            season_xp_reward: 100,
            completed: false,
            claimed: false,
        };

        assert!(!quest.claim_reward()); // not marked complete yet
        quest.check_complete();
        assert!(quest.completed);
        assert!(quest.claim_reward());
        assert!(!quest.claim_reward()); // can't double claim
    }

    #[test]
    fn test_quest_reset() {
        let mut quest = RecurringQuest {
            id: "test".into(),
            name: "Test".into(),
            reset_type: QuestResetType::Daily,
            objective: DailyObjective::KillMonsters {
                count: 5,
                current: 5,
            },
            shard_reward: 50,
            mastery_xp_reward: 10,
            season_xp_reward: 100,
            completed: true,
            claimed: true,
        };

        quest.reset();
        assert!(!quest.completed);
        assert!(!quest.claimed);
        assert!(!quest.objective.is_complete());
    }

    #[test]
    fn test_season_pass_xp() {
        let mut pass = SeasonPass::new(1, "Tower Awakening".into());
        assert_eq!(pass.level, 0);

        let gained = pass.add_xp(2500);
        assert_eq!(pass.level, 2);
        assert_eq!(gained, 2);
    }

    #[test]
    fn test_season_pass_max_level() {
        let mut pass = SeasonPass::new(1, "Test".into());
        pass.add_xp(999999);
        assert_eq!(pass.level, 50); // capped
    }

    #[test]
    fn test_season_pass_claim() {
        let mut pass = SeasonPass::new(1, "Test".into());
        pass.add_xp(5000); // level 5

        let reward = SeasonReward {
            level: 3,
            name: "Test".into(),
            reward_type: SeasonRewardType::Shards(100),
            premium_only: false,
        };

        assert!(pass.can_claim(&reward));
        assert!(pass.claim(3));
        assert!(!pass.claim(3)); // already claimed
    }

    #[test]
    fn test_season_pass_premium_gate() {
        let mut pass = SeasonPass::new(1, "Test".into());
        pass.add_xp(10000); // level 10

        let premium_reward = SeasonReward {
            level: 5,
            name: "Premium".into(),
            reward_type: SeasonRewardType::Cosmetic("outfit".into()),
            premium_only: true,
        };

        assert!(!pass.can_claim(&premium_reward)); // not premium
        pass.upgrade_to_premium();
        assert!(pass.can_claim(&premium_reward));
    }

    #[test]
    fn test_season_rewards_generated() {
        let rewards = generate_season_rewards(1);
        assert!(rewards.len() >= 50); // at least 50 free + premium rewards
                                      // Level 50 should have a title
        assert!(rewards.iter().any(|r| r.level == 50));
    }

    #[test]
    fn test_level_progress() {
        let mut pass = SeasonPass::new(1, "Test".into());
        pass.add_xp(500); // halfway through level 0→1
        assert!((pass.level_progress() - 0.5).abs() < 0.01);
    }
}
