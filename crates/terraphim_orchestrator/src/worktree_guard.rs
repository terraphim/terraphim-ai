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
    /// When `Some`, `Drop` runs `git -C <repo_path> worktree remove
    /// --force <path>` first and falls back to a filesystem-only
    /// removal on non-zero exit or when the git CLI is not
    /// invokable. When `None`, only the filesystem path runs (the
    /// existing per-agent caller in `lib.rs`, unchanged).
    repo_path: Option<PathBuf>,
}

impl WorktreeGuard {
    /// Create a new worktree guard for the given path.
    ///
    /// The path will be removed when the guard is dropped unless
    /// `keep()` is called. This constructor performs filesystem-only
    /// cleanup; use `for_managed` for git-aware cleanup of worktrees
    /// created via `WorktreeManager::create_worktree`.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        debug!(path = %path.display(), "worktree guard created");
        Self {
            path,
            should_cleanup: true,
            repo_path: None,
        }
    }

    /// Create a managed guard whose `Drop` invokes `git worktree
    /// remove --force` against `repo_path` before falling back to
    /// filesystem removal.
    ///
    /// Use this when the worktree was created via
    /// `WorktreeManager::create_worktree` so the git admin registry
    /// at `<repo>/.git/worktrees/<name>` is reconciled along with the
    /// directory itself.
    pub fn for_managed<R: AsRef<Path>, P: AsRef<Path>>(repo_path: R, worktree_path: P) -> Self {
        let path = worktree_path.as_ref().to_path_buf();
        let repo = repo_path.as_ref().to_path_buf();
        debug!(
            repo_path = %repo.display(),
            worktree_path = %path.display(),
            "managed worktree guard created"
        );
        Self {
            path,
            should_cleanup: true,
            repo_path: Some(repo),
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

        // Managed path: try `git worktree remove --force` first so the
        // git admin entry at `<repo>/.git/worktrees/<name>` is
        // reconciled. The synchronous std Command is intentional --
        // Drop cannot be async, and git worktree remove is sub-second.
        if let Some(ref repo) = self.repo_path {
            let start = std::time::Instant::now();
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(repo)
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(&self.path)
                .env_remove("GIT_INDEX_FILE")
                .status();

            match status {
                Ok(s) if s.success() => {
                    info!(
                        path = %self.path.display(),
                        duration_ms = start.elapsed().as_millis() as u64,
                        "worktree cleaned up via git"
                    );
                    return;
                }
                Ok(s) => {
                    warn!(
                        path = %self.path.display(),
                        exit_code = ?s.code(),
                        "git worktree remove failed, falling back to fs"
                    );
                }
                Err(e) => {
                    warn!(
                        path = %self.path.display(),
                        error = %e,
                        "git CLI not invokable, falling back to fs"
                    );
                }
            }
        }

        // Fallback / unmanaged path: filesystem-only removal.
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

    /// Minimal real git repo bootstrap for guard tests. Mirrors the
    /// helper in `scope::tests::setup_git_repo` but kept inline here
    /// so the unit tests are self-contained.
    fn init_git_repo() -> TempDir {
        std::env::remove_var("GIT_INDEX_FILE");
        let temp_dir = TempDir::new().expect("temp dir");
        let repo = temp_dir.path();
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(repo)
                .args(args)
                .env_remove("GIT_INDEX_FILE")
                .status()
                .expect("git invocation");
            assert!(status.success(), "git {:?} failed", args);
        };
        std::process::Command::new("git")
            .arg("init")
            .arg(repo)
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("git init");
        run(&["config", "user.email", "test@test.com"]);
        run(&["config", "user.name", "Test User"]);
        std::fs::write(repo.join("README.md"), "# Test").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "init"]);
        temp_dir
    }

    #[test]
    fn test_managed_guard_invokes_git_remove() {
        let repo = init_git_repo();
        let worktree = repo.path().join(".worktrees/managed-remove");

        // Use real git worktree add so the admin entry exists.
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo.path())
            .arg("worktree")
            .arg("add")
            .arg(&worktree)
            .arg("HEAD")
            .env_remove("GIT_INDEX_FILE")
            .status()
            .expect("git worktree add");
        assert!(status.success(), "git worktree add failed");
        assert!(worktree.exists());
        // git admin registry entry exists
        let admin = repo.path().join(".git/worktrees/managed-remove");
        assert!(admin.exists(), "git admin entry should exist");

        {
            let _guard = WorktreeGuard::for_managed(repo.path(), &worktree);
        }

        assert!(
            !worktree.exists(),
            "managed guard should remove worktree dir"
        );
        assert!(
            !admin.exists(),
            "managed guard should reconcile git admin entry"
        );
    }

    #[test]
    fn test_managed_guard_fallback_on_git_failure() {
        // Point repo_path at a non-git directory so `git worktree
        // remove` exits non-zero, exercising the fs fallback.
        let temp_dir = TempDir::new().unwrap();
        let not_a_repo = temp_dir.path().join("not-a-repo");
        std::fs::create_dir(&not_a_repo).unwrap();

        let worktree = temp_dir.path().join("orphan-worktree");
        std::fs::create_dir(&worktree).unwrap();
        File::create(worktree.join("payload.txt")).unwrap();

        {
            let _guard = WorktreeGuard::for_managed(&not_a_repo, &worktree);
        }

        assert!(
            !worktree.exists(),
            "fallback fs removal should remove worktree dir"
        );
    }

    #[test]
    fn test_managed_guard_keep_disarms() {
        let temp_dir = TempDir::new().unwrap();
        let fake_repo = temp_dir.path().join("repo");
        std::fs::create_dir(&fake_repo).unwrap();
        let worktree = temp_dir.path().join("kept-worktree");
        std::fs::create_dir(&worktree).unwrap();

        let guard = WorktreeGuard::for_managed(&fake_repo, &worktree);
        guard.keep();

        assert!(
            worktree.exists(),
            "managed guard with keep() must not remove"
        );
    }
}
