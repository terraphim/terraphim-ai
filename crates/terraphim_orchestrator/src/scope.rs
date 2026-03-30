use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Check if `prefix` is a proper path prefix of `path`.
/// Ensures "src/" matches "src/main.rs" but not "src-backup/".
pub(crate) fn is_path_prefix(prefix: &str, path: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }
    path.starts_with(prefix)
        && (prefix.ends_with('/')
            || path.len() == prefix.len()
            || path.as_bytes().get(prefix.len()) == Some(&b'/'))
}

/// A single scope reservation tracking which agent owns which file patterns.
#[derive(Debug, Clone)]
pub struct ScopeReservation {
    /// Unique identifier for this reservation
    pub id: Uuid,
    /// Name of the agent that holds this reservation
    pub agent_name: String,
    /// File patterns (globs) covered by this reservation
    pub file_patterns: HashSet<String>,
    /// When the reservation was created
    pub created_at: Instant,
    /// Correlation ID linking related reservations (e.g., compound review)
    pub correlation_id: Uuid,
}

impl ScopeReservation {
    /// Create a new scope reservation.
    pub fn new(
        agent_name: impl Into<String>,
        file_patterns: HashSet<String>,
        correlation_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            agent_name: agent_name.into(),
            file_patterns,
            created_at: Instant::now(),
            correlation_id,
        }
    }

    /// Check if this reservation's patterns overlap with another set of patterns.
    /// Simple string-based overlap check - patterns are considered overlapping
    /// if any pattern in this reservation is a prefix of or equals any pattern in the other set.
    pub fn overlaps(&self, other_patterns: &HashSet<String>) -> bool {
        for self_pattern in &self.file_patterns {
            for other_pattern in other_patterns {
                // Direct match
                if self_pattern == other_pattern {
                    return true;
                }
                // Prefix overlap: "src/" overlaps with "src/main.rs" but not "src-backup/"
                let self_prefix = self_pattern.trim_end_matches('*');
                let other_prefix = other_pattern.trim_end_matches('*');
                if is_path_prefix(self_prefix, other_pattern)
                    || is_path_prefix(other_prefix, self_pattern)
                {
                    return true;
                }
            }
        }
        false
    }
}

/// Registry for tracking file scope reservations by agents.
///
/// In exclusive mode (nightly loop Phase 2), overlapping patterns are rejected.
/// In non-exclusive mode (compound review), overlapping reads are permitted.
#[derive(Debug)]
pub struct ScopeRegistry {
    reservations: HashMap<Uuid, ScopeReservation>,
    exclusive: bool,
}

impl ScopeRegistry {
    /// Create a new scope registry.
    ///
    /// * `exclusive` - If true, rejects reservations with overlapping patterns.
    ///   If false, allows overlapping reservations.
    pub fn new(exclusive: bool) -> Self {
        Self {
            reservations: HashMap::new(),
            exclusive,
        }
    }

    /// Attempt to reserve a scope for an agent.
    ///
    /// Returns the reservation ID on success, or an error message if the reservation
    /// cannot be made (e.g., overlapping patterns in exclusive mode).
    pub fn reserve(
        &mut self,
        agent_name: &str,
        file_patterns: HashSet<String>,
        correlation_id: Uuid,
    ) -> Result<Uuid, String> {
        if self.exclusive {
            // Check for overlapping patterns in exclusive mode
            for reservation in self.reservations.values() {
                if reservation.overlaps(&file_patterns) {
                    return Err(format!(
                        "Pattern overlap detected with existing reservation {} owned by {}",
                        reservation.id, reservation.agent_name
                    ));
                }
            }
        }

        let reservation = ScopeReservation::new(agent_name, file_patterns, correlation_id);
        let id = reservation.id;
        self.reservations.insert(id, reservation);

        debug!(
            reservation_id = %id,
            agent_name = %agent_name,
            correlation_id = %correlation_id,
            "scope reserved"
        );

        Ok(id)
    }

