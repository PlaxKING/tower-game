//! Seed + Delta Replication System
//!
//! Instead of replicating full world state, only the seed and player-caused
//! mutations (deltas) are sent. Any client can reconstruct the world:
//! 1. Generate floor from seed (deterministic)
//! 2. Apply delta log in order → exact same state
//!
//! Delta types: monster kills, chest opens, shrine activations, loot pickups,
//! environmental changes, player placements.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

use crate::generation::TowerSeed;

pub struct ReplicationPlugin;

impl Plugin for ReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DeltaLog::default())
            .add_event::<DeltaEvent>()
            .add_systems(Update, process_delta_events);
    }
}

/// Types of mutations that modify the procedurally generated world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeltaType {
    MonsterKill,
    ChestOpen,
    ShrineActivate,
    LootPickup,
    TrapDisarm,
    DoorUnlock,
    EnvironmentChange,
    PlayerSpawn,
    PlayerDeath,
    StairsUnlock,
    CraftComplete,
    QuestProgress,
}

/// A single mutation to the world state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    /// Monotonic sequence number
    pub seq: u64,
    /// Server tick when delta occurred
    pub tick: u64,
    /// Type of mutation
    pub delta_type: DeltaType,
    /// Floor where the mutation happened
    pub floor_id: u32,
    /// Entity hash (identifies which monster/chest/etc was affected)
    pub entity_hash: u64,
    /// Player who caused the mutation
    pub player_id: String,
    /// Optional payload (JSON for flexible data)
    pub payload: String,
    /// Hash of this delta (for integrity verification)
    pub hash: u64,
}

impl Delta {
    pub fn new(
        seq: u64,
        tick: u64,
        delta_type: DeltaType,
        floor_id: u32,
        entity_hash: u64,
        player_id: &str,
        payload: &str,
    ) -> Self {
        let mut delta = Self {
            seq,
            tick,
            delta_type,
            floor_id,
            entity_hash,
            player_id: player_id.to_string(),
            payload: payload.to_string(),
            hash: 0,
        };
        delta.hash = delta.compute_hash();
        delta
    }

    /// Deterministic hash of delta contents for integrity verification
    fn compute_hash(&self) -> u64 {
        let mut hasher = Sha3_256::new();
        hasher.update(self.seq.to_le_bytes());
        hasher.update(self.tick.to_le_bytes());
        hasher.update((self.delta_type as u32).to_le_bytes());
        hasher.update(self.floor_id.to_le_bytes());
        hasher.update(self.entity_hash.to_le_bytes());
        hasher.update(self.player_id.as_bytes());
        hasher.update(self.payload.as_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[0..8].try_into().unwrap())
    }

    /// Verify integrity of this delta
    pub fn verify(&self) -> bool {
        self.hash == self.compute_hash()
    }
}

/// Ordered log of all deltas for a floor
#[derive(Resource, Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeltaLog {
    pub deltas: Vec<Delta>,
    pub next_seq: u64,
}

impl DeltaLog {
    /// Add a new delta to the log
    pub fn push(&mut self, delta: Delta) -> u64 {
        let seq = delta.seq;
        self.deltas.push(delta);
        self.next_seq = seq + 1;
        seq
    }

    /// Create and add a delta
    pub fn record(
        &mut self,
        tick: u64,
        delta_type: DeltaType,
        floor_id: u32,
        entity_hash: u64,
        player_id: &str,
        payload: &str,
    ) -> u64 {
        let seq = self.next_seq;
        let delta = Delta::new(
            seq,
            tick,
            delta_type,
            floor_id,
            entity_hash,
            player_id,
            payload,
        );
        self.push(delta)
    }

    /// Get deltas since a sequence number (for incremental sync)
    pub fn since(&self, from_seq: u64) -> &[Delta] {
        if let Some(start) = self.deltas.iter().position(|d| d.seq >= from_seq) {
            &self.deltas[start..]
        } else {
            &[]
        }
    }

    /// Get deltas for a specific floor
    pub fn for_floor(&self, floor_id: u32) -> Vec<&Delta> {
        self.deltas
            .iter()
            .filter(|d| d.floor_id == floor_id)
            .collect()
    }

    /// Verify the entire log integrity
    pub fn verify_all(&self) -> bool {
        for (i, delta) in self.deltas.iter().enumerate() {
            if delta.seq != i as u64 {
                return false;
            }
            if !delta.verify() {
                return false;
            }
        }
        true
    }

    /// Total byte size estimate for network transfer
    pub fn estimated_size_bytes(&self) -> usize {
        self.deltas
            .iter()
            .map(|d| 8 + 8 + 4 + 4 + 8 + d.player_id.len() + d.payload.len() + 8)
            .sum()
    }

