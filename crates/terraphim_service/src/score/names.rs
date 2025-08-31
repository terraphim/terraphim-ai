use std::fmt;

use serde::{Deserialize, Serialize};

/// The type of scorer that the name index should use.
///
/// The default is OkapiBM25. If you aren't sure which scorer to use, then
/// stick with the default.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub enum QueryScorer {
    /// OkapiBM25 is a TF-IDF-like ranking function, which takes name length
    /// into account.
    #[default]
    OkapiBM25,
    /// Tfidf is the traditional TF-IDF ranking function, which does not
    /// incorporate document length.
    #[allow(dead_code)]
    Tfidf,
    /// Jaccard is a ranking function determined by computing the similarity
    /// of ngrams between the query and a name in the index. The similarity
    /// is computed by dividing the number of ngrams in common by the total
    /// number of distinct ngrams in both the query and the name combined.
    #[allow(dead_code)]
    Jaccard,
    /// QueryRatio is a ranking function that represents the ratio of query
    /// terms that matched a name. It is computed by dividing the number of
    /// ngrams in common by the total number of ngrams in the query only.
    #[allow(dead_code)]
    QueryRatio,
    /// BM25 is the Okapi BM25 ranking function, which is a probabilistic
    /// relevance function based on term frequency and inverse document frequency.
    #[allow(dead_code)]
    BM25,
    /// BM25F is a fielded version of BM25 that applies different weights
    /// to different document fields (title, body, description, tags).
    #[allow(dead_code)]
    BM25F,
    /// BM25Plus is an enhanced version of BM25 with additional parameters
    /// for fine-tuning the ranking algorithm.
    #[allow(dead_code)]
    BM25Plus,
}

impl QueryScorer {
    /// Returns a list of strings representing the possible scorer values.
    #[allow(dead_code)]
    pub fn possible_names() -> &'static [&'static str] {
        &["okapibm25", "tfidf", "jaccard", "queryratio"]
    }

    /// Return a string representation of this scorer.
    ///
    /// The string returned can be parsed back into a `QueryScorer`.
    pub fn as_str(&self) -> &'static str {
        match *self {
            QueryScorer::OkapiBM25 => "okapibm25",
            QueryScorer::Tfidf => "tfidf",
            QueryScorer::Jaccard => "jaccard",
            QueryScorer::QueryRatio => "queryratio",
            QueryScorer::BM25 => "bm25",
            QueryScorer::BM25F => "bm25f",
            QueryScorer::BM25Plus => "bm25plus",
        }
    }
}

impl fmt::Display for QueryScorer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The style of ngram extraction to use.
///
/// The same style of ngram extraction is always used at index time and at
/// query time.
///
/// Each ngram type uses the ngram size configuration differently.
///
/// All ngram styles used Unicode codepoints as the definition of a character.
/// For example, a 3-gram might contain up to 4 bytes, if it contains 3 Unicode
/// codepoints that each require 4 UTF-8 code units.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum NgramType {
    /// A windowing ngram.
    ///
    /// This is the tradition style of ngram, where sliding window of size
    /// `N` is moved across the entire content to be index. For example, the
    /// 3-grams for the string `homer` are hom, ome and mer.
    #[serde(rename = "window")]
    Window,
    /// An edge ngram.
    ///
    /// This style of ngram produces ever longer ngrams, where each ngram is
    /// anchored to the start of a word. Words are determined simply by
    /// splitting whitespace.
    ///
    /// For example, the edge ngrams of `homer simpson`, where the max ngram
    /// size is 5, would be: hom, home, homer, sim, simp, simps. Generally,
    /// for this ngram type, one wants to use a large maximum ngram size.
    /// Perhaps somewhere close to the maximum number of ngrams in any word
    /// in the corpus.
    ///
    /// Note that there is no way to set the minimum ngram size (which is 3).
    #[serde(rename = "edge")]
    Edge,
}
