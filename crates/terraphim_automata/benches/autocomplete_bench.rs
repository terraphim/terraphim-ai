//! Benchmarks for testing the throughput and performance of the autocomplete functionality.
//!
//! To run a single benchmark use:
//!
//! ```sh
//! cargo bench --bench autocomplete_bench -- build_index
//! ```
//!
//! To run all autocomplete benchmarks:
//!
//! ```sh
//! cargo bench --bench autocomplete_bench
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use terraphim_automata::autocomplete::{
    autocomplete_search, build_autocomplete_index, deserialize_autocomplete_index,
    fuzzy_autocomplete_search, fuzzy_autocomplete_search_levenshtein, serialize_autocomplete_index,
    AutocompleteIndex,
};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

#[cfg(feature = "tokio-runtime")]
use tokio::runtime::Runtime;

#[cfg(feature = "tokio-runtime")]
lazy_static::lazy_static! {
    static ref TOKIO_RUNTIME: Runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
}

// We can use this `block_on` function to run async code in the benchmarks when tokio is available
#[cfg(feature = "tokio-runtime")]
#[inline]
pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    TOKIO_RUNTIME.block_on(future)
}

/// Create a large test thesaurus for performance testing
fn create_large_thesaurus(size: usize) -> Thesaurus {
    let mut thesaurus = Thesaurus::new("Performance Test".to_string());

    // Create terms with various patterns to test different FST scenarios
    for i in 0..size {
        let base_terms = vec![
            format!("term_{:06}", i),
            format!("concept_{:06}", i),
            format!("idea_{:06}", i),
            format!("definition_{:06}", i),
            format!("machine_learning_technique_{:06}", i),
            format!("artificial_intelligence_method_{:06}", i),
            format!("data_science_algorithm_{:06}", i),
            format!("programming_language_feature_{:06}", i),
        ];

        for (j, term) in base_terms.into_iter().enumerate() {
            let normalized_term = NormalizedTerm {
                id: (i * 8 + j) as u64 + 1,
                value: NormalizedTermValue::from(term.clone()),
                url: Some(format!("https://example.com/{}", term.replace('_', "-"))),
            };
            thesaurus.insert(NormalizedTermValue::from(term), normalized_term);
        }
    }

    thesaurus
}

/// Load a sample thesaurus for benchmarking
fn load_sample_thesaurus() -> Thesaurus {
    // Try to load from local example, fallback to creating synthetic data
    #[cfg(all(feature = "remote-loading", feature = "tokio-runtime"))]
    {
        match block_on(load_thesaurus(&AutomataPath::local_example())) {
            Ok(thesaurus) => return thesaurus,
            Err(_) => {
                log::warn!("Could not load local example, creating synthetic thesaurus");
            }
        }
    }

    // Fallback to synthetic data
    create_large_thesaurus(100)
}

/// Create an autocomplete index for benchmarking
fn create_benchmark_index(size: usize) -> AutocompleteIndex {
    let thesaurus = create_large_thesaurus(size);
    build_autocomplete_index(thesaurus, None).unwrap()
}

/// Benchmark FST index building with different thesaurus sizes
fn bench_build_index_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_index_throughput");

    // Test with different thesaurus sizes
    for size in &[100, 500, 1000, 2500, 5000, 10000] {
        let thesaurus = create_large_thesaurus(*size);
        let thesaurus_bytes = bincode::serialize(&thesaurus).unwrap().len() as u64;

        group.throughput(Throughput::Bytes(thesaurus_bytes));
        group.bench_with_input(
            BenchmarkId::new("build_autocomplete_index", size),
            size,
            |b, &size| {
                let thesaurus = create_large_thesaurus(size);
                b.iter(|| build_autocomplete_index(thesaurus.clone(), None).unwrap())
            },
        );
    }
    group.finish();
}

/// Benchmark autocomplete search with different query lengths and result sizes
fn bench_search_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_throughput");

    let index = create_benchmark_index(5000); // Use a reasonably large index

    let test_queries = vec![
        ("single_char", "a"),
        ("short_prefix", "ma"),
        ("medium_prefix", "mach"),
        ("long_prefix", "machine"),
        ("very_long_prefix", "machine_learning"),
        ("exact_match", "machine_learning_technique_001234"),
    ];

    for (query_name, query) in test_queries {
        let query_bytes = query.len() as u64;
        group.throughput(Throughput::Bytes(query_bytes));
        group.bench_with_input(
            BenchmarkId::new("autocomplete_search", query_name),
            &query,
            |b, &query| b.iter(|| autocomplete_search(&index, query, Some(10)).unwrap()),
        );
    }
    group.finish();
}

/// Benchmark autocomplete search with different result limits
fn bench_search_result_limits(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_result_limits");

    let index = create_benchmark_index(10000);
    let query = "term"; // This should match many terms

    // Test different result limits
    for limit in &[1, 5, 10, 25, 50, 100, 250] {
        group.bench_with_input(
            BenchmarkId::new("search_with_limit", limit),
            limit,
            |b, &limit| b.iter(|| autocomplete_search(&index, query, Some(limit)).unwrap()),
        );
    }
    group.finish();
}

/// Benchmark fuzzy search performance
fn bench_fuzzy_search(c: &mut Criterion) {
    let index = create_benchmark_index(2000);

    let fuzzy_queries = vec![
        ("typo_single", "machne"),      // machine with typo
        ("typo_double", "artifical"),   // artificial with typo
        ("missing_char", "dat"),        // data with missing char
        ("extra_char", "programmingg"), // programming with extra char
    ];

    for (query_name, query) in fuzzy_queries {
        c.bench_function(&format!("fuzzy_search_{}", query_name), |b| {
            b.iter(|| fuzzy_autocomplete_search(&index, query, 0.6, Some(10)).unwrap())
        });
    }
}

