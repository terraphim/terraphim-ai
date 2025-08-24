use terraphim_atomic_client::{store::Store, types::Config, Agent};
use terraphim_types::{Document, Index};
use terraphim_persistence::Persistable;

use crate::{indexer::IndexMiddleware, Result};
use terraphim_config::Haystack;

/// Middleware that uses an Atomic Server as a haystack.
#[derive(Default, Clone)]
pub struct AtomicHaystackIndexer {
    // We can add configuration here, like the server URL
}

impl AtomicHaystackIndexer {
    /// Normalize document ID to match persistence layer expectations
    fn normalize_document_id(&self, original_id: &str) -> String {
        // Create a dummy document to access the normalize_key method
        let dummy_doc = Document {
            id: "dummy".to_string(),
            title: "dummy".to_string(),
            body: "dummy".to_string(),
            url: "dummy".to_string(),
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
        };
        dummy_doc.normalize_key(original_id)
    }
}

impl IndexMiddleware for AtomicHaystackIndexer {
    /// Index the haystack using an Atomic Server and return an index of documents
    ///
    /// # Errors
    ///
    /// Returns an error if the middleware fails to index the haystack
    async fn index(&self, needle: &str, haystack: &Haystack) -> Result<Index> {
        let haystack_url = &haystack.location;
        log::debug!(
            "AtomicHaystackIndexer::index called with needle: '{}' haystack: {:?}",
            needle,
            haystack_url
        );
        log::info!("🔍 AtomicHaystackIndexer searching for: '{}'", needle);

        if haystack_url.is_empty() {
            log::warn!("Haystack location is empty");
            return Ok(Index::default());
        }

        // Validate URL format before proceeding
        if !is_valid_url(haystack_url) {
            log::warn!(
                "Invalid URL format: '{}', returning empty index",
                haystack_url
            );
            return Ok(Index::default());
        }

        // Create agent from secret if provided
        let agent = if let Some(secret) = &haystack.atomic_server_secret {
            match Agent::from_base64(secret) {
                Ok(agent) => {
                    log::debug!("Successfully created agent from secret");
                    Some(agent)
                }
                Err(e) => {
                    log::error!("Failed to create agent from secret: {}", e);
                    return Err(crate::Error::Indexation(format!(
                        "Invalid atomic server secret: {}",
                        e
                    )));
                }
            }
        } else {
            log::debug!("No atomic server secret provided, using anonymous access");
            None
        };

        // Initialize the atomic store
        let config = Config {
            server_url: haystack_url.to_string(),
            agent,
        };
        let store = match Store::new(config) {
            Ok(store) => store,
            Err(e) => {
                log::warn!(
                    "Failed to create atomic store for URL '{}': {}, returning empty index",
                    haystack_url,
                    e
                );
                return Ok(Index::default());
            }
        };

        // Perform a search
        log::debug!("Performing search for: '{}'", needle);
        let search_result = match store.search(needle).await {
            Ok(result) => result,
            Err(e) => {
                log::warn!(
                    "Search failed for URL '{}': {}, returning empty index",
                    haystack_url,
                    e
                );
                return Ok(Index::default());
            }
        };

        log::debug!(
            "📊 Search result structure: {}",
            serde_json::to_string_pretty(&search_result)
                .unwrap_or_else(|_| format!("{:?}", search_result))
        );

        // Convert search results to documents
        let mut index = Index::new();

        // Handle Atomic Server search response format
        // The response is an object with "https://atomicdata.dev/properties/endpoint/results" array
        if let Some(obj) = search_result.as_object() {
            log::debug!("📋 Search result is object format");

            // Check for the endpoint/results property (standard Atomic Server search response)
            if let Some(results) = obj
                .get("https://atomicdata.dev/properties/endpoint/results")
                .and_then(|v| v.as_array())
            {
                log::info!(
                    "📋 Found {} results in endpoint/results format",
                    results.len()
                );
                let server_prefix = store.config.server_url.trim_end_matches('/');

                for result_val in results {
                    if let Some(subject) = result_val.as_str() {
                        // Skip external URLs that don't belong to our server
                        if !subject.starts_with(server_prefix) {
                            log::debug!("  ⏭️ Skipping external URL: {}", subject);
                            continue;
                        }

                        log::debug!("  📄 Processing result: {}", subject);
                        match store.get_resource(subject).await {
                            Ok(resource) => {
                                // Try to extract meaningful body content from various properties
                                let body = extract_document_body(&resource.properties);

                                let original_id = resource.subject.clone();
                                let normalized_id = self.normalize_document_id(&original_id);
                                let document = Document {
                                    id: normalized_id,
                                    url: resource.subject.clone(),
                                    title: resource
                                        .properties
                                        .get("https://atomicdata.dev/properties/name")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or(&resource.subject)
                                        .to_string(),
                                    description: resource
                                        .properties
                                        .get("https://atomicdata.dev/properties/description")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    body,
                                    summarization: None,
                                    stub: None,
                                    tags: None,
                                    rank: None,
                                };

                                log::debug!(
                                    "  ✅ Created document: {} ({})",
                                    document.title,
                                    document.id
                                );
                                index.insert(document.id.clone(), document);
                            }
                            Err(e) => {
                                log::warn!("  ❌ Failed to get resource {}: {}", subject, e);
                                continue;
                            }
                        }
                    } else {
                        log::warn!("  ❌ Result is not a string: {:?}", result_val);
                    }
                }
            } else {
                log::debug!("❌ No 'endpoint/results' array found in object response");
                // Fallback: check for simple array format or subjects array
                if let Some(subjects) = obj.get("subjects").and_then(|v| v.as_array()) {
                    log::info!("📋 Found {} subjects in fallback format", subjects.len());
                    for subject_val in subjects {
                        if let Some(subject) = subject_val.as_str() {
                            log::debug!("  📄 Processing subject: {}", subject);
                            match store.get_resource(subject).await {
                                Ok(resource) => {
                                    let body = extract_document_body(&resource.properties);

                                    let original_id = resource.subject.clone();
                                    let normalized_id = self.normalize_document_id(&original_id);
                                    let document = Document {
                                        id: normalized_id,
                                        url: resource.subject.clone(),
                                        title: resource
                                            .properties
                                            .get("https://atomicdata.dev/properties/name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or(&resource.subject)
                                            .to_string(),
                                        description: resource
                                            .properties
                                            .get("https://atomicdata.dev/properties/description")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string()),
                                        body,
                                        summarization: None,
                                        stub: None,
                                        tags: None,
                                        rank: None,
                                    };

                                    log::debug!(
                                        "  ✅ Created document: {} ({})",
                                        document.title,
                                        document.id
                                    );
                                    index.insert(document.id.clone(), document);
                                }
                                Err(e) => {
                                    log::warn!("  ❌ Failed to get resource {}: {}", subject, e);
                                    continue;
                                }
                            }
                        }
                    }
                } else {
                    log::debug!("❌ No recognized result format found in response");
                }
            }
        } else if let Some(results) = search_result.as_array() {
            // Direct array format (legacy support)
            log::info!("📋 Found {} results in direct array format", results.len());
            for result in results {
                if let Some(subject) = result.as_str() {
                    log::debug!("  📄 Processing result: {}", subject);
                    match store.get_resource(subject).await {
                        Ok(resource) => {
                            let body = extract_document_body(&resource.properties);

                            let original_id = resource.subject.clone();
                            let normalized_id = self.normalize_document_id(&original_id);
                            let document = Document {
                                id: normalized_id,
                                url: resource.subject.clone(),
                                title: resource
                                    .properties
                                    .get("https://atomicdata.dev/properties/name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(&resource.subject)
                                    .to_string(),
                                description: resource
                                    .properties
                                    .get("https://atomicdata.dev/properties/description")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                                body,
                                summarization: None,
                                stub: None,
                                tags: None,
                                rank: None,
                            };

                            log::debug!(
                                "  ✅ Created document: {} ({})",
                                document.title,
                                document.id
                            );
                            index.insert(document.id.clone(), document);
                        }
                        Err(e) => {
                            log::warn!("  ❌ Failed to get resource {}: {}", subject, e);
                            continue;
                        }
                    }
                } else {
                    log::warn!("  ❌ Result is not a string: {:?}", result);
                }
            }
        } else {
            log::warn!(
                "❌ Search result is neither array nor object: {:?}",
                search_result
            );
        }

        log::info!("🎯 Final index contains {} documents", index.len());
        Ok(index)
    }
}

/// Check if a URL is valid by parsing it
fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Extract meaningful document body from resource properties.
/// Tries multiple sources in order of preference.
fn extract_document_body(
    properties: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    // First try Terraphim-specific body property
    if let Some(body) = properties
        .get("http://localhost:9883/terraphim-drive/terraphim/property/body")
        .and_then(|v| v.as_str())
    {
        return body.to_string();
    }

    // Then try standard atomic data description
    if let Some(description) = properties
        .get("https://atomicdata.dev/properties/description")
        .and_then(|v| v.as_str())
    {
        return description.to_string();
    }

    // Try name as fallback
    if let Some(name) = properties
        .get("https://atomicdata.dev/properties/name")
        .and_then(|v| v.as_str())
    {
        return name.to_string();
    }

    // Fallback to serialized properties
    serde_json::to_string(properties).unwrap_or_default()
}
