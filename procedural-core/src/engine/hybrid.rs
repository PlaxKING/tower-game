use crate::engine::config::EngineConfig;
use crate::engine::messages::{
    DamageCalcResultMsg, FloorResponseMsg, LootItemMsg, MasteryProgressResultMsg, WorldCycleMsg,
};
use crate::engine::services::{
    CombatService, EconomyService, GameStateService, GenerationService, MasteryService, WalletMsg,
};

/// The main game engine that orchestrates all services.
/// Holds the service instances and manages the game loop.
pub struct HybridEngine {
    pub config: EngineConfig,
    pub game_state: GameStateService,
    pub combat: CombatService,
    pub generation: GenerationService,
    pub mastery: MasteryService,
    pub economy: EconomyService,
    elapsed_seconds: f32,
}

impl HybridEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            game_state: GameStateService::new(&config),
            combat: CombatService::new(&config),
            generation: GenerationService::new(&config),
            mastery: MasteryService::new(&config),
            economy: EconomyService::new(&config),
            config,
            elapsed_seconds: 0.0,
        }
    }

    /// Advance the engine by one tick
    pub fn tick(&mut self, delta_seconds: f32) {
        self.elapsed_seconds += delta_seconds;
        self.game_state.tick();
    }

    /// Get current world cycle state
    pub fn world_cycle(&self) -> WorldCycleMsg {
        self.game_state.get_world_cycle(self.elapsed_seconds)
    }

    /// Generate a full floor with layout, monsters, and loot
    pub fn generate_floor(&self, floor_id: u32) -> FloorResponseMsg {
        self.generation.generate_floor(floor_id)
    }

    /// Calculate damage for a combat interaction
    pub fn calculate_damage(
        &self,
        base_damage: f32,
        angle_id: u32,
        combo_step: u32,
        attacker_tags: &[(String, f32)],
        defender_tags: &[(String, f32)],
    ) -> DamageCalcResultMsg {
        self.combat.calculate_damage(
            base_damage,
            angle_id,
            combo_step,
            attacker_tags,
            defender_tags,
        )
    }

    /// Track mastery progress for a player
    pub fn track_mastery(
        &mut self,
        player_id: u64,
        domain: &str,
        xp: f32,
    ) -> MasteryProgressResultMsg {
        self.mastery.track_progress(player_id, domain, xp)
    }

    /// Generate loot from a kill
    pub fn generate_loot(
        &self,
        source_tags: &[(String, f32)],
        floor_level: u32,
        hash: u64,
    ) -> Vec<LootItemMsg> {
        self.generation
            .generate_loot(source_tags, floor_level, hash)
    }

    /// Get player's wallet
    pub fn get_wallet(&self, player_id: u64) -> WalletMsg {
        self.economy.get_wallet(player_id)
    }

    pub fn elapsed(&self) -> f32 {
        self.elapsed_seconds
    }
}
