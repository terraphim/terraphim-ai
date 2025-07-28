use std::f64;
use std::fmt;
use std::result;

mod names;
mod scored;
mod bm25;
pub mod bm25_additional;
#[cfg(test)]
mod bm25_test;

pub use names::QueryScorer;
use scored::{Scored, SearchResults};
use bm25::{BM25FScorer, BM25PlusScorer};
use bm25_additional::{OkapiBM25Scorer, TFIDFScorer, JaccardScorer, QueryRatioScorer};
use serde::{Serialize, Serializer};

use terraphim_types::Document;
use terraphim_types::SearchQuery;
use crate::ServiceError;

/// Score module local Result type using TerraphimService's error enum.
type Result<T> = std::result::Result<T, ServiceError>;

/// Sort the documents by relevance.
///
/// The `relevance_function` parameter is used to determine how the documents
/// should be sorted.
pub fn sort_documents(query: &Query, documents: Vec<Document>) -> Vec<Document> {
    let mut scorer = Scorer::new().with_similarity(query.similarity);
    
    // Initialize the appropriate scorer based on the query's name_scorer
    match query.name_scorer {
        QueryScorer::BM25 => {
            let bm25_scorer = OkapiBM25Scorer::new();
            scorer = scorer.with_scorer(Box::new(bm25_scorer));
        }
        QueryScorer::BM25F => {
            let bm25f_scorer = BM25FScorer::new();
            scorer = scorer.with_scorer(Box::new(bm25f_scorer));
        }
        QueryScorer::BM25Plus => {
            let bm25plus_scorer = BM25PlusScorer::new();
            scorer = scorer.with_scorer(Box::new(bm25plus_scorer));
        }
        QueryScorer::TFIDF => {
            let tfidf_scorer = TFIDFScorer::new();
            scorer = scorer.with_scorer(Box::new(tfidf_scorer));
        }
        QueryScorer::Jaccard => {
            let jaccard_scorer = JaccardScorer::new();
            scorer = scorer.with_scorer(Box::new(jaccard_scorer));
        }
        QueryScorer::QueryRatio => {
            let query_ratio_scorer = QueryRatioScorer::new();
            scorer = scorer.with_scorer(Box::new(query_ratio_scorer));
        }
        _ => {
            // For OkapiBM25 and other cases, use similarity scoring
        }
    }
    
    match scorer.score_documents(query, documents.clone()) {
        Ok(results) => results.into_iter().map(|scored| scored.into_value()).collect(),
        Err(_) => documents,
    }
}

#[derive(Debug)]
pub struct Scorer {
    similarity: Similarity,
    scorer: Option<Box<dyn std::any::Any>>,
}

impl Scorer {
    pub fn new() -> Scorer {
        Scorer {
            similarity: Similarity::default(),
            scorer: None,
        }
    }

    pub fn with_similarity(mut self, similarity: Similarity) -> Scorer {
        self.similarity = similarity;
        self
    }

    pub fn with_scorer(mut self, scorer: Box<dyn std::any::Any>) -> Scorer {
        self.scorer = Some(scorer);
        self
    }

