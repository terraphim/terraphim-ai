//! Pricing database for cost management

use crate::cost::{CostConfig, PricingInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

/// Database for storing and retrieving pricing information
#[derive(Debug, Clone)]
pub struct PricingDatabase {
    #[allow(dead_code)]
    config: Arc<RwLock<CostConfig>>,
    pricing_data: Arc<RwLock<HashMap<String, HashMap<String, ModelPricing>>>>,
}

/// Extended pricing information with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub base_info: PricingInfo,
    pub provider: String,
    pub model: String,
    pub tier: Option<String>,
    pub region: Option<String>,
    pub min_tokens: Option<u32>,
    pub max_tokens: Option<u32>,
    pub discounts: Vec<DiscountInfo>,
}

/// Discount information for bulk usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountInfo {
    pub min_tokens: u32,
    pub discount_percentage: f64,
    pub description: String,
}

impl PricingDatabase {
    /// Create a new pricing database
    pub fn new(config: CostConfig) -> Self {
        let mut pricing_data = HashMap::new();

        // Convert config pricing to internal format
        for (provider, models) in config.pricing.iter() {
            let mut provider_pricing = HashMap::new();
            for (model, pricing_info) in models.iter() {
                provider_pricing.insert(
                    model.clone(),
                    ModelPricing {
                        base_info: pricing_info.clone(),
                        provider: provider.clone(),
                        model: model.clone(),
                        tier: None,
                        region: None,
                        min_tokens: None,
                        max_tokens: None,
                        discounts: Vec::new(),
                    },
                );
            }
            pricing_data.insert(provider.clone(), provider_pricing);
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            pricing_data: Arc::new(RwLock::new(pricing_data)),
        }
    }

    /// Get pricing information for a specific provider and model
    pub async fn get_pricing(&self, provider: &str, model: &str) -> Option<ModelPricing> {
        let pricing_data = self.pricing_data.read().await;
        pricing_data.get(provider)?.get(model).cloned()
    }

