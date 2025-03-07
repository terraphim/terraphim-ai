use std::f64;
use std::fmt;
use std::result;

mod names;
mod scored;
mod bm25;

use crate::error::Result;
use names::NameScorer;
use scored::{Scored, SearchResults};
use serde::{Serialize, Serializer};
use bm25::{BM25FScorer, BM25PlusScorer};

use terraphim_types::{Document, RelevanceFunction, SearchQuery};

/// Sort the documents by relevance.
///
/// The `relevance_function` parameter is used to determine how the documents
/// should be sorted.
pub fn sort_documents(search_query: &SearchQuery, documents: Vec<Document>) -> Vec<Document> {
    log::debug!("Sorting documents by relevance");

    // Determine which relevance function to use
    let relevance_function = search_query.role.as_ref()
        .and_then(|_| None) // In the future, this could look up the role's configured relevance function
        .unwrap_or(RelevanceFunction::TitleScorer);

    match relevance_function {
        RelevanceFunction::TerraphimGraph => {
            // Use the existing Terraphim graph scoring logic
            // This is likely handled elsewhere in the codebase
            documents
        },
        RelevanceFunction::TitleScorer => {
            // Use the existing title scorer
            let mut scorer = Scorer::new();
            let query = Query::new(&search_query.search_term.as_str()).similarity(Similarity::Levenshtein);
            let mut results = scorer.score(&query, documents).unwrap();
            results.rescore(|doc| query.similarity.similarity(&query.name, &doc.title));
            log::debug!("Rescore results {:#?}", results);
            results
                .into_vec()
                .iter()
                .map(|s| s.clone().into_value())
                .collect()
        },
        RelevanceFunction::BM25F => {
            // Use BM25F scorer
            let mut scorer = BM25FScorer::new();
            scorer.initialize(&documents);
            
            // Score each document
            let mut scored_docs: Vec<(f64, Document)> = documents.into_iter()
                .map(|doc| {
                    let score = scorer.score(&search_query.search_term.as_str(), &doc);
                    (score, doc)
                })
                .collect();
            
            // Sort by score in descending order
            scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            
            // Return documents in order of relevance
            scored_docs.into_iter().map(|(_, doc)| doc).collect()
        },
        RelevanceFunction::BM25Plus => {
            // Use BM25Plus scorer
            let mut scorer = BM25PlusScorer::new();
            scorer.initialize(&documents);
            
            // Score each document
            let mut scored_docs: Vec<(f64, Document)> = documents.into_iter()
                .map(|doc| {
                    let score = scorer.score(&search_query.search_term.as_str(), &doc);
                    (score, doc)
                })
                .collect();
            
            // Sort by score in descending order
            scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            
            // Return documents in order of relevance
            scored_docs.into_iter().map(|(_, doc)| doc).collect()
        },
    }
}

/// Rescore documents from multiple haystacks
///
/// This function takes documents from multiple sources (haystacks) and rescores them
/// using the specified relevance function.
pub fn rescore_documents(search_query: &SearchQuery, documents: Vec<Document>, relevance_function: RelevanceFunction) -> Vec<Document> {
    log::debug!("Rescoring documents from multiple haystacks");
    
    match relevance_function {
        RelevanceFunction::TerraphimGraph => {
            // Use the existing Terraphim graph scoring logic
            // This is likely handled elsewhere in the codebase
            documents
        },
        RelevanceFunction::TitleScorer => {
            // Use the existing title scorer
            let mut scorer = Scorer::new();
            let query = Query::new(&search_query.search_term.as_str()).similarity(Similarity::Levenshtein);
            let mut results = scorer.score(&query, documents).unwrap();
            results.rescore(|doc| query.similarity.similarity(&query.name, &doc.title));
            log::debug!("Rescore results {:#?}", results);
            results
                .into_vec()
                .iter()
                .map(|s| s.clone().into_value())
                .collect()
        },
        RelevanceFunction::BM25F => {
            // Use BM25F scorer
            let mut scorer = BM25FScorer::new();
            scorer.initialize(&documents);
            
            // Score each document
            let mut scored_docs: Vec<(f64, Document)> = documents.into_iter()
                .map(|doc| {
                    let score = scorer.score(&search_query.search_term.as_str(), &doc);
                    (score, doc)
                })
                .collect();
            
            // Sort by score in descending order
            scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            
            // Return documents in order of relevance
            scored_docs.into_iter().map(|(_, doc)| doc).collect()
        },
        RelevanceFunction::BM25Plus => {
            // Use BM25Plus scorer
            let mut scorer = BM25PlusScorer::new();
            scorer.initialize(&documents);
            
            // Score each document
            let mut scored_docs: Vec<(f64, Document)> = documents.into_iter()
                .map(|doc| {
                    let score = scorer.score(&search_query.search_term.as_str(), &doc);
                    (score, doc)
                })
                .collect();
            
            // Sort by score in descending order
            scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            
            // Return documents in order of relevance
            scored_docs.into_iter().map(|(_, doc)| doc).collect()
        },
    }
}

