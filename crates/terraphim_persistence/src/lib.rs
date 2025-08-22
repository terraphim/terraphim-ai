pub mod document;
pub mod error;
pub mod memory;
pub mod settings;
pub mod thesaurus;

use async_once_cell::OnceCell as AsyncOnceCell;
use async_trait::async_trait;
use opendal::Operator;
use serde::{de::DeserializeOwned, Serialize};
use terraphim_settings::DeviceSettings;

use std::collections::HashMap;
use terraphim_types::Document;

pub use error::{Error, Result};

static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

pub struct DeviceStorage {
    pub ops: HashMap<String, (Operator, u128)>,
    pub fastest_op: Operator,
}

impl DeviceStorage {
    pub async fn instance() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let initialized_storage = init_device_storage().await?;
                Ok::<DeviceStorage, Error>(initialized_storage)
            })
            .await?;
        Ok(storage)
    }

    /// Initialize device storage with memory-only settings for tests
    ///
    /// This is useful for tests that don't want to use filesystem or external services
    pub async fn init_memory_only() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let settings = memory::create_memory_only_device_settings()?;
                let initialized_storage = init_device_storage_with_settings(settings).await?;
                Ok::<DeviceStorage, Error>(initialized_storage)
            })
            .await?;
        Ok(storage)
    }
}

async fn init_device_storage() -> Result<DeviceStorage> {
    // Use local dev settings by default to avoid RocksDB lock issues
    let settings_path = std::env::var("TERRAPHIM_SETTINGS_PATH")
        .map(|p| std::path::PathBuf::from(p))
        .unwrap_or_else(|_| {
            // Default to local dev settings directory (not file)
            std::path::PathBuf::from("crates/terraphim_settings/default")
        });
    
    let settings = DeviceSettings::load_from_env_and_file(Some(settings_path))?;
    init_device_storage_with_settings(settings).await
}

async fn init_device_storage_with_settings(settings: DeviceSettings) -> Result<DeviceStorage> {
    log::info!("Loaded settings: {:?}", settings);
    
    // Pre-create directories for storage backends that need them
    for (_profile_name, profile) in &settings.profiles {
        let unknown = "unknown".to_string();
        let profile_type = profile.get("type").unwrap_or(&unknown);
        match profile_type.as_str() {
            "sqlite" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        log::info!("🔧 Pre-creating SQLite directory: {}", datadir);
                        if let Err(e) = std::fs::create_dir_all(datadir) {
                            log::warn!("Failed to create SQLite directory '{}': {}", datadir, e);
                        } else {
                            log::info!("✅ Created SQLite directory: {}", datadir);
                        }
                    }
                }
            }
            "redb" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        log::info!("🔧 Pre-creating ReDB directory: {}", datadir);
                        if let Err(e) = std::fs::create_dir_all(datadir) {
                            log::warn!("Failed to create ReDB directory '{}': {}", datadir, e);
                        } else {
                            log::info!("✅ Created ReDB directory: {}", datadir);
                        }
                    }
                }
            }
            "dashmap" => {
                if let Some(root) = profile.get("root") {
                    if !root.is_empty() {
                        log::info!("🔧 Pre-creating DashMap directory: {}", root);
                        if let Err(e) = std::fs::create_dir_all(root) {
                            log::warn!("Failed to create DashMap directory '{}': {}", root, e);
                        } else {
                            log::info!("✅ Created DashMap directory: {}", root);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let operators = settings::parse_profiles(&settings).await?;
    let mut ops_vec: Vec<(&String, &(Operator, u128))> = operators.iter().collect();
    ops_vec.sort_by_key(|&(_, (_, speed))| speed);

    let ops: HashMap<String, (Operator, u128)> = ops_vec
        .into_iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let fastest_op = match ops.values().next() {
        Some((op, _)) => op.clone(),
        None => return Err(Error::NoOperator),
    };

    Ok(DeviceStorage { ops, fastest_op })
}

/// A trait for persisting objects
///
/// This trait is used to save and load objects to and from the fastest operator
/// An operator is a storage backend that implements the `opendal::Operator`
/// trait, such as a file system, a database, or a cloud storage service.
#[async_trait]
pub trait Persistable: Serialize + DeserializeOwned {
    /// Create a new instance
    fn new(key: String) -> Self;

    /// Save to all profiles
    async fn save(&self) -> Result<()>;

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> Result<()>;

    /// Load a key from the fastest operator
    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized;

    /// Load the configuration
    async fn load_config(&self) -> Result<(HashMap<String, (Operator, u128)>, Operator)> {
        let state = DeviceStorage::instance().await?;
        Ok((state.ops.clone(), state.fastest_op.clone()))
    }

    /// Save to all profiles
    async fn save_to_all(&self) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self)?;
        for (op, _time) in ops.values() {
            log::debug!("Saving to operator: {:?}", op);
            op.write(&key, serde_str.clone()).await?;
        }
        Ok(())
    }

    /// Save to a single profile
    async fn save_to_profile(&self, profile_name: &str) -> Result<()> {
        let (ops, _fastest_op) = &self.load_config().await?;
        let key = self.get_key();
        let serde_str = serde_json::to_string(&self)?;

        ops.get(profile_name)
            .ok_or_else(|| {
                Error::Profile(format!(
                    "Unknown profile name: {profile_name}. Available profiles: {}",
                    ops.keys()
                        .map(|k| k.as_str())
                        .collect::<Vec<&str>>()
                        .join(", ")
                ))
            })?
            .0
            .write(&key, serde_str.clone())
            .await
            .map_err(Error::OpenDal)?;

        Ok(())
    }

    /// Load from the fastest operator
    async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
    where
        Self: Sized,
    {
        let (_ops, fastest_op) = &self.load_config().await?;
        let bs = fastest_op.read(key).await?;
        let obj = serde_json::from_slice(&bs)?;
        Ok(obj)
    }

    fn get_key(&self) -> String;
    fn normalize_key(&self, key: &str) -> String {
        let re = regex::Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create regex");
        re.replace_all(key, "").to_lowercase()
    }
}

/// Load multiple documents by their IDs
///
/// This function efficiently loads multiple documents using their IDs.
/// It attempts to load each document, but continues processing even if some documents fail to load.
/// Returns a vector of successfully loaded documents.
pub async fn load_documents_by_ids(document_ids: &[String]) -> Result<Vec<Document>> {
    log::debug!(
        "Loading {} documents by IDs: {:?}",
        document_ids.len(),
        document_ids
    );

    let mut documents = Vec::new();

    for doc_id in document_ids {
        let mut doc = Document::new(doc_id.clone());
        match doc.load().await {
            Ok(loaded_doc) => {
                documents.push(loaded_doc);
                log::trace!("Successfully loaded document: {}", doc_id);
            }
            Err(e) => {
                log::warn!("Failed to load document '{}': {}", doc_id, e);
                // Continue processing other documents even if this one fails
            }
        }
    }

    log::debug!(
        "Successfully loaded {} out of {} documents",
        documents.len(),
        document_ids.len()
    );
    Ok(documents)
}
