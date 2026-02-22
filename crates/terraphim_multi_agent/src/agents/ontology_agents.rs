//! Specialized Agents for Dynamic Ontology
//!
//! This module contains specialized agents for the Dynamic Ontology pipeline:
//! - Extraction Agent: Extracts entities and relationships from text
//! - Normalization Agent: Grounds entities to ontology using graph + fuzzy matching
//! - Coverage Agent: Computes coverage governance signal
//! - Review Agent: Improves low-confidence normalizations

use crate::{GenAiLlmClient, LlmMessage, LlmRequest, MultiAgentResult, ProviderConfig};
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_types::{CoverageSignal, ExtractedEntity, GroundingMetadata, SchemaSignal};
use tokio::sync::RwLock;

/// Configuration for ontology agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyAgentConfig {
    /// LLM provider to use
    pub provider: String,
    /// Model to use
    pub model: String,
    /// Temperature for LLM calls
    pub temperature: f32,
    /// Maximum tokens in response
    pub max_tokens: u32,
}

impl Default for OntologyAgentConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            model: "gemma3:270m".to_string(),
            temperature: 0.3,
            max_tokens: 2000,
        }
    }
}

/// Extraction Agent: Extracts entities and relationships from text
pub struct ExtractionAgent {
    llm_client: Arc<RwLock<GenAiLlmClient>>,
    config: OntologyAgentConfig,
}

impl ExtractionAgent {
    /// Create a new Extraction Agent
    pub fn new(config: OntologyAgentConfig) -> MultiAgentResult<Self> {
        let llm_client = GenAiLlmClient::new(
            config.provider.clone(),
            ProviderConfig {
                model: config.model.clone(),
            },
        )?;

        Ok(Self {
            llm_client: Arc::new(RwLock::new(llm_client)),
            config,
        })
    }

    /// Extract entities and relationships from text
    pub async fn extract(&self, text: &str) -> MultiAgentResult<SchemaSignal> {
        let client = self.llm_client.write().await;

        let extraction_prompt = format!(
            r#"You are an Extraction Agent for Dynamic Ontology.
Extract entities and relationships from the given text.

Extract entities with the following types:
- CancerDiagnosis: Cancer diagnosis terms
- Tumor: Tumor-related terms
- GenomicVariant: Genetic variants
- Biomarker: Biomarker terms
- Drug: Medication names
- Treatment: Treatment procedures
- SideEffect: Adverse effects

Extract relationships between entities using:
- HasTumor, HasVariant, HasBiomarker, TreatedWith, Causes, HasDiagnosis

Respond with ONLY valid JSON in this format:
{{
  "entities": [
    {{"entity_type": "CancerDiagnosis", "raw_value": "lung carcinoma"}}
  ],
  "relationships": [
    {{"relationship_type": "HasTumor", "source": "lung carcinoma", "target": "tumor", "confidence": 0.9}}
  ],
  "confidence": 0.85
}}

Text to analyze:
{}"#,
            text
        );

        let request = LlmRequest::new(vec![
            LlmMessage::system(
                "You extract entities and relationships for ontology building.".to_string(),
            ),
            LlmMessage::user(extraction_prompt),
        ])
        .with_temperature(self.config.temperature)
        .with_max_tokens(self.config.max_tokens.into());

        let response = client.generate(request).await?;

        let schema_signal: SchemaSignal = serde_json::from_str(&response.content).map_err(|e| {
            crate::MultiAgentError::LlmError(format!("Failed to parse extraction response: {}", e))
        })?;

        info!(
            "Extracted {} entities and {} relationships",
            schema_signal.entities.len(),
            schema_signal.relationships.len()
        );

        Ok(schema_signal)
    }
}

/// Normalization Agent: Grounds entities to ontology
pub struct NormalizationAgent {
    llm_client: Arc<RwLock<GenAiLlmClient>>,
    config: OntologyAgentConfig,
    /// Available ontology terms
    ontology_terms: Vec<String>,
}

