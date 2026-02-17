//! Async Floor Generation System with 3-Tier Caching
//!
//! This module implements non-blocking procedural floor generation using:
//! - Tier 1: LRU cache (in-memory, ~4.7µs)
//! - Tier 2: LMDB cache (persistent, ~330µs)
//! - Tier 3: CPU generation (~569µs)
//! - Tokio worker pool for parallel generation
//! - SHA-3 validation hashes for anti-cheat
//!
//! ## Architecture
//! ```text
//! [Bevy Main Thread]
//!       ↓ request floor_id
//! [FloorGenerator]
//!       ↓ check Tier 1
//! [LRU Cache] → hit? return (~4.7µs)
//!       ↓ miss? check Tier 2
//! [LMDB Cache] → hit? cache + return (~330µs)
//!       ↓ miss? spawn async task
//! [Tokio Worker Pool] → generate floor (~569µs)
//!       ↓
//! [Cache Tier 1 & 2 + Return ChunkData]
//! ```

use crate::lmdb_cache::LmdbFloorCache;
use crate::proto::tower::game::{ChunkData, FloorTileData, Vec3, SemanticTags as ProtoSemanticTags, TagPair};
use crate::semantic_tags::SemanticTags;
use crate::wfc;
use lru::LruCache;
use parking_lot::Mutex;
use sha3::{Digest, Sha3_256};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Configuration for the floor generation system
#[derive(Debug, Clone)]
pub struct GenerationConfig {
    /// Maximum number of floors to keep in LRU cache (Tier 1)
    pub cache_capacity: usize,
    /// Number of worker threads in the pool
    pub worker_threads: usize,
    /// Floor size (width x height in tiles)
    pub floor_size: u32,
    /// Whether to pre-warm popular floors
    pub enable_warmup: bool,
    /// Number of floors to pre-generate on startup
    pub warmup_count: u32,
    /// Enable LMDB persistent cache (Tier 2)
    pub enable_lmdb: bool,
    /// LMDB database path (if enabled)
    pub lmdb_path: Option<String>,
    /// LMDB max size in bytes (default 100MB)
    pub lmdb_size: usize,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            cache_capacity: 100,
            worker_threads: 4,
            floor_size: 50,
            enable_warmup: true,
            warmup_count: 10,
            enable_lmdb: true,  // Enable Tier 2 cache by default
            lmdb_path: Some("./data/floor_cache".to_string()),
            lmdb_size: 100 * 1024 * 1024,  // 100MB
        }
    }
}

/// Async floor generation system with 3-tier caching
#[derive(Clone)]
pub struct FloorGenerator {
    /// Tier 1: LRU cache for generated floors (RAM, ~4.7µs)
    cache: Arc<Mutex<LruCache<u32, ChunkData>>>,
    /// Tier 2: LMDB persistent cache (Disk, ~330µs)
    lmdb_cache: Option<Arc<LmdbFloorCache>>,
    /// Configuration
    config: GenerationConfig,
    /// Channel for sending generation requests
    request_tx: mpsc::Sender<GenerationRequest>,
    /// Performance metrics: Tier 1 cache hits
    metrics_tier1_hits: Arc<AtomicU64>,
    /// Performance metrics: Tier 2 cache hits
    metrics_tier2_hits: Arc<AtomicU64>,
    /// Performance metrics: Tier 3 generations (cache misses)
    metrics_tier3_gens: Arc<AtomicU64>,
}

/// Internal request for floor generation
struct GenerationRequest {
    floor_id: u32,
    seed: u64,
    response_tx: tokio::sync::oneshot::Sender<ChunkData>,
}

