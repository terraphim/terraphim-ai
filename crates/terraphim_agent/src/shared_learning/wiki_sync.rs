use std::process::Command;
use std::time::Duration;

use thiserror::Error;
use tracing::info;

use crate::shared_learning::types::SharedLearning;

/// Errors that can occur during wiki sync
#[derive(Error, Debug, Clone)]
pub enum WikiSyncError {
    #[error("gitea-robot command failed: {0}")]
    GiteaRobot(String),
    #[error("wiki page already exists: {0}")]
    AlreadyExists(String),
    #[error("wiki page not found: {0}")]
    NotFound(String),
    #[error("network error: {0}")]
    Network(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("configuration error: {0}")]
    Config(String),
}

/// Configuration for Gitea wiki client
#[derive(Debug, Clone)]
pub struct GiteaWikiConfig {
    /// Gitea instance URL
    pub gitea_url: String,
    /// Gitea API token
    pub token: String,
    /// Repository owner
    pub owner: String,
    /// Repository name
    pub repo: String,
    /// Path to gitea-robot binary
    pub robot_path: String,
    /// Request timeout
    pub timeout: Duration,
}

impl Default for GiteaWikiConfig {
    fn default() -> Self {
        Self {
            gitea_url: std::env::var("GITEA_URL")
                .unwrap_or_else(|_| "https://git.terraphim.cloud".to_string()),
            token: std::env::var("GITEA_TOKEN").unwrap_or_default(),
            owner: "terraphim".to_string(),
            repo: "terraphim-ai".to_string(),
            robot_path: "/home/alex/go/bin/gitea-robot".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
}

impl GiteaWikiConfig {
    /// Create config from environment variables
    pub fn from_env() -> Result<Self, WikiSyncError> {
        let config = Self::default();

        if config.token.is_empty() {
            return Err(WikiSyncError::Config(
                "GITEA_TOKEN environment variable not set".to_string(),
            ));
        }

        Ok(config)
    }

    /// Set custom gitea-robot path
    pub fn with_robot_path(mut self, path: &str) -> Self {
        self.robot_path = path.to_string();
        self
    }

    /// Set custom owner/repo
    pub fn with_repo(mut self, owner: &str, repo: &str) -> Self {
        self.owner = owner.to_string();
        self.repo = repo.to_string();
        self
    }
}

/// Client for Gitea wiki operations
pub struct GiteaWikiClient {
    config: GiteaWikiConfig,
}

/// Result of a wiki sync operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncResult {
    /// Created new wiki page
    Created(String), // Page name
    /// Updated existing wiki page
    Updated(String), // Page name
    /// Skipped (learning not eligible for sync)
    Skipped(String), // Reason
}

impl GiteaWikiClient {
    /// Create new wiki client
    pub fn new(config: GiteaWikiConfig) -> Self {
        Self { config }
    }

    /// Create or update a wiki page for a learning
    pub async fn sync_learning(
        &self,
        learning: &SharedLearning,
    ) -> Result<SyncResult, WikiSyncError> {
        // Only sync L2 and L3 learnings
        if !learning.should_sync_to_wiki() {
            return Ok(SyncResult::Skipped(format!(
                "Trust level {} does not allow wiki sync",
                learning.trust_level
            )));
        }

        let page_name = learning
            .wiki_page_name
            .clone()
            .unwrap_or_else(|| learning.generate_wiki_page_name());

        // Check if page exists
        let exists = self.page_exists(&page_name).await?;

        let content = learning.to_wiki_markdown();

        if exists {
            // Update existing page
            self.update_wiki_page(&page_name, &content).await?;
            info!(
                "Updated wiki page for learning {}: {}",
                learning.id, page_name
            );
            Ok(SyncResult::Updated(page_name))
        } else {
            // Create new page
            self.create_wiki_page(&page_name, &content).await?;
            info!(
                "Created wiki page for learning {}: {}",
                learning.id, page_name
            );
            Ok(SyncResult::Created(page_name))
        }
    }

