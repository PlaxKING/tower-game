/// Complete Cache Comparison Benchmarks
///
/// Compares 3-tier caching architecture:
/// 1. LRU Cache (RAM) - ~4.7µs (Tier 1: hot cache)
/// 2. LMDB Cache (Embedded DB) - ~330µs (Tier 2: persistent cache)
/// 3. CPU Generation - ~569µs (Tier 3: on-demand generation)
///
/// Result: 3-tier with LMDB is optimal (Redis removed as too slow)
use criterion::{criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tower_bevy_server::async_generation::{FloorGenerator, GenerationConfig};
use tower_bevy_server::lmdb_cache::LmdbFloorCache;

/// Benchmark: Complete 3-tier comparison
fn bench_complete_comparison(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("complete_cache_comparison");

    // Prepare test data ONCE
    let (generator, chunk) = runtime.block_on(async {
        let config = GenerationConfig {
            cache_capacity: 10,
            worker_threads: 2,
            floor_size: 100,
            enable_warmup: false,
            warmup_count: 0,
        };
        let generator = FloorGenerator::new(config.clone());
        let chunk = generator.get_or_generate(1, 0x12345678).await.unwrap();
        (generator, chunk)
    });

    // Tier 1: LRU Cache (RAM)
    group.bench_function("1_lru_ram_cache", |b| {
        let gen = generator.clone();

        // Pre-populate ONCE
        runtime.block_on(async {
            gen.get_or_generate(1, 0x12345678).await.unwrap();
        });

        b.to_async(&runtime).iter(|| {
            let gen_clone = gen.clone();
            async move { gen_clone.get_or_generate(1, 0x12345678).await.unwrap() }
        });
    });

    // Tier 2: LMDB Cache (Embedded DB)
    group.bench_function("2_lmdb_embedded_db", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_bench_{}", std::process::id()));
        let cache = LmdbFloorCache::new(temp_dir, 100 * 1024 * 1024) // 100MB
            .expect("Failed to create LMDB cache");

        // Pre-populate ONCE
        cache.set(1, &chunk).expect("Failed to set floor in LMDB");

        b.iter(|| cache.get(1).expect("Failed to get floor from LMDB"));
    });

    // Tier 3: CPU Generation (Baseline)
    group.bench_function("3_cpu_generation", |b| {
        let gen = runtime.block_on(async {
            let config = GenerationConfig {
                cache_capacity: 1,
                worker_threads: 1,
                floor_size: 100,
                enable_warmup: false,
                warmup_count: 0,
            };
            FloorGenerator::new(config)
        });
        let mut floor_id_counter = 0u32;

        b.to_async(&runtime).iter(|| {
            let gen_clone = gen.clone();
            floor_id_counter += 1;
            let floor_id = floor_id_counter;

            async move {
                gen_clone
                    .get_or_generate(floor_id, 0x12345678 + floor_id as u64)
                    .await
                    .unwrap()
            }
        });
    });

    group.finish();
}

/// Benchmark: LMDB-specific operations
fn bench_lmdb_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let chunk = runtime.block_on(async {
        let config = GenerationConfig {
            cache_capacity: 1,
            worker_threads: 1,
            floor_size: 100,
            enable_warmup: false,
            warmup_count: 0,
        };
        let generator = FloorGenerator::new(config);
        generator.get_or_generate(1, 0x12345678).await.unwrap()
    });

    let chunk = Arc::new(chunk);

    c.bench_function("lmdb_get_hit", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_get_hit_{}", std::process::id()));
        let cache =
            LmdbFloorCache::new(&temp_dir, 100 * 1024 * 1024).expect("Failed to create LMDB cache");

        // Pre-populate
        cache.set(1, &chunk).unwrap();

        b.iter(|| cache.get(1).unwrap());
    });

    c.bench_function("lmdb_get_miss", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_get_miss_{}", std::process::id()));
        let cache =
            LmdbFloorCache::new(&temp_dir, 100 * 1024 * 1024).expect("Failed to create LMDB cache");

        b.iter(|| cache.get(99999));
    });

    c.bench_function("lmdb_set", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_set_{}", std::process::id()));
        let cache =
            LmdbFloorCache::new(&temp_dir, 500 * 1024 * 1024).expect("Failed to create LMDB cache"); // 500MB for intensive writes
        let mut floor_id_counter = 0u32;

        b.iter(|| {
            floor_id_counter += 1;
            cache.set(floor_id_counter, &chunk).unwrap()
        });
    });

    c.bench_function("lmdb_roundtrip", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_roundtrip_{}", std::process::id()));
        let cache =
            LmdbFloorCache::new(&temp_dir, 500 * 1024 * 1024).expect("Failed to create LMDB cache"); // 500MB for intensive writes
        let mut floor_id_counter = 0u32;

        b.iter(|| {
            floor_id_counter += 1;
            cache.set(floor_id_counter, &chunk).unwrap();
            cache.get(floor_id_counter).unwrap()
        });
    });
}

/// Benchmark: LMDB persistent cache performance
fn bench_persistence_comparison(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("persistence_comparison");

    let chunk = runtime.block_on(async {
        let config = GenerationConfig {
            cache_capacity: 1,
            worker_threads: 1,
            floor_size: 100,
            enable_warmup: false,
            warmup_count: 0,
        };
        let generator = FloorGenerator::new(config);
        generator.get_or_generate(1, 0x12345678).await.unwrap()
    });
    let chunk = Arc::new(chunk);

    // LMDB (embedded persistent cache - Tier 2)
    group.bench_function("lmdb_persistent_get", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_persist_{}", std::process::id()));
        let cache = LmdbFloorCache::new(temp_dir, 100 * 1024 * 1024).unwrap();
        cache.set(1, &chunk).unwrap();

        b.iter(|| cache.get(1).unwrap());
    });

    group.finish();
}

/// Benchmark: Batch operations comparison
fn bench_batch_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("batch_operations");

    // Generate 10 floors ONCE
    let chunks = runtime.block_on(async {
        let config = GenerationConfig {
            cache_capacity: 100,
            worker_threads: 4,
            floor_size: 50,
            enable_warmup: false,
            warmup_count: 0,
        };
        let generator = FloorGenerator::new(config);

        let mut chunks = Vec::new();
        for i in 1..=10 {
            let chunk = generator
                .get_or_generate(i, 0x12345678 + i as u64)
                .await
                .unwrap();
            chunks.push(chunk);
        }
        chunks
    });

    // LMDB batch SET
    group.bench_function("lmdb_batch_set_10", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_batch_set_{}", std::process::id()));
        let cache = LmdbFloorCache::new(temp_dir, 1024 * 1024 * 1024).unwrap(); // 1GB for intensive batch writes
        let mut floor_id_base = 0u32;

        b.iter(|| {
            for (i, chunk) in chunks.iter().enumerate() {
                cache.set(floor_id_base + i as u32, chunk).unwrap();
            }
            floor_id_base += 10;
        });
    });

    // LMDB batch GET
    group.bench_function("lmdb_batch_get_10", |b| {
        let temp_dir = std::env::temp_dir().join(format!("lmdb_batch_get_{}", std::process::id()));
        let cache = LmdbFloorCache::new(temp_dir, 100 * 1024 * 1024).unwrap();

        // Pre-populate
        for (i, chunk) in chunks.iter().enumerate() {
            cache.set(i as u32, chunk).unwrap();
        }

        b.iter(|| {
            for i in 0..10 {
                cache.get(i as u32).unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_complete_comparison,
    bench_lmdb_operations,
    bench_persistence_comparison,
    bench_batch_operations
);
criterion_main!(benches);
