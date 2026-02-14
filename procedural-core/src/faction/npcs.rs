//! NPC dialog and quest system.
//!
//! NPCs are faction-aligned and offer quests based on semantic tags,
//! player reputation, and current tower state.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::Faction;
use crate::semantic::SemanticTags;

/// NPC entity marker
#[derive(Component, Debug)]
pub struct Npc {
    pub name: String,
    pub faction: Faction,
    pub dialog_state: DialogState,
    pub semantic_tags: SemanticTags,
}

/// Current state of NPC conversation
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DialogState {
    #[default]
    Idle,
    Greeting,
    QuestOffer {
        quest_id: u32,
    },
    Trading,
    Farewell,
}

/// Dialog entry — a node in the conversation tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogNode {
    pub id: u32,
    pub speaker: String,
    pub text: String,
    pub choices: Vec<DialogChoice>,
    pub requirements: Vec<DialogRequirement>,
}

/// Player choice in dialog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogChoice {
    pub text: String,
    pub next_node: u32,
    pub effects: Vec<DialogEffect>,
}

/// Requirements to show a dialog option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogRequirement {
    MinReputation { faction: String, tier: String },
    HasItem { item_name: String },
    QuestComplete { quest_id: u32 },
    FloorReached { floor: u32 },
}

/// Effects triggered by dialog choices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DialogEffect {
    StartQuest { quest_id: u32 },
    CompleteQuest { quest_id: u32 },
    GiveShards { amount: u64 },
    ModifyReputation { faction: String, delta: f32 },
    GiveItem { item_name: String },
    OpenShop,
}

/// Quest definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub giver_faction: Faction,
    pub objectives: Vec<QuestObjective>,
    pub rewards: Vec<QuestReward>,
    pub required_floor: u32,
}

/// Quest objectives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestObjective {
    KillMonsters {
        tag: String,
        count: u32,
        current: u32,
    },
    ReachFloor {
        floor: u32,
    },
    CollectItems {
        item_name: String,
        count: u32,
        current: u32,
    },
    DefeatBoss {
        floor: u32,
    },
    DiscoverRoom {
        room_type: String,
    },
}

impl QuestObjective {
    pub fn is_complete(&self) -> bool {
        match self {
            Self::KillMonsters { count, current, .. } => current >= count,
            Self::ReachFloor { .. } => false, // checked externally
            Self::CollectItems { count, current, .. } => current >= count,
            Self::DefeatBoss { .. } => false,
            Self::DiscoverRoom { .. } => false,
        }
    }

    pub fn progress_text(&self) -> String {
        match self {
            Self::KillMonsters {
                tag,
                count,
                current,
            } => format!("Kill {} {} monsters ({}/{})", count, tag, current, count),
            Self::ReachFloor { floor } => format!("Reach floor {}", floor),
            Self::CollectItems {
                item_name,
                count,
                current,
            } => format!("Collect {} {} ({}/{})", count, item_name, current, count),
            Self::DefeatBoss { floor } => format!("Defeat the boss on floor {}", floor),
            Self::DiscoverRoom { room_type } => format!("Discover a {} room", room_type),
        }
    }
}

/// Quest rewards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestReward {
    Shards(u64),
    EchoFragments(u32),
    Reputation { faction: String, amount: f32 },
    Xp(u32),
}

/// Player's quest log
#[derive(Component, Debug, Default)]
pub struct QuestLog {
    pub active: Vec<Quest>,
    pub completed: Vec<u32>,
}

impl QuestLog {
    pub fn accept_quest(&mut self, quest: Quest) -> bool {
        if self.active.iter().any(|q| q.id == quest.id) {
            return false; // already active
        }
        if self.completed.contains(&quest.id) {
            return false; // already done
        }
        self.active.push(quest);
        true
    }

    pub fn complete_quest(&mut self, quest_id: u32) -> Option<Quest> {
        if let Some(pos) = self.active.iter().position(|q| q.id == quest_id) {
            let quest = self.active.remove(pos);
            self.completed.push(quest_id);
            Some(quest)
        } else {
            None
        }
    }

    pub fn is_quest_complete(&self, quest_id: u32) -> bool {
        self.completed.contains(&quest_id)
    }
}

