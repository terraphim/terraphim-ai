//! Hover with provenance and "Replace with" a stored alternative (R-2.2,
//! R-3.6, R-4.4) through the tower-lsp service.
//!
//! No mocks: the server is built from the real fixture thesaurus committed
//! with `terraphim_lsp_core`, and annotated documents are made with the
//! core's own `add_alternative` and `ghost_range`. Each action's edit is
//! applied to the open document, sent back with `didChange`, and the result
//! is parsed with the editor's parser and read back through the server's
//! hover, code actions and pulled diagnostics.

use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::core::{
    LineIndex, LinePosition, SpanKind, TextRange, add_alternative, apply_edits, ghost_range,
};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
/// A span over "paperclip" (original, human "binder clip"), and a ghost over
/// "Drop this aside."
const ANNOTATED_DOC: &str = include_str!("../tests/fixtures/annotated_doc.md");

fn uri() -> Url {
    Url::parse("file:///tmp/hover-provenance.md").unwrap()
}

fn alternative_kind() -> CodeActionKind {
    CodeActionKind::new("refactor.terraphim.alternative")
}

async fn open_server(
    text: &str,
    versioned: bool,
) -> (LspService<TerraphimLspServer>, ClientSocket) {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).expect("fixture thesaurus");
    let (service, socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
    let capabilities = ClientCapabilities {
        workspace: Some(WorkspaceClientCapabilities {
            workspace_edit: Some(WorkspaceEditClientCapabilities {
                document_changes: Some(versioned),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    service
        .inner()
        .initialize(InitializeParams {
            capabilities,
            ..Default::default()
        })
        .await
        .unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
    (service, socket)
}

fn lsp_position(text: &str, byte: usize) -> Position {
    let LinePosition { line, character } = LineIndex::new(text).position(byte);
    Position { line, character }
}

fn byte_of(text: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in text"))
}

/// A cursor one character into the first `needle`.
fn cursor_in(text: &str, needle: &str) -> Range {
    let at = lsp_position(text, byte_of(text, needle) + 1);
    Range { start: at, end: at }
}

/// `text` with `alternative` stored for the first `needle`, written by the
/// core exactly as the editor would.
fn with_alternative(text: &str, needle: &str, alternative: &str, kind: Option<SpanKind>) -> String {
    let start = byte_of(text, needle);
    let range = TextRange::from_bytes(text, start, start + needle.len());
    let added = add_alternative(text, range, alternative, kind).expect("alternative added");
    apply_edits(text, std::slice::from_ref(&added.edit))
}

/// `text` with the first `needle` ghosted.
fn with_ghost(text: &str, needle: &str) -> String {
    let start = byte_of(text, needle);
    let range = TextRange::from_bytes(text, start, start + needle.len());
    let ghosted = ghost_range(text, range).expect("ghosted");
    apply_edits(text, std::slice::from_ref(&ghosted.edit))
}

async fn hover_text(server: &TerraphimLspServer, text: &str, needle: &str) -> Option<String> {
    let hover = server
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                position: cursor_in(text, needle).start,
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()?;
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("markdown hover");
    };
    assert_eq!(markup.kind, MarkupKind::Markdown);
    Some(markup.value)
}

async fn actions(
    server: &TerraphimLspServer,
    range: Range,
    only: Option<Vec<CodeActionKind>>,
) -> Vec<CodeAction> {
    server
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range,
            context: CodeActionContext {
                diagnostics: vec![],
                only,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect()
}

fn titles_of(actions: &[CodeAction], kind: &CodeActionKind) -> Vec<String> {
    actions
        .iter()
        .filter(|action| action.kind.as_ref() == Some(kind))
        .map(|action| action.title.clone())
        .collect()
}

/// The version and edits of a versioned workspace edit with one document.
fn versioned_edits(action: &CodeAction) -> (Option<i32>, Vec<TextEdit>) {
    let edit = action.edit.as_ref().expect("an edit");
    let Some(DocumentChanges::Edits(edits)) = edit.document_changes.as_ref() else {
        panic!("versioned document changes");
    };
    assert_eq!(edits.len(), 1);
    let edits_of_doc = edits[0]
        .edits
        .iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(annotated) => annotated.text_edit.clone(),
        })
        .collect();
    (edits[0].text_document.version, edits_of_doc)
}

fn apply(text: &str, edits: &[TextEdit]) -> String {
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    let mut out = text.to_string();
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| std::cmp::Reverse(byte(edit.range.start)));
    for edit in sorted {
        out.replace_range(byte(edit.range.start)..byte(edit.range.end), &edit.new_text);
    }
    out
}

async fn change(server: &TerraphimLspServer, text: &str, version: i32) {
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        })
        .await;
}

