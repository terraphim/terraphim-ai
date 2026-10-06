//! Integration tests for the Terraphim LSP server.
//!
//! These tests drive the LSP service directly via `tower-lsp` without needing a
//! running editor or stdio transport.

use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

fn sample_thesaurus() -> Thesaurus {
    let mut thesaurus = Thesaurus::new("programming".to_string());
    thesaurus.insert(
        NormalizedTermValue::from("rust"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("rust programming language"))
            .with_url("https://rust-lang.org".to_string()),
    );
    thesaurus.insert(
        NormalizedTermValue::from("tokio"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("tokio async runtime")),
    );
    thesaurus.insert(
        NormalizedTermValue::from("async"),
        NormalizedTerm::with_auto_id(NormalizedTermValue::from("asynchronous programming")),
    );
    thesaurus
}

fn build_service() -> (LspService<TerraphimLspServer>, ClientSocket) {
    LspService::new(|client| TerraphimLspServer::new(client, sample_thesaurus()))
}

/// Initialisation with the opt-in unknown-term diagnostics switched on.
fn unknown_terms_on() -> InitializeParams {
    InitializeParams {
        initialization_options: Some(serde_json::json!({"unknownTerms": true})),
        ..InitializeParams::default()
    }
}

fn doc_uri() -> Url {
    Url::parse("file:///tmp/unknown.md").unwrap()
}

/// A server initialised with `init`, with `text` open, and its pulled
/// diagnostics.
async fn pulled_diagnostics(init: InitializeParams, text: &str) -> Vec<Diagnostic> {
    let (service, _socket) = build_service();
    service.inner().initialize(init).await.unwrap();
    open(service.inner(), text).await;
    pull(service.inner()).await
}

async fn open(server: &TerraphimLspServer, text: &str) {
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: doc_uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
}

async fn pull(server: &TerraphimLspServer) -> Vec<Diagnostic> {
    let report = server
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri: doc_uri() },
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
        other => panic!("expected full diagnostic report, got {other:?}"),
    }
}

/// `(line, start column, end column)` of each diagnostic with `message`.
fn ranges_of(diagnostics: &[Diagnostic], message: &str) -> Vec<(u32, u32, u32)> {
    diagnostics
        .iter()
        .filter(|d| d.message == message)
        .map(|d| {
            (
                d.range.start.line,
                d.range.start.character,
                d.range.end.character,
            )
        })
        .collect()
}

#[tokio::test]
async fn unknown_terms_are_off_by_default() {
    let diagnostics =
        pulled_diagnostics(InitializeParams::default(), "rust and xyz\nthe xyz end").await;
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[tokio::test]
async fn unknown_terms_report_every_occurrence_at_its_own_range() {
    // Issue #3436: a repeated word on a later line used to be reported at
    // its first occurrence.
    let text = "rust and the xyz\nthe tokio, the end\nthe";
    let diagnostics = pulled_diagnostics(unknown_terms_on(), text).await;
    assert_eq!(
        ranges_of(&diagnostics, "Unknown term: the"),
        [(0, 9, 12), (1, 0, 3), (1, 11, 14), (2, 0, 3)]
    );
    assert_eq!(ranges_of(&diagnostics, "Unknown term: xyz"), [(0, 13, 16)]);
    assert!(ranges_of(&diagnostics, "Unknown term: rust").is_empty());
    assert!(ranges_of(&diagnostics, "Unknown term: tokio").is_empty());
}

#[tokio::test]
async fn unknown_term_ranges_handle_crlf_and_multi_byte_text() {
    let text = "😀 café xyz\r\nxyz é xyz\r\n";
    let diagnostics = pulled_diagnostics(unknown_terms_on(), text).await;
    // 😀 is two UTF-16 units; é is one, though two bytes in UTF-8.
    assert_eq!(
        ranges_of(&diagnostics, "Unknown term: xyz"),
        [(0, 8, 11), (1, 0, 3), (1, 6, 9)]
    );
    assert_eq!(ranges_of(&diagnostics, "Unknown term: café"), [(0, 3, 7)]);
    assert_eq!(ranges_of(&diagnostics, "Unknown term: é"), [(1, 4, 5)]);
}

#[tokio::test]
async fn unknown_terms_follow_did_change_configuration() {
    let (service, _socket) = build_service();
    let server = service.inner();
    server
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    open(server, "rust and xyz").await;
    assert!(pull(server).await.is_empty());
    server
        .did_change_configuration(DidChangeConfigurationParams {
            settings: serde_json::json!({"terraphim": {"unknownTerms": true}}),
        })
        .await;
    assert_eq!(pull(server).await.len(), 2, "and, xyz");
    server
        .did_change_configuration(DidChangeConfigurationParams {
            settings: serde_json::json!({}),
        })
        .await;
    assert!(pull(server).await.is_empty());
}

#[tokio::test]
async fn test_initialize_returns_capabilities() {
    let (service, _) = build_service();
    let init_params = InitializeParams::default();
    let response = service.inner().initialize(init_params).await.unwrap();

    assert!(response.capabilities.hover_provider.is_some());
    assert!(response.capabilities.completion_provider.is_some());
    // Push-only client: diagnostics are pushed, the pull provider is not
    // advertised (see diagnostic_model_tests.rs).
    assert!(response.capabilities.diagnostic_provider.is_none());
    match response.capabilities.text_document_sync {
        Some(TextDocumentSyncCapability::Options(options)) => {
            assert_eq!(options.change, Some(TextDocumentSyncKind::FULL));
            assert_eq!(options.open_close, Some(true));
            assert_eq!(
                options.save,
                Some(TextDocumentSyncSaveOptions::Supported(true))
            );
        }
        other => panic!("expected sync options, got {other:?}"),
    }
}

#[tokio::test]
async fn test_hover_returns_description_for_matched_term() {
    let (service, _) = build_service();
    let _ = service
        .inner()
        .initialize(InitializeParams::default())
        .await;

    let uri = Url::parse("file:///tmp/test.md").unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "rust is great".to_string(),
            },
        })
        .await;

    let hover = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 1,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(hover.is_some());
    let contents = match hover.unwrap().contents {
        HoverContents::Markup(m) => m.value,
        _ => panic!("expected markup contents"),
    };
    assert!(contents.contains("rust programming language"));
}

