//! Language Server Protocol (LSP) support for Terraphim knowledge graphs.
//!
//! Provides LSP hover, completion, diagnostics (with faded ghosted text),
//! synonym code actions, `[i/n]` inlay hints and `workspace/executeCommand`
//! commands for KG markdown files, enabling editor support for authoring
//! Terraphim knowledge-graph content. The analysis itself lives in the pure,
//! WASM-buildable [`terraphim_lsp_core`] crate, re-exported as [`core`].

pub mod commands;
pub mod completion;
mod convert;
pub mod diagnostics;
pub mod handshake;
pub mod kg_analysis;
pub mod server;
pub mod settings;
pub mod thesaurus;

pub use server::TerraphimLspServer;
pub use terraphim_lsp_core as core;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_module_is_public() {
        // Compilation test: server module and struct are reachable.
        let _ = std::any::type_name::<TerraphimLspServer>();
    }
}
