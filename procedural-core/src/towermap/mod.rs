//! Tower Map System (FEAT-005)
//!
//! Tracks player's exploration progress across all floors of the tower.
//! Provides global statistics, per-floor details, and visualization data for UI.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::generation::FloorTier;

pub struct TowerMapPlugin;

impl Plugin for TowerMapPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TowerMap::default())
            .add_event::<MapEvent>()
            .add_systems(Update, process_map_events);
    }
}

/// Single floor entry in the tower map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorMapEntry {
    pub floor_id: u32,
    pub tier: FloorTier,
    pub discovered: bool,
    pub cleared: bool,
    pub visited_count: u32,
    pub death_count: u32,
    pub best_clear_time_secs: Option<f32>,
    pub completion_percent: f32, // 0.0-1.0
    pub discovered_rooms: u32,
    pub total_rooms: u32,
    pub discovered_secrets: u32,
    pub total_secrets: u32,
    pub monsters_killed: u32,
    pub total_monsters: u32,
    pub chests_opened: u32,
    pub total_chests: u32,
    pub shrine_faction: Option<String>,
    pub first_discovered_utc: u64,
    pub last_visited_utc: u64,
    pub notes: String, // Player notes for this floor
}

impl FloorMapEntry {
    pub fn new(floor_id: u32, tier: FloorTier) -> Self {
        Self {
            floor_id,
            tier,
            discovered: false,
            cleared: false,
            visited_count: 0,
            death_count: 0,
            best_clear_time_secs: None,
            completion_percent: 0.0,
            discovered_rooms: 0,
            total_rooms: 0,
            discovered_secrets: 0,
            total_secrets: 0,
            monsters_killed: 0,
            total_monsters: 0,
            chests_opened: 0,
            total_chests: 0,
            shrine_faction: None,
            first_discovered_utc: 0,
            last_visited_utc: 0,
            notes: String::new(),
        }
    }

    /// Discover this floor for the first time
    pub fn discover(&mut self, total_rooms: u32, total_monsters: u32, total_chests: u32) {
        if !self.discovered {
            self.discovered = true;
            self.first_discovered_utc = current_time_utc();
        }
        self.last_visited_utc = current_time_utc();
        self.visited_count += 1;
        self.total_rooms = total_rooms;
        self.total_monsters = total_monsters;
        self.total_chests = total_chests;
        self.recalculate_completion();
    }

    /// Mark floor as cleared
    pub fn mark_cleared(&mut self, clear_time_secs: f32) {
        if !self.cleared {
            self.cleared = true;
        }

        if let Some(best) = self.best_clear_time_secs {
            if clear_time_secs < best {
                self.best_clear_time_secs = Some(clear_time_secs);
            }
        } else {
            self.best_clear_time_secs = Some(clear_time_secs);
        }

        self.completion_percent = 1.0;
    }

    /// Record a death on this floor
    pub fn record_death(&mut self) {
        self.death_count += 1;
    }

    /// Discover a room
    pub fn discover_room(&mut self) {
        if self.discovered_rooms < self.total_rooms {
            self.discovered_rooms += 1;
            self.recalculate_completion();
        }
    }

    /// Discover a secret
    pub fn discover_secret(&mut self, total_secrets: u32) {
        self.total_secrets = total_secrets;
        if self.discovered_secrets < total_secrets {
            self.discovered_secrets += 1;
            self.recalculate_completion();
        }
    }

    /// Kill a monster
    pub fn kill_monster(&mut self) {
        if self.monsters_killed < self.total_monsters {
            self.monsters_killed += 1;
            self.recalculate_completion();
        }
    }

    /// Open a chest
    pub fn open_chest(&mut self) {
        if self.chests_opened < self.total_chests {
            self.chests_opened += 1;
            self.recalculate_completion();
        }
    }

    /// Activate shrine (record faction)
    pub fn activate_shrine(&mut self, faction: &str) {
        self.shrine_faction = Some(faction.to_string());
    }

