pub mod error;
pub mod hybrid_searcher;
pub mod kg_curation;
pub mod rlm_context;
pub mod signatures;
pub mod sufficiency_judge;

use std::sync::Arc;

pub use error::{Result, TerraphimGrepError};
pub use hybrid_searcher::{GrepOptions, Haystack, HybridResults, HybridSearcher, KgConcept, RetrievedChunk};
pub use kg_curation::KgCurationRlm;
pub use rlm_context::RlmContext;
pub use signatures::{AnswerWithCitations, Citation, Match, NewConcept, RlmSignature};
pub use sufficiency_judge::{HeuristicThresholds, Sufficiency, SufficiencyJudge};

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepResult {
    pub chunks: Vec<RetrievedChunk>,
    pub answer: Option<AnswerWithCitations>,
    pub concepts: Vec<KgConcept>,
    pub sufficiency: SufficiencyState,
    pub stats: GrepStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum SufficiencyState {
    SearchOnly,
    RlmSynthesis,
    RlmInsufficient,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GrepStats {
    pub search_latency_ms: u64,
    pub rlm_latency_ms: Option<u64>,
    pub chunks_returned: usize,
    pub kg_hits: usize,
}

pub struct TerraphimGrep {
    hybrid_searcher: Arc<HybridSearcher>,
    sufficiency_judge: Arc<SufficiencyJudge>,
    #[cfg(feature = "llm")]
    kg_curation: Option<Arc<KgCurationRlm>>,
    #[cfg(feature = "llm")]
    llm_client: Option<Arc<dyn terraphim_service::llm::LlmClient>>,
}

impl TerraphimGrep {
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

    #[cfg(feature = "llm")]
    pub fn with_kg_curation(mut self, kg_curation: Arc<KgCurationRlm>) -> Self {
        self.kg_curation = Some(kg_curation);
        self
    }

    #[cfg(feature = "llm")]
    pub fn with_llm_client(
        mut self,
        llm_client: Arc<dyn terraphim_service::llm::LlmClient>,
    ) -> Self {
        self.llm_client = Some(llm_client);
        self
    }

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
            return Err(TerraphimGrepError::LlmNotConfigured(
                "LLM client not configured".to_string(),
            ));
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

    #[test]
    fn test_grep_options_default() {
        let options = GrepOptions::default();
        assert_eq!(options.haystack, Haystack::All);
        assert_eq!(options.context_lines, 0);
        assert_eq!(options.max_results, 50);
        assert!(!options.force_rlm);
        assert!(!options.include_answer);
    }
}
