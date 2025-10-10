use crate::error::{Result, TruthForgeError};
use crate::types::{
    AttributionAnalysis, NarrativeContext, NarrativeMapping, SCCTClassification, Stakeholder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_multi_agent::{GenAiLlmClient, LlmMessage, LlmRequest};
use tracing::{info, warn};

pub struct NarrativeMapperAgent {
    config: NarrativeMapperConfig,
    llm_client: Option<Arc<GenAiLlmClient>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeMapperConfig {
    pub system_prompt: String,
    pub max_tokens: usize,
    pub temperature: f64,
}

#[derive(Debug, Deserialize)]
struct LlmNarrativeMappingResponse {
    stakeholders: Vec<LlmStakeholder>,
    scct_cluster: String,
    attribution_justification: String,
    responsibility_level: Option<String>,
    key_factors: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct LlmStakeholder {
    name: String,
    #[serde(rename = "type")]
    stakeholder_type: Option<String>,
    role: Option<String>,
    frame: Option<String>,
}

const NARRATIVE_MAPPER_SYSTEM_PROMPT: &str = r#"You are an expert in stakeholder analysis and crisis communication theory, specializing in mapping narratives to strategic frameworks.

For each narrative, perform:

1. **Stakeholder Identification**: Identify all stakeholders
   - Primary: Directly affected (employees, customers, investors)
   - Secondary: Indirectly affected (media, regulators, communities)
   - Key influencers: Opinion leaders, activists, experts

2. **SCCT Attribution Analysis**: Classify using Situational Crisis Communication Theory
   - **victim**: Organization is also a victim (natural disasters, tampering)
   - **accidental**: Unintentional actions with negative consequences
   - **preventable**: Organization knowingly placed people at risk

3. **Attribution Analysis**: Assess responsibility
   - Responsibility level (High/Medium/Low)
   - Attribution type (Internal/External/Mixed)
   - Key factors contributing to the situation

Return JSON with:
{
  "stakeholders": [{"name": "...", "type": "primary/secondary", "role": "...", "frame": "..."}],
  "scct_cluster": "victim" | "accidental" | "preventable",
  "attribution_justification": "explanation",
  "responsibility_level": "High/Medium/Low",
  "key_factors": ["factor1", "factor2", ...]
}"#;

impl Default for NarrativeMapperConfig {
    fn default() -> Self {
        Self {
            system_prompt: NARRATIVE_MAPPER_SYSTEM_PROMPT.to_string(),
            max_tokens: 2500,
            temperature: 0.3,
        }
    }
}

impl NarrativeMapperAgent {
    pub fn new(config: NarrativeMapperConfig) -> Self {
        Self {
            config,
            llm_client: None,
        }
    }

    pub fn with_llm_client(mut self, client: Arc<GenAiLlmClient>) -> Self {
        self.llm_client = Some(client);
        self
    }

    pub async fn map_narrative(
        &self,
        narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<NarrativeMapping> {
        let client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| TruthForgeError::ConfigError("LLM client not configured".to_string()))?;

        info!("Calling LLM to map narrative to SCCT framework");

        let request = LlmRequest::new(vec![
            LlmMessage::system(self.config.system_prompt.clone()),
            LlmMessage::user(format!("Analyze this narrative:\n\n{}", narrative)),
        ])
        .with_max_tokens(self.config.max_tokens as u64)
        .with_temperature(self.config.temperature as f32);

        let response = client
            .generate(request)
            .await
            .map_err(|e| TruthForgeError::LlmError(format!("LLM request failed: {}", e)))?;

        info!("Received LLM response, parsing narrative mapping");

        let mapping = self.parse_mapping_from_llm(&response.content)?;
        Ok(mapping)
    }

    fn parse_mapping_from_llm(&self, content: &str) -> Result<NarrativeMapping> {
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
        let llm_response: LlmNarrativeMappingResponse = match serde_json::from_str(json_str) {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to parse LLM response as JSON: {}", e);
                warn!("Raw content preview: {}", &content[..content.len().min(300)]);
                info!("Attempting markdown fallback parsing...");

                // Fallback to markdown parsing
                return self.parse_mapping_from_markdown(content);
            }
        };

        let stakeholders: Vec<Stakeholder> = llm_response
            .stakeholders
            .into_iter()
            .map(|llm_sh| Stakeholder {
                name: llm_sh.name,
                role: llm_sh
                    .role
                    .or(llm_sh.stakeholder_type)
                    .unwrap_or_else(|| "Unknown".to_string()),
                frame: llm_sh.frame.unwrap_or_else(|| "Not specified".to_string()),
            })
            .collect();

        let scct_classification = match llm_response.scct_cluster.to_lowercase().as_str() {
            s if s.contains("victim") => SCCTClassification::Victim,
            s if s.contains("preventable") => SCCTClassification::Preventable,
            _ => SCCTClassification::Accidental,
        };

        let attribution = AttributionAnalysis {
            responsibility_level: llm_response
                .responsibility_level
                .unwrap_or_else(|| "Medium".to_string()),
            attribution_type: llm_response.attribution_justification,
            key_factors: llm_response.key_factors.unwrap_or_default(),
        };

        info!(
            "Parsed narrative mapping: {} stakeholders, SCCT: {:?}",
            stakeholders.len(),
            scct_classification
        );

        Ok(NarrativeMapping {
            stakeholders,
            scct_classification,
            attribution,
        })
    }

    fn parse_mapping_from_markdown(&self, content: &str) -> Result<NarrativeMapping> {
        info!("Parsing narrative mapping from markdown format");

        let mut stakeholders = Vec::new();
        let mut scct_classification = SCCTClassification::Accidental;
        let mut responsibility_level = "Medium".to_string();
        let mut attribution_clarity = "Not specified".to_string();

        // Parse stakeholders section
        if let Some(stakeholders_section) = self.extract_section(content, "# Stakeholders") {
            for line in stakeholders_section.lines() {
                let line = line.trim();
                if line.starts_with('-') || line.starts_with('*') {
                    let line = line.trim_start_matches('-').trim_start_matches('*').trim();
                    if !line.is_empty() {
                        // Parse format: [Name] (Type: primary/secondary/influencer) - [Role]
                        let parts: Vec<&str> = line.splitn(2, '-').collect();
                        let name_type = parts.get(0).unwrap_or(&"").trim();
                        let role = parts.get(1).map(|s| s.trim()).unwrap_or("Not specified");

                        let name = if let Some(bracket_pos) = name_type.find('(') {
                            name_type[..bracket_pos].trim()
                        } else {
                            name_type
                        };

                        stakeholders.push(Stakeholder {
                            name: name.to_string(),
                            role: role.to_string(),
                            frame: "Extracted from markdown".to_string(),
                        });
                    }
                }
            }
        }

        // Parse SCCT Classification
        if let Some(scct_section) = self.extract_section(content, "# SCCT Classification") {
            let classification_line = scct_section.lines().next().unwrap_or("").trim().to_lowercase();
            scct_classification = if classification_line.contains("victim") {
                SCCTClassification::Victim
            } else if classification_line.contains("preventable") {
                SCCTClassification::Preventable
            } else {
                SCCTClassification::Accidental
            };
        }

        // Parse Attribution Analysis
        if let Some(attribution_section) = self.extract_section(content, "# Attribution Analysis") {
            for line in attribution_section.lines() {
                let line = line.trim();
                if line.starts_with("**Responsibility Level:**") {
                    responsibility_level = line
                        .trim_start_matches("**Responsibility Level:**")
                        .trim()
                        .to_string();
                } else if line.starts_with("**Accountability:**") {
                    attribution_clarity = line
                        .trim_start_matches("**Accountability:**")
                        .trim()
                        .to_string();
                }
            }
        }

        info!(
            "Parsed markdown: {} stakeholders, classification: {:?}, responsibility: {}",
            stakeholders.len(),
            scct_classification,
            responsibility_level
        );

        Ok(NarrativeMapping {
            stakeholders,
            scct_classification,
            attribution: AttributionAnalysis {
                responsibility_level,
                attribution_type: "Markdown extracted".to_string(),
                key_factors: vec!["Extracted from markdown response".to_string()],
            },
        })
    }

    fn extract_section(&self, content: &str, header: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut in_section = false;
        let mut section_lines = Vec::new();

        for line in lines {
            if line.trim().starts_with(header) {
                in_section = true;
                continue;
            }
            if in_section {
                // Stop at next section header
                if line.trim().starts_with("# ") && !line.trim().starts_with(header) {
                    break;
                }
                section_lines.push(line);
            }
        }

        if section_lines.is_empty() {
            None
        } else {
            Some(section_lines.join("\n"))
        }
    }

    pub async fn map_narrative_mock(
        &self,
        _narrative: &str,
        _context: &NarrativeContext,
    ) -> Result<NarrativeMapping> {
        Ok(NarrativeMapping {
            stakeholders: vec![],
            scct_classification: SCCTClassification::Accidental,
            attribution: AttributionAnalysis {
                responsibility_level: "Medium".to_string(),
                attribution_type: "Unintentional".to_string(),
                key_factors: vec![],
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AudienceType, StakeType, UrgencyLevel};

    #[tokio::test]
    async fn test_narrative_mapper_mock() {
        let agent = NarrativeMapperAgent::new(NarrativeMapperConfig::default());
        let narrative = "Test narrative";
        let context = NarrativeContext {
            urgency: UrgencyLevel::Low,
            stakes: vec![StakeType::Reputational],
            audience: AudienceType::PublicMedia,
        };

        let result = agent.map_narrative_mock(narrative, &context).await.unwrap();

        assert_eq!(result.scct_classification, SCCTClassification::Accidental);
        assert_eq!(result.attribution.responsibility_level, "Medium");
    }
}