    /// Execute a search with the given `Query`.
    ///
    /// Generally, the results returned are ranked in relevance order, where
    /// each result has a score associated with it. The score is between
    /// `0` and `1.0` (inclusive), where a score of `1.0` means "most similar"
    /// and a score of `0` means "least similar."
    ///
    /// Depending on the query, the behavior of search can vary:
    ///
    /// * When the query specifies a similarity function, then the results are
    ///   ranked by that function.
    /// * When the query contains a name to search by and a name scorer, then
    ///   results are ranked by the name scorer. If the query specifies a
    ///   similarity function, then results are first ranked by the name
    ///   scorer, and then re-ranked by the similarity function.
    /// * When no name or no name scorer are specified by the query, then
    ///   this search will do a (slow) exhaustive search over all media records
    ///   in IMDb. As a special case, if the query contains a TV show ID, then
    ///   only records in that TV show are searched, and this is generally
    ///   fast.
    /// * If the query is empty, then no results are returned.
    ///
    /// If there was a problem reading the underlying index or the IMDb data,
    /// then an error is returned.
    pub fn score(
        &mut self,
        query: &Query,
        documents: Vec<Document>,
    ) -> Result<SearchResults<Document>> {
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
    ) -> Result<SearchResults<Document>> {
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
            QueryScorer::TFIDF => {
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
                // Fall back to similarity scoring for OkapiBM25 and other cases
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

/// A query that can be used to search records.
///
/// A query typically consists of a fuzzy name query along with zero or more
/// filters.
///
/// A search result must satisfy every filter on a query to match.
///
/// Empty queries always return no results.
///
/// The `Serialize` and `Deserialize` implementations for this type use the
/// free-form query syntax.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Query {
    name: String,
    name_scorer: QueryScorer,
    similarity: Similarity,
    size: usize,
}

impl Query {
    /// Create a new empty query.
    pub fn new(name: &str) -> Query {
        Query {
            name: name.to_string(),
            name_scorer: QueryScorer::default(),
            similarity: Similarity::default(),
            size: 30,
        }
    }

    /// Return true if and only if this query is empty.
    ///
    /// Searching with an empty query always yields no results.
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }

    /// Set the name scorer to use for ranking.
    ///
    /// The name scorer determines which algorithm is used to rank documents.
    pub fn name_scorer(mut self, scorer: QueryScorer) -> Query {
        self.name_scorer = scorer;
        self
    }

    /// Set the similarity function.
    ///
    /// The similarity function can be selected from a predefined set of
    /// choices defined by the
    /// [`Similarity`](enum.Similarity.html) type.
    ///
    /// When a similarity function is used, then any results from searching
    /// the name index are re-ranked according to their similarity with the
    /// query.
    ///
    /// By default, no similarity function is used.
    pub fn similarity(mut self, sim: Similarity) -> Query {
        self.similarity = sim;
        self
    }
}

impl Serialize for Query {
    fn serialize<S>(&self, s: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&self.to_string())
    }
}

// impl<'a> Deserialize<'a> for Query {
//     fn deserialize<D>(d: D) -> result::Result<Query, D::Error>
//     where
//         D: Deserializer<'a>,
//     {
//         use serde::de::Error;

//         let querystr = String::deserialize(d)?;
//         Ok(querystr)
//     }
// }

impl fmt::Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{scorer:{}}}", self.name_scorer)?;
        write!(f, " {{sim:{}}}", self.similarity)?;
        write!(f, " {{size:{}}}", self.size)?;
        write!(f, " {}", self.name)?;
        Ok(())
    }
}

/// A ranking function to use when searching IMDb records.
///
/// A similarity ranking function computes a score between `0.0` and `1.0` (not
/// including `0` but including `1.0`) for a query and a candidate result. The
/// score is determined by the corresponding names for a query and a candidate,
/// and a higher score indicates more similarity.
///
/// This ranking function can be used to increase the precision of a set
/// of results. In particular, when a similarity function is provided to
/// a [`Query`](struct.Query.html), then any results returned by querying
/// the IMDb name index will be rescored according to this function. If no
/// similarity function is provided, then the results will be ranked according
/// to scores produced by the name index.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub enum Similarity {
    /// Do not use a similarity function.
    #[default]
    None,
    /// Computes the Levenshtein edit distance between two names and converts
    /// it to a similarity.
    Levenshtein,
    /// Computes the Jaro edit distance between two names and converts it to a
    /// similarity.
    #[allow(dead_code)]
    Jaro,
    /// Computes the Jaro-Winkler edit distance between two names and converts
    /// it to a similarity.
    #[allow(dead_code)]
    JaroWinkler,
}

impl Similarity {
    /// Computes the similarity between the given strings according to the
    /// underlying similarity function. If no similarity function is present,
    /// then this always returns `1.0`.
    ///
    /// The returned value is always in the range `(0, 1]`.
    pub fn similarity(&self, q1: &str, q2: &str) -> f64 {
        let sim = match *self {
            Similarity::None => 1.0,
            Similarity::Levenshtein => {
                let distance = strsim::levenshtein(q1, q2) as f64;
                // We do a simple conversion of distance to similarity. This
                // will produce very low scores even for very similar names,
                // but callers may normalize scores.
                //
                // We also add `1` to the denominator to avoid division by
                // zero. Incidentally, this causes the similarity of identical
                // strings to be exactly 1.0, which is what we want.
                1.0 / (1.0 + distance)
            }
            Similarity::Jaro => strsim::jaro(q1, q2),
            Similarity::JaroWinkler => strsim::jaro_winkler(q1, q2),
        };
        // Don't permit a score to actually be zero. This prevents division
        // by zero during normalization if all results have a score of zero.
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
