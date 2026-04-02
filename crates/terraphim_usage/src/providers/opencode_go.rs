use crate::{ProviderUsage, Result, UsageError, UsageProvider};
use std::path::PathBuf;

pub struct OpenCodeGoProvider {
    db_path: PathBuf,
}

impl OpenCodeGoProvider {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            db_path: PathBuf::from(format!("{}/.local/share/opencode/opencode.db", home)),
        }
    }

    pub fn with_db_path(path: PathBuf) -> Self {
        Self { db_path: path }
    }
}

impl Default for OpenCodeGoProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for OpenCodeGoProvider {
    fn id(&self) -> &str {
        "opencode-go"
    }

    fn display_name(&self) -> &str {
        "OpenCode Go"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO: Query local SQLite database
            // SELECT cost FROM message WHERE providerID = 'opencode-go' AND role = 'assistant'
            // Aggregate into 5h ($12), weekly ($30), monthly ($60) windows
            if !self.db_path.exists() {
                return Err(UsageError::ProviderNotFound(
                    "opencode-go database not found".to_string(),
                ));
            }

            Err(UsageError::FetchFailed {
                provider: "opencode-go".to_string(),
                source: "Not yet implemented".into(),
            })
        })
    }
}
