//! Shared learning types for knowledge graph integration
//!
//! These types support cross-agent learning capture, trust management,
//! quality tracking, and the shared `LearningStore` trait used by both
//! `terraphim_orchestrator` and `terraphim_agent`.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Trust level for a shared learning
///
/// Represents the validation state of a learning:
/// - L0: Extracted (just captured, not yet reviewed)
/// - L1: Unverified (auto-captured)
/// - L2: Peer-validated (tested across multiple agents)
/// - L3: Human-approved (reviewed by CTO)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum TrustLevel {
    /// Just extracted from agent output, not yet reviewed
    L0,
    /// Unverified learning, auto-captured from various sources
    #[default]
    L1,
    /// Peer-validated: applied 3+ times across 2+ agents with positive outcome
    L2,
    /// Human-approved: CTO review via `/evolve` or Gitea issue approval
    L3,
}

impl TrustLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustLevel::L0 => "L0",
            TrustLevel::L1 => "L1",
            TrustLevel::L2 => "L2",
            TrustLevel::L3 => "L3",
        }
    }

    pub fn weight(&self) -> u8 {
        match self {
            TrustLevel::L0 => 0,
            TrustLevel::L1 => 1,
            TrustLevel::L2 => 2,
            TrustLevel::L3 => 3,
        }
    }

    pub fn allows_wiki_sync(&self) -> bool {
        matches!(self, TrustLevel::L2 | TrustLevel::L3)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TrustLevel::L0 => "Extracted",
            TrustLevel::L1 => "Unverified",
            TrustLevel::L2 => "Peer-Validated",
            TrustLevel::L3 => "Human-Approved",
        }
    }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl std::str::FromStr for TrustLevel {
    type Err = TrustLevelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "L0" | "EXTRACTED" => Ok(TrustLevel::L0),
            "L1" | "UNVERIFIED" => Ok(TrustLevel::L1),
            "L2" | "PEER-VALIDATED" | "PEER_VALIDATED" => Ok(TrustLevel::L2),
            "L3" | "HUMAN-APPROVED" | "HUMAN_APPROVED" => Ok(TrustLevel::L3),
            _ => Err(TrustLevelError::InvalidTrustLevel(s.to_string())),
        }
    }
}

impl PartialOrd for TrustLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.weight().cmp(&other.weight()))
    }
}

#[derive(Error, Debug)]
pub enum TrustLevelError {
    #[error("invalid trust level: {0}")]
    InvalidTrustLevel(String),
}

/// Category of a learning for classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningCategory {
    Technical,
    Process,
    Domain,
    Failure,
    SuccessPattern,
}

impl std::fmt::Display for LearningCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LearningCategory::Technical => write!(f, "technical"),
            LearningCategory::Process => write!(f, "process"),
            LearningCategory::Domain => write!(f, "domain"),
            LearningCategory::Failure => write!(f, "failure"),
            LearningCategory::SuccessPattern => write!(f, "success_pattern"),
        }
    }
}

/// Synchronous trait for a learning store shared between orchestrator and agent.
///
/// Implementations may persist to DeviceStorage, markdown files, or in-memory
/// maps. The trait is intentionally synchronous so that `terraphim_types`
/// remains free of async runtime dependencies. Implementations that need
/// async I/O can use internal synchronisation (e.g. `tokio::runtime::Handle`).
pub trait LearningStore: Send + Sync {
    fn insert(&self, learning: SharedLearning) -> Result<String, StoreError>;
    fn get(&self, id: &str) -> Result<SharedLearning, StoreError>;
    fn query_relevant(
        &self,
        agent: &str,
        context: &str,
        min_trust: TrustLevel,
        limit: usize,
    ) -> Result<Vec<SharedLearning>, StoreError>;
    fn record_applied(&self, id: &str) -> Result<(), StoreError>;
    fn record_effective(&self, id: &str) -> Result<(), StoreError>;
    fn list_by_trust(&self, min_trust: TrustLevel) -> Result<Vec<SharedLearning>, StoreError>;
    fn archive_stale(&self, max_age_days: u32) -> Result<usize, StoreError>;
}

