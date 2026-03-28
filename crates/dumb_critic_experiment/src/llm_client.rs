use crate::models::{ModelReview, ModelTier, ParsedDefect, ReviewConfig, TokenUsage};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::{Duration, Instant};

/// OpenRouter API client for conducting reviews
pub struct LlmClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl LlmClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }

    /// Create client from environment variable
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| anyhow!("OPENROUTER_API_KEY environment variable not set"))?;
        Ok(Self::new(api_key))
    }

    /// Run a plan review with a specific model
    pub async fn review_plan(
        &self,
        model_tier: ModelTier,
        plan_id: &str,
        plan_content: &str,
        config: &ReviewConfig,
    ) -> Result<ModelReview> {
        let start = Instant::now();

        let model = model_tier.model_identifier();
        let messages = vec![
            json!({
                "role": "system",
                "content": config.system_prompt
            }),
            json!({
                "role": "user",
                "content": format!("Please review the following implementation plan and identify all defects:\n\n{}", plan_content)
            }),
        ];

        let body = json!({
            "model": model,
            "messages": messages,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .timeout(Duration::from_secs(120))
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenRouter API error ({}): {}", status, error_text));
        }

        let api_response: OpenRouterResponse = response.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let choice = api_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No completion choice returned"))?;

        let raw_output = choice.message.content.clone();
        let parsed_defects = parse_defects_from_output(&raw_output);

        let token_usage = api_response.usage.map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(ModelReview {
            model_tier,
            plan_id: plan_id.to_string(),
            raw_output,
            parsed_defects,
            token_usage,
            latency_ms,
            reviewed_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Check if API is accessible
    pub async fn health_check(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

/// OpenRouter API response structure
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct Message {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

/// Parse defects from model output
fn parse_defects_from_output(output: &str) -> Vec<ParsedDefect> {
    let mut defects = Vec::new();
    let mut current_defect: Option<ParsedDefectBuilder> = None;

    for line in output.lines() {
        let line = line.trim();

        // Start of a new defect
        if line.to_uppercase().starts_with("DEFECT") && line.contains(":") {
            if let Some(builder) = current_defect.take() {
                if let Some(defect) = builder.build() {
                    defects.push(defect);
                }
            }
            current_defect = Some(ParsedDefectBuilder::default());
            continue;
        }

        // Parse fields
        if let Some(ref mut builder) = current_defect {
            if let Some((key, value)) = parse_field_line(line) {
                match key.as_str() {
                    "type" | "defect_type" | "category" => builder.defect_type = Some(value),
                    "location" | "where" | "section" => builder.location = Some(value),
                    "description" | "issue" | "problem" => builder.description = Some(value),
                    "suggested fix" | "fix" | "recommendation" => {
                        builder.suggested_fix = Some(value)
                    }
                    _ => {}
                }
            }
        }
    }

    // Don't forget the last defect
    if let Some(builder) = current_defect {
        if let Some(defect) = builder.build() {
            defects.push(defect);
        }
    }

    defects
}

/// Parse a field line like "- Type: missing_prerequisite"
fn parse_field_line(line: &str) -> Option<(String, String)> {
    // Remove leading bullet or dash
    let line = line.trim_start_matches('-').trim_start_matches('*').trim();

    // Split on first colon
    if let Some(colon_pos) = line.find(':') {
        let key = line[..colon_pos].trim().to_ascii_lowercase();
        let value = line[colon_pos + 1..].trim().to_string();
        if !key.is_empty() && !value.is_empty() {
            return Some((key, value));
        }
    }

    None
}

/// Builder for parsing defects
#[derive(Default)]
struct ParsedDefectBuilder {
    defect_type: Option<String>,
    location: Option<String>,
    description: Option<String>,
    suggested_fix: Option<String>,
}

impl ParsedDefectBuilder {
    fn build(self) -> Option<ParsedDefect> {
        // Require at least description and location to consider it a valid defect
        self.description.as_ref()?;

        Some(ParsedDefect {
            defect_type: self.defect_type.unwrap_or_else(|| "unknown".to_string()),
            location: self.location.unwrap_or_else(|| "unspecified".to_string()),
            description: self.description.unwrap_or_default(),
            suggested_fix: self
                .suggested_fix
                .unwrap_or_else(|| "none provided".to_string()),
            matches_ground_truth: None,
            matched_gt_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_defects_from_output() {
        let output = r#"DEFECT 1:
- Type: missing_prerequisite
- Location: Section 2, Step 3
- Description: The plan assumes database schema exists but doesn't mention creating it
- Suggested fix: Add a prerequisite step to create database schema

DEFECT 2:
- Type: ambiguous_acceptance_criteria
- Location: Acceptance Criteria section
- Description: "Fast response time" is not defined numerically
- Suggested fix: Specify exact latency threshold (e.g., < 200ms)"#;

        let defects = parse_defects_from_output(output);
        assert_eq!(defects.len(), 2);

        assert_eq!(defects[0].defect_type, "missing_prerequisite");
        assert_eq!(defects[0].location, "Section 2, Step 3");

        assert_eq!(defects[1].defect_type, "ambiguous_acceptance_criteria");
        assert!(defects[1].description.contains("Fast response time"));
    }

    #[test]
    fn test_parse_no_defects() {
        let output = "No defects found.";
        let defects = parse_defects_from_output(output);
        assert!(defects.is_empty());
    }
}
