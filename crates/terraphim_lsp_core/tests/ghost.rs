//! Ghost and revive edits (R-5.1, R-5.2) against the editor's own parser.
//!
//! No mocks: every result is parsed back with `terraphim_alternatives::parse`
//! and the ghosts are rendered through `split_annotation_block`, the path the
//! LSP uses for its `ghosted` diagnostics.

use terraphim_alternatives::parse;
use terraphim_lsp_core::{
    GhostError, TextRange, apply_edits, ghost_availability, ghost_range, revive_range,
    split_annotation_block,
};

const BODY: &str = "Alpha beta gamma delta.";

/// A schema-v1 span over "beta" with a human alternative, to prove spans
/// survive ghosting.
const SPAN: &str = r#"{"id": "s1", "kind": "word", "anchor": {"start": 6, "end": 10, "text": "beta"}, "active": 0, "alts": [{"text": "beta", "source": "original"}, {"text": "bravo", "source": "human"}]}"#;

fn block(json: &str) -> String {
    format!("```terraphim-alternatives\n{json}\n```\n")
}

fn with_block(json: &str) -> String {
    format!("{BODY}\n\n{}", block(json))
}

fn range(text: &str, needle: &str) -> TextRange {
    let start = text.find(needle).expect("needle in text");
    TextRange::from_bytes(text, start, start + needle.len())
}

fn ghost(text: &str, needle: &str) -> String {
    let edited = ghost_range(text, range(text, needle)).expect("ghosted");
    apply_edits(text, std::slice::from_ref(&edited.edit))
}

fn revive(text: &str, needle: &str) -> String {
    let edited = revive_range(text, range(text, needle)).expect("revived");
    apply_edits(text, std::slice::from_ref(&edited.edit))
}

/// The ghosted body texts, as the diagnostics see them.
fn faded(text: &str) -> Vec<String> {
    // Reviving the last ghost leaves nothing to store, so the writer drops
    // the block: no block, no ghosts.
    split_annotation_block(text)
        .block
        .map(|block| block.ghost_diagnostics())
        .unwrap_or_default()
        .iter()
        .map(|diagnostic| text[diagnostic.range.bytes()].to_string())
        .collect()
}

fn body_of(text: &str) -> String {
    parse(text).unwrap().body
}

#[test]
fn ghosting_on_a_plain_document_creates_the_block() {
    let saved = ghost(BODY, "beta");
    assert!(saved.starts_with(&format!("{BODY}\n\n```terraphim-alternatives\n")));
    assert_eq!(body_of(&saved), BODY);
    assert_eq!(faded(&saved), ["beta"]);
}

#[test]
fn overlapping_and_touching_ghosts_merge() {
    let once = ghost(BODY, "Alpha beta");
    let overlapping = ghost(&once, "beta gamma");
    assert_eq!(faded(&overlapping), ["Alpha beta gamma"]);

    // "Alpha beta" and " gamma" touch at the space boundary.
    let touching = ghost(&once, " gamma");
    assert_eq!(faded(&touching), ["Alpha beta gamma"]);
    assert_eq!(parse(&touching).unwrap().annotations.ghosts.len(), 1);

    let apart = ghost(&once, "delta");
    assert_eq!(faded(&apart), ["Alpha beta", "delta"]);
}

#[test]
fn reviving_part_of_a_ghost_splits_or_trims_it() {
    let whole = ghost(BODY, "Alpha beta gamma delta");
    let split = revive(&whole, "beta");
    assert_eq!(faded(&split), ["Alpha ", " gamma delta"]);
    assert_eq!(body_of(&split), BODY);

    let trimmed = revive(&whole, "Alpha ");
    assert_eq!(faded(&trimmed), ["beta gamma delta"]);
}

#[test]
fn reviving_a_whole_ghost_removes_it() {
    let ghosted = ghost(BODY, "beta");
    let revived = revive(&ghosted, "beta");
    assert!(faded(&revived).is_empty());
    assert_eq!(revived, BODY, "an empty block is not written");
}