/// In-memory `LearningStore` for tests and development.
///
/// No persistence -- data lives only for the lifetime of the struct.
/// Thread-safe via `std::sync::Mutex`.
pub struct InMemoryLearningStore {
    learnings: std::sync::Mutex<HashMap<String, SharedLearning>>,
}

impl InMemoryLearningStore {
    pub fn new() -> Self {
        Self {
            learnings: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryLearningStore {
    fn default() -> Self {
        Self::new()
    }
}

impl LearningStore for InMemoryLearningStore {
    fn insert(&self, learning: SharedLearning) -> Result<String, StoreError> {
        let id = learning.id.clone();
        let mut map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        map.insert(id.clone(), learning);
        Ok(id)
    }

    fn get(&self, id: &str) -> Result<SharedLearning, StoreError> {
        let map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        map.get(id)
            .cloned()
            .ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    fn query_relevant(
        &self,
        agent: &str,
        context: &str,
        min_trust: TrustLevel,
        limit: usize,
    ) -> Result<Vec<SharedLearning>, StoreError> {
        let map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        let context_lower = context.to_lowercase();
        let mut results: Vec<SharedLearning> = map
            .values()
            .filter(|l| l.trust_level >= min_trust)
            .filter(|l| {
                if l.applicable_agents.is_empty() {
                    true
                } else {
                    l.applicable_agents
                        .iter()
                        .any(|a| a.eq_ignore_ascii_case(agent))
                }
            })
            .filter(|l| {
                let text = l.extract_searchable_text();
                text.contains(&context_lower) || context_lower.is_empty()
            })
            .cloned()
            .collect();
        results.sort_by_key(|l| std::cmp::Reverse(l.quality.effective_count));
        results.truncate(limit);
        Ok(results)
    }

    fn record_applied(&self, id: &str) -> Result<(), StoreError> {
        let mut map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        let learning = map
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning
            .quality
            .record_application(&learning.source_agent, false);
        learning.updated_at = Utc::now();
        Ok(())
    }

    fn record_effective(&self, id: &str) -> Result<(), StoreError> {
        let mut map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        let learning = map
            .get_mut(id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        learning
            .quality
            .record_application(&learning.source_agent, true);
        learning.updated_at = Utc::now();
        if learning.quality.meets_l2_criteria() && learning.trust_level == TrustLevel::L1 {
            learning.promote_to_l2();
        }
        Ok(())
    }

    fn list_by_trust(&self, min_trust: TrustLevel) -> Result<Vec<SharedLearning>, StoreError> {
        let map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        Ok(map
            .values()
            .filter(|l| l.trust_level >= min_trust)
            .cloned()
            .collect())
    }

    fn archive_stale(&self, max_age_days: u32) -> Result<usize, StoreError> {
        let mut map = self
            .learnings
            .lock()
            .map_err(|e| StoreError::Persistence(e.to_string()))?;
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let before = map.len();
        map.retain(|_, l| l.trust_level > TrustLevel::L0 || l.updated_at > cutoff);
        Ok(before - map.len())
    }
}

/// Quality metrics for a shared learning
///
/// Tracks the effectiveness and usage of a learning across agents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QualityMetrics {
    /// Number of times this learning has been applied
    pub applied_count: u32,
    /// Number of times application resulted in positive outcome
    pub effective_count: u32,
    /// Number of distinct agents that have applied this learning
    pub agent_count: u32,
    /// List of agent names that have used this learning
    pub agent_names: Vec<String>,
    /// Last time this learning was applied
    pub last_applied_at: Option<DateTime<Utc>>,
    /// Success rate (effective_count / applied_count)
    pub success_rate: Option<f64>,
}

impl QualityMetrics {
    /// Create new quality metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an application of this learning
    pub fn record_application(&mut self, agent_name: &str, effective: bool) {
        self.applied_count += 1;
        if effective {
            self.effective_count += 1;
        }

        if !self.agent_names.contains(&agent_name.to_string()) {
            self.agent_names.push(agent_name.to_string());
            self.agent_count = self.agent_names.len() as u32;
        }

        self.last_applied_at = Some(Utc::now());
        self.recalculate_success_rate();
    }

    /// Recalculate success rate
    fn recalculate_success_rate(&mut self) {
        if self.applied_count > 0 {
            self.success_rate = Some(self.effective_count as f64 / self.applied_count as f64);
        }
    }

    /// Check if this learning meets L2 promotion criteria
    pub fn meets_l2_criteria(&self) -> bool {
        self.applied_count >= 3 && self.agent_count >= 2
    }
}

/// Suggestion status for the approval workflow
///
/// Tracks whether a captured suggestion has been reviewed by a human.
/// Orthogonal to `TrustLevel`: a learning can be L2 (peer-validated)
/// but still Rejected if the human disagrees with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    #[default]
    Pending,
    Approved,
    Rejected,
}

impl std::fmt::Display for SuggestionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuggestionStatus::Pending => write!(f, "pending"),
            SuggestionStatus::Approved => write!(f, "approved"),
            SuggestionStatus::Rejected => write!(f, "rejected"),
        }
    }
}

