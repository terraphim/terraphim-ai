use criterion::{
    criterion_group, criterion_main, black_box, Bencher, BenchmarkGroup, Criterion, Throughput,
};
use criterion::BenchmarkId;


use itertools::Itertools;
use terraphim_automata::load_automata;
use terraphim_automata::matcher::{find_matches, find_matches_ids, replace_matches, Dictionary};
use terraphim_pipeline::split_paragraphs;
use terraphim_pipeline::input::TEST_CORPUS;
use terraphim_pipeline::{magic_pair, magic_unpair, RoleGraph};
use ulid::Ulid;
use lazy_static::lazy_static;
use ahash::{AHashMap, HashMap};
// use once_cell::sync::Lazy;

lazy_static! {
    static ref AUTOMATA: AHashMap<String, Dictionary> = {
        let dict_hash = load_automata("https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json").unwrap();
        dict_hash
    };
}

// static ROLEGRAPH: Lazy<RoleGraph> = Lazy::new(|| {
//     let role = "system operator".to_string();
//     let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
//     RoleGraph::new(role, automata_url)
// });

lazy_static! {
    static ref ROLEGRAPH: RoleGraph= {
        let role = "system operator".to_string();
        let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
        let rolegraph = RoleGraph::new(role, automata_url);
        rolegraph
    };
}

fn bench_find_matches_ids(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    
    c.bench_function_over_inputs(
        "find_matches_ids",
        move |b, &&size| {
            let query = query.repeat(size);
            b.iter(|| find_matches_ids(&query, &AUTOMATA).unwrap())
        },
        &[1, 10, 100, 1000],
    );
}
fn bench_find_matches(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    
    c.bench_function("find_matches", |b| {
        b.iter(|| find_matches(query, AUTOMATA.clone(), false))
    });
}
fn bench_split_paragraphs(c: &mut Criterion) {
    let paragraph = "This is the first sentence.\n\n This is the second sentence. This is the second sentence? This is the second sentence| This is the second sentence!\n\nThis is the third sentence. Mr. John Johnson Jr. was born in the U.S.A but earned his Ph.D. in Israel before joining Nike Inc. as an engineer. He also worked at craigslist.org as a business analyst.";
    c.bench_function("split_paragraphs", |b| {
        b.iter(|| split_paragraphs(paragraph))
    });
}
fn bench_replace_matches(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    
    c.bench_function("replace_matches", |b| {
        b.iter(|| replace_matches(query, AUTOMATA.clone()).unwrap())
    });
}
fn bench_parse_document_to_pair(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4= "ArticleID4".to_string();
    let mut rolegraph=ROLEGRAPH.clone();
    c.bench_function("parse_document_to_pair", |b| {
        b.iter(|| rolegraph.parse_document_to_pair(article_id4.clone(),query))
    });
}

fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4= "ArticleID4".to_string();
    let mut rolegraph=ROLEGRAPH.clone();
    // for size in &[1000, 10000, 100000, 1000000, 10000000, 100000000] {
    for size in &[100, 1000, 2000] {
        let input = query.repeat(*size);
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse_document_to_pair", size), size, |b, &size| {
            b.iter(|| rolegraph.parse_document_to_pair(article_id4.clone(), &input))
        });
    }
    group.finish();
}
fn bench_throughput_corpus(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    let article_id4= "ArticleID4".to_string();
    let mut rolegraph=ROLEGRAPH.clone();
    for size in TEST_CORPUS {
        let input = size.to_string();
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse_document_to_pair", size), size, |b, &size| {
            b.iter(|| rolegraph.parse_document_to_pair(article_id4.clone(), &input))
        });
    }
    group.finish();
}

fn bench_query_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("query throughput");
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = ROLEGRAPH.clone();
    rolegraph.parse_document_to_pair(article_id4.clone(), query);
    let query_term = "Life cycle concepts and project direction".to_string();
    for size in &[1, 10, 100, 1000] {
        group.throughput(Throughput::Bytes(query_term.len() as u64 * *size as u64));
        group.bench_with_input(BenchmarkId::new("query", size), size, |b, &size| {
            let query_term = query_term.repeat(size);
            b.iter(|| rolegraph.query(&query_term))
        });
    }
    group.finish();
}

fn bench_query(c: &mut Criterion) {
    let query = "I am a text with the word Life cycle concepts and bar and Trained operators and maintainers, project direction, some bingo words Paradigm Map and project planning, then again: some bingo words Paradigm Map and project planning, then repeats: Trained operators and maintainers, project direction";
    let article_id4 = "ArticleID4".to_string();
    let mut rolegraph = ROLEGRAPH.clone();
    rolegraph.parse_document_to_pair(article_id4.clone(), query);
    let query_term = "Life cycle concepts and project direction".to_string();
    c.bench_function("query_response", |b| {
        b.iter(|| rolegraph.query(&query_term))
    });
}

criterion_group!(benches, bench_find_matches_ids,bench_find_matches,bench_split_paragraphs,bench_replace_matches,bench_parse_document_to_pair,bench_throughput, bench_throughput_corpus, bench_query_throughput, bench_query);
criterion_main!(benches);