//! "Stash in Overflow", "Pull back" (R-6.3, R-6.4) and "Export clean
//! Markdown" (R-9.3) code actions, and R-6.5 (overflow survives every
//! rewrite of the annotation block), through the tower-lsp service.
//!
//! No mocks: the server is built from the committed fixture thesaurus. Each
//! action's edit is applied to the open document and sent back with
//! `didChange`, so the next request sees what the client would.

use serde_json::{Value, json};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::commands;
use terraphim_lsp::core::{LineIndex, LinePosition};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
/// A span over "paperclip" and a ghost over "Drop this aside."
const ANNOTATED_DOC: &str = include_str!("../tests/fixtures/annotated_doc.md");

fn uri() -> Url {
    Url::parse("file:///tmp/overflow-actions.md").unwrap()
}

fn overflow_kind() -> CodeActionKind {
    CodeActionKind::new("refactor.terraphim.overflow")
}

fn export_kind() -> CodeActionKind {
    CodeActionKind::new("source.terraphim.export")
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

fn range_of(text: &str, needle: &str) -> Range {
    let start = text.find(needle).unwrap_or_else(|| panic!("{needle:?}"));
    Range {
        start: lsp_position(text, start),
        end: lsp_position(text, start + needle.len()),
    }
}

fn cursor_at(text: &str, byte: usize) -> Range {
    let at = lsp_position(text, byte);
    Range { start: at, end: at }
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
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action,
            other => panic!("expected a code action, got {other:?}"),
        })
        .collect()
}

/// The overflow actions at `range`.
async fn overflow_actions(server: &TerraphimLspServer, range: Range) -> Vec<CodeAction> {
    actions(server, range, Some(vec![overflow_kind()])).await
}

fn titles(actions: &[CodeAction]) -> Vec<&str> {
    actions.iter().map(|a| a.title.as_str()).collect()
}

fn action_titled<'a>(actions: &'a [CodeAction], title: &str) -> &'a CodeAction {
    actions
        .iter()
        .find(|a| a.title == title)
        .unwrap_or_else(|| panic!("{title:?} not in {:?}", titles(actions)))
}

/// The version and edits of an action's workspace edit (versioned).
fn edits_of(action: &CodeAction) -> (Option<i32>, Vec<TextEdit>) {
    assert!(action.command.is_none(), "an edit, not a command");
    let edit = action.edit.as_ref().expect("an edit");
    let Some(DocumentChanges::Edits(edits)) = edit.document_changes.as_ref() else {
        panic!("versioned document changes");
    };
    assert_eq!(edits.len(), 1);
    let edits_in = edits[0]
        .edits
        .iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(annotated) => annotated.text_edit.clone(),
        })
        .collect();
    (edits[0].text_document.version, edits_in)
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

/// Apply `action` to `text` and send the result back as the client would.
async fn run(server: &TerraphimLspServer, action: &CodeAction, text: &str, version: i32) -> String {
    let (edit_version, edits) = edits_of(action);
    assert_eq!(
        edit_version,
        Some(version),
        "pinned to the document version"
    );
    let applied = apply(text, &edits);
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri(),
                version: version + 1,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: applied.clone(),
            }],
        })
        .await;
    applied
}

fn parse(text: &str) -> terraphim_alternatives::Document {
    terraphim_alternatives::parse(text).expect("a well-formed block")
}

const PLAIN: &str = "Alpha beta gamma delta.\nSecond line here.";

fn with_overflow(body: &str, overflow: &str) -> String {
    format!(
        "{body}\n\n```terraphim-alternatives\n{{\"version\": 1, \"spans\": [], \"ghosts\": [], \"overflow\": {}}}\n```\n",
        serde_json::to_string(overflow).unwrap()
    )
}

// ---------------------------------------------------------------- stash --

#[tokio::test]
async fn a_selection_can_be_stashed_and_pulled_back() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let stash = overflow_actions(server, range_of(PLAIN, "gamma ")).await;
    assert_eq!(titles(&stash), ["Stash in Overflow"]);
    assert_eq!(stash[0].kind, Some(overflow_kind()));
    let stashed = run(server, &stash[0], PLAIN, 1).await;
    let document = parse(&stashed);
    assert_eq!(document.body, "Alpha beta delta.\nSecond line here.");
    assert_eq!(document.annotations.overflow, "gamma ");

    // The paragraph is now offered at the cursor.
    let at = document.body.find("delta").unwrap();
    let pulls = overflow_actions(server, cursor_at(&stashed, at)).await;
    assert_eq!(titles(&pulls), ["Pull back: «gamma»"]);
    let back = run(server, &pulls[0], &stashed, 2).await;
    let document = parse(&back);
    assert_eq!(document.body, "Alpha beta gamma delta.\nSecond line here.");
    assert_eq!(
        document.annotations.overflow, "gamma ",
        "a copy: the overflow keeps the text"
    );
    // The paragraph is still on offer, to use again.
    let again = overflow_actions(server, cursor_at(&back, 0)).await;
    assert_eq!(titles(&again), ["Pull back: «gamma»"]);
}

