//! Model Router for Judge LLM tier selection
//!
//! Maps judge tiers (quick/deep/tiebreaker/oracle) to provider+model pairs
//! based on configuration from automation/judge/model-mapping.json.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{JudgeError, JudgeResult};

/// Configuration for a specific judge tier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// The LLM provider identifier
    pub provider: String,
    /// The model name within the provider
    pub model: String,
}

impl TierConfig {
    /// Create a new tier configuration
    pub fn new(provider: String, model: String) -> Self {
        Self { provider, model }
    }

    /// Get the provider and model as a tuple
    pub fn as_tuple(&self) -> (String, String) {
        (self.provider.clone(), self.model.clone())
    }
}

/// Complete model mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMappingConfig {
    /// Quick tier configuration (fast, cheap)
    pub quick: TierConfig,
    /// Deep tier configuration (thorough analysis)
    pub deep: TierConfig,
    /// Tiebreaker tier configuration (final arbitration)
    pub tiebreaker: TierConfig,
    /// Oracle tier configuration (highest quality)
    pub oracle: TierConfig,
    /// Profile definitions mapping profile names to tier sequences
    #[serde(default)]
    pub profiles: HashMap<String, Vec<String>>,
}

impl Default for ModelMappingConfig {
    fn default() -> Self {
        Self {
            quick: TierConfig::new("opencode-go".to_string(), "minimax-m2.5".to_string()),
            deep: TierConfig::new("opencode-go".to_string(), "glm-5".to_string()),
            tiebreaker: TierConfig::new("kimi-for-coding".to_string(), "k2p5".to_string()),
            oracle: TierConfig::new("claude-code".to_string(), "opus-4-6".to_string()),
            profiles: {
                let mut profiles = HashMap::new();
                profiles.insert("default".to_string(), vec!["quick".to_string()]);
                profiles.insert(
                    "thorough".to_string(),
                    vec!["quick".to_string(), "deep".to_string()],
                );
                profiles.insert(
                    "critical".to_string(),
                    vec!["deep".to_string(), "tiebreaker".to_string()],
                );
                profiles.insert(
                    "exhaustive".to_string(),
                    vec![
                        "quick".to_string(),
                        "deep".to_string(),
                        "tiebreaker".to_string(),
                        "oracle".to_string(),
                    ],
                );
                profiles
            },
        }
    }
}

/// JudgeModelRouter maps judge tiers to provider+model pairs
#[derive(Debug, Clone)]
pub struct JudgeModelRouter {
    config: ModelMappingConfig,
}

impl JudgeModelRouter {
    /// Create a new router with default configuration
    pub fn new() -> Self {
        Self {
            config: ModelMappingConfig::default(),
        }
    }

    /// Load configuration from a JSON file
    ///
    /// # Example
    /// ```
    /// use terraphim_judge_evaluator::JudgeModelRouter;
    /// use std::path::Path;
    ///
    /// // let router = JudgeModelRouter::from_config(Path::new("automation/judge/model-mapping.json")).unwrap();
    /// ```
    pub fn from_config(path: &Path) -> JudgeResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            JudgeError::ConfigLoadError(format!("Failed to read {}: {}", path.display(), e))
        })?;

        let config: ModelMappingConfig = serde_json::from_str(&content)
            .map_err(|e| JudgeError::ConfigLoadError(format!("Failed to parse config: {}", e)))?;

        Ok(Self { config })
    }

    /// Resolve a judge tier to its provider and model
    ///
    /// Returns a tuple of (provider, model) for the given tier.
    ///
    /// # Example
    /// ```
    /// use terraphim_judge_evaluator::JudgeModelRouter;
    ///
    /// let router = JudgeModelRouter::new();
    /// let (provider, model) = router.resolve_tier("quick").unwrap();
    /// assert_eq!(provider, "opencode-go");
    /// ```
    pub fn resolve_tier(&self, tier: &str) -> JudgeResult<(String, String)> {
        match tier {
            "quick" => Ok(self.config.quick.as_tuple()),
            "deep" => Ok(self.config.deep.as_tuple()),
            "tiebreaker" => Ok(self.config.tiebreaker.as_tuple()),
            "oracle" => Ok(self.config.oracle.as_tuple()),
            _ => Err(JudgeError::UnknownTier(tier.to_string())),
        }
    }

    /// Resolve a profile to its sequence of tiers
    ///
    /// Returns a vector of (provider, model) tuples for the given profile.
    ///
    /// # Example
    /// ```
    /// use terraphim_judge_evaluator::JudgeModelRouter;
    ///
    /// let router = JudgeModelRouter::new();
    /// let tiers = router.resolve_profile("thorough").unwrap();
    /// assert_eq!(tiers.len(), 2);
    /// ```
    pub fn resolve_profile(&self, profile: &str) -> JudgeResult<Vec<(String, String)>> {
        let tier_names = self
            .config
            .profiles
            .get(profile)
            .ok_or_else(|| JudgeError::UnknownProfile(profile.to_string()))?;

        let mut result = Vec::new();
        for tier_name in tier_names {
            let tier_config = self.resolve_tier(tier_name)?;
            result.push(tier_config);
        }

        Ok(result)
    }

    /// Get the raw configuration
    pub fn config(&self) -> &ModelMappingConfig {
        &self.config
    }

    /// Get all available tier names
    pub fn available_tiers(&self) -> Vec<&str> {
        vec!["quick", "deep", "tiebreaker", "oracle"]
    }

    /// Get all available profile names
    pub fn available_profiles(&self) -> Vec<&String> {
        self.config.profiles.keys().collect()
    }
}

