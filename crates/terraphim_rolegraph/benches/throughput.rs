//! Benchmarks for testing the throughput of the rolegraph.
//!
//! To run a single benchmark use:
//!
//! ```sh
//! cargo bench --bench throughput -- query
//! ```
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use terraphim_automata::load_thesaurus;
use terraphim_automata::matcher::find_matches;
use terraphim_rolegraph::input::TEST_CORPUS;
use terraphim_rolegraph::split_paragraphs;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::Thesaurus;

use tokio::runtime::Runtime;

lazy_static::lazy_static! {
    static ref TOKIO_RUNTIME: Runtime = Runtime::new().unwrap();
}

// We can use this `block_on` function to run async code in the benchmarks
// without having to use `async fn`, which is not supported by the `criterion` library.
#[inline]
pub fn block_on<F, T>(future: F) -> T
where
    F: std::future::Future<Output = T>,
{
    TOKIO_RUNTIME.block_on(future)
}

// Create a sample rolegraph for the benchmarks
async fn get_rolegraph() -> RoleGraph {
    let role = "system operator".to_string();
    let thesaurus =
        load_thesaurus("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json")
            .await
            .unwrap();
    let rolegraph = RoleGraph::new(role, thesaurus).await;
    rolegraph.unwrap()
}

/// Loads a sample thesaurus
fn load_sample_thesaurus() -> Thesaurus {
    let thesaurus = block_on(load_thesaurus(
        "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
    ));
    thesaurus.unwrap()
}

fn bench_find_matches_ids(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let rolegraph = block_on(get_rolegraph());

    let sizes = &[1, 10, 100, 1000];
    for size in sizes {
        let input = query.repeat(*size);
        c.benchmark_group("find_matches_ids").bench_with_input(
            BenchmarkId::new("find_matches_ids", size),
            size,
            |b, _| b.iter(|| rolegraph.find_matches_ids(&input)),
        );
    }
}

fn bench_find_matches(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";

    c.bench_function("find_matches", |b| {
        b.iter(|| find_matches(query, load_sample_thesaurus(), false))
    });
}

fn bench_split_paragraphs(c: &mut Criterion) {
    let paragraph = "This is the first sentence.\n\n This is the second sentence. This is the second sentence? This is the second sentence| This is the second sentence!\n\nThis is the third sentence. Mr. John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer. He also worked at craigslist.org as a business analyst.";
    c.bench_function("split_paragraphs", |b| {
        b.iter(|| split_paragraphs(paragraph))
    });
}

fn bench_parse_document_to_pair(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());
    c.bench_function("parse_document_to_pair", |b| {
        b.iter(|| rolegraph.parse_document_to_pair(&article_id4, query))
    });
}

/// Test throughput based on query string.
///
/// We want to measure if parsing from input query strings is fast.
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());

    // for size in &[1000, 10000, 100000, 1000000, 10000000, 100000000] {
    for size in &[100, 1000, 2000, 3000, 4000, 5000, 10000] {
        let input = query.repeat(*size);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_document_to_pair", size),
            size,
            |b, _| b.iter(|| rolegraph.parse_document_to_pair(&article_id4, &input)),
        );
    }
    group.finish();
}

/// A benchmark for measuring throughput by iterating over the set of corpuses
/// and parsing the documents. We only want to measure the throughput of the
/// parsing part.
///
/// Throughput is the most important measure for parsing.
fn bench_throughput_corpus(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_corpus");
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());

    for input in TEST_CORPUS {
        let input = input.to_string();
        let size: usize = input.len();
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_document_to_pair", size),
            &input,
            |b, input| b.iter(|| rolegraph.parse_document_to_pair(&article_id4, &input)),
        );
    }
    group.finish();
}

fn bench_query_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("query throughput");
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());
    rolegraph.parse_document_to_pair(&article_id4, query);
    let query_term = "Life cycle concepts and project direction".to_string();

    for size in &[1, 10, 100, 1000] {
        group.throughput(Throughput::Bytes(query_term.len() as u64 * *size as u64));
        group.bench_with_input(BenchmarkId::new("query", size), size, |b, &size| {
            let query_term = query_term.repeat(size);
            b.iter(|| rolegraph.query(&query_term, None, None))
        });
    }
    group.finish();
}

fn bench_query(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());
    rolegraph.parse_document_to_pair(&article_id4, query);
    let query_term = "Life cycle concepts and project direction".to_string();
    c.bench_function("query_response", |b| {
        b.iter(|| rolegraph.query(&query_term, None, None))
    });
}

criterion_group!(
    benches,
    bench_find_matches_ids,
    bench_find_matches,
    bench_split_paragraphs,
    bench_parse_document_to_pair,
    bench_throughput,
    bench_throughput_corpus,
    bench_query_throughput,
    bench_query
);
criterion_main!(benches);
