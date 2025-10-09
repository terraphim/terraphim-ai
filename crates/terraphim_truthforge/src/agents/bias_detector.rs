use crate::error::{Result, TruthForgeError};
use crate::types::{BiasAnalysis, BiasPattern, NarrativeContext};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};
use tracing::{info, warn};

pub struct BiasDetectorAgent {
    config: BiasDetectorConfig,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiasDetectorConfig {
    pub system_prompt: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

#[derive(Debug, Deserialize)]
struct LlmBiasResponse {
    biases: Vec<LlmBiasPattern>,
    overall_bias_score: f64,
}

#[derive(Debug, Deserialize)]
struct LlmBiasPattern {
    bias_type: String,
    text: String,
    explanation: String,
    severity: Option<f64>,
}

const BIAS_DETECTOR_SYSTEM_PROMPT: &str = r#"You are an expert content critic specializing in identifying bias, framing tactics, and rhetorical manipulation in narratives.

Analyze the narrative for:

1. **Loaded Language**: Emotionally charged words designed to influence rather than inform
   - Identify adjectives/adverbs that carry judgment
   - Note euphemisms or dysphemisms
   - Flag sensationalist or alarmist language

2. **Selective Framing**: How information is presented to shape perception
   - Active vs. passive voice (who is agent vs. recipient?)
   - Emphasis on certain aspects while downplaying others
   - Cherry-picking favorable data points

3. **Logical Fallacies**: Reasoning errors that undermine arguments
   - Ad hominem attacks
   - Straw man arguments
   - False dichotomies
   - Hasty generalizations
   - Appeals to emotion, authority, or tradition

4. **Disqualification Tactics**: Attempts to delegitimize opposing views
   - Guilt by association
   - Questioning motives rather than arguments
   - Dismissing criticism without engagement

5. **Rhetorical Devices**: Persuasion techniques masquerading as information
   - Repetition for emphasis
   - Rhetorical questions
   - Analogies that oversimplify

For each bias pattern, provide:
- **bias_type**: Category from above
- **text**: Specific quote demonstrating the bias
- **explanation**: How this text exhibits bias (100-200 words)
- **severity**: 0.0-1.0 (how egregious is the bias?)

Calculate **overall_bias_score**: 0.0 (neutral) to 1.0 (highly biased)

Return JSON object with:
{
  "biases": [array of bias patterns],
  "overall_bias_score": number
}

CRITICAL: Your response MUST be ONLY valid JSON matching this structure:
{
  "biases": [
    {
      "bias_type": "FramingBias",
      "text": "specific quote from narrative",
      "explanation": "how this exhibits bias",
      "severity": 0.75
    }
  ],
  "overall_bias_score": 0.68
}

Do NOT include any explanatory text, commentary, preamble, or postamble. Return only the JSON object."#;

impl Default for BiasDetectorConfig {
    fn default() -> Self {
        Self {
            system_prompt: BIAS_DETECTOR_SYSTEM_PROMPT.to_string(),
            max_tokens: 2000,
            temperature: 0.3,
        }
    }
}

impl BiasDetectorAgent {
    pub fn new(config: BiasDetectorConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub async fn analyze_bias(
        &self,
        narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<BiasAnalysis> {
        let client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| TruthForgeError::ConfigError("LLM client not configured".to_string()))?;

        info!("Calling LLM to analyze bias in narrative");

        let request = LlmRequest::new(vec![
            LlmMessage::system(self.config.system_prompt.clone()),
            LlmMessage::user(format!(
                "Analyze the following narrative for bias:\n\n{}",
                narrative
            )),
        ])
        .with_max_tokens(self.config.max_tokens as u64)
        .with_temperature(self.config.temperature as f32);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("LLM request failed: {}", e)))?;

        info!("Received LLM response, parsing bias analysis");

        let bias_analysis = self.parse_bias_from_llm(&response.content)?;
        Ok(bias_analysis)
    }

    fn parse_bias_from_llm(&self, content: &str) -> Result<BiasAnalysis> {
        let content = content.trim();

        let json_str = if content.starts_with("```json") {
            content
                .trim_start_matches("```json")
                .trim_end_matches("```")
                .trim()
        } else if content.starts_with("```") {
            content
                .trim_start_matches("```")
                .trim_end_matches("```")
                .trim()
        } else {
            content
        };

        // Try JSON parsing first
        let llm_response: LlmBiasResponse = match serde_json::from_str(json_str) {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to parse LLM response as JSON: {}", e);
                warn!("Raw content preview: {}", &content[..content.len().min(200)]);
                info!("Falling back to markdown parsing");
                return self.parse_bias_from_markdown(content);
            }
        };

        let biases: Vec<BiasPattern> = llm_response
            .biases
            .into_iter()
            .map(|llm_bias| BiasPattern {
                bias_type: llm_bias.bias_type,
                text: llm_bias.text,
                explanation: llm_bias.explanation,
            })
            .collect();

        let overall_bias_score = llm_response.overall_bias_score.clamp(0.0, 1.0);

        let confidence = if biases.is_empty() { 0.9 } else { 0.75 };

        info!(
            "Parsed {} bias patterns from LLM response, overall score: {:.2}",
            biases.len(),
            overall_bias_score
        );

        Ok(BiasAnalysis {
            biases,
            overall_bias_score,
            confidence,
        })
    }

