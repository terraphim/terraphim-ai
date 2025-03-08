use std::cmp;
use std::collections::{HashMap, BinaryHeap};
use std::fmt;

use terraphim_types::Document;

/// BM25 parameters
pub struct BM25Params {
    /// k1 parameter controls term frequency saturation
    pub k1: f64,
    /// b parameter controls document length normalization
    pub b: f64,
    /// delta parameter for BM25+ to address the lower-bounding problem
    pub delta: f64,
}

impl Default for BM25Params {
    fn default() -> Self {
        Self {
            k1: 1.2,
            b: 0.75,
            delta: 1.0,
        }
    }
}

/// Field weights for BM25F
pub struct FieldWeights {
    /// Weight for document title
    pub title: f64,
    /// Weight for document body
    pub body: f64,
    /// Weight for document description (if available)
    pub description: f64,
    /// Weight for document tags (if available)
    pub tags: f64,
}

impl Default for FieldWeights {
    fn default() -> Self {
        Self {
            title: 3.0,
            body: 1.0,
            description: 2.0,
            tags: 2.5,
        }
    }
}

/// Okapi BM25 scorer implementation
pub struct OkapiBM25Scorer {
    params: BM25Params,
    avg_doc_length: f64,
    doc_count: usize,
    term_doc_frequencies: HashMap<String, usize>,
}

impl OkapiBM25Scorer {
    /// Create a new Okapi BM25 scorer with default parameters
    pub fn new() -> Self {
        Self {
            params: BM25Params::default(),
            avg_doc_length: 0.0,
            doc_count: 0,
            term_doc_frequencies: HashMap::new(),
        }
    }

    /// Create a new Okapi BM25 scorer with custom parameters
    pub fn with_params(params: BM25Params) -> Self {
        Self {
            params,
            avg_doc_length: 0.0,
            doc_count: 0,
            term_doc_frequencies: HashMap::new(),
        }
    }

    /// Initialize the scorer with a corpus of documents
    pub fn initialize(&mut self, documents: &[Document]) {
        self.doc_count = documents.len();
        
        // Calculate average document length
        let total_length: usize = documents.iter()
            .map(|doc| doc.body.split_whitespace().count())
            .sum();
        
        if self.doc_count > 0 {
            self.avg_doc_length = total_length as f64 / self.doc_count as f64;
        }
        
        // Calculate term document frequencies
        let mut term_doc_frequencies = HashMap::new();
        
        for doc in documents {
            let mut terms = Vec::new();
            
            // Extract terms from document body
            terms.extend(doc.body.split_whitespace().map(|s| s.to_lowercase()));
            
            // Count unique terms in this document
            let mut doc_terms = std::collections::HashSet::new();
            for term in terms {
                doc_terms.insert(term);
            }
            
            // Update term document frequencies
            for term in doc_terms {
                *term_doc_frequencies.entry(term).or_insert(0) += 1;
            }
        }
        
        self.term_doc_frequencies = term_doc_frequencies;
    }

    /// Score a document using Okapi BM25 algorithm
    pub fn score(&self, query: &str, doc: &Document) -> f64 {
        let query_terms: Vec<String> = query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return 0.0;
        }
        
        let mut score = 0.0;
        
        for term in &query_terms {
            // Calculate IDF component
            let n_docs_with_term = self.term_doc_frequencies.get(term).copied().unwrap_or(0);
            if n_docs_with_term == 0 {
                continue;
            }
            
            let idf = f64::ln((self.doc_count as f64 - n_docs_with_term as f64 + 0.5) / 
                             (n_docs_with_term as f64 + 0.5) + 1.0);
            
            // Calculate term frequency
            let tf = count_term_occurrences(&doc.body, term) as f64;
            
            // Calculate document length normalization
            let doc_length = doc.body.split_whitespace().count() as f64;
            let length_norm = 1.0 - self.params.b + self.params.b * (doc_length / self.avg_doc_length);
            
            // Okapi BM25 formula
            let term_score = idf * ((tf * (self.params.k1 + 1.0)) / 
                                  (self.params.k1 * length_norm + tf));
            
            score += term_score;
        }
        
        score
    }
}

/// TFIDF scorer implementation
pub struct TFIDFScorer {
    doc_count: usize,
    term_doc_frequencies: HashMap<String, usize>,
}

impl TFIDFScorer {
    /// Create a new TFIDF scorer
    pub fn new() -> Self {
        Self {
            doc_count: 0,
            term_doc_frequencies: HashMap::new(),
        }
    }

    /// Initialize the scorer with a corpus of documents
    pub fn initialize(&mut self, documents: &[Document]) {
        self.doc_count = documents.len();
        
        // Calculate term document frequencies
        let mut term_doc_frequencies = HashMap::new();
        
        for doc in documents {
            let mut terms = Vec::new();
            
            // Extract terms from document body
            terms.extend(doc.body.split_whitespace().map(|s| s.to_lowercase()));
            
            // Count unique terms in this document
            let mut doc_terms = std::collections::HashSet::new();
            for term in terms {
                doc_terms.insert(term);
            }
            
            // Update term document frequencies
            for term in doc_terms {
                *term_doc_frequencies.entry(term).or_insert(0) += 1;
            }
        }
        
        self.term_doc_frequencies = term_doc_frequencies;
    }

    /// Score a document using TFIDF algorithm
    pub fn score(&self, query: &str, doc: &Document) -> f64 {
        let query_terms: Vec<String> = query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return 0.0;
        }
        
        let mut score = 0.0;
        
