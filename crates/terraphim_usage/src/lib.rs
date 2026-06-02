//! LLM usage metering and pricing for Terraphim AI.
//!
//! Tracks token consumption across providers (OpenRouter, Ollama, …), applies
//! per-model pricing tables, and persists usage records. Optional `cli` and
//! `providers` feature flags unlock the reporting CLI and provider adapters.
/// CLI interface for the usage reporting tool.
#[cfg(feature = "cli")]
pub mod cli;
/// Text, JSON, and CSV formatters for provider usage snapshots.
pub mod formatter;
/// Per-model pricing tables used to estimate request costs.
pub mod pricing;
/// Built-in provider adapters (requires the `providers` feature).
#[cfg(feature = "providers")]
pub mod providers;
/// Persistable storage records for usage metrics and budget snapshots.
#[cfg(feature = "persistence")]
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for usage metering operations.
#[derive(Error, Debug)]
pub enum UsageError {
    /// The requested provider was not registered.
    #[error("Provider {0} not found")]
    ProviderNotFound(String),

    /// A network or API error occurred while fetching usage data.
    #[error("Failed to fetch usage from {provider}: {source}")]
    FetchFailed {
        /// Provider identifier.
        provider: String,
        /// Underlying error source.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Authentication credentials were missing or invalid.
    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed {
        /// Provider identifier.
        provider: String,
        /// Human-readable failure description.
        message: String,
    },

    /// The provider API rate limit was exceeded.
    #[error("Rate limit exceeded for {provider}")]
    RateLimited {
        /// Provider identifier.
        provider: String,
    },

    /// A persistence layer error occurred.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// JSON (de)serialisation failed.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Convenience `Result` alias that uses [`UsageError`] as the error type.
pub type Result<T> = std::result::Result<T, UsageError>;

/// Aggregated usage snapshot returned by a [`UsageProvider`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Stable machine-readable provider identifier (e.g. `"claude"`).
    pub provider_id: String,
    /// Human-readable provider name for display.
    pub display_name: String,
    /// Subscription plan name, if known.
    pub plan: Option<String>,
    /// Individual metric lines to display.
    pub lines: Vec<MetricLine>,
    /// RFC 3339 timestamp of when this snapshot was fetched.
    pub fetched_at: String,
}

/// A single display metric emitted by a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricLine {
    /// A simple key-value text metric.
    Text {
        /// Metric label.
        label: String,
        /// Metric value string.
        value: String,
        /// Optional colour hint for rendering.
        color: Option<String>,
        /// Optional subtitle / secondary text.
        subtitle: Option<String>,
    },
    /// A numeric metric with a progress bar.
    Progress {
        /// Metric label.
        label: String,
        /// Current consumed quantity.
        used: f64,
        /// Total allowed quantity.
        limit: f64,
        /// How the quantity should be formatted.
        format: ProgressFormat,
        /// RFC 3339 timestamp when the quota resets.
        resets_at: Option<String>,
        /// Duration of the quota period in milliseconds.
        period_duration_ms: Option<u64>,
        /// Optional colour hint for rendering.
        color: Option<String>,
    },
    /// A badge / pill metric.
    Badge {
        /// Metric label.
        label: String,
        /// Badge text.
        text: String,
        /// Optional colour hint for rendering.
        color: Option<String>,
        /// Optional subtitle / secondary text.
        subtitle: Option<String>,
    },
}

/// Display format for a [`MetricLine::Progress`] value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressFormat {
    /// Render as a percentage.
    Percent,
    /// Render as a dollar amount.
    Dollars,
    /// Render as a count with a unit suffix.
    Count {
        /// Unit suffix appended after the numeric value.
        suffix: String,
    },
}

/// Trait implemented by each provider adapter to fetch live usage data.
pub trait UsageProvider: Send + Sync {
    /// Returns the stable machine-readable provider identifier.
    fn id(&self) -> &str;
    /// Returns the human-readable display name.
    fn display_name(&self) -> &str;
    /// Asynchronously fetches the current usage snapshot.
    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>;
}

/// Registry that holds all registered [`UsageProvider`] instances.
pub struct UsageRegistry {
    providers: std::collections::HashMap<String, Box<dyn UsageProvider>>,
}

impl UsageRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Registers a provider, replacing any existing provider with the same id.
    pub fn register(&mut self, provider: Box<dyn UsageProvider>) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    /// Looks up a provider by its id.
    pub fn get(&self, id: &str) -> Option<&dyn UsageProvider> {
        self.providers.get(id).map(|p| p.as_ref())
    }

    /// Returns references to all registered providers.
    pub fn all(&self) -> Vec<&dyn UsageProvider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    /// Returns all registered provider ids.
    pub fn ids(&self) -> Vec<&str> {
        self.providers.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for UsageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
