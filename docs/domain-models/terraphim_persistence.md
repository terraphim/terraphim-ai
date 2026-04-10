# terraphim_persistence - Multi-Backend Storage Abstraction

## Overview

`terraphim_persistence` provides a multi-backend storage abstraction layer for the Terraphim AI system. It supports multiple storage backends with automatic fallback, cache warm-up, and transparent compression. The crate uses the OpenDAL project for unified storage operations.

## Domain Model

### Core Concepts

#### DeviceStorage
Multi-backend storage manager with speed-based selection and caching.

```rust
pub struct DeviceStorage {
    pub ops: HashMap<String, (Operator, u128)>,
    pub fastest_op: Operator,
}
```

**Key Responsibilities:**
- Manage multiple storage backends
- Track backend performance
- Provide fastest backend selection
- Handle backend fallbacks

#### Operator
Storage backend implementation (OpenDAL abstraction).

```rust
// From opendal crate
pub trait Operator: Send + Sync {
    // Read/write operations
    // Metadata operations
    // Lifecycle management
}
```

**Common Implementations:**
- Memory: In-memory storage
- DashMap: Concurrent hashmap
- SQLite: File-based database
- ReDB: Embedded database
- S3: Cloud object storage

### Persistence Interface

#### Persistable
Trait for objects that can be saved and loaded from storage.

```rust
#[async_trait]
pub trait Persistable: Serialize + DeserializeOwned {
    /// Create a new instance
    fn new(key: String) -> Self;

    /// Save to all profiles
    async fn save(&self) -> Result<()>;

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> Result<()>;

    /// Load a key from fastest operator
    async fn load(&mut self) -> Result<Self>
    where
        Self: Sized;

    /// Load configuration
    async fn load_config(&self) -> Result<(HashMap<String, (Operator, u128)>, Operator)>;

    /// Save to all profiles
    async fn save_to_all(&self) -> Result<()>;

    /// Save to a single profile
    async fn save_to_profile(&self, profile_name: &str) -> Result<()>;
}
```

**Key Responsibilities:**
- Define save/load interface
- Support multi-profile writes
- Enable selective profile writes
- Provide configuration loading

## Data Models

### Storage Profiles

#### Profile
Storage backend configuration.

```rust
// Example profile structure (from DeviceSettings)
{
    "type": "memory",
    "root": "/path/to/storage"
}
```

**Profile Types:**
- **memory**: In-memory storage (fastest, no persistence)
- **dashmap**: Concurrent hashmap (fast, in-memory)
- **sqlite**: File-based database (persistent, moderate speed)
- **redb**: Embedded database (persistent, fast)
- **s3**: Cloud object storage (persistent, network latency)

### Compression

#### Compression Strategy
Automatic compression for large objects.

```rust
fn maybe_compress(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_THRESHOLD: usize = 1_000_000; // 1MB

    if data.len() > COMPRESSION_THRESHOLD {
        zstd::encode_all(data, 3).unwrap_or_else(|_| data.to_vec())
    } else {
        data.to_vec()
    }
}

fn maybe_decompress(data: &[u8]) -> Result<Vec<u8>> {
    // Try to decompress
    // Fall back to raw data if decompression fails
}
```

**Pattern:**
- Compress objects over 1MB
- Use zstd compression (level 3)
- Handle decompression gracefully
- Fall back to raw data on error

## Implementation Patterns

### Storage Initialisation

#### Multi-Backend Setup
```rust
async fn init_device_storage_with_settings(settings: DeviceSettings) -> Result<DeviceStorage> {
    // Pre-create directories for storage backends
    for profile in settings.profiles.values() {
        let profile_type = profile.get("type").unwrap_or(&"unknown");
        match profile_type.as_str() {
            "sqlite" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        let expanded = expand_tilde(datadir);
                        std::fs::create_dir_all(&expanded)?;
                    }
                }
            }
            "redb" => {
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        let expanded = expand_tilde(datadir);
                        std::fs::create_dir_all(&expanded)?;
                    }
                }
            }
            "dashmap" => {
                if let Some(root) = profile.get("root") {
                    if !root.is_empty() {
                        let expanded = expand_tilde(root);
                        std::fs::create_dir_all(&expanded)?;
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
```