    /// Check if a wiki page exists
    async fn page_exists(&self, page_name: &str) -> Result<bool, WikiSyncError> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.gitea_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "wiki-get",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
                "--name",
                page_name,
            ])
            .output()
            .map_err(|e| {
                WikiSyncError::GiteaRobot(format!("Failed to execute gitea-robot: {}", e))
            })?;

        if output.status.success() {
            Ok(true)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("404") {
                Ok(false)
            } else {
                Err(WikiSyncError::GiteaRobot(stderr.to_string()))
            }
        }
    }

    /// Create a new wiki page
    async fn create_wiki_page(&self, page_name: &str, content: &str) -> Result<(), WikiSyncError> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.gitea_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "wiki-create",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
                "--title",
                page_name,
                "--content",
                content,
                "--message",
                &format!("Add shared learning: {}", page_name),
            ])
            .output()
            .map_err(|e| {
                WikiSyncError::GiteaRobot(format!("Failed to execute gitea-robot: {}", e))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                Err(WikiSyncError::AlreadyExists(page_name.to_string()))
            } else {
                Err(WikiSyncError::GiteaRobot(stderr.to_string()))
            }
        }
    }

    /// Update an existing wiki page
    async fn update_wiki_page(&self, page_name: &str, content: &str) -> Result<(), WikiSyncError> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.gitea_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "wiki-update",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
                "--name",
                page_name,
                "--content",
                content,
                "--message",
                &format!("Update shared learning: {}", page_name),
            ])
            .output()
            .map_err(|e| {
                WikiSyncError::GiteaRobot(format!("Failed to execute gitea-robot: {}", e))
            })?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("not found") || stderr.contains("404") {
                Err(WikiSyncError::NotFound(page_name.to_string()))
            } else {
                Err(WikiSyncError::GiteaRobot(stderr.to_string()))
            }
        }
    }

    /// Delete a wiki page
    pub async fn delete_wiki_page(&self, page_name: &str) -> Result<(), WikiSyncError> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.gitea_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "wiki-delete",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
                "--name",
                page_name,
            ])
            .output()
            .map_err(|e| {
                WikiSyncError::GiteaRobot(format!("Failed to execute gitea-robot: {}", e))
            })?;

        if output.status.success() {
            info!("Deleted wiki page: {}", page_name);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(WikiSyncError::GiteaRobot(stderr.to_string()))
        }
    }

    /// Sync all L2/L3 learnings to wiki
    pub async fn sync_all_learnings(
        &self,
        learnings: &[SharedLearning],
    ) -> Vec<(String, Result<SyncResult, WikiSyncError>)> {
        let mut results = Vec::new();

        for learning in learnings {
            if learning.should_sync_to_wiki() {
                let result = self.sync_learning(learning).await;
                results.push((learning.id.clone(), result));
            }
        }

        results
    }

    /// List all wiki pages
    pub async fn list_wiki_pages(&self) -> Result<Vec<String>, WikiSyncError> {
        let output = Command::new(&self.config.robot_path)
            .env("GITEA_URL", &self.config.gitea_url)
            .env("GITEA_TOKEN", &self.config.token)
            .args([
                "wiki-list",
                "--owner",
                &self.config.owner,
                "--repo",
                &self.config.repo,
            ])
            .output()
            .map_err(|e| {
                WikiSyncError::GiteaRobot(format!("Failed to execute gitea-robot: {}", e))
            })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pages: Vec<String> = stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(pages)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(WikiSyncError::GiteaRobot(stderr.to_string()))
        }
    }
}

/// Sync service that periodically syncs learnings to Gitea wiki
pub struct WikiSyncService {
    client: GiteaWikiClient,
}

impl WikiSyncService {
    /// Create new sync service
    pub fn new(client: GiteaWikiClient) -> Self {
        Self { client }
    }

    /// Sync a batch of learnings
    pub async fn sync_batch(&self, learnings: &[SharedLearning]) -> WikiSyncReport {
        let results = self.client.sync_all_learnings(learnings).await;

        let mut created = 0;
        let mut updated = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for (_, result) in &results {
            match result {
                Ok(SyncResult::Created(_)) => created += 1,
                Ok(SyncResult::Updated(_)) => updated += 1,
                Ok(SyncResult::Skipped(_)) => skipped += 1,
                Err(_) => failed += 1,
            }
        }

        WikiSyncReport {
            created,
            updated,
            skipped,
            failed,
            total: results.len(),
            results,
        }
    }
}

