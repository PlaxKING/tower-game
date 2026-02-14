//! Tutorial & Onboarding System
//!
//! From ddopensource.txt Category 11:
//! Tutorial system, guides, hints, practice mode, mentor system.
//! New player guidance, progression tips, FAQ.
//!
//! Tutorial is context-sensitive — triggers when player encounters new mechanics.
//! Practice mode allows training without consequences.
//! Hint system shows tips based on player behavior analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tutorial step category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TutorialCategory {
    BasicControls,
    Combat,
    Mastery,
    Crafting,
    Social,
    Tower,
    Equipment,
    Specialization,
    Trading,
    Navigation,
}

impl TutorialCategory {
    pub fn display_name(&self) -> &str {
        match self {
            Self::BasicControls => "Basic Controls",
            Self::Combat => "Combat",
            Self::Mastery => "Mastery System",
            Self::Crafting => "Crafting",
            Self::Social => "Social",
            Self::Tower => "Tower Mechanics",
            Self::Equipment => "Equipment",
            Self::Specialization => "Specialization",
            Self::Trading => "Trading",
            Self::Navigation => "Navigation",
        }
    }
}

/// A single tutorial step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TutorialStep {
    pub id: String,
    pub category: TutorialCategory,
    pub title: String,
    pub description: String,
    /// Trigger condition — when this step activates
    pub trigger: TutorialTrigger,
    /// Whether it requires player action to dismiss
    pub requires_action: bool,
    /// Action description if requires_action
    pub action_hint: String,
    /// Steps that must be completed before this one
    pub prerequisites: Vec<String>,
    /// Position hint for UI placement
    pub ui_anchor: UIAnchor,
    /// Priority (higher = shown first when multiple queue)
    pub priority: u32,
}

/// When a tutorial step triggers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TutorialTrigger {
    /// On first game start
    FirstLogin,
    /// When entering a specific floor number
    FloorReached(u32),
    /// When picking up first item of a type
    FirstItemPickup(String),
    /// When health drops below threshold
    HealthBelow(f32),
    /// When first entering combat
    FirstCombat,
    /// When first crafting attempt
    FirstCraft,
    /// When mastery tier reached
    MasteryTierReached(String),
    /// When joining a guild
    GuildJoined,
    /// When receiving a trade request
    TradeStarted,
    /// Manual trigger from NPC dialog
    NPCDialog(String),
    /// When player has been idle for N seconds
    IdleTimeout(f32),
    /// When dying for the first time
    FirstDeath,
    /// When specialization becomes available
    SpecializationAvailable,
    /// When finding equipment with sockets
    SocketedItemFound,
}

/// Where to anchor the tutorial UI element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UIAnchor {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    AboveHotbar,
    NearMinimap,
    NearHealthBar,
}

/// Context-sensitive hint (less intrusive than tutorial)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameHint {
    pub id: String,
    pub category: TutorialCategory,
    pub text: String,
    /// Show after N occurrences of the trigger event
    pub show_after_count: u32,
    /// Max times to show this hint
    pub max_shows: u32,
    pub trigger: HintTrigger,
}

/// Trigger for showing a hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HintTrigger {
    /// Player failed to parry N times
    FailedParry(u32),
    /// Player died N times on same floor
    RepeatedDeath(u32),
    /// Player has unspent mastery points
    UnspentMasteryPoints,
    /// Player has full inventory
    InventoryFull,
    /// Player hasn't crafted in a while
    NoCraftingRecently,
    /// Player has unclaimed season rewards
    UnclaimedRewards,
    /// Player has available specialization
    SpecAvailable,
    /// Low on resources during combat
    LowResources(String),
    /// Has empty equipment sockets
    EmptySockets,
}

/// Player's tutorial progress
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TutorialProgress {
    /// Completed tutorial step IDs
    pub completed: Vec<String>,
    /// Dismissed hint IDs and show counts
    pub hint_counts: HashMap<String, u32>,
    /// Whether practice mode is active
    pub practice_mode: bool,
    /// Whether to show hints at all
    pub hints_enabled: bool,
    /// Queue of pending tutorial steps
    pub pending_queue: Vec<String>,
}

impl TutorialProgress {
    pub fn new() -> Self {
        Self {
            completed: Vec::new(),
            hint_counts: HashMap::new(),
            practice_mode: false,
            hints_enabled: true,
            pending_queue: Vec::new(),
        }
    }

