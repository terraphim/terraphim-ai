//! Unknown-term analysis on a large document against the real fixture
//! thesaurus: the scan is linear in words plus matches (#3437 review).

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use terraphim_lsp::core::KgEngine;
use terraphim_lsp::kg_analysis::analyse_kg_document;

const THESAURUS_JSON: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json");

fn large_document(words: usize) -> String {
    let sentence = "We made a choice to use the rubber in the coffee shop with an LLM today. ";
    let per_sentence = sentence.split_whitespace().count();
    sentence.repeat(words.div_ceil(per_sentence))
}

fn bench_unknown_terms(c: &mut Criterion) {
    let engine = KgEngine::from_json(THESAURUS_JSON).expect("fixture thesaurus");
    for words in [1_000, 10_000, 50_000] {
        let text = large_document(words);
        c.bench_function(&format!("analyse_kg_document/{words}_words"), |b| {
            b.iter(|| analyse_kg_document(black_box(&text), &engine));
        });
    }
}

criterion_group!(benches, bench_unknown_terms);
criterion_main!(benches);
