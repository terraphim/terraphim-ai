//! Markdown-based learning store
//!
//! Provides durable storage for shared learnings as markdown files with YAML frontmatter.
//! Uses the filesystem directly, organised by agent ID for cross-agent sharing.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{info, warn};

use crate::shared_learning::types::{LearningSource, QualityMetrics, SharedLearning, TrustLevel};

#[derive(Error, Debug)]
pub enum MarkdownStoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("learning not found: {0}")]
    NotFound(String),

    #[error("invalid markdown format: {0}")]
    InvalidFormat(String),
}

/// Configuration for the markdown learning store
#[derive(Debug, Clone)]
pub struct MarkdownStoreConfig {
    /// Root directory for learning storage
    pub learnings_dir: PathBuf,
    /// Subdirectory for shared (cross-agent) learnings
    pub shared_dir_name: String,
}

impl Default for MarkdownStoreConfig {
    fn default() -> Self {
        let learnings_dir = std::env::var("TERRAPHIM_LEARNINGS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                directories::ProjectDirs::from("com", "aks", "terraphim")
                    .map(|pd| pd.data_local_dir().to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("learnings")
            });

        Self {
            learnings_dir,
            shared_dir_name: "shared".to_string(),
        }
    }
}

/// A markdown-based learning store that saves learnings as files with YAML frontmatter
#[derive(Debug, Clone)]
pub struct MarkdownLearningStore {
    config: MarkdownStoreConfig,
}

/// YAML frontmatter for a shared learning markdown file
#[derive(Debug, Serialize, Deserialize)]
struct LearningFrontmatter {
    id: String,
    title: String,
    agent_id: String,
    #[serde(default)]
    captured_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    promoted_at: Option<String>,
    trust_level: String,
    source: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    applicable_agents: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    keywords: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    verify_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    quality: Option<QualityMetrics>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    original_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    error_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    correction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    wiki_page_name: Option<String>,
}

impl MarkdownLearningStore {
    /// Create a new markdown learning store with default configuration
    pub fn new() -> Self {
        Self {
            config: MarkdownStoreConfig::default(),
        }
    }

    /// Create a new markdown learning store with custom configuration
    pub fn with_config(config: MarkdownStoreConfig) -> Self {
        Self { config }
    }

    /// Get the root directory for all learnings
    pub fn learnings_dir(&self) -> &Path {
        &self.config.learnings_dir
    }

    /// Get the directory for a specific agent's learnings
    pub fn agent_dir(&self, agent_id: &str) -> PathBuf {
        self.config.learnings_dir.join(agent_id)
    }

    /// Get the shared directory for cross-agent learnings
    pub fn shared_dir(&self) -> PathBuf {
        self.config.learnings_dir.join(&self.config.shared_dir_name)
    }

    /// Save a learning to the store
    ///
    /// The learning is saved as `{learnings_dir}/{agent_id}/{learning_id}.md`
    pub async fn save(&self, learning: &SharedLearning) -> Result<(), MarkdownStoreError> {
        let agent_dir = self.agent_dir(&learning.source_agent);
        tokio::fs::create_dir_all(&agent_dir).await?;

        let file_path = agent_dir.join(format!("{}.md", learning.id));
        let content = Self::to_markdown(learning)?;

        tokio::fs::write(&file_path, content).await?;
        info!("Saved learning {} to {}", learning.id, file_path.display());

        Ok(())
    }

    /// Save a learning to the shared directory (for cross-agent access)
    pub async fn save_to_shared(
        &self,
        learning: &SharedLearning,
    ) -> Result<(), MarkdownStoreError> {
        let shared_dir = self.shared_dir();
        tokio::fs::create_dir_all(&shared_dir).await?;

        let file_path = shared_dir.join(format!("{}-{}.md", learning.source_agent, learning.id));
        let content = Self::to_markdown(learning)?;

        tokio::fs::write(&file_path, content).await?;
        info!(
            "Saved shared learning {} to {}",
            learning.id,
            file_path.display()
        );

        Ok(())
    }

