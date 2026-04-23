//! Error types for manifest loading and validation.

use std::path::PathBuf;

/// Errors that can occur when loading or validating an evaluation manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The specified path does not exist.
    #[error("manifest file not found: {0}")]
    InvalidPath(PathBuf),

    /// The file extension is not a supported manifest format.
    #[error("unsupported manifest format: '{0}' (expected .toml, .yaml, or .yml)")]
    UnsupportedFormat(String),

    /// An error occurred during parsing (TOML or YAML).
    #[error("failed to parse manifest: {context}")]
    ParseError {
        /// The underlying error.
        source: Box<dyn std::error::Error + Send + Sync>,
        /// Additional context about where the error occurred.
        context: String,
    },

    /// The manifest failed validation.
    #[error("manifest validation failed: {0}")]
    Validation(String),
}
