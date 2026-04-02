//! Usage tracking system for Terraphim AI.
//!
//! This crate provides the foundation for tracking AI provider usage metrics,
//! including token counts, costs, and persistence.
//!
//! # Architecture
//!
//! - [`UsageProvider`] trait: Abstraction for fetching usage from different providers
//! - [`ProviderUsage`] model: Data structure for provider usage metrics

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for a usage provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderId(pub String);

impl fmt::Display for ProviderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ProviderId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ProviderId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Provider usage metrics capturing token counts and metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderUsage {
    /// Unique provider identifier (e.g., "openai", "anthropic")
    pub provider_id: ProviderId,
    /// Human-readable display name for the provider
    pub display_name: String,
    /// Number of lines/tokens used
    pub lines: u64,
    /// When this usage data was fetched
    pub fetched_at: DateTime<Utc>,
}

impl ProviderUsage {
    /// Creates a new ProviderUsage instance with current timestamp.
    pub fn new(
        provider_id: impl Into<ProviderId>,
        display_name: impl Into<String>,
        lines: u64,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            lines,
            fetched_at: Utc::now(),
        }
    }
}

/// Trait for providers that can fetch usage data.
#[async_trait]
pub trait UsageProvider: Send + Sync {
    /// Returns the unique identifier for this provider.
    fn id(&self) -> ProviderId;

    /// Fetches current usage data from the provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the provider is unavailable or returns invalid data.
    async fn fetch_usage(&self) -> Result<ProviderUsage, UsageError>;
}

/// Errors that can occur during usage operations.
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    /// Provider is not available or returned an error.
    #[error("provider error: {0}")]
    Provider(String),

    /// Failed to parse provider response.
    #[error("parse error: {0}")]
    Parse(String),

    /// Storage operation failed.
    #[error("storage error: {0}")]
    Storage(String),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_usage_new_sets_timestamp() {
        let usage = ProviderUsage::new("test-provider", "Test Provider", 100);

        assert_eq!(usage.provider_id.0, "test-provider");
        assert_eq!(usage.display_name, "Test Provider");
        assert_eq!(usage.lines, 100);
        // Verify timestamp is recent (within last second)
        let now = Utc::now();
        assert!(usage.fetched_at <= now);
        assert!(usage.fetched_at >= now - chrono::Duration::seconds(1));
    }

    #[test]
    fn provider_id_from_string() {
        let id: ProviderId = "my-provider".into();
        assert_eq!(id.0, "my-provider");
    }

    #[test]
    fn provider_id_display() {
        let id = ProviderId("test".to_string());
        assert_eq!(format!("{}", id), "test");
    }
}