**Pattern:**
- Parse configuration profiles
- Sort by speed (lower = faster)
- Select fastest as default
- Create operator map

#### Singleton Pattern
```rust
static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

impl DeviceStorage {
    pub async fn instance() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let initialised_storage = init_device_storage().await?;
                Ok::<DeviceStorage, Error>(initialised_storage)
            })
            .await?;
        Ok(storage)
    }

    pub async fn init_memory_only() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let settings = memory::create_memory_only_device_settings()?;
                let initialised_storage = init_device_storage_with_settings(settings).await?;
                Ok::<DeviceStorage, Error>(initialised_storage)
            })
            .await?;
        Ok(storage)
    }
}
```

**Pattern:**
- Use `AsyncOnceCell` for lazy initialisation
- Support normal and memory-only modes
- Return shared reference
- Initialise once per process

### Multi-Backend Operations

#### Save to All Profiles
```rust
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
```

**Pattern:**
- Load configuration
- Serialise object
- Write to all profiles
- Handle failures gracefully

#### Load with Fallback
```rust
async fn load_from_operator(&self, key: &str, _op: &Operator) -> Result<Self>
where
    Self: Sized,
{
    let span = debug_span!("load_from_operator", key = %key);
    async {
        let (ops, fastest_op) = &self.load_config().await?;

        async fn try_read_from_op<T: DeserializeOwned>(
            op: &Operator,
            key: &str,
            profile_name: Option<&str>,
        ) -> Option<std::result::Result<T, Error>> {
            let span = debug_span!("try_read", profile = ?profile_name);
            async {
                match op.stat(key).await {
                    Ok(_) => {
                        match op.read(key).await {
                            Ok(bs) => {
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
                            }
                            Err(e) => {
                                log::debug!("Failed to read '{}' after stat: {}", key, e);
                                Some(Err(e.into()))
                            }
                        }
                    }
                    Err(e) if e.kind() == opendal::ErrorKind::NotFound => {
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

        // First try fastest operator
        let schema_evolution_detected = {
            let fastest_result = try_read_from_op::<Self>(fastest_op, key, None).await;

            match fastest_result {
                Some(Ok(obj)) => return Ok(obj),
                Some(Err(Error::Json(_))) => true,
                Some(Err(_)) => false,
                None => false,
            }
        };

        // Handle schema evolution
        if schema_evolution_detected {
            log::info!("Schema evolution detected for '{}': clearing cache and refetching", key);
            let delete_span = debug_span!("cache_clear", key = %key);
            async {
                if let Err(e) = fastest_op.delete(key).await {
                    log::debug!("Failed to delete stale cache entry '{}': {}", key, e);
                }
            }.instrument(delete_span).await;
        }

        // Try all operators in speed order
        let mut ops_vec: Vec<(&String, &(Operator, u128))> = ops.iter().collect();
        ops_vec.sort_by_key(|&(_, (_, speed))| speed);

        for (profile_name, (op, _speed)) in ops_vec {
            if std::ptr::eq(op as *const Operator, fastest_op as *const Operator) {
                continue;
            }

            log::debug!("Trying to load '{}' from profile '{}'", key, profile_name);

            if let Some(result) = try_read_from_op::<Self>(op, key, Some(profile_name)).await {
                match result {
                    Ok(obj) => {
                        log::info!("Successfully loaded '{}' from fallback profile '{}'", key, profile_name);

                        // Cache write-back: write to fastest operator (non-blocking)
                        if let Ok(serialised) = serde_json::to_vec(&obj) {
                            let fastest = fastest_op.clone();
                            let k = key.to_string();
                            let data_len = serialised.len();

                            tokio::spawn(async move {
                                let span = debug_span!("cache_writeback", key = %k, size = data_len);
                                async {
                                    let data = maybe_compress(&serialised);
                                    let compressed = data.len() < serialised.len();

                                    match fastest.write(&k, data).await {
                                        Ok(_) => {
                                            if compressed {
                                                log::debug!("Cached '{}' to fastest operator ({} bytes compressed)", k, data_len);
                                            } else {
                                                log::debug!("Cached '{}' to fastest operator ({} bytes)", k, data_len);
                                            }
                                        }
                                        Err(e) => {
                                            log::debug!("Failed to cache '{}' to fastest operator: {}", k, e);
                                        }
                                    }
                                }.instrument(span).await
                            });
                        }

                        return Ok(obj);
                    }
                    Err(e) => {
                        log::error!("Failed to load '{}' from profile '{}': {}", key, profile_name, e);
                    }
                }
            }
        }

        Err(Error::NoOperatorAvailable)
    }.instrument(span).await
}
```

