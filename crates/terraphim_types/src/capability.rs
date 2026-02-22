//! Capability-based routing types for unified LLM and Agent routing.
//!
//! This module provides the core types for the unified routing system that handles
//! both LLM model selection and agent spawning based on capabilities.
//!
//! # Example
//!
//! ```
//! use terraphim_types::capability::{Capability, Provider, ProviderType, CostLevel, Latency};
//! use std::path::PathBuf;
//!
//! // Define an LLM provider
//! let llm_provider = Provider {
//!     id: "claude-opus".into(),
//!     name: "Claude Opus".into(),
//!     provider_type: ProviderType::Llm {
//!         model_id: "claude-3-opus-20240229".into(),
//!         api_endpoint: "https://api.anthropic.com/v1".into(),
//!     },
//!     capabilities: vec![Capability::DeepThinking, Capability::CodeGeneration],
//!     cost_level: CostLevel::Expensive,
//!     latency: Latency::Slow,
//!     keywords: vec!["think".into(), "reasoning".into(), "complex".into()],
//! };
//!
//! // Define an Agent provider
//! let agent_provider = Provider {
//!     id: "@coder".into(),
//!     name: "Coder Agent".into(),
//!     provider_type: ProviderType::Agent {
//!         agent_id: "@coder".into(),
//!         cli_command: "opencode".into(),
//!         working_dir: PathBuf::from("/workspace"),
//!     },
//!     capabilities: vec![Capability::CodeGeneration, Capability::CodeReview],
//!     cost_level: CostLevel::Moderate,
//!     latency: Latency::Medium,
//!     keywords: vec!["implement".into(), "code".into(), "create".into()],
//! };
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A capability that can be provided by either an LLM or an Agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // Reasoning capabilities
    /// Complex reasoning, multi-step analysis
    DeepThinking,
    /// Quick responses, simple queries
    FastThinking,

    // Code capabilities
    /// Write code
    CodeGeneration,
    /// Review code for issues
    CodeReview,
    /// System design and architecture
    Architecture,
    /// Write tests
    Testing,
    /// Refactor code
    Refactoring,

    // Communication capabilities
    /// Write documentation
    Documentation,
    /// Explain concepts
    Explanation,

    // Specialized capabilities
    /// Security audit and review
    SecurityAudit,
    /// Performance optimization
    Performance,
}

impl Capability {
    /// Get all available capabilities
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

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Capability::DeepThinking => "Complex reasoning and multi-step analysis",
            Capability::FastThinking => "Quick responses for simple queries",
            Capability::CodeGeneration => "Write code",
            Capability::CodeReview => "Review code for issues",
            Capability::Architecture => "System design and architecture",
            Capability::Testing => "Write tests",
            Capability::Refactoring => "Refactor and improve code",
            Capability::Documentation => "Write documentation",
            Capability::Explanation => "Explain concepts",
            Capability::SecurityAudit => "Security audit and review",
            Capability::Performance => "Performance optimization",
        }
    }
}

/// Cost level for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostLevel {
    /// Cheap, suitable for high-volume tasks
    Cheap = 1,
    /// Moderate cost, balanced
    Moderate = 2,
    /// Expensive, use sparingly
    Expensive = 3,
}

impl CostLevel {
    /// Get human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            CostLevel::Cheap => "cheap",
            CostLevel::Moderate => "moderate",
            CostLevel::Expensive => "expensive",
        }
    }
}

/// Latency level for a provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Latency {
    /// Fast response time
    Fast = 1,
    /// Medium response time
    Medium = 2,
    /// Slow response time
    Slow = 3,
}

impl Latency {
    /// Get human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            Latency::Fast => "fast",
            Latency::Medium => "medium",
            Latency::Slow => "slow",
        }
    }
}

/// Type of provider (LLM or Agent)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ProviderType {
    /// LLM model provider
    Llm {
        /// Model identifier (e.g., "claude-3-opus-20240229")
        model_id: String,
        /// API endpoint URL
        api_endpoint: String,
    },
    /// Agent provider (CLI-based)
    Agent {
        /// Agent identifier (e.g., "@coder")
        agent_id: String,
        /// CLI command to spawn the agent
        cli_command: String,
        /// Working directory for the agent
        working_dir: PathBuf,
    },
}

impl ProviderType {
    /// Check if this is an LLM provider
    pub fn is_llm(&self) -> bool {
        matches!(self, ProviderType::Llm { .. })
    }

    /// Check if this is an Agent provider
    pub fn is_agent(&self) -> bool {
        matches!(self, ProviderType::Agent { .. })
    }

    /// Get the provider type label
    pub fn type_label(&self) -> &'static str {
        match self {
            ProviderType::Llm { .. } => "llm",
            ProviderType::Agent { .. } => "agent",
        }
    }
}

