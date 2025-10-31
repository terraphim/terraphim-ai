// Git-based recovery system for file operations
//
// Provides auto-commit and undo functionality for safe file editing

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

#[cfg(feature = "typescript")]
use tsify::Tsify;

/// Git recovery manager for auto-commit and undo
pub struct GitRecovery {
    repo_path: PathBuf,
    commit_history: Vec<CommitRecord>,
}

/// Record of a commit for undo tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitRecord {
    pub commit_hash: String,
    pub message: String,
    pub files: Vec<String>,
    pub timestamp: String,
    pub operation: String,
}

impl GitRecovery {
    /// Create new git recovery manager
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            commit_history: Vec::new(),
        }
    }

    /// Auto-commit a file after successful edit
    pub async fn auto_commit(
        &mut self,
        file_path: &str,
        operation: &str,
        strategy_used: &str,
    ) -> Result<String> {
        info!("Auto-committing file: {} ({})", file_path, operation);

        // For Phase 5, we'll use simple git commands via tokio::process
        // This avoids adding git2 dependency for now

        let message = format!(
            "{} using {} strategy\n\nFile: {}\nTimestamp: {}",
            operation,
            strategy_used,
            file_path,
            chrono::Utc::now().to_rfc3339()
        );

        // Execute git add
        let add_output = tokio::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["add", file_path])
            .output()
            .await?;

        if !add_output.status.success() {
            warn!(
                "Git add failed: {}",
                String::from_utf8_lossy(&add_output.stderr)
            );
            return Err(anyhow::anyhow!("Git add failed"));
        }

        // Execute git commit
        let commit_output = tokio::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["commit", "-m", &message])
            .output()
            .await?;

        if !commit_output.status.success() {
            let stderr = String::from_utf8_lossy(&commit_output.stderr);
            // Check if it's just "nothing to commit"
            if stderr.contains("nothing to commit") {
                debug!("Nothing to commit for {}", file_path);
                return Ok("no-changes".to_string());
            }
            warn!("Git commit failed: {}", stderr);
            return Err(anyhow::anyhow!("Git commit failed"));
        }

        // Get the commit hash
        let hash_output = tokio::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["rev-parse", "HEAD"])
            .output()
            .await?;

        let commit_hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();

        // Record commit
        let record = CommitRecord {
            commit_hash: commit_hash.clone(),
            message,
            files: vec![file_path.to_string()],
            timestamp: chrono::Utc::now().to_rfc3339(),
            operation: operation.to_string(),
        };

        self.commit_history.push(record);

        info!("Auto-committed: {}", commit_hash);
        Ok(commit_hash)
    }

    /// Undo last N commits
    pub async fn undo(&mut self, steps: usize) -> Result<Vec<String>> {
        if steps == 0 {
            return Ok(Vec::new());
        }

        if steps > self.commit_history.len() {
            return Err(anyhow::anyhow!(
                "Cannot undo {} steps, only {} commits in history",
                steps,
                self.commit_history.len()
            ));
        }

        let mut undone = Vec::new();

        for _ in 0..steps {
            // Git reset --soft HEAD~1
            let output = tokio::process::Command::new("git")
                .current_dir(&self.repo_path)
                .args(&["reset", "--soft", "HEAD~1"])
                .output()
                .await?;

            if !output.status.success() {
                return Err(anyhow::anyhow!(
                    "Git reset failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }

            if let Some(record) = self.commit_history.pop() {
                undone.push(record.commit_hash);
            }
        }

        info!("Undid {} commits", steps);
        Ok(undone)
    }

    /// Get commit history
    pub fn get_history(&self) -> &[CommitRecord] {
        &self.commit_history
    }

    /// Generate diff for a file
    pub async fn get_diff(&self, file_path: Option<&str>) -> Result<String> {
        let args = if let Some(path) = file_path {
            vec!["diff", "HEAD", path]
        } else {
            vec!["diff", "HEAD"]
        };

        let output = tokio::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(&args)
            .output()
            .await?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Check if repository is clean (no uncommitted changes)
    pub async fn is_clean(&self) -> Result<bool> {
        let output = tokio::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(&["status", "--porcelain"])
            .output()
            .await?;

        Ok(output.stdout.is_empty())
    }
}

/// Snapshot system for state preservation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Snapshot {
    pub id: String,
    pub timestamp: String,
    pub description: String,
    pub files: Vec<FileSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct FileSnapshot {
    pub path: String,
    pub content: String,
}

/// Snapshot manager for file state preservation
pub struct SnapshotManager {
    snapshot_dir: PathBuf,
    snapshots: Vec<Snapshot>,
}

impl SnapshotManager {
    pub fn new(snapshot_dir: PathBuf) -> Self {
        Self {
            snapshot_dir,
            snapshots: Vec::new(),
        }
    }

    /// Create snapshot of files
    pub async fn create_snapshot(
        &mut self,
        description: String,
        files: Vec<String>,
    ) -> Result<String> {
        let id = format!("snapshot_{}", chrono::Utc::now().timestamp());
        let timestamp = chrono::Utc::now().to_rfc3339();

        let mut file_snapshots = Vec::new();

        for file_path in files {
            match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => {
                    file_snapshots.push(FileSnapshot {
                        path: file_path,
                        content,
                    });
                }
                Err(e) => {
                    warn!("Failed to snapshot {}: {}", file_path, e);
                }
            }
        }

        let snapshot = Snapshot {
            id: id.clone(),
            timestamp,
            description,
            files: file_snapshots,
        };

        // Save to disk
        tokio::fs::create_dir_all(&self.snapshot_dir).await?;
        let snapshot_path = self.snapshot_dir.join(format!("{}.json", id));
        let json = serde_json::to_string_pretty(&snapshot)?;
        tokio::fs::write(snapshot_path, json).await?;

        self.snapshots.push(snapshot);

        info!("Created snapshot: {}", id);
        Ok(id)
    }

    /// Restore snapshot by ID
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot = self
            .snapshots
            .iter()
            .find(|s| s.id == snapshot_id)
            .ok_or_else(|| anyhow::anyhow!("Snapshot not found: {}", snapshot_id))?;

        for file_snapshot in &snapshot.files {
            tokio::fs::write(&file_snapshot.path, &file_snapshot.content).await?;
        }

        info!("Restored snapshot: {}", snapshot_id);
        Ok(())
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> &[Snapshot] {
        &self.snapshots
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_snapshot_creation() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_dir = temp_dir.path().join("snapshots");
        let mut manager = SnapshotManager::new(snapshot_dir);

        // Create test file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "original content")
            .await
            .unwrap();

        let snapshot_id = manager
            .create_snapshot(
                "Test snapshot".to_string(),
                vec![test_file.to_str().unwrap().to_string()],
            )
            .await
            .unwrap();

        assert!(snapshot_id.starts_with("snapshot_"));
        assert_eq!(manager.snapshots.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot_restore() {
        let temp_dir = TempDir::new().unwrap();
        let snapshot_dir = temp_dir.path().join("snapshots");
        let mut manager = SnapshotManager::new(snapshot_dir);

        // Create and snapshot original file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "original").await.unwrap();

        let snapshot_id = manager
            .create_snapshot(
                "Original".to_string(),
                vec![test_file.to_str().unwrap().to_string()],
            )
            .await
            .unwrap();

        // Modify file
        tokio::fs::write(&test_file, "modified").await.unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&test_file).await.unwrap(),
            "modified"
        );

        // Restore snapshot
        manager.restore_snapshot(&snapshot_id).await.unwrap();

        // Verify restored
        assert_eq!(
            tokio::fs::read_to_string(&test_file).await.unwrap(),
            "original"
        );
    }

    #[tokio::test]
    async fn test_git_recovery_is_clean() {
        // This test requires a git repository
        // For now, test basic construction
        let recovery = GitRecovery::new(PathBuf::from("/tmp"));
        assert_eq!(recovery.commit_history.len(), 0);
    }

    #[test]
    fn test_commit_record_creation() {
        let record = CommitRecord {
            commit_hash: "abc123".to_string(),
            message: "Test commit".to_string(),
            files: vec!["test.rs".to_string()],
            timestamp: "2025-10-29T12:00:00Z".to_string(),
            operation: "edit".to_string(),
        };

        assert_eq!(record.commit_hash, "abc123");
        assert_eq!(record.files.len(), 1);
    }
}
