use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use tower_bevy_server::async_generation::{FloorGenerator, GenerationConfig};

// Make async_generation module public for benchmarks
// Note: You may need to add `pub mod async_generation;` to lib.rs or make it a library crate

fn bench_single_floor_generation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("generate_single_floor_10x10", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 1,
                worker_threads: 1,
                floor_size: 10,
                enable_warmup: false,
                warmup_count: 0,
            };

            let generator = FloorGenerator::new(config);
            generator.get_or_generate(1, 0x12345678).await.unwrap()
        });
    });

    c.bench_function("generate_single_floor_50x50", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 1,
                worker_threads: 1,
                floor_size: 50,
                enable_warmup: false,
                warmup_count: 0,
            };

            let generator = FloorGenerator::new(config);
            generator.get_or_generate(1, 0xABCDEF).await.unwrap()
        });
    });

    c.bench_function("generate_single_floor_100x100", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 1,
                worker_threads: 1,
                floor_size: 100,
                enable_warmup: false,
                warmup_count: 0,
            };

            let generator = FloorGenerator::new(config);
            generator.get_or_generate(1, 0x999999).await.unwrap()
        });
    });
}

fn bench_cache_performance(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("cache_hit", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 10,
                worker_threads: 2,
                floor_size: 50,
                enable_warmup: false,
                warmup_count: 0,
            };

            let generator = FloorGenerator::new(config);

            // Pre-populate cache
            generator.get_or_generate(1, 0x12345678).await.unwrap();

            // Measure cache hit time
            generator.get_or_generate(1, 0x12345678).await.unwrap()
        });
    });

    c.bench_function("cache_miss", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 10,
                worker_threads: 2,
                floor_size: 50,
                enable_warmup: false,
                warmup_count: 0,
            };

            let generator = FloorGenerator::new(config);

            // Always generate different floor (cache miss)
            use std::sync::atomic::{AtomicU32, Ordering};
            static COUNTER: AtomicU32 = AtomicU32::new(1);
            let floor_id = COUNTER.fetch_add(1, Ordering::SeqCst);

            generator
                .get_or_generate(floor_id, 0x12345678)
                .await
                .unwrap()
        });
    });
}

fn bench_parallel_generation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut group = c.benchmark_group("parallel_generation");

    for workers in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_workers", workers)),
            workers,
            |b, &workers| {
                b.to_async(&runtime).iter(|| async move {
                    let config = GenerationConfig {
                        cache_capacity: 100,
                        worker_threads: workers,
                        floor_size: 50,
                        enable_warmup: false,
                        warmup_count: 0,
                    };

                    let generator = FloorGenerator::new(config);

                    // Generate 10 floors in parallel
                    let mut handles = Vec::new();
                    for floor_id in 1..=10 {
                        let gen = generator.clone();
                        let handle = tokio::spawn(async move {
                            gen.get_or_generate(floor_id, 0x12345678 + floor_id as u64)
                                .await
                                .unwrap()
                        });
                        handles.push(handle);
                    }

                    // Wait for all to complete
                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_warmup_time(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("warmup_10_floors", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 100,
                worker_threads: 4,
                floor_size: 50,
                enable_warmup: true,
                warmup_count: 10,
            };

            let generator = FloorGenerator::new(config);
            generator.warmup(0x1234).await;
        });
    });

    c.bench_function("warmup_50_floors", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = GenerationConfig {
                cache_capacity: 100,
                worker_threads: 4,
                floor_size: 50,
                enable_warmup: true,
                warmup_count: 50,
            };

            let generator = FloorGenerator::new(config);
            generator.warmup(0x1234).await;
        });
    });
}

criterion_group!(
    benches,
    bench_single_floor_generation,
    bench_cache_performance,
    bench_parallel_generation,
    bench_warmup_time
);
criterion_main!(benches);