**Pattern:**
- Try fastest operator first
- Detect schema evolution
- Clear stale cache if needed
- Try fallback operators in speed order
- Asynchronous cache write-back
- Handle all error cases

### Cache Warm-Up

#### Cache Write-Back
```rust
// Inside load_from_operator after successful fallback load

if let Ok(serialised) = serde_json::to_vec(&obj) {
    let fastest = fastest_op.clone();
    let k = key.to_string();
    let data_len = serialised.len();

    tokio::spawn(async move {
        let span = debug_span!("cache_writeback", key = %k, size = data_len);
        async {
            let data = maybe_compress(&serialised);
            let compressed = data.len() < serialised.len();

            match fastest.write(&k, data).await {
                Ok(_) => {
                    if compressed {
                        log::debug!("Cached '{}' to fastest operator ({} bytes compressed)", k, data_len);
                    } else {
                        log::debug!("Cached '{}' to fastest operator ({} bytes)", k, data_len);
                    }
                }
                Err(e) => {
                    log::debug!("Failed to cache '{}' to fastest operator: {}", k, e);
                }
            }
        }.instrument(span).await
    });
}
```

**Pattern:**
- Spawn fire-and-forget task
- Compress if beneficial
- Log at debug level
- Don't block load operation

#### Same-Operator Detection
```rust
async fn save_to_profile(&self, profile_name: &str) -> Result<()> {
    let (ops, _fastest_op) = &self.load_config().await?;
    let key = self.get_key();
    let serde_str = serde_json::to_string(&self)?;

    ops.get(profile_name)
        .ok_or_else(|| {
            Error::Profile(format!(
                "Unknown profile name: {profile_name}. Available profiles: {}",
                ops.keys().map(|k| k.as_str()).collect::<Vec<&str>>().join(", ")
            ))
        })?
        .0
        .write(&key, serde_str.clone())
        .await
        .map_err(|e| Error::OpenDal(Box::new(e)))?;

    Ok(())
}
```

**Pattern:**
- Look up profile by name
- Check if operator is fastest
- Skip write if same-operator to avoid redundant work

## Error Handling

