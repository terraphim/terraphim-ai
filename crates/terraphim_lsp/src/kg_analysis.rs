//! Knowledge-graph analysis helpers for Terraphim LSP.
//!
//! Term matching, concept ids and the annotation-block boundary come from
//! [`terraphim_lsp_core`]; this module adds the server-only list of
//! unrecognised words used for diagnostics.

use std::collections::HashSet;

use terraphim_lsp_core::{Diagnostic, KgEngine};

pub use terraphim_lsp_core::TermMatch;

/// Result of analysing a document against a knowledge graph.
#[derive(Debug, Clone, PartialEq)]
pub struct KgAnalysis {
    /// Terms from the thesaurus found in the document body, with concept id,
    /// normalised concept name and byte plus UTF-16 ranges.
    pub matched_terms: Vec<TermMatch>,
    /// Words in the body that did not match any thesaurus entry.
    pub unknown_terms: Vec<String>,
    /// Problems found by the core, such as a malformed annotation block.
    pub diagnostics: Vec<Diagnostic>,
}

impl KgAnalysis {
    /// Create an empty analysis result.
    pub fn empty() -> Self {
        Self {
            matched_terms: Vec::new(),
            unknown_terms: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Returns true if nothing was matched, no unknown terms were found and
    /// there are no diagnostics.
    pub fn is_empty(&self) -> bool {
        self.matched_terms.is_empty()
            && self.unknown_terms.is_empty()
            && self.diagnostics.is_empty()
    }
}

/// Analyse a markdown document with a knowledge-graph engine.
///
/// Matching is delegated to [`KgEngine::analyse`], which excludes a trailing
/// `terraphim-alternatives` annotation block. Unknown terms are the
/// whitespace-separated words of the body that were not part of any match.
pub fn analyse_kg_document(text: &str, engine: &KgEngine) -> KgAnalysis {
    if text.trim().is_empty() {
        return KgAnalysis::empty();
    }

    let analysis = engine.analyse(text);
    if engine.concept_index().is_empty() {
        // No knowledge graph loaded: every word would be "unknown", which is
        // noise. Block diagnostics do not depend on the graph, so keep them.
        return KgAnalysis {
            diagnostics: analysis.diagnostics,
            ..KgAnalysis::empty()
        };
    }

    let body = &text[..analysis.body_end.byte];
    let matched_words: HashSet<String> = analysis
        .matches
        .iter()
        .flat_map(|m| m.term.split_whitespace().map(str::to_lowercase))
        .collect();
    let matched_spans: Vec<String> = analysis
        .matches
        .iter()
        .map(|m| m.text.to_lowercase())
        .collect();

    // Treat any non-empty whitespace-separated token that was not part of a
    // match as an unknown term. This is intentionally simple: multi-word
    // unknown phrases are not reconstructed here.
    let unknown_terms = body
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| {
            let lower = word.to_lowercase();
            !lower.is_empty()
                && !matched_words.contains(&lower)
                && !matched_spans.iter().any(|span| span.contains(&lower))
        })
        .map(String::from)
        .collect();

    KgAnalysis {
        matched_terms: analysis.matches,
        unknown_terms,
        diagnostics: analysis.diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    fn sample_engine() -> KgEngine {
        let mut thesaurus = Thesaurus::new("programming".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("rust"),
            NormalizedTerm::new(1, NormalizedTermValue::from("rust programming language"))
                .with_url("https://rust-lang.org".to_string()),
        );
        thesaurus.insert(
            NormalizedTermValue::from("async"),
            NormalizedTerm::new(2, NormalizedTermValue::from("asynchronous programming")),
        );
        thesaurus.insert(
            NormalizedTermValue::from("tokio"),
            NormalizedTerm::new(3, NormalizedTermValue::from("tokio async runtime")),
        );
        KgEngine::new(&thesaurus).expect("sample thesaurus compiles")
    }

    #[test]
    fn test_empty_text_returns_empty() {
        let analysis = analyse_kg_document("", &sample_engine());
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_empty_thesaurus_returns_empty() {
        let analysis = analyse_kg_document("rust is great", &KgEngine::empty());
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_matched_terms_found() {
        let analysis = analyse_kg_document("rust and tokio are great", &sample_engine());
        let terms: Vec<String> = analysis
            .matched_terms
            .iter()
            .map(|m| m.term.clone())
            .collect();
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_matches_keep_concept_id_and_nterm() {
        let analysis = analyse_kg_document("tokio rocks", &sample_engine());
        let tokio = &analysis.matched_terms[0];
        assert_eq!(tokio.concept_id, 3);
        assert_eq!(tokio.nterm, "tokio async runtime");
    }

    #[test]
    fn test_unknown_terms_found() {
        let analysis = analyse_kg_document("rust and xyz are great", &sample_engine());
        assert!(analysis.unknown_terms.contains(&"xyz".to_string()));
    }

    #[test]
    fn test_positions_are_populated() {
        let analysis = analyse_kg_document("rust is great", &sample_engine());
        let rust_match = analysis
            .matched_terms
            .iter()
            .find(|m| m.term == "rust")
            .expect("rust should match");
        assert_eq!(rust_match.range.bytes(), 0..4);
    }

    #[test]
    fn test_annotation_block_is_not_analysed() {
        let text = "rust\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"overflow\": \"tokio zzz\"}\n```\n";
        let analysis = analyse_kg_document(text, &sample_engine());
        assert_eq!(analysis.matched_terms.len(), 1);
        assert!(
            analysis.unknown_terms.is_empty(),
            "{:?}",
            analysis.unknown_terms
        );
        assert!(analysis.diagnostics.is_empty());
    }

    #[test]
    fn test_malformed_annotation_block_yields_one_diagnostic() {
        let text = "rust\n\n```terraphim-alternatives\n{\"version\": 1}\n";
        let analysis = analyse_kg_document(text, &sample_engine());
        assert_eq!(analysis.diagnostics.len(), 1);
        assert_eq!(analysis.matched_terms.len(), 1);
    }

    #[test]
    fn test_analyse_never_panics_on_arbitrary_input() {
        let engine = sample_engine();
        let inputs = [
            "!@#$%^&*()",
            "rust\n\ntokio\tasync",
            "",
            "RUST",
            "a b c d e f g",
            "😀 rust é",
        ];
        for input in inputs {
            let _ = analyse_kg_document(input, &engine);
        }
    }
}
