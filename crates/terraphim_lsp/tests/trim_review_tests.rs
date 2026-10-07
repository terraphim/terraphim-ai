//! The full trim review in a Zed-shaped client (#3449, R-8.3 to R-8.5):
//! five levels, the status card (`window/showMessageRequest`), Keep, Make
//! the cuts (`workspace/applyEdit`), Walk through (`window/showDocument`)
//! and Done.
//!
//! No mocks: the server runs the real Lab engine on a committed 480-word
//! document and is driven through its tower `Service`, so tower-lsp is in
//! the initialised state and really sends requests to the client. The test
//! plays the client: it reads every server message off the client socket
//! and answers the server's requests through the same socket, as Zed does.

use std::collections::VecDeque;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::jsonrpc::{ErrorCode, Request, Response};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, Loopback, LspService};

/// The half of the client socket that answers the server's requests.
type ResponseSink = <ClientSocket as Loopback>::ResponseSink;

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::commands;
use terraphim_lsp::core::{
    CutId, LabConfig, LineIndex, LinePosition, TrimLevel, trim_cuts, trim_plan_for,
};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json");
/// A 713-word prose document (paragraphs of terraphim-editor's
/// alternative-control.md and sublime-plugin-fit.md), measured by the lead
/// on terraphim_lsp-v1.22.0-rc.1: 10/20/31/50%.
const LONG_PROSE: &str = include_str!("fixtures/trim_long_prose.md");

/// A realistic document of about 480 words (not the 61-word Lab sample).
const DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/trim_review_doc.md");

const TRIM_KIND: &str = "refactor.terraphim.trim";
const HINT: &str = "Faded words would go. Keep one with the Keep action.";

fn uri() -> Url {
    Url::parse("file:///tmp/review.md").unwrap()
}

// ---------------------------------------------------------------- client --

/// What the client supports.
#[derive(Clone, Copy)]
struct Caps {
    pull: bool,
    prompt: bool,
    show_document: bool,
    apply_edit: bool,
}

const ZED: Caps = Caps {
    pull: true,
    prompt: true,
    show_document: true,
    apply_edit: true,
};

fn capabilities(caps: Caps) -> Value {
    let mut value = json!({
        "workspace": {
            "applyEdit": caps.apply_edit,
            "workspaceEdit": {"documentChanges": true},
            "diagnostics": {"refreshSupport": true},
            "diagnostic": {"refreshSupport": true},
        },
        "window": {
            "showDocument": {"support": caps.show_document},
        },
    });
    if caps.pull {
        value["textDocument"] = json!({"diagnostic": {"dynamicRegistration": false}});
    }
    if caps.prompt {
        value["window"]["showMessage"] =
            json!({"messageActionItem": {"additionalPropertiesSupport": false}});
    }
    value
}

/// A Zed-shaped client session: the server, the messages it sent, and the
/// socket half used to answer its requests.
struct Session {
    service: LspService<TerraphimLspServer>,
    inbox: UnboundedReceiver<Request>,
    replies: ResponseSink,
    /// Messages received but not consumed yet, in order.
    pending: VecDeque<Request>,
    text: String,
    version: i32,
}

