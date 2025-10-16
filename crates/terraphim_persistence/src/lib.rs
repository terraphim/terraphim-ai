pub mod conversation;
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
use std::sync::Arc;
use terraphim_types::Document;

pub use error::{Error, Result};

static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

#[derive(Debug)]
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

    /// Get an Arc<DeviceStorage> instance safely
    ///
    /// This is a safe alternative to using unsafe ptr::read operations.
    /// It initializes storage if needed and returns an Arc clone.
    pub async fn arc_instance() -> Result<Arc<DeviceStorage>> {
        let storage_ref = Self::instance().await?;

        // Create a new DeviceStorage with cloned data rather than using unsafe code
        let safe_storage = DeviceStorage {
            ops: storage_ref.ops.clone(),
            fastest_op: storage_ref.fastest_op.clone(),
        };

        Ok(Arc::new(safe_storage))
    }

    /// Get an Arc<DeviceStorage> instance using memory-only backend safely
    ///
    /// This is a safe alternative to using unsafe ptr::read operations for tests.
    pub async fn arc_memory_only() -> Result<Arc<DeviceStorage>> {
        let storage_ref = Self::init_memory_only().await?;

        // Create a new DeviceStorage with cloned data rather than using unsafe code
        let safe_storage = DeviceStorage {
            ops: storage_ref.ops.clone(),
            fastest_op: storage_ref.fastest_op.clone(),
        };

        Ok(Arc::new(safe_storage))
    }
}

async fn init_device_storage() -> Result<DeviceStorage> {
    // Use local dev settings by default to avoid RocksDB lock issues
    let settings_path = std::env::var("TERRAPHIM_SETTINGS_PATH")
        .map(std::path::PathBuf::from)
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
    for profile in settings.profiles.values() {
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
            .map_err(|e| Error::OpenDal(Box::new(e)))?;

        Ok(())
    }

    /// Load from operators with fallback mechanism
    ///
    /// This function tries to load the object from storage backends in speed order.
    /// If the fastest operator fails, it will try the next fastest, and so on.
    /// This provides resilience when different storage backends have different content.
    async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
    where
        Self: Sized,
    {
        let (ops, fastest_op) = &self.load_config().await?;

        // First try the fastest operator as before
        match fastest_op.read(key).await {
            Ok(bs) => match serde_json::from_slice(&bs.to_vec()) {
                Ok(obj) => {
                    log::debug!("✅ Loaded '{}' from fastest operator", key);
                    return Ok(obj);
                }
                Err(e) => {
                    log::warn!(
                        "Failed to deserialize '{}' from fastest operator: {}",
                        key,
                        e
                    );
                }
            },
            Err(e) => {
                log::debug!(
                    "Failed to read '{}' from fastest operator: {}, trying fallback",
                    key,
                    e
                );
            }
        }

        // If fastest operator failed, try all operators in speed order
        let mut ops_vec: Vec<(&String, &(Operator, u128))> = ops.iter().collect();
        ops_vec.sort_by_key(|&(_, (_, speed))| speed);

        for (profile_name, (op, _speed)) in ops_vec {
            // Skip if this is the same as the fastest operator we already tried
            if std::ptr::eq(op as *const Operator, fastest_op as *const Operator) {
                continue;
            }

            log::debug!(
                "🔄 Trying to load '{}' from profile '{}'",
                key,
                profile_name
            );

            match op.read(key).await {
                Ok(bs) => match serde_json::from_slice(&bs.to_vec()) {
                    Ok(obj) => {
                        log::info!(
                            "✅ Successfully loaded '{}' from fallback profile '{}'",
                            key,
                            profile_name
                        );
                        return Ok(obj);
                    }
                    Err(e) => {
                        log::warn!(
                            "Failed to deserialize '{}' from profile '{}': {}",
                            key,
                            profile_name,
                            e
                        );
                    }
                },
                Err(e) => {
                    log::debug!(
                        "Failed to read '{}' from profile '{}': {}",
                        key,
                        profile_name,
                        e
                    );
                }
            }
        }

        // If all operators failed, return the original error from the fastest operator
        let bs = fastest_op.read(key).await?;
        let obj = serde_json::from_slice(&bs.to_vec())?;
        Ok(obj)
    }

    fn get_key(&self) -> String;
    fn normalize_key(&self, key: &str) -> String {
        // Replace non-alphanumeric characters with underscores to preserve semantic meaning
        let re = regex::Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to create regex");
        let normalized = re.replace_all(key, "_").to_lowercase();

        // Remove leading/trailing underscores and collapse multiple underscores
        let cleaned = normalized.trim_matches('_').to_string();
        let re_multi = regex::Regex::new(r"_+").expect("Failed to create regex");
        let final_key = re_multi.replace_all(&cleaned, "_").to_string();

        log::debug!("Key normalization: '{}' → '{}'", key, final_key);

        // Validate that the normalized key is filesystem-safe and reasonable
        if final_key.is_empty() {
            log::warn!(
                "Key normalization resulted in empty string for input: '{}'",
                key
            );
            // Fallback to hash if normalization fails completely
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            key.hash(&mut hasher);
            return format!("fallback_{:x}", hasher.finish());
        } else if final_key.len() > 200 {
            log::warn!(
                "Normalized key is very long ({} chars) for input: '{}' → '{}'",
                final_key.len(),
                key,
                final_key
            );
        } else if final_key.len() < key.len() / 3 && key.len() > 15 {
            log::debug!(
                "Key normalization significantly shortened: '{}' ({} chars) → '{}' ({} chars)",
                key,
                key.len(),
                final_key,
                final_key.len()
            );
        }

        final_key
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
