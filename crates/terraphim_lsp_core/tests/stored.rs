//! Stored alternatives (R-2.2, R-3.6, R-4.4): reading the span under a
//! cursor with each alternative's provenance, making a stored alternative
//! active, and the stored anchors that a plain rewrite must not touch.
//!
//! No mocks: every edit is applied and the result parsed back with
//! `terraphim_alternatives::parse`, and checked byte for byte against the
//! editor's own `Document::set_active` and `write`.

use terraphim_alternatives::{Document, parse, write};
use terraphim_lsp_core::{
    SelectAlternativeError, Source, SpanKind, TextRange, apply_edits, select_alternative,
    stored_anchor_ranges, stored_span_at,
};

/// A span over "paperclip" (original, human) and a ghost over "Drop this
/// aside."
const ANNOTATED: &str = include_str!("fixtures/annotated_doc.md");

fn block(json: &str) -> String {
    format!("```terraphim-alternatives\n{json}\n```\n")
}

/// `body` followed by a block holding one word span over the first `word`.
fn with_word_span(body: &str, word: &str, active: usize, alts: &str) -> String {
    let start = body.find(word).expect("word in body");
    let (s, e) = (
        body[..start].encode_utf16().count(),
        body[..start + word.len()].encode_utf16().count(),
    );
    let anchor_text = parse_alts(alts)[active].clone();
    let json = format!(
        r#"{{"version": 1, "spans": [{{"id": "s1", "kind": "word", "anchor": {{"start": {s}, "end": {e}, "text": {anchor_text:?}}}, "active": {active}, "alts": {alts}}}], "ghosts": []}}"#
    );
    format!("{body}\n\n{}", block(&json))
}

fn parse_alts(alts: &str) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_str(alts).unwrap();
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|alt| alt["text"].as_str().unwrap().to_string())
        .collect()
}

fn byte_of(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in text"))
}

/// What the editor itself writes for `set_active(id, index)`.
fn editor_select(text: &str, id: &str, index: usize) -> String {
    let mut document: Document = parse(text).unwrap();
    assert!(document.reanchor().unresolved.is_empty());
    document.set_active(id, index).unwrap();
    write(&document)
}

#[test]
fn the_span_under_the_cursor_lists_alternatives_with_provenance() {
    let at = byte_of(ANNOTATED, "paperclip") + 3;
    let span = stored_span_at(ANNOTATED, at).expect("span over paperclip");
    assert_eq!(span.span_id, "s1");
    assert_eq!(span.kind, SpanKind::Word);
    assert_eq!(&ANNOTATED[span.range.bytes()], "paperclip");
    assert_eq!(span.active, 0);
    let listed: Vec<(usize, &str, Source)> = span
        .alts
        .iter()
        .map(|alt| (alt.index, alt.text.as_str(), alt.source))
        .collect();
    assert_eq!(
        listed,
        [
            (0, "paperclip", Source::Original),
            (1, "binder clip", Source::Human)
        ]
    );
    // Both edges of the span count, as for a KG term.
    let start = byte_of(ANNOTATED, "paperclip");
    assert!(stored_span_at(ANNOTATED, start).is_some());
    assert!(stored_span_at(ANNOTATED, start + "paperclip".len()).is_some());
}

#[test]
fn no_span_off_the_spans_in_the_block_or_without_a_block() {
    assert!(stored_span_at(ANNOTATED, byte_of(ANNOTATED, "choice")).is_none());
    // The block itself is not body.
    assert!(stored_span_at(ANNOTATED, byte_of(ANNOTATED, "binder clip")).is_none());
    assert!(stored_span_at("Pass me a paperclip.", 12).is_none());
}

#[test]
fn an_ai_alternative_keeps_its_model() {
    let text = with_word_span(
        "Pass me a paperclip.",
        "paperclip",
        0,
        r#"[{"text": "paperclip", "source": "original"}, {"text": "fastener", "source": "ai", "model": "claude-test"}]"#,
    );
    let span = stored_span_at(&text, byte_of(&text, "paperclip")).unwrap();
    assert_eq!(span.alts[1].source, Source::Ai);
    assert_eq!(span.alts[1].model.as_deref(), Some("claude-test"));
    assert_eq!(span.alts[0].model, None);
}

#[test]
fn a_malformed_block_has_no_stored_spans_or_anchors() {
    let text = format!("Pass me a paperclip.\n\n{}", block("{ not json"));
    assert!(stored_span_at(&text, 12).is_none());
    assert!(stored_anchor_ranges(&text).is_empty());
    assert!(matches!(
        select_alternative(&text, "s1", 1),
        Err(SelectAlternativeError::MalformedBlock { .. })
    ));
}

#[test]
fn a_moved_span_is_found_where_its_text_now_is() {
    // Text inserted before the span: the stored offsets are a hint, and
    // re-anchoring finds "paperclip" again, as the editor does on open.
    let text = format!("Please, {ANNOTATED}");
    let span = stored_span_at(&text, byte_of(&text, "paperclip") + 1).expect("re-anchored span");
    assert_eq!(&text[span.range.bytes()], "paperclip");
}

