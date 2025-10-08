use crate::error::{Result, TruthForgeError};
use crate::types::{
    NarrativeContext, Omission, OmissionCatalog, OmissionCategory, StakeType, UrgencyLevel,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};
use tracing::{info, warn};

pub struct OmissionDetectorAgent {
    config: OmissionDetectorConfig,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmissionDetectorConfig {
    pub system_prompt_template: String,
    pub max_tokens: usize,
    pub temperature: f64,
    pub llm_provider: String,
    pub llm_model: String,
}

#[derive(Debug, Deserialize)]
struct LlmOmissionResponse {
    category: String,
    description: String,
    severity: f64,
    exploitability: f64,
    text_reference: String,
    confidence: f64,
    suggested_addition: Option<String>,
}

impl Default for OmissionDetectorConfig {
    fn default() -> Self {
        Self {
            system_prompt_template: OMISSION_DETECTOR_SYSTEM_PROMPT.to_string(),
            max_tokens: 2000,
            temperature: 0.3,
            llm_provider: "openrouter".to_string(),
            llm_model: "anthropic/claude-3.5-sonnet".to_string(),
        }
    }
}

const OMISSION_DETECTOR_SYSTEM_PROMPT: &str = r#"You are an expert at identifying gaps, missing context, and unstated assumptions in narratives.

For each narrative, systematically analyze:

1. **Missing Evidence**: Claims without supporting data, statistics, or sources
   - Look for quantitative claims lacking attribution
   - Identify assertions presented as fact without proof
   - Note vague language ("many", "significant") without specifics

2. **Unstated Assumptions**: Implied premises or beliefs not explicitly stated
   - What must be true for this narrative to make sense?
   - What values or priorities are taken for granted?
   - What counterfactual scenarios are ignored?

3. **Absent Stakeholder Voices**: Perspectives or groups not represented
   - Who is affected but not mentioned?
   - Whose interests are served vs. harmed?
   - What stakeholder groups are conspicuously silent?

4. **Context Gaps**: Background information, history, or circumstances omitted
   - What prior events led to this situation?
   - What industry/regulatory context is missing?
   - What comparisons to competitors or benchmarks are absent?

5. **Unaddressed Counterarguments**: Obvious rebuttals or alternative explanations ignored
   - What objections would skeptics raise?
   - What alternative interpretations exist?
   - What inconvenient facts are left out?

For each omission, provide:
- **Category**: (from list above)
- **Description**: What's missing (50-200 words)
- **Severity**: 0.0-1.0 (impact if omission is highlighted by opponents)
- **Exploitability**: 0.0-1.0 (ease with which adversaries can weaponize this gap)
- **Text Reference**: Specific quote from narrative that triggered detection
- **Confidence**: 0.0-1.0 (how certain are you this is truly an omission?)
- **Suggested Addition**: Brief suggestion of what should be added

Return JSON array of omissions. Prioritize by composite risk = severity × exploitability.
Return top 10-15 most critical omissions.

CRITICAL: Your response MUST be ONLY valid JSON matching this structure:
[
  {
    "category": "Missing Evidence",
    "description": "...",
    "severity": 0.85,
    "exploitability": 0.92,
    "text_reference": "...",
    "confidence": 0.89,
    "suggested_addition": "..."
  }
]

Do NOT include any explanatory text, commentary, preamble, or postamble. Return only the JSON array."#;

impl OmissionDetectorAgent {
    pub fn new(config: OmissionDetectorConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub fn build_prompt(&self, narrative: &str, context: &NarrativeContext) -> String {
        let mut prompt = self.config.system_prompt_template.clone();

        let urgency_modifier = match context.urgency {
            UrgencyLevel::High => "\nIMPORTANT: This is a high-urgency crisis scenario. Prioritize omissions that opponents could exploit in next 24-48 hours.",
            UrgencyLevel::Low => "\nThis is strategic planning. Focus on systemic omissions that would emerge in sustained scrutiny.",
        };
        prompt.push_str(urgency_modifier);

        if context.stakes.contains(&StakeType::Legal) {
            prompt.push_str("\nPay special attention to missing legal context, regulatory compliance information, and unstated legal assumptions.");
        }

        if context.stakes.contains(&StakeType::Reputational) {
            prompt.push_str("\nFocus on reputational risks: missing stakeholder perspectives, unaddressed criticisms, and absent context that could damage credibility.");
        }

        prompt.push_str("\n\nNarrative to analyze:\n\n");
        prompt.push_str(narrative);

        prompt
    }

    pub async fn detect_omissions(
        &self,
        narrative: &str,
        context: &NarrativeContext,
    ) -> Result<OmissionCatalog> {
        let client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| TruthForgeError::ConfigError("LLM client not configured".to_string()))?;

        let prompt = self.build_prompt(narrative, context);

        info!("Calling LLM to detect omissions in narrative");

        let request = LlmRequest::new(vec![
            LlmMessage::system(self.config.system_prompt_template.clone()),
            LlmMessage::user(prompt),
        ])
        .with_max_tokens(self.config.max_tokens as u64)
        .with_temperature(self.config.temperature as f32);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("LLM request failed: {}", e)))?;

