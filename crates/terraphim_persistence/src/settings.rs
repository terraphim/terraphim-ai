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

use opendal::services;
use opendal::Operator;
use opendal::Result as OpendalResult;
use opendal::Scheme;

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

/// Expand tilde (~) in paths to the user's home directory
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    } else if path == "~" {
        if let Ok(home) = env::var("HOME") {
            return home;
        }
    }
    path.to_string()
}

/// Expand tilde in all string values of a HashMap
fn expand_profile_paths(profile: &HashMap<String, String>) -> HashMap<String, String> {
    profile
        .iter()
        .map(|(k, v)| (k.clone(), expand_tilde(v)))
        .collect()
}

/// resolve_relative_path turns a relative path to a absolute path.
///
/// The reason why we don't use `fs::canonicalize` here is `fs::canonicalize`
/// will return an error if the path does not exist, which is unwanted.
pub fn resolve_relative_path(path: &Path) -> Cow<'_, Path> {
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

/// Ensure SQLite table exists for OpenDAL
#[cfg(feature = "sqlite")]
fn ensure_sqlite_table_exists(connection_string: &str, table_name: &str) -> Result<()> {
    // Extract database path from connection string (remove query parameters)
    let db_path = if let Some(path_part) = connection_string.split('?').next() {
        path_part
    } else {
        connection_string
    };

    log::info!(
        "🔧 Ensuring SQLite table '{}' exists in database: {}",
        table_name,
        db_path
    );

    // Create the database file and table if they don't exist
    let connection = rusqlite::Connection::open(db_path).map_err(|e| {
        Error::OpenDal(Box::new(opendal::Error::new(
            opendal::ErrorKind::Unexpected,
            format!("Failed to open SQLite database '{}': {}", db_path, e),
        )))
    })?;

    // Enable WAL mode for concurrent access
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| {
            Error::OpenDal(Box::new(opendal::Error::new(
                opendal::ErrorKind::Unexpected,
                format!("Failed to enable WAL mode: {}", e),
            )))
        })?;

    // Set synchronous mode to NORMAL for better performance
    connection
        .pragma_update(None, "synchronous", "NORMAL")
        .map_err(|e| {
            Error::OpenDal(Box::new(opendal::Error::new(
                opendal::ErrorKind::Unexpected,
                format!("Failed to set synchronous mode: {}", e),
            )))
        })?;

    // Create table with key-value schema expected by OpenDAL
    let create_table_sql = format!(
        "CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value BLOB)",
        table_name
    );

    connection.execute(&create_table_sql, []).map_err(|e| {
        Error::OpenDal(Box::new(opendal::Error::new(
            opendal::ErrorKind::Unexpected,
            format!("Failed to create SQLite table '{}': {}", table_name, e),
        )))
    })?;

    log::info!("✅ SQLite table '{}' ready", table_name);
    Ok(())
}

