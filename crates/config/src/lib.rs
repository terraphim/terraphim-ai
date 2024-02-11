use std::collections::HashMap;

use async_trait::async_trait;
use persistence::Persistable;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use ulid::Ulid;

pub type Result<T> = std::result::Result<T, TerraphimConfigError>;

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
    TracingSubscriber(Box<dyn std::error::Error>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum RelevanceFunction {
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    #[serde(rename = "redis-search")]
    RedisSearch,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum KnowledgeGraphType {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "json")]
    Json,
}

/// A role is a collection of settings for a specific user
///
/// It contains a user's knowledge graph, a list of haystacks, as
/// well as preferences for the relevance function and theme
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    pub shortname: Option<String>,
    pub name: String,
    pub relevance_function: RelevanceFunction,
    pub theme: String,
    #[serde(rename = "serverUrl")]
    pub server_url: String,
    pub kg: KnowledgeGraph,
    pub haystacks: Vec<Haystack>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// The service used for indexing documents
///
/// Each service assumes documents to be stored in a specific format
/// and uses a specific indexing algorithm
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ServiceType {
    /// Use logseq as the indexing service
    Logseq,
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
    pub haystack: String,
    /// The service used for indexing documents in the haystack
    pub service: ServiceType,
}

/// A knowledge graph is the collection of documents which were indexed
/// using a specific service
// TODO: Make the fields private once `TerraphimConfig` is more flexible
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeGraph {
    pub automata_url: String,
    pub kg_type: KnowledgeGraphType,
    pub kg_path: String,
    pub public: bool,
    pub publish: bool,
}

/// The TerraphimConfig is the main configuration for terraphim
/// It contains the global shortcut, roles, and default role
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerraphimConfig {
    /// Global shortcut for terraphim desktop
    pub global_shortcut: String,
    /// User roles with their respective settings
    pub roles: HashMap<String, Role>,
    /// The default role to use if no role is specified
    pub default_role: String,
    /// Unique identifier for the config
    // TODO: Make the fields private once `TerraphimConfig` is more flexible
    pub id: String,
}


impl TerraphimConfig {
    // TODO: In order to make the config more flexible, we should pass in the
    // roles from the outside. This way, we can define the service (ripgrep,
    // logseq, etc) for each role. This will allow us to support different
    // services for different roles more easily.
    // For now, we pass in the service type and use it for all roles.
    pub fn new(service: ServiceType) -> Self {
        let mut roles = HashMap::new();

        let kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let haystack = Haystack {
            haystack: "localsearch".to_string(),
            service,
        };
        let default_role = Role {
            shortname: Some("Default".to_string()),
            name: "Default".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "spacelab".to_string(),
            server_url: "http://localhost:8000/articles/search".to_string(),
            kg,
            haystacks: vec![haystack],
            extra: HashMap::new(),
        };
        roles.insert("Default".to_lowercase().to_string(), default_role);

        let engineer_kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let engineer_haystack = Haystack {
            haystack: "localsearch".to_string(),
            service,
        };
        let engineer_role = Role {
            shortname: Some("Engineer".to_string()),
            name: "Engineer".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "lumen".to_string(),
            server_url: "http://localhost:8000/articles/search".to_string(),
            kg: engineer_kg,
            haystacks: vec![engineer_haystack],
            extra: HashMap::new(),
        };
        roles.insert("Engineer".to_lowercase().to_string(), engineer_role);

        let system_operator_kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let system_operator_haystack = Haystack {
            haystack: "/tmp/system_operator/pages/".to_string(),
            service,
        };
        let system_operator_role = Role {
            shortname: Some("operator".to_string()),
            name: "System Operator".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "superhero".to_string(),
            server_url: "http://localhost:8000/articles/search".to_string(),
            kg: system_operator_kg,
            haystacks: vec![system_operator_haystack],
            extra: HashMap::new(),
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
            default_role: "Default".to_string(),
        }
    }

    pub fn update(&mut self, new_config: TerraphimConfig) {
        self.global_shortcut = new_config.global_shortcut;
        self.roles = new_config.roles;
        self.default_role = new_config.default_role;
    }
}

#[async_trait]
impl Persistable for TerraphimConfig {
    fn new() -> Self {
        TerraphimConfig::new(ServiceType::Ripgrep)
    }

    async fn save_to_one(&self, profile_name: &str) -> PersistenceResult<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // saves to all profiles
    async fn save(&self) -> PersistenceResult<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tokio::test;

    #[test]
    async fn test_write_config_to_json() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
    }

    #[test]
    async fn test_get_key() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
    }
    #[tokio::test]
    async fn test_save_all() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        let _ = config.save().await.unwrap();
    }
    #[tokio::test]
    async fn test_save_one_s3() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        config.save_to_one("s3").await.unwrap();
        assert!(true);
    }
    #[tokio::test]
    async fn test_save_one_sled() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        println!("json_str: {}", json_str);
        println!("key: {}", config.get_key());
        config.save_to_one("sled").await.unwrap();
        assert!(true);
    }

    #[test]
    async fn test_write_config_to_toml() {
        let config = TerraphimConfig::new(ServiceType::Ripgrep);
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    async fn test_init_global_config_to_toml() {
        let mut config = TerraphimConfig::new(ServiceType::Ripgrep);
        config.global_shortcut = "Ctrl+/".to_string();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config_shortcut.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    async fn test_update_global() {
        let mut config = TerraphimConfig::new(ServiceType::Ripgrep);
        config.global_shortcut = "Ctrl+/".to_string();

        let mut new_config = TerraphimConfig::new(ServiceType::Ripgrep);
        new_config.global_shortcut = "Ctrl+.".to_string();

        config.update(new_config);

        assert_eq!(config.global_shortcut, "Ctrl+.");
    }
    #[test]
    async fn test_update_roles() {
        let mut config = TerraphimConfig::new(ServiceType::Ripgrep);
        let mut new_config = TerraphimConfig::new(ServiceType::Ripgrep);
        let new_role = Role {
            shortname: Some("farther".to_string()),
            name: "Farther".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "lumen".to_string(),
            server_url: "http://localhost:8080".to_string(),
            kg: KnowledgeGraph {
                automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                    .to_string(),
                kg_type: KnowledgeGraphType::Markdown,
                kg_path: "~/pkm".to_string(),
                public: true,
                publish: true,
            },
            haystacks: vec![Haystack {
                haystack: "localsearch".to_string(),
                service: ServiceType::Ripgrep,
            }],
            extra: HashMap::new(),
        };
        new_config.roles.insert("Father".to_string(), new_role);
        config.update(new_config);
        assert!(config.roles.contains_key("Father"));
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        let mut file = File::create("test-data/config_updated.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
        // assert_eq!(config.roles.len(),4);
    }
}