    /// Release a specific reservation by ID.
    ///
    /// Returns true if the reservation was found and removed, false otherwise.
    pub fn release(&mut self, reservation_id: Uuid) -> bool {
        let removed = self.reservations.remove(&reservation_id).is_some();
        if removed {
            debug!(reservation_id = %reservation_id, "scope released");
        }
        removed
    }

    /// Release all reservations associated with a correlation ID.
    ///
    /// Returns the number of reservations removed.
    pub fn release_by_correlation(&mut self, correlation_id: Uuid) -> usize {
        let to_remove: Vec<Uuid> = self
            .reservations
            .values()
            .filter(|r| r.correlation_id == correlation_id)
            .map(|r| r.id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            self.reservations.remove(&id);
        }

        if count > 0 {
            debug!(correlation_id = %correlation_id, count = count, "scopes released by correlation");
        }

        count
    }

    /// Get all active reservations.
    pub fn active_reservations(&self) -> Vec<&ScopeReservation> {
        self.reservations.values().collect()
    }

    /// Check if an agent has any active reservations.
    pub fn has_reservation(&self, agent_name: &str) -> bool {
        self.reservations
            .values()
            .any(|r| r.agent_name == agent_name)
    }

    /// Get reservations for a specific agent.
    pub fn reservations_for_agent(&self, agent_name: &str) -> Vec<&ScopeReservation> {
        self.reservations
            .values()
            .filter(|r| r.agent_name == agent_name)
            .collect()
    }

    /// Check if the registry is in exclusive mode.
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// Get the number of active reservations.
    pub fn len(&self) -> usize {
        self.reservations.len()
    }

    /// Check if there are no active reservations.
    pub fn is_empty(&self) -> bool {
        self.reservations.is_empty()
    }
}

/// Manages git worktrees for isolated agent workspaces.
///
/// Worktrees allow agents to work on different branches/refs without
/// interfering with the main working directory.
#[derive(Debug, Clone)]
pub struct WorktreeManager {
    repo_path: PathBuf,
    worktree_base: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager for a git repository.
    ///
    /// Worktrees will be created under `<repo>/.worktrees/<name>`.
    pub fn new(repo_path: impl AsRef<Path>) -> Self {
        let repo_path = repo_path.as_ref().to_path_buf();
        let worktree_base = repo_path.join(".worktrees");

        Self {
            repo_path,
            worktree_base,
        }
    }

    /// Create a worktree manager with a custom base directory.
    ///
    /// Worktrees will be created under `<worktree_base>/<name>`.
    pub fn with_base(repo_path: impl AsRef<Path>, worktree_base: impl AsRef<Path>) -> Self {
        let repo = repo_path.as_ref().to_path_buf();
        let base = worktree_base.as_ref().to_path_buf();
        // Resolve relative worktree_base against repo_path to avoid CWD-dependent behaviour
        let resolved_base = if base.is_relative() {
            repo.join(&base)
        } else {
            base
        };
        Self {
            repo_path: repo,
            worktree_base: resolved_base,
        }
    }

    /// Get the base path where worktrees are created.
    pub fn worktree_base(&self) -> &Path {
        &self.worktree_base
    }

