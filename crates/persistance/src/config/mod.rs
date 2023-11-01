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

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use opendal::layers::LoggingLayer;
use anyhow::anyhow;
use opendal::services;
use opendal::Operator;
use opendal::Result;
use opendal::Scheme;
use serde::Deserialize;
use toml;
use log::debug;


use std::time::{Duration, Instant};

#[derive(Deserialize, Default, Debug)]
pub struct Config {
    pub profiles: HashMap<String, HashMap<String, String>>,
}

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

impl Config {
    /// Load profiles from both environment variables and local config file,
    /// environment variables have higher precedence.
    pub fn load(fp: &Path) -> Result<Config> {
        let cfg = Config::load_from_file(fp)?;
        let profiles = Config::load_from_env().profiles.into_iter().fold(
            cfg.profiles,
            |mut acc, (name, opts)| {
                acc.entry(name).or_default().extend(opts);
                acc
            },
        );
        Ok(Config { profiles })
    }
    /// Parse a local config file.
    ///
    /// - If the config file is not present, a default Config is returned.
    pub fn load_from_file(config_path: &Path) -> Result<Config> {
        if !config_path.exists() {
            return Ok(Config::default());
        }
        let data = fs::read_to_string(config_path).unwrap_or_else(|_| "".to_string());
        Ok(toml::from_str(&data).unwrap())
    }

    /// Load config from environment variables.
    ///
    /// The format of each environment variable should be `TERRAPHIM_PROFILE_{PROFILE NAME}_{OPTION}`,
    /// such as `TERRAPHIM_PROFILE_PROFILE1_TYPE`, `TERRAPHIM_PROFILE_MY-PROFILE_ACCESS_KEY_ID`.
    ///
    /// Please note that the profile name cannot contain underscores.
    pub(crate) fn load_from_env() -> Config {
        let prefix = "terraphim_profile_";
        let profiles = env::vars()
            .filter_map(|(k, v)| {
                k.to_lowercase().strip_prefix(prefix).and_then(
                    |k| -> Option<(String, String, String)> {
                        if let Some((profile_name, param)) = k.split_once('_') {
                            return Some((profile_name.to_string(), param.to_string(), v));
                        }
                        None
                    },
                )
            })
            .fold(
                HashMap::new(),
                |mut acc: HashMap<String, HashMap<_, _>>, (profile_name, key, val)| {
                    acc.entry(profile_name).or_default().insert(key, val);
                    acc
                },
            );
        Config { profiles }
    }

    

