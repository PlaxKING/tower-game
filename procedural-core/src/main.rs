use bevy::prelude::*;

mod abilities;
mod achievements;
mod aerial;
mod analytics;
mod anticheat;
mod balance;
mod bridge;
mod combat;
mod constants;
mod cosmetics;
mod death;
mod economy;
mod engine;
mod equipment;
mod events;
mod faction;
mod gameflow;
mod generation;
mod hotreload;
mod logging;
mod loot;
mod mastery;
mod monster;
mod movement;
mod mutators;
mod player;
mod replay;
mod replication;
mod savemigration;
mod seasons;
mod semantic;
mod social;
mod sockets;
mod specialization;
mod towermap;
mod tutorial;
mod visualization;
mod world;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Tower Game - Procedural Core".into(),
                resolution: (1280., 720.).into(),
                ..default()
            }),
            ..default()
        }))
        // Core systems
        .add_plugins(semantic::SemanticPlugin)
        .add_plugins(generation::GenerationPlugin)
        // Gameplay systems
        .add_plugins(combat::CombatPlugin)
        .add_plugins(movement::MovementPlugin)
        .add_plugins(aerial::AerialPlugin)
        .add_plugins(death::DeathPlugin)
        // Entity systems
        .add_plugins(monster::MonsterPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(loot::LootPlugin)
        // World systems
        .add_plugins(world::WorldPlugin)
        .add_plugins(faction::FactionPlugin)
        .add_plugins(economy::EconomyPlugin)
        // Replication & Events
        .add_plugins(replication::ReplicationPlugin)
        .add_plugins(events::EventsPlugin)
        // Visualization
        .add_plugins(visualization::VisualizationPlugin)
        // Hybrid Engine
        .add_plugins(engine::EnginePlugin)
        // Floor Mutators
        .add_plugins(mutators::MutatorsPlugin)
        // Game Flow States
        .add_plugins(gameflow::GameFlowPlugin)
        // Logging
        .add_plugins(logging::LoggingPlugin)
        // Hot-reload
        .add_plugins(hotreload::HotReloadPlugin)
        // Analytics
        .add_plugins(analytics::AnalyticsPlugin)
        // Replay System
        .add_plugins(replay::ReplayPlugin)
        // Tower Map
        .add_plugins(towermap::TowerMapPlugin)
        // Startup
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
    ));

    info!("Tower Game Procedural Core initialized");
    info!("Phase 1: All systems loaded — semantic, generation, combat, movement, aerial, death, monster, player, loot, world, faction, economy");
}
