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

#[tokio::test]
async fn test_initialize_returns_capabilities() {
    let (service, _) = build_service();
    let init_params = InitializeParams::default();
    let response = service.inner().initialize(init_params).await.unwrap();

    assert!(response.capabilities.hover_provider.is_some());
    assert!(response.capabilities.completion_provider.is_some());
    assert!(response.capabilities.diagnostic_provider.is_some());
    assert_eq!(
        response.capabilities.text_document_sync,
        Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL))
    );
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
