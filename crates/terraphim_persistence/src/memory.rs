//! Memory-only utilities for testing and development
//!
//! This module provides utilities for creating memory-only configurations
//! that don't require filesystem or external services.

use crate::Result;
use std::collections::HashMap;
use terraphim_settings::DeviceSettings;

/// Create a DeviceSettings instance that uses only memory storage
///
/// This is useful for tests and development where you don't want to
/// persist data to disk or external services.
pub fn create_memory_only_device_settings() -> Result<DeviceSettings> {
    let mut profiles = HashMap::new();

    // Add memory profile - this will use OpenDAL's Memory service
    let mut memory_profile = HashMap::new();
    memory_profile.insert("type".to_string(), "memory".to_string());
    profiles.insert("memory".to_string(), memory_profile);

    let settings = DeviceSettings {
        server_hostname: "localhost".to_string(),
        api_endpoint: "http://localhost:8080".to_string(),
        initialized: true,
        default_data_path: "/tmp/terraphim_test".to_string(),
        profiles,
    };

    Ok(settings)
}

/// Create a test-specific DeviceSettings with memory storage
///
/// This creates a minimal configuration for testing purposes
/// that uses only in-memory storage.
pub fn create_test_device_settings() -> Result<DeviceSettings> {
    create_memory_only_device_settings()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::parse_profiles;

    #[tokio::test]
    async fn test_memory_only_device_settings() -> Result<()> {
        let settings = create_memory_only_device_settings()?;

        // Verify we have memory profile
        assert!(settings.profiles.contains_key("memory"));
        assert_eq!(
            settings
                .profiles
                .get("memory")
                .unwrap()
                .get("type")
                .unwrap(),
            "memory"
        );

        // Test that we can parse the profiles
        let operators = parse_profiles(&settings).await?;
        assert!(operators.contains_key("memory"));

        let (memory_op, _speed) = operators.get("memory").unwrap();

        // Test basic write/read operations
        memory_op.write("test_key", "test_value").await.unwrap();
        let result = memory_op.read("test_key").await.unwrap();
        assert_eq!(result.to_vec(), "test_value".as_bytes());

        Ok(())
    }

    #[tokio::test]
    async fn test_memory_persistable() -> Result<()> {
        use crate::settings::parse_profiles;
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        impl TestData {
            async fn save_to_memory_operator(&self, operator: &opendal::Operator) -> Result<()> {
                let key = format!("test_data_{}", self.name.to_lowercase());
                let data = serde_json::to_string(self)?;
                operator.write(&key, data).await?;
                Ok(())
            }

            async fn load_from_memory_operator(
                name: &str,
                operator: &opendal::Operator,
            ) -> Result<Self> {
                let key = format!("test_data_{}", name.to_lowercase());
                let data = operator.read(&key).await?;
                let obj: TestData = serde_json::from_slice(&data.to_vec())?;
                Ok(obj)
            }
        }

        // Create memory-only settings and get the operator
        let settings = create_memory_only_device_settings()?;
        let operators = parse_profiles(&settings).await?;
        let (memory_op, _speed) = operators.get("memory").unwrap();

        let test_data = TestData {
            name: "test_item".to_string(),
            value: 42,
        };

        // Save and load using the memory operator directly
        test_data.save_to_memory_operator(memory_op).await?;
        let loaded_data = TestData::load_from_memory_operator("test_item", memory_op).await?;

        assert_eq!(test_data, loaded_data);

        Ok(())
    }

    #[tokio::test]
    async fn test_thesaurus_memory_persistence() -> Result<()> {
        use crate::settings::parse_profiles;
        use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};

        // Create memory-only settings and get the operator
        let settings = create_memory_only_device_settings()?;
        let operators = parse_profiles(&settings).await?;
        let (memory_op, _speed) = operators.get("memory").unwrap();

        // Create a test thesaurus
        let mut thesaurus = Thesaurus::new("Test Engineer".to_string());

        // Add some test terms
        let term1 = NormalizedTerm::new(1, NormalizedTermValue::from("machine learning"));
        let term2 = NormalizedTerm::new(2, NormalizedTermValue::from("artificial intelligence"));

        thesaurus.insert(NormalizedTermValue::from("ml"), term1.clone());
        thesaurus.insert(NormalizedTermValue::from("ai"), term2.clone());
        thesaurus.insert(NormalizedTermValue::from("machine learning"), term1.clone());
        thesaurus.insert(
            NormalizedTermValue::from("artificial intelligence"),
            term2.clone(),
        );

        // Save the thesaurus using memory operator
        let key = format!("thesaurus_{}.json", "testengineer");
        let thesaurus_json = serde_json::to_string(&thesaurus)?;
        memory_op.write(&key, thesaurus_json).await?;

        // Load the thesaurus back
        let loaded_data = memory_op.read(&key).await?;
        let loaded_thesaurus: Thesaurus = serde_json::from_slice(&loaded_data.to_vec())?;

        // Verify the thesaurus was properly saved and loaded
        assert_eq!(thesaurus.name(), loaded_thesaurus.name());
        assert_eq!(thesaurus.len(), loaded_thesaurus.len());
        assert_eq!(thesaurus.len(), 4);

        // Test that we can retrieve terms correctly
        let ml_term = loaded_thesaurus
            .get(&NormalizedTermValue::from("ml"))
            .unwrap();
        assert_eq!(ml_term.id, 1);
        assert_eq!(ml_term.value, NormalizedTermValue::from("machine learning"));

        let ai_term = loaded_thesaurus
            .get(&NormalizedTermValue::from("ai"))
            .unwrap();
        assert_eq!(ai_term.id, 2);
        assert_eq!(
            ai_term.value,
            NormalizedTermValue::from("artificial intelligence")
        );

        println!("✅ Thesaurus memory persistence test passed");
        println!("   Saved and loaded {} terms", loaded_thesaurus.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_config_memory_persistence() -> Result<()> {
        use crate::settings::parse_profiles;
        use terraphim_config::{ConfigBuilder, ConfigId};

        // Create memory-only settings and get the operator
        let settings = create_memory_only_device_settings()?;
        let operators = parse_profiles(&settings).await?;
        let (memory_op, _speed) = operators.get("memory").unwrap();

        // Create a test config using ConfigBuilder
        let config = ConfigBuilder::new().build().unwrap();

        // Save the config using memory operator
        let key = format!(
            "{}_config.json",
            match config.id {
                ConfigId::Desktop => "desktop",
                ConfigId::Server => "server",
                ConfigId::Embedded => "embedded",
            }
        );
        let config_json = serde_json::to_string(&config)?;
        memory_op.write(&key, config_json).await?;

        // Load the config back
        let loaded_data = memory_op.read(&key).await?;
        let loaded_config: terraphim_config::Config =
            serde_json::from_slice(&loaded_data.to_vec())?;

        // Verify the config was properly saved and loaded
        assert_eq!(config.id, loaded_config.id);
        assert_eq!(config.default_role, loaded_config.default_role);

        println!("✅ Config memory persistence test passed");

        Ok(())
    }
}
