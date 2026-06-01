//! Language Server Protocol (LSP) support for Terraphim knowledge graphs.
//!
//! Provides LSP diagnostics for KG markdown and Rust files via the
//! Explicit Deferral Marker (EDM) scanner, enabling editor support for
//! authoring Terraphim knowledge-graph content.

mod config;
mod diagnostic;
mod server;

pub use config::LspConfig;
pub use diagnostic::finding_to_diagnostic;
pub use server::TerraphimLspServer;
