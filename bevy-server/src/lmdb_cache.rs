//! LMDB Floor Cache Integration
//!
//! Persistent embedded caching layer for procedurally generated floors.
//! Sits between LRU cache (RAM) and generation (CPU).
//!
//! ## Architecture
//! ```text
//! [Request] → [LRU Cache] → [LMDB Cache] → [Generate]
//!               (instant)     (~10-50µs)      (~580µs)
//! ```
//!
//! ## Why LMDB over Redis?
//! - **Faster**: ~10-50µs vs Redis ~1-2ms (20-40x faster!)
//! - **Embedded**: No separate server process
//! - **Zero-copy**: Memory-mapped file access
//! - **ACID**: Transactions, crash-safe
//!
//! ## Usage
//! ```rust,ignore
//! use tower_bevy_server::lmdb_cache::LmdbFloorCache;
//!
//! let cache = LmdbFloorCache::new("./data/floor_cache", 1_000_000)?;
//! cache.set(1, &chunk_data)?;
//! let chunk = cache.get(1)?;
//! ```

use crate::proto::tower::game::ChunkData;
use heed::{Database, Env, EnvOpenOptions};
use prost::Message;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// LMDB cache for persistent floor storage (embedded database)
#[derive(Clone)]
pub struct LmdbFloorCache {
    /// LMDB environment (holds the database)
    env: Arc<Env>,
    /// Database handle for floor storage (key: u32 floor_id, value: Vec<u8> protobuf)
    db: Database<heed::types::U32<heed::byteorder::NativeEndian>, heed::types::Bytes>,
}

impl LmdbFloorCache {
    /// Create new LMDB cache
    ///
    /// # Arguments
    /// * `path` - Directory path for LMDB database files
    /// * `max_size_bytes` - Maximum database size in bytes (e.g., 1GB = 1_000_000_000)
    ///
    /// # Example
    /// ```rust,ignore
    /// let cache = LmdbFloorCache::new("./data/floor_cache", 1_000_000_000)?;
    /// ```
    pub fn new<P: AsRef<Path>>(path: P, max_size_bytes: usize) -> Result<Self, LmdbError> {
        info!(
            "Opening LMDB cache at {:?} (max size: {} bytes)",
            path.as_ref(),
            max_size_bytes
        );

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&path)?;

        // Open LMDB environment
        let env = unsafe {
            EnvOpenOptions::new()
                .map_size(max_size_bytes)
                .max_dbs(1)
                .open(path)?
        };

        // Create/open database (requires write transaction)
        let mut wtxn = env.write_txn()?;
        let db = env
            .create_database::<heed::types::U32<heed::byteorder::NativeEndian>, heed::types::Bytes>(
                &mut wtxn,
                Some("floors"),
            )?;
        wtxn.commit()?;

        info!("LMDB cache opened successfully");

        Ok(Self {
            env: Arc::new(env),
            db,
        })
    }

    /// Get floor from LMDB cache
    ///
    /// Returns `None` if floor not in cache or deserialization fails.
    ///
    /// # Performance
    /// ~10-50µs (memory-mapped, zero-copy read)
    pub fn get(&self, floor_id: u32) -> Option<ChunkData> {
        let rtxn = match self.env.read_txn() {
            Ok(txn) => txn,
            Err(e) => {
                error!("Failed to create LMDB read transaction: {}", e);
                return None;
            }
        };

        match self.db.get(&rtxn, &floor_id) {
            Ok(Some(bytes)) => match ChunkData::decode(bytes) {
                Ok(chunk) => {
                    debug!("LMDB HIT for floor {}", floor_id);
                    Some(chunk)
                }
                Err(e) => {
                    warn!("Failed to decode floor {} from LMDB: {}", floor_id, e);
                    None
                }
            },
            Ok(None) => {
                debug!("LMDB MISS for floor {} (not found)", floor_id);
                None
            }
            Err(e) => {
                error!("LMDB GET error for floor {}: {}", floor_id, e);
                None
            }
        }
    }

    /// Store floor in LMDB cache
    ///
    /// # Performance
    /// ~20-100µs (memory-mapped write + fsync)
    pub fn set(&self, floor_id: u32, chunk: &ChunkData) -> Result<(), LmdbError> {
        // Serialize to Protobuf binary
        let mut bytes = Vec::new();
        if let Err(e) = chunk.encode(&mut bytes) {
            error!("Failed to encode floor {} for LMDB: {}", floor_id, e);
            return Err(LmdbError::EncodingError(e.to_string()));
        }

        // Write transaction
        let mut wtxn = self.env.write_txn()?;
        self.db.put(&mut wtxn, &floor_id, &bytes)?;
        wtxn.commit()?;

        debug!("LMDB SET floor {}", floor_id);
        Ok(())
    }

    /// Check if floor exists in LMDB
    pub fn exists(&self, floor_id: u32) -> bool {
        let rtxn = match self.env.read_txn() {
            Ok(txn) => txn,
            Err(_) => return false,
        };

        self.db
            .get(&rtxn, &floor_id)
            .map(|opt: Option<&[u8]>| opt.is_some())
            .unwrap_or(false)
    }

    /// Delete floor from LMDB
    pub fn delete(&self, floor_id: u32) -> Result<(), LmdbError> {
        let mut wtxn = self.env.write_txn()?;
        self.db.delete(&mut wtxn, &floor_id)?;
        wtxn.commit()?;
        debug!("LMDB DELETE floor {}", floor_id);
        Ok(())
    }

    /// Clear all cached floors
    pub fn clear_all(&self) -> Result<(), LmdbError> {
        let mut wtxn = self.env.write_txn()?;
        self.db.clear(&mut wtxn)?;
        wtxn.commit()?;
        info!("LMDB cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<LmdbCacheStats, LmdbError> {
        let rtxn = self.env.read_txn()?;
        let floor_count = self.db.len(&rtxn)? as usize;

        Ok(LmdbCacheStats {
            floor_count,
            page_size: 4096, // Default LMDB page size
            depth: 0,
            branch_pages: 0,
            leaf_pages: 0,
            overflow_pages: 0,
            entries: floor_count as u64,
        })
    }

    /// Sync database to disk (explicit fsync)
    pub fn sync(&self) -> Result<(), LmdbError> {
        self.env.force_sync()?;
        debug!("LMDB synced to disk");
        Ok(())
    }
}

