//! Game Flow State Machine (ARCH-004)
//!
//! Manages the top-level game states using Bevy's States system:
//! Loading → MainMenu → CharacterSelect → InGame → Paused → Death → FloorTransition
//!
//! Each state has OnEnter/OnExit systems for proper initialization/cleanup.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct GameFlowPlugin;

impl Plugin for GameFlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<InGameSubState>()
            .add_systems(OnEnter(GameState::Loading), on_enter_loading)
            .add_systems(OnEnter(GameState::MainMenu), on_enter_main_menu)
            .add_systems(OnEnter(GameState::InGame), on_enter_in_game)
            .add_systems(OnExit(GameState::InGame), on_exit_in_game)
            .add_systems(OnEnter(GameState::Paused), on_enter_paused)
            .add_systems(OnExit(GameState::Paused), on_exit_paused)
            .add_systems(OnEnter(GameState::Death), on_enter_death)
            .add_event::<GameFlowEvent>();
    }
}

/// Top-level game states
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum GameState {
    /// Initial asset loading, DLL loading, connection setup
    #[default]
    Loading,
    /// Main menu: play, settings, quit
    MainMenu,
    /// Character creation / selection screen
    CharacterSelect,
    /// Actively playing — sub-states control specifics
    InGame,
    /// Game paused (single-player only)
    Paused,
    /// Player died — echo determination, stats display
    Death,
    /// Transitioning between floors (fade out → generate → fade in)
    FloorTransition,
}

/// Sub-states while InGame
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[source(GameState = GameState::InGame)]
pub enum InGameSubState {
    /// Normal gameplay — exploration, combat, interactions
    #[default]
    Exploring,
    /// In combat encounter (affects UI, controls, camera)
    Combat,
    /// Talking to NPC (dialog widget active)
    Dialog,
    /// Crafting at a station
    Crafting,
    /// Trading with another player
    Trading,
    /// Viewing inventory/equipment
    Inventory,
    /// Viewing skill tree / mastery
    SkillTree,
}

/// Events that trigger state transitions
#[derive(Event, Debug, Clone)]
pub enum GameFlowEvent {
    /// Assets finished loading
    LoadingComplete,
    /// Player pressed "Play" from main menu
    StartGame,
    /// Character created/selected, entering tower
    EnterTower { floor_id: u32 },
    /// Player pressed pause
    Pause,
    /// Player resumed from pause
    Resume,
    /// Player died
    PlayerDied { echo_type: String },
    /// Player chose to respawn
    Respawn { floor_id: u32 },
    /// Floor clear — transition to next
    FloorCleared { next_floor_id: u32 },
    /// Floor transition animation complete
    TransitionComplete,
    /// Return to main menu
    ReturnToMenu,
    /// Open a sub-state screen
    OpenSubScreen { sub_state: InGameSubState },
    /// Close sub-state, return to exploring
    CloseSubScreen,
}

/// Resource tracking current floor during gameplay
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct CurrentFloorInfo {
    pub floor_id: u32,
    pub time_entered: f64,
    pub monsters_slain: u32,
    pub damage_dealt: f64,
    pub damage_taken: f64,
    pub items_collected: u32,
}

impl CurrentFloorInfo {
    pub fn new(floor_id: u32) -> Self {
        Self {
            floor_id,
            time_entered: 0.0,
            monsters_slain: 0,
            damage_dealt: 0.0,
            damage_taken: 0.0,
            items_collected: 0,
        }
    }
}

/// Resource for death screen data
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct DeathInfo {
    pub echo_type: String,
    pub floor_reached: u32,
    pub monsters_slain: u32,
    pub time_survived_secs: f64,
    pub damage_dealt: f64,
}

// State transition systems

fn on_enter_loading(mut _commands: Commands) {
    info!("GameFlow: Entering Loading state");
    // In production: start loading assets, DLL, connect to server
}

fn on_enter_main_menu(mut _commands: Commands) {
    info!("GameFlow: Entering MainMenu state");
    // In production: show main menu widget
}

fn on_enter_in_game(mut commands: Commands) {
    info!("GameFlow: Entering InGame state");
    commands.insert_resource(CurrentFloorInfo::new(1));
}

fn on_exit_in_game(mut commands: Commands) {
    info!("GameFlow: Exiting InGame state");
    commands.remove_resource::<CurrentFloorInfo>();
}

fn on_enter_paused(mut _commands: Commands) {
    info!("GameFlow: Game Paused");
    // In production: show pause menu, freeze game time
}

fn on_exit_paused(mut _commands: Commands) {
    info!("GameFlow: Game Resumed");
    // In production: hide pause menu, resume game time
}

fn on_enter_death(mut _commands: Commands) {
    info!("GameFlow: Player Died");
    // In production: show death screen widget
}

/// Serializable game flow state for FFI (used by UE5 to query current state)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameFlowSnapshot {
    pub state: String,
    pub sub_state: Option<String>,
    pub floor_info: Option<CurrentFloorInfo>,
}