impl std::str::FromStr for SuggestionStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(SuggestionStatus::Pending),
            "approved" => Ok(SuggestionStatus::Approved),
            "rejected" => Ok(SuggestionStatus::Rejected),
            _ => Err(format!("invalid suggestion status: {}", s)),
        }
    }
}

/// Source of a shared learning
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LearningSource {
    /// Captured from failed bash commands via `learn hook`
    BashHook,
    /// Auto-extracted from session transcripts
    AutoExtract,
    /// Tool health metrics aggregation
    ToolHealth,
    /// Extracted from Gitea comment corrections
    GiteaComment,
    /// Derived from CJE (Critical Judge Evaluation) verdicts
    CjeVerdict,
    /// Manually created or imported
    Manual,
}

impl std::fmt::Display for LearningSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LearningSource::BashHook => write!(f, "bash-hook"),
            LearningSource::AutoExtract => write!(f, "auto-extract"),
            LearningSource::ToolHealth => write!(f, "tool-health"),
            LearningSource::GiteaComment => write!(f, "gitea-comment"),
            LearningSource::CjeVerdict => write!(f, "cje-verdict"),
            LearningSource::Manual => write!(f, "manual"),
        }
    }
}

/// A shared learning that can be persisted and synchronized
///
/// This struct wraps markdown+frontmatter content and adds metadata
/// for trust management, quality tracking, and cross-agent sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedLearning {
    /// Unique ID (UUID)
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Markdown content with YAML frontmatter
    pub content: String,
    /// Trust level (L1/L2/L3)
    pub trust_level: TrustLevel,
    /// Quality metrics
    pub quality: QualityMetrics,
    /// Source of this learning
    pub source: LearningSource,
    /// Agent that originally captured this learning
    pub source_agent: String,
    /// List of agent types this learning applies to (e.g., ["security-audit", "code-review"])
    pub applicable_agents: Vec<String>,
    /// Keywords for search and BM25 matching
    pub keywords: Vec<String>,
    /// Verify pattern (regex or pattern to test if learning is valid)
    pub verify_pattern: Option<String>,
    /// When this learning was created
    pub created_at: DateTime<Utc>,
    /// When this learning was last updated
    pub updated_at: DateTime<Utc>,
    /// When this learning was promoted to current trust level
    pub promoted_at: Option<DateTime<Utc>>,
    /// Gitea wiki page name (if synced)
    pub wiki_page_name: Option<String>,
    /// Original command or context that triggered this learning
    pub original_command: Option<String>,
    /// Error output or context
    pub error_context: Option<String>,
    /// Suggested correction or solution
    pub correction: Option<String>,
    /// Suggestion approval status (Pending/Approved/Rejected)
    #[serde(default)]
    pub suggestion_status: SuggestionStatus,
    /// Reason for rejection (only set when status is Rejected)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    /// BM25 confidence score from the suggestion engine
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bm25_confidence: Option<f64>,
}

impl SharedLearning {
    /// Create a new shared learning
    pub fn new(
        title: String,
        content: String,
        source: LearningSource,
        source_agent: String,
    ) -> Self {
        let id = format!(
            "learning-{}-{}",
            Uuid::new_v4().simple(),
            timestamp_millis()
        );

        Self {
            id,
            title,
            content,
            trust_level: TrustLevel::L1,
            quality: QualityMetrics::new(),
            source,
            source_agent,
            applicable_agents: Vec::new(),
            keywords: Vec::new(),
            verify_pattern: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            promoted_at: None,
            wiki_page_name: None,
            original_command: None,
            error_context: None,
            correction: None,
            suggestion_status: SuggestionStatus::Pending,
            rejection_reason: None,
            bm25_confidence: None,
        }
    }

