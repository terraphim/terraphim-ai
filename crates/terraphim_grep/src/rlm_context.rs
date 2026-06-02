use std::collections::HashMap;

use super::hybrid_searcher::{KgConcept, RetrievedChunk};

/// Context assembled from retrieval results and passed to the RLM synthesis step.
#[derive(Debug, Clone)]
pub struct RlmContext {
    /// The original user query.
    pub query: String,
    /// Chunks retrieved from haystacks to use as evidence.
    pub retrieved_chunks: Vec<RetrievedChunk>,
    /// Knowledge-graph concepts that matched the query.
    pub kg_concepts: Vec<KgConcept>,
    /// Per-source metadata keyed by the chunk's `source` field.
    pub source_metadata: HashMap<String, DocumentMetadata>,
}

/// Lightweight metadata describing a document source.
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Haystack category the document came from (e.g. `"code"` or `"docs"`).
    pub source_type: String,
    /// ISO-8601 last-modified timestamp, if available.
    pub last_modified: Option<String>,
}

impl RlmContext {
    /// Create an empty context for `query` with no chunks or concepts yet.
    pub fn new(query: String) -> Self {
        Self {
            query,
            retrieved_chunks: Vec::new(),
            kg_concepts: Vec::new(),
            source_metadata: HashMap::new(),
        }
    }

    /// Attach retrieved chunks and populate `source_metadata` from their haystack labels.
    pub fn with_chunks(mut self, chunks: Vec<RetrievedChunk>) -> Self {
        self.retrieved_chunks = chunks;
        for chunk in &self.retrieved_chunks {
            self.source_metadata.insert(
                chunk.source.clone(),
                DocumentMetadata {
                    source_type: chunk.haystack.to_string(),
                    last_modified: None,
                },
            );
        }
        self
    }

    /// Attach knowledge-graph concepts matched for this query.
    pub fn with_concepts(mut self, concepts: Vec<KgConcept>) -> Self {
        self.kg_concepts = concepts;
        self
    }

    /// Render a textual prompt containing the query, retrieved context, and KG concepts.
    pub fn build_prompt(&self) -> String {
        let mut prompt = format!("Query: {}\n\n", self.query);

        if !self.retrieved_chunks.is_empty() {
            prompt.push_str("## Retrieved Context\n\n");
            for (i, chunk) in self.retrieved_chunks.iter().enumerate() {
                prompt.push_str(&format!(
                    "[{}] {} (line {:?}):\n{}\n\n",
                    i + 1,
                    chunk.source,
                    chunk.line_start,
                    chunk.content
                ));
            }
        }

        if !self.kg_concepts.is_empty() {
            prompt.push_str("## Knowledge Graph Concepts\n\n");
            for concept in &self.kg_concepts {
                prompt.push_str(&format!(
                    "- {} (score: {:.2})\n",
                    concept.name, concept.score
                ));
            }
            prompt.push('\n');
        }

        prompt
    }

    /// Return the character length of the rendered prompt.
    pub fn context_length(&self) -> usize {
        self.build_prompt().len()
    }

    /// Drop trailing chunks until the rendered prompt fits within `max_chars`.
    pub fn truncate(&mut self, max_chars: usize) {
        if self.context_length() > max_chars {
            let mut remaining = max_chars;
            let mut truncated_chunks = Vec::new();

            for chunk in &self.retrieved_chunks {
                if remaining > chunk.content.len() + 100 {
                    truncated_chunks.push(chunk.clone());
                    remaining -= chunk.content.len() + 100;
                } else {
                    break;
                }
            }

            self.retrieved_chunks = truncated_chunks;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_chunk(content: &str, source: &str, haystack: &'static str) -> RetrievedChunk {
        RetrievedChunk {
            content: content.to_string(),
            source: source.to_string(),
            line_start: Some(1),
            line_end: Some(1),
            relevance_score: 0.8,
            haystack,
        }
    }

    #[test]
    fn test_rlm_context_new() {
        let ctx = RlmContext::new("test query".to_string());
        assert_eq!(ctx.query, "test query");
        assert!(ctx.retrieved_chunks.is_empty());
        assert!(ctx.kg_concepts.is_empty());
    }

    #[test]
    fn test_rlm_context_with_chunks() {
        let chunks = vec![
            make_chunk("test content", "file.rs", "code"),
            make_chunk("more content", "file2.rs", "code"),
        ];
        let ctx = RlmContext::new("test".to_string()).with_chunks(chunks);

        assert_eq!(ctx.retrieved_chunks.len(), 2);
        assert_eq!(ctx.source_metadata.len(), 2);
    }

    #[test]
    fn test_rlm_context_with_concepts() {
        let concepts = vec![KgConcept {
            id: 1,
            name: "test concept".to_string(),
            display_value: None,
            score: 0.9,
        }];
        let ctx = RlmContext::new("test".to_string()).with_concepts(concepts);

        assert_eq!(ctx.kg_concepts.len(), 1);
    }

    #[test]
    fn test_build_prompt() {
        let chunks = vec![make_chunk("retry configuration", "src/retry.rs", "code")];
        let concepts = vec![KgConcept {
            id: 1,
            name: "retry".to_string(),
            display_value: None,
            score: 0.9,
        }];

        let ctx = RlmContext::new("retry".to_string())
            .with_chunks(chunks)
            .with_concepts(concepts);

        let prompt = ctx.build_prompt();
        assert!(prompt.contains("Query: retry"));
        assert!(prompt.contains("Retrieved Context"));
        assert!(prompt.contains("retry configuration"));
        assert!(prompt.contains("Knowledge Graph Concepts"));
        assert!(prompt.contains("retry"));
    }

    #[test]
    fn test_truncate() {
        let chunks = vec![
            make_chunk(&"x".repeat(1000), "file1.rs", "code"),
            make_chunk(&"y".repeat(1000), "file2.rs", "code"),
        ];
        let mut ctx = RlmContext::new("test".to_string()).with_chunks(chunks);

        assert!(ctx.context_length() > 2000);
        ctx.truncate(500);
        assert!(ctx.context_length() <= 500);
    }
}
