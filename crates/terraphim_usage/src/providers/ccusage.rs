use crate::{MetricLine, ProgressFormat, ProviderUsage, Result, UsageError, UsageProvider};
use std::time::Duration;

/// Usage provider that reads Claude Code session data via the `terraphim_ccusage` client
pub struct CcusageProvider {
    client: std::sync::Mutex<terraphim_ccusage::CcusageClient>,
}

impl CcusageProvider {
    /// Create a new provider with a 5-minute in-memory cache
    pub fn new() -> Self {
        let client =
            terraphim_ccusage::CcusageClient::new(terraphim_ccusage::CcusageProvider::Claude)
                .with_cache_ttl(Duration::from_secs(300));
        Self {
            client: std::sync::Mutex::new(client),
        }
    }
}

impl Default for CcusageProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl UsageProvider for CcusageProvider {
    fn id(&self) -> &str {
        "ccusage"
    }

    fn display_name(&self) -> &str {
        "Claude Code (ccusage)"
    }

    fn fetch_usage(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ProviderUsage>> + Send + '_>>
    {
        Box::pin(async move {
            let since = chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::days(30))
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "2020-01-01".to_string());

            let report = {
                let mut client = self.client.lock().map_err(|e| UsageError::FetchFailed {
                    provider: "ccusage".to_string(),
                    source: format!("Lock poisoned: {}", e).into(),
                })?;
                client
                    .query(&since, None)
                    .map_err(|e| UsageError::FetchFailed {
                        provider: "ccusage".to_string(),
                        source: e.into(),
                    })?
            };

            let mut lines = Vec::new();

            let total_cost: f64 = report
                .daily
                .iter()
                .filter_map(|d| d.total_cost.or(d.cost_usd))
                .sum();
            let total_tokens: u64 = report.daily.iter().filter_map(|d| d.total_tokens).sum();

            let daily_cost: f64 = report
                .daily
                .last()
                .and_then(|d| d.total_cost.or(d.cost_usd))
                .unwrap_or(0.0);

            let daily_tokens: u64 = report
                .daily
                .last()
                .and_then(|d| d.total_tokens)
                .unwrap_or(0);

            let days_with_usage = report.daily.len() as f64;

            lines.push(MetricLine::Progress {
                label: "30-day spend".to_string(),
                used: total_cost,
                limit: 50.0,
                format: ProgressFormat::Dollars,
                resets_at: None,
                period_duration_ms: Some(30 * 24 * 3600 * 1000),
                color: None,
            });

            lines.push(MetricLine::Text {
                label: "Today".to_string(),
                value: format!("${:.2} ({} tokens)", daily_cost, daily_tokens),
                color: None,
                subtitle: None,
            });

            lines.push(MetricLine::Text {
                label: "30-day total".to_string(),
                value: format!(
                    "${:.2} ({} tokens, {} days)",
                    total_cost, total_tokens, days_with_usage as u64
                ),
                color: None,
                subtitle: None,
            });

            Ok(ProviderUsage {
                provider_id: "ccusage".to_string(),
                display_name: "Claude Code (ccusage)".to_string(),
                plan: None,
                lines,
                fetched_at: chrono::Utc::now().to_rfc3339(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ccusage_provider_id() {
        let provider = CcusageProvider::new();
        assert_eq!(provider.id(), "ccusage");
    }

    #[test]
    fn test_ccusage_provider_display_name() {
        let provider = CcusageProvider::new();
        assert_eq!(provider.display_name(), "Claude Code (ccusage)");
    }
}
