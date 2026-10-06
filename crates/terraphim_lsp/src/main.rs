//! Terraphim LSP binary.
//!
//! Starts a Language Server Protocol server over stdio. The server provides
//! hover, completion, diagnostics, synonym code actions and inlay hints for
//! Terraphim knowledge-graph markdown documents.
//!
//! The thesaurus comes from the client's `thesaurus` setting, else
//! `--thesaurus <path>`, else the `TERRAPHIM_THESAURUS` environment
//! variable; see [`terraphim_lsp::thesaurus`].

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::thesaurus::{LaunchOptions, USAGE, parse_args};

#[tokio::main]
async fn main() {
    env_logger::init();
    let cli = match parse_args(std::env::args_os().skip(1)) {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("terraphim-lsp: {error}\n\n{USAGE}");
            std::process::exit(2);
        }
    };
    if cli.help {
        print!("{USAGE}");
        return;
    }
    if cli.version {
        println!("terraphim-lsp {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if !cli.ignored.is_empty() {
        log::info!("terraphim-lsp: ignoring arguments {:?}", cli.ignored);
    }
    let launch = LaunchOptions::from_process(&cli);
    TerraphimLspServer::run_stdio_with_launch_options(launch).await;
}
