use crate::Result;
use async_trait::async_trait;
use terraphim_types::Thesaurus;

use crate::Persistable;

#[async_trait]
impl Persistable for Thesaurus {
    fn new(key: String) -> Self {
        Thesaurus::new(key)
    }

    /// Save to a single profile
    async fn save_to_one(&self, profile_name: &str) -> Result<()> {
        self.save_to_profile(profile_name).await?;
        Ok(())
    }

    // Saves to all profiles
    async fn save(&self) -> Result<()> {
        let _op = &self.load_config().await?.1;
        let _ = self.save_to_all().await?;
        Ok(())
    }

    /// Load key from the fastest operator
    async fn load(&mut self) -> Result<Self> {
        let op = &self.load_config().await?.1;
        let key = self.get_key();
        let obj = self.load_from_operator(&key, op).await?;
        Ok(obj)
    }

    /// returns key + .json
    fn get_key(&self) -> String {
        let name = self.name();
        let normalized = self.normalize_key(name);
        let key = format!("thesaurus_{}.json", normalized);

        log::debug!("Thesaurus key generation: name='{}' → key='{}'", name, key);

        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test saving and loading a struct to a dashmap profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load() -> Result<()> {
        // Create a test object
        let test_obj = Thesaurus::new("Test Thesaurus".to_string());

        // Save the object
        test_obj.save_to_one("memory").await?;

        // Load the object
        let mut loaded_obj = Thesaurus::new("Test Thesaurus".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded object does not match the original"
        );

        Ok(())
    }
    /// Test saving and loading a struct to all profiles
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_all() -> Result<()> {
        // Create a test object
        let test_obj = Thesaurus::new("Test Thesaurus".to_string());

        // Save the object
        test_obj.save().await?;

        // Load the object
        let mut loaded_obj = Thesaurus::new("Test Thesaurus".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded object does not match the original"
        );

        Ok(())
    }

    // RocksDB support disabled - causes locking issues
    // #[cfg(feature = "services-rocksdb")]
    // #[tokio::test]
    // #[serial_test::serial]
    // async fn test_save_and_load_thesaurus_rocksdb() -> Result<()> {
    //     use tempfile::TempDir;
    //     use terraphim_settings::DeviceSettings;
    //
    //     // Create temporary directory for test
    //     let temp_dir = TempDir::new().unwrap();
    //     let rocksdb_path = temp_dir.path().join("test_thesaurus_rocksdb");
    //
    //     // Create test settings with rocksdb profile
    //     let mut profiles = std::collections::HashMap::new();
    //
    //     // Memory profile (needed as fastest operator fallback)
    //     let mut memory_profile = std::collections::HashMap::new();
    //     memory_profile.insert("type".to_string(), "memory".to_string());
    //     profiles.insert("memory".to_string(), memory_profile);
    //
    //     // RocksDB profile for testing
    //     let mut rocksdb_profile = std::collections::HashMap::new();
    //     rocksdb_profile.insert("type".to_string(), "rocksdb".to_string());
    //     rocksdb_profile.insert(
    //         "datadir".to_string(),
    //         rocksdb_path.to_string_lossy().to_string(),
    //     );
    //     profiles.insert("rocksdb".to_string(), rocksdb_profile);
    //
    //     let settings = DeviceSettings {
    //         server_hostname: "localhost:8000".to_string(),
    //         api_endpoint: "http://localhost:8000/api".to_string(),
    //         initialized: false,
    //         default_data_path: temp_dir.path().to_string_lossy().to_string(),
    //         profiles,
    //     };
    //
    //     // Initialize storage with custom settings
    //     let storage = crate::init_device_storage_with_settings(settings).await?;
    //
    //     // Verify rocksdb profile is available
    //     assert!(
    //         storage.ops.contains_key("rocksdb"),
    //         "RocksDB profile should be available. Available profiles: {:?}",
    //         storage.ops.keys().collect::<Vec<_>>()
    //     );
    //
    //     // Test direct operator write/read with thesaurus data
    //     let rocksdb_op = &storage.ops.get("rocksdb").unwrap().0;
    //     let test_key = "thesaurus_test_rocksdb_thesaurus.json";
    //     let test_thesaurus = Thesaurus::new("Test RocksDB Thesaurus".to_string());
    //     let test_data = serde_json::to_string(&test_thesaurus).unwrap();
    //
    //     rocksdb_op.write(test_key, test_data.clone()).await?;
    //     let read_data = rocksdb_op.read(test_key).await?;
    //     let read_str = String::from_utf8(read_data.to_vec()).unwrap();
    //     let loaded_thesaurus: Thesaurus = serde_json::from_str(&read_str).unwrap();
    //
    //     assert_eq!(
    //         test_thesaurus, loaded_thesaurus,
    //         "Loaded RocksDB thesaurus does not match the original"
    //     );
    //
    //     Ok(())
    // }

    /// Test saving and loading a thesaurus to memory profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_thesaurus_memory() -> Result<()> {
        // Create a test thesaurus
        let test_obj = Thesaurus::new("Test Memory Thesaurus".to_string());

        // Save the object to memory
        test_obj.save_to_one("memory").await?;

        // Load the object
        let mut loaded_obj = Thesaurus::new("Test Memory Thesaurus".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded memory thesaurus does not match the original"
        );

        Ok(())
    }

    /// Test saving and loading a thesaurus to redb profile (if available)
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_thesaurus_redb() -> Result<()> {
        // Create a test thesaurus
        let test_obj = Thesaurus::new("Test ReDB Thesaurus".to_string());

        // Try to save the object to redb - this might not be configured in all environments
        match test_obj.save_to_one("redb").await {
            Ok(()) => {
                // Load the object
                let mut loaded_obj = Thesaurus::new("Test ReDB Thesaurus".to_string());
                loaded_obj = loaded_obj.load().await?;

                // Compare the original and loaded objects
                assert_eq!(
                    test_obj, loaded_obj,
                    "Loaded ReDB thesaurus does not match the original"
                );
            }
            Err(e) => {
                println!("ReDB profile not available for thesaurus (expected in some environments): {:?}", e);
                // This is okay - not all environments may have redb configured
            }
        }

        Ok(())
    }

    /// Test saving and loading a thesaurus to sqlite profile (if available)
    #[cfg(feature = "sqlite")]
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_thesaurus_sqlite() -> Result<()> {
        // Create a test thesaurus
        let test_obj = Thesaurus::new("Test SQLite Thesaurus".to_string());

        // Try to save the object to sqlite - this might not be configured in all environments
        match test_obj.save_to_one("sqlite").await {
            Ok(()) => {
                // Load the object
                let mut loaded_obj = Thesaurus::new("Test SQLite Thesaurus".to_string());
                loaded_obj = loaded_obj.load().await?;

                // Compare the original and loaded objects
                assert_eq!(
                    test_obj, loaded_obj,
                    "Loaded SQLite thesaurus does not match the original"
                );
            }
            Err(e) => {
                println!("SQLite profile not available for thesaurus (expected in some environments): {:?}", e);
                // This is okay - not all environments may have sqlite configured
            }
        }

        Ok(())
    }
}