/// Get a snapshot of current game flow state
pub fn create_snapshot(state: GameState, sub_state: Option<InGameSubState>) -> GameFlowSnapshot {
    GameFlowSnapshot {
        state: format!("{:?}", state),
        sub_state: sub_state.map(|s| format!("{:?}", s)),
        floor_info: None,
    }
}

/// Get all valid game states as strings (for UI)
pub fn all_game_states() -> Vec<String> {
    vec![
        "Loading".to_string(),
        "MainMenu".to_string(),
        "CharacterSelect".to_string(),
        "InGame".to_string(),
        "Paused".to_string(),
        "Death".to_string(),
        "FloorTransition".to_string(),
    ]
}

/// Get all valid in-game sub-states as strings (for UI)
pub fn all_sub_states() -> Vec<String> {
    vec![
        "Exploring".to_string(),
        "Combat".to_string(),
        "Dialog".to_string(),
        "Crafting".to_string(),
        "Trading".to_string(),
        "Inventory".to_string(),
        "SkillTree".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_game_state() {
        let state = GameState::default();
        assert_eq!(state, GameState::Loading);
    }

    #[test]
    fn test_default_sub_state() {
        let sub = InGameSubState::default();
        assert_eq!(sub, InGameSubState::Exploring);
    }

    #[test]
    fn test_current_floor_info() {
        let info = CurrentFloorInfo::new(42);
        assert_eq!(info.floor_id, 42);
        assert_eq!(info.monsters_slain, 0);
        assert_eq!(info.damage_dealt, 0.0);
    }

    #[test]
    fn test_death_info() {
        let info = DeathInfo {
            echo_type: "Aggressive".to_string(),
            floor_reached: 15,
            monsters_slain: 42,
            time_survived_secs: 300.0,
            damage_dealt: 12500.0,
        };
        assert_eq!(info.floor_reached, 15);
        assert_eq!(info.echo_type, "Aggressive");
    }

    #[test]
    fn test_snapshot_creation() {
        let snap = create_snapshot(GameState::InGame, Some(InGameSubState::Combat));
        assert_eq!(snap.state, "InGame");
        assert_eq!(snap.sub_state, Some("Combat".to_string()));
    }

    #[test]
    fn test_snapshot_no_substate() {
        let snap = create_snapshot(GameState::MainMenu, None);
        assert_eq!(snap.state, "MainMenu");
        assert!(snap.sub_state.is_none());
    }

    #[test]
    fn test_all_game_states() {
        let states = all_game_states();
        assert_eq!(states.len(), 7);
        assert!(states.contains(&"Loading".to_string()));
        assert!(states.contains(&"InGame".to_string()));
        assert!(states.contains(&"Death".to_string()));
    }

    #[test]
    fn test_all_sub_states() {
        let states = all_sub_states();
        assert_eq!(states.len(), 7);
        assert!(states.contains(&"Exploring".to_string()));
        assert!(states.contains(&"Combat".to_string()));
        assert!(states.contains(&"SkillTree".to_string()));
    }

    #[test]
    fn test_game_state_serialization() {
        let state = GameState::InGame;
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: GameState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_sub_state_serialization() {
        let sub = InGameSubState::Combat;
        let json = serde_json::to_string(&sub).unwrap();
        let deserialized: InGameSubState = serde_json::from_str(&json).unwrap();
        assert_eq!(sub, deserialized);
    }

    #[test]
    fn test_death_info_serialization() {
        let info = DeathInfo {
            echo_type: "Lingering".to_string(),
            floor_reached: 10,
            monsters_slain: 25,
            time_survived_secs: 180.0,
            damage_dealt: 5000.0,
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: DeathInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.floor_reached, 10);
        assert_eq!(deserialized.echo_type, "Lingering");
    }

    #[test]
    fn test_game_flow_event_variants() {
        // Ensure all event variants can be created
        let events = vec![
            GameFlowEvent::LoadingComplete,
            GameFlowEvent::StartGame,
            GameFlowEvent::EnterTower { floor_id: 1 },
            GameFlowEvent::Pause,
            GameFlowEvent::Resume,
            GameFlowEvent::PlayerDied {
                echo_type: "Helpful".into(),
            },
            GameFlowEvent::Respawn { floor_id: 1 },
            GameFlowEvent::FloorCleared { next_floor_id: 2 },
            GameFlowEvent::TransitionComplete,
            GameFlowEvent::ReturnToMenu,
            GameFlowEvent::OpenSubScreen {
                sub_state: InGameSubState::Inventory,
            },
            GameFlowEvent::CloseSubScreen,
        ];
        assert_eq!(events.len(), 12);
    }

    #[test]
    fn test_snapshot_roundtrip() {
        let snap = create_snapshot(GameState::Death, None);
        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: GameFlowSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.state, "Death");
        assert!(deserialized.sub_state.is_none());
        assert!(deserialized.floor_info.is_none());
    }
}
