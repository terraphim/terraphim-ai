//! Overflow stash and pull back (R-6.3 to R-6.5), clean export (R-9.3),
//! block re-anchoring (R-9.2) and block folding, against the editor's own
//! parser.
//!
//! No mocks: every result is parsed back with `terraphim_alternatives::parse`.

use terraphim_alternatives::parse;
use terraphim_lsp_core::{
    ExportError, OverflowError, ReanchorError, ReanchorOutcome, SpanKind, TextRange,
    add_alternative, apply_edits, export_clean, ghost_range, overflow_paragraphs, pull_back,
    reanchor_block, revive_range, split_annotation_block, stash_range,
};

const BODY: &str = "Alpha beta gamma delta.\nSecond line here.";

fn block(json: &str) -> String {
    format!("```terraphim-alternatives\n{json}\n```\n")
}

fn with(body: &str, json: &str) -> String {
    format!("{body}\n\n{}", block(json))
}

fn range(text: &str, needle: &str) -> TextRange {
    let start = text.find(needle).expect("needle in text");
    TextRange::from_bytes(text, start, start + needle.len())
}

fn overflow_of(text: &str) -> String {
    parse(text).expect("parses").annotations.overflow
}

fn stash(text: &str, needle: &str) -> String {
    let edited = stash_range(text, range(text, needle)).expect("stashed");
    apply_edits(text, &edited.edits)
}

const BETA_SPAN: &str = r#"{"id": "s1", "kind": "word", "anchor": {"start": 6, "end": 10, "text": "beta"}, "active": 0, "alts": [{"text": "beta", "source": "original"}, {"text": "bravo", "source": "human"}]}"#;

fn overflow_json(overflow: &str) -> String {
    format!(
        r#"{{"version": 1, "spans": [], "ghosts": [], "overflow": {}}}"#,
        serde_json::to_string(overflow).unwrap()
    )
}

// ---- stash ----

#[test]
fn stash_moves_text_to_a_new_overflow() {
    let text = "Keep this. Move this. And keep.";
    let saved = stash(text, "Move this. ");
    let document = parse(&saved).unwrap();
    assert_eq!(document.body, "Keep this. And keep.");
    assert_eq!(document.annotations.overflow, "Move this. ");
}

#[test]
fn stash_appends_after_a_blank_line() {
    let text = with("One. Two. Three.", &overflow_json("Earlier note."));
    let saved = stash(&text, "Two. ");
    assert_eq!(overflow_of(&saved), "Earlier note.\n\nTwo. ");
    let again = stash(&saved, "Three.");
    assert_eq!(overflow_of(&again), "Earlier note.\n\nTwo. \n\nThree.");
    assert_eq!(parse(&again).unwrap().body, "One.");
}

#[test]
fn stash_of_a_whole_line_takes_its_line_break() {
    let saved = stash(BODY, "Alpha beta gamma delta.");
    assert_eq!(parse(&saved).unwrap().body, "Second line here.");
    assert_eq!(overflow_of(&saved), "Alpha beta gamma delta.");
}

#[test]
fn stash_trims_surrounding_line_breaks_of_the_selection() {
    let text = "A.\n\nMove me.\n\nB.";
    let start = text.find("\nMove me.\n").unwrap();
    let selection = TextRange::from_bytes(text, start, start + "\nMove me.\n".len());
    let edited = stash_range(text, selection).unwrap();
    assert_eq!(overflow_of(&apply_edits(text, &edited.edits)), "Move me.");
}

