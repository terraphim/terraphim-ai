#![warn(missing_docs)]
//! Configuration management for Terraphim AI.
//!
//! Provides role-based configuration where each [`Role`] describes a user profile with
//! a set of [`Haystack`]s (data sources), a relevance function, and optional LLM settings.
//!
//! # Loading Priority
//!
//! 1. Explicit path via `TERRAPHIM_CONFIG` environment variable
//! 2. Saved config retrieved from the persistence layer
//! 3. Hard-coded defaults in `terraphim_server/default/`
//!
//! # Key Types
//!
//! - [`Config`] -- top-level configuration holding all roles
//! - [`Role`] -- user profile with haystacks, relevance function, and theme
//! - [`Haystack`] -- a data source descriptor (path, service type, extra parameters)
//! - [`ServiceType`] -- enum of supported haystack backends

use std::{path::PathBuf, sync::Arc};

use terraphim_automata::{
    AutomataPath,
    builder::{Logseq, ThesaurusBuilder},
    load_thesaurus, parse_markdown_directives_dir,
};
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, IndexedDocument, KnowledgeGraphInputType, RelevanceFunction, RoleName, SearchQuery,
};

use terraphim_settings::DeviceSettings;

use ahash::AHashMap;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;
#[cfg(feature = "typescript")]
use tsify::Tsify;

use crate::llm_router::LlmRouterConfig;

// LLM Router configuration
pub mod llm_router;

/// Project-level configuration discovery for `.terraphim/config.json` files.
pub mod project;

/// Convenience alias for `Result<T, TerraphimConfigError>` used throughout this crate.
pub type Result<T> = std::result::Result<T, TerraphimConfigError>;

use opendal::Result as OpendalResult;

type PersistenceResult<T> = std::result::Result<T, terraphim_persistence::Error>;

/// Errors arising from loading, validating, or persisting Terraphim configuration.
#[derive(Error, Debug)]
pub enum TerraphimConfigError {
    /// No configuration file was found at the expected location.
    #[error("Unable to load config")]
    NotFound,

    /// Configuration contains no role definitions; at least one role is required.
    #[error("At least one role is required")]
    NoRoles,

    /// A named profile was requested but could not be applied.
    #[error("Profile error")]
    Profile(String),

    /// An error from the underlying persistence layer.
    #[error("Persistence error")]
    Persistence(Box<terraphim_persistence::Error>),

    /// JSON serialisation or deserialisation failed.
    #[error("Serde JSON error")]
    Json(#[from] serde_json::Error),

    /// Failed to initialise the tracing subscriber.
    #[error("Cannot initialize tracing subscriber")]
    TracingSubscriber(Box<dyn std::error::Error + Send + Sync>),

    /// An error propagated from the rolegraph pipeline.
    #[error("Pipe error")]
    Pipe(#[from] terraphim_rolegraph::Error),

    /// An error from the Aho-Corasick automata layer.
    #[error("Automata error")]
    Automata(#[from] terraphim_automata::TerraphimAutomataError),

    /// A URL could not be parsed.
    #[error("Url error")]
    Url(#[from] url::ParseError),

    /// An I/O error occurred while reading or writing configuration.
    #[error("IO error")]
    Io(#[from] std::io::Error),

    /// A general configuration error with a descriptive message.
    #[error("Config error")]
    Config(String),
}

impl From<terraphim_persistence::Error> for TerraphimConfigError {
    fn from(error: terraphim_persistence::Error) -> Self {
        TerraphimConfigError::Persistence(Box::new(error))
    }
}

/// Expand shell-like variables in a path string.
///
/// Supports:
/// - `${HOME}` or `$HOME` -> user's home directory
/// - `${TERRAPHIM_DATA_PATH:-default}` -> environment variable with default value
/// - `~` at the start -> user's home directory
pub fn expand_path(path: &str) -> PathBuf {
    let mut result = path.to_string();

    /// Get home directory using multiple fallback strategies
    fn get_home_dir() -> Option<PathBuf> {
        // Try dirs crate first
        if let Some(home) = dirs::home_dir() {
            return Some(home);
        }
        // Fallback to HOME environment variable
        if let Ok(home) = std::env::var("HOME") {
            return Some(PathBuf::from(home));
        }
        // Fallback to USERPROFILE on Windows
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Some(PathBuf::from(profile));
        }
        None
    }

    // Handle ${VAR:-default} syntax (environment variable with default)
    // This regex handles nested ${...} in the default value by using a greedy match
    // that captures everything until the last }
    loop {
        // Find ${VAR:-...} pattern manually to handle nested braces
        if let Some(start) = result.find("${") {
            if let Some(colon_pos) = result[start..].find(":-") {
                let colon_pos = start + colon_pos;
                // Find the variable name
                let var_name = &result[start + 2..colon_pos];
                // Find the matching closing brace by counting braces
                let after_colon = colon_pos + 2;
                let mut depth = 1;
                let mut end_pos = after_colon;
                for (i, c) in result[after_colon..].char_indices() {
                    match c {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = after_colon + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                if depth == 0 {
                    let default_value = &result[after_colon..end_pos];
                    let replacement =
                        std::env::var(var_name).unwrap_or_else(|_| default_value.to_string());
                    result = format!(
                        "{}{}{}",
                        &result[..start],
                        replacement,
                        &result[end_pos + 1..]
                    );
                    continue; // Process again in case there are more patterns
                }
            }
        }
        break;
    }

    // Handle ${VAR} syntax
    let re_braces = regex::Regex::new(r"\$\{([^}]+)\}").unwrap();
    result = re_braces
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if var_name == "HOME" {
                get_home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("${{{}}}", var_name))
            } else {
                std::env::var(var_name).unwrap_or_else(|_| format!("${{{}}}", var_name))
            }
        })
        .to_string();

    // Handle $VAR syntax (without braces)
    let re_dollar = regex::Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();
    result = re_dollar
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            if var_name == "HOME" {
                get_home_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("${}", var_name))
            } else {
                std::env::var(var_name).unwrap_or_else(|_| format!("${}", var_name))
            }
        })
        .to_string();

    // Handle ~ at the beginning of the path
    if result.starts_with('~') {
        if let Some(home) = get_home_dir() {
            result = result.replacen('~', &home.to_string_lossy(), 1);
        }
    }

    PathBuf::from(result)
}

/// Default context window size for LLM requests
fn default_context_window() -> Option<u64> {
    Some(32768)
}

