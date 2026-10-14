//! The Lab marks as code actions (#3451, R-8.1 and R-8.2): Zed can only run
//! commands that code actions carry, so each Lab action and "Clear marks"
//! is a stateless, command-only action of kind `refactor.terraphim.lab`.
//!
//! No mocks: the real Lab engine runs on a committed fixture that has a
//! typo, a punctuation slip, hedges, filler, long and convoluted sentences
//! and tone words. The server is driven through its tower `Service`, so
//! tower-lsp is initialised and really sends requests to the client; pull
//! and push clients are both played.

use std::collections::BTreeSet;
use std::time::Duration;

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::jsonrpc::{ErrorCode, Request};
use tower_lsp::lsp_types::*;
use tower_lsp::{LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::commands;
use terraphim_lsp::core::LabAction;
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
const DOC: &str = include_str!("fixtures/lab_actions_doc.md");
const LAB_KIND: &str = "refactor.terraphim.lab";

/// The Lab menu: the six actions in popover order, then Clear.
const MENU: [&str; 7] = [
    "Lab: Fix punctuation and typos",
    "Lab: Mark the weakest sentences",
    "Lab: Mark sentences that run long",
    "Lab: Mark convoluted sentences",
    "Lab: Mark words that don't fit the tone",
    "Lab: Mark hedges and filler",
    "Lab: Clear marks",
];

fn uri() -> Url {
    Url::parse("file:///tmp/lab-actions.md").unwrap()
}

fn lab_kind() -> CodeActionKind {
    CodeActionKind::new(LAB_KIND)
}

/// The Lab mark codes each action may produce.
fn codes_of(action: LabAction) -> &'static [&'static str] {
    match action {
        LabAction::TyposAndPunctuation => &["lab-typo", "lab-punctuation"],
        LabAction::WeakestSentences => &["lab-weak-sentence"],
        LabAction::LongSentences => &["lab-long-sentence"],
        LabAction::ConvolutedSentences => &["lab-convoluted-sentence"],
        LabAction::OffTone => &["lab-off-tone"],
        LabAction::HedgesAndFiller => &["lab-hedge", "lab-filler"],
    }
}