impl Session {
    async fn start(caps: Caps) -> Self {
        let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
        let (mut service, socket) =
            LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
        let (mut stream, replies) = socket.split();
        let (sender, inbox) = unbounded_channel();
        tokio::spawn(async move {
            while let Some(request) = stream.next().await {
                if sender.send(request).is_err() {
                    break;
                }
            }
        });
        let params =
            json!({"processId": null, "rootUri": null, "capabilities": capabilities(caps)});
        let initialize = Request::build("initialize").params(params).id(1).finish();
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
        let session = Self {
            service,
            inbox,
            replies,
            pending: VecDeque::new(),
            text: DOC.to_string(),
            version: 1,
        };
        session
            .server()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri(),
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: DOC.to_string(),
                },
            })
            .await;
        session
    }

    fn server(&self) -> &TerraphimLspServer {
        self.service.inner()
    }

    /// The next server message with `method`; others stay pending.
    async fn next(&mut self, method: &str) -> Request {
        if let Some(at) = self.pending.iter().position(|m| m.method() == method) {
            return self.pending.remove(at).unwrap();
        }
        loop {
            let request = tokio::time::timeout(Duration::from_secs(20), self.inbox.recv())
                .await
                .unwrap_or_else(|_| panic!("no {method} from the server"))
                .expect("socket open");
            if request.method() == method {
                return request;
            }
            self.keep_pending(request).await;
        }
    }

    /// Answer refresh requests at once; keep everything else.
    async fn keep_pending(&mut self, request: Request) {
        if request.method() == "workspace/diagnostic/refresh" {
            self.reply(&request, Value::Null).await;
        } else {
            self.pending.push_back(request);
        }
    }

    /// Wait until the server has been quiet for a moment; the messages
    /// received meanwhile (also left pending).
    async fn quiet(&mut self) -> Vec<Request> {
        let mut seen = Vec::new();
        while let Ok(Some(request)) =
            tokio::time::timeout(Duration::from_millis(300), self.inbox.recv()).await
        {
            seen.push(request.clone());
            self.keep_pending(request).await;
        }
        seen
    }

    async fn reply(&mut self, request: &Request, result: Value) {
        let id = request.id().cloned().expect("a request");
        self.replies
            .send(Response::from_ok(id, result))
            .await
            .unwrap();
    }

    /// The trim code actions at `position`, as Zed requests them.
    async fn actions_at(&self, position: Position) -> Vec<CodeAction> {
        let actions = self
            .server()
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                range: Range {
                    start: position,
                    end: position,
                },
                context: CodeActionContext {
                    diagnostics: vec![],
                    only: Some(vec![CodeActionKind::new(TRIM_KIND)]),
                    trigger_kind: None,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            })
            .await
            .unwrap()
            .unwrap_or_default();
        actions
            .into_iter()
            .map(|action| match action {
                CodeActionOrCommand::CodeAction(action) => action,
                other => panic!("expected a code action, got {other:?}"),
            })
            .collect()
    }

    /// Run the command of the action titled `title` at `position`, exactly
    /// as Zed does; the command's result.
    async fn run(&self, position: Position, title: &str) -> Value {
        let action = self
            .actions_at(position)
            .await
            .into_iter()
            .find(|action| action.title == title)
            .unwrap_or_else(|| panic!("no action {title:?}"));
        assert!(action.edit.is_none(), "{title}: a command-only action");
        let command = action.command.expect("a command");
        self.execute(&command.command, command.arguments.unwrap_or_default())
            .await
            .unwrap()
            .unwrap_or(Value::Null)
    }

    async fn execute(
        &self,
        command: &str,
        arguments: Vec<Value>,
    ) -> tower_lsp::jsonrpc::Result<Option<Value>> {
        self.server()
            .execute_command(ExecuteCommandParams {
                command: command.to_string(),
                arguments,
                work_done_progress_params: Default::default(),
            })
            .await
    }

    async fn preview(&mut self, level: &str) -> Value {
        let title = level_title(level);
        self.run(Position::default(), &title).await
    }

    /// The faded trim hints the client shows: pulled, or the last push.
    async fn fades(&mut self, caps: Caps) -> Vec<Diagnostic> {
        let diagnostics = if caps.pull {
            let report = self
                .server()
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
        } else {
            // The latest push, once the server has settled.
            self.quiet().await;
            let pushes: Vec<Request> = self
                .pending
                .iter()
                .filter(|m| m.method() == "textDocument/publishDiagnostics")
                .cloned()
                .collect();
            self.pending
                .retain(|m| m.method() != "textDocument/publishDiagnostics");
            let push = pushes.last().expect("a push");
            let params: PublishDiagnosticsParams =
                serde_json::from_value(push.params().cloned().unwrap()).unwrap();
            params.diagnostics
        };
        diagnostics
            .into_iter()
            .filter(|d| d.code == Some(NumberOrString::String("trim-candidate".to_string())))
            .collect()
    }

    /// Edit the document as the client does after applying an edit.
    async fn change(&mut self, text: String) {
        self.version += 1;
        self.text = text;
        self.server()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version: self.version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: self.text.clone(),
                }],
            })
            .await;
    }

    async fn save(&self) {
        self.server()
            .did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                text: None,
            })
            .await;
    }
}