#[test]
fn stash_shifts_later_spans_exactly() {
    let text = with(
        BODY,
        &format!(r#"{{"version": 1, "spans": [{BETA_SPAN}]}}"#),
    );
    let saved = stash(&text, "Second line here.");
    let document = parse(&saved).unwrap();
    assert_eq!(document.body, "Alpha beta gamma delta.");
    assert_eq!(document.annotations.spans[0].anchor.text, "beta");
    // A stash before the span moves it.
    let before = stash(&text, "Alpha ");
    let moved = parse(&before).unwrap();
    assert_eq!(moved.annotations.spans[0].anchor.start, 0);
    assert_eq!(moved.annotations.spans[0].anchor.text, "beta");
}

#[test]
fn stash_touching_a_span_is_refused_and_changes_nothing() {
    let text = with(
        BODY,
        &format!(r#"{{"version": 1, "spans": [{BETA_SPAN}]}}"#),
    );
    let error = stash_range(&text, range(&text, "a beta g")).unwrap_err();
    assert_eq!(error, OverflowError::TouchesSpan);
}

#[test]
fn stash_of_a_ghost_drops_the_ghost_with_the_text() {
    let ghosted = ghost_range(BODY, range(BODY, "gamma")).unwrap();
    let text = apply_edits(BODY, std::slice::from_ref(&ghosted.edit));
    let saved = stash(&text, "gamma ");
    let document = parse(&saved).unwrap();
    assert!(document.annotations.ghosts.is_empty());
    assert_eq!(document.annotations.overflow, "gamma ");
}

#[test]
fn stash_at_the_end_of_the_body_merges_the_edits() {
    let text = with("Head. Tail.", &overflow_json("Old."));
    let edited = stash_range(&text, range(&text, "Tail.")).unwrap();
    assert_eq!(edited.edits.len(), 1);
    let saved = apply_edits(&text, &edited.edits);
    assert_eq!(parse(&saved).unwrap().body, "Head.");
    assert_eq!(overflow_of(&saved), "Old.\n\nTail.");
}

#[test]
fn stash_in_the_middle_gives_two_ordered_edits() {
    let text = with(BODY, &overflow_json("Old."));
    let edited = stash_range(&text, range(&text, "beta ")).unwrap();
    assert_eq!(edited.edits.len(), 2);
    assert!(edited.edits[0].range.end <= edited.edits[1].range.start);
}

#[test]
fn stash_refuses_empty_outside_malformed_and_unresolved() {
    assert_eq!(
        stash_range(BODY, TextRange::from_bytes(BODY, 3, 3)).unwrap_err(),
        OverflowError::OutsideBody
    );
    let only_breaks = "A.\n\n\nB.";
    assert_eq!(
        stash_range(only_breaks, TextRange::from_bytes(only_breaks, 2, 5)).unwrap_err(),
        OverflowError::OutsideBody
    );
    let malformed = format!("{BODY}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    assert!(matches!(
        stash_range(&malformed, range(&malformed, "beta")).unwrap_err(),
        OverflowError::MalformedBlock { .. }
    ));
    let lost = with(
        "Totally different.",
        r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 0, "end": 4, "text": "gone"}}]}"#,
    );
    assert_eq!(
        stash_range(&lost, range(&lost, "different")).unwrap_err(),
        OverflowError::UnresolvedAnchors(1)
    );
}

#[test]
fn stash_handles_astral_and_accented_text() {
    let text = "Café 🙂 naïve text. Move 🙂 this.";
    let saved = stash(text, "Move 🙂 this.");
    assert_eq!(parse(&saved).unwrap().body, "Café 🙂 naïve text.");
    assert_eq!(overflow_of(&saved), "Move 🙂 this.");
}

// ---- paragraphs and pull back ----

#[test]
fn paragraphs_are_runs_of_non_blank_lines() {
    let text = with(
        "Body.",
        &overflow_json("One.\n\nTwo\nlines.\n\n\n  \nThree."),
    );
    let texts: Vec<String> = overflow_paragraphs(&text)
        .into_iter()
        .map(|p| p.text)
        .collect();
    assert_eq!(texts, ["One.", "Two\nlines.", "Three."]);
    assert!(overflow_paragraphs("No block.").is_empty());
    assert!(overflow_paragraphs(&with("B.", &overflow_json(""))).is_empty());
}

