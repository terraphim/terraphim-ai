#[cfg(feature = "llm")]
use crate::signatures::{ConceptExtractionSignature, RlmSignature};
#[cfg(feature = "llm")]
use std::sync::Arc;
#[cfg(feature = "llm")]
use terraphim_service::llm::LlmClient;

use crate::error::Result;
use crate::signatures::NewConcept;

/// Represents the RLM-based knowledge-graph curation pipeline (requires the `llm` feature).
#[cfg(feature = "llm")]
pub struct KgCurationRlm {
    llm_client: Arc<dyn LlmClient>,
    kg_path: Option<std::path::PathBuf>,
}

#[cfg(feature = "llm")]
impl KgCurationRlm {
    /// Builds a new `KgCurationRlm` backed by the supplied LLM client.
    pub fn new(llm_client: Arc<dyn LlmClient>) -> Self {
        Self {
            llm_client,
            kg_path: None,
        }
    }

    /// Sets the filesystem path where extracted concept markdown files are persisted.
    pub fn with_kg_path(mut self, path: std::path::PathBuf) -> Self {
        self.kg_path = Some(path);
        self
    }

    /// Extracts new KG concepts from the query and RLM answer, then persists them to disk.
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

        if let Some(ref kg_path) = self.kg_path {
            self.persist_concepts(&concepts, query, kg_path);
        }

        Ok(concepts)
    }

    fn persist_concepts(
        &self,
        concepts: &[NewConcept],
        source_query: &str,
        kg_path: &std::path::Path,
    ) {
        if concepts.is_empty() {
            return;
        }
        if let Err(e) = std::fs::create_dir_all(kg_path) {
            log::warn!("Failed to create KG directory {:?}: {}", kg_path, e);
            return;
        }
        for concept in concepts {
            let slug = concept
                .name
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
            let filename = format!("learned-{}.md", slug);
            let filepath = kg_path.join(&filename);

            if filepath.exists() {
                continue;
            }

            let synonyms_line = if concept.synonyms.is_empty() {
                String::new()
            } else {
                format!("\nsynonyms:: {}", concept.synonyms.join(", "))
            };

            let relationships_line = if concept.relationships.is_empty() {
                String::new()
            } else {
                format!("\nrelated:: {}", concept.relationships.join(", "))
            };

            let content = format!(
                "# {}\n\nDiscovered during search: \"{}\"{}{}\n",
                concept.name, source_query, synonyms_line, relationships_line
            );

            if let Err(e) = std::fs::write(&filepath, &content) {
                log::warn!("Failed to write KG file {:?}: {}", filepath, e);
            } else {
                log::info!("Learned new KG concept: {} -> {:?}", concept.name, filepath);
            }
        }
    }
}

/// Represents a stub `KgCurationRlm` used when the `llm` feature is disabled.
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
    /// Builds a new no-op `KgCurationRlm` stub when the `llm` feature is disabled.
    pub fn new() -> Self {
        Self
    }

    /// Returns an error indicating that the `llm` feature is not enabled.
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
    use super::*;
    use std::fs;

    fn persist_concepts(concepts: &[NewConcept], source_query: &str, kg_path: &std::path::Path) {
        if concepts.is_empty() {
            return;
        }
        if let Err(e) = std::fs::create_dir_all(kg_path) {
            log::warn!("Failed to create KG directory {:?}: {}", kg_path, e);
            return;
        }
        for concept in concepts {
            let slug = concept
                .name
                .to_lowercase()
                .replace(|c: char| !c.is_alphanumeric() && c != '-', "-");
            let filename = format!("learned-{}.md", slug);
            let filepath = kg_path.join(&filename);

            if filepath.exists() {
                continue;
            }

            let synonyms_line = if concept.synonyms.is_empty() {
                String::new()
            } else {
                format!("\nsynonyms:: {}", concept.synonyms.join(", "))
            };

            let relationships_line = if concept.relationships.is_empty() {
                String::new()
            } else {
                format!("\nrelated:: {}", concept.relationships.join(", "))
            };

            let content = format!(
                "# {}\n\nDiscovered during search: \"{}\"{}{}\n",
                concept.name, source_query, synonyms_line, relationships_line
            );

            if let Err(e) = std::fs::write(&filepath, &content) {
                log::warn!("Failed to write KG file {:?}: {}", filepath, e);
            }
        }
    }

    #[test]
    fn test_persist_concepts_writes_markdown_files() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let kg_path = tmp.path().to_path_buf();

        let concepts = vec![
            NewConcept {
                name: "Retry Policy".to_string(),
                synonyms: vec!["backoff".to_string(), "retry configuration".to_string()],
                relationships: vec!["tokio::time".to_string(), "Duration".to_string()],
            },
            NewConcept {
                name: "Circuit Breaker".to_string(),
                synonyms: vec!["fuse".to_string()],
                relationships: vec![],
            },
        ];

        persist_concepts(&concepts, "how does retry work", &kg_path);

        let retry_path = kg_path.join("learned-retry-policy.md");
        assert!(retry_path.exists(), "expected learned-retry-policy.md");
        let retry_content = fs::read_to_string(&retry_path).unwrap();
        assert!(retry_content.starts_with("# Retry Policy"));
        assert!(retry_content.contains("synonyms:: backoff, retry configuration"));
        assert!(retry_content.contains("related:: tokio::time, Duration"));
        assert!(retry_content.contains("Discovered during search"));

        let cb_path = kg_path.join("learned-circuit-breaker.md");
        assert!(cb_path.exists(), "expected learned-circuit-breaker.md");
        let cb_content = fs::read_to_string(&cb_path).unwrap();
        assert!(cb_content.starts_with("# Circuit Breaker"));
        assert!(cb_content.contains("synonyms:: fuse"));
        assert!(!cb_content.contains("related::"));
    }

    #[test]
    fn test_persist_concepts_skips_existing_files() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let kg_path = tmp.path().to_path_buf();
        let existing = kg_path.join("learned-retry-policy.md");
        fs::write(&existing, "# existing content").unwrap();

        let concepts = vec![NewConcept {
            name: "Retry Policy".to_string(),
            synonyms: vec![],
            relationships: vec![],
        }];

        persist_concepts(&concepts, "test", &kg_path);

        let content = fs::read_to_string(&existing).unwrap();
        assert_eq!(content, "# existing content");
    }

    #[test]
    fn test_persist_concepts_empty_is_noop() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let kg_path = tmp.path().to_path_buf();

        persist_concepts(&[], "test", &kg_path);

        assert!(fs::read_dir(&kg_path).unwrap().next().is_none());
    }
}