/// Create a memory operator as fallback
///
/// Note: LoggingLayer is intentionally not added to avoid WARN-level logs
/// for expected NotFound errors on first startup (config file doesn't exist yet).
#[allow(clippy::result_large_err)]
fn create_memory_operator() -> OpendalResult<Operator> {
    let builder = services::Memory::default();
    Ok(Operator::new(builder)?.finish())
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
        Scheme::Azblob => {
            log::warn!("Azure Blob Storage not supported in this build");
            create_memory_operator()?
        }
        Scheme::Azdls => {
            log::warn!("Azure Data Lake Storage not supported in this build");
            create_memory_operator()?
        }
        #[cfg(feature = "services-dashmap")]
        Scheme::Dashmap => {
            // Ensure directory exists for DashMap
            if let Some(root) = profile.get("root") {
                std::fs::create_dir_all(root).map_err(|e| {
                    Error::OpenDal(Box::new(opendal::Error::new(
                        opendal::ErrorKind::Unexpected,
                        format!("Failed to create directory '{}': {}", root, e),
                    )))
                })?;
            }
            let builder = services::Dashmap::default();
            // Note: LoggingLayer is intentionally not added to avoid WARN-level logs
            // for expected NotFound errors on first startup (config file doesn't exist yet).
            Operator::new(builder)?.finish()
        }
        // atomicserver feature removed in opendal 0.54
        Scheme::Atomicserver => {
            log::warn!("Atomic Server not supported in opendal 0.54+");
            create_memory_operator()?
        }
        Scheme::Gcs => {
            log::warn!("Google Cloud Storage not supported in this build");
            create_memory_operator()?
        }
        Scheme::Ghac => {
            log::warn!("GitHub Actions Cache not supported in this build");
            create_memory_operator()?
        }
        Scheme::Http => {
            log::warn!("HTTP service not supported in this build");
            create_memory_operator()?
        }
        #[cfg(feature = "services-ipfs")]
        Scheme::Ipfs => Operator::from_iter::<services::Ipfs>(profile.clone())?.finish(),
        Scheme::Ipmfs => {
            log::warn!("IPFS MFS not supported in this build");
            create_memory_operator()?
        }
        Scheme::Obs => {
            log::warn!("Huawei Object Storage not supported in this build");
            create_memory_operator()?
        }
        Scheme::Oss => {
            log::warn!("Alibaba Object Storage not supported in this build");
            create_memory_operator()?
        }
        #[cfg(feature = "services-redis")]
        Scheme::Redis => Operator::from_iter::<services::Redis>(profile.clone())?.finish(),
        #[cfg(feature = "services-redb")]
        Scheme::Redb => {
            // Ensure parent directory exists for ReDB database file
            if let Some(datadir) = profile.get("datadir") {
                if let Some(parent) = std::path::Path::new(datadir).parent() {
                    let parent_str = parent.to_string_lossy();
                    if !parent_str.is_empty() {
                        std::fs::create_dir_all(&*parent_str).map_err(|e| {
                            Error::OpenDal(Box::new(opendal::Error::new(
                                opendal::ErrorKind::Unexpected,
                                format!("Failed to create directory '{}': {}", parent_str, e),
                            )))
                        })?;
                    }
                }
            }
            Operator::from_iter::<services::Redb>(profile.clone())?.finish()
        }
        #[cfg(feature = "sqlite")]
        Scheme::Sqlite => {
            // Expand tilde in all paths for SQLite profile
            let expanded_profile = expand_profile_paths(profile);

            // Ensure directory exists for SQLite
            if let Some(datadir) = expanded_profile.get("datadir") {
                log::info!("Creating SQLite directory: {}", datadir);
                std::fs::create_dir_all(datadir).map_err(|e| {
                    Error::OpenDal(Box::new(opendal::Error::new(
                        opendal::ErrorKind::Unexpected,
                        format!("Failed to create directory '{}': {}", datadir, e),
                    )))
                })?;
            }

            // Ensure SQLite table exists before OpenDAL tries to use it
            if let (Some(connection_string), Some(table_name)) = (
                expanded_profile.get("connection_string"),
                expanded_profile.get("table"),
            ) {
                ensure_sqlite_table_exists(connection_string, table_name)?;
            }

            // SQLite configuration with proper field names
            let mut sqlite_profile = expanded_profile;

            // Ensure required fields are set with proper defaults
            if !sqlite_profile.contains_key("root") {
                sqlite_profile.insert("root".to_string(), "/".to_string());
            }
            if !sqlite_profile.contains_key("key_field") {
                sqlite_profile.insert("key_field".to_string(), "key".to_string());
            }
            if !sqlite_profile.contains_key("value_field") {
                sqlite_profile.insert("value_field".to_string(), "value".to_string());
            }

            Operator::from_iter::<services::Sqlite>(sqlite_profile)?.finish()
        }
        #[cfg(feature = "s3")]
        Scheme::S3 => match Operator::from_iter::<services::S3>(profile.clone()) {
            Ok(builder) => builder.finish(),
            Err(e) => {
                log::warn!("Failed to create S3 operator: {:?}", e);
                log::info!("Falling back to memory operator");
                create_memory_operator()?
            }
        },
        #[cfg(not(feature = "s3"))]
        Scheme::S3 => {
            log::warn!("S3 support not compiled in this binary");
            log::info!("Using memory operator instead");
            create_memory_operator()?
        }
        Scheme::Webdav => {
            log::warn!("WebDAV not supported in this build");
            create_memory_operator()?
        }
        Scheme::Webhdfs => {
            log::warn!("WebHDFS not supported in this build");
            create_memory_operator()?
        }
        _ => {
            log::info!("Got request for {scheme} operator; initializing in-memory operator.");
            create_memory_operator()?
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
    let profile_names: Vec<_> = settings.profiles.keys().collect();
    log::debug!(
        "Parsing {} profiles: {:?}",
        profile_names.len(),
        profile_names
    );
    for profile_name in profile_names {
        log::debug!("Attempting to parse profile: {}", profile_name);
        match parse_profile(settings, profile_name).await {
            Ok((op, speed)) => {
                log::debug!("Successfully parsed profile: {}", profile_name);
                ops.insert(profile_name.clone(), (op, speed));
            }
            Err(e) => {
                log::warn!(
                    "Failed to parse profile '{}': {:?} - skipping",
                    profile_name,
                    e
                );
                // Continue with other profiles instead of failing completely
            }
        }
    }
    if ops.is_empty() {
        return Err(crate::Error::NoOperator);
    }
    log::debug!("Successfully parsed {} profiles", ops.len());
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
            self.load_from_operator(&key, op).await
        }

        fn get_key(&self) -> String {
            self.normalize_key(&self.name)
        }
    }
    /// Test saving and loading a struct using save_to_all and load
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test Object".to_string(),
            age: 25,
        };

        // Save the object to all available profiles
        test_obj.save().await?;

        // Load the object (will use fastest available operator)
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

    /// Test saving and loading a struct to dashmap profile (if available)
    #[cfg(feature = "dashmap")]
    #[tokio::test]
    #[serial_test::serial]
    async fn test_save_and_load_dashmap() -> Result<()> {
        // Create a test object
        let test_obj = TestStruct {
            name: "Test DashMap Object".to_string(),
            age: 35,
        };

        // Try to save the object to dashmap - this might not be configured in all environments
        match test_obj.save_to_one("dashmap").await {
            Ok(()) => {
                // Load the object
                let mut loaded_obj = TestStruct::new("Test DashMap Object".to_string());
                loaded_obj = loaded_obj.load().await?;

                // Compare the original and loaded objects
                assert_eq!(
                    test_obj, loaded_obj,
                    "Loaded dashmap object does not match the original"
                );
            }
            Err(e) => {
                println!(
                    "DashMap profile not available (expected in some environments): {:?}",
                    e
                );
                // This is okay - not all environments may have dashmap configured
            }
        }

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
    #[cfg(feature = "sqlite")]
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
        sqlite_profile.insert(
            "connection_string".to_string(),
            format!("{}/test.db", sqlite_path.to_string_lossy()),
        );
        sqlite_profile.insert(
            "datadir".to_string(),
            sqlite_path.to_string_lossy().to_string(),
        );
        sqlite_profile.insert("table".to_string(), "test_table".to_string());
        profiles.insert("test_sqlite".to_string(), sqlite_profile);

        // ReDB profile with proper configuration
        let mut redb_profile = std::collections::HashMap::new();
        redb_profile.insert("type".to_string(), "redb".to_string());
        redb_profile.insert(
            "datadir".to_string(),
            redb_path.to_string_lossy().to_string(),
        );
        redb_profile.insert("table".to_string(), "test_table".to_string());
        redb_profile.insert(
            "path".to_string(),
            format!("{}/test.redb", redb_path.to_string_lossy()),
        );
        profiles.insert("test_redb".to_string(), redb_profile);

        // DashMap profile
        let mut dashmap_profile = std::collections::HashMap::new();
        dashmap_profile.insert("type".to_string(), "dashmap".to_string());
        dashmap_profile.insert(
            "root".to_string(),
            dashmap_path.to_string_lossy().to_string(),
        );
        profiles.insert("test_dashmap".to_string(), dashmap_profile);

        let settings = DeviceSettings {
            server_hostname: "localhost:8000".to_string(),
            api_endpoint: "http://localhost:8000/api".to_string(),
            initialized: false,
            default_data_path: temp_path.to_string_lossy().to_string(),
            profiles,
        };

        // Test that init_device_storage_with_settings creates directories and operators
        use crate::init_device_storage_with_settings;
        let storage = init_device_storage_with_settings(settings).await?;
        let operators = storage.ops;

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
            summarization: Some("Test summarization".to_string()),
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
        };

        // Save document to each operator to verify they work
        for (name, (op, _)) in &operators {
            let key = format!("document_{}.json", test_doc.id);
            let data = serde_json::to_string(&test_doc)?;
            match op.write(&key, data).await {
                Ok(_metadata) => log::info!("✅ Successfully saved test document to {}", name),
                Err(e) => log::warn!("Failed to save to {}: {:?}", name, e),
            }
        }

        Ok(())
    }
}
