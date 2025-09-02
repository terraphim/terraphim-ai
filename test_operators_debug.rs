use terraphim_types::{NormalizedTermValue, SearchQuery, LogicalOperator};

fn main() {
    // Test case 1: Basic AND query with 2 terms
    let query1 = SearchQuery {
        search_term: NormalizedTermValue::from("rust"),
        search_terms: Some(vec![
            NormalizedTermValue::from("rust"),
            NormalizedTermValue::from("async"),
        ]),
        operator: Some(LogicalOperator::And),
        skip: None,
        limit: None,
        role: None,
    };
    
    let terms1 = query1.get_all_terms();
    println!("Test 1 - Expected: [rust, async], Got: {:?}", terms1);
    println!("  Terms count: {}", terms1.len());
    
    // Test case 2: AND query with 3 terms
    let query2 = SearchQuery {
        search_term: NormalizedTermValue::from("system"),
        search_terms: Some(vec![
            NormalizedTermValue::from("system"),
            NormalizedTermValue::from("operation"),
            NormalizedTermValue::from("management"),
        ]),
        operator: Some(LogicalOperator::And),
        skip: None,
        limit: None,
        role: None,
    };
    
    let terms2 = query2.get_all_terms();
    println!("\nTest 2 - Expected: [system, operation, management], Got: {:?}", terms2);
    println!("  Terms count: {}", terms2.len());
    
    // Test case 3: OR query with 4 terms
    let query3 = SearchQuery {
        search_term: NormalizedTermValue::from("api"),
        search_terms: Some(vec![
            NormalizedTermValue::from("api"),
            NormalizedTermValue::from("sdk"),
            NormalizedTermValue::from("library"),
            NormalizedTermValue::from("framework"),
        ]),
        operator: Some(LogicalOperator::Or),
        skip: None,
        limit: None,
        role: None,
    };
    
    let terms3 = query3.get_all_terms();
    println!("\nTest 3 - Expected: [api, sdk, library, framework], Got: {:?}", terms3);
    println!("  Terms count: {}", terms3.len());
    
    // Document test for AND operator
    let test_doc_all_terms = "This document contains rust async programming concepts";
    let test_doc_partial = "This document only contains rust concepts";
    
    println!("\n--- AND Operator Test ---");
    println!("Document 1: '{}'", test_doc_all_terms);
    println!("Document 2: '{}'", test_doc_partial);
    
    let searchable1 = test_doc_all_terms.to_lowercase();
    let searchable2 = test_doc_partial.to_lowercase();
    
    let and_terms = vec!["rust", "async"];
    let doc1_and_match = and_terms.iter().all(|term| searchable1.contains(term));
    let doc2_and_match = and_terms.iter().all(|term| searchable2.contains(term));
    
    println!("  AND query [rust, async]:");
    println!("    Doc1 matches (should be true): {}", doc1_and_match);
    println!("    Doc2 matches (should be false): {}", doc2_and_match);
    
    // Document test for OR operator
    println!("\n--- OR Operator Test ---");
    let or_terms = vec!["rust", "python"];
    let doc1_or_match = or_terms.iter().any(|term| searchable1.contains(term));
    let doc2_or_match = or_terms.iter().any(|term| searchable2.contains(term));
    
    println!("  OR query [rust, python]:");
    println!("    Doc1 matches (should be true): {}", doc1_or_match);
    println!("    Doc2 matches (should be true): {}", doc2_or_match);
    
    let doc3 = "This document is about javascript only".to_lowercase();
    let doc3_or_match = or_terms.iter().any(|term| doc3.contains(term));
    println!("    Doc3 ('javascript only') matches (should be false): {}", doc3_or_match);
}