//! Loading the thesaurus from `initializationOptions`, the launch options
//! (`--thesaurus` / `TERRAPHIM_THESAURUS`) and `didChangeConfiguration`.
//!
//! No mocks: the real fixture thesaurus committed with `terraphim_lsp_core`
//! is read from disk, and messages to the client are read back from the
//! service's client socket. The socket is drained continuously by a
//! forwarding task, as a real client would: tower-lsp's client channel is
//! bounded, so a second unread message would block the server.

use std::path::PathBuf;

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::thesaurus::{CliArgs, LaunchOptions};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json"
);
const INVALID: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/invalid_thesaurus.json"
);
const MISSING: &str = "/nonexistent/terraphim-lsp/thesaurus.json";

const TEXT: &str = "We made a choice.\n";

fn uri() -> Url {
    Url::parse("file:///tmp/thesaurus.md").unwrap()
}

/// The `binary` server: empty thesaurus plus launch options.
fn launch_service(thesaurus: Option<&str>) -> (LspService<TerraphimLspServer>, ClientSocket) {
    let launch = LaunchOptions {
        thesaurus: thesaurus.map(PathBuf::from),
        home: None,
    };
    LspService::new(move |client| TerraphimLspServer::with_launch_options(client, launch.clone()))
}

fn init(options: Option<Value>) -> InitializeParams {
    InitializeParams {
        initialization_options: options,
        ..InitializeParams::default()
    }
}

async fn open(server: &TerraphimLspServer) {
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: TEXT.to_string(),
            },
        })
        .await;
}

async fn configure(server: &TerraphimLspServer, settings: Value) {
    server
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;
}

/// Position of "choice" in [`TEXT`].
fn on_choice() -> Position {
    Position {
        line: 0,
        character: 11,
    }
}

async fn hover(server: &TerraphimLspServer) -> Option<Hover> {
    server
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                position: on_choice(),
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
}

async fn action_titles(server: &TerraphimLspServer) -> Vec<String> {
    let actions = server
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range: Range {
                start: on_choice(),
                end: on_choice(),
            },
            context: CodeActionContext::default(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap_or_default();
    actions
        .into_iter()
        .map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title,
            CodeActionOrCommand::Command(command) => command.title,
        })
        .collect()
}

