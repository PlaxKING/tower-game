//! Hybrid Game Engine — Integration Layer
//!
//! Implements the 5 gRPC service interfaces defined in shared/proto/services.proto
//! using the existing procedural-core modules as backend.
//!
//! Architecture:
//!   Rust/Bevy (this crate) ←→ gRPC/JSON ←→ UE5 Client
//!   Rust/Bevy (this crate) ←→ FFI/DLL   ←→ UE5 Client (fallback)
//!
//! Services:
//!   1. GameStateService  — World state management & streaming
//!   2. CombatService     — Combat logic processing
//!   3. GenerationService — Procedural content generation
//!   4. MasteryService    — Skill mastery & progression
//!   5. EconomyService    — Trading, crafting, economy

pub mod config;
mod helpers;
pub mod hybrid;
pub mod messages;
pub mod plugin;
pub mod services;

#[allow(unused_imports)]
pub use config::{EngineConfig, TransportMode};
#[allow(unused_imports)]
pub use hybrid::HybridEngine;
#[allow(unused_imports)]
pub use messages::*;
#[allow(unused_imports)]
pub use plugin::{EnginePlugin, EngineResource};
#[allow(unused_imports)]
pub use services::{
    CombatService, CraftResultMsg, EconomyService, GameStateService, GenerationService,
    MasteryService, TradeResultMsg, WalletMsg, WalletState,
};

