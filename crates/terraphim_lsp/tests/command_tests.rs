//! Inlay hints, ghost and Lab diagnostics, trim previews and
//! `workspace/executeCommand` through the tower-lsp service.
//!
//! No mocks: the server is built from the real fixture thesaurus committed
//! with `terraphim_lsp_core`, runs the real Lab engine with its embedded
//! lists, and is driven through `LanguageServer` (and, for pushed
//! diagnostics, through the service and its client socket).

use futures::StreamExt;
use serde_json::{Value, json};
use tower::{Service, ServiceExt};
use tower_lsp::jsonrpc::{ErrorCode, Request};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::commands;
use terraphim_lsp::core::{LineIndex, LinePosition};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json");
const ANNOTATED_DOC: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/annotated_doc.md");
const LAB_DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/lab_doc.md");

fn build_service() -> (LspService<TerraphimLspServer>, ClientSocket) {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).expect("fixture thesaurus");
    LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()))
}

fn uri() -> Url {
    Url::parse("file:///tmp/commands.md").unwrap()
}

/// Client capabilities that accept versioned `documentChanges`, with the
/// given `initializationOptions`.
fn client(options: Option<Value>) -> InitializeParams {
    InitializeParams {
        initialization_options: options,
        capabilities: ClientCapabilities {
            workspace: Some(WorkspaceClientCapabilities {
                workspace_edit: Some(WorkspaceEditClientCapabilities {
                    document_changes: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A fresh, initialised server with `text` open at version 1.
async fn open_server(
    text: &str,
    init: InitializeParams,
) -> (LspService<TerraphimLspServer>, ClientSocket) {
    let (service, socket) = build_service();
    service.inner().initialize(init).await.unwrap();
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

async fn save(server: &TerraphimLspServer) {
    server
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            text: None,
        })
        .await;
}

async fn execute(
    server: &TerraphimLspServer,
    command: &str,
    argument: Value,
) -> tower_lsp::jsonrpc::Result<Option<Value>> {
    server
        .execute_command(ExecuteCommandParams {
            command: command.to_string(),
            arguments: vec![argument],
            work_done_progress_params: Default::default(),
        })
        .await
}

/// The pulled diagnostics: the same set the server pushes.
async fn diagnostics(server: &TerraphimLspServer) -> Vec<Diagnostic> {
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
    match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.items
        }
        other => panic!("expected a full report, got {other:?}"),
    }
}

/// A diagnostic's string code; empty for the code-less unknown-term
/// warnings.
fn code_of(diagnostic: &Diagnostic) -> &str {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => code,
        None => "",
        other => panic!("expected a string code, got {other:?}"),
    }
}

fn with_code<'a>(diagnostics: &'a [Diagnostic], code: &str) -> Vec<&'a Diagnostic> {
    diagnostics.iter().filter(|d| code_of(d) == code).collect()
}

/// The text an LSP range covers.
fn covered(text: &str, range: Range) -> String {
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    text[byte(range.start)..byte(range.end)].to_string()
}

fn position_of(text: &str, needle: &str) -> Position {
    let byte = text.find(needle).unwrap_or_else(|| panic!("{needle:?}"));
    let LinePosition { line, character } = LineIndex::new(text).position(byte);
    Position { line, character }
}

fn range_of(text: &str, needle: &str) -> Range {
    let start = position_of(text, needle);
    let byte = text.find(needle).unwrap() + needle.len();
    let LinePosition { line, character } = LineIndex::new(text).position(byte);
    Range {
        start,
        end: Position { line, character },
    }
}

/// The single versioned text-document edit of a workspace edit.
fn versioned_edits(edit: &WorkspaceEdit) -> (Option<i32>, Vec<TextEdit>) {
    match edit.document_changes.as_ref().expect("document changes") {
        DocumentChanges::Edits(documents) => {
            assert_eq!(documents.len(), 1);
            assert_eq!(documents[0].text_document.uri, uri());
            let edits = documents[0]
                .edits
                .iter()
                .map(|edit| match edit {
                    OneOf::Left(edit) => edit.clone(),
                    OneOf::Right(annotated) => annotated.text_edit.clone(),
                })
                .collect();
            (documents[0].text_document.version, edits)
        }
        other => panic!("expected text-document edits, got {other:?}"),
    }
}

/// Apply LSP edits to `text`, last first, as an editor would.
fn apply(text: &str, edits: &[TextEdit]) -> String {
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| std::cmp::Reverse(byte(edit.range.start)));
    let mut out = text.to_string();
    for edit in sorted {
        out.replace_range(byte(edit.range.start)..byte(edit.range.end), &edit.new_text);
    }
    out
}

