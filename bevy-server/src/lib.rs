//! Tower Bevy Server Library
//!
//! This library provides the core modules for the Tower game server:
//! - Async floor generation with worker pool
//! - Protobuf types for network communication
//! - LMDB embedded database caching (persistent Tier 2 cache)
//! - FFI C API for UE5 integration (Protobuf ↔ JSON bridge)
//! - Semantic tag system for procedural content interconnection
//! - Dynamic scaling and hybrid generation systems

pub mod proto;
#[allow(dead_code)]
pub mod async_generation;
#[allow(dead_code)]
pub mod lmdb_cache;
pub mod ffi;  // C FFI for UE5
#[allow(dead_code)]
pub mod semantic_tags;  // Semantic tag system
pub mod storage;  // Unified data storage (LMDB + PostgreSQL)
pub mod api;  // HTTP/JSON API endpoints for UE5 client
pub mod metrics;  // Server metrics (Prometheus + JSON export)
pub mod destruction;  // Battlefield-style environmental destruction
pub mod ecs_bridge;  // API ↔ Bevy ECS communication bridge
pub mod components;  // Shared ECS components (Player, Monster, FloorTile)
pub mod combat;  // Skill-based non-target action combat system
pub mod monster_gen;  // Grammar-based monster generation with FSM AI
pub mod loot;  // Semantic loot generation with rarity and equipment effects
pub mod physics;  // Physics integration (bevy_rapier3d collision, knockback)
pub mod input;  // Player input types, validation, anti-cheat
pub mod wfc;  // Wave Function Collapse floor generation

// Re-export commonly used types
pub use async_generation::{FloorGenerator, GenerationConfig, CacheStats};
pub use lmdb_cache::{LmdbFloorCache, LmdbCacheStats, LmdbError};
pub use semantic_tags::{SemanticTags, MasteryDomain, DomainCategory};
pub use storage::lmdb_templates::LmdbTemplateStore;
pub use storage::postgres::PostgresStore;