    fn recalculate_completion(&mut self) {
        let mut score = 0.0;
        let mut max = 0.0;

        // Rooms: 30%
        if self.total_rooms > 0 {
            score += (self.discovered_rooms as f32 / self.total_rooms as f32) * 0.3;
        }
        max += 0.3;

        // Monsters: 40%
        if self.total_monsters > 0 {
            score += (self.monsters_killed as f32 / self.total_monsters as f32) * 0.4;
        }
        max += 0.4;

        // Chests: 20%
        if self.total_chests > 0 {
            score += (self.chests_opened as f32 / self.total_chests as f32) * 0.2;
        }
        max += 0.2;

        // Secrets: 10%
        if self.total_secrets > 0 {
            score += (self.discovered_secrets as f32 / self.total_secrets as f32) * 0.1;
        }
        max += 0.1;

        self.completion_percent = if max > 0.0 { score / max } else { 0.0 };
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Complete tower exploration map (player's persistent data)
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct TowerMap {
    pub floors: HashMap<u32, FloorMapEntry>,
    pub highest_floor_reached: u32,
    pub total_floors_discovered: u32,
    pub total_floors_cleared: u32,
    pub total_deaths: u32,
    pub total_playtime_secs: f32,
    pub first_session_utc: u64,
    pub last_session_utc: u64,
}

impl Default for TowerMap {
    fn default() -> Self {
        Self {
            floors: HashMap::new(),
            highest_floor_reached: 0,
            total_floors_discovered: 0,
            total_floors_cleared: 0,
            total_deaths: 0,
            total_playtime_secs: 0.0,
            first_session_utc: 0,
            last_session_utc: 0,
        }
    }
}

impl TowerMap {
    pub fn get_floor(&self, floor_id: u32) -> Option<&FloorMapEntry> {
        self.floors.get(&floor_id)
    }

    pub fn get_floor_mut(&mut self, floor_id: u32) -> Option<&mut FloorMapEntry> {
        self.floors.get_mut(&floor_id)
    }

    pub fn discover_floor(
        &mut self,
        floor_id: u32,
        tier: FloorTier,
        total_rooms: u32,
        total_monsters: u32,
        total_chests: u32,
    ) {
        let entry = self
            .floors
            .entry(floor_id)
            .or_insert_with(|| FloorMapEntry::new(floor_id, tier));

        let was_discovered = entry.discovered;
        entry.discover(total_rooms, total_monsters, total_chests);

        if !was_discovered {
            self.total_floors_discovered += 1;
        }

        if floor_id > self.highest_floor_reached {
            self.highest_floor_reached = floor_id;
        }

        self.update_session_time();
    }

    pub fn clear_floor(&mut self, floor_id: u32, clear_time_secs: f32) {
        if let Some(entry) = self.floors.get_mut(&floor_id) {
            let was_cleared = entry.cleared;
            entry.mark_cleared(clear_time_secs);

            if !was_cleared {
                self.total_floors_cleared += 1;
            }
        }
    }

    pub fn record_death(&mut self, floor_id: u32) {
        if let Some(entry) = self.floors.get_mut(&floor_id) {
            entry.record_death();
        }
        self.total_deaths += 1;
    }

    pub fn get_cleared_floors(&self) -> Vec<&FloorMapEntry> {
        self.floors.values().filter(|e| e.cleared).collect()
    }

    pub fn get_discovered_floors(&self) -> Vec<&FloorMapEntry> {
        self.floors.values().filter(|e| e.discovered).collect()
    }

    pub fn get_floors_by_tier(&self, tier: FloorTier) -> Vec<&FloorMapEntry> {
        self.floors.values().filter(|e| e.tier == tier).collect()
    }

    pub fn average_completion(&self) -> f32 {
        if self.floors.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.floors.values().map(|e| e.completion_percent).sum();
        sum / self.floors.len() as f32
    }

    fn update_session_time(&mut self) {
        let now = current_time_utc();
        if self.first_session_utc == 0 {
            self.first_session_utc = now;
        }
        self.last_session_utc = now;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }
}

/// Overview statistics for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TowerMapOverview {
    pub highest_floor: u32,
    pub total_discovered: u32,
    pub total_cleared: u32,
    pub total_deaths: u32,
    pub average_completion: f32,
    pub floors_per_tier: HashMap<String, u32>,
    pub cleared_per_tier: HashMap<String, u32>,
    pub total_playtime_hours: f32,
    pub first_session_date: String,
    pub last_session_date: String,
}

impl TowerMapOverview {
    pub fn from_map(map: &TowerMap) -> Self {
        let mut floors_per_tier: HashMap<String, u32> = HashMap::new();
        let mut cleared_per_tier: HashMap<String, u32> = HashMap::new();

        for entry in map.floors.values() {
            let tier_name = format!("{:?}", entry.tier);
            *floors_per_tier.entry(tier_name.clone()).or_insert(0) += 1;
            if entry.cleared {
                *cleared_per_tier.entry(tier_name).or_insert(0) += 1;
            }
        }

        Self {
            highest_floor: map.highest_floor_reached,
            total_discovered: map.total_floors_discovered,
            total_cleared: map.total_floors_cleared,
            total_deaths: map.total_deaths,
            average_completion: map.average_completion(),
            floors_per_tier,
            cleared_per_tier,
            total_playtime_hours: map.total_playtime_secs / 3600.0,
            first_session_date: format_utc(map.first_session_utc),
            last_session_date: format_utc(map.last_session_utc),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Map events for real-time updates
#[derive(Event, Debug, Clone)]
pub enum MapEvent {
    FloorDiscovered {
        floor_id: u32,
        tier: FloorTier,
        total_rooms: u32,
        total_monsters: u32,
        total_chests: u32,
    },
    FloorCleared {
        floor_id: u32,
        clear_time_secs: f32,
    },
    Death {
        floor_id: u32,
    },
    RoomDiscovered {
        floor_id: u32,
    },
    SecretDiscovered {
        floor_id: u32,
        total_secrets: u32,
    },
    MonsterKilled {
        floor_id: u32,
    },
    ChestOpened {
        floor_id: u32,
    },
    ShrineActivated {
        floor_id: u32,
        faction: String,
    },
}

fn process_map_events(mut events: EventReader<MapEvent>, mut map: ResMut<TowerMap>) {
    for event in events.read() {
        match event {
            MapEvent::FloorDiscovered {
                floor_id,
                tier,
                total_rooms,
                total_monsters,
                total_chests,
            } => {
                map.discover_floor(
                    *floor_id,
                    *tier,
                    *total_rooms,
                    *total_monsters,
                    *total_chests,
                );
            }
            MapEvent::FloorCleared {
                floor_id,
                clear_time_secs,
            } => {
                map.clear_floor(*floor_id, *clear_time_secs);
            }
            MapEvent::Death { floor_id } => {
                map.record_death(*floor_id);
            }
            MapEvent::RoomDiscovered { floor_id } => {
                if let Some(entry) = map.get_floor_mut(*floor_id) {
                    entry.discover_room();
                }
            }
            MapEvent::SecretDiscovered {
                floor_id,
                total_secrets,
            } => {
                if let Some(entry) = map.get_floor_mut(*floor_id) {
                    entry.discover_secret(*total_secrets);
                }
            }
            MapEvent::MonsterKilled { floor_id } => {
                if let Some(entry) = map.get_floor_mut(*floor_id) {
                    entry.kill_monster();
                }
            }
            MapEvent::ChestOpened { floor_id } => {
                if let Some(entry) = map.get_floor_mut(*floor_id) {
                    entry.open_chest();
                }
            }
            MapEvent::ShrineActivated { floor_id, faction } => {
                if let Some(entry) = map.get_floor_mut(*floor_id) {
                    entry.activate_shrine(faction);
                }
            }
        }
    }
}

fn current_time_utc() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_utc(timestamp: u64) -> String {
    if timestamp == 0 {
        return "Never".to_string();
    }
    // Simple format: YYYY-MM-DD (would need chrono for full formatting)
    format!("Timestamp: {}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_entry_creation() {
        let entry = FloorMapEntry::new(10, FloorTier::Echelon2);
        assert_eq!(entry.floor_id, 10);
        assert_eq!(entry.tier, FloorTier::Echelon2);
        assert!(!entry.discovered);
        assert!(!entry.cleared);
        assert_eq!(entry.visited_count, 0);
    }

    #[test]
    fn test_floor_entry_discover() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.discover(5, 10, 3);

        assert!(entry.discovered);
        assert_eq!(entry.visited_count, 1);
        assert_eq!(entry.total_rooms, 5);
        assert_eq!(entry.total_monsters, 10);
        assert_eq!(entry.total_chests, 3);
        assert!(entry.first_discovered_utc > 0);
    }

    #[test]
    fn test_floor_entry_clear() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.discover(5, 10, 3);
        entry.mark_cleared(120.5);

        assert!(entry.cleared);
        assert_eq!(entry.best_clear_time_secs, Some(120.5));
        assert_eq!(entry.completion_percent, 1.0);

        entry.mark_cleared(100.0);
        assert_eq!(entry.best_clear_time_secs, Some(100.0)); // improved time
    }

    #[test]
    fn test_floor_entry_death() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.record_death();
        entry.record_death();
        assert_eq!(entry.death_count, 2);
    }

    #[test]
    fn test_floor_entry_progression() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.discover(4, 10, 2);

        entry.discover_room();
        entry.discover_room();
        assert_eq!(entry.discovered_rooms, 2);

        entry.kill_monster();
        entry.kill_monster();
        entry.kill_monster();
        assert_eq!(entry.monsters_killed, 3);

        entry.open_chest();
        assert_eq!(entry.chests_opened, 1);

        entry.discover_secret(1);
        assert_eq!(entry.discovered_secrets, 1);

        assert!(entry.completion_percent > 0.0);
        assert!(entry.completion_percent < 1.0);
    }

    #[test]
    fn test_floor_entry_completion_calculation() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.discover(4, 4, 2);

        // 100% rooms (30%) + 100% monsters (40%) + 100% chests (20%) + 0% secrets (10%)
        entry.discovered_rooms = 4;
        entry.monsters_killed = 4;
        entry.chests_opened = 2;
        entry.recalculate_completion();

        assert!((entry.completion_percent - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_tower_map_creation() {
        let map = TowerMap::default();
        assert_eq!(map.floors.len(), 0);
        assert_eq!(map.highest_floor_reached, 0);
        assert_eq!(map.total_floors_discovered, 0);
    }

    #[test]
    fn test_tower_map_discover_floor() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);

        assert_eq!(map.total_floors_discovered, 1);
        assert_eq!(map.highest_floor_reached, 1);

        let entry = map.get_floor(1).unwrap();
        assert!(entry.discovered);
        assert_eq!(entry.total_rooms, 5);
    }

    #[test]
    fn test_tower_map_highest_floor() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(5, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(3, FloorTier::Echelon1, 5, 10, 3);

        assert_eq!(map.highest_floor_reached, 5);
        assert_eq!(map.total_floors_discovered, 3);
    }

    #[test]
    fn test_tower_map_clear_floor() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.clear_floor(1, 150.0);

        assert_eq!(map.total_floors_cleared, 1);

        let entry = map.get_floor(1).unwrap();
        assert!(entry.cleared);
        assert_eq!(entry.best_clear_time_secs, Some(150.0));
    }