/// The codes of the server's pulled diagnostics.
async fn diagnostic_codes(server: &TerraphimLspServer) -> Vec<String> {
    let report = server
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();
    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(report)) = report
    else {
        panic!("a full report");
    };
    report
        .full_document_diagnostic_report
        .items
        .iter()
        .filter_map(|d| match &d.code {
            Some(NumberOrString::String(code)) => Some(code.clone()),
            _ => None,
        })
        .collect()
}

fn parsed(text: &str) -> terraphim_alternatives::Document {
    terraphim_alternatives::parse(text).expect("the editor accepts the block")
}

#[tokio::test]
async fn hover_on_a_stored_span_lists_its_alternatives_with_provenance() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let hover = hover_text(service.inner(), ANNOTATED_DOC, "paperclip")
        .await
        .expect("hover on the span");
    // The KG term line stays first.
    assert!(hover.starts_with("**paperclip**"), "{hover}");
    assert!(hover.contains("- paperclip (original, active)"), "{hover}");
    assert!(hover.contains("- binder clip (human)"), "{hover}");
    // "paperclip" is a single-term concept: no KG synonyms to list.
    assert!(!hover.contains("(kg"), "{hover}");
}

#[tokio::test]
async fn hover_on_a_kg_term_lists_its_synonyms_as_kg() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let hover = hover_text(service.inner(), ANNOTATED_DOC, "choice")
        .await
        .expect("hover on the term");
    assert!(hover.contains("- choice (kg, active)"), "{hover}");
    for synonym in ["decision", "judgment", "option"] {
        assert!(hover.contains(&format!("- {synonym} (kg)")), "{hover}");
    }
    assert!(!hover.contains("original"), "{hover}");
}

#[tokio::test]
async fn hover_names_the_model_of_an_ai_alternative() {
    let body = "Pass me a paperclip.";
    let start = byte_of(body, "paperclip");
    let mut document = terraphim_alternatives::Document::new(body);
    let id = document
        .add_span(SpanKind::Word, start, start + "paperclip".len())
        .unwrap();
    document
        .add_alternative(
            &id,
            "fastener",
            terraphim_alternatives::Source::Ai,
            Some("claude-test".to_string()),
        )
        .unwrap();
    let text = terraphim_alternatives::write(&document);
    let (service, _socket) = open_server(&text, true).await;
    let hover = hover_text(service.inner(), &text, "paperclip")
        .await
        .expect("hover");
    assert!(hover.contains("- fastener (ai, claude-test)"), "{hover}");
    let offered = actions(service.inner(), cursor_in(&text, "paperclip"), None).await;
    assert_eq!(
        titles_of(&offered, &alternative_kind()),
        ["Replace with fastener (ai, claude-test)"]
    );
}

#[tokio::test]
async fn hover_on_a_sentence_span_that_is_no_kg_term_shows_its_alternatives() {
    let text = with_alternative(
        "Keep going. Drop this aside.",
        "Drop this aside.",
        "Leave it out.",
        None,
    );
    let (service, _socket) = open_server(&text, true).await;
    let hover = hover_text(service.inner(), &text, "this aside")
        .await
        .expect("hover on the sentence span");
    assert!(
        hover.contains("- Drop this aside. (original, active)"),
        "{hover}"
    );
    assert!(hover.contains("- Leave it out. (human)"), "{hover}");
    assert!(
        hover_text(service.inner(), &text, "going").await.is_none(),
        "no hover off spans and terms"
    );
}