#[test]
fn pull_back_copies_a_paragraph_into_the_body() {
    let text = with("Start end.", &overflow_json("One.\n\nTwo.\n\nThree."));
    let pulled = pull_back(&text, 1, "Start".len()).unwrap();
    let saved = apply_edits(&text, &pulled.edits);
    assert_eq!(parse(&saved).unwrap().body, "StartTwo. end.");
    assert_eq!(
        overflow_of(&saved),
        "One.\n\nTwo.\n\nThree.",
        "a copy: the overflow keeps every paragraph"
    );
}

#[test]
fn pull_back_of_any_paragraph_leaves_the_overflow_alone() {
    let text = with("Body.", &overflow_json("One.\n\nTwo.\n\nThree."));
    let first = apply_edits(&text, &pull_back(&text, 0, 0).unwrap().edits);
    assert_eq!(overflow_of(&first), "One.\n\nTwo.\n\nThree.");
    assert!(parse(&first).unwrap().body.starts_with("One."));
    let last = apply_edits(&text, &pull_back(&text, 2, 0).unwrap().edits);
    assert_eq!(overflow_of(&last), "One.\n\nTwo.\n\nThree.");
    assert!(parse(&last).unwrap().body.starts_with("Three."));
    let only = with("Body.", &overflow_json("Only."));
    let kept = apply_edits(&only, &pull_back(&only, 0, 0).unwrap().edits);
    assert_eq!(parse(&kept).unwrap().annotations.overflow, "Only.");
    // The same paragraph can be used again.
    let again = apply_edits(&kept, &pull_back(&kept, 0, 0).unwrap().edits);
    assert_eq!(parse(&again).unwrap().body, "Only.\n\nOnly.\n\nBody.");
    assert_eq!(parse(&again).unwrap().annotations.overflow, "Only.");
}

#[test]
fn pull_back_without_spans_or_ghosts_leaves_the_block_untouched() {
    // Only the body edit: the block text is byte for byte what it was.
    let text = with("Body.", &overflow_json("More."));
    let edited = pull_back(&text, 0, "Body.".len()).unwrap();
    assert_eq!(edited.edits.len(), 1);
    assert!(
        edited.edits[0].range.end.byte <= "Body.".len(),
        "before the block"
    );
    let saved = apply_edits(&text, &edited.edits);
    assert_eq!(
        &saved[saved.find("\n\n```").unwrap()..],
        &text[text.find("\n\n```").unwrap()..]
    );
    assert_eq!(
        parse(&apply_edits(&text, &edited.edits)).unwrap().body,
        "Body.\n\nMore."
    );
}

