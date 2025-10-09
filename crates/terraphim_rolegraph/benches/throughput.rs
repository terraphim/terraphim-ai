//! Benchmarks for testing the throughput of the rolegraph.
//!
//! To run a single benchmark use:
//!
//! ```sh
//! cargo bench --bench throughput -- query
//! ```
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;

use terraphim_automata::load_thesaurus;
use terraphim_automata::matcher::find_matches;
use terraphim_automata::AutomataPath;
use terraphim_rolegraph::input::TEST_CORPUS;
use terraphim_rolegraph::split_paragraphs;
use terraphim_rolegraph::RoleGraph;
use terraphim_types::Document;
use terraphim_types::Thesaurus;

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
    let thesaurus = load_thesaurus(&AutomataPath::remote_example())
        .await
        .unwrap();
    let rolegraph = RoleGraph::new(role.into(), thesaurus).await;
    rolegraph.unwrap()
}

/// Loads a sample thesaurus
fn load_sample_thesaurus() -> Thesaurus {
    let thesaurus = block_on(load_thesaurus(&AutomataPath::remote_example()));
    thesaurus.unwrap()
}

/// A dummy document for testing the query function.
fn dummy_document(id: String, body: String) -> Document {
    Document {
        id,
        title: "Title".to_string(),
        url: "URL".to_string(),
        description: None,
        summarization: None,
        stub: None,
        rank: None,
        tags: None,
        body,
        source_haystack: None,
    }
}

fn bench_find_matching_node_idss(c: &mut Criterion) {
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let rolegraph = block_on(get_rolegraph());

    let sizes = &[1, 10, 100, 1000];
    for size in sizes {
        let input = body.repeat(*size);
        c.benchmark_group("find_matching_node_idss")
            .bench_with_input(
                BenchmarkId::new("find_matching_node_idss", size),
                size,
                |b, _| b.iter(|| rolegraph.find_matching_node_ids(&input)),
            );
    }
}

fn bench_find_matches(c: &mut Criterion) {
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";

    c.bench_function("find_matches", |b| {
        b.iter(|| find_matches(body, load_sample_thesaurus(), false))
    });
}

fn bench_split_paragraphs(c: &mut Criterion) {
    let paragraph = "This is the first sentence.\n\n This is the second sentence. This is the second sentence? This is the second sentence| This is the second sentence!\n\nThis is the third sentence. Mr. John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer. He also worked at craigslist.org as a business analyst.";
    c.bench_function("split_paragraphs", |b| {
        b.iter(|| split_paragraphs(paragraph))
    });
}

fn bench_parse_document_to_pair(c: &mut Criterion) {
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let id = "DocumentID4".to_string();
    let document = dummy_document(id.clone(), body.to_string());

    let mut rolegraph = block_on(get_rolegraph());
    c.bench_function("parse_document_to_pair", |b| {
        b.iter(|| rolegraph.insert_document(&id, document.clone()))
    });
}

/// Test throughput based on query string.
///
/// We want to measure if parsing from input query strings is fast.
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    let id = "DocumentID4".to_string();
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let document = dummy_document(id.clone(), body.to_string());

    let mut rolegraph = block_on(get_rolegraph());

    // for size in &[1000, 10000, 100000, 1000000, 10000000, 100000000] {
    for size in &[100, 1000, 2000, 3000, 4000, 5000, 10000] {
        let input = body.repeat(*size);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_document_to_pair", size),
            size,
            |b, _| b.iter(|| rolegraph.insert_document(&id, document.clone())),
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
    let id = "DocumentID4".to_string();
    let mut rolegraph = block_on(get_rolegraph());

    for input in TEST_CORPUS {
        let document = dummy_document(id.clone(), input.to_string());

        let size: usize = input.len();
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_document_to_pair", size),
            &document,
            |b, document| b.iter(|| rolegraph.insert_document(&id, document.clone())),
        );
    }
    group.finish();
}

fn bench_query_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("query throughput");
    let id = "DocumentID4".to_string();
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let document = dummy_document(id.clone(), body.to_string());

    let mut rolegraph = block_on(get_rolegraph());
    rolegraph.insert_document(&id, document);
    let query_term = "Life cycle concepts and project direction".to_string();

    for size in &[1, 10, 100, 1000] {
        group.throughput(Throughput::Bytes(query_term.len() as u64 * *size as u64));
        group.bench_with_input(BenchmarkId::new("query", size), size, |b, &size| {
            let query_term = query_term.repeat(size);
            b.iter(|| rolegraph.query_graph(&query_term, None, None))
        });
    }
    group.finish();
}

fn bench_query(c: &mut Criterion) {
    let mut rolegraph = block_on(get_rolegraph());

    let id = "DocumentID4".to_string();
    let body = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let document = dummy_document(id.clone(), body.to_string());

    rolegraph.insert_document(&id, document);
    let query_term = "Life cycle concepts and project direction".to_string();
    c.bench_function("query_response", |b| {
        b.iter(|| rolegraph.query_graph(&query_term, None, None))
    });
}

fn bench_is_all_terms_connected_by_path(c: &mut Criterion) {
    let rolegraph = block_on(get_rolegraph());
    let text = "Life cycle concepts ... Paradigm Map ... project planning";
    c.bench_function("is_all_terms_connected_by_path", |b| {
        b.iter(|| rolegraph.is_all_terms_connected_by_path(text))
    });
}

criterion_group!(
    benches,
    bench_find_matching_node_idss,
    bench_find_matches,
    bench_split_paragraphs,
    bench_parse_document_to_pair,
    bench_throughput,
    bench_throughput_corpus,
    bench_query_throughput,
    bench_query,
    bench_is_all_terms_connected_by_path
);
criterion_main!(benches);
