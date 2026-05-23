use serde::{Deserialize, Serialize};

use crate::error::TerraphimGrepError;

pub trait RlmSignature: Send + Sync {
    type Output: serde::Serialize + serde::de::DeserializeOwned;

    fn instructions(&self) -> String;
    fn parse(&self, raw: &str) -> Result<Self::Output, TerraphimGrepError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Match {
    pub path: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line_end: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub context: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerWithCitations {
    pub answer: String,
    pub citations: Vec<Citation>,
    pub confidence: f64,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConcept {
    pub name: String,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub relationships: Vec<String>,
}

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
