use terraphim_pipeline::{Document};
use serde::{Deserialize, Serialize};
use terraphim_config::TerraphimConfig;
use persistance::Persistable;
use opendal::Result as OpendalResult;

/// Query type for searching documents in the `RoleGraph`.
/// It contains the search term, skip and limit parameters.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchQuery {
    pub search_term: String,
    pub skip: Option<usize>,
    pub limit: Option<usize>,
    pub  role: Option<String>,
}

/// Create article schema
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Article {
    pub id: Option<String>,
    pub stub: Option<String>,
    pub title: String,
    pub url: String,
    pub body: String,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl From<Article> for Document {
    fn from(val: Article) -> Self {
        // If the ID is not provided, generate a new one
        let id = match val.id {
            Some(id) => id,
            None => ulid::Ulid::new().to_string(),
        };

        Document {
            id,
            title: val.title,
            body: Some(val.body),
            description: val.description,
        }
    }
}

use terraphim_pipeline::{RoleGraph, IndexedDocument};
use tokio::sync::Mutex;
use anyhow::{Result};


use std::sync::Arc;
use std::collections::HashMap;


/// ConfigState for the Terraphim (Actor)
/// Config state can be updated using the API or Atomic Server
#[derive(Default, Debug, Clone)]
pub struct ConfigState {
    /// Terraphim Config
    pub config: Arc<Mutex<TerraphimConfig>>,
    pub roles: HashMap<String, RoleGraphState>
}

impl ConfigState {

    pub async fn new() -> Result<Self> {

        let mut config=TerraphimConfig::new();
        // Try to load the existing state
        // FIXMME: use better error handling
        if let Ok(config)=config.load("configstate").await {
            println!("config loaded");
        }
        let mut config_state= ConfigState {
            config: Arc::new(Mutex::new(config.clone())),
            roles: HashMap::new()
        };
    
        // for each role in a config initialize a rolegraph
        // and add it to the config state
        for (role_name,each_role) in config.roles {
            let automata_url= each_role.kg.automata_url.as_str();
            println!("{} - {}",role_name.clone(),automata_url);
            let rolegraph = RoleGraph::new(role_name.clone(), automata_url).await?;        
            config_state.roles.insert(role_name, RoleGraphState {
                rolegraph: Arc::new(Mutex::new(rolegraph))
            });
    
        }
        Ok(config_state)
    }

}

#[derive(Debug, Clone)]
pub struct RoleGraphState {
    /// RoleGraph for ingesting documents
    pub rolegraph: Arc<Mutex<RoleGraph>>,
}