    /// Complete a tutorial step
    pub fn complete(&mut self, step_id: &str) {
        if !self.completed.contains(&step_id.to_string()) {
            self.completed.push(step_id.to_string());
        }
        self.pending_queue.retain(|id| id != step_id);
    }

    /// Check if step was completed
    pub fn is_completed(&self, step_id: &str) -> bool {
        self.completed.iter().any(|s| s == step_id)
    }

    /// Check if step prerequisites are met
    pub fn prerequisites_met(&self, step: &TutorialStep) -> bool {
        step.prerequisites
            .iter()
            .all(|prereq| self.is_completed(prereq))
    }

    /// Try to trigger a tutorial step
    pub fn try_trigger(&mut self, step: &TutorialStep) -> bool {
        if self.is_completed(&step.id) {
            return false;
        }
        if !self.prerequisites_met(step) {
            return false;
        }
        if !self.pending_queue.contains(&step.id) {
            self.pending_queue.push(step.id.clone());
        }
        true
    }

    /// Track hint show count and determine if should show
    pub fn should_show_hint(&mut self, hint: &GameHint) -> bool {
        if !self.hints_enabled {
            return false;
        }
        let count = self.hint_counts.entry(hint.id.clone()).or_insert(0);
        if *count >= hint.max_shows {
            return false;
        }
        *count += 1;
        true
    }

    /// Toggle practice mode
    pub fn toggle_practice_mode(&mut self) -> bool {
        self.practice_mode = !self.practice_mode;
        self.practice_mode
    }

    /// Get next pending tutorial step
    pub fn next_pending(&self) -> Option<&str> {
        self.pending_queue.first().map(|s| s.as_str())
    }

