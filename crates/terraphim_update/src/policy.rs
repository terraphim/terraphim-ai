//! Runtime detection of whether the current binary is managed by a system
//! package manager, as opposed to Terraphim's own self-update mechanism.
//!
//! Detection applies only to the canonical `terraphim_server` executable at
//! `<prefix>/bin/terraphim_server`, and only when the matching receipt at
//! `<prefix>/share/terraphim/package-manager.d/terraphim_server` contains one
//! supported manager value. Missing, malformed, mismatched, receipt-only,
//! layout-only, or unrelated receipts resolve to [`UpdatePolicy::SelfManaged`].
//!
//! This module is plain data + pure functions: no `cfg!`, no Cargo feature.
//! [`detect_update_policy`] takes an executable path as a parameter, so tests
//! can exercise it against `tempfile::TempDir`-rooted stand-ins without
//! touching the real `/usr` tree or process environment.
//! [`detect_update_policy_default`] is the only function that touches real
//! process state.

use std::fs;
use std::path::{Path, PathBuf};

const TRACKED_EXECUTABLE: &str = "terraphim_server";

/// A supported system package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageManager {
    Dpkg,
    Rpm,
    Homebrew,
}

impl PackageManager {
    /// The exact receipt-file value (case-sensitive) that identifies this
    /// manager. Also used as the manager's human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            PackageManager::Dpkg => "dpkg",
            PackageManager::Rpm => "rpm",
            PackageManager::Homebrew => "homebrew",
        }
    }

    /// The operator-facing update command for this manager and binary.
    pub fn update_command(&self, bin_name: &str) -> String {
        match self {
            PackageManager::Dpkg => "sudo apt update && sudo apt upgrade".to_string(),
            PackageManager::Rpm => "sudo dnf upgrade".to_string(),
            PackageManager::Homebrew => format!("brew upgrade {bin_name}"),
        }
    }

    /// Parse an exact receipt-file value into a supported manager. Returns
    /// `None` for anything that isn't an exact byte match: unsupported name,
    /// wrong case, leading/trailing whitespace, embedded whitespace, invalid
    /// UTF-8, extra line endings, or trailing bytes.
    fn from_marker_value(value: &[u8]) -> Option<Self> {
        match value {
            b"dpkg" | b"dpkg\n" | b"dpkg\r\n" => Some(PackageManager::Dpkg),
            b"rpm" | b"rpm\n" | b"rpm\r\n" => Some(PackageManager::Rpm),
            b"homebrew" | b"homebrew\n" | b"homebrew\r\n" => Some(PackageManager::Homebrew),
            _ => None,
        }
    }
}

/// Runtime update policy for a Terraphim binary: whether self-update
/// (network check/download/install) is safe, or whether updates must be
/// deferred to a system package manager instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdatePolicy {
    /// Default: self-update is safe. Resolved whenever detection is
    /// ambiguous, fails, or simply doesn't match a package-managed install.
    SelfManaged,
    /// The running binary was installed by a package manager; self-update
    /// must be a no-op refusal that guides the operator to that manager's
    /// own update command instead.
    PackageManaged {
        manager: PackageManager,
        update_command: String,
    },
}

/// Read and validate the receipt file, returning the supported manager it
/// names, or `None` if the file is missing/unreadable or its contents don't
/// exactly match one supported manager with no line ending, one LF, or one
/// CRLF.
fn read_receipt(receipt_path: &Path) -> Option<PackageManager> {
    let contents = fs::read(receipt_path).ok()?;
    PackageManager::from_marker_value(&contents)
}

/// Infer the install prefix from `<prefix>/bin/<actual-executable-name>`.
pub fn inferred_prefix(current_exe: &Path) -> Option<PathBuf> {
    if actual_executable_basename(current_exe)? != TRACKED_EXECUTABLE {
        return None;
    }
    let bin_dir = current_exe.parent()?;
    if bin_dir.file_name()?.to_str()? != "bin" {
        return None;
    }
    bin_dir.parent().map(Path::to_path_buf)
}

fn actual_executable_basename(current_exe: &Path) -> Option<&str> {
    current_exe.file_name()?.to_str()
}

fn receipt_path(prefix: &Path, bin_name: &str) -> PathBuf {
    prefix
        .join("share/terraphim/package-manager.d")
        .join(bin_name)
}

/// Pure detection: takes the executable path as a parameter. No env var
/// reads, no hardcoded `/usr` paths. Safe to call with temp-dir stand-ins.
///
/// Never panics: any I/O error resolves to [`UpdatePolicy::SelfManaged`].
pub fn detect_update_policy(current_exe: &Path) -> UpdatePolicy {
    let Ok(canonical_exe) = fs::canonicalize(current_exe) else {
        return UpdatePolicy::SelfManaged;
    };

    let Some(actual_bin_name) = actual_executable_basename(&canonical_exe) else {
        return UpdatePolicy::SelfManaged;
    };
    if actual_bin_name != TRACKED_EXECUTABLE {
        return UpdatePolicy::SelfManaged;
    }
    let Some(prefix) = inferred_prefix(&canonical_exe) else {
        return UpdatePolicy::SelfManaged;
    };
    let Some(manager) = read_receipt(&receipt_path(&prefix, TRACKED_EXECUTABLE)) else {
        return UpdatePolicy::SelfManaged;
    };

    UpdatePolicy::PackageManaged {
        manager,
        update_command: manager.update_command(TRACKED_EXECUTABLE),
    }
}

/// The only function that touches real process state: resolves
/// `std::env::current_exe()`, then delegates to [`detect_update_policy`].
pub fn detect_update_policy_default() -> UpdatePolicy {
    let Ok(current_exe) = std::env::current_exe() else {
        return UpdatePolicy::SelfManaged;
    };
    detect_update_policy(&current_exe)
}

/// Stable operator-facing guidance for a `PackageManaged` policy. Returns an
/// empty string for `SelfManaged` (there is nothing to guide toward).
pub fn guidance(policy: &UpdatePolicy, bin_name: &str) -> String {
    match policy {
        UpdatePolicy::SelfManaged => String::new(),
        UpdatePolicy::PackageManaged { update_command, .. } => format!(
            "{bin_name} was installed via a system package manager; run `{update_command}` to update it."
        ),
    }
}
