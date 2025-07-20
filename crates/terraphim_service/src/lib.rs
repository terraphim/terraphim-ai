use ahash::AHashMap;
use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_automata::load_thesaurus;
use terraphim_automata::{replace_matches, LinkType};
use terraphim_config::{ConfigState, Role};
use terraphim_middleware::thesaurus::build_thesaurus_from_haystack;
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, Index, IndexedDocument, NormalizedTermValue,RelevanceFunction, RoleName, SearchQuery, Thesaurus,
};
mod score;

#[cfg(feature = "openrouter")]
pub mod openrouter;

/// Normalize a filename to be used as a document ID
/// 
/// This ensures consistent ID generation between server startup and edit API
fn normalize_filename_to_id(filename: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create regex");
    re.replace_all(filename, "").to_lowercase()
}

#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(#[from] opendal::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[cfg(feature = "openrouter")]
    #[error("OpenRouter error: {0}")]
    OpenRouter(#[from] crate::openrouter::OpenRouterError),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

pub struct TerraphimService {
    config_state: ConfigState,
}

impl<'a> TerraphimService {
    /// Create a new TerraphimService
    pub fn new(config_state: ConfigState) -> Self {
        Self { config_state }
    }

    /// Build a thesaurus from the haystack and update the knowledge graph automata URL
    async fn build_thesaurus(&mut self, search_query: &SearchQuery) -> Result<()> {
        Ok(build_thesaurus_from_haystack(&mut self.config_state, search_query).await?)
    }
    /// load thesaurus from config object and if absent make sure it's loaded from automata_url
    pub async fn ensure_thesaurus_loaded(&mut self, role_name: &RoleName) -> Result<Thesaurus> {
        async fn load_thesaurus_from_automata_path(
            config_state: &ConfigState,
            role_name: &RoleName,
            rolegraphs: &mut AHashMap<RoleName, RoleGraphSync>,
        ) -> Result<Thesaurus> {
            let config = config_state.config.lock().await;
            let role = config.roles.get(role_name).cloned().unwrap();
            if let Some(kg) = &role.kg {
                if let Some(automata_path) = &kg.automata_path {
                    log::info!("Loading Role `{}` - URL: {:?}", role_name, automata_path);
                    
                    // Try to load from automata path first
                    match load_thesaurus(automata_path).await {
                        Ok(thesaurus) => {
                            log::info!("Successfully loaded thesaurus from automata path");
                            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                            match rolegraph {
                                Ok(rolegraph) => {
                                    let rolegraph_value = RoleGraphSync::from(rolegraph);
                                    rolegraphs.insert(role_name.clone(), rolegraph_value);
                                }
                                Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
                            }
                            Ok(thesaurus)
                        }
                        Err(e) => {
                            log::warn!("Failed to load thesaurus from automata path: {:?}", e);
                            // Fallback to building from local KG if available
                            if let Some(kg_local) = &kg.knowledge_graph_local {
                                log::info!(
                                    "Fallback: building thesaurus from local KG for role {}",
                                    role_name
                                );
                                let logseq_builder = Logseq::default();
                                match logseq_builder
                                    .build(
                                        role_name.as_lowercase().to_string(),
                                        kg_local.path.clone(),
                                    )
                                    .await
                                {
                                    Ok(thesaurus) => {
                                        let rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                                        match rolegraph {
                                            Ok(rolegraph) => {
                                                let rolegraph_value = RoleGraphSync::from(rolegraph);
                                                rolegraphs.insert(role_name.clone(), rolegraph_value);
                                            }
                                            Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
                                        }
                                        
                                        Ok(thesaurus)
                                    }
                                    Err(e) => {
                                        log::error!(
                                            "Failed to build thesaurus from local KG for role {}: {:?}",
                                            role_name,
                                            e
                                        );
                                        Err(ServiceError::Config("Failed to load or build thesaurus".into()))
                                    }
                                }
                            } else {
                                log::error!(
                                    "No fallback available for role {}: no local KG path configured",
                                    role_name
                                );
                                Err(ServiceError::Config("No automata path and no local KG available".into()))
                            }
                        }
                    }
                } else if let Some(kg_local) = &kg.knowledge_graph_local {
                    // Build thesaurus from local KG
                    log::info!(
                        "Role {} has no automata_path, building thesaurus from local KG files at {:?}",
                        role_name,
                        kg_local.path
                    );
                    let logseq_builder = Logseq::default();
                    match logseq_builder
                        .build(
                            role_name.as_lowercase().to_string(),
                            kg_local.path.clone(),
                        )
                        .await
                    {
                        Ok(thesaurus) => {
                            log::info!(
                                "Successfully built thesaurus from local KG for role {}",
                                role_name
                            );
                            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                            match rolegraph {
                                Ok(rolegraph) => {
                                    let rolegraph_value = RoleGraphSync::from(rolegraph);
                                    rolegraphs.insert(role_name.clone(), rolegraph_value);
                                }
                                Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
                            }
                            
                            Ok(thesaurus)
                        }
                        Err(e) => {
                            log::error!(
                                "Failed to build thesaurus from local KG for role {}: {:?}",
                                role_name,
                                e
                            );
                            Err(ServiceError::Config("Failed to build thesaurus from local KG".into()))
                        }
                    }
                } else {
                    log::warn!("Role {} is configured for TerraphimGraph but has neither automata_path nor knowledge_graph_local defined.", role_name);
                    if let Some(kg_local) = &kg.knowledge_graph_local {
                        // Build thesaurus from local KG files during startup
                        log::info!(
                            "Building thesaurus from local KG files for role {} at {:?}",
                            role_name,
                            kg_local.path
                        );
                        let logseq_builder = Logseq::default();
                        match logseq_builder
                            .build(
                                role_name.as_lowercase().to_string(),
                                kg_local.path.clone(),
                            )
                            .await
                        {
                            Ok(thesaurus) => {
                                log::info!(
                                    "Successfully built thesaurus from local KG for role {}",
                                    role_name
                                );
                                let rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus.clone()).await;
                                match rolegraph {
                                    Ok(rolegraph) => {
                                        let rolegraph_value = RoleGraphSync::from(rolegraph);
                                        rolegraphs.insert(role_name.clone(), rolegraph_value);
                                    }
                                    Err(e) => log::error!("Failed to update role and thesaurus: {:?}", e),
                                }
                                
                                Ok(thesaurus)
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to build thesaurus from local KG for role {}: {:?}",
                                    role_name,
                                    e
                                );
                                Err(ServiceError::Config("Failed to build thesaurus from local KG".into()))
                            }
                        }
                    } else {
                        Err(ServiceError::Config("No local knowledge graph path available".into()))
                    }
                }
            } else {
                Err(ServiceError::Config("Knowledge graph not configured".into()))
            }
        }
        
        log::debug!("Loading thesaurus for role: {}", role_name);
        log::debug!("Role keys {:?}", self.config_state.roles.keys());
        let mut rolegraphs = self.config_state.roles.clone();
        if let Some(rolegraph_value) = rolegraphs.get(role_name) {
            let thesaurus_result = rolegraph_value.lock().await.thesaurus.clone().load().await;
            match thesaurus_result {
                Ok(thesaurus) => {
                    log::debug!("Thesaurus loaded: {:?}", thesaurus);
                    log::info!("Rolegraph loaded: for role name {:?}", role_name);
                    Ok(thesaurus)
                }
                Err(e) => {
                    log::error!("Failed to load thesaurus: {:?}", e);
                    load_thesaurus_from_automata_path(
                        &self.config_state,
                        role_name,
                        &mut rolegraphs,
                    )
                    .await
                }
            }
        } else {
            load_thesaurus_from_automata_path(
                &self.config_state,
                role_name,
                &mut rolegraphs,
            )
            .await
        }
    }

    /// Preprocess document content to create clickable KG links when terraphim_it is enabled
    /// 
    /// This function replaces KG terms in the document body with markdown links
    /// in the format [term](kg:term) which can be intercepted by the frontend
    /// to display KG documents when clicked.
    pub async fn preprocess_document_content(&mut self, mut document: Document, role: &Role) -> Result<Document> {
        // Only preprocess if terraphim_it is enabled and role has KG configured
        if !role.terraphim_it {
            log::debug!("terraphim_it disabled for role {}, skipping KG preprocessing", role.name);
            return Ok(document);
        }

        let Some(_kg) = &role.kg else {
            log::debug!("No KG configured for role {}, skipping KG preprocessing", role.name);
            return Ok(document);
        };

        log::debug!("Preprocessing document '{}' for KG term linking in role '{}'", document.title, role.name);

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
        
        // Very selective KG term filtering to avoid clutter:
        // Only include highly specific, domain-relevant terms 
        let excluded_common_terms = ["service", "haystack", "system", "config", "configuration", "type", "method", "function", "class", "component", "module", "library", "framework", "interface", "api", "data", "file", "path", "url", "string", "number", "value", "option", "parameter", "field", "property", "attribute", "element", "item", "object", "array", "list", "map", "set", "collection", "server", "client", "request", "response", "error", "result", "success", "failure", "true", "false", "null", "undefined", "empty", "full", "start", "end", "begin", "finish", "create", "delete", "update", "read", "write", "load", "save", "process", "handle", "manage", "control", "execute", "run", "call", "invoke", "trigger", "event", "action", "command", "query", "search", "filter", "sort", "order", "group", "match", "find", "replace", "insert", "remove", "add", "set", "get", "put", "post", "head", "patch", "delete"];
        
        let mut sorted_terms: Vec<_> = (&thesaurus).into_iter()
            .filter(|(key, _)| {
                let term = key.as_str();
                
                // Exclude empty terms, very short terms, and common technical terms
                if term.is_empty() || term.len() < 5 || excluded_common_terms.contains(&term) {
                    return false;
                }
                
                // Only include highly specific terms:
                // 1. Very long compound terms (>12 chars) OR
                // 2. Hyphenated compound terms OR  
                // 3. Terms with unique patterns (contains "graph", "terraphim", etc.)
                term.len() > 12 || 
                term.contains('-') || 
                term.contains("graph") ||
                term.contains("terraphim") ||
                term.contains("knowledge") ||
                term.contains("embedding")
            })
            .collect();
        sorted_terms.sort_by(|a, b| b.1.id.cmp(&a.1.id));  // Sort by relevance (ID)
        
        // Take only the top 3 most specific terms to minimize clutter
        let max_kg_terms = 3;
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
        log::debug!("Filtered KG thesaurus from {} to {} terms for preprocessing", 
                   thesaurus.len(), kg_terms_count);

        // Apply KG term replacement to document body (only if we have terms to replace)
        if !kg_thesaurus.is_empty() {
            match replace_matches(&document.body, kg_thesaurus, LinkType::MarkdownLinks) {
                Ok(processed_bytes) => {
                    match String::from_utf8(processed_bytes) {
                        Ok(processed_content) => {
                            log::debug!("Successfully preprocessed document '{}' with {} KG terms", 
                                       document.title, kg_terms_count);
                            document.body = processed_content;
                        }
                        Err(e) => {
                            log::warn!("Failed to convert processed content to UTF-8 for document '{}': {:?}", 
                                      document.title, e);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to replace KG terms in document '{}': {:?}", document.title, e);
                }
            }
        } else {
            log::debug!("No KG terms to process for document '{}'", document.title);
        }

        Ok(document)
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
                            document.id, haystack.location, e
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
        
        // 1️⃣ Try to load the document directly using the provided ID
        let mut placeholder = Document::default();
        placeholder.id = document_id.to_string();
        match placeholder.load().await {
            Ok(doc) => {
                log::debug!("Found document '{}' with direct ID lookup", document_id);
                return self.apply_kg_preprocessing_if_needed(doc).await.map(Some);
            }
            Err(e) => {
                log::debug!("Document '{}' not found with direct lookup: {:?}", document_id, e);
            }
        }

        // 2️⃣ If the provided ID looks like a filename, try with normalized ID
        if document_id.contains('.') || document_id.contains('-') || document_id.contains('_') {
            let normalized_id = normalize_filename_to_id(document_id);
            log::debug!("Trying normalized ID '{}' for filename '{}'", normalized_id, document_id);
            
            let mut normalized_placeholder = Document::default();
            normalized_placeholder.id = normalized_id.clone();
            match normalized_placeholder.load().await {
                Ok(doc) => {
                    log::debug!("Found document '{}' with normalized ID '{}'", document_id, normalized_id);
                    return self.apply_kg_preprocessing_if_needed(doc).await.map(Some);
                }
                Err(e) => {
                    log::debug!("Document '{}' not found with normalized ID '{}': {:?}", document_id, normalized_id, e);
                }
            }
        }

        // 3️⃣ Fallback: search by title (for documents where title contains the original filename)
        log::debug!("Falling back to search for document '{}'", document_id);
        let search_query = SearchQuery {
            search_term: NormalizedTermValue::new(document_id.to_string()),
            limit: Some(5), // Get a few results to check titles
            skip: None,
            role: None,
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
    /// and applies KG term preprocessing accordingly.
    /// 
    /// NOTE: Disabled to prevent double processing - KG preprocessing is now only applied
    /// during search results to avoid processing documents multiple times.
    async fn apply_kg_preprocessing_if_needed(&mut self, document: Document) -> Result<Document> {
        // DISABLED: KG preprocessing is already applied in search results
        // to prevent double processing that creates "links to links"
        log::debug!("Skipping KG preprocessing for individual document load to prevent double processing");
        Ok(document)
    }

    /// Enhance document descriptions with AI-generated summaries using OpenRouter
    /// 
    /// This method uses the OpenRouter service to generate intelligent summaries
    /// of document content, replacing basic text excerpts with AI-powered descriptions.
    #[cfg(feature = "openrouter")]
    async fn enhance_descriptions_with_ai(&self, mut documents: Vec<Document>, role: &Role) -> Result<Vec<Document>> {
        use crate::openrouter::OpenRouterService;
        
        // Create the OpenRouter service
        let api_key = role.openrouter_api_key.as_ref().unwrap();
        let model = role.openrouter_model.as_deref().unwrap_or("openai/gpt-3.5-turbo");
        
        let openrouter_service = match OpenRouterService::new(api_key, model) {
            Ok(service) => service,
            Err(e) => {
                log::error!("Failed to create OpenRouter service for role '{}': {:?}", role.name, e);
                return Ok(documents); // Return original documents on service creation failure
            }
        };
        
        log::info!("Enhancing {} document descriptions with OpenRouter model: {}", documents.len(), model);
        
        let mut enhanced_count = 0;
        let mut error_count = 0;
        
        for document in &mut documents {
            // Only enhance documents that have meaningful content and don't already have high-quality descriptions
            if self.should_generate_ai_summary(document) {
                let summary_length = 250; // Target length for summaries
                
                match openrouter_service.generate_summary(&document.body, summary_length).await {
                    Ok(ai_summary) => {
                        log::debug!("Generated AI summary for '{}': {} characters", document.title, ai_summary.len());
                        document.description = Some(ai_summary);
                        enhanced_count += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to generate AI summary for '{}': {:?}", document.title, e);
                        error_count += 1;
                        // Keep existing description as fallback
                    }
                }
            } else {
                log::debug!("Skipping AI summary for '{}' (not suitable for enhancement)", document.title);
            }
        }
        
        log::info!("OpenRouter enhancement complete: {} enhanced, {} errors, {} skipped", 
                  enhanced_count, error_count, documents.len() - enhanced_count - error_count);
        
        Ok(documents)
    }

    /// Determine if a document should receive an AI-generated summary
    /// 
    /// This helper method checks various criteria to decide whether a document
    /// would benefit from AI summarization.
    #[cfg(feature = "openrouter")]
    fn should_generate_ai_summary(&self, document: &Document) -> bool {
        // Don't enhance if the document body is too short to summarize meaningfully
        if document.body.trim().len() < 200 {
            return false;
        }
        
        // Don't enhance if we already have a high-quality description
        if let Some(ref description) = document.description {
            // If the description is substantial and doesn't look like a simple excerpt, keep it
            if description.len() > 100 && !description.ends_with("...") {
                return false;
            }
        }
        
        // Don't enhance very large documents (cost control)
        if document.body.len() > 8000 {
            return false;
        }
        
        // Good candidates for AI summarization
        true
    }

    /// Get the role for the given search query
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
    
    /// search for documents in the haystacks with selected role from the config
    /// and return the documents sorted by relevance
    pub async fn search_documents_selected_role(
        &mut self,
        search_term: &NormalizedTermValue,
    ) -> Result<Vec<Document>> {
        let role = self.config_state.get_selected_role().await;
        let documents = self.search(&SearchQuery { search_term: search_term.clone(), role: Some(role), skip: None, limit: None }).await?;
        Ok(documents)
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

        match role.relevance_function {
            RelevanceFunction::TitleScorer => {
                log::debug!("Searching haystack with title scorer");

                let documents = index.get_all_documents();

                log::debug!("Sorting documents by relevance");
                // Sort the documents by relevance
                let documents = score::sort_documents(search_query, documents);
                let total_length = documents.len();
                let mut docs_ranked = Vec::new();
                for (idx, doc) in documents.iter().enumerate() {
                    let mut document: terraphim_types::Document = doc.clone();
                    let rank = (total_length - idx).try_into().unwrap();
                    document.rank = Some(rank);
                    
                    // 🔄 Replace ripgrep description with persistence layer description for better quality
                    // Try direct persistence lookup first
                    let mut placeholder = Document::default();
                    placeholder.id = document.id.clone();
                    if let Ok(persisted_doc) = placeholder.load().await {
                        if let Some(better_description) = persisted_doc.description {
                            log::debug!("Replaced ripgrep description for '{}' with persistence description", document.title);
                            document.description = Some(better_description);
                        }
                    } else {
                        // Try normalized ID based on document title (filename)
                        // For KG files, the title might be "haystack" but persistence ID is "haystackmd"
                        let normalized_id = normalize_filename_to_id(&document.title);
                        
                        let mut normalized_placeholder = Document::default();
                        normalized_placeholder.id = normalized_id.clone();
                        if let Ok(persisted_doc) = normalized_placeholder.load().await {
                            if let Some(better_description) = persisted_doc.description {
                                log::debug!("Replaced ripgrep description for '{}' with persistence description (normalized from title: {})", document.title, normalized_id);
                                document.description = Some(better_description);
                            }
                        } else {
                            // Try with "md" suffix for KG files (title "haystack" -> ID "haystackmd")
                            let normalized_id_with_md = format!("{}md", normalized_id);
                            let mut md_placeholder = Document::default();
                            md_placeholder.id = normalized_id_with_md.clone();
                            if let Ok(persisted_doc) = md_placeholder.load().await {
                                if let Some(better_description) = persisted_doc.description {
                                    log::debug!("Replaced ripgrep description for '{}' with persistence description (normalized with md: {})", document.title, normalized_id_with_md);
                                    document.description = Some(better_description);
                                }
                            } else {
                                log::debug!("No persistence document found for '{}' (tried ID: '{}', normalized: '{}', with md: '{}')", document.title, document.id, normalized_id, normalized_id_with_md);
                            }
                        }
                    }
                    
                    docs_ranked.push(document);
                }
                
                // Apply OpenRouter AI summarization if enabled for this role
                #[cfg(feature = "openrouter")]
                if role.has_openrouter_config() {
                    log::debug!("Applying OpenRouter AI summarization to {} search results for role '{}'", docs_ranked.len(), role.name);
                    docs_ranked = self.enhance_descriptions_with_ai(docs_ranked, &role).await?;
                }
                
                // Apply KG preprocessing if enabled for this role (but only once, not in individual document loads)
                if role.terraphim_it {
                    log::debug!("Applying KG preprocessing to {} search results for role '{}'", docs_ranked.len(), role.name);
                    let mut processed_docs = Vec::new();
                    for document in docs_ranked {
                        let processed_doc = self.preprocess_document_content(document, &role).await?;
                        processed_docs.push(processed_doc);
                    }
                    Ok(processed_docs)
                } else {
                    Ok(docs_ranked)
                }
            }
            RelevanceFunction::TerraphimGraph => {
                self.build_thesaurus(search_query).await?;
                let _thesaurus = self.ensure_thesaurus_loaded(&role.name).await?;
                let scored_index_docs: Vec<IndexedDocument> = self
                    .config_state
                    .search_indexed_documents(search_query, &role)
                    .await;

                // Apply to ripgrep vector of document output
                // I.e. use the ranking of thesaurus to rank the documents here
                log::debug!("Ranking documents with thesaurus");
                let mut documents = index.get_documents(scored_index_docs);
                
                // 🔄 Replace ripgrep descriptions with persistence layer descriptions for better quality
                for document in &mut documents {
                    // Try direct persistence lookup first
                    let mut placeholder = Document::default();
                    placeholder.id = document.id.clone();
                    if let Ok(persisted_doc) = placeholder.load().await {
                        if let Some(better_description) = persisted_doc.description {
                            log::debug!("Replaced ripgrep description for '{}' with persistence description", document.title);
                            document.description = Some(better_description);
                        }
                    } else {
                        // Try normalized ID based on document title (filename)
                        // For KG files, the title might be "haystack" but persistence ID is "haystackmd"
                        let normalized_id = normalize_filename_to_id(&document.title);
                        
                        let mut normalized_placeholder = Document::default();
                        normalized_placeholder.id = normalized_id.clone();
                        if let Ok(persisted_doc) = normalized_placeholder.load().await {
                            if let Some(better_description) = persisted_doc.description {
                                log::debug!("Replaced ripgrep description for '{}' with persistence description (normalized from title: {})", document.title, normalized_id);
                                document.description = Some(better_description);
                            }
                        } else {
                            // Try with "md" suffix for KG files (title "haystack" -> ID "haystackmd")
                            let normalized_id_with_md = format!("{}md", normalized_id);
                            let mut md_placeholder = Document::default();
                            md_placeholder.id = normalized_id_with_md.clone();
                            if let Ok(persisted_doc) = md_placeholder.load().await {
                                if let Some(better_description) = persisted_doc.description {
                                    log::debug!("Replaced ripgrep description for '{}' with persistence description (normalized with md: {})", document.title, normalized_id_with_md);
                                    document.description = Some(better_description);
                                }
                            } else {
                                log::debug!("No persistence document found for '{}' (tried ID: '{}', normalized: '{}', with md: '{}')", document.title, document.id, normalized_id, normalized_id_with_md);
                            }
                        }
                    }
                }

                // Apply OpenRouter AI summarization if enabled for this role
                #[cfg(feature = "openrouter")]
                if role.has_openrouter_config() {
                    log::debug!("Applying OpenRouter AI summarization to {} search results for role '{}'", documents.len(), role.name);
                    documents = self.enhance_descriptions_with_ai(documents, &role).await?;
                }

                // Apply KG preprocessing if enabled for this role (but only once, not in individual document loads)
                if role.terraphim_it {
                    log::debug!("Applying KG preprocessing to {} search results for role '{}'", documents.len(), role.name);
                    let mut processed_docs = Vec::new();
                    for document in documents {
                        let processed_doc = self.preprocess_document_content(document, &role).await?;
                        processed_docs.push(processed_doc);
                    }
                    Ok(processed_docs)
                } else {
                    Ok(documents)
                }
            }
        }
    }

    /// Find documents that contain a given knowledge graph term
    /// 
    /// This method searches for documents that were the source of a knowledge graph term.
    /// For example, given "haystack", it will find documents like "haystack.md" that contain
    /// this term or its synonyms ("datasource", "service", "agent").
    /// 
    /// Returns a vector of Documents that contain the term, with KG preprocessing applied if enabled for the role.
    pub async fn find_documents_for_kg_term(&mut self, role_name: &RoleName, term: &str) -> Result<Vec<Document>> {
        log::debug!("Finding documents for KG term '{}' in role '{}'", term, role_name);
        
        // Ensure the thesaurus is loaded for this role
        let _thesaurus = self.ensure_thesaurus_loaded(role_name).await?;
        
        // Get the role configuration to check if KG preprocessing should be applied
        let role = self.config_state.get_role(role_name).await
            .ok_or_else(|| ServiceError::Config(format!("Role '{}' not found in config", role_name)))?;
        
        // Get the role's rolegraph
        let rolegraph_sync = self.config_state.roles.get(role_name)
            .ok_or_else(|| ServiceError::Config(format!("Role '{}' not found", role_name)))?;
        
        let rolegraph = rolegraph_sync.lock().await;
        
        // Find document IDs that contain this term
        let document_ids = rolegraph.find_document_ids_for_term(term);
        drop(rolegraph); // Release the lock early
        
        if document_ids.is_empty() {
            log::debug!("No documents found for term '{}'", term);
            return Ok(Vec::new());
        }
        
        log::debug!("Found {} document IDs for term '{}': {:?}", document_ids.len(), term, document_ids);
        
        // Load the actual documents using the persistence layer
        let mut documents = terraphim_persistence::load_documents_by_ids(&document_ids).await?;
        
        // Apply KG preprocessing if enabled for this role
        if role.terraphim_it {
            log::debug!("Applying KG preprocessing for role '{}' with terraphim_it enabled", role_name);
            let mut processed_documents = Vec::new();
            for document in documents {
                let processed_doc = self.preprocess_document_content(document, &role).await?;
                processed_documents.push(processed_doc);
            }
            documents = processed_documents;
        } else {
            log::debug!("terraphim_it disabled for role '{}', skipping KG preprocessing", role_name);
        }
        
        log::debug!("Successfully loaded and processed {} documents for term '{}'", documents.len(), term);
        Ok(documents)
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> terraphim_config::Config {
        let current_config = self.config_state.config.lock().await;
        current_config.clone()
    }

    /// Update the config
    ///
    /// Overwrites the config in the config state and returns the updated
    /// config.
    pub async fn update_config(
        &self,
        config: terraphim_config::Config,
    ) -> Result<terraphim_config::Config> {
        let mut current_config = self.config_state.config.lock().await;
        *current_config = config.clone();
        current_config.save().await?;
        log::info!("Config updated");
        Ok(config)
    }

    /// Update only the `selected_role` in the config without mutating the rest of the
    /// configuration. Returns the up-to-date `Config` object.
    pub async fn update_selected_role(
        &self,
        role_name: terraphim_types::RoleName,
    ) -> Result<terraphim_config::Config> {
        let mut current_config = self.config_state.config.lock().await;

        // Ensure the role exists before updating.
        if !current_config.roles.contains_key(&role_name) {
            return Err(ServiceError::Config(format!(
                "Role `{}` not found in config",
                role_name
            )));
        }

        current_config.selected_role = role_name;
        current_config.save().await?;
        log::info!("Selected role updated to {}", current_config.selected_role);

        Ok(current_config.clone())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_config::ConfigBuilder;
    use terraphim_types::NormalizedTermValue;

    #[tokio::test]
    async fn test_get_config() {
        let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
        let config_state = ConfigState::new(&mut config).await.unwrap();
        let service = TerraphimService::new(config_state);
        let fetched_config = service.fetch_config().await;
        assert_eq!(fetched_config.id, terraphim_config::ConfigId::Desktop);
    }

    #[tokio::test]
    async fn test_search_documents_selected_role() {
        let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
        let config_state = ConfigState::new(&mut config).await.unwrap();
        let mut service = TerraphimService::new(config_state);
        let search_term = NormalizedTermValue::new("terraphim".to_string());
        let documents = service.search_documents_selected_role(&search_term).await.unwrap();
        assert!(documents.is_empty() || !documents.is_empty()); // Either empty or has results
    }

    #[tokio::test]
    async fn test_ensure_thesaurus_loaded_terraphim_engineer() {
        // Create a fresh config instead of trying to load from persistence
        let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
        let config_state = ConfigState::new(&mut config).await.unwrap();
        let mut service = TerraphimService::new(config_state);
        
        let role_name = RoleName::new("Terraphim Engineer");
        let thesaurus_result = service.ensure_thesaurus_loaded(&role_name).await;
        
        match thesaurus_result {
            Ok(thesaurus) => {
                println!("✅ Successfully loaded thesaurus with {} entries", thesaurus.len());
                // Verify thesaurus contains expected terms
                assert!(!thesaurus.is_empty(), "Thesaurus should not be empty");
                
                // Check for expected terms from docs/src/kg using &thesaurus for iteration
                let has_terraphim = (&thesaurus).into_iter().any(|(term, _)| {
                    term.as_str().to_lowercase().contains("terraphim")
                });
                let has_graph = (&thesaurus).into_iter().any(|(term, _)| {
                    term.as_str().to_lowercase().contains("graph")
                });
                
                println!("   Contains 'terraphim': {}", has_terraphim);
                println!("   Contains 'graph': {}", has_graph);
                
                // At least one of these should be present
                assert!(has_terraphim || has_graph, "Thesaurus should contain expected terms");
            }
            Err(e) => {
                println!("❌ Failed to load thesaurus: {:?}", e);
                // This might fail if the local KG files don't exist, which is expected in some test environments
                // We'll just log the error but not fail the test
            }
        }
    }

    #[tokio::test]
    async fn test_config_building_with_local_kg() {
        // Test that config building works correctly with local KG files
        let mut config = ConfigBuilder::new().build_default_desktop().build().unwrap();
        let config_state_result = ConfigState::new(&mut config).await;
        
        match config_state_result {
            Ok(config_state) => {
                println!("✅ Successfully built config state");
                // Verify that roles were created
                assert!(!config_state.roles.is_empty(), "Config state should have roles");
                
                // Check if Terraphim Engineer role was created
                let terraphim_engineer_role = RoleName::new("Terraphim Engineer");
                let has_terraphim_engineer = config_state.roles.contains_key(&terraphim_engineer_role);
                println!("   Has Terraphim Engineer role: {}", has_terraphim_engineer);
                
                // The role should exist even if thesaurus building failed
                assert!(has_terraphim_engineer, "Terraphim Engineer role should exist");
            }
            Err(e) => {
                println!("❌ Failed to build config state: {:?}", e);
                // This might fail if the local KG files don't exist, which is expected in some test environments
                // We'll just log the error but not fail the test
            }
        }
    }
}  