#[derive(Debug)]
pub struct Scorer {}

impl Scorer {
    pub fn new() -> Scorer {
        Scorer {}
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
        log::debug!("Similarity {:?}", query.similarity);
        log::debug!("Query {:?}", query);
        results.rescore(|document| self.similarity(query, &document.title));
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
    name_scorer: NameScorer,
    similarity: Similarity,
    size: usize,
}

impl Query {
    /// Create a new empty query.
    pub fn new(name: &str) -> Query {
        Query {
            name: name.to_string(),
            name_scorer: NameScorer::default(),
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
    Jaro,
    /// Computes the Jaro-Winkler edit distance between two names and converts
    /// it to a similarity.
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

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::{NormalizedTermValue, RoleName};

    #[test]
    fn test_sort_documents_with_different_relevance_functions() {
        // Create test documents
        let documents = vec![
            Document {
                id: "1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "Rust Programming Language".to_string(),
                body: "Rust is a systems programming language focused on safety, speed, and concurrency.".to_string(),
                description: Some("Learn about Rust programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "systems".to_string()]),
                rank: None,
            },
            Document {
                id: "2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "Python Programming Tutorial".to_string(),
                body: "Python is a high-level programming language known for its readability.".to_string(),
                description: Some("Learn Python programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "tutorial".to_string()]),
                rank: None,
            },
        ];

        // Create search query
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new("rust programming".to_string()),
            skip: None,
            limit: None,
            role: Some(RoleName::new("developer")),
        };

        // Test TitleScorer
        let title_results = rescore_documents(&search_query, documents.clone(), RelevanceFunction::TitleScorer);
        assert_eq!(title_results.len(), 2);
        // The first document should be the Rust one since the query is "rust programming"
        assert_eq!(title_results[0].id, "1");

        // Test BM25F
        let bm25f_results = rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25F);
        assert_eq!(bm25f_results.len(), 2);
        // The first document should be the Rust one since the query is "rust programming"
        assert_eq!(bm25f_results[0].id, "1");

        // Test BM25Plus
        let bm25plus_results = rescore_documents(&search_query, documents.clone(), RelevanceFunction::BM25Plus);
        assert_eq!(bm25plus_results.len(), 2);
        // The first document should be the Rust one since the query is "rust programming"
        assert_eq!(bm25plus_results[0].id, "1");
    }

    #[test]
    fn test_rescore_documents_from_multiple_haystacks() {
        // Create test documents from multiple haystacks
        let haystack1_docs = vec![
            Document {
                id: "1".to_string(),
                url: "http://example.com/1".to_string(),
                title: "Rust Programming Language".to_string(),
                body: "Rust is a systems programming language focused on safety, speed, and concurrency.".to_string(),
                description: Some("Learn about Rust programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "systems".to_string()]),
                rank: None,
            },
        ];

        let haystack2_docs = vec![
            Document {
                id: "2".to_string(),
                url: "http://example.com/2".to_string(),
                title: "Python Programming Tutorial".to_string(),
                body: "Python is a high-level programming language known for its readability.".to_string(),
                description: Some("Learn Python programming".to_string()),
                stub: None,
                tags: Some(vec!["programming".to_string(), "tutorial".to_string()]),
                rank: None,
            },
        ];

        // Combine documents from multiple haystacks
        let mut combined_docs = Vec::new();
        combined_docs.extend(haystack1_docs);
        combined_docs.extend(haystack2_docs);

        // Create search query
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new("rust programming".to_string()),
            skip: None,
            limit: None,
            role: Some(RoleName::new("developer")),
        };

        // Test rescoring with BM25F
        let rescored_docs = rescore_documents(&search_query, combined_docs.clone(), RelevanceFunction::BM25F);
        assert_eq!(rescored_docs.len(), 2);
        // The first document should be the Rust one since the query is "rust programming"
        assert_eq!(rescored_docs[0].id, "1");

        // Test with a different query that should favor the Python document
        let python_query = SearchQuery {
            search_term: NormalizedTermValue::new("python tutorial".to_string()),
            skip: None,
            limit: None,
            role: Some(RoleName::new("developer")),
        };

        let rescored_docs = rescore_documents(&python_query, combined_docs.clone(), RelevanceFunction::BM25F);
        assert_eq!(rescored_docs.len(), 2);
        // The first document should be the Python one since the query is "python tutorial"
        assert_eq!(rescored_docs[0].id, "2");
    }
}
