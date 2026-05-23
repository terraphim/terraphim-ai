#[cfg(feature = "llm")]
use crate::signatures::{ConceptExtractionSignature, RlmSignature};
#[cfg(feature = "llm")]
use std::sync::Arc;
#[cfg(feature = "llm")]
use terraphim_service::llm::LlmClient;

use crate::error::Result;
use crate::signatures::NewConcept;

#[cfg(feature = "llm")]
pub struct KgCurationRlm {
    llm_client: Arc<dyn LlmClient>,
}

#[cfg(feature = "llm")]
impl KgCurationRlm {
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self { llm_client }
    }

    pub async fn extract_and_index(
        &self,
        query: &str,
        rlm_answer: &str,
    ) -> Result<Vec<NewConcept>> {
        use crate::error::TerraphimGrepError;

        let prompt = format!(
            "Extract new concepts from this interaction:\n\n\
            Query: {}\n\
            Answer: {}\n\n\
            {}\n\n\
            Output JSON only.",
            query,
            rlm_answer,
            ConceptExtractionSignature {}.instructions()
        );

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": prompt
        })];

        let response = self
            .llm_client
            .chat_completion(
                messages,
                terraphim_service::llm::ChatOptions {
                    max_tokens: Some(1000),
                    temperature: Some(0.3),
                },
            )
            .await
            .map_err(|e| TerraphimGrepError::RlmFailed(e.to_string()))?;

        let signature = ConceptExtractionSignature {};
        let concepts = signature.parse(&response)?;

        Ok(concepts)
    }
}

#[cfg(not(feature = "llm"))]
pub struct KgCurationRlm;

#[cfg(not(feature = "llm"))]
impl Default for KgCurationRlm {
    fn default() -> Self {
        Self
    }
}

#[cfg(not(feature = "llm"))]
impl KgCurationRlm {
    pub fn new() -> Self {
        Self
    }

    pub async fn extract_and_index(
        &self,
        _query: &str,
        _rlm_answer: &str,
    ) -> Result<Vec<NewConcept>> {
        use crate::error::TerraphimGrepError;
        Err(TerraphimGrepError::LlmNotConfigured(
            "LLM feature not enabled".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "llm"))]
    use super::*;

    #[cfg(not(feature = "llm"))]
    #[test]
    fn test_kg_curation_new() {
        let _curation = KgCurationRlm::new();
    }
}
