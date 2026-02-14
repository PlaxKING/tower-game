//! Player character controller.
//!
//! Combines movement, combat, aerial, and resource systems into a single
//! controllable player entity.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::aerial::{DiveAttack, FlightState};
use crate::combat::{CombatResources, CombatState};
use crate::death::Mortal;
use crate::economy::Wallet;
use crate::faction::FactionStanding;
use crate::movement::{DashAbility, MovementInput, MovementState};
use crate::semantic::SemanticTags;

pub mod inventory;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerActionEvent>()
            .add_systems(Startup, spawn_player)
            .add_systems(
                Update,
                (
                    read_keyboard_input,
                    process_player_actions,
                    inventory::auto_pickup_loot,
                    update_player_level,
                )
                    .chain(),
            );
    }
}

/// Marker for the player entity
#[derive(Component, Debug)]
pub struct Player {
    pub name: String,
    pub level: u32,
    pub xp: u64,
    pub xp_to_next: u64,
    pub current_floor: u32,
    pub highest_floor: u32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            name: "Climber".into(),
            level: 1,
            xp: 0,
            xp_to_next: 100,
            current_floor: 1,
            highest_floor: 1,
        }
    }
}

/// Player actions (input abstraction layer)
#[derive(Event, Debug, Clone)]
pub enum PlayerActionEvent {
    Move(Vec2),
    Jump,
    Dash,
    Attack,
    Parry,
    Dodge,
    UseAbility(u32), // ability slot index
    Interact,
    ToggleFlight,
    DiveAttack,
}

/// Player ability slot
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct AbilitySlots {
    pub slots: Vec<Option<Ability>>,
    pub max_slots: usize,
}

impl Default for AbilitySlots {
    fn default() -> Self {
        Self {
            slots: vec![None; 4],
            max_slots: 4,
        }
    }
}

/// An equipped ability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ability {
    pub name: String,
    pub cooldown: f32,
    pub current_cooldown: f32,
    pub resource_cost: AbilityResourceCost,
    pub semantic_tags: Vec<(String, f32)>,
}

/// Which combat resource an ability consumes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityResourceCost {
    pub kinetic: f32,
    pub thermal: f32,
    pub semantic: f32,
    pub rage: f32,
}

impl Default for AbilityResourceCost {
    fn default() -> Self {
        Self {
            kinetic: 0.0,
            thermal: 0.0,
            semantic: 0.0,
            rage: 0.0,
        }
    }
}

/// XP required to reach next level (exponential curve)
fn xp_for_level(level: u32) -> u64 {
    (100.0 * (level as f64).powf(1.5)) as u64
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        // Core identity
        Player::default(),
        Transform::from_xyz(0.0, 1.0, 0.0),
        // Movement
        MovementState {
            grounded: true,
            ..default()
        },
        MovementInput::default(),
        DashAbility::default(),
        // Combat
        CombatState::default(),
        CombatResources::default(),
        // Aerial
        FlightState::default(),
        DiveAttack::default(),
        // Death
        Mortal {
            hp: 100.0,
            max_hp: 100.0,
            echo_power_factor: 1.0,
        },
        // Social
        FactionStanding::default(),
        Wallet::default(),
        AbilitySlots::default(),
        // Semantic identity (evolves based on player choices)
        SemanticTags::new(vec![("exploration", 0.5), ("neutral", 0.5)]),
    ));

    info!("Player entity spawned with all systems");
}

