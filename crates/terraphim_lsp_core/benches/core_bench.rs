//! Benchmarks for the KG core: engine build, document analysis, the
//! alternatives lookup the editor runs on every cursor move, inlay-hint
//! data, the add-alternative block rewrite, and Lab marks and trim previews
//! (which the server runs only on open, save or command, never per
//! keystroke).
//!
//! Run with `cargo bench -p terraphim_lsp_core`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use terraphim_lsp_core::{
    KgEngine, LabAction, LabConfig, TextRange, TrimLevel, add_alternative, lab_findings,
    trim_preview,
};

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
    c.bench_function("synonym_positions_10kb", |b| {
        b.iter(|| engine.synonym_positions(black_box(&text)))
    });
    let range = TextRange::from_bytes(&text, cursor, cursor + "honour".len());
    c.bench_function("add_alternative_10kb", |b| {
        b.iter(|| add_alternative(black_box(&text), range, "distinction", None).unwrap())
    });
    let lab = LabConfig::with_defaults().expect("embedded Lab lists");
    c.bench_function("lab_findings_all_10kb", |b| {
        b.iter(|| lab_findings(black_box(&text), &lab, &LabAction::ALL))
    });
    c.bench_function("trim_preview_sharper_10kb", |b| {
        b.iter(|| trim_preview(black_box(&text), &lab, TrimLevel::Sharper))
    });
}

criterion_group!(benches, bench_core);
criterion_main!(benches);
