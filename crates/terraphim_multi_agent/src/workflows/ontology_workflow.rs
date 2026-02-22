//! Dynamic Ontology Multi-Agent Workflow
//!
//! This module implements a LeadWithSpecialists workflow for Dynamic Ontology:
//! 1. Extraction Agent - Extract entities and relationships from text
//! 2. Normalization Agent - Normalize and ground entities to ontology
//! 3. Coverage Agent - Check ontology coverage of extracted entities
//! 4. Review Agent - Review and improve coverage when needed
//!
//! The workflow returns a grounded knowledge graph with normalized entities.

use crate::{
    GenAiLlmClient, LlmMessage, LlmRequest, MultiAgentError, MultiAgentResult, ProviderConfig,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use terraphim_types::{CoverageSignal, ExtractedEntity, GroundingMetadata, SchemaSignal};
use tokio::sync::RwLock;

/// Result of the Dynamic Ontology workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyWorkflowResult {
    /// The extracted schema signal with entities and relationships
    pub schema_signal: SchemaSignal,
    /// Coverage signal showing ontology matching
    pub coverage: CoverageSignal,
    /// Final grounded entities after normalization
    pub grounded_entities: Vec<ExtractedEntity>,
    /// Workflow execution metadata
    pub metadata: OntologyWorkflowMetadata,
}

/// Metadata about workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyWorkflowMetadata {
    /// Number of extraction iterations
    pub extraction_iterations: usize,
    /// Number of normalization iterations
    pub normalization_iterations: usize,
    /// Whether review was triggered
    pub review_triggered: bool,
    /// Total LLM tokens used
    pub total_tokens: u64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Agent role in the ontology workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OntologyAgentRole {
    /// Lead agent that orchestrates the workflow
    Lead,
    /// Extraction specialist - extracts entities from text
    Extraction,
    /// Normalization specialist - grounds entities to ontology
    Normalization,
    /// Coverage specialist - checks ontology coverage
    Coverage,
    /// Review specialist - improves low coverage
    Review,
}

impl std::fmt::Display for OntologyAgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OntologyAgentRole::Lead => write!(f, "Lead"),
            OntologyAgentRole::Extraction => write!(f, "Extraction"),
            OntologyAgentRole::Normalization => write!(f, "Normalization"),
            OntologyAgentRole::Coverage => write!(f, "Coverage"),
            OntologyAgentRole::Review => write!(f, "Review"),
        }
    }
}

/// Configuration for the Dynamic Ontology workflow
#[derive(Debug, Clone)]
pub struct OntologyWorkflowConfig {
    /// LLM provider to use
    pub provider: String,
    /// Model to use for extraction
    pub extraction_model: String,
    /// Model to use for normalization
    pub normalization_model: String,
    /// Model to use for coverage checking
    pub coverage_model: String,
    /// Model to use for review
    pub review_model: String,
    /// Coverage threshold - below this triggers review
    pub coverage_threshold: f32,
    /// Maximum workflow iterations
    pub max_iterations: usize,
    /// Temperature for LLM calls
    pub temperature: f32,
}

impl Default for OntologyWorkflowConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            extraction_model: "gemma3:270m".to_string(),
            normalization_model: "gemma3:270m".to_string(),
            coverage_model: "gemma3:270m".to_string(),
            review_model: "gemma3:270m".to_string(),
            coverage_threshold: 0.7,
            max_iterations: 3,
            temperature: 0.3,
        }
    }
}

/// Dynamic Ontology Workflow using LeadWithSpecialists pattern
pub struct OntologyWorkflow {
    config: OntologyWorkflowConfig,
    llm_client: Arc<RwLock<GenAiLlmClient>>,
    /// Available ontology terms for coverage checking
    ontology_terms: Vec<String>,
}

