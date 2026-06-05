use serde::{Deserialize, Serialize};

use crate::error::TerraphimGrepError;

/// Defines the contract for an RLM output signature: instruction generation and JSON parsing.
pub trait RlmSignature: Send + Sync {
    /// The deserialisable output type produced by this signature's `parse` method.
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    /// Returns the prompt instructions that tell the LLM which JSON schema to emit.
    fn instructions(&self) -> String;
    /// Parses the raw LLM response string into the typed `Output`.
    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError>;
}

/// Represents a single file-match location produced by the search signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    /// The file path where the match was found.
    pub path: String,
    /// The starting line number of the match.
    pub line: usize,
    /// The optional ending line number when the match spans multiple lines.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line_end: Option<usize>,
    /// The surrounding context lines included with the match.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context: Vec<String>,
}

/// Represents the RLM signature for parsing a list of file-match results.
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

/// Represents a source citation linking an answer claim to a specific file location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// The file path or URL of the cited source.
    pub source: String,
    /// The line number within the source, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// A short excerpt from the source that supports the claim.
    pub excerpt: String,
}

/// Represents a synthesised answer accompanied by supporting source citations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerWithCitations {
    /// The synthesised answer text produced by the RLM.
    pub answer: String,
    /// The source citations that back the synthesised answer.
    pub citations: Vec<Citation>,
    /// The model's self-reported confidence score in the range `[0.0, 1.0]`.
    pub confidence: f64,
}

/// Represents the RLM signature for parsing a synthesised answer with citations.
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

/// Represents a newly extracted knowledge-graph concept identified by the RLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConcept {
    /// The canonical name of the extracted concept.
    pub name: String,
    /// Alternative names or synonyms for this concept.
    #[serde(default)]
    pub synonyms: Vec<String>,
    /// Related concept names linked to this concept.
    #[serde(default)]
    pub relationships: Vec<String>,
}

/// Represents the RLM signature for extracting new KG concepts from a query-answer pair.
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
