use tauri::command;
use tauri::State;

use serde::{Deserialize, Serialize};

use terraphim_config::{Config, ConfigState};
use terraphim_service::TerraphimService;
use terraphim_settings::DeviceSettings;
use terraphim_types::Thesaurus;
use terraphim_types::{Document, SearchQuery};

use serde::Serializer;
use schemars::schema_for;
use serde_json::Value;

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

/// Response type for showing the search results
///
/// This is used when searching for documents
/// and returning the results
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResponse {
    /// Status of the search
    pub status: Status,
    /// The search results
    pub results: Vec<Document>,
}

/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<SearchResponse> {
    log::info!("Search called with {:?}", search_query);
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let results = terraphim_service.search(&search_query).await?;
    Ok(SearchResponse {
        status: Status::Success,
        results,
    })
}

#[command]
pub async fn get_config(config_state: tauri::State<'_, ConfigState>) -> Result<ConfigResponse> {
    log::info!("Get config called");
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service.fetch_config().await;

    Ok(ConfigResponse {
        status: Status::Success,
        config: config,
    })
}

#[command]
pub async fn update_config(
    config_state: tauri::State<'_, ConfigState>,
    config_new: Config,
) -> Result<ConfigResponse> {
    log::info!("Update config called with {:?}", config_new);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service.update_config(config_new).await?;
    Ok(ConfigResponse {
        status: Status::Success,
        config: config,
    })
}

/// Command to expose thesaurus if publish=true in knowledge graph
#[command]
pub async fn publish_thesaurus(
    config_state: tauri::State<'_, ConfigState>,
    role_name: String,
) -> Result<Thesaurus> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let thesaurus = terraphim_service
        .ensure_thesaurus_loaded(&role_name.into())
        .await?;
    Ok(thesaurus)
}

use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct InitialSettings {
    data_folder: PathBuf,
    global_shortcut: String,
}
use tauri::async_runtime::Mutex;
use std::sync::Arc;

#[tauri::command]
pub async fn save_initial_settings( config_state: tauri::State<'_, ConfigState>,
    device_settings: tauri::State<'_, Arc<Mutex<DeviceSettings>>>,new_settings: InitialSettings) -> Result<()> {
    println!("Saving initial settings: {:?}", new_settings);
    println!("Device settings: {:?}", device_settings);
    let mut settings = device_settings.lock().await;
    let mut config = config_state.config.lock().await;
    let data_folder = PathBuf::from(&new_settings.data_folder);
    println!("Data folder: {:?}", data_folder);
    if !data_folder.exists() {
        println!("Data folder does not exist");
    }
    if !data_folder.is_dir() {
        println!("Selected path is not a folder");
    }
    
    // Here you would typically save these settings to a file or database
    // For this example, we'll just print them
    println!("Data folder: {:?}", new_settings.data_folder);
    println!("Global shortcut: {}", new_settings.global_shortcut);
    config.global_shortcut = new_settings.global_shortcut;
    let updated_config = config.clone();
    drop(config);  // Release the lock before calling update_config
    update_config(config_state.clone(), updated_config).await?;
    // settings.data_folder = data_folder;
    settings.update_initialized_flag(None, true).unwrap();
    drop(settings);
    Ok(())
}
use tauri::{Manager, Window};

#[tauri::command]
pub async fn close_splashscreen(window: Window) {
  // Close splashscreen
  window.get_window("splashscreen").expect("no window labeled 'splashscreen' found").close().unwrap();
  // Show main window
  window.get_window("main").expect("no window labeled 'main' found").show().unwrap();
}

/// Response type for a single document
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentResponse {
    pub status: Status,
    pub document: Option<Document>,
}

/// Create (or update) a document and persist it
#[command]
pub async fn create_document(
    config_state: State<'_, ConfigState>,
    document: Document,
) -> Result<DocumentResponse> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let doc = terraphim_service.create_document(document).await?;
    Ok(DocumentResponse {
        status: Status::Success,
        document: Some(doc),
    })
}

/// Fetch a single document by its ID (tries persistence first, then falls back to search)
#[command]
pub async fn get_document(
    config_state: State<'_, ConfigState>,
    document_id: String,
) -> Result<DocumentResponse> {
    let mut terraphim_service = TerraphimService::new(config_state.inner().clone());
    let doc_opt = terraphim_service.get_document_by_id(&document_id).await?;
    Ok(DocumentResponse {
        status: Status::Success,
        document: doc_opt,
    })
}

/// Get JSON Schema for Config for dynamic forms
#[command]
pub async fn get_config_schema() -> Result<Value> {
    let schema = schema_for!(Config);
    Ok(serde_json::to_value(&schema).expect("schema serialization"))
}

/// Select a role (change `selected_role`) without sending the entire config back and
/// forth. Returns the updated `Config` so the frontend can reflect the change.
#[command]
pub async fn select_role(
    config_state: State<'_, ConfigState>,
    role_name: String,
) -> Result<ConfigResponse> {
    log::info!("Select role called: {}", role_name);
    let terraphim_service = TerraphimService::new(config_state.inner().clone());
    let config = terraphim_service
        .update_selected_role(terraphim_types::RoleName::new(&role_name))
        .await?;

    Ok(ConfigResponse {
        status: Status::Success,
        config,
    })
}