#[test]
fn pull_back_errors() {
    let text = with("Body.", &overflow_json("One."));
    assert_eq!(
        pull_back(&text, 3, 0).unwrap_err(),
        OverflowError::NoSuchParagraph(3)
    );
    assert_eq!(
        pull_back(&text, 0, 99).unwrap_err(),
        OverflowError::OutsideBody
    );
    let with_span = with(
        BODY,
        &format!(r#"{{"version": 1, "spans": [{BETA_SPAN}], "overflow": "Note."}}"#),
    );
    // Inside the span "beta".
    assert_eq!(
        pull_back(&with_span, 0, 8).unwrap_err(),
        OverflowError::TouchesSpan
    );
}

#[test]
fn stash_then_pull_back_restores_the_body() {
    let text = "Keep this. Move this. And keep.";
    let stashed = stash(text, "Move this. ");
    let at = parse(&stashed).unwrap().body.find("And").unwrap();
    let back = apply_edits(&stashed, &pull_back(&stashed, 0, at).unwrap().edits);
    assert_eq!(
        parse(&back).unwrap().body,
        "Keep this. Move this. And keep."
    );
    assert_eq!(overflow_of(&back), "Move this. ", "still stashed");
}

// ---- R-6.5: overflow survives every block rewrite ----

#[test]
fn overflow_survives_a_block_rewrite() {
    let overflow = "Notes line one.\n\nA ```fence``` and a URL https://example.com\n\nfinal";
    let text = with(BODY, &overflow_json(overflow));

    let ghosted = ghost_range(&text, range(&text, "gamma")).unwrap();
    let after_ghost = apply_edits(&text, std::slice::from_ref(&ghosted.edit));
    assert_eq!(overflow_of(&after_ghost), overflow);

    let revived = revive_range(&after_ghost, range(&after_ghost, "gamma")).unwrap();
    let after_revive = apply_edits(&after_ghost, std::slice::from_ref(&revived.edit));
    assert_eq!(overflow_of(&after_revive), overflow);

    let added = add_alternative(
        &after_revive,
        range(&after_revive, "beta"),
        "bravo",
        Some(SpanKind::Word),
    )
    .unwrap();
    let after_add = apply_edits(&after_revive, std::slice::from_ref(&added.edit));
    assert_eq!(overflow_of(&after_add), overflow);

    // An external edit makes the anchors stale; re-anchoring keeps overflow.
    let shifted = after_add.replacen("Alpha", "Intro. Alpha", 1);
    let ReanchorOutcome::Rewritten(edit) = reanchor_block(&shifted).unwrap() else {
        panic!("anchors moved, so the block is rewritten");
    };
    let after_reanchor = apply_edits(&shifted, std::slice::from_ref(&edit));
    assert_eq!(overflow_of(&after_reanchor), overflow);

    // A stash appends to it and a pull back leaves it alone.
    let stashed = stash(&after_reanchor, "Second line here.");
    assert!(overflow_of(&stashed).starts_with(overflow));
    let paragraphs = overflow_paragraphs(&stashed);
    let last = paragraphs.len() - 1;
    let pulled = apply_edits(&stashed, &pull_back(&stashed, last, 0).unwrap().edits);
    assert_eq!(overflow_of(&pulled), overflow_of(&stashed));
    assert!(
        parse(&pulled)
            .unwrap()
            .body
            .starts_with("Second line here.")
    );
}

// ---- export ----

#[test]
fn export_cuts_ghosts_and_omits_overflow_and_block() {
    let ghosted = ghost_range(BODY, range(BODY, " beta gamma")).unwrap();
    let text = apply_edits(BODY, std::slice::from_ref(&ghosted.edit));
    let text = apply_edits(
        &text,
        &stash_range(&text, range(&text, "Second line here."))
            .unwrap()
            .edits,
    );
    let exported = export_clean(&text).unwrap();
    assert_eq!(exported, "Alpha delta.");
    assert!(!exported.contains("terraphim-alternatives"));
    assert!(!exported.contains("Second line"));
}

#[test]
fn export_of_a_plain_document_is_its_body() {
    assert_eq!(export_clean(BODY).unwrap(), BODY);
}

#[test]
fn export_finds_a_ghost_whose_offsets_went_stale() {
    let text = with(
        "Intro. Keep. Drop this.",
        r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 6, "end": 16, "text": "Drop this."}}]}"#,
    );
    assert_eq!(export_clean(&text).unwrap(), "Intro. Keep.");
}

#[test]
fn export_refuses_malformed_blocks_and_lost_ghosts() {
    let malformed = format!("{BODY}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    assert!(matches!(
        export_clean(&malformed).unwrap_err(),
        ExportError::MalformedBlock { .. }
    ));
    let lost = with(
        "Totally different.",
        r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 0, "end": 4, "text": "gone"}}]}"#,
    );
    assert_eq!(
        export_clean(&lost).unwrap_err(),
        ExportError::UnresolvedGhosts(1)
    );
}

// ---- re-anchoring ----

