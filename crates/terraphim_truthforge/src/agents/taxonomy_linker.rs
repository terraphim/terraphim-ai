use crate::error::{Result, TruthForgeError};
use crate::types::{NarrativeContext, TaxonomyLinking};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};
use tracing::{info, warn};

pub struct TaxonomyLinkerAgent {
    config: TaxonomyLinkerConfig,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxonomyLinkerConfig {
    pub system_prompt: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

#[derive(Debug, Deserialize)]
struct LlmTaxonomyLinkingResponse {
    primary_domain: Option<String>,
    primary_function: Option<String>,
    secondary_functions: Option<Vec<String>>,
    subfunctions: Vec<String>,
    lifecycle_stage: Option<String>,
    applicable_playbooks: Option<Vec<String>>,
    recommended_playbooks: Option<Vec<String>>,
}

const TAXONOMY_LINKER_SYSTEM_PROMPT: &str = r#"You are an expert in linking scenarios to strategic communication taxonomies.

Map the narrative to the TruthForge Taxonomy:

**Taxonomy Domains**:
1. **relationship_management**: Stakeholder engagement, dialogue
   - Subfunctions: stakeholder_mapping, engagement_design, community_building
   
2. **issue_crisis_management**: Crisis anticipation and response
   - Subfunctions: horizon_scanning, risk_assessment, war_room_operations, recovery_and_learning
   - Issue types: operational, product_safety, ethics_compliance, cybersecurity, leadership
   
3. **strategic_management_function**: Communication as strategic capability
   - Subfunctions: strategy_alignment, executive_advisory, policy_standards, enterprise_listening

**Lifecycle Stages**:
- prepare_and_scan: Early warning, horizon scanning
- assess_and_classify: Risk assessment, SCCT classification
- respond_and_engage: Active crisis management
- recover_and_learn: Post-crisis analysis

Return JSON with:
{
  "primary_function": "domain name",
  "secondary_functions": ["additional domains"],
  "subfunctions": ["specific subfunctions"],
  "lifecycle_stage": "stage name",
  "recommended_playbooks": ["playbook names"]
}

CRITICAL: Your response MUST be ONLY valid JSON matching this structure:
{
  "primary_function": "stakeholder_engagement",
  "secondary_functions": ["media_relations", "transparency_reporting"],
  "subfunctions": ["crisis_spokesperson_prep", "q_and_a_document"],
  "lifecycle_stage": "respond_and_engage",
  "recommended_playbooks": ["Playbook: Media Crisis Response"]
}

Do NOT include any explanatory text, commentary, preamble, or postamble. Return only the JSON object."#;

impl Default for TaxonomyLinkerConfig {
    fn default() -> Self {
        Self {
            system_prompt: TAXONOMY_LINKER_SYSTEM_PROMPT.to_string(),
            max_tokens: 2000,
            temperature: 0.2,
        }
    }
}

impl TaxonomyLinkerAgent {
    pub fn new(config: TaxonomyLinkerConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub async fn link_taxonomy(
        &self,
        narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<TaxonomyLinking> {
        let client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| TruthForgeError::ConfigError("LLM client not configured".to_string()))?;

        info!("Calling LLM to link narrative to taxonomy");

        let request = LlmRequest::new(vec![
            LlmMessage::system(self.config.system_prompt.clone()),
            LlmMessage::user(format!(
                "Map this narrative to the taxonomy:\n\n{}",
                narrative
            )),
        ])
        .with_max_tokens(self.config.max_tokens as u64)
        .with_temperature(self.config.temperature as f32);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("LLM request failed: {}", e)))?;

        info!("Received LLM response, parsing taxonomy linking");

        let linking = self.parse_linking_from_llm(&response.content)?;
        Ok(linking)
    }

    fn parse_linking_from_llm(&self, content: &str) -> Result<TaxonomyLinking> {
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

        let llm_response: LlmTaxonomyLinkingResponse = match serde_json::from_str(json_str) {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to parse LLM response as JSON: {}", e);
                warn!(
                    "Raw content preview: {}",
                    &content[..content.len().min(200)]
                );
                info!("Falling back to markdown parsing");
                return self.parse_linking_from_markdown(content);
            }
        };

        let primary_function = llm_response
            .primary_function
            .or(llm_response.primary_domain)
            .unwrap_or_else(|| "issue_crisis_management".to_string());

        let secondary_functions = llm_response.secondary_functions.unwrap_or_default();

        let lifecycle_stage = llm_response
            .lifecycle_stage
            .unwrap_or_else(|| "assess_and_classify".to_string());

        let recommended_playbooks = llm_response
            .recommended_playbooks
            .or(llm_response.applicable_playbooks)
            .unwrap_or_else(|| vec!["SCCT_response_matrix".to_string()]);

        info!(
            "Parsed taxonomy linking: primary={}, subfunctions={}, stage={}",
            primary_function,
            llm_response.subfunctions.len(),
            lifecycle_stage
        );

        Ok(TaxonomyLinking {
            primary_function,
            secondary_functions,
            subfunctions: llm_response.subfunctions,
            lifecycle_stage,
            recommended_playbooks,
        })
    }

