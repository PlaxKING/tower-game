use bevy::prelude::*;
use std::time::Duration;

/// Dynamic scaling based on server load and player count
#[derive(Resource)]
pub struct DynamicScaling {
    // Performance metrics
    pub current_tick_time: Duration,
    pub target_tick_time: Duration,  // 50ms for 20 Hz
    pub avg_tick_time: Duration,

    // Player distribution
    pub players_per_floor: HashMap<u32, usize>,
    pub total_players: usize,

    // Scaling parameters (adjust based on performance)
    pub max_players_per_floor: usize,
    pub current_floor_capacity: usize,

    // Timer for periodic capacity updates
    last_capacity_update: Duration,
    capacity_update_interval: Duration,
}

impl Default for DynamicScaling {
    fn default() -> Self {
        Self {
            current_tick_time: Duration::from_millis(50),
            target_tick_time: Duration::from_millis(50),
            avg_tick_time: Duration::from_millis(50),
            players_per_floor: HashMap::new(),
            total_players: 0,
            max_players_per_floor: 100,
            current_floor_capacity: 100,
            last_capacity_update: Duration::ZERO,
            capacity_update_interval: Duration::from_secs(5),
        }
    }
}

impl DynamicScaling {
    /// Adjust capacity based on server performance
    pub fn update_capacity(&mut self) {
        let performance_ratio = self.avg_tick_time.as_secs_f32()
                              / self.target_tick_time.as_secs_f32();

        match performance_ratio {
            // Server running smoothly - increase capacity
            r if r < 0.7 => {
                self.current_floor_capacity = (self.max_players_per_floor * 120 / 100).min(150);
                info!("📈 Performance good, increasing capacity to {}", self.current_floor_capacity);
            }

            // Server normal - maintain
            r if r < 0.9 => {
                self.current_floor_capacity = self.max_players_per_floor;
            }

            // Server struggling - reduce capacity
            r if r < 1.2 => {
                self.current_floor_capacity = self.max_players_per_floor * 80 / 100;
                warn!("⚠️ Performance degraded, reducing capacity to {}", self.current_floor_capacity);
            }

            // Server overloaded - emergency reduction
            _ => {
                self.current_floor_capacity = self.max_players_per_floor * 60 / 100;
                error!("🚨 Server overloaded! Emergency capacity reduction to {}", self.current_floor_capacity);
            }
        }
    }

    /// Check if floor can accept more players
    pub fn can_join_floor(&self, floor_id: u32) -> bool {
        let current_count = self.players_per_floor.get(&floor_id).copied().unwrap_or(0);
        current_count < self.current_floor_capacity
    }

    /// Suggest best floor for new player (load balancing)
    pub fn suggest_floor(&self, preferred_floor: u32) -> u32 {
        // Try preferred floor first
        if self.can_join_floor(preferred_floor) {
            return preferred_floor;
        }

        // Find nearby floor with space
        for offset in 1..10 {
            for floor in [preferred_floor + offset, preferred_floor.saturating_sub(offset)] {
                if self.can_join_floor(floor) {
                    info!("🔀 Redirecting player from floor {} to {} (load balancing)",
                          preferred_floor, floor);
                    return floor;
                }
            }
        }

        // Create new instance if needed
        preferred_floor
    }
}

/// System to monitor and adjust scaling
pub fn monitor_performance_system(
    mut scaling: ResMut<DynamicScaling>,
    time: Res<Time>,
    players: Query<&Player>,
) {
    // Update tick time
    scaling.current_tick_time = time.delta();

    // Exponential moving average (smoothing)
    scaling.avg_tick_time = Duration::from_secs_f32(
        0.9 * scaling.avg_tick_time.as_secs_f32()
        + 0.1 * scaling.current_tick_time.as_secs_f32()
    );

    // Count players per floor
    scaling.players_per_floor.clear();
    for player in &players {
        *scaling.players_per_floor.entry(player.current_floor).or_insert(0) += 1;
    }
    scaling.total_players = players.iter().count();

    // Adjust capacity every 5 seconds
    let elapsed = time.elapsed();
    if elapsed - scaling.last_capacity_update >= scaling.capacity_update_interval {
        scaling.update_capacity();
        scaling.last_capacity_update = elapsed;
    }
}

use std::collections::HashMap;
use super::Player;