fn level_title(level: &str) -> String {
    match level {
        "original" => "Trim: Original",
        "slight" => "Trim: Slight trim ~10%",
        "tighten" => "Trim: Tighten more ~20%",
        "sharper" => "Trim: Even sharper ~30%",
        "half" => "Trim: Cut in half ~50%",
        other => panic!("{other}"),
    }
    .to_string()
}

/// The status card a `window/showMessageRequest` carries.
struct Card {
    request: Request,
    params: ShowMessageRequestParams,
}

impl Card {
    fn titles(&self) -> Vec<String> {
        self.params
            .actions
            .iter()
            .flatten()
            .map(|action| action.title.clone())
            .collect()
    }

    /// `(before, after)` from `479 → 433 words · −10%`.
    fn words(&self) -> (usize, usize) {
        let line = &self.params.message;
        let numbers: Vec<usize> = line
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(|part| part.parse().unwrap())
            .collect();
        (numbers[0], numbers[1])
    }
}

async fn card(session: &mut Session) -> Card {
    let request = session.next("window/showMessageRequest").await;
    let params = serde_json::from_value(request.params().cloned().unwrap()).unwrap();
    Card { request, params }
}

async fn choose(session: &mut Session, card: &Card, title: Option<&str>) {
    let result = title.map_or(Value::Null, |title| json!({"title": title}));
    session.reply(&card.request, result).await;
}

fn byte_of(text: &str, position: Position) -> usize {
    LineIndex::new(text).byte_offset(LinePosition {
        line: position.line,
        character: position.character,
    })
}

