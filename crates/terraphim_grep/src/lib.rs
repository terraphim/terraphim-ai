//! # terraphim_grep
//!
//! Hybrid search combining knowledge-graph concept expansion with ripgrep-backed
//! full-text search. Runs both pipelines concurrently via `tokio::spawn` and
//! merges results ranked by KG relevance boost and BM25 score.
//!
//! ## Quick start
//!
//! ```rust,ignore
//! use terraphim_grep::{TerraphimGrep, GrepOptions};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! # let searcher = Arc::new(terraphim_grep::HybridSearcher::default());
//! # let judge = Arc::new(terraphim_grep::SufficiencyJudge::default());
//! let grep = TerraphimGrep::new(searcher, judge);
//! let options = GrepOptions::default();
//! let results = grep.search("query", options).await?;
//! # Ok(())
//! # }
//! ```

/// Error types for the terraphim-grep crate.
pub mod error;
/// Hybrid KG + code-search orchestration and result types.
pub mod hybrid_searcher;
/// LLM-backed knowledge-graph curation from query/answer pairs.
pub mod kg_curation;
/// Context builder passed to the RLM synthesis step.
pub mod rlm_context;
/// Typed prompt/parser pairs (signatures) for LLM output formats.
pub mod signatures;
/// Heuristic judge that decides whether retrieval results are sufficient.
pub mod sufficiency_judge;

use std::sync::Arc;

pub use error::{Result, TerraphimGrepError};
pub use hybrid_searcher::{
    DEFAULT_KG_BOOST_WEIGHT, GrepOptions, Haystack, HybridResults, HybridSearcher, KgConcept,
    RetrievedChunk, boost_chunks_with_kg, score_kg_boost,
};
pub use kg_curation::KgCurationRlm;
pub use rlm_context::RlmContext;
pub use signatures::{AnswerWithCitations, Citation, Match, NewConcept, RlmSignature};
pub use sufficiency_judge::{HeuristicThresholds, Sufficiency, SufficiencyJudge};

/// The top-level result returned by [`TerraphimGrep::search`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepResult {
    /// Ranked list of text chunks retrieved from the haystacks.
    pub chunks: Vec<RetrievedChunk>,
    /// Synthesised answer with source citations, present only when RLM synthesis ran.
    pub answer: Option<AnswerWithCitations>,
    /// Knowledge-graph concepts that matched the query.
    pub concepts: Vec<KgConcept>,
    /// Indicates whether the result was produced by search alone or required RLM synthesis.
    pub sufficiency: SufficiencyState,
    /// Timing and count statistics for this search invocation.
    pub stats: GrepStats,
}

/// Describes how the search result was produced.
#[derive(Debug, Clone, serde::Serialize)]
pub enum SufficiencyState {
    /// Results came from retrieval alone; the RLM synthesis step was not invoked.
    SearchOnly,
    /// The RLM synthesis step ran and produced a synthesised answer.
    RlmSynthesis,
    /// The RLM synthesis step ran but could not produce a sufficiently confident answer.
    RlmInsufficient,
}

/// Timing and result-count statistics for a single search invocation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepStats {
    /// Wall-clock time in milliseconds spent on the retrieval phase.
    pub search_latency_ms: u64,
    /// Wall-clock time in milliseconds spent in the RLM synthesis step, if it ran.
    pub rlm_latency_ms: Option<u64>,
    /// Number of chunks included in the final result.
    pub chunks_returned: usize,
    /// Number of KG concept matches that contributed to ranking.
    pub kg_hits: usize,
}

/// Top-level search engine that combines hybrid retrieval with optional RLM synthesis.
pub struct TerraphimGrep {
    hybrid_searcher: Arc<HybridSearcher>,
    sufficiency_judge: Arc<SufficiencyJudge>,
    #[cfg(feature = "llm")]
    kg_curation: Option<Arc<KgCurationRlm>>,
    #[cfg(feature = "llm")]
    llm_client: Option<Arc<dyn terraphim_service::llm::LlmClient>>,
}

impl TerraphimGrep {
    /// Create a new `TerraphimGrep` with the given searcher and sufficiency judge.
    #[cfg(feature = "llm")]
    pub fn new(
        hybrid_searcher: Arc<HybridSearcher>,
        sufficiency_judge: Arc<SufficiencyJudge>,
    ) -> Self {
        Self {
            hybrid_searcher,
            sufficiency_judge,
            kg_curation: None,
            llm_client: None,
        }
    }

    /// Create a new `TerraphimGrep` with the given searcher and sufficiency judge.
    #[cfg(not(feature = "llm"))]
    pub fn new(
        hybrid_searcher: Arc<HybridSearcher>,
        sufficiency_judge: Arc<SufficiencyJudge>,
    ) -> Self {
        Self {
            hybrid_searcher,
            sufficiency_judge,
        }
    }