#[test]
fn reanchor_is_unchanged_without_a_block_and_when_current() {
    assert_eq!(reanchor_block(BODY).unwrap(), ReanchorOutcome::Unchanged);
    let ghosted = ghost_range(BODY, range(BODY, "gamma")).unwrap();
    let text = apply_edits(BODY, std::slice::from_ref(&ghosted.edit));
    assert_eq!(reanchor_block(&text).unwrap(), ReanchorOutcome::Unchanged);
}

#[test]
fn reanchor_follows_text_inserted_before_and_is_idempotent() {
    let ghosted = ghost_range(BODY, range(BODY, "gamma")).unwrap();
    let text = apply_edits(BODY, std::slice::from_ref(&ghosted.edit));
    let edited = text.replacen("Alpha", "Preface. Alpha", 1);
    let ReanchorOutcome::Rewritten(edit) = reanchor_block(&edited).unwrap() else {
        panic!("the ghost moved");
    };
    let saved = apply_edits(&edited, std::slice::from_ref(&edit));
    let document = parse(&saved).unwrap();
    assert_eq!(document.annotations.ghosts[0].anchor.start, 20);
    assert_eq!(document.annotations.ghosts[0].anchor.text, "gamma");
    assert_eq!(reanchor_block(&saved).unwrap(), ReanchorOutcome::Unchanged);
}

#[test]
fn reanchor_refreshes_stale_context() {
    let ghosted = ghost_range(BODY, range(BODY, "gamma")).unwrap();
    let text = apply_edits(BODY, std::slice::from_ref(&ghosted.edit));
    // Same offsets, but the neighbouring words changed.
    let edited = text.replacen("beta", "zeta", 1);
    let ReanchorOutcome::Rewritten(edit) = reanchor_block(&edited).unwrap() else {
        panic!("the context changed");
    };
    let saved = apply_edits(&edited, std::slice::from_ref(&edit));
    let ghost = &parse(&saved).unwrap().annotations.ghosts[0];
    assert!(ghost.anchor.before.as_deref().unwrap().contains("zeta"));
}

#[test]
fn reanchor_leaves_the_block_alone_when_an_anchor_is_lost() {
    let text = with(
        "Totally different.",
        r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 0, "end": 4, "text": "gone"}}]}"#,
    );
    assert_eq!(
        reanchor_block(&text).unwrap(),
        ReanchorOutcome::Unresolved(1)
    );
}

#[test]
fn reanchor_refuses_a_malformed_block() {
    let malformed = format!("{BODY}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    assert!(matches!(
        reanchor_block(&malformed).unwrap_err(),
        ReanchorError::MalformedBlock { .. }
    ));
}

// ---- folding ----

#[test]
fn the_block_folds_from_its_opening_to_its_closing_fence() {
    let text = with(BODY, &overflow_json("x"));
    let block = split_annotation_block(&text).block.unwrap();
    // Body is two lines, a blank line, then the 3-line block.
    assert_eq!(block.fold_lines(&text), Some((3, 5)));
}

#[test]
fn a_crlf_block_folds_by_its_lines() {
    let text = "Body.\r\n\r\n```terraphim-alternatives\r\n{\"version\": 1, \"spans\": [], \"overflow\": \"x\"}\r\n```\r\n";
    let block = split_annotation_block(text).block.unwrap();
    assert_eq!(block.fold_lines(text), Some((2, 4)));
}

#[test]
fn an_unterminated_block_folds_to_its_last_line() {
    let text = "Body.\n\n```terraphim-alternatives\n{ nope\nstill going\n";
    let block = split_annotation_block(text).block.unwrap();
    assert_eq!(block.fold_lines(text), Some((2, 4)));
}

#[test]
fn a_single_line_block_does_not_fold() {
    let text = "Body.\n\n```terraphim-alternatives";
    let block = split_annotation_block(text).block.unwrap();
    assert_eq!(block.fold_lines(text), None);
}

