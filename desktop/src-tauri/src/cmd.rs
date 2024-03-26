// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use tauri::command;
use tauri::State;

use persistence::Persistable;
use terraphim_config::{Config, ConfigState};
use terraphim_middleware::thesaurus::create_thesaurus_from_haystack;
use terraphim_types::{Article, IndexedDocument, SearchQuery};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RequestBody {
    id: i32,
    name: String,
}

// Everything we return from commands must implement `Serialize`.
// This includes Errors and `anyhow`'s `Error` type doesn't implement it.
// See https://github.com/tauri-apps/tauri/discussions/3913
#[derive(Debug, thiserror::Error)]
pub enum TerraphimTauriError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] persistence::Error),
}

// Manually implement `Serialize` for our error type because some of the
// lower-level types don't implement it.
impl Serialize for TerraphimTauriError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = anyhow::Result<T, TerraphimTauriError>;

struct TerraphimService<'a> {
    config_state: State<'a, ConfigState>,
}

impl<'a> TerraphimService<'a> {
    /// Create a new TerraphimService
    pub fn new(config_state: State<'a, ConfigState>) -> Self {
        Self { config_state }
    }

    /// Create a thesaurus from a haystack
    pub async fn create_thesaurus(&self, search_query: SearchQuery) -> Result<()> {
        Ok(
            create_thesaurus_from_haystack(self.config_state.inner().clone(), search_query.clone())
                .await?,
        )
    }

    /// Search for articles in the haystacks
    pub async fn search_articles(&self, search_query: SearchQuery) -> Result<Vec<Article>> {
        let cached_articles = terraphim_middleware::search_haystacks(
            self.config_state.inner().clone(),
            search_query.clone(),
        )
        .await?;
        let docs: Vec<IndexedDocument> = self.config_state.search_articles(search_query).await;
        let articles = terraphim_types::merge_and_serialize(cached_articles, docs);

        Ok(articles)
    }

    /// Fetch the current config
    pub async fn fetch_config(&self) -> Result<terraphim_config::Config> {
        let current_config = self.config_state.config.lock().await;
        Ok(current_config.clone())
    }

    /// Update the current config
    pub async fn update_config(&self, config_new: Config) -> Result<terraphim_config::Config> {
        let mut config_state_lock = self.config_state.config.lock().await;
        config_state_lock.update(config_new.clone());
        config_state_lock.save().await?;
        Ok(config_state_lock.clone())
    }
}

/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<Vec<Article>> {
    log::info!("Search called with {:?}", search_query);
    let terraphim_service = TerraphimService::new(config_state);
    terraphim_service
        .create_thesaurus(search_query.clone())
        .await?;

    terraphim_service.search_articles(search_query).await
}

#[command]
pub async fn get_config(
    config_state: tauri::State<'_, ConfigState>,
) -> Result<terraphim_config::Config> {
    log::info!("Get config called");
    let terraphim_service = TerraphimService::new(config_state);
    terraphim_service.fetch_config().await
}

#[command]
pub async fn update_config(
    config_state: tauri::State<'_, ConfigState>,
    config_new: Config,
) -> Result<terraphim_config::Config> {
    log::info!("Update config called with {:?}", config_new);
    let terraphim_service = TerraphimService::new(config_state);
    terraphim_service.update_config(config_new).await
}
