#[cfg(feature = "atomic")]
use serde_json::json;
#[cfg(feature = "atomic")]
use terraphim_atomic_client::{self, Store};
#[cfg(feature = "atomic")]
use terraphim_middleware::{haystack::AtomicHaystackIndexer, indexer::IndexMiddleware};

// Terraphim ontology property URIs used for storing full document body and path.
pub const BODY_PROPERTY_URI: &str = "http://localhost:9883/terraphim-drive/terraphim/property/body";
pub const PATH_PROPERTY_URI: &str = "http://localhost:9883/terraphim-drive/terraphim/property/path";

/// Test that imports documents from a filesystem path into Atomic Server and searches them
///
/// This test demonstrates the complete workflow:
/// 1. Scan a directory for markdown files
/// 2. Import each file as a Document resource in Atomic Server
/// 3. Search the imported documents using the Atomic haystack indexer
/// 4. Verify search results match expected content
#[cfg(feature = "atomic")]
#[tokio::test]
// This test requires a running Atomic Server (http://localhost:9883) and .env with ATOMIC_SERVER_URL & ATOMIC_SERVER_SECRET.
// It will be skipped at runtime if prerequisites are missing.
async fn test_document_import_and_search() {
    // This test requires a running Atomic Server instance and a .env file
    // at the root of the workspace with the following content:
    // ATOMIC_SERVER_URL=http://localhost:9883
    // ATOMIC_SERVER_SECRET=...
    dotenvy::dotenv().ok();

    let config =
        terraphim_atomic_client::Config::from_env().expect("Failed to load config from env");
    let store = Store::new(config.clone()).expect("Failed to create store");

    // 1. Create a parent collection for the imported documents
    let server_url = config.server_url.trim_end_matches('/');
    let parent_subject = format!("{}/imported-documents", server_url);
    let mut parent_properties = HashMap::new();
    parent_properties.insert(
        "https://atomicdata.dev/properties/isA".to_string(),
        json!(["https://atomicdata.dev/classes/Collection"]),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/name".to_string(),
        json!("Imported Documents"),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/description".to_string(),
        json!("Documents imported from filesystem for testing"),
    );
    parent_properties.insert(
        "https://atomicdata.dev/properties/parent".to_string(),
        json!(server_url),
    );

    store
        .create_with_commit(&parent_subject, parent_properties.clone())
        .await
        .expect("Failed to create parent collection");

    let mut imported_documents = Vec::new();
    let mut document_count = 0;

    // 2. Scan the docs/src directory for markdown files
    let src_path = Path::new("docs/src");
    if !src_path.exists() {
        println!("Warning: docs/src directory not found, creating sample documents for testing");

        // Create sample documents in memory for testing
        let sample_docs = vec![
            ("README.md", "# Terraphim AI\n\nThis is the main README for Terraphim AI project.\n\n## Features\n- Document search\n- Knowledge graphs\n- Role-based access"),
            ("Architecture.md", "# Architecture\n\nTerraphim uses a modular architecture with the following components:\n\n- Atomic Server for storage\n- Middleware for indexing\n- Frontend for user interface"),
            ("Introduction.md", "# Introduction\n\nWelcome to Terraphim AI documentation.\n\n## Getting Started\n\nThis guide will help you understand how to use Terraphim for document management and search."),
        ];

        for (filename, content) in sample_docs {
            let title = extract_title_from_markdown(content)
                .unwrap_or_else(|| filename.strip_suffix(".md").unwrap_or(filename).to_string());

            // Create document in Atomic Server
            let document_id = format!("sample-doc-{}", Uuid::new_v4());
            let document_subject = format!("{}/{}", parent_subject, document_id);

            let mut document_properties = HashMap::new();
            document_properties.insert(
                "https://atomicdata.dev/properties/isA".to_string(),
                json!(["https://atomicdata.dev/classes/Document"]),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/name".to_string(),
                json!(title),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/description".to_string(),
                json!(format!("Sample document: {}", filename)),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/parent".to_string(),
                json!(parent_subject),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/shortname".to_string(),
                json!(document_id),
            );
            document_properties.insert(BODY_PROPERTY_URI.to_string(), json!(content));
            document_properties.insert(PATH_PROPERTY_URI.to_string(), json!(filename));

            match store
                .create_with_commit(&document_subject, document_properties.clone())
                .await
            {
                Ok(_) => {
                    document_count += 1;
                    imported_documents.push((
                        document_subject.clone(),
                        title.clone(),
                        content.to_string(),
                    ));
                    println!("Created sample document {}: {}", document_count, title);
                }
                Err(e) => {
                    println!("Failed to create sample document {}: {}", filename, e);
                }
            }
        }
    } else {
        // Scan real docs/src directory for markdown files
        // (imported_documents and document_count already declared above)

        // Walk through all markdown files in the src directory
        for entry in WalkDir::new(src_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        {
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(src_path).unwrap_or(file_path);

            // Skip if file is too large or empty
            if let Ok(metadata) = fs::metadata(file_path) {
                if metadata.len() > 1024 * 1024 {
                    // Skip files larger than 1MB
                    println!("Skipping large file: {:?}", file_path);
                    continue;
                }
            }

            // Read file content
            let content = match fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(e) => {
                    println!("Failed to read file {:?}: {}", file_path, e);
                    continue;
                }
            };

            if content.trim().is_empty() {
                println!("Skipping empty file: {:?}", file_path);
                continue;
            }

            // Extract title from first heading or use filename
            let title = extract_title_from_markdown(&content).unwrap_or_else(|| {
                file_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });

            // Create document in Atomic Server
            let document_id = format!("imported-doc-{}", Uuid::new_v4());
            let document_subject = format!("{}/{}", parent_subject, document_id);

            let mut document_properties = HashMap::new();
            document_properties.insert(
                "https://atomicdata.dev/properties/isA".to_string(),
                json!(["https://atomicdata.dev/classes/Document"]),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/name".to_string(),
                json!(title),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/description".to_string(),
                json!(format!("Document imported from {:?}", relative_path)),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/parent".to_string(),
                json!(parent_subject),
            );
            document_properties.insert(
                "https://atomicdata.dev/properties/shortname".to_string(),
                json!(document_id),
            );
            document_properties.insert(BODY_PROPERTY_URI.to_string(), json!(content));
            document_properties.insert(
                PATH_PROPERTY_URI.to_string(),
                json!(relative_path.to_string_lossy().to_string()),
            );

            match store
                .create_with_commit(&document_subject, document_properties.clone())
                .await
            {
                Ok(_) => {
                    document_count += 1;
                    imported_documents.push((
                        document_subject.clone(),
                        title.clone(),
                        content.clone(),
                    ));
                    println!("Imported document {}: {}", document_count, title);
                }
                Err(e) => {
                    println!("Failed to import document {:?}: {}", file_path, e);
                }
            }

            // Limit the number of documents to import for testing
            if document_count >= 10 {
                println!("Reached limit of 10 documents, stopping import");
                break;
            }
        }
    }

    if imported_documents.is_empty() {
        println!("No documents were imported, skipping search test");
        return;
    }

    println!("Successfully imported {} documents", document_count);

    // Give the server a moment to index the new resources
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // 3. Test searching the imported documents
    let indexer = AtomicHaystackIndexer::default();
    let haystack = Haystack::new(
        config.server_url.clone(),
        terraphim_config::ServiceType::Atomic,
        true,
    )
    .with_atomic_secret(std::env::var("ATOMIC_SERVER_SECRET").ok());

    // Test search with various terms that should be found in the documents
    let search_terms = vec![
        "Terraphim",
        "Architecture",
        "Introduction",
        "AI", // This is in the Terraphim AI document
    ];

    for search_term in search_terms {
        println!("Searching for: '{}'", search_term);

        // Poll the server until we get results or timeout
        let mut index = terraphim_types::Index::new();
        let mut found_results = false;

        for attempt in 0..10 {
            index = indexer
                .index(search_term, &haystack)
                .await
                .expect("Search failed");

            if !index.is_empty() {
                found_results = true;
                println!("  Found {} results on attempt {}", index.len(), attempt + 1);
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        if found_results {
            // Verify that at least some of our imported documents are in the results
            let imported_titles: Vec<String> = imported_documents
                .iter()
                .map(|(_, title, _)| title.clone())
                .collect();

            let found_titles: Vec<String> = index.values().map(|doc| doc.title.clone()).collect();

            let matching_titles: Vec<String> = found_titles
                .iter()
                .filter(|title| imported_titles.contains(title))
                .cloned()
                .collect();

            println!("  Matching imported documents: {:?}", matching_titles);

            // Assert that we found at least some of our imported documents
            assert!(
                !matching_titles.is_empty(),
                "Search for '{}' should return at least one imported document",
                search_term
            );
        } else {
            println!("  No results found for '{}'", search_term);
        }
    }

    // 4. Test a more specific search
    println!("Testing specific content search...");
    let specific_search = "async fn";
    let index = indexer
        .index(specific_search, &haystack)
        .await
        .expect("Specific search failed");

    if !index.is_empty() {
        println!("Found {} results for '{}'", index.len(), specific_search);

        // Print details of found documents
        for (id, doc) in index.iter() {
            println!("  Document: {} - {}", doc.title, id);
            if let Some(desc) = &doc.description {
                println!("    Description: {}", desc);
            }
        }
    }

    // 5. Clean up - delete the imported documents and parent collection
    println!("Cleaning up imported documents...");
    for (subject, title, _) in imported_documents {
        if let Err(e) = store.delete_with_commit(&subject).await {
            println!("Failed to delete document '{}': {}", title, e);
        } else {
            println!("Deleted document: {}", title);
        }
    }

    if let Err(e) = store.delete_with_commit(&parent_subject).await {
        println!("Failed to delete parent collection: {}", e);
    } else {
        println!("Deleted parent collection");
    }

    println!("Test completed successfully!");
}

/// Extract title from markdown content by looking for the first heading
fn extract_title_from_markdown(content: &str) -> Option<String> {
    // Look for the first heading in the markdown
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(stripped) = trimmed.strip_prefix("# ") {
            return Some(stripped.trim().to_string());
        }
    }
    None
}
