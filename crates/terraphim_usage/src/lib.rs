pub mod cli;
pub mod formatter;
pub mod providers;
pub mod store;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UsageError {
    #[error("Provider {0} not found")]
    ProviderNotFound(String),

    #[error("Failed to fetch usage from {provider}: {source}")]
    FetchFailed {
        provider: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Authentication failed for {provider}: {message}")]
    AuthFailed { provider: String, message: String },

    #[error("Rate limit exceeded for {provider}")]
    RateLimited { provider: String },

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, UsageError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderUsage {
    pub provider_id: String,
    pub display_name: String,
    pub plan: Option<String>,
    pub lines: Vec<MetricLine>,
    pub fetched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetricLine {
    Text {
        label: String,
        value: String,
        color: Option<String>,
        subtitle: Option<String>,
    },
    Progress {
        label: String,
        used: f64,
        limit: f64,
        format: ProgressFormat,
        resets_at: Option<String>,
        period_duration_ms: Option<u64>,
        color: Option<String>,
    },
    Badge {
        label: String,
        text: String,
        color: Option<String>,
        subtitle: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProgressFormat {
    Percent,
    Dollars,
    Count { suffix: String },
}

pub trait UsageProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>;
}

pub struct UsageRegistry {
    providers: std::collections::HashMap<String, Box<dyn UsageProvider>>,
}

impl UsageRegistry {
    pub fn new() -> Self {
        Self {
            providers: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, provider: Box<dyn UsageProvider>) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    pub fn get(&self, id: &str) -> Option<&dyn UsageProvider> {
        self.providers.get(id).map(|p| p.as_ref())
    }

    pub fn all(&self) -> Vec<&dyn UsageProvider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    pub fn ids(&self) -> Vec<&str> {
        self.providers.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for UsageRegistry {
    fn default() -> Self {
        Self::new()
    }
}
