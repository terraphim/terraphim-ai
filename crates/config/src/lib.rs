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

#[derive(Debug, Serialize, Deserialize)]
pub enum RelevanceFunction {
    #[serde(rename = "terraphim-graph")]
    TerraphimGraph,
    #[serde(rename = "redis-search")]
    RedisSearch,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Haystack {
    haystack: String,
    service: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    automata_url: String,
    //"markdown" or "json
    kg_type: KnowledgeGraphType,
    kg_path: String,
    public: bool,
    publish: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_write_config_to_json() {
        let mut config = TerraphimConfig {
            global_shortcut: "Ctrl+X".to_string(),
            roles: HashMap::new(),
        };
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
            theme: "Default".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg: kg,
            haystacks: vec![haystack],
            extra: HashMap::new(),
        };
        config.roles.insert("Default".to_string(), role);
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
            theme:"spacelab".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg: kg_engineer,
            haystacks: vec![eng_haystack],
            extra: HashMap::new(),
        };
        let json_str = serde_json::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.json").unwrap();
        file.write_all(json_str.as_bytes()).unwrap();
    }
    #[test]
    fn test_write_config_to_toml() {
        let mut config = TerraphimConfig {
            global_shortcut: "Ctrl+X".to_string(),
            roles: HashMap::new(),
        };
        let haystack = Haystack {
            haystack: "localsearch".to_string(),
            service: "ripgrep".to_string(),
        };
        let kg = KnowledgeGraph {
            automata_url: "https://localhost/kg_url".to_string(),
            kg_type: KnowledgeGraphType::Markdown,
            kg_path: "~/pkm".to_string(),
            public: true,
            publish: true,
        };
        let role = Role {
            shortname: Some("Default".to_string()),
            name: "Default".to_string(),
            relevance_function: RelevanceFunction::TerraphimGraph,
            theme: "Default".to_string(),
            server_url: "https://localhost:8080".to_string(),
            kg: kg,
            haystacks: vec![haystack],
            extra: HashMap::new(),
        };
        config.roles.insert("Default".to_string(), role);
        let toml_str = toml::to_string_pretty(&config).unwrap();

        let mut file = File::create("test-data/config.toml").unwrap();
        file.write_all(toml_str.as_bytes()).unwrap();
    }
}
