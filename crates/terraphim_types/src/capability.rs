//! Capability-based routing types for unified LLM and Agent providers.
//!
//! This module provides types for capability-based routing that works with both
//! LLM models and spawned agents, using the same routing logic.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A capability that a provider can fulfill
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Deep thinking and reasoning
    DeepThinking,
    /// Fast, responsive thinking
    FastThinking,
    /// Code generation
    CodeGeneration,
    /// Code review and analysis
    CodeReview,
    /// System architecture design
    Architecture,
    /// Testing and test generation
    Testing,
    /// Code refactoring
    Refactoring,
    /// Documentation generation
    Documentation,
    /// Explanation and teaching
    Explanation,
    /// Security auditing
    SecurityAudit,
    /// Performance optimization
    Performance,
}

impl Capability {
    /// Get all capabilities as a vector
    pub fn all() -> Vec<Capability> {
        vec![
            Capability::DeepThinking,
            Capability::FastThinking,
            Capability::CodeGeneration,
            Capability::CodeReview,
            Capability::Architecture,
            Capability::Testing,
            Capability::Refactoring,
            Capability::Documentation,
            Capability::Explanation,
            Capability::SecurityAudit,
            Capability::Performance,
        ]
    }
}

/// Type of provider (LLM or Agent)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// LLM provider with model configuration
    Llm {
        /// Model identifier (e.g., "gpt-4", "claude-3-opus")
        model_id: String,
        /// API endpoint URL
        api_endpoint: String,
    },
    /// Agent provider with CLI configuration
    Agent {
        /// Agent identifier (e.g., "@codex")
        agent_id: String,
        /// CLI command to spawn the agent
        cli_command: String,
        /// Working directory for the agent
        working_dir: PathBuf,
    },
}

/// Cost level for routing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostLevel {
    /// Cheap/affordable
    Cheap,
    /// Moderate cost
    Moderate,
    /// Expensive
    Expensive,
}

/// Latency expectation for routing decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Latency {
    /// Fast response
    Fast,
    /// Medium response time
    Medium,
    /// Slow response
    Slow,
}

/// A provider that can fulfill capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Unique identifier for this provider
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of provider (LLM or Agent)
    #[serde(rename = "type")]
    pub provider_type: ProviderType,
    /// Capabilities this provider can fulfill
    pub capabilities: Vec<Capability>,
    /// Cost level
    pub cost_level: CostLevel,
    /// Expected latency
    pub latency: Latency,
    /// Keywords that trigger this provider
    #[serde(default)]
    pub keywords: Vec<String>,
}

impl Provider {
    /// Create a new provider
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        provider_type: ProviderType,
        capabilities: Vec<Capability>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider_type,
            capabilities,
            cost_level: CostLevel::Moderate,
            latency: Latency::Medium,
            keywords: Vec::new(),
        }
    }

    /// Set cost level
    pub fn with_cost(mut self, cost: CostLevel) -> Self {
        self.cost_level = cost;
        self
    }

    /// Set latency
    pub fn with_latency(mut self, latency: Latency) -> Self {
        self.latency = latency;
        self
    }

    /// Set keywords
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords = keywords;
        self
    }

    /// Check if this provider has a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if this provider has all the given capabilities
    pub fn has_all_capabilities(&self, capabilities: &[Capability]) -> bool {
        capabilities.iter().all(|c| self.has_capability(c))
    }

    /// Check if any of the provider's keywords match the given text
    pub fn matches_keywords(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        self.keywords
            .iter()
            .any(|k| text_lower.contains(&k.to_lowercase()))
    }

    /// Get capabilities as strings
    pub fn capability_names(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .map(|c| format!("{:?}", c).to_lowercase())
            .collect()
    }
}

/// Process ID for spawned agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

impl ProcessId {
    /// Generate a new unique process ID
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        ProcessId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let provider = Provider::new(
            "test-llm",
            "Test LLM",
            ProviderType::Llm {
                model_id: "gpt-4".to_string(),
                api_endpoint: "https://api.openai.com".to_string(),
            },
            vec![Capability::CodeGeneration, Capability::CodeReview],
        )
        .with_cost(CostLevel::Expensive)
        .with_latency(Latency::Medium);

        assert_eq!(provider.id, "test-llm");
        assert!(provider.has_capability(&Capability::CodeGeneration));
        assert!(!provider.has_capability(&Capability::DeepThinking));
        assert_eq!(provider.cost_level, CostLevel::Expensive);
    }

    #[test]
    fn test_agent_provider() {
        let provider = Provider::new(
            "@codex",
            "Codex Agent",
            ProviderType::Agent {
                agent_id: "@codex".to_string(),
                cli_command: "opencode".to_string(),
                working_dir: PathBuf::from("/workspace"),
            },
            vec![Capability::CodeGeneration],
        );

        assert!(matches!(provider.provider_type, ProviderType::Agent { .. }));
    }

    #[test]
    fn test_process_id_generation() {
        let id1 = ProcessId::new();
        let id2 = ProcessId::new();
        assert_ne!(id1, id2);
        assert!(id2.0 > id1.0);
    }
}