    /// Clear deltas for a floor (when everyone leaves)
    pub fn clear_floor(&mut self, floor_id: u32) {
        self.deltas.retain(|d| d.floor_id != floor_id);
    }

    /// Compact: keep only last N deltas per floor
    pub fn compact(&mut self, max_per_floor: usize) {
        use std::collections::HashMap;
        let mut counts: HashMap<u32, usize> = HashMap::new();

        // Count per floor from the end
        let mut keep = vec![false; self.deltas.len()];
        for i in (0..self.deltas.len()).rev() {
            let floor = self.deltas[i].floor_id;
            let count = counts.entry(floor).or_insert(0);
            if *count < max_per_floor {
                keep[i] = true;
                *count += 1;
            }
        }

        let mut idx = 0;
        self.deltas.retain(|_| {
            let k = keep[idx];
            idx += 1;
            k
        });
    }
}

/// Snapshot: seed + deltas = full state reconstruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloorSnapshot {
    pub seed: u64,
    pub floor_id: u32,
    pub deltas: Vec<Delta>,
    pub snapshot_tick: u64,
}

impl FloorSnapshot {
    /// Create snapshot from current state
    pub fn capture(seed: &TowerSeed, floor_id: u32, log: &DeltaLog, current_tick: u64) -> Self {
        let floor_deltas: Vec<Delta> = log.for_floor(floor_id).into_iter().cloned().collect();

        Self {
            seed: seed.seed,
            floor_id,
            deltas: floor_deltas,
            snapshot_tick: current_tick,
        }
    }

    /// Check if an entity has been mutated (killed, opened, etc.)
    pub fn is_entity_mutated(&self, entity_hash: u64) -> bool {
        self.deltas.iter().any(|d| d.entity_hash == entity_hash)
    }

    /// Get all deltas of a specific type
    pub fn deltas_of_type(&self, delta_type: DeltaType) -> Vec<&Delta> {
        self.deltas
            .iter()
            .filter(|d| d.delta_type == delta_type)
            .collect()
    }

    /// Count of killed monsters on this floor
    pub fn monsters_killed(&self) -> usize {
        self.deltas
            .iter()
            .filter(|d| d.delta_type == DeltaType::MonsterKill)
            .count()
    }

    /// Count of opened chests on this floor
    pub fn chests_opened(&self) -> usize {
        self.deltas
            .iter()
            .filter(|d| d.delta_type == DeltaType::ChestOpen)
            .count()
    }

    /// Serialize to JSON for network transfer
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Estimated network size in bytes
    pub fn estimated_size(&self) -> usize {
        // seed(8) + floor_id(4) + tick(8) + deltas
        20 + self.deltas.len() * 60 // ~60 bytes per delta average
    }
}

/// Event fired when a new delta is recorded
#[derive(Event, Debug, Clone)]
pub struct DeltaEvent {
    pub delta_type: DeltaType,
    pub floor_id: u32,
    pub entity_hash: u64,
    pub player_id: String,
    pub payload: String,
}

