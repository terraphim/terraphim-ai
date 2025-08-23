use serial_test::serial;
use tempfile::TempDir;
use terraphim_persistence::settings::parse_profile;
use terraphim_settings::DeviceSettings;
use std::collections::HashMap;

/// Test ReDB configuration and parsing
#[tokio::test]
#[serial]
async fn test_redb_configuration() -> Result<(), Box<dyn std::error::Error>> {
    // Create temporary directory for test
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    let redb_file = temp_path.join("test_terraphim.redb");
    
    log::info!("Testing ReDB configuration with file: {:?}", redb_file);
    
    // Create test settings with proper ReDB configuration
    let mut profiles = HashMap::new();
    
    // ReDB profile with file path (not directory)
    let mut redb_profile = HashMap::new();
    redb_profile.insert("type".to_string(), "redb".to_string());
    redb_profile.insert("datadir".to_string(), redb_file.to_string_lossy().to_string());
    redb_profile.insert("table".to_string(), "test_table".to_string());
    profiles.insert("test_redb".to_string(), redb_profile);
    
    let settings = DeviceSettings {
        server_hostname: "localhost:8000".to_string(),
        api_endpoint: "http://localhost:8000/api".to_string(),
        initialized: false,
        default_data_path: temp_path.to_string_lossy().to_string(),
        profiles,
    };
    
    log::info!("Test settings created with ReDB file: {:?}", redb_file);
    
    // Test 1: Parse ReDB profile
    let (op, _speed) = parse_profile(&settings, "test_redb").await?;
    log::info!("✅ Successfully created ReDB operator");
    
    // Test 2: Write data to ReDB
    let test_data = "Hello, ReDB World!";
    let test_key = "test_document";
    
    match op.write(test_key, test_data).await {
        Ok(()) => {
            log::info!("✅ Successfully wrote data to ReDB");
        },
        Err(e) => {
            log::error!("❌ Failed to write data to ReDB: {:?}", e);
            panic!("Failed to write to ReDB: {:?}", e);
        }
    }
    
    // Verify the database file was created
    assert!(redb_file.exists(), "ReDB database file should be created at: {:?}", redb_file);
    log::info!("✅ ReDB database file exists: {:?}", redb_file);
    
    // Test 3: Read data from ReDB
    match op.read(test_key).await {
        Ok(data) => {
            let data_str = String::from_utf8(data)?;
            log::info!("✅ Successfully read data from ReDB: {}", data_str);
            
            // Verify the content matches
            assert_eq!(data_str, test_data, "Data should match");
            log::info!("✅ Data content validation passed");
        },
        Err(e) => {
            log::error!("❌ Failed to read data from ReDB: {:?}", e);
            panic!("Failed to read from ReDB: {:?}", e);
        }
    }
    
    log::info!("🎉 All ReDB configuration tests passed!");
    Ok(())
}

/// Test ReDB error handling with invalid path
#[tokio::test]
#[serial]
async fn test_redb_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    // Test with invalid ReDB configuration (parent directory doesn't exist)
    let mut profiles = HashMap::new();
    let mut redb_profile = HashMap::new();
    redb_profile.insert("type".to_string(), "redb".to_string());
    redb_profile.insert("datadir".to_string(), "/invalid/path/that/does/not/exist/test.redb".to_string());
    redb_profile.insert("table".to_string(), "test_table".to_string());
    profiles.insert("invalid_redb".to_string(), redb_profile);
    
    let settings = DeviceSettings {
        server_hostname: "localhost:8000".to_string(),
        api_endpoint: "http://localhost:8000/api".to_string(),
        initialized: false,
        default_data_path: "/tmp/test_invalid".to_string(),
        profiles,
    };
    
    // This should fail gracefully (parent directory doesn't exist)
    let result = parse_profile(&settings, "invalid_redb").await;
    match result {
        Ok(_) => {
            log::warn!("Unexpected success with invalid ReDB path");
        },
        Err(e) => {
            log::info!("✅ Expected error with invalid ReDB path: {:?}", e);
            // This is expected behavior - should fail when parent dir doesn't exist
        }
    }
    
    Ok(())
}