/// A role is a collection of settings for a specific user
///
/// It contains a user's knowledge graph, a list of haystacks, as
/// well as preferences for the relevance function and theme
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Default)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Role {
    /// Optional short alias used for command-line role selection.
    pub shortname: Option<String>,
    /// Primary name that uniquely identifies this role.
    pub name: RoleName,
    /// The relevance function used to rank search results
    pub relevance_function: RelevanceFunction,
    /// Whether Terraphim-specific post-processing is applied to search results.
    pub terraphim_it: bool,
    /// UI theme name applied when this role is active.
    pub theme: String,
    /// Optional knowledge graph configuration for semantic search.
    pub kg: Option<KnowledgeGraph>,
    /// Haystack data sources searched under this role.
    pub haystacks: Vec<Haystack>,
    /// Enable AI-powered article summaries using LLM providers
    #[serde(default)]
    pub llm_enabled: bool,
    /// API key for LLM service
    #[serde(default)]
    pub llm_api_key: Option<String>,
    /// Model to use for generating summaries (e.g., "openai/gpt-3.5-turbo", "gemma3:270m")
    #[serde(default)]
    pub llm_model: Option<String>,
    /// Automatically summarize search results using LLM
    #[serde(default)]
    pub llm_auto_summarize: bool,
    /// Enable Chat interface backed by LLM
    #[serde(default)]
    pub llm_chat_enabled: bool,
    /// Optional system prompt to use for chat conversations
    #[serde(default)]
    pub llm_chat_system_prompt: Option<String>,
    /// Optional chat model override (falls back to llm_model)
    #[serde(default)]
    pub llm_chat_model: Option<String>,
    /// Maximum tokens for LLM context window (default: 32768)
    #[serde(default = "default_context_window")]
    pub llm_context_window: Option<u64>,
    /// Arbitrary provider-specific or experiment-specific key/value pairs.
    #[serde(flatten)]
    #[schemars(skip)]
    #[cfg_attr(feature = "typescript", tsify(type = "Record<string, unknown>"))]
    pub extra: AHashMap<String, Value>,
    /// Enable intelligent LLM routing with 6-phase architecture
    #[serde(default)]
    pub llm_router_enabled: bool,
    /// Configuration for intelligent routing behavior
    #[serde(default)]
    pub llm_router_config: Option<LlmRouterConfig>,
}

impl std::fmt::Debug for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Role")
            .field("shortname", &self.shortname)
            .field("name", &self.name)
            .field("relevance_function", &self.relevance_function)
            .field("terraphim_it", &self.terraphim_it)
            .field("theme", &self.theme)
            .field("kg", &self.kg)
            .field("haystacks", &self.haystacks)
            .field("llm_enabled", &self.llm_enabled)
            .field(
                "llm_api_key",
                &self.llm_api_key.as_ref().map(|_| "***REDACTED***"),
            )
            .field("llm_model", &self.llm_model)
            .field("llm_auto_summarize", &self.llm_auto_summarize)
            .field("llm_chat_enabled", &self.llm_chat_enabled)
            .field("llm_chat_system_prompt", &self.llm_chat_system_prompt)
            .field("llm_chat_model", &self.llm_chat_model)
            .field("llm_context_window", &self.llm_context_window)
            .field("llm_router_enabled", &self.llm_router_enabled)
            .field("llm_router_config", &self.llm_router_config)
            .finish_non_exhaustive()
    }
}

impl Role {
    /// Create a new Role with default values for all fields
    pub fn new(name: impl Into<RoleName>) -> Self {
        Self {
            shortname: None,
            name: name.into(),
            relevance_function: RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "default".to_string(),
            kg: None,
            haystacks: vec![],
            llm_enabled: false,
            llm_api_key: None,
            llm_model: None,
            llm_auto_summarize: false,
            llm_chat_enabled: false,
            llm_chat_system_prompt: None,
            llm_chat_model: None,
            llm_context_window: default_context_window(),
            extra: AHashMap::new(),
            llm_router_enabled: false,
            llm_router_config: None,
        }
    }

    /// Check if LLM is properly configured for this role
    pub fn has_llm_config(&self) -> bool {
        self.llm_enabled && self.llm_api_key.is_some() && self.llm_model.is_some()
    }

    /// Get the LLM model name, providing a sensible default
    pub fn get_llm_model(&self) -> Option<&str> {
        self.llm_model.as_deref()
    }
}

use anyhow::Context;
/// The service used for indexing documents
///
/// Each service assumes documents to be stored in a specific format
/// and uses a specific indexing algorithm
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum ServiceType {
    /// Use ripgrep as the indexing service
    Ripgrep,
    /// Use an Atomic Server as the indexing service
    Atomic,
    /// Use query.rs as the indexing service
    QueryRs,
    /// Use ClickUp API as the indexing service
    ClickUp,
    /// Use an MCP client to query a Model Context Protocol server
    Mcp,
    /// Use Perplexity AI-powered web search for indexing
    Perplexity,
    /// Use grep.app for searching code across GitHub repositories
    GrepApp,
    /// Use AI coding assistant session logs (Claude Code, OpenCode, Cursor, Aider, Codex)
    AiAssistant,
    /// Use Quickwit search engine for log and observability data indexing
    Quickwit,
    /// Use JMAP protocol for email search (RFC 8620/8621)
    Jmap,
}

/// A haystack is a collection of documents that can be indexed and searched
///
/// One user can have multiple haystacks
/// Each haystack is indexed using a specific service
#[derive(Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Haystack {
    /// The location of the haystack - can be a filesystem path or URL
    pub location: String,
    /// The service used for indexing documents in the haystack
    pub service: ServiceType,
    /// When set to `true` the haystack is treated as read-only; documents found
    /// inside will not be modified on disk by Terraphim (e.g. via the Novel
    /// editor). Defaults to `false` for backwards-compatibility.
    #[serde(default)]
    pub read_only: bool,
    /// When set to `true`, fetch the actual content of documents from URLs
    /// instead of just indexing metadata. Useful for web-based haystacks.
    /// Defaults to `false` for backwards-compatibility.
    #[serde(default)]
    pub fetch_content: bool,
    /// The secret for connecting to an Atomic Server.
    /// This field is only serialized for Atomic service haystacks.
    #[serde(default)]
    pub atomic_server_secret: Option<String>,
    /// Extra parameters specific to the service type.
    /// For Ripgrep: can include additional command-line arguments like filtering by tags.
    /// For Atomic: can include additional API parameters.
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub extra_parameters: std::collections::HashMap<String, String>,
}