// =====================================================
// Tests
// =====================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> HybridEngine {
        HybridEngine::new(EngineConfig::default())
    }

    #[test]
    fn test_engine_creation() {
        let engine = test_engine();
        assert_eq!(engine.config.tower_seed, 42);
        assert_eq!(engine.config.tick_rate, 60);
        assert_eq!(engine.config.transport, TransportMode::Json);
    }

    #[test]
    fn test_engine_tick() {
        let mut engine = test_engine();
        assert_eq!(engine.game_state.current_tick(), 0);
        engine.tick(1.0 / 60.0);
        assert_eq!(engine.game_state.current_tick(), 1);
        engine.tick(1.0 / 60.0);
        assert_eq!(engine.game_state.current_tick(), 2);
    }

    #[test]
    fn test_world_cycle() {
        let engine = test_engine();
        let cycle = engine.world_cycle();
        assert_eq!(cycle.current_phase, "Inhale");
        assert_eq!(cycle.phase_progress, 0.0);
    }

    #[test]
    fn test_generate_floor() {
        let engine = test_engine();
        let floor = engine.generate_floor(1);
        assert_eq!(floor.floor_id, 1);
        assert!(!floor.biome_tags.is_empty());
        assert!(floor.layout.width > 0);
        assert!(floor.layout.height > 0);
        assert!(!floor.monsters.is_empty());
    }

    #[test]
    fn test_floor_deterministic() {
        let engine = test_engine();
        let floor1 = engine.generate_floor(5);
        let floor2 = engine.generate_floor(5);
        assert_eq!(floor1.floor_hash, floor2.floor_hash);
        assert_eq!(floor1.monsters.len(), floor2.monsters.len());
        assert_eq!(floor1.tier, floor2.tier);
    }

    #[test]
    fn test_calculate_damage() {
        let engine = test_engine();
        let result = engine.calculate_damage(
            100.0,
            2,
            1,
            &[("fire".into(), 0.8)],
            &[("water".into(), 0.9)],
        );
        assert!(result.modified_damage > 100.0); // back attack + combo
        assert!(result.modifiers.len() >= 2);
    }

    #[test]
    fn test_damage_angle_variants() {
        let engine = test_engine();
        let front = engine.calculate_damage(100.0, 0, 0, &[], &[]);
        let side = engine.calculate_damage(100.0, 1, 0, &[], &[]);
        let back = engine.calculate_damage(100.0, 2, 0, &[], &[]);

        assert!(side.modified_damage < front.modified_damage);
        assert!(back.modified_damage > front.modified_damage);
    }

    #[test]
    fn test_semantic_similarity() {
        let engine = test_engine();
        let sim = engine.generation.query_semantic_similarity(
            &[("fire".into(), 0.8), ("attack".into(), 0.6)],
            &[("fire".into(), 0.9), ("attack".into(), 0.5)],
        );
        assert!(sim > 0.0);
    }

    #[test]
    fn test_track_mastery() {
        let mut engine = test_engine();
        let result = engine.track_mastery(1, "sword", 50.0);
        assert_eq!(result.domain, "sword");
        assert!(result.new_xp > 0.0);
    }

    #[test]
    fn test_mastery_tier_up() {
        let mut engine = test_engine();
        // Add enough XP to tier up (Novice -> Apprentice at 100 XP)
        let result = engine.track_mastery(1, "sword", 150.0);
        assert!(result.tier_up);
        assert!(result.new_tier >= 1);
    }

    #[test]
    fn test_mastery_profile() {
        let mut engine = test_engine();
        engine.track_mastery(1, "sword", 50.0);
        engine.track_mastery(1, "dodge", 30.0);
        let profile = engine.mastery.get_mastery_profile(1);
        assert_eq!(profile.domains.len(), 21);
    }

    #[test]
    fn test_economy_wallet() {
        let mut engine = test_engine();
        let wallet = engine.get_wallet(1);
        assert_eq!(wallet.gold, 0);

        engine.economy.add_gold(1, 1000);
        let wallet = engine.get_wallet(1);
        assert_eq!(wallet.gold, 1000);
    }

    #[test]
    fn test_economy_spend() {
        let mut engine = test_engine();
        engine.economy.add_gold(1, 500);
        assert!(engine.economy.try_spend_gold(1, 200));
        assert_eq!(engine.get_wallet(1).gold, 300);
        assert!(!engine.economy.try_spend_gold(1, 400));
    }

    #[test]
    fn test_economy_trade() {
        let mut engine = test_engine();
        engine.economy.add_gold(1, 1000);
        engine.economy.add_gold(2, 500);

        let result = engine.economy.trade(1, 2, 300, 200);
        assert!(result.success);
        assert_eq!(engine.get_wallet(1).gold, 900);
        assert_eq!(engine.get_wallet(2).gold, 600);
    }

    #[test]
    fn test_trade_insufficient_gold() {
        let mut engine = test_engine();
        engine.economy.add_gold(1, 100);
        engine.economy.add_gold(2, 500);

        let result = engine.economy.trade(1, 2, 200, 0);
        assert!(!result.success);
        assert!(result.failure_reason.contains("insufficient"));
    }

    #[test]
    fn test_generate_loot() {
        let engine = test_engine();
        let loot =
            engine.generate_loot(&[("fire".into(), 0.8), ("corruption".into(), 0.3)], 10, 42);
        assert!(!loot.is_empty());
    }

    #[test]
    fn test_engine_config_serialization() {
        let config = EngineConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("server_host"));
        assert!(json.contains("tower_seed"));
        let parsed: EngineConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tower_seed, config.tower_seed);
    }

    #[test]
    fn test_transport_modes() {
        let config = EngineConfig {
            transport: TransportMode::Protobuf,
            ..Default::default()
        };
        assert_eq!(config.transport, TransportMode::Protobuf);

        let config = EngineConfig {
            transport: TransportMode::Ffi,
            ..Default::default()
        };
        assert_eq!(config.transport, TransportMode::Ffi);
    }

    #[test]
    fn test_floor_monsters_have_grammar() {
        let engine = test_engine();
        let floor = engine.generate_floor(5);
        for monster in &floor.monsters {
            assert!(!monster.grammar.body_type.is_empty());
            assert!(!monster.grammar.locomotion.is_empty());
            assert!(!monster.grammar.attack_style.is_empty());
        }
    }

    #[test]
    fn test_world_cycle_phases() {
        let engine = test_engine();
        // Inhale phase (0-360s)
        let inhale = engine.game_state.get_world_cycle(180.0);
        assert_eq!(inhale.current_phase, "Inhale");
        assert!((inhale.phase_progress - 0.5).abs() < 0.01);

        // Hold phase (360-600s)
        let hold = engine.game_state.get_world_cycle(480.0);
        assert_eq!(hold.current_phase, "Hold");

        // Exhale phase (600-960s)
        let exhale = engine.game_state.get_world_cycle(780.0);
        assert_eq!(exhale.current_phase, "Exhale");

        // Pause phase (960-1080s)
        let pause = engine.game_state.get_world_cycle(1020.0);
        assert_eq!(pause.current_phase, "Pause");
    }
}