    /// Set applicable agents
    pub fn with_applicable_agents(mut self, agents: Vec<String>) -> Self {
        self.applicable_agents = agents;
        self
    }

    /// Set keywords
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Set verify pattern
    pub fn with_verify_pattern(mut self, pattern: String) -> Self {
        self.verify_pattern = Some(pattern);
        self
    }

    /// Set original command
    pub fn with_original_command(mut self, command: String) -> Self {
        self.original_command = Some(command);
        self
    }

    /// Set error context
    pub fn with_error_context(mut self, error: String) -> Self {
        self.error_context = Some(error);
        self
    }

    /// Set correction
    pub fn with_correction(mut self, correction: String) -> Self {
        self.correction = Some(correction);
        self
    }

    /// Promote to L2 (peer-validated)
    pub fn promote_to_l2(&mut self) {
        if self.trust_level == TrustLevel::L1 {
            self.trust_level = TrustLevel::L2;
            self.promoted_at = Some(Utc::now());
            self.updated_at = Utc::now();
        }
    }

    /// Promote to L3 (human-approved)
    pub fn promote_to_l3(&mut self) {
        self.trust_level = TrustLevel::L3;
        self.promoted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if this learning should be synced to Gitea wiki
    pub fn should_sync_to_wiki(&self) -> bool {
        self.trust_level.allows_wiki_sync()
    }

    /// Generate wiki page name from title
    pub fn generate_wiki_page_name(&self) -> String {
        let normalized: String = self
            .title
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric() && c != ' ', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("-");

        format!("learning-{}", normalized)
    }

    /// Convert to markdown format for wiki storage
    pub fn to_wiki_markdown(&self) -> String {
        let mut md = String::new();

        // Metadata table
        md.push_str("## Metadata\n\n");
        md.push_str("| Field | Value |\n");
        md.push_str("|-------|-------|\n");
        md.push_str(&format!("| ID | `{}` |\n", self.id));
        md.push_str(&format!("| Trust Level | {} |\n", self.trust_level));
        md.push_str(&format!("| Source | {} |\n", self.source));
        md.push_str(&format!("| Source Agent | {} |\n", self.source_agent));
        md.push_str(&format!("| Created | {} |\n", self.created_at.to_rfc3339()));

        if let Some(ref cmd) = self.original_command {
            md.push_str(&format!("| Original Command | `{}` |\n", cmd));
        }

        // Quality metrics table
        md.push_str("\n## Quality Metrics\n\n");
        md.push_str("| Metric | Value |\n");
        md.push_str("|--------|-------|\n");
        md.push_str(&format!(
            "| Applied Count | {} |\n",
            self.quality.applied_count
        ));
        md.push_str(&format!(
            "| Effective Count | {} |\n",
            self.quality.effective_count
        ));
        md.push_str(&format!("| Agent Count | {} |\n", self.quality.agent_count));

        if let Some(rate) = self.quality.success_rate {
            md.push_str(&format!("| Success Rate | {:.1}% |\n", rate * 100.0));
        }

        // Applicable agents
        if !self.applicable_agents.is_empty() {
            md.push_str(&format!(
                "\n## Applicable Agents\n\n{}\n",
                self.applicable_agents
                    .iter()
                    .map(|a| format!("- `{}`", a))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // Keywords
        if !self.keywords.is_empty() {
            md.push_str(&format!(
                "\n## Keywords\n\n{}\n",
                self.keywords
                    .iter()
                    .map(|k| format!("- `{}`", k))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        // Verify pattern
        if let Some(ref pattern) = self.verify_pattern {
            md.push_str(&format!("\n## Verify Pattern\n\n```\n{}\n```\n", pattern));
        }

        // Content
        md.push_str("\n## Content\n\n");
        md.push_str(&self.content);

        md
    }

    /// Extract searchable text for BM25 scoring
    pub fn extract_searchable_text(&self) -> String {
        let mut text = format!("{} ", self.title);
        text.push_str(&self.content);
        text.push_str(&self.keywords.join(" "));

        if let Some(ref cmd) = self.original_command {
            text.push(' ');
            text.push_str(cmd);
        }

        if let Some(ref error) = self.error_context {
            text.push(' ');
            text.push_str(error);
        }

        text.to_lowercase()
    }
}

/// Timestamp in milliseconds
fn timestamp_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Error type for store operations
#[derive(Error, Debug)]
pub enum StoreError {
    #[error("persistence error: {0}")]
    Persistence(String),
    #[error("learning not found: {0}")]
    NotFound(String),
    #[error("BM25 calculation error: {0}")]
    Bm25(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_level_weight() {
        assert_eq!(TrustLevel::L0.weight(), 0);
        assert_eq!(TrustLevel::L1.weight(), 1);
        assert_eq!(TrustLevel::L2.weight(), 2);
        assert_eq!(TrustLevel::L3.weight(), 3);
    }

    #[test]
    fn test_trust_level_allows_wiki_sync() {
        assert!(!TrustLevel::L0.allows_wiki_sync());
        assert!(!TrustLevel::L1.allows_wiki_sync());
        assert!(TrustLevel::L2.allows_wiki_sync());
        assert!(TrustLevel::L3.allows_wiki_sync());
    }

    #[test]
    fn test_trust_level_from_str() {
        assert_eq!("L0".parse::<TrustLevel>().unwrap(), TrustLevel::L0);
        assert_eq!("extracted".parse::<TrustLevel>().unwrap(), TrustLevel::L0);
        assert_eq!("L1".parse::<TrustLevel>().unwrap(), TrustLevel::L1);
        assert_eq!("l1".parse::<TrustLevel>().unwrap(), TrustLevel::L1);
        assert_eq!("L2".parse::<TrustLevel>().unwrap(), TrustLevel::L2);
        assert_eq!("L3".parse::<TrustLevel>().unwrap(), TrustLevel::L3);
        assert_eq!(
            "peer-validated".parse::<TrustLevel>().unwrap(),
            TrustLevel::L2
        );
        assert!("invalid".parse::<TrustLevel>().is_err());
    }

    #[test]
    fn test_trust_level_ordering() {
        assert!(TrustLevel::L3 > TrustLevel::L2);
        assert!(TrustLevel::L2 > TrustLevel::L1);
        assert!(TrustLevel::L1 > TrustLevel::L0);
    }

    #[test]
    fn test_quality_metrics_record_application() {
        let mut metrics = QualityMetrics::new();
        metrics.record_application("agent1", true);
        assert_eq!(metrics.applied_count, 1);
        assert_eq!(metrics.effective_count, 1);
        assert_eq!(metrics.agent_count, 1);

        metrics.record_application("agent2", false);
        assert_eq!(metrics.applied_count, 2);
        assert_eq!(metrics.effective_count, 1);
        assert_eq!(metrics.agent_count, 2);

        // Same agent again shouldn't increment agent_count
        metrics.record_application("agent1", true);
        assert_eq!(metrics.applied_count, 3);
        assert_eq!(metrics.agent_count, 2);
    }

    #[test]
    fn test_quality_metrics_meets_l2_criteria() {
        let mut metrics = QualityMetrics::new();
        assert!(!metrics.meets_l2_criteria());

        // Need 3+ applications across 2+ agents
        metrics.record_application("agent1", true);
        metrics.record_application("agent1", true);
        metrics.record_application("agent1", true);
        assert!(!metrics.meets_l2_criteria()); // Only 1 agent

        metrics.record_application("agent2", true);
        assert!(metrics.meets_l2_criteria()); // 4 apps, 2 agents
    }

    #[test]
    fn test_shared_learning_new() {
        let learning = SharedLearning::new(
            "Test Learning".to_string(),
            "Content here".to_string(),
            LearningSource::Manual,
            "test-agent".to_string(),
        );

        assert!(learning.id.starts_with("learning-"));
        assert_eq!(learning.title, "Test Learning");
        assert_eq!(learning.trust_level, TrustLevel::L1);
        assert_eq!(learning.source_agent, "test-agent");
    }

    #[test]
    fn test_shared_learning_promotion() {
        let mut learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );

        assert_eq!(learning.trust_level, TrustLevel::L1);

        learning.promote_to_l2();
        assert_eq!(learning.trust_level, TrustLevel::L2);
        assert!(learning.promoted_at.is_some());

        learning.promote_to_l3();
        assert_eq!(learning.trust_level, TrustLevel::L3);
    }

    #[test]
    fn test_shared_learning_should_sync_to_wiki() {
        let l1 = SharedLearning::new(
            "L1".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        assert!(!l1.should_sync_to_wiki());

        let mut l2 = SharedLearning::new(
            "L2".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        l2.promote_to_l2();
        assert!(l2.should_sync_to_wiki());
    }

    #[test]
    fn test_shared_learning_generate_wiki_page_name() {
        let learning = SharedLearning::new(
            "Git Push Force Error".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );

        let name = learning.generate_wiki_page_name();
        assert!(name.starts_with("learning-"));
        assert!(name.contains("git-push-force-error"));
    }

    #[test]
    fn test_shared_learning_extract_searchable_text() {
        let learning = SharedLearning::new(
            "Git Error".to_string(),
            "Use git push".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        )
        .with_keywords(vec!["git".to_string(), "push".to_string()])
        .with_original_command("git push -f".to_string())
        .with_error_context("rejected".to_string());

        let text = learning.extract_searchable_text();
        assert!(text.contains("git error"));
        assert!(text.contains("use git push"));
        assert!(text.contains("git"));
        assert!(text.contains("push"));
        assert!(text.contains("git push -f"));
        assert!(text.contains("rejected"));
    }

    #[test]
    fn test_suggestion_status_display() {
        assert_eq!(SuggestionStatus::Pending.to_string(), "pending");
        assert_eq!(SuggestionStatus::Approved.to_string(), "approved");
        assert_eq!(SuggestionStatus::Rejected.to_string(), "rejected");
    }

    #[test]
    fn test_suggestion_status_from_str_roundtrip() {
        assert_eq!(
            "pending".parse::<SuggestionStatus>().unwrap(),
            SuggestionStatus::Pending
        );
        assert_eq!(
            "approved".parse::<SuggestionStatus>().unwrap(),
            SuggestionStatus::Approved
        );
        assert_eq!(
            "rejected".parse::<SuggestionStatus>().unwrap(),
            SuggestionStatus::Rejected
        );
        assert_eq!(
            "PENDING".parse::<SuggestionStatus>().unwrap(),
            SuggestionStatus::Pending
        );
        assert!("invalid".parse::<SuggestionStatus>().is_err());
    }

    #[test]
    fn test_shared_learning_default_suggestion_status() {
        let learning = SharedLearning::new(
            "Test".to_string(),
            "Content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        assert_eq!(learning.suggestion_status, SuggestionStatus::Pending);
        assert!(learning.rejection_reason.is_none());
        assert!(learning.bm25_confidence.is_none());
    }

    #[test]
    fn test_suggestion_status_serde_default() {
        let json = r#"{"id":"x","title":"t","content":"c","trust_level":"L1","quality":{"applied_count":0,"effective_count":0,"agent_count":0,"agent_names":[],"success_rate":null},"source":"manual","source_agent":"a","applicable_agents":[],"keywords":[],"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}"#;
        let learning: SharedLearning = serde_json::from_str(json).unwrap();
        assert_eq!(learning.suggestion_status, SuggestionStatus::Pending);
    }

    #[test]
    fn test_learning_category_display() {
        assert_eq!(LearningCategory::Technical.to_string(), "technical");
        assert_eq!(LearningCategory::Failure.to_string(), "failure");
        assert_eq!(
            LearningCategory::SuccessPattern.to_string(),
            "success_pattern"
        );
    }

    #[test]
    fn test_l0_trust_level_display() {
        assert_eq!(TrustLevel::L0.as_str(), "L0");
        assert_eq!(TrustLevel::L0.display_name(), "Extracted");
        assert_eq!(TrustLevel::L0.to_string(), "Extracted");
    }

    #[test]
    fn test_in_memory_store_insert_and_get() {
        let store = InMemoryLearningStore::new();
        let learning = SharedLearning::new(
            "Test".to_string(),
            "cargo build failed".to_string(),
            LearningSource::BashHook,
            "test-agent".to_string(),
        );
        let id = store.insert(learning).unwrap();
        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.title, "Test");
    }

    #[test]
    fn test_in_memory_store_get_not_found() {
        let store = InMemoryLearningStore::new();
        assert!(store.get("nonexistent").is_err());
    }

    #[test]
    fn test_in_memory_store_query_relevant() {
        let store = InMemoryLearningStore::new();
        let learning = SharedLearning::new(
            "Rust compilation error".to_string(),
            "Use cargo clippy".to_string(),
            LearningSource::Manual,
            "test-agent".to_string(),
        )
        .with_keywords(vec!["rust".to_string(), "clippy".to_string()]);
        store.insert(learning).unwrap();

        let results = store
            .query_relevant("test-agent", "rust clippy", TrustLevel::L1, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("compilation"));
    }

    #[test]
    fn test_in_memory_store_query_respects_trust() {
        let store = InMemoryLearningStore::new();
        let mut learning = SharedLearning::new(
            "Test".to_string(),
            "content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        learning.trust_level = TrustLevel::L0;
        store.insert(learning).unwrap();

        let results = store
            .query_relevant("agent", "test", TrustLevel::L1, 10)
            .unwrap();
        assert!(results.is_empty());

        let results = store
            .query_relevant("agent", "test", TrustLevel::L0, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_in_memory_store_record_applied_and_effective() {
        let store = InMemoryLearningStore::new();
        let learning = SharedLearning::new(
            "Test".to_string(),
            "content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let id = store.insert(learning).unwrap();

        store.record_applied(&id).unwrap();
        let l = store.get(&id).unwrap();
        assert_eq!(l.quality.applied_count, 1);
        assert_eq!(l.quality.effective_count, 0);

        store.record_effective(&id).unwrap();
        let l = store.get(&id).unwrap();
        assert_eq!(l.quality.applied_count, 2);
        assert_eq!(l.quality.effective_count, 1);
    }

    #[test]
    fn test_in_memory_store_auto_promote_on_effective() {
        let store = InMemoryLearningStore::new();
        let learning = SharedLearning::new(
            "Test".to_string(),
            "content".to_string(),
            LearningSource::Manual,
            "agent".to_string(),
        );
        let id = store.insert(learning).unwrap();

        store.record_effective(&id).unwrap();
        let l = store.get(&id).unwrap();
        assert_eq!(l.quality.effective_count, 1);
        assert_eq!(l.trust_level, TrustLevel::L1);
    }

    #[test]
    fn test_in_memory_store_list_by_trust() {
        let store = InMemoryLearningStore::new();
        let mut l0 = SharedLearning::new(
            "L0".to_string(),
            "c".to_string(),
            LearningSource::Manual,
            "a".to_string(),
        );
        l0.trust_level = TrustLevel::L0;
        let mut l2 = SharedLearning::new(
            "L2".to_string(),
            "c".to_string(),
            LearningSource::Manual,
            "a".to_string(),
        );
        l2.trust_level = TrustLevel::L2;
        store.insert(l0).unwrap();
        store.insert(l2).unwrap();

        let l1_plus = store.list_by_trust(TrustLevel::L1).unwrap();
        assert_eq!(l1_plus.len(), 1);
        assert_eq!(l1_plus[0].title, "L2");
    }

    #[test]
    fn test_in_memory_store_archive_stale() {
        let store = InMemoryLearningStore::new();
        let mut l0 = SharedLearning::new(
            "stale".to_string(),
            "c".to_string(),
            LearningSource::Manual,
            "a".to_string(),
        );
        l0.trust_level = TrustLevel::L0;
        l0.updated_at = Utc::now() - chrono::Duration::days(60);
        let mut l1 = SharedLearning::new(
            "fresh".to_string(),
            "c".to_string(),
            LearningSource::Manual,
            "a".to_string(),
        );
        l1.trust_level = TrustLevel::L1;
        l1.updated_at = Utc::now() - chrono::Duration::days(60);
        store.insert(l0).unwrap();
        store.insert(l1).unwrap();

        let archived = store.archive_stale(30).unwrap();
        assert_eq!(archived, 1);

        let all = store.list_by_trust(TrustLevel::L0).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].trust_level, TrustLevel::L1);
    }
}