impl NormalizationAgent {
    /// Create a new Normalization Agent
    pub fn new(config: OntologyAgentConfig, ontology_terms: Vec<String>) -> MultiAgentResult<Self> {
        let llm_client = GenAiLlmClient::new(
            config.provider.clone(),
            ProviderConfig {
                model: config.model.clone(),
            },
        )?;

        Ok(Self {
            llm_client: Arc::new(RwLock::new(llm_client)),
            config,
            ontology_terms,
        })
    }

    /// Normalize entities to ontology
    pub async fn normalize(
        &self,
        entities: Vec<ExtractedEntity>,
    ) -> MultiAgentResult<Vec<ExtractedEntity>> {
        let client = self.llm_client.write().await;
        let ontology_list = self.ontology_terms.join(", ");

        let mut grounded_entities = Vec::new();

        for entity in entities {
            let normalization_prompt = format!(
                r#"You are a Normalization Agent for Dynamic Ontology.
Your task is to normalize extracted entities to the ontology.

Available ontology terms:
{}

Extract: {}
Type: {}

Respond with ONLY valid JSON:
{{
  "normalized_uri": "http://example.org/term",
  "normalized_label": "canonical term",
  "normalized_prov": "ontology_source",
  "normalized_score": 0.95,
  "normalized_method": "exact|fuzzy|graph_rank"
}}

If no match found, respond with:
{{
  "normalized_uri": null,
  "normalized_label": null,
  "normalized_prov": null,
  "normalized_score": null,
  "normalized_method": null
}}"#,
                ontology_list,
                entity.raw_value,
                format!("{:?}", entity.entity_type)
            );

            let request = LlmRequest::new(vec![
                LlmMessage::system("You normalize entities to ontology terms.".to_string()),
                LlmMessage::user(normalization_prompt),
            ])
            .with_temperature(self.config.temperature)
            .with_max_tokens(500);

            let response = client.generate(request).await?;

            let grounding: Option<GroundingMetadata> = serde_json::from_str(&response.content).ok();

            let mut normalized_entity = entity;
            normalized_entity.grounding = grounding.clone();

            if let Some(ref g) = normalized_entity.grounding {
                if let Some(ref label) = g.normalized_label {
                    normalized_entity.normalized_value = Some(label.clone());
                }
            }

            grounded_entities.push(normalized_entity);
        }

        info!("Normalized {} entities", grounded_entities.len());

        Ok(grounded_entities)
    }
}

/// Coverage Agent: Computes coverage governance signal
pub struct CoverageAgent {
    threshold: f32,
}

impl CoverageAgent {
    /// Create a new Coverage Agent
    pub fn new(threshold: f32) -> Self {
        Self { threshold }
    }

    /// Compute coverage signal
    pub fn compute_coverage(&self, entities: &[ExtractedEntity]) -> CoverageSignal {
        let categories: Vec<String> = entities
            .iter()
            .map(|e| e.normalized_value.clone().unwrap_or(e.raw_value.clone()))
            .collect();

        let matched = entities.iter().filter(|e| e.grounding.is_some()).count();

        CoverageSignal::compute(&categories, matched, self.threshold)
    }
}

/// Review Agent: Improves low-confidence normalizations
pub struct ReviewAgent {
    llm_client: Arc<RwLock<GenAiLlmClient>>,
    config: OntologyAgentConfig,
    /// Available ontology terms
    ontology_terms: Vec<String>,
    /// Minimum confidence threshold
    min_confidence: f32,
}

impl ReviewAgent {
    /// Create a new Review Agent
    pub fn new(
        config: OntologyAgentConfig,
        ontology_terms: Vec<String>,
        min_confidence: f32,
    ) -> MultiAgentResult<Self> {
        let llm_client = GenAiLlmClient::new(
            config.provider.clone(),
            ProviderConfig {
                model: config.model.clone(),
            },
        )?;

        Ok(Self {
            llm_client: Arc::new(RwLock::new(llm_client)),
            config,
            ontology_terms,
            min_confidence,
        })
    }