    /// Load a learning by ID from a specific agent's directory
    pub async fn load(
        &self,
        agent_id: &str,
        learning_id: &str,
    ) -> Result<SharedLearning, MarkdownStoreError> {
        let file_path = self.agent_dir(agent_id).join(format!("{}.md", learning_id));
        self.load_from_path(&file_path).await
    }

    /// Load a learning from a file path
    pub async fn load_from_path(&self, path: &Path) -> Result<SharedLearning, MarkdownStoreError> {
        let content = tokio::fs::read_to_string(path).await?;
        Self::from_markdown(&content)
    }

    /// List all learnings for a specific agent
    pub async fn list_for_agent(
        &self,
        agent_id: &str,
    ) -> Result<Vec<SharedLearning>, MarkdownStoreError> {
        let agent_dir = self.agent_dir(agent_id);
        self.list_from_dir(&agent_dir).await
    }

    /// List all learnings from the shared directory
    pub async fn list_shared(&self) -> Result<Vec<SharedLearning>, MarkdownStoreError> {
        let shared_dir = self.shared_dir();
        self.list_from_dir(&shared_dir).await
    }

    /// List all learnings across all agents
    pub async fn list_all(&self) -> Result<Vec<SharedLearning>, MarkdownStoreError> {
        let all_with_origin = self.list_all_with_origin().await?;
        Ok(all_with_origin
            .into_iter()
            .map(|(_, learning)| learning)
            .collect())
    }