fn action_name(action: LabAction) -> String {
    serde_json::to_value(action)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

/// A Zed-shaped (`pull`) or push-only client session.
struct Session {
    service: LspService<TerraphimLspServer>,
    inbox: UnboundedReceiver<Request>,
    pull: bool,
}

impl Session {
    async fn start(pull: bool) -> Self {
        let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
        let (mut service, socket) =
            LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
        let (mut stream, _replies) = socket.split();
        let (sender, inbox) = unbounded_channel();
        tokio::spawn(async move {
            while let Some(request) = stream.next().await {
                if sender.send(request).is_err() {
                    break;
                }
            }
        });
        let mut capabilities = json!({
            "workspace": {
                "workspaceEdit": {"documentChanges": true},
                "diagnostics": {"refreshSupport": true},
                "diagnostic": {"refreshSupport": true},
            },
        });
        if pull {
            capabilities["textDocument"] = json!({"diagnostic": {"dynamicRegistration": false}});
        }
        let initialize = Request::build("initialize")
            .params(json!({"processId": null, "rootUri": null, "capabilities": capabilities}))
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
        let mut session = Self {
            service,
            inbox,
            pull,
        };
        session.open(DOC, 1).await;
        session
    }

    fn server(&self) -> &TerraphimLspServer {
        self.service.inner()
    }

    async fn open(&mut self, text: &str, version: i32) {
        self.server()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri(),
                    language_id: "markdown".to_string(),
                    version,
                    text: text.to_string(),
                },
            })
            .await;
    }

    async fn change(&self, text: &str, version: i32) {
        self.server()
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

    /// The code actions at the start of the document, as a client asks.
    async fn actions(&self, only: Option<Vec<CodeActionKind>>) -> Vec<CodeAction> {
        let position = Position::new(0, 0);
        self.server()
            .code_action(CodeActionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                range: Range {
                    start: position,
                    end: position,
                },
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

    async fn lab_actions(&self) -> Vec<CodeAction> {
        self.actions(Some(vec![lab_kind()])).await
    }

    /// Run the command of the Lab action titled `title`, as Zed does.
    async fn run_titled(&self, title: &str) -> tower_lsp::jsonrpc::Result<Option<Value>> {
        let action = self
            .lab_actions()
            .await
            .into_iter()
            .find(|action| action.title == title)
            .unwrap_or_else(|| panic!("no action {title:?}"));
        self.run(&action).await
    }

    async fn run(&self, action: &CodeAction) -> tower_lsp::jsonrpc::Result<Option<Value>> {
        assert!(action.edit.is_none(), "{}: command-only", action.title);
        let command = action.command.clone().expect("a command");
        self.server()
            .execute_command(ExecuteCommandParams {
                command: command.command,
                arguments: command.arguments.unwrap_or_default(),
                work_done_progress_params: Default::default(),
            })
            .await
    }

    /// The Lab marks the client shows now: a pull for a pulling client,
    /// the latest push for a push client.
    async fn marks(&mut self) -> Vec<Diagnostic> {
        let diagnostics = if self.pull {
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
            let mut latest = None;
            for request in self.settle().await {
                if request.method() == "textDocument/publishDiagnostics" {
                    latest = Some(request);
                }
            }
            let params: PublishDiagnosticsParams =
                serde_json::from_value(latest.expect("a push").params().cloned().unwrap()).unwrap();
            params.diagnostics
        };
        diagnostics
            .into_iter()
            .filter(|d| code(d).starts_with("lab-"))
            .collect()
    }

    /// Everything the server sent until it was quiet for a moment.
    async fn settle(&mut self) -> Vec<Request> {
        let mut seen = Vec::new();
        while let Ok(Some(request)) =
            tokio::time::timeout(Duration::from_millis(300), self.inbox.recv()).await
        {
            seen.push(request);
        }
        seen
    }
}

fn code(diagnostic: &Diagnostic) -> &str {
    match &diagnostic.code {
        Some(NumberOrString::String(code)) => code,
        _ => "",
    }
}

fn code_set(marks: &[Diagnostic]) -> BTreeSet<&str> {
    marks.iter().map(code).collect()
}

fn covered(range: Range) -> String {
    let line = DOC.lines().nth(range.start.line as usize).unwrap();
    line.chars()
        .skip(range.start.character as usize)
        .take((range.end.character - range.start.character) as usize)
        .collect()
}

// --------------------------------------------------------------- the menu --

#[tokio::test]
async fn lab_actions_are_command_only_and_titled_after_the_editor() {
    let session = Session::start(true).await;
    let actions = session.lab_actions().await;
    let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(titles, MENU, "stateless menu: always every action");
    // The titles come from terraphim_lab's own labels.
    for (action, title) in LabAction::ALL.into_iter().zip(MENU) {
        assert_eq!(title, format!("Lab: {}", action.label()));
    }
    for action in &actions {
        assert_eq!(action.kind, Some(lab_kind()));
        assert!(action.edit.is_none(), "command-only so clients execute it");
        assert_eq!(action.command.as_ref().unwrap().title, action.title);
    }
    for (action, lab) in actions.iter().zip(LabAction::ALL) {
        let command = action.command.as_ref().unwrap();
        assert_eq!(command.command, "terraphim.lab.mark");
        assert_eq!(
            command.arguments,
            Some(vec![
                json!({"uri": uri(), "version": 1, "action": action_name(lab)})
            ])
        );
    }
    let clear = actions[6].command.as_ref().unwrap();
    assert_eq!(clear.command, "terraphim.lab.clear");
    assert_eq!(clear.arguments, Some(vec![json!({"uri": uri()})]));
}

#[tokio::test]
async fn lab_actions_carry_the_current_document_version() {
    let session = Session::start(true).await;
    session.change(DOC, 7).await;
    let actions = session.lab_actions().await;
    let mark = actions[0].command.as_ref().unwrap();
    assert_eq!(mark.arguments.as_ref().unwrap()[0]["version"], 7);
}

#[tokio::test]
async fn lab_actions_honour_the_only_filter() {
    let session = Session::start(true).await;
    for only in [
        vec![lab_kind()],
        vec![CodeActionKind::REFACTOR],
        vec![CodeActionKind::new("refactor.terraphim")],
    ] {
        let actions = session.actions(Some(only)).await;
        let labs: Vec<&str> = actions
            .iter()
            .filter(|a| a.kind == Some(lab_kind()))
            .map(|a| a.title.as_str())
            .collect();
        assert_eq!(labs, MENU);
    }
    // Asking for the Lab kind alone offers nothing else.
    let only_lab = session.lab_actions().await;
    assert_eq!(only_lab.len(), MENU.len());
    for only in [
        vec![CodeActionKind::QUICKFIX],
        vec![CodeActionKind::REFACTOR_REWRITE],
        vec![CodeActionKind::new("refactor.terraphim.lab.x")],
        vec![CodeActionKind::new("refactor.terraphim.tri")],
    ] {
        let actions = session.actions(Some(only)).await;
        assert!(
            actions.iter().all(|a| a.kind != Some(lab_kind())),
            "excluded by `only`"
        );
    }
    let all = session.actions(None).await;
    assert_eq!(
        all.iter().filter(|a| a.kind == Some(lab_kind())).count(),
        MENU.len()
    );
}

#[tokio::test]
async fn lab_kind_is_advertised() {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
    let (service, _socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
    let result = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let Some(CodeActionProviderCapability::Options(options)) =
        result.capabilities.code_action_provider
    else {
        panic!("code action options");
    };
    assert!(options.code_action_kinds.unwrap().contains(&lab_kind()));
}

// ------------------------------------------------------- running a command --

#[tokio::test]
async fn each_action_marks_exactly_its_own_kinds() {
    for pull in [true, false] {
        for (title, lab) in MENU.into_iter().zip(LabAction::ALL) {
            let mut session = Session::start(pull).await;
            let result = session.run_titled(title).await.unwrap().unwrap();
            let marks = session.marks().await;
            assert!(!marks.is_empty(), "{title} (pull {pull}): no marks");
            let allowed: BTreeSet<&str> = codes_of(lab).iter().copied().collect();
            assert!(
                code_set(&marks).is_subset(&allowed),
                "{title} (pull {pull}): {:?} not within {allowed:?}",
                code_set(&marks)
            );
            assert_eq!(result["action"], json!(action_name(lab)));
            assert_eq!(result["marks"], json!(marks.len()));
        }
    }
}

#[tokio::test]
async fn off_tone_marks_the_tone_words() {
    for pull in [true, false] {
        let mut session = Session::start(pull).await;
        session
            .run_titled("Lab: Mark words that don't fit the tone")
            .await
            .unwrap();
        let marks = session.marks().await;
        assert_eq!(code_set(&marks), BTreeSet::from(["lab-off-tone"]));
        let words: BTreeSet<String> = marks.iter().map(|m| covered(m.range)).collect();
        for expected in ["awesome", "guys", "a lot of", "cool", "stuff"] {
            assert!(words.contains(expected), "{expected:?} not in {words:?}");
        }
        assert!(
            marks
                .iter()
                .all(|m| m.severity == Some(DiagnosticSeverity::HINT))
        );
    }
}

#[tokio::test]
async fn typo_action_marks_the_typo_and_the_punctuation() {
    let mut session = Session::start(true).await;
    session
        .run_titled("Lab: Fix punctuation and typos")
        .await
        .unwrap();
    let marks = session.marks().await;
    assert_eq!(
        code_set(&marks),
        BTreeSet::from(["lab-typo", "lab-punctuation"])
    );
    let typo = marks.iter().find(|m| code(m) == "lab-typo").unwrap();
    assert_eq!(covered(typo.range), "recieve");
}

#[tokio::test]
async fn clear_removes_the_marks() {
    for pull in [true, false] {
        let mut session = Session::start(pull).await;
        session
            .run_titled("Lab: Mark hedges and filler")
            .await
            .unwrap();
        session
            .run_titled("Lab: Mark words that don't fit the tone")
            .await
            .unwrap();
        let marked = session.marks().await;
        assert_eq!(
            code_set(&marked)
                .into_iter()
                .filter(|c| *c == "lab-off-tone" || *c == "lab-hedge" || *c == "lab-filler")
                .count(),
            code_set(&marked).len()
        );
        assert!(code_set(&marked).contains("lab-off-tone"));
        let cleared = session.run_titled("Lab: Clear marks").await.unwrap();
        assert_eq!(cleared, None);
        assert!(session.marks().await.is_empty(), "pull {pull}");
    }
}

#[tokio::test]
async fn a_stale_version_is_rejected_and_marks_nothing() {
    for pull in [true, false] {
        let mut session = Session::start(pull).await;
        // The menu was built at version 1; the document moved on.
        let stale = session
            .lab_actions()
            .await
            .into_iter()
            .find(|a| a.title == "Lab: Mark words that don't fit the tone")
            .unwrap();
        session.change(&format!("{DOC}\nMore.\n"), 2).await;
        let error = session.run(&stale).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::ContentModified, "pull {pull}");
        session.settle().await;
        let marks = if pull {
            session.marks().await
        } else {
            // No push carries a mark.
            Vec::new()
        };
        assert!(marks.is_empty());
        // A freshly built menu carries the new version and works.
        session
            .run_titled("Lab: Mark words that don't fit the tone")
            .await
            .unwrap();
        assert!(!session.marks().await.is_empty());
    }
}

#[tokio::test]
async fn a_pulling_client_is_asked_to_refresh_and_never_pushed_to() {
    let mut session = Session::start(true).await;
    session.settle().await;
    session
        .run_titled("Lab: Mark words that don't fit the tone")
        .await
        .unwrap();
    let sent = session.settle().await;
    let methods: Vec<&str> = sent.iter().map(|r| r.method()).collect();
    assert!(
        methods.contains(&"workspace/diagnostic/refresh"),
        "{methods:?}"
    );
    assert!(!methods.contains(&"textDocument/publishDiagnostics"));
}

#[tokio::test]
async fn a_push_client_gets_the_marks_in_a_publish() {
    let mut session = Session::start(false).await;
    session.settle().await;
    session
        .run_titled("Lab: Mark words that don't fit the tone")
        .await
        .unwrap();
    let sent = session.settle().await;
    let methods: Vec<&str> = sent.iter().map(|r| r.method()).collect();
    assert!(
        methods.contains(&"textDocument/publishDiagnostics"),
        "{methods:?}"
    );
    assert!(!methods.contains(&"workspace/diagnostic/refresh"));
    let published: PublishDiagnosticsParams =
        serde_json::from_value(sent.last().unwrap().params().cloned().unwrap()).unwrap();
    assert_eq!(published.uri, uri());
    assert!(
        published
            .diagnostics
            .iter()
            .any(|d| code(d) == "lab-off-tone")
    );
}

#[tokio::test]
async fn a_closed_document_offers_nothing() {
    let session = Session::start(true).await;
    session
        .server()
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri() },
        })
        .await;
    assert!(session.lab_actions().await.is_empty());
}

#[test]
fn the_commands_are_still_the_ones_advertised() {
    assert!(commands::ALL.contains(&commands::LAB_MARK));
    assert!(commands::ALL.contains(&commands::LAB_CLEAR));
}