impl Serialize for Haystack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        // Determine how many fields to include based on service type
        let mut field_count = 3; // location, service, read_only

        // Include atomic_server_secret only for Atomic service and if it's present
        let include_atomic_secret =
            self.service == ServiceType::Atomic && self.atomic_server_secret.is_some();
        if include_atomic_secret {
            field_count += 1;
        }

        // Include extra_parameters if not empty
        if !self.extra_parameters.is_empty() {
            field_count += 1;
        }

        let mut state = serializer.serialize_struct("Haystack", field_count)?;
        state.serialize_field("location", &self.location)?;
        state.serialize_field("service", &self.service)?;
        state.serialize_field("read_only", &self.read_only)?;

        // Only include atomic_server_secret for Atomic service
        if include_atomic_secret {
            state.serialize_field("atomic_server_secret", &self.atomic_server_secret)?;
        }

        // Include extra_parameters if not empty
        if !self.extra_parameters.is_empty() {
            state.serialize_field("extra_parameters", &self.extra_parameters)?;
        }

        state.end()
    }
}

impl std::fmt::Debug for Haystack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Haystack")
            .field("location", &self.location)
            .field("service", &self.service)
            .field("read_only", &self.read_only)
            .field("fetch_content", &self.fetch_content)
            .field(
                "atomic_server_secret",
                &self.atomic_server_secret.as_ref().map(|_| "***REDACTED***"),
            )
            .field("extra_parameters", &self.extra_parameters)
            .finish()
    }
}

impl Haystack {
    /// Create a new Haystack with extra parameters
    pub fn new(location: String, service: ServiceType, read_only: bool) -> Self {
        Self {
            location,
            service,
            read_only,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }
    }

    /// Create a new Haystack with atomic server secret
    pub fn with_atomic_secret(mut self, secret: Option<String>) -> Self {
        // Only set secret for Atomic service
        if self.service == ServiceType::Atomic {
            self.atomic_server_secret = secret;
        }
        self
    }

    /// Add extra parameters to the haystack
    pub fn with_extra_parameters(
        mut self,
        params: std::collections::HashMap<String, String>,
    ) -> Self {
        self.extra_parameters = params;
        self
    }

    /// Add a single extra parameter
    pub fn with_extra_parameter(mut self, key: String, value: String) -> Self {
        self.extra_parameters.insert(key, value);
        self
    }

    /// Get a reference to extra parameters for this service type
    pub fn get_extra_parameters(&self) -> &std::collections::HashMap<String, String> {
        &self.extra_parameters
    }
}

/// A knowledge graph is the collection of documents which were indexed
/// using a specific service
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct KnowledgeGraph {
    /// automata path refering to the published automata and can be online url or local file with pre-build automata
    #[schemars(with = "Option<String>")]
    pub automata_path: Option<AutomataPath>,
    /// Knowlege graph can be re-build from local files, for example Markdown files
    pub knowledge_graph_local: Option<KnowledgeGraphLocal>,
    /// Whether this knowledge graph is publicly accessible.
    pub public: bool,
    /// Whether this knowledge graph should be published to the registry.
    pub publish: bool,
}
impl KnowledgeGraph {
    /// Return `true` if either an automata path or a local KG directory is configured.
    pub fn is_set(&self) -> bool {
        self.automata_path.is_some() || self.knowledge_graph_local.is_some()
    }
}

/// Local knowledge-graph source: an input type paired with a filesystem path.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct KnowledgeGraphLocal {
    /// Format of the source documents (e.g. Markdown, plain text).
    pub input_type: KnowledgeGraphInputType,
    /// Filesystem path to the directory containing the source documents.
    pub path: PathBuf,
}
/// Builder, which allows to create a new `Config`
///
/// The first role added will be set as the default role.
/// This can be changed by calling `default_role` with the role name.
///
/// # Example
///
/// ```rs
/// use terraphim_config::ConfigBuilder;
///
/// let config = ConfigBuilder::new()
///    .global_shortcut("Ctrl+X")
///    .with_role("Default", role)
///    .with_role("Engineer", role)
///    .with_role("System Operator", role)
///    .default_role("Default")
///    .build();
/// ```
#[derive(Debug)]
pub struct ConfigBuilder {
    config: Config,
    device_settings: DeviceSettings,
    #[allow(dead_code)]
    settings_path: PathBuf,
}

impl ConfigBuilder {
    /// Create a new `ConfigBuilder`
    pub fn new() -> Self {
        Self {
            config: Config::empty(),
            device_settings: DeviceSettings::new(),
            settings_path: PathBuf::new(),
        }
    }
    /// Create a new builder seeded with the given [`ConfigId`].
    ///
    /// Chooses the appropriate [`DeviceSettings`] variant for the id:
    /// `ConfigId::Embedded` uses embedded defaults; all others call
    /// [`DeviceSettings::new`].
    pub fn new_with_id(id: ConfigId) -> Self {
        let device_settings = match id {
            ConfigId::Embedded => DeviceSettings::default_embedded(),
            _ => DeviceSettings::new(),
        };

        Self {
            config: Config {
                id,
                ..Config::empty()
            },
            device_settings,
            settings_path: PathBuf::new(),
        }
    }
    /// Populate the builder with a minimal embedded (WASM/library) configuration.
    pub fn build_default_embedded(mut self) -> Self {
        self.config.id = ConfigId::Embedded;

        // Add Default role with basic functionality
        let mut default_role = Role::new("Default");
        default_role.shortname = Some("Default".to_string());
        default_role.theme = "spacelab".to_string();
        default_role.extra.insert(
            "llm_provider".to_string(),
            Value::String("ollama".to_string()),
        );
        default_role.extra.insert(
            "llm_model".to_string(),
            Value::String("llama3.2:3b".to_string()),
        );
        default_role.haystacks = vec![Haystack {
            location: "docs/src".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }];

        self = self.add_role("Default", default_role);

        // Add Terraphim Engineer role with knowledge graph
        let mut terraphim_role = Role::new("Terraphim Engineer");
        terraphim_role.shortname = Some("TerraEng".to_string());
        terraphim_role.relevance_function = RelevanceFunction::TerraphimGraph;
        terraphim_role.terraphim_it = true;
        terraphim_role.theme = "lumen".to_string();
        terraphim_role.extra.insert(
            "llm_provider".to_string(),
            Value::String("ollama".to_string()),
        );
        terraphim_role.extra.insert(
            "llm_model".to_string(),
            Value::String("llama3.2:3b".to_string()),
        );
        terraphim_role.kg = Some(KnowledgeGraph {
            automata_path: None,
            knowledge_graph_local: Some(KnowledgeGraphLocal {
                input_type: KnowledgeGraphInputType::Markdown,
                path: self.get_default_data_path().join("kg"),
            }),
            public: true,
            publish: true,
        });
        terraphim_role.haystacks = vec![Haystack {
            location: "docs/src".to_string(),
            service: ServiceType::Ripgrep,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }];

        self = self.add_role("Terraphim Engineer", terraphim_role);

        // Add Rust Engineer role with QueryRs
        let mut rust_engineer_role = Role::new("Rust Engineer");
        rust_engineer_role.shortname = Some("rust-engineer".to_string());
        rust_engineer_role.theme = "cosmo".to_string();
        rust_engineer_role.extra.insert(
            "llm_provider".to_string(),
            Value::String("ollama".to_string()),
        );
        rust_engineer_role.extra.insert(
            "llm_model".to_string(),
            Value::String("qwen2.5-coder:latest".to_string()),
        );
        rust_engineer_role.haystacks = vec![Haystack {
            location: "https://query.rs".to_string(),
            service: ServiceType::QueryRs,
            read_only: true,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }];

        self = self.add_role("Rust Engineer", rust_engineer_role);

        // Set Terraphim Engineer as default and selected role
        self.config.default_role = RoleName::new("Terraphim Engineer");
        self.config.selected_role = RoleName::new("Terraphim Engineer");
        self
    }

