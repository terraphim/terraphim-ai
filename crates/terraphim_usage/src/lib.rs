//! LLM usage metering and pricing for Terraphim AI.
//!
//! Tracks token consumption across providers (OpenRouter, Ollama, …), applies
//! per-model pricing tables, and persists usage records. Optional `cli` and
//! `providers` feature flags unlock the reporting CLI and provider adapters.
#[cfg(feature = "cli")]
pub mod cli;
pub mod formatter;
pub mod pricing;
#[cfg(feature = "providers")]
pub mod providers;
#[cfg(feature = "persistence")]
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Describes all error conditions for the usage metering subsystem.
#[derive(Error, Debug)]
pub enum UsageError {
    /// The requested provider identifier was not registered.
    #[error("Provider {0} not found")]
    ProviderNotFound(String),

    /// A network or parsing failure occurred while fetching provider usage.
    #[error("Failed to fetch usage from {provider}: {source}")]
    FetchFailed {
        /// The provider that failed.
        provider: String,
        /// The underlying error source.
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Authentication credentials were rejected by the provider.
    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed {
        /// The provider that rejected the credentials.
        provider: String,
        /// Human-readable failure description.
        message: String,
    },

    /// The provider's API rate limit was exceeded.
    #[error("Rate limit exceeded for {provider}")]
    RateLimited {
        /// The rate-limited provider identifier.
        provider: String,
    },

    /// A persistence backend operation failed.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// A JSON serialisation or deserialisation error occurred.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Convenience alias for `Result<T, UsageError>`.
pub type Result<T> = std::result::Result<T, UsageError>;

/// Represents a snapshot of usage data returned by a single provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Stable machine-readable identifier for the provider.
    pub provider_id: String,
    /// Human-readable name shown in reports.
    pub display_name: String,
    /// Subscription plan name, if known.
    pub plan: Option<String>,
    /// Ordered list of metric lines for this provider.
    pub lines: Vec<MetricLine>,
    /// RFC 3339 timestamp of when usage was fetched.
    pub fetched_at: String,
}

/// Describes a single display line within a provider usage report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricLine {
    /// A plain key-value text line.
    Text {
        /// Display label for the metric.
        label: String,
        /// Formatted value string.
        value: String,
        /// Optional colour hint for the UI.
        color: Option<String>,
        /// Optional secondary text beneath the value.
        subtitle: Option<String>,
    },
    /// A progress bar showing consumption against a limit.
    Progress {
        /// Display label for the metric.
        label: String,
        /// Amount consumed so far.
        used: f64,
        /// Maximum allowed amount.
        limit: f64,
        /// How to format the progress values.
        format: ProgressFormat,
        /// RFC 3339 timestamp when the counter resets, if known.
        resets_at: Option<String>,
        /// Length of the current period in milliseconds, if known.
        period_duration_ms: Option<u64>,
        /// Optional colour hint for the UI.
        color: Option<String>,
    },
    /// A small badge displaying a short status text.
    Badge {
        /// Display label for the badge.
        label: String,
        /// Badge text content.
        text: String,
        /// Optional colour hint for the UI.
        color: Option<String>,
        /// Optional secondary text beneath the badge.
        subtitle: Option<String>,
    },
}

/// Describes how a progress value should be formatted for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressFormat {
    /// Render as a percentage (e.g. "42%").
    Percent,
    /// Render as a dollar amount (e.g. "$3.50").
    Dollars,
    /// Render as a count with a unit suffix (e.g. "120 prompts").
    Count {
        /// Unit label appended after the number.
        suffix: String,
    },
}

/// Defines the interface that every LLM provider adapter must implement.
pub trait UsageProvider: Send + Sync {
    /// Returns the stable machine-readable identifier for this provider.
    fn id(&self) -> &str;
    /// Returns the human-readable display name for this provider.
    fn display_name(&self) -> &str;
    /// Fetches current usage data from the provider, returning a `ProviderUsage` snapshot.
    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>;
}

/// Represents a collection of registered usage providers indexed by their identifier.
pub struct UsageRegistry {
    providers: std::collections::HashMap<String, Box<dyn UsageProvider>>,
}

impl UsageRegistry {
    /// Creates a new empty `UsageRegistry`.
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    /// Registers a provider, replacing any existing entry with the same identifier.
    pub fn register(&mut self, provider: Box<dyn UsageProvider>) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    /// Returns a reference to the provider with the given identifier, if registered.
    pub fn get(&self, id: &str) -> Option<&dyn UsageProvider> {
        self.providers.get(id).map(|p| p.as_ref())
    }

    /// Returns references to all registered providers.
    pub fn all(&self) -> Vec<&dyn UsageProvider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    /// Returns the identifiers of all registered providers.
    pub fn ids(&self) -> Vec<&str> {
        self.providers.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for UsageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
