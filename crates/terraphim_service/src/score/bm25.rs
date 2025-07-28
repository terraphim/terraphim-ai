use std::collections::HashMap;
use std::f64;

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

/// BM25F scorer implementation
pub struct BM25FScorer {
    params: BM25Params,
    weights: FieldWeights,
    avg_doc_length: f64,
    doc_count: usize,
    term_doc_frequencies: HashMap<String, usize>,
}

impl BM25FScorer {
    /// Create a new BM25F scorer with default parameters
    pub fn new() -> Self {
        Self {
            params: BM25Params::default(),
            weights: FieldWeights::default(),
            avg_doc_length: 0.0,
            doc_count: 0,
            term_doc_frequencies: HashMap::new(),
        }
    }

    /// Create a new BM25F scorer with custom parameters
    pub fn with_params(params: BM25Params, weights: FieldWeights) -> Self {
        Self {
            params,
            weights,
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
            .map(|doc| {
                let title_len = doc.title.split_whitespace().count();
                let body_len = doc.body.split_whitespace().count();
                let desc_len = doc.description.as_ref().map_or(0, |d| d.split_whitespace().count());
                let tags_len = doc.tags.as_ref().map_or(0, |t| t.iter().map(|tag| tag.split_whitespace().count()).sum());
                
                title_len + body_len + desc_len + tags_len
            })
            .sum();
        
        if self.doc_count > 0 {
            self.avg_doc_length = total_length as f64 / self.doc_count as f64;
        }
        
        // Calculate term document frequencies
        let mut term_doc_frequencies = HashMap::new();
        
        for doc in documents {
            let mut terms = Vec::new();
            
            // Extract terms from all fields
            terms.extend(doc.title.split_whitespace().map(|s| s.to_lowercase()));
            terms.extend(doc.body.split_whitespace().map(|s| s.to_lowercase()));
            
            if let Some(desc) = &doc.description {
                terms.extend(desc.split_whitespace().map(|s| s.to_lowercase()));
            }
            
            if let Some(tags) = &doc.tags {
                for tag in tags {
                    terms.extend(tag.split_whitespace().map(|s| s.to_lowercase()));
                }
            }
            
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

    /// Score a document using BM25F algorithm
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
            
            // Calculate weighted term frequency across all fields
            let mut weighted_tf = 0.0;
            
            // Title field
            let title_tf = count_term_occurrences(&doc.title, term);
            weighted_tf += self.weights.title * title_tf as f64;
            
            // Body field
            let body_tf = count_term_occurrences(&doc.body, term);
            weighted_tf += self.weights.body * body_tf as f64;
            
            // Description field (if available)
            if let Some(desc) = &doc.description {
                let desc_tf = count_term_occurrences(desc, term);
                weighted_tf += self.weights.description * desc_tf as f64;
            }
            
            // Tags field (if available)
            if let Some(tags) = &doc.tags {
                for tag in tags {
                    let tag_tf = count_term_occurrences(tag, term);
                    weighted_tf += self.weights.tags * tag_tf as f64;
                }
            }
            
            // Calculate document length normalization
            let doc_length = doc.title.split_whitespace().count() + 
                            doc.body.split_whitespace().count() +
                            doc.description.as_ref().map_or(0, |d| d.split_whitespace().count()) +
                            doc.tags.as_ref().map_or(0, |t| t.iter().map(|tag| tag.split_whitespace().count()).sum());
            
            let length_norm = 1.0 - self.params.b + self.params.b * (doc_length as f64 / self.avg_doc_length);
            
            // BM25F formula
            let term_score = idf * (weighted_tf / (self.params.k1 * length_norm + weighted_tf));
            score += term_score;
        }
        
        score
    }
}

/// BM25+ scorer implementation
pub struct BM25PlusScorer {
    params: BM25Params,
    avg_doc_length: f64,
    doc_count: usize,
    term_doc_frequencies: HashMap<String, usize>,
}

impl BM25PlusScorer {
    /// Create a new BM25+ scorer with default parameters
    pub fn new() -> Self {
        Self {
            params: BM25Params::default(),
            avg_doc_length: 0.0,
            doc_count: 0,
            term_doc_frequencies: HashMap::new(),
        }
    }

    /// Create a new BM25+ scorer with custom parameters
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

    /// Score a document using BM25+ algorithm
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
            
            // BM25+ formula (adds delta parameter to address lower-bounding problem)
            let term_score = idf * ((tf * (self.params.k1 + 1.0)) / 
                                  (self.params.k1 * length_norm + tf) + 
                                  self.params.delta);
            
            score += term_score;
        }
        
        score
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
    fn test_bm25f_scorer() {
        let mut scorer = BM25FScorer::new();
        
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
    fn test_bm25plus_scorer() {
        let mut scorer = BM25PlusScorer::new();
        
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
    fn test_count_term_occurrences() {
        let text = "Rust is a systems programming language. Rust is safe and fast.";
        
        assert_eq!(count_term_occurrences(text, "rust"), 2);
        assert_eq!(count_term_occurrences(text, "is"), 2);
        assert_eq!(count_term_occurrences(text, "programming"), 1);
        assert_eq!(count_term_occurrences(text, "python"), 0);
    }
} 