//! Provider registry for loading and managing providers from markdown files.
//!
//! This module loads providers from markdown files with YAML frontmatter,
//! similar to how Jekyll, Hugo, and other static site generators work.

use crate::types::RoutingError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use terraphim_types::capability::{Capability, CostLevel, Latency, Provider, ProviderType};

#[cfg(feature = "persistence")]
use async_trait::async_trait;
#[cfg(feature = "persistence")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "persistence")]
use terraphim_persistence::Persistable;

/// Registry of capability providers loaded from markdown files
#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    providers: HashMap<String, Provider>,
    source_path: Option<PathBuf>,
}

/// Parsed markdown file with YAML frontmatter
#[derive(Debug, Clone)]
pub struct MarkdownProvider {
    /// YAML frontmatter as structured data
    pub frontmatter: ProviderFrontmatter,
    /// Markdown body content
    pub body: String,
    /// Source file path
    pub path: PathBuf,
}

/// YAML frontmatter structure for provider configuration
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProviderFrontmatter {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub provider_type: String,
    /// For LLM providers
    pub model_id: Option<String>,
    pub api_endpoint: Option<String>,
    /// For Agent providers
    pub agent_id: Option<String>,
    pub cli_command: Option<String>,
    pub working_dir: Option<PathBuf>,
    /// Common fields
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub cost: String,
    #[serde(default)]
    pub latency: String,
    pub keywords: Vec<String>,
}

