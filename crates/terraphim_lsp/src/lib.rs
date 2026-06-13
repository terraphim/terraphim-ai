//! Language Server Protocol (LSP) support for Terraphim knowledge graphs.
//!
//! Provides LSP hover, completion, and diagnostics for KG markdown files,
//! enabling editor support for authoring Terraphim knowledge-graph content.

pub mod kg_analysis;
pub mod server;

pub use server::TerraphimLspServer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_module_is_public() {
        // Compilation test: server module and struct are reachable.
        let _ = std::any::type_name::<TerraphimLspServer>();
    }
}
