//! Workspace vocabulary for the Terraphim platform runtime.

use std::path::PathBuf;

/// Snapshot of a workspace that is ready for execution or validation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Workspace {
    /// Absolute path to the workspace root.
    pub root: PathBuf,
    /// Content-derived hash of the workspace snapshot.
    pub hash: String,
    /// How the workspace root is materialised.
    pub mode: WorkspaceMode,
}

/// Transport or isolation mode for a workspace snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WorkspaceMode {
    /// Direct path on the host filesystem.
    HostPath,
    /// Mirrored via rsync before execution.
    RsyncMirror,
    /// Packaged as a Firecracker rootfs image.
    FirecrackerRootfs,
}
