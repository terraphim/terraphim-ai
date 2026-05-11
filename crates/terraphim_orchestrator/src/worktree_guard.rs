//! RAII worktree guard for automatic cleanup on agent crash or panic.
//!
//! Ensures that agent worktrees are cleaned up even when the agent process
//! exits abnormally (SIGKILL, OOM, panic).
//!
//! # Example
//!
//! ```rust,ignore
//! use terraphim_orchestrator::worktree_guard::WorktreeGuard;
//!
//! {
//!     let guard = WorktreeGuard::new("/tmp/agent-worktree-123");
//!     // ... run agent ...
//! } // worktree automatically cleaned up here
//! ```

use std::path::{Path, PathBuf};

use tracing::{debug, info, warn};

/// RAII guard that removes a worktree directory when dropped.
///
/// Call `keep()` to prevent cleanup (e.g., when the agent succeeds and
/// you want to preserve the worktree for inspection).
#[derive(Debug)]
pub struct WorktreeGuard {
    path: PathBuf,
    should_cleanup: bool,
}

impl WorktreeGuard {
    /// Create a new worktree guard for the given path.
    ///
    /// The path will be removed when the guard is dropped unless
    /// `keep()` is called.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        debug!(path = %path.display(), "worktree guard created");
        Self {
            path,
            should_cleanup: true,
        }
    }

    /// Prevent cleanup when the guard is dropped.
    ///
    /// Call this when the agent succeeds and you want to keep the worktree.
    pub fn keep(mut self) {
        self.should_cleanup = false;
        debug!(path = %self.path.display(), "worktree guard disarmed");
    }

    /// Get the worktree path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Perform the cleanup.
    fn cleanup(&self) {
        if !self.should_cleanup {
            return;
        }

        if !self.path.exists() {
            debug!(path = %self.path.display(), "worktree already removed");
            return;
        }

        match std::fs::remove_dir_all(&self.path) {
            Ok(_) => {
                info!(path = %self.path.display(), "worktree cleaned up");
            }
            Err(e) => {
                warn!(path = %self.path.display(), error = %e, "failed to remove worktree");
                // Try to at least remove the directory entry
                if let Err(e2) = std::fs::remove_dir(&self.path) {
                    debug!(path = %self.path.display(), error = %e2, "failed to remove worktree dir");
                }
            }
        }
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        self.cleanup();
    }
}

/// Scoped worktree guard that wraps a closure and ensures cleanup.
///
/// This is useful when you want to run an agent in a closure and guarantee
/// cleanup regardless of how the closure exits.
pub fn with_worktree_guard<F, T, P: AsRef<Path>>(path: P, f: F) -> T
where
    F: FnOnce(&WorktreeGuard) -> T,
{
    let guard = WorktreeGuard::new(path);
    f(&guard)
}

/// Async version of `with_worktree_guard`.
pub async fn with_worktree_guard_async<F, T, P: AsRef<Path>>(path: P, f: F) -> T
where
    F: std::future::Future<Output = T>,
{
    let _guard = WorktreeGuard::new(path);
    let result = f.await;
    // _guard dropped here, cleanup happens
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_worktree_guard_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree-123");
        std::fs::create_dir(&worktree).unwrap();
        File::create(worktree.join("file.txt")).unwrap();

        assert!(worktree.exists());

        {
            let _guard = WorktreeGuard::new(&worktree);
            // Guard should not cleanup yet
            assert!(worktree.exists());
        }

        // After drop, should be cleaned up
        assert!(!worktree.exists());
    }

    #[test]
    fn test_worktree_guard_keep() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree-456");
        std::fs::create_dir(&worktree).unwrap();

        {
            let guard = WorktreeGuard::new(&worktree);
            guard.keep();
        }

        // Should still exist after keep()
        assert!(worktree.exists());
    }

    #[test]
    fn test_worktree_guard_already_removed() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree-789");
        std::fs::create_dir(&worktree).unwrap();

        {
            let _guard = WorktreeGuard::new(&worktree);
            // Remove manually before guard drops
            std::fs::remove_dir_all(&worktree).unwrap();
        }

        // Should not panic even though already removed
        assert!(!worktree.exists());
    }

    #[test]
    fn test_with_worktree_guard() {
        let temp_dir = TempDir::new().unwrap();
        let worktree = temp_dir.path().join("worktree-scoped");
        std::fs::create_dir(&worktree).unwrap();

        let result = with_worktree_guard(&worktree, |guard| {
            assert!(guard.path().exists());
            42
        });

        assert_eq!(result, 42);
        assert!(!worktree.exists());
    }
}