    pub(crate) async fn list_all_with_origin(
        &self,
    ) -> Result<Vec<(bool, SharedLearning)>, MarkdownStoreError> {
        let mut all_learnings = Vec::new();

        if !self.config.learnings_dir.exists() {
            return Ok(all_learnings);
        }

        let mut entries = tokio::fs::read_dir(&self.config.learnings_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let is_shared = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == self.config.shared_dir_name);

                let learnings = self.list_from_dir(&path).await?;
                all_learnings.extend(learnings.into_iter().map(|learning| (is_shared, learning)));
            }
        }

        Ok(all_learnings)
    }

    /// List learnings filtered by trust level
    pub async fn list_by_trust_level(
        &self,
        trust_level: TrustLevel,
    ) -> Result<Vec<SharedLearning>, MarkdownStoreError> {
        let all = self.list_all().await?;
        Ok(all
            .into_iter()
            .filter(|l| l.trust_level == trust_level)
            .collect())
    }

    /// Delete a learning
    pub async fn delete(
        &self,
        agent_id: &str,
        learning_id: &str,
    ) -> Result<(), MarkdownStoreError> {
        let file_path = self.agent_dir(agent_id).join(format!("{}.md", learning_id));
        tokio::fs::remove_file(&file_path).await?;
        info!("Deleted learning {} from agent {}", learning_id, agent_id);
        Ok(())
    }

    /// Convert a SharedLearning to markdown with YAML frontmatter
    fn to_markdown(learning: &SharedLearning) -> Result<String, MarkdownStoreError> {
        let frontmatter = LearningFrontmatter {
            id: learning.id.clone(),
            title: learning.title.clone(),
            agent_id: learning.source_agent.clone(),
            captured_at: Some(learning.created_at.to_rfc3339()),
            updated_at: Some(learning.updated_at.to_rfc3339()),
            promoted_at: learning.promoted_at.map(|dt| dt.to_rfc3339()),
            trust_level: learning.trust_level.as_str().to_string(),
            source: Self::learning_source_to_string(&learning.source),
            applicable_agents: learning.applicable_agents.clone(),
            keywords: learning.keywords.clone(),
            verify_pattern: learning.verify_pattern.clone(),
            quality: Some(learning.quality.clone()),
            original_command: learning.original_command.clone(),
            error_context: learning.error_context.clone(),
            correction: learning.correction.clone(),
            wiki_page_name: learning.wiki_page_name.clone(),
        };

        let yaml = serde_yaml::to_string(&frontmatter)?;
        let body = &learning.content;

        Ok(format!("---\n{}---\n\n{}", yaml, body))
    }

    /// Parse a SharedLearning from markdown with YAML frontmatter
    fn from_markdown(content: &str) -> Result<SharedLearning, MarkdownStoreError> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(MarkdownStoreError::InvalidFormat(
                "Missing YAML frontmatter".to_string(),
            ));
        }

        let yaml_content = parts[1].trim();
        let body = parts[2].trim();

        let frontmatter: LearningFrontmatter = serde_yaml::from_str(yaml_content).map_err(|e| {
            MarkdownStoreError::InvalidFormat(format!("Invalid YAML frontmatter: {}", e))
        })?;

        let trust_level = frontmatter.trust_level.parse::<TrustLevel>().map_err(|e| {
            MarkdownStoreError::InvalidFormat(format!("Invalid trust level: {}", e))
        })?;

        let source = Self::parse_learning_source(&frontmatter.source);

        let mut learning = SharedLearning::new(
            frontmatter.title,
            body.to_string(),
            source,
            frontmatter.agent_id,
        );

        learning.id = frontmatter.id;
        learning.trust_level = trust_level;
        learning.created_at = frontmatter
            .captured_at
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Utc::now);
        learning.updated_at = frontmatter
            .updated_at
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(Utc::now);
        learning.promoted_at = frontmatter.promoted_at.and_then(|s| s.parse().ok());
        learning.applicable_agents = frontmatter.applicable_agents;
        learning.keywords = frontmatter.keywords;
        learning.verify_pattern = frontmatter.verify_pattern;
        learning.quality = frontmatter.quality.unwrap_or_default();
        learning.original_command = frontmatter.original_command;
        learning.error_context = frontmatter.error_context;
        learning.correction = frontmatter.correction;
        learning.wiki_page_name = frontmatter.wiki_page_name;

        Ok(learning)
    }

    /// Convert a LearningSource to a snake_case string for frontmatter
    fn learning_source_to_string(source: &LearningSource) -> String {
        match source {
            LearningSource::BashHook => "bash_hook",
            LearningSource::AutoExtract => "auto_extract",
            LearningSource::ToolHealth => "tool_health",
            LearningSource::GiteaComment => "gitea_comment",
            LearningSource::CjeVerdict => "cje_verdict",
            LearningSource::Manual => "manual",
        }
        .to_string()
    }

    /// Parse a LearningSource from a string, supporting multiple formats
    fn parse_learning_source(s: &str) -> LearningSource {
        match s {
            // snake_case (current format)
            "bash_hook" => LearningSource::BashHook,
            "auto_extract" => LearningSource::AutoExtract,
            "tool_health" => LearningSource::ToolHealth,
            "gitea_comment" => LearningSource::GiteaComment,
            "cje_verdict" => LearningSource::CjeVerdict,
            "manual" => LearningSource::Manual,
            // PascalCase (legacy format from format!("{:?}"))
            "BashHook" => LearningSource::BashHook,
            "AutoExtract" => LearningSource::AutoExtract,
            "ToolHealth" => LearningSource::ToolHealth,
            "GiteaComment" => LearningSource::GiteaComment,
            "CjeVerdict" => LearningSource::CjeVerdict,
            "Manual" => LearningSource::Manual,
            _ => LearningSource::AutoExtract,
        }
    }

    /// Helper to list learnings from a directory
    async fn list_from_dir(&self, dir: &Path) -> Result<Vec<SharedLearning>, MarkdownStoreError> {
        let mut learnings = Vec::new();

        if !dir.exists() {
            return Ok(learnings);
        }

        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                match self.load_from_path(&path).await {
                    Ok(learning) => learnings.push(learning),
                    Err(e) => warn!("Failed to load learning from {}: {}", path.display(), e),
                }
            }
        }

        Ok(learnings)
    }
}

