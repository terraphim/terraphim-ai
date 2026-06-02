//! Search capability for `TerraphimService` (query execution, relevance
//! scoring, logical operators, and knowledge-graph term lookup). Split from
//! lib.rs as part of the Gitea #1910 god-file decomposition; behaviour
//! unchanged. Methods remain on `TerraphimService`, so the public API is identical.

use terraphim_config::Role;
use terraphim_persistence::Persistable;
use terraphim_types::{
    Document, Index, IndexedDocument, Layer, NormalizedTermValue, RelevanceFunction, RoleName,
    SearchQuery,
};

use super::{Result, ServiceError, TerraphimService, normalize_filename_to_id};
use crate::score::{self, Query};

impl TerraphimService {
    async fn get_search_role(&self, search_query: &SearchQuery) -> Result<Role> {
        let search_role = match &search_query.role {
            Some(role) => role.clone(),
            None => self.config_state.get_default_role().await,
        };

        log::debug!("Searching for role: {:?}", search_role);
        let Some(role) = self.config_state.get_role(&search_role).await else {
            return Err(ServiceError::Config(format!(
                "Role `{}` not found in config",
                search_role
            )));
        };
        Ok(role)
    }

    /// Check if a character is a word boundary (not alphanumeric or underscore).
    /// This provides Unicode-aware word boundary detection.
    fn is_word_boundary_char(c: char) -> bool {
        !c.is_alphanumeric() && c != '_'
    }

    /// Check if a match position is at word boundaries in the text.
    /// Returns true if the character before start (or start of string) and
    /// the character after end (or end of string) are word boundary characters.
    fn is_at_word_boundary(text: &str, start: usize, end: usize) -> bool {
        let before_ok = if start == 0 {
            true
        } else {
            text[..start]
                .chars()
                .last()
                .map(Self::is_word_boundary_char)
                .unwrap_or(true)
        };

        let after_ok = if end >= text.len() {
            true
        } else {
            text[end..]
                .chars()
                .next()
                .map(Self::is_word_boundary_char)
                .unwrap_or(true)
        };

        before_ok && after_ok
    }

    /// Match a term against text using unicode-aware word boundaries.
    /// Returns true if the term appears as a complete word (not as part of another word).
    /// Both inputs should already be lowercase for efficiency.
    fn term_matches_with_word_boundaries(term: &str, text: &str) -> bool {
        // Find all occurrences of the term in the text
        let mut start = 0;
        while let Some(pos) = text[start..].find(term) {
            let abs_start = start + pos;
            let abs_end = abs_start + term.len();

            if Self::is_at_word_boundary(text, abs_start, abs_end) {
                return true;
            }
            start = abs_end;
        }
        false
    }

    /// Apply logical operators (AND/OR) to filter documents based on multiple search terms
    pub async fn apply_logical_operators_to_documents(
        &mut self,
        search_query: &SearchQuery,
        documents: Vec<Document>,
    ) -> Result<Vec<Document>> {
        use terraphim_types::LogicalOperator;

        let all_terms = search_query.get_all_terms();
        let operator = search_query.get_operator();

        let initial_doc_count = documents.len();

        log::debug!(
            "Applying {:?} operator to {} documents with {} search terms",
            operator,
            initial_doc_count,
            all_terms.len()
        );

        // Pre-compute lowercase terms once for efficiency
        let terms_lower: Vec<String> = all_terms
            .iter()
            .map(|t| t.as_str().to_lowercase())
            .collect();

        let filtered_docs: Vec<Document> = documents
            .into_iter()
            .filter(|doc| {
                // Create searchable text from document
                let searchable_text = format!(
                    "{} {} {}",
                    doc.title.to_lowercase(),
                    doc.body.to_lowercase(),
                    doc.description
                        .as_ref()
                        .unwrap_or(&String::new())
                        .to_lowercase()
                );

                match operator {
                    LogicalOperator::And => {
                        // Document must contain ALL terms as whole words
                        terms_lower.iter().all(|term| {
                            Self::term_matches_with_word_boundaries(term, &searchable_text)
                        })
                    }
                    LogicalOperator::Or => {
                        // Document must contain ANY term as a whole word
                        terms_lower.iter().any(|term| {
                            Self::term_matches_with_word_boundaries(term, &searchable_text)
                        })
                    }
                }
            })
            .collect();

        log::debug!(
            "Logical operator filtering: {} -> {} documents",
            initial_doc_count,
            filtered_docs.len()
        );

        // Sort filtered documents by relevance using a combined query
        let combined_query_string = terms_lower.join(" ");
        let query = Query::new(&combined_query_string);
        let sorted_docs = score::sort_documents(&query, filtered_docs);

        Ok(sorted_docs)
    }

    /// search for documents in the haystacks with selected role from the config
    /// and return the documents sorted by relevance
    pub async fn search_documents_selected_role(
        &mut self,
        search_term: &NormalizedTermValue,
    ) -> Result<Vec<Document>> {
        let role = self.config_state.get_selected_role().await;
        let documents = self
            .search(&SearchQuery {
                search_term: search_term.clone(),
                search_terms: None,
                operator: None,
                role: Some(role),
                skip: None,
                limit: None,
                layer: Layer::default(),
                include_pinned: false,
                min_quality: None,
            })
            .await?;
        Ok(documents)
    }

