use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use terraphim_types::Thesaurus;

use crate::completion::{build_completions, word_at_position};
use crate::diagnostics::build_diagnostics_with_positions;
use crate::kg_analysis::analyse_kg_document;

/// Terraphim LSP server backed by a knowledge-graph thesaurus.
///
/// The server tracks open text documents and provides:
///
/// - `textDocument/hover` - concept descriptions for matched KG terms
/// - `textDocument/completion` - thesaurus term suggestions
/// - `textDocument/diagnostic` - warnings for unknown terms
#[derive(Debug)]
pub struct TerraphimLspServer {
    client: Client,
    thesaurus: Thesaurus,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

impl TerraphimLspServer {
    /// Create a new LSP server instance tied to the given LSP client and
    /// knowledge-graph thesaurus.
    pub fn new(client: Client, thesaurus: Thesaurus) -> Self {
        Self {
            client,
            thesaurus,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Convenience constructor used by `LspService::new` when no custom
    /// thesaurus is available. It creates an empty thesaurus so the server
    /// can still start; real deployments should use [`Self::new`].
    pub fn new_with_empty_thesaurus(client: Client) -> Self {
        Self::new(client, Thesaurus::new("empty".to_string()))
    }

    /// Run the LSP server over stdio using an empty thesaurus.
    ///
    /// This is the entry point for the `terraphim-lsp` binary. For programmatic
    /// use with a custom thesaurus, construct the server via [`LspService::new`]
    /// and [`Self::new`].
    pub async fn run_stdio() {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(Self::new_with_empty_thesaurus);
        Server::new(stdin, stdout, socket).serve(service).await;
    }

    /// Run the LSP server over stdio with the given thesaurus.
    pub async fn run_stdio_with_thesaurus(thesaurus: Thesaurus) {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(move |client| Self::new(client, thesaurus.clone()));
        Server::new(stdin, stdout, socket).serve(service).await;
    }

    /// Re-analyse a document and publish diagnostics to the client.
    async fn publish_diagnostics(&self, uri: &Url, text: &str) {
        let analysis = analyse_kg_document(text, &self.thesaurus);
        let diagnostics = build_diagnostics_with_positions(&analysis, text);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TerraphimLspServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: None,
                    resolve_provider: Some(false),
                    ..CompletionOptions::default()
                }),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: Some("terraphim-lsp".to_string()),
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                ..ServerCapabilities::default()
            },
            ..InitializeResult::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("terraphim_lsp initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents
                .write()
                .await
                .insert(uri.clone(), text.clone());
            self.publish_diagnostics(&uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let text = {
            let documents = self.documents.read().await;
            match documents.get(&uri) {
                Some(text) => text.clone(),
                None => return Ok(None),
            }
        };

        let analysis = analyse_kg_document(&text, &self.thesaurus);
        let offset = position_to_byte_offset(&text, position);

        for matched in &analysis.matched_terms {
            if matched.range.0 <= offset && offset <= matched.range.1 {
                let contents = match &matched.description {
                    Some(desc) => format!("**{}**\n\n{}", matched.term, desc),
                    None => format!("**{}**", matched.term),
                };
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(byte_range_to_lsp_range(
                        &text,
                        matched.range.0,
                        matched.range.1,
                    )),
                }));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let text = {
            let documents = self.documents.read().await;
            match documents.get(&uri) {
                Some(text) => text.clone(),
                None => return Ok(None),
            }
        };

        let word = word_at_position(&text, position);
        let items = build_completions(&self.thesaurus, &word);

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn diagnostic(
        &self,
        params: DocumentDiagnosticParams,
    ) -> Result<DocumentDiagnosticReportResult> {
        let uri = &params.text_document.uri;
        let text = {
            let documents = self.documents.read().await;
            match documents.get(uri) {
                Some(text) => text.clone(),
                None => {
                    return Ok(DocumentDiagnosticReportResult::Report(
                        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                                result_id: None,
                                items: vec![],
                            },
                            related_documents: None,
                        }),
                    ));
                }
            }
        };

        let analysis = analyse_kg_document(&text, &self.thesaurus);
        let items = build_diagnostics_with_positions(&analysis, &text);

        Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items,
                },
                related_documents: None,
            }),
        ))
    }
}

/// Convert an LSP position to a byte offset in a UTF-8 string.
fn position_to_byte_offset(text: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (line_idx, line) in text.lines().enumerate() {
        if line_idx == position.line as usize {
            let line_offset = position.character as usize;
            return offset + line_offset.min(line.len());
        }
        offset += line.len() + 1; // +1 for '\n'
    }
    offset
}

/// Convert a byte range to an LSP range.
fn byte_range_to_lsp_range(text: &str, start: usize, end: usize) -> Range {
    Range {
        start: byte_offset_to_position(text, start),
        end: byte_offset_to_position(text, end),
    }
}

/// Convert a byte offset to an LSP position.
fn byte_offset_to_position(text: &str, byte_offset: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;

    for (idx, ch) in text.char_indices() {
        if idx >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += ch.len_utf16() as u32;
        }
    }

    Position { line, character }
}
