use std::{path::PathBuf, sync::Arc};

use terraphim_automata::{
    builder::{Logseq, ThesaurusBuilder},
    load_thesaurus, AutomataPath,
};
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, IndexedDocument, KnowledgeGraphInputType, RelevanceFunction, RoleName, SearchQuery,
};

use terraphim_settings::DeviceSettings;

use ahash::AHashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;
use schemars::JsonSchema;
#[cfg(feature = "typescript")]
use tsify::Tsify;

pub type Result<T> = std::result::Result<T, TerraphimConfigError>;

use opendal::Result as OpendalResult;

type PersistenceResult<T> = std::result::Result<T, terraphim_persistence::Error>;

#[derive(Error, Debug)]
pub enum TerraphimConfigError {
    #[error("Unable to load config")]
    NotFound,

    #[error("At least one role is required")]
    NoRoles,

    #[error("Profile error")]
    Profile(String),

    #[error("Persistence error")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Serde JSON error")]
    Json(#[from] serde_json::Error),

    #[error("Cannot initialize tracing subscriber")]
    TracingSubscriber(Box<dyn std::error::Error + Send + Sync>),

    #[error("Pipe error")]
    Pipe(#[from] terraphim_rolegraph::Error),

    #[error("Automata error")]
    Automata(#[from] terraphim_automata::TerraphimAutomataError),

    #[error("Url error")]
    Url(#[from] url::ParseError),

    #[error("IO error")]
    Io(#[from] std::io::Error),

    #[error("Config error")]
    Config(String),
}

/// A role is a collection of settings for a specific user
///
/// It contains a user's knowledge graph, a list of haystacks, as
/// well as preferences for the relevance function and theme
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Role {
    pub shortname: Option<String>,
    pub name: RoleName,
    /// The relevance function used to rank search results
    pub relevance_function: RelevanceFunction,
    pub terraphim_it: bool,
    pub theme: String,
    pub kg: Option<KnowledgeGraph>,
    pub haystacks: Vec<Haystack>,
    /// Enable AI-powered article summaries using OpenRouter
    #[cfg(feature = "openrouter")]
    #[serde(default)]
    pub openrouter_enabled: bool,
    /// API key for OpenRouter service
    #[cfg(feature = "openrouter")]
    #[serde(default)]
    pub openrouter_api_key: Option<String>,
    /// Model to use for generating summaries (e.g., "openai/gpt-3.5-turbo")
    #[cfg(feature = "openrouter")]
    #[serde(default)]
    pub openrouter_model: Option<String>,
    #[serde(flatten)]
    #[schemars(skip)]
    pub extra: AHashMap<String, Value>,
}

impl Role {
    /// Check if OpenRouter is properly configured for this role
    #[cfg(feature = "openrouter")]
    pub fn has_openrouter_config(&self) -> bool {
        self.openrouter_enabled && 
        self.openrouter_api_key.is_some() && 
        self.openrouter_model.is_some()
    }

    /// Check if OpenRouter is properly configured (stub when feature is disabled)
    #[cfg(not(feature = "openrouter"))]
    pub fn has_openrouter_config(&self) -> bool {
        false
    }

    /// Get the OpenRouter model name, providing a sensible default
    #[cfg(feature = "openrouter")]
    pub fn get_openrouter_model(&self) -> Option<&str> {
        self.openrouter_model.as_deref()
    }

    /// Get the OpenRouter model name (stub when feature is disabled)
    #[cfg(not(feature = "openrouter"))]
    pub fn get_openrouter_model(&self) -> Option<&str> {
        None
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
}

/// A haystack is a collection of documents that can be indexed and searched
///
/// One user can have multiple haystacks
/// Each haystack is indexed using a specific service
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
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
        let include_atomic_secret = self.service == ServiceType::Atomic && self.atomic_server_secret.is_some();
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

impl Haystack {
    /// Create a new Haystack with extra parameters
    pub fn new(location: String, service: ServiceType, read_only: bool) -> Self {
        Self {
            location,
            service,
            read_only,
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
    pub fn with_extra_parameters(mut self, params: std::collections::HashMap<String, String>) -> Self {
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
    pub public: bool,
    pub publish: bool,
}
/// check KG set correctly
impl KnowledgeGraph {
    pub fn is_set(&self) -> bool {
        self.automata_path.is_some() || self.knowledge_graph_local.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub struct KnowledgeGraphLocal {
    pub input_type: KnowledgeGraphInputType,
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
    settings_path: PathBuf
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
    pub fn new_with_id(id: ConfigId) -> Self {
        Self {
            config: Config { id, ..Config::empty() },
            device_settings: DeviceSettings::new(),
            settings_path: PathBuf::new(),
        }
    }
    pub fn build_default_embedded(mut self) -> Self {
        self.config.id = ConfigId::Embedded;
        self
    }

    pub fn get_default_data_path(&self) -> PathBuf {
        PathBuf::from(&self.device_settings.default_data_path)
    }
    pub fn build_default_server(mut self) -> Self {
        self.config.id = ConfigId::Server;
        // mind where cargo run is triggered from
        let cwd = std::env::current_dir().context("Failed to get current directory").unwrap();
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
        let automata_remote =
            AutomataPath::from_remote("https://staging-storage.terraphim.io/thesaurus_Default.json")
                .unwrap();
        log::info!("Automata remote URL: {automata_remote}");
        self.global_shortcut("Ctrl+X")
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "Engineer",
            Role {
                shortname: Some("Engineer".into()),
                name: "Engineer".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "lumen".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(automata_remote.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_haystack.clone(),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "System Operator",
            Role {
                shortname: Some("operator".to_string()),
                name: "System Operator".into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "superhero".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(automata_remote.clone()),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_haystack.clone(),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: system_operator_haystack.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                extra: AHashMap::new(),
            },
        )
        .default_role("Default").unwrap()
    }

    pub fn build_default_desktop(mut self) -> Self {
        let default_data_path = self.get_default_data_path();
        // Remove the automata_path - let it be built from local KG files during startup
        log::info!("Documents path: {:?}", default_data_path);
        self.config.id = ConfigId::Desktop;
        self.global_shortcut("Ctrl+X")
        .add_role(
            "Default",  
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".to_string().into(),
                relevance_function: RelevanceFunction::TitleScorer,
                terraphim_it: false,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    location: default_data_path.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "Terraphim Engineer",
            Role {
                shortname: Some("TerraEng".to_string()),
                name: "Terraphim Engineer".to_string().into(),
                relevance_function: RelevanceFunction::TerraphimGraph,
                terraphim_it: true,
                theme: "lumen".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: None, // Set to None so it builds from local KG files during startup
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: default_data_path.join("kg"),
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: default_data_path.to_string_lossy().to_string(),
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                }],
                #[cfg(feature = "openrouter")]
                openrouter_enabled: false,
                #[cfg(feature = "openrouter")]
                openrouter_api_key: None,
                #[cfg(feature = "openrouter")]
                openrouter_model: None,
                extra: AHashMap::new(),
            },
        )
        .default_role("Terraphim Engineer").unwrap()
    }


    /// Start from an existing config
    ///
    /// This is useful when you want to start from an setup and modify some
    /// fields
    pub fn from_config(config: Config, device_settings: DeviceSettings, settings_path: PathBuf) -> Self {
        Self { config, device_settings, settings_path }
    }

    /// Set the global shortcut for the config
    pub fn global_shortcut(mut self, global_shortcut: &str) -> Self {
        self.config.global_shortcut = global_shortcut.to_string();
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum ConfigId {
    Server,
    Desktop,
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
    pub global_shortcut: String,
    /// User roles with their respective settings
    #[schemars(skip)]
    pub roles: AHashMap<RoleName, Role>,
    /// The default role to use if no role is specified
    pub default_role: RoleName,
    pub selected_role: RoleName
}

impl Config {
    fn empty() -> Self {
        Self {
            id: ConfigId::Server, // Default to Server
            global_shortcut: "Ctrl+X".to_string(),
            roles: AHashMap::new(),
            default_role: RoleName::new("Default"),
            selected_role: RoleName::new("Default")
        }
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
        }.to_string() + "_config.json"
    }
}

/// ConfigState for the Terraphim (Actor)
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
                        log::info!("Role {} is configured correctly with automata_path", role_name);
                        log::info!("Loading Role `{}` - URL: {:?}", role_name, automata_path);

                        // Try to load from automata path first
                        match load_thesaurus(automata_path).await {
                            Ok(thesaurus) => {
                                log::info!("Successfully loaded thesaurus from automata path");
                                let rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus).await?;
                                roles.insert(role_name.clone(), RoleGraphSync::from(rolegraph));
                            }
                            Err(e) => {
                                log::warn!("Failed to load thesaurus from automata path: {:?}", e);
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
                            .build(
                                role_name.as_lowercase().to_string(),
                                kg_local.path.clone(),
                            )
                            .await
                        {
                            Ok(thesaurus) => {
                                log::info!(
                                    "Successfully built thesaurus from local KG for role {}",
                                    role_name
                                );
                                let rolegraph =
                                    RoleGraph::new(role_name.clone(), thesaurus).await?;
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
                        log::warn!("Role {} is configured for TerraphimGraph but has neither automata_path nor knowledge_graph_local defined.", role_name);
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
        let role = match self.roles.get(&role_name) {
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
        let documents = role
            .query_graph(
                search_query.search_term.as_str(),
                search_query.skip,
                search_query.limit,
            )
            .unwrap_or_else(|e| {
                log::error!("Error while searching graph for documents: {:?}", e);
                vec![]
            });

        documents.into_iter().map(|(_id, doc)| doc).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempfile;
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
        let config = Config::empty();
        config.save().await.unwrap();
    }

    #[tokio::test]
    async fn test_save_one_s3() {
        let config = Config::empty();
        println!("{:#?}", config);
        config.save_to_one("s3").await.unwrap();
    }

    #[tokio::test]
    async fn load_s3() {
        let mut config = Config::empty();
        match config.load().await {
            Ok(loaded_config) => {
                println!("Successfully loaded config: {:#?}", loaded_config);
            }
            Err(e) => {
                println!("Expected error loading config (no S3 data in test environment): {:?}", e);
                // This is expected in test environment where S3 data doesn't exist
            }
        }
    }

    #[tokio::test]
    async fn test_save_one_memory() {
        let config = Config::empty();
        config.save_to_one("memory").await.unwrap();
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
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    terraphim_it: false,
                    theme: "spacelab".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        location: "localsearch".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "Engineer",
                Role {
                    shortname: Some("Engineer".to_string()),
                    name: "Engineer".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    terraphim_it: false,
                    theme: "lumen".to_string(),
                    kg: None,
                    haystacks: vec![Haystack {
                        location: "localsearch".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "System Operator",
                Role {
                    shortname: Some("operator".to_string()),
                    name: "System Operator".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
                    terraphim_it: true,
                    theme: "superhero".to_string(),
                    kg: Some(KnowledgeGraph {
                        automata_path: Some(automata_remote.clone()),
                        knowledge_graph_local: Some(KnowledgeGraphLocal {
                            input_type: KnowledgeGraphInputType::Markdown,
                            path: PathBuf::from("~/pkm"),
                        }),
                        public: true,
                        publish: true,
                    }),
                    haystacks: vec![Haystack {
                        location: "/tmp/system_operator/pages/".to_string(),
                        service: ServiceType::Ripgrep,
                        read_only: false,
                        atomic_server_secret: None,
                        extra_parameters: std::collections::HashMap::new(),
                    }],
                    extra: AHashMap::new(),
                },
            )
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
        assert_eq!(config.global_shortcut, "Ctrl+X");
        let device_settings = DeviceSettings::new();    
        let settings_path = PathBuf::from(".");
        let new_config = ConfigBuilder::from_config(config, device_settings, settings_path)
            .global_shortcut("Ctrl+/")
            .build()
            .unwrap();

        assert_eq!(new_config.global_shortcut, "Ctrl+/");
    }

    fn dummy_role() -> Role {
        Role {
            shortname: Some("father".into()),
            name: "Father".into(),
            relevance_function: RelevanceFunction::TitleScorer,
            terraphim_it: false,
            theme: "lumen".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::local_example()),
                knowledge_graph_local: None,
                public: true,
                publish: true,
            }),
            haystacks: vec![Haystack {
                location: "localsearch".to_string(),
                service: ServiceType::Ripgrep,
                read_only: false,
                atomic_server_secret: None,
                extra_parameters: std::collections::HashMap::new(),
            }],
            extra: AHashMap::new(),
        }
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
                    let config = ConfigBuilder::new().build_default_desktop().build().unwrap();
                    config
                },
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
                    let config = ConfigBuilder::new().build_default_server().build().unwrap();
                    config
                },
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
                    let config = ConfigBuilder::new().build_default_embedded().build().unwrap();
                    config
                },
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
        assert!(json.len() > 0);
    }

    #[tokio::test]
    async fn test_toml_serialization() {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        log::debug!("Config: {:#?}", config);
        assert!(toml.len() > 0);
    }
}