#[test]
fn stash_keeps_indentation_and_hard_break_spaces() {
    let code = "Keep.\n    indented code\nDone.";
    let saved = stash(code, "    indented code");
    assert_eq!(overflow_of(&saved), "    indented code");
    assert_eq!(parse(&saved).unwrap().body, "Keep.\nDone.");
    let hard = "Line one  \nLine two.";
    let saved = stash(hard, "Line one  ");
    assert_eq!(overflow_of(&saved), "Line one  ");
    let tabbed = "A.\n\tTabbed.\nB.";
    assert_eq!(overflow_of(&stash(tabbed, "\tTabbed.")), "\tTabbed.");
}

// ---- pull back separates the paragraph from its surroundings ----

/// The body after pulling the only overflow paragraph, "Para.", into `body`
/// at byte `at`.
fn pulled(body: &str, at: usize) -> String {
    let text = with(body, &overflow_json("Para."));
    let edited = pull_back(&text, 0, at).unwrap();
    let saved = apply_edits(&text, &edited.edits);
    assert_eq!(
        overflow_of(&saved),
        "Para.",
        "copied: still in the overflow"
    );
    parse(&saved).unwrap().body
}

#[test]
fn pull_back_mid_line_inserts_inline() {
    assert_eq!(pulled("Start end.", 5), "StartPara. end.");
    assert_eq!(pulled("Start end.", 6), "Start Para.end.");
}

#[test]
fn pull_back_at_a_line_start_makes_its_own_paragraph() {
    assert_eq!(
        pulled("First line.\nSecond line.", 12),
        "First line.\n\nPara.\n\nSecond line."
    );
    assert_eq!(pulled("Body.", 0), "Para.\n\nBody.");
}

#[test]
fn pull_back_at_a_line_end_makes_its_own_paragraph() {
    assert_eq!(pulled("Body.", 5), "Body.\n\nPara.");
    assert_eq!(pulled("A line.\nNext.", 7), "A line.\n\nPara.\n\nNext.");
}

#[test]
fn pull_back_between_paragraphs_keeps_one_blank_line_each_side() {
    let body = "One.\n\nTwo.";
    let expected = "One.\n\nPara.\n\nTwo.";
    assert_eq!(pulled(body, 4), expected, "end of the first paragraph");
    assert_eq!(pulled(body, 5), expected, "on the blank line");
    assert_eq!(pulled(body, 6), expected, "start of the second paragraph");
}

#[test]
fn pull_back_into_an_empty_body_is_the_paragraph() {
    assert_eq!(pulled("", 0), "Para.");
}

#[test]
fn pull_back_follows_crlf_line_endings() {
    assert_eq!(pulled("A.\r\nB.", 4), "A.\r\n\r\nPara.\r\n\r\nB.");
    assert_eq!(pulled("A.\r\n\r\nB.", 2), "A.\r\n\r\nPara.\r\n\r\nB.");
}

#[test]
fn pull_back_of_a_multi_line_paragraph_keeps_its_lines() {
    let text = with("Head.\nTail.", &overflow_json("Line one\nline two"));
    let edited = pull_back(&text, 0, 6).unwrap();
    let saved = apply_edits(&text, &edited.edits);
    assert_eq!(
        parse(&saved).unwrap().body,
        "Head.\n\nLine one\nline two\n\nTail."
    );
}

