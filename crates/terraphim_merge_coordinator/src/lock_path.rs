//! Project-scoped cross-process kernel lease for the standalone
//! merge-coordinator binary (#3295, design §D5).
//!
//! `merge-coordinator` historically took a single global lock at
//! `/tmp/merge-coordinator.lock` (world-writable, no project identity, with
//! timestamp-based stealing). This module replaces it with the shared
//! `terraphim_lockfile::LeaseLock` primitive, parameterised by `(owner, repo)`
//! so unrelated targets do not serialise each other. The shared byte contract
//! (path shape `merge-coordinator-{owner}--{repo}.lock`, payload
//! `pid=<pid> acquired=<unix_secs>`, no steal / no unlink) is frozen in
//! `terraphim_lockfile` and is identical across both repositories
//! (`terraphim-ai` and `terraphim-agents`).
//!
//! Lock-root resolution:
//! - `MERGE_COORDINATOR_LOCK_DIR` env var if set
//! - default `/opt/ai-dark-factory/data/locks`
//!
//! Charset enforcement on `owner` / `repo` is delegated to
//! `terraphim_lockfile::validate_key_component` (regex
//! `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$`). Violations fail closed at config
//! load (binary exit `Critical`).

use std::path::{Path, PathBuf};

use terraphim_lockfile::{LeaseError, LeaseGuard, LeaseLock, validate_key_component};

use crate::types::ExitCode;

/// Build the per-(owner,repo) kernel lease file path.
///
/// Returns the `LeaseError::Io` variant (mapped by callers to
/// `ExitCode::Critical`) when either component violates the shared charset
/// `^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$` (e.g. `../`, `/`, `\`, non-ASCII,
/// or empty). Otherwise the returned path has the form
/// `<dir>/merge-coordinator-{owner}--{repo}.lock` and is guaranteed to be
/// inside `dir` (defence-in-depth against path traversal).
pub fn resolve_lock_path(
    dir: &Path,
    owner: &str,
    repo: &str,
) -> Result<PathBuf, terraphim_lockfile::LeaseError> {
    validate_key_component(owner).map_err(|e| {
        LeaseError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid owner {owner:?}: {e}"),
        ))
    })?;
    validate_key_component(repo).map_err(|e| {
        LeaseError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid repo {repo:?}: {e}"),
        ))
    })?;
    let key = format!("merge-coordinator-{owner}--{repo}");
    Ok(dir.join(format!("{key}.lock")))
}

/// Default lock directory if `MERGE_COORDINATOR_LOCK_DIR` is unset.
pub const LOCK_DIR_DEFAULT: &str = "/opt/ai-dark-factory/data/locks";

/// Resolve the lock directory for this run.
pub fn resolve_lock_dir() -> PathBuf {
    if let Ok(p) = std::env::var("MERGE_COORDINATOR_LOCK_DIR")
        && !p.is_empty()
    {
        return PathBuf::from(p);
    }
    PathBuf::from(LOCK_DIR_DEFAULT)
}

/// Acquire the project-scoped nonblocking kernel lease for `(owner, repo)`.
///
/// - `LockHeld` (`LeaseHeld { holder_pid }`) is the SINGLE benign outcome
///   (single-flight overlap, design §D5).
/// - Every I/O or charset error fails closed with `LeaseError::Io`.
pub fn acquire_project_lock(
    dir: &Path,
    owner: &str,
    repo: &str,
) -> Result<LeaseGuard, terraphim_lockfile::LeaseError> {
    let path = resolve_lock_path(dir, owner, repo)?;
    let key = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".lock"))
        .unwrap_or("merge-coordinator");
    LeaseLock::acquire(dir, key)
}

/// Acquire the run-level project lease consumed by the binary's `main`.
///
/// Thin seam over [`acquire_project_lock`]: maps the single benign kernel
/// contention outcome into [`RunOutcome::LockHeld`] so `main` can emit
/// `lock.held` and exit `Success` (benign no-op), while every I/O or
/// configuration failure surfaces as [`RunOutcome::Io`] and fails closed
/// with `ExitCode::Critical`. The returned [`LeaseGuard`] must be held for
/// the whole coordinator run (RAII release on drop; never unlinked).
pub fn acquire_run_lock(dir: &Path, owner: &str, repo: &str) -> Result<LeaseGuard, RunOutcome> {
    acquire_project_lock(dir, owner, repo).map_err(RunOutcome::from)
}

