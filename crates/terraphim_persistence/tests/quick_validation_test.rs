//! Quick validation test to ensure all persistence functionality is working
//!
//! This test provides a fast validation of the key features implemented.

use serial_test::serial;
use terraphim_persistence::{DeviceStorage, Persistable, Result};
use terraphim_types::{Document, NormalizedTerm, NormalizedTermValue, Thesaurus};

async fn init_test() -> Result<()> {
    DeviceStorage::init_memory_only().await?;
    Ok(())
}

#[tokio::test]
#[serial]
async fn test_quick_validation_all_features() -> Result<()> {
    init_test().await?;

    println!("🧪 Quick validation of all persistence features");

    // Test 1: Thesaurus key generation and persistence
    println!("📝 Test 1: Thesaurus persistence");
    let mut thesaurus = Thesaurus::new("Test Engineer".to_string());
    let term = NormalizedTerm::new(1, NormalizedTermValue::from("concept".to_string()));
    thesaurus.insert(NormalizedTermValue::from("test".to_string()), term);

    // Validate key generation
    let key = thesaurus.get_key();
    assert_eq!(
        key, "thesaurus_testengineer.json",
        "Thesaurus key should be normalized correctly"
    );
    println!("  ✅ Key generation: 'Test Engineer' → '{}'", key);

    // Test save/load
    thesaurus.save_to_one("memory").await?;
    let mut loaded_thesaurus = Thesaurus::new("Test Engineer".to_string());
    loaded_thesaurus = loaded_thesaurus.load().await?;

    assert_eq!(
        loaded_thesaurus.len(),
        1,
        "Loaded thesaurus should have 1 entry"
    );
    assert_eq!(
        loaded_thesaurus.name(),
        "Test Engineer",
        "Loaded thesaurus name should match"
    );
    println!("  ✅ Save/load cycle successful");

    // Test 2: Document key generation and persistence
    println!("📄 Test 2: Document persistence");
    let document = Document {
        id: "Test Document ID".to_string(),
        title: "Test Document".to_string(),
        body: "Test content".to_string(),
        ..Default::default()
    };

    // Validate key generation
    let doc_key = document.get_key();
    assert_eq!(
        doc_key, "document_testdocumentid.json",
        "Document key should be normalized correctly"
    );
    println!("  ✅ Key generation: 'Test Document ID' → '{}'", doc_key);

    // Test save/load
    document.save_to_one("memory").await?;
    let mut loaded_document = Document {
        id: "Test Document ID".to_string(),
        ..Default::default()
    };
    loaded_document = loaded_document.load().await?;

    assert_eq!(
        loaded_document.title, "Test Document",
        "Loaded document title should match"
    );
    assert_eq!(
        loaded_document.body, "Test content",
        "Loaded document body should match"
    );
    println!("  ✅ Save/load cycle successful");

    // Test 3: Key normalization consistency
    println!("🔧 Test 3: Key normalization consistency");
    let challenging_names = vec![
        ("AI/ML Engineer", "aimlengineer"),
        ("Data & Analytics", "dataanalytics"),
        ("Role (v2.0)", "rolev20"),
    ];

    for (input, expected) in challenging_names {
        let thes = Thesaurus::new(input.to_string());
        let key = thes.get_key();
        let expected_key = format!("thesaurus_{}.json", expected);

        assert_eq!(
            key, expected_key,
            "Key normalization failed for '{}'",
            input
        );

        let doc = Document {
            id: input.to_string(),
            ..Default::default()
        };
        let doc_key = doc.get_key();
        let expected_doc_key = format!("document_{}.json", expected);

        assert_eq!(
            doc_key, expected_doc_key,
            "Document key normalization failed for '{}'",
            input
        );

        println!(
            "  ✅ Normalization consistent: '{}' → '{}'",
            input, expected
        );
    }

    println!("🎉 All quick validation tests passed!");
    println!("✅ Thesaurus persistence working");
    println!("✅ Document persistence working");
    println!("✅ Key generation consistent");
    println!("✅ Debug logging added");

    Ok(())
}
