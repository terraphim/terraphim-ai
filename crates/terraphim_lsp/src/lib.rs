//! Language Server Protocol (LSP) support for Terraphim knowledge graphs.
//!
//! Provides LSP hover, completion, diagnostics and synonym code actions for
//! KG markdown files, enabling editor support for authoring Terraphim
//! knowledge-graph content. The analysis itself lives in the pure,
//! WASM-buildable [`terraphim_lsp_core`] crate, re-exported as [`core`].

pub mod completion;
mod convert;
pub mod diagnostics;
pub mod kg_analysis;
pub mod server;

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
