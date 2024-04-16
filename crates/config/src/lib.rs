use std::{path::PathBuf, sync::Arc};

use ahash::AHashMap;
use async_trait::async_trait;
use persistence::Persistable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use terraphim_automata::load_thesaurus;
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub shortname: Option<String>,
    pub name: String,
    /// The relevance function used to rank search results
    // TODO: use this
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
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ServiceType {
    /// Use ripgrep as the indexing service
    Ripgrep,
}

/// A haystack is a collection of documents that can be indexed and searched
///
/// One user can have multiple haystacks
/// Each haystack is indexed using a specific service
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Haystack {
    /// The path to the haystack
    pub path: PathBuf,
    /// The service used for indexing documents in the haystack
    pub service: ServiceType,
}

/// A knowledge graph is the collection of documents which were indexed
/// using a specific service
// TODO: Make the fields private once `TerraphimConfig` is more flexible
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeGraph {
    pub automata_url: Url,
    pub input_type: KnowledgeGraphInputType,
    pub path: PathBuf,
    pub public: bool,
    pub publish: bool,
}

/// The Terraphim config is the main configuration for terraphim
/// It contains the global shortcut, roles, and default role
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Global shortcut for terraphim desktop
    pub global_shortcut: String,
    /// User roles with their respective settings
    pub roles: AHashMap<String, Role>,
    /// The default role to use if no role is specified
    pub default_role: String,
    /// Unique identifier for the config
    pub id: String,
}

impl Config {
    // TODO: In order to make the config more flexible, we should pass in the
    // roles from the outside. This way, we can define the service (ripgrep,
    // logseq, etc) for each role. This will allow us to support different
    // services for different roles more easily.
    pub fn new() -> Self {
        let mut roles = AHashMap::new();

        let kg = KnowledgeGraph {
            automata_url: Url::parse(
                "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
            )
            .unwrap(),
            input_type: KnowledgeGraphInputType::Markdown,
            path: PathBuf::from("~/pkm"),
            public: true,
            publish: true,
        };
        let haystack = Haystack {
            path: PathBuf::from("localsearch"),
            service: ServiceType::Ripgrep,
        };
        let default_role = Role {
            shortname: Some("Default".to_string()),
            name: "Default".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "spacelab".to_string(),
            server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
            kg,
            haystacks: vec![haystack],
            extra: AHashMap::new(),
        };
        roles.insert("Default".to_lowercase().to_string(), default_role);

        let engineer_kg = KnowledgeGraph {
            automata_url: Url::parse(
                "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
            )
            .unwrap(),
            input_type: KnowledgeGraphInputType::Markdown,
            path: PathBuf::from("~/pkm"),
            public: true,
            publish: true,
        };
        let engineer_haystack = Haystack {
            path: PathBuf::from("localsearch"),
            service: ServiceType::Ripgrep,
        };
        let engineer_role = Role {
            shortname: Some("Engineer".to_string()),
            name: "Engineer".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "lumen".to_string(),
            server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
            kg: engineer_kg,
            haystacks: vec![engineer_haystack],
            extra: AHashMap::new(),
        };
        roles.insert("Engineer".to_lowercase().to_string(), engineer_role);

        let system_operator_kg = KnowledgeGraph {
            automata_url: Url::parse(
                "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
            )
            .unwrap(),
            input_type: KnowledgeGraphInputType::Markdown,
            path: PathBuf::from("~/pkm"),
            public: true,
            publish: true,
        };
        let system_operator_haystack = Haystack {
            path: PathBuf::from("/tmp/system_operator/pages/"),
            service: ServiceType::Ripgrep,
        };
        let system_operator_role = Role {
            shortname: Some("operator".to_string()),
            name: "System Operator".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "superhero".to_string(),
            server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
            kg: system_operator_kg,
            haystacks: vec![system_operator_haystack],
            extra: AHashMap::new(),
        };
        roles.insert(
            "System Operator".to_lowercase().to_string(),
            system_operator_role,
        );

        Self {
            id: Ulid::new().to_string(),
            // global shortcut for terraphim desktop
            global_shortcut: "Ctrl+X".to_string(),
            roles,
            default_role: "default".to_string(),
        }
    }