/// Outcome categories for the merge-coordinator run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunOutcome {
    /// Another holder holds the kernel lease. Single-flight overlap is benign.
    LockHeld {
        /// Best-effort diagnostics from the payload; never used for correctness.
        holder_pid: Option<u32>,
    },
    /// I/O or configuration failure; the run must fail closed.
    Io(String),
}

impl From<LeaseError> for RunOutcome {
    fn from(e: LeaseError) -> Self {
        match e {
            LeaseError::LockHeld { holder_pid } => RunOutcome::LockHeld { holder_pid },
            LeaseError::Io(io) => RunOutcome::Io(io.to_string()),
        }
    }
}

impl From<RunOutcome> for crate::types::MergeCoordinatorError {
    fn from(o: RunOutcome) -> Self {
        match o {
            RunOutcome::LockHeld { holder_pid } => crate::types::MergeCoordinatorError::LockHeld {
                pid: holder_pid.map(|p| p as i32).unwrap_or(0),
                age_secs: 0,
            },
            RunOutcome::Io(msg) => crate::types::MergeCoordinatorError::Api(msg),
        }
    }
}

/// Map a `RunOutcome` to the binary's exit code.
pub fn exit_code_for_run_outcome(outcome: RunOutcome) -> ExitCode {
    match outcome {
        RunOutcome::LockHeld { .. } => ExitCode::Success,
        RunOutcome::Io(_) => ExitCode::Critical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_lock_path_uses_owner_repo_in_filename() {
        let p = resolve_lock_path(Path::new("/var/locks"), "terraphim", "terraphim-ai")
            .expect("valid components");
        assert_eq!(
            p,
            PathBuf::from("/var/locks/merge-coordinator-terraphim--terraphim-ai.lock"),
            "exact filename per design D5"
        );
    }

    #[test]
    fn resolve_lock_path_rejects_path_traversal() {
        for bad_owner in [
            "../",
            "a/b",
            "terraphim/ai",
            "",
            "..",
            "-leading",
            ".leading",
        ] {
            assert!(
                resolve_lock_path(Path::new("/var/locks"), bad_owner, "repo").is_err(),
                "invalid owner {bad_owner:?} must be rejected"
            );
        }
    }

    #[test]
    fn resolve_lock_path_rejects_non_ascii_repo() {
        for bad_repo in ["a/b", "a\\b", "café", "", "terraphim/ai"] {
            assert!(
                resolve_lock_path(Path::new("/var/locks"), "terraphim", bad_repo).is_err(),
                "invalid repo {bad_repo:?} must be rejected"
            );
        }
    }

    #[test]
    fn resolve_lock_path_accepts_dotted_hyphen_underscore() {
        let p = resolve_lock_path(Path::new("/opt/x"), "my.org", "repo_name-v2")
            .expect("valid components");
        assert_eq!(
            p,
            PathBuf::from("/opt/x/merge-coordinator-my.org--repo_name-v2.lock")
        );
    }

    #[test]
    fn distinct_owner_repo_pairs_yield_distinct_lock_paths() {
        let a =
            resolve_lock_path(Path::new("/var/locks"), "terraphim", "terraphim-ai").expect("valid");
        let b = resolve_lock_path(Path::new("/var/locks"), "other", "repo").expect("valid");
        assert_ne!(a, b, "per-repo keys must not collide");
    }

    #[test]
    fn exit_code_for_lock_held_is_success() {
        let o = RunOutcome::LockHeld {
            holder_pid: Some(42),
        };
        assert_eq!(exit_code_for_run_outcome(o), ExitCode::Success);
    }

    #[test]
    fn exit_code_for_io_is_critical() {
        let o = RunOutcome::Io("/var/locks not writable".into());
        assert_eq!(exit_code_for_run_outcome(o), ExitCode::Critical);
    }

    #[test]
    fn lease_error_to_run_outcome_lock_held() {
        let o: RunOutcome = LeaseError::LockHeld {
            holder_pid: Some(99),
        }
        .into();
        assert_eq!(
            o,
            RunOutcome::LockHeld {
                holder_pid: Some(99)
            }
        );
    }

    #[test]
    fn lease_error_to_run_outcome_io() {
        let o: RunOutcome = LeaseError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ))
        .into();
        match o {
            RunOutcome::Io(msg) => assert!(msg.contains("denied"), "got {msg:?}"),
            other => panic!("expected Io, got {other:?}"),
        }
    }
}
