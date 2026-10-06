//! Ghost diagnostics, `[i/n]` inlay-hint data and the add-alternative edit,
//! against the real fixture thesaurus and a committed annotated fixture
//! document written by terraphim-editor's own writer.
//!
//! No mocks: blocks are produced and read back with
//! `terraphim_alternatives::{write, parse}`, the editor's span model.

use terraphim_alternatives::{Document, Source, SpanKind as EditorSpanKind, parse, write};
use terraphim_lsp_core::{
    AddAlternativeError, DiagnosticCode, DiagnosticTag, GHOSTED_MESSAGE, KgEngine, LineIndex,
    Severity, SpanKind, TextRange, add_alternative, add_alternative_utf16, apply_edits,
    split_annotation_block,
};

const THESAURUS_JSON: &str = include_str!("fixtures/writing_thesaurus.json");
/// Written by `annotated_document()` through `terraphim_alternatives::write`;
/// `committed_fixture_is_the_editor_writer_output` keeps the two in step.
const ANNOTATED_DOC: &str = include_str!("fixtures/annotated_doc.md");

const BODY: &str = "Every choice is a judgment.\nA café visit is an honour. Drop this aside.\nPass me a paperclip.";

fn engine() -> KgEngine {
    KgEngine::from_json(THESAURUS_JSON).expect("fixture thesaurus loads")
}

fn utf16_of(text: &str, needle: &str) -> (usize, usize) {
    let byte = text.find(needle).unwrap_or_else(|| panic!("{needle:?}"));
    let start = text[..byte].encode_utf16().count();
    (start, start + needle.encode_utf16().count())
}

/// The fixture: one span ("paperclip" with a human alternative) and one
/// ghost ("Drop this aside.").
fn annotated_document(body: &str) -> Document {
    let mut document = Document::new(body);
    let (start, end) = utf16_of(body, "paperclip");
    let span = document
        .add_span(EditorSpanKind::Word, start, end)
        .expect("span");
    document
        .add_alternative(&span, "binder clip", Source::Human, None)
        .expect("alternative");
    let (start, end) = utf16_of(body, "Drop this aside.");
    document.ghost(start, end).expect("ghost");
    document
}

fn range_of(text: &str, needle: &str) -> TextRange {
    let start = text.find(needle).unwrap_or_else(|| panic!("{needle:?}"));
    TextRange::from_bytes(text, start, start + needle.len())
}

#[test]
fn committed_fixture_is_the_editor_writer_output() {
    assert_eq!(ANNOTATED_DOC, write(&annotated_document(BODY)));
}

// ---------------------------------------------------------------- ghosts --

#[test]
fn ghosts_become_unnecessary_hints() {
    let block = split_annotation_block(ANNOTATED_DOC).block.expect("block");
    assert!(block.problem.is_none());
    let faded = block.ghost_diagnostics();
    assert_eq!(faded.len(), 1);
    let ghost = &faded[0];
    assert_eq!(&ANNOTATED_DOC[ghost.range.bytes()], "Drop this aside.");
    assert_eq!(ghost.severity, Severity::Hint);
    assert_eq!(ghost.code, DiagnosticCode::Ghosted);
    assert_eq!(ghost.tags, [DiagnosticTag::Unnecessary]);
    assert_eq!(ghost.message, GHOSTED_MESSAGE);
    // UTF-16 offsets account for the two-byte 'é' before the ghost.
    let (start, end) = utf16_of(ANNOTATED_DOC, "Drop this aside.");
    assert_eq!(
        (ghost.range.start.utf16, ghost.range.end.utf16),
        (start, end)
    );
}

#[test]
fn ghost_ranges_survive_crlf_line_endings() {
    let body = BODY.replace('\n', "\r\n");
    let text = write(&annotated_document(&body));
    let faded = split_annotation_block(&text)
        .block
        .unwrap()
        .ghost_diagnostics();
    assert_eq!(&text[faded[0].range.bytes()], "Drop this aside.");
    let position = LineIndex::new(&text).position(faded[0].range.start.byte);
    assert_eq!((position.line, position.character), (1, 27));
}

#[test]
fn a_ghost_moved_by_an_outside_edit_is_found_again() {
    // Text inserted at the start of the body shifts every stored offset.
    let moved = format!("Preface. {ANNOTATED_DOC}");
    let faded = split_annotation_block(&moved)
        .block
        .unwrap()
        .ghost_diagnostics();
    assert_eq!(faded.len(), 1);
    assert_eq!(&moved[faded[0].range.bytes()], "Drop this aside.");
}

