//! Benchmarks for `terraphim_grep::HybridSearcher`.
//!
//! Three groups:
//!   - `code_only`           -- code haystack only (fff-search), no KG
//!   - `hybrid_with_kg`      -- code + KG concept extraction in parallel
//!   - `fuse_and_rank`       -- isolated sort/rank cost across chunk batches
//!
//! Run all:
//!   cargo bench -p terraphim_grep --features code-search --bench hybrid_search
//!
//! Run a single group:
//!   cargo bench -p terraphim_grep --features code-search --bench hybrid_search -- hybrid_with_kg
//!
//! The benchmarks build their own synthetic corpus in a tempdir so they do not depend on
//! the host filesystem layout. The thesaurus is constructed in-memory with deterministic
//! sizes to make latency differences attributable to KG size rather than disk variance.

use std::path::PathBuf;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tempfile::TempDir;
use tokio::runtime::Runtime;

use terraphim_grep::{GrepOptions, Haystack, HybridSearcher, RetrievedChunk};
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Create a temp directory populated with `file_count` Rust source files, each containing
/// the query token plus filler. Returns the tempdir handle (must outlive the benchmark) and
/// its path.
fn make_corpus(file_count: usize, query_token: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create tempdir");
    for i in 0..file_count {
        let path = dir.path().join(format!("src_{i}.rs"));
        let body = format!(
            "// auto-generated for bench\n\
             fn {token}_{i}() -> u32 {{ {i} }}\n\
             fn unrelated_{i}() {{ let x = {i}; let _ = x * 2; }}\n\
             // {token} appears in this comment too\n",
            token = query_token,
            i = i
        );
        std::fs::write(&path, body).expect("write fixture");
    }
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Build a synthetic thesaurus with `term_count` entries. Each term is mapped to a unique id
/// and a fake URL. Used to size the KG so we can measure KG-search cost as a function of
/// thesaurus size.
fn make_thesaurus(term_count: usize, query_token: &str) -> Thesaurus {
    let mut t = Thesaurus::new("bench".to_string());
    // Always include the query token so KG search has at least one hit.
    let key = NormalizedTermValue::new(query_token.to_string());
    let mut term = NormalizedTerm::new(1, key.clone());
    term.url = Some(format!("https://example.org/{query_token}"));
    t.insert(key, term);

    for i in 0..term_count {
        let name = format!("filler_term_{i}");
        let key = NormalizedTermValue::new(name.clone());
        let mut term = NormalizedTerm::new((i as u64) + 2, key.clone());
        term.url = Some(format!("https://example.org/{name}"));
        t.insert(key, term);
    }
    t
}

fn make_searcher(thesaurus: Thesaurus, search_path: PathBuf) -> Arc<HybridSearcher> {
    let searcher = HybridSearcher::new("bench-role".to_string(), thesaurus)
        .expect("build hybrid searcher")
        .with_search_path(search_path);
    Arc::new(searcher)
}

fn bench_code_only(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("code_only");

    // Empty thesaurus -- KG side returns nothing, so we measure the fff-search path alone.
    let thesaurus = Thesaurus::new("empty".to_string());
    let query = "parse_grep_query";

    for &file_count in &[10usize, 100, 500] {
        let (_dir_guard, corpus_path) = make_corpus(file_count, query);
        let searcher = make_searcher(thesaurus.clone(), corpus_path);
        let options = GrepOptions {
            haystack: Haystack::Code,
            max_results: 50,
            ..GrepOptions::default()
        };

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("files", file_count),
            &file_count,
            |b, _| {
                b.to_async(&rt)
                    .iter(|| async { searcher.search(query, &options).await.expect("search") });
            },
        );
    }

    group.finish();
}

fn bench_hybrid_with_kg(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");
    let mut group = c.benchmark_group("hybrid_with_kg");

    let query = "parse_grep_query";
    let file_count = 100usize;

    // Vary thesaurus size to measure the KG cost contribution to hybrid latency.
    for &term_count in &[10usize, 100, 1_000, 10_000] {
        let thesaurus = make_thesaurus(term_count, query);
        let (_dir_guard, corpus_path) = make_corpus(file_count, query);
        let searcher = make_searcher(thesaurus, corpus_path);
        let options = GrepOptions {
            haystack: Haystack::All,
            max_results: 50,
            ..GrepOptions::default()
        };

        group.throughput(Throughput::Elements(file_count as u64));
        group.bench_with_input(
            BenchmarkId::new("thesaurus_terms", term_count),
            &term_count,
            |b, _| {
                b.to_async(&rt)
                    .iter(|| async { searcher.search(query, &options).await.expect("search") });
            },
        );
    }

    group.finish();
}

fn bench_fuse_and_rank(c: &mut Criterion) {
    let mut group = c.benchmark_group("fuse_and_rank");

    let thesaurus = Thesaurus::new("rank".to_string());
    let (_dir_guard, corpus_path) = make_corpus(1, "noop");
    let searcher = make_searcher(thesaurus, corpus_path);

    for &chunk_count in &[10usize, 100, 1_000, 10_000] {
        let chunks: Vec<RetrievedChunk> = (0..chunk_count)
            .map(|i| RetrievedChunk {
                content: format!("line {i}"),
                source: format!("file_{i}.rs"),
                line_start: Some(i),
                line_end: Some(i),
                // Deliberately unsorted to exercise the comparator on every call.
                relevance_score: ((i * 37) % 1000) as f64 / 1000.0,
                haystack: "code",
            })
            .collect();

        group.throughput(Throughput::Elements(chunk_count as u64));
        group.bench_with_input(
            BenchmarkId::new("chunks", chunk_count),
            &chunk_count,
            |b, _| {
                b.iter(|| {
                    let fresh = chunks.clone();
                    let ranked = searcher.fuse_and_rank(fresh);
                    std::hint::black_box(ranked);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_code_only,
    bench_hybrid_with_kg,
    bench_fuse_and_rank
);
criterion_main!(benches);
