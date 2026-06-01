//! Document capability for `TerraphimService` (document creation, retrieval,
//! and KG-link preprocessing). Split from lib.rs as part of the Gitea #1910
//! god-file decomposition; behaviour unchanged. Methods remain on
//! `TerraphimService`, so the public API is identical.

use terraphim_automata::{LinkType, replace_matches};
use terraphim_config::Role;
use terraphim_persistence::Persistable;
use terraphim_types::{Document, Layer, NormalizedTermValue, SearchQuery, Thesaurus};

use super::{Result, TerraphimService, normalize_filename_to_id, snippet_around};

impl TerraphimService {
    /// Preprocess document content to create clickable KG links when terraphim_it is enabled
    ///
    /// This function replaces KG terms in the document body with markdown links
    /// in the format `[term](kg:term)` which can be intercepted by the frontend
    /// to display KG documents when clicked.
    pub async fn preprocess_document_content(
        &mut self,
        mut document: Document,
        role: &Role,
    ) -> Result<Document> {
        // Only preprocess if terraphim_it is enabled and role has KG configured
        if !role.terraphim_it {
            log::info!(
                "🔍 terraphim_it disabled for role '{}', skipping KG preprocessing",
                role.name
            );
            return Ok(document);
        }

        let Some(_kg) = &role.kg else {
            log::info!(
                "⚠️ No KG configured for role '{}', skipping KG preprocessing",
                role.name
            );
            return Ok(document);
        };

        log::info!(
            "🧠 Starting KG preprocessing for document '{}' in role '{}' (terraphim_it enabled)",
            document.title,
            role.name
        );
        log::debug!(
            "📄 Document preview: {} characters starting with: {}",
            document.body.len(),
            &document.body.chars().take(100).collect::<String>()
        );

        // Load thesaurus for the role
        let thesaurus = match self.ensure_thesaurus_loaded(&role.name).await {
            Ok(thesaurus) => thesaurus,
            Err(e) => {
                log::warn!("Failed to load thesaurus for role {}: {:?}", role.name, e);
                return Ok(document); // Return original document if thesaurus fails to load
            }
        };

        // Filter thesaurus to only include meaningful terms and avoid over-linking
        let mut kg_thesaurus = Thesaurus::new(format!("kg_links_{}", role.name));

        // Prioritize important KG terms while excluding overly generic ones
        // Key KG concepts should always be included even if they're common
        let important_kg_terms = [
            "graph",
            "haystack",
            "service",
            "terraphim",
            "knowledge",
            "embedding",
            "search",
            "automata",
            "thesaurus",
            "rolegraph",
        ];

        // Exclude only very generic programming/technical terms that don't add value
        let excluded_common_terms = [
            "system",
            "config",
            "configuration",
            "type",
            "method",
            "function",
            "class",
            "component",
            "module",
            "library",
            "framework",
            "interface",
            "api",
            "data",
            "file",
            "path",
            "url",
            "string",
            "number",
            "value",
            "option",
            "parameter",
            "field",
            "property",
            "attribute",
            "element",
            "item",
            "object",
            "array",
            "list",
            "map",
            "set",
            "collection",
            "server",
            "client",
            "request",
            "response",
            "error",
            "result",
            "success",
            "failure",
            "true",
            "false",
            "null",
            "undefined",
            "empty",
            "full",
            "start",
            "end",
            "begin",
            "finish",
            "create",
            "delete",
            "update",
            "read",
            "write",
            "load",
            "save",
            "process",
            "handle",
            "manage",
            "control",
            "execute",
            "run",
            "call",
            "invoke",
            "trigger",
            "event",
            "action",
            "command",
            "query",
            "search",
            "filter",
            "sort",
            "order",
            "group",
            "match",
            "find",
            "replace",
            "insert",
            "remove",
            "add",
            "set",
            "get",
            "put",
            "post",
            "head",
            "patch",
            "delete",
        ];

        let mut sorted_terms: Vec<_> = (&thesaurus)
            .into_iter()
            .filter(|(key, _)| {
                let term = key.as_str();

                // Always exclude empty or very short terms
                if term.is_empty() || term.len() < 3 {
                    return false;
                }

                // Always include important KG terms, even if they're short
                if important_kg_terms.contains(&term) {
                    return true;
                }

                // Exclude generic technical terms
                if excluded_common_terms.contains(&term) {
                    return false;
                }

                // Include terms that are:
                // 1. Moderately long (>5 chars) OR
                // 2. Hyphenated compound terms OR
                // 3. Underscore-separated compound terms OR
                // 4. Capitalized terms (likely proper nouns or important concepts)
                term.len() > 5
                    || term.contains('-')
                    || term.contains('_')
                    || term.chars().next().is_some_and(|c| c.is_uppercase())
            })
            .collect();

        // Sort by relevance, but prioritize important KG terms
        #[allow(clippy::unnecessary_sort_by)]
        sorted_terms.sort_by(|a, b| {
            let a_important = important_kg_terms.contains(&a.0.as_str());
            let b_important = important_kg_terms.contains(&b.0.as_str());

            match (a_important, b_important) {
                (true, false) => std::cmp::Ordering::Less, // a comes first
                (false, true) => std::cmp::Ordering::Greater, // b comes first
                _ => b.1.id.cmp(&a.1.id),                  // Both or neither important, sort by ID
            }
        });

        // Take more terms since we're being more selective about quality
        let max_kg_terms = 8;
        for (key, value) in sorted_terms.into_iter().take(max_kg_terms) {
            let mut kg_value = value.clone();
            // IMPORTANT: Keep the original term (key) as visible text, link to root concept (value.value)
            // This creates links like: [graph embeddings](kg:terraphim-graph)
            // where "graph embeddings" stays visible but links to the root concept "terraphim-graph"
            kg_value.value = key.clone(); // Keep original term as visible text
            kg_value.url = Some(format!("kg:{}", value.value)); // Link to the root concept
            kg_thesaurus.insert(key.clone(), kg_value);
        }

        let kg_terms_count = kg_thesaurus.len();
        log::info!(
            "📋 KG thesaurus filtering: {} → {} terms (prioritizing: {}, filters: len>5, hyphenated, or important KG terms)",
            thesaurus.len(),
            kg_terms_count,
            important_kg_terms.join(", ")
        );

        // Log the actual terms that passed filtering for debugging
        if kg_terms_count > 0 {
            let terms: Vec<String> = (&kg_thesaurus)
                .into_iter()
                .map(|(k, v)| format!("'{}' → kg:{}", k, v.value))
                .collect();
            log::info!("🔍 KG terms selected for linking: {}", terms.join(", "));
        } else {
            log::info!(
                "⚠️ No KG terms passed filtering criteria - document '{}' will have no KG links",
                document.title
            );
        }

        // Apply KG term replacement to document body (only if we have terms to replace)
        if !kg_thesaurus.is_empty() {
            // Debug: log what we're about to pass to replace_matches
            let debug_thesaurus: Vec<String> = (&kg_thesaurus)
                .into_iter()
                .map(|(k, v)| format!("'{}' -> '{}' (url: {:?})", k, v.value, v.url))
                .take(3) // Limit to first 3 entries to avoid spam
                .collect();
            log::info!(
                "🔧 Passing to replace_matches: {} (total terms: {})",
                debug_thesaurus.join(", "),
                kg_thesaurus.len()
            );
            let preview = if document.body.chars().count() > 200 {
                document.body.chars().take(200).collect::<String>() + "..."
            } else {
                document.body.clone()
            };
            log::info!("📝 Document body preview (first 200 chars): {}", preview);

            match replace_matches(&document.body, kg_thesaurus, LinkType::MarkdownLinks) {
                Ok(processed_bytes) => {
                    match String::from_utf8(processed_bytes) {
                        Ok(processed_content) => {
                            log::info!(
                                "✅ Successfully preprocessed document '{}' with {} KG terms → created [term](kg:concept) links",
                                document.title,
                                kg_terms_count
                            );

                            // Debug: Check if content actually changed
                            let content_changed = processed_content != document.body;
                            log::info!(
                                "🔄 Content changed: {} (original: {} chars, processed: {} chars)",
                                content_changed,
                                document.body.len(),
                                processed_content.len()
                            );

                            // Debug: Show actual KG links in the processed content
                            let kg_links: Vec<&str> = processed_content
                                .split("[")
                                .filter_map(|s| s.find("](kg:").map(|closing| &s[..closing]))
                                .collect();

                            if !kg_links.is_empty() {
                                log::info!(
                                    "🔗 Found KG links in processed content: [{}](kg:...)",
                                    kg_links.join("], [")
                                );

                                let snippet = snippet_around(&processed_content, "](kg:", 50, 100);
                                if !snippet.is_empty() {
                                    log::info!(
                                        "📄 Content snippet with KG link: ...{}...",
                                        snippet
                                    );
                                }
                            } else {
                                log::warn!(
                                    "⚠️ No KG links found in processed content despite successful replacement"
                                );
                            }

                            document.body = processed_content;
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to convert processed content to UTF-8 for document '{}': {:?}",
                                document.title,
                                e
                            );
                        }
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to replace KG terms in document '{}': {:?}",
                        document.title,
                        e
                    );
                }
            }
        } else {
            log::info!(
                "💭 No specific KG terms found for document '{}' (filters excluded generic terms)",
                document.title
            );
        }