// ---------------------------------------------------------- capabilities --

#[tokio::test]
async fn initialize_advertises_inlay_hints_and_every_command() {
    let (service, _) = build_service();
    let response = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    assert_eq!(
        response.capabilities.inlay_hint_provider,
        Some(OneOf::Left(true))
    );
    let commands = response
        .capabilities
        .execute_command_provider
        .expect("executeCommandProvider")
        .commands;
    assert_eq!(
        commands,
        [
            "terraphim.alternative.add",
            "terraphim.lab.mark",
            "terraphim.lab.clear",
            "terraphim.trim.preview",
            "terraphim.trim.clear",
        ]
    );
}

// ----------------------------------------------------------- inlay hints --

async fn hints(server: &TerraphimLspServer, range: Range) -> Option<Vec<InlayHint>> {
    server
        .inlay_hint(InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range,
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
}

fn whole() -> Range {
    Range {
        start: Position::new(0, 0),
        end: Position::new(u32::MAX, 0),
    }
}

fn labels(hints: &[InlayHint]) -> Vec<String> {
    hints
        .iter()
        .map(|hint| match &hint.label {
            InlayHintLabel::String(label) => label.clone(),
            other => panic!("expected a string label, got {other:?}"),
        })
        .collect()
}

#[tokio::test]
async fn inlay_hints_are_off_by_default() {
    let (service, _socket) = open_server(ANNOTATED_DOC, client(None)).await;
    assert!(hints(service.inner(), whole()).await.is_none());
}

#[tokio::test]
async fn inlay_hints_on_through_initialization_options() {
    let (service, _socket) =
        open_server(ANNOTATED_DOC, client(Some(json!({"inlayHints": true})))).await;
    let all = hints(service.inner(), whole()).await.expect("hints");
    assert_eq!(labels(&all), ["[2/4]", "[3/4]", "[1/3]", "[1/2]"]);
    // Placed just after the term, in UTF-16 columns: "café" ends at column
    // 6 of line 1 ("A café").
    let cafe = &all[2];
    assert_eq!(cafe.position, Position::new(1, 6));
    assert_eq!(cafe.padding_left, Some(true));
    match &cafe.tooltip {
        Some(InlayHintTooltip::String(tooltip)) => {
            assert_eq!(tooltip, "Synonym 1 of 3 for café");
        }
        other => panic!("expected a tooltip, got {other:?}"),
    }
    // Only the requested range.
    let line_two = Range {
        start: Position::new(1, 0),
        end: Position::new(1, 40),
    };
    let some = hints(service.inner(), line_two).await.expect("hints");
    assert_eq!(labels(&some), ["[1/3]", "[1/2]"]);
}

#[tokio::test]
async fn inlay_hints_and_ghosts_follow_did_change_configuration() {
    let (service, _socket) = open_server(ANNOTATED_DOC, client(None)).await;
    let server = service.inner();
    assert_eq!(with_code(&diagnostics(server).await, "ghosted").len(), 1);
    server
        .did_change_configuration(DidChangeConfigurationParams {
            settings: json!({"terraphim": {"inlayHints": true, "ghostDiagnostics": false}}),
        })
        .await;
    assert_eq!(hints(server, whole()).await.expect("hints").len(), 4);
    assert!(with_code(&diagnostics(server).await, "ghosted").is_empty());
}

// ---------------------------------------------------------------- ghosts --

#[tokio::test]
async fn ghosts_are_published_as_unnecessary_hints() {
    let (service, _socket) = open_server(ANNOTATED_DOC, client(None)).await;
    let all = diagnostics(service.inner()).await;
    let ghosts = with_code(&all, "ghosted");
    assert_eq!(ghosts.len(), 1);
    let ghost = ghosts[0];
    assert_eq!(covered(ANNOTATED_DOC, ghost.range), "Drop this aside.");
    assert_eq!(ghost.severity, Some(DiagnosticSeverity::HINT));
    assert_eq!(ghost.tags, Some(vec![DiagnosticTag::UNNECESSARY]));
    // Columns count UTF-16 units: "A café visit is an honour. " is 27.
    assert_eq!(ghost.range.start, Position::new(1, 27));
}

#[tokio::test]
async fn ghost_hints_can_be_switched_off() {
    let (service, _socket) = open_server(
        ANNOTATED_DOC,
        client(Some(json!({"ghostDiagnostics": false}))),
    )
    .await;
    assert!(with_code(&diagnostics(service.inner()).await, "ghosted").is_empty());
}

// ------------------------------------------------------- add alternative --

fn add_args(text: &str, needle: &str, alternative: &str, version: Option<i32>) -> Value {
    let mut args = json!({
        "uri": uri(),
        "range": range_of(text, needle),
        "text": alternative,
    });
    if let Some(version) = version {
        args["version"] = json!(version);
    }
    args
}

#[tokio::test]
async fn add_alternative_returns_a_versioned_block_rewrite() {
    let (service, _socket) = open_server(ANNOTATED_DOC, client(None)).await;
    let result = execute(
        service.inner(),
        commands::ADD_ALTERNATIVE,
        add_args(ANNOTATED_DOC, "paperclip", "staple", Some(1)),
    )
    .await
    .unwrap()
    .expect("a workspace edit");
    let edit: WorkspaceEdit = serde_json::from_value(result).unwrap();
    let (version, edits) = versioned_edits(&edit);
    assert_eq!(version, Some(1));
    assert_eq!(edits.len(), 1);
    // The edit starts where the body ends: line 2, after "Pass me a
    // paperclip.".
    assert_eq!(edits[0].range.start, Position::new(2, 20));
    let saved = apply(ANNOTATED_DOC, &edits);
    let document = terraphim_alternatives::parse(&saved).expect("block parses");
    let before = terraphim_alternatives::parse(ANNOTATED_DOC).unwrap();
    assert_eq!(document.body, before.body);
    assert_eq!(document.annotations.ghosts, before.annotations.ghosts);
    let texts: Vec<&str> = document.annotations.spans[0]
        .alts
        .iter()
        .map(|alt| alt.text.as_str())
        .collect();
    assert_eq!(texts, ["paperclip", "binder clip", "staple"]);
}

#[tokio::test]
async fn add_alternative_on_a_plain_crlf_document_for_a_plain_client() {
    let text = "Every choice\r\nis a judgment.";
    let (service, _socket) = open_server(text, InitializeParams::default()).await;
    let result = execute(
        service.inner(),
        commands::ADD_ALTERNATIVE,
        json!({
            "uri": uri(),
            "range": range_of(text, "judgment"),
            "text": "call",
            "kind": "word",
        }),
    )
    .await
    .unwrap()
    .unwrap();
    let edit: WorkspaceEdit = serde_json::from_value(result).unwrap();
    assert!(edit.document_changes.is_none());
    let edits = edit.changes.expect("plain changes")[&uri()].clone();
    let saved = apply(text, &edits);
    let document = terraphim_alternatives::parse(&saved).unwrap();
    assert_eq!(document.body, text);
    assert_eq!(document.annotations.spans[0].anchor.text, "judgment");
}

#[tokio::test]
async fn add_alternative_refusals() {
    let (service, _socket) = open_server(ANNOTATED_DOC, client(None)).await;
    let server = service.inner();
    // Stale version.
    let stale = execute(
        server,
        commands::ADD_ALTERNATIVE,
        add_args(ANNOTATED_DOC, "paperclip", "staple", Some(7)),
    )
    .await
    .unwrap_err();
    assert_eq!(stale.code, ErrorCode::ContentModified);
    // The current text, a duplicate, and malformed arguments.
    for argument in [
        add_args(ANNOTATED_DOC, "paperclip", "paperclip", None),
        add_args(ANNOTATED_DOC, "paperclip", "binder clip", None),
        json!({"uri": uri()}),
    ] {
        let error = execute(server, commands::ADD_ALTERNATIVE, argument)
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidParams, "{error:?}");
    }
    let unknown = execute(server, "terraphim.nope", json!({}))
        .await
        .unwrap_err();
    assert_eq!(unknown.code, ErrorCode::InvalidParams);
}

