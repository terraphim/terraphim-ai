use serde::{Deserialize, Serialize};

use crate::error::TerraphimGrepError;

/// A typed prompt-and-parser pair for a single RLM output format.
pub trait RlmSignature: Send + Sync {
    /// The Rust type this signature deserialises the LLM response into.
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    /// Returns the natural-language instructions to include in the LLM prompt.
    fn instructions(&self) -> String;
    /// Parse the raw LLM response string into [`Self::Output`].
    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError>;
}

/// A single file-level match returned by the search-result RLM signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    /// Relative path of the matching file.
    pub path: String,
    /// Line number of the match (1-based).
    pub line: usize,
    /// Inclusive end line of the match, if the match spans multiple lines.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line_end: Option<usize>,
    /// Surrounding lines included as context for the match.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context: Vec<String>,
}

/// RLM signature that parses a JSON array of [`Match`] objects from the LLM response.
pub struct SearchResultSignature;

impl RlmSignature for SearchResultSignature {
    type Output = Vec<Match>;

    fn instructions(&self) -> String {
        "Return a JSON array of matches with path, line, line_end (optional), and context (optional).".to_string()
    }

    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError> {
        serde_json::from_str(raw).map_err(|e| {
            TerraphimGrepError::RlmFailed(format!("failed to parse search results: {}", e))
        })
    }
}

/// A source citation accompanying a synthesised answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// File path or URL of the cited source.
    pub source: String,
    /// Line number within the source, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Short excerpt from the cited source supporting the answer.
    pub excerpt: String,
}

/// A synthesised answer produced by the RLM, paired with its source citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerWithCitations {
    /// The synthesised natural-language answer.
    pub answer: String,
    /// Sources cited in support of the answer.
    pub citations: Vec<Citation>,
    /// Confidence score in the range `[0.0, 1.0]` reported by the LLM.
    pub confidence: f64,
}

/// RLM signature that parses an [`AnswerWithCitations`] object from the LLM response.
pub struct AnswerSignature;

impl RlmSignature for AnswerSignature {
    type Output = AnswerWithCitations;

    fn instructions(&self) -> String {
        r#"Return a JSON object with:
- "answer": the synthesised answer
- "citations": array of {source, line (optional), excerpt}
- "confidence": a number between 0 and 1"#
            .to_string()
    }

    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError> {
        serde_json::from_str(raw)
            .map_err(|e| TerraphimGrepError::RlmFailed(format!("failed to parse answer: {}", e)))
    }
}

/// A newly discovered knowledge-graph concept extracted from a query/answer pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConcept {
    /// Canonical name of the concept (e.g. `"retry configuration"`).
    pub name: String,
    /// Alternative names or synonyms for the concept.
    #[serde(default)]
    pub synonyms: Vec<String>,
    /// Names of related concepts in the knowledge graph.
    #[serde(default)]
    pub relationships: Vec<String>,
}

/// RLM signature that parses a JSON array of [`NewConcept`] objects from the LLM response.
pub struct ConceptExtractionSignature;

impl RlmSignature for ConceptExtractionSignature {
    type Output = Vec<NewConcept>;

    fn instructions(&self) -> String {
        r#"Extract new concepts from the query and answer.
Return a JSON array of objects with:
- "name": concept name (e.g., "retry configuration")
- "synonyms": array of alternative names (e.g., ["backoff", "retry policy"])
- "relationships": array of related concepts (e.g., ["tokio::time", "Duration"])"#
            .to_string()
    }

    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError> {
        serde_json::from_str(raw)
            .map_err(|e| TerraphimGrepError::RlmFailed(format!("failed to parse concepts: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_result_signature_parse() {
        let signature = SearchResultSignature {};
        let raw = r#"[
            {"path": "src/main.rs", "line": 42, "context": ["fn main()", "test"]},
            {"path": "src/lib.rs", "line": 10}
        ]"#;

        let result = signature.parse(raw);
        if let Err(e) = &result {
            eprintln!("Parse error: {:?}", e);
        }
        assert!(result.is_ok(), "parse failed: {:?}", result.as_ref().err());
        let matches = result.unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].path, "src/main.rs");
        assert_eq!(matches[0].line, 42);
    }

    #[test]
    fn test_answer_signature_parse() {
        let signature = AnswerSignature {};
        let raw = r#"{
            "answer": "Retry is configured in src/main.rs",
            "citations": [
                {"source": "src/main.rs", "line": 42, "excerpt": "pub retry_policy"}
            ],
            "confidence": 0.95
        }"#;

        let result = signature.parse(raw);
        assert!(result.is_ok());
        let answer = result.unwrap();
        assert!(answer.answer.contains("Retry"));
        assert_eq!(answer.citations.len(), 1);
        assert!((answer.confidence - 0.95).abs() < 0.001);
    }

    #[test]
    fn test_concept_extraction_signature_parse() {
        let signature = ConceptExtractionSignature {};
        let raw = r#"[
            {
                "name": "retry configuration",
                "synonyms": ["backoff", "retry policy"],
                "relationships": ["tokio::time", "Duration"]
            }
        ]"#;

        let result = signature.parse(raw);
        assert!(result.is_ok());
        let concepts = result.unwrap();
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].name, "retry configuration");
        assert_eq!(concepts[0].synonyms.len(), 2);
    }

    #[test]
    fn test_search_result_signature_invalid() {
        let signature = SearchResultSignature {};
        let raw = "not valid json";

        let result = signature.parse(raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_signature_instructions() {
        let search_sig = SearchResultSignature {};
        let answer_sig = AnswerSignature {};
        let concept_sig = ConceptExtractionSignature {};

        assert!(!search_sig.instructions().is_empty());
        assert!(!answer_sig.instructions().is_empty());
        assert!(!concept_sig.instructions().is_empty());
    }
}
