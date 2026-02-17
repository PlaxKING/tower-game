//! Tower Bevy Server Library
//!
//! This library provides the core modules for the Tower game server:
//! - Async floor generation with worker pool
//! - Protobuf types for network communication
//! - LMDB embedded database caching (persistent Tier 2 cache)
//! - FFI C API for UE5 integration (Protobuf ↔ JSON bridge)
//! - Semantic tag system for procedural content interconnection
//! - Dynamic scaling and hybrid generation systems

pub mod api; // HTTP/JSON API endpoints for UE5 client
#[allow(dead_code)]
pub mod async_generation;
pub mod combat; // Skill-based non-target action combat system
pub mod components; // Shared ECS components (Player, Monster, FloorTile)
pub mod destruction; // Battlefield-style environmental destruction
pub mod ecs_bridge; // API ↔ Bevy ECS communication bridge
pub mod ffi; // C FFI for UE5
pub mod input; // Player input types, validation, anti-cheat
#[allow(dead_code)]
pub mod lmdb_cache;
pub mod loot; // Semantic loot generation with rarity and equipment effects
pub mod metrics; // Server metrics (Prometheus + JSON export)
pub mod monster_gen; // Grammar-based monster generation with FSM AI
pub mod physics; // Physics integration (bevy_rapier3d collision, knockback)
pub mod proto;
#[allow(dead_code)]
pub mod semantic_tags; // Semantic tag system
pub mod storage; // Unified data storage (LMDB + PostgreSQL)
pub mod wfc; // Wave Function Collapse floor generation

// Re-export commonly used types
pub use async_generation::{CacheStats, FloorGenerator, GenerationConfig};
pub use lmdb_cache::{LmdbCacheStats, LmdbError, LmdbFloorCache};
pub use semantic_tags::{DomainCategory, MasteryDomain, SemanticTags};
pub use storage::lmdb_templates::LmdbTemplateStore;
pub use storage::postgres::PostgresStore;
