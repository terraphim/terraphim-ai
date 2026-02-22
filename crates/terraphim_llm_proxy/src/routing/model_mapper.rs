//! Model mapper module for translating between client and provider model names
//!
//! When the proxy routes a request to a different provider than the client expects,
//! the model name must be translated. For example:
//! - Client requests "claude-3-5-sonnet"
//! - Router selects OpenRouter provider
//! - Model mapper translates to "anthropic/claude-3.5-sonnet"
//!
//! This module provides configurable, pattern-based model name translation.

use crate::client_detection::ClientType;
use std::collections::HashMap;
use tracing::{debug, trace, warn};

/// Error type for model mapping operations
#[derive(Debug, thiserror::Error)]
pub enum ModelMappingError {
    #[error("no mapping found for model '{model}' on provider '{provider}'")]
    NoMappingFound { model: String, provider: String },

    #[error("ambiguous mapping: multiple candidates for '{model}' on provider '{provider}'")]
    AmbiguousMapping { model: String, provider: String },

    #[error("provider '{0}' not found in configuration")]
    ProviderNotFound(String),
}

/// A single model mapping rule
#[derive(Debug, Clone)]
pub struct ModelMapping {
    /// Pattern to match (e.g., "claude-3-5-*" or exact match)
    pub pattern: String,
    /// Target provider name
    pub provider: String,
    /// Target model name on the provider
    pub target_model: String,
    /// Optional: only apply for specific client types
    pub client_type_filter: Option<ClientType>,
    /// Priority (higher = applied first)
    pub priority: i32,
}

impl ModelMapping {
    /// Create a new model mapping
    pub fn new(
        pattern: impl Into<String>,
        provider: impl Into<String>,
        target_model: impl Into<String>,
    ) -> Self {
        Self {
            pattern: pattern.into(),
            provider: provider.into(),
            target_model: target_model.into(),
            client_type_filter: None,
            priority: 0,
        }
    }

    /// Set client type filter
    pub fn for_client(mut self, client_type: ClientType) -> Self {
        self.client_type_filter = Some(client_type);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Check if this mapping applies to the given model name
    pub fn matches(&self, model: &str) -> bool {
        // Support glob patterns with * wildcard
        if self.pattern.contains('*') {
            let pattern_parts: Vec<&str> = self.pattern.split('*').collect();
            if pattern_parts.len() == 2 {
                // Simple prefix/suffix pattern like "claude-3-5-*"
                let prefix = pattern_parts[0];
                let suffix = pattern_parts[1];
                return model.starts_with(prefix) && model.ends_with(suffix);
            }
        }
        // Exact match
        self.pattern == model
    }

    /// Check if this mapping applies to the given client type
    pub fn matches_client(&self, client_type: ClientType) -> bool {
        match &self.client_type_filter {
            Some(filter) => *filter == client_type,
            None => true, // No filter means applies to all clients
        }
    }
}

/// Strategy for handling models without explicit mappings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Try to use the model name as-is
    Passthrough,
    /// Return an error if no mapping found
    Error,
    /// Try to find a similar model name
    FuzzyMatch,
}

/// Model mapper that translates client model names to provider-specific names
#[derive(Debug, Clone)]
pub struct ModelMapper {
    mappings: Vec<ModelMapping>,
    fallback_strategy: FallbackStrategy,
    /// Cache of successful translations
    cache: HashMap<(String, String, ClientType), String>, // (model, provider, client) -> target_model
}

