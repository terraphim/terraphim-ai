//! "Ghost it" and "Revive" code actions (R-5.1, R-5.2) through the tower-lsp
//! service.
//!
//! No mocks: the server is built from the real fixture thesaurus committed
//! with `terraphim_lsp_core`. Each action's edit is applied to the open
//! document, sent back with `didChange`, and the result is read through the
//! server's own pulled diagnostics, so a new ghost is proved to render.

use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::core::{LineIndex, LinePosition};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
/// A span over "paperclip" with a human alternative, and a ghost over
/// "Drop this aside."
const ANNOTATED_DOC: &str = include_str!("../tests/fixtures/annotated_doc.md");
const PLAIN: &str = "Alpha beta gamma delta.";

fn uri() -> Url {
    Url::parse("file:///tmp/ghost-actions.md").unwrap()
}

fn ghost_kind() -> CodeActionKind {
    CodeActionKind::new("refactor.terraphim.ghost")
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

/// The LSP range of the first `needle` in `text`.
fn range_of(text: &str, needle: &str) -> Range {
    let start = text.find(needle).unwrap_or_else(|| panic!("{needle:?}"));
    Range {
        start: lsp_position(text, start),
        end: lsp_position(text, start + needle.len()),
    }
}

fn cursor_in(text: &str, needle: &str) -> Range {
    let at = lsp_position(text, text.find(needle).unwrap() + 1);
    Range { start: at, end: at }
}

async fn ghost_actions(
    server: &TerraphimLspServer,
    range: Range,
    only: Option<Vec<CodeActionKind>>,
) -> Vec<CodeAction> {
    let response = server
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
        .unwrap_or_default();
    response
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) if action.kind == Some(ghost_kind()) => {
                Some(action)
            }
            _ => None,
        })
        .collect()
}

fn titles(actions: &[CodeAction]) -> Vec<&str> {
    actions.iter().map(|action| action.title.as_str()).collect()
}

/// The version and edits of an action's workspace edit, which must be a
/// single versioned document edit (not a command).
fn edits_of(action: &CodeAction) -> (Option<i32>, Vec<TextEdit>) {
    assert!(action.command.is_none());
    let edit = action.edit.as_ref().expect("an edit, not a command");
    let Some(DocumentChanges::Edits(edits)) = edit.document_changes.as_ref() else {
        panic!("versioned document changes");
    };
    assert_eq!(edits.len(), 1);
    let version = edits[0].text_document.version;
    let edits = edits[0]
        .edits
        .iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(annotated) => annotated.text_edit.clone(),
        })
        .collect();
    (version, edits)
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

/// The texts the server fades as `ghosted` hints.
async fn faded(server: &TerraphimLspServer, text: &str) -> Vec<String> {
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
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    report
        .full_document_diagnostic_report
        .items
        .iter()
        .filter(|d| matches!(&d.code, Some(NumberOrString::String(code)) if code == "ghosted"))
        .map(|d| text[byte(d.range.start)..byte(d.range.end)].to_string())
        .collect()
}

/// Apply the action titled `title` to the open document (at `version`) and
/// return the new text, after sending it back as the next version.
async fn run(
    server: &TerraphimLspServer,
    text: &str,
    range: Range,
    title: &str,
    version: i32,
) -> String {
    let actions = ghost_actions(server, range, None).await;
    let action = actions
        .iter()
        .find(|action| action.title == title)
        .unwrap_or_else(|| panic!("{title:?} not in {:?}", titles(&actions)));
    let (_, edits) = edits_of(action);
    let saved = apply(text, &edits);
    change(server, &saved, version + 1).await;
    saved
}

fn parsed(text: &str) -> terraphim_alternatives::Document {
    terraphim_alternatives::parse(text).expect("the editor accepts the block")
}

#[tokio::test]
async fn ghost_it_on_a_plain_document_creates_the_block_and_renders() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let actions = ghost_actions(server, range_of(PLAIN, "beta"), None).await;
    assert_eq!(titles(&actions), ["Ghost it"]);
    let (version, edits) = edits_of(&actions[0]);
    assert_eq!(version, Some(1));
    assert_eq!(edits.len(), 1);
    // The edit starts where the body ends: nothing of the body is replaced.
    assert_eq!(edits[0].range.start, edits[0].range.end);
    assert_eq!(edits[0].range.start, lsp_position(PLAIN, PLAIN.len()));

    let saved = apply(PLAIN, &edits);
    assert!(saved.starts_with(PLAIN));
    assert_eq!(parsed(&saved).body, PLAIN);
    assert!(faded(server, PLAIN).await.is_empty());
    change(server, &saved, 2).await;
    assert_eq!(faded(server, &saved).await, ["beta"]);
}