#[test]
fn a_ghost_whose_text_is_gone_is_not_guessed_at() {
    let edited = ANNOTATED_DOC.replacen("Drop this aside.", "Something new.", 1);
    let block = split_annotation_block(&edited).block.unwrap();
    assert!(block.problem.is_none());
    assert!(block.ghost_diagnostics().is_empty());
}

#[test]
fn a_malformed_block_has_no_ghosts() {
    let broken = ANNOTATED_DOC.replacen("\"version\": 1", "\"version\": 99", 1);
    let block = split_annotation_block(&broken).block.unwrap();
    assert!(block.problem.is_some());
    assert!(block.ghosts.is_empty());
}

#[test]
fn analysis_is_unchanged_by_ghosts() {
    // Ghosts are offered separately; `Analysis::diagnostics` still holds
    // only a malformed-block problem.
    let analysis = engine().analyse(ANNOTATED_DOC);
    assert!(analysis.diagnostics.is_empty());
    assert_eq!(analysis.block.unwrap().ghosts.len(), 1);
}

// ----------------------------------------------------------- inlay hints --

#[test]
fn synonym_positions_follow_concept_index_order() {
    let hints = engine().synonym_positions(ANNOTATED_DOC);
    let labels: Vec<(&str, String)> = hints
        .iter()
        .map(|hint| (hint.term.text.as_str(), hint.label()))
        .collect();
    // decision: [decision, choice, judgment, option]; café: [café, coffee
    // shop, coffeehouse]; honour: [honour, privilege]. "paperclip" is a
    // single-term concept and gets no hint; nothing in the block is
    // analysed.
    assert_eq!(
        labels,
        [
            ("choice", "[2/4]".to_string()),
            ("judgment", "[3/4]".to_string()),
            ("café", "[1/3]".to_string()),
            ("honour", "[1/2]".to_string()),
        ]
    );
    let cafe = &hints[2];
    assert_eq!((cafe.index, cafe.count), (1, 3));
    let (start, end) = utf16_of(ANNOTATED_DOC, "café");
    assert_eq!(
        (cafe.term.range.start.utf16, cafe.term.range.end.utf16),
        (start, end)
    );
}

#[test]
fn synonym_positions_on_crlf_text() {
    let text = "a choice\r\nA Judgment";
    let hints = engine().synonym_positions(text);
    assert_eq!(hints.len(), 2);
    let index = LineIndex::new(text);
    let end = index.position(hints[1].term.range.end.byte);
    assert_eq!((end.line, end.character), (1, 10));
    assert_eq!(hints[1].label(), "[3/4]");
}

// ------------------------------------------------------- add alternative --

/// Apply an add-alternative edit and parse the result with the editor's
/// parser.
fn saved(text: &str, edit: &terraphim_lsp_core::TextEdit) -> (String, Document) {
    let out = apply_edits(text, std::slice::from_ref(edit));
    let document = parse(&out).expect("rewritten block parses");
    (out, document)
}

#[test]
fn adds_a_span_to_a_plain_document() {
    let text = "Every choice is a judgment.";
    let added = add_alternative(text, range_of(text, "judgment"), "call", None).unwrap();
    assert!(added.new_span);
    assert_eq!(added.index, 1);
    // The edit is an insertion at the end: the body is untouched.
    assert_eq!(added.edit.range.start.byte, text.len());
    assert_eq!(added.edit.range.end.byte, text.len());
    let (out, document) = saved(text, &added.edit);
    assert_eq!(document.body, text);
    let span = document.span(&added.span_id).unwrap();
    assert_eq!(span.kind, EditorSpanKind::Word);
    assert_eq!(span.anchor.text, "judgment");
    assert_eq!(span.alts[1].text, "call");
    assert_eq!(span.alts[1].source, Source::Human);
    assert_eq!(span.active, 0);
    // Byte for byte what the editor writes for the same document.
    assert_eq!(out, write(&document));
}

#[test]
fn adds_to_the_existing_span_and_keeps_the_ghost() {
    let text = ANNOTATED_DOC;
    let added = add_alternative(text, range_of(text, "paperclip"), "staple", None).unwrap();
    assert!(!added.new_span);
    assert_eq!(added.index, 2);
    // The edit replaces exactly the old block.
    let body_end = split_annotation_block(text).body_end.byte;
    assert_eq!(added.edit.range.start.byte, body_end);
    assert_eq!(added.edit.range.end.byte, text.len());
    let (_, document) = saved(text, &added.edit);
    let before = parse(text).unwrap();
    assert_eq!(document.body, before.body);
    assert_eq!(document.annotations.ghosts, before.annotations.ghosts);
    let texts: Vec<&str> = document.annotations.spans[0]
        .alts
        .iter()
        .map(|alt| alt.text.as_str())
        .collect();
    assert_eq!(texts, ["paperclip", "binder clip", "staple"]);
}

