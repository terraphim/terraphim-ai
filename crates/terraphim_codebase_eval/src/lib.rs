//! Terraphim Codebase Evaluation Manifest.
//!
//! Provides typed manifest models and loaders for the before/after
//! AI-agent codebase evaluation flow defined in the
//! `terraphim-codebase-eval-check` specification.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//! use terraphim_codebase_eval::load_manifest;
//!
//! let manifest = load_manifest(Path::new("fixtures/manifest-minimal.toml"))?;
//! println!("Loaded manifest with {} queries", manifest.queries.len());
//! # Ok::<(), terraphim_codebase_eval::ManifestError>(())
//! ```

mod error;
mod manifest;

pub use error::ManifestError;
pub use manifest::{
    BaselineOrCandidate, EvaluationManifest, HaystackDescriptor, MetricRecord, QuerySpec,
    RoleDefinition, ScoringWeights, Thresholds,
};

use std::path::Path;

/// Load an evaluation manifest from a file.
///
/// Auto-detects format by extension:
/// - `.toml` -- TOML format
/// - `.yaml` / `.yml` -- YAML format (if `yaml` feature enabled)
///
/// # Errors
///
/// Returns `ManifestError::InvalidPath` if the file does not exist,
/// `ManifestError::UnsupportedFormat` if the extension is unrecognised,
/// or `ManifestError::ParseError` if deserialisation fails.
pub fn load_manifest(path: &Path) -> Result<EvaluationManifest, ManifestError> {
    if !path.exists() {
        return Err(ManifestError::InvalidPath(path.to_path_buf()));
    }

    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let content = std::fs::read_to_string(path).map_err(|e| ManifestError::ParseError {
        source: e.into(),
        context: format!("reading {}", path.display()),
    })?;

    match extension.as_str() {
        "toml" => {
            let manifest: EvaluationManifest =
                toml::from_str(&content).map_err(|e| ManifestError::ParseError {
                    source: e.into(),
                    context: format!("parsing TOML from {}", path.display()),
                })?;
            Ok(manifest)
        }
        _ => Err(ManifestError::UnsupportedFormat(extension)),
    }
}