/// Apply LSP edits last first, as an editor does.
fn apply(text: &str, edits: &[TextEdit]) -> String {
    let mut sorted: Vec<&TextEdit> = edits.iter().collect();
    sorted.sort_by_key(|edit| std::cmp::Reverse(byte_of(text, edit.range.start)));
    let mut out = text.to_string();
    for edit in sorted {
        out.replace_range(
            byte_of(text, edit.range.start)..byte_of(text, edit.range.end),
            &edit.new_text,
        );
    }
    out
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

/// What "Make the cuts" must produce: the engine's edits for `text` at
/// `level` with `kept`, as LSP edits.
fn expected_cuts(text: &str, level: TrimLevel, kept: &[CutId]) -> (Vec<TextEdit>, String) {
    let config = LabConfig::with_defaults().unwrap();
    let plan = trim_plan_for(text, &config);
    let made = trim_cuts(text, &plan, level, kept);
    let index = LineIndex::new(text);
    let position = |byte| {
        let LinePosition { line, character } = index.position(byte);
        Position { line, character }
    };
    let edits = made
        .edits
        .iter()
        .map(|edit| TextEdit {
            range: Range {
                start: position(edit.range.start.byte),
                end: position(edit.range.end.byte),
            },
            new_text: edit.new_text.clone(),
        })
        .collect();
    (edits, made.text)
}

fn lab_words(text: &str) -> usize {
    trim_plan_for(text, &LabConfig::with_defaults().unwrap()).total_words()
}

// ----------------------------------------------------------------- tests --

#[tokio::test]
async fn the_menu_offers_five_levels_and_the_review_actions() {
    let session = Session::start(ZED).await;
    let titles: Vec<String> = session
        .actions_at(Position::default())
        .await
        .into_iter()
        .map(|action| action.title)
        .collect();
    assert_eq!(
        titles,
        [
            "Trim: Original",
            "Trim: Slight trim ~10%",
            "Trim: Tighten more ~20%",
            "Trim: Even sharper ~30%",
            "Trim: Cut in half ~50%",
            "Trim: Make the cuts",
            "Trim: Walk through",
        ]
    );
    // The level titles are the engine's own labels.
    for (title, level) in titles.iter().zip(TrimLevel::ALL) {
        assert!(title.contains(level.label()), "{title} / {}", level.label());
    }
}

/// Preview each level of `text`; the words cut at each, each within
/// `tolerance` percentage points of its target and strictly increasing.
async fn level_cuts(text: &str, tolerance: f64) -> Vec<u64> {
    let mut session = Session::start(ZED).await;
    if text != DOC {
        session.change(text.to_string()).await;
    }
    let mut cut = Vec::new();
    for (level, target) in [
        ("slight", 10.0),
        ("tighten", 20.0),
        ("sharper", 30.0),
        ("half", 50.0),
    ] {
        let result = session.preview(level).await;
        let before = result["words_before"].as_u64().unwrap();
        let after = result["words_after"].as_u64().unwrap();
        assert!(before >= 400, "a realistic document: {before} words");
        let percent = 100.0 * (before - after) as f64 / before as f64;
        assert!(
            (percent - target).abs() <= tolerance,
            "{level}: {percent:.1}% against {target}%"
        );
        cut.push(before - after);
    }
    assert!(cut.windows(2).all(|pair| pair[0] < pair[1]), "{cut:?}");
    cut
}

#[tokio::test]
async fn the_levels_separate_on_the_713_word_prose_document() {
    let cut = level_cuts(LONG_PROSE, 5.0).await;
    // 713 words: "Cut in half" removes about 357 (the 480-word fixture, 240).
    assert!((340..=375).contains(&cut[3]), "{cut:?}");
}

#[tokio::test]
async fn the_levels_cut_strictly_more_near_ten_twenty_thirty_and_fifty_percent() {
    assert_eq!(level_cuts(DOC, 3.0).await.len(), 4);
}

#[tokio::test]
async fn a_preview_returns_first_then_sends_the_card_with_three_actions() {
    let mut session = Session::start(ZED).await;
    // The command answers before the client has replied to any prompt.
    let result = session.preview("slight").await;
    let status = result["status"].as_str().unwrap().to_string();
    let card = card(&mut session).await;
    assert_eq!(card.params.typ, MessageType::INFO);
    assert_eq!(card.titles(), ["Make the cuts", "Walk through", "Done"]);
    assert!(
        card.params.message.starts_with("Slight trim"),
        "{}",
        card.params.message
    );
    assert!(
        card.params.message.contains(&status),
        "{}",
        card.params.message
    );
    assert!(
        card.params.message.contains(HINT),
        "{}",
        card.params.message
    );
}

#[tokio::test]
async fn hovering_a_faded_span_shows_the_card_and_the_reason() {
    let mut session = Session::start(ZED).await;
    session.preview("slight").await;
    let fades = session.fades(ZED).await;
    let basically = fades
        .iter()
        .find(|d| d.message.contains("basically"))
        .expect("the basically filler");
    let inside = Position {
        line: basically.range.start.line,
        character: basically.range.start.character + 2,
    };
    let hover = session
        .server()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                position: inside,
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .expect("a hover");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("markdown hover");
    };
    assert!(markup.value.contains("Slight trim"), "{}", markup.value);
    assert!(markup.value.contains("words"), "{}", markup.value);
    assert!(
        markup.value.contains("filler \"basically\""),
        "{}",
        markup.value
    );
    assert_eq!(hover.range, Some(basically.range));
}

async fn keep_flow(caps: Caps) {
    let mut session = Session::start(caps).await;
    session.preview("slight").await;
    let first = card(&mut session).await;
    let fades = session.fades(caps).await;
    let basically = fades
        .iter()
        .find(|d| d.message.contains("basically"))
        .cloned()
        .expect("the basically filler");
    let inside = Position {
        line: basically.range.start.line,
        character: basically.range.start.character + 2,
    };
    let keep = session
        .actions_at(inside)
        .await
        .into_iter()
        .find(|action| action.title.starts_with("Keep: "))
        .expect("a Keep action inside the fade");
    assert_eq!(keep.title, "Keep: \u{ab}basically\u{bb}");
    let command = keep.command.unwrap();
    let result = session
        .execute(&command.command, command.arguments.unwrap())
        .await
        .unwrap()
        .unwrap();
    let after = session.fades(caps).await;
    assert_eq!(after.len(), fades.len() - 1);
    assert!(after.iter().all(|d| d.range != basically.range));
    let again = card(&mut session).await;
    assert_eq!(again.words().0, first.words().0);
    assert_eq!(again.words().1, first.words().1 + 1, "one word kept");
    assert_eq!(result["words_after"], json!(again.words().1));
}

