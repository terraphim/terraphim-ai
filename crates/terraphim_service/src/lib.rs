//! Main service layer for Terraphim AI.
//!
//! Provides document search, indexing, and AI-assisted summarisation across
//! multiple haystack backends. Integrates the knowledge graph, thesaurus,
//! and relevance-scoring pipeline into a single async service facade.
use terraphim_config::ConfigState;
use terraphim_persistence::Persistable;
use terraphim_types::SearchQuery;
mod document;
mod score;
mod search;
mod summary;
mod thesaurus;

pub mod auto_route;
pub use auto_route::{
    AutoRouteContext, AutoRouteReason, AutoRouteResult, JMAP_MISSING_TOKEN_PENALTY,
    auto_select_role,
};

#[cfg(feature = "openrouter")]
pub mod openrouter;

// Generic LLM layer for multiple providers (OpenRouter, Ollama, etc.)
pub mod llm;

// LLM proxy service for unified provider management

// LLM Proxy service\npub mod proxy_client;
// LLM Router configuration integration\n

pub mod llm_proxy;

// LLM Router configuration integration\n

// Centralized HTTP client creation and configuration
pub mod http_client;

// Standardized logging initialization utilities
pub mod logging;

// Summarization queue system for production-ready async processing
pub mod conversation_service;
pub mod rate_limiter;
pub mod summarization_manager;
pub mod summarization_queue;
pub mod summarization_worker;

// Centralized error handling patterns and utilities
pub mod error;

// Context management for LLM conversations
pub mod context;

#[cfg(test)]
mod context_tests;

/// Normalize a filename to be used as a document ID
///
/// This ensures consistent ID generation between server startup and edit API
pub(crate) fn normalize_filename_to_id(filename: &str) -> String {
    let re = regex::Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create regex");
    re.replace_all(filename, "").to_lowercase()
}

