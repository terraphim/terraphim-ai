//! Git workspace management
//!
//! Provides git operations for workspace branch management:
//! - Branch creation and checkout
//! - Stash management
//! - State restoration

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Errors that can occur during git operations
#[derive(thiserror::Error, Debug)]
pub enum GitError {
    #[error("Git command failed: {0}")]
    CommandFailed(String),

    #[error("Not a git repository: {0}")]
    NotARepository(PathBuf),

    #[error("Branch operation failed: {0}")]
    BranchError(String),

    #[error("Stash operation failed: {0}")]
    StashError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for git operations
pub type Result<T> = std::result::Result<T, GitError>;

/// Git workspace for managing git operations
#[derive(Debug, Clone)]
pub struct GitWorkspace {
    working_dir: PathBuf,
    original_branch: Option<String>,
    stashed: bool,
}

impl GitWorkspace {
    /// Create a new git workspace
    pub fn new(working_dir: &Path) -> Result<Self> {
        if !Self::is_git_repo(working_dir) {
            return Err(GitError::NotARepository(working_dir.to_path_buf()));
        }

        Ok(Self {
            working_dir: working_dir.to_path_buf(),
            original_branch: None,
            stashed: false,
        })
    }

    /// Check if a directory is a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").exists()
            || path
                .parent()
                .map(|p| p.join(".git").exists())
                .unwrap_or(false)
    }

