//! One diagnostics model per client (#3438): clients that pull
//! (`textDocument.diagnostic`) get pulls and `workspace/diagnostic/refresh`,
//! never pushed `publishDiagnostics`; other clients get pushes only.
//!
//! No mocks: the service is initialised through its tower `Service`
//! interface, so tower-lsp marks it initialised and really sends client
//! notifications, which a forwarding task reads off the client socket (the
//! client channel is bounded, so it must be drained as a real client would).

use futures::StreamExt;
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::jsonrpc::Request;
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json");
const LAB_DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/lab_doc.md");
/// One core diagnostic: a truncated annotation block.
const BROKEN: &str = "body\n\n```terraphim-alternatives\n{\n";

fn uri() -> Url {
    Url::parse("file:///tmp/model.md").unwrap()
}

struct Inbox(UnboundedReceiver<(String, Value)>);

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

impl Inbox {
    /// The methods (with params) sent to the client so far.
    async fn drain(&mut self) -> Vec<(String, Value)> {
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        let mut messages = Vec::new();
        while let Ok(message) = self.0.try_recv() {
            messages.push(message);
        }
        messages
    }
}

fn count(messages: &[(String, Value)], method: &str) -> usize {
    messages.iter().filter(|(m, _)| m == method).count()
}

/// Client capabilities: pull support and refresh support as given.
fn capabilities(pull: bool, refresh: bool) -> ClientCapabilities {
    ClientCapabilities {
        text_document: pull.then(|| TextDocumentClientCapabilities {
            diagnostic: Some(DiagnosticClientCapabilities::default()),
            ..Default::default()
        }),
        workspace: Some(WorkspaceClientCapabilities {
            diagnostic: Some(DiagnosticWorkspaceClientCapabilities {
                refresh_support: Some(refresh),
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// An initialised (in tower-lsp's state too) server and its inbox, plus the
/// advertised capabilities.
async fn start(
    client: ClientCapabilities,
) -> (LspService<TerraphimLspServer>, Inbox, ServerCapabilities) {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
    let (mut service, socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
    let inbox = inbox(socket);
    let params = InitializeParams {
        capabilities: client,
        ..Default::default()
    };
    let initialize = Request::build("initialize")
        .params(serde_json::to_value(params).unwrap())
        .id(1)
        .finish();
    let response = service
        .ready()
        .await
        .unwrap()
        .call(initialize)
        .await
        .unwrap()
        .expect("initialize response");
    let (_, result) = response.into_parts();
    let result: InitializeResult = serde_json::from_value(result.unwrap()).unwrap();
    let initialized = Request::build("initialized").params(json!({})).finish();
    service
        .ready()
        .await
        .unwrap()
        .call(initialized)
        .await
        .unwrap();
    (service, inbox, result.capabilities)
}

async fn open(server: &TerraphimLspServer, text: &str) {
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
}

async fn change(server: &TerraphimLspServer, text: &str) {
    server
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: text.to_string(),
            }],
        })
        .await;
}

async fn pull(server: &TerraphimLspServer) -> Vec<Diagnostic> {
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

async fn lab_mark(server: &TerraphimLspServer) {
    server
        .execute_command(ExecuteCommandParams {
            command: "terraphim.lab.mark".to_string(),
            arguments: vec![json!({"uri": uri(), "action": "hedges_and_filler"})],
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn push_client_gets_pushes_only() {
    let (service, mut inbox, capabilities) = start(capabilities(false, true)).await;
    assert!(capabilities.diagnostic_provider.is_none());
    let server = service.inner();
    open(server, BROKEN).await;
    change(server, &format!("edited {BROKEN}")).await;
    lab_mark(server).await;
    let messages = inbox.drain().await;
    let pushes: Vec<&Value> = messages
        .iter()
        .filter(|(m, _)| m == "textDocument/publishDiagnostics")
        .map(|(_, params)| params)
        .collect();
    assert!(pushes.len() >= 3, "open, change and Lab mark: {messages:?}");
    assert_eq!(pushes[0]["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(count(&messages, "workspace/diagnostic/refresh"), 0);
}

#[tokio::test]
async fn pull_client_is_never_pushed_and_is_refreshed() {
    let (service, mut inbox, capabilities) = start(capabilities(true, true)).await;
    assert!(capabilities.diagnostic_provider.is_some());
    let server = service.inner();
    open(server, BROKEN).await;
    change(server, &format!("edited {BROKEN}")).await;
    let messages = inbox.drain().await;
    assert_eq!(count(&messages, "textDocument/publishDiagnostics"), 0);
    assert_eq!(
        count(&messages, "workspace/diagnostic/refresh"),
        0,
        "the client pulls after its own edits: {messages:?}"
    );
    assert_eq!(pull(server).await.len(), 1, "served by pull, once");

    // Results changing outside a client edit trigger one refresh.
    change(server, LAB_DOC).await;
    lab_mark(server).await;
    let messages = inbox.drain().await;
    assert_eq!(count(&messages, "textDocument/publishDiagnostics"), 0);
    assert_eq!(count(&messages, "workspace/diagnostic/refresh"), 1);
    assert!(
        pull(server)
            .await
            .iter()
            .any(|d| { d.code == Some(NumberOrString::String("lab-hedge".to_string())) })
    );

    // A settings change also refreshes once.
    server
        .did_change_configuration(DidChangeConfigurationParams {
            settings: json!({"ghostDiagnostics": false}),
        })
        .await;
    let messages = inbox.drain().await;
    assert_eq!(count(&messages, "textDocument/publishDiagnostics"), 0);
    assert_eq!(count(&messages, "workspace/diagnostic/refresh"), 1);
}

#[tokio::test]
async fn pull_client_without_refresh_support_is_neither_pushed_nor_refreshed() {
    let (service, mut inbox, capabilities) = start(capabilities(true, false)).await;
    assert!(capabilities.diagnostic_provider.is_some());
    let server = service.inner();
    open(server, LAB_DOC).await;
    lab_mark(server).await;
    let messages = inbox.drain().await;
    assert_eq!(count(&messages, "textDocument/publishDiagnostics"), 0);
    assert_eq!(count(&messages, "workspace/diagnostic/refresh"), 0);
    assert!(!pull(server).await.is_empty());
}
