use crate::constants::*;
use crate::engine::config::EngineConfig;
use crate::engine::messages::WorldCycleMsg;
use crate::generation::TowerSeed;
use crate::world::BreathPhase;

/// GameStateService — manages world state queries and synchronization
pub struct GameStateService {
    #[allow(dead_code)]
    config: EngineConfig,
    #[allow(dead_code)]
    tower_seed: TowerSeed,
    current_tick: u64,
}

impl GameStateService {
    pub fn new(config: &EngineConfig) -> Self {
        Self {
            config: config.clone(),
            tower_seed: TowerSeed {
                seed: config.tower_seed,
            },
            current_tick: 0,
        }
    }

    pub fn get_world_cycle(&self, elapsed_seconds: f32) -> WorldCycleMsg {
        let cycle_pos = elapsed_seconds % BREATH_CYCLE_TOTAL;

        let hold_start = BREATH_INHALE_SECS;
        let exhale_start = hold_start + BREATH_HOLD_SECS;
        let pause_start = exhale_start + BREATH_EXHALE_SECS;

        let (phase, phase_progress) = if cycle_pos < hold_start {
            (BreathPhase::Inhale, cycle_pos / BREATH_INHALE_SECS)
        } else if cycle_pos < exhale_start {
            (
                BreathPhase::Hold,
                (cycle_pos - hold_start) / BREATH_HOLD_SECS,
            )
        } else if cycle_pos < pause_start {
            (
                BreathPhase::Exhale,
                (cycle_pos - exhale_start) / BREATH_EXHALE_SECS,
            )
        } else {
            (
                BreathPhase::Pause,
                (cycle_pos - pause_start) / BREATH_PAUSE_SECS,
            )
        };

        WorldCycleMsg {
            current_phase: format!("{:?}", phase),
            phase_progress,
            monster_spawn_mult: phase.monster_spawn_multiplier(),
            resource_mult: phase.resource_multiplier(),
            semantic_intensity: phase.semantic_intensity(),
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }
}
