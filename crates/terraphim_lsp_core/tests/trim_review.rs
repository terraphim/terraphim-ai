//! The trim review (R-8.4, R-8.5) positioned in the full text: the view
//! after keeps, and "Make the cuts" as full-text edits. No mocks: the real
//! Lab engine with its embedded lists, on committed fixtures.

use terraphim_lsp_core::{
    LabConfig, TrimLevel, apply_edits, split_annotation_block, trim_cuts, trim_plan_for,
    trim_preview, trim_view,
};

const LAB_DOC: &str = include_str!("fixtures/lab_doc.md");

fn config() -> LabConfig {
    LabConfig::with_defaults().expect("embedded lists")
}

#[test]
fn the_view_without_keeps_matches_the_preview() {
    let plan = trim_plan_for(LAB_DOC, &config());
    for level in TrimLevel::ALL {
        let view = trim_view(LAB_DOC, &plan, level, &[]);
        let preview = trim_preview(LAB_DOC, &config(), level);
        assert_eq!(view.diagnostics, preview.diagnostics, "{level:?}");
        assert_eq!(view.status.card_text(), preview.status, "{level:?}");
        assert_eq!(view.spans.len(), view.diagnostics.len());
    }
}

#[test]
fn spans_and_cuts_are_positioned_in_the_body_only() {
    let plan = trim_plan_for(LAB_DOC, &config());
    let view = trim_view(LAB_DOC, &plan, TrimLevel::Half, &[]);
    let body_end = split_annotation_block(LAB_DOC).body_end.byte;
    assert!(!view.cuts.is_empty());
    for cut in &view.cuts {
        assert!(cut.range.end.byte <= body_end, "{cut:?}");
        assert!(!LAB_DOC[cut.range.bytes()].trim().is_empty(), "{cut:?}");
    }
    assert_eq!(&LAB_DOC[view.spans[0].range.bytes()], " basically");
}

#[test]
fn keeping_a_cut_unfades_it_and_lowers_the_count() {
    let plan = trim_plan_for(LAB_DOC, &config());
    let before = trim_view(LAB_DOC, &plan, TrimLevel::Slight, &[]);
    let basically = before.spans[0].id;
    let after = trim_view(LAB_DOC, &plan, TrimLevel::Slight, &[basically]);
    assert_eq!(after.spans.len(), before.spans.len() - 1);
    assert!(after.spans.iter().all(|span| span.id != basically));
    assert_eq!(after.status.words_after, before.status.words_after + 1);
}

#[test]
fn keeping_a_filler_inside_a_faded_sentence_splits_the_sentence() {
    let text = include_str!("fixtures/trim_review_doc.md");
    let plan = trim_plan_for(text, &config());
    let view = trim_view(text, &plan, TrimLevel::Half, &[]);
    // A word-tier cut nested inside a longer faded span.
    let (inner, outer) = view
        .cuts
        .iter()
        .find_map(|inner| {
            let outer = view.spans.iter().find(|span| {
                span.id != inner.id
                    && span.range.start.byte <= inner.range.start.byte
                    && inner.range.end.byte <= span.range.end.byte
            })?;
            Some((inner.clone(), outer.clone()))
        })
        .expect("a nested cut in the fixture");
    let kept = trim_view(text, &plan, TrimLevel::Half, &[inner.id]);
    // The kept words survive; the rest of the enclosing span is still cut.
    assert!(kept.cuts.iter().all(|cut| {
        cut.range.end.byte <= inner.range.start.byte || cut.range.start.byte >= inner.range.end.byte
    }));
    assert!(
        kept.cuts
            .iter()
            .any(|cut| { cut.id == outer.id && cut.range.bytes() != outer.range.bytes() })
    );
    // Keeping the enclosing span keeps the nested one too.
    let outer_kept = trim_view(text, &plan, TrimLevel::Half, &[outer.id]);
    assert!(outer_kept.cuts.iter().all(|cut| {
        cut.range.end.byte <= outer.range.start.byte || cut.range.start.byte >= outer.range.end.byte
    }));
}