#[tokio::test]
async fn replace_with_a_stored_alternative_round_trips_and_updates_active() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    let offered = actions(
        server,
        cursor_in(ANNOTATED_DOC, "paperclip"),
        Some(vec![alternative_kind()]),
    )
    .await;
    assert_eq!(
        titles_of(&offered, &alternative_kind()),
        ["Replace with binder clip (human)"]
    );
    assert_eq!(offered.len(), 1, "only the requested kind");
    let (version, edits) = versioned_edits(&offered[0]);
    assert_eq!(version, Some(1));
    assert_eq!(edits.len(), 2, "the body edit and the block edit");

    let saved = apply(ANNOTATED_DOC, &edits);
    let document = parsed(&saved);
    assert!(document.body.ends_with("Pass me a binder clip."));
    let span = &document.annotations.spans[0];
    assert_eq!(span.active, 1);
    assert_eq!(span.anchor.text, "binder clip");
    assert_eq!(document.annotations.ghosts.len(), 1, "the ghost is kept");

    change(server, &saved, 2).await;
    let codes = diagnostic_codes(server).await;
    assert!(
        codes
            .iter()
            .all(|code| !code.starts_with("annotation-block")),
        "{codes:?}"
    );
    assert!(codes.iter().any(|code| code == "ghosted"), "{codes:?}");
    let hover = hover_text(server, &saved, "binder clip")
        .await
        .expect("hover on the swapped span");
    assert!(hover.contains("- paperclip (original)"), "{hover}");
    assert!(hover.contains("- binder clip (human, active)"), "{hover}");

    // And back: the original is now on offer, at the new version.
    let back = actions(server, cursor_in(&saved, "binder clip"), None).await;
    let back: Vec<&CodeAction> = back
        .iter()
        .filter(|action| action.kind == Some(alternative_kind()))
        .collect();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].title, "Replace with paperclip (original)");
    let (version, edits) = versioned_edits(back[0]);
    assert_eq!(version, Some(2));
    let restored = apply(&saved, &edits);
    assert_eq!(parsed(&restored).body, parsed(ANNOTATED_DOC).body);
    assert_eq!(parsed(&restored).annotations.spans[0].active, 0);
}

#[tokio::test]
async fn a_plain_client_gets_changes_not_document_changes() {
    let (service, _socket) = open_server(ANNOTATED_DOC, false).await;
    let offered = actions(
        service.inner(),
        cursor_in(ANNOTATED_DOC, "paperclip"),
        Some(vec![alternative_kind()]),
    )
    .await;
    let edit = offered[0].edit.as_ref().unwrap();
    assert!(edit.document_changes.is_none());
    let edits = &edit.changes.as_ref().unwrap()[&uri()];
    let saved = apply(ANNOTATED_DOC, edits);
    assert_eq!(parsed(&saved).annotations.spans[0].active, 1);
}

#[tokio::test]
async fn the_kind_is_requested_by_prefix_and_hidden_otherwise() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    let initialised = server
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let Some(CodeActionProviderCapability::Options(options)) =
        initialised.capabilities.code_action_provider
    else {
        panic!("code action options");
    };
    assert!(
        options
            .code_action_kinds
            .unwrap()
            .contains(&alternative_kind()),
        "the kind is advertised"
    );
    let at = cursor_in(ANNOTATED_DOC, "paperclip");
    let by_refactor = actions(server, at, Some(vec![CodeActionKind::REFACTOR])).await;
    assert_eq!(titles_of(&by_refactor, &alternative_kind()).len(), 1);
    let by_quickfix = actions(server, at, Some(vec![CodeActionKind::QUICKFIX])).await;
    assert!(titles_of(&by_quickfix, &alternative_kind()).is_empty());
    let by_rewrite = actions(server, at, Some(vec![CodeActionKind::REFACTOR_REWRITE])).await;
    assert!(titles_of(&by_rewrite, &alternative_kind()).is_empty());
}

#[tokio::test]
async fn no_kg_rewrite_of_a_stored_word_span_only_its_stored_alternatives() {
    // "café" is a KG term with synonyms and a stored word span.
    let text = with_alternative(
        "A café visit is an honour.",
        "café",
        "bistro",
        Some(SpanKind::Word),
    );
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let offered = actions(server, cursor_in(&text, "café"), None).await;
    assert!(
        titles_of(&offered, &CodeActionKind::REFACTOR_REWRITE).is_empty(),
        "{:?}",
        titles_of(&offered, &CodeActionKind::REFACTOR_REWRITE)
    );
    assert_eq!(
        titles_of(&offered, &alternative_kind()),
        ["Replace with bistro (human)"]
    );
    // The synonyms stay visible in the hover.
    let hover = hover_text(server, &text, "café").await.unwrap();
    assert!(hover.contains("- café (original, active)"), "{hover}");
    assert!(hover.contains("- bistro (human)"), "{hover}");
    assert!(hover.contains("- coffee shop (kg)"), "{hover}");
    assert!(hover.contains("- coffeehouse (kg)"), "{hover}");
}

