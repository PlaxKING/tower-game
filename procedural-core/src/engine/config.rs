use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub server_host: String,
    pub server_port: u16,
    pub tick_rate: u32,
    pub update_radius: f32,
    pub max_players_per_floor: u32,
    pub transport: TransportMode,
    pub tower_seed: u64,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            server_host: "127.0.0.1".into(),
            server_port: 50051,
            tick_rate: 60,
            update_radius: 100.0,
            max_players_per_floor: 32,
            transport: TransportMode::Json,
            tower_seed: 42,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportMode {
    Json,
    Protobuf,
    Ffi,
}