#[test]
fn a_cursor_revives_the_whole_ghost_it_is_in() {
    let ghosted = ghost(BODY, "beta gamma");
    let at = ghosted.find("gamma").unwrap() + 2;
    let cursor = TextRange::from_bytes(&ghosted, at, at);
    assert!(ghost_availability(&ghosted, cursor).can_revive);
    let edited = revive_range(&ghosted, cursor).unwrap();
    let saved = apply_edits(&ghosted, std::slice::from_ref(&edited.edit));
    assert!(faded(&saved).is_empty());
}

#[test]
fn a_ghost_over_a_word_with_alternatives_keeps_the_span() {
    let text = with_block(&format!(
        r#"{{"version": 1, "spans": [{SPAN}], "ghosts": []}}"#
    ));
    let ghosted = ghost(&text, "beta");
    let document = parse(&ghosted).unwrap();
    assert_eq!(document.annotations.spans.len(), 1);
    let span = &document.annotations.spans[0];
    assert_eq!(span.id, "s1");
    assert_eq!(span.alts[1].text, "bravo");
    assert_eq!(faded(&ghosted), ["beta"]);

    let revived = revive(&ghosted, "beta");
    assert_eq!(
        parse(&revived).unwrap().annotations.spans[0].alts[1].text,
        "bravo"
    );
}

#[test]
fn overflow_survives_ghost_and_revive() {
    let text = with_block(&format!(
        r#"{{"version": 1, "spans": [{SPAN}], "ghosts": [], "overflow": "an eraser"}}"#
    ));
    let ghosted = ghost(&text, "gamma");
    assert_eq!(parse(&ghosted).unwrap().annotations.overflow, "an eraser");
    let revived = revive(&ghosted, "gamma");
    assert_eq!(parse(&revived).unwrap().annotations.overflow, "an eraser");
}

#[test]
fn a_malformed_block_is_never_rewritten() {
    let text = format!("{BODY}\n\n{}", block(r#"{"version": 9, "spans": []}"#));
    assert!(matches!(
        ghost_range(&text, range(&text, "beta")),
        Err(GhostError::MalformedBlock { .. })
    ));
    assert!(matches!(
        revive_range(&text, range(&text, "beta")),
        Err(GhostError::MalformedBlock { .. })
    ));
    let availability = ghost_availability(&text, range(&text, "beta"));
    assert!(!availability.can_ghost && !availability.can_revive);
}

#[test]
fn the_edit_only_touches_the_block() {
    let text = with_block(&format!(
        r#"{{"version": 1, "spans": [{SPAN}], "ghosts": [], "overflow": "x"}}"#
    ));
    let body_len = parse(&text).unwrap().body.len();
    let edited = ghost_range(&text, range(&text, "gamma")).unwrap();
    assert_eq!(edited.edit.range.start.byte, body_len);
    assert_eq!(edited.edit.range.end.byte, text.len());
    let plain = ghost_range(BODY, range(BODY, "gamma")).unwrap();
    assert_eq!(plain.edit.range.start.byte, BODY.len());
    assert_eq!(plain.edit.range.end.byte, BODY.len());
    let saved = ghost(&text, "gamma");
    assert!(saved.starts_with(&text[..body_len]));
}

#[test]
fn availability_follows_the_selection() {
    let ghosted = ghost(BODY, "beta");
    let on = |needle: &str| ghost_availability(&ghosted, range(&ghosted, needle));
    // Wholly ghosted: nothing to ghost, something to revive.
    assert!(!on("beta").can_ghost && on("beta").can_revive);
    // Partly ghosted: both.
    assert!(on("Alpha beta").can_ghost && on("Alpha beta").can_revive);
    // Clear text: only ghost.
    assert!(on("gamma").can_ghost && !on("gamma").can_revive);
    // A cursor (empty range) outside a ghost offers neither.
    let at = ghosted.find("delta").unwrap();
    let cursor = ghost_availability(&ghosted, TextRange::from_bytes(&ghosted, at, at));
    assert!(!cursor.can_ghost && !cursor.can_revive);
    assert_eq!(
        ghost_range(&ghosted, range(&ghosted, "beta")),
        Err(GhostError::AlreadyGhosted)
    );
    assert_eq!(
        revive_range(&ghosted, range(&ghosted, "gamma")),
        Err(GhostError::NothingToRevive)
    );
}

#[test]
fn a_selection_running_into_the_block_is_cut_back_to_the_body() {
    let ghosted = ghost(BODY, "delta");
    let start = ghosted.find("gamma").unwrap();
    let selection = TextRange::from_bytes(&ghosted, start, ghosted.len());
    let edited = ghost_range(&ghosted, selection).unwrap();
    let saved = apply_edits(&ghosted, std::slice::from_ref(&edited.edit));
    assert_eq!(faded(&saved), ["gamma delta."]);
    assert_eq!(body_of(&saved), BODY);
}

#[test]
fn offsets_are_utf16_aware() {
    let body = "Caf\u{e9} \u{1F600} ok";
    let saved = ghost(body, "\u{1F600} ok");
    assert_eq!(faded(&saved), ["\u{1F600} ok"]);
}

/// A well-formed block whose ghost text is no longer in the body.
fn stale_ghost() -> String {
    with_block(
        r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 0, "end": 3, "text": "zzz"}}]}"#,
    )
}