    /// Filter documents by minimum composite quality score.
    ///
    /// Documents without a quality score are excluded when `min_quality` is set.
    /// Out-of-range thresholds are clamped to `[0.0, 1.0]` before comparison.
    pub(crate) fn apply_min_quality_filter(
        docs: Vec<Document>,
        min_quality: Option<f64>,
    ) -> Vec<Document> {
        let Some(threshold) = min_quality else {
            return docs;
        };
        let threshold = threshold.clamp(0.0, 1.0);
        docs.into_iter()
            .filter(|doc| {
                doc.quality_score
                    .as_ref()
                    .map(|qs| qs.composite() >= threshold)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Search for documents in the haystacks
    pub async fn search(&mut self, search_query: &SearchQuery) -> Result<Vec<Document>> {
        // Get the role from the config
        log::debug!("Role for searching: {:?}", search_query.role);
        let role = self.get_search_role(search_query).await?;

        log::trace!("Building index for search query: {:?}", search_query);
        let index: Index =
            terraphim_middleware::search_haystacks(self.config_state.clone(), search_query.clone())
                .await?;

        let min_quality = search_query.min_quality;

        let docs_result: Result<Vec<Document>> = match role.relevance_function {
            RelevanceFunction::TitleScorer => {
                log::debug!("Searching haystack with title scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by relevance");

                let documents = if search_query.is_multi_term_query() {
                    // Handle multi-term queries with logical operators
                    self.apply_logical_operators_to_documents(search_query, documents)
                        .await?
                } else {
                    // Single term query (backward compatibility)
                    let query = Query::new(&search_query.search_term.to_string());
                    score::sort_documents(&query, documents)
                };
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let mut document: terraphim_types::Document = doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);

                    // 🔄 Enhanced persistence layer integration for both local and Atomic Data documents
                    if document.id.starts_with("http://") || document.id.starts_with("https://") {
                        // Atomic Data document: Check persistence first, then save for future queries
                        log::debug!(
                            "Processing Atomic Data document '{}' (URL: {})",
                            document.title,
                            document.id
                        );

                        // Try to load from persistence first (for cached Atomic Data documents)
                        let mut placeholder = Document {
                            id: document.id.clone(),
                            ..Default::default()
                        };
                        match placeholder.load().await {
                            Ok(persisted_doc) => {
                                // Found in persistence - use cached version
                                log::debug!(
                                    "Found cached Atomic Data document '{}' in persistence",
                                    document.title
                                );
                                if let Some(better_description) = persisted_doc.description {
                                    document.description = Some(better_description);
                                }
                                // Update body if the persisted version has better content
                                // But DO NOT overwrite if this role uses KG preprocessing (terraphim_it)
                                // because we need to preserve the processed content with KG links
                                if !persisted_doc.body.is_empty() && !role.terraphim_it {
                                    log::debug!(
                                        "Updated body from persistence for Atomic document '{}' (role: '{}', terraphim_it: {})",
                                        document.title,
                                        role.name,
                                        role.terraphim_it
                                    );
                                    document.body = persisted_doc.body;
                                } else if role.terraphim_it {
                                    log::debug!(
                                        "Keeping search result body for Atomic document '{}' because role '{}' uses KG preprocessing (terraphim_it=true)",
                                        document.title,
                                        role.name
                                    );
                                }
                            }
                            Err(_) => {
                                // Not in persistence - save this Atomic Data document for future queries
                                log::debug!(
                                    "Caching Atomic Data document '{}' to persistence for future queries",
                                    document.title
                                );

                                // Save in background to avoid blocking the response
                                let doc_to_save = document.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = doc_to_save.save().await {
                                        log::warn!(
                                            "Failed to cache Atomic Data document '{}': {}",
                                            doc_to_save.title,
                                            e
                                        );
                                    } else {
                                        log::debug!(
                                            "Successfully cached Atomic Data document '{}'",
                                            doc_to_save.title
                                        );
                                    }
                                });
                            }
                        }
                    } else {
                        // Local document: Try direct persistence lookup first
                        let should_lookup_persistence = document
                            .get_source_haystack()
                            .and_then(|source| {
                                role.haystacks
                                    .iter()
                                    .find(|haystack| haystack.location == *source)
                            })
                            .map(|haystack| haystack.fetch_content)
                            .unwrap_or(true);

                        if !should_lookup_persistence {
                            log::trace!(
                                "Skipping persistence lookup for '{}' (haystack fetch_content=false)",
                                document.title
                            );
                        } else {
                            let mut placeholder = Document {
                                id: document.id.clone(),
                                ..Default::default()
                            };
                            if let Ok(persisted_doc) = placeholder.load().await {
                                if let Some(better_description) = persisted_doc.description {
                                    log::debug!(
                                        "Replaced ripgrep description for '{}' with persistence description",
                                        document.title
                                    );
                                    document.description = Some(better_description);
                                }
                            } else {
                                // Try normalized ID based on document title (filename)
                                // For KG files, the title might be "haystack" but persistence ID is "haystackmd"
                                let normalized_id = normalize_filename_to_id(&document.title);

                                let mut normalized_placeholder = Document {
                                    id: normalized_id.clone(),
                                    ..Default::default()
                                };
                                if let Ok(persisted_doc) = normalized_placeholder.load().await {
                                    if let Some(better_description) = persisted_doc.description {
                                        log::debug!(
                                            "Replaced ripgrep description for '{}' with persistence description (normalized from title: {})",
                                            document.title,
                                            normalized_id
                                        );
                                        document.description = Some(better_description);
                                    }
                                } else {
                                    // Try with "md" suffix for KG files (title "haystack" -> ID "haystackmd")
                                    let normalized_id_with_md = format!("{}md", normalized_id);
                                    let mut md_placeholder = Document {
                                        id: normalized_id_with_md.clone(),
                                        ..Default::default()
                                    };
                                    if let Ok(persisted_doc) = md_placeholder.load().await {
                                        if let Some(better_description) = persisted_doc.description
                                        {
                                            log::debug!(
                                                "Replaced ripgrep description for '{}' with persistence description (normalized with md: {})",
                                                document.title,
                                                normalized_id_with_md
                                            );
                                            document.description = Some(better_description);
                                        }
                                    } else {
                                        log::debug!(
                                            "No persistence document found for '{}' (tried ID: '{}', normalized: '{}', with md: '{}')",
                                            document.title,
                                            document.id,
                                            normalized_id,
                                            normalized_id_with_md
                                        );
                                    }
                                }
                            }
                        }
                    }

                    docs_ranked.push(document);
                }

                // Apply OpenRouter AI summarization if enabled for this role and auto-summarize is on
                // Apply AI summarization if enabled via OpenRouter or generic LLM config
                #[cfg(feature = "openrouter")]
                if role.has_llm_config() && role.llm_auto_summarize {
                    log::debug!(
                        "Applying OpenRouter AI summarization to {} search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                } else {
                    // Always apply LLM AI summarization if LLM client is available
                    eprintln!(
                        "📋 Entering LLM AI summarization branch for role: {}",
                        role.name
                    );
                    log::debug!(
                        "Applying LLM AI summarization to {} search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                }

                // Apply KG preprocessing if enabled for this role (but only once, not in individual document loads)
                if role.terraphim_it {
                    log::info!(
                        "🧠 Applying KG preprocessing to {} TerraphimGraph search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    let mut processed_docs = Vec::new();
                    let mut total_kg_terms = 0;
                    let mut docs_with_kg_links = 0;

                    for document in docs_ranked {
                        let original_body_len = document.body.len();
                        let processed_doc =
                            self.preprocess_document_content(document, &role).await?;

                        // Count KG links added (rough estimate by body size increase)
                        let new_body_len = processed_doc.body.len();
                        if new_body_len > original_body_len {
                            docs_with_kg_links += 1;
                            // Rough estimate: each KG link adds ~15-20 chars on average
                            let estimated_links = (new_body_len - original_body_len) / 17;
                            total_kg_terms += estimated_links;
                        }

                        processed_docs.push(processed_doc);
                    }

                    log::info!(
                        "✅ KG preprocessing complete: {} documents processed, {} received KG links (~{} total links)",
                        processed_docs.len(),
                        docs_with_kg_links,
                        total_kg_terms
                    );
                    Ok(processed_docs)
                } else {
                    Ok(docs_ranked)
                }
            }
            RelevanceFunction::BM25 => {
                log::debug!("Searching haystack with BM25 scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by BM25 relevance");

                let documents = if search_query.is_multi_term_query() {
                    // Handle multi-term queries with logical operators
                    let filtered_docs = self
                        .apply_logical_operators_to_documents(search_query, documents)
                        .await?;
                    // Apply BM25 scoring to filtered documents
                    let combined_query_string = search_query
                        .get_all_terms()
                        .iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let query =
                        Query::new(&combined_query_string).name_scorer(score::QueryScorer::BM25);
                    score::sort_documents(&query, filtered_docs)
                } else {
                    // Single term query (backward compatibility)
                    let query = Query::new(&search_query.search_term.to_string())
                        .name_scorer(score::QueryScorer::BM25);
                    score::sort_documents(&query, documents)
                };
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let mut document: terraphim_types::Document = doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);
                    docs_ranked.push(document);
                }

                // Apply OpenRouter AI summarization if enabled for this role and auto-summarize is on
                #[cfg(feature = "openrouter")]
                if role.has_llm_config() && role.llm_auto_summarize {
                    log::debug!(
                        "Applying OpenRouter AI summarization to {} BM25 search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                } else {
                    // Always apply LLM AI summarization if LLM client is available
                    log::debug!(
                        "Applying LLM AI summarization to {} BM25 search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                }

                // Apply KG preprocessing if enabled for this role
                if role.terraphim_it {
                    log::info!(
                        "🧠 Applying KG preprocessing to {} BM25 search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    let mut processed_docs = Vec::new();
                    let mut total_kg_terms = 0;
                    let mut docs_with_kg_links = 0;

                    for document in docs_ranked {
                        let original_body_len = document.body.len();
                        let processed_doc =
                            self.preprocess_document_content(document, &role).await?;

                        // Count KG links added (rough estimate by body size increase)
                        let new_body_len = processed_doc.body.len();
                        if new_body_len > original_body_len {
                            docs_with_kg_links += 1;
                            let estimated_links = (new_body_len - original_body_len) / 17;
                            total_kg_terms += estimated_links;
                        }

                        processed_docs.push(processed_doc);
                    }

                    log::info!(
                        "✅ KG preprocessing complete: {} documents processed, {} received KG links (~{} total links)",
                        processed_docs.len(),
                        docs_with_kg_links,
                        total_kg_terms
                    );
                    Ok(processed_docs)
                } else {
                    Ok(docs_ranked)
                }
            }
            RelevanceFunction::BM25F => {
                log::debug!("Searching haystack with BM25F scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by BM25F relevance");

                let documents = if search_query.is_multi_term_query() {
                    // Handle multi-term queries with logical operators
                    let filtered_docs = self
                        .apply_logical_operators_to_documents(search_query, documents)
                        .await?;
                    // Apply BM25F scoring to filtered documents
                    let combined_query_string = search_query
                        .get_all_terms()
                        .iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let query =
                        Query::new(&combined_query_string).name_scorer(score::QueryScorer::BM25F);
                    score::sort_documents(&query, filtered_docs)
                } else {
                    // Single term query (backward compatibility)
                    let query = Query::new(&search_query.search_term.to_string())
                        .name_scorer(score::QueryScorer::BM25F);
                    score::sort_documents(&query, documents)
                };
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let mut document: terraphim_types::Document = doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);
                    docs_ranked.push(document);
                }

                // Apply OpenRouter AI summarization if enabled for this role and auto-summarize is on
                #[cfg(feature = "openrouter")]
                if role.has_llm_config() && role.llm_auto_summarize {
                    log::debug!(
                        "Applying OpenRouter AI summarization to {} BM25F search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                } else {
                    // Always apply LLM AI summarization if LLM client is available
                    log::debug!(
                        "Applying LLM AI summarization to {} BM25F search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                }

                // Apply KG preprocessing if enabled for this role
                if role.terraphim_it {
                    log::info!(
                        "🧠 Applying KG preprocessing to {} BM25F search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    let mut processed_docs = Vec::new();
                    let mut total_kg_terms = 0;
                    let mut docs_with_kg_links = 0;

                    for document in docs_ranked {
                        let original_body_len = document.body.len();
                        let processed_doc =
                            self.preprocess_document_content(document, &role).await?;

                        // Count KG links added (rough estimate by body size increase)
                        let new_body_len = processed_doc.body.len();
                        if new_body_len > original_body_len {
                            docs_with_kg_links += 1;
                            let estimated_links = (new_body_len - original_body_len) / 17;
                            total_kg_terms += estimated_links;
                        }

                        processed_docs.push(processed_doc);
                    }

                    log::info!(
                        "✅ KG preprocessing complete: {} documents processed, {} received KG links (~{} total links)",
                        processed_docs.len(),
                        docs_with_kg_links,
                        total_kg_terms
                    );
                    Ok(processed_docs)
                } else {
                    Ok(docs_ranked)
                }
            }
            RelevanceFunction::BM25Plus => {
                log::debug!("Searching haystack with BM25Plus scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by BM25Plus relevance");

                let documents = if search_query.is_multi_term_query() {
                    // Handle multi-term queries with logical operators
                    let filtered_docs = self
                        .apply_logical_operators_to_documents(search_query, documents)
                        .await?;
                    // Apply BM25Plus scoring to filtered documents
                    let combined_query_string = search_query
                        .get_all_terms()
                        .iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(" ");
                    let query = Query::new(&combined_query_string)
                        .name_scorer(score::QueryScorer::BM25Plus);
                    score::sort_documents(&query, filtered_docs)
                } else {
                    // Single term query (backward compatibility)
                    let query = Query::new(&search_query.search_term.to_string())
                        .name_scorer(score::QueryScorer::BM25Plus);
                    score::sort_documents(&query, documents)
                };
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let mut document: terraphim_types::Document = doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);
                    docs_ranked.push(document);
                }

                // Apply OpenRouter AI summarization if enabled for this role and auto-summarize is on
                #[cfg(feature = "openrouter")]
                if role.has_llm_config() && role.llm_auto_summarize {
                    log::debug!(
                        "Applying OpenRouter AI summarization to {} BM25Plus search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    docs_ranked = self
                        .enhance_descriptions_with_ai(docs_ranked, &role)
                        .await?;
                }

                // Apply KG preprocessing if enabled for this role
                if role.terraphim_it {
                    log::info!(
                        "🧠 Applying KG preprocessing to {} BM25Plus search results for role '{}'",
                        docs_ranked.len(),
                        role.name
                    );
                    let mut processed_docs = Vec::new();
                    let mut total_kg_terms = 0;
                    let mut docs_with_kg_links = 0;

                    for document in docs_ranked {
                        let original_body_len = document.body.len();
                        let processed_doc =
                            self.preprocess_document_content(document, &role).await?;

                        // Count KG links added (rough estimate by body size increase)
                        let new_body_len = processed_doc.body.len();
                        if new_body_len > original_body_len {
                            docs_with_kg_links += 1;
                            let estimated_links = (new_body_len - original_body_len) / 17;
                            total_kg_terms += estimated_links;
                        }

                        processed_docs.push(processed_doc);
                    }

                    log::info!(
                        "✅ KG preprocessing complete: {} documents processed, {} received KG links (~{} total links)",
                        processed_docs.len(),
                        docs_with_kg_links,
                        total_kg_terms
                    );
                    Ok(processed_docs)
                } else {
                    Ok(docs_ranked)
                }
            }
            RelevanceFunction::TerraphimGraph => {
                log::debug!("TerraphimGraph search initiated for role: {}", role.name);
                self.build_thesaurus(search_query).await?;
                let _thesaurus = self.ensure_thesaurus_loaded(&role.name).await?;
                let scored_index_docs: Vec<IndexedDocument> = self
                    .config_state
                    .search_indexed_documents(search_query, &role)
                    .await;

                log::debug!(
                    "TerraphimGraph search found {} indexed documents",
                    scored_index_docs.len()
                );

                // Apply to ripgrep vector of document output
                // I.e. use the ranking of thesaurus to rank the documents here
                log::debug!("Ranking documents with thesaurus");
                let mut documents = index.get_documents(scored_index_docs.clone());

                // CRITICAL FIX: Index all haystack documents into rolegraph if not already present
                // This ensures TerraphimGraph search can find documents discovered by haystacks
                let all_haystack_docs = index.get_all_documents();
                log::debug!(
                    "Found {} total documents from haystacks, checking which need indexing",
                    all_haystack_docs.len()
                );
                let mut need_reindexing = false;

                if let Some(rolegraph_sync) = self.config_state.roles.get(&role.name) {
                    let mut rolegraph = rolegraph_sync.lock().await;
                    let mut newly_indexed = 0;

                    for doc in &all_haystack_docs {
                        // Only index documents that aren't already in the rolegraph
                        if !rolegraph.has_document(&doc.id) && !doc.body.is_empty() {
                            log::debug!(
                                "Indexing new document '{}' into rolegraph for TerraphimGraph search",
                                doc.id
                            );
                            rolegraph.insert_document(&doc.id, doc.clone());

                            // Save document to persistence to ensure it's available for kg_search
                            // Drop the rolegraph lock temporarily to avoid deadlocks during async save
                            drop(rolegraph);
                            if let Err(e) = doc.save().await {
                                log::warn!(
                                    "Failed to save document '{}' to persistence: {}",
                                    doc.id,
                                    e
                                );
                            } else {
                                log::debug!(
                                    "Successfully saved document '{}' to persistence",
                                    doc.id
                                );
                            }
                            // Re-acquire the lock
                            rolegraph = rolegraph_sync.lock().await;

                            newly_indexed += 1;
                        }
                    }

                    if newly_indexed > 0 {
                        log::info!(
                            "✅ Indexed {} new documents into rolegraph for role '{}'",
                            newly_indexed,
                            role.name
                        );
                        log::debug!(
                            "RoleGraph now has {} nodes, {} edges, {} documents",
                            rolegraph.get_node_count(),
                            rolegraph.get_edge_count(),
                            rolegraph.get_document_count()
                        );
                        need_reindexing = true; // We'll use the existing re-search logic below
                    }
                }

                // CRITICAL FIX: Ensure documents have body content loaded from persistence
                // If documents don't have body content, they won't contribute to graph nodes properly
                let mut documents_with_content = Vec::new();

                for mut document in documents {
                    // Check if document body is empty or missing
                    if document.body.is_empty() {
                        log::debug!(
                            "Document '{}' has empty body, attempting to load from persistence",
                            document.id
                        );

                        // Try to load full document from persistence with fallback
                        let mut full_doc = Document::new(document.id.clone());
                        match full_doc.load().await {
                            Ok(loaded_doc) => {
                                if !loaded_doc.body.is_empty() {
                                    log::info!(
                                        "✅ Loaded body content for document '{}' from persistence",
                                        document.id
                                    );
                                    document.body = loaded_doc.body.clone();
                                    if loaded_doc.description.is_some() {
                                        document.description = loaded_doc.description.clone();
                                    }

                                    // Re-index document into rolegraph with proper content
                                    if let Some(rolegraph_sync) =
                                        self.config_state.roles.get(&role.name)
                                    {
                                        let mut rolegraph = rolegraph_sync.lock().await;
                                        rolegraph.insert_document(&document.id, loaded_doc);
                                        need_reindexing = true;
                                        log::debug!(
                                            "Re-indexed document '{}' into rolegraph with content",
                                            document.id
                                        );
                                    }
                                } else {
                                    log::warn!(
                                        "Document '{}' still has empty body after loading from persistence",
                                        document.id
                                    );
                                }
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to load document '{}' from persistence: {}",
                                    document.id,
                                    e
                                );

                                // Try to read from original file path if it's a local file
                                if document.url.starts_with('/')
                                    || document.url.starts_with("docs/")
                                {
                                    match tokio::fs::read_to_string(&document.url).await {
                                        Ok(content) => {
                                            log::info!(
                                                "✅ Loaded content for '{}' from file: {}",
                                                document.id,
                                                document.url
                                            );
                                            document.body = content.clone();

                                            // Create and save full document
                                            let full_doc = Document {
                                                id: document.id.clone(),
                                                title: document.title.clone(),
                                                body: content,
                                                url: document.url.clone(),
                                                description: document.description.clone(),
                                                summarization: document.summarization.clone(),
                                                stub: None,
                                                tags: document.tags.clone(),
                                                rank: document.rank,
                                                source_haystack: document.source_haystack.clone(),
                                                doc_type: terraphim_types::DocumentType::KgEntry,
                                                synonyms: None,
                                                route: None,
                                                priority: None,
                                                quality_score: None,
                                            };

                                            // Save to persistence for future use
                                            if let Err(e) = full_doc.save().await {
                                                log::warn!(
                                                    "Failed to save document '{}' to persistence: {}",
                                                    document.id,
                                                    e
                                                );
                                            }

                                            // Re-index into rolegraph
                                            if let Some(rolegraph_sync) =
                                                self.config_state.roles.get(&role.name)
                                            {
                                                let mut rolegraph = rolegraph_sync.lock().await;
                                                rolegraph.insert_document(&document.id, full_doc);
                                                need_reindexing = true;
                                                log::debug!(
                                                    "Re-indexed document '{}' into rolegraph from file",
                                                    document.id
                                                );
                                            }
                                        }
                                        Err(file_e) => {
                                            log::warn!(
                                                "Failed to read file '{}' for document '{}': {}",
                                                document.url,
                                                document.id,
                                                file_e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                    documents_with_content.push(document);
                }

                documents = documents_with_content;

                if need_reindexing {
                    log::info!("🔄 Re-running TerraphimGraph search after indexing new documents");

                    // Re-run the rolegraph search to get updated rankings
                    let updated_scored_docs: Vec<IndexedDocument> = self
                        .config_state
                        .search_indexed_documents(search_query, &role)
                        .await;

                    if !updated_scored_docs.is_empty() {
                        log::debug!(
                            "✅ Updated rolegraph search found {} documents",
                            updated_scored_docs.len()
                        );
                        // Update documents with new ranking from rolegraph
                        let updated_documents = index.get_documents(updated_scored_docs);
                        if !updated_documents.is_empty() {
                            documents = updated_documents;
                        }
                    }
                }

                if documents.is_empty() && !all_haystack_docs.is_empty() {
                    log::info!(
                        "TerraphimGraph returned no results for role '{}'; falling back to lexical haystack ranking",
                        role.name
                    );
                    documents = if search_query.is_multi_term_query() {
                        let filtered_docs = self
                            .apply_logical_operators_to_documents(
                                search_query,
                                all_haystack_docs.clone(),
                            )
                            .await?;
                        let combined_query_string = search_query
                            .get_all_terms()
                            .iter()
                            .map(|t| t.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let query = Query::new(&combined_query_string);
                        score::sort_documents(&query, filtered_docs)
                    } else {
                        let query = Query::new(&search_query.search_term.to_string());
                        score::sort_documents(&query, all_haystack_docs.clone())
                    };
                }

                // Apply TF-IDF scoring to enhance Terraphim Graph ranking
                if !documents.is_empty() {
                    log::debug!(
                        "Applying TF-IDF scoring to {} documents for enhanced ranking",
                        documents.len()
                    );

                    use crate::score::bm25_additional::TFIDFScorer;
                    let mut tfidf_scorer = TFIDFScorer::new();
                    tfidf_scorer.initialize(&documents);

                    // Re-score documents using TF-IDF
                    let query_text = &search_query.search_term.to_string();
                    for document in &mut documents {
                        let tfidf_score = tfidf_scorer.score(query_text, document);
                        // Combine TF-IDF score with existing rank using a weighted approach
                        if let Some(rank) = document.rank {
                            document.rank = Some(rank + (tfidf_score * 0.3) as u64);
                        // 30% weight for TF-IDF
                        } else {
                            document.rank = Some((tfidf_score * 10.0) as u64); // Scale TF-IDF for ranking
                        }
                    }

                    // Re-sort documents by the new combined rank
                    documents.sort_by_key(|d| std::cmp::Reverse(d.rank.unwrap_or(0)));

                    log::debug!("TF-IDF scoring applied successfully");
                }

                // 🔄 Enhanced persistence layer integration for both local and Atomic Data documents
                for document in &mut documents {
                    if document.id.starts_with("http://") || document.id.starts_with("https://") {
                        // Atomic Data document: Check persistence first, then save for future queries
                        log::debug!(
                            "Processing Atomic Data document '{}' (URL: {})",
                            document.title,
                            document.id
                        );

                        // Try to load from persistence first (for cached Atomic Data documents)
                        let mut placeholder = Document {
                            id: document.id.clone(),
                            ..Default::default()
                        };
                        match placeholder.load().await {
                            Ok(persisted_doc) => {
                                // Found in persistence - use cached version
                                log::debug!(
                                    "Found cached Atomic Data document '{}' in persistence",
                                    document.title
                                );
                                if let Some(better_description) = persisted_doc.description {
                                    document.description = Some(better_description);
                                }
                                // Update body if the persisted version has better content
                                // But DO NOT overwrite if this role uses KG preprocessing (terraphim_it)
                                // because we need to preserve the processed content with KG links
                                if !persisted_doc.body.is_empty() && !role.terraphim_it {
                                    log::debug!(
                                        "Updated body from persistence for Atomic document '{}' (role: '{}', terraphim_it: {})",
                                        document.title,
                                        role.name,
                                        role.terraphim_it
                                    );
                                    document.body = persisted_doc.body;
                                } else if role.terraphim_it {
                                    log::debug!(
                                        "Keeping search result body for Atomic document '{}' because role '{}' uses KG preprocessing (terraphim_it=true)",
                                        document.title,
                                        role.name
                                    );
                                }
                            }
                            Err(_) => {
                                // Not in persistence - save this Atomic Data document for future queries
                                log::debug!(
                                    "Caching Atomic Data document '{}' to persistence for future queries",
                                    document.title
                                );

                                // Save in background to avoid blocking the response
                                let doc_to_save = document.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = doc_to_save.save().await {
                                        log::warn!(
                                            "Failed to cache Atomic Data document '{}': {}",
                                            doc_to_save.title,
                                            e
                                        );
                                    } else {
                                        log::debug!(
                                            "Successfully cached Atomic Data document '{}'",
                                            doc_to_save.title
                                        );
                                    }
                                });
                            }
                        }
                    } else {
                        // Local document: Try direct persistence lookup first
                        let mut placeholder = Document {
                            id: document.id.clone(),
                            ..Default::default()
                        };
                        if let Ok(persisted_doc) = placeholder.load().await {
                            if let Some(better_description) = persisted_doc.description {
                                log::debug!(
                                    "Replaced ripgrep description for '{}' with persistence description",
                                    document.title
                                );
                                document.description = Some(better_description);
                            }
                        } else {
                            // Try normalized ID based on document title (filename)
                            // For KG files, the title might be "haystack" but persistence ID is "haystackmd"
                            let normalized_id = normalize_filename_to_id(&document.title);

                            let mut normalized_placeholder = Document {
                                id: normalized_id.clone(),
                                ..Default::default()
                            };
                            if let Ok(persisted_doc) = normalized_placeholder.load().await {
                                if let Some(better_description) = persisted_doc.description {
                                    log::debug!(
                                        "Replaced ripgrep description for '{}' with persistence description (normalized from title: {})",
                                        document.title,
                                        normalized_id
                                    );
                                    document.description = Some(better_description);
                                }
                            } else {
                                // Try with "md" suffix for KG files (title "haystack" -> ID "haystackmd")
                                let normalized_id_with_md = format!("{}md", normalized_id);
                                let mut md_placeholder = Document {
                                    id: normalized_id_with_md.clone(),
                                    ..Default::default()
                                };
                                if let Ok(persisted_doc) = md_placeholder.load().await {
                                    if let Some(better_description) = persisted_doc.description {
                                        log::debug!(
                                            "Replaced ripgrep description for '{}' with persistence description (normalized with md: {})",
                                            document.title,
                                            normalized_id_with_md
                                        );
                                        document.description = Some(better_description);
                                    }
                                } else {
                                    log::debug!(
                                        "No persistence document found for '{}' (tried ID: '{}', normalized: '{}', with md: '{}')",
                                        document.title,
                                        document.id,
                                        normalized_id,
                                        normalized_id_with_md
                                    );
                                }
                            }
                        }
                    }
                }

                // Apply OpenRouter AI summarization if enabled for this role
                #[cfg(feature = "openrouter")]
                if role.has_llm_config() {
                    log::debug!(
                        "Applying OpenRouter AI summarization to {} search results for role '{}'",
                        documents.len(),
                        role.name
                    );
                    documents = self.enhance_descriptions_with_ai(documents, &role).await?;
                } else {
                    // Always apply LLM AI summarization if LLM client is available
                    log::debug!(
                        "Applying LLM AI summarization to {} search results for role '{}'",
                        documents.len(),
                        role.name
                    );
                    documents = self.enhance_descriptions_with_ai(documents, &role).await?;
                }

                // Apply KG preprocessing if enabled for this role (but only once, not in individual document loads)
                if role.terraphim_it {
                    log::debug!(
                        "Applying KG preprocessing to {} search results for role '{}'",
                        documents.len(),
                        role.name
                    );
                    let mut processed_docs = Vec::new();
                    for document in documents {
                        let processed_doc =
                            self.preprocess_document_content(document, &role).await?;
                        processed_docs.push(processed_doc);
                    }
                    Ok(processed_docs)
                } else {
                    Ok(documents)
                }
            }
        };
        let docs = docs_result?;
        Ok(Self::apply_min_quality_filter(docs, min_quality))
    }

    /// Check if a document ID appears to be hash-based (16 hex characters)
    fn is_hash_based_id(id: &str) -> bool {
        id.len() == 16 && id.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Find documents that contain a given knowledge graph term
    ///
    /// This method searches for documents that were the source of a knowledge graph term.
    /// For example, given "haystack", it will find documents like "haystack.md" that contain
    /// this term or its synonyms ("datasource", "service", "agent").
    ///
    /// For KG protocol resolution, this method also directly looks for KG definition documents
    /// when the term appears to be a KG concept (like "terraphim-graph" -> "./docs/src/kg/terraphim-graph.md").
    ///
    /// Returns a vector of Documents that contain the term, with KG preprocessing applied if enabled for the role.
    pub async fn find_documents_for_kg_term(
        &mut self,
        role_name: &RoleName,
        term: &str,
    ) -> Result<Vec<Document>> {
        log::debug!(
            "Finding documents for KG term '{}' in role '{}'",
            term,
            role_name
        );

        // Ensure the thesaurus is loaded for this role
        let thesaurus = self.ensure_thesaurus_loaded(role_name).await?;

        // Get the role configuration to check if KG preprocessing should be applied
        let role = self.config_state.get_role(role_name).await.ok_or_else(|| {
            ServiceError::Config(format!("Role '{}' not found in config", role_name))
        })?;

        let mut documents = Vec::new();

        // ENHANCEMENT: First, check if this is a direct KG definition document request
        // This handles KG protocol resolution like kg:terraphim-graph -> ./docs/src/kg/terraphim-graph.md
        // Also handles synonyms like kg:graph -> terraphim-graph -> ./docs/src/kg/terraphim-graph.md
        if let Some(kg_config) = &role.kg {
            log::debug!("Found KG config for role");
            if let Some(kg_local) = &kg_config.knowledge_graph_local {
                let mut potential_concepts = vec![term.to_string()];

                // Use the loaded thesaurus to resolve synonyms to root concepts
                log::debug!("Checking thesaurus for term '{}'", term);

                // Create normalized term to look up in thesaurus
                let normalized_search_term =
                    terraphim_types::NormalizedTermValue::new(term.to_string());

                // Look up the term in the thesaurus - this will find the root concept if term is a synonym
                if let Some(root_concept) = thesaurus.get(&normalized_search_term) {
                    log::debug!("Found root concept for '{}': {:?}", term, root_concept);

                    // The root concept's value contains the canonical concept name
                    let root_concept_name = root_concept.value.as_str();

                    // If we have a URL, extract concept name from it, otherwise use the concept value
                    let concept_name = if let Some(url) = &root_concept.url {
                        url.split('/')
                            .next_back()
                            .and_then(|s| s.strip_suffix(".md"))
                            .unwrap_or(root_concept_name)
                    } else {
                        root_concept_name
                    };

                    if !potential_concepts.contains(&concept_name.to_string()) {
                        potential_concepts.push(concept_name.to_string());
                        log::debug!(
                            "Added concept from thesaurus: {} (root: {})",
                            concept_name,
                            root_concept_name
                        );
                    }
                } else {
                    log::debug!("No direct mapping found for '{}' in thesaurus", term);
                }

                log::debug!(
                    "Trying {} potential concepts: {:?}",
                    potential_concepts.len(),
                    potential_concepts
                );

                // Try to find KG definition documents for all potential concepts
                for concept in potential_concepts {
                    let potential_kg_file = kg_local.path.join(format!("{}.md", concept));
                    log::debug!("Looking for KG definition file: {:?}", potential_kg_file);

                    if potential_kg_file.exists() {
                        log::info!("Found KG definition file: {:?}", potential_kg_file);

                        // Check if we already have this document to avoid duplicates
                        let file_path = potential_kg_file.to_string_lossy().to_string();
                        if documents.iter().any(|d: &Document| d.url == file_path) {
                            log::debug!("Skipping duplicate KG document: {}", file_path);
                            continue;
                        }

                        // Load the KG definition document directly from filesystem
                        // Don't use Document::load() as it relies on persistence layer
                        match std::fs::read_to_string(&potential_kg_file) {
                            Ok(content) => {
                                let mut kg_doc =
                                    Document::new(potential_kg_file.to_string_lossy().to_string());
                                kg_doc.url = potential_kg_file.to_string_lossy().to_string();
                                kg_doc.body = content.clone();

                                // Extract title from markdown content (first # line)
                                let title = content
                                    .lines()
                                    .find(|line| line.starts_with("# "))
                                    .map(|line| line.trim_start_matches("# ").trim())
                                    .unwrap_or(&concept)
                                    .to_string();
                                kg_doc.title = title;

                                log::debug!(
                                    "Successfully loaded KG definition document: {}",
                                    kg_doc.title
                                );
                                documents.push(kg_doc);

                                // Found the definition document, no need to check other concepts
                                break;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Failed to read KG definition file '{}': {}",
                                    potential_kg_file.display(),
                                    e
                                );
                            }
                        }
                    } else {
                        log::debug!("KG definition file not found: {:?}", potential_kg_file);
                    }
                }
            } else {
                log::debug!("No KG local config found");
            }
        } else {
            log::debug!("No KG config found for role");
        }

        // Also search through the rolegraph for any documents that contain this term
        let rolegraph_sync = self
            .config_state
            .roles
            .get(role_name)
            .ok_or_else(|| ServiceError::Config(format!("Role '{}' not found", role_name)))?;

        let rolegraph = rolegraph_sync.lock().await;
        let document_ids = rolegraph.find_document_ids_for_term(term);
        drop(rolegraph); // Release the lock early

        log::debug!(
            "Found {} document IDs from rolegraph for term '{}'",
            document_ids.len(),
            term
        );

        // Load documents found in the rolegraph (if any)
        for doc_id in &document_ids {
            // Skip if we already have this document from the KG definition lookup
            if documents
                .iter()
                .any(|d| d.id == *doc_id || d.url == *doc_id)
            {
                log::debug!("Skipping duplicate document from rolegraph: {}", doc_id);
                continue;
            }

            // Load the actual documents using the persistence layer
            // Handle both local and Atomic Data documents properly
            if doc_id.starts_with("http://") || doc_id.starts_with("https://") {
                // Atomic Data document: Try to load from persistence first
                log::debug!("Loading Atomic Data document '{}' from persistence", doc_id);
                let mut placeholder = Document {
                    id: doc_id.clone(),
                    ..Default::default()
                };
                match placeholder.load().await {
                    Ok(loaded_doc) => {
                        log::debug!(
                            "Found cached Atomic Data document '{}' in persistence",
                            doc_id
                        );
                        documents.push(loaded_doc);
                    }
                    Err(_) => {
                        log::warn!(
                            "Atomic Data document '{}' not found in persistence - this may indicate the document hasn't been cached yet",
                            doc_id
                        );
                        // Skip this document for now - it will be cached when accessed through search
                        // In a production system, you might want to fetch it from the Atomic Server here
                    }
                }
            } else {
                // Local document: Use the standard persistence loading
                let mut doc = Document::new(doc_id.clone());
                match doc.load().await {
                    Ok(loaded_doc) => {
                        documents.push(loaded_doc);
                        log::trace!("Successfully loaded local document: {}", doc_id);
                    }
                    Err(e) => {
                        log::warn!("Failed to load local document '{}': {}", doc_id, e);

                        // Check if this might be a hash-based ID from old ripgrep documents
                        if Self::is_hash_based_id(doc_id) {
                            log::debug!(
                                "Document ID '{}' appears to be hash-based (legacy document), skipping for now",
                                doc_id
                            );
                            log::info!(
                                "💡 Hash-based document IDs are deprecated. This document will be re-indexed with normalized IDs on next haystack search."
                            );
                            // Skip legacy hash-based documents - they will be re-indexed with proper normalized IDs
                            // when the haystack is searched again
                        }

                        // Continue processing other documents even if this one fails
                    }
                }
            }
        }

        // Apply KG preprocessing if enabled for this role
        if role.terraphim_it {
            log::info!(
                "🧠 Applying KG preprocessing to {} KG term documents for role '{}' (terraphim_it enabled)",
                documents.len(),
                role_name
            );
            let mut processed_documents = Vec::new();
            let mut total_kg_terms = 0;
            let mut docs_with_kg_links = 0;

            for document in documents {
                let original_body_len = document.body.len();
                let processed_doc = self.preprocess_document_content(document, &role).await?;

                // Count KG links added (rough estimate by body size increase)
                let new_body_len = processed_doc.body.len();
                if new_body_len > original_body_len {
                    docs_with_kg_links += 1;
                    let estimated_links = (new_body_len - original_body_len) / 17;
                    total_kg_terms += estimated_links;
                }

                processed_documents.push(processed_doc);
            }

            log::info!(
                "✅ KG preprocessing complete: {} documents processed, {} received KG links (~{} total links)",
                processed_documents.len(),
                docs_with_kg_links,
                total_kg_terms
            );
            documents = processed_documents;
        } else {
            log::info!(
                "🔍 terraphim_it disabled for role '{}', skipping KG preprocessing for {} documents",
                role_name,
                documents.len()
            );
        }

        // Assign ranks based on order (same logic as regular search)
        // Higher rank for earlier results to maintain consistency
        let total_length = documents.len();
        for (idx, doc) in documents.iter_mut().enumerate() {
            let rank = (total_length - idx) as u64;
            doc.rank = Some(rank);
            log::trace!("Assigned rank {} to document '{}'", rank, doc.title);
        }

        log::debug!(
            "Successfully loaded and processed {} documents for term '{}', ranks assigned from {} to 1",
            documents.len(),
            term,
            total_length
        );
        Ok(documents)
    }
}