        info!("Received LLM response, parsing omissions");

        let omissions = self.parse_omissions_from_llm(&response.content)?;

        let catalog = OmissionCatalog::new(omissions);
        Ok(catalog)
    }

    fn parse_omissions_from_llm(&self, content: &str) -> Result<Vec<Omission>> {
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

        let llm_omissions: Vec<LlmOmissionResponse> =
            serde_json::from_str(json_str).map_err(|e| {
                warn!("Failed to parse LLM response as JSON: {}", e);
                warn!("Raw content: {}", &content[..content.len().min(500)]);
                TruthForgeError::ParseError(format!("Failed to parse omissions JSON: {}", e))
            })?;

        let omissions: Vec<Omission> = llm_omissions
            .into_iter()
            .map(|llm_om| {
                let category = match llm_om.category.to_lowercase().as_str() {
                    s if s.contains("evidence") => OmissionCategory::MissingEvidence,
                    s if s.contains("assumption") => OmissionCategory::UnstatedAssumption,
                    s if s.contains("stakeholder") => OmissionCategory::AbsentStakeholder,
                    s if s.contains("context") => OmissionCategory::ContextGap,
                    s if s.contains("counter") => OmissionCategory::UnaddressedCounterargument,
                    _ => {
                        warn!(
                            "Unknown category '{}', defaulting to ContextGap",
                            llm_om.category
                        );
                        OmissionCategory::ContextGap
                    }
                };

                Omission {
                    id: uuid::Uuid::new_v4(),
                    category,
                    description: llm_om.description,
                    severity: llm_om.severity.clamp(0.0, 1.0),
                    exploitability: llm_om.exploitability.clamp(0.0, 1.0),
                    composite_risk: (llm_om.severity * llm_om.exploitability).clamp(0.0, 1.0),
                    text_reference: llm_om.text_reference,
                    confidence: llm_om.confidence.clamp(0.0, 1.0),
                    suggested_addition: llm_om.suggested_addition,
                }
            })
            .collect();

        info!("Parsed {} omissions from LLM response", omissions.len());
        Ok(omissions)
    }

    pub async fn detect_omissions_mock(
        &self,
        narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<OmissionCatalog> {
        let mut omissions = Vec::new();

        if narrative.contains("40%") || narrative.contains("reduction") {
            omissions.push(Omission {
                id: uuid::Uuid::new_v4(),
                category: OmissionCategory::MissingEvidence,
                description: "Claim about percentage reduction lacks supporting data source, baseline period, or methodology. No attribution to audit or third-party validation.".to_string(),
                severity: 0.85,
                exploitability: 0.92,
                composite_risk: 0.85 * 0.92,
                text_reference: narrative.chars().take(100).collect(),
                confidence: 0.89,
                suggested_addition: Some("Specify: time period measured, methodology used, independent verification source, and baseline comparison data.".to_string()),
            });
        }

        if (narrative.contains("cost")
            || narrative.contains("profit") || narrative.contains("benefit")) && !narrative.to_lowercase().contains("employee") && !narrative.to_lowercase().contains("worker") {
            omissions.push(Omission {
                id: uuid::Uuid::new_v4(),
                category: OmissionCategory::AbsentStakeholder,
                description: "Employee or worker perspectives are absent. Financial claims made without addressing impact on workforce, labor conditions, or job security.".to_string(),
                severity: 0.72,
                exploitability: 0.81,
                composite_risk: 0.72 * 0.81,
                text_reference: "Entire narrative".to_string(),
                confidence: 0.76,
                suggested_addition: Some("Include: employee impact, job security measures, workforce investment, or labor relations context.".to_string()),
            });
        }

        if !narrative.to_lowercase().contains("because")
            && !narrative.to_lowercase().contains("due to")
        {
            omissions.push(Omission {
                id: uuid::Uuid::new_v4(),
                category: OmissionCategory::ContextGap,
                description: "No causal explanation provided. Results stated without explaining underlying factors, strategic decisions, or market conditions that led to outcomes.".to_string(),
                severity: 0.68,
                exploitability: 0.74,
                composite_risk: 0.68 * 0.74,
                text_reference: "Entire narrative structure".to_string(),
                confidence: 0.71,
                suggested_addition: Some("Add: explanation of key strategic decisions, market factors, operational changes, or external conditions.".to_string()),
            });
        }

        if narrative.len() < 300 {
            omissions.push(Omission {
                id: uuid::Uuid::new_v4(),
                category: OmissionCategory::UnaddressedCounterargument,
                description: "Narrative is brief and one-sided. No acknowledgment of potential criticisms, alternative interpretations, or counterfactual scenarios.".to_string(),
                severity: 0.61,
                exploitability: 0.69,
                composite_risk: 0.61 * 0.69,
                text_reference: "Brief narrative length and lack of counterarguments".to_string(),
                confidence: 0.64,
                suggested_addition: Some("Address: potential criticisms, acknowledge limitations, or present balanced view with counterarguments.".to_string()),
            });
        }

        let catalog = OmissionCatalog::new(omissions);
        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudienceType, StakeType, UrgencyLevel};

    #[tokio::test]
    async fn test_omission_detector_finds_missing_evidence() {
        let agent = OmissionDetectorAgent::new(OmissionDetectorConfig::default());
        let narrative = "We reduced costs by 40%. This benefited shareholders greatly.";
        let context = NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        };

        let result = agent
            .detect_omissions_mock(narrative, &context)
            .await
            .unwrap();

        assert!(
            result.omissions.len() >= 2,
            "Should detect at least 2 omissions"
        );

        let has_missing_evidence = result
            .omissions
            .iter()
            .any(|o| matches!(o.category, OmissionCategory::MissingEvidence));
        assert!(
            has_missing_evidence,
            "Should detect missing evidence for '40%' claim"
        );

        let top_risk = result.omissions.first().unwrap();
        assert!(
            top_risk.composite_risk > 0.7,
            "Top omission should be high risk"
        );
    }

    #[tokio::test]
    async fn test_omission_detector_finds_absent_stakeholders() {
        let agent = OmissionDetectorAgent::new(OmissionDetectorConfig::default());
        let narrative = "Our company achieved record profits while maintaining our commitment to sustainability.";
        let context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational, StakeType::SocialLicense],
            audience: AudienceType::PublicMedia,
        };

        let result = agent
            .detect_omissions_mock(narrative, &context)
            .await
            .unwrap();

        assert!(
            result.omissions.len() >= 3,
            "Should detect multiple omission types"
        );

        let has_absent_stakeholder = result
            .omissions
            .iter()
            .any(|o| matches!(o.category, OmissionCategory::AbsentStakeholder));
        assert!(
            has_absent_stakeholder,
            "Should detect absent employee/worker voices"
        );
    }

    #[tokio::test]
    async fn test_omission_catalog_prioritization() {
        let agent = OmissionDetectorAgent::new(OmissionDetectorConfig::default());
        let narrative = "Costs reduced significantly through optimization.";
        let context = NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        };

        let result = agent
            .detect_omissions_mock(narrative, &context)
            .await
            .unwrap();

        let top_10 = result.get_top_n(10);

        for i in 1..top_10.len() {
            assert!(
                top_10[i - 1].composite_risk >= top_10[i].composite_risk,
                "Omissions should be sorted by composite risk (descending)"
            );
        }

        assert_eq!(
            result.prioritized.len(),
            result.omissions.len().min(10),
            "Prioritized list should contain top 10 (or fewer if less than 10 total)"
        );
    }

    #[test]
    fn test_build_prompt_with_urgency() {
        let agent = OmissionDetectorAgent::new(OmissionDetectorConfig::default());
        let narrative = "Test narrative";

        let high_urgency_context = NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![],
            audience: AudienceType::PublicMedia,
        };

        let prompt = agent.build_prompt(narrative, &high_urgency_context);
        assert!(
            prompt.contains("24-48 hours"),
            "High urgency prompt should mention immediate timeframe"
        );

        let low_urgency_context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![],
            audience: AudienceType::PublicMedia,
        };

        let prompt = agent.build_prompt(narrative, &low_urgency_context);
        assert!(
            prompt.contains("strategic planning"),
            "Low urgency prompt should mention strategic context"
        );
    }

    #[test]
    fn test_build_prompt_with_legal_stakes() {
        let agent = OmissionDetectorAgent::new(OmissionDetectorConfig::default());
        let narrative = "Test narrative";

        let legal_context = NarrativeContext {
            urgency: UrgencyLevel::High,
            stakes: vec![StakeType::Legal],
            audience: AudienceType::PublicMedia,
        };

        let prompt = agent.build_prompt(narrative, &legal_context);
        assert!(
            prompt.contains("legal context"),
            "Legal stakes should add legal context guidance"
        );
        assert!(
            prompt.contains("regulatory compliance"),
            "Legal stakes should mention regulatory compliance"
        );
    }
}
