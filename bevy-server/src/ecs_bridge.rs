//! ECS Bridge — Connects HTTP/JSON API handlers to the live Bevy ECS world
//!
//! The API runs on a separate tokio runtime, while Bevy runs its own game loop.
//! This module provides two-way communication:
//!
//! ```text
//! Axum Handler (tokio async)
//!       │
//!       ▼
//! GameCommand → mpsc channel → Bevy System (process_game_commands)
//!       │                           │
//!       │                           ▼
//!       │                     Mutate ECS World
//!       │                           │
//!       ▼                           ▼
//! oneshot::Receiver ◄── oneshot::Sender (response)
//!       │
//!       ▼
//! Axum Handler returns JSON
//! ```
//!
//! For read-only queries, handlers read from a shared `GameWorldSnapshot`
//! that is updated every tick by a Bevy system.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

use crate::components::{Monster, Player};

// ============================================================================
// Game World Snapshot (read-only, updated every tick)
// ============================================================================

/// Snapshot of live game state, readable by API handlers without blocking ECS
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameWorldSnapshot {
    /// Current server tick number
    pub tick: u64,
    /// All connected players
    pub players: HashMap<u64, PlayerSnapshot>,
    /// All active monsters per floor
    pub monsters_per_floor: HashMap<u32, Vec<MonsterSnapshot>>,
    /// Number of active entities
    pub entity_count: usize,
    /// Server uptime in seconds
    pub uptime_secs: f64,
    /// World cycle info
    pub world_cycle_phase: u32,
    /// Destruction stats per floor: (total, destroyed, percentage)
    pub destruction_stats: HashMap<u32, (u32, u32, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: u64,
    pub position: [f32; 3],
    pub health: f32,
    pub max_health: f32,
    pub current_floor: u32,
    pub in_combat: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterSnapshot {
    pub entity_id: u64,
    pub monster_type: String,
    pub position: [f32; 3],
    pub health: f32,
    pub max_health: f32,
}

/// Shared handle to the world snapshot (Arc<RwLock<>> for API access)
pub type SharedWorldSnapshot = Arc<RwLock<GameWorldSnapshot>>;

// ============================================================================
// Game Commands (API → Bevy ECS)
// ============================================================================

/// Commands sent from API handlers to the Bevy ECS world
#[derive(Debug)]
pub enum GameCommand {
    /// Move a player to a new position
    MovePlayer {
        player_id: u64,
        position: [f32; 3],
        reply: oneshot::Sender<CommandResult>,
    },
    /// Deal damage to a target
    DealDamage {
        attacker_id: u64,
        target_id: u64,
        damage: f32,
        reply: oneshot::Sender<DamageResult>,
    },
    /// Spawn a monster on a floor
    SpawnMonster {
        floor_id: u32,
        monster_type: String,
        position: [f32; 3],
        health: f32,
        reply: oneshot::Sender<SpawnResult>,
    },
    /// Apply destruction damage (from DestructionService)
    DestroyObject {
        entity_id: u64,
        floor_id: u32,
        impact_point: [f32; 3],
        damage: f32,
        radius: f32,
        damage_type: String,
        reply: oneshot::Sender<DestructionCommandResult>,
    },
    /// Get live player count
    GetPlayerCount { reply: oneshot::Sender<usize> },
    /// Get live player state
    GetPlayer {
        player_id: u64,
        reply: oneshot::Sender<Option<PlayerSnapshot>>,
    },
    /// Process a combat action (attack, parry, dodge, block)
    CombatAction {
        player_id: u64,
        action: crate::combat::ActionType,
        position: [f32; 3],
        facing: f32,
        reply: oneshot::Sender<CombatActionCommandResult>,
    },
}

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct DamageResult {
    pub success: bool,
    pub damage_dealt: f32,
    pub target_health: f32,
    pub target_killed: bool,
}

#[derive(Debug, Serialize)]
pub struct SpawnResult {
    pub success: bool,
    pub entity_id: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct DestructionCommandResult {
    pub success: bool,
    pub damage_dealt: f32,
    pub newly_destroyed: Vec<u8>,
    pub collapsed: bool,
}

#[derive(Debug, Serialize)]
pub struct CombatActionCommandResult {
    pub success: bool,
    pub action_result: Option<crate::combat::CombatActionResult>,
    pub message: String,
}

/// Channel sender type for API handlers to send commands
pub type CommandSender = mpsc::UnboundedSender<GameCommand>;
/// Channel receiver type for Bevy system to receive commands
pub type CommandReceiver = mpsc::UnboundedReceiver<GameCommand>;

// ============================================================================
// Bevy Resources
// ============================================================================

/// Resource holding the command receiver (consumed by Bevy system)
#[derive(Resource)]
pub struct GameCommandReceiver {
    pub receiver: CommandReceiver,
}

/// Resource holding the shared world snapshot
#[derive(Resource)]
pub struct WorldSnapshotResource {
    pub snapshot: SharedWorldSnapshot,
}

/// Resource tracking server uptime
#[derive(Resource, Default)]
pub struct ServerUptime {
    pub ticks: u64,
    pub total_time: f64,
}

// ============================================================================
// Bevy Systems
// ============================================================================

/// System: Update the world snapshot every tick (runs at 20 Hz)
pub fn update_world_snapshot(
    snapshot_res: Res<WorldSnapshotResource>,
    players: Query<&Player>,
    monsters: Query<&Monster>,
    uptime: Res<ServerUptime>,
    destruction: Res<crate::destruction::FloorDestructionManager>,
) {
    let mut snap = GameWorldSnapshot {
        tick: uptime.ticks,
        uptime_secs: uptime.total_time,
        entity_count: 0,
        ..Default::default()
    };

    // Snapshot players
    for player in &players {
        snap.players.insert(
            player.id,
            PlayerSnapshot {
                id: player.id,
                position: [player.position.x, player.position.y, player.position.z],
                health: player.health,
                max_health: 100.0, // TODO: from player stats
                current_floor: player.current_floor,
                in_combat: false, // TODO: from combat state
            },
        );
    }
    snap.entity_count += snap.players.len();

    // Snapshot monsters
    for (i, monster) in monsters.iter().enumerate() {
        let floor_id = 1u32; // TODO: monster floor tracking
        snap.monsters_per_floor
            .entry(floor_id)
            .or_default()
            .push(MonsterSnapshot {
                entity_id: i as u64,
                monster_type: monster.monster_type.clone(),
                position: [monster.position.x, monster.position.y, monster.position.z],
                health: monster.health,
                max_health: monster.max_health,
            });
    }
    for list in snap.monsters_per_floor.values() {
        snap.entity_count += list.len();
    }

    // Snapshot destruction stats
    for (&floor_id, floor) in &destruction.floors {
        let total = floor.len() as u32;
        let destroyed = floor.values().filter(|d| d.collapsed).count() as u32;
        let pct = if total > 0 {
            destroyed as f32 / total as f32
        } else {
            0.0
        };
        snap.destruction_stats
            .insert(floor_id, (total, destroyed, pct));
    }

    // Write snapshot (blocks readers briefly)
    if let Ok(mut lock) = snapshot_res.snapshot.write() {
        *lock = snap;
    }
}

/// System: Process incoming game commands from API handlers
#[allow(clippy::too_many_arguments)]
pub fn process_game_commands(
    mut cmd_res: ResMut<GameCommandReceiver>,
    mut commands: Commands,
    mut players: Query<(Entity, &mut Player)>,
    mut monsters: Query<(Entity, &mut Monster)>,
    mut destruction: ResMut<crate::destruction::FloorDestructionManager>,
    mut combat_states: Query<&mut crate::combat::CombatState>,
    weapons: Query<&crate::combat::EquippedWeapon>,
    movesets: Res<crate::combat::WeaponMovesets>,
) {
    // Process up to 64 commands per tick to avoid stalling the game loop
    let mut processed = 0;
    while let Ok(cmd) = cmd_res.receiver.try_recv() {
        if processed >= 64 {
            break;
        }
        processed += 1;

        match cmd {
            GameCommand::MovePlayer {
                player_id,
                position,
                reply,
            } => {
                let result = if let Some((_, mut player)) =
                    players.iter_mut().find(|(_, p)| p.id == player_id)
                {
                    player.position = Vec3::new(position[0], position[1], position[2]);
                    CommandResult {
                        success: true,
                        message: "Moved".into(),
                    }
                } else {
                    CommandResult {
                        success: false,
                        message: "Player not found".into(),
                    }
                };
                let _ = reply.send(result);
            }

            GameCommand::DealDamage {
                attacker_id: _,
                target_id,
                damage,
                reply,
            } => {
                // Try to find target as monster
                let result = if let Some((_, mut monster)) = monsters
                    .iter_mut()
                    .enumerate()
                    .find(|(i, _)| *i as u64 == target_id)
                    .map(|(_, m)| m)
                {
                    monster.health -= damage;
                    let killed = monster.health <= 0.0;
                    if killed {
                        monster.health = 0.0;
                    }
                    DamageResult {
                        success: true,
                        damage_dealt: damage,
                        target_health: monster.health,
                        target_killed: killed,
                    }
                } else {
                    DamageResult {
                        success: false,
                        damage_dealt: 0.0,
                        target_health: 0.0,
                        target_killed: false,
                    }
                };
                let _ = reply.send(result);
            }

            GameCommand::SpawnMonster {
                floor_id: _,
                monster_type,
                position,
                health,
                reply,
            } => {
                let entity = commands
                    .spawn((Monster {
                        monster_type,
                        position: Vec3::new(position[0], position[1], position[2]),
                        health,
                        max_health: health,
                    },))
                    .id();
                let _ = reply.send(SpawnResult {
                    success: true,
                    entity_id: Some(entity.index() as u64),
                });
            }

            GameCommand::DestroyObject {
                entity_id,
                floor_id,
                impact_point,
                damage,
                radius,
                damage_type,
                reply,
            } => {
                let dt = match damage_type.as_str() {
                    "explosive" => crate::destruction::DestructionDamageType::Explosive,
                    "fire" => crate::destruction::DestructionDamageType::ElementalFire,
                    "ice" => crate::destruction::DestructionDamageType::ElementalIce,
                    "lightning" => crate::destruction::DestructionDamageType::ElementalLightning,
                    "semantic" => crate::destruction::DestructionDamageType::Semantic,
                    _ => crate::destruction::DestructionDamageType::Kinetic,
                };

                let result = match destruction.apply_damage(
                    entity_id,
                    floor_id,
                    Vec3::new(impact_point[0], impact_point[1], impact_point[2]),
                    Vec3::ZERO, // entity position (would come from Transform)
                    damage,
                    radius,
                    dt,
                ) {
                    Some(r) => DestructionCommandResult {
                        success: true,
                        damage_dealt: r.damage_dealt,
                        newly_destroyed: r.newly_destroyed_clusters,
                        collapsed: r.structural_collapse,
                    },
                    None => DestructionCommandResult {
                        success: false,
                        damage_dealt: 0.0,
                        newly_destroyed: vec![],
                        collapsed: false,
                    },
                };
                let _ = reply.send(result);
            }

            GameCommand::GetPlayerCount { reply } => {
                let _ = reply.send(players.iter().count());
            }

            GameCommand::GetPlayer { player_id, reply } => {
                let snap = players
                    .iter()
                    .find(|(_, p)| p.id == player_id)
                    .map(|(_, p)| PlayerSnapshot {
                        id: p.id,
                        position: [p.position.x, p.position.y, p.position.z],
                        health: p.health,
                        max_health: 100.0,
                        current_floor: p.current_floor,
                        in_combat: false,
                    });
                let _ = reply.send(snap);
            }

            GameCommand::CombatAction {
                player_id,
                action,
                position: _,
                facing,
                reply,
            } => {
                // Find the player entity
                let player_entity = players
                    .iter()
                    .find(|(_, p)| p.id == player_id)
                    .map(|(e, _)| e);

                let result = if let Some(entity) = player_entity {
                    // Get combat state and weapon
                    let combat_state = combat_states.get_mut(entity);
                    let weapon = weapons.get(entity);

                    match (combat_state, weapon) {
                        (Ok(mut cs), Ok(w)) => {
                            cs.facing = facing;
                            let action_result =
                                crate::combat::try_combat_action(&mut cs, action, w, &movesets);
                            CombatActionCommandResult {
                                success: action_result.success,
                                message: action_result.message.clone(),
                                action_result: Some(action_result),
                            }
                        }
                        _ => CombatActionCommandResult {
                            success: false,
                            action_result: None,
                            message: "Player has no combat state or weapon".into(),
                        },
                    }
                } else {
                    CombatActionCommandResult {
                        success: false,
                        action_result: None,
                        message: "Player not found".into(),
                    }
                };
                let _ = reply.send(result);
            }
        }
    }
}

/// System: Track server uptime
pub fn update_uptime(time: Res<Time>, mut uptime: ResMut<ServerUptime>) {
    uptime.ticks += 1;
    uptime.total_time += time.delta_secs() as f64;
}

// ============================================================================
// Channel Factory
// ============================================================================

/// Create the bridge channels and shared resources.
/// Returns (CommandSender for API, GameCommandReceiver for Bevy, SharedWorldSnapshot)
pub fn create_bridge() -> (CommandSender, GameCommandReceiver, SharedWorldSnapshot) {
    let (tx, rx) = mpsc::unbounded_channel();
    let snapshot = Arc::new(RwLock::new(GameWorldSnapshot::default()));

    (tx, GameCommandReceiver { receiver: rx }, snapshot)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bridge() {
        let (tx, _rx, snapshot) = create_bridge();

        // Sender should be usable
        assert!(!tx.is_closed());

        // Snapshot should be default
        let snap = snapshot.read().unwrap();
        assert_eq!(snap.tick, 0);
        assert!(snap.players.is_empty());
    }

    #[test]
    fn test_snapshot_write_read() {
        let snapshot: SharedWorldSnapshot = Arc::new(RwLock::new(GameWorldSnapshot::default()));

        // Write
        {
            let mut snap = snapshot.write().unwrap();
            snap.tick = 42;
            snap.players.insert(
                1,
                PlayerSnapshot {
                    id: 1,
                    position: [10.0, 0.0, 20.0],
                    health: 80.0,
                    max_health: 100.0,
                    current_floor: 5,
                    in_combat: true,
                },
            );
        }

        // Read
        {
            let snap = snapshot.read().unwrap();
            assert_eq!(snap.tick, 42);
            assert_eq!(snap.players.len(), 1);
            let p = snap.players.get(&1).unwrap();
            assert_eq!(p.health, 80.0);
            assert_eq!(p.current_floor, 5);
            assert!(p.in_combat);
        }
    }

    #[tokio::test]
    async fn test_command_channel() {
        let (tx, mut rx, _) = create_bridge();

        // Send a command
        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(GameCommand::GetPlayerCount { reply: reply_tx })
            .unwrap();

        // Receive on the Bevy side
        let cmd = rx.receiver.recv().await.unwrap();
        match cmd {
            GameCommand::GetPlayerCount { reply } => {
                reply.send(5).unwrap();
            }
            _ => panic!("Wrong command type"),
        }

        // API handler gets the response
        let count = reply_rx.await.unwrap();
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_move_player_command() {
        let (tx, mut rx, _) = create_bridge();

        let (reply_tx, reply_rx) = oneshot::channel();
        tx.send(GameCommand::MovePlayer {
            player_id: 42,
            position: [1.0, 2.0, 3.0],
            reply: reply_tx,
        })
        .unwrap();

        // Simulate Bevy processing
        let cmd = rx.receiver.recv().await.unwrap();
        match cmd {
            GameCommand::MovePlayer {
                player_id,
                position,
                reply,
            } => {
                assert_eq!(player_id, 42);
                assert_eq!(position, [1.0, 2.0, 3.0]);
                reply
                    .send(CommandResult {
                        success: true,
                        message: "Moved".into(),
                    })
                    .unwrap();
            }
            _ => panic!("Wrong command"),
        }

        let result = reply_rx.await.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_snapshot_destruction_stats() {
        let snapshot: SharedWorldSnapshot = Arc::new(RwLock::new(GameWorldSnapshot::default()));

        {
            let mut snap = snapshot.write().unwrap();
            snap.destruction_stats.insert(1, (50, 10, 0.2));
            snap.destruction_stats.insert(2, (30, 30, 1.0));
        }

        let snap = snapshot.read().unwrap();
        assert_eq!(snap.destruction_stats.get(&1), Some(&(50, 10, 0.2)));
        assert_eq!(snap.destruction_stats.get(&2), Some(&(30, 30, 1.0)));
    }
}