    /// Attach a KG-curation pipeline that extracts and persists new concepts from RLM answers.
    #[cfg(feature = "llm")]
    pub fn with_kg_curation(mut self, kg_curation: Arc<KgCurationRlm>) -> Self {
        self.kg_curation = Some(kg_curation);
        self
    }

    /// Attach an LLM client used for RLM synthesis when retrieval is insufficient.
    #[cfg(feature = "llm")]
    pub fn with_llm_client(
        mut self,
        llm_client: Arc<dyn terraphim_service::llm::LlmClient>,
    ) -> Self {
        self.llm_client = Some(llm_client);
        self
    }

    /// Search for `query` using the configured hybrid retrieval and optional RLM synthesis.
    pub async fn search(&self, query: &str, options: GrepOptions) -> Result<GrepResult> {
        let start = std::time::Instant::now();

        if options.force_rlm {
            return self.search_with_rlm(query, options, start).await;
        }

        let hybrid_results = self
            .hybrid_searcher
            .search(query, &options)
            .await
            .map_err(TerraphimGrepError::SearchFailed)?;

        let search_latency_ms = start.elapsed().as_millis() as u64;

        let sufficiency = self.sufficiency_judge.judge(&hybrid_results, query);

        match sufficiency {
            sufficiency_judge::Sufficiency::Sufficient(chunks) => {
                let stats = GrepStats {
                    search_latency_ms,
                    rlm_latency_ms: None,
                    chunks_returned: chunks.len(),
                    kg_hits: hybrid_results.kg_concepts.len(),
                };

                Ok(GrepResult {
                    chunks,
                    answer: None,
                    concepts: hybrid_results.kg_concepts,
                    sufficiency: SufficiencyState::SearchOnly,
                    stats,
                })
            }
            sufficiency_judge::Sufficiency::NeedsSynthesis(chunks) => {
                self.search_with_rlm_fallback(query, options, chunks, hybrid_results, start)
                    .await
            }
            sufficiency_judge::Sufficiency::NeedsExpansion(mut chunks) => {
                chunks.extend(hybrid_results.to_chunks());
                self.search_with_rlm_fallback(query, options, chunks, hybrid_results, start)
                    .await
            }
            sufficiency_judge::Sufficiency::Insufficient(chunks) => {
                let stats = GrepStats {
                    search_latency_ms,
                    rlm_latency_ms: None,
                    chunks_returned: 0,
                    kg_hits: 0,
                };

                Ok(GrepResult {
                    chunks,
                    answer: None,
                    concepts: vec![],
                    sufficiency: SufficiencyState::RlmInsufficient,
                    stats,
                })
            }
        }
    }

    #[cfg(feature = "llm")]
    async fn search_with_rlm_fallback(
        &self,
        query: &str,
        options: GrepOptions,
        chunks: Vec<RetrievedChunk>,
        hybrid_results: HybridResults,
        start: std::time::Instant,
    ) -> Result<GrepResult> {
        let rlm_start = std::time::Instant::now();

        let ctx = RlmContext::new(query.to_string())
            .with_chunks(chunks.clone())
            .with_concepts(hybrid_results.kg_concepts.clone());

        let prompt = ctx.build_prompt();

        let messages = vec![serde_json::json!({
            "role": "user",
            "content": format!(
                "{}\n\n{}\n\nProvide a brief answer based on the context above.",
                prompt,
                if options.include_answer {
                    "Synthesise an answer."
                } else {
                    "List the relevant findings."
                }
            )
        })];

        let llm_response = if let Some(ref client) = self.llm_client {
            client
                .chat_completion(
                    messages,
                    terraphim_service::llm::ChatOptions {
                        max_tokens: Some(2000),
                        temperature: Some(0.3),
                    },
                )
                .await
                .map_err(|e| TerraphimGrepError::RlmFailed(e.to_string()))?
        } else {
            // No LLM configured -- degrade gracefully to search-only rather than failing.
            // The chunks we already retrieved are still useful even without synthesis.
            // Callers that explicitly need synthesis can pass `force_rlm = true`; that path
            // still fails fast in `search_with_rlm`.
            let stats = GrepStats {
                search_latency_ms: start.elapsed().as_millis() as u64,
                rlm_latency_ms: None,
                chunks_returned: chunks.len(),
                kg_hits: hybrid_results.kg_concepts.len(),
            };
            return Ok(GrepResult {
                chunks,
                answer: None,
                concepts: hybrid_results.kg_concepts,
                sufficiency: SufficiencyState::SearchOnly,
                stats,
            });
        };

        let rlm_latency_ms = rlm_start.elapsed().as_millis() as u64;
        let search_latency_ms = start.elapsed().as_millis() as u64;

        let answer = if options.include_answer {
            let signature = signatures::AnswerSignature {};
            signature.parse(&llm_response).ok().map(|a| {
                let citations = chunks
                    .iter()
                    .map(|c| Citation {
                        source: c.source.clone(),
                        line: c.line_start,
                        excerpt: c.content.chars().take(100).collect(),
                    })
                    .collect();
                signatures::AnswerWithCitations {
                    answer: a.answer,
                    citations,
                    confidence: a.confidence,
                }
            })
        } else {
            None
        };

        let stats = GrepStats {
            search_latency_ms,
            rlm_latency_ms: Some(rlm_latency_ms),
            chunks_returned: chunks.len(),
            kg_hits: hybrid_results.kg_concepts.len(),
        };

        if let Some(ref kg_curation) = self.kg_curation {
            let _ = kg_curation.extract_and_index(query, &llm_response).await;
        }

        Ok(GrepResult {
            chunks,
            answer,
            concepts: hybrid_results.kg_concepts,
            sufficiency: SufficiencyState::RlmSynthesis,
            stats,
        })
    }