    pub fn update(&mut self, new_config: Config) {
        self.global_shortcut = new_config.global_shortcut;
        self.roles = new_config.roles;
        self.default_role = new_config.default_role;
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Persistable for Config {
    fn new(_key: String) -> Self {
        // Key is not used because we use the `id` field
        Config::new()
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
    pub async fn new(config: &mut Config) -> Result<Self> {
        // For each role in a config, initialize a rolegraph
        // and add it to the config state
        let mut roles = AHashMap::new();
        for (name, role) in &config.roles {
            let role_name = name.to_lowercase();
            let automata_url = role.kg.automata_url.clone();
            log::info!("Loading Role {} - Url {}", role_name, automata_url);

            let thesaurus = load_thesaurus(automata_url).await?;
            let rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;
            roles.insert(role_name, RoleGraphSync::from(rolegraph));
        }

        Ok(ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles,
        })
    }

    /// Index document into all rolegraphs
    // TODO: This should probably be moved to the `persistance` crate
    pub async fn index_document(&mut self, document: &Document) -> OpendalResult<()> {
        let id = document.id.clone();

        for rolegraph_state in self.roles.values() {
            let mut rolegraph = rolegraph_state.lock().await;
            rolegraph.insert_document(&id, document.clone());
        }
        Ok(())
    }

    /// Search documents in rolegraph using the search query
    pub async fn search_documents(&self, search_query: &SearchQuery) -> Vec<IndexedDocument> {
        log::debug!("search_documents: {:?}", search_query);
        let current_config_state = self.config.lock().await.clone();
        let default_role = current_config_state.default_role.clone();

        // if role is not provided, use the default role from the config
        let role = search_query.role.clone().unwrap_or(default_role);
        log::debug!("Role for search_documents: {:#?}", role);

        let role = role.to_lowercase();
        let rolegraph = self.roles.get(&role).unwrap().lock().await;
        let documents: Vec<(String, IndexedDocument)> = match rolegraph.query(
            &search_query.search_term,
            search_query.skip,
            search_query.limit,
        ) {
            Ok(docs) => docs,
            Err(e) => {
                log::error!("Error: {}", e);
                return Vec::default();
            }
        };

        documents.into_iter().map(|(_id, doc)| doc).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tokio::test;

    #[test]
    async fn test_write_config_to_json() {
        let config = Config::new();
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
    }

    #[test]
    async fn test_get_key() {
        let config = Config::new();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
    }
    #[tokio::test]
    async fn test_save_all() {
        let config = Config::new();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        let _ = config.save().await.unwrap();
    }
    #[tokio::test]
    async fn test_save_one_s3() {
        let config = Config::new();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        config.save_to_one("s3").await.unwrap();
        assert!(true);
    }
    #[tokio::test]
    async fn test_save_one_sled() {
        let config = Config::new();
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        config.save_to_one("sled").await.unwrap();
        assert!(true);
    }

    #[test]
    async fn test_write_config_to_toml() {
        let config = Config::new();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    async fn test_init_global_config_to_toml() {
        let mut config = Config::new();
        config.global_shortcut = "Ctrl+/".to_string();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config_shortcut.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    async fn test_update_global() {
        let mut config = Config::new();
        config.global_shortcut = "Ctrl+/".to_string();

        let mut new_config = Config::new();
        new_config.global_shortcut = "Ctrl+.".to_string();

        config.update(new_config);

        assert_eq!(config.global_shortcut, "Ctrl+.");
    }
    #[test]
    async fn test_update_roles() {
        let mut config = Config::new();
        let mut new_config = Config::new();
        let new_role = Role {
            shortname: Some("father".to_string()),
            name: "Father".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "lumen".to_string(),
            server_url: Url::parse("http://localhost:8080").unwrap(),
            kg: KnowledgeGraph {
                automata_url: Url::parse(
                    "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
                )
                .unwrap(),
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
        };
        new_config.roles.insert("Father".to_string(), new_role);
        config.update(new_config);
        assert!(config.roles.contains_key("Father"));
        assert_eq!(config.roles.len(), 4);

        // Test serialization
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        let mut file = File::create("test-data/config_updated.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
    }
}