#[tokio::test]
async fn keep_unfades_exactly_that_cut_for_a_pull_client() {
    keep_flow(ZED).await;
}

#[tokio::test]
async fn keep_unfades_exactly_that_cut_for_a_push_client() {
    keep_flow(Caps { pull: false, ..ZED }).await;
}

/// The applied text has no fades after a save, and its word count is the
/// card's.
async fn assert_cut(session: &mut Session, caps: Caps, words_after: usize) {
    session.save().await;
    assert!(session.fades(caps).await.is_empty(), "no fades remain");
    assert_eq!(lab_words(&session.text), words_after);
    // The preview is over: the menu starts from the original again.
    let result = session
        .execute(
            commands::TRIM_MAKE_CUTS,
            vec![json!({"uri": uri(), "version": session.version})],
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["edits"], json!(0));
}

async fn make_the_cuts_from_the_card(caps: Caps) {
    let mut session = Session::start(caps).await;
    session.preview("tighten").await;
    let card = card(&mut session).await;
    choose(&mut session, &card, Some("Make the cuts")).await;
    let request = session.next("workspace/applyEdit").await;
    let params: ApplyWorkspaceEditParams =
        serde_json::from_value(request.params().cloned().unwrap()).unwrap();
    let (version, edits) = versioned_edits(&params.edit);
    assert_eq!(version, Some(1), "rejected by the client if stale");
    let (expected, made) = expected_cuts(DOC, TrimLevel::Tighten, &[]);
    assert_eq!(edits, expected, "exactly the engine's make_cuts");
    let applied = apply(DOC, &edits);
    assert_eq!(applied, made);
    session.change(applied).await;
    session.reply(&request, json!({"applied": true})).await;
    assert_cut(&mut session, caps, card.words().1).await;
}

#[tokio::test]
async fn make_the_cuts_from_the_card_for_a_pull_client() {
    make_the_cuts_from_the_card(ZED).await;
}

#[tokio::test]
async fn make_the_cuts_from_the_card_for_a_push_client() {
    make_the_cuts_from_the_card(Caps { pull: false, ..ZED }).await;
}

#[tokio::test]
async fn make_the_cuts_from_the_menu_honours_a_keep_made_after_the_menu() {
    let mut session = Session::start(ZED).await;
    session.preview("slight").await;
    // The menu Zed cached before the keep.
    let cached = session.actions_at(Position::default()).await;
    let fades = session.fades(ZED).await;
    let basically = fades
        .iter()
        .find(|d| d.message.contains("basically"))
        .unwrap();
    let inside = Position {
        line: basically.range.start.line,
        character: basically.range.start.character + 2,
    };
    let keep = session
        .actions_at(inside)
        .await
        .into_iter()
        .find(|action| action.title.starts_with("Keep: "))
        .unwrap()
        .command
        .unwrap();
    let kept: CutId =
        serde_json::from_value(keep.arguments.as_ref().unwrap()[0]["cut"].clone()).unwrap();
    session
        .execute(&keep.command, keep.arguments.unwrap())
        .await
        .unwrap();
    let make = cached
        .into_iter()
        .find(|action| action.title == "Trim: Make the cuts")
        .unwrap()
        .command
        .unwrap();
    let result = session
        .execute(&make.command, make.arguments.unwrap())
        .await
        .unwrap()
        .unwrap();
    let request = session.next("workspace/applyEdit").await;
    let params: ApplyWorkspaceEditParams =
        serde_json::from_value(request.params().cloned().unwrap()).unwrap();
    let (_, edits) = versioned_edits(&params.edit);
    let (expected, made) = expected_cuts(DOC, TrimLevel::Slight, &[kept]);
    assert_eq!(edits, expected);
    assert_eq!(result["edits"], json!(expected.len()));
    let applied = apply(DOC, &edits);
    assert_eq!(applied, made);
    assert!(applied.contains("basically"), "the kept word survives");
    let words_after = result["words_after"].as_u64().unwrap() as usize;
    session.change(applied).await;
    session.reply(&request, json!({"applied": true})).await;
    assert_cut(&mut session, ZED, words_after).await;
}

