use crate::{ProviderUsage, Result, UsageError, UsageProvider};

pub struct MiniMaxProvider;

impl MiniMaxProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MiniMaxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for MiniMaxProvider {
    fn id(&self) -> &str {
        "minimax"
    }

    fn display_name(&self) -> &str {
        "MiniMax"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            // TODO: Implement MiniMax Coding Plan API call
            // 1. Read MINIMAX_API_KEY or MINIMAX_CN_API_KEY from env
            // 2. Auto-detect region (CN vs GLOBAL)
            // 3. GET https://api.minimax.io/v1/api/openplatform/coding_plan/remains
            // 4. Parse model_remains for session usage
            Err(UsageError::FetchFailed {
                provider: "minimax".to_string(),
                source: "Not yet implemented".into(),
            })
        })
    }
}