        Ok(document)
    }

    /// Preprocess document content with both KG linking and search term highlighting
    pub async fn preprocess_document_content_with_search(
        &mut self,
        document: Document,
        role: &Role,
        search_query: Option<&SearchQuery>,
    ) -> Result<Document> {
        // First apply KG preprocessing if enabled
        let mut processed_doc = self.preprocess_document_content(document, role).await?;

        // Then apply search term highlighting if query is provided
        if let Some(query) = search_query {
            log::debug!(
                "Applying search term highlighting to document '{}'",
                processed_doc.title
            );
            processed_doc.body = Self::highlight_search_terms(&processed_doc.body, query);
        }

        Ok(processed_doc)
    }

    /// Create document
    pub async fn create_document(&mut self, document: Document) -> Result<Document> {
        // Persist the document using the fastest available Operator. The document becomes
        // available on all profiles/devices thanks to the Persistable implementation.
        document.save().await?;

        // Index the freshly-saved document inside all role graphs so it can be discovered via
        // search immediately.
        self.config_state.add_to_roles(&document).await?;

        // 🔄 Persist the updated body back to on-disk Markdown files for every writable
        // ripgrep haystack so that subsequent searches (and external tooling) see the
        // changes instantly.
        use terraphim_config::ServiceType;
        use terraphim_middleware::indexer::RipgrepIndexer;

        let ripgrep = RipgrepIndexer::default();
        let config_snapshot = { self.config_state.config.lock().await.clone() };

        for role in config_snapshot.roles.values() {
            for haystack in &role.haystacks {
                if haystack.service == ServiceType::Ripgrep && !haystack.read_only {
                    if let Err(e) = ripgrep.update_document(&document).await {
                        log::warn!(
                            "Failed to write document {} to haystack {:?}: {:?}",
                            document.id,
                            haystack.location,
                            e
                        );
                    }
                }
            }
        }

        Ok(document)
    }

    /// Get document by ID
    ///
    /// This method supports both normalized IDs (e.g., "haystackmd") and original filenames (e.g., "haystack.md").
    /// It tries to find the document using the provided ID first, then tries with a normalized version,
    /// and finally falls back to searching by title.
    pub async fn get_document_by_id(&mut self, document_id: &str) -> Result<Option<Document>> {
        log::debug!("Getting document by ID: '{}'", document_id);

        // Validate document_id is not empty or whitespace-only
        if document_id.trim().is_empty() {
            log::warn!("Empty or whitespace-only document_id provided");
            return Ok(None);
        }

        // 1️⃣ Try to load the document directly using the provided ID
        let mut placeholder = Document {
            id: document_id.to_string(),
            ..Default::default()
        };
        match placeholder.load().await {
            Ok(doc) => {
                log::debug!("Found document '{}' with direct ID lookup", document_id);
                return self.apply_kg_preprocessing_if_needed(doc).await.map(Some);
            }
            Err(e) => {
                log::debug!(
                    "Document '{}' not found with direct lookup: {:?}",
                    document_id,
                    e
                );
            }
        }

        // 2️⃣ If the provided ID looks like a filename, try with normalized ID
        if document_id.contains('.') || document_id.contains('-') || document_id.contains('_') {
            let normalized_id = normalize_filename_to_id(document_id);
            log::debug!(
                "Trying normalized ID '{}' for filename '{}'",
                normalized_id,
                document_id
            );

            let mut normalized_placeholder = Document {
                id: normalized_id.clone(),
                ..Default::default()
            };
            match normalized_placeholder.load().await {
                Ok(doc) => {
                    log::debug!(
                        "Found document '{}' with normalized ID '{}'",
                        document_id,
                        normalized_id
                    );
                    return self.apply_kg_preprocessing_if_needed(doc).await.map(Some);
                }
                Err(e) => {
                    log::debug!(
                        "Document '{}' not found with normalized ID '{}': {:?}",
                        document_id,
                        normalized_id,
                        e
                    );
                }
            }
        }

        // 3️⃣ Fallback: search by title (for documents where title contains the original filename)
        log::debug!("Falling back to search for document '{}'", document_id);
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(document_id.to_string()),
            search_terms: None,
            operator: None,
            limit: Some(5), // Get a few results to check titles
            skip: None,
            role: None,
            layer: Layer::default(),
            include_pinned: false,
            min_quality: None,
        };

        let documents = self.search(&search_query).await?;

        // Look for a document whose title matches the requested ID
        for doc in documents {
            if doc.title == document_id || doc.id == document_id {
                log::debug!("Found document '{}' via search fallback", document_id);
                return self.apply_kg_preprocessing_if_needed(doc).await.map(Some);
            }
        }

        log::debug!("Document '{}' not found anywhere", document_id);
        Ok(None)
    }

    /// Apply KG preprocessing to a document if needed based on the current selected role
    ///
    /// This helper method checks if the selected role has terraphim_it enabled
    /// and applies KG term preprocessing accordingly. It prevents double processing
    /// by checking if KG links already exist in the document.
    async fn apply_kg_preprocessing_if_needed(&mut self, document: Document) -> Result<Document> {
        log::debug!(
            "🔍 [KG-DEBUG] apply_kg_preprocessing_if_needed called for document: '{}'",
            document.title
        );
        log::debug!(
            "🔍 [KG-DEBUG] Document body preview: {}",
            document.body.chars().take(100).collect::<String>()
        );

        let role = {
            let config = self.config_state.config.lock().await;
            let selected_role = &config.selected_role;

            log::debug!("🔍 [KG-DEBUG] Selected role: '{}'", selected_role);

            match config.roles.get(selected_role) {
                Some(role) => {
                    log::debug!(
                        "🔍 [KG-DEBUG] Role found: '{}', terraphim_it: {}",
                        role.name,
                        role.terraphim_it
                    );
                    role.clone() // Clone to avoid borrowing issues
                }
                None => {
                    log::warn!(
                        "❌ [KG-DEBUG] Selected role '{}' not found in config, skipping KG preprocessing",
                        selected_role
                    );
                    return Ok(document);
                }
            }
        }; // Release the lock here

        // Only apply preprocessing if role has terraphim_it enabled
        if !role.terraphim_it {
            log::info!(
                "🔍 [KG-DEBUG] terraphim_it disabled for role '{}', skipping KG preprocessing",
                role.name
            );
            return Ok(document);
        }

        // Check if document already has KG links to prevent double processing
        let has_existing_kg_links = document.body.contains("](kg:");
        log::debug!(
            "🔍 [KG-DEBUG] Document already has KG links: {}",
            has_existing_kg_links
        );
        if has_existing_kg_links {
            log::info!(
                "🔍 [KG-DEBUG] Document '{}' already has KG links, skipping preprocessing to prevent double processing",
                document.title
            );
            return Ok(document);
        }

        log::info!(
            "🧠 [KG-DEBUG] Starting KG preprocessing for document '{}' with role '{}' (terraphim_it enabled)",
            document.title,
            role.name
        );

        // Apply KG preprocessing
        let document_title = document.title.clone(); // Save title before moving document
        let processed_doc = match self.preprocess_document_content(document, &role).await {
            Ok(doc) => {
                let links_added = doc.body.contains("](kg:");
                log::info!(
                    "✅ [KG-DEBUG] KG preprocessing completed for document '{}'. Links added: {}",
                    doc.title,
                    links_added
                );
                if links_added {
                    log::debug!(
                        "🔍 [KG-DEBUG] Processed body preview: {}",
                        doc.body.chars().take(200).collect::<String>()
                    );
                }
                doc
            }
            Err(e) => {
                log::error!(
                    "❌ [KG-DEBUG] KG preprocessing failed for document '{}': {:?}",
                    document_title,
                    e
                );
                return Err(e);
            }
        };

        Ok(processed_doc)
    }
}