/// Generate quests for a faction based on floor level
pub fn generate_faction_quests(faction: Faction, floor_level: u32) -> Vec<Quest> {
    let mut quests = Vec::new();

    match faction {
        Faction::AscendingOrder => {
            quests.push(Quest {
                id: 1000 + floor_level,
                name: format!("Clear Floor {}", floor_level),
                description: "Defeat all monsters on this floor to prove your worth.".into(),
                giver_faction: Faction::AscendingOrder,
                objectives: vec![QuestObjective::KillMonsters {
                    tag: "any".into(),
                    count: 5 + floor_level / 10,
                    current: 0,
                }],
                rewards: vec![
                    QuestReward::Shards(50 * floor_level as u64),
                    QuestReward::Reputation {
                        faction: "ascending_order".into(),
                        amount: 10.0,
                    },
                    QuestReward::Xp(100 * floor_level),
                ],
                required_floor: floor_level,
            });
        }
        Faction::DeepDwellers => {
            quests.push(Quest {
                id: 2000 + floor_level,
                name: format!("Deep Harvest (Floor {})", floor_level),
                description: "Collect materials from the deep floors.".into(),
                giver_faction: Faction::DeepDwellers,
                objectives: vec![QuestObjective::CollectItems {
                    item_name: "Essence".into(),
                    count: 3 + floor_level / 20,
                    current: 0,
                }],
                rewards: vec![
                    QuestReward::Shards(30 * floor_level as u64),
                    QuestReward::Reputation {
                        faction: "deep_dwellers".into(),
                        amount: 15.0,
                    },
                ],
                required_floor: floor_level.saturating_sub(5),
            });
        }
        Faction::EchoKeepers => {
            quests.push(Quest {
                id: 3000 + floor_level,
                name: "Corruption Study".into(),
                description: "Defeat corrupted monsters and study their echoes.".into(),
                giver_faction: Faction::EchoKeepers,
                objectives: vec![QuestObjective::KillMonsters {
                    tag: "corruption".into(),
                    count: 3,
                    current: 0,
                }],
                rewards: vec![
                    QuestReward::EchoFragments(2),
                    QuestReward::Reputation {
                        faction: "echo_keepers".into(),
                        amount: 20.0,
                    },
                    QuestReward::Xp(150 * floor_level),
                ],
                required_floor: floor_level,
            });
        }
        Faction::FreeClimbers => {
            quests.push(Quest {
                id: 4000 + floor_level,
                name: format!("Reach Floor {}", floor_level + 10),
                description: "Push higher into the tower.".into(),
                giver_faction: Faction::FreeClimbers,
                objectives: vec![QuestObjective::ReachFloor {
                    floor: floor_level + 10,
                }],
                rewards: vec![
                    QuestReward::Shards(100 * floor_level as u64),
                    QuestReward::Reputation {
                        faction: "free_climbers".into(),
                        amount: 25.0,
                    },
                ],
                required_floor: floor_level,
            });
        }
    }

    quests
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_log_basic() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: 1,
            name: "Test Quest".into(),
            description: "Test".into(),
            giver_faction: Faction::AscendingOrder,
            objectives: vec![],
            rewards: vec![QuestReward::Shards(100)],
            required_floor: 1,
        };

        assert!(log.accept_quest(quest.clone()));
        assert!(!log.accept_quest(quest.clone()), "Can't accept twice");
        assert_eq!(log.active.len(), 1);
    }

    #[test]
    fn test_quest_completion() {
        let mut log = QuestLog::default();
        let quest = Quest {
            id: 42,
            name: "Complete Me".into(),
            description: "Test".into(),
            giver_faction: Faction::FreeClimbers,
            objectives: vec![],
            rewards: vec![],
            required_floor: 1,
        };

        log.accept_quest(quest);
        let completed = log.complete_quest(42);
        assert!(completed.is_some());
        assert!(log.is_quest_complete(42));
        assert!(log.active.is_empty());
    }

    #[test]
    fn test_objective_progress() {
        let obj = QuestObjective::KillMonsters {
            tag: "fire".into(),
            count: 5,
            current: 3,
        };
        assert!(!obj.is_complete());
        assert!(obj.progress_text().contains("3/5"));

        let obj_done = QuestObjective::KillMonsters {
            tag: "fire".into(),
            count: 5,
            current: 5,
        };
        assert!(obj_done.is_complete());
    }

    #[test]
    fn test_generate_quests() {
        let quests = generate_faction_quests(Faction::AscendingOrder, 10);
        assert!(!quests.is_empty());
        assert_eq!(quests[0].giver_faction, Faction::AscendingOrder);
    }

    #[test]
    fn test_all_factions_have_quests() {
        for faction in [
            Faction::AscendingOrder,
            Faction::DeepDwellers,
            Faction::EchoKeepers,
            Faction::FreeClimbers,
        ] {
            let quests = generate_faction_quests(faction, 5);
            assert!(!quests.is_empty(), "{:?} should generate quests", faction);
        }
    }
}
