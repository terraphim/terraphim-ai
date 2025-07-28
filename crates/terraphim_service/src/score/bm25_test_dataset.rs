use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use terraphim_types::Document;

use super::bm25::{BM25FScorer, BM25PlusScorer, BM25Params, FieldWeights};

/// Test document structure from the test dataset
#[derive(Debug, Deserialize, Serialize)]
struct TestDocument {
    id: String,
    url: String,
    title: String,
    body: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
    rank: Option<u64>,
}

/// Test query structure from the test dataset
#[derive(Debug, Deserialize, Serialize)]
struct TestQuery {
    id: String,
    query: String,
    expected_results: Option<HashMap<String, Vec<String>>>,
    description: String,
}

/// Test dataset structure
#[derive(Debug, Deserialize, Serialize)]
struct TestDataset {
    documents: Vec<TestDocument>,
}

/// Queries dataset structure
#[derive(Debug, Deserialize, Serialize)]
struct QueriesDataset {
    queries: Vec<TestQuery>,
}

/// Convert a test document to a terraphim document
fn convert_test_document(doc: &TestDocument) -> Document {
    Document {
        id: doc.id.clone(),
        url: doc.url.clone(),
        title: doc.title.clone(),
        body: doc.body.clone(),
        description: doc.description.clone(),
        stub: None,
        tags: doc.tags.clone(),
        rank: doc.rank,
    }
}

/// Load test documents from a JSON file
fn load_test_documents(file_path: &str) -> Vec<Document> {
    let path = Path::new(file_path);
    let mut file = File::open(path).expect("Failed to open test dataset file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read test dataset file");
    
    let dataset: TestDataset = serde_json::from_str(&contents).expect("Failed to parse test dataset");
    
    dataset.documents.iter()
        .map(|doc| convert_test_document(doc))
        .collect()
}

/// Load test queries from a JSON file
fn load_test_queries(file_path: &str) -> Vec<TestQuery> {
    let path = Path::new(file_path);
    let mut file = File::open(path).expect("Failed to open queries file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read queries file");
    
    let dataset: QueriesDataset = serde_json::from_str(&contents).expect("Failed to parse queries dataset");
    
    dataset.queries
}

/// Score documents using BM25F and return them sorted by score
fn score_documents_bm25f(query: &str, documents: &[Document]) -> Vec<(String, f64)> {
    let mut scorer = BM25FScorer::new();
    scorer.initialize(documents);
    
    let mut scored_docs: Vec<(String, f64)> = documents.iter()
        .map(|doc| {
            let score = scorer.score(query, doc);
            (doc.id.clone(), score)
        })
        .collect();
    
    // Sort by score in descending order
    scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    scored_docs
}

/// Score documents using BM25Plus and return them sorted by score
fn score_documents_bm25plus(query: &str, documents: &[Document]) -> Vec<(String, f64)> {
    let mut scorer = BM25PlusScorer::new();
    scorer.initialize(documents);
    
    let mut scored_docs: Vec<(String, f64)> = documents.iter()
        .map(|doc| {
            let score = scorer.score(query, doc);
            (doc.id.clone(), score)
        })
        .collect();
    
    // Sort by score in descending order
    scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    
    scored_docs
}