impl ModelMapper {
    /// Create a new model mapper with default settings
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
            fallback_strategy: FallbackStrategy::Passthrough,
            cache: HashMap::new(),
        }
    }

    /// Create a model mapper with default mappings
    pub fn with_defaults() -> Self {
        let mut mapper = Self::new();
        mapper.add_default_mappings();
        mapper
    }

    /// Set the fallback strategy
    pub fn with_fallback(mut self, strategy: FallbackStrategy) -> Self {
        self.fallback_strategy = strategy;
        self
    }

    /// Add a mapping rule
    pub fn add_mapping(&mut self, mapping: ModelMapping) {
        self.mappings.push(mapping);
        // Sort by priority (highest first)
        self.mappings.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Add multiple mappings
    pub fn add_mappings(&mut self, mappings: Vec<ModelMapping>) {
        for mapping in mappings {
            self.add_mapping(mapping);
        }
    }

    /// Translate a client model name to a provider-specific model name
    ///
    /// # Arguments
    /// * `client_model` - The model name from the client request
    /// * `client_type` - The detected client type
    /// * `target_provider` - The provider to route to
    /// * `available_models` - List of models available on the target provider
    ///
    /// # Returns
    /// The translated model name or an error if no mapping found
    pub fn translate(
        &mut self,
        client_model: &str,
        client_type: ClientType,
        target_provider: &str,
        available_models: &[String],
    ) -> Result<String, ModelMappingError> {
        let cache_key = (
            client_model.to_string(),
            target_provider.to_string(),
            client_type,
        );

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            trace!(
                "Cache hit for model translation: {} -> {}",
                client_model,
                cached
            );
            return Ok(cached.clone());
        }

        // Find matching mappings
        let mut candidates: Vec<&ModelMapping> = self
            .mappings
            .iter()
            .filter(|m| {
                m.provider == target_provider
                    && m.matches(client_model)
                    && m.matches_client(client_type)
            })
            .collect();

        // If no client-specific mappings, try generic ones
        if candidates.is_empty() {
            candidates = self
                .mappings
                .iter()
                .filter(|m| {
                    m.provider == target_provider
                        && m.matches(client_model)
                        && m.client_type_filter.is_none()
                })
                .collect();
        }

        if candidates.len() > 1 {
            // Multiple mappings matched - use the highest priority one
            warn!(
                "Multiple model mappings matched for '{}' on provider '{}', using highest priority",
                client_model, target_provider
            );
        }

        if let Some(mapping) = candidates.first() {
            let target_model = mapping.target_model.clone();

            // Verify the target model is available on the provider
            if !available_models.contains(&target_model) {
                warn!(
                    "Mapped model '{}' not found in provider '{}' available models: {:?}",
                    target_model, target_provider, available_models
                );
            }

            // Cache the result
            self.cache.insert(cache_key, target_model.clone());

            debug!(
                "Model translated: {} -> {} (provider: {}, client: {})",
                client_model, target_model, target_provider, client_type
            );

            return Ok(target_model);
        }

        // No explicit mapping found - apply fallback strategy
        match self.fallback_strategy {
            FallbackStrategy::Passthrough => {
                // Check if the client model is directly available
                if available_models.contains(&client_model.to_string()) {
                    trace!(
                        "Passthrough: model '{}' available on provider '{}'",
                        client_model,
                        target_provider
                    );
                    Ok(client_model.to_string())
                } else {
                    warn!(
                        "Model '{}' not available on provider '{}', available: {:?}",
                        client_model, target_provider, available_models
                    );
                    // Still passthrough - let the provider return the error
                    Ok(client_model.to_string())
                }
            }
            FallbackStrategy::Error => Err(ModelMappingError::NoMappingFound {
                model: client_model.to_string(),
                provider: target_provider.to_string(),
            }),
            FallbackStrategy::FuzzyMatch => {
                // Try to find a similar model name
                if let Some(similar) = self.find_similar_model(client_model, available_models) {
                    debug!("Fuzzy match: {} -> {}", client_model, similar);
                    Ok(similar)
                } else {
                    Err(ModelMappingError::NoMappingFound {
                        model: client_model.to_string(),
                        provider: target_provider.to_string(),
                    })
                }
            }
        }
    }

    /// Find a similar model name from available models
    fn find_similar_model(&self, model: &str, available: &[String]) -> Option<String> {
        // Simple heuristic: find model with most common substring
        available
            .iter()
            .max_by_key(|m| common_substring_length(model, m))
            .cloned()
    }

    /// Clear the translation cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Add default model mappings for common scenarios
    fn add_default_mappings(&mut self) {
        // Claude models to OpenRouter format
        self.add_mapping(
            ModelMapping::new(
                "claude-3-5-sonnet",
                "openrouter",
                "anthropic/claude-3.5-sonnet",
            )
            .with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new(
                "claude-3-5-haiku",
                "openrouter",
                "anthropic/claude-3.5-haiku",
            )
            .with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new("claude-3-opus", "openrouter", "anthropic/claude-3-opus")
                .with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new(
                "claude-sonnet-4-5",
                "openrouter",
                "anthropic/claude-sonnet-4.5",
            )
            .with_priority(100),
        );

        // Pattern-based mappings (lower priority)
        self.add_mapping(
            ModelMapping::new("claude-*", "openrouter", "anthropic/claude-3.5-sonnet")
                .with_priority(50),
        );

        // OpenAI models to Groq (when routing for speed)
        self.add_mapping(
            ModelMapping::new("gpt-4o", "groq", "llama-3.1-70b-versatile").with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new("gpt-4", "groq", "llama-3.1-70b-versatile").with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new("gpt-3.5-turbo", "groq", "llama-3.1-8b-instant").with_priority(100),
        );

        // Pattern-based for any gpt-*
        self.add_mapping(
            ModelMapping::new("gpt-*", "groq", "llama-3.1-70b-versatile").with_priority(50),
        );

        // DeepSeek models (usually direct passthrough, but add explicit mappings)
        self.add_mapping(
            ModelMapping::new("deepseek-chat", "deepseek", "deepseek-chat").with_priority(100),
        );
        self.add_mapping(
            ModelMapping::new("deepseek-reasoner", "deepseek", "deepseek-reasoner")
                .with_priority(100),
        );

        debug!("Added {} default model mappings", self.mappings.len());
    }
}

