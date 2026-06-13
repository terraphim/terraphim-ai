use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Placeholder LSP server for Terraphim knowledge graphs.
///
/// Step 3 of the components-functionality epic will add document tracking,
/// KG analysis, and handlers for hover, completion, and diagnostics.
#[derive(Debug)]
pub struct TerraphimLspServer {
    #[allow(dead_code)]
    client: Client,
}

impl TerraphimLspServer {
    /// Create a new LSP server instance tied to the given LSP client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Run the LSP server over stdio.
    pub async fn run_stdio(self) {
        let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
        let (service, socket) = LspService::new(Self::new);
        Server::new(stdin, stdout, socket).serve(service).await;
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
}