#[tokio::test]
async fn test_hover_returns_none_for_unknown_term() {
    let (service, _) = build_service();
    let _ = service
        .inner()
        .initialize(InitializeParams::default())
        .await;

    let uri = Url::parse("file:///tmp/test.md").unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "xyz is unknown".to_string(),
            },
        })
        .await;

    let hover = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 1,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(hover.is_none());
}

#[tokio::test]
async fn test_completion_returns_thesaurus_terms() {
    let (service, _) = build_service();
    let _ = service
        .inner()
        .initialize(InitializeParams::default())
        .await;

    let uri = Url::parse("file:///tmp/test.md").unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "to".to_string(),
            },
        })
        .await;

    let completion = service
        .inner()
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 0,
                    character: 2,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        })
        .await
        .unwrap();

    let items = match completion {
        Some(CompletionResponse::Array(items)) => items,
        _ => panic!("expected completion array"),
    };
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "tokio");
}

#[tokio::test]
async fn test_diagnostic_reports_unknown_term() {
    let (service, _) = build_service();
    let _ = service.inner().initialize(unknown_terms_on()).await;

    let uri = Url::parse("file:///tmp/test.md").unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "rust and xyz".to_string(),
            },
        })
        .await;

    let report = service
        .inner()
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    let items = match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.items
        }
        _ => panic!("expected full diagnostic report"),
    };
    assert_eq!(items.len(), 2);
    let messages: Vec<String> = items.iter().map(|d| d.message.clone()).collect();
    assert!(messages.contains(&"Unknown term: xyz".to_string()));
    assert!(messages.contains(&"Unknown term: and".to_string()));
}

#[tokio::test]
async fn test_did_change_updates_diagnostics() {
    let (service, socket) = build_service();
    let _ = service.inner().initialize(unknown_terms_on()).await;

    let uri = Url::parse("file:///tmp/test.md").unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "rust".to_string(),
            },
        })
        .await;

    service
        .inner()
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "rust and xyz".to_string(),
            }],
        })
        .await;

    let report = service
        .inner()
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();

    let items = match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.items
        }
        _ => panic!("expected full diagnostic report"),
    };
    assert_eq!(items.len(), 2);
    let messages: Vec<String> = items.iter().map(|d| d.message.clone()).collect();
    assert!(messages.contains(&"Unknown term: xyz".to_string()));
    assert!(messages.contains(&"Unknown term: and".to_string()));

    // Drive the socket briefly so any pending client notifications are processed.
    let _ = socket;
}
