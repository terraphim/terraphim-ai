//! Artifact vocabulary for the Terraphim platform runtime.

use std::path::PathBuf;

/// A file produced or consumed by the platform runtime.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    /// Absolute or workspace-relative path.
    pub path: PathBuf,
    /// Content-addressable hash.
    pub content_hash: String,
    /// Classification of the artifact.
    pub kind: ArtifactKind,
}

/// Classification of an artifact for routing and retention policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactKind {
    /// Cargo build target directory contents.
    CargoTarget,
    /// Compiled binary executable.
    BinaryExecutable,
    /// Plain-text log file.
    LogFile,
    /// Structured JSON report.
    JsonReport,
}
