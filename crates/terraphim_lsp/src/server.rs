use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use terraphim_negative_contribution::NegativeContributionScanner;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::config::LspConfig;
use crate::diagnostic::finding_to_diagnostic;

pub struct TerraphimLspServer {
    client: Client,
    scanner: NegativeContributionScanner,
    config: LspConfig,
    documents: Arc<Mutex<HashMap<Url, String>>>,
    pending: Arc<Mutex<HashMap<Url, tokio::task::AbortHandle>>>,
}

impl TerraphimLspServer {
    pub fn new(client: Client) -> Self {
        Self::with_config(client, LspConfig::default())
    }

    pub fn with_config(client: Client, config: LspConfig) -> Self {
        Self {
            client,
            scanner: NegativeContributionScanner::new(),
            config,
            documents: Arc::new(Mutex::new(HashMap::new())),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn schedule_diagnostics(&self, uri: Url) {
        let documents = self.documents.clone();
        let pending = self.pending.clone();
        let scanner = self.scanner.clone();
        let client = self.client.clone();
        let debounce_ms = self.config.debounce_ms;
        let uri_for_insert = uri.clone();
        let uri_for_spawn = uri.clone();

        let mut guard = pending.lock().await;
        if let Some(handle) = guard.remove(&uri) {
            handle.abort();
        }

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(debounce_ms)).await;

            let content = {
                let docs = documents.lock().await;
                docs.get(&uri_for_spawn).cloned()
            };

            let diagnostics = match content {
                Some(content) => {
                    let path = uri_for_spawn.path();
                    let findings = scanner.scan_file(path, &content);
                    findings.iter().map(finding_to_diagnostic).collect()
                }
                None => Vec::new(),
            };

            client
                .publish_diagnostics(uri_for_spawn, diagnostics, None)
                .await;
        });

        guard.insert(uri_for_insert, handle.abort_handle());
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TerraphimLspServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.documents
            .lock()
            .await
            .insert(uri.clone(), params.text_document.text);
        self.schedule_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.lock().await.insert(uri.clone(), change.text);
        }
        self.schedule_diagnostics(uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        self.documents.lock().await.remove(&uri);
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }
}