impl OntologyWorkflow {
    /// Create a new ontology workflow
    pub fn new(
        config: OntologyWorkflowConfig,
        ontology_terms: Vec<String>,
    ) -> MultiAgentResult<Self> {
        let llm_client = GenAiLlmClient::new(
            config.provider.clone(),
            ProviderConfig {
                model: config.extraction_model.clone(),
            },
        )?;

        Ok(Self {
            config,
            llm_client: Arc::new(RwLock::new(llm_client)),
            ontology_terms,
        })
    }

    /// Create with default configuration
    pub fn with_defaults(ontology_terms: Vec<String>) -> MultiAgentResult<Self> {
        Self::new(OntologyWorkflowConfig::default(), ontology_terms)
    }

    /// Execute the full Dynamic Ontology workflow
    pub async fn execute(&self, text: &str) -> MultiAgentResult<OntologyWorkflowResult> {
        let start_time = std::time::Instant::now();
        let total_tokens = 0u64;
        let extraction_iterations;
        let mut normalization_iterations;
        let mut review_triggered = false;

        // Step 1: Lead agent routes to Extraction Agent
        log::info!("Starting Dynamic Ontology workflow with LeadWithSpecialists pattern");

        // Step 2: Extraction Agent extracts entities and relationships
        let schema_signal = self.extract_entities(text).await?;
        extraction_iterations = 1;
        log::info!(
            "Extracted {} entities and {} relationships",
            schema_signal.entities.len(),
            schema_signal.relationships.len()
        );

        // Step 3: Normalization Agent grounds entities to ontology
        let mut grounded_entities = self
            .normalize_entities(schema_signal.entities.clone())
            .await?;
        normalization_iterations = 1;
        log::info!("Normalized {} entities", grounded_entities.len());

        // Step 4: Coverage Agent checks ontology coverage
        let coverage = self.check_coverage(&grounded_entities).await?;
        log::info!(
            "Coverage: {}/{} = {:.2}% (threshold: {:.2}%)",
            coverage.matched_categories,
            coverage.total_categories,
            coverage.coverage_ratio * 100.0,
            coverage.threshold * 100.0
        );

        // Step 5: If low coverage, route to Review Agent
        if coverage.needs_review && self.config.max_iterations > 1 {
            review_triggered = true;
            log::info!("Low coverage detected, routing to Review Agent");

            let reviewed = self
                .review_and_improve(&mut grounded_entities, &coverage)
                .await?;
            grounded_entities = reviewed;
            normalization_iterations += 1;
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Build final schema signal with grounded entities
        let final_schema_signal = SchemaSignal {
            entities: grounded_entities.clone(),
            relationships: schema_signal.relationships,
            confidence: coverage.coverage_ratio,
        };

        Ok(OntologyWorkflowResult {
            schema_signal: final_schema_signal,
            coverage,
            grounded_entities,
            metadata: OntologyWorkflowMetadata {
                extraction_iterations,
                normalization_iterations,
                review_triggered,
                total_tokens,
                execution_time_ms,
            },
        })
    }

    /// Extraction Agent: Extract entities and relationships from text
    async fn extract_entities(&self, text: &str) -> MultiAgentResult<SchemaSignal> {
        let mut client = self.llm_client.write().await;

        // Update model for extraction
        client.set_model(self.config.extraction_model.clone());

        let extraction_prompt = format!(
            r#"You are an Extraction Agent for Dynamic Ontology.
Your task is to extract entities and relationships from the given text.

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
        .with_max_tokens(2000);

        let response = client.generate(request).await?;

        // Parse the JSON response
        let schema_signal: SchemaSignal = serde_json::from_str(&response.content).map_err(|e| {
            MultiAgentError::LlmError(format!("Failed to parse extraction response: {}", e))
        })?;

        Ok(schema_signal)
    }

    /// Normalization Agent: Ground entities to ontology
    async fn normalize_entities(
        &self,
        entities: Vec<ExtractedEntity>,
    ) -> MultiAgentResult<Vec<ExtractedEntity>> {
        let mut client = self.llm_client.write().await;
        client.set_model(self.config.normalization_model.clone());

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

            // Parse grounding metadata
            let grounding: Option<GroundingMetadata> = serde_json::from_str(&response.content).ok();

            let mut normalized_entity = entity;
            normalized_entity.grounding = grounding;

            // Also set normalized value if we have a label
            if let Some(ref g) = normalized_entity.grounding {
                if let Some(ref label) = g.normalized_label {
                    normalized_entity.normalized_value = Some(label.clone());
                }
            }

            grounded_entities.push(normalized_entity);
        }

        Ok(grounded_entities)
    }

    /// Coverage Agent: Check ontology coverage
    async fn check_coverage(
        &self,
        entities: &[ExtractedEntity],
    ) -> MultiAgentResult<CoverageSignal> {
        // Extract unique entity categories (types + raw values)
        let categories: Vec<String> = entities
            .iter()
            .map(|e| e.normalized_value.clone().unwrap_or(e.raw_value.clone()))
            .collect();

        // Count matched entities (those with grounding)
        let matched = entities.iter().filter(|e| e.grounding.is_some()).count();

        // Calculate coverage
        let coverage =
            CoverageSignal::compute(&categories, matched, self.config.coverage_threshold);

        Ok(coverage)
    }

    /// Review Agent: Review and improve low coverage
    async fn review_and_improve(
        &self,
        entities: &mut Vec<ExtractedEntity>,
        _coverage: &CoverageSignal,
    ) -> MultiAgentResult<Vec<ExtractedEntity>> {
        let mut client = self.llm_client.write().await;
        client.set_model(self.config.review_model.clone());

        // Find unmatched entities
        let unmatched: Vec<_> = entities.iter().filter(|e| e.grounding.is_none()).collect();

        let review_prompt = format!(
            r#"You are a Review Agent for Dynamic Ontology.
Your task is to improve coverage for unmatched entities by suggesting better matches.

Unmatched entities:
{}

Available ontology terms:
{}

For each unmatched entity, suggest:
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
            unmatched
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

        // Try to parse suggestions and update entities
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

        Ok(entities.clone())
    }
}

/// Execute workflow using MultiAgentWorkflow::LeadWithSpecialists pattern
pub async fn execute_ontology_workflow(
    text: &str,
    provider: &str,
    model: &str,
    ontology_terms: Vec<String>,
) -> MultiAgentResult<OntologyWorkflowResult> {
    let config = OntologyWorkflowConfig {
        provider: provider.to_string(),
        extraction_model: model.to_string(),
        normalization_model: model.to_string(),
        coverage_model: model.to_string(),
        review_model: model.to_string(),
        coverage_threshold: 0.7,
        max_iterations: 3,
        temperature: 0.3,
    };

    let workflow = OntologyWorkflow::new(config, ontology_terms)?;
    workflow.execute(text).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontology_workflow_config_default() {
        let config = OntologyWorkflowConfig::default();
        assert_eq!(config.provider, "ollama");
        assert_eq!(config.coverage_threshold, 0.7);
    }

    #[test]
    fn test_coverage_signal_computation() {
        let categories = vec![
            "lung carcinoma".to_string(),
            "EGFR mutation".to_string(),
            "Osimertinib".to_string(),
        ];
        let coverage = CoverageSignal::compute(&categories, 2, 0.7);
        assert_eq!(coverage.total_categories, 3);
        assert_eq!(coverage.matched_categories, 2);
        assert!((coverage.coverage_ratio - 0.667).abs() < 0.01);
        assert!(coverage.needs_review);
    }

    #[test]
    fn test_coverage_above_threshold() {
        let categories = vec!["term1".to_string(), "term2".to_string()];
        let coverage = CoverageSignal::compute(&categories, 2, 0.5);
        assert!(!coverage.needs_review);
        assert_eq!(coverage.coverage_ratio, 1.0);
    }

    #[tokio::test]
    async fn test_ontology_workflow_creation() {
        let terms = vec!["lung carcinoma".to_string(), "EGFR".to_string()];
        let workflow = OntologyWorkflow::with_defaults(terms.clone());
        assert!(workflow.is_ok());
    }
}
