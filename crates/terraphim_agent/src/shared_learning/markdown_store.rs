//! Markdown-based learning store
//!
//! Provides durable storage for shared learnings as markdown files with YAML frontmatter.
//! Uses the filesystem directly, organised by agent ID for cross-agent sharing.

use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use terraphim_types::Document;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::shared_learning::types::{LearningSource, SharedLearning, TrustLevel};

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
                dirs::data_local_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("terraphim")
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
    agent_name: String,
    captured_at: String,
    trust_level: String,
    source: String,
    importance: f64,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    entities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    correction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
        let mut all_learnings = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.config.learnings_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                let mut learnings = self.list_from_dir(&path).await?;
                all_learnings.append(&mut learnings);
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
            agent_name: learning.source_agent.clone(), // TODO: add agent_name field
            captured_at: learning.created_at.to_rfc3339(),
            trust_level: learning.trust_level.as_str().to_string(),
            source: format!("{:?}", learning.source),
            importance: 0.0,      // TODO: add importance field
            entities: Vec::new(), // TODO: add entities field
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

        let source = match frontmatter.source.as_str() {
            "BashHook" => LearningSource::BashHook,
            "AutoExtract" => LearningSource::AutoExtract,
            "ToolHealth" => LearningSource::ToolHealth,
            "GiteaComment" => LearningSource::GiteaComment,
            "CjeVerdict" => LearningSource::CjeVerdict,
            "Manual" => LearningSource::Manual,
            _ => LearningSource::AutoExtract,
        };

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
            .parse()
            .unwrap_or_else(|_| Utc::now());
        learning.updated_at = Utc::now();
        learning.original_command = frontmatter.original_command;
        learning.error_context = frontmatter.error_context;
        learning.correction = frontmatter.correction;
        learning.wiki_page_name = frontmatter.wiki_page_name;

        Ok(learning)
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
}