#[test]
fn a_new_span_beside_existing_ones_round_trips() {
    let text = ANNOTATED_DOC;
    let added = add_alternative(
        text,
        range_of(text, "A café visit is an honour."),
        "A café visit is a privilege.",
        None,
    )
    .unwrap();
    assert!(added.new_span);
    let (out, document) = saved(text, &added.edit);
    assert_eq!(document.annotations.spans.len(), 2);
    let span = document.span(&added.span_id).unwrap();
    assert_eq!(span.kind, EditorSpanKind::Sentence);
    let (start, end) = utf16_of(&out, "A café visit is an honour.");
    assert_eq!((span.anchor.start, span.anchor.end), (start, end));
    // Re-analysing the rewritten file still excludes the block.
    let analysis = engine().analyse(&out);
    assert!(analysis.diagnostics.is_empty());
    assert!(
        analysis
            .matches
            .iter()
            .all(|m| m.range.end.byte <= analysis.body_end.byte)
    );
}

#[test]
fn crlf_bodies_are_kept_byte_for_byte() {
    let body = BODY.replace('\n', "\r\n");
    let text = write(&annotated_document(&body));
    let added = add_alternative(&text, range_of(&text, "honour"), "privilege", None).unwrap();
    let (_, document) = saved(&text, &added.edit);
    assert_eq!(document.body, body);
    let span = document.span(&added.span_id).unwrap();
    let (start, end) = utf16_of(&body, "honour");
    assert_eq!((span.anchor.start, span.anchor.end), (start, end));
}

#[test]
fn explicit_kind_and_utf16_entry_point() {
    let text = "A café visit.";
    let (start, end) = utf16_of(text, "café");
    let added =
        add_alternative_utf16(text, start, end, "coffee shop", Some(SpanKind::Paragraph)).unwrap();
    let (_, document) = saved(text, &added.edit);
    assert_eq!(
        document.annotations.spans[0].kind,
        EditorSpanKind::Paragraph
    );
    // An offset inside a surrogate pair is rejected, not rounded.
    let astral = "a \u{1F600} b";
    assert_eq!(
        add_alternative_utf16(astral, 2, 3, "x", None),
        Err(AddAlternativeError::OutsideBody)
    );
}

#[test]
fn refusals_leave_the_document_alone() {
    let text = ANNOTATED_DOC;
    let body_end = split_annotation_block(text).body_end.byte;
    // Range inside the block, or empty.
    let in_block = TextRange::from_bytes(text, body_end + 5, body_end + 9);
    assert_eq!(
        add_alternative(text, in_block, "x", None),
        Err(AddAlternativeError::OutsideBody)
    );
    let empty = TextRange::from_bytes(text, 3, 3);
    assert_eq!(
        add_alternative(text, empty, "x", None),
        Err(AddAlternativeError::OutsideBody)
    );
    let paperclip = range_of(text, "paperclip");
    assert_eq!(
        add_alternative(text, paperclip, "", None),
        Err(AddAlternativeError::EmptyText)
    );
    assert_eq!(
        add_alternative(text, paperclip, "paperclip", None),
        Err(AddAlternativeError::SameAsCurrent)
    );
    assert_eq!(
        add_alternative(text, paperclip, "binder clip", None),
        Err(AddAlternativeError::AlreadyPresent {
            span_id: "s1".to_string(),
            index: 1
        })
    );
    // Overlapping the span without matching it exactly.
    assert_eq!(
        add_alternative(text, range_of(text, "a paperclip"), "a clip", None),
        Err(AddAlternativeError::Overlap("s1".to_string()))
    );
}

#[test]
fn a_malformed_block_is_never_rewritten() {
    let broken = ANNOTATED_DOC.replacen("\"version\": 1", "\"version\": 99", 1);
    match add_alternative(&broken, range_of(&broken, "choice"), "option", None) {
        Err(AddAlternativeError::MalformedBlock { code, .. }) => {
            assert_eq!(code, DiagnosticCode::AnnotationBlockUnknownVersion);
        }
        other => panic!("expected a malformed-block refusal, got {other:?}"),
    }
}

#[test]
fn unresolvable_anchors_refuse_rather_than_drop_them() {
    // The span's text is gone from the body: rewriting would lose it.
    let edited = ANNOTATED_DOC.replacen("paperclip.", "stapler.", 1);
    assert_eq!(
        add_alternative(&edited, range_of(&edited, "choice"), "option", None),
        Err(AddAlternativeError::UnresolvedAnchors(1))
    );
}