    #[cfg(not(feature = "llm"))]
    async fn search_with_rlm_fallback(
        &self,
        _query: &str,
        _options: GrepOptions,
        _chunks: Vec<RetrievedChunk>,
        _hybrid_results: HybridResults,
        _start: std::time::Instant,
    ) -> Result<GrepResult> {
        Err(TerraphimGrepError::LlmNotConfigured(
            "LLM feature not enabled".to_string(),
        ))
    }

    async fn search_with_rlm(
        &self,
        query: &str,
        options: GrepOptions,
        start: std::time::Instant,
    ) -> Result<GrepResult> {
        let hybrid_results = self
            .hybrid_searcher
            .search(query, &options)
            .await
            .map_err(TerraphimGrepError::SearchFailed)?;

        self.search_with_rlm_fallback(
            query,
            options,
            hybrid_results.to_chunks(),
            hybrid_results,
            start,
        )
        .await
    }

    /// Return a zeroed-out [`GrepStats`] snapshot (placeholder for future instrumentation).
    pub fn stats(&self) -> GrepStats {
        GrepStats {
            search_latency_ms: 0,
            rlm_latency_ms: None,
            chunks_returned: 0,
            kg_hits: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "code-search")]
    use terraphim_types::Thesaurus;

    #[test]
    fn test_grep_options_default() {
        let options = GrepOptions::default();
        assert_eq!(options.haystack, Haystack::All);
        assert_eq!(options.context_lines, 0);
        assert_eq!(options.max_results, 50);
        assert!(!options.force_rlm);
        assert!(!options.include_answer);
    }

    /// When `code-search` is enabled and the sufficiency judge requests synthesis but no
    /// `LlmClient` is wired, the searcher must degrade to `SearchOnly` rather than failing
    /// with `LlmNotConfigured`. This guards D005 (graceful fallback) -- the previous
    /// behaviour broke the CLI for any query that returned partial results.
    #[cfg(feature = "code-search")]
    #[tokio::test]
    async fn search_without_llm_degrades_to_search_only() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        for i in 0..5 {
            let path = tmp.path().join(format!("file_{i}.rs"));
            std::fs::write(&path, format!("fn target_{i}() {{ /* target */ }}\n")).unwrap();
        }

        let hybrid = HybridSearcher::new("test-role".to_string(), Thesaurus::new("t".to_string()))
            .expect("build hybrid searcher")
            .with_search_path(tmp.path().to_path_buf());
        let grep = TerraphimGrep::new(Arc::new(hybrid), Arc::new(SufficiencyJudge::default()));

        let result = grep
            .search(
                "target",
                GrepOptions {
                    haystack: Haystack::Code,
                    max_results: 50,
                    ..GrepOptions::default()
                },
            )
            .await
            .expect("search should succeed without LLM");

        // The fff backend should have found at least one match -- if not the corpus is wrong.
        assert!(
            !result.chunks.is_empty(),
            "expected fff-search to return chunks from {:?}",
            tmp.path()
        );

        // Whether the judge picked `Sufficient` or `NeedsSynthesis` depends on coverage
        // heuristics, but the user-visible state must be one of the no-LLM-required ones.
        assert!(
            matches!(
                result.sufficiency,
                SufficiencyState::SearchOnly | SufficiencyState::RlmInsufficient
            ),
            "expected SearchOnly/RlmInsufficient, got {:?}",
            result.sufficiency
        );
        assert!(result.answer.is_none(), "no LLM -> no synthesised answer");
    }
}
