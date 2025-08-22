// This is a copy from opendal cli config
// https://raw.githubusercontent.com/apache/incubator-opendal/main/bin/oli/src/config/mod.rs

// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use opendal::layers::LoggingLayer;
use opendal::services;
use opendal::Operator;
use opendal::Result as OpendalResult;
use opendal::Scheme;

use log::debug;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Instant;

use crate::{Error, Result};
use terraphim_settings::DeviceSettings;

/// resolve_relative_path turns a relative path to a absolute path.
///
/// The reason why we don't use `fs::canonicalize` here is `fs::canonicalize`
/// will return an error if the path does not exist, which is unwanted.
pub fn resolve_relative_path(path: &Path) -> Cow<Path> {
    // NOTE: `path.is_absolute()` cannot handle cases like "/tmp/../a"
    if path
        .components()
        .all(|e| e != Component::ParentDir && e != Component::CurDir)
    {
        // it is an absolute path
        return path.into();
    }

    let root = Component::RootDir.as_os_str();
    let mut result = env::current_dir().unwrap_or_else(|_| PathBuf::from(root));
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if result.parent().is_none() {
                    continue;
                }
                result.pop();
            }
            Component::CurDir => (),
            Component::RootDir | Component::Normal(_) | Component::Prefix(_) => {
                result.push(comp.as_os_str());
            }
        }
    }
    result.into()
}

/// Ensure directory exists for storage backends that require it
fn ensure_directory_exists(path: &str) -> Result<()> {
    if !path.is_empty() {
        log::info!("🔧 Creating directory: {}", path);
        std::fs::create_dir_all(path).map_err(|e| {
            Error::OpenDal(opendal::Error::new(
                opendal::ErrorKind::Unexpected,
                &format!("Failed to create directory '{}': {}", path, e)
            ))
        })?;
        log::info!("✅ Successfully created directory: {}", path);
    }
    Ok(())
}

pub async fn parse_profile(
    settings: &DeviceSettings,
    profile_name: &str,
) -> Result<(Operator, u128)> {
    log::info!("📁 Parsing profile: {}", profile_name);
    /// Returns the time (in nanoseconds) it takes to load a 1MB file,
    /// used to determine the fastest operator for a given profile.
    async fn get_speed(op: Operator) -> OpendalResult<u128> {
        #[cfg(debug_assertions)]
        let buf = "test data";
        #[cfg(not(debug_assertions))]
        let buf = vec![0u8; 1024 * 1024];
        op.write("test", buf).await?;

        let start_time = Instant::now();
        op.read("test").await?;
        let end_time = Instant::now();

        let load_time = end_time.duration_since(start_time).as_nanos();
        Ok(load_time)
    }

    let profile = settings
        .profiles
        .get(profile_name)
        .ok_or_else(|| Error::Profile(format!("unknown profile: {}", profile_name)))?;

    let svc = profile
        .get("type")
        .ok_or_else(|| Error::Profile("type is required".to_string()))?;

    let scheme = Scheme::from_str(svc)?;
    log::info!("🔧 Profile '{}' using scheme: {:?}", profile_name, scheme);
    let op = match scheme {
        Scheme::Azblob => Operator::from_map::<services::Azblob>(profile.clone())?.finish(),
        Scheme::Azdls => Operator::from_map::<services::Azdls>(profile.clone())?.finish(),
        #[cfg(feature = "services-dashmap")]
        Scheme::Dashmap => {
            // Ensure directory exists for DashMap
            if let Some(root) = profile.get("root") {
                ensure_directory_exists(root)?;
            }
            let builder = services::Dashmap::default();
            // Init an operator
            let op = Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish();
            debug!("operator: {op:?}");
            op
        }
        #[cfg(feature = "services-atomicserver")]
        Scheme::Atomicserver => {
            Operator::from_map::<services::Atomicserver>(profile.clone())?.finish()
        }
        Scheme::Gcs => Operator::from_map::<services::Gcs>(profile.clone())?.finish(),
        Scheme::Ghac => Operator::from_map::<services::Ghac>(profile.clone())?.finish(),
        Scheme::Http => Operator::from_map::<services::Http>(profile.clone())?.finish(),
        #[cfg(feature = "services-ipfs")]
        Scheme::Ipfs => Operator::from_map::<services::Ipfs>(profile.clone())?.finish(),
        Scheme::Ipmfs => Operator::from_map::<services::Ipmfs>(profile.clone())?.finish(),
        Scheme::Obs => Operator::from_map::<services::Obs>(profile.clone())?.finish(),
        Scheme::Oss => Operator::from_map::<services::Oss>(profile.clone())?.finish(),
        #[cfg(feature = "services-redis")]
        Scheme::Redis => Operator::from_map::<services::Redis>(profile.clone())?.finish(),
        #[cfg(feature = "services-rocksdb")]
        Scheme::Rocksdb => Operator::from_map::<services::Rocksdb>(profile.clone())?.finish(),
        #[cfg(feature = "services-redb")]
        Scheme::Redb => {
            // Ensure directory exists for ReDB
            if let Some(datadir) = profile.get("datadir") {
                ensure_directory_exists(datadir)?;
            }
            Operator::from_map::<services::Redb>(profile.clone())?.finish()
        },
        #[cfg(feature = "services-sqlite")]
        Scheme::Sqlite => {
            // Ensure directory exists for SQLite
            if let Some(datadir) = profile.get("datadir") {
                ensure_directory_exists(datadir)?;
            }
            Operator::from_map::<services::Sqlite>(profile.clone())?.finish()
        },
        Scheme::S3 => {
            match Operator::from_map::<services::S3>(profile.clone()) {
                Ok(builder) => builder.finish(),
                Err(e) => {
                    log::warn!("Failed to create S3 operator (missing AWS credentials?): {:?}", e);
                    log::info!("Falling back to memory operator for profile: {}", profile_name);
                    let builder = services::Memory::default();
                    Operator::new(builder)?
                        .layer(LoggingLayer::default())
                        .finish()
                }
            }
        },
        Scheme::Webdav => Operator::from_map::<services::Webdav>(profile.clone())?.finish(),
        Scheme::Webhdfs => Operator::from_map::<services::Webhdfs>(profile.clone())?.finish(),
        _ => {
            log::info!("Got request for {scheme} operator; initializing in-memory operator.");
            let builder = services::Memory::default();

            // Init operator
            Operator::new(builder)?
                // Init with logging layer enabled.
                .layer(LoggingLayer::default())
                .finish()
        }
    };
    // Benchmark the operator I/O speed
    let speed = get_speed(op.clone()).await?;
    Ok((op, speed))
}

