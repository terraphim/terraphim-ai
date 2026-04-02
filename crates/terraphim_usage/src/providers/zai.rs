use crate::{ProviderUsage, Result, UsageError, UsageProvider};

pub struct ZaiProvider;

impl ZaiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ZaiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for ZaiProvider {
    fn id(&self) -> &str {
        "zai"
    }

    fn display_name(&self) -> &str {
        "Z.ai"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO: Implement Zhipu AI API call
            // 1. Read ZAI_API_KEY or GLM_API_KEY from env
            // 2. GET https://api.z.ai/api/monitor/usage/quota/limit
            // 3. Parse TOKENS_LIMIT (session 5h, weekly 7d) and TIME_LIMIT (web searches)
            Err(UsageError::FetchFailed {
                provider: "zai".to_string(),
                source: "Not yet implemented".into(),
            })
        })
    }
}