#[tokio::test]
async fn no_kg_rewrite_inside_a_stored_sentence_span_but_one_outside() {
    let body = "Every choice is a judgment. Another choice waits.";
    let text = with_alternative(
        body,
        "Every choice is a judgment.",
        "Every call is a judgment.",
        None,
    );
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let inside = actions(server, cursor_in(&text, "choice is"), None).await;
    assert!(
        titles_of(&inside, &CodeActionKind::REFACTOR_REWRITE).is_empty(),
        "{:?}",
        titles_of(&inside, &CodeActionKind::REFACTOR_REWRITE)
    );
    assert_eq!(
        titles_of(&inside, &alternative_kind()),
        ["Replace with Every call is a judgment. (human)"]
    );
    let hover = hover_text(server, &text, "choice is").await.unwrap();
    assert!(hover.contains("- decision (kg)"), "{hover}");
    assert!(
        hover.contains("- Every choice is a judgment. (original, active)"),
        "{hover}"
    );

    // The same term outside the span is rewritten as before.
    let outside = actions(server, cursor_in(&text, "choice waits"), None).await;
    let rewrites = titles_of(&outside, &CodeActionKind::REFACTOR_REWRITE);
    assert!(
        rewrites.contains(&"Replace with decision".to_string()),
        "{rewrites:?}"
    );
    assert!(titles_of(&outside, &alternative_kind()).is_empty());
    let rewrite = outside
        .iter()
        .find(|action| action.title == "Replace with decision")
        .unwrap();
    let saved = apply(&text, &versioned_edits(rewrite).1);
    change(server, &saved, 2).await;
    let document = parsed(&saved);
    assert!(document.body.ends_with("Another decision waits."));
    assert_eq!(
        document.annotations.spans[0].anchor.text,
        "Every choice is a judgment."
    );
    assert_eq!(
        stored_after(&saved),
        1,
        "the span still re-anchors after the rewrite outside it"
    );
}

/// How many stored spans re-anchor in `text`, as the editor does on open.
fn stored_after(text: &str) -> usize {
    let mut document = parsed(text);
    let report = document.reanchor();
    assert!(report.unresolved.is_empty(), "{:?}", report.unresolved);
    document.annotations.spans.len()
}

#[tokio::test]
async fn no_kg_rewrite_inside_a_ghost() {
    let text = with_ghost("A café visit is an honour.", "café visit");
    let (service, _socket) = open_server(&text, true).await;
    let offered = actions(service.inner(), cursor_in(&text, "café"), None).await;
    assert!(titles_of(&offered, &CodeActionKind::REFACTOR_REWRITE).is_empty());
    // "honour" is outside the ghost: still rewritten.
    let honour = actions(service.inner(), cursor_in(&text, "honour"), None).await;
    assert!(
        titles_of(&honour, &CodeActionKind::REFACTOR_REWRITE)
            .contains(&"Replace with privilege".to_string())
    );
}

#[tokio::test]
async fn no_kg_rewrite_whose_article_fix_up_falls_in_a_stored_anchor() {
    // "an honour" -> "a privilege": the article edit lands in the ghost over
    // "is an", though "honour" itself is outside it.
    let text = with_ghost("A café visit is an honour.", "is an");
    let (service, _socket) = open_server(&text, true).await;
    let offered = actions(service.inner(), cursor_in(&text, "honour"), None).await;
    let rewrites = titles_of(&offered, &CodeActionKind::REFACTOR_REWRITE);
    assert!(
        !rewrites.contains(&"Replace with privilege".to_string()),
        "{rewrites:?}"
    );
}

