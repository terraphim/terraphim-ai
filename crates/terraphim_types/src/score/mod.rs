//! Relevance scoring algorithms for document search.
//!
//! Provides BM25 variants, name-based scoring, and common scoring utilities
//! used by the search layer to rank documents against a query.
/// BM25F and BM25Plus scoring implementations.
pub mod bm25;
/// Okapi BM25, TF-IDF, Jaccard, and query-ratio scorers.
pub mod bm25_additional;
/// Shared scoring traits and helpers.
pub mod common;
/// Query scorer name resolution.
pub mod names;
mod scored;

pub use bm25::{BM25FScorer, BM25PlusScorer};
pub use bm25_additional::{JaccardScorer, OkapiBM25Scorer, QueryRatioScorer, TFIDFScorer};
pub use names::QueryScorer;
pub use scored::{Scored, SearchResults};

use std::f64;
use std::fmt;

use serde::{Serialize, Serializer};

use crate::Document;

/// Scores and sorts `documents` according to the algorithm specified in `query`.
///
/// Falls back to the original order if scoring fails.
pub fn sort_documents(query: &Query, documents: Vec<Document>) -> Vec<Document> {
    let mut scorer = Scorer::new().with_similarity(query.similarity);

    match query.name_scorer {
        QueryScorer::BM25 => {
            let mut bm25_scorer = OkapiBM25Scorer::new();
            bm25_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(bm25_scorer));
        }
        QueryScorer::BM25F => {
            let mut bm25f_scorer = BM25FScorer::new();
            bm25f_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(bm25f_scorer));
        }
        QueryScorer::BM25Plus => {
            let mut bm25plus_scorer = BM25PlusScorer::new();
            bm25plus_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(bm25plus_scorer));
        }
        QueryScorer::Tfidf => {
            let mut tfidf_scorer = TFIDFScorer::new();
            tfidf_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(tfidf_scorer));
        }
        QueryScorer::Jaccard => {
            let mut jaccard_scorer = JaccardScorer::new();
            jaccard_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(jaccard_scorer));
        }
        QueryScorer::QueryRatio => {
            let mut query_ratio_scorer = QueryRatioScorer::new();
            query_ratio_scorer.initialize(&documents);
            scorer = scorer.with_scorer(Box::new(query_ratio_scorer));
        }
        _ => {}
    }

    match scorer.score_documents(query, documents.clone()) {
        Ok(results) => results
            .into_iter()
            .map(|scored| scored.into_value())
            .collect(),
        Err(_) => documents,
    }
}

/// Orchestrates relevance scoring for a set of documents against a query.
#[derive(Debug, Default)]
pub struct Scorer {
    similarity: Similarity,
    scorer: Option<Box<dyn std::any::Any>>,
}

impl Scorer {
    /// Creates a new `Scorer` with default settings.
    pub fn new() -> Scorer {
        Scorer::default()
    }

    /// Sets the string-similarity metric used for title-based fallback scoring.
    pub fn with_similarity(mut self, similarity: Similarity) -> Scorer {
        self.similarity = similarity;
        self
    }

    /// Attaches a concrete scoring implementation (e.g. BM25 scorer).
    pub fn with_scorer(mut self, scorer: Box<dyn std::any::Any>) -> Scorer {
        self.scorer = Some(scorer);
        self
    }

    /// Scores `documents`, trims to `query.size`, normalises, and returns results.
    pub fn score(
        &mut self,
        query: &Query,
        documents: Vec<Document>,
    ) -> Result<SearchResults<Document>, ScoreError> {
        if query.is_empty() {
            return Ok(SearchResults::new());
        }
        let mut results = self.score_documents(query, documents)?;
        results.trim(query.size);
        results.normalize();
        Ok(results)
    }