fn process_delta_events(
    mut events: EventReader<DeltaEvent>,
    mut log: ResMut<DeltaLog>,
    time: Res<Time>,
) {
    let tick = (time.elapsed_secs() * 10.0) as u64; // 10 ticks per second
    for event in events.read() {
        log.record(
            tick,
            event.delta_type,
            event.floor_id,
            event.entity_hash,
            &event.player_id,
            &event.payload,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_creation_and_verify() {
        let delta = Delta::new(
            0,
            100,
            DeltaType::MonsterKill,
            5,
            12345,
            "player1",
            r#"{"xp":50}"#,
        );
        assert_eq!(delta.seq, 0);
        assert!(delta.verify(), "Delta hash should verify correctly");
    }

    #[test]
    fn test_delta_tamper_detection() {
        let mut delta = Delta::new(
            0,
            100,
            DeltaType::MonsterKill,
            5,
            12345,
            "player1",
            r#"{"xp":50}"#,
        );
        delta.payload = r#"{"xp":99999}"#.to_string(); // Tamper!
        assert!(!delta.verify(), "Tampered delta should fail verification");
    }

    #[test]
    fn test_delta_log_record() {
        let mut log = DeltaLog::default();
        let seq = log.record(100, DeltaType::MonsterKill, 1, 555, "p1", "");
        assert_eq!(seq, 0);
        let seq = log.record(101, DeltaType::ChestOpen, 1, 666, "p1", "");
        assert_eq!(seq, 1);
        assert_eq!(log.deltas.len(), 2);
    }

    #[test]
    fn test_delta_log_since() {
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 1, "p1", "");
        log.record(101, DeltaType::ChestOpen, 1, 2, "p1", "");
        log.record(102, DeltaType::ShrineActivate, 1, 3, "p1", "");

        let since_1 = log.since(1);
        assert_eq!(since_1.len(), 2);
        assert_eq!(since_1[0].delta_type, DeltaType::ChestOpen);
    }

    #[test]
    fn test_delta_log_for_floor() {
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 1, "p1", "");
        log.record(101, DeltaType::ChestOpen, 2, 2, "p1", "");
        log.record(102, DeltaType::ShrineActivate, 1, 3, "p1", "");

        let floor1 = log.for_floor(1);
        assert_eq!(floor1.len(), 2);
        let floor2 = log.for_floor(2);
        assert_eq!(floor2.len(), 1);
    }

    #[test]
    fn test_delta_log_verify() {
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 1, "p1", "");
        log.record(101, DeltaType::ChestOpen, 1, 2, "p1", "");
        assert!(log.verify_all());
    }

    #[test]
    fn test_floor_snapshot() {
        let seed = TowerSeed { seed: 42 };
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 5, 111, "p1", "");
        log.record(101, DeltaType::MonsterKill, 5, 222, "p2", "");
        log.record(102, DeltaType::ChestOpen, 5, 333, "p1", "");
        log.record(103, DeltaType::MonsterKill, 6, 444, "p1", ""); // different floor

        let snapshot = FloorSnapshot::capture(&seed, 5, &log, 103);
        assert_eq!(snapshot.seed, 42);
        assert_eq!(snapshot.floor_id, 5);
        assert_eq!(snapshot.deltas.len(), 3);
        assert_eq!(snapshot.monsters_killed(), 2);
        assert_eq!(snapshot.chests_opened(), 1);
    }

    #[test]
    fn test_snapshot_entity_mutated() {
        let seed = TowerSeed { seed: 42 };
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 999, "p1", "");

        let snapshot = FloorSnapshot::capture(&seed, 1, &log, 100);
        assert!(snapshot.is_entity_mutated(999));
        assert!(!snapshot.is_entity_mutated(888));
    }

    #[test]
    fn test_snapshot_serialization() {
        let seed = TowerSeed { seed: 42 };
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 111, "p1", r#"{"xp":50}"#);

        let snapshot = FloorSnapshot::capture(&seed, 1, &log, 100);
        let json = snapshot.to_json();
        assert!(!json.is_empty());

        let restored = FloorSnapshot::from_json(&json);
        assert!(restored.is_some());
        let restored = restored.unwrap();
        assert_eq!(restored.seed, 42);
        assert_eq!(restored.deltas.len(), 1);
    }

    #[test]
    fn test_delta_log_compact() {
        let mut log = DeltaLog::default();
        for i in 0..20 {
            log.record(i, DeltaType::MonsterKill, 1, i, "p1", "");
        }
        assert_eq!(log.deltas.len(), 20);

        log.compact(10);
        assert_eq!(log.deltas.len(), 10);
    }

    #[test]
    fn test_delta_log_clear_floor() {
        let mut log = DeltaLog::default();
        log.record(100, DeltaType::MonsterKill, 1, 1, "p1", "");
        log.record(101, DeltaType::MonsterKill, 2, 2, "p1", "");
        log.record(102, DeltaType::MonsterKill, 1, 3, "p1", "");

        log.clear_floor(1);
        assert_eq!(log.deltas.len(), 1);
        assert_eq!(log.deltas[0].floor_id, 2);
    }

    #[test]
    fn test_delta_types_coverage() {
        let types = vec![
            DeltaType::MonsterKill,
            DeltaType::ChestOpen,
            DeltaType::ShrineActivate,
            DeltaType::LootPickup,
            DeltaType::TrapDisarm,
            DeltaType::DoorUnlock,
            DeltaType::EnvironmentChange,
            DeltaType::PlayerSpawn,
            DeltaType::PlayerDeath,
            DeltaType::StairsUnlock,
            DeltaType::CraftComplete,
            DeltaType::QuestProgress,
        ];
        assert_eq!(types.len(), 12, "Should have 12 delta types");

        for (i, dt) in types.iter().enumerate() {
            let delta = Delta::new(i as u64, 0, *dt, 1, 0, "p1", "");
            assert!(delta.verify());
        }
    }

    #[test]
    fn test_estimated_sizes() {
        let mut log = DeltaLog::default();
        log.record(0, DeltaType::MonsterKill, 1, 1, "player1", r#"{"xp":50}"#);
        assert!(log.estimated_size_bytes() > 0);

        let seed = TowerSeed { seed: 42 };
        let snapshot = FloorSnapshot::capture(&seed, 1, &log, 0);
        assert!(snapshot.estimated_size() > 20);
    }
}
