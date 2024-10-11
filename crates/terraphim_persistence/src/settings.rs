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

pub async fn parse_profile(
    settings: &DeviceSettings,
    profile_name: &str,
) -> Result<(Operator, u128)> {
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
    let op = match scheme {
        Scheme::Azblob => Operator::from_map::<services::Azblob>(profile.clone())?.finish(),
        Scheme::Azdls => Operator::from_map::<services::Azdls>(profile.clone())?.finish(),
        #[cfg(feature = "services-dashmap")]
        Scheme::Dashmap => {
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
        #[cfg(feature = "services-etcd")]
        Scheme::Etcd => Operator::from_map::<services::Etcd>(profile.clone())?.finish(),
        Scheme::Gcs => Operator::from_map::<services::Gcs>(profile.clone())?.finish(),
        Scheme::Ghac => Operator::from_map::<services::Ghac>(profile.clone())?.finish(),
        #[cfg(feature = "services-hdfs")]
        Scheme::Hdfs => Operator::from_map::<services::Hdfs>(profile.clone())?.finish(),
        Scheme::Http => Operator::from_map::<services::Http>(profile.clone())?.finish(),
        #[cfg(feature = "services-ftp")]
        Scheme::Ftp => Operator::from_map::<services::Ftp>(profile.clone())?.finish(),
        #[cfg(feature = "services-ipfs")]
        Scheme::Ipfs => Operator::from_map::<services::Ipfs>(profile.clone())?.finish(),
        Scheme::Ipmfs => Operator::from_map::<services::Ipmfs>(profile.clone())?.finish(),
        #[cfg(feature = "services-memcached")]
        Scheme::Memcached => Operator::from_map::<services::Memcached>(profile.clone())?.finish(),
        Scheme::Obs => Operator::from_map::<services::Obs>(profile.clone())?.finish(),
        Scheme::Oss => Operator::from_map::<services::Oss>(profile.clone())?.finish(),
        #[cfg(feature = "services-redis")]
        Scheme::Redis => Operator::from_map::<services::Redis>(profile.clone())?.finish(),
        #[cfg(feature = "services-rocksdb")]
        Scheme::Rocksdb => Operator::from_map::<services::Rocksdb>(profile.clone())?.finish(),
        Scheme::S3 => Operator::from_map::<services::S3>(profile.clone())?.finish(),
        #[cfg(feature = "services-sled")]
        Scheme::Sled => Operator::from_map::<services::Sled>(profile.clone())?.finish(),
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
    use serde::{Deserialize, Serialize};
    use async_trait::async_trait;
    use crate::Persistable;

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
            // make sure name doesn't contain any special characters or spaces and all lowercase
            self.name.clone().chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase()
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
        test_obj.save_to_one("dash").await?;

        // Load the object
        let mut loaded_obj = TestStruct::new("Test Object".to_string());
        loaded_obj = loaded_obj.load().await?;

        // Compare the original and loaded objects
        assert_eq!(test_obj, loaded_obj, "Loaded object does not match the original");

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
        assert_eq!(test_obj, loaded_obj, "Loaded object does not match the original");

        Ok(())
    }
}
