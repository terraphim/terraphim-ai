//! Binary discovery utilities for terraphim-agent.

use std::path::{Path, PathBuf};

/// Location where terraphim-agent was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryLocation {
    /// Found in system PATH
    Path,
    /// Found in local target/release directory
    LocalRelease(PathBuf),
    /// Found in ~/.cargo/bin
    CargoHome(PathBuf),
}

impl BinaryLocation {
    /// Get the path to the binary.
    pub fn path(&self) -> PathBuf {
        match self {
            BinaryLocation::Path => PathBuf::from("terraphim-agent"),
            BinaryLocation::LocalRelease(p) | BinaryLocation::CargoHome(p) => p.clone(),
        }
    }
}

/// Discover terraphim-agent binary location.
///
/// Searches in order:
/// 1. System PATH
/// 2. ./target/release/terraphim-agent (local build)
/// 3. ~/.cargo/bin/terraphim-agent (cargo install)
///
/// Returns `None` if binary not found in any location.
pub fn discover_binary() -> Option<BinaryLocation> {
    // Check PATH first
    if which_in_path("terraphim-agent") {
        return Some(BinaryLocation::Path);
    }

    // Check local release build
    let local_release = PathBuf::from("./target/release/terraphim-agent");
    if local_release.exists() && is_executable(&local_release) {
        return Some(BinaryLocation::LocalRelease(local_release));
    }

    // Check cargo home
    if let Some(home) = dirs::home_dir() {
        let cargo_bin = home.join(".cargo/bin/terraphim-agent");
        if cargo_bin.exists() && is_executable(&cargo_bin) {
            return Some(BinaryLocation::CargoHome(cargo_bin));
        }
    }

    None
}

fn which_in_path(binary: &str) -> bool {
    std::process::Command::new("which")
        .arg(binary)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_returns_some_or_none() {
        // This test just verifies the function doesn't panic
        let _ = discover_binary();
    }

    #[test]
    fn test_binary_location_path() {
        let loc = BinaryLocation::Path;
        assert_eq!(loc.path(), PathBuf::from("terraphim-agent"));
    }

    #[test]
    fn test_binary_location_local_release() {
        let path = PathBuf::from("/some/path/terraphim-agent");
        let loc = BinaryLocation::LocalRelease(path.clone());
        assert_eq!(loc.path(), path);
    }
}