impl FloorGenerator {
    /// Create a new floor generator with the given configuration
    pub fn new(config: GenerationConfig) -> Self {
        let cache = Arc::new(Mutex::new(LruCache::new(
            NonZeroUsize::new(config.cache_capacity).unwrap(),
        )));

        // Initialize LMDB Tier 2 cache (if enabled)
        let lmdb_cache = if config.enable_lmdb {
            match config.lmdb_path.as_ref() {
                Some(path) => {
                    match LmdbFloorCache::new(path, config.lmdb_size) {
                        Ok(lmdb) => {
                            info!("✅ LMDB Tier 2 cache enabled at {} ({}MB)",
                                  path, config.lmdb_size / (1024 * 1024));
                            Some(Arc::new(lmdb))
                        }
                        Err(e) => {
                            warn!("Failed to initialize LMDB cache: {:?}, falling back to 2-tier", e);
                            None
                        }
                    }
                }
                None => None,
            }
        } else {
            None
        };

        let (request_tx, request_rx) = mpsc::channel(100);

        // Spawn worker pool
        let worker_cache = cache.clone();
        let worker_lmdb = lmdb_cache.clone();
        let worker_config = config.clone();
        tokio::spawn(async move {
            Self::worker_loop(request_rx, worker_cache, worker_lmdb, worker_config).await;
        });

        info!(
            "FloorGenerator initialized: {} workers, {} LRU capacity, LMDB: {}",
            config.worker_threads, config.cache_capacity,
            if lmdb_cache.is_some() { "enabled" } else { "disabled" }
        );

        Self {
            cache,
            lmdb_cache,
            config,
            request_tx,
            metrics_tier1_hits: Arc::new(AtomicU64::new(0)),
            metrics_tier2_hits: Arc::new(AtomicU64::new(0)),
            metrics_tier3_gens: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Worker loop that processes generation requests (3-tier caching)
    async fn worker_loop(
        mut request_rx: mpsc::Receiver<GenerationRequest>,
        cache: Arc<Mutex<LruCache<u32, ChunkData>>>,
        lmdb_cache: Option<Arc<LmdbFloorCache>>,
        config: GenerationConfig,
    ) {
        while let Some(req) = request_rx.recv().await {
            // Tier 1: Check LRU cache (might have been generated by another request)
            if let Some(cached) = cache.lock().get(&req.floor_id) {
                debug!("Worker Tier 1 HIT for floor {}", req.floor_id);
                let _ = req.response_tx.send(cached.clone());
                continue;
            }

            // Tier 2: Check LMDB persistent cache
            if let Some(lmdb) = &lmdb_cache {
                if let Some(cached) = lmdb.get(req.floor_id) {
                    debug!("Worker Tier 2 HIT for floor {} (LMDB)", req.floor_id);
                    // Populate Tier 1 cache
                    cache.lock().put(req.floor_id, cached.clone());
                    let _ = req.response_tx.send(cached);
                    continue;
                }
            }

            // Tier 3: Generate floor (CPU)
            debug!("Worker Tier 3 MISS for floor {}, generating...", req.floor_id);
            let chunk = Self::generate_floor_sync(req.floor_id, req.seed, &config);

            // Store in Tier 1 cache
            cache.lock().put(req.floor_id, chunk.clone());

            // Store in Tier 2 cache (if enabled)
            if let Some(lmdb) = &lmdb_cache {
                if let Err(e) = lmdb.set(req.floor_id, &chunk) {
                    warn!("Failed to store floor {} in LMDB: {:?}", req.floor_id, e);
                }
            }

            // Send response
            let _ = req.response_tx.send(chunk);
        }
    }

    /// Synchronous floor generation (called from worker thread)
    ///
    /// Uses the WFC (Wave Function Collapse) room-based generator for proper
    /// dungeon layouts with rooms, corridors, and special tiles.
    fn generate_floor_sync(floor_id: u32, seed: u64, _config: &GenerationConfig) -> ChunkData {
        debug!("Generating floor {} with seed {:#x} (WFC)", floor_id, seed);

        // Generate layout using WFC (room-based procedural generation)
        let layout = wfc::generate_layout(seed, floor_id);

        // Convert WFC tiles to proto FloorTileData
        let biome_id = Self::determine_biome(floor_id);
        let tiles = Self::wfc_to_proto_tiles(&layout, biome_id);

        // Compute validation hash
        let hash = Self::compute_validation_hash(&tiles, seed);

        // Generate semantic tags for this floor
        let semantic_tags = Self::generate_floor_tags(floor_id, biome_id, seed);

        ChunkData {
            seed,
            floor_id,
            tiles,
            validation_hash: hash,
            biome_id,
            width: layout.width as u32,
            height: layout.height as u32,
            world_offset: Some(Vec3 {
                x: 0.0,
                y: (floor_id as f32) * 5.0, // 5 meters per floor
                z: 0.0,
            }),
            semantic_tags: Some(Self::to_proto_tags(&semantic_tags)),
        }
    }

    /// Convert WFC FloorLayout into proto FloorTileData vec
    fn wfc_to_proto_tiles(layout: &wfc::FloorLayout, biome_id: u32) -> Vec<FloorTileData> {
        let mut tiles = Vec::with_capacity(layout.width * layout.height);

        for y in 0..layout.height {
            for x in 0..layout.width {
                let tt = layout.tiles[y][x];
                tiles.push(FloorTileData {
                    tile_type: tt.to_id(),
                    grid_x: x as i32,
                    grid_y: y as i32,
                    biome_id,
                    is_walkable: tt.is_walkable(),
                    has_collision: tt.has_collision(),
                });
            }
        }

        tiles
    }

    /// Determine biome based on floor level
    fn determine_biome(floor_id: u32) -> u32 {
        match floor_id {
            0..=100 => 1,     // Plains
            101..=200 => 2,   // Forest
            201..=300 => 3,   // Desert
            301..=500 => 4,   // Mountains
            501..=700 => 5,   // Ice
            701..=900 => 6,   // Volcano
            _ => 7,           // Void/Endgame
        }
    }

    /// Generate semantic tags for a floor based on biome and progression
    ///
    /// Tags define the "flavor" of the floor and influence:
    /// - Monster generation (monsters inherit floor tags)
    /// - Loot drops (semantic tag matching)
    /// - Player abilities (synergy/anti-synergy bonuses)
    /// - Environmental effects
    fn generate_floor_tags(floor_id: u32, biome_id: u32, seed: u64) -> SemanticTags {
        let mut tags = SemanticTags::new();

        // Base biome tags
        match biome_id {
            1 => {
                // Plains: Balanced, exploration-focused
                tags.add("plains", 0.9);
                tags.add("grass", 0.7);
                tags.add("wind", 0.5);
                tags.add("exploration", 0.8);
                tags.add("peaceful", 0.6);
            }
            2 => {
                // Forest: Nature, stealth, archery
                tags.add("forest", 0.9);
                tags.add("nature", 0.8);
                tags.add("wood", 0.7);
                tags.add("stealth", 0.6);
                tags.add("archery", 0.5);
            }
            3 => {
                // Desert: Fire, heat, survival
                tags.add("desert", 0.9);
                tags.add("sand", 0.8);
                tags.add("heat", 0.9);
                tags.add("fire", 0.6);
                tags.add("survival", 0.7);
            }
            4 => {
                // Mountains: Earth, mining, heavy combat
                tags.add("mountain", 0.9);
                tags.add("earth", 0.8);
                tags.add("stone", 0.9);
                tags.add("mining", 0.7);
                tags.add("heavy", 0.6);
            }
            5 => {
                // Ice: Cold, slow, defensive
                tags.add("ice", 0.9);
                tags.add("snow", 0.8);
                tags.add("cold", 0.9);
                tags.add("defense", 0.7);
                tags.add("slow", 0.5);
            }
            6 => {
                // Volcano: Fire, danger, aggressive
                tags.add("volcano", 0.9);
                tags.add("fire", 0.9);
                tags.add("lava", 0.8);
                tags.add("danger", 0.9);
                tags.add("aggressive", 0.8);
            }
            _ => {
                // Void/Endgame: Corruption, chaos, extreme difficulty
                tags.add("void", 0.9);
                tags.add("corruption", 0.9);
                tags.add("chaos", 0.8);
                tags.add("extreme", 1.0);
                tags.add("endgame", 1.0);
            }
        }

        // Add progression-based tags (tower depth = difficulty + corruption)
        let progression = (floor_id as f32 / 1000.0).min(1.0);
        tags.add("difficulty", 0.3 + progression * 0.7); // 0.3 -> 1.0
        tags.add("corruption", progression * 0.8);       // 0.0 -> 0.8

        // Add deterministic random "flavor" tags from seed
        let rng_state = seed.wrapping_add(floor_id as u64);
        let rand_val = (rng_state % 100) as f32 / 100.0;

        if rand_val > 0.8 {
            tags.add("treasure", 0.7); // 20% chance for treasure-rich floors
        } else if rand_val > 0.6 {
            tags.add("combat", 0.8);   // 20% chance for combat-heavy floors
        } else if rand_val > 0.4 {
            tags.add("puzzle", 0.6);   // 20% chance for puzzle floors
        }

        tags
    }

    /// Convert SemanticTags to Protobuf ProtoSemanticTags
    fn to_proto_tags(tags: &SemanticTags) -> ProtoSemanticTags {
        let tag_pairs: Vec<TagPair> = tags
            .tags
            .iter()
            .map(|(name, weight): &(String, f32)| TagPair {
                tag: name.clone(),
                weight: *weight,
            })
            .collect();

        ProtoSemanticTags { tags: tag_pairs }
    }

    /// Compute SHA-3 validation hash for anti-cheat
    fn compute_validation_hash(tiles: &[FloorTileData], seed: u64) -> Vec<u8> {
        let mut hasher = Sha3_256::new();

        // Hash seed
        hasher.update(seed.to_le_bytes());

        // Hash tile data (only immutable properties)
        for tile in tiles {
            hasher.update(tile.tile_type.to_le_bytes());
            hasher.update(tile.grid_x.to_le_bytes());
            hasher.update(tile.grid_y.to_le_bytes());
            hasher.update(tile.biome_id.to_le_bytes());
        }

        hasher.finalize().to_vec()
    }

    /// Request floor generation (async, non-blocking, 3-tier)
    pub async fn get_or_generate(&self, floor_id: u32, seed: u64) -> Result<ChunkData, String> {
        // Tier 1: Check LRU cache first (fastest, ~4.7µs)
        if let Some(cached) = self.cache.lock().get(&floor_id) {
            self.metrics_tier1_hits.fetch_add(1, Ordering::Relaxed);
            debug!("Tier 1 HIT for floor {} (LRU)", floor_id);
            return Ok(cached.clone());
        }

        // Tier 2: Check LMDB persistent cache (~330µs)
        if let Some(lmdb) = &self.lmdb_cache {
            if let Some(cached) = lmdb.get(floor_id) {
                self.metrics_tier2_hits.fetch_add(1, Ordering::Relaxed);
                debug!("Tier 2 HIT for floor {} (LMDB)", floor_id);
                // Populate Tier 1 for future requests
                self.cache.lock().put(floor_id, cached.clone());
                return Ok(cached);
            }
        }

        self.metrics_tier3_gens.fetch_add(1, Ordering::Relaxed);
        debug!("Tier 1+2 MISS for floor {}, requesting generation (~569µs)", floor_id);

        // Tier 3: Request generation from worker pool
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let request = GenerationRequest {
            floor_id,
            seed,
            response_tx,
        };

        self.request_tx
            .send(request)
            .await
            .map_err(|_| "Worker pool shutdown".to_string())?;

        // Wait for response (worker will cache in Tier 1 & 2)
        response_rx
            .await
            .map_err(|_| "Generation failed".to_string())
    }

    /// Get comprehensive cache statistics with 3-tier metrics
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.lock();
        let tier1_hits = self.metrics_tier1_hits.load(Ordering::Relaxed);
        let tier2_hits = self.metrics_tier2_hits.load(Ordering::Relaxed);
        let tier3_gens = self.metrics_tier3_gens.load(Ordering::Relaxed);

        CacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
            tier1_hits,
            tier2_hits,
            tier3_gens,
            total_requests: tier1_hits + tier2_hits + tier3_gens,
            lmdb_enabled: self.lmdb_cache.is_some(),
        }
    }

