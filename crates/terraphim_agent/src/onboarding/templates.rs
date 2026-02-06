//! Template registry for quick start configurations
//!
//! Provides embedded JSON templates for common use cases:
//! - Terraphim Engineer (graph embeddings)
//! - LLM Enforcer (bun install KG)
//! - Rust Developer
//! - Local Notes
//! - AI Engineer
//! - Log Analyst

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use terraphim_automata::AutomataPath;
use terraphim_config::{Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role, ServiceType};
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

/// A pre-built configuration template for quick start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTemplate {
    /// Unique identifier for the template
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Short description of use case
    pub description: String,
    /// Whether this template requires a path parameter
    pub requires_path: bool,
    /// Default path if applicable
    pub default_path: Option<String>,
    /// Whether this template includes LLM configuration
    pub has_llm: bool,
    /// Whether this template includes knowledge graph
    pub has_kg: bool,
}

impl ConfigTemplate {
    /// Build the Role from this template, optionally with a custom path
    pub fn build_role(&self, custom_path: Option<&str>) -> Role {
        match self.id.as_str() {
            "terraphim-engineer" => self.build_terraphim_engineer(custom_path),
            "llm-enforcer" => self.build_llm_enforcer(custom_path),
            "rust-engineer" => self.build_rust_engineer(),
            "local-notes" => self.build_local_notes(custom_path),
            "ai-engineer" => self.build_ai_engineer(custom_path),
            "log-analyst" => self.build_log_analyst(),
            _ => self.build_terraphim_engineer(custom_path), // Default fallback
        }
    }

