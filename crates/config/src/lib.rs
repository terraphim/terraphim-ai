use log::debug;
use log::info;
use opendal::layers::LoggingLayer;
use opendal::services;
use opendal::Operator;
use opendal::Result;
use opendal::Scheme;
use serde::de;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fs;

#[derive(Debug, Serialize, Deserialize,Clone)]
pub enum RelevanceFunction {
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    #[serde(rename = "redis-search")]
    RedisSearch,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum KnowledgeGraphType {
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TerraphimConfig {
    pub global_shortcut: String,
    pub roles: HashMap<String, Role>,
}

#[derive(Debug, Serialize, Deserialize,Clone)]
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

#[derive(Debug, Serialize, Deserialize,Clone)]
pub struct Haystack {
    haystack: String,
    service: String,
}

#[derive(Debug, Serialize, Deserialize,Clone)]
pub struct KnowledgeGraph {
    automata_url: String,
    //"markdown" or "json
    kg_type: KnowledgeGraphType,
    kg_path: String,
    public: bool,
    publish: bool,
}

impl TerraphimConfig {
    pub fn init() -> Self {
        let mut roles=HashMap::new();
            let haystack = Haystack {
            haystack: "localsearch".to_string(),
            service: "ripgrep".to_string(),
        };
        let kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let role = Role {
            shortname: Some("Default".to_string()),
            name: "Default".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "spacelab".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg,
            haystacks: vec![haystack],
            extra: HashMap::new(),
        };
        roles.insert("Default".to_string(), role);
        let kg_engineer = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let eng_haystack = Haystack {
            haystack: "localsearch".to_string(),
            service: "ripgrep".to_string(),
        };
        let engineer= Role {
            shortname: Some("Engineer".to_string()),
            name: "Engineer".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme:"lumen".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg: kg_engineer.clone(),
            haystacks: vec![eng_haystack.clone()],
            extra: HashMap::new(),
        };
        roles.insert("Engineer".to_string(), engineer);
        let system_operator_kg = KnowledgeGraph {
            automata_url: "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json"
                .to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let system_operator_haystack = Haystack {
            haystack: "localsearch".to_string(),
            service: "ripgrep".to_string(),
        };
        let system_operator= Role {
            shortname: Some("operator".to_string()),
            name: "System Operator".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme:"superhero".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg: system_operator_kg,
            haystacks: vec![system_operator_haystack],
            extra: HashMap::new(),
        };
        roles.insert("System Operator".to_string(), system_operator);


        Self {
            /// global shortcut for terraphim desktop
            global_shortcut: "Ctrl+X".to_string(),
            roles,
        }
    }
    pub fn update(&mut self, new_config: TerraphimConfig) {
        self.global_shortcut = new_config.global_shortcut;
        self.roles = new_config.roles;
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;


    #[test]
    fn test_write_config_to_json() {
        let config = TerraphimConfig::init();
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
    }
    #[test]
    fn test_write_config_to_toml() {
        let config = TerraphimConfig::init();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    fn test_init_global_config_to_toml() {
        let mut config = TerraphimConfig::init();
        config.global_shortcut="Ctrl+/".to_string();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config_shortcut.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
    #[test]
    fn test_update_global() {
        let mut config = TerraphimConfig::init();
        config.global_shortcut = "Ctrl+/".to_string();

        let mut new_config = TerraphimConfig::init();
        new_config.global_shortcut = "Ctrl+.".to_string();

        config.update(new_config);

        assert_eq!(config.global_shortcut, "Ctrl+.");
    }
    #[test]
    fn test_update_roles(){
        let mut config=TerraphimConfig::init();
        let mut new_config=TerraphimConfig::init();
        let new_role= Role {
            shortname: Some("farther".to_string()),
            name: "Farther".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme:"lumen".to_string(),
            server_url: "https://localhost:8080".to_string(),
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
                service: "ripgrep".to_string(),
            }],
            extra: HashMap::new(),
        };
        new_config.roles.insert("Father".to_string(),new_role);
        config.update(new_config);
        assert!(config.roles.contains_key("Father"));
        let json_str = serde_json::to_string_pretty(&config).unwrap();
        let mut file = File::create("test-data/config_updated.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
        // assert_eq!(config.roles.len(),4);

    }

}