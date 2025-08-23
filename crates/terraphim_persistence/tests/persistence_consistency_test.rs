//! Comprehensive tests for persistence key generation consistency
//!
//! This test suite validates that:
//! 1. Key normalization works consistently for both Thesaurus and Document
//! 2. Save/load operations work correctly across different backends  
//! 3. Edge cases with special characters, spaces, and unicode are handled
//! 4. Cross-backend consistency is maintained

use serial_test::serial;
use terraphim_persistence::{DeviceStorage, Persistable, Result};
use terraphim_types::{Document, NormalizedTerm, NormalizedTermValue, Thesaurus};

/// Initialize memory-only persistence for testing
async fn init_test_persistence() -> Result<()> {
    DeviceStorage::init_memory_only().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_key_normalization_consistency() -> Result<()> {
    init_test_persistence().await?;

    // Test data with various challenging characters
    let test_cases = vec![
        ("Simple", "simple.json"),
        ("Test Name", "testname.json"),
        ("Test-Name_123", "testname123.json"), 
        ("TeSt NaMe", "testname.json"),
        ("AI/ML Engineer", "aimlengineer.json"),  // Fixed: A-I-M-L not A-M-L-L
        ("Data & Analytics", "dataanalytics.json"),
        ("Role (v2.0)", "rolev20.json"),
        ("Engineer@Company", "engineercompany.json"),
        ("Terraphim Engineer", "terraphimengineer.json"),
    ];

    for (input, expected_suffix) in test_cases {
        // Test Thesaurus key normalization
        let thesaurus = Thesaurus::new(input.to_string());
        let thesaurus_key = thesaurus.get_key();
        let expected_thesaurus_key = format!("thesaurus_{}", expected_suffix);
        assert_eq!(thesaurus_key, expected_thesaurus_key, 
                   "Thesaurus key mismatch for input '{}': got '{}', expected '{}'", 
                   input, thesaurus_key, expected_thesaurus_key);

        // Test Document key normalization (using input as document ID)
        let mut document = Document::default();
        document.id = input.to_string();
        let document_key = document.get_key();
        let expected_document_key = format!("document_{}", expected_suffix);
        assert_eq!(document_key, expected_document_key,
                   "Document key mismatch for input '{}': got '{}', expected '{}'",
                   input, document_key, expected_document_key);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_thesaurus_persistence_with_complex_names() -> Result<()> {
    init_test_persistence().await?;

    let test_names = vec![
        "Terraphim Engineer",
        "AI/ML Specialist", 
        "Data & Analytics Expert",
        "Software Engineer (Senior)",
        "Full-Stack Developer",
        "Product Manager - Technical"
    ];

    for name in test_names {
        println!("Testing thesaurus persistence for name: '{}'", name);
        
        // Create and populate thesaurus
        let mut thesaurus = Thesaurus::new(name.to_string());
        let test_term = NormalizedTerm::new(
            1,
            NormalizedTermValue::from("test-concept".to_string())
        );
        thesaurus.insert(NormalizedTermValue::from("test".to_string()), test_term.clone());
        
        assert_eq!(thesaurus.len(), 1, "Thesaurus should have 1 entry");
        assert_eq!(thesaurus.name(), name, "Thesaurus name should match");

        // Save thesaurus
        println!("  Saving thesaurus with key: '{}'", thesaurus.get_key());
        thesaurus.save().await?;
        
        // Load with same name
        let mut loaded_thesaurus = Thesaurus::new(name.to_string());
        loaded_thesaurus = loaded_thesaurus.load().await?;
        
        // Verify loaded data
        assert_eq!(loaded_thesaurus.len(), 1, "Loaded thesaurus should have 1 entry for name '{}'", name);
        assert_eq!(loaded_thesaurus.name(), name, "Loaded thesaurus name should match for '{}'", name);
        
        let loaded_term = loaded_thesaurus.get(&NormalizedTermValue::from("test".to_string()));
        assert!(loaded_term.is_some(), "Loaded thesaurus should contain test term for name '{}'", name);
        assert_eq!(loaded_term.unwrap(), &test_term, "Loaded term should match original for name '{}'", name);
        
        println!("  ✅ Successfully persisted and loaded thesaurus for name: '{}'", name);
    }

    Ok(())
}

#[tokio::test] 
#[serial]
async fn test_document_persistence_with_various_ids() -> Result<()> {
    init_test_persistence().await?;

    let test_ids = vec![
        "simple-id",
        "a33bd45bece9c7cb", // Hex hash format
        "http://example.com/document/123",
        "file:///path/to/document.txt", 
        "document with spaces",
        "document-with-special@chars#123",
        "UPPERCASE_DOCUMENT_ID",
        "mixed_Case_Document_123"
    ];

    for id in test_ids {
        println!("Testing document persistence for ID: '{}'", id);
        
        // Create document
        let mut document = Document::default();
        document.id = id.to_string();
        document.title = format!("Test Document for {}", id);
        document.body = format!("This is the body content for document {}", id);
        document.url = format!("https://example.com/{}", id);
        document.description = Some(format!("Description for document {}", id));

        // Save document
        println!("  Saving document with key: '{}'", document.get_key());
        document.save().await?;
        
        // Load with same ID
        let mut loaded_document = Document::default();
        loaded_document.id = id.to_string(); 
        loaded_document = loaded_document.load().await?;
        
        // Verify loaded data
        assert_eq!(loaded_document.id, id, "Loaded document ID should match for '{}'", id);
        assert_eq!(loaded_document.title, format!("Test Document for {}", id), "Loaded document title should match for '{}'", id);
        assert_eq!(loaded_document.body, format!("This is the body content for document {}", id), "Loaded document body should match for '{}'", id);
        assert_eq!(loaded_document.url, format!("https://example.com/{}", id), "Loaded document URL should match for '{}'", id);
        assert_eq!(loaded_document.description, Some(format!("Description for document {}", id)), "Loaded document description should match for '{}'", id);
        
        println!("  ✅ Successfully persisted and loaded document for ID: '{}'", id);
    }

    Ok(())
}

#[tokio::test]
#[serial] 
async fn test_cross_backend_consistency() -> Result<()> {
    init_test_persistence().await?;

    // Test thesaurus consistency across save/load cycles
    let thesaurus_name = "Cross Backend Test";
    let mut thesaurus = Thesaurus::new(thesaurus_name.to_string());
    
    // Add test data
    let term1 = NormalizedTerm::new(1, NormalizedTermValue::from("concept1".to_string()));
    let term2 = NormalizedTerm::new(2, NormalizedTermValue::from("concept2".to_string()));
    thesaurus.insert(NormalizedTermValue::from("term1".to_string()), term1.clone());
    thesaurus.insert(NormalizedTermValue::from("term2".to_string()), term2.clone());
    
    // Save to memory backend
    thesaurus.save_to_one("memory").await?;
    
    // Load from memory backend
    let mut loaded_thesaurus = Thesaurus::new(thesaurus_name.to_string());
    loaded_thesaurus = loaded_thesaurus.load().await?;
    
    assert_eq!(loaded_thesaurus.len(), 2, "Thesaurus should have 2 entries after memory round-trip");
    assert!(loaded_thesaurus.get(&NormalizedTermValue::from("term1".to_string())).is_some(), "Term1 should exist after memory round-trip");
    assert!(loaded_thesaurus.get(&NormalizedTermValue::from("term2".to_string())).is_some(), "Term2 should exist after memory round-trip");

    // Test document consistency
    let document_id = "cross-backend-test-doc";
    let mut document = Document::default();
    document.id = document_id.to_string();
    document.title = "Cross Backend Test Document".to_string();
    document.body = "This is a test document for cross-backend consistency.".to_string();
    
    // Save to memory backend
    document.save_to_one("memory").await?;
    
    // Load from memory backend 
    let mut loaded_document = Document::default();
    loaded_document.id = document_id.to_string();
    loaded_document = loaded_document.load().await?;
    
    assert_eq!(loaded_document.id, document_id, "Document ID should match after memory round-trip");
    assert_eq!(loaded_document.title, "Cross Backend Test Document", "Document title should match after memory round-trip");
    assert_eq!(loaded_document.body, "This is a test document for cross-backend consistency.", "Document body should match after memory round-trip");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_unicode_and_emoji_handling() -> Result<()> {
    init_test_persistence().await?;

    let unicode_test_cases = vec![
        ("Simple ASCII", "simpleascii"),
        ("Unicode: café", "unicodecaf"),
        ("Emoji: 🚀 Engineer", "emojiengineer"),
        ("Mixed: AI/ML 🤖 Expert", "aimlexpert"),
        ("Chinese: 人工智能", ""),  // All non-ASCII chars should be removed
        ("Arabic: مهندس البرمجيات", ""), // All non-ASCII chars should be removed
    ];

    for (input, expected_normalized) in unicode_test_cases {
        println!("Testing unicode handling for: '{}'", input);
        
        // Test thesaurus with unicode name
        let thesaurus = Thesaurus::new(input.to_string());
        let key = thesaurus.get_key();
        let expected_key = if expected_normalized.is_empty() {
            "thesaurus_.json".to_string()
        } else {
            format!("thesaurus_{}.json", expected_normalized)
        };
        
        assert_eq!(key, expected_key, "Unicode normalization failed for '{}': got '{}', expected '{}'", 
                   input, key, expected_key);
        
        // Only test save/load for cases that result in valid keys
        if !expected_normalized.is_empty() {
            let mut test_thesaurus = Thesaurus::new(input.to_string());
            let term = NormalizedTerm::new(1, NormalizedTermValue::from("test".to_string()));
            test_thesaurus.insert(NormalizedTermValue::from("test".to_string()), term);
            
            test_thesaurus.save().await?;
            
            let mut loaded = Thesaurus::new(input.to_string());
            loaded = loaded.load().await?;
            
            assert_eq!(loaded.len(), 1, "Unicode thesaurus should persist correctly for '{}'", input);
        }
        
        println!("  ✅ Unicode handling validated for: '{}'", input);
    }

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_empty_and_edge_case_keys() -> Result<()> {
    init_test_persistence().await?;

    // Test edge cases
    let edge_cases = vec![
        ("", ""), // Empty name
        ("   ", ""), // Only spaces
        ("!!!", ""), // Only special chars
        ("123", "123"), // Only numbers
        ("a", "a"), // Single character
    ];

    for (input, expected_normalized) in edge_cases {
        println!("Testing edge case: '{}'", input);
        
        let thesaurus = Thesaurus::new(input.to_string());
        let key = thesaurus.get_key();
        let expected_key = format!("thesaurus_{}.json", expected_normalized);
        
        assert_eq!(key, expected_key, "Edge case normalization failed for '{}': got '{}', expected '{}'", 
                   input, key, expected_key);
        
        println!("  ✅ Edge case handled: '{}' → '{}'", input, key);
    }

    Ok(())
}

#[tokio::test]
#[serial] 
async fn test_key_generation_performance() -> Result<()> {
    init_test_persistence().await?;

    let start = std::time::Instant::now();
    
    // Generate many keys to test performance
    for i in 0..1000 {
        let name = format!("Performance Test Role {}", i);
        let thesaurus = Thesaurus::new(name);
        let _key = thesaurus.get_key();
        
        let id = format!("performance-test-doc-{}", i);
        let mut document = Document::default();
        document.id = id;
        let _doc_key = document.get_key();
    }
    
    let duration = start.elapsed();
    println!("Generated 2000 keys in {:?}", duration);
    
    // Performance should be reasonable (less than 100ms for 2000 keys)
    assert!(duration.as_millis() < 100, "Key generation should be fast, took {:?}", duration);

    Ok(())
}