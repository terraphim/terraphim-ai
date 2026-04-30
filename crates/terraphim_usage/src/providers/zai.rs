use crate::{MetricLine, ProgressFormat, ProviderUsage, Result, UsageError, UsageProvider};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ZaiApiResponse {
    code: i64,
    data: ZaiUsageData,
    #[allow(dead_code)]
    success: bool,
}

#[derive(Debug, Deserialize)]
struct ZaiUsageData {
    limits: Vec<ZaiLimit>,
}

#[derive(Debug, Deserialize)]
struct ZaiLimit {
    #[serde(rename = "type")]
    limit_type: String,
    unit: i64,
    number: i64,
    usage: i64,
    #[serde(rename = "currentValue")]
    current_value: i64,
    #[allow(dead_code)]
    remaining: i64,
    #[allow(dead_code)]
    percentage: i64,
    #[serde(rename = "nextResetTime")]
    next_reset_time: Option<i64>,
    #[serde(rename = "usageDetails")]
    usage_details: Option<Vec<ZaiUsageDetail>>,
}

#[derive(Debug, Deserialize)]
struct ZaiUsageDetail {
    #[serde(rename = "modelCode")]
    model_code: String,
    usage: i64,
}

#[derive(Debug, Deserialize)]
struct ZaiSubscription {
    #[allow(dead_code)]
    code: i64,
    data: Vec<ZaiSubscriptionItem>,
    #[allow(dead_code)]
    success: bool,
}

#[derive(Debug, Deserialize)]
struct ZaiSubscriptionItem {
    #[serde(rename = "productName")]
    product_name: String,
    #[serde(rename = "nextRenewTime")]
    #[allow(dead_code)]
    next_renew_time: Option<String>,
}

/// Usage provider for the Zestic AI (ZAI) platform
pub struct ZaiProvider {
    api_key: Option<String>,
    client: reqwest::Client,
}

