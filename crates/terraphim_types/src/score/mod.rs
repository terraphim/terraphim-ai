//! Relevance scoring infrastructure for document search.
//!
//! Provides BM25 variants, Jaccard, TF-IDF, and query-ratio scorers, plus
//! a `Scorer` orchestrator that dispatches to the chosen algorithm.

/// BM25F and BM25Plus scorer implementations.
pub mod bm25;
/// Additional scorers: Okapi BM25, TF-IDF, Jaccard, and QueryRatio.
pub mod bm25_additional;
/// Shared scoring helpers and traits.
pub mod common;
/// [`QueryScorer`] enum and display logic.
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

/// Sorts `documents` by relevance to `query` in descending order.
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

/// Orchestrates relevance scoring over a collection of documents.
#[derive(Debug, Default)]
pub struct Scorer {
    similarity: Similarity,
    scorer: Option<Box<dyn std::any::Any>>,
}

impl Scorer {
    /// Create a new `Scorer` with default settings.
    pub fn new() -> Scorer {
        Scorer::default()
    }

    /// Set the string similarity metric (builder method).
    pub fn with_similarity(mut self, similarity: Similarity) -> Scorer {
        self.similarity = similarity;
        self
    }

    /// Attach a concrete scoring algorithm (builder method).
    pub fn with_scorer(mut self, scorer: Box<dyn std::any::Any>) -> Scorer {
        self.scorer = Some(scorer);
        self
    }

    /// Score `documents` against `query`, trim to `query.size`, and normalise.
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

/// Error returned when scoring fails.
#[derive(Debug, thiserror::Error)]
pub enum ScoreError {
    /// A scorer reported an error during computation.
    #[error("scoring error: {0}")]
    Scoring(String),
}

/// Describes a search query including the scoring algorithm and size limits.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Query {
    /// The raw query string to match against document titles.
    pub name: String,
    /// Which scoring algorithm to apply.
    pub name_scorer: QueryScorer,
    /// String similarity metric used for fuzzy title matching.
    pub similarity: Similarity,
    /// Maximum number of documents to return.
    pub size: usize,
}

impl Query {
    /// Create a new query with default scorer and similarity settings.
    pub fn new(name: &str) -> Query {
        Query {
            name: name.to_string(),
            name_scorer: QueryScorer::default(),
            similarity: Similarity::default(),
            size: 30,
        }
    }

    /// Return `true` when the query string is empty.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// Override the scoring algorithm (builder method).
    pub fn name_scorer(mut self, scorer: QueryScorer) -> Query {
        self.name_scorer = scorer;
        self
    }

    /// Override the similarity metric (builder method).
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

/// String similarity metric applied to document titles during scoring.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub enum Similarity {
    /// No similarity weighting; all titles score equally.
    #[default]
    None,
    /// Levenshtein edit-distance similarity (lower distance → higher score).
    Levenshtein,
    /// Jaro similarity.
    Jaro,
    /// Jaro-Winkler similarity (boosts prefix matches).
    JaroWinkler,
}

impl Similarity {
    /// Compute the similarity score between two strings in the range (0.0, 1.0].
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