#[tokio::test]
async fn nothing_to_stash_without_a_selection() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let found = overflow_actions(service.inner(), cursor_at(PLAIN, 3)).await;
    assert!(found.is_empty(), "{:?}", titles(&found));
}

#[tokio::test]
async fn a_stash_touching_a_span_is_not_offered() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    // "a paperclip" overlaps the span over "paperclip".
    let touching = overflow_actions(server, range_of(ANNOTATED_DOC, "a paperclip")).await;
    assert!(touching.is_empty(), "{:?}", titles(&touching));
    // Other text is fine, and stashing it moves the span with the text.
    let other = overflow_actions(server, range_of(ANNOTATED_DOC, "Every choice")).await;
    assert_eq!(titles(&other), ["Stash in Overflow"]);
    let stashed = run(server, &other[0], ANNOTATED_DOC, 1).await;
    let document = parse(&stashed);
    assert_eq!(document.annotations.spans[0].anchor.text, "paperclip");
    assert!(document.body.contains("paperclip"));
}

#[tokio::test]
async fn pull_back_is_not_offered_inside_a_span() {
    let text = ANNOTATED_DOC.replacen(
        "\"spans\": [",
        "\"overflow\": \"Spare line.\",\n  \"spans\": [",
        1,
    );
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let inside = text.find("paperclip").unwrap() + 3;
    assert!(
        overflow_actions(server, cursor_at(&text, inside))
            .await
            .is_empty()
    );
    let outside = overflow_actions(server, cursor_at(&text, 3)).await;
    assert_eq!(titles(&outside), ["Pull back: «Spare line.»"]);
}

#[tokio::test]
async fn pull_back_offers_the_first_five_paragraphs_with_collapsed_excerpts() {
    let overflow = "One.\n\nTwo\nlines here.\n\nThree.\n\nFour.\n\nFive.\n\nSix.";
    let text = with_overflow(PLAIN, overflow);
    let (service, _socket) = open_server(&text, true).await;
    let pulls = overflow_actions(service.inner(), cursor_at(&text, 0)).await;
    assert_eq!(
        titles(&pulls),
        [
            "Pull back: «One.»",
            "Pull back: «Two lines here.»",
            "Pull back: «Three.»",
            "Pull back: «Four.»",
            "Pull back: «Five.»",
        ]
    );
    // A pull back copies: the overflow is left as it was.
    let second = run(service.inner(), &pulls[1], &text, 1).await;
    assert_eq!(parse(&second).annotations.overflow, overflow);
    assert!(parse(&second).body.starts_with("Two\nlines here.\n\nAlpha"));
}

#[tokio::test]
async fn long_paragraphs_get_a_short_title() {
    let long = "word ".repeat(30);
    let text = with_overflow(PLAIN, long.trim());
    let (service, _socket) = open_server(&text, true).await;
    let pulls = overflow_actions(service.inner(), cursor_at(&text, 0)).await;
    assert!(pulls[0].title.chars().count() < 60, "{}", pulls[0].title);
    assert!(pulls[0].title.ends_with("\u{2026}»"));
}