impl Default for MarkdownLearningStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let learning = SharedLearning::new(
            "Test Learning".to_string(),
            "This is the body of the learning.".to_string(),
            LearningSource::AutoExtract,
            "test-agent".to_string(),
        );

        store.save(&learning).await.unwrap();
        let loaded = store.load("test-agent", &learning.id).await.unwrap();

        assert_eq!(loaded.id, learning.id);
        assert_eq!(loaded.title, learning.title);
        assert_eq!(loaded.content, learning.content);
        assert_eq!(loaded.source_agent, learning.source_agent);
    }

    #[tokio::test]
    async fn test_list_for_agent() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let learning1 = SharedLearning::new(
            "Learning 1".to_string(),
            "Body 1".to_string(),
            LearningSource::AutoExtract,
            "agent-a".to_string(),
        );
        let learning2 = SharedLearning::new(
            "Learning 2".to_string(),
            "Body 2".to_string(),
            LearningSource::AutoExtract,
            "agent-a".to_string(),
        );

        store.save(&learning1).await.unwrap();
        store.save(&learning2).await.unwrap();

        let learnings = store.list_for_agent("agent-a").await.unwrap();
        assert_eq!(learnings.len(), 2);
    }

    #[tokio::test]
    async fn test_shared_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let learning = SharedLearning::new(
            "Shared Learning".to_string(),
            "This is shared.".to_string(),
            LearningSource::AutoExtract,
            "agent-b".to_string(),
        );

        store.save_to_shared(&learning).await.unwrap();
        let shared = store.list_shared().await.unwrap();
        assert_eq!(shared.len(), 1);
        assert_eq!(shared[0].title, "Shared Learning");
    }

    #[tokio::test]
    async fn test_save_and_load_roundtrip_preserves_full_state() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let mut learning = SharedLearning::new(
            "Full State Learning".to_string(),
            "This is the body.".to_string(),
            LearningSource::BashHook,
            "test-agent".to_string(),
        );
        learning.id = "custom-id-123".to_string();
        learning.trust_level = TrustLevel::L2;
        learning.created_at = "2024-01-15T10:30:00Z".parse().unwrap();
        learning.updated_at = "2024-06-20T14:45:00Z".parse().unwrap();
        learning.promoted_at = Some("2024-06-20T14:45:00Z".parse().unwrap());
        learning.applicable_agents = vec!["security-audit".to_string(), "code-review".to_string()];
        learning.keywords = vec!["git".to_string(), "force-push".to_string()];
        learning.verify_pattern = Some("git push --force-with-lease".to_string());
        learning.quality.applied_count = 5;
        learning.quality.effective_count = 4;
        learning.quality.agent_count = 3;
        learning.quality.agent_names = vec![
            "agent1".to_string(),
            "agent2".to_string(),
            "agent3".to_string(),
        ];
        learning.quality.last_applied_at = Some("2024-06-19T12:00:00Z".parse().unwrap());
        learning.quality.success_rate = Some(0.8);
        learning.original_command = Some("git push -f".to_string());
        learning.error_context = Some("rejected".to_string());
        learning.correction = Some("use --force-with-lease".to_string());
        learning.wiki_page_name = Some("learning-git-push".to_string());

        store.save(&learning).await.unwrap();
        let loaded = store.load("test-agent", &learning.id).await.unwrap();

        assert_eq!(loaded.id, learning.id);
        assert_eq!(loaded.title, learning.title);
        assert_eq!(loaded.content, learning.content);
        assert_eq!(loaded.source_agent, learning.source_agent);
        assert_eq!(loaded.trust_level, learning.trust_level);
        assert_eq!(loaded.source, learning.source);
        assert_eq!(loaded.created_at, learning.created_at);
        assert_eq!(loaded.updated_at, learning.updated_at);
        assert_eq!(loaded.promoted_at, learning.promoted_at);
        assert_eq!(loaded.applicable_agents, learning.applicable_agents);
        assert_eq!(loaded.keywords, learning.keywords);
        assert_eq!(loaded.verify_pattern, learning.verify_pattern);
        assert_eq!(loaded.quality.applied_count, learning.quality.applied_count);
        assert_eq!(
            loaded.quality.effective_count,
            learning.quality.effective_count
        );
        assert_eq!(loaded.quality.agent_count, learning.quality.agent_count);
        assert_eq!(loaded.quality.agent_names, learning.quality.agent_names);
        assert_eq!(
            loaded.quality.last_applied_at,
            learning.quality.last_applied_at
        );
        assert_eq!(loaded.quality.success_rate, learning.quality.success_rate);
        assert_eq!(loaded.original_command, learning.original_command);
        assert_eq!(loaded.error_context, learning.error_context);
        assert_eq!(loaded.correction, learning.correction);
        assert_eq!(loaded.wiki_page_name, learning.wiki_page_name);
    }

    #[tokio::test]
    async fn test_sparse_old_frontmatter_still_loads() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        // Create a markdown file with only the old sparse frontmatter
        let sparse_content = r#"---