    fn score_documents(
        &mut self,
        query: &Query,
        documents: Vec<Document>,
    ) -> Result<SearchResults<Document>, ScoreError> {
        let mut results = SearchResults::new();
        for document in documents {
            results.push(Scored::new(document));
        }

        match query.name_scorer {
            QueryScorer::BM25 => {
                if let Some(scorer) = &self.scorer {
                    if let Some(bm25_scorer) = scorer.downcast_ref::<OkapiBM25Scorer>() {
                        results.rescore(|document| bm25_scorer.score(&query.name, document));
                    }
                }
            }
            QueryScorer::BM25F => {
                if let Some(scorer) = &self.scorer {
                    if let Some(bm25f_scorer) = scorer.downcast_ref::<BM25FScorer>() {
                        results.rescore(|document| bm25f_scorer.score(&query.name, document));
                    }
                }
            }
            QueryScorer::BM25Plus => {
                if let Some(scorer) = &self.scorer {
                    if let Some(bm25plus_scorer) = scorer.downcast_ref::<BM25PlusScorer>() {
                        results.rescore(|document| bm25plus_scorer.score(&query.name, document));
                    }
                }
            }
            QueryScorer::Tfidf => {
                if let Some(scorer) = &self.scorer {
                    if let Some(tfidf_scorer) = scorer.downcast_ref::<TFIDFScorer>() {
                        results.rescore(|document| tfidf_scorer.score(&query.name, document));
                    }
                }
            }
            QueryScorer::Jaccard => {
                if let Some(scorer) = &self.scorer {
                    if let Some(jaccard_scorer) = scorer.downcast_ref::<JaccardScorer>() {
                        results.rescore(|document| jaccard_scorer.score(&query.name, document));
                    }
                }
            }
            QueryScorer::QueryRatio => {
                if let Some(scorer) = &self.scorer {
                    if let Some(query_ratio_scorer) = scorer.downcast_ref::<QueryRatioScorer>() {
                        results.rescore(|document| query_ratio_scorer.score(&query.name, document));
                    }
                }
            }
            _ => {
                log::debug!("Similarity {:?}", query.similarity);
                log::debug!("Query {:?}", query);
                results.rescore(|document| self.similarity(query, &document.title));
            }
        }

        log::debug!("results after rescoring: {:#?}", results);
        Ok(results)
    }

    fn similarity(&self, query: &Query, name: &str) -> f64 {
        log::debug!("Similarity {:?}", query.similarity);
        log::debug!("Query {:?}", query);
        log::debug!("Name {:?}", name);
        let result = query.similarity.similarity(&query.name, name);
        log::debug!("Similarity calculation {:?}", result);
        result
    }
}

/// Errors that can occur during document scoring.
#[derive(Debug, thiserror::Error)]
pub enum ScoreError {
    /// A scorer-specific error with a descriptive message.
    #[error("scoring error: {0}")]
    Scoring(String),
}

/// A search query with scoring and similarity configuration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Query {
    /// The query term to score documents against.
    pub name: String,
    /// Which relevance algorithm to apply.
    pub name_scorer: QueryScorer,
    /// String-similarity metric for title-based fallback.
    pub similarity: Similarity,
    /// Maximum number of results to return.
    pub size: usize,
}

impl Query {
    /// Creates a `Query` for `name` with default scorer, similarity, and size (30).
    pub fn new(name: &str) -> Query {
        Query {
            name: name.to_string(),
            name_scorer: QueryScorer::default(),
            similarity: Similarity::default(),
            size: 30,
        }
    }

    /// Returns `true` if the query name is empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// Sets the relevance scoring algorithm.
    pub fn name_scorer(mut self, scorer: QueryScorer) -> Query {
        self.name_scorer = scorer;
        self
    }

    /// Sets the string-similarity metric.
    pub fn similarity(mut self, sim: Similarity) -> Query {
        self.similarity = sim;
        self
    }
}

impl Serialize for Query {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{scorer:{}}}", self.name_scorer)?;
        write!(f, " {{sim:{}}}", self.similarity)?;
        write!(f, " {{size:{}}}", self.size)?;
        write!(f, " {}", self.name)?;
        Ok(())
    }
}

/// String-similarity metric used to rank documents by title when no field scorer is active.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub enum Similarity {
    /// No similarity adjustment; all documents receive a score of 1.0.
    #[default]
    None,
    /// Normalised Levenshtein edit distance (lower distance = higher score).
    Levenshtein,
    /// Jaro similarity coefficient.
    Jaro,
    /// Jaro–Winkler similarity, boosting common prefixes.
    JaroWinkler,
}

impl Similarity {
    /// Computes a similarity score in `(0.0, 1.0]` between `q1` and `q2`.
    pub fn similarity(&self, q1: &str, q2: &str) -> f64 {
        let sim = match *self {
            Similarity::None => 1.0,
            Similarity::Levenshtein => {
                let distance = strsim::levenshtein(q1, q2) as f64;
                1.0 / (1.0 + distance)
            }
            Similarity::Jaro => strsim::jaro(q1, q2),
            Similarity::JaroWinkler => strsim::jaro_winkler(q1, q2),
        };
        if sim < f64::EPSILON {
            f64::EPSILON
        } else {
            sim
        }
    }
}

impl fmt::Display for Similarity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Similarity::None => write!(f, "none"),
            Similarity::Levenshtein => write!(f, "levenshtein"),
            Similarity::Jaro => write!(f, "jaro"),
            Similarity::JaroWinkler => write!(f, "jarowinkler"),
        }
    }
}