#[test]
fn pull_back_shifts_later_spans_past_the_separators() {
    let text = with(
        BODY,
        &format!(r#"{{"version": 1, "spans": [{BETA_SPAN}], "overflow": "Para."}}"#),
    );
    let at = text.find("Second").unwrap();
    let saved = apply_edits(&text, &pull_back(&text, 0, at).unwrap().edits);
    let document = parse(&saved).unwrap();
    assert_eq!(
        document.body,
        "Alpha beta gamma delta.\n\nPara.\n\nSecond line here."
    );
    assert_eq!(document.annotations.spans[0].anchor.text, "beta");
    assert_eq!(document.annotations.spans[0].anchor.start, 6);
    assert_eq!(document.annotations.overflow, "Para.");
}

#[test]
fn pull_back_before_a_span_moves_its_anchor_in_the_block() {
    let text = with(
        BODY,
        &format!(r#"{{"version": 1, "spans": [{BETA_SPAN}], "overflow": "Para."}}"#),
    );
    let edited = pull_back(&text, 0, 0).unwrap();
    assert_eq!(edited.edits.len(), 2, "the body and the shifted anchors");
    let document = parse(&apply_edits(&text, &edited.edits)).unwrap();
    assert_eq!(
        document.annotations.spans[0].anchor.start,
        6 + "Para.\n\n".len()
    );
    assert_eq!(document.annotations.overflow, "Para.");
}

#[test]
fn pull_back_never_splits_a_crlf_pair() {
    // Byte 3 of "A.\r\nB." lies between the \r and the \n.
    let expected = "A.\r\n\r\nPara.\r\n\r\nB.";
    assert_eq!(pulled("A.\r\nB.", 3), expected);
    assert_eq!(pulled("A.\r\nB.", 4), expected);
}

#[test]
fn pull_back_after_all_anchors_still_writes_back_a_stale_block() {
    // The ghost's stored offsets are stale but it can be found again; the
    // insertion at the end of the body moves nothing, yet the refreshed
    // block is written, as for every other rewrite.
    let text = with(
        "Intro. Drop this. Keep.",
        r#"{"version": 1, "spans": [], "overflow": "Para.", "ghosts": [{"id": "g1", "anchor": {"start": 2, "end": 12, "text": "Drop this."}}]}"#,
    );
    let at = "Intro. Drop this. Keep.".len();
    let saved = apply_edits(&text, &pull_back(&text, 0, at).unwrap().edits);
    let document = parse(&saved).unwrap();
    assert_eq!(document.body, "Intro. Drop this. Keep.\n\nPara.");
    assert_eq!(
        document.annotations.ghosts[0].anchor.start, 7,
        "re-anchored"
    );
    assert_eq!(document.annotations.overflow, "Para.");
}

#[test]
fn pull_back_after_all_anchors_still_writes_back_a_stale_span() {
    // A stale offset on a span, not a ghost: same rule.
    let span = r#"{"id": "s1", "kind": "word", "anchor": {"start": 0, "end": 4, "text": "beta"}, "active": 0, "alts": [{"text": "beta", "source": "original"}, {"text": "bravo", "source": "human"}]}"#;
    let text = with(
        "Intro beta tail.",
        &format!(r#"{{"version": 1, "spans": [{span}], "overflow": "Para."}}"#),
    );
    let at = "Intro beta tail.".len();
    let saved = apply_edits(&text, &pull_back(&text, 0, at).unwrap().edits);
    let document = parse(&saved).unwrap();
    assert_eq!(document.annotations.spans[0].anchor.start, 6, "re-anchored");
    assert_eq!(document.annotations.overflow, "Para.");
}

#[test]
fn pull_back_after_all_anchors_still_writes_back_stale_offsets_with_current_context() {
    // The context is current, only the offset is stale (text was added in
    // front elsewhere): re-anchoring changes just the offset.
    let text = with(
        "Intro. Drop this. Keep.",
        r#"{"version": 1, "spans": [], "overflow": "Para.", "ghosts": [{"id": "g1", "anchor": {"start": 2, "end": 12, "text": "Drop this.", "before": "Intro. ", "after": " Keep."}}]}"#,
    );
    let at = "Intro. Drop this. Keep.".len();
    let saved = apply_edits(&text, &pull_back(&text, 0, at).unwrap().edits);
    let document = parse(&saved).unwrap();
    assert_eq!(
        document.annotations.ghosts[0].anchor.start, 7,
        "re-anchored"
    );
}