#[tokio::test]
async fn add_alternative_never_rewrites_a_malformed_block() {
    let broken = ANNOTATED_DOC.replacen("\"version\": 1", "\"version\": 99", 1);
    let (service, _socket) = open_server(&broken, client(None)).await;
    let error = execute(
        service.inner(),
        commands::ADD_ALTERNATIVE,
        add_args(&broken, "choice", "option", None),
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidParams);
    assert!(error.message.contains("malformed"), "{}", error.message);
}

// ------------------------------------------------------------ Lab marks --

#[tokio::test]
async fn lab_marks_are_off_until_requested() {
    let (service, _socket) = open_server(LAB_DOC, client(None)).await;
    let all = diagnostics(service.inner()).await;
    assert!(all.iter().all(|d| !code_of(d).starts_with("lab-")));
}

#[tokio::test]
async fn lab_mark_publishes_one_action_with_fixes() {
    let (service, _socket) = open_server(LAB_DOC, client(None)).await;
    let server = service.inner();
    let result = execute(
        server,
        commands::LAB_MARK,
        json!({"uri": uri(), "action": "typos_and_punctuation", "version": 1}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        result,
        json!({"action": "typos_and_punctuation", "marks": 2})
    );
    let all = diagnostics(server).await;
    let typo = with_code(&all, "lab-typo");
    assert_eq!(typo.len(), 1);
    assert_eq!(covered(LAB_DOC, typo[0].range), "recieve");
    assert_eq!(typo[0].severity, Some(DiagnosticSeverity::INFORMATION));
    assert_eq!(typo[0].message, "typo: \"recieve\" -> \"receive\"");
    assert_eq!(with_code(&all, "lab-punctuation").len(), 1);
    assert!(with_code(&all, "lab-hedge").is_empty());

    // The quick fix, versioned like the synonym replacements.
    let actions = server
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range: typo[0].range,
            context: CodeActionContext {
                diagnostics: vec![],
                only: Some(vec![CodeActionKind::QUICKFIX]),
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap()
        .expect("a quick fix");
    let CodeActionOrCommand::CodeAction(fix) = &actions[0] else {
        panic!("expected a code action");
    };
    assert_eq!(actions.len(), 1);
    assert_eq!(fix.title, "Apply fix: receive");
    assert_eq!(fix.kind, Some(CodeActionKind::QUICKFIX));
    assert_eq!(
        fix.diagnostics.as_ref().unwrap()[0].message,
        typo[0].message
    );
    let (version, edits) = versioned_edits(fix.edit.as_ref().unwrap());
    assert_eq!(version, Some(1));
    assert!(apply(LAB_DOC, &edits).contains("We receive the café report"));
}

#[tokio::test]
async fn lab_marks_drop_on_change_return_on_save_and_clear() {
    let (service, _socket) = open_server(LAB_DOC, client(None)).await;
    let server = service.inner();
    execute(
        server,
        commands::LAB_MARK,
        json!({"uri": uri(), "action": "hedges_and_filler"}),
    )
    .await
    .unwrap();
    assert_eq!(with_code(&diagnostics(server).await, "lab-hedge").len(), 2);

    // Edits make the ranges stale: marks go until the next save.
    let edited = LAB_DOC.replacen("I think ", "", 1);
    change(server, &edited, 2).await;
    assert!(with_code(&diagnostics(server).await, "lab-hedge").is_empty());
    // A command against the old version is refused.
    let stale = execute(
        server,
        commands::LAB_MARK,
        json!({"uri": uri(), "action": "hedges_and_filler", "version": 1}),
    )
    .await
    .unwrap_err();
    assert_eq!(stale.code, ErrorCode::ContentModified);
    save(server).await;
    let hedges = with_code(&diagnostics(server).await, "lab-hedge")
        .into_iter()
        .map(|d| covered(&edited, d.range))
        .collect::<Vec<_>>();
    assert_eq!(hedges, ["perhaps"]);

    assert_eq!(
        execute(server, commands::LAB_CLEAR, json!({"uri": uri()}))
            .await
            .unwrap(),
        None
    );
    save(server).await;
    assert!(
        diagnostics(server)
            .await
            .iter()
            .all(|d| !code_of(d).starts_with("lab-"))
    );
}

#[tokio::test]
async fn configured_lab_actions_run_on_open_with_the_save_trigger() {
    let options = json!({"lab": {"actions": ["long_sentences"]}});
    let (service, _socket) = open_server(LAB_DOC, client(Some(options))).await;
    let long = diagnostics(service.inner()).await;
    let long = with_code(&long, "lab-long-sentence");
    assert_eq!(long.len(), 1);
    assert_eq!(long[0].message, "40 words (limit 30)");
    assert_eq!(long[0].severity, Some(DiagnosticSeverity::HINT));
}

#[tokio::test]
async fn the_command_trigger_never_runs_lab_actions_by_itself() {
    let options = json!({"lab": {"actions": ["long_sentences"], "trigger": "command"}});
    let (service, _socket) = open_server(LAB_DOC, client(Some(options))).await;
    let server = service.inner();
    save(server).await;
    assert!(with_code(&diagnostics(server).await, "lab-long-sentence").is_empty());
    // An explicit command runs the requested and the configured actions.
    execute(
        server,
        commands::LAB_MARK,
        json!({"uri": uri(), "action": "off_tone"}),
    )
    .await
    .unwrap();
    assert_eq!(
        with_code(&diagnostics(server).await, "lab-long-sentence").len(),
        1
    );
}

#[tokio::test]
async fn a_change_during_a_lab_command_wins() {
    // A large document so the Lab run (on the blocking pool) is still in
    // flight when the change is handled.
    let body = &LAB_DOC[..LAB_DOC.find("```").unwrap()];
    let text = body.repeat(300);
    let (service, _socket) = open_server(&text, client(None)).await;
    let server = service.inner();
    let edited = format!("Preface. {text}");
    let (marked, ()) = tokio::join!(
        execute(
            server,
            commands::LAB_MARK,
            json!({"uri": uri(), "action": "hedges_and_filler", "version": 1}),
        ),
        change(server, &edited, 2),
    );
    // The command's results were computed for version 1: refused, not
    // installed, so nothing stale is published.
    assert_eq!(marked.unwrap_err().code, ErrorCode::ContentModified);
    assert!(with_code(&diagnostics(server).await, "lab-hedge").is_empty());
    // The requested action stays; the next save computes it for version 2.
    save(server).await;
    let hedges = with_code(&diagnostics(server).await, "lab-hedge").len();
    assert_eq!(hedges, 2 * 300);
}

// ----------------------------------------------------------------- trim --

#[tokio::test]
async fn trim_preview_fades_candidates_until_cleared() {
    let (service, _socket) = open_server(LAB_DOC, client(None)).await;
    let server = service.inner();
    let result = execute(
        server,
        commands::TRIM_PREVIEW,
        json!({"uri": uri(), "level": "slight", "version": 1}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(
        result,
        json!({"level": "slight", "candidates": 5, "status": "61 \u{2192} 55 words \u{b7} \u{2212}10%"})
    );
    let all = diagnostics(server).await;
    let trimmed = with_code(&all, "trim-candidate");
    assert_eq!(trimmed.len(), 5);
    assert_eq!(covered(LAB_DOC, trimmed[0].range), " basically");
    assert_eq!(trimmed[0].tags, Some(vec![DiagnosticTag::UNNECESSARY]));
    assert_eq!(trimmed[0].severity, Some(DiagnosticSeverity::HINT));
    assert_eq!(trimmed[0].message, "Slight trim: filler \"basically\"");

    // A higher level replaces the lower one.
    execute(
        server,
        commands::TRIM_PREVIEW,
        json!({"uri": uri(), "level": "sharper"}),
    )
    .await
    .unwrap();
    assert_eq!(
        with_code(&diagnostics(server).await, "trim-candidate").len(),
        3
    );

    execute(server, commands::TRIM_CLEAR, json!({"uri": uri()}))
        .await
        .unwrap();
    assert!(with_code(&diagnostics(server).await, "trim-candidate").is_empty());
    // "original" previews nothing.
    let original = execute(
        server,
        commands::TRIM_PREVIEW,
        json!({"uri": uri(), "level": "original"}),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(original["candidates"], json!(0));
}

#[tokio::test]
async fn commands_on_unopened_documents_are_invalid() {
    let (service, _socket) = build_service();
    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    for (command, argument) in [
        (
            commands::LAB_MARK,
            json!({"uri": uri(), "action": "off_tone"}),
        ),
        (
            commands::TRIM_PREVIEW,
            json!({"uri": uri(), "level": "half"}),
        ),
        (commands::TRIM_CLEAR, json!({"uri": uri()})),
    ] {
        let error = execute(service.inner(), command, argument)
            .await
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidParams);
    }
}

// ------------------------------------------------------ pushed diagnostics --

/// Forward every client-bound message to a channel, so the server never
/// waits on the socket while the test is busy.
fn drain(mut socket: ClientSocket) -> tokio::sync::mpsc::UnboundedReceiver<Request> {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Some(request) = socket.next().await {
            if sender.send(request).is_err() {
                break;
            }
        }
    });
    receiver
}

/// The next `textDocument/publishDiagnostics` carrying `code`.
async fn next_publish_with(
    messages: &mut tokio::sync::mpsc::UnboundedReceiver<Request>,
    code: &str,
) -> PublishDiagnosticsParams {
    while let Some(request) = messages.recv().await {
        if request.method() != "textDocument/publishDiagnostics" {
            continue;
        }
        let params: PublishDiagnosticsParams =
            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
        if params.diagnostics.iter().any(|d| code_of(d) == code) {
            return params;
        }
    }
    panic!("no publishDiagnostics with {code}");
}

#[tokio::test]
async fn trim_preview_is_pushed_with_unnecessary_tags() {
    let (mut service, socket) = build_service();
    let mut messages = drain(socket);
    // Drive initialize and initialized through the service, so the client
    // is in the initialised state and notifications are really sent.
    let initialize = Request::build("initialize")
        .params(serde_json::to_value(client(None)).unwrap())
        .id(1)
        .finish();
    service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .unwrap();
    let initialized = Request::build("initialized").params(json!({})).finish();
    service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .unwrap();
    let server = service.inner();
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 3,
                text: LAB_DOC.to_string(),
            },
        })
        .await;
    execute(
        server,
        commands::TRIM_PREVIEW,
        json!({"uri": uri(), "level": "slight"}),
    )
    .await
    .unwrap();
    let pushed = next_publish_with(&mut messages, "trim-candidate").await;
    assert_eq!(pushed.uri, uri());
    assert_eq!(pushed.version, Some(3));
    let trimmed = with_code(&pushed.diagnostics, "trim-candidate");
    assert_eq!(trimmed.len(), 5);
    assert!(
        trimmed
            .iter()
            .all(|d| d.tags == Some(vec![DiagnosticTag::UNNECESSARY]))
    );
}
