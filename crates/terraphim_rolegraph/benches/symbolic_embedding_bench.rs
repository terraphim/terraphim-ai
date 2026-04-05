//! Benchmarks for symbolic embedding operations
//!
//! These benchmarks validate performance targets for embedding construction
//! and similarity queries at various scales.

use ahash::{AHashMap, AHashSet};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use terraphim_rolegraph::symbolic_embeddings::SymbolicEmbeddingIndex;
use terraphim_types::MedicalNodeType;

/// Generate a random DAG with specified node count for benchmarking
fn generate_dag(
    node_count: usize,
) -> (AHashMap<u64, AHashSet<u64>>, AHashMap<u64, MedicalNodeType>) {
    let mut isa_parents: AHashMap<u64, AHashSet<u64>> = AHashMap::new();
    let mut node_types: AHashMap<u64, MedicalNodeType> = AHashMap::new();

    // Define node types cyclically
    let types = [
        MedicalNodeType::Disease,
        MedicalNodeType::Drug,
        MedicalNodeType::Gene,
        MedicalNodeType::Anatomy,
        MedicalNodeType::Protein,
        MedicalNodeType::Symptom,
    ];

    for i in 0..node_count {
        node_types.insert(i as u64, types[i % types.len()]);
    }

    // Create edges: each node (except 0) connects to 1-3 parents with lower IDs
    for i in 1..node_count {
        let num_parents = if i == 1 {
            1
        } else {
            let max_parents = (i - 1).min(3);
            1 + (i % max_parents)
        };

        let mut parents = AHashSet::new();
        for j in 0..num_parents {
            let parent_id = ((i - 1 - j) % i) as u64;
            parents.insert(parent_id);
        }

        if !parents.is_empty() {
            isa_parents.insert(i as u64, parents);
        }
    }

    (isa_parents, node_types)
}

/// Benchmark: Build index from hierarchy at various scales
/// Targets: 100 < 1ms, 1000 < 10ms, 10000 < 100ms
fn bench_build_from_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_from_hierarchy");

    for size in [100, 1000, 10000].iter() {
        let (isa_parents, node_types) = generate_dag(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
                index
            })
        });
    }

    group.finish();
}

/// Benchmark: Similarity query with cold cache (empty cache)
/// Target: < 10us for single pair
fn bench_similarity_cold_cache(c: &mut Criterion) {
    let (isa_parents, node_types) = generate_dag(1000);

    c.bench_function("similarity_cold_cache", |b| {
        b.iter(|| {
            // Create a fresh index each iteration to ensure cold cache
            let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);
            // Query a single pair
            let _ = index.similarity(100, 200);
        })
    });
}

/// Benchmark: Similarity query with warm cache (populated cache)
/// Target: < 1us for single pair
fn bench_similarity_warm_cache(c: &mut Criterion) {
    let (isa_parents, node_types) = generate_dag(1000);
    let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

    // Warm up the cache
    let _ = index.similarity(100, 200);
    let _ = index.similarity(100, 300);
    let _ = index.similarity(200, 300);

    c.bench_function("similarity_warm_cache", |b| {
        b.iter(|| {
            // Query the same pair repeatedly (should hit cache)
            let _ = index.similarity(100, 200);
        })
    });
}

/// Benchmark: Nearest neighbors query
/// Targets: n=1000, k=20 < 50ms; n=10000, k=20 < 500ms
fn bench_nearest_neighbors(c: &mut Criterion) {
    let mut group = c.benchmark_group("nearest_neighbors");

    for size in [1000, 10000].iter() {
        let (isa_parents, node_types) = generate_dag(*size);
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        group.bench_with_input(BenchmarkId::new("k20", size), size, |b, _| {
            b.iter(|| {
                let _ = index.nearest_neighbors(100, 20);
            })
        });
    }

    group.finish();
}

/// Benchmark: Nearest neighbors by type filter
fn bench_nearest_neighbors_by_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("nearest_neighbors_by_type");

    for size in [1000, 10000].iter() {
        let (isa_parents, node_types) = generate_dag(*size);
        let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

        group.bench_with_input(BenchmarkId::new("k20_disease", size), size, |b, _| {
            b.iter(|| {
                let _ = index.nearest_neighbors_by_type(100, MedicalNodeType::Disease, 20);
            })
        });
    }

    group.finish();
}

/// Benchmark: All embeddings iteration
fn bench_all_embeddings(c: &mut Criterion) {
    let (isa_parents, node_types) = generate_dag(10000);
    let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

    c.bench_function("all_embeddings_iteration", |b| {
        b.iter(|| {
            let count: usize = index.all_embeddings().count();
            criterion::black_box(count);
        })
    });
}

/// Benchmark: Cache statistics and clear operations
fn bench_cache_operations(c: &mut Criterion) {
    let (isa_parents, node_types) = generate_dag(1000);
    let index = SymbolicEmbeddingIndex::build_from_hierarchy(&isa_parents, &node_types);

    // Populate cache
    for i in 0..100 {
        for j in (i + 1)..100 {
            let _ = index.similarity(i as u64, j as u64);
        }
    }

    c.bench_function("cache_stats", |b| {
        b.iter(|| {
            let (cache_size, total) = index.cache_stats();
            criterion::black_box((cache_size, total));
        })
    });

    c.bench_function("cache_clear", |b| {
        b.iter(|| {
            index.clear_cache();
        })
    });
}

criterion_group!(
    benches,
    bench_build_from_hierarchy,
    bench_similarity_cold_cache,
    bench_similarity_warm_cache,
    bench_nearest_neighbors,
    bench_nearest_neighbors_by_type,
    bench_all_embeddings,
    bench_cache_operations
);
criterion_main!(benches);