#[tokio::test]
async fn a_plain_client_gets_changes_not_document_changes() {
    let (service, _socket) = open_server(PLAIN, false).await;
    let actions = ghost_actions(service.inner(), range_of(PLAIN, "beta"), None).await;
    let edit = actions[0].edit.as_ref().unwrap();
    assert!(edit.document_changes.is_none());
    assert_eq!(edit.changes.as_ref().unwrap()[&uri()].len(), 1);
}

#[tokio::test]
async fn touching_and_overlapping_ghosts_merge() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let one = run(server, PLAIN, range_of(PLAIN, "Alpha beta"), "Ghost it", 1).await;
    assert_eq!(faded(server, &one).await, ["Alpha beta"]);
    // Touching.
    let two = run(server, &one, range_of(&one, " gamma"), "Ghost it", 2).await;
    assert_eq!(faded(server, &two).await, ["Alpha beta gamma"]);
    // Overlapping (partly ghosted, so still offered).
    let three = run(server, &two, range_of(&two, "gamma delta"), "Ghost it", 3).await;
    assert_eq!(faded(server, &three).await, ["Alpha beta gamma delta"]);
    assert_eq!(parsed(&three).annotations.ghosts.len(), 1);
}

#[tokio::test]
async fn revive_splits_trims_and_removes() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let all = run(
        server,
        PLAIN,
        range_of(PLAIN, "Alpha beta gamma delta"),
        "Ghost it",
        1,
    )
    .await;
    // Partial revive splits.
    let split = run(server, &all, range_of(&all, "beta"), "Revive", 2).await;
    assert_eq!(faded(server, &split).await, ["Alpha ", " gamma delta"]);
    // A cursor revives the whole ghost it is in.
    let right = run(server, &split, cursor_in(&split, "gamma"), "Revive", 3).await;
    assert_eq!(faded(server, &right).await, ["Alpha "]);
    // Reviving the whole of the last ghost removes it.
    let none = run(server, &right, range_of(&right, "Alpha "), "Revive", 4).await;
    assert!(faded(server, &none).await.is_empty());
    assert_eq!(none, PLAIN, "an empty block is not written");
}

#[tokio::test]
async fn availability_follows_the_selection() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    // Wholly ghosted: Revive only.
    let aside = ghost_actions(server, range_of(ANNOTATED_DOC, "Drop this aside."), None).await;
    assert_eq!(titles(&aside), ["Revive"]);
    // Partly ghosted: both.
    let partly = ghost_actions(server, range_of(ANNOTATED_DOC, "honour. Drop"), None).await;
    assert_eq!(titles(&partly), ["Ghost it", "Revive"]);
    // Clear text: Ghost it only. A cursor there offers nothing.
    let clear = ghost_actions(server, range_of(ANNOTATED_DOC, "Every choice"), None).await;
    assert_eq!(titles(&clear), ["Ghost it"]);
    assert!(
        ghost_actions(server, cursor_in(ANNOTATED_DOC, "Every"), None)
            .await
            .is_empty()
    );
    // A cursor in a ghost offers Revive.
    let inside = ghost_actions(server, cursor_in(ANNOTATED_DOC, "Drop this"), None).await;
    assert_eq!(titles(&inside), ["Revive"]);
}

#[tokio::test]
async fn kinds_are_advertised_and_filtered_with_only() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let initialised = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let Some(CodeActionProviderCapability::Options(options)) =
        initialised.capabilities.code_action_provider
    else {
        panic!("code action options");
    };
    assert!(options.code_action_kinds.unwrap().contains(&ghost_kind()));

    let selection = range_of(PLAIN, "beta");
    for only in [vec![ghost_kind()], vec![CodeActionKind::REFACTOR]] {
        assert_eq!(
            titles(&ghost_actions(server, selection, Some(only)).await),
            ["Ghost it"]
        );
    }
    for only in [
        vec![CodeActionKind::QUICKFIX],
        vec![CodeActionKind::REFACTOR_REWRITE],
    ] {
        assert!(
            ghost_actions(server, selection, Some(only))
                .await
                .is_empty()
        );
    }
}