### Error Types

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Profile error: {0}")]
    Profile(String),

    #[error("OpenDal error: {0}")]
    OpenDal(Box<opendal::Error>),

    #[error("No operator available")]
    NoOperator,

    #[error("No operator available")]
    NoOperatorAvailable,

    #[error("Serde JSON error")]
    Json(#[from] serde_json::Error),

    #[error("IO error")]
    Io(#[from] std::io::Error),
}
```

**Categories:**
- **Configuration**: Profile errors
- **Storage**: OpenDal errors
- **Availability**: No operator errors
- **Serialisation**: JSON errors
- **I/O**: File system errors

## Performance Optimisations

### Compression Strategy

#### Adaptive Compression
```rust
fn maybe_compress(data: &[u8]) -> Vec<u8> {
    const COMPRESSION_THRESHOLD: usize = 1_000_000; // 1MB

    if data.len() > COMPRESSION_THRESHOLD {
        zstd::encode_all(data, 3).unwrap_or_else(|_| data.to_vec())
    } else {
        data.to_vec()
    }
}
```

**Threshold:**
- 1MB compression threshold
- Level 3 compression (balanced)
- Fall back to raw on failure

### Backend Selection

#### Speed-Based Ordering
```rust
let mut ops_vec: Vec<(&String, &(Operator, u128))> = operators.iter().collect();
ops_vec.sort_by_key(|&(_, (_, speed))| speed);
```

**Strategy:**
- Measure backend speed during initialisation
- Sort by speed (lower = faster)
- Use fastest for all reads
- Use all for writes

### Caching

#### Lazy Initialisation
```rust
static DEVICE_STORAGE: AsyncOnceCell<DeviceStorage> = AsyncOnceCell::new();

impl DeviceStorage {
    pub async fn instance() -> Result<&'static DeviceStorage> {
        let storage = DEVICE_STORAGE
            .get_or_try_init(async {
                let initialised_storage = init_device_storage().await?;
                Ok::<DeviceStorage, Error>(initialised_storage)
            })
            .await?;
        Ok(storage)
    }
}
```

**Pattern:**
- Initialise on first access
- Cache initialised instance
- Thread-safe lazy loading

## Testing Patterns

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_only_storage() {
        let storage = DeviceStorage::init_memory_only().await.unwrap();
        assert!(storage.fastest_op.is_some());
    }

    #[tokio::test]
    async fn test_persistable_trait() {
        struct TestData {
            key: String,
            value: String,
        }

        impl Serialize for TestData {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_struct("TestData", 2)?
                    .serialize_field("key", &self.key)?
                    .serialize_field("value", &self.value)?
                    .end()
            }
        }

        impl<'de> Deserialize<'de> for TestData {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                enum Field { Key, Value }
                struct TestDataVisitor;

                impl<'de> serde::de::Visitor<'de> for TestDataVisitor {
                    type Value = TestData;

                    fn expecting(&self) -> &'static str {
                        "struct TestData"
                    }

                    fn visit_seq<A>(self, mut seq: A) -> Result<TestData, A::Error>
                    where
                        A: serde::de::SeqAccess<'de>,
                    {
                        let mut key = None;
                        let mut value = None;

                        while let Some(field) = seq.next_element()? {
                            match field {
                                Field::Key => key = Some(serde::de::MapAccess::next_value(&mut seq)?),
                                Field::Value => value = Some(serde::de::MapAccess::next_value(&mut seq)?),
                            }
                        }

                        Ok(TestData {
                            key: key.ok_or_else(|| serde::de::Error::missing_field("key"))?,
                            value: value.ok_or_else(|| serde::de::Error::missing_field("value"))?,
                        })
                    }
                }

                deserializer.deserialize_struct("TestData", &["key", "value"], TestDataVisitor)
            }
        }

        impl Persistable for TestData {
            fn new(key: String) -> Self {
                Self { key, value: String::new() }
            }

            async fn load(&mut self) -> Result<Self>
            where
                Self: Sized,
            {
                // Implementation
                Ok(self.clone())
            }

            async fn save(&self) -> Result<()> {
                // Implementation
                Ok(())
            }

            async fn load_config(&self) -> Result<(HashMap<String, (opendal::Operator, u128)>, opendal::Operator)> {
                // Implementation
                unimplemented!()
            }

            async fn save_to_all(&self) -> Result<()> {
                // Implementation
                Ok(())
            }

            async fn save_to_profile(&self, profile_name: &str) -> Result<()> {
                // Implementation
                Ok(())
            }
        }

        let mut test_data = TestData {
            key: "test-key".to_string(),
            value: "test-value".to_string(),
        };

        // Test load/save operations
    }
}
```

## Best Practices

### Storage Design

- Use multiple backends for resilience
- Prioritise speed for reads
- Write to all backends for safety
- Implement graceful degradation

### Error Handling

- Provide context in errors
- Categorise error types
- Support fallback mechanisms
- Log at appropriate levels

### Performance

- Use async throughout
- Minimise lock duration
- Implement caching
- Use compression for large objects

### Testing

- Test with memory-only backend
- Test multi-backend scenarios
- Test cache warm-up
- Test error handling

## Future Enhancements

### Planned Features

#### Write-Ahead Caching
```rust
async fn cache_prefetch(&self, keys: Vec<String>) -> Result<()> {
    // Preload common keys into fastest backend
}
```

#### Storage Quotas
```rust
struct StorageQuota {
    max_size: u64,
    current_size: u64,
}

async fn enforce_quota(&self, data_size: usize) -> Result<()> {
    // Enforce storage limits
}
```

#### Automatic Cleanup
```rust
async fn cleanup_old_entries(&self, ttl: Duration) -> Result<()> {
    // Remove entries older than TTL
}
```

## References

- [OpenDAL project](https://opendal.org/)
- [zstd compression](https://github.com/facebook/zstd)
- [async_trait for traits](https://docs.rs/async-trait/)
- [thiserror for error handling](https://docs.rs/thiserror/)