    fn build_terraphim_engineer(&self, custom_path: Option<&str>) -> Role {
        let location = custom_path
            .map(|s| s.to_string())
            .unwrap_or_else(|| "~/Documents".to_string());

        let mut role = Role::new("Terraphim Engineer");
        role.shortname = Some("terra".to_string());
        role.relevance_function = RelevanceFunction::TerraphimGraph;
        role.terraphim_it = true;
        role.theme = "spacelab".to_string();
        role.kg = Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote(
                "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json".to_string(),
            )),
            knowledge_graph_local: None,
            public: true,
            publish: false,
        });
        role.haystacks = vec![Haystack {
            location,
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role.llm_enabled = false;
        role
    }

    fn build_llm_enforcer(&self, custom_path: Option<&str>) -> Role {
        let kg_path = custom_path
            .map(|s| s.to_string())
            .unwrap_or_else(|| "docs/src/kg".to_string());

        let mut role = Role::new("LLM Enforcer");
        role.shortname = Some("enforce".to_string());
        role.relevance_function = RelevanceFunction::TitleScorer;
        role.terraphim_it = true;
        role.theme = "darkly".to_string();
        role.kg = Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from(kg_path),
            }),
            public: false,
            publish: false,
        });
        role.haystacks = vec![Haystack {
            location: ".".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role.llm_enabled = false;
        role
    }

    fn build_rust_engineer(&self) -> Role {
        let mut role = Role::new("Rust Engineer");
        role.shortname = Some("rust".to_string());
        role.relevance_function = RelevanceFunction::TitleScorer;
        role.terraphim_it = false;
        role.theme = "cosmo".to_string();
        role.kg = None;
        role.haystacks = vec![Haystack {
            location: "https://query.rs".to_string(),
            service: ServiceType::QueryRs,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role.llm_enabled = false;
        role
    }

    fn build_local_notes(&self, custom_path: Option<&str>) -> Role {
        let location = custom_path
            .map(|s| s.to_string())
            .unwrap_or_else(|| ".".to_string());

        let mut role = Role::new("Local Notes");
        role.shortname = Some("notes".to_string());
        role.relevance_function = RelevanceFunction::TitleScorer;
        role.terraphim_it = false;
        role.theme = "lumen".to_string();
        role.kg = None;
        role.haystacks = vec![Haystack {
            location,
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role.llm_enabled = false;
        role
    }

    fn build_ai_engineer(&self, custom_path: Option<&str>) -> Role {
        let location = custom_path
            .map(|s| s.to_string())
            .unwrap_or_else(|| "~/Documents".to_string());

        let mut role = Role::new("AI Engineer");
        role.shortname = Some("ai".to_string());
        role.relevance_function = RelevanceFunction::TerraphimGraph;
        role.terraphim_it = true;
        role.theme = "united".to_string();
        role.kg = Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::Remote(
                "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json".to_string(),
            )),
            knowledge_graph_local: None,
            public: true,
            publish: false,
        });
        role.haystacks = vec![Haystack {
            location,
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        // AI Engineer has Ollama LLM configured
        role.llm_enabled = true;
        role.extra.insert(
            "llm_provider".to_string(),
            serde_json::Value::String("ollama".to_string()),
        );
        role.extra.insert(
            "ollama_base_url".to_string(),
            serde_json::Value::String("http://127.0.0.1:11434".to_string()),
        );
        role.extra.insert(
            "ollama_model".to_string(),
            serde_json::Value::String("llama3.2:3b".to_string()),
        );
        role
    }

    fn build_log_analyst(&self) -> Role {
        let mut role = Role::new("Log Analyst");
        role.shortname = Some("logs".to_string());
        role.relevance_function = RelevanceFunction::BM25;
        role.terraphim_it = false;
        role.theme = "darkly".to_string();
        role.kg = None;
        role.haystacks = vec![Haystack {
            location: "http://localhost:7280".to_string(),
            service: ServiceType::Quickwit,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: Default::default(),
        }];
        role.llm_enabled = false;
        role
    }
}

/// Registry of all available templates
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: Vec<ConfigTemplate>,
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateRegistry {
    /// Create a new registry with all embedded templates
    pub fn new() -> Self {
        let templates = vec![
            ConfigTemplate {
                id: "terraphim-engineer".to_string(),
                name: "Terraphim Engineer".to_string(),
                description: "Full-featured semantic search with knowledge graph embeddings"
                    .to_string(),
                requires_path: false,
                default_path: Some("~/Documents".to_string()),
                has_llm: false,
                has_kg: true,
            },
            ConfigTemplate {
                id: "llm-enforcer".to_string(),
                name: "LLM Enforcer".to_string(),
                description: "AI agent hooks with bun install knowledge graph for npm replacement"
                    .to_string(),
                requires_path: false,
                default_path: Some("docs/src/kg".to_string()),
                has_llm: false,
                has_kg: true,
            },
            ConfigTemplate {
                id: "rust-engineer".to_string(),
                name: "Rust Developer".to_string(),
                description: "Search Rust docs and crates.io via QueryRs".to_string(),
                requires_path: false,
                default_path: None,
                has_llm: false,
                has_kg: false,
            },
            ConfigTemplate {
                id: "local-notes".to_string(),
                name: "Local Notes".to_string(),
                description: "Search markdown files in a local folder".to_string(),
                requires_path: true,
                default_path: None,
                has_llm: false,
                has_kg: false,
            },
            ConfigTemplate {
                id: "ai-engineer".to_string(),
                name: "AI Engineer".to_string(),
                description: "Local Ollama LLM with knowledge graph support".to_string(),
                requires_path: false,
                default_path: Some("~/Documents".to_string()),
                has_llm: true,
                has_kg: true,
            },
            ConfigTemplate {
                id: "log-analyst".to_string(),
                name: "Log Analyst".to_string(),
                description: "Quickwit integration for log analysis".to_string(),
                requires_path: false,
                default_path: None,
                has_llm: false,
                has_kg: false,
            },
        ];

        Self { templates }
    }

    /// Get a template by its ID
    pub fn get(&self, id: &str) -> Option<&ConfigTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    /// List all available templates
    pub fn list(&self) -> &[ConfigTemplate] {
        &self.templates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_registry_has_terraphim_engineer() {
        let registry = TemplateRegistry::new();
        let template = registry.get("terraphim-engineer");
        assert!(template.is_some());
        let t = template.unwrap();
        assert_eq!(t.name, "Terraphim Engineer");
        assert!(t.has_kg);
    }

    #[test]
    fn test_template_registry_has_llm_enforcer() {
        let registry = TemplateRegistry::new();
        let template = registry.get("llm-enforcer");
        assert!(template.is_some());
        let t = template.unwrap();
        assert_eq!(t.name, "LLM Enforcer");
        assert!(t.has_kg);
        assert_eq!(t.default_path, Some("docs/src/kg".to_string()));
    }

    #[test]
    fn test_template_registry_has_all_six_templates() {
        let registry = TemplateRegistry::new();
        assert_eq!(registry.list().len(), 6);

        assert!(registry.get("terraphim-engineer").is_some());
        assert!(registry.get("llm-enforcer").is_some());
        assert!(registry.get("rust-engineer").is_some());
        assert!(registry.get("local-notes").is_some());
        assert!(registry.get("ai-engineer").is_some());
        assert!(registry.get("log-analyst").is_some());
    }

    #[test]
    fn test_local_notes_requires_path() {
        let registry = TemplateRegistry::new();
        let template = registry.get("local-notes").unwrap();
        assert!(template.requires_path);
    }

    #[test]
    fn test_build_terraphim_engineer_role() {
        let registry = TemplateRegistry::new();
        let template = registry.get("terraphim-engineer").unwrap();
        let role = template.build_role(None);

        assert_eq!(role.name.to_string(), "Terraphim Engineer");
        assert_eq!(role.shortname, Some("terra".to_string()));
        assert_eq!(role.relevance_function, RelevanceFunction::TerraphimGraph);
        assert!(role.kg.is_some());
        assert!(!role.haystacks.is_empty());
    }

    #[test]
    fn test_build_terraphim_engineer_with_custom_path() {
        let registry = TemplateRegistry::new();
        let template = registry.get("terraphim-engineer").unwrap();
        let role = template.build_role(Some("/custom/path"));

        assert_eq!(role.haystacks[0].location, "/custom/path");
    }

    #[test]
    fn test_build_llm_enforcer_has_local_kg() {
        let registry = TemplateRegistry::new();
        let template = registry.get("llm-enforcer").unwrap();
        let role = template.build_role(None);

        assert!(role.kg.is_some());
        let kg = role.kg.unwrap();
        assert!(kg.knowledge_graph_local.is_some());
        assert!(kg.automata_path.is_none());
    }

    #[test]
    fn test_build_ai_engineer_has_ollama() {
        let registry = TemplateRegistry::new();
        let template = registry.get("ai-engineer").unwrap();
        let role = template.build_role(None);

        assert!(role.llm_enabled);
        assert!(role.extra.contains_key("llm_provider"));
        assert!(role.extra.contains_key("ollama_model"));
    }
}