async fn inlay_labels(server: &TerraphimLspServer) -> Vec<String> {
    let hints = server
        .inlay_hint(InlayHintParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range: Range {
                start: Position::default(),
                end: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .unwrap_or_default();
    hints
        .into_iter()
        .map(|hint| match hint.label {
            InlayHintLabel::String(label) => label,
            InlayHintLabel::LabelParts(parts) => parts.into_iter().map(|part| part.value).collect(),
        })
        .collect()
}

/// Messages the server sent to the client, read off the socket by a
/// forwarding task.
struct Inbox(UnboundedReceiver<(String, Value)>);

/// Start reading `socket` into an [`Inbox`].
fn inbox(mut socket: ClientSocket) -> Inbox {
    let (sender, receiver) = unbounded_channel();
    tokio::spawn(async move {
        while let Some(request) = socket.next().await {
            let params = request.params().cloned().unwrap_or(Value::Null);
            if sender.send((request.method().to_string(), params)).is_err() {
                break;
            }
        }
    });
    Inbox(receiver)
}

/// The `(method, params)` of every message the server has sent so far.
async fn drain(inbox: &mut Inbox) -> Vec<(String, Value)> {
    // Let the forwarding task (woken by each send) catch up.
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }
    let mut messages = Vec::new();
    while let Ok(message) = inbox.0.try_recv() {
        messages.push(message);
    }
    messages
}

/// The `window/showMessage` warnings among `messages`.
fn warnings(messages: &[(String, Value)]) -> Vec<String> {
    messages
        .iter()
        .filter(|(method, _)| method == "window/showMessage")
        .filter(|(_, params)| params["type"] == json!(MessageType::WARNING))
        .map(|(_, params)| params["message"].as_str().unwrap_or_default().to_string())
        .collect()
}

async fn assert_kg_features_work(server: &TerraphimLspServer) {
    let hover = hover(server).await.expect("hover on a KG term");
    let HoverContents::Markup(markup) = hover.contents else {
        panic!("markdown hover expected");
    };
    assert!(markup.value.contains("choice"), "{}", markup.value);
    let titles = action_titles(server).await;
    for expected in [
        "Replace with decision",
        "Replace with judgment",
        "Replace with option",
    ] {
        assert!(titles.iter().any(|t| t == expected), "{titles:?}");
    }
    assert_eq!(inlay_labels(server).await, ["[2/4]"]);
}

async fn assert_kg_features_off(server: &TerraphimLspServer) {
    assert!(hover(server).await.is_none());
    assert!(action_titles(server).await.is_empty());
    assert!(inlay_labels(server).await.is_empty());
}

#[tokio::test]
async fn initialization_options_load_the_thesaurus() {
    let (service, socket) = launch_service(None);
    let mut messages = inbox(socket);
    let server = service.inner();
    server
        .initialize(init(Some(
            json!({"thesaurus": FIXTURE, "inlayHints": true}),
        )))
        .await
        .unwrap();
    open(server).await;
    assert_eq!(server.thesaurus_path(), Some(PathBuf::from(FIXTURE)));
    assert!(server.thesaurus_len() > 0);
    assert_kg_features_work(server).await;
    assert!(warnings(&drain(&mut messages).await).is_empty());
}

#[tokio::test]
async fn thesaurus_setting_nested_under_terraphim() {
    let (service, _socket) = launch_service(None);
    let server = service.inner();
    let options = json!({"terraphim": {"thesaurus": FIXTURE, "inlayHints": true}});
    server.initialize(init(Some(options))).await.unwrap();
    open(server).await;
    assert_kg_features_work(server).await;
}

/// The binary resolves `--thesaurus` / `TERRAPHIM_THESAURUS` into launch
/// options; with no client setting, they apply.
#[tokio::test]
async fn launch_path_applies_without_a_setting() {
    let cli = CliArgs::default();
    let launch = LaunchOptions::new(&cli, Some(FIXTURE.as_ref()), None);
    assert_eq!(launch.thesaurus, Some(PathBuf::from(FIXTURE)));
    let (service, _socket) = launch_service(Some(FIXTURE));
    let server = service.inner();
    server
        .initialize(init(Some(json!({"inlayHints": true}))))
        .await
        .unwrap();
    open(server).await;
    assert_kg_features_work(server).await;
}

#[tokio::test]
async fn setting_overrides_launch_path() {
    let (service, socket) = launch_service(Some(FIXTURE));
    let mut messages = inbox(socket);
    let server = service.inner();
    server
        .initialize(init(Some(
            json!({"thesaurus": MISSING, "inlayHints": true}),
        )))
        .await
        .unwrap();
    open(server).await;
    assert_eq!(server.thesaurus_path(), Some(PathBuf::from(MISSING)));
    assert_kg_features_off(server).await;
    assert_eq!(warnings(&drain(&mut messages).await).len(), 1);
}

#[tokio::test]
async fn invalid_thesaurus_warns_once_and_keeps_running() {
    for path in [INVALID, MISSING] {
        let (service, socket) = launch_service(None);
        let mut messages = inbox(socket);
        let server = service.inner();
        let options = json!({"thesaurus": path, "inlayHints": true, "unknownTerms": true});
        server
            .initialize(init(Some(options.clone())))
            .await
            .unwrap();
        open(server).await;
        let shown = warnings(&drain(&mut messages).await);
        assert_eq!(shown.len(), 1, "{shown:?}");
        assert!(shown[0].contains(path), "{}", shown[0]);
        assert!(shown[0].contains("cannot load thesaurus"), "{}", shown[0]);
        assert_eq!(server.thesaurus_len(), 0);
        assert_kg_features_off(server).await;

        // Other settings changing does not retry (or re-report) the path.
        let mut changed = options.clone();
        changed["ghostDiagnostics"] = json!(false);
        configure(server, changed).await;
        assert!(warnings(&drain(&mut messages).await).is_empty());
    }
}

#[tokio::test]
async fn missing_thesaurus_is_logged_not_shown() {
    let (service, socket) = launch_service(None);
    let mut messages = inbox(socket);
    service.inner().initialize(init(None)).await.unwrap();
    let sent = drain(&mut messages).await;
    assert!(warnings(&sent).is_empty());
    let logged: Vec<&Value> = sent
        .iter()
        .filter(|(method, _)| method == "window/logMessage")
        .map(|(_, params)| params)
        .collect();
    assert_eq!(logged.len(), 1, "{sent:?}");
    assert!(
        logged[0]["message"]
            .as_str()
            .unwrap()
            .contains("no thesaurus configured")
    );
}

/// Zed sends `initialization_options` at `initialize` and its `settings`
/// through `didChangeConfiguration`: a thesaurus given only in the former
/// survives the latter, which a configured `thesaurus` still overrides.
#[tokio::test]
async fn initialization_thesaurus_survives_settings_without_one() {
    let (service, socket) = launch_service(None);
    let mut messages = inbox(socket);
    let server = service.inner();
    server
        .initialize(init(Some(json!({"thesaurus": FIXTURE}))))
        .await
        .unwrap();
    open(server).await;
    configure(
        server,
        json!({"inlayHints": true, "ghostDiagnostics": true}),
    )
    .await;
    assert_eq!(server.thesaurus_path(), Some(PathBuf::from(FIXTURE)));
    assert_kg_features_work(server).await;

    configure(server, json!({"thesaurus": MISSING, "inlayHints": true})).await;
    assert_kg_features_off(server).await;
    assert_eq!(warnings(&drain(&mut messages).await).len(), 1);

    // Dropping the configured path returns to the initialisation one.
    configure(server, json!({"inlayHints": true})).await;
    assert_kg_features_work(server).await;
}

#[tokio::test]
async fn did_change_configuration_reloads_the_thesaurus() {
    let (service, socket) = launch_service(None);
    let mut messages = inbox(socket);
    let server = service.inner();
    server
        .initialize(init(Some(json!({"inlayHints": true}))))
        .await
        .unwrap();
    open(server).await;
    assert_kg_features_off(server).await;

    configure(server, json!({"thesaurus": FIXTURE, "inlayHints": true})).await;
    assert_kg_features_work(server).await;

    configure(server, json!({"thesaurus": INVALID, "inlayHints": true})).await;
    assert_kg_features_off(server).await;
    assert_eq!(warnings(&drain(&mut messages).await).len(), 1);

    configure(server, json!({"thesaurus": FIXTURE, "inlayHints": true})).await;
    assert_kg_features_work(server).await;

    // Removing the setting returns to the (empty) launch thesaurus.
    configure(server, json!({"inlayHints": true})).await;
    assert_eq!(server.thesaurus_path(), None);
    assert_kg_features_off(server).await;
}

/// The constructor's thesaurus stays in use while no path is configured.
#[tokio::test]
async fn programmatic_thesaurus_is_kept_without_a_path() {
    let thesaurus = serde_json::from_str(&std::fs::read_to_string(FIXTURE).unwrap()).unwrap();
    let (service, _socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus));
    let server = service.inner();
    server
        .initialize(init(Some(json!({"inlayHints": true}))))
        .await
        .unwrap();
    open(server).await;
    assert_kg_features_work(server).await;
    configure(server, json!({"thesaurus": MISSING, "inlayHints": true})).await;
    assert_kg_features_off(server).await;
    configure(server, json!({"inlayHints": true})).await;
    assert_kg_features_work(server).await;
}

/// A bare `thesaurus` next to a nested `terraphim` object is kept, in
/// `initializationOptions` and in `didChangeConfiguration` alike.
#[tokio::test]
async fn mixed_bare_and_nested_settings_merge() {
    let (service, _socket) = launch_service(None);
    let server = service.inner();
    let mixed = json!({"thesaurus": FIXTURE, "terraphim": {"inlayHints": true}});
    server.initialize(init(Some(mixed))).await.unwrap();
    open(server).await;
    assert_eq!(server.thesaurus_path(), Some(PathBuf::from(FIXTURE)));
    assert!(server.settings().inlay_hints);
    assert_kg_features_work(server).await;

    let (service, _socket) = launch_service(None);
    let server = service.inner();
    server.initialize(init(None)).await.unwrap();
    open(server).await;
    configure(
        server,
        json!({"thesaurus": FIXTURE, "terraphim": {"inlayHints": true, "unknownTerms": true}}),
    )
    .await;
    assert!(server.settings().unknown_terms);
    assert_kg_features_work(server).await;
}