fn read_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut actions: EventWriter<PlayerActionEvent>,
    mut move_query: Query<&mut MovementInput, With<Player>>,
) {
    let Ok(mut move_input) = move_query.get_single_mut() else {
        return;
    };

    // Movement WASD
    let mut dir = Vec2::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        dir.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        dir.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        dir.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        dir.x += 1.0;
    }

    if dir.length_squared() > 0.01 {
        dir = dir.normalize();
    }
    move_input.direction = dir;

    // Jump
    move_input.jump = keyboard.just_pressed(KeyCode::Space);
    if move_input.jump {
        actions.send(PlayerActionEvent::Jump);
    }

    // Dash
    move_input.dash = keyboard.just_pressed(KeyCode::ShiftLeft);
    if move_input.dash {
        actions.send(PlayerActionEvent::Dash);
    }

    // Combat
    if mouse.just_pressed(MouseButton::Left) {
        actions.send(PlayerActionEvent::Attack);
    }
    if mouse.just_pressed(MouseButton::Right) {
        actions.send(PlayerActionEvent::Parry);
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        actions.send(PlayerActionEvent::Dodge);
    }

    // Abilities (1-4)
    for (key, slot) in [
        (KeyCode::Digit1, 0),
        (KeyCode::Digit2, 1),
        (KeyCode::Digit3, 2),
        (KeyCode::Digit4, 3),
    ] {
        if keyboard.just_pressed(key) {
            actions.send(PlayerActionEvent::UseAbility(slot));
        }
    }

    // Interact
    if keyboard.just_pressed(KeyCode::KeyE) {
        actions.send(PlayerActionEvent::Interact);
    }

    // Flight toggle
    if keyboard.just_pressed(KeyCode::KeyF) {
        actions.send(PlayerActionEvent::ToggleFlight);
    }

    // Dive attack
    if keyboard.just_pressed(KeyCode::KeyR) {
        actions.send(PlayerActionEvent::DiveAttack);
    }
}

fn process_player_actions(
    mut actions: EventReader<PlayerActionEvent>,
    mut query: Query<
        (
            &mut CombatState,
            &mut CombatResources,
            &mut FlightState,
            &mut DiveAttack,
            &AbilitySlots,
        ),
        With<Player>,
    >,
) {
    let Ok((mut combat, mut resources, mut flight, mut dive, abilities)) = query.get_single_mut()
    else {
        return;
    };

    for action in actions.read() {
        match action {
            PlayerActionEvent::Attack => {
                if combat.phase == crate::combat::AttackPhase::Idle {
                    combat.phase = crate::combat::AttackPhase::Windup;
                    combat.phase_timer = 0.0;
                    combat.combo_step = (combat.combo_step + 1).min(combat.max_combo);
                }
            }
            PlayerActionEvent::ToggleFlight => {
                use crate::aerial::FlightMode;
                flight.mode = match flight.mode {
                    FlightMode::Grounded => {
                        if flight.stamina > 10.0 {
                            FlightMode::Ascending
                        } else {
                            FlightMode::Grounded
                        }
                    }
                    FlightMode::Ascending | FlightMode::Hovering => FlightMode::Gliding,
                    FlightMode::Gliding => FlightMode::Grounded,
                    FlightMode::Diving => FlightMode::Gliding,
                };
            }
            PlayerActionEvent::DiveAttack => {
                if flight.mode != crate::aerial::FlightMode::Grounded && flight.altitude > 3.0 {
                    flight.mode = crate::aerial::FlightMode::Diving;
                    dive.active = true;
                }
            }
            PlayerActionEvent::UseAbility(slot) => {
                if let Some(Some(ability)) = abilities.slots.get(*slot as usize) {
                    // Check resource cost
                    let cost = &ability.resource_cost;
                    if resources.kinetic_energy >= cost.kinetic
                        && resources.thermal_energy >= cost.thermal
                        && resources.semantic_energy >= cost.semantic
                    {
                        resources.kinetic_energy -= cost.kinetic;
                        resources.thermal_energy -= cost.thermal;
                        resources.semantic_energy -= cost.semantic;
                    }
                }
            }
            _ => {}
        }
    }
}

fn update_player_level(mut query: Query<&mut Player>) {
    for mut player in &mut query {
        while player.xp >= player.xp_to_next {
            player.xp -= player.xp_to_next;
            player.level += 1;
            player.xp_to_next = xp_for_level(player.level);
            info!("Player leveled up to {}", player.level);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xp_curve() {
        let l1 = xp_for_level(1);
        let l10 = xp_for_level(10);
        let l50 = xp_for_level(50);

        assert_eq!(l1, 100);
        assert!(l10 > l1 * 5, "Level 10 should require much more XP");
        assert!(l50 > l10 * 10, "Level 50 should be significantly harder");
    }

    #[test]
    fn test_player_defaults() {
        let player = Player::default();
        assert_eq!(player.level, 1);
        assert_eq!(player.current_floor, 1);
        assert_eq!(player.xp, 0);
    }

    #[test]
    fn test_ability_resource_cost_default() {
        let cost = AbilityResourceCost::default();
        assert!((cost.kinetic - 0.0).abs() < f32::EPSILON);
        assert!((cost.thermal - 0.0).abs() < f32::EPSILON);
    }
}
