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
