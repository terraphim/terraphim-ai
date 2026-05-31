//! Scoring algorithms and query types for document relevance ranking.
//!
//! This module provides BM25 variants, similarity metrics, and the `Scorer` / `Query`
//! types used to rank [`Document`](crate::Document) results.

/// BM25F multi-field relevance scoring implementation.
pub mod bm25;
/// Additional BM25 variants: Okapi BM25, BM25+, TF-IDF, Jaccard, and QueryRatio scorers.
pub mod bm25_additional;
/// Common scoring traits and utilities shared across scorer implementations.
pub mod common;
/// Query scorer selector enum mapping scorer names to implementations.
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

/// Sorts `documents` by relevance to `query` using the scorer specified in the query.
///
/// Falls back to the original ordering if scoring fails.
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

/// Configurable document scorer combining a similarity metric with an optional BM25 variant.
#[derive(Debug, Default)]
pub struct Scorer {
    similarity: Similarity,
    scorer: Option<Box<dyn std::any::Any>>,
}

impl Scorer {
    /// Creates a default scorer (no similarity, no BM25).
    pub fn new() -> Scorer {
        Scorer::default()
    }

    /// Sets the string similarity algorithm used for title matching.
    pub fn with_similarity(mut self, similarity: Similarity) -> Scorer {
        self.similarity = similarity;
        self
    }

    /// Attaches a BM25 scorer (or any other `Any`-based scorer).
    pub fn with_scorer(mut self, scorer: Box<dyn std::any::Any>) -> Scorer {
        self.scorer = Some(scorer);
        self
    }

    /// Scores `documents` against `query`, trims to the requested result size, and normalises scores.
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

/// Error type returned when document scoring fails.
#[derive(Debug, thiserror::Error)]
pub enum ScoreError {
    /// The underlying scorer returned an error.
    #[error("scoring error: {0}")]
    Scoring(String),
}

/// Search query carrying the term, scoring algorithm, similarity metric, and result size.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Query {
    /// The raw search term.
    pub name: String,
    /// Which BM25 variant (or other scorer) to use.
    pub name_scorer: QueryScorer,
    /// Optional string similarity metric applied to title matching.
    pub similarity: Similarity,
    /// Maximum number of results to return.
    pub size: usize,
}

impl Query {
    /// Creates a query for `name` with default scorer, no similarity, and a limit of 30 results.
    pub fn new(name: &str) -> Query {
        Query {
            name: name.to_string(),
            name_scorer: QueryScorer::default(),
            similarity: Similarity::default(),
            size: 30,
        }
    }

    /// Returns `true` when the query term is the empty string.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// Sets the scorer algorithm and returns the updated query.
    pub fn name_scorer(mut self, scorer: QueryScorer) -> Query {
        self.name_scorer = scorer;
        self
    }

    /// Sets the similarity metric and returns the updated query.
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

/// String similarity algorithm applied to title comparisons.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub enum Similarity {
    /// No similarity: always returns 1.0 (treats all titles as equally relevant).
    #[default]
    None,
    /// Levenshtein edit-distance converted to a similarity score in (0, 1].
    Levenshtein,
    /// Jaro similarity in [0, 1].
    Jaro,
    /// Jaro-Winkler similarity (boosts common-prefix matches) in [0, 1].
    JaroWinkler,
}

impl Similarity {
    /// Computes the similarity score between `q1` and `q2` using this metric.
    ///
    /// Returns a value in (0, 1]. Never returns exactly 0 — floored at `f64::EPSILON`.
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