/// A provider of capabilities (LLM or Agent)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provider {
    /// Unique identifier (e.g., "claude-opus" or "@coder")
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Type of provider (LLM or Agent)
    #[serde(flatten)]
    pub provider_type: ProviderType,
    /// Capabilities this provider offers
    pub capabilities: Vec<Capability>,
    /// Cost level
    pub cost_level: CostLevel,
    /// Latency level
    pub latency: Latency,
    /// Keywords that trigger routing to this provider
    pub keywords: Vec<String>,
}

impl Provider {
    /// Create a new provider
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        provider_type: ProviderType,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            provider_type,
            capabilities: Vec::new(),
            cost_level: CostLevel::Moderate,
            latency: Latency::Medium,
            keywords: Vec::new(),
        }
    }

    /// Add a capability
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Set cost level
    pub fn with_cost_level(mut self, cost_level: CostLevel) -> Self {
        self.cost_level = cost_level;
        self
    }

    /// Set latency level
    pub fn with_latency(mut self, latency: Latency) -> Self {
        self.latency = latency;
        self
    }

    /// Add a keyword
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Check if provider has a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if provider matches any of the given capabilities
    pub fn has_any_capability(&self, capabilities: &[Capability]) -> bool {
        capabilities.iter().any(|c| self.has_capability(c))
    }

    /// Check if provider matches all given capabilities
    pub fn has_all_capabilities(&self, capabilities: &[Capability]) -> bool {
        capabilities.iter().all(|c| self.has_capability(c))
    }

    /// Check if provider matches keywords
    pub fn matches_keywords(&self, text: &str) -> bool {
        let text_lower = text.to_lowercase();
        self.keywords
            .iter()
            .any(|k| text_lower.contains(&k.to_lowercase()))
    }
}

/// Result of a routing decision
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingDecision {
    /// Selected provider
    pub provider: Provider,
    /// Capabilities that matched
    pub matched_capabilities: Vec<Capability>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Reason for routing decision
    pub reason: RoutingReason,
}

/// Reason for routing decision
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingReason {
    /// Matched by keyword
    KeywordMatch { keyword: String },
    /// Matched by capability
    CapabilityMatch { capabilities: Vec<Capability> },
    /// Explicit @mention
    ExplicitMention { mention: String },
    /// Fallback to default
    Fallback,
}

/// Context for routing decisions
#[derive(Debug, Clone, Default)]
pub struct RoutingContext {
    /// Source agent (if routing from agent output)
    pub source_agent: Option<String>,
    /// Conversation ID
    pub conversation_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Preferred strategy override
    pub strategy_override: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_description() {
        assert_eq!(
            Capability::DeepThinking.description(),
            "Complex reasoning and multi-step analysis"
        );
        assert_eq!(Capability::CodeGeneration.description(), "Write code");
    }

    #[test]
    fn test_provider_builder() {
        let provider = Provider::new(
            "claude-opus",
            "Claude Opus",
            ProviderType::Llm {
                model_id: "claude-3-opus".into(),
                api_endpoint: "https://api.anthropic.com".into(),
            },
        )
        .with_capability(Capability::DeepThinking)
        .with_capability(Capability::CodeGeneration)
        .with_cost_level(CostLevel::Expensive)
        .with_latency(Latency::Slow)
        .with_keyword("think")
        .with_keyword("reasoning");

        assert_eq!(provider.id, "claude-opus");
        assert!(provider.has_capability(&Capability::DeepThinking));
        assert!(provider.has_capability(&Capability::CodeGeneration));
        assert!(!provider.has_capability(&Capability::FastThinking));
        assert_eq!(provider.cost_level, CostLevel::Expensive);
        assert_eq!(provider.latency, Latency::Slow);
        assert!(provider.matches_keywords("I need to think about this"));
        assert!(!provider.matches_keywords("Quick question"));
    }

    #[test]
    fn test_provider_type() {
        let llm = ProviderType::Llm {
            model_id: "gpt-4".into(),
            api_endpoint: "https://api.openai.com".into(),
        };
        assert!(llm.is_llm());
        assert!(!llm.is_agent());
        assert_eq!(llm.type_label(), "llm");

        let agent = ProviderType::Agent {
            agent_id: "@coder".into(),
            cli_command: "opencode".into(),
            working_dir: PathBuf::from("/workspace"),
        };
        assert!(!agent.is_llm());
        assert!(agent.is_agent());
        assert_eq!(agent.type_label(), "agent");
    }

    #[test]
    fn test_cost_level_ordering() {
        assert!(CostLevel::Cheap < CostLevel::Moderate);
        assert!(CostLevel::Moderate < CostLevel::Expensive);
    }

    #[test]
    fn test_latency_ordering() {
        assert!(Latency::Fast < Latency::Medium);
        assert!(Latency::Medium < Latency::Slow);
    }
}