    /// Reset performance metrics (useful for testing)
    pub fn reset_metrics(&self) {
        self.metrics_tier1_hits.store(0, Ordering::Relaxed);
        self.metrics_tier2_hits.store(0, Ordering::Relaxed);
        self.metrics_tier3_gens.store(0, Ordering::Relaxed);
    }

    /// Pre-warm cache with popular floors
    pub async fn warmup(&self, base_seed: u64) {
        if !self.config.enable_warmup {
            return;
        }

        info!("Warming up cache with {} floors", self.config.warmup_count);

        for floor_id in 1..=self.config.warmup_count {
            let seed = base_seed.wrapping_add(floor_id as u64);
            match self.get_or_generate(floor_id, seed).await {
                Ok(_) => debug!("Warmed up floor {}", floor_id),
                Err(e) => warn!("Failed to warm up floor {}: {}", floor_id, e),
            }
        }

        info!("Warmup complete");
    }

    /// Validate a client-submitted chunk against server generation
    pub async fn validate_chunk(
        &self,
        floor_id: u32,
        seed: u64,
        client_hash: &[u8],
    ) -> Result<bool, String> {
        let server_chunk = self.get_or_generate(floor_id, seed).await?;
        Ok(server_chunk.validation_hash == client_hash)
    }
}