    /// Total completion percentage
    pub fn completion_percent(&self, total_steps: usize) -> f32 {
        if total_steps == 0 {
            return 1.0;
        }
        self.completed.len() as f32 / total_steps as f32
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// All predefined tutorial steps
pub fn tutorial_steps() -> Vec<TutorialStep> {
    vec![
        TutorialStep {
            id: "tut_movement".into(),
            category: TutorialCategory::BasicControls,
            title: "Movement".into(),
            description: "Use WASD to move. Hold Shift to dodge. Press Space to jump.".into(),
            trigger: TutorialTrigger::FirstLogin,
            requires_action: true,
            action_hint: "Move around using WASD".into(),
            prerequisites: vec![],
            ui_anchor: UIAnchor::Center,
            priority: 100,
        },
        TutorialStep {
            id: "tut_camera".into(),
            category: TutorialCategory::BasicControls,
            title: "Camera Control".into(),
            description: "Move the mouse to look around. Scroll wheel to zoom.".into(),
            trigger: TutorialTrigger::FirstLogin,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_movement".into()],
            ui_anchor: UIAnchor::Center,
            priority: 99,
        },
        TutorialStep {
            id: "tut_combat_basic".into(),
            category: TutorialCategory::Combat,
            title: "Basic Combat".into(),
            description: "Click LMB to attack. Chain attacks for combos. Timing matters!".into(),
            trigger: TutorialTrigger::FirstCombat,
            requires_action: true,
            action_hint: "Defeat the training dummy".into(),
            prerequisites: vec!["tut_movement".into()],
            ui_anchor: UIAnchor::AboveHotbar,
            priority: 90,
        },
        TutorialStep {
            id: "tut_dodge".into(),
            category: TutorialCategory::Combat,
            title: "Dodging".into(),
            description: "Press Shift to dodge. You're invulnerable during the roll. Costs kinetic energy.".into(),
            trigger: TutorialTrigger::HealthBelow(0.8),
            requires_action: true,
            action_hint: "Successfully dodge an attack".into(),
            prerequisites: vec!["tut_combat_basic".into()],
            ui_anchor: UIAnchor::AboveHotbar,
            priority: 85,
        },
        TutorialStep {
            id: "tut_parry".into(),
            category: TutorialCategory::Combat,
            title: "Parrying".into(),
            description: "Press RMB just before an attack lands to parry. Perfect timing deals counter damage!".into(),
            trigger: TutorialTrigger::FloorReached(2),
            requires_action: true,
            action_hint: "Successfully parry an attack".into(),
            prerequisites: vec!["tut_dodge".into()],
            ui_anchor: UIAnchor::AboveHotbar,
            priority: 80,
        },
        TutorialStep {
            id: "tut_mastery".into(),
            category: TutorialCategory::Mastery,
            title: "Mastery System".into(),
            description: "You gain mastery XP by USING skills. Swing swords → Sword Mastery. Parry → Parry Mastery. Higher tiers unlock skill tree nodes.".into(),
            trigger: TutorialTrigger::MasteryTierReached("Apprentice".into()),
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_combat_basic".into()],
            ui_anchor: UIAnchor::Center,
            priority: 75,
        },
        TutorialStep {
            id: "tut_loot".into(),
            category: TutorialCategory::Equipment,
            title: "Loot & Equipment".into(),
            description: "Walk near items to pick them up. Open inventory (I) to equip gear. Equipment gives special effects, not just stats!".into(),
            trigger: TutorialTrigger::FirstItemPickup("any".into()),
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_combat_basic".into()],
            ui_anchor: UIAnchor::TopRight,
            priority: 70,
        },
        TutorialStep {
            id: "tut_crafting".into(),
            category: TutorialCategory::Crafting,
            title: "Crafting".into(),
            description: "Materials with matching semantic tags combine better. Higher crafting mastery = better results.".into(),
            trigger: TutorialTrigger::FirstCraft,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_loot".into()],
            ui_anchor: UIAnchor::Center,
            priority: 60,
        },
        TutorialStep {
            id: "tut_death".into(),
            category: TutorialCategory::Tower,
            title: "Death & Echoes".into(),
            description: "When you die, you leave an Echo that helps future players. Echoes retain traces of your actions.".into(),
            trigger: TutorialTrigger::FirstDeath,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec![],
            ui_anchor: UIAnchor::Center,
            priority: 95,
        },
        TutorialStep {
            id: "tut_specialization".into(),
            category: TutorialCategory::Specialization,
            title: "Specialization".into(),
            description: "At Expert tier, choose a specialization branch. This defines your playstyle and unlocks ultimate abilities!".into(),
            trigger: TutorialTrigger::SpecializationAvailable,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_mastery".into()],
            ui_anchor: UIAnchor::Center,
            priority: 65,
        },
        TutorialStep {
            id: "tut_sockets".into(),
            category: TutorialCategory::Equipment,
            title: "Gems & Runes".into(),
            description: "Equipment can have sockets. Insert gems for stat bonuses or runes for special effects. Match socket colors!".into(),
            trigger: TutorialTrigger::SocketedItemFound,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec!["tut_loot".into()],
            ui_anchor: UIAnchor::Center,
            priority: 55,
        },
        TutorialStep {
            id: "tut_trading".into(),
            category: TutorialCategory::Trading,
            title: "Player Trading".into(),
            description: "Trade items and shards with other players. Lock → Confirm → Execute. Both sides must confirm!".into(),
            trigger: TutorialTrigger::TradeStarted,
            requires_action: false,
            action_hint: "".into(),
            prerequisites: vec![],
            ui_anchor: UIAnchor::Center,
            priority: 50,
        },
    ]
}

/// Predefined game hints
pub fn game_hints() -> Vec<GameHint> {
    vec![
        GameHint {
            id: "hint_parry_timing".into(),
            category: TutorialCategory::Combat,
            text: "Try parrying just before the attack hits — timing is key!".into(),
            show_after_count: 0,
            max_shows: 3,
            trigger: HintTrigger::FailedParry(5),
        },
        GameHint {
            id: "hint_dodge_iframes".into(),
            category: TutorialCategory::Combat,
            text: "You're invulnerable at the start of your dodge. Use it to avoid big attacks."
                .into(),
            show_after_count: 0,
            max_shows: 2,
            trigger: HintTrigger::RepeatedDeath(2),
        },
        GameHint {
            id: "hint_mastery_points".into(),
            category: TutorialCategory::Mastery,
            text: "You have mastery nodes available to unlock! Check your skill tree.".into(),
            show_after_count: 0,
            max_shows: 5,
            trigger: HintTrigger::UnspentMasteryPoints,
        },
        GameHint {
            id: "hint_inventory_full".into(),
            category: TutorialCategory::Equipment,
            text: "Inventory full! Salvage or sell items to make room.".into(),
            show_after_count: 0,
            max_shows: 10,
            trigger: HintTrigger::InventoryFull,
        },
        GameHint {
            id: "hint_season_rewards".into(),
            category: TutorialCategory::Tower,
            text: "You have unclaimed season rewards! Open the Season Pass menu.".into(),
            show_after_count: 0,
            max_shows: 3,
            trigger: HintTrigger::UnclaimedRewards,
        },
        GameHint {
            id: "hint_empty_sockets".into(),
            category: TutorialCategory::Equipment,
            text: "Your equipment has empty sockets. Insert gems or runes for bonuses!".into(),
            show_after_count: 0,
            max_shows: 3,
            trigger: HintTrigger::EmptySockets,
        },
        GameHint {
            id: "hint_low_resources".into(),
            category: TutorialCategory::Combat,
            text: "Resources low! Wait for regen or find resource pickups.".into(),
            show_after_count: 0,
            max_shows: 5,
            trigger: HintTrigger::LowResources("kinetic".into()),
        },
    ]
}

/// Bevy plugin stub
pub struct TutorialPlugin;
impl bevy::prelude::Plugin for TutorialPlugin {
    fn build(&self, _app: &mut bevy::prelude::App) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tutorial_steps_exist() {
        let steps = tutorial_steps();
        assert!(steps.len() >= 12, "Should have at least 12 tutorial steps");
    }

    #[test]
    fn test_tutorial_progress_complete() {
        let mut progress = TutorialProgress::new();
        assert!(!progress.is_completed("tut_movement"));

        progress.complete("tut_movement");
        assert!(progress.is_completed("tut_movement"));
    }

    #[test]
    fn test_prerequisites_check() {
        let mut progress = TutorialProgress::new();
        let steps = tutorial_steps();

        let camera_step = steps.iter().find(|s| s.id == "tut_camera").unwrap();
        assert!(!progress.prerequisites_met(camera_step)); // needs tut_movement

        progress.complete("tut_movement");
        assert!(progress.prerequisites_met(camera_step));
    }

    #[test]
    fn test_trigger_queues_step() {
        let mut progress = TutorialProgress::new();
        let steps = tutorial_steps();

        let movement = steps.iter().find(|s| s.id == "tut_movement").unwrap();
        assert!(progress.try_trigger(movement));
        assert_eq!(progress.next_pending(), Some("tut_movement"));
    }

    #[test]
    fn test_completed_step_not_retriggered() {
        let mut progress = TutorialProgress::new();
        let steps = tutorial_steps();
        let movement = steps.iter().find(|s| s.id == "tut_movement").unwrap();

        progress.complete("tut_movement");
        assert!(!progress.try_trigger(movement));
    }

    #[test]
    fn test_hint_show_limit() {
        let mut progress = TutorialProgress::new();
        let hints = game_hints();
        let hint = &hints[0]; // max_shows: 3

        assert!(progress.should_show_hint(hint));
        assert!(progress.should_show_hint(hint));
        assert!(progress.should_show_hint(hint));
        assert!(!progress.should_show_hint(hint)); // exceeded max
    }

    #[test]
    fn test_hints_disabled() {
        let mut progress = TutorialProgress::new();
        progress.hints_enabled = false;
        let hints = game_hints();

        assert!(!progress.should_show_hint(&hints[0]));
    }

    #[test]
    fn test_practice_mode_toggle() {
        let mut progress = TutorialProgress::new();
        assert!(!progress.practice_mode);

        let result = progress.toggle_practice_mode();
        assert!(result);
        assert!(progress.practice_mode);

        let result2 = progress.toggle_practice_mode();
        assert!(!result2);
        assert!(!progress.practice_mode);
    }

    #[test]
    fn test_completion_percent() {
        let mut progress = TutorialProgress::new();
        assert_eq!(progress.completion_percent(10), 0.0);

        progress.complete("step1");
        progress.complete("step2");
        assert!((progress.completion_percent(10) - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_game_hints_exist() {
        let hints = game_hints();
        assert!(hints.len() >= 7);
    }

    #[test]
    fn test_tutorial_json() {
        let mut progress = TutorialProgress::new();
        progress.complete("tut_movement");

        let json = progress.to_json();
        assert!(!json.is_empty());
        assert!(json.contains("tut_movement"));
    }

    #[test]
    fn test_tutorial_categories() {
        let steps = tutorial_steps();
        let categories: Vec<TutorialCategory> = steps.iter().map(|s| s.category).collect();
        assert!(categories.contains(&TutorialCategory::BasicControls));
        assert!(categories.contains(&TutorialCategory::Combat));
        assert!(categories.contains(&TutorialCategory::Mastery));
    }
}
