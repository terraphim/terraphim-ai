// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT


use tauri::command;
use tauri::State;

use terraphim_config::{Config, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_types::{Document, SearchQuery};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RequestBody {
    id: i32,
    name: String,
}


use serde::Serializer;
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub status: Status,
    pub message: String,
}



// Everything we return from commands must implement `Serialize`.
// This includes Errors and `anyhow`'s `Error` type doesn't implement it.
// See https://github.com/tauri-apps/tauri/discussions/3913
#[derive(Debug, thiserror::Error)]
pub enum TerraphimTauriError {
    #[error("An error occurred: {0}")]
    Middleware(#[from] terraphim_middleware::Error),

    #[error("Persistence error: {0}")]
    Persistence(#[from] terraphim_persistence::Error),

    #[error("Service error: {0}")]
    Service(#[from] terraphim_service::ServiceError),
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

/// Response type for showing the config
///
/// This is also used when updating the config
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigResponse {
    /// Status of the config fetch
    pub status: Status,
    /// The config
    pub config: Config,
}



/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<Vec<Document>> {
    log::info!("Search called with {:?}", search_query);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    Ok(terraphim_service.search(&search_query).await?)
}

#[command]
pub async fn get_config(
    config_state: tauri::State<'_, ConfigState>,
) -> Result<ConfigResponse> {
    log::info!("Get config called");
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config =terraphim_service.fetch_config().await;
    Ok(ConfigResponse {
        status: Status::Success,
        config,
    })
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