impl ProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            source_path: None,
        }
    }

    /// Create with a source path for loading
    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self {
            providers: HashMap::new(),
            source_path: Some(path.into()),
        }
    }

    /// Add a provider to the registry
    pub fn add_provider(&mut self, provider: Provider) {
        self.providers.insert(provider.id.clone(), provider);
    }

    /// Get a provider by ID
    pub fn get(&self, id: &str) -> Option<&Provider> {
        self.providers.get(id)
    }

    /// Get all providers
    pub fn all(&self) -> Vec<&Provider> {
        self.providers.values().collect()
    }

    /// Find providers that have a specific capability
    pub fn find_by_capability(&self, capability: &Capability) -> Vec<&Provider> {
        self.providers
            .values()
            .filter(|p| p.has_capability(capability))
            .collect()
    }

    /// Find providers that match any of the given capabilities
    pub fn find_by_capabilities(&self, capabilities: &[Capability]) -> Vec<&Provider> {
        self.providers
            .values()
            .filter(|p| capabilities.iter().any(|c| p.has_capability(c)))
            .collect()
    }

    /// Load providers from a directory of markdown files
    pub async fn load_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<usize, RoutingError> {
        let dir = dir.as_ref();
        let _span = tracing::info_span!("router.registry.load", directory = ?dir).entered();
        let mut count = 0;

        // Read directory entries
        let mut entries = tokio::fs::read_dir(dir)
            .await
            .map_err(|e| RoutingError::Io(e.to_string()))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| RoutingError::Io(e.to_string()))?
        {
            let path = entry.path();

            // Only process .md files
            if path.extension().map(|e| e == "md").unwrap_or(false) {
                match Self::load_markdown_file(&path).await {
                    Ok(markdown) => match Self::provider_from_markdown(markdown) {
                        Ok(provider) => {
                            self.add_provider(provider);
                            count += 1;
                        }
                        Err(e) => {
                            tracing::warn!(
                                file_path = ?path,
                                error = %e,
                                "Failed to parse provider from markdown file"
                            );
                        }
                    },
                    Err(e) => {
                        tracing::warn!(
                            file_path = ?path,
                            error = %e,
                            "Failed to load markdown file"
                        );
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load a single markdown file with YAML frontmatter
    async fn load_markdown_file(path: &Path) -> Result<MarkdownProvider, RoutingError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| RoutingError::Io(e.to_string()))?;

        Self::parse_markdown(content, path.to_path_buf())
    }

    /// Parse markdown content with YAML frontmatter
    fn parse_markdown(content: String, path: PathBuf) -> Result<MarkdownProvider, RoutingError> {
        // Check for YAML frontmatter (starts with ---)
        if !content.starts_with("---") {
            return Err(RoutingError::RegistryError(format!(
                "No YAML frontmatter found in {:?}",
                path
            )));
        }

        // Find the end of frontmatter (second ---)
        let rest = &content[3..]; // Skip first ---
        let Some(end_pos) = rest.find("---") else {
            return Err(RoutingError::RegistryError(format!(
                "Unclosed YAML frontmatter in {:?}",
                path
            )));
        };

        let yaml_content = &rest[..end_pos];
        let body = &rest[end_pos + 3..];

        // Parse YAML frontmatter
        let frontmatter: ProviderFrontmatter = serde_yaml::from_str(yaml_content).map_err(|e| {
            RoutingError::Serialization(format!("Failed to parse YAML frontmatter: {}", e))
        })?;

        Ok(MarkdownProvider {
            frontmatter,
            body: body.trim().to_string(),
            path,
        })
    }

    /// Convert MarkdownProvider to Provider
    fn provider_from_markdown(markdown: MarkdownProvider) -> Result<Provider, RoutingError> {
        let fm = markdown.frontmatter;

        // Parse provider type
        let provider_type = match fm.provider_type.as_str() {
            "llm" => {
                let model_id = fm.model_id.ok_or_else(|| {
                    RoutingError::RegistryError("LLM provider missing model_id".to_string())
                })?;
                let api_endpoint = fm
                    .api_endpoint
                    .unwrap_or_else(|| "https://api.openai.com/v1".to_string());
                ProviderType::Llm {
                    model_id,
                    api_endpoint,
                }
            }
            "agent" => {
                let agent_id = fm.agent_id.clone().ok_or_else(|| {
                    RoutingError::RegistryError("Agent provider missing agent_id".to_string())
                })?;
                let cli_command = fm.cli_command.ok_or_else(|| {
                    RoutingError::RegistryError("Agent provider missing cli_command".to_string())
                })?;
                let working_dir = fm.working_dir.unwrap_or_else(|| PathBuf::from("/tmp"));
                ProviderType::Agent {
                    agent_id,
                    cli_command,
                    working_dir,
                }
            }
            other => {
                return Err(RoutingError::RegistryError(format!(
                    "Unknown provider type: {}",
                    other
                )));
            }
        };

        // Parse capabilities from strings
        let capabilities = fm
            .capabilities
            .iter()
            .filter_map(|c| Self::parse_capability(c))
            .collect();

        // Parse cost level
        let cost_level = match fm.cost.to_lowercase().as_str() {
            "cheap" | "low" => CostLevel::Cheap,
            "expensive" | "high" => CostLevel::Expensive,
            _ => CostLevel::Moderate,
        };

        // Parse latency
        let latency = match fm.latency.to_lowercase().as_str() {
            "fast" | "quick" => Latency::Fast,
            "slow" => Latency::Slow,
            _ => Latency::Medium,
        };

        Ok(Provider {
            id: fm.id,
            name: fm.name,
            provider_type,
            capabilities,
            cost_level,
            latency,
            keywords: fm.keywords,
        })
    }

    /// Parse capability from string
    fn parse_capability(s: &str) -> Option<Capability> {
        match s.to_lowercase().replace("-", "_").as_str() {
            "deep_thinking" | "deepthinking" => Some(Capability::DeepThinking),
            "fast_thinking" | "fastthinking" => Some(Capability::FastThinking),
            "code_generation" | "codegeneration" => Some(Capability::CodeGeneration),
            "code_review" | "codereview" => Some(Capability::CodeReview),
            "architecture" => Some(Capability::Architecture),
            "testing" => Some(Capability::Testing),
            "refactoring" => Some(Capability::Refactoring),
            "documentation" => Some(Capability::Documentation),
            "explanation" => Some(Capability::Explanation),
            "security_audit" | "securityaudit" => Some(Capability::SecurityAudit),
            "performance" => Some(Capability::Performance),
            _ => {
                tracing::warn!(capability_string = s, "Unknown capability string, skipping");
                None
            }
        }
    }

    /// Create a registry that auto-loads from DeviceStorage on startup.
    ///
    /// If no persisted data is found, starts empty. Use
    /// [`add_provider_persisted`] and [`remove_provider_persisted`] to
    /// write-through changes to storage.
    #[cfg(feature = "persistence")]
    pub async fn with_persistence(registry_id: &str) -> Result<Self, RoutingError> {
        let mut persisted = PersistedProviderRegistry::new(registry_id.to_string());

        let providers = match persisted.load().await {
            Ok(loaded) => {
                tracing::info!(
                    registry_id = registry_id,
                    count = loaded.providers.len(),
                    "Loaded providers from persistent storage"
                );
                loaded.providers
            }
            Err(_) => {
                tracing::debug!(
                    registry_id = registry_id,
                    "No persisted providers found, starting empty"
                );
                HashMap::new()
            }
        };

        Ok(Self {
            providers,
            source_path: None,
        })
    }

    /// Add a provider and persist the full registry to storage.
    #[cfg(feature = "persistence")]
    pub async fn add_provider_persisted(
        &mut self,
        provider: Provider,
        registry_id: &str,
    ) -> Result<(), RoutingError> {
        self.providers.insert(provider.id.clone(), provider);
        self.persist_all(registry_id).await
    }

    /// Remove a provider and persist the change to storage.
    #[cfg(feature = "persistence")]
    pub async fn remove_provider_persisted(
        &mut self,
        provider_id: &str,
        registry_id: &str,
    ) -> Result<Option<Provider>, RoutingError> {
        let removed = self.providers.remove(provider_id);
        self.persist_all(registry_id).await?;
        Ok(removed)
    }

    /// Write the full registry state to all storage backends.
    #[cfg(feature = "persistence")]
    async fn persist_all(&self, registry_id: &str) -> Result<(), RoutingError> {
        let persisted = PersistedProviderRegistry {
            registry_id: registry_id.to_string(),
            providers: self.providers.clone(),
        };
        persisted
            .save()
            .await
            .map_err(|e| RoutingError::RegistryError(format!("Persistence error: {}", e)))
    }

    /// Load from default location (~/.terraphim/providers/)
    pub async fn load_default() -> Result<Self, RoutingError> {
        let mut registry = Self::new();

        let home = dirs::home_dir()
            .ok_or_else(|| RoutingError::Io("Could not find home directory".to_string()))?;

        let providers_dir = home.join(".terraphim").join("providers");

        if providers_dir.exists() {
            registry.load_from_dir(&providers_dir).await?;
        }

        Ok(registry)
    }
}

/// Serializable snapshot of the provider registry for persistence.
///
/// Stored as a single JSON document in DeviceStorage. The `registry_id`
/// is used to derive the storage key.
#[cfg(feature = "persistence")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedProviderRegistry {
    pub registry_id: String,
    pub providers: HashMap<String, Provider>,
}