impl Default for ModelMapper {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Calculate length of common substring between two strings
fn common_substring_length(a: &str, b: &str) -> usize {
    let a_lower = a.to_lowercase();
    let b_lower = b.to_lowercase();

    let min_len = a_lower.len().min(b_lower.len());
    let mut common = 0;

    for i in 0..min_len {
        if a_lower.chars().nth(i) == b_lower.chars().nth(i) {
            common += 1;
        } else {
            break;
        }
    }

    common
}

/// Convenience function to translate a model name
pub fn translate_model(
    client_model: &str,
    client_type: ClientType,
    target_provider: &str,
    available_models: &[String],
) -> Result<String, ModelMappingError> {
    let mut mapper = ModelMapper::with_defaults();
    mapper.translate(client_model, client_type, target_provider, available_models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_mapping_exact_match() {
        let mapping = ModelMapping::new(
            "claude-3-5-sonnet",
            "openrouter",
            "anthropic/claude-3.5-sonnet",
        );
        assert!(mapping.matches("claude-3-5-sonnet"));
        assert!(!mapping.matches("claude-3-5-haiku"));
    }

    #[test]
    fn test_model_mapping_glob_pattern() {
        let mapping = ModelMapping::new("claude-*", "openrouter", "anthropic/claude-3.5-sonnet");
        assert!(mapping.matches("claude-3-5-sonnet"));
        assert!(mapping.matches("claude-3-5-haiku"));
        assert!(mapping.matches("claude-3-opus"));
        assert!(!mapping.matches("gpt-4o"));
    }

    #[test]
    fn test_model_mapping_client_filter() {
        let mapping = ModelMapping::new(
            "claude-3-5-sonnet",
            "openrouter",
            "anthropic/claude-3.5-sonnet",
        )
        .for_client(ClientType::ClaudeCode);

        assert!(mapping.matches_client(ClientType::ClaudeCode));
        assert!(!mapping.matches_client(ClientType::CodexCli));
        assert!(!mapping.matches_client(ClientType::OpenAiGeneric));
    }

    #[test]
    fn test_mapper_translate_claude_to_openrouter() {
        let available = vec![
            "anthropic/claude-3.5-sonnet".to_string(),
            "anthropic/claude-3.5-haiku".to_string(),
        ];
        let mut mapper = ModelMapper::with_defaults();

        let result = mapper.translate(
            "claude-3-5-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert_eq!(result.unwrap(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_mapper_translate_gpt_to_groq() {
        let available = vec![
            "llama-3.1-70b-versatile".to_string(),
            "llama-3.1-8b-instant".to_string(),
        ];
        let mut mapper = ModelMapper::with_defaults();

        let result = mapper.translate("gpt-4o", ClientType::CodexCli, "groq", &available);
        assert_eq!(result.unwrap(), "llama-3.1-70b-versatile");
    }

    #[test]
    fn test_mapper_passthrough_for_available_model() {
        let available = vec!["custom-model".to_string()];
        let mut mapper = ModelMapper::with_defaults();

        // Model not in mappings but available on provider
        let result = mapper.translate(
            "custom-model",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert_eq!(result.unwrap(), "custom-model");
    }

    #[test]
    fn test_mapper_error_when_no_mapping_and_not_available() {
        let available = vec!["other-model".to_string()];
        let mut mapper = ModelMapper::new().with_fallback(FallbackStrategy::Error);

        // No mapping and not available
        let result = mapper.translate(
            "unknown-model",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert!(matches!(
            result,
            Err(ModelMappingError::NoMappingFound { .. })
        ));
    }

    #[test]
    fn test_mapper_caching() {
        let available = vec!["anthropic/claude-3.5-sonnet".to_string()];
        let mut mapper = ModelMapper::with_defaults();

        // First call - should compute and cache
        let result1 = mapper.translate(
            "claude-3-5-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert_eq!(result1.unwrap(), "anthropic/claude-3.5-sonnet");

        // Check cache has entry
        let cache_key = (
            "claude-3-5-sonnet".to_string(),
            "openrouter".to_string(),
            ClientType::ClaudeCode,
        );
        assert!(mapper.cache.contains_key(&cache_key));

        // Second call - should use cache
        let result2 = mapper.translate(
            "claude-3-5-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert_eq!(result2.unwrap(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_mapper_priority_ordering() {
        let mut mapper = ModelMapper::new();

        // Add lower priority mapping first
        mapper.add_mapping(
            ModelMapping::new("claude-*", "openrouter", "generic-claude").with_priority(50),
        );

        // Add higher priority mapping
        mapper.add_mapping(
            ModelMapping::new("claude-3-5-sonnet", "openrouter", "specific-claude")
                .with_priority(100),
        );

        let available = vec!["specific-claude".to_string(), "generic-claude".to_string()];
        let result = mapper.translate(
            "claude-3-5-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );

        // Should use higher priority (specific) mapping
        assert_eq!(result.unwrap(), "specific-claude");
    }

    #[test]
    fn test_mapper_ambiguous_mapping_warning() {
        let mut mapper = ModelMapper::new();

        // Add two mappings with same priority that both match
        mapper
            .add_mapping(ModelMapping::new("claude-*", "openrouter", "option-a").with_priority(50));
        mapper
            .add_mapping(ModelMapping::new("*sonnet*", "openrouter", "option-b").with_priority(50));

        let available = vec!["option-a".to_string(), "option-b".to_string()];
        let result = mapper.translate(
            "claude-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );

        // Should still work, using first match
        assert!(result.is_ok());
    }

    #[test]
    fn test_common_substring_length() {
        assert_eq!(
            common_substring_length("claude-3-5-sonnet", "claude-3-5-haiku"),
            11
        ); // "claude-3-5-"
        assert_eq!(common_substring_length("gpt-4o", "gpt-4-turbo"), 5); // "gpt-4"
        assert_eq!(common_substring_length("abc", "xyz"), 0);
        assert_eq!(common_substring_length("same", "same"), 4);
    }

    #[test]
    fn test_fuzzy_match_fallback() {
        let available = vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "gpt-4o".to_string(),
        ];
        let mut mapper = ModelMapper::new().with_fallback(FallbackStrategy::FuzzyMatch);

        // No explicit mapping, but should fuzzy match to closest
        let result = mapper.translate(
            "claude-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );

        // Should match "claude-3-5-sonnet-20241022" as closest
        let matched = result.unwrap();
        assert!(matched.contains("claude"));
    }

    #[test]
    fn test_translate_model_convenience_function() {
        let available = vec!["anthropic/claude-3.5-sonnet".to_string()];

        let result = translate_model(
            "claude-3-5-sonnet",
            ClientType::ClaudeCode,
            "openrouter",
            &available,
        );
        assert_eq!(result.unwrap(), "anthropic/claude-3.5-sonnet");
    }

    #[test]
    fn test_default_mappings_count() {
        let mapper = ModelMapper::with_defaults();
        // Should have at least the default mappings we added
        assert!(mapper.mappings.len() >= 8);
    }
}
