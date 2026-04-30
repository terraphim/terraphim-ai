//! LLM usage tracking across multiple providers.
//!
//! Provides a unified interface for fetching token consumption, spend, and quota
//! data from cloud AI providers (Claude, Kimi, MiniMax, etc.) and presenting it
//! as structured [`MetricLine`] values suitable for CLI display or persistence.

#[cfg(feature = "cli")]
/// CLI sub-commands for the usage reporting feature
pub mod cli;
/// Human-readable formatting helpers for usage metrics
pub mod formatter;
/// Per-model pricing tables and cost calculation
pub mod pricing;
#[cfg(feature = "providers")]
/// Concrete [`UsageProvider`] implementations for each supported service
pub mod providers;
#[cfg(feature = "persistence")]
/// SQLite-backed store for persisting historical usage snapshots and budget records
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur when fetching or storing LLM usage data
#[derive(Error, Debug)]
pub enum UsageError {
    /// The requested provider ID is not registered in the [`UsageRegistry`]
    #[error("Provider {0} not found")]
    ProviderNotFound(String),

    /// A network or API error occurred while contacting the provider
    #[error("Failed to fetch usage from {provider}: {source}")]
    FetchFailed {
        /// Provider whose fetch failed
        provider: String,
        /// Underlying error from the provider client
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The provider rejected the request due to missing or invalid credentials
    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed {
        /// Provider that rejected the credentials
        provider: String,
        /// Human-readable description from the provider
        message: String,
    },

    /// The provider is temporarily throttling requests
    #[error("Rate limit exceeded for {provider}")]
    RateLimited {
        /// Provider that enforced the rate limit
        provider: String,
    },

    /// An error occurred when reading from or writing to the persistence store
    #[error("Storage error: {0}")]
    StorageError(String),

    /// JSON (de)serialisation failed
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Convenience alias for [`Result`](std::result::Result) using [`UsageError`]
pub type Result<T> = std::result::Result<T, UsageError>;

/// Aggregated usage snapshot returned by a single provider fetch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Stable machine-readable identifier (matches the registered provider ID)
    pub provider_id: String,
    /// Human-readable name shown in reports and CLI output
    pub display_name: String,
    /// Subscription plan name reported by the provider, if available
    pub plan: Option<String>,
    /// Ordered list of metric rows to display for this provider
    pub lines: Vec<MetricLine>,
    /// ISO-8601 timestamp of when this snapshot was fetched
    pub fetched_at: String,
}

/// A single displayable metric row within a [`ProviderUsage`] snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricLine {
    /// Simple labelled text value
    Text {
        /// Row label
        label: String,
        /// Formatted value string
        value: String,
        /// Optional ANSI/CSS colour hint for the value
        color: Option<String>,
        /// Secondary descriptive text shown below the value
        subtitle: Option<String>,
    },
    /// Progress bar showing consumed vs allowed quota
    Progress {
        /// Row label
        label: String,
        /// Amount already consumed
        used: f64,
        /// Maximum allowed quota
        limit: f64,
        /// How to format the numeric values
        format: ProgressFormat,
        /// ISO-8601 timestamp when the quota resets
        resets_at: Option<String>,
        /// Duration of the current billing period in milliseconds
        period_duration_ms: Option<u64>,
        /// Optional colour hint (e.g. "green", "yellow", "red")
        color: Option<String>,
    },
    /// Coloured badge with a short label and text
    Badge {
        /// Row label
        label: String,
        /// Badge text (e.g. plan tier)
        text: String,
        /// Optional colour hint for the badge
        color: Option<String>,
        /// Secondary descriptive text shown below the badge
        subtitle: Option<String>,
    },
}

/// Determines how a [`MetricLine::Progress`] value is rendered
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressFormat {
    /// Render as a percentage (0–100%)
    Percent,
    /// Render as a dollar amount
    Dollars,
    /// Render as a raw count with a unit suffix
    Count {
        /// Unit suffix appended after the number (e.g. "tokens")
        suffix: String,
    },
}

/// Trait implemented by every supported LLM usage data source
pub trait UsageProvider: Send + Sync {
    /// Stable unique identifier for this provider (e.g. `"claude"`, `"kimi"`)
    fn id(&self) -> &str;
    /// Display name shown in CLI output and reports
    fn display_name(&self) -> &str;
    /// Asynchronously fetch the latest usage snapshot from the provider's API
    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>;
}

/// Registry that holds all registered [`UsageProvider`] instances
pub struct UsageRegistry {
    providers: std::collections::HashMap<String, Box<dyn UsageProvider>>,
}

impl UsageRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Add a provider to the registry, keyed by its [`UsageProvider::id`]
    pub fn register(&mut self, provider: Box<dyn UsageProvider>) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    /// Look up a provider by its ID
    pub fn get(&self, id: &str) -> Option<&dyn UsageProvider> {
        self.providers.get(id).map(|p| p.as_ref())
    }

    /// Return all registered providers in unspecified order
    pub fn all(&self) -> Vec<&dyn UsageProvider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    /// Return all registered provider IDs
    pub fn ids(&self) -> Vec<&str> {
        self.providers.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for UsageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
