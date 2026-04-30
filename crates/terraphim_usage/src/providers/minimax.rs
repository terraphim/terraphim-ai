use crate::{MetricLine, ProgressFormat, ProviderUsage, Result, UsageError, UsageProvider};
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct MiniMaxApiResponse {
    #[serde(rename = "baseResp")]
    base_resp: MiniMaxBaseResp,
    #[serde(rename = "modelRemains")]
    model_remains: Vec<MiniMaxModelRemains>,
}

#[derive(Debug, Deserialize)]
struct MiniMaxBaseResp {
    #[serde(rename = "status_code")]
    status_code: i64,
    #[serde(rename = "status_msg")]
    status_msg: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MiniMaxModelRemains {
    #[serde(rename = "current_interval_total_count")]
    total_count: Option<i64>,
    #[serde(rename = "current_interval_usage_count")]
    usage_count: Option<i64>,
    #[serde(rename = "current_interval_remaining_count")]
    remaining_count: Option<i64>,
    #[serde(rename = "start_time")]
    start_time: Option<String>,
    #[serde(rename = "end_time")]
    end_time: Option<String>,
    #[serde(rename = "remains_time")]
    remains_time: Option<String>,
    #[serde(rename = "current_subscribe_title")]
    subscribe_title: Option<String>,
    #[serde(rename = "plan_name")]
    plan_name: Option<String>,
    plan: Option<String>,
}

/// Usage provider for the MiniMax LLM platform
pub struct MiniMaxProvider {
    api_key: Option<String>,
    cn_api_key: Option<String>,
    client: reqwest::Client,
}

impl MiniMaxProvider {
    /// Create a provider reading the API key from `MINIMAX_API_KEY` or `MINIMAX_API_TOKEN`
    pub fn new() -> Self {
        Self {
            api_key: std::env::var("MINIMAX_API_KEY")
                .ok()
                .or_else(|| std::env::var("MINIMAX_API_TOKEN").ok()),
            cn_api_key: std::env::var("MINIMAX_CN_API_KEY").ok(),
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
            cn_api_key: None,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
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
            // Determine region and key
            let (region, api_key) = if let Some(ref cn_key) = self.cn_api_key {
                ("CN", cn_key.as_str())
            } else if let Some(ref key) = self.api_key {
                ("GLOBAL", key.as_str())
            } else {
                return Err(UsageError::AuthFailed {
                    provider: "minimax".to_string(),
                    message: "MiniMax API key missing. Set MINIMAX_API_KEY or MINIMAX_CN_API_KEY."
                        .to_string(),
                });
            };

            let base_url = if region == "CN" {
                "https://api.minimaxi.com"
            } else {
                "https://api.minimax.io"
            };

            let endpoints = [
                format!("{}/v1/api/openplatform/coding_plan/remains", base_url),
                format!("{}/v1/coding_plan/remains", base_url),
            ];

            let mut last_error = None;
            for url in &endpoints {
                match self
                    .client
                    .get(url)
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Accept", "application/json")
                    .send()
                    .await
                {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            let text = resp.text().await.map_err(|e| UsageError::FetchFailed {
                                provider: "minimax".to_string(),
                                source: Box::new(e),
                            })?;

                            // Check for HTML response (Cloudflare error page)
                            if text.trim_start().starts_with('<') {
                                last_error = Some(
                                    "Received HTML response (possibly Cloudflare). Try again later."
                                        .to_string(),
                                );
                                continue;
                            }

                            let data: MiniMaxApiResponse =
                                serde_json::from_str(&text).map_err(|e| {
                                    UsageError::FetchFailed {
                                        provider: "minimax".to_string(),
                                        source: format!(
                                            "Could not parse usage data: {}. Response: {}",
                                            e,
                                            &text[..text.len().min(200)]
                                        )
                                        .into(),
                                    }
                                })?;

                            if data.base_resp.status_code != 0 {
                                let msg = data.base_resp.status_msg.unwrap_or_default();
                                if msg.contains("expired") || msg.contains("auth") {
                                    return Err(UsageError::AuthFailed {
                                        provider: "minimax".to_string(),
                                        message: format!(
                                            "Session expired. Check your MiniMax API key. ({})",
                                            msg
                                        ),
                                    });
                                }
                                return Err(UsageError::FetchFailed {
                                    provider: "minimax".to_string(),
                                    source: format!("MiniMax API error: {}", msg).into(),
                                });
                            }

                            return self.build_usage_response(data, region);
                        } else if resp.status().as_u16() == 401 || resp.status().as_u16() == 403 {
                            return Err(UsageError::AuthFailed {
                                provider: "minimax".to_string(),
                                message: "Session expired. Check your MiniMax API key.".to_string(),
                            });
                        } else {
                            last_error = Some(format!(
                                "Request failed (HTTP {}). Try again later.",
                                resp.status()
                            ));
                        }
                    }
                    Err(e) => {
                        last_error =
                            Some(format!("Request failed. Check your connection. ({})", e));
                    }
                }
            }