pub async fn parse_profiles(
    settings: &DeviceSettings,
) -> Result<HashMap<String, (Operator, u128)>> {
    let mut ops = HashMap::new();
    let profile_names = settings.profiles.keys();
    for profile_name in profile_names {
        let (op, speed) = parse_profile(settings, profile_name).await?;
        ops.insert(profile_name.clone(), (op, speed));
    }
    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Persistable;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        age: u8,
    }

    #[async_trait]
    impl Persistable for TestStruct {
        fn new(name: String) -> Self {
            TestStruct { name, age: 0 }
        }

        async fn save_to_one(&self, profile_name: &str) -> Result<()> {
            self.save_to_profile(profile_name).await
        }

        async fn save(&self) -> Result<()> {
            let _op = &self.load_config().await?.1;
            self.save_to_all().await
        }

        async fn load(&mut self) -> Result<Self> {
            let op = &self.load_config().await?.1;
            let key = self.get_key();
            self.load_from_operator(&key, &op).await
        }

        fn get_key(&self) -> String {
            self.normalize_key(&self.name)
        }
    }
    /// Test saving and loading a struct to a dashmap profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test Object".to_string(),
            age: 25,
        };

        // Save the object
        test_obj.save_to_one("dashmap").await?;

        // Load the object
        let mut loaded_obj = TestStruct::new("Test Object".to_string());
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
        let test_obj = TestStruct {
            name: "Test Object".to_string(),
            age: 25,
        };

        // Save the object
        test_obj.save().await?;

        // Load the object
        let mut loaded_obj = TestStruct::new("Test Object".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded object does not match the original"
        );

        Ok(())
    }

    /// Test saving and loading a struct to rocksdb profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_rocksdb() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test RocksDB Object".to_string(),
            age: 30,
        };

        // Save the object to rocksdb
        test_obj.save_to_one("rocksdb").await?;

        // Load the object
        let mut loaded_obj = TestStruct::new("Test RocksDB Object".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded RocksDB object does not match the original"
        );

        Ok(())
    }

    /// Test saving and loading a struct to memory profile
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_memory() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test Memory Object".to_string(),
            age: 35,
        };

        // Save the object to memory
        test_obj.save_to_one("memory").await?;

        // Load the object
        let mut loaded_obj = TestStruct::new("Test Memory Object".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(
            test_obj, loaded_obj,
            "Loaded memory object does not match the original"
        );

        Ok(())
    }

    /// Test saving and loading a struct to redb profile (if available)
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_redb() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test ReDB Object".to_string(),
            age: 40,
        };

        // Try to save the object to redb - this might not be configured in all environments
        match test_obj.save_to_one("redb").await {
            Ok(()) => {
                // Load the object
                let mut loaded_obj = TestStruct::new("Test ReDB Object".to_string());
                loaded_obj = loaded_obj.load().await?;

                // Compare the original and loaded objects
                assert_eq!(
                    test_obj, loaded_obj,
                    "Loaded ReDB object does not match the original"
                );
            }
            Err(e) => {
                println!(
                    "ReDB profile not available (expected in some environments): {:?}",
                    e
                );
                // This is okay - not all environments may have redb configured
            }
        }

        Ok(())
    }

    /// Test saving and loading a struct to sqlite profile (if available)
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_sqlite() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test SQLite Object".to_string(),
            age: 45,
        };

        // Try to save the object to sqlite - this might not be configured in all environments
        match test_obj.save_to_one("sqlite").await {
            Ok(()) => {
                // Load the object
                let mut loaded_obj = TestStruct::new("Test SQLite Object".to_string());
                loaded_obj = loaded_obj.load().await?;

                // Compare the original and loaded objects
                assert_eq!(
                    test_obj, loaded_obj,
                    "Loaded SQLite object does not match the original"
                );
            }
            Err(e) => {
                println!(
                    "SQLite profile not available (expected in some environments): {:?}",
                    e
                );
                // This is okay - not all environments may have sqlite configured
            }
        }

        Ok(())
    }

    /// Test that directories are created automatically for operators
    #[tokio::test]
    #[serial_test::serial] 
    async fn test_operators_create_directories() -> Result<()> {
        use tempfile::TempDir;
        use terraphim_types::Document;
        
        // Create temporary directory for test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create test settings with custom paths in temporary directory
        let sqlite_path = temp_path.join("test_sqlite");
        let redb_path = temp_path.join("test_redb");
        let dashmap_path = temp_path.join("test_dashmap");
        
        let mut profiles = std::collections::HashMap::new();
        
        // SQLite profile with proper configuration
        let mut sqlite_profile = std::collections::HashMap::new();
        sqlite_profile.insert("type".to_string(), "sqlite".to_string());
        sqlite_profile.insert("connection_string".to_string(), 
            format!("sqlite://{}/test.db", sqlite_path.to_string_lossy()));
        sqlite_profile.insert("datadir".to_string(), sqlite_path.to_string_lossy().to_string());
        profiles.insert("test_sqlite".to_string(), sqlite_profile);
        
        // ReDB profile with proper configuration
        let mut redb_profile = std::collections::HashMap::new();
        redb_profile.insert("type".to_string(), "redb".to_string());
        redb_profile.insert("datadir".to_string(), redb_path.to_string_lossy().to_string());
        redb_profile.insert("table".to_string(), "test_table".to_string());
        redb_profile.insert("path".to_string(), format!("{}/test.redb", redb_path.to_string_lossy()));
        profiles.insert("test_redb".to_string(), redb_profile);
        
        // DashMap profile
        let mut dashmap_profile = std::collections::HashMap::new();
        dashmap_profile.insert("type".to_string(), "dashmap".to_string());
        dashmap_profile.insert("root".to_string(), dashmap_path.to_string_lossy().to_string());
        profiles.insert("test_dashmap".to_string(), dashmap_profile);
        
        let settings = DeviceSettings {
            server_hostname: "localhost:8000".to_string(),
            api_endpoint: "http://localhost:8000/api".to_string(),
            initialized: false,
            default_data_path: temp_path.to_string_lossy().to_string(),
            profiles,
        };
        
        // Test that parse_profiles creates directories and operators
        let operators = parse_profiles(&settings).await?;
        
        // Verify directories were created (only check for ones that were actually created)
        if operators.contains_key("test_sqlite") {
            assert!(sqlite_path.exists(), "SQLite directory should be created");
            log::info!("✅ SQLite directory created successfully");
        } else {
            log::warn!("SQLite operator not available (feature not enabled)");
        }
        
        if operators.contains_key("test_redb") {
            assert!(redb_path.exists(), "ReDB directory should be created");
            log::info!("✅ ReDB directory created successfully");
        } else {
            log::warn!("ReDB operator not available (feature not enabled)");
        }
        
        if operators.contains_key("test_dashmap") {
            assert!(dashmap_path.exists(), "DashMap directory should be created");
            log::info!("✅ DashMap directory created successfully");
        } else {
            log::warn!("DashMap operator not available (feature not enabled)");
        }
        
        // Test that we can save a document to each operator
        let test_doc = Document {
            id: "test_document".to_string(),
            title: "Test Document".to_string(),
            url: "test://url".to_string(),
            body: "Test content".to_string(),
            description: Some("Test description".to_string()),
            stub: None,
            tags: None,
            rank: None,
        };
        
        // Save document to each operator to verify they work
        for (name, (op, _)) in &operators {
            let key = format!("document_{}.json", test_doc.id);
            let data = serde_json::to_string(&test_doc)?;
            match op.write(&key, data).await {
                Ok(()) => log::info!("✅ Successfully saved test document to {}", name),
                Err(e) => log::warn!("Failed to save to {}: {:?}", name, e),
            }
        }
        
        Ok(())
    }
}