    /// Get the repository path.
    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }

    /// Create a new worktree.
    ///
    /// * `name` - Name of the worktree (used as directory name)
    /// * `git_ref` - Git reference (branch, tag, commit) to check out
    ///
    /// Returns the path to the created worktree.
    pub async fn create_worktree(
        &self,
        name: &str,
        git_ref: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let worktree_path = self.worktree_base.join(name);

        // Create parent directory if needed
        if let Some(parent) = worktree_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        info!(
            repo_path = %self.repo_path.display(),
            worktree_path = %worktree_path.display(),
            git_ref = %git_ref,
            "creating git worktree"
        );

        let output = tokio::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("add")
            .arg(&worktree_path)
            .arg(git_ref)
            .env_remove("GIT_INDEX_FILE")
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(name = %name, stderr = %stderr, "git worktree add failed");
            return Err(std::io::Error::other(format!(
                "Failed to create worktree '{}': {}",
                name, stderr
            )));
        }

        info!(name = %name, path = %worktree_path.display(), "worktree created");
        Ok(worktree_path)
    }

    /// Remove a worktree.
    ///
    /// * `name` - Name of the worktree to remove
    pub async fn remove_worktree(&self, name: &str) -> Result<(), std::io::Error> {
        let worktree_path = self.worktree_base.join(name);

        if !worktree_path.exists() {
            warn!(name = %name, path = %worktree_path.display(), "worktree does not exist");
            return Ok(());
        }

        info!(name = %name, "removing git worktree");

        let output = tokio::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo_path)
            .arg("worktree")
            .arg("remove")
            .arg(&worktree_path)
            .env_remove("GIT_INDEX_FILE")
            .output()
            .await?;

        if !output.status.success() {
            // Try force removal if normal removal fails
            let output = tokio::process::Command::new("git")
                .arg("-C")
                .arg(&self.repo_path)
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(&worktree_path)
                .env_remove("GIT_INDEX_FILE")
                .output()
                .await?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!(name = %name, stderr = %stderr, "git worktree remove failed");
                return Err(std::io::Error::other(format!(
                    "Failed to remove worktree '{}': {}",
                    name, stderr
                )));
            }
        }

        // Clean up empty parent directories
        if let Some(parent) = worktree_path.parent() {
            let _ = tokio::fs::remove_dir(parent).await;
        }

        info!(name = %name, "worktree removed");
        Ok(())
    }

    /// Remove all worktrees managed by this manager.
    ///
    /// Returns the number of worktrees removed.
    pub async fn cleanup_all(&self) -> Result<usize, std::io::Error> {
        let worktrees = self.list_worktrees()?;
        let mut count = 0;

        for name in &worktrees {
            if let Err(e) = self.remove_worktree(name).await {
                error!(name = %name, error = %e, "failed to remove worktree during cleanup");
            } else {
                count += 1;
            }
        }

        info!(count = count, "cleaned up all worktrees");
        Ok(count)
    }

    /// List all worktrees managed by this manager.
    ///
    /// Returns a list of worktree names (directory names, not full paths).
    pub fn list_worktrees(&self) -> Result<Vec<String>, std::io::Error> {
        if !self.worktree_base.exists() {
            return Ok(Vec::new());
        }

        let mut worktrees = Vec::new();

        for entry in std::fs::read_dir(&self.worktree_base)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Verify this is actually a git worktree by checking for .git file or directory
                if path.join(".git").exists() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        worktrees.push(name.to_string());
                    }
                }
            }
        }

        Ok(worktrees)
    }

    /// Check if a worktree exists.
    pub fn worktree_exists(&self, name: &str) -> bool {
        self.worktree_base.join(name).join(".git").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::process::Command;
    use tempfile::TempDir;

    // ==================== ScopeRegistry Tests ====================

    #[test]
    fn test_reserve_and_release() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();
        let patterns: HashSet<String> = ["src/".to_string(), "tests/".to_string()].into();

        let id = registry
            .reserve("agent1", patterns.clone(), correlation_id)
            .expect("should reserve");

        assert!(registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
        assert_eq!(registry.len(), 1);

        let released = registry.release(id);
        assert!(released);
        assert!(!registry.has_reservation("agent1"));
        assert_eq!(registry.len(), 0);

        // Release again should return false
        assert!(!registry.release(id));
    }

    #[test]
    fn test_reserve_exclusive_conflict() {
        let mut registry = ScopeRegistry::new(true); // exclusive mode
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns1, correlation_id)
            .expect("first reserve should succeed");

        // Overlapping pattern should fail in exclusive mode
        let patterns2: HashSet<String> = ["src/main.rs".to_string()].into();
        let result = registry.reserve("agent2", patterns2, correlation_id);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("overlap"));
    }

    #[test]
    fn test_reserve_non_exclusive_overlap_allowed() {
        let mut registry = ScopeRegistry::new(false); // non-exclusive mode
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns1, correlation_id)
            .expect("first reserve should succeed");

        // Overlapping pattern should succeed in non-exclusive mode
        let patterns2: HashSet<String> = ["src/main.rs".to_string()].into();
        let result = registry.reserve("agent2", patterns2, correlation_id);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_release_by_correlation() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id1 = Uuid::new_v4();
        let correlation_id2 = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["tests/".to_string()].into();
        let patterns3: HashSet<String> = ["docs/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id1)
            .unwrap();
        registry
            .reserve("agent2", patterns2, correlation_id1)
            .unwrap();
        registry
            .reserve("agent3", patterns3, correlation_id2)
            .unwrap();

        assert_eq!(registry.len(), 3);

        let released = registry.release_by_correlation(correlation_id1);
        assert_eq!(released, 2);
        assert_eq!(registry.len(), 1);
        assert!(!registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
        assert!(registry.has_reservation("agent3"));
    }

    #[test]
    fn test_active_reservations() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["tests/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id)
            .unwrap();
        registry
            .reserve("agent2", patterns2, correlation_id)
            .unwrap();

        let active = registry.active_reservations();
        assert_eq!(active.len(), 2);

        let agent_names: Vec<&str> = active.iter().map(|r| r.agent_name.as_str()).collect();
        assert!(agent_names.contains(&"agent1"));
        assert!(agent_names.contains(&"agent2"));
    }

    #[test]
    fn test_has_reservation() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        assert!(!registry.has_reservation("agent1"));

        let patterns: HashSet<String> = ["src/".to_string()].into();
        registry
            .reserve("agent1", patterns, correlation_id)
            .unwrap();

        assert!(registry.has_reservation("agent1"));
        assert!(!registry.has_reservation("agent2"));
    }

    #[test]
    fn test_reservations_for_agent() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns1: HashSet<String> = ["src/".to_string()].into();
        let patterns2: HashSet<String> = ["lib/".to_string()].into();

        registry
            .reserve("agent1", patterns1, correlation_id)
            .unwrap();
        registry
            .reserve("agent1", patterns2, correlation_id)
            .unwrap();
        registry
            .reserve("agent2", ["tests/".to_string()].into(), correlation_id)
            .unwrap();

        let agent1_reservations = registry.reservations_for_agent("agent1");
        assert_eq!(agent1_reservations.len(), 2);

        let agent2_reservations = registry.reservations_for_agent("agent2");
        assert_eq!(agent2_reservations.len(), 1);

        let agent3_reservations = registry.reservations_for_agent("agent3");
        assert!(agent3_reservations.is_empty());
    }

    #[test]
    fn test_reservation_overlap_detection() {
        let res1 = ScopeReservation::new("agent1", ["src/".to_string()].into(), Uuid::new_v4());

        // Exact overlap
        assert!(res1.overlaps(&["src/".to_string()].into()));

        // Sub-path overlap
        assert!(res1.overlaps(&["src/main.rs".to_string()].into()));

        // No overlap
        assert!(!res1.overlaps(&["tests/".to_string()].into()));

        // Sibling overlap check
        let res2 =
            ScopeReservation::new("agent2", ["src/main.rs".to_string()].into(), Uuid::new_v4());
        assert!(res2.overlaps(&["src/".to_string()].into()));
    }

    #[test]
    fn test_exclusive_mode_rejects_exact_match() {
        let mut registry = ScopeRegistry::new(true);
        let correlation_id = Uuid::new_v4();

        let patterns: HashSet<String> = ["src/main.rs".to_string()].into();
        registry
            .reserve("agent1", patterns.clone(), correlation_id)
            .unwrap();

        // Exact same pattern should fail
        let result = registry.reserve("agent2", patterns, correlation_id);
        assert!(result.is_err());
    }

    // ==================== WorktreeManager Tests ====================

    fn setup_git_repo() -> (TempDir, PathBuf) {
        // Clear GIT_INDEX_FILE so git commands use their own index.
        // During pre-commit hooks, git sets this to a lock file which
        // causes git operations in test temp repos to fail.
        std::env::remove_var("GIT_INDEX_FILE");

        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize git repo
        let output = Command::new("git")
            .arg("init")
            .arg(&repo_path)
            .output()
            .expect("failed to run git init");
        assert!(output.status.success(), "git init failed");

        // Configure git user for commits
        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.email")
            .arg("test@test.com")
            .output()
            .expect("failed to config git email");

        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .arg("config")
            .arg("user.name")
            .arg("Test User")
            .output()
            .expect("failed to config git name");

        // Create initial commit
        std::fs::write(repo_path.join("README.md"), "# Test Repo").expect("failed to write file");

        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .arg("add")
            .arg(".")
            .output()
            .expect("failed to git add");

        Command::new("git")
            .arg("-C")
            .arg(&repo_path)
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .output()
            .expect("failed to git commit");

        (temp_dir, repo_path)
    }

    #[tokio::test]
    async fn test_create_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        let worktree_path = manager.create_worktree("feature-branch", "HEAD").await;
        assert!(
            worktree_path.is_ok(),
            "create_worktree failed: {:?}",
            worktree_path.err()
        );

        let path = worktree_path.unwrap();
        assert!(path.exists());
        assert!(path.join(".git").exists());
        assert!(path.join("README.md").exists());
    }

    #[tokio::test]
    async fn test_remove_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Create worktree
        manager.create_worktree("to-remove", "HEAD").await.unwrap();
        let path = manager.worktree_base().join("to-remove");
        assert!(path.exists());

        // Remove worktree
        let result = manager.remove_worktree("to-remove").await;
        assert!(result.is_ok(), "remove_worktree failed: {:?}", result.err());
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_remove_nonexistent_worktree() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Should succeed (no-op) for non-existent worktree
        let result = manager.remove_worktree("nonexistent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cleanup_all() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Create multiple worktrees
        manager.create_worktree("wt1", "HEAD").await.unwrap();
        manager.create_worktree("wt2", "HEAD").await.unwrap();
        manager.create_worktree("wt3", "HEAD").await.unwrap();

        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 3);

        // Cleanup all
        let cleaned = manager.cleanup_all().await.unwrap();
        assert_eq!(cleaned, 3);

        let worktrees = manager.list_worktrees().unwrap();
        assert!(worktrees.is_empty());
    }

    #[tokio::test]
    async fn test_list_worktrees() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        // Empty initially
        let worktrees = manager.list_worktrees().unwrap();
        assert!(worktrees.is_empty());

        // Create worktrees
        manager.create_worktree("wt-a", "HEAD").await.unwrap();
        manager.create_worktree("wt-b", "HEAD").await.unwrap();

        let worktrees = manager.list_worktrees().unwrap();
        assert_eq!(worktrees.len(), 2);
        assert!(worktrees.contains(&"wt-a".to_string()));
        assert!(worktrees.contains(&"wt-b".to_string()));
    }

    #[tokio::test]
    async fn test_worktree_exists() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        assert!(!manager.worktree_exists("test-wt"));

        manager.create_worktree("test-wt", "HEAD").await.unwrap();
        assert!(manager.worktree_exists("test-wt"));

        manager.remove_worktree("test-wt").await.unwrap();
        assert!(!manager.worktree_exists("test-wt"));
    }

    #[test]
    fn test_worktree_paths() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        assert_eq!(manager.repo_path(), repo_path);
        assert_eq!(manager.worktree_base(), repo_path.join(".worktrees"));
    }

    #[tokio::test]
    async fn test_create_duplicate_worktree_fails() {
        let (_temp_dir, repo_path) = setup_git_repo();
        let manager = WorktreeManager::new(&repo_path);

        manager.create_worktree("duplicate", "HEAD").await.unwrap();

        // Creating duplicate should fail
        let result = manager.create_worktree("duplicate", "HEAD").await;
        assert!(result.is_err());
    }
}
