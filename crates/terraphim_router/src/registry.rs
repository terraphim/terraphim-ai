//! Provider registry for loading and managing providers.
//!
//! This is a placeholder for Phase 2 implementation.
//! For now, it provides an in-memory registry.

use crate::types::RoutingError;
use std::collections::HashMap;
use terraphim_types::capability::Provider;

/// Registry of capability providers
#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Provider>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }
    
    /// Add a provider to the registry
    pub fn add_provider(&mut self,
        provider: Provider,
    ) {
        self.providers.insert(provider.id.clone(), provider);
    }
    
    /// Get a provider by ID
    pub fn get(&self,
        id: &str,
    ) -> Option<&Provider> {
        self.providers.get(id)
    }
    
    /// Get all providers
    pub fn all(&self) -> Vec<&Provider> {
        self.providers.values().collect()
    }
    
    /// Find providers that have a specific capability
    pub fn find_by_capability(
        &self,
        capability: &terraphim_types::capability::Capability,
    ) -> Vec<&Provider> {
        self.providers
            .values()
            .filter(|p| p.has_capability(capability))
            .collect()
    }
    
    /// Find providers that match all given capabilities
    pub fn find_by_capabilities(
        &self,
        capabilities: &[terraphim_types::capability::Capability],
    ) -> Vec<&Provider> {
        self.providers
            .values()
            .filter(|p| p.has_all_capabilities(capabilities))
            .collect()
    }
    
    /// Load from disk (placeholder for Phase 2)
    pub async fn load_from_disk() -> Result<Self, RoutingError> {
        // TODO: Implement markdown file loading in Phase 2
        Ok(Self::new())
    }
    
    /// Save to disk (placeholder for Phase 2)
    pub async fn save_to_disk(&self,
    ) -> Result<(), RoutingError> {
        // TODO: Implement markdown file saving in Phase 2
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_types::capability::{ProviderType, Capability, CostLevel, Latency};
    
    fn create_test_provider(id: &str) -> Provider {
        Provider {
            id: id.to_string(),
            name: id.to_string(),
            provider_type: ProviderType::Llm {
                model_id: id.to_string(),
                api_endpoint: "https://example.com".to_string(),
            },
            capabilities: vec![Capability::CodeGeneration],
            cost_level: CostLevel::Moderate,
            latency: Latency::Medium,
            keywords: vec![],
        }
    }
    
    #[test]
    fn test_add_and_get() {
        let mut registry = ProviderRegistry::new();
        let provider = create_test_provider("test");
        
        registry.add_provider(provider);
        
        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
    
    #[test]
    fn test_find_by_capability() {
        let mut registry = ProviderRegistry::new();
        
        let mut provider = create_test_provider("coder");
        provider.capabilities = vec![Capability::CodeGeneration, Capability::CodeReview];
        registry.add_provider(provider);
        
        let found = registry.find_by_capability(&Capability::CodeGeneration
        );
        assert_eq!(found.len(), 1);
        
        let found = registry.find_by_capability(&Capability::DeepThinking
        );
        assert_eq!(found.len(), 0);
    }
}