#[test]
fn make_the_cuts_edits_equal_the_engine_and_keep_the_block() {
    let config = config();
    let plan = trim_plan_for(LAB_DOC, &config);
    for level in [TrimLevel::Slight, TrimLevel::Half] {
        let view = trim_view(LAB_DOC, &plan, level, &[]);
        let made = trim_cuts(LAB_DOC, &plan, level, &[]);
        let applied = apply_edits(LAB_DOC, &made.edits);
        assert_eq!(applied, made.text, "{level:?}");
        let body_end = split_annotation_block(LAB_DOC).body_end.byte;
        assert!(made.text.ends_with(&LAB_DOC[body_end..]), "block untouched");
        let after = trim_plan_for(&made.text, &config);
        assert_eq!(after.total_words(), view.status.words_after, "{level:?}");
    }
    let none = trim_cuts(LAB_DOC, &plan, TrimLevel::Original, &[]);
    assert!(none.edits.is_empty());
    assert_eq!(none.text, LAB_DOC);
}

#[test]
fn done_persists_exact_utf16_fades_without_body_or_overflow_rewrites() {
    use terraphim_lsp_core::trim_done;
    let body = format!(
        "😀 Café.\r\n{}",
        include_str!("fixtures/trim_review_doc.md")
    );
    let mut doc = terraphim_alternatives::Document::new(&body);
    doc.annotations.overflow = "Keep my notes 😀".to_string();
    let text = terraphim_alternatives::write(&doc);
    let plan = trim_plan_for(&text, &config());
    let view = trim_view(&text, &plan, TrimLevel::Sharper, &[]);
    assert!(!view.spans.is_empty());
    let promoted = trim_done(&text, &view).unwrap();
    let saved = apply_edits(&text, &[promoted.edit]);
    let reopened = terraphim_alternatives::parse(&saved).unwrap();
    assert_eq!(reopened.body, body);
    assert_eq!(reopened.annotations.overflow, doc.annotations.overflow);
    assert_eq!(reopened.annotations.ghosts.len(), view.spans.len());
    for (ghost, span) in reopened.annotations.ghosts.iter().zip(&view.spans) {
        assert_eq!(ghost.anchor.text, body[span.range.bytes()]);
        assert_eq!(
            ghost.anchor.start,
            body[..span.range.start.byte].encode_utf16().count()
        );
        assert_eq!(
            ghost.anchor.end,
            body[..span.range.end.byte].encode_utf16().count()
        );
    }
}

#[test]
fn done_refuses_manual_overlap_and_crossing_cuts_atomically() {
    use terraphim_lsp_core::{TextRange, trim_done};
    let text = include_str!("fixtures/trim_review_doc.md");
    let plan = trim_plan_for(text, &config());
    let mut view = trim_view(text, &plan, TrimLevel::Sharper, &[]);
    let first = view.spans[0].clone();
    let mut document = terraphim_alternatives::Document::new(text);
    document
        .ghost(first.range.start.utf16, first.range.end.utf16)
        .unwrap();
    let saved = terraphim_alternatives::write(&document);
    assert!(trim_done(&saved, &view).is_err());
    assert_eq!(terraphim_alternatives::parse(&saved).unwrap(), document);
    let mut crossing = first.clone();
    crossing.range =
        TextRange::from_bytes(text, first.range.start.byte + 1, first.range.end.byte + 1);
    view.spans.push(crossing);
    assert!(trim_done(text, &view).is_err());
}

#[test]
fn done_refuses_crossings_hidden_by_an_enclosing_cut() {
    use terraphim_lsp_core::{TextRange, trim_done};
    let text = include_str!("fixtures/trim_review_doc.md");
    let plan = trim_plan_for(text, &config());
    let mut view = trim_view(text, &plan, TrimLevel::Sharper, &[]);
    let template = view.cuts[0].clone();
    view.cuts = [(0, 20), (2, 10), (5, 15)]
        .into_iter()
        .map(|(a, b)| {
            let mut cut = template.clone();
            cut.range = TextRange::from_bytes(text, a, b);
            cut
        })
        .collect();
    view.spans = vec![view.cuts[0].clone()];
    assert!(trim_done(text, &view).is_err());
}