#[tokio::test]
async fn malformed_and_unresolved_blocks_offer_nothing() {
    let malformed = format!("{PLAIN}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    let (service, _socket) = open_server(&malformed, true).await;
    assert!(
        overflow_actions(service.inner(), range_of(&malformed, "beta"))
            .await
            .is_empty()
    );
    let lost = "Totally different.\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"overflow\": \"x\", \"ghosts\": [{\"id\": \"g1\", \"anchor\": {\"start\": 0, \"end\": 4, \"text\": \"gone\"}}]}\n```\n";
    let (service, _socket) = open_server(lost, true).await;
    assert!(
        overflow_actions(service.inner(), range_of(lost, "different"))
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn an_unversioned_client_gets_plain_changes() {
    let (service, _socket) = open_server(PLAIN, false).await;
    let stash = overflow_actions(service.inner(), range_of(PLAIN, "beta")).await;
    let edit = stash[0].edit.as_ref().unwrap();
    assert!(edit.document_changes.is_none());
    let changes = edit.changes.as_ref().unwrap();
    let edits = &changes[&uri()];
    assert_eq!(edits.len(), 2, "the body change and the block");
    assert_eq!(parse(&apply(PLAIN, edits)).annotations.overflow, "beta");
}

#[tokio::test]
async fn the_only_filter_selects_the_overflow_kind() {
    let text = with_overflow(PLAIN, "Spare.");
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let selection = range_of(&text, "beta");
    for only in [
        vec![overflow_kind()],
        vec![CodeActionKind::REFACTOR],
        vec![CodeActionKind::new("refactor.terraphim")],
    ] {
        let found = actions(server, selection, Some(only)).await;
        let overflow: Vec<_> = found
            .iter()
            .filter(|a| a.kind == Some(overflow_kind()))
            .collect();
        assert_eq!(overflow.len(), 2, "stash and pull back");
    }
    for only in [
        vec![CodeActionKind::QUICKFIX],
        vec![CodeActionKind::new("refactor.terraphim.ghost")],
        vec![CodeActionKind::new("refactor.terraphim.over")],
        vec![CodeActionKind::SOURCE],
    ] {
        let found = actions(server, selection, Some(only)).await;
        assert!(
            found.iter().all(|a| a.kind != Some(overflow_kind())),
            "excluded by `only`"
        );
    }
}

// ----------------------------------------------------------------- export --

#[tokio::test]
async fn export_is_a_command_only_source_action() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    for only in [
        None,
        Some(vec![export_kind()]),
        Some(vec![CodeActionKind::SOURCE]),
    ] {
        let found = actions(server, cursor_at(ANNOTATED_DOC, 0), only).await;
        let export = action_titled(&found, "Export clean Markdown");
        assert_eq!(export.kind, Some(export_kind()));
        assert!(export.edit.is_none());
        let command = export.command.as_ref().unwrap();
        assert_eq!(command.command, commands::EXPORT);
        assert_eq!(command.arguments, Some(vec![json!({"uri": uri()})]));
    }
    // Run it as Zed does: the command's own arguments.
    let found = actions(
        server,
        cursor_at(ANNOTATED_DOC, 0),
        Some(vec![export_kind()]),
    )
    .await;
    let command = found[0].command.clone().unwrap();
    let result = server
        .execute_command(ExecuteCommandParams {
            command: command.command,
            arguments: command.arguments.unwrap(),
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap();
    let markdown = result["markdown"].as_str().unwrap();
    assert!(markdown.contains("Pass me a paperclip."));
    assert!(!markdown.contains("Drop this aside"));
    assert!(!markdown.contains("terraphim-alternatives"));
}

#[tokio::test]
async fn export_is_not_offered_when_it_would_change_nothing_or_be_refused() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let found = actions(service.inner(), cursor_at(PLAIN, 0), None).await;
    assert!(found.iter().all(|a| a.kind != Some(export_kind())));

    let malformed = format!("{PLAIN}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    let (service, _socket) = open_server(&malformed, true).await;
    let found = actions(service.inner(), cursor_at(&malformed, 0), None).await;
    assert!(found.iter().all(|a| a.kind != Some(export_kind())));

    let lost = "Totally different.\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"ghosts\": [{\"id\": \"g1\", \"anchor\": {\"start\": 0, \"end\": 4, \"text\": \"gone\"}}]}\n```\n";
    let (service, _socket) = open_server(lost, true).await;
    let found = actions(service.inner(), cursor_at(lost, 0), None).await;
    assert!(found.iter().all(|a| a.kind != Some(export_kind())));
}

#[tokio::test]
async fn the_export_follows_the_text_after_a_stash() {
    let (service, _socket) = open_server(PLAIN, true).await;
    let server = service.inner();
    let stash = overflow_actions(server, range_of(PLAIN, "Second line here.")).await;
    let stashed = run(server, &stash[0], PLAIN, 1).await;
    let found = actions(server, cursor_at(&stashed, 0), Some(vec![export_kind()])).await;
    // The document now has an overflow block, so the export differs.
    let command = found[0].command.clone().unwrap();
    assert_eq!(command.arguments, Some(vec![json!({"uri": uri()})]));
    let result: Value = server
        .execute_command(ExecuteCommandParams {
            command: command.command,
            arguments: command.arguments.unwrap(),
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["markdown"], json!("Alpha beta gamma delta."));
}

// ------------------------------------------------------------------ R-6.5 --

#[tokio::test]
async fn overflow_survives_every_rewrite_of_the_block() {
    let overflow = "Keep me.\n\n```fence``` and https://example.com";
    let text = with_overflow(PLAIN, overflow);
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let mut text = text;
    let mut version = 1;

    // Ghost it.
    let ghost = actions(
        server,
        range_of(&text, "gamma"),
        Some(vec![CodeActionKind::new("refactor.terraphim.ghost")]),
    )
    .await;
    text = run(server, action_titled(&ghost, "Ghost it"), &text, version).await;
    version += 1;
    assert_eq!(parse(&text).annotations.overflow, overflow);

    // Revive.
    let revive = actions(
        server,
        range_of(&text, "gamma"),
        Some(vec![CodeActionKind::new("refactor.terraphim.ghost")]),
    )
    .await;
    text = run(server, action_titled(&revive, "Revive"), &text, version).await;
    version += 1;
    assert_eq!(parse(&text).annotations.overflow, overflow);

    // Add an alternative (the command returns the edit).
    let added = server
        .execute_command(ExecuteCommandParams {
            command: commands::ADD_ALTERNATIVE.to_string(),
            arguments: vec![json!({
                "uri": uri(), "range": range_of(&text, "beta"),
                "text": "bravo", "version": version,
            })],
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap();
    let edit: WorkspaceEdit = serde_json::from_value(added).unwrap();
    let Some(DocumentChanges::Edits(edits)) = edit.document_changes else {
        panic!("versioned edit")
    };
    let edits: Vec<TextEdit> = edits[0]
        .edits
        .iter()
        .map(|e| match e {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(annotated) => annotated.text_edit.clone(),
        })
        .collect();
    text = apply(&text, &edits);
    version += 1;
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri(),
                version,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.clone(),
            }],
        })
        .await;
    assert_eq!(parse(&text).annotations.overflow, overflow);

    // Stash and pull back: the stashed text joins the overflow, the rest stays.
    let stash = overflow_actions(server, range_of(&text, "Second line here.")).await;
    text = run(
        server,
        action_titled(&stash, "Stash in Overflow"),
        &text,
        version,
    )
    .await;
    version += 1;
    let stashed = parse(&text).annotations.overflow;
    assert!(stashed.starts_with(overflow));
    assert!(stashed.ends_with("Second line here."));
    let pulls = overflow_actions(server, cursor_at(&text, 0)).await;
    let last = pulls
        .iter()
        .rfind(|a| a.title.contains("Second line"))
        .expect("the stashed paragraph");
    text = run(server, last, &text, version).await;
    assert_eq!(
        parse(&text).annotations.overflow,
        stashed,
        "a pull back copies"
    );
    assert_eq!(parse(&text).annotations.spans[0].alts.len(), 2);
}

#[tokio::test]
async fn a_cached_export_action_exports_the_current_text() {
    let (service, _socket) = open_server(ANNOTATED_DOC, true).await;
    let server = service.inner();
    let found = actions(
        server,
        cursor_at(ANNOTATED_DOC, 0),
        Some(vec![export_kind()]),
    )
    .await;
    let command = found[0].command.clone().unwrap();
    // The document changes after Zed cached the action.
    let edited = ANNOTATED_DOC.replacen("Every choice", "Each choice", 1);
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: edited,
            }],
        })
        .await;
    let result = server
        .execute_command(ExecuteCommandParams {
            command: command.command,
            arguments: command.arguments.unwrap(),
            work_done_progress_params: Default::default(),
        })
        .await
        .expect("not ContentModified")
        .unwrap();
    assert!(
        result["markdown"]
            .as_str()
            .unwrap()
            .starts_with("Each choice")
    );
}

