//! LSP server implementation for Terraphim knowledge graphs.
//!
//! Implements the `LanguageServer` trait from `tower-lsp`, wiring up hover,
//! completion, diagnostics and synonym code actions to the pure
//! [`terraphim_lsp_core`] analysis engine.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use terraphim_lsp_core::{KgEngine, LineIndex};
use terraphim_types::Thesaurus;

use crate::completion::{build_completions, word_at_position};
use crate::convert;
use crate::diagnostics::build_diagnostics_with_positions;
use crate::kg_analysis::analyse_kg_document;

/// Terraphim LSP server backed by a knowledge-graph thesaurus.
///
/// The server tracks open text documents and provides:
///
/// - `textDocument/hover` - concept descriptions for matched KG terms
/// - `textDocument/completion` - thesaurus term suggestions
/// - `textDocument/diagnostic` - warnings for unknown terms and a malformed
///   `terraphim-alternatives` annotation block
/// - `textDocument/codeAction` - "Replace with X" for every other synonym of
///   the KG term at the cursor, keeping capitalisation and fixing `a`/`an`
#[derive(Debug)]
pub struct TerraphimLspServer {
    client: Client,
    thesaurus: Thesaurus,
    engine: KgEngine,
    documents: Arc<RwLock<HashMap<Url, String>>>,
}

/// The code-action kind of the synonym replacements.
const REPLACE_KIND: CodeActionKind = CodeActionKind::REFACTOR_REWRITE;

impl TerraphimLspServer {
    /// Create a new LSP server instance tied to the given LSP client and
    /// knowledge-graph thesaurus.
    ///
    /// If the thesaurus cannot be compiled into a matcher, the error is logged
    /// and the server runs without KG matches rather than failing to start.
    pub fn new(client: Client, thesaurus: Thesaurus) -> Self {
        let engine = KgEngine::new(&thesaurus).unwrap_or_else(|error| {
            log::error!("terraphim_lsp: KG engine unavailable: {error}");
            KgEngine::empty()
        });
        if !engine.skipped_patterns().is_empty() {
            log::warn!(
                "terraphim_lsp: {} thesaurus keys too short to match: {:?}",
                engine.skipped_patterns().len(),
                engine.skipped_patterns()
            );
        }
        Self {
            client,
            thesaurus,
            engine,
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

    /// The current text of an open document.
    async fn document_text(&self, uri: &Url) -> Option<String> {
        self.documents.read().await.get(uri).cloned()
    }

    /// "Replace with X" code actions for the KG term at `position`.
    fn replacement_actions(&self, uri: &Url, text: &str, position: Position) -> Vec<CodeAction> {
        let index = LineIndex::new(text);
        let offset = convert::byte_offset(&index, position);
        let Some(set) = self.engine.alternatives_at(text, offset) else {
            return Vec::new();
        };
        set.replacements
            .iter()
            .map(|replacement| {
                let edits = replacement
                    .edits
                    .iter()
                    .map(|edit| convert::text_edit(&index, edit))
                    .collect();
                CodeAction {
                    title: format!("Replace with {}", replacement.text),
                    kind: Some(REPLACE_KIND),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from([(uri.clone(), edits)])),
                        ..WorkspaceEdit::default()
                    }),
                    ..CodeAction::default()
                }
            })
            .collect()
    }

    /// Re-analyse a document and publish diagnostics to the client.
    async fn publish_diagnostics(&self, uri: &Url, text: &str) {
        let analysis = analyse_kg_document(text, &self.engine);
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
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![REPLACE_KIND]),
                        resolve_provider: Some(false),
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
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

        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };

        let analysis = analyse_kg_document(&text, &self.engine);
        let index = LineIndex::new(&text);
        let offset = convert::byte_offset(&index, position);

        let hover = analysis
            .matched_terms
            .iter()
            .find(|matched| matched.range.touches_byte(offset))
            .map(|matched| {
                let contents = match &matched.description {
                    Some(desc) => format!("**{}**\n\n{}", matched.term, desc),
                    None => format!("**{}**", matched.term),
                };
                Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: contents,
                    }),
                    range: Some(convert::range(&index, matched.range)),
                }
            });
        Ok(hover)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        if let Some(only) = &params.context.only
            && !only.iter().any(|kind| kind_includes(kind, &REPLACE_KIND))
        {
            return Ok(None);
        }
        let uri = params.text_document.uri;
        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };
        let actions: Vec<CodeActionOrCommand> = self
            .replacement_actions(&uri, &text, params.range.start)
            .into_iter()
            .map(CodeActionOrCommand::CodeAction)
            .collect();
        Ok((!actions.is_empty()).then_some(actions))
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

        let analysis = analyse_kg_document(&text, &self.engine);
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

/// Whether a requested code-action kind (`params.context.only`) includes
/// `kind`: equal, or a dot-separated prefix of it (`refactor` includes
/// `refactor.rewrite`).
fn kind_includes(requested: &CodeActionKind, kind: &CodeActionKind) -> bool {
    let (requested, kind) = (requested.as_str(), kind.as_str());
    kind == requested
        || kind
            .strip_prefix(requested)
            .is_some_and(|rest| rest.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_kinds_include_by_prefix() {
        let rewrite = CodeActionKind::REFACTOR_REWRITE;
        assert!(kind_includes(&CodeActionKind::REFACTOR, &rewrite));
        assert!(kind_includes(&CodeActionKind::REFACTOR_REWRITE, &rewrite));
        assert!(!kind_includes(&CodeActionKind::QUICKFIX, &rewrite));
        assert!(!kind_includes(
            &CodeActionKind::new("refactor.re"),
            &rewrite
        ));
    }
}