id: old-learning-456
title: Old Sparse Learning
agent_id: legacy-agent
captured_at: "2024-03-10T08:00:00Z"
trust_level: L3
source: Manual
---

This is content from an old learning.
"#;

        let agent_dir = store.agent_dir("legacy-agent");
        tokio::fs::create_dir_all(&agent_dir).await.unwrap();
        let file_path = agent_dir.join("old-learning-456.md");
        tokio::fs::write(&file_path, sparse_content).await.unwrap();

        let loaded = store
            .load("legacy-agent", "old-learning-456")
            .await
            .unwrap();

        assert_eq!(loaded.id, "old-learning-456");
        assert_eq!(loaded.title, "Old Sparse Learning");
        assert_eq!(loaded.source_agent, "legacy-agent");
        assert_eq!(loaded.trust_level, TrustLevel::L3);
        assert_eq!(loaded.source, LearningSource::Manual);
        assert_eq!(loaded.content, "This is content from an old learning.");
        // Missing fields should have safe defaults
        assert!(loaded.applicable_agents.is_empty());
        assert!(loaded.keywords.is_empty());
        assert!(loaded.verify_pattern.is_none());
        assert_eq!(loaded.quality.applied_count, 0);
        assert!(loaded.original_command.is_none());
        assert!(loaded.error_context.is_none());
        assert!(loaded.correction.is_none());
        assert!(loaded.wiki_page_name.is_none());
        assert!(loaded.promoted_at.is_none());
    }

    #[tokio::test]
    async fn test_list_all_supports_dedup_inputs() {
        let temp_dir = TempDir::new().unwrap();
        let config = MarkdownStoreConfig {
            learnings_dir: temp_dir.path().to_path_buf(),
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let learning1 = SharedLearning::new(
            "Agent Learning".to_string(),
            "Body from agent.".to_string(),
            LearningSource::AutoExtract,
            "agent-a".to_string(),
        );

        let learning2 = SharedLearning::new(
            "Shared Learning".to_string(),
            "Body from shared.".to_string(),
            LearningSource::Manual,
            "agent-b".to_string(),
        );

        store.save(&learning1).await.unwrap();
        store.save_to_shared(&learning2).await.unwrap();

        let all = store.list_all().await.unwrap();
        assert_eq!(all.len(), 2);
        let titles: Vec<String> = all.into_iter().map(|l| l.title).collect();
        assert!(titles.contains(&"Agent Learning".to_string()));
        assert!(titles.contains(&"Shared Learning".to_string()));
    }

    #[tokio::test]
    async fn test_list_all_returns_empty_when_root_missing() {
        let temp_dir = TempDir::new().unwrap();
        let missing_root = temp_dir.path().join("does-not-exist");

        let config = MarkdownStoreConfig {
            learnings_dir: missing_root,
            shared_dir_name: "shared".to_string(),
        };
        let store = MarkdownLearningStore::with_config(config);

        let all = store.list_all().await.unwrap();
        assert!(all.is_empty());
    }
}
