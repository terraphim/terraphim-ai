use terraphim_service::TerraphimService;
use terraphim_types::{Document, NormalizedTermValue, SearchQuery, RoleName};
use terraphim_automata::{AutocompleteIndex, AutocompleteConfig, AutocompleteResult, build_autocomplete_index, autocomplete_search};
use terraphim_config::ConfigState as CoreConfigState;
use anyhow::Result;

/// Search service wrapper for Dioxus frontend
pub struct SearchService {
    backend_service: TerraphimService,
    autocomplete_index: Option<AutocompleteIndex>,
}

impl SearchService {
    /// Create a new search service
    pub fn new(config_state: CoreConfigState) -> Self {
        Self {
            backend_service: TerraphimService::new(config_state),
            autocomplete_index: None,
        }
    }

    /// Initialize autocomplete index from current role's thesaurus
    pub async fn initialize_autocomplete(&mut self, role_name: &RoleName) -> Result<()> {
        tracing::info!("Initializing autocomplete for role: {}", role_name);

        // Load thesaurus for the role
        match self.backend_service.ensure_thesaurus_loaded(role_name).await {
            Ok(thesaurus) => {
                tracing::info!("Loaded thesaurus with {} entries", thesaurus.len());

                // Build autocomplete index
                let config = AutocompleteConfig {
                    max_results: 10,
                    min_prefix_length: 2,
                    case_sensitive: false,
                };

                match build_autocomplete_index(thesaurus, Some(config)) {
                    Ok(index) => {
                        tracing::info!("Built autocomplete index with {} terms", index.len());
                        self.autocomplete_index = Some(index);
                        Ok(())
                    }
                    Err(e) => {
                        tracing::error!("Failed to build autocomplete index: {:?}", e);
                        Err(anyhow::anyhow!("Failed to build autocomplete index: {:?}", e))
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to load thesaurus: {:?}", e);
                Err(anyhow::anyhow!("Failed to load thesaurus: {:?}", e))
            }
        }
    }

    /// Get autocomplete suggestions for a prefix
    pub fn autocomplete(&self, prefix: &str) -> Vec<AutocompleteResult> {
        if let Some(index) = &self.autocomplete_index {
            match autocomplete_search(index, prefix, None) {
                Ok(results) => results,
                Err(e) => {
                    tracing::error!("Autocomplete error: {:?}", e);
                    Vec::new()
                }
            }
        } else {
            tracing::warn!("Autocomplete index not initialized");
            Vec::new()
        }
    }

    /// Search for documents using the selected role
    pub async fn search(&mut self, search_term: &str) -> Result<Vec<Document>> {
        tracing::info!("Searching for: {}", search_term);

        let normalized_term = NormalizedTermValue::from(search_term);

        match self.backend_service.search_documents_selected_role(&normalized_term).await {
            Ok(documents) => {
                tracing::info!("Found {} documents", documents.len());
                Ok(documents)
            }
            Err(e) => {
                tracing::error!("Search error: {:?}", e);
                Err(anyhow::anyhow!("Search error: {:?}", e))
            }
        }
    }

    /// Search with a full SearchQuery for advanced searches
    pub async fn search_advanced(&mut self, query: SearchQuery) -> Result<Vec<Document>> {
        tracing::info!("Advanced search with query: {:?}", query);

        match self.backend_service.search(&query).await {
            Ok(documents) => {
                tracing::info!("Found {} documents", documents.len());
                Ok(documents)
            }
            Err(e) => {
                tracing::error!("Advanced search error: {:?}", e);
                Err(anyhow::anyhow!("Search error: {:?}", e))
            }
        }
    }
}