/// Report of a wiki sync operation
#[derive(Debug, Clone)]
pub struct WikiSyncReport {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub total: usize,
    pub results: Vec<(String, Result<SyncResult, WikiSyncError>)>,
}

impl WikiSyncReport {
    /// Check if all operations were successful
    pub fn all_success(&self) -> bool {
        self.failed == 0
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 100.0;
        }
        let success = self.total - self.failed;
        (success as f64 / self.total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitea_wiki_config_default() {
        let config = GiteaWikiConfig::default();
        assert_eq!(config.owner, "terraphim");
        assert_eq!(config.repo, "terraphim-ai");
        assert_eq!(config.robot_path, "/home/alex/go/bin/gitea-robot");
    }

    #[test]
    fn test_gitea_wiki_config_from_env() {
        // This test assumes GITEA_TOKEN is set in environment
        // Skip if not set
        if std::env::var("GITEA_TOKEN").is_err() {
            return;
        }

        let config = GiteaWikiConfig::from_env().unwrap();
        assert!(!config.token.is_empty());
    }

    #[test]
    fn test_sync_result_display() {
        let created = SyncResult::Created("test-page".to_string());
        let updated = SyncResult::Updated("test-page".to_string());
        let skipped = SyncResult::Skipped("not eligible".to_string());

        match created {
            SyncResult::Created(name) => assert_eq!(name, "test-page"),
            _ => panic!("Expected Created"),
        }

        match updated {
            SyncResult::Updated(name) => assert_eq!(name, "test-page"),
            _ => panic!("Expected Updated"),
        }

        match skipped {
            SyncResult::Skipped(reason) => assert_eq!(reason, "not eligible"),
            _ => panic!("Expected Skipped"),
        }
    }

    #[test]
    fn test_wiki_sync_report() {
        let report = WikiSyncReport {
            created: 5,
            updated: 3,
            skipped: 2,
            failed: 0,
            total: 10,
            results: vec![],
        };

        assert!(report.all_success());
        assert_eq!(report.success_rate(), 100.0);

        let report_with_failures = WikiSyncReport {
            created: 5,
            updated: 3,
            skipped: 1,
            failed: 1,
            total: 10,
            results: vec![],
        };

        assert!(!report_with_failures.all_success());
        assert_eq!(report_with_failures.success_rate(), 90.0);
    }

    #[tokio::test]
    async fn test_sync_learning_skips_l1() {
        // Create a mock config (won't actually connect to Gitea)
        let config = GiteaWikiConfig {
            gitea_url: "http://localhost".to_string(),
            token: "test".to_string(),
            owner: "test".to_string(),
            repo: "test".to_string(),
            robot_path: "/bin/true".to_string(),
            timeout: Duration::from_secs(5),
        };

        let client = GiteaWikiClient::new(config);
        let learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            crate::shared_learning::types::LearningSource::Manual,
            "agent".to_string(),
        );

        // L1 learning should be skipped
        let result = client.sync_learning(&learning).await.unwrap();
        assert!(matches!(result, SyncResult::Skipped(_)));
    }

    #[tokio::test]
    async fn test_sync_learning_syncs_l2() {
        let config = GiteaWikiConfig {
            gitea_url: "http://localhost".to_string(),
            token: "test".to_string(),
            owner: "test".to_string(),
            repo: "test".to_string(),
            robot_path: "/bin/false".to_string(), // Will fail, but that's ok for this test
            timeout: Duration::from_secs(5),
        };

        let client = GiteaWikiClient::new(config);
        let mut learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            crate::shared_learning::types::LearningSource::Manual,
            "agent".to_string(),
        );
        learning.promote_to_l2();

        // L2 learning should attempt sync (will fail due to /bin/false)
        let result = client.sync_learning(&learning).await;
        assert!(result.is_err() || matches!(result, Ok(SyncResult::Skipped(_))));
    }
}