/// Benchmark comparison between Levenshtein and Jaro-Winkler fuzzy search
fn bench_fuzzy_algorithm_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuzzy_algorithm_comparison");

    let index = create_benchmark_index(2000);

    let test_cases = vec![
        ("transposition", "machien"),    // machine with transposed i/e
        ("missing_char", "machne"),      // machine with missing i
        ("extra_char", "machinee"),      // machine with extra e
        ("prefix_match", "mach"),        // partial prefix
        ("complex_typo", "aritificial"), // artificial with multiple errors
        ("word_space", "datascience"),   // data science without space
    ];

    for (case_name, query) in test_cases {
        // Benchmark Levenshtein distance approach
        group.bench_function(&format!("levenshtein_{}", case_name), |b| {
            b.iter(|| fuzzy_autocomplete_search_levenshtein(&index, query, 2, Some(10)).unwrap())
        });

        // Benchmark Jaro-Winkler similarity approach (now the default)
        group.bench_function(&format!("jaro_winkler_{}", case_name), |b| {
            b.iter(|| fuzzy_autocomplete_search(&index, query, 0.5, Some(10)).unwrap())
        });
    }

    group.finish();
}

/// Benchmark serialization and deserialization performance
fn bench_serialization_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization_throughput");

    // Test serialization with different index sizes
    for size in &[100, 500, 1000, 2500, 5000] {
        let index = create_benchmark_index(*size);

        group.bench_with_input(
            BenchmarkId::new("serialize_index", size),
            &index,
            |b, index| b.iter(|| serialize_autocomplete_index(index).unwrap()),
        );

        // Benchmark deserialization
        let serialized = serialize_autocomplete_index(&index).unwrap();
        let serialized_bytes = serialized.len() as u64;

        group.throughput(Throughput::Bytes(serialized_bytes));
        group.bench_with_input(
            BenchmarkId::new("deserialize_index", size),
            &serialized,
            |b, data| b.iter(|| deserialize_autocomplete_index(data).unwrap()),
        );
    }
    group.finish();
}

/// Benchmark memory usage and index size scaling
fn bench_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scaling");

    // Measure index building time vs memory usage
    for size in &[100, 500, 1000, 2500] {
        let thesaurus = create_large_thesaurus(*size);
        let estimated_memory = size * 200; // Rough estimate: 200 bytes per term

        group.throughput(Throughput::Bytes(estimated_memory as u64));
        group.bench_with_input(
            BenchmarkId::new("memory_efficient_build", size),
            size,
            |b, &size| {
                let thesaurus = create_large_thesaurus(size);
                b.iter(|| {
                    let index = build_autocomplete_index(thesaurus.clone(), None).unwrap();
                    // Perform a search to ensure the index is actually used
                    autocomplete_search(&index, "term", Some(5)).unwrap()
                })
            },
        );
    }
    group.finish();
}

/// Benchmark concurrent search performance
fn bench_concurrent_search(c: &mut Criterion) {
    let index = std::sync::Arc::new(create_benchmark_index(5000));

    c.bench_function("concurrent_search_10_threads", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..10)
                .map(|i| {
                    let index = index.clone();
                    let query = format!("term_{:02}", i % 100);
                    std::thread::spawn(move || {
                        autocomplete_search(&index, &query, Some(10)).unwrap()
                    })
                })
                .collect();

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

/// Benchmark real-world usage patterns
fn bench_realistic_usage(c: &mut Criterion) {
    let index = create_benchmark_index(3000);

    // Simulate realistic user typing patterns
    let typing_patterns = vec![
        vec!["m", "ma", "mac", "mach", "machi", "machin", "machine"],
        vec![
            "a",
            "ar",
            "art",
            "arti",
            "artif",
            "artifi",
            "artifici",
            "artificial",
        ],
        vec!["d", "da", "dat", "data"],
        vec!["p", "pr", "pro", "prog", "progr", "progra", "program"],
    ];

    for (i, pattern) in typing_patterns.iter().enumerate() {
        c.bench_function(&format!("typing_pattern_{}", i), |b| {
            b.iter(|| {
                for prefix in pattern {
                    autocomplete_search(&index, prefix, Some(10)).unwrap();
                }
            })
        });
    }
}

/// Compare autocomplete performance with existing Aho-Corasick matcher
fn bench_comparison_with_matcher(c: &mut Criterion) {
    let mut group = c.benchmark_group("autocomplete_vs_matcher");

    let thesaurus = load_sample_thesaurus();
    let index = build_autocomplete_index(thesaurus.clone(), None).unwrap();

    let test_text = "This text contains machine learning and artificial intelligence concepts that we want to find using both autocomplete and the existing matcher functionality for comparison.";
    let search_query = "machine";

    // Benchmark autocomplete search
    group.bench_function("autocomplete_search", |b| {
        b.iter(|| autocomplete_search(&index, search_query, Some(10)).unwrap())
    });

    // Benchmark existing matcher functionality
    group.bench_function("aho_corasick_matcher", |b| {
        b.iter(|| terraphim_automata::find_matches(test_text, thesaurus.clone(), false).unwrap())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_build_index_throughput,
    bench_search_throughput,
    bench_search_result_limits,
    bench_fuzzy_search,
    bench_fuzzy_algorithm_comparison,
    bench_serialization_throughput,
    bench_memory_scaling,
    bench_concurrent_search,
    bench_realistic_usage,
    bench_comparison_with_matcher
);
criterion_main!(benches);