/// The body after running the "Pull back" action for the only paragraph,
/// "Para.", of `body` with the cursor at `at`.
async fn pulled(body: &str, at: usize) -> String {
    let text = with_overflow(body, "Para.");
    let (service, _socket) = open_server(&text, true).await;
    let server = service.inner();
    let pulls = overflow_actions(server, cursor_at(&text, at)).await;
    assert_eq!(titles(&pulls), ["Pull back: «Para.»"]);
    let saved = run(server, &pulls[0], &text, 1).await;
    assert_eq!(parse(&saved).annotations.overflow, "Para.", "a copy");
    parse(&saved).body
}

#[tokio::test]
async fn pull_back_makes_its_own_paragraph_at_a_line_start_or_end() {
    let doc = "First line.\nSecond line.";
    assert_eq!(
        pulled(doc, doc.find("Second").unwrap()).await,
        "First line.\n\nPara.\n\nSecond line.",
        "line start"
    );
    assert_eq!(
        pulled(doc, "First line.".len()).await,
        "First line.\n\nPara.\n\nSecond line.",
        "line end"
    );
    assert_eq!(
        pulled("Body.", 5).await,
        "Body.\n\nPara.",
        "end of the body"
    );
}

#[tokio::test]
async fn pull_back_between_paragraphs_leaves_one_blank_line_each_side() {
    let doc = "One.\n\nTwo.";
    for at in [4, 5, 6] {
        assert_eq!(
            pulled(doc, at).await,
            "One.\n\nPara.\n\nTwo.",
            "cursor at {at}"
        );
    }
}

#[tokio::test]
async fn pull_back_mid_line_stays_inline() {
    assert_eq!(pulled("Start end.", 5).await, "StartPara. end.");
}

#[tokio::test]
async fn pull_back_follows_crlf_documents() {
    assert_eq!(pulled("A.\r\nB.", 4).await, "A.\r\n\r\nPara.\r\n\r\nB.");
}