#[tokio::test]
async fn a_ghost_over_a_word_with_alternatives_keeps_the_span_and_overflow() {
    let doc = ANNOTATED_DOC.replacen(
        "\"ghosts\": [",
        "\"overflow\": \"an eraser\",\n  \"ghosts\": [",
        1,
    );
    let (service, _socket) = open_server(&doc, true).await;
    let server = service.inner();
    let saved = run(server, &doc, range_of(&doc, "paperclip"), "Ghost it", 1).await;
    let document = parsed(&saved);
    assert_eq!(document.body, parsed(&doc).body);
    assert_eq!(document.annotations.overflow, "an eraser");
    assert_eq!(document.annotations.spans.len(), 1);
    assert_eq!(document.annotations.spans[0].alts[1].text, "binder clip");
    assert_eq!(document.annotations.ghosts.len(), 2);
    assert_eq!(
        faded(server, &saved).await,
        ["Drop this aside.", "paperclip"]
    );

    // Reviving the new ghost leaves the old ghost, the span and overflow.
    let back = run(server, &saved, range_of(&saved, "paperclip"), "Revive", 2).await;
    let document = parsed(&back);
    assert_eq!(document.annotations.overflow, "an eraser");
    assert_eq!(document.annotations.spans[0].alts[1].text, "binder clip");
    assert_eq!(faded(server, &back).await, ["Drop this aside."]);
}

#[tokio::test]
async fn the_edit_replaces_only_the_block() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let actions = ghost_actions(
        service.inner(),
        range_of(ANNOTATED_DOC, "Every choice"),
        None,
    )
    .await;
    let (_, edits) = edits_of(&actions[0]);
    assert_eq!(edits.len(), 1);
    let body_len = parsed(ANNOTATED_DOC).body.len();
    assert_eq!(edits[0].range.start, lsp_position(ANNOTATED_DOC, body_len));
    assert_eq!(
        edits[0].range.end,
        lsp_position(ANNOTATED_DOC, ANNOTATED_DOC.len())
    );
    let saved = apply(ANNOTATED_DOC, &edits);
    assert!(saved.starts_with(&ANNOTATED_DOC[..body_len]));
}

#[tokio::test]
async fn a_malformed_block_offers_nothing() {
    let broken = ANNOTATED_DOC.replacen("\"version\": 1", "\"version\": 99", 1);
    let (service, _socket) = open_server(&broken, true).await;
    let server = service.inner();
    assert!(
        ghost_actions(server, range_of(&broken, "Every choice"), None)
            .await
            .is_empty()
    );
    assert!(
        ghost_actions(server, range_of(&broken, "Drop this aside."), None)
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn a_stale_action_carries_its_version_and_a_fresh_one_the_new_version() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let first = ghost_actions(server, range_of(PLAIN, "beta"), None).await;
    assert_eq!(edits_of(&first[0]).0, Some(1));
    // The document moves on: the client refuses the version-1 edit, and a
    // new request carries version 2.
    let changed = format!("{PLAIN} More.");
    change(server, &changed, 2).await;
    let second = ghost_actions(server, range_of(&changed, "beta"), None).await;
    assert_eq!(edits_of(&second[0]).0, Some(2));
    assert_ne!(edits_of(&first[0]).1, edits_of(&second[0]).1);
}

#[tokio::test]
async fn unresolvable_anchors_offer_nothing() {
    let stale = ANNOTATED_DOC.replacen(
        "Drop this aside.\",\n        \"before\"",
        "Drop this ASIDE.\",\n        \"before\"",
        1,
    );
    assert_ne!(stale, ANNOTATED_DOC);
    terraphim_alternatives::parse(&stale).expect("the block itself is well formed");
    let (service, _socket) = open_server(&stale, true).await;
    let server = service.inner();
    for needle in ["Every choice", "paperclip", "Drop this aside."] {
        assert!(
            ghost_actions(server, range_of(&stale, needle), None)
                .await
                .is_empty(),
            "{needle}"
        );
    }
}

#[tokio::test]
async fn ranges_are_utf16_after_astral_characters() {
    let body = "\u{1F600} Alpha \u{1F7E2} beta gamma";
    let (service, _socket) = open_server(body, true).await;
    let server = service.inner();
    let ghosted = run(
        server,
        body,
        range_of(body, "\u{1F7E2} beta"),
        "Ghost it",
        1,
    )
    .await;
    assert_eq!(faded(server, &ghosted).await, ["\u{1F7E2} beta"]);
    let split = run(
        server,
        &ghosted,
        range_of(&ghosted, "\u{1F7E2} "),
        "Revive",
        2,
    )
    .await;
    assert_eq!(faded(server, &split).await, ["beta"]);
    let none = run(server, &split, cursor_in(&split, "beta"), "Revive", 3).await;
    assert!(faded(server, &none).await.is_empty());
    assert_eq!(none, body);
}
