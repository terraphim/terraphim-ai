use std::{path::PathBuf, sync::Arc};

use ahash::AHashMap;
use async_trait::async_trait;
use persistence::Persistable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use terraphim_automata::{load_thesaurus, AutomataPath};
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, IndexedDocument, KnowledgeGraphInputType, RelevanceFunction, SearchQuery,
};
use thiserror::Error;
use tokio::sync::Mutex;
use ulid::Ulid;

pub type Result<T> = std::result::Result<T, TerraphimConfigError>;

use opendal::Result as OpendalResult;
use url::Url;

type PersistenceResult<T> = std::result::Result<T, persistence::Error>;

#[derive(Error, Debug)]
pub enum TerraphimConfigError {
    #[error("Unable to load config")]
    NotFound,

    #[error("At least one role is required")]
    NoRoles,

    #[error("Profile error")]
    Profile(String),

    #[error("Persistence error")]
    Persistence(#[from] persistence::Error),

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
}

/// A role is a collection of settings for a specific user
///
/// It contains a user's knowledge graph, a list of haystacks, as
/// well as preferences for the relevance function and theme
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Role {
    pub shortname: Option<String>,
    pub name: String,
    /// The relevance function used to rank search results
    pub relevance_function: RelevanceFunction,
    pub theme: String,
    #[serde(rename = "serverUrl")]
    pub server_url: Url,
    pub kg: KnowledgeGraph,
    pub haystacks: Vec<Haystack>,
    #[serde(flatten)]
    pub extra: AHashMap<String, Value>,
}

/// The service used for indexing documents
///
/// Each service assumes documents to be stored in a specific format
/// and uses a specific indexing algorithm
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ServiceType {
    /// Use ripgrep as the indexing service
    Ripgrep,
    /// Use gmail as the indexing service
    Gmail,
}

/// A haystack is a collection of documents that can be indexed and searched
///
/// One user can have multiple haystacks
/// Each haystack is indexed using a specific service
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Haystack {
    /// The path to the haystack
    pub path: PathBuf,
    /// The service used for indexing documents in the haystack
    pub service: ServiceType,
}

/// A knowledge graph is the collection of documents which were indexed
/// using a specific service
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct KnowledgeGraph {
    pub automata_path: AutomataPath,
    pub input_type: KnowledgeGraphInputType,
    pub path: PathBuf,
    pub public: bool,
    pub publish: bool,
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
}

impl ConfigBuilder {
    /// Create a new `ConfigBuilder`
    pub fn new() -> Self {
        Self {
            config: Config::empty(),
        }
    }

    /// Start from an existing config
    ///
    /// This is useful when you want to start from an setup and modify some
    /// fields
    pub fn from_config(config: Config) -> Self {
        Self { config }
    }

    /// Set the global shortcut for the config
    pub fn global_shortcut(mut self, global_shortcut: &str) -> Self {
        self.config.global_shortcut = global_shortcut.to_string();
        self
    }

    /// Add a new role to the config
    pub fn add_role(mut self, role_name: &str, role: Role) -> Self {
        // Set to default role if this is the first role
        if self.config.roles.is_empty() {
            self.config.default_role = role_name.to_string();
        }

        self.config.roles.insert(role_name.to_string(), role);

        self
    }

    /// Set the default role for the config
    pub fn default_role(mut self, default_role: &str) -> Result<Self> {
        // Check if the role exists
        if !self.config.roles.contains_key(default_role) {
            return Err(TerraphimConfigError::Profile(format!(
                "Role `{}` does not exist",
                default_role
            )));
        }

        self.config.default_role = default_role.to_string();
        Ok(self)
    }