/// LMDB cache statistics
#[derive(Debug, Clone)]
pub struct LmdbCacheStats {
    pub floor_count: usize,
    pub page_size: usize,
    pub depth: usize,
    pub branch_pages: u64,
    pub leaf_pages: u64,
    pub overflow_pages: u64,
    pub entries: u64,
}

/// LMDB error types
#[derive(Debug)]
pub enum LmdbError {
    HeedError(heed::Error),
    IoError(std::io::Error),
    EncodingError(String),
}

impl From<heed::Error> for LmdbError {
    fn from(e: heed::Error) -> Self {
        LmdbError::HeedError(e)
    }
}

impl From<std::io::Error> for LmdbError {
    fn from(e: std::io::Error) -> Self {
        LmdbError::IoError(e)
    }
}

impl std::fmt::Display for LmdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LmdbError::HeedError(e) => write!(f, "LMDB error: {}", e),
            LmdbError::IoError(e) => write!(f, "IO error: {}", e),
            LmdbError::EncodingError(e) => write!(f, "Encoding error: {}", e),
        }
    }
}

impl std::error::Error for LmdbError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::tower::game::{FloorTileData, Vec3};

    fn create_test_cache() -> LmdbFloorCache {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_test_{}", std::process::id()));
        LmdbFloorCache::new(temp_dir, 10 * 1024 * 1024) // 10MB, aligned to page size
            .expect("Failed to create LMDB cache")
    }

    fn create_test_chunk(floor_id: u32) -> ChunkData {
        ChunkData {
            seed: 0x12345678,
            floor_id,
            tiles: vec![
                FloorTileData {
                    tile_type: 1,
                    grid_x: 0,
                    grid_y: 0,
                    biome_id: 1,
                    is_walkable: true,
                    has_collision: false,
                },
                FloorTileData {
                    tile_type: 2,
                    grid_x: 1,
                    grid_y: 0,
                    biome_id: 1,
                    is_walkable: true,
                    has_collision: true,
                },
            ],
            validation_hash: vec![0xAB, 0xCD, 0xEF],
            biome_id: 1,
            width: 10,
            height: 10,
            world_offset: Some(Vec3 {
                x: 0.0,
                y: (floor_id as f32) * 5.0,
                z: 0.0,
            }),
            semantic_tags: None,
        }
    }

    #[test]
    fn test_lmdb_set_get() {
        let cache = create_test_cache();
        let floor_id = 999;
        let chunk = create_test_chunk(floor_id);

        // Store in LMDB
        cache.set(floor_id, &chunk).unwrap();

        // Retrieve from LMDB
        let retrieved = cache.get(floor_id).unwrap();

        assert_eq!(retrieved.floor_id, floor_id);
        assert_eq!(retrieved.seed, chunk.seed);
        assert_eq!(retrieved.tiles.len(), chunk.tiles.len());
    }

    #[test]
    fn test_lmdb_miss() {
        let cache = create_test_cache();
        let floor_id = 8888;

        // Should return None
        let result = cache.get(floor_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_lmdb_exists() {
        let cache = create_test_cache();
        let floor_id = 777;
        let chunk = create_test_chunk(floor_id);

        // Store in LMDB
        cache.set(floor_id, &chunk).unwrap();

        // Check existence
        assert!(cache.exists(floor_id));

        // Delete
        cache.delete(floor_id).unwrap();

        // Should not exist
        assert!(!cache.exists(floor_id));
    }

    #[test]
    fn test_lmdb_stats() {
        // Use a dedicated temp dir to avoid contamination from parallel tests
        let temp_dir = std::env::temp_dir().join(format!("lmdb_stats_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        let cache = LmdbFloorCache::new(&temp_dir, 10 * 1024 * 1024).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.floor_count, 0);

        // Add some floors
        for i in 1..=10 {
            cache.set(i, &create_test_chunk(i)).unwrap();
        }

        let stats = cache.stats().unwrap();
        assert_eq!(stats.floor_count, 10);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