/// Zed may report the edited text with a difference the server did not
/// predict, and may answer `applyEdit` before or after its `didChange`: the
/// preview still ends.
#[tokio::test]
async fn the_preview_ends_after_an_applied_edit_even_if_the_text_differs() {
    for answer_first in [false, true] {
        let mut session = Session::start(ZED).await;
        session.preview("half").await;
        let card = card(&mut session).await;
        choose(&mut session, &card, Some("Make the cuts")).await;
        let request = session.next("workspace/applyEdit").await;
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
        let (_, edits) = versioned_edits(&params.edit);
        let applied = format!("{}\n", apply(DOC, &edits));
        if answer_first {
            session.reply(&request, json!({"applied": true})).await;
            session.quiet().await;
            session.change(applied).await;
        } else {
            session.change(applied).await;
            session.reply(&request, json!({"applied": true})).await;
            session.quiet().await;
        }
        session.save().await;
        assert!(
            session.fades(ZED).await.is_empty(),
            "answer first: {answer_first}"
        );
    }
}

/// "Make the cuts" chosen twice before the editor answers: only one edit
/// goes out, so a second answer cannot undo the first; the confirmed edit
/// with a trailing-newline difference still ends the preview.
#[tokio::test]
async fn a_second_make_the_cuts_while_one_is_pending_sends_nothing() {
    let mut session = Session::start(ZED).await;
    session.preview("tighten").await;
    let card = card(&mut session).await;
    choose(&mut session, &card, Some("Make the cuts")).await;
    let request = session.next("workspace/applyEdit").await;
    // The menu action, while the card's edit is still pending.
    session
        .run(Position::default(), "Trim: Make the cuts")
        .await;
    let info = session.next("window/showMessage").await;
    let params: ShowMessageParams =
        serde_json::from_value(info.params().cloned().unwrap()).unwrap();
    assert!(
        params.message.contains("already sent"),
        "{}",
        params.message
    );
    assert!(
        session
            .quiet()
            .await
            .iter()
            .all(|m| m.method() != "workspace/applyEdit"),
        "one edit only"
    );
    let apply_params: ApplyWorkspaceEditParams =
        serde_json::from_value(request.params().cloned().unwrap()).unwrap();
    let (_, edits) = versioned_edits(&apply_params.edit);
    session.reply(&request, json!({"applied": true})).await;
    session.quiet().await;
    session.change(format!("{}\n", apply(DOC, &edits))).await;
    session.save().await;
    assert!(session.fades(ZED).await.is_empty(), "the preview ended");
}

#[tokio::test]
async fn walk_through_selects_each_cut_in_order_and_wraps() {
    let mut session = Session::start(ZED).await;
    session.preview("slight").await;
    let fades = session.fades(ZED).await;
    assert!(fades.len() > 2);
    let mut position = Position::default();
    let mut visited = Vec::new();
    for _ in 0..=fades.len() {
        let result = session.run(position, "Trim: Walk through").await;
        let shown = session.next("window/showDocument").await;
        let params: ShowDocumentParams =
            serde_json::from_value(shown.params().cloned().unwrap()).unwrap();
        session.reply(&shown, json!({"success": true})).await;
        assert_eq!(params.uri, uri());
        let selection = params.selection.expect("a selection");
        assert_eq!(result["range"], serde_json::to_value(selection).unwrap());
        visited.push(selection);
        position = selection.start;
    }
    let ranges: Vec<Range> = fades.iter().map(|d| d.range).collect();
    assert_eq!(&visited[..fades.len()], &ranges[..], "every cut, in order");
    assert_eq!(visited[fades.len()], ranges[0], "wraps to the first");
}