impl ZaiProvider {
    /// Create a provider reading the API key from `ZAI_API_KEY` or `GLM_API_KEY`
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("ZAI_API_KEY")
                .ok()
                .or_else(|| std::env::var("GLM_API_KEY").ok()),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Create a provider with an explicit API key
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            api_key: Some(api_key),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    async fn fetch_plan(&self) -> Option<String> {
        let api_key = self.api_key.as_ref()?;
        let resp = self
            .client
            .get("https://api.z.ai/api/biz/subscription/list")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .send()
            .await
            .ok()?;

        if resp.status().is_success() {
            let text = resp.text().await.ok()?;
            if let Ok(subs) = serde_json::from_str::<ZaiSubscription>(&text) {
                return subs.data.first().map(|s| s.product_name.clone());
            }
        }
        None
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
            let api_key = self
                .api_key
                .as_ref()
                .ok_or_else(|| UsageError::AuthFailed {
                    provider: "zai".to_string(),
                    message: "No ZAI_API_KEY or GLM_API_KEY found. Set environment variable first."
                        .to_string(),
                })?;

            let resp = self
                .client
                .get("https://api.z.ai/api/monitor/usage/quota/limit")
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Accept", "application/json")
                .send()
                .await
                .map_err(|e| UsageError::FetchFailed {
                    provider: "zai".to_string(),
                    source: Box::new(e),
                })?;

            if !resp.status().is_success() {
                let status = resp.status();
                return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
                    UsageError::AuthFailed {
                        provider: "zai".to_string(),
                        message: "API key invalid. Check your Z.ai API key.".to_string(),
                    }
                } else {
                    UsageError::FetchFailed {
                        provider: "zai".to_string(),
                        source: format!("Request failed (HTTP {}). Try again later.", status)
                            .into(),
                    }
                });
            }

            let text = resp.text().await.map_err(|e| UsageError::FetchFailed {
                provider: "zai".to_string(),
                source: Box::new(e),
            })?;

            let data: ZaiApiResponse =
                serde_json::from_str(&text).map_err(|e| UsageError::FetchFailed {
                    provider: "zai".to_string(),
                    source: format!("Usage response invalid: {}. Try again later.", e).into(),
                })?;

            if data.code != 200 {
                return Err(UsageError::FetchFailed {
                    provider: "zai".to_string(),
                    source: format!("API returned code {}. Try again later.", data.code).into(),
                });
            }

            let mut lines = Vec::new();
            let now = Utc::now();

            for limit in &data.data.limits {
                match limit.limit_type.as_str() {
                    "TOKENS_LIMIT" => {
                        // unit: 3 = 5-hour session, unit: 6 = 7-day weekly
                        let label = if limit.unit == 3 && limit.number == 5 {
                            "Session"
                        } else if limit.unit == 6 && limit.number == 7 {
                            "Weekly"
                        } else {
                            "Tokens"
                        };

                        let resets_at = limit.next_reset_time.map(|ms| {
                            DateTime::<Utc>::from_timestamp_millis(ms)
                                .map(|dt| dt.to_rfc3339())
                                .unwrap_or_default()
                        });

                        lines.push(MetricLine::Progress {
                            label: label.to_string(),
                            used: limit.current_value as f64,
                            limit: limit.usage as f64,
                            format: ProgressFormat::Percent,
                            resets_at,
                            period_duration_ms: None,
                            color: None,
                        });
                    }
                    "TIME_LIMIT" => {
                        // Web search/reader calls
                        if let Some(details) = &limit.usage_details {
                            let total_used: i64 = details.iter().map(|d| d.usage).sum();
                            lines.push(MetricLine::Text {
                                label: "Web Searches".to_string(),
                                value: format!("{} / {}", total_used, limit.usage),
                                color: None,
                                subtitle: Some(
                                    details
                                        .iter()
                                        .map(|d| format!("{}: {}", d.model_code, d.usage))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                ),
                            });
                        }
                    }
                    _ => {}
                }
            }

            // Fetch plan name
            let plan = self.fetch_plan().await;

            Ok(ProviderUsage {
                provider_id: "zai".to_string(),
                display_name: "Z.ai".to_string(),
                plan,
                lines,
                fetched_at: now.to_rfc3339(),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zai_usage_response() {
        let json = r#"{
            "code": 200,
            "data": {
                "limits": [
                    {
                        "type": "TOKENS_LIMIT",
                        "unit": 3,
                        "number": 5,
                        "usage": 800000000,
                        "currentValue": 127694464,
                        "remaining": 672305536,
                        "percentage": 15,
                        "nextResetTime": 1770648402389
                    },
                    {
                        "type": "TIME_LIMIT",
                        "unit": 5,
                        "number": 1,
                        "usage": 4000,
                        "currentValue": 1828,
                        "remaining": 2172,
                        "percentage": 45,
                        "usageDetails": [
                            {"modelCode": "search-prime", "usage": 1433},
                            {"modelCode": "web-reader", "usage": 462}
                        ]
                    }
                ]
            },
            "success": true
        }"#;

        let data: ZaiApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(data.data.limits.len(), 2);
        assert_eq!(data.data.limits[0].limit_type, "TOKENS_LIMIT");
        assert_eq!(data.data.limits[0].percentage, 15);
        assert_eq!(data.data.limits[1].usage_details.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_zai_subscription_response() {
        let json = r#"{
            "code": 200,
            "data": [
                {
                    "productName": "GLM Coding Max",
                    "nextRenewTime": "2026-02-12"
                }
            ],
            "success": true
        }"#;

        let subs: ZaiSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(subs.data[0].product_name, "GLM Coding Max");
    }

    #[test]
    fn test_zai_provider_no_api_key() {
        let has_key = std::env::var("ZAI_API_KEY")
            .or_else(|_| std::env::var("GLM_API_KEY"))
            .is_ok();
        if has_key {
            return;
        }
        let provider = ZaiProvider::new();
        assert!(
            provider.api_key.is_none(),
            "ZaiProvider should have no API key when ZAI_API_KEY and GLM_API_KEY are unset"
        );
    }

    #[test]
    fn test_zai_provider_with_explicit_key() {
        let provider = ZaiProvider::with_api_key("test-key".to_string());
        assert_eq!(provider.api_key.as_deref(), Some("test-key"));
    }
}