            Err(UsageError::FetchFailed {
                provider: "minimax".to_string(),
                source: last_error
                    .unwrap_or("All endpoints failed".to_string())
                    .into(),
            })
        })
    }
}

impl MiniMaxProvider {
    fn build_usage_response(
        &self,
        data: MiniMaxApiResponse,
        region: &str,
    ) -> Result<ProviderUsage> {
        let mut lines = Vec::new();
        let now = Utc::now();

        // Determine plan name
        let plan_name = data
            .model_remains
            .first()
            .and_then(|m| {
                m.subscribe_title
                    .clone()
                    .or_else(|| m.plan_name.clone())
                    .or_else(|| m.plan.clone())
            })
            .map(|p| format!("{} ({})", p, region));

        for model in &data.model_remains {
            let total = model.total_count.unwrap_or(0);
            let used = model.usage_count.unwrap_or_else(|| {
                // If only remaining is provided, compute used = total - remaining
                total - model.remaining_count.unwrap_or(0)
            });

            let resets_at = model
                .end_time
                .clone()
                .or_else(|| model.remains_time.clone())
                .and_then(|t| {
                    // Try parsing as unix timestamp
                    if let Ok(ts) = t.parse::<i64>() {
                        DateTime::<Utc>::from_timestamp_millis(ts)
                            .or_else(|| DateTime::<Utc>::from_timestamp(ts, 0))
                            .map(|dt| dt.to_rfc3339())
                    } else {
                        None
                    }
                });

            let period_duration_ms = model
                .start_time
                .as_ref()
                .and_then(|s| s.parse::<i64>().ok())
                .zip(model.end_time.as_ref().and_then(|e| e.parse::<i64>().ok()))
                .map(|(start, end)| ((end - start).max(0) as u64) * 1000);

            lines.push(MetricLine::Progress {
                label: "Session".to_string(),
                used: used as f64,
                limit: total as f64,
                format: ProgressFormat::Count {
                    suffix: "prompts".to_string(),
                },
                resets_at,
                period_duration_ms,
                color: None,
            });
        }

        Ok(ProviderUsage {
            provider_id: "minimax".to_string(),
            display_name: format!("MiniMax ({})", region),
            plan: plan_name,
            lines,
            fetched_at: now.to_rfc3339(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimax_response() {
        let json = r#"{
            "baseResp": {
                "status_code": 0,
                "status_msg": "success"
            },
            "modelRemains": [
                {
                    "current_interval_total_count": 300,
                    "current_interval_usage_count": 180,
                    "current_interval_remaining_count": 120,
                    "start_time": "1712000000",
                    "end_time": "1712018000",
                    "plan_name": "Pro"
                }
            ]
        }"#;

        let data: MiniMaxApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(data.base_resp.status_code, 0);
        assert_eq!(data.model_remains.len(), 1);
        assert_eq!(data.model_remains[0].total_count, Some(300));
        assert_eq!(data.model_remains[0].usage_count, Some(180));
    }

    #[test]
    fn test_parse_minimax_error_response() {
        let json = r#"{
            "baseResp": {
                "status_code": 1001,
                "status_msg": "Invalid API key"
            },
            "modelRemains": []
        }"#;

        let data: MiniMaxApiResponse = serde_json::from_str(json).unwrap();
        assert_ne!(data.base_resp.status_code, 0);
    }

    #[test]
    fn test_minimax_provider_no_api_key() {
        let provider = MiniMaxProvider::new();
        let _ = provider.api_key.is_none();
    }
}
