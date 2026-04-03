use crate::{ProviderUsage, Result, UsageError, UsageProvider};
use std::path::PathBuf;

#[allow(dead_code)]
pub struct ClaudeProvider {
    #[allow(dead_code)]
    credentials_path: PathBuf,
}

impl ClaudeProvider {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            credentials_path: PathBuf::from(format!("{}/.claude/.credentials.json", home)),
        }
    }

    pub fn with_credentials_path(path: PathBuf) -> Self {
        Self {
            credentials_path: path,
        }
    }
}

impl Default for ClaudeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude"
    }

    fn display_name(&self) -> &str {
        "Claude Code"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO: Implement OAuth token reading and API call
            // 1. Read ~/.claude/.credentials.json for access_token
            // 2. Call GET https://api.anthropic.com/api/oauth/usage
            // 3. Parse five_hour, seven_day, extra_usage
            // 4. Optionally call ccusage for token-level tracking
            Err(UsageError::FetchFailed {
                provider: "claude".to_string(),
                source: "Not yet implemented".into(),
            })
        })
    }
}
