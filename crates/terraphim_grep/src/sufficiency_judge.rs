use super::hybrid_searcher::{HybridResults, RetrievedChunk};

/// Configurable thresholds used by [`SufficiencyJudge`] to classify search quality.
#[derive(Debug, Clone)]
pub struct HeuristicThresholds {
    /// Minimum fraction of query terms that must appear in the retrieved chunks.
    pub min_coverage: f64,
    /// Minimum average KG concept score required to consider results confident.
    pub min_kg_confidence: f64,
    /// Minimum number of distinct haystacks that must contribute chunks.
    pub min_diversity: usize,
    /// Minimum total number of chunks required before any positive verdict.
    pub min_results: usize,
}

impl Default for HeuristicThresholds {
    fn default() -> Self {
        Self {
            min_coverage: 0.7,
            min_kg_confidence: 0.5,
            min_diversity: 2,
            min_results: 3,
        }
    }
}

/// The verdict returned by [`SufficiencyJudge::judge`], along with the ranked chunks.
#[derive(Debug, Clone)]
pub enum Sufficiency {
    /// Retrieval results are good enough to return directly without synthesis.
    Sufficient(Vec<RetrievedChunk>),
    /// Results exist but need LLM synthesis to produce a coherent answer.
    NeedsSynthesis(Vec<RetrievedChunk>),
    /// Results are sparse; additional expansion or synthesis is required.
    NeedsExpansion(Vec<RetrievedChunk>),
    /// Results are too poor to be useful; return empty or a failure message.
    Insufficient(Vec<RetrievedChunk>),
}

/// Evaluates whether a set of hybrid search results is sufficient or requires further synthesis.
pub struct SufficiencyJudge {
    thresholds: HeuristicThresholds,
}

impl SufficiencyJudge {
    /// Create a new judge with the given heuristic thresholds.
    pub fn new(thresholds: HeuristicThresholds) -> Self {
        Self { thresholds }
    }

    /// Classify `results` for `query` and return the appropriate [`Sufficiency`] variant.
    pub fn judge(&self, results: &HybridResults, query: &str) -> Sufficiency {
        let chunks = results.to_chunks();

        if chunks.is_empty() && results.kg_concepts.is_empty() {
            return Sufficiency::Insufficient(vec![]);
        }

        let coverage = self.calculate_coverage(query, &chunks);
        let confidence = self.calculate_kg_confidence(&results.kg_concepts);
        let diversity = self.calculate_diversity(&chunks);

        if chunks.len() < self.thresholds.min_results {
            return Sufficiency::Insufficient(chunks);
        }

        if coverage >= self.thresholds.min_coverage
            && confidence >= self.thresholds.min_kg_confidence
            && diversity >= self.thresholds.min_diversity
        {
            Sufficiency::Sufficient(chunks)
        } else if coverage >= 0.3 && !chunks.is_empty() {
            Sufficiency::NeedsSynthesis(chunks)
        } else if coverage > 0.0 {
            Sufficiency::NeedsExpansion(chunks)
        } else {
            Sufficiency::Insufficient(chunks)
        }
    }

    fn calculate_coverage(&self, query: &str, chunks: &[RetrievedChunk]) -> f64 {
        if chunks.is_empty() {
            return 0.0;
        }

        let query_terms: std::collections::HashSet<String> =
            query.split_whitespace().map(|s| s.to_lowercase()).collect();

        if query_terms.is_empty() {
            return 1.0;
        }

        let mut covered_terms = 0usize;
        for term in &query_terms {
            let term_found = chunks.iter().any(|chunk| {
                chunk.content.to_lowercase().contains(term)
                    || chunk.source.to_lowercase().contains(term)
            });
            if term_found {
                covered_terms += 1;
            }
        }

        covered_terms as f64 / query_terms.len() as f64
    }

    fn calculate_kg_confidence(&self, kg_concepts: &[super::hybrid_searcher::KgConcept]) -> f64 {
        if kg_concepts.is_empty() {
            return 0.0;
        }

        let avg_score: f64 =
            kg_concepts.iter().map(|c| c.score).sum::<f64>() / kg_concepts.len() as f64;
        avg_score.min(1.0)
    }

    fn calculate_diversity(&self, chunks: &[RetrievedChunk]) -> usize {
        let haystacks: std::collections::HashSet<&str> =
            chunks.iter().map(|c| c.haystack).collect();
        haystacks.len()
    }
}

impl Default for SufficiencyJudge {
    fn default() -> Self {
        Self::new(HeuristicThresholds::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid_searcher::KgConcept;

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
    fn test_empty_results_insufficient() {
        let judge = SufficiencyJudge::default();
        let results = HybridResults {
            code_results: vec![],
            doc_results: vec![],
            kg_concepts: vec![],
        };

        let sufficiency = judge.judge(&results, "test query");
        assert!(matches!(sufficiency, Sufficiency::Insufficient(_)));
    }

    #[test]
    fn test_low_results_insufficient() {
        let judge = SufficiencyJudge::default();
        let results = HybridResults {
            code_results: vec![make_chunk("test", "file.rs", "code")],
            doc_results: vec![],
            kg_concepts: vec![],
        };

        let sufficiency = judge.judge(&results, "test query");
        assert!(matches!(sufficiency, Sufficiency::Insufficient(_)));
    }

    #[test]
    fn test_high_coverage_sufficient() {
        let judge = SufficiencyJudge::default();
        let results = HybridResults {
            code_results: vec![
                make_chunk("retry configuration in test file", "retry.rs", "code"),
                make_chunk("backoff settings", "config.rs", "code"),
            ],
            doc_results: vec![make_chunk("retry docs", "docs.md", "docs")],
            kg_concepts: vec![KgConcept {
                id: 1,
                name: "retry".to_string(),
                display_value: None,
                score: 0.9,
            }],
        };

        let sufficiency = judge.judge(&results, "retry configuration");
        assert!(matches!(sufficiency, Sufficiency::Sufficient(_)));
    }

    #[test]
    fn test_coverage_calculation() {
        let judge = SufficiencyJudge::default();
        let chunks = vec![make_chunk("retry configuration", "file.rs", "code")];

        let coverage = judge.calculate_coverage("retry configuration", &chunks);
        assert!(coverage >= 0.9);

        let coverage2 = judge.calculate_coverage("missing term", &chunks);
        assert!(coverage2 < 0.5);
    }

    #[test]
    fn test_diversity_calculation() {
        let judge = SufficiencyJudge::default();
        let chunks = vec![
            make_chunk("code result", "file.rs", "code"),
            make_chunk("code result 2", "file2.rs", "code"),
        ];
        assert_eq!(judge.calculate_diversity(&chunks), 1);

        let chunks2 = vec![
            make_chunk("code result", "file.rs", "code"),
            make_chunk("doc result", "file.md", "docs"),
        ];
        assert_eq!(judge.calculate_diversity(&chunks2), 2);
    }

    #[test]
    fn test_kg_confidence_calculation() {
        let judge = SufficiencyJudge::default();
        let concepts = vec![
            KgConcept {
                id: 1,
                name: "test".to_string(),
                display_value: None,
                score: 0.9,
            },
            KgConcept {
                id: 2,
                name: "test2".to_string(),
                display_value: None,
                score: 0.7,
            },
        ];
        let confidence = judge.calculate_kg_confidence(&concepts);
        assert!((confidence - 0.8).abs() < 0.001);

        let empty_confidence = judge.calculate_kg_confidence(&[]);
        assert_eq!(empty_confidence, 0.0);
    }
}