#[cfg(feature = "persistence")]
#[async_trait]
impl Persistable for PersistedProviderRegistry {
    fn new(key: String) -> Self {
        Self {
            registry_id: key,
            providers: HashMap::new(),
        }
    }

    async fn save(&self) -> terraphim_persistence::Result<()> {
        self.save_to_all().await
    }

    async fn save_to_one(&self, profile_name: &str) -> terraphim_persistence::Result<()> {
        self.save_to_profile(profile_name).await
    }

    async fn load(&mut self) -> terraphim_persistence::Result<Self>
    where
        Self: Sized,
    {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        self.load_from_operator(&key, op).await
    }

    fn get_key(&self) -> String {
        let normalized = self.normalize_key(&self.registry_id);
        format!("provider_registry_{}.json", normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_markdown_with_frontmatter() {
        let content = r#"---
id: "claude-opus"
name: "Claude Opus"
type: "llm"
model_id: "claude-3-opus-20240229"
api_endpoint: "https://api.anthropic.com/v1"
capabilities:
  - deep-thinking
  - code-generation
cost: expensive
latency: slow
keywords:
  - think
  - reasoning
---

# Claude Opus

Anthropic's most capable model.
"#;

        let markdown =
            ProviderRegistry::parse_markdown(content.to_string(), PathBuf::from("test.md"))
                .unwrap();

        assert_eq!(markdown.frontmatter.id, "claude-opus");
        assert_eq!(markdown.frontmatter.provider_type, "llm");
        assert_eq!(markdown.frontmatter.capabilities.len(), 2);
        assert!(markdown.body.contains("Anthropic's most capable model"));
    }

    #[test]
    fn test_provider_from_markdown_llm() {
        let markdown = MarkdownProvider {
            frontmatter: ProviderFrontmatter {
                id: "test-llm".to_string(),
                name: "Test LLM".to_string(),
                provider_type: "llm".to_string(),
                model_id: Some("gpt-4".to_string()),
                api_endpoint: Some("https://api.openai.com".to_string()),
                agent_id: None,
                cli_command: None,
                working_dir: None,
                capabilities: vec!["code-generation".to_string()],
                cost: "moderate".to_string(),
                latency: "medium".to_string(),
                keywords: vec!["code".to_string()],
            },
            body: "Test body".to_string(),
            path: PathBuf::from("test.md"),
        };

        let provider = ProviderRegistry::provider_from_markdown(markdown).unwrap();

        assert_eq!(provider.id, "test-llm");
        assert!(provider.has_capability(&Capability::CodeGeneration));
        assert_eq!(provider.cost_level, CostLevel::Moderate);
    }

    #[test]
    fn test_provider_from_markdown_agent() {
        let markdown = MarkdownProvider {
            frontmatter: ProviderFrontmatter {
                id: "@coder".to_string(),
                name: "Coder Agent".to_string(),
                provider_type: "agent".to_string(),
                model_id: None,
                api_endpoint: None,
                agent_id: Some("@coder".to_string()),
                cli_command: Some("opencode".to_string()),
                working_dir: Some(PathBuf::from("/workspace")),
                capabilities: vec!["code-generation".to_string(), "code-review".to_string()],
                cost: "cheap".to_string(),
                latency: "fast".to_string(),
                keywords: vec!["implement".to_string()],
            },
            body: "Test body".to_string(),
            path: PathBuf::from("test.md"),
        };

        let provider = ProviderRegistry::provider_from_markdown(markdown).unwrap();

        assert_eq!(provider.id, "@coder");
        assert!(matches!(provider.provider_type, ProviderType::Agent { .. }));
        assert_eq!(provider.cost_level, CostLevel::Cheap);
    }

    #[tokio::test]
    async fn test_load_from_dir() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create a test markdown file
        let mut file = NamedTempFile::new_in(temp_dir.path()).unwrap();
        file.write_all(
            r#"---
id: "test-provider"
name: "Test Provider"
type: "llm"
model_id: "test-model"
capabilities:
  - code-generation
cost: cheap
latency: fast
keywords:
  - test
---

# Test Provider

This is a test.
"#
            .as_bytes(),
        )
        .unwrap();

        // Rename to .md extension
        let md_path = temp_dir.path().join("test.md");
        std::fs::rename(file.path(), &md_path).unwrap();

        let mut registry = ProviderRegistry::new();
        let count = registry.load_from_dir(temp_dir.path()).await.unwrap();

        assert_eq!(count, 1);
        assert!(registry.get("test-provider").is_some());
    }

    // Persistence tests -- use real DeviceStorage with memory backend
    #[cfg(feature = "persistence")]
    mod persistence_tests {
        use super::*;
        use serial_test::serial;
        use terraphim_persistence::Persistable;

        async fn init_test_persistence() {
            terraphim_persistence::DeviceStorage::init_memory_only()
                .await
                .expect("Failed to initialize memory-only DeviceStorage");
        }

        fn test_provider(id: &str) -> Provider {
            Provider::new(
                id,
                format!("Test {}", id),
                ProviderType::Llm {
                    model_id: format!("model-{}", id),
                    api_endpoint: "https://api.example.com".to_string(),
                },
                vec![Capability::CodeGeneration, Capability::CodeReview],
            )
        }

        #[tokio::test]
        #[serial]
        async fn test_persist_and_load_empty_registry() {
            init_test_persistence().await;

            let persisted = PersistedProviderRegistry {
                registry_id: "test-empty".to_string(),
                providers: HashMap::new(),
            };
            persisted.save().await.unwrap();

            let mut loaded = PersistedProviderRegistry::new("test-empty".to_string());
            loaded = loaded.load().await.unwrap();

            assert_eq!(loaded.providers.len(), 0);
            assert_eq!(loaded.registry_id, "test-empty");
        }

        #[tokio::test]
        #[serial]
        async fn test_persist_and_load_with_providers() {
            init_test_persistence().await;

            let mut providers = HashMap::new();
            providers.insert("p1".to_string(), test_provider("p1"));
            providers.insert("p2".to_string(), test_provider("p2"));

            let persisted = PersistedProviderRegistry {
                registry_id: "test-with-providers".to_string(),
                providers,
            };
            persisted.save().await.unwrap();

            let mut loaded = PersistedProviderRegistry::new("test-with-providers".to_string());
            loaded = loaded.load().await.unwrap();

            assert_eq!(loaded.providers.len(), 2);
            assert!(loaded.providers.contains_key("p1"));
            assert!(loaded.providers.contains_key("p2"));
            assert_eq!(loaded.providers["p1"].name, "Test p1");
        }

        #[tokio::test]
        #[serial]
        async fn test_with_persistence_loads_existing() {
            init_test_persistence().await;

            // Pre-populate storage
            let mut providers = HashMap::new();
            providers.insert("existing".to_string(), test_provider("existing"));
            let persisted = PersistedProviderRegistry {
                registry_id: "test-load".to_string(),
                providers,
            };
            persisted.save().await.unwrap();

            // Create registry via with_persistence
            let registry = ProviderRegistry::with_persistence("test-load")
                .await
                .unwrap();

            assert_eq!(registry.all().len(), 1);
            assert!(registry.get("existing").is_some());
        }

        #[tokio::test]
        #[serial]
        async fn test_with_persistence_starts_empty_when_no_data() {
            init_test_persistence().await;

            let registry = ProviderRegistry::with_persistence("test-nonexistent")
                .await
                .unwrap();

            assert_eq!(registry.all().len(), 0);
        }

        #[tokio::test]
        #[serial]
        async fn test_add_provider_persisted_roundtrip() {
            init_test_persistence().await;

            let mut registry = ProviderRegistry::with_persistence("test-add")
                .await
                .unwrap();

            registry
                .add_provider_persisted(test_provider("new-provider"), "test-add")
                .await
                .unwrap();

            assert_eq!(registry.all().len(), 1);

            // Verify it was persisted by loading fresh
            let registry2 = ProviderRegistry::with_persistence("test-add")
                .await
                .unwrap();
            assert_eq!(registry2.all().len(), 1);
            assert!(registry2.get("new-provider").is_some());
        }

        #[tokio::test]
        #[serial]
        async fn test_remove_provider_persisted() {
            init_test_persistence().await;

            let mut registry = ProviderRegistry::with_persistence("test-remove")
                .await
                .unwrap();

            registry
                .add_provider_persisted(test_provider("to-remove"), "test-remove")
                .await
                .unwrap();
            assert_eq!(registry.all().len(), 1);

            let removed = registry
                .remove_provider_persisted("to-remove", "test-remove")
                .await
                .unwrap();
            assert!(removed.is_some());
            assert_eq!(registry.all().len(), 0);

            // Verify removal persisted
            let registry2 = ProviderRegistry::with_persistence("test-remove")
                .await
                .unwrap();
            assert_eq!(registry2.all().len(), 0);
        }

        #[tokio::test]
        #[serial]
        async fn test_backward_compat_non_persisted_still_works() {
            // Ensure the existing in-memory API is unaffected
            let mut registry = ProviderRegistry::new();
            registry.add_provider(test_provider("in-memory"));
            assert_eq!(registry.all().len(), 1);
            assert!(registry.get("in-memory").is_some());
        }
    }
}