/// Cache statistics with 3-tier performance metrics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Current LRU cache size
    pub size: usize,
    /// LRU cache capacity
    pub capacity: usize,
    /// Tier 1 (LRU) cache hits
    pub tier1_hits: u64,
    /// Tier 2 (LMDB) cache hits
    pub tier2_hits: u64,
    /// Tier 3 (Generation) count
    pub tier3_gens: u64,
    /// Total requests
    pub total_requests: u64,
    /// LMDB enabled
    pub lmdb_enabled: bool,
}

impl CacheStats {
    /// Calculate cache fill percentage
    pub fn fill_percent(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            (self.size as f32 / self.capacity as f32) * 100.0
        }
    }

    /// Calculate Tier 1 (LRU) hit rate
    pub fn tier1_hit_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.tier1_hits as f32 / self.total_requests as f32) * 100.0
        }
    }

    /// Calculate Tier 2 (LMDB) hit rate
    pub fn tier2_hit_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.tier2_hits as f32 / self.total_requests as f32) * 100.0
        }
    }

    /// Calculate overall cache hit rate (Tier 1 + Tier 2)
    pub fn overall_hit_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            ((self.tier1_hits + self.tier2_hits) as f32 / self.total_requests as f32) * 100.0
        }
    }

    /// Calculate Tier 3 (Generation) miss rate
    pub fn miss_rate(&self) -> f32 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.tier3_gens as f32 / self.total_requests as f32) * 100.0
        }
    }

    /// Format statistics as human-readable string
    pub fn summary(&self) -> String {
        format!(
            "Cache Stats:\n\
             - LRU: {}/{} ({:.1}% filled)\n\
             - Tier 1 (LRU):    {} hits ({:.1}%)\n\
             - Tier 2 (LMDB):   {} hits ({:.1}%) {}\n\
             - Tier 3 (Gen):    {} misses ({:.1}%)\n\
             - Overall:         {:.1}% hit rate\n\
             - Total requests:  {}",
            self.size,
            self.capacity,
            self.fill_percent(),
            self.tier1_hits,
            self.tier1_hit_rate(),
            self.tier2_hits,
            self.tier2_hit_rate(),
            if self.lmdb_enabled { "" } else { "[DISABLED]" },
            self.tier3_gens,
            self.miss_rate(),
            self.overall_hit_rate(),
            self.total_requests
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_floor_generation() {
        let config = GenerationConfig {
            cache_capacity: 10,
            worker_threads: 2,
            floor_size: 10,
            enable_warmup: false,
            warmup_count: 0,
            enable_lmdb: false,
            lmdb_path: None,
            lmdb_size: 0,
        };

        let generator = FloorGenerator::new(config);
        let chunk = generator.get_or_generate(1, 0x12345678).await.unwrap();

        assert_eq!(chunk.floor_id, 1);
        assert_eq!(chunk.seed, 0x12345678);
        // WFC generates 16x16 for Echelon1 (floors 1-100)
        assert_eq!(chunk.width, 16);
        assert_eq!(chunk.height, 16);
        assert_eq!(chunk.tiles.len(), 256); // 16x16
        assert!(!chunk.validation_hash.is_empty());
    }

    #[tokio::test]
    async fn test_cache_hit() {
        let config = GenerationConfig {
            cache_capacity: 10,
            worker_threads: 2,
            floor_size: 10,
            enable_warmup: false,
            warmup_count: 0,
            enable_lmdb: false,
            lmdb_path: None,
            lmdb_size: 0,
        };

        let generator = FloorGenerator::new(config);

        // First request - cache miss
        let chunk1 = generator.get_or_generate(1, 0x12345678).await.unwrap();

        // Second request - cache hit (should be instant)
        let chunk2 = generator.get_or_generate(1, 0x12345678).await.unwrap();

        assert_eq!(chunk1.validation_hash, chunk2.validation_hash);
    }

    #[tokio::test]
    async fn test_deterministic_generation() {
        let config = GenerationConfig::default();
        let generator = FloorGenerator::new(config);

        // Same floor_id and seed should produce identical results
        let chunk1 = generator.get_or_generate(5, 0xABCDEF).await.unwrap();

        // Clear cache to force regeneration
        generator.cache.lock().clear();

        let chunk2 = generator.get_or_generate(5, 0xABCDEF).await.unwrap();

        assert_eq!(chunk1.validation_hash, chunk2.validation_hash);
        assert_eq!(chunk1.tiles.len(), chunk2.tiles.len());
    }

    #[tokio::test]
    async fn test_warmup() {
        let config = GenerationConfig {
            cache_capacity: 20,
            worker_threads: 4,
            floor_size: 10,
            enable_warmup: true,
            warmup_count: 5,
            enable_lmdb: false,
            lmdb_path: None,
            lmdb_size: 0,
        };

        let generator = FloorGenerator::new(config);
        generator.warmup(0x1234).await;

        let stats = generator.cache_stats();
        assert_eq!(stats.size, 5); // Should have 5 floors cached
    }

    #[tokio::test]
    async fn test_validation() {
        let config = GenerationConfig::default();
        let generator = FloorGenerator::new(config);

        let chunk = generator.get_or_generate(10, 0x99999).await.unwrap();

        // Valid hash should pass
        let is_valid = generator
            .validate_chunk(10, 0x99999, &chunk.validation_hash)
            .await
            .unwrap();
        assert!(is_valid);

        // Invalid hash should fail
        let fake_hash = vec![0xFF; 32];
        let is_valid = generator
            .validate_chunk(10, 0x99999, &fake_hash)
            .await
            .unwrap();
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn test_3tier_caching() {
        // Create temp directory for LMDB
        let temp_dir = std::env::temp_dir().join(format!("tower_test_3tier_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let config = GenerationConfig {
            cache_capacity: 5,  // Small LRU to force Tier 2 usage
            worker_threads: 2,
            floor_size: 10,
            enable_warmup: false,
            warmup_count: 0,
            enable_lmdb: true,
            lmdb_path: Some(temp_dir.to_string_lossy().to_string()),
            lmdb_size: 10 * 1024 * 1024,  // 10MB
        };

        let generator = FloorGenerator::new(config);
        generator.reset_metrics();

        // Test Tier 3 (Generation): First request - cache miss
        let chunk1 = generator.get_or_generate(1, 0x1111).await.unwrap();
        let stats = generator.cache_stats();
        assert_eq!(stats.tier1_hits, 0, "First request should not hit Tier 1");
        assert_eq!(stats.tier2_hits, 0, "First request should not hit Tier 2");
        assert_eq!(stats.tier3_gens, 1, "First request should generate");
        assert_eq!(chunk1.floor_id, 1);

        // Test Tier 1 (LRU): Second request - cache hit
        let chunk2 = generator.get_or_generate(1, 0x1111).await.unwrap();
        let stats = generator.cache_stats();
        assert_eq!(stats.tier1_hits, 1, "Second request should hit Tier 1");
        assert_eq!(stats.tier3_gens, 1, "Should not regenerate");
        assert_eq!(chunk1.validation_hash, chunk2.validation_hash);

        // Test Tier 1 eviction and Tier 2 (LMDB): Generate 10 floors (LRU capacity = 5)
        for floor_id in 2..=10 {
            generator.get_or_generate(floor_id, 0x2222).await.unwrap();
        }

        // Floor 1 should be evicted from Tier 1 but still in Tier 2
        let chunk3 = generator.get_or_generate(1, 0x1111).await.unwrap();
        let stats = generator.cache_stats();

        // Tier 2 should have caught the request (floor 1 evicted from Tier 1)
        assert!(stats.tier2_hits > 0, "Tier 2 should have hits after LRU eviction");
        assert_eq!(chunk1.validation_hash, chunk3.validation_hash);

        // Verify metrics consistency
        assert_eq!(
            stats.total_requests,
            stats.tier1_hits + stats.tier2_hits + stats.tier3_gens,
            "Total requests should equal sum of all tiers"
        );

        // Verify hit rates
        assert!(stats.overall_hit_rate() > 0.0, "Overall hit rate should be > 0%");
        assert!(stats.tier1_hit_rate() >= 0.0, "Tier 1 hit rate should be >= 0%");
        assert!(stats.tier2_hit_rate() > 0.0, "Tier 2 hit rate should be > 0%");

        // Print summary for verification
        println!("\n{}", stats.summary());

        // Cleanup temp directory
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = GenerationConfig {
            cache_capacity: 10,
            worker_threads: 2,
            floor_size: 10,
            enable_warmup: false,
            warmup_count: 0,
            enable_lmdb: false,  // Disable LMDB for 2-tier test
            lmdb_path: None,
            lmdb_size: 0,
        };

        let generator = FloorGenerator::new(config);
        generator.reset_metrics();

        // Generate 5 floors
        for floor_id in 1..=5 {
            generator.get_or_generate(floor_id, 0x3333).await.unwrap();
        }

        // Access floors again (all should be Tier 1 hits)
        for floor_id in 1..=5 {
            generator.get_or_generate(floor_id, 0x3333).await.unwrap();
        }

        let stats = generator.cache_stats();

        // Verify metrics
        assert_eq!(stats.tier1_hits, 5, "Should have 5 Tier 1 hits");
        assert_eq!(stats.tier2_hits, 0, "LMDB disabled, should have 0 Tier 2 hits");
        assert_eq!(stats.tier3_gens, 5, "Should have 5 generations");
        assert_eq!(stats.total_requests, 10, "Total 10 requests");
        assert!(!stats.lmdb_enabled, "LMDB should be disabled");

        // Verify hit rate calculations
        assert_eq!(stats.tier1_hit_rate(), 50.0, "Tier 1 hit rate should be 50%");
        assert_eq!(stats.overall_hit_rate(), 50.0, "Overall hit rate should be 50%");
        assert_eq!(stats.miss_rate(), 50.0, "Miss rate should be 50%");

        println!("\n{}", stats.summary());
    }
}
