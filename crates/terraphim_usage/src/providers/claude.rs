use crate::{MetricLine, ProgressFormat, ProviderUsage, Result, UsageError, UsageProvider};
use std::path::PathBuf;
use std::time::Duration;

/// Usage provider for Anthropic Claude, backed by the local ccusage session database
pub struct ClaudeProvider {
    #[allow(dead_code)]
    credentials_path: PathBuf,
    ccusage: std::sync::Mutex<terraphim_ccusage::CcusageClient>,
}

impl ClaudeProvider {
    /// Create a provider using the default credentials path (`~/.claude/.credentials.json`)
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_default();
        Self {
            credentials_path: PathBuf::from(format!("{}/.claude/.credentials.json", home)),
            ccusage: std::sync::Mutex::new(
                terraphim_ccusage::CcusageClient::new(terraphim_ccusage::CcusageProvider::Claude)
                    .with_cache_ttl(Duration::from_secs(300)),
            ),
        }
    }

    /// Create a provider using an explicit credentials file path
    pub fn with_credentials_path(path: PathBuf) -> Self {
        let ccusage =
            terraphim_ccusage::CcusageClient::new(terraphim_ccusage::CcusageProvider::Claude)
                .with_cache_ttl(Duration::from_secs(300));
        Self {
            credentials_path: path,
            ccusage: std::sync::Mutex::new(ccusage),
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
            let since = chrono::Utc::now()
                .checked_sub_signed(chrono::Duration::days(7))
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "2020-01-01".to_string());

            let report = {
                let mut client = self.ccusage.lock().map_err(|e| UsageError::FetchFailed {
                    provider: "claude".to_string(),
                    source: format!("Lock poisoned: {}", e).into(),
                })?;
                client
                    .query(&since, None)
                    .map_err(|e| UsageError::FetchFailed {
                        provider: "claude".to_string(),
                        source: e.into(),
                    })?
            };

            let mut lines = Vec::new();

            let total_cost: f64 = report
                .daily
                .iter()
                .filter_map(|d| d.total_cost.or(d.cost_usd))
                .sum();
            let total_input: u64 = report.daily.iter().filter_map(|d| d.input_tokens).sum();
            let total_output: u64 = report.daily.iter().filter_map(|d| d.output_tokens).sum();

            let today_cost = report
                .daily
                .last()
                .and_then(|d| d.total_cost.or(d.cost_usd))
                .unwrap_or(0.0);

            let today_tokens: u64 = report
                .daily
                .last()
                .and_then(|d| d.total_tokens)
                .unwrap_or(0);

            lines.push(MetricLine::Progress {
                label: "7-day spend".to_string(),
                used: total_cost,
                limit: 50.0,
                format: ProgressFormat::Dollars,
                resets_at: None,
                period_duration_ms: Some(7 * 24 * 3600 * 1000),
                color: None,
            });

            lines.push(MetricLine::Text {
                label: "Today".to_string(),
                value: format!("${:.2} ({} tokens)", today_cost, today_tokens),
                color: None,
                subtitle: None,
            });

            lines.push(MetricLine::Text {
                label: "7-day total".to_string(),
                value: format!(
                    "${:.2} ({} in / {} out / {} days)",
                    total_cost,
                    total_input,
                    total_output,
                    report.daily.len()
                ),
                color: None,
                subtitle: None,
            });

            Ok(ProviderUsage {
                provider_id: "claude".to_string(),
                display_name: "Claude Code".to_string(),
                plan: Some("Subscription".to_string()),
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
    fn test_claude_provider_id() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.id(), "claude");
    }

    #[test]
    fn test_claude_provider_display_name() {
        let provider = ClaudeProvider::new();
        assert_eq!(provider.display_name(), "Claude Code");
    }

    #[test]
    fn test_claude_provider_default() {
        let provider = ClaudeProvider::default();
        assert!(
            provider
                .credentials_path
                .to_string_lossy()
                .contains(".claude")
        );
    }
}