/// A well-formed block whose span text is no longer in the body.
fn stale_span() -> String {
    with_block(
        r#"{"version": 1, "spans": [{"id": "s1", "kind": "word", "anchor": {"start": 0, "end": 3, "text": "zzz"}, "active": 0, "alts": [{"text": "zzz", "source": "original"}, {"text": "yyy", "source": "human"}]}], "ghosts": []}"#,
    )
}

#[test]
fn unresolvable_anchors_are_never_dropped() {
    for text in [stale_ghost(), stale_span()] {
        parse(&text).expect("the block itself is well formed");
        let selection = range(&text, "beta");
        assert!(matches!(
            ghost_range(&text, selection),
            Err(GhostError::UnresolvedAnchors(1))
        ));
        assert!(matches!(
            revive_range(&text, selection),
            Err(GhostError::UnresolvedAnchors(1))
        ));
        let availability = ghost_availability(&text, selection);
        assert!(!availability.can_ghost && !availability.can_revive);
    }
}

#[test]
fn revive_is_utf16_aware_after_astral_characters() {
    let body = "\u{1F600} Alpha \u{1F7E2} beta gamma";
    let ghosted = ghost(body, "\u{1F7E2} beta");
    assert_eq!(faded(&ghosted), ["\u{1F7E2} beta"]);
    let split = revive(&ghosted, "\u{1F7E2} ");
    assert_eq!(faded(&split), ["beta"]);
    assert_eq!(body_of(&split), body);
    let at = split.find("beta").unwrap() + 1;
    let cursor = TextRange::from_bytes(&split, at, at);
    let edited = revive_range(&split, cursor).unwrap();
    let saved = apply_edits(&split, std::slice::from_ref(&edited.edit));
    assert!(faded(&saved).is_empty());
}

#[test]
fn a_cursor_at_the_end_of_the_body_revives_a_trailing_ghost() {
    // The body ends where the block's separator begins, so this position is
    // the end of the last body word, not inside the block.
    let ghosted = ghost(BODY, "delta.");
    let end = parse(&ghosted).unwrap().body.len();
    let cursor = TextRange::from_bytes(&ghosted, end, end);
    assert!(ghost_availability(&ghosted, cursor).can_revive);
    // A cursor in the block itself offers nothing.
    let in_block = ghosted.find("terraphim-alternatives").unwrap();
    let cursor = TextRange::from_bytes(&ghosted, in_block, in_block);
    let availability = ghost_availability(&ghosted, cursor);
    assert!(!availability.can_ghost && !availability.can_revive);
    assert_eq!(
        revive_range(&ghosted, cursor),
        Err(GhostError::NothingToRevive)
    );
}
