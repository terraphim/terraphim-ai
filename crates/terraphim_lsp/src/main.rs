//! Terraphim LSP binary.
//!
//! Starts a Language Server Protocol server over stdio. The server provides
//! hover, completion, and diagnostics for Terraphim knowledge-graph markdown
//! documents.

use terraphim_lsp::TerraphimLspServer;

#[tokio::main]
async fn main() {
    env_logger::init();
    TerraphimLspServer::run_stdio().await;
}