/// Zed leaves the cursor at the end of the selection, and a faded span can
/// start exactly where the previous one ends: the walk still visits every
/// span exactly once, in order, then wraps.
#[tokio::test]
async fn walk_through_from_the_selection_end_visits_touching_cuts_once() {
    let mut session = Session::start(ZED).await;
    session.change(LONG_PROSE.to_string()).await;
    session.preview("half").await;
    let fades = session.fades(ZED).await;
    let ranges: Vec<Range> = fades.iter().map(|d| d.range).collect();
    let touching = ranges
        .windows(2)
        .filter(|pair| pair[0].end == pair[1].start)
        .count();
    assert!(touching > 0, "the fixture has touching cuts");
    let mut position = Position::default();
    let mut visited = Vec::new();
    for _ in 0..=ranges.len() {
        let result = session.run(position, "Trim: Walk through").await;
        let shown = session.next("window/showDocument").await;
        session.reply(&shown, json!({"success": true})).await;
        let selection: Range = serde_json::from_value(result["range"].clone()).unwrap();
        visited.push(selection);
        position = selection.end;
    }
    assert_eq!(
        &visited[..ranges.len()],
        &ranges[..],
        "every cut once, in order"
    );
    assert_eq!(visited[ranges.len()], ranges[0], "then wraps");
}

/// Typing while "Make the cuts" is pending, then the client rejecting the
/// edit, keeps the preview: nothing was cut.
#[tokio::test]
async fn a_rejected_cut_edit_keeps_the_preview_despite_an_edit_meanwhile() {
    let mut session = Session::start(ZED).await;
    session.preview("tighten").await;
    let card = card(&mut session).await;
    choose(&mut session, &card, Some("Make the cuts")).await;
    let request = session.next("workspace/applyEdit").await;
    session.change(format!("{DOC}\nTyped meanwhile.\n")).await;
    session
        .reply(&request, json!({"applied": false, "failureReason": "busy"}))
        .await;
    session.quiet().await;
    session.save().await;
    assert!(
        !session.fades(ZED).await.is_empty(),
        "the preview is recomputed for the edited text"
    );
}

#[tokio::test]
async fn walk_through_from_the_card_steps_and_comes_back() {
    let mut session = Session::start(ZED).await;
    session.preview("slight").await;
    let fades = session.fades(ZED).await;
    let first = card(&mut session).await;
    choose(&mut session, &first, Some("Walk through")).await;
    for expected in &fades[..2] {
        let shown = session.next("window/showDocument").await;
        let params: ShowDocumentParams =
            serde_json::from_value(shown.params().cloned().unwrap()).unwrap();
        session.reply(&shown, json!({"success": true})).await;
        assert_eq!(params.selection, Some(expected.range));
        let next = card(&mut session).await;
        assert_eq!(next.titles(), ["Make the cuts", "Walk through", "Done"]);
        choose(&mut session, &next, Some("Walk through")).await;
    }
    let shown = session.next("window/showDocument").await;
    let params: ShowDocumentParams =
        serde_json::from_value(shown.params().cloned().unwrap()).unwrap();
    assert_eq!(params.selection, Some(fades[2].range));
}

#[tokio::test]
async fn done_or_dismissing_the_card_leaves_the_fades() {
    for reply in [Some("Done"), None] {
        let mut session = Session::start(ZED).await;
        session.preview("sharper").await;
        let fades = session.fades(ZED).await;
        let card = card(&mut session).await;
        choose(&mut session, &card, reply).await;
        let after = session.quiet().await;
        assert!(
            after
                .iter()
                .all(|m| m.method() != "workspace/applyEdit" && m.method() != "window/showDocument"),
            "{reply:?}: nothing happens"
        );
        assert_eq!(session.fades(ZED).await, fades, "{reply:?}");
    }
}