    /// Get the current branch name
    pub async fn current_branch(&self) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch.is_empty() {
                Ok(None)
            } else {
                Ok(Some(branch))
            }
        } else {
            Err(GitError::CommandFailed(format!(
                "Failed to get current branch: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Check if the working directory is clean
    pub async fn is_clean(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().is_empty())
        } else {
            Err(GitError::CommandFailed(format!(
                "Failed to check status: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Stash any uncommitted changes
    pub async fn stash(&mut self) -> Result<()> {
        if self.is_clean().await? {
            debug!("Working directory is clean, no need to stash");
            return Ok(());
        }

        info!("Stashing uncommitted changes");

        let output = Command::new("git")
            .args(["stash", "push", "-m", "terraphim-workspace-auto-stash"])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            self.stashed = true;
            info!("Changes stashed successfully");
            Ok(())
        } else {
            Err(GitError::StashError(format!(
                "Failed to stash: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Pop the most recent stash
    pub async fn stash_pop(&mut self) -> Result<()> {
        if !self.stashed {
            debug!("No stash to pop");
            return Ok(());
        }

        info!("Popping stash");

        let output = Command::new("git")
            .args(["stash", "pop"])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            self.stashed = false;
            info!("Stash popped successfully");
            Ok(())
        } else {
            // Don't clear stashed flag on failure - might need manual intervention
            Err(GitError::StashError(format!(
                "Failed to pop stash: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Create a new branch
    pub async fn create_branch(&self, branch_name: &str) -> Result<()> {
        info!(branch = %branch_name, "Creating new branch");

        let output = Command::new("git")
            .args(["checkout", "-b", branch_name])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            info!(branch = %branch_name, "Branch created and checked out");
            Ok(())
        } else {
            Err(GitError::BranchError(format!(
                "Failed to create branch '{}': {}",
                branch_name,
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Checkout an existing branch
    pub async fn checkout_branch(&self, branch_name: &str) -> Result<()> {
        info!(branch = %branch_name, "Checking out branch");

        let output = Command::new("git")
            .args(["checkout", branch_name])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            info!(branch = %branch_name, "Branch checked out");
            Ok(())
        } else {
            Err(GitError::BranchError(format!(
                "Failed to checkout branch '{}': {}",
                branch_name,
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Checkout a branch, creating it if it doesn't exist
    pub async fn checkout_or_create_branch(&self, branch_name: &str) -> Result<()> {
        // First try to checkout existing branch
        match self.checkout_branch(branch_name).await {
            Ok(()) => Ok(()),
            Err(_) => {
                // Branch doesn't exist, create it
                self.create_branch(branch_name).await
            }
        }
    }

    /// Get list of branches
    pub async fn list_branches(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args(["branch", "-a"])
            .current_dir(&self.working_dir)
            .output()
            .await?;

        if output.status.success() {
            let branches = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().trim_start_matches('*').trim().to_string())
                .filter(|b| !b.is_empty())
                .collect();
            Ok(branches)
        } else {
            Err(GitError::CommandFailed(format!(
                "Failed to list branches: {}",
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    /// Check if a branch exists
    pub async fn branch_exists(&self, branch_name: &str) -> Result<bool> {
        let branches = self.list_branches().await?;
        Ok(branches
            .iter()
            .any(|b| b == branch_name || b.ends_with(&format!("/{}", branch_name))))
    }

    /// Save the current state (branch and stash)
    pub async fn save_state(&mut self) -> Result<()> {
        self.original_branch = self.current_branch().await?;
        self.stash().await?;
        Ok(())
    }

    /// Restore the saved state
    pub async fn restore_state(&mut self) -> Result<()> {
        // Pop stash first
        if let Err(e) = self.stash_pop().await {
            warn!(error = %e, "Failed to pop stash during restore");
        }

        // Restore original branch if different
        if let Some(ref original) = self.original_branch {
            let current = self.current_branch().await?;
            if current.as_ref() != Some(original) {
                if let Err(e) = self.checkout_branch(original).await {
                    error!(error = %e, branch = %original, "Failed to restore original branch");
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Get the working directory
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// Check if we have stashed changes
    pub fn has_stashed(&self) -> bool {
        self.stashed
    }

    /// Get the original branch (if saved)
    pub fn original_branch(&self) -> Option<&str> {
        self.original_branch.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::process::Command;

    async fn create_test_repo(path: &Path) -> Result<()> {
        // Initialize git repo
        let output = Command::new("git")
            .args(["init"])
            .current_dir(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await?;

        if !output.status.success() {
            return Err(GitError::CommandFailed("git init failed".to_string()));
        }

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@terraphim.ai"])
            .current_dir(path)
            .output()
            .await?;

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(path)
            .output()
            .await?;

        // Create initial commit
        let readme = path.join("README.md");
        tokio::fs::write(&readme, "# Test Repo\n").await?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .await?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await?;

        Ok(())
    }

    #[test]
    fn test_is_git_repo() {
        let temp_dir = tempfile::tempdir().unwrap();
        assert!(!GitWorkspace::is_git_repo(temp_dir.path()));

        // Create .git directory
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();
        assert!(GitWorkspace::is_git_repo(temp_dir.path()));
    }

    #[tokio::test]
    async fn test_git_workspace_creation() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Should fail for non-git directory
        assert!(GitWorkspace::new(temp_dir.path()).is_err());

        // Create git repo
        create_test_repo(temp_dir.path()).await.unwrap();

        // Should succeed now
        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();
        assert_eq!(workspace.working_dir(), temp_dir.path());
    }

    #[tokio::test]
    async fn test_current_branch() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();
        let branch = workspace.current_branch().await.unwrap();

        // Should have a branch (usually "master" or "main")
        assert!(branch.is_some());
    }

    #[tokio::test]
    async fn test_is_clean() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();

        // Should be clean initially
        assert!(workspace.is_clean().await.unwrap());

        // Create a file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Should not be clean now
        assert!(!workspace.is_clean().await.unwrap());
    }

    #[tokio::test]
    async fn test_branch_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();
        let original = workspace.current_branch().await.unwrap().unwrap();

        // Create a new branch
        workspace.create_branch("test-branch").await.unwrap();

        // Should be on the new branch
        let current = workspace.current_branch().await.unwrap().unwrap();
        assert_eq!(current, "test-branch");

        // Switch back to original
        workspace.checkout_branch(&original).await.unwrap();
        let current = workspace.current_branch().await.unwrap().unwrap();
        assert_eq!(current, original);

        // Test checkout_or_create_branch with existing branch
        workspace
            .checkout_or_create_branch("test-branch")
            .await
            .unwrap();
        let current = workspace.current_branch().await.unwrap().unwrap();
        assert_eq!(current, "test-branch");

        // Test checkout_or_create_branch with new branch
        workspace
            .checkout_or_create_branch("another-branch")
            .await
            .unwrap();
        let current = workspace.current_branch().await.unwrap().unwrap();
        assert_eq!(current, "another-branch");
    }

    #[tokio::test]
    async fn test_branch_exists() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();
        let original = workspace.current_branch().await.unwrap().unwrap();

        // Original branch should exist
        assert!(workspace.branch_exists(&original).await.unwrap());

        // New branch should not exist
        assert!(!workspace.branch_exists("nonexistent-branch").await.unwrap());

        // Create branch
        workspace.create_branch("new-branch").await.unwrap();
        assert!(workspace.branch_exists("new-branch").await.unwrap());
    }

    #[tokio::test]
    async fn test_stash_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let mut workspace = GitWorkspace::new(temp_dir.path()).unwrap();

        // Create a file and add it to git
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Add the file to git index so it can be stashed
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(temp_dir.path())
            .output()
            .await
            .unwrap();

        // Should not be clean (file is staged)
        assert!(!workspace.is_clean().await.unwrap());

        // Stash
        workspace.stash().await.unwrap();
        assert!(workspace.has_stashed());

        // Should be clean now
        assert!(workspace.is_clean().await.unwrap());

        // Pop stash
        workspace.stash_pop().await.unwrap();
        assert!(!workspace.has_stashed());

        // Should not be clean again (file is restored)
        assert!(!workspace.is_clean().await.unwrap());
    }

    #[tokio::test]
    async fn test_save_and_restore_state() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let mut workspace = GitWorkspace::new(temp_dir.path()).unwrap();
        let _original_branch = workspace.current_branch().await.unwrap().unwrap();

        // Create a file before switching branches
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test content").await.unwrap();

        // Create and switch to a new branch
        workspace.create_branch("temp-branch").await.unwrap();
        assert_eq!(
            workspace.current_branch().await.unwrap().unwrap(),
            "temp-branch"
        );

        // Create another file on the new branch
        let test_file2 = temp_dir.path().join("test2.txt");
        tokio::fs::write(&test_file2, "more content").await.unwrap();

        // Save state - this should record original_branch as the branch we came from
        // and stash current changes
        workspace.save_state().await.unwrap();
        assert!(workspace.has_stashed());
        // Note: original_branch is set by save_state, which gets current branch
        // Since we switched to temp-branch, that's what's saved
        assert_eq!(workspace.original_branch(), Some("temp-branch"));

        // Restore state
        workspace.restore_state().await.unwrap();

        // Should be back on temp-branch (which was the original when save_state was called)
        assert_eq!(
            workspace.current_branch().await.unwrap().unwrap(),
            "temp-branch"
        );
    }

    #[tokio::test]
    async fn test_list_branches() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_repo(temp_dir.path()).await.unwrap();

        let workspace = GitWorkspace::new(temp_dir.path()).unwrap();

        // Create some branches
        workspace.create_branch("branch-a").await.unwrap();
        workspace.checkout_branch("master").await.unwrap();
        workspace.create_branch("branch-b").await.unwrap();
        workspace.checkout_branch("master").await.unwrap();

        let branches = workspace.list_branches().await.unwrap();

        // Should have at least master, branch-a, and branch-b
        assert!(branches.len() >= 3);
        assert!(branches.iter().any(|b| b == "master" || b == "main"));
        assert!(branches.iter().any(|b| b == "branch-a"));
        assert!(branches.iter().any(|b| b == "branch-b"));
    }
}
