// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use service::TerraphimService;
use tauri::command;
use tauri::State;

use terraphim_config::{Config, ConfigState};
use terraphim_types::{Document, SearchQuery};

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

    #[error("Service error: {0}")]
    Service(#[from] service::ServiceError),
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

/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<Vec<Document>> {
    log::info!("Search called with {:?}", search_query);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    Ok(terraphim_service.search_documents(&search_query).await?)
}

#[command]
pub async fn get_config(
    config_state: tauri::State<'_, ConfigState>,
) -> Result<terraphim_config::Config> {
    log::info!("Get config called");
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    Ok(terraphim_service.fetch_config().await)
}

#[command]
pub async fn update_config(
    config_state: tauri::State<'_, ConfigState>,
    config_new: Config,
) -> Result<terraphim_config::Config> {
    log::info!("Update config called with {:?}", config_new);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    Ok(terraphim_service.update_config(config_new).await?)
}
