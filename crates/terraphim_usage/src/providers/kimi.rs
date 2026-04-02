use crate::{ProviderUsage, Result, UsageError, UsageProvider};

pub struct KimiProvider;

impl KimiProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KimiProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for KimiProvider {
    fn id(&self) -> &str {
        "kimi"
    }

    fn display_name(&self) -> &str {
        "Kimi Code"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO: Implement Moonshot AI API call
            // 1. Read MOONSHOT_API_KEY or KIMI_API_KEY from env
            // 2. Call Moonshot API endpoint (TBD - needs live verification)
            // 3. Parse session and weekly usage
            Err(UsageError::FetchFailed {
                provider: "kimi".to_string(),
                source: "Not yet implemented".into(),
            })
        })
    }
}