#[tokio::test]
async fn a_malformed_block_or_stale_anchor_falls_back_to_the_kg() {
    let malformed = "A choice.\n\n```terraphim-alternatives\n{\"version\": 1}\n";
    let (service, _socket) = open_server(malformed, true).await;
    let server = service.inner();
    let hover = hover_text(server, malformed, "choice").await.unwrap();
    assert!(hover.contains("- decision (kg)"), "{hover}");
    let offered = actions(server, cursor_in(malformed, "choice"), None).await;
    assert!(titles_of(&offered, &alternative_kind()).is_empty());
    assert!(!titles_of(&offered, &CodeActionKind::REFACTOR_REWRITE).is_empty());

    // The span's text is gone from the body: nothing stored is offered.
    let stale = ANNOTATED_DOC.replacen("Pass me a paperclip.", "Pass me a stapler.", 1);
    parsed(&stale);
    let (service, _socket) = open_server(&stale, true).await;
    let offered = actions(service.inner(), cursor_in(&stale, "stapler"), None).await;
    assert!(titles_of(&offered, &alternative_kind()).is_empty());
    assert!(
        hover_text(service.inner(), &stale, "stapler")
            .await
            .is_none()
    );
}

#[tokio::test]
async fn authored_kg_synonyms_follow_committed_changes_history_and_reopen() {
    let body = "An original decision.";
    let (service, _socket) = open_server(body, true).await;
    let server = service.inner();
    let start = lsp_position(body, byte_of(body, "decision"));
    let end = lsp_position(body, byte_of(body, "decision") + "decision".len());
    let result = server.execute_command(ExecuteCommandParams {
        command: terraphim_lsp::commands::ADD_ALTERNATIVE.to_string(),
        arguments: vec![serde_json::json!({"uri":uri(),"version":1,"range":{"start":start,"end":end},"text":"Judgment","kind":"word"})],
        work_done_progress_params: Default::default(),
    }).await.unwrap().unwrap();
    let edit: WorkspaceEdit = serde_json::from_value(result).unwrap();
    let action = CodeAction {
        title: String::new(),
        edit: Some(edit),
        ..Default::default()
    };
    let (version, edits) = versioned_edits(&action);
    assert_eq!(version, Some(1));
    let created = apply(body, &edits);
    // Calculating the actual server edit cannot publish a document concept.
    let before = hover_text(server, body, "decision").await.unwrap();
    assert!(!before.contains("Document KG synonyms"));
    change(server, &created, 2).await;
    let registered = hover_text(server, &created, "decision").await.unwrap();
    eprintln!("ACTUAL AUTHORED KG HOVER OUTPUT:\n{registered}");
    assert!(registered.contains("Document KG synonyms"), "{registered}");
    assert!(
        registered.contains("synonyms:: decision, Judgment"),
        "{registered}"
    );
    assert!(registered.contains("decision → concept 1"), "{registered}");
    assert!(registered.contains("judgment → concept 1"), "{registered}");
    let serialized = terraphim_alternatives::write(&parsed(&created));
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("authored-synonyms.md");
    std::fs::write(&path, serialized.as_bytes()).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(saved, serialized, "reopen uses actual saved file bytes");
    server
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri() },
        })
        .await;
    assert!(hover_text(server, &saved, "decision").await.is_none());
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".into(),
                version: 1,
                text: saved.clone(),
            },
        })
        .await;
    assert_eq!(
        hover_text(server, &saved, "decision").await,
        Some(registered.clone())
    );
    let (reopened, _socket2) = open_server(&saved, true).await;
    assert_eq!(
        hover_text(reopened.inner(), &saved, "decision").await,
        Some(registered.clone())
    );
    change(server, body, 3).await;
    assert!(
        !hover_text(server, body, "decision")
            .await
            .unwrap()
            .contains("Document KG synonyms")
    );
    change(server, &created, 4).await;
    assert_eq!(
        hover_text(server, &created, "decision").await,
        Some(registered)
    );
    let mut document = parsed(&created);
    let id = document.annotations.spans[0].id.clone();
    document.remove_alternative(&id, 1).unwrap();
    let removed = terraphim_alternatives::write(&document);
    change(server, &removed, 5).await;
    assert!(
        !hover_text(server, &removed, "decision")
            .await
            .unwrap()
            .contains("Document KG synonyms")
    );
}