    /// Resolve the configured `default_data_path`, expanding `~` and `${VAR}` tokens.
    pub fn get_default_data_path(&self) -> PathBuf {
        expand_path(&self.device_settings.default_data_path)
    }

    /// Populate the builder with a server configuration sourced from the current working directory.
    pub fn build_default_server(mut self) -> Self {
        self.config.id = ConfigId::Server;
        // mind where cargo run is triggered from
        let cwd = std::env::current_dir()
            .context("Failed to get current directory")
            .unwrap();
        log::info!("Current working directory: {}", cwd.display());
        let system_operator_haystack = if cwd.ends_with("terraphim_server") {
            cwd.join("fixtures/haystack/")
        } else {
            cwd.join("terraphim_server/fixtures/haystack/")
        };

        log::debug!("system_operator_haystack: {:?}", system_operator_haystack);
        let automata_test_path = if cwd.ends_with("terraphim_server") {
            cwd.join("fixtures/term_to_id.json")
        } else {
            cwd.join("terraphim_server/fixtures/term_to_id.json")
        };
        log::debug!("Test automata_test_path {:?}", automata_test_path);
        let automata_remote = AutomataPath::from_remote(
            "https://staging-storage.terraphim.io/thesaurus_Default.json",
        )
        .unwrap();
        log::info!("Automata remote URL: {automata_remote}");
        self.global_shortcut("Ctrl+X")
            .add_role("Default", {
                let mut default_role = Role::new("Default");
                default_role.shortname = Some("Default".to_string());
                default_role.theme = "spacelab".to_string();
                default_role.haystacks = vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                default_role
            })
            .add_role("Engineer", {
                let mut engineer_role = Role::new("Engineer");
                engineer_role.shortname = Some("Engineer".into());
                engineer_role.relevance_function = RelevanceFunction::TerraphimGraph;
                engineer_role.terraphim_it = true;
                engineer_role.theme = "lumen".to_string();
                engineer_role.kg = Some(KnowledgeGraph {
                    automata_path: Some(automata_remote.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_haystack.clone(),
                    }),
                    public: true,
                    publish: true,
                });
                engineer_role.haystacks = vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                engineer_role
            })
            .add_role("System Operator", {
                let mut system_operator_role = Role::new("System Operator");
                system_operator_role.shortname = Some("operator".to_string());
                system_operator_role.relevance_function = RelevanceFunction::TerraphimGraph;
                system_operator_role.terraphim_it = true;
                system_operator_role.theme = "superhero".to_string();
                system_operator_role.kg = Some(KnowledgeGraph {
                    automata_path: Some(automata_remote.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_haystack.clone(),
                    }),
                    public: true,
                    publish: true,
                });
                system_operator_role.haystacks = vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                system_operator_role
            })
            .default_role("Default")
            .unwrap()
    }

    /// Populate the builder with desktop defaults, using the resolved data path for haystacks.
    pub fn build_default_desktop(mut self) -> Self {
        let default_data_path = self.get_default_data_path();
        // Remove the automata_path - let it be built from local KG files during startup
        log::info!("Documents path: {:?}", default_data_path);
        self.config.id = ConfigId::Desktop;
        self.global_shortcut("Ctrl+X")
            .add_role("Default", {
                let mut default_role = Role::new("Default");
                default_role.shortname = Some("Default".to_string());
                default_role.theme = "spacelab".to_string();
                default_role.haystacks = vec![Haystack {
                    location: default_data_path.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                default_role
            })
            .add_role("Terraphim Engineer", {
                let mut terraphim_engineer_role = Role::new("Terraphim Engineer");
                terraphim_engineer_role.shortname = Some("TerraEng".to_string());
                terraphim_engineer_role.relevance_function = RelevanceFunction::TerraphimGraph;
                terraphim_engineer_role.terraphim_it = true;
                terraphim_engineer_role.theme = "lumen".to_string();
                terraphim_engineer_role.kg = Some(KnowledgeGraph {
                    automata_path: None, // Set to None so it builds from local KG files during startup
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: default_data_path.join("kg"),
                    }),
                    public: true,
                    publish: true,
                });
                terraphim_engineer_role.haystacks = vec![Haystack {
                    location: default_data_path.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                terraphim_engineer_role
            })
            .add_role("Rust Engineer", {
                let mut rust_engineer_role = Role::new("Rust Engineer");
                rust_engineer_role.shortname = Some("rust-engineer".to_string());
                rust_engineer_role.theme = "cosmo".to_string();
                rust_engineer_role.haystacks = vec![Haystack {
                    location: "https://query.rs".to_string(),
                    service: ServiceType::QueryRs,
                    read_only: true,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                rust_engineer_role
            })
            .default_role("Terraphim Engineer")
            .unwrap()
    }

    /// Start from an existing config
    ///
    /// This is useful when you want to start from an setup and modify some
    /// fields
    pub fn from_config(
        config: Config,
        device_settings: DeviceSettings,
        settings_path: PathBuf,
    ) -> Self {
        Self {
            config,
            device_settings,
            settings_path,
        }
    }

    /// Set the global shortcut for the config
    pub fn global_shortcut(mut self, global_shortcut: &str) -> Self {
        self.config.global_shortcut = Some(global_shortcut.to_string());
        self
    }

    /// Merge with a project config.
    ///
    /// Project roles fully replace global roles (by RoleName), not deep-merge.
    /// Project global_shortcut, if present, overrides the global one.
    pub fn merge_with(mut self, project_config: &crate::project::ProjectConfig) -> Self {
        if let Some(ref shortcut) = project_config.global_shortcut {
            self.config.global_shortcut = Some(shortcut.clone());
        }
        for (name, role) in &project_config.roles {
            let role_name = RoleName::new(name);
            self.config.roles.insert(role_name, role.clone());
        }
        self
    }

    /// Apply project config discovery and merge if found.
    ///
    /// Returns self unchanged if no project config is found.
    pub fn with_project(self) -> Self {
        if let Ok(Some(path)) = crate::project::discover(None) {
            let config_path = path.join("config.json");
            if config_path.is_file() {
                if let Ok(project_config) = crate::project::ProjectConfig::from_file(&config_path) {
                    return self.merge_with(&project_config);
                }
            }
        }
        self
    }

    /// Add a new role to the config
    pub fn add_role(mut self, role_name: &str, role: Role) -> Self {
        let role_name = RoleName::new(role_name);
        // Set to default role if this is the first role
        if self.config.roles.is_empty() {
            self.config.default_role = role_name.clone();
        }
        self.config.roles.insert(role_name, role);

        self
    }

    /// Set the default role for the config
    pub fn default_role(mut self, default_role: &str) -> Result<Self> {
        let default_role = RoleName::new(default_role);
        // Check if the role exists
        if !self.config.roles.contains_key(&default_role) {
            return Err(TerraphimConfigError::Profile(format!(
                "Role `{}` does not exist",
                default_role
            )));
        }

        self.config.default_role = default_role;
        Ok(self)
    }

    /// Build the config
    pub fn build(self) -> Result<Config> {
        // Make sure that we have at least one role
        // if self.config.roles.is_empty() {
        //     return Err(TerraphimConfigError::NoRoles);
        // }

        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Distinguishes how the configuration is deployed: as a background server,
/// a desktop application, or compiled-in (WASM/library) mode.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum ConfigId {
    /// Configuration deployed as a background HTTP server.
    Server,
    /// Configuration deployed as a desktop (Tauri) application.
    Desktop,
    /// Configuration compiled in (WASM/library) with no external server.
    Embedded,
}

/// The Terraphim config is the main configuration for terraphim
///
/// It contains the global shortcut, roles, and the default role
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Config {
    /// Identifier for the config
    pub id: ConfigId,
    /// Global shortcut for activating terraphim desktop
    #[schemars(default)]
    pub global_shortcut: Option<String>,
    /// User roles with their respective settings
    #[schemars(skip)]
    pub roles: AHashMap<RoleName, Role>,
    /// The default role to use if no role is specified
    pub default_role: RoleName,
    /// The role currently selected by the user (may differ from `default_role`).
    pub selected_role: RoleName,
}

impl Config {
    fn empty() -> Self {
        Self {
            id: ConfigId::Server, // Default to Server
            global_shortcut: None,
            roles: AHashMap::new(),
            default_role: RoleName::new("Default"),
            selected_role: RoleName::new("Default"),
        }
    }

    /// Load a Config from a JSON file path.
    ///
    /// The path is expanded using `expand_path()` to support ~, $HOME, and
    /// ${VAR:-default} syntax.
    pub fn load_from_json_file(path: &str) -> Result<Self> {
        let expanded = expand_path(path);
        log::info!("Loading role configuration from: {}", expanded.display());

        let content = std::fs::read_to_string(&expanded).map_err(|e| {
            log::error!(
                "Failed to read role config file '{}' (expanded from '{}'): {}",
                expanded.display(),
                path,
                e
            );
            TerraphimConfigError::Config(format!(
                "Cannot read role config file '{}': {}",
                expanded.display(),
                e
            ))
        })?;

        let config: Config = serde_json::from_str(&content)?;

        log::info!(
            "Loaded {} role(s) from '{}': {:?}",
            config.roles.len(),
            expanded.display(),
            config
                .roles
                .keys()
                .map(|k| k.to_string())
                .collect::<Vec<_>>()
        );

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::empty()
    }
}

#[async_trait]
impl Persistable for Config {
    fn new(_key: String) -> Self {
        // Key is not used because we use the `id` field
        Config::empty()
    }

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> PersistenceResult<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // Saves to all profiles
    async fn save(&self) -> PersistenceResult<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

    /// Load key from the fastest operator
    async fn load(&mut self) -> PersistenceResult<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        let obj = self.load_from_operator(&key, op).await?;
        Ok(obj)
    }

    /// returns ulid as key + .json
    fn get_key(&self) -> String {
        match self.id {
            ConfigId::Server => "server",
            ConfigId::Desktop => "desktop",
            ConfigId::Embedded => "embedded",
        }
        .to_string()
            + "_config.json"
    }
}

/// ConfigState for the Terraphim (Actor)
/// Extract trigger and pinned directives from KG markdown files.
///
/// Parses markdown directives from the given directory, looks up each concept
/// in the thesaurus to obtain its node ID, and returns a map of node IDs to
/// trigger text plus a list of pinned node IDs.
fn extract_triggers_from_kg(
    kg_path: &PathBuf,
    thesaurus: &terraphim_types::Thesaurus,
) -> (ahash::AHashMap<u64, String>, Vec<u64>) {
    let mut triggers = ahash::AHashMap::new();
    let mut pinned = Vec::new();

    let parsed = match parse_markdown_directives_dir(kg_path.as_path()) {
        Ok(result) => result,
        Err(err) => {
            log::warn!(
                "Failed to parse markdown directives from {:?}: {}",
                kg_path,
                err
            );
            return (triggers, pinned);
        }
    };

    for (concept_name, directives) in parsed.directives {
        let normalized_value = terraphim_types::NormalizedTermValue::new(concept_name.clone());
        if let Some(term) = thesaurus.get(&normalized_value) {
            let node_id = term.id;
            if let Some(trigger_text) = directives.trigger {
                if !trigger_text.trim().is_empty() {
                    triggers.insert(node_id, trigger_text.trim().to_string());
                }
            }
            if directives.pinned {
                pinned.push(node_id);
            }
        } else {
            log::debug!(
                "Concept '{}' not found in thesaurus for trigger extraction",
                concept_name
            );
        }
    }

    (triggers, pinned)
}

/// Config state can be updated using the API or Atomic Server
///
/// Holds the Terraphim Config and the RoleGraphs
#[derive(Debug, Clone)]
pub struct ConfigState {
    /// Terraphim Config
    pub config: Arc<Mutex<Config>>,
    /// RoleGraphs
    pub roles: AHashMap<RoleName, RoleGraphSync>,
}

impl ConfigState {
    /// Create a new ConfigState
    ///
    /// For each role in a config, initialize a rolegraph
    /// and add it to the config state
    pub async fn new(config: &mut Config) -> Result<Self> {
        let mut roles = AHashMap::new();
        for (name, role) in &config.roles {
            let role_name = name.clone();
            log::info!("Creating role {}", role_name);
            if role.relevance_function == RelevanceFunction::TerraphimGraph {
                if let Some(kg) = &role.kg {
                    if let Some(automata_path) = &kg.automata_path {
                        log::info!(
                            "Role {} is configured correctly with automata_path",
                            role_name
                        );
                        log::info!("Loading Role `{}` - URL: {:?}", role_name, automata_path);

                        // Try to load from automata path first
                        match load_thesaurus(automata_path).await {
                            Ok(thesaurus) => {
                                log::info!("Successfully loaded thesaurus from automata path");
                                let mut rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus.clone()).await?;
                                // Load trigger/pinned directives from local KG if available
                                if let Some(kg_local) = &kg.knowledge_graph_local {
                                    let (triggers, pinned) =
                                        extract_triggers_from_kg(&kg_local.path, &thesaurus);
                                    if !triggers.is_empty() || !pinned.is_empty() {
                                        log::info!(
                                            "Loading {} triggers and {} pinned entries for role {} from local KG",
                                            triggers.len(),
                                            pinned.len(),
                                            role_name
                                        );
                                        rolegraph.load_trigger_index(triggers, pinned, 0.3);
                                    }
                                }
                                roles.insert(role_name.clone(), RoleGraphSync::from(rolegraph));
                            }
                            Err(e) => {
                                log::warn!("Failed to load thesaurus from automata path: {:?}", e);
                                if let Some(kg_local) = &kg.knowledge_graph_local {
                                    log::info!(
                                        "Falling back to local KG for role {} at {:?}",
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
                                                "Successfully built thesaurus from local KG fallback for role {}",
                                                role_name
                                            );
                                            let mut rolegraph = RoleGraph::new(
                                                role_name.clone(),
                                                thesaurus.clone(),
                                            )
                                            .await?;
                                            let (triggers, pinned) = extract_triggers_from_kg(
                                                &kg_local.path,
                                                &thesaurus,
                                            );
                                            if !triggers.is_empty() || !pinned.is_empty() {
                                                log::info!(
                                                    "Loading {} triggers and {} pinned entries for role {} from local KG fallback",
                                                    triggers.len(),
                                                    pinned.len(),
                                                    role_name
                                                );
                                                rolegraph.load_trigger_index(triggers, pinned, 0.3);
                                            }
                                            roles.insert(
                                                role_name.clone(),
                                                RoleGraphSync::from(rolegraph),
                                            );
                                        }
                                        Err(e2) => {
                                            log::error!(
                                                "Failed to build thesaurus from local KG fallback for role {}: {:?}",
                                                role_name,
                                                e2
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(kg_local) = &kg.knowledge_graph_local {
                        // If automata_path is None, but a local KG is defined, build it now
                        log::info!(
                            "Role {} has no automata_path, building thesaurus from local KG files at {:?}",
                            role_name,
                            kg_local.path
                        );
                        let logseq_builder = Logseq::default();
                        match logseq_builder
                            .build(role_name.as_lowercase().to_string(), kg_local.path.clone())
                            .await
                        {
                            Ok(thesaurus) => {
                                log::info!(
                                    "Successfully built thesaurus from local KG for role {}",
                                    role_name
                                );
                                let mut rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus.clone()).await?;
                                let (triggers, pinned) =
                                    extract_triggers_from_kg(&kg_local.path, &thesaurus);
                                if !triggers.is_empty() || !pinned.is_empty() {
                                    log::info!(
                                        "Loading {} triggers and {} pinned entries for role {} from local KG",
                                        triggers.len(),
                                        pinned.len(),
                                        role_name
                                    );
                                    rolegraph.load_trigger_index(triggers, pinned, 0.3);
                                }
                                roles.insert(role_name.clone(), RoleGraphSync::from(rolegraph));
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to build thesaurus from local KG for role {}: {:?}",
                                    role_name,
                                    e
                                );
                            }
                        }
                    } else {
                        log::warn!(
                            "Role {} is configured for TerraphimGraph but has neither automata_path nor knowledge_graph_local defined.",
                            role_name
                        );
                    }
                }
            }
        }

        Ok(ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles,
        })
    }

    /// Get the default role from the config
    pub async fn get_default_role(&self) -> RoleName {
        let config = self.config.lock().await;
        config.default_role.clone()
    }

    /// Return the currently selected role name.
    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config.lock().await;
        config.selected_role.clone()
    }

    /// Get a role from the config
    pub async fn get_role(&self, role: &RoleName) -> Option<Role> {
        let config = self.config.lock().await;
        config.roles.get(role).cloned()
    }

    /// Insert document into all rolegraphs
    pub async fn add_to_roles(&mut self, document: &Document) -> OpendalResult<()> {
        let id = document.id.clone();

        for rolegraph_state in self.roles.values() {
            let mut rolegraph = rolegraph_state.lock().await;
            rolegraph.insert_document(&id, document.clone());
        }
        Ok(())
    }

    /// Search documents in rolegraph index using matching Knowledge Graph
    /// If knowledge graph isn't defined for the role, RoleGraph isn't build for the role
    pub async fn search_indexed_documents(
        &self,
        search_query: &SearchQuery,
        role: &Role,
    ) -> Vec<IndexedDocument> {
        log::debug!("search_documents: {:?}", search_query);

        log::debug!("Role for search_documents: {:#?}", role);

        let role_name = &role.name;
        log::debug!("Role name for searching {role_name}");
        log::debug!("All roles defined  {:?}", self.roles.clone().into_keys());
        //FIXME: breaks here for ripgrep, means KB based search is triggered before KG build
        let role = match self.roles.get(role_name) {
            Some(role) => role.lock().await,
            None => {
                // Handle the None case, e.g., return an empty vector since the function expects Vec<IndexedDocument>
                log::error!(
                    "Role `{}` does not exist or RoleGraph isn't populated",
                    role_name
                );
                return Vec::new();
            }
        };
        let documents = if search_query.is_multi_term_query() {
            // Use multi-term search with logical operators
            let all_terms: Vec<&str> = search_query
                .get_all_terms()
                .iter()
                .map(|t| t.as_str())
                .collect();
            let operator = search_query.get_operator();

            log::debug!(
                "Performing multi-term search with {} terms using {:?} operator",
                all_terms.len(),
                operator
            );

            role.query_graph_with_operators(
                &all_terms,
                &operator,
                search_query.skip,
                search_query.limit,
            )
            .unwrap_or_else(|e| {
                log::error!(
                    "Error while searching graph with operators for documents: {:?}",
                    e
                );
                vec![]
            })
        } else if search_query.include_pinned {
            // Use trigger fallback path which supports include_pinned
            role.query_graph_with_trigger_fallback(
                search_query.search_term.as_str(),
                search_query.skip,
                search_query.limit,
                true,
            )
            .unwrap_or_else(|e| {
                log::error!(
                    "Error while searching graph with trigger fallback for documents: {:?}",
                    e
                );
                vec![]
            })
        } else {
            // Use single-term search (backward compatibility)
            role.query_graph(
                search_query.search_term.as_str(),
                search_query.skip,
                search_query.limit,
            )
            .unwrap_or_else(|e| {
                log::error!("Error while searching graph for documents: {:?}", e);
                vec![]
            })
        };

        documents.into_iter().map(|(_id, doc)| doc).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;
    use terraphim_test_utils::EnvVarGuard;
    use tokio::test;

    #[test]
    async fn test_write_config_to_json() {
        let config = Config::empty();
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut tempfile = tempfile().unwrap();
        tempfile.write_all(json_str.as_bytes()).unwrap();
    }

    #[test]
    async fn test_get_key() {
        let config = Config::empty();
        serde_json::to_string_pretty(&config).unwrap();
        assert!(config.get_key().ends_with(".json"));
    }

    #[tokio::test]
    async fn test_save_all() {
        // Force in-memory persistence to avoid external/backing store locks in CI
        terraphim_persistence::DeviceStorage::init_memory_only()
            .await
            .unwrap();
        let config = Config::empty();
        config.save().await.unwrap();
    }

    #[tokio::test]
    async fn test_save_one_s3() {
        // Force in-memory persistence to avoid external/backing store locks in CI
        terraphim_persistence::DeviceStorage::init_memory_only()
            .await
            .unwrap();
        let config = Config::empty();
        println!("{:#?}", config);
        match config.save_to_one("s3").await {
            Ok(_) => println!("Successfully saved to s3 (env provides s3 profile)"),
            Err(e) => {
                println!(
                    "Expected error saving to s3 in test environment without s3 profile: {:?}",
                    e
                );
                // Acceptable in CI: no s3 profile available when using memory-only init
            }
        }
    }

    #[tokio::test]
    async fn load_s3() {
        let mut config = Config::empty();
        match config.load().await {
            Ok(loaded_config) => {
                println!("Successfully loaded config: {:#?}", loaded_config);
            }
            Err(e) => {
                println!(
                    "Expected error loading config (no S3 data in test environment): {:?}",
                    e
                );
                // This is expected in test environment where S3 data doesn't exist
            }
        }
    }

    #[tokio::test]
    async fn test_save_one_memory() {
        // Try to force in-memory persistence; may be a no-op if another test
        // already initialized the global singleton with different profiles.
        let _ = terraphim_persistence::DeviceStorage::init_memory_only().await;
        let config = Config::empty();
        match config.save_to_one("memory").await {
            Ok(_) => println!("Successfully saved to memory profile"),
            Err(_) => {
                // Memory profile not available (global storage initialized with
                // different profiles by another test). Save to first available profile.
                config.save().await.unwrap();
            }
        }
    }

    #[test]
    async fn test_write_config_to_toml() {
        let config = Config::empty();
        let toml = toml::to_string_pretty(&config).unwrap();
        // Ensure that the toml is valid
        toml::from_str::<Config>(&toml).unwrap();
    }

    #[tokio::test]
    async fn test_config_builder() {
        let automata_remote = AutomataPath::from_remote(
            "https://staging-storage.terraphim.io/thesaurus_Default.json",
        )
        .unwrap();
        let config = ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role("Default", {
                let mut default_role = Role::new("Default");
                default_role.shortname = Some("Default".to_string());
                default_role.theme = "spacelab".to_string();
                default_role.haystacks = vec![Haystack {
                    location: "localsearch".to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                default_role
            })
            .add_role("Engineer", {
                let mut engineer_role = Role::new("Engineer");
                engineer_role.shortname = Some("Engineer".to_string());
                engineer_role.theme = "lumen".to_string();
                engineer_role.haystacks = vec![Haystack {
                    location: "localsearch".to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                engineer_role
            })
            .add_role("System Operator", {
                let mut system_operator_role = Role::new("System Operator");
                system_operator_role.shortname = Some("operator".to_string());
                system_operator_role.relevance_function = RelevanceFunction::TerraphimGraph;
                system_operator_role.terraphim_it = true;
                system_operator_role.theme = "superhero".to_string();
                system_operator_role.kg = Some(KnowledgeGraph {
                    automata_path: Some(automata_remote.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("~/pkm"),
                    }),
                    public: true,
                    publish: true,
                });
                system_operator_role.haystacks = vec![Haystack {
                    location: "/tmp/system_operator/pages/".to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    fetch_content: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }];
                system_operator_role
            })
            .default_role("Default")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.roles.len(), 3);
        assert_eq!(config.default_role, RoleName::new("Default"));
    }

    #[test]
    async fn test_update_global_shortcut() {
        let config = ConfigBuilder::new()
            .add_role("dummy", dummy_role())
            .build()
            .unwrap();
        assert_eq!(config.global_shortcut, None);
        let device_settings = DeviceSettings::new();
        let settings_path = PathBuf::from(".");
        let new_config = ConfigBuilder::from_config(config, device_settings, settings_path)
            .global_shortcut("Ctrl+/")
            .build()
            .unwrap();

        assert_eq!(new_config.global_shortcut, Some("Ctrl+/".to_string()));
    }

    fn dummy_role() -> Role {
        let mut role = Role::new("Father");
        role.shortname = Some("father".into());
        role.theme = "lumen".to_string();
        role.kg = Some(KnowledgeGraph {
            automata_path: Some(AutomataPath::local_example()),
            knowledge_graph_local: None,
            public: true,
            publish: true,
        });
        role.haystacks = vec![Haystack {
            location: "localsearch".to_string(),
            service: ServiceType::Ripgrep,
            read_only: false,
            fetch_content: false,
            atomic_server_secret: None,
            extra_parameters: std::collections::HashMap::new(),
        }];
        role
    }

    #[test]
    async fn test_add_role() {
        // Create a new role by building a new config
        let config = ConfigBuilder::new()
            .add_role("Father", dummy_role())
            .build()
            .unwrap();

        assert!(config.roles.contains_key(&RoleName::new("Father")));
        assert_eq!(config.roles.len(), 1);
        assert_eq!(&config.default_role, &RoleName::new("Father"));
        assert_eq!(config.roles[&RoleName::new("Father")], dummy_role());
    }

    ///test to create config with different id - server, desktop, embedded
    #[tokio::test]
    async fn test_config_with_id_desktop() {
        let config = match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::info!("Failed to load config: {:?}", e);

                    ConfigBuilder::new()
                        .build_default_desktop()
                        .build()
                        .unwrap()
                }
            },
            Err(e) => panic!("Failed to build config: {:?}", e),
        };
        assert_eq!(config.id, ConfigId::Desktop);
    }
    /// repeat the test with server and embedded
    #[tokio::test]
    async fn test_config_with_id_server() {
        let config = match ConfigBuilder::new_with_id(ConfigId::Server).build() {
            Ok(mut local_config) => match local_config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::info!("Failed to load config: {:?}", e);

                    ConfigBuilder::new().build_default_server().build().unwrap()
                }
            },
            Err(e) => panic!("Failed to build config: {:?}", e),
        };
        assert_eq!(config.id, ConfigId::Server);
    }

    #[tokio::test]
    async fn test_config_with_id_embedded() {
        let config = match ConfigBuilder::new_with_id(ConfigId::Embedded).build() {
            Ok(mut config) => match config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::info!("Failed to load config: {:?}", e);

                    ConfigBuilder::new()
                        .build_default_embedded()
                        .build()
                        .unwrap()
                }
            },
            Err(e) => panic!("Failed to build config: {:?}", e),
        };
        assert_eq!(config.id, ConfigId::Embedded);
    }

    #[tokio::test]
    #[ignore]
    async fn test_at_least_one_role() {
        let config = ConfigBuilder::new().build();
        assert!(config.is_err());
        assert!(matches!(config.unwrap_err(), TerraphimConfigError::NoRoles));
    }

    #[tokio::test]
    async fn test_json_serialization() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        log::debug!("Config: {:#?}", config);
        assert!(!json.is_empty());
    }

    #[tokio::test]
    async fn test_toml_serialization() {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        log::debug!("Config: {:#?}", config);
        assert!(!toml.is_empty());
    }

    #[tokio::test]
    async fn test_expand_path_home() {
        let home = dirs::home_dir().expect("HOME should be set");
        let home_str = home.to_string_lossy();

        // Test ${HOME} expansion
        let result = expand_path("${HOME}/.terraphim");
        assert_eq!(result, home.join(".terraphim"));

        // Test $HOME expansion
        let result = expand_path("$HOME/.terraphim");
        assert_eq!(result, home.join(".terraphim"));

        // Test ~ expansion
        let result = expand_path("~/.terraphim");
        assert_eq!(result, home.join(".terraphim"));

        // Test nested ${VAR:-default} with ${HOME}
        let result = expand_path("${TERRAPHIM_DATA_PATH:-${HOME}/.terraphim}");
        assert_eq!(result, home.join(".terraphim"));

        // Test when env var is set
        let _guard = EnvVarGuard::set("TERRAPHIM_TEST_PATH", "/custom/path");
        let result = expand_path("${TERRAPHIM_TEST_PATH:-${HOME}/.default}");
        assert_eq!(result, PathBuf::from("/custom/path"));
        drop(_guard);

        println!("expand_path tests passed!");
        println!("HOME = {}", home_str);
        println!(
            "${{HOME}}/.terraphim -> {:?}",
            expand_path("${HOME}/.terraphim")
        );
    }

    #[test]
    async fn test_load_from_json_file_with_fixture() {
        // Build path relative to workspace root
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let fixture_path =
            workspace_root.join("terraphim_server/default/terraphim_engineer_config.json");
        let config = Config::load_from_json_file(fixture_path.to_str().unwrap()).unwrap();
        assert!(
            !config.roles.is_empty(),
            "Config should have at least one role"
        );
        assert!(
            config
                .roles
                .contains_key(&RoleName::new("Terraphim Engineer")),
            "Config should contain Terraphim Engineer role"
        );
    }

    #[test]
    async fn test_load_from_json_file_not_found() {
        let result = Config::load_from_json_file("/nonexistent/path/does_not_exist.json");
        assert!(result.is_err(), "Should return error for missing file");
    }

    #[test]
    async fn test_load_from_json_file_invalid_json() {
        // Create a temp file with invalid JSON
        let mut tmpfile = tempfile().unwrap();
        tmpfile.write_all(b"this is not json").unwrap();
        // We can't easily get a path from tempfile(), so test with a known bad path pattern
        // The real test is that the error type is correct
        let result = Config::load_from_json_file("/dev/null");
        assert!(
            result.is_err(),
            "Should return error for empty/invalid JSON"
        );
    }

    #[test]
    async fn role_llm_api_key_redacted_in_debug() {
        let role = Role {
            llm_api_key: Some("super-secret-api-key-do-not-leak".to_string()),
            ..Default::default()
        };
        let dbg = format!("{:?}", role);
        assert!(
            !dbg.contains("super-secret-api-key-do-not-leak"),
            "llm_api_key must be redacted in Debug output, got: {dbg}"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark llm_api_key as redacted, got: {dbg}"
        );
    }

    #[test]
    async fn role_none_llm_api_key_debug_shows_none() {
        let role = Role::default();
        let dbg = format!("{:?}", role);
        assert!(
            dbg.contains("llm_api_key: None"),
            "None llm_api_key should show as None in Debug, got: {dbg}"
        );
    }

    #[test]
    async fn haystack_atomic_server_secret_redacted_in_debug() {
        let mut haystack =
            Haystack::new("http://example.com".to_string(), ServiceType::Atomic, false);
        haystack.atomic_server_secret = Some("atomic-secret-do-not-leak".to_string());
        let dbg = format!("{:?}", haystack);
        assert!(
            !dbg.contains("atomic-secret-do-not-leak"),
            "atomic_server_secret must be redacted in Debug output, got: {dbg}"
        );
        assert!(
            dbg.contains("***REDACTED***"),
            "Debug output should mark atomic_server_secret as redacted, got: {dbg}"
        );
    }

    #[test]
    async fn haystack_none_atomic_server_secret_debug_shows_none() {
        let haystack = Haystack::new(
            "http://example.com".to_string(),
            ServiceType::Ripgrep,
            false,
        );
        let dbg = format!("{:?}", haystack);
        assert!(
            dbg.contains("atomic_server_secret: None"),
            "None atomic_server_secret should show as None in Debug, got: {dbg}"
        );
    }
}