    fn parse_bias_from_markdown(&self, content: &str) -> Result<BiasAnalysis> {
        info!("Parsing bias analysis from markdown format");
        let mut biases = Vec::new();
        let mut overall_score = 0.6; // Default moderate bias

        // Look for "Overall Bias Score:" or similar
        for line in content.lines() {
            if line.to_lowercase().contains("overall") && line.to_lowercase().contains("score") {
                // Try to extract score from line like "Overall Bias Score: 0.7" or "Overall Score: 70%"
                if let Some(score_str) = line.split(':').nth(1) {
                    let score_str = score_str.trim().replace('%', "");
                    if let Ok(score) = score_str.parse::<f64>() {
                        overall_score = if score > 1.0 { score / 100.0 } else { score };
                        overall_score = overall_score.clamp(0.0, 1.0);
                    }
                }
            }
        }

        // Extract bias patterns from markdown sections
        let mut current_type = String::new();
        let mut current_text = String::new();
        let mut current_explanation = String::new();
        let mut in_explanation = false;

        for line in content.lines() {
            let line = line.trim();

            // Check for bias type headers (like "1. Loaded Language:" or "### Loaded Language")
            if line.starts_with('#') || (line.chars().next().map(|c| c.is_numeric()).unwrap_or(false) && line.contains(':')) {
                // Save previous bias if we have one
                if !current_type.is_empty() && !current_text.is_empty() {
                    biases.push(BiasPattern {
                        bias_type: current_type.clone(),
                        text: current_text.clone(),
                        explanation: current_explanation.clone(),
                    });
                }

                // Extract new type
                current_type = line
                    .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ' ' || c == '#')
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                current_text = String::new();
                current_explanation = String::new();
                in_explanation = false;
            }
            // Check for "Text:" or "Quote:"
            else if line.to_lowercase().starts_with("text:") || line.to_lowercase().starts_with("quote:") {
                current_text = line.split(':').nth(1).unwrap_or("").trim().trim_matches('"').to_string();
            }
            // Check for "Explanation:"
            else if line.to_lowercase().starts_with("explanation:") {
                current_explanation = line.split(':').nth(1).unwrap_or("").trim().to_string();
                in_explanation = true;
            }
            // Continue explanation if we're in it
            else if in_explanation && !line.is_empty() && !line.starts_with("**") {
                if !current_explanation.is_empty() {
                    current_explanation.push(' ');
                }
                current_explanation.push_str(line);
            }
        }

        // Add the last bias if we have one
        if !current_type.is_empty() && !current_text.is_empty() {
            biases.push(BiasPattern {
                bias_type: current_type,
                text: current_text,
                explanation: current_explanation,
            });
        }

        // If we couldn't extract any structured biases, create a generic one from the content
        if biases.is_empty() {
            warn!("Could not extract structured bias patterns from markdown, creating generic pattern");
            biases.push(BiasPattern {
                bias_type: "General Bias".to_string(),
                text: "See full analysis".to_string(),
                explanation: content[..content.len().min(500)].to_string(),
            });
        }

        let confidence = if biases.len() > 1 { 0.75 } else { 0.6 };

        info!(
            "Extracted {} bias patterns from markdown (overall score: {:.2})",
            biases.len(),
            overall_score
        );

        Ok(BiasAnalysis {
            biases,
            overall_bias_score: overall_score,
            confidence,
        })
    }

    pub async fn analyze_bias_mock(
        &self,
        narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<BiasAnalysis> {
        let mut biases = Vec::new();

        if narrative.to_lowercase().contains("greatest")
            || narrative.to_lowercase().contains("best")
        {
            biases.push(BiasPattern {
                bias_type: "Loaded Language".to_string(),
                text: "Superlative language without evidence".to_string(),
                explanation: "Use of absolute terms like 'greatest' or 'best' without supporting evidence creates bias through unsubstantiated claims.".to_string(),
            });
        }

        let overall_bias_score = if biases.is_empty() { 0.3 } else { 0.6 };

        Ok(BiasAnalysis {
            biases,
            overall_bias_score,
            confidence: 0.7,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudienceType, StakeType, UrgencyLevel};

    #[tokio::test]
    async fn test_bias_detector_mock_detects_loaded_language() {
        let agent = BiasDetectorAgent::new(BiasDetectorConfig::default());
        let narrative = "We are the greatest company with the best solutions on the market.";
        let context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        };

        let result = agent.analyze_bias_mock(narrative, &context).await.unwrap();

        assert!(!result.biases.is_empty(), "Should detect loaded language");
        assert!(
            result.overall_bias_score > 0.5,
            "Should have elevated bias score"
        );
    }

    #[tokio::test]
    async fn test_bias_detector_mock_neutral_text() {
        let agent = BiasDetectorAgent::new(BiasDetectorConfig::default());
        let narrative = "The company reported quarterly results yesterday.";
        let context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![],
            audience: AudienceType::PublicMedia,
        };

        let result = agent.analyze_bias_mock(narrative, &context).await.unwrap();

        assert_eq!(
            result.biases.len(),
            0,
            "Should not detect bias in neutral text"
        );
        assert!(
            result.overall_bias_score < 0.5,
            "Should have low bias score"
        );
    }
}