#[tokio::test]
async fn stale_versions_are_rejected() {
    let mut session = Session::start(ZED).await;
    session.preview("slight").await;
    let fades = session.fades(ZED).await;
    let inside = Position {
        line: fades[0].range.start.line,
        character: fades[0].range.start.character + 2,
    };
    let keep = session
        .actions_at(inside)
        .await
        .into_iter()
        .find(|action| action.title.starts_with("Keep: "))
        .unwrap()
        .command
        .unwrap();
    let card = card(&mut session).await;
    // The document changes under the menu and the card.
    let edited = format!("{DOC}\nOne more line.\n");
    session.change(edited).await;
    let error = session
        .execute(&keep.command, keep.arguments.unwrap())
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ContentModified);
    for (command, argument) in [
        (
            commands::TRIM_MAKE_CUTS,
            json!({"uri": uri(), "version": 1}),
        ),
        (
            commands::TRIM_NEXT,
            json!({"uri": uri(), "version": 1, "position": {"line": 0, "character": 0}}),
        ),
    ] {
        let error = session.execute(command, vec![argument]).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::ContentModified, "{command}");
    }
    // A save previews the same level on the new text; the card shown for
    // the old text still must not cut it.
    session.save().await;
    assert!(!session.fades(ZED).await.is_empty());
    choose(&mut session, &card, Some("Make the cuts")).await;
    let warning = session.next("window/showMessage").await;
    let params: ShowMessageParams =
        serde_json::from_value(warning.params().cloned().unwrap()).unwrap();
    assert_eq!(params.typ, MessageType::WARNING);
    assert!(params.message.contains("changed"), "{}", params.message);
    let rest = session.quiet().await;
    assert!(rest.iter().all(|m| m.method() != "workspace/applyEdit"));
}

#[tokio::test]
async fn clients_without_prompts_edits_or_show_document_still_get_everything() {
    let caps = Caps {
        pull: true,
        prompt: false,
        show_document: false,
        apply_edit: false,
    };
    let mut session = Session::start(caps).await;
    let result = session.preview("slight").await;
    // The card as a plain message.
    let message = session.next("window/showMessage").await;
    let params: ShowMessageParams =
        serde_json::from_value(message.params().cloned().unwrap()).unwrap();
    assert!(params.message.contains(result["status"].as_str().unwrap()));
    // Walk through answers with the range only.
    let walked = session.run(Position::default(), "Trim: Walk through").await;
    let fades = session.fades(caps).await;
    assert_eq!(
        walked["range"],
        serde_json::to_value(fades[0].range).unwrap()
    );
    // Make the cuts returns the edit for the client to apply.
    let made = session
        .run(Position::default(), "Trim: Make the cuts")
        .await;
    let edit: WorkspaceEdit = serde_json::from_value(made["edit"].clone()).unwrap();
    let (_, edits) = versioned_edits(&edit);
    let (expected, text) = expected_cuts(DOC, TrimLevel::Slight, &[]);
    assert_eq!(edits, expected);
    let rest = session.quiet().await;
    assert!(rest.iter().all(|m| {
        !matches!(
            m.method(),
            "workspace/applyEdit" | "window/showDocument" | "window/showMessageRequest"
        )
    }));
    session.change(apply(DOC, &edits)).await;
    assert_eq!(session.text, text);
    session.save().await;
    assert!(session.fades(caps).await.is_empty());
}

#[tokio::test]
async fn trim_original_clears_and_keeps_are_forgotten() {
    let mut session = Session::start(ZED).await;
    session.preview("half").await;
    assert!(!session.fades(ZED).await.is_empty());
    let result = session.preview("original").await;
    assert_eq!(result["candidates"], json!(0));
    assert!(session.fades(ZED).await.is_empty());
    let none = session
        .execute(
            commands::TRIM_NEXT,
            vec![json!({"uri": uri(), "position": {"line": 0, "character": 0}})],
        )
        .await
        .unwrap();
    assert_eq!(none, None, "nothing to walk");
}