    #[test]
    fn test_tower_map_death() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.record_death(1);
        map.record_death(1);

        assert_eq!(map.total_deaths, 2);

        let entry = map.get_floor(1).unwrap();
        assert_eq!(entry.death_count, 2);
    }

    #[test]
    fn test_tower_map_queries() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(2, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(150, FloorTier::Echelon2, 6, 15, 4);

        map.clear_floor(1, 100.0);

        let cleared = map.get_cleared_floors();
        assert_eq!(cleared.len(), 1);

        let discovered = map.get_discovered_floors();
        assert_eq!(discovered.len(), 3);

        let tier1 = map.get_floors_by_tier(FloorTier::Echelon1);
        assert_eq!(tier1.len(), 2);

        let tier2 = map.get_floors_by_tier(FloorTier::Echelon2);
        assert_eq!(tier2.len(), 1);
    }

    #[test]
    fn test_tower_map_average_completion() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 4, 10, 2);
        map.discover_floor(2, FloorTier::Echelon1, 4, 10, 2);

        if let Some(entry) = map.get_floor_mut(1) {
            entry.completion_percent = 0.5;
        }
        if let Some(entry) = map.get_floor_mut(2) {
            entry.completion_percent = 1.0;
        }

        let avg = map.average_completion();
        assert!((avg - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_tower_map_json_roundtrip() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.clear_floor(1, 120.0);

        let json = map.to_json();
        assert!(!json.is_empty());

        let restored = TowerMap::from_json(&json).unwrap();
        assert_eq!(restored.highest_floor_reached, 1);
        assert_eq!(restored.total_floors_discovered, 1);
        assert_eq!(restored.total_floors_cleared, 1);

        let entry = restored.get_floor(1).unwrap();
        assert!(entry.cleared);
    }

    #[test]
    fn test_tower_map_overview() {
        let mut map = TowerMap::default();
        map.discover_floor(1, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(2, FloorTier::Echelon1, 5, 10, 3);
        map.discover_floor(150, FloorTier::Echelon2, 6, 15, 4);

        map.clear_floor(1, 100.0);
        map.clear_floor(150, 200.0);

        map.record_death(2);

        let overview = TowerMapOverview::from_map(&map);

        assert_eq!(overview.highest_floor, 150);
        assert_eq!(overview.total_discovered, 3);
        assert_eq!(overview.total_cleared, 2);
        assert_eq!(overview.total_deaths, 1);

        let json = overview.to_json();
        assert!(json.contains("highest_floor"));
        assert!(json.contains("total_discovered"));
    }

    #[test]
    fn test_floor_entry_json() {
        let mut entry = FloorMapEntry::new(10, FloorTier::Echelon2);
        entry.discover(5, 10, 3);

        let json = entry.to_json();
        assert!(json.contains("floor_id"));
        assert!(json.contains("Echelon2"));
    }

    #[test]
    fn test_shrine_activation() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.activate_shrine("Seekers");

        assert_eq!(entry.shrine_faction, Some("Seekers".to_string()));
    }

    #[test]
    fn test_floor_visited_multiple_times() {
        let mut entry = FloorMapEntry::new(1, FloorTier::Echelon1);
        entry.discover(5, 10, 3);
        entry.discover(5, 10, 3);
        entry.discover(5, 10, 3);

        assert_eq!(entry.visited_count, 3);
    }
}