    /// Build the config
    pub fn build(self) -> Result<Config> {
        // Make sure that we have at least one role
        if self.config.roles.is_empty() {
            return Err(TerraphimConfigError::NoRoles);
        }

        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The Terraphim config is the main configuration for terraphim
///
/// It contains the global shortcut, roles, and the default role
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Unique identifier for the config
    pub id: String,
    /// Global shortcut for activating terraphim desktop
    pub global_shortcut: String,
    /// User roles with their respective settings
    pub roles: AHashMap<String, Role>,
    /// The default role to use if no role is specified
    pub default_role: String,
}

impl Config {
    fn empty() -> Self {
        Self {
            id: Ulid::new().to_string(),
            // global shortcut for terraphim desktop
            global_shortcut: "Ctrl+X".to_string(),
            roles: AHashMap::new(),
            default_role: "default".to_string(),
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
    async fn load(&mut self, key: &str) -> PersistenceResult<Self> {
        let op = &self.load_config().await?.1;
        let obj = self.load_from_operator(key, op).await?;
        Ok(obj)
    }

    /// returns ulid as key + .json
    fn get_key(&self) -> String {
        self.id.clone() + ".json"
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
    pub roles: AHashMap<String, RoleGraphSync>,
}

impl ConfigState {
    /// Create a new ConfigState
    ///
    /// For each role in a config, initialize a rolegraph
    /// and add it to the config state
    pub async fn new(config: &mut Config) -> Result<Self> {
        let mut roles = AHashMap::new();
        for (name, role) in &config.roles {
            let role_name = name.to_lowercase();
            let automata_url = role.kg.automata_path.clone();
            log::info!("Loading Role `{}` - URL: {}", role_name, automata_url);

            let thesaurus = load_thesaurus(&automata_url).await?;
            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;
            roles.insert(role_name, RoleGraphSync::from(rolegraph));
        }

        Ok(ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles,
        })
    }

    /// Get the default role from the config
    pub async fn get_default_role(&self) -> String {
        let config = self.config.lock().await;
        config.default_role.clone()
    }

    /// Get a role from the config
    pub async fn get_role(&self, role: &str) -> Option<Role> {
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

    /// Search documents in rolegraph index, which match the search query
    pub async fn search_indexed_documents(
        &self,
        search_query: &SearchQuery,
    ) -> Vec<IndexedDocument> {
        log::debug!("search_documents: {:?}", search_query);
        let current_config_state = self.config.lock().await.clone();
        let default_role = current_config_state.default_role.clone();

        // if role is not provided, use the default role from the config
        let role = search_query.role.clone().unwrap_or(default_role);
        log::debug!("Role for search_documents: {:#?}", role);

        let role_name = role.to_lowercase();
        let role = self.roles.get(&role_name).unwrap().lock().await;

        let documents = role
            .query_graph(
                &search_query.search_term,
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
        config.save_to_one("s3").await.unwrap();
    }

    #[tokio::test]
    async fn test_save_one_sled() {
        let config = Config::empty();
        config.save_to_one("sled").await.unwrap();
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
        let config = ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role(
                "Default",
                Role {
                    shortname: Some("Default".to_string()),
                    name: "Default".to_string(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "spacelab".to_string(),
                    server_url: Url::parse("http://localhost:8000/documents/search").unwrap(),
                    kg: KnowledgeGraph {
                        automata_path: AutomataPath::local_example(),
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("~/pkm"),
                        public: true,
                        publish: true,
                    },
                    haystacks: vec![Haystack {
                        path: PathBuf::from("localsearch"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "Engineer",
                Role {
                    shortname: Some("Engineer".to_string()),
                    name: "Engineer".to_string(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "lumen".to_string(),
                    server_url: Url::parse("http://localhost:8000/documents/search").unwrap(),
                    kg: KnowledgeGraph {
                        automata_path: AutomataPath::local_example(),
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("~/pkm"),
                        public: true,
                        publish: true,
                    },
                    haystacks: vec![Haystack {
                        path: PathBuf::from("localsearch"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .add_role(
                "System Operator",
                Role {
                    shortname: Some("operator".to_string()),
                    name: "System Operator".to_string(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "superhero".to_string(),
                    server_url: Url::parse("http://localhost:8000/documents/search").unwrap(),
                    kg: KnowledgeGraph {
                        automata_path: AutomataPath::local_example(),
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: PathBuf::from("~/pkm"),
                        public: true,
                        publish: true,
                    },
                    haystacks: vec![Haystack {
                        path: PathBuf::from("/tmp/system_operator/pages/"),
                        service: ServiceType::Ripgrep,
                    }],
                    extra: AHashMap::new(),
                },
            )
            .default_role("Default")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(config.roles.len(), 3);
        assert_eq!(config.default_role, "Default");
    }

    #[test]
    async fn test_update_global_shortcut() {
        let config = ConfigBuilder::new()
            .add_role("dummy", dummy_role())
            .build()
            .unwrap();
        assert_eq!(config.global_shortcut, "Ctrl+X");

        let new_config = ConfigBuilder::from_config(config)
            .global_shortcut("Ctrl+/")
            .build()
            .unwrap();

        assert_eq!(new_config.global_shortcut, "Ctrl+/");
    }

    fn dummy_role() -> Role {
        Role {
            shortname: Some("father".to_string()),
            name: "Father".to_string(),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "lumen".to_string(),
            server_url: Url::parse("http://localhost:8080").unwrap(),
            kg: KnowledgeGraph {
                automata_path: AutomataPath::local_example(),
                input_type: KnowledgeGraphInputType::Markdown,
                path: PathBuf::from("~/pkm"),
                public: true,
                publish: true,
            },
            haystacks: vec![Haystack {
                path: PathBuf::from("localsearch"),
                service: ServiceType::Ripgrep,
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

        assert!(config.roles.contains_key("Father"));
        assert_eq!(config.roles.len(), 1);
        assert_eq!(&config.default_role, "Father");
        assert_eq!(config.roles["Father"], dummy_role());
    }

    #[tokio::test]
    async fn test_at_least_one_role() {
        let config = ConfigBuilder::new().build();
        assert!(config.is_err());
        assert!(matches!(config.unwrap_err(), TerraphimConfigError::NoRoles));
    }
}
