//! Search service for managing autocomplete and search operations
//!
//! This module provides the service layer for search functionality,
//! including autocomplete index management and thesaurus loading.

use std::sync::{Arc, Mutex};
use tracing::{info, warn, error};
use terraphim_automata::{
    build_autocomplete_index, autocomplete_search, AutocompleteIndex, AutocompleteConfig,
    AutocompleteResult, TerraphimAutomataError,
};
use terraphim_types::Thesaurus;

/// Search service for managing search operations
pub struct SearchService {
    /// Autocomplete index (Arc<Mutex> for thread safety)
    autocomplete_index: Arc<Mutex<Option<AutocompleteIndex>>>,

    /// Autocomplete configuration
    autocomplete_config: AutocompleteConfig,

    /// Current thesaurus name
    current_thesaurus_name: String,
}

impl SearchService {
    /// Create a new SearchService
    pub fn new() -> Self {
        info!("Initializing SearchService");
        Self {
            autocomplete_index: Arc::new(Mutex::new(None)),
            autocomplete_config: AutocompleteConfig {
                max_results: 8,
                min_prefix_length: 1,
                case_sensitive: false,
            },
            current_thesaurus_name: "default".to_string(),
        }
    }

    /// Load a thesaurus from file path and build autocomplete index
    pub fn load_thesaurus_from_path(&self, thesaurus_path: &str) -> Result<(), TerraphimAutomataError> {
        info!("Loading thesaurus from: {}", thesaurus_path);

        // Read thesaurus file
        let thesaurus_data = std::fs::read_to_string(thesaurus_path)
            .map_err(|e| TerraphimAutomataError::Io(e))?;

        // Parse as Thesaurus
        let thesaurus: Thesaurus = serde_json::from_str(&thesaurus_data)
            .map_err(|e| TerraphimAutomataError::Serde(e))?;

        // Build autocomplete index
        let autocomplete_index = build_autocomplete_index(thesaurus, Some(self.autocomplete_config.clone()))
            .map_err(|e| {
                error!("Failed to build autocomplete index: {}", e);
                e
            })?;

        // Store in shared state
        {
            let mut index = self.autocomplete_index.lock().expect("Failed to lock autocomplete_index");
            *index = Some(autocomplete_index);
        }

        info!("Successfully loaded thesaurus and built autocomplete index");
        Ok(())
    }

    /// Load a thesaurus from static data
    pub fn load_thesaurus_from_data(&self, thesaurus_data: &str) -> Result<(), TerraphimAutomataError> {
        info!("Loading thesaurus from data");

        // Parse as Thesaurus
        let thesaurus: Thesaurus = serde_json::from_str(thesaurus_data)
            .map_err(|e| TerraphimAutomataError::Serde(e))?;

        self.current_thesaurus_name = thesaurus.name().to_string();

        // Build autocomplete index
        let autocomplete_index = build_autocomplete_index(thesaurus, Some(self.autocomplete_config.clone()))
            .map_err(|e| {
                error!("Failed to build autocomplete index: {}", e);
                e
            })?;

        // Store in shared state
        {
            let mut index = self.autocomplete_index.lock().expect("Failed to lock autocomplete_index");
            *index = Some(autocomplete_index);
        }

        info!("Successfully loaded thesaurus and built autocomplete index");
        Ok(())
    }

    /// Perform autocomplete search
    pub fn autocomplete_search(&self, query: &str, limit: Option<usize>) -> Vec<AutocompleteResult> {
        let index_guard = self.autocomplete_index.lock().expect("Failed to lock autocomplete_index");

        if let Some(ref index) = *index_guard {
            match autocomplete_search(index, query, limit) {
                Ok(results) => {
                    info!("Autocomplete search for '{}' returned {} results", query, results.len());
                    results
                }
                Err(e) => {
                    error!("Autocomplete search failed: {}", e);
                    Vec::new()
                }
            }
        } else {
            warn!("Autocomplete index not loaded");
            Vec::new()
        }
    }

    /// Get autocomplete index for sharing
    pub fn get_autocomplete_index(&self) -> Arc<Mutex<Option<AutocompleteIndex>>> {
        self.autocomplete_index.clone()
    }

    /// Check if autocomplete index is loaded
    pub fn is_ready(&self) -> bool {
        let index_guard = self.autocomplete_index.lock().expect("Failed to lock autocomplete_index");
        index_guard.is_some()
    }

    /// Get the number of terms in the autocomplete index
    pub fn autocomplete_size(&self) -> usize {
        let index_guard = self.autocomplete_index.lock().expect("Failed to lock autocomplete_index");
        if let Some(ref index) = *index_guard {
            index.len()
        } else {
            0
        }
    }

    /// Get current thesaurus name
    pub fn current_thesaurus(&self) -> &str {
        &self.current_thesaurus_name
    }
}

impl Default for SearchService {
    fn default() -> Self {
        Self::new()
    }
}

/// Default thesaurus with common programming/AI terms
pub const DEFAULT_TERMS: &str = r#"{
  "name": "Programming & AI Terms",
  "data": {
    "rust": { "id": 1, "nterm": "rust programming language" },
    "egui": { "id": 2, "nterm": "egui ui framework" },
    "terraphim": { "id": 3, "nterm": "terraphim ai" },
    "autocomplete": { "id": 4, "nterm": "autocomplete search" },
    "knowledge graph": { "id": 5, "nterm": "knowledge graph database" },
    "wasm": { "id": 6, "nterm": "webassembly" },
    "async": { "id": 7, "nterm": "asynchronous programming" },
    "llm": { "id": 8, "nterm": "large language model" },
    "machine learning": { "id": 9, "nterm": "machine learning" },
    "search": { "id": 10, "nterm": "search algorithm" },
    "context": { "id": 11, "nterm": "context management" },
    "role": { "id": 12, "nterm": "role based access" },
    "config": { "id": 13, "nterm": "configuration management" },
    "thesaurus": { "id": 14, "nterm": "thesaurus dictionary" },
    "middleware": { "id": 15, "nterm": "middleware service" },
    "service": { "id": 16, "nterm": "microservice" },
    "agent": { "id": 17, "nterm": "intelligent agent" },
    "provider": { "id": 18, "nterm": "service provider" },
    "datasource": { "id": 19, "nterm": "data source" },
    "backend": { "id": 20, "nterm": "backend service" },
    "frontend": { "id": 21, "nterm": "frontend interface" },
    "api": { "id": 22, "nterm": "application programming interface" },
    "cli": { "id": 23, "nterm": "command line interface" },
    "gui": { "id": 24, "nterm": "graphical user interface" },
    "database": { "id": 25, "nterm": "database system" },
    "framework": { "id": 26, "nterm": "software framework" },
    "library": { "id": 27, "nterm": "software library" },
    "package": { "id": 28, "nterm": "software package" },
    "module": { "id": 29, "nterm": "code module" },
    "compile": { "id": 30, "nterm": "compilation process" },
    "debug": { "id": 31, "nterm": "debugging process" },
    "test": { "id": 32, "nterm": "testing framework" },
    "build": { "id": 33, "nterm": "build system" },
    "deploy": { "id": 34, "nterm": "deployment process" },
    "docker": { "id": 35, "nterm": "docker container" },
    "kubernetes": { "id": 36, "nterm": "kubernetes orchestration" },
    "cloud": { "id": 37, "nterm": "cloud computing" },
    "server": { "id": 38, "nterm": "server application" },
    "client": { "id": 39, "nterm": "client application" },
    "protocol": { "id": 40, "nterm": "communication protocol" }
  }
}"#;
