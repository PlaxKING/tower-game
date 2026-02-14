use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(BreathOfTower::default())
            .add_systems(Update, update_breath_cycle);
    }
}

/// Breath of the Tower - global 18-hour cycle affecting all gameplay
/// Inhale (6h) -> Hold (4h) -> Exhale (6h) -> Pause (2h)
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct BreathOfTower {
    pub phase: BreathPhase,
    pub phase_timer: f32, // seconds elapsed in current phase
    pub cycle_count: u32, // how many full cycles completed
    pub intensity: f32,   // 0.0-1.0, ramps within each phase
}

impl Default for BreathOfTower {
    fn default() -> Self {
        Self {
            phase: BreathPhase::Inhale,
            phase_timer: 0.0,
            cycle_count: 0,
            intensity: 0.0,
        }
    }
}

/// Phases of the Tower's breath cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BreathPhase {
    /// 6 hours: Tower draws energy inward
    /// - Monster spawn rates increase
    /// - Resources become more abundant
    /// - Semantic tags shift toward intensity
    Inhale,

    /// 4 hours: Peak energy, tower at maximum power
    /// - Boss encounters available
    /// - Rare resources spawn
    /// - Maximum semantic tag intensity
    Hold,

    /// 6 hours: Tower releases energy outward
    /// - Environmental hazards increase
    /// - New paths/shortcuts open
    /// - Semantic tags shift toward exploration
    Exhale,

    /// 2 hours: Calm period between breaths
    /// - Safe trading/crafting window
    /// - Reduced monster activity
    /// - Echo visibility maximized
    Pause,
}

impl BreathPhase {
    /// Duration in real seconds (for development: 1 min = 1 game hour)
    /// Production: multiply by 60 for real hours
    pub fn duration_secs(&self) -> f32 {
        match self {
            Self::Inhale => 360.0, // 6 minutes (dev) / 6 hours (prod)
            Self::Hold => 240.0,   // 4 minutes / 4 hours
            Self::Exhale => 360.0, // 6 minutes / 6 hours
            Self::Pause => 120.0,  // 2 minutes / 2 hours
        }
    }

    /// Next phase in the cycle
    pub fn next(&self) -> Self {
        match self {
            Self::Inhale => Self::Hold,
            Self::Hold => Self::Exhale,
            Self::Exhale => Self::Pause,
            Self::Pause => Self::Inhale,
        }
    }

    /// Monster spawn rate multiplier
    pub fn monster_spawn_multiplier(&self) -> f32 {
        match self {
            Self::Inhale => 1.5,
            Self::Hold => 2.0,
            Self::Exhale => 1.0,
            Self::Pause => 0.3,
        }
    }

    /// Resource abundance multiplier
    pub fn resource_multiplier(&self) -> f32 {
        match self {
            Self::Inhale => 1.3,
            Self::Hold => 1.8,
            Self::Exhale => 1.0,
            Self::Pause => 0.8,
        }
    }

    /// Semantic tag intensity modifier
    pub fn semantic_intensity(&self) -> f32 {
        match self {
            Self::Inhale => 0.8,
            Self::Hold => 1.0,
            Self::Exhale => 0.6,
            Self::Pause => 0.4,
        }
    }
}

/// Tower environmental effect applied to a region
#[derive(Component, Debug)]
pub struct TowerEnvironment {
    pub floor_id: u32,
    pub ambient_danger: f32,   // 0.0-1.0
    pub visibility_range: f32, // view distance
    pub semantic_field: Vec3,  // dominant semantic direction
}

fn update_breath_cycle(time: Res<Time>, mut breath: ResMut<BreathOfTower>) {
    let dt = time.delta_secs();
    breath.phase_timer += dt;

    let duration = breath.phase.duration_secs();

    // Update intensity: ramps up then down within phase
    let progress = breath.phase_timer / duration;
    breath.intensity = if progress < 0.5 {
        progress * 2.0 // ramp up
    } else {
        (1.0 - progress) * 2.0 // ramp down
    };

    // Phase transition
    if breath.phase_timer >= duration {
        let next = breath.phase.next();
        if next == BreathPhase::Inhale {
            breath.cycle_count += 1;
        }
        breath.phase = next;
        breath.phase_timer = 0.0;
        breath.intensity = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breath_cycle_order() {
        assert_eq!(BreathPhase::Inhale.next(), BreathPhase::Hold);
        assert_eq!(BreathPhase::Hold.next(), BreathPhase::Exhale);
        assert_eq!(BreathPhase::Exhale.next(), BreathPhase::Pause);
        assert_eq!(BreathPhase::Pause.next(), BreathPhase::Inhale);
    }

    #[test]
    fn test_total_cycle_duration() {
        let total = BreathPhase::Inhale.duration_secs()
            + BreathPhase::Hold.duration_secs()
            + BreathPhase::Exhale.duration_secs()
            + BreathPhase::Pause.duration_secs();
        // 18 minutes in dev mode (6+4+6+2)
        assert!((total - 1080.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_monster_spawn_multipliers() {
        // Hold phase has highest spawn rate
        assert!(
            BreathPhase::Hold.monster_spawn_multiplier()
                > BreathPhase::Inhale.monster_spawn_multiplier()
        );
        // Pause has lowest
        assert!(BreathPhase::Pause.monster_spawn_multiplier() < 1.0);
    }

    #[test]
    fn test_default_breath() {
        let breath = BreathOfTower::default();
        assert_eq!(breath.phase, BreathPhase::Inhale);
        assert_eq!(breath.cycle_count, 0);
    }
}