    fn parse_linking_from_markdown(&self, content: &str) -> Result<TaxonomyLinking> {
        info!("Parsing taxonomy linking from markdown format");
        let mut primary_function = "issue_crisis_management".to_string();
        let mut secondary_functions = Vec::new();
        let mut subfunctions = Vec::new();
        let mut lifecycle_stage = "assess_and_classify".to_string();
        let mut recommended_playbooks = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            let lower = line.to_lowercase();

            // Extract primary function/domain
            if lower.contains("primary") && (lower.contains("function") || lower.contains("domain"))
            {
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim().trim_matches('"').to_string();
                    if !value.is_empty() && value.len() < 100 {
                        primary_function = value;
                    }
                }
            }
            // Extract secondary functions
            else if lower.contains("secondary") && lower.contains("function") {
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim();
                    // Handle both array format and comma-separated
                    for func in value.split(',') {
                        let func = func
                            .trim()
                            .trim_matches(|c| c == '[' || c == ']' || c == '"');
                        if !func.is_empty() && func.len() < 100 {
                            secondary_functions.push(func.to_string());
                        }
                    }
                }
            }
            // Extract subfunctions
            else if lower.contains("subfunction") {
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim();
                    for subf in value.split(',') {
                        let subf = subf
                            .trim()
                            .trim_matches(|c| c == '[' || c == ']' || c == '"');
                        if !subf.is_empty() && subf.len() < 100 {
                            subfunctions.push(subf.to_string());
                        }
                    }
                }
            }
            // Extract lifecycle stage
            else if lower.contains("lifecycle") && lower.contains("stage") {
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim().trim_matches('"').to_string();
                    if !value.is_empty() && value.len() < 100 {
                        lifecycle_stage = value;
                    }
                }
            }
            // Extract recommended playbooks
            else if lower.contains("playbook") {
                if let Some(value) = line.split(':').nth(1) {
                    let value = value.trim();
                    for playbook in value.split(',') {
                        let playbook = playbook
                            .trim()
                            .trim_matches(|c| c == '[' || c == ']' || c == '"');
                        if !playbook.is_empty() && playbook.len() < 200 {
                            recommended_playbooks.push(playbook.to_string());
                        }
                    }
                }
                // Also extract playbooks from lines like "- Playbook Name"
                else if line.starts_with('-') || line.starts_with('*') {
                    let playbook = line.trim_start_matches(['-', '*', ' ']);
                    if !playbook.is_empty() && playbook.len() < 200 {
                        recommended_playbooks.push(playbook.to_string());
                    }
                }
            }
        }

        // Ensure we have at least one subfunction
        if subfunctions.is_empty() {
            subfunctions.push("risk_assessment".to_string());
        }

        // Ensure we have at least one playbook
        if recommended_playbooks.is_empty() {
            recommended_playbooks.push("SCCT_response_matrix".to_string());
        }

        info!(
            "Extracted taxonomy from markdown: primary={}, {} subfunctions, stage={}",
            primary_function,
            subfunctions.len(),
            lifecycle_stage
        );

        Ok(TaxonomyLinking {
            primary_function,
            secondary_functions,
            subfunctions,
            lifecycle_stage,
            recommended_playbooks,
        })
    }

    pub async fn link_taxonomy_mock(
        &self,
        _narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<TaxonomyLinking> {
        Ok(TaxonomyLinking {
            primary_function: "issue_crisis_management".to_string(),
            secondary_functions: vec![],
            subfunctions: vec!["risk_assessment".to_string()],
            lifecycle_stage: "assess_and_classify".to_string(),
            recommended_playbooks: vec!["SCCT_response_matrix".to_string()],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudienceType, StakeType, UrgencyLevel};

    #[tokio::test]
    async fn test_taxonomy_linker_mock() {
        let agent = TaxonomyLinkerAgent::new(TaxonomyLinkerConfig::default());
        let narrative = "Test narrative";
        let context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        };

        let result = agent.link_taxonomy_mock(narrative, &context).await.unwrap();

        assert_eq!(result.primary_function, "issue_crisis_management");
        assert_eq!(result.lifecycle_stage, "assess_and_classify");
        assert!(!result.subfunctions.is_empty());
    }
}
