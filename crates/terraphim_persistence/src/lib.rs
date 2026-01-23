pub mod compression;
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
use tracing::{debug_span, Instrument};

use std::collections::HashMap;
use std::sync::Arc;
use terraphim_types::Document;

use crate::compression::{maybe_compress, maybe_decompress};

/// Expand tilde (~) in paths to the user's home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    } else if path == "~" {
        if let Ok(home) = std::env::var("HOME") {
            return home;
        }
    }
    path.to_string()
}

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

    log::debug!("Loading settings from: {:?}", settings_path);
    let settings = DeviceSettings::load_from_env_and_file(Some(settings_path.clone()))?;
    log::debug!(
        "Loaded settings with {} profiles: {:?}",
        settings.profiles.len(),
        settings.profiles.keys().collect::<Vec<_>>()
    );
    init_device_storage_with_settings(settings).await
}

async fn init_device_storage_with_settings(settings: DeviceSettings) -> Result<DeviceStorage> {
    log::info!("Loaded settings: {:?}", settings);

    // Pre-create directories for storage backends that need them
    // Expand tilde in paths to support home directory references
    for profile in settings.profiles.values() {
        let unknown = "unknown".to_string();
        let profile_type = profile.get("type").unwrap_or(&unknown);
        match profile_type.as_str() {
            "sqlite" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        let expanded = expand_tilde(datadir);
                        log::info!("Pre-creating SQLite directory: {}", expanded);
                        if let Err(e) = std::fs::create_dir_all(&expanded) {
                            log::warn!("Failed to create SQLite directory '{}': {}", expanded, e);
                        } else {
                            log::info!("Created SQLite directory: {}", expanded);
                        }
                    }
                }
            }
            "redb" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        let expanded = expand_tilde(datadir);
                        log::info!("Pre-creating ReDB directory: {}", expanded);
                        if let Err(e) = std::fs::create_dir_all(&expanded) {
                            log::warn!("Failed to create ReDB directory '{}': {}", expanded, e);
                        } else {
                            log::info!("Created ReDB directory: {}", expanded);
                        }
                    }
                }
            }
            "dashmap" => {
                if let Some(root) = profile.get("root") {
                    if !root.is_empty() {
                        let expanded = expand_tilde(root);
                        log::info!("Pre-creating DashMap directory: {}", expanded);
                        if let Err(e) = std::fs::create_dir_all(&expanded) {
                            log::warn!("Failed to create DashMap directory '{}': {}", expanded, e);
                        } else {
                            log::info!("Created DashMap directory: {}", expanded);
                        }
                    }
                }
            }
            "rocksdb" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        let expanded = expand_tilde(datadir);
                        log::info!("Pre-creating RocksDB directory: {}", expanded);
                        if let Err(e) = std::fs::create_dir_all(&expanded) {
                            log::warn!("Failed to create RocksDB directory '{}': {}", expanded, e);
                        } else {
                            log::info!("Created RocksDB directory: {}", expanded);
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

    /// Load from operators with fallback mechanism and cache warm-up
    ///
    /// This function tries to load the object from storage backends in speed order.
    /// If the fastest operator fails, it will try the next fastest, and so on.
    /// When data is successfully loaded from a fallback (slower) operator,
    /// it is asynchronously written to the fastest operator for future access.
    ///
    /// # Cache Write-back Behavior
    /// - Non-blocking: Uses tokio::spawn for fire-and-forget
    /// - Best-effort: Failures logged at debug level, don't affect load
    /// - Compressed: Objects over 1MB are compressed with zstd
    /// - Schema evolution: If cached data fails to deserialize, cache is cleared and refetched
    async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
    where
        Self: Sized,
    {
        let span = debug_span!("load_from_operator", key = %key);
        async {
            let (ops, fastest_op) = &self.load_config().await?;

            // Helper to check existence and read from an operator with decompression support
            async fn try_read_from_op<T: DeserializeOwned>(
                op: &Operator,
                key: &str,
                profile_name: Option<&str>,
            ) -> Option<std::result::Result<T, Error>> {
                let span = debug_span!("try_read", profile = ?profile_name);
                async {
                    // Use stat() first to check existence - this doesn't trigger WARN-level logging
                    match op.stat(key).await {
                        Ok(_) => {
                            // File exists, proceed with read
                            match op.read(key).await {
                                Ok(bs) => {
                                    // Try to decompress if needed
                                    let data = match maybe_decompress(&bs.to_vec()) {
                                        Ok(decompressed) => decompressed,
                                        Err(e) => {
                                            log::debug!("Decompression failed for '{}', using raw data: {}", key, e);
                                            bs.to_vec()
                                        }
                                    };

                                    match serde_json::from_slice(&data) {
                                        Ok(obj) => {
                                            if let Some(name) = profile_name {
                                                log::debug!("Loaded '{}' from profile '{}'", key, name);
                                            } else {
                                                log::debug!("Loaded '{}' from fastest operator (cache hit)", key);
                                            }
                                            Some(Ok(obj))
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to deserialize '{}': {}", key, e);
                                            Some(Err(Error::Json(e)))
                                        }
                                    }
                                },
                                Err(e) => {
                                    log::debug!("Failed to read '{}' after stat: {}", key, e);
                                    Some(Err(e.into()))
                                }
                            }
                        }
                        Err(e) if e.kind() == opendal::ErrorKind::NotFound => {
                            // File doesn't exist - this is expected on first run, log at debug
                            log::debug!("File '{}' not found in storage (cache miss)", key);
                            None
                        }
                        Err(e) => {
                            log::debug!("Failed to stat '{}': {}", key, e);
                            Some(Err(e.into()))
                        }
                    }
                }.instrument(span).await
            }

            // First try the fastest operator
            let schema_evolution_detected = {
                let fastest_result = try_read_from_op::<Self>(fastest_op, key, None).await;

                // Process the result - consume it fully before any awaits
                match fastest_result {
                    Some(Ok(obj)) => return Ok(obj),
                    Some(Err(Error::Json(_))) => true,   // Schema evolution detected
                    Some(Err(_)) => false,              // Other error, try fallback
                    None => false,                      // Not found, try fallback
                }
                // fastest_result is dropped here
            };

            // Handle schema evolution outside the scope to avoid Send issues
            if schema_evolution_detected {
                log::info!(
                    "Schema evolution detected for '{}': clearing cache and refetching",
                    key
                );
                let delete_span = debug_span!("cache_clear", key = %key);
                async {
                    if let Err(e) = fastest_op.delete(key).await {
                        log::debug!("Failed to delete stale cache entry '{}': {}", key, e);
                    } else {
                        log::debug!("Deleted stale cache entry '{}'", key);
                    }
                }.instrument(delete_span).await;
            }

            // If fastest operator failed or file not found, try all operators in speed order
            let mut ops_vec: Vec<(&String, &(Operator, u128))> = ops.iter().collect();
            ops_vec.sort_by_key(|&(_, (_, speed))| speed);

            for (profile_name, (op, _speed)) in ops_vec {
                // Skip if this is the same as the fastest operator we already tried
                if std::ptr::eq(op as *const Operator, fastest_op as *const Operator) {
                    continue;
                }

                log::debug!("Trying to load '{}' from profile '{}'", key, profile_name);

                if let Some(result) = try_read_from_op::<Self>(op, key, Some(profile_name)).await {
                    match result {
                        Ok(obj) => {
                            log::info!(
                                "Successfully loaded '{}' from fallback profile '{}'",
                                key,
                                profile_name
                            );

                            // Cache write-back: write to fastest operator (non-blocking)
                            // Only if fastest_op is different from current operator (already checked above)
                            if let Ok(serialized) = serde_json::to_vec(&obj) {
                                let fastest = fastest_op.clone();
                                let k = key.to_string();
                                let data_len = serialized.len();

                                tokio::spawn(async move {
                                    let span = debug_span!("cache_writeback", key = %k, size = data_len);
                                    async {
                                        // Compress large objects
                                        let data = maybe_compress(&serialized);
                                        let compressed = data.len() < serialized.len();

                                        match fastest.write(&k, data).await {
                                            Ok(_) => {
                                                if compressed {
                                                    log::debug!(
                                                        "Cached '{}' to fastest operator ({} bytes compressed)",
                                                        k,
                                                        data_len
                                                    );
                                                } else {
                                                    log::debug!(
                                                        "Cached '{}' to fastest operator ({} bytes)",
                                                        k,
                                                        data_len
                                                    );
                                                }
                                            }
                                            Err(e) => {
                                                log::debug!("Cache write-back failed for '{}': {}", k, e);
                                            }
                                        }
                                    }.instrument(span).await
                                });
                            }

                            return Ok(obj);
                        }
                        Err(Error::Json(_)) => {
                            // Deserialization error, continue to next
                        }
                        Err(_) => {
                            // Other error, continue to next
                        }
                    }
                }
            }

            // If all operators failed, return NotFound error (no WARN logged)
            log::debug!("Config file '{}' not found in any storage backend", key);
            Err(Error::NotFound(key.to_string()))
        }.instrument(span).await
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
/// It uses `tokio::task::JoinSet` to load documents concurrently, which significantly
/// improves performance when loading many documents from slower storage backends.
/// Returns a vector of successfully loaded documents.
pub async fn load_documents_by_ids(document_ids: &[String]) -> Result<Vec<Document>> {
    log::debug!(
        "Loading {} documents by IDs: {:?}",
        document_ids.len(),
        document_ids
    );

    let mut set = tokio::task::JoinSet::new();

    for doc_id in document_ids {
        let id = doc_id.clone();
        set.spawn(async move {
            let mut doc = Document::new(id.clone());
            match doc.load().await {
                Ok(loaded_doc) => {
                    log::trace!("Successfully loaded document: {}", id);
                    Some(loaded_doc)
                }
                Err(e) => {
                    log::warn!("Failed to load document '{}': {}", id, e);
                    None
                }
            }
        });
    }

    let mut documents = Vec::with_capacity(document_ids.len());
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Some(doc)) => documents.push(doc),
            Ok(None) => {} // Document failed to load, already logged
            Err(e) => log::error!("Join error in load_documents_by_ids: {}", e),
        }
    }

    log::debug!(
        "Successfully loaded {} out of {} documents",
        documents.len(),
        document_ids.len()
    );
    Ok(documents)
}