#[test]
fn selecting_a_stored_alternative_matches_the_editor_byte_for_byte() {
    let selected = select_alternative(ANNOTATED, "s1", 1).unwrap();
    let saved = apply_edits(ANNOTATED, &selected.edits);
    assert_eq!(saved, editor_select(ANNOTATED, "s1", 1));
    let document = parse(&saved).unwrap();
    assert!(document.body.ends_with("Pass me a binder clip."));
    let span = &document.annotations.spans[0];
    assert_eq!(span.active, 1);
    assert_eq!(span.anchor.text, "binder clip");
    // Two edits: the body change, then the block rewrite.
    assert_eq!(selected.edits.len(), 2);
    assert_eq!(&ANNOTATED[selected.edits[0].range.bytes()], "paperclip");
    assert_eq!(selected.edits[0].new_text, "binder clip");
    assert!(selected.edits[0].range.end.byte <= selected.edits[1].range.start.byte);

    // And back to the original.
    let back = select_alternative(&saved, "s1", 0).unwrap();
    let restored = apply_edits(&saved, &back.edits);
    assert_eq!(restored, editor_select(&saved, "s1", 0));
    assert_eq!(
        parse(&restored).unwrap().body,
        parse(ANNOTATED).unwrap().body
    );
}

#[test]
fn the_article_fix_up_is_part_of_the_body_edit() {
    let text = with_word_span(
        "Pass me a paperclip.",
        "paperclip",
        0,
        r#"[{"text": "paperclip", "source": "original"}, {"text": "elastic band", "source": "human"}]"#,
    );
    let selected = select_alternative(&text, "s1", 1).unwrap();
    let saved = apply_edits(&text, &selected.edits);
    assert_eq!(saved, editor_select(&text, "s1", 1));
    assert!(saved.starts_with("Pass me an elastic band."));
    // One body edit covering the article and the span.
    assert_eq!(selected.edits.len(), 2);
    let body_edit = &selected.edits[0];
    assert!(body_edit.range.start.byte <= byte_of(&text, "a paperclip") + 1);
    assert!(body_edit.range.end.byte >= byte_of(&text, "paperclip") + "paperclip".len());
}

#[test]
fn a_multibyte_body_keeps_edits_on_character_boundaries() {
    let text = with_word_span(
        "Un café, señor.",
        "café",
        0,
        r#"[{"text": "café", "source": "original"}, {"text": "caféine", "source": "human"}]"#,
    );
    let selected = select_alternative(&text, "s1", 1).unwrap();
    let saved = apply_edits(&text, &selected.edits);
    assert_eq!(saved, editor_select(&text, "s1", 1));
    assert!(saved.starts_with("Un caféine, señor."));
}

#[test]
fn refused_selections_change_nothing() {
    assert!(matches!(
        select_alternative(ANNOTATED, "s1", 0),
        Err(SelectAlternativeError::AlreadyActive { index: 0, .. })
    ));
    assert!(matches!(
        select_alternative(ANNOTATED, "s1", 2),
        Err(SelectAlternativeError::InvalidIndex { index: 2, .. })
    ));
    assert!(matches!(
        select_alternative(ANNOTATED, "s9", 1),
        Err(SelectAlternativeError::UnknownSpan(id)) if id == "s9"
    ));
    assert!(matches!(
        select_alternative("Plain text.", "s1", 1),
        Err(SelectAlternativeError::UnknownSpan(_))
    ));
    // The span's text is gone from the body: the block cannot be rewritten
    // without dropping it.
    let stale = ANNOTATED.replacen("Pass me a paperclip.", "Pass me a stapler.", 1);
    assert!(matches!(
        select_alternative(&stale, "s1", 1),
        Err(SelectAlternativeError::UnresolvedAnchors(1))
    ));
}

#[test]
fn stored_anchors_are_the_spans_and_ghosts_in_body_order() {
    let ranges: Vec<&str> = stored_anchor_ranges(ANNOTATED)
        .iter()
        .map(|range: &TextRange| &ANNOTATED[range.bytes()])
        .collect();
    assert_eq!(ranges, ["Drop this aside.", "paperclip"]);
    assert!(stored_anchor_ranges("No block here.").is_empty());
    // An anchor that cannot be placed is not reported.
    let stale = ANNOTATED.replacen("Pass me a paperclip.", "Pass me a stapler.", 1);
    let ranges: Vec<&str> = stored_anchor_ranges(&stale)
        .iter()
        .map(|range| &stale[range.bytes()])
        .collect();
    assert_eq!(ranges, ["Drop this aside."]);
}

#[test]
fn an_alternative_with_the_current_text_changes_only_the_block() {
    // A hand-written block may hold the same text twice with different
    // provenance: selecting the other one changes `active`, not the body.
    let text = with_word_span(
        "Pass me a paperclip, then the stapler.",
        "paperclip",
        0,
        r#"[{"text": "paperclip", "source": "original"}, {"text": "paperclip", "source": "ai", "model": "m"}]"#,
    );
    let selected = select_alternative(&text, "s1", 1).unwrap();
    let saved = apply_edits(&text, &selected.edits);
    assert_eq!(saved, editor_select(&text, "s1", 1));
    assert_eq!(parse(&saved).unwrap().annotations.spans[0].active, 1);
    // The body edit, if any, stays within the span: it does not rewrite the
    // rest of the body.
    let body_end = parse(&text).unwrap().body.len();
    for edit in &selected.edits {
        if edit.range.start.byte < body_end {
            assert!(edit.range.end.byte <= byte_of(&text, "paperclip") + "paperclip".len());
            assert!(edit.range.start.byte >= byte_of(&text, "paperclip"));
        }
    }
}
