use terraphim_lsp::TerraphimLspServer;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(TerraphimLspServer::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
