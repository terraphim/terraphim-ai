use std::{path::PathBuf, sync::Arc};

use terraphim_automata::{load_thesaurus, AutomataPath};
use terraphim_persistence::Persistable;
use terraphim_rolegraph::{RoleGraph, RoleGraphSync};
use terraphim_types::{
    Document, IndexedDocument, KnowledgeGraphInputType, RelevanceFunction, RoleName, SearchQuery,
};

use ahash::AHashMap;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::Mutex;
use ulid::Ulid;

pub type Result<T> = std::result::Result<T, TerraphimConfigError>;

use opendal::Result as OpendalResult;

type PersistenceResult<T> = std::result::Result<T, terraphim_persistence::Error>;
use serde_json_any_key::*;

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
}

/// A role is a collection of settings for a specific user
///
/// It contains a user's knowledge graph, a list of haystacks, as
/// well as preferences for the relevance function and theme
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Role {
    pub shortname: Option<String>,
    pub name: RoleName,
    /// The relevance function used to rank search results
    pub relevance_function: RelevanceFunction,
    pub theme: String,
    pub kg: Option<KnowledgeGraph>,
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
    /// automata path refering to the published automata and can be online url or local file with pre-build automata
    pub automata_path: Option<AutomataPath>,
    /// Knowlege graph can be re-build from local files, for example Markdown files
    pub knowledge_graph_local: Option<KnowledgeGraphLocal>,
    pub public: bool,
    pub publish: bool,
}
/// check KG set correctly
impl KnowledgeGraph {
    fn is_set(&self) -> bool {
        self.automata_path.is_some() || self.knowledge_graph_local.is_some()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
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
}

impl ConfigBuilder {
    /// Create a new `ConfigBuilder`
    pub fn new() -> Self {
        Self {
            config: Config::new(None),
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

    /// Set the id for the config
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.config.id = id.into();
        self
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
    #[serde(with = "any_key_map")]
    pub roles: AHashMap<RoleName, Role>,
    /// The default role to use if no role is specified
    pub default_role: RoleName,
    pub selected_role: Option<RoleName>,
}

impl Config {
    fn new(id: Option<String>) -> Self {
        Self {
            id: id.unwrap_or_else(|| "default".to_string()),
            // global shortcut for terraphim desktop
            global_shortcut: "Ctrl+X".to_string(),
            roles: AHashMap::new(),
            default_role: RoleName::new("default"),
            selected_role: None,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Persistable for Config {
    fn new(key: String) -> Self {
        // Key is not used because we use the `id` field
        Config::new(Some(key))
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
            // FIXME: this looks like local KG is never re-build
            // check if role have configured local KG or automata_path
            // skip role if incorrectly configured
            if role.relevance_function == RelevanceFunction::TerraphimGraph {
                if role.kg.as_ref().is_some_and(|kg| kg.is_set()) {
                    //FIXME: turn into errors
                    log::info!("Role {} is configured correctly", role_name);
                    let automata_url = role
                        .kg
                        .as_ref()
                        .unwrap()
                        .automata_path
                        .as_ref()
                        .unwrap()
                        .clone();
                    log::info!("Loading Role `{}` - URL: {:?}", role_name, automata_url);
                    let thesaurus = load_thesaurus(&automata_url).await?;
                    let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;
                    roles.insert(role_name.clone(), RoleGraphSync::from(rolegraph));
                } else {
                    log::info!("Role {} is configured to use KG ranking but is missing remote url or local configuration", role_name );
                }
            }
        }

        Ok(ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles,
        })
    }
    /// get selected role
    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config.lock().await;
        config.selected_role.clone().unwrap()
    }

    /// Get the default role from the config
    pub async fn get_default_role(&self) -> RoleName {
        let config = self.config.lock().await;
        config.default_role.clone()
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
        let config = Config::new(None);
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut tempfile = tempfile().unwrap();
        tempfile.write_all(json_str.as_bytes()).unwrap();
    }

    #[test]
    async fn test_get_key() {
        let config = Config::new(None);
        serde_json::to_string_pretty(&config).unwrap();
        assert!(config.get_key().ends_with(".json"));
    }

    #[tokio::test]
    async fn test_save_all() {
        let config = Config::new(None);
        config.save().await.unwrap();
    }

    #[tokio::test]
    async fn test_save_one_s3() {
        let config = Config::new(None);
        config.save_to_one("s3").await.unwrap();
    }

    #[tokio::test]
    async fn test_save_one_sled() {
        let config = Config::new(None);
        config.save_to_one("sled").await.unwrap();
    }

    #[test]
    async fn test_write_config_to_toml() {
        let config = Config::new(None);
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
                    theme: "spacelab".to_string(),
                    kg: None,
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
                    name: "Engineer".into(),
                    relevance_function: RelevanceFunction::TitleScorer,
                    theme: "lumen".to_string(),
                    kg: None,
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
                    name: "System Operator".into(),
                    relevance_function: RelevanceFunction::TerraphimGraph,
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
        assert_eq!(config.default_role, RoleName::new("Default"));
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
            shortname: Some("father".into()),
            name: "Father".into(),
            relevance_function: RelevanceFunction::TitleScorer,
            theme: "lumen".to_string(),
            kg: Some(KnowledgeGraph {
                automata_path: Some(AutomataPath::local_example()),
                knowledge_graph_local: None,
                public: true,
                publish: true,
            }),
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

        assert!(config.roles.contains_key(&RoleName::new("Father")));
        assert_eq!(config.roles.len(), 1);
        assert_eq!(&config.default_role, &RoleName::new("Father"));
        assert_eq!(config.roles[&RoleName::new("Father")], dummy_role());
    }

    #[tokio::test]
    async fn test_at_least_one_role() {
        let config = ConfigBuilder::new().build();
        assert!(config.is_err());
        assert!(matches!(config.unwrap_err(), TerraphimConfigError::NoRoles));
    }

    #[test]
    async fn test_config_builder_with_id() {
        let config = ConfigBuilder::new()
            .id("custom_id".to_string())
            .global_shortcut("Ctrl+X")
            .add_role("Default", dummy_role())
            .build()
            .unwrap();


        assert_eq!(config.id, "custom_id");
    }

    #[test]
    async fn test_config_builder_default_id() {
        let config = ConfigBuilder::new()
            .global_shortcut("Ctrl+X")
            .add_role("Default", dummy_role())
            .build()
            .unwrap();

        assert_eq!(config.id, "default");
    }
}