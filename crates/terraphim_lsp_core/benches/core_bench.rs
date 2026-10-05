//! Benchmarks for the KG core: engine build, document analysis and the
//! alternatives lookup the editor runs on every cursor move.
//!
//! Run with `cargo bench -p terraphim_lsp_core`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use terraphim_lsp_core::KgEngine;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
const PARAGRAPH: &str = "Every choice is a judgment. A choice made in a café is an honour, \
    and an eraser or a rubber is a paperclip's best friend when you write Rust. ";

/// About 10 KB of prose with a trailing annotation block.
fn document() -> String {
    let mut text = PARAGRAPH.repeat(64);
    text.push_str("\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": []}\n```\n");
    text
}

fn bench_core(c: &mut Criterion) {
    let text = document();
    let engine = KgEngine::from_json(THESAURUS_JSON).expect("fixture loads");
    let cursor = text.rfind("honour").expect("term present");

    c.bench_function("engine_from_json", |b| {
        b.iter(|| KgEngine::from_json(black_box(THESAURUS_JSON)).unwrap())
    });
    c.bench_function("analyse_10kb", |b| {
        b.iter(|| engine.analyse(black_box(&text)))
    });
    c.bench_function("alternatives_at_10kb", |b| {
        b.iter(|| engine.alternatives_at(black_box(&text), black_box(cursor)))
    });
}

criterion_group!(benches, bench_core);
criterion_main!(benches);
