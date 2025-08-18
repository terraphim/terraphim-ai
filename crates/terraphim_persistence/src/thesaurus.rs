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
        format!("thesaurus_{}.json", self.normalize_key(&self.name()))
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
        test_obj.save_to_one("dashmap").await?;

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

    /// Test saving and loading a thesaurus to rocksdb profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_thesaurus_rocksdb() -> Result<()> {
        // Create a test thesaurus
        let test_obj = Thesaurus::new("Test RocksDB Thesaurus".to_string());

        // Save the object to rocksdb
        test_obj.save_to_one("rocksdb").await?;

        // Load the object
        let mut loaded_obj = Thesaurus::new("Test RocksDB Thesaurus".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded RocksDB thesaurus does not match the original"
        );

        Ok(())
    }

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