        for term in &query_terms {
            // Calculate IDF component
            let n_docs_with_term = self.term_doc_frequencies.get(term).copied().unwrap_or(0);
            if n_docs_with_term == 0 {
                continue;
            }
            
            let idf = f64::ln((self.doc_count as f64) / (n_docs_with_term as f64));
            
            // Calculate term frequency
            let tf = count_term_occurrences(&doc.body, term) as f64;
            
            // TFIDF formula
            let term_score = tf * idf;
            
            score += term_score;
        }
        
        score
    }
}

/// Jaccard similarity scorer implementation
pub struct JaccardScorer {
    doc_count: usize,
}

impl JaccardScorer {
    /// Create a new Jaccard scorer
    pub fn new() -> Self {
        Self {
            doc_count: 0,
        }
    }

    /// Initialize the scorer with a corpus of documents
    pub fn initialize(&mut self, documents: &[Document]) {
        self.doc_count = documents.len();
    }

    /// Score a document using Jaccard similarity
    pub fn score(&self, query: &str, doc: &Document) -> f64 {
        let query_terms: Vec<String> = query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return 0.0;
        }
        
        let doc_terms: Vec<String> = doc.body.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if doc_terms.is_empty() {
            return 0.0;
        }
        
        // Create sets for query and document terms
        let query_set: std::collections::HashSet<&String> = query_terms.iter().collect();
        let doc_set: std::collections::HashSet<&String> = doc_terms.iter().collect();
        
        // Calculate intersection size
        let intersection_size = query_set.intersection(&doc_set).count();
        
        // Calculate union size
        let union_size = query_set.union(&doc_set).count();
        
        // Jaccard similarity formula
        if union_size > 0 {
            intersection_size as f64 / union_size as f64
        } else {
            0.0
        }
    }
}

/// QueryRatio scorer implementation
pub struct QueryRatioScorer {
    doc_count: usize,
}

impl QueryRatioScorer {
    /// Create a new QueryRatio scorer
    pub fn new() -> Self {
        Self {
            doc_count: 0,
        }
    }

    /// Initialize the scorer with a corpus of documents
    pub fn initialize(&mut self, documents: &[Document]) {
        self.doc_count = documents.len();
    }

    /// Score a document using QueryRatio
    pub fn score(&self, query: &str, doc: &Document) -> f64 {
        let query_terms: Vec<String> = query.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if query_terms.is_empty() || self.doc_count == 0 {
            return 0.0;
        }
        
        let doc_terms: Vec<String> = doc.body.split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
        
        if doc_terms.is_empty() {
            return 0.0;
        }
        
        // Create sets for query and document terms
        let query_set: std::collections::HashSet<&String> = query_terms.iter().collect();
        let doc_set: std::collections::HashSet<&String> = doc_terms.iter().collect();
        
        // Calculate intersection size
        let intersection_size = query_set.intersection(&doc_set).count();
        
        // QueryRatio formula
        if query_set.len() > 0 {
            intersection_size as f64 / query_set.len() as f64
        } else {
            0.0
        }
    }
}

/// Count occurrences of a term in a text
fn count_term_occurrences(text: &str, term: &str) -> usize {
    text.to_lowercase()
        .split_whitespace()
        .filter(|word| *word == term)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_okapi_bm25_scorer() {
        let mut scorer = OkapiBM25Scorer::new();
        
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
        
        scorer.initialize(&documents);
        
        // Test scoring
        let score1 = scorer.score("rust programming", &documents[0]);
        let score2 = scorer.score("rust programming", &documents[1]);
        
        // Rust document should score higher for "rust programming" query
        assert!(score1 > score2);
        
        let score1 = scorer.score("python tutorial", &documents[0]);
        let score2 = scorer.score("python tutorial", &documents[1]);
        
        // Python document should score higher for "python tutorial" query
        assert!(score2 > score1);
    }
    
    #[test]
    fn test_tfidf_scorer() {
        let mut scorer = TFIDFScorer::new();
        
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
        
        scorer.initialize(&documents);
        
        // Test scoring
        let score1 = scorer.score("rust programming", &documents[0]);
        let score2 = scorer.score("rust programming", &documents[1]);
        
        // Rust document should score higher for "rust programming" query
        assert!(score1 > score2);
        
        let score1 = scorer.score("python tutorial", &documents[0]);
        let score2 = scorer.score("python tutorial", &documents[1]);
        
        // Python document should score higher for "python tutorial" query
        assert!(score2 > score1);
    }
    
    #[test]
    fn test_jaccard_scorer() {
        let mut scorer = JaccardScorer::new();
        
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
        
        scorer.initialize(&documents);
        
        // Test scoring
        let score1 = scorer.score("rust programming", &documents[0]);
        let score2 = scorer.score("rust programming", &documents[1]);
        
        // Rust document should score higher for "rust programming" query
        assert!(score1 > score2);
        
        let score1 = scorer.score("python tutorial", &documents[0]);
        let score2 = scorer.score("python tutorial", &documents[1]);
        
        // Python document should score higher for "python tutorial" query
        assert!(score2 > score1);
    }
    
    #[test]
    fn test_query_ratio_scorer() {
        let mut scorer = QueryRatioScorer::new();
        
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
        
        scorer.initialize(&documents);
        
        // Test scoring
        let score1 = scorer.score("rust programming", &documents[0]);
        let score2 = scorer.score("rust programming", &documents[1]);
        
        // Rust document should score higher for "rust programming" query
        assert!(score1 > score2);
        
        let score1 = scorer.score("python tutorial", &documents[0]);
        let score2 = scorer.score("python tutorial", &documents[1]);
        
        // Python document should score higher for "python tutorial" query
        assert!(score2 > score1);
    }
} 