    pub async fn parse_profile(&self, profile_name:&str) -> Result<(Operator, u128)> {
        async fn get_speed(op:Operator)->Result<u128>{
            let start_time = Instant::now();
            // let mut buf = vec![0u8; 1024*1024];
            let buf = "test data";
            op.write("test", buf).await?;
            let end_time = Instant::now();
            let save_time = end_time.duration_since(start_time).as_millis();
            let start_time = Instant::now();
            op.read("test").await?;
            let end_time = Instant::now();
            let load_time = end_time.duration_since(start_time).as_nanos();
            Ok(load_time)
            
        }
        
        let profile = self
            .profiles
            .get(profile_name)
            .ok_or_else(|| anyhow!("unknown profile: {}", profile_name)).unwrap();

        let svc = profile
            .get("type")
            .ok_or_else(|| anyhow!("missing 'type' in profile")).unwrap();
        let scheme = Scheme::from_str(svc)?;
        let op = match scheme {
            Scheme::Azblob => {
                Operator::from_map::<services::Azblob>(profile.clone())?.finish()
            },
            Scheme::Azdls =>{ 
                Operator::from_map::<services::Azdls>(profile.clone())?.finish()
            },
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
            },
            #[cfg(feature = "services-etcd")]
            Scheme::Etcd => {
                Operator::from_map::<services::Etcd>(profile.clone())?.finish()
            },
            Scheme::Gcs => {
                Operator::from_map::<services::Gcs>(profile.clone())?.finish()
            }
            Scheme::Ghac =>{ 
                Operator::from_map::<services::Ghac>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-hdfs")]
            Scheme::Hdfs => {
                Operator::from_map::<services::Hdfs>(profile.clone())?.finish()
            }
            Scheme::Http => {
                Operator::from_map::<services::Http>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-ftp")]
            Scheme::Ftp => {
                Operator::from_map::<services::Ftp>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-ipfs")]
            Scheme::Ipfs => {
                Operator::from_map::<services::Ipfs>(profile.clone())?.finish()
            }
            Scheme::Ipmfs => {
                Operator::from_map::<services::Ipmfs>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-memcached")]
            Scheme::Memcached => {
                Operator::from_map::<services::Memcached>(profile.clone())?.finish()
            }
            Scheme::Obs => {
                Operator::from_map::<services::Obs>(profile.clone())?.finish()
            }
            Scheme::Oss => {
                Operator::from_map::<services::Oss>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-redis")]
            Scheme::Redis =>{ 
                Operator::from_map::<services::Redis>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-rocksdb")]
            Scheme::Rocksdb =>{ 
                Operator::from_map::<services::Rocksdb>(profile.clone())?.finish()
            }
            Scheme::S3 => {
                Operator::from_map::<services::S3>(profile.clone())?.finish()
            }
            #[cfg(feature = "services-sled")]
            Scheme::Sled =>{ 
                Operator::from_map::<services::Sled>(profile.clone())?.finish()
            }
            Scheme::Webdav =>{ 
                Operator::from_map::<services::Webdav>(profile.clone())?.finish()
            }
            Scheme::Webhdfs =>{ 
                Operator::from_map::<services::Webhdfs>(profile.clone())?.finish()
            }
            _ => {
                let builder = services::Memory::default();
                // Init an operator
                let op = Operator::new(builder)?
                    // Init with logging layer enabled.
                    .layer(LoggingLayer::default())
                    .finish();
                op
            }
        };
        let speed = get_speed(op.clone()).await?;
        Ok((op, speed))
    }
    pub async fn parse_profiles(&self) -> Result<HashMap<String,(Operator, u128)>> {
        let mut ops = HashMap::new();
        let profile_names = self.profiles.keys();
        for profile_name in profile_names {
            let (op, speed) = self.parse_profile(profile_name).await?;
            ops.insert(profile_name.clone(), (op, speed));
        }
        Ok(ops)
    }
}
#[cfg(test)]
mod tests {
    use opendal::Scheme;

    use super::*;

    #[test]
    fn test_load_from_env() {
        let env_vars = vec![
            ("TERRAPHIM_PROFILE_TEST1_TYPE", "s3"),
            ("TERRAPHIM_PROFILE_TEST1_ACCESS_KEY_ID", "foo"),
            ("TERRAPHIM_PROFILE_TEST2_TYPE", "oss"),
            ("TERRAPHIM_PROFILE_TEST2_ACCESS_KEY_ID", "bar"),
        ];
        for (k, v) in &env_vars {
            env::set_var(k, v);
        }

        let profiles = Config::load_from_env().profiles;

        let profile1 = profiles["test1"].clone();
        assert_eq!(profile1["type"], "s3");
        assert_eq!(profile1["access_key_id"], "foo");

        let profile2 = profiles["test2"].clone();
        assert_eq!(profile2["type"], "oss");
        assert_eq!(profile2["access_key_id"], "bar");

        for (k, _) in &env_vars {
            env::remove_var(k);
        }
    }

    #[test]
    fn test_load_from_toml() -> Result<()> {
        let dir = tempfile::tempdir().unwrap();
        let tmpfile = dir.path().join("oli1.toml");
        fs::write(
            &tmpfile,
            r#"
[profiles.mys3]
type = "s3"
region = "us-east-1"
access_key_id = "foo"
enable_virtual_host_style = "on"
"#,
        ).unwrap();
        let cfg = Config::load_from_file(&tmpfile)?;
        let profile = cfg.profiles["mys3"].clone();
        assert_eq!(profile["region"], "us-east-1");
        assert_eq!(profile["access_key_id"], "foo");
        assert_eq!(profile["enable_virtual_host_style"], "on");
        Ok(())
    }

    #[test]
    fn test_load_config_from_file_and_env() -> Result<()> {
        let dir = tempfile::tempdir().unwrap();
        let tmpfile = dir.path().join("oli2.toml");
        fs::write(
            &tmpfile,
            r#"
    [profiles.mys3]
    type = "s3"
    region = "us-east-1"
    access_key_id = "foo"
    "#,
        ).unwrap();
        let env_vars = vec![
            ("TERRAPHIM_PROFILE_MYS3_REGION", "us-west-1"),
            ("TERRAPHIM_PROFILE_MYS3_ENABLE_VIRTUAL_HOST_STYLE", "on"),
        ];
        for (k, v) in &env_vars {
            env::set_var(k, v);
        }
        let cfg = Config::load(&tmpfile)?;
        let profile = cfg.profiles["mys3"].clone();
        assert_eq!(profile["region"], "us-west-1");
        assert_eq!(profile["access_key_id"], "foo");
        assert_eq!(profile["enable_virtual_host_style"], "on");

        for (k, _) in &env_vars {
            env::remove_var(k);
        }
        Ok(())
    }
}