/// Compare the ranking of documents with the expected ranking
fn compare_rankings(actual: &[(String, f64)], expected: &[String]) -> bool {
    if actual.len() < expected.len() {
        return false;
    }
    
    // Check if all expected documents are in the top results
    // (not necessarily in the exact same order)
    let actual_top_n: Vec<&String> = actual.iter()
        .take(expected.len())
        .map(|(id, _)| id)
        .collect();
    
    for expected_id in expected {
        if !actual_top_n.contains(&expected_id) {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper function to get the document path
    fn get_document_path() -> String {
        let base_path = env!("CARGO_MANIFEST_DIR");
        format!("{}/../../docs/en/test_data/bm25_test_dataset/documents.json", base_path)
    }
    
    // Helper function to get the queries path
    fn get_queries_path() -> String {
        let base_path = env!("CARGO_MANIFEST_DIR");
        format!("{}/../../docs/en/test_data/bm25_test_dataset/queries.json", base_path)
    }
    
    #[test]
    fn test_bm25_key_characteristics() {
        // Test key characteristics of BM25F and BM25Plus instead of exact rankings
        
        // 1. Test that BM25F gives more weight to matches in title fields
        test_field_weighting_in_bm25f();
        
        // 2. Test that BM25+ handles rare terms better
        test_rare_terms_in_bm25plus();
        
        // 3. Test that both algorithms normalize document length
        test_document_length_normalization();
        
        // 4. Test that both algorithms handle term frequency saturation
        test_term_frequency_saturation();
        
        // 5. Test that both algorithms rank relevant documents higher
        let documents = load_test_documents(&get_document_path());
        
        // Test BM25F on a simple query
        {
            let query = "rust programming";
            let scored_docs = score_documents_bm25f(query, &documents);
            let top_docs: Vec<&String> = scored_docs.iter().take(2).map(|(id, _)| id).collect();
            
            println!("BM25F top docs for 'rust programming': {:?}", top_docs);
            
            // Check that doc1 and doc5 are in the top results (they're about Rust)
            assert!(
                top_docs.contains(&&"doc1".to_string()) && top_docs.contains(&&"doc5".to_string()),
                "BM25F should rank doc1 and doc5 in the top results for 'rust programming'"
            );
        }
        
        // Test BM25+ on a simple query
        {
            let query = "rust programming";
            let scored_docs = score_documents_bm25plus(query, &documents);
            let top_docs: Vec<&String> = scored_docs.iter().take(2).map(|(id, _)| id).collect();
            
            println!("BM25+ top docs for 'rust programming': {:?}", top_docs);
            
            // Check that doc1 and doc5 are in the top results (they're about Rust)
            assert!(
                top_docs.contains(&&"doc1".to_string()) && top_docs.contains(&&"doc5".to_string()),
                "BM25+ should rank doc1 and doc5 in the top results for 'rust programming'"
            );
        }
        
        // Test BM25F on another query
        {
            let query = "database systems";
            let scored_docs = score_documents_bm25f(query, &documents);
            let top_doc = &scored_docs[0].0;
            
            println!("BM25F top doc for 'database systems': {}", top_doc);
            
            // Check that doc8 is the top result (it's about databases)
            assert_eq!(
                top_doc, "doc8",
                "BM25F should rank doc8 as the top result for 'database systems'"
            );
        }
        
        // Test BM25+ on another query
        {
            let query = "database systems";
            let scored_docs = score_documents_bm25plus(query, &documents);
            let top_doc = &scored_docs[0].0;
            
            println!("BM25+ top doc for 'database systems': {}", top_doc);
            
            // Check that doc8 is the top result (it's about databases)
            assert_eq!(
                top_doc, "doc8",
                "BM25+ should rank doc8 as the top result for 'database systems'"
            );
        }
    }
    
    #[test]
    fn test_field_weighting_in_bm25f() {
        let base_path = env!("CARGO_MANIFEST_DIR");
        let file_path = format!("{}/../../docs/en/test_data/bm25_test_dataset/field_weighting_test.json", base_path);
        
        // Load the field weighting test dataset
        let path = Path::new(&file_path);
        let mut file = File::open(path).expect("Failed to open field weighting test file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read field weighting test file");
        
        #[derive(Debug, Deserialize)]
        struct FieldWeightingTest {
            documents: Vec<TestDocument>,
            queries: Vec<TestQuery>,
            field_weights: HashMap<String, HashMap<String, f64>>,
        }
        
        let test_data: FieldWeightingTest = serde_json::from_str(&contents).expect("Failed to parse field weighting test");
        
        // Convert test documents to terraphim documents
        let documents: Vec<Document> = test_data.documents.iter()
            .map(|doc| convert_test_document(doc))
            .collect();
        
        // Test with title priority
        if let Some(title_weights) = test_data.field_weights.get("title_priority") {
            let field_weights = FieldWeights {
                title: *title_weights.get("title").unwrap_or(&3.0),
                body: *title_weights.get("body").unwrap_or(&1.0),
                description: *title_weights.get("description").unwrap_or(&1.0),
                tags: *title_weights.get("tags").unwrap_or(&1.0),
            };
            
            let params = BM25Params::default();
            let mut scorer = BM25FScorer::with_params(params, field_weights);
            scorer.initialize(&documents);
            
            // Use the first query (fwq1)
            let query = &test_data.queries[0];
            
            let mut scored_docs: Vec<(String, f64)> = documents.iter()
                .map(|doc| {
                    let score = scorer.score(&query.query, doc);
                    (doc.id.clone(), score)
                })
                .collect();
            
            // Sort by score in descending order
            scored_docs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Extract just the document IDs for comparison
            let ranked_ids: Vec<String> = scored_docs.iter()
                .map(|(id, _)| id.clone())
                .collect();
            
            println!("Query: {} (title priority)", query.query);
            println!("Ranking with title priority: {:?}", ranked_ids);
            
            // When title field is weighted higher, fw3 should rank higher than fw2
            // despite fw2 having more occurrences in the body
            assert!(
                scored_docs.iter().position(|(id, _)| id == "fw3").unwrap() <
                scored_docs.iter().position(|(id, _)| id == "fw2").unwrap(),
                "With title priority, fw3 should rank higher than fw2"
            );
        }
    }
    
    #[test]
    fn test_rare_terms_in_bm25plus() {
        let base_path = env!("CARGO_MANIFEST_DIR");
        let file_path = format!("{}/../../docs/en/test_data/bm25_test_dataset/rare_terms_test.json", base_path);
        
        // Load the rare terms test dataset
        let path = Path::new(&file_path);
        let mut file = File::open(path).expect("Failed to open rare terms test file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read rare terms test file");
        
        #[derive(Debug, Deserialize)]
        struct RareTermsTest {
            documents: Vec<TestDocument>,
            queries: Vec<TestQuery>,
            expected_results: HashMap<String, HashMap<String, Vec<String>>>,
        }
        
        let test_data: RareTermsTest = serde_json::from_str(&contents).expect("Failed to parse rare terms test");
        
        // Convert test documents to terraphim documents
        let documents: Vec<Document> = test_data.documents.iter()
            .map(|doc| convert_test_document(doc))
            .collect();
        
        // Initialize scorers
        let mut bm25plus_scorer = BM25PlusScorer::new();
        bm25plus_scorer.initialize(&documents);
        
        // Test with the first query (rtq1)
        let query = &test_data.queries[0];
        
        // Score documents with BM25+
        let mut bm25plus_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25plus_scorer.score(&query.query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score in descending order
        bm25plus_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Extract just the document IDs for comparison
        let bm25plus_ranked_ids: Vec<String> = bm25plus_scores.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        println!("Query: {} (rare terms)", query.query);
        println!("BM25+ ranking: {:?}", bm25plus_ranked_ids);
        
        // Check if the expected document (rt3) is ranked first
        assert_eq!(
            bm25plus_ranked_ids[0], "rt3",
            "BM25+ should rank rt3 first for query '{}'", query.query
        );
        
        // Check if BM25+ assigns scores to documents that don't contain the query terms
        // This is a key feature of BM25+
        let non_matching_docs = bm25plus_scores.iter()
            .filter(|(id, score)| *id != "rt3" && *score > 0.0)
            .count();
        
        assert!(
            non_matching_docs > 0,
            "BM25+ should assign scores to documents that don't contain the query terms"
        );
    }
    
    #[test]
    fn test_document_length_normalization() {
        let base_path = env!("CARGO_MANIFEST_DIR");
        let file_path = format!("{}/../../docs/en/test_data/bm25_test_dataset/document_length_test.json", base_path);
        
        // Load the document length test dataset
        let path = Path::new(&file_path);
        let mut file = File::open(path).expect("Failed to open document length test file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read document length test file");
        
        #[derive(Debug, Deserialize)]
        struct DocumentLengthTest {
            documents: Vec<TestDocument>,
            queries: Vec<TestQuery>,
        }
        
        let test_data: DocumentLengthTest = serde_json::from_str(&contents).expect("Failed to parse document length test");
        
        // Convert test documents to terraphim documents
        let documents: Vec<Document> = test_data.documents.iter()
            .map(|doc| convert_test_document(doc))
            .collect();
        
        // Initialize scorers
        let mut bm25f_scorer = BM25FScorer::new();
        bm25f_scorer.initialize(&documents);
        
        let mut bm25plus_scorer = BM25PlusScorer::new();
        bm25plus_scorer.initialize(&documents);
        
        // Test with the first query (dlq1)
        let query = &test_data.queries[0];
        
        // Score documents with BM25F
        let mut bm25f_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25f_scorer.score(&query.query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score in descending order
        bm25f_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Extract just the document IDs for comparison
        let bm25f_ranked_ids: Vec<String> = bm25f_scores.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        println!("Query: {} (document length)", query.query);
        println!("BM25F ranking: {:?}", bm25f_ranked_ids);
        
        // Score documents with BM25+
        let mut bm25plus_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25plus_scorer.score(&query.query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score in descending order
        bm25plus_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Extract just the document IDs for comparison
        let bm25plus_ranked_ids: Vec<String> = bm25plus_scores.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        println!("BM25+ ranking: {:?}", bm25plus_ranked_ids);
        
        // Check if shorter documents are ranked higher when term frequency is similar
        assert!(
            bm25f_ranked_ids.iter().position(|id| id == "dl2").unwrap() <
            bm25f_ranked_ids.iter().position(|id| id == "dl5").unwrap(),
            "BM25F should rank shorter document dl2 higher than longer document dl5"
        );
        
        assert!(
            bm25plus_ranked_ids.iter().position(|id| id == "dl2").unwrap() <
            bm25plus_ranked_ids.iter().position(|id| id == "dl5").unwrap(),
            "BM25+ should rank shorter document dl2 higher than longer document dl5"
        );
    }
    
    #[test]
    fn test_term_frequency_saturation() {
        let base_path = env!("CARGO_MANIFEST_DIR");
        let file_path = format!("{}/../../docs/en/test_data/bm25_test_dataset/term_frequency_test.json", base_path);
        
        // Load the term frequency test dataset
        let path = Path::new(&file_path);
        let mut file = File::open(path).expect("Failed to open term frequency test file");
        let mut contents = String::new();
        file.read_to_string(&mut contents).expect("Failed to read term frequency test file");
        
        #[derive(Debug, Deserialize)]
        struct TermFrequencyTest {
            documents: Vec<TestDocument>,
            queries: Vec<TestQuery>,
        }
        
        let test_data: TermFrequencyTest = serde_json::from_str(&contents).expect("Failed to parse term frequency test");
        
        // Convert test documents to terraphim documents
        let documents: Vec<Document> = test_data.documents.iter()
            .map(|doc| convert_test_document(doc))
            .collect();
        
        // Initialize scorers
        let mut bm25f_scorer = BM25FScorer::new();
        bm25f_scorer.initialize(&documents);
        
        let mut bm25plus_scorer = BM25PlusScorer::new();
        bm25plus_scorer.initialize(&documents);
        
        // Test with the first query (tfq1)
        let query = &test_data.queries[0];
        
        // Score documents with BM25F
        let mut bm25f_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25f_scorer.score(&query.query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score in descending order
        bm25f_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Extract just the document IDs for comparison
        let bm25f_ranked_ids: Vec<String> = bm25f_scores.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        println!("Query: {} (term frequency)", query.query);
        println!("BM25F ranking: {:?}", bm25f_ranked_ids);
        
        // Score documents with BM25+
        let mut bm25plus_scores: Vec<(String, f64)> = documents.iter()
            .map(|doc| {
                let score = bm25plus_scorer.score(&query.query, doc);
                (doc.id.clone(), score)
            })
            .collect();
        
        // Sort by score in descending order
        bm25plus_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        // Extract just the document IDs for comparison
        let bm25plus_ranked_ids: Vec<String> = bm25plus_scores.iter()
            .map(|(id, _)| id.clone())
            .collect();
        
        println!("BM25+ ranking: {:?}", bm25plus_ranked_ids);
        
        // Check if documents with extreme term frequency (tf5) are not ranked significantly higher
        // than documents with high term frequency (tf3, tf4)
        let tf5_position_bm25f = bm25f_ranked_ids.iter().position(|id| id == "tf5").unwrap_or(usize::MAX);
        let tf3_position_bm25f = bm25f_ranked_ids.iter().position(|id| id == "tf3").unwrap_or(usize::MAX);
        
        let tf5_position_bm25plus = bm25plus_ranked_ids.iter().position(|id| id == "tf5").unwrap_or(usize::MAX);
        let tf3_position_bm25plus = bm25plus_ranked_ids.iter().position(|id| id == "tf3").unwrap_or(usize::MAX);
        
        // tf5 should not be ranked significantly higher than tf3 despite having many more occurrences of 'rust'
        assert!(
            tf5_position_bm25f >= tf3_position_bm25f,
            "BM25F should not rank tf5 significantly higher than tf3 despite having many more occurrences of 'rust'"
        );
        
        assert!(
            tf5_position_bm25plus >= tf3_position_bm25plus,
            "BM25+ should not rank tf5 significantly higher than tf3 despite having many more occurrences of 'rust'"
        );
    }
} 