    /// Get all models for a provider
    pub async fn get_provider_models(&self, provider: &str) -> Vec<String> {
        let pricing_data = self.pricing_data.read().await;
        pricing_data
            .get(provider)
            .map(|models| models.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all providers
    pub async fn get_providers(&self) -> Vec<String> {
        let pricing_data = self.pricing_data.read().await;
        pricing_data.keys().cloned().collect()
    }

    /// Add or update pricing information
    pub async fn update_pricing(&self, provider: &str, model: &str, pricing: ModelPricing) {
        let mut pricing_data = self.pricing_data.write().await;
        pricing_data
            .entry(provider.to_string())
            .or_insert_with(HashMap::new)
            .insert(model.to_string(), pricing);

        debug!("Updated pricing for {}/{}", provider, model);
    }

    /// Remove pricing information
    pub async fn remove_pricing(&self, provider: &str, model: &str) -> bool {
        let mut pricing_data = self.pricing_data.write().await;
        if let Some(models) = pricing_data.get_mut(provider) {
            let removed = models.remove(model).is_some();
            if removed {
                debug!("Removed pricing for {}/{}", provider, model);
            }
            removed
        } else {
            false
        }
    }

    /// Find cheapest model for given requirements
    pub async fn find_cheapest_model(
        &self,
        provider: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
    ) -> Option<(String, String, f64)> {
        let pricing_data = self.pricing_data.read().await;
        let mut cheapest: Option<(String, String, f64)> = None;

        let providers_to_check = match provider {
            Some(p) => vec![p.to_string()],
            None => pricing_data.keys().cloned().collect(),
        };

        for prov in providers_to_check {
            if let Some(models) = pricing_data.get(&prov) {
                for (model, model_pricing) in models.iter() {
                    let cost = model_pricing
                        .base_info
                        .calculate_cost(input_tokens, output_tokens);

                    match cheapest {
                        None => cheapest = Some((prov.clone(), model.clone(), cost)),
                        Some((_, _, current_cost)) if cost < current_cost => {
                            cheapest = Some((prov.clone(), model.clone(), cost));
                        }
                        _ => {}
                    }
                }
            }
        }

        cheapest
    }

    /// Get models within budget
    pub async fn get_models_within_budget(
        &self,
        provider: Option<&str>,
        input_tokens: u32,
        output_tokens: u32,
        budget: f64,
    ) -> Vec<(String, String, f64)> {
        let pricing_data = self.pricing_data.read().await;
        let mut affordable = Vec::new();

        let providers_to_check = match provider {
            Some(p) => vec![p.to_string()],
            None => pricing_data.keys().cloned().collect(),
        };

        for prov in providers_to_check {
            if let Some(models) = pricing_data.get(&prov) {
                for (model, model_pricing) in models.iter() {
                    let cost = model_pricing
                        .base_info
                        .calculate_cost(input_tokens, output_tokens);
                    if cost <= budget {
                        affordable.push((prov.clone(), model.clone(), cost));
                    }
                }
            }
        }

        // Sort by cost (cheapest first)
        affordable.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        affordable
    }

    /// Check if pricing is stale and needs update
    pub async fn check_stale_pricing(&self, hours: u64) -> Vec<(String, String)> {
        let pricing_data = self.pricing_data.read().await;
        let mut stale = Vec::new();

        for (provider, models) in pricing_data.iter() {
            for (model, model_pricing) in models.iter() {
                if model_pricing.base_info.is_stale(hours) {
                    stale.push((provider.clone(), model.clone()));
                }
            }
        }

        stale
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> PricingDatabaseStats {
        let pricing_data = self.pricing_data.read().await;
        let mut total_models = 0;
        let total_providers = pricing_data.len();

        for models in pricing_data.values() {
            total_models += models.len();
        }

        PricingDatabaseStats {
            total_providers,
            total_models,
            last_updated: chrono::Utc::now(),
        }
    }
}

/// Statistics for the pricing database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingDatabaseStats {
    pub total_providers: usize,
    pub total_models: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pricing_database_creation() {
        let config = CostConfig::default();
        let db = PricingDatabase::new(config);

        let providers = db.get_providers().await;
        assert!(!providers.is_empty());
        assert!(providers.contains(&"openai".to_string()));
    }

    #[tokio::test]
    async fn test_get_pricing() {
        let config = CostConfig::default();
        let db = PricingDatabase::new(config);

        let pricing = db.get_pricing("openai", "gpt-4").await;
        assert!(pricing.is_some());

        let pricing = pricing.unwrap();
        assert_eq!(pricing.provider, "openai");
        assert_eq!(pricing.model, "gpt-4");
        assert_eq!(pricing.base_info.input_token_price, 0.03);
    }

    #[tokio::test]
    async fn test_find_cheapest_model() {
        let config = CostConfig::default();
        let db = PricingDatabase::new(config);

        let cheapest = db.find_cheapest_model(None, 1000, 500).await;
        assert!(cheapest.is_some());

        let (provider, model, _cost) = cheapest.unwrap();
        assert_eq!(provider, "openai");
        assert_eq!(model, "gpt-3.5-turbo"); // Should be the cheapest
    }

    #[tokio::test]
    async fn test_update_pricing() {
        let config = CostConfig::default();
        let db = PricingDatabase::new(config);

        let new_pricing = ModelPricing {
            base_info: PricingInfo {
                input_token_price: 0.05,
                output_token_price: 0.10,
                per_request_price: None,
                currency: "USD".to_string(),
                last_updated: chrono::Utc::now(),
            },
            provider: "test".to_string(),
            model: "test-model".to_string(),
            tier: None,
            region: None,
            min_tokens: None,
            max_tokens: None,
            discounts: Vec::new(),
        };

        db.update_pricing("test", "test-model", new_pricing).await;

        let pricing = db.get_pricing("test", "test-model").await;
        assert!(pricing.is_some());
        assert_eq!(pricing.unwrap().base_info.input_token_price, 0.05);
    }
}
