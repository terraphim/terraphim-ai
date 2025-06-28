use std::path::Path;
use terraphim_types::{Document, Index};
use terraphim_atomic_client::{
    store::Store,
    types::Config,
    Agent,
};

use crate::{Result, indexer::IndexMiddleware};
use terraphim_config::Haystack;

/// Middleware that uses an Atomic Server as a haystack.
#[derive(Default, Clone)]
pub struct AtomicHaystackIndexer {
    // We can add configuration here, like the server URL
}

impl IndexMiddleware for AtomicHaystackIndexer {
    /// Index the haystack using an Atomic Server and return an index of documents
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let haystack_url = haystack.path.to_str().unwrap_or_default();
        log::debug!("AtomicHaystackIndexer::index called with needle: '{}' haystack: {:?}", needle, haystack_url);
        println!("🔍 AtomicHaystackIndexer searching for: '{}'", needle);

        if haystack_url.is_empty() {
            log::warn!("Haystack path is empty");
            return Ok(Index::default());
        }

        let agent = if let Some(secret) = &haystack.atomic_server_secret {
            Some(Agent::from_base64(secret)
                .map_err(|e| crate::Error::Indexation(e.to_string()))?)
        } else {
            None
        };

        // Initialize the atomic store
        let config = Config {
            server_url: haystack_url.to_string(),
            agent,
        };
        let store = Store::new(config).map_err(|e| crate::Error::Indexation(e.to_string()))?;

        // Perform a search
        let search_result = store.search(needle).await.map_err(|e| crate::Error::Indexation(e.to_string()))?;
        println!("📊 Search result structure: {}", serde_json::to_string_pretty(&search_result).unwrap_or_else(|_| format!("{:?}", search_result)));

        // Convert search results to documents
        let mut index = Index::new();
        
        // Handle Atomic Server search response format
        // The response is an object with "https://atomicdata.dev/properties/endpoint/results" array
        if let Some(obj) = search_result.as_object() {
            println!("📋 Search result is object format");
            
            // Check for the endpoint/results property (standard Atomic Server search response)
            if let Some(results) = obj.get("https://atomicdata.dev/properties/endpoint/results").and_then(|v| v.as_array()) {
                println!("📋 Found {} results in endpoint/results format", results.len());
                let server_prefix = store.config.server_url.trim_end_matches('/');
                for result_val in results {
                    if let Some(subject) = result_val.as_str() {
                        // Skip external URLs that don't belong to our server
                        if !subject.starts_with(server_prefix) {
                            println!("  ⏭️ Skipping external URL: {}", subject);
                            continue;
                        }
                        println!("  📄 Processing result: {}", subject);
                        let resource = store.get_resource(subject).await
                            .map_err(|e| crate::Error::Indexation(format!("Failed to get resource: {}", e)))?;

                        let document = Document {
                            id: resource.subject.clone(),
                            url: resource.subject.clone(),
                            title: resource.properties.get("https://atomicdata.dev/properties/name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&resource.subject)
                                .to_string(),
                            description: resource.properties.get("https://atomicdata.dev/properties/description")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            body: serde_json::to_string(&resource.properties).unwrap_or_default(),
                            ..Default::default()
                        };
                        
                        println!("  ✅ Created document: {} ({})", document.title, document.id);
                        index.insert(document.id.clone(), document);
                    } else {
                        println!("  ❌ Result is not a string: {:?}", result_val);
                    }
                }
            } else {
                println!("❌ No 'endpoint/results' array found in object response");
                // Fallback: check for simple array format or subjects array
                if let Some(subjects) = obj.get("subjects").and_then(|v| v.as_array()) {
                    println!("📋 Found {} subjects in fallback format", subjects.len());
                    for subject_val in subjects {
                        if let Some(subject) = subject_val.as_str() {
                            println!("  📄 Processing subject: {}", subject);
                            let resource = store.get_resource(subject).await
                                .map_err(|e| crate::Error::Indexation(format!("Failed to get resource: {}", e)))?;

                            let document = Document {
                                id: resource.subject.clone(),
                                url: resource.subject.clone(),
                                title: resource.properties.get("https://atomicdata.dev/properties/name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&resource.subject)
                                    .to_string(),
                                description: resource.properties.get("https://atomicdata.dev/properties/description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                body: serde_json::to_string(&resource.properties).unwrap_or_default(),
                                ..Default::default()
                            };
                            
                            println!("  ✅ Created document: {} ({})", document.title, document.id);
                            index.insert(document.id.clone(), document);
                        }
                    }
                } else {
                    println!("❌ No recognized result format found in response");
                }
            }
        } else if let Some(results) = search_result.as_array() {
            // Direct array format (legacy support)
            println!("📋 Found {} results in direct array format", results.len());
            for result in results {
                if let Some(subject) = result.as_str() {
                    println!("  📄 Processing result: {}", subject);
                    let resource = store.get_resource(subject).await
                        .map_err(|e| crate::Error::Indexation(format!("Failed to get resource: {}", e)))?;

                    let document = Document {
                        id: resource.subject.clone(),
                        url: resource.subject.clone(),
                        title: resource.properties.get("https://atomicdata.dev/properties/name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&resource.subject)
                            .to_string(),
                        description: resource.properties.get("https://atomicdata.dev/properties/description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        body: serde_json::to_string(&resource.properties).unwrap_or_default(),
                        ..Default::default()
                    };
                    
                    println!("  ✅ Created document: {} ({})", document.title, document.id);
                    index.insert(document.id.clone(), document);
                } else {
                    println!("  ❌ Result is not a string: {:?}", result);
                }
            }
        } else {
            println!("❌ Search result is neither array nor object: {:?}", search_result);
        }
        
        println!("🎯 Final index contains {} documents", index.len());
        Ok(index)
    }
} 