/// Top-level error type for the Terraphim service layer.
#[derive(thiserror::Error, Debug)]
pub enum ServiceError {
    #[error("Middleware error: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("OpenDal error: {0}")]
    OpenDal(Box<opendal::Error>),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[cfg(feature = "openrouter")]
    #[error("OpenRouter error: {0}")]
    OpenRouter(#[from] crate::openrouter::OpenRouterError),

    #[error("Common error: {0}")]
    Common(#[from] crate::error::CommonError),
}

impl From<opendal::Error> for ServiceError {
    fn from(err: opendal::Error) -> Self {
        ServiceError::OpenDal(Box::new(err))
    }
}

impl crate::error::TerraphimError for ServiceError {
    fn category(&self) -> crate::error::ErrorCategory {
        use crate::error::ErrorCategory;
        match self {
            ServiceError::Middleware(_) => ErrorCategory::Integration,
            ServiceError::OpenDal(_) => ErrorCategory::Storage,
            ServiceError::Persistence(_) => ErrorCategory::Storage,
            ServiceError::Config(_) => ErrorCategory::Configuration,
            #[cfg(feature = "openrouter")]
            ServiceError::OpenRouter(_) => ErrorCategory::Integration,
            ServiceError::Common(err) => err.category(),
        }
    }

    fn is_recoverable(&self) -> bool {
        match self {
            ServiceError::Middleware(_) => true,
            ServiceError::OpenDal(_) => false,
            ServiceError::Persistence(_) => false,
            ServiceError::Config(_) => false,
            #[cfg(feature = "openrouter")]
            ServiceError::OpenRouter(_) => true,
            ServiceError::Common(err) => err.is_recoverable(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Main entry point for search, indexing, and AI operations in Terraphim.
pub struct TerraphimService {
    config_state: ConfigState,
}

impl TerraphimService {
    /// Create a new TerraphimService
    pub fn new(config_state: ConfigState) -> Self {
        Self { config_state }
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> terraphim_config::Config {
        let current_config = self.config_state.config.lock().await;
        current_config.clone()
    }

    // Test helper methods
    #[cfg(test)]
    pub async fn get_role(
        &self,
        role_name: &terraphim_types::RoleName,
    ) -> Result<terraphim_config::Role> {
        let config = self.config_state.config.lock().await;
        config
            .roles
            .get(role_name)
            .cloned()
            .ok_or_else(|| ServiceError::Config(format!("Role '{}' not found", role_name)))
    }

    /// Update the config
    ///
    /// Overwrites the config in the config state and returns the updated
    /// config.
    pub async fn update_config(
        &self,
        config: terraphim_config::Config,
    ) -> Result<terraphim_config::Config> {
        // Lock briefly to swap in the new config, then drop before save so
        // the disk write doesn't block other /config endpoints.
        {
            let mut current_config = self.config_state.config.lock().await;
            *current_config = config.clone();
        }
        config.save().await?;
        log::info!("Config updated");
        Ok(config)
    }

    /// Update only the `selected_role` in the config without mutating the rest of the
    /// configuration. Returns the up-to-date `Config` object.
    pub async fn update_selected_role(
        &self,
        role_name: terraphim_types::RoleName,
    ) -> Result<terraphim_config::Config> {
        // Lock briefly: validate, mutate in-memory state, snapshot. Drop the
        // lock BEFORE the disk save -- holding the config mutex across an
        // async I/O write blocks every other endpoint that touches /config
        // (e.g. concurrent search, get_config) for the duration of the save.
        let snapshot = {
            let mut current_config = self.config_state.config.lock().await;

            if !current_config.roles.contains_key(&role_name) {
                return Err(ServiceError::Config(format!(
                    "Role `{}` not found in config",
                    role_name
                )));
            }

            current_config.selected_role = role_name.clone();
            current_config.clone()
        };
        // Persist asynchronously: in-memory update is the source of truth for
        // subsequent reads; disk save is best-effort and must not delay the
        // HTTP response. save_to_all() can take many seconds depending on the
        // configured persistence profiles (sled WAL flush, S3 PUT, etc.) and
        // should never block role selection.
        let snapshot_for_save = snapshot.clone();
        let role_for_log = role_name.clone();
        tokio::spawn(async move {
            if let Err(e) = snapshot_for_save.save().await {
                log::warn!(
                    "background persist of selected_role={} failed: {}",
                    role_for_log,
                    e
                );
            }
        });
        // Log role selection from the snapshot (no need to re-lock).
        if let Some(role) = snapshot.roles.get(&role_name) {
            if role.terraphim_it {
                log::info!(
                    "🎯 Selected role '{}' → terraphim_it: ENABLED (KG preprocessing will be applied)",
                    role_name
                );
            } else {
                log::info!("🎯 Selected role '{}' → terraphim_it: DISABLED", role_name);
            }
        }

        Ok(snapshot)
    }

    /// Highlight search terms in the given text content
    ///
    /// This method wraps matching search terms with HTML-style highlighting tags
    /// to make them visually distinct in the frontend.
    pub(crate) fn highlight_search_terms(content: &str, search_query: &SearchQuery) -> String {
        let mut highlighted_content = content.to_string();

        // Get all terms from the search query
        let terms = search_query.get_all_terms();

        // Sort terms by length (longest first) to avoid partial replacements
        let mut sorted_terms: Vec<&str> = terms.iter().map(|t| t.as_str()).collect();
        sorted_terms.sort_by_key(|term| std::cmp::Reverse(term.len()));

        for term in sorted_terms {
            if term.trim().is_empty() {
                continue;
            }

            // Create case-insensitive regex for the term
            // Escape special regex characters in the search term
            let escaped_term = regex::escape(term);

            if let Ok(regex) = regex::RegexBuilder::new(&escaped_term)
                .case_insensitive(true)
                .build()
            {
                // Replace all matches with highlighted version
                // Use a unique delimiter to avoid conflicts with existing HTML
                let highlight_open = "<mark class=\"search-highlight\">";
                let highlight_close = "</mark>";

                highlighted_content = regex
                    .replace_all(
                        &highlighted_content,
                        format!("{}{}{}", highlight_open, "$0", highlight_close),
                    )
                    .to_string();
            }
        }

        highlighted_content
    }
}

pub(crate) fn snippet_around(s: &str, marker: &str, before: usize, after: usize) -> String {
    let Some(marker_byte) = s.find(marker) else {
        return String::new();
    };
    let marker_char_index = s[..marker_byte].chars().count();
    let total_chars = s.chars().count();

    let start_char_index = marker_char_index.saturating_sub(before);
    let end_char_index = (marker_char_index + marker.len() + after).min(total_chars);

    if start_char_index >= end_char_index {
        return String::new();
    }

    s.chars()
        .skip(start_char_index)
        .take(end_char_index - start_char_index)
        .collect()
}

#[cfg(test)]
mod lib_tests;