    /// Review and improve low-confidence normalizations
    pub async fn review(
        &self,
        entities: &mut Vec<ExtractedEntity>,
    ) -> MultiAgentResult<Vec<ExtractedEntity>> {
        let client = self.llm_client.write().await;

        // Find entities with low confidence
        let low_confidence: Vec<_> = entities
            .iter()
            .filter(|e| {
                e.grounding
                    .as_ref()
                    .map(|g| g.normalized_score.unwrap_or(0.0) < self.min_confidence)
                    .unwrap_or(true)
            })
            .collect();

        if low_confidence.is_empty() {
            return Ok(entities.clone());
        }

        let review_prompt = format!(
            r#"You are a Review Agent for Dynamic Ontology.
Review low-confidence normalizations and suggest improvements.

Low-confidence entities:
{}

Available ontology terms:
{}

For each entity, suggest better matches with:
1. Possible ontology matches
2. Alternative search terms
3. Whether the entity should be added to the ontology

Respond with ONLY valid JSON array:
[
  {{
    "original": "entity_value",
    "suggested_uri": "http://example.org/suggested",
    "suggested_label": "Suggested Term",
    "confidence": 0.8,
    "reason": "explanation"
  }}
]"#,
            low_confidence
                .iter()
                .map(|e| format!("{} ({:?})", e.raw_value, e.entity_type))
                .collect::<Vec<_>>()
                .join(", "),
            self.ontology_terms.join(", ")
        );

        let request = LlmRequest::new(vec![
            LlmMessage::system(
                "You improve ontology coverage by suggesting better matches.".to_string(),
            ),
            LlmMessage::user(review_prompt),
        ])
        .with_temperature(self.config.temperature)
        .with_max_tokens(1000);

        let response = client.generate(request).await?;

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ReviewSuggestion {
            original: String,
            suggested_uri: Option<String>,
            suggested_label: Option<String>,
            confidence: f32,
            reason: Option<String>,
        }

        if let Ok(suggestions) = serde_json::from_str::<Vec<ReviewSuggestion>>(&response.content) {
            for suggestion in suggestions {
                if let Some(entity) = entities
                    .iter_mut()
                    .find(|e| e.raw_value == suggestion.original)
                {
                    if let (Some(uri), Some(label)) =
                        (suggestion.suggested_uri, suggestion.suggested_label)
                    {
                        entity.grounding = Some(GroundingMetadata::new(
                            uri,
                            label.clone(),
                            "review".to_string(),
                            suggestion.confidence,
                            terraphim_types::NormalizationMethod::Fuzzy,
                        ));
                        entity.normalized_value = Some(label);
                    }
                }
            }
        }

        info!("Review complete for {} entities", entities.len());

        Ok(entities.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_agent_default_threshold() {
        let agent = CoverageAgent::new(0.7);
        let entities = vec![
            ExtractedEntity {
                entity_type: "cancer_diagnosis".to_string(),
                raw_value: "lung carcinoma".to_string(),
                normalized_value: Some("lung carcinoma".to_string()),
                grounding: Some(GroundingMetadata::new(
                    "http://example.org/lung_carcinoma".to_string(),
                    "Lung Carcinoma".to_string(),
                    "NCIt".to_string(),
                    0.95,
                    terraphim_types::NormalizationMethod::Exact,
                )),
            },
            ExtractedEntity {
                entity_type: "genomic_variant".to_string(),
                raw_value: "EGFR".to_string(),
                normalized_value: None,
                grounding: None,
            },
        ];

        let coverage = agent.compute_coverage(&entities);
        assert_eq!(coverage.total_categories, 2);
        assert_eq!(coverage.matched_categories, 1);
        assert!((coverage.coverage_ratio - 0.5).abs() < 0.01);
        assert!(coverage.needs_review);
    }

    #[test]
    fn test_extraction_agent_config_default() {
        let config = OntologyAgentConfig::default();
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.temperature, 0.3);
    }
}
