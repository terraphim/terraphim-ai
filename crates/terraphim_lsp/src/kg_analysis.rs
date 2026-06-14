use std::collections::HashSet;

use terraphim_automata::{Matched, find_matches};
use terraphim_types::Thesaurus;

/// Result of analysing a document against a knowledge graph thesaurus.
#[derive(Debug, Clone, PartialEq)]
pub struct KgAnalysis {
    /// Terms from the thesaurus found in the document.
    pub matched_terms: Vec<TermMatch>,
    /// Words in the document that did not match any thesaurus entry.
    pub unknown_terms: Vec<String>,
}

/// A single knowledge-graph term match inside a document.
#[derive(Debug, Clone, PartialEq)]
pub struct TermMatch {
    /// The matched term text.
    pub term: String,
    /// Byte offset range `(start, end)` inside the document.
    pub range: (usize, usize),
    /// Optional definition/description from the thesaurus.
    pub description: Option<String>,
}

impl KgAnalysis {
    /// Create an empty analysis result.
    pub fn empty() -> Self {
        Self {
            matched_terms: Vec::new(),
            unknown_terms: Vec::new(),
        }
    }

    /// Returns true if no terms were matched and no unknown terms were found.
    pub fn is_empty(&self) -> bool {
        self.matched_terms.is_empty() && self.unknown_terms.is_empty()
    }
}

/// Analyse a markdown document against a knowledge graph thesaurus.
///
/// Returns matched terms with byte positions and a list of unknown words.
/// The current implementation matches whole thesaurus entries using
/// `terraphim_automata::find_matches`. Unknown terms are extracted as the
/// set of whitespace-separated words that were not part of any match.
pub fn analyse_kg_document(text: &str, thesaurus: &Thesaurus) -> KgAnalysis {
    if text.trim().is_empty() || thesaurus.is_empty() {
        return KgAnalysis::empty();
    }

    let matches = match find_matches(text, thesaurus.clone(), true) {
        Ok(matches) => matches,
        Err(err) => {
            log::warn!("KG analysis find_matches failed: {}", err);
            return KgAnalysis::empty();
        }
    };

    let mut matched_terms: Vec<TermMatch> = Vec::with_capacity(matches.len());
    let mut matched_words: HashSet<String> = HashSet::new();

    for Matched {
        term,
        normalized_term,
        pos,
    } in matches
    {
        if let Some((start, end)) = pos {
            // Record each word of the matched term as known.
            for word in term.split_whitespace() {
                matched_words.insert(word.to_lowercase());
            }

            matched_terms.push(TermMatch {
                term: term.clone(),
                range: (start, end),
                description: Some(normalized_term.display().to_string()).filter(|s| !s.is_empty()),
            });
        }
    }

    // Treat any non-empty whitespace-separated token that was not part of a
    // match as an unknown term. This is intentionally simple: multi-word
    // unknown phrases are not reconstructed here.
    let unknown_terms: Vec<String> = text
        .split_whitespace()
        .map(|word| word.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|word| {
            let lower = word.to_lowercase();
            !lower.is_empty()
                && !matched_words.contains(&lower)
                && !matched_terms.iter().any(|m| {
                    m.range.0 <= text.len()
                        && m.range.1 <= text.len()
                        && text[m.range.0..m.range.1].to_lowercase().contains(&lower)
                })
        })
        .map(String::from)
        .collect();

    KgAnalysis {
        matched_terms,
        unknown_terms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

    fn sample_thesaurus() -> Thesaurus {
        let mut thesaurus = Thesaurus::new("programming".to_string());
        thesaurus.insert(
            NormalizedTermValue::from("rust"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("rust programming language"))
                .with_url("https://rust-lang.org".to_string()),
        );
        thesaurus.insert(
            NormalizedTermValue::from("async"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("asynchronous programming")),
        );
        thesaurus.insert(
            NormalizedTermValue::from("tokio"),
            NormalizedTerm::with_auto_id(NormalizedTermValue::from("tokio async runtime")),
        );
        thesaurus
    }

    #[test]
    fn test_empty_text_returns_empty() {
        let thesaurus = sample_thesaurus();
        let analysis = analyse_kg_document("", &thesaurus);
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_empty_thesaurus_returns_empty() {
        let thesaurus = Thesaurus::new("empty".to_string());
        let analysis = analyse_kg_document("rust is great", &thesaurus);
        assert!(analysis.is_empty());
    }

    #[test]
    fn test_matched_terms_found() {
        let thesaurus = sample_thesaurus();
        let analysis = analyse_kg_document("rust and tokio are great", &thesaurus);
        let terms: Vec<String> = analysis
            .matched_terms
            .iter()
            .map(|m| m.term.clone())
            .collect();
        assert!(terms.contains(&"rust".to_string()));
        assert!(terms.contains(&"tokio".to_string()));
    }

    #[test]
    fn test_unknown_terms_found() {
        let thesaurus = sample_thesaurus();
        let analysis = analyse_kg_document("rust and xyz are great", &thesaurus);
        assert!(analysis.unknown_terms.contains(&"xyz".to_string()));
    }

    #[test]
    fn test_positions_are_populated() {
        let thesaurus = sample_thesaurus();
        let analysis = analyse_kg_document("rust is great", &thesaurus);
        let rust_match = analysis
            .matched_terms
            .iter()
            .find(|m| m.term == "rust")
            .expect("rust should match");
        assert_eq!(rust_match.range, (0, 4));
    }

    #[test]
    fn test_analyse_never_panics_on_arbitrary_input() {
        let thesaurus = sample_thesaurus();
        let inputs = [
            "!@#$%^&*()",
            "rust\n\ntokio\tasync",
            "",
            "RUST",
            "a b c d e f g",
        ];
        for input in inputs {
            let _ = analyse_kg_document(input, &thesaurus);
        }
    }
}