impl Default for JudgeModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_config() -> String {
        r#"{
            "quick": {
                "provider": "test-provider",
                "model": "test-quick"
            },
            "deep": {
                "provider": "test-provider",
                "model": "test-deep"
            },
            "tiebreaker": {
                "provider": "test-tiebreaker",
                "model": "test-tb"
            },
            "oracle": {
                "provider": "test-oracle",
                "model": "test-oracle-model"
            },
            "profiles": {
                "default": ["quick"],
                "thorough": ["quick", "deep"],
                "critical": ["deep", "tiebreaker"],
                "exhaustive": ["quick", "deep", "tiebreaker", "oracle"],
                "custom": ["quick", "oracle"]
            }
        }"#
        .to_string()
    }

    #[test]
    fn test_load_config() {
        let config_json = create_test_config();
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();

        let router = JudgeModelRouter::from_config(temp_file.path()).unwrap();

        // Verify all tiers loaded correctly
        let (provider, model) = router.resolve_tier("quick").unwrap();
        assert_eq!(provider, "test-provider");
        assert_eq!(model, "test-quick");

        let (provider, model) = router.resolve_tier("deep").unwrap();
        assert_eq!(provider, "test-provider");
        assert_eq!(model, "test-deep");

        let (provider, model) = router.resolve_tier("tiebreaker").unwrap();
        assert_eq!(provider, "test-tiebreaker");
        assert_eq!(model, "test-tb");

        let (provider, model) = router.resolve_tier("oracle").unwrap();
        assert_eq!(provider, "test-oracle");
        assert_eq!(model, "test-oracle-model");
    }

    #[test]
    fn test_resolve_quick_tier() {
        let router = JudgeModelRouter::new();

        let (provider, model) = router.resolve_tier("quick").unwrap();
        assert_eq!(provider, "opencode-go");
        assert_eq!(model, "minimax-m2.5");
    }

    #[test]
    fn test_resolve_deep_tier() {
        let router = JudgeModelRouter::new();

        let (provider, model) = router.resolve_tier("deep").unwrap();
        assert_eq!(provider, "opencode-go");
        assert_eq!(model, "glm-5");
    }

    #[test]
    fn test_resolve_tiebreaker_tier() {
        let router = JudgeModelRouter::new();

        let (provider, model) = router.resolve_tier("tiebreaker").unwrap();
        assert_eq!(provider, "kimi-for-coding");
        assert_eq!(model, "k2p5");
    }

    #[test]
    fn test_resolve_oracle_tier() {
        let router = JudgeModelRouter::new();

        let (provider, model) = router.resolve_tier("oracle").unwrap();
        assert_eq!(provider, "claude-code");
        assert_eq!(model, "opus-4-6");
    }

    #[test]
    fn test_unknown_tier_error() {
        let router = JudgeModelRouter::new();

        let result = router.resolve_tier("unknown");
        assert!(result.is_err());
        match result {
            Err(JudgeError::UnknownTier(tier)) => assert_eq!(tier, "unknown"),
            _ => panic!("Expected UnknownTier error"),
        }
    }

    #[test]
    fn test_resolve_default_profile() {
        let router = JudgeModelRouter::new();

        let tiers = router.resolve_profile("default").unwrap();
        assert_eq!(tiers.len(), 1);
        assert_eq!(tiers[0].0, "opencode-go");
        assert_eq!(tiers[0].1, "minimax-m2.5");
    }

    #[test]
    fn test_resolve_thorough_profile() {
        let router = JudgeModelRouter::new();

        let tiers = router.resolve_profile("thorough").unwrap();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].0, "opencode-go");
        assert_eq!(tiers[0].1, "minimax-m2.5");
        assert_eq!(tiers[1].0, "opencode-go");
        assert_eq!(tiers[1].1, "glm-5");
    }

    #[test]
    fn test_resolve_critical_profile() {
        let router = JudgeModelRouter::new();

        let tiers = router.resolve_profile("critical").unwrap();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].1, "glm-5");
        assert_eq!(tiers[1].1, "k2p5");
    }

    #[test]
    fn test_resolve_exhaustive_profile() {
        let router = JudgeModelRouter::new();

        let tiers = router.resolve_profile("exhaustive").unwrap();
        assert_eq!(tiers.len(), 4);
        assert_eq!(tiers[0].1, "minimax-m2.5");
        assert_eq!(tiers[1].1, "glm-5");
        assert_eq!(tiers[2].1, "k2p5");
        assert_eq!(tiers[3].1, "opus-4-6");
    }

    #[test]
    fn test_unknown_profile_error() {
        let router = JudgeModelRouter::new();

        let result = router.resolve_profile("nonexistent");
        assert!(result.is_err());
        match result {
            Err(JudgeError::UnknownProfile(profile)) => assert_eq!(profile, "nonexistent"),
            _ => panic!("Expected UnknownProfile error"),
        }
    }

    #[test]
    fn test_available_tiers() {
        let router = JudgeModelRouter::new();
        let tiers = router.available_tiers();

        assert_eq!(tiers.len(), 4);
        assert!(tiers.contains(&"quick"));
        assert!(tiers.contains(&"deep"));
        assert!(tiers.contains(&"tiebreaker"));
        assert!(tiers.contains(&"oracle"));
    }

    #[test]
    fn test_available_profiles() {
        let router = JudgeModelRouter::new();
        let profiles = router.available_profiles();

        assert!(profiles.contains(&&"default".to_string()));
        assert!(profiles.contains(&&"thorough".to_string()));
        assert!(profiles.contains(&&"critical".to_string()));
        assert!(profiles.contains(&&"exhaustive".to_string()));
    }

    #[test]
    fn test_tier_config_creation() {
        let config = TierConfig::new("test-provider".to_string(), "test-model".to_string());

        assert_eq!(config.provider, "test-provider");
        assert_eq!(config.model, "test-model");

        let (provider, model) = config.as_tuple();
        assert_eq!(provider, "test-provider");
        assert_eq!(model, "test-model");
    }

    #[test]
    fn test_default_config() {
        let config = ModelMappingConfig::default();

        assert_eq!(config.quick.provider, "opencode-go");
        assert_eq!(config.quick.model, "minimax-m2.5");
        assert_eq!(config.deep.provider, "opencode-go");
        assert_eq!(config.deep.model, "glm-5");
        assert_eq!(config.tiebreaker.provider, "kimi-for-coding");
        assert_eq!(config.tiebreaker.model, "k2p5");
        assert_eq!(config.oracle.provider, "claude-code");
        assert_eq!(config.oracle.model, "opus-4-6");

        assert!(config.profiles.contains_key("default"));
        assert!(config.profiles.contains_key("thorough"));
    }

    #[test]
    fn test_custom_profile_resolution() {
        let config_json = create_test_config();
        let mut temp_file = tempfile::NamedTempFile::new().unwrap();
        temp_file.write_all(config_json.as_bytes()).unwrap();

        let router = JudgeModelRouter::from_config(temp_file.path()).unwrap();

        // Test custom profile with non-standard tier sequence
        let tiers = router.resolve_profile("custom").unwrap();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].1, "test-quick");
        assert_eq!(tiers[1].1, "test-oracle-model");
    }
}
