use std::path::{Path, PathBuf};

use config::{Config, ConfigError, Environment, File};
use directories::ProjectDirs;
use serde::Deserialize;

use std::collections::HashMap;

/// Configuration settings for the device or server.
/// These values are set when the server initializes, and do not change while running.
/// These are constructed from default or local files and ENV variables.
#[derive(Clone, Debug, Deserialize, Default)]
pub struct Settings {
    /// The address to listen on
    pub server_url: Option<String>,
    pub config_file: String,
    pub api_endpoint: Option<String>,
    pub profiles: AHashMap<String, AHashMap<String, String>>,
}
impl Settings {
    pub fn new(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let config_file = match config_path {
            Some(path) => create_config_folder(&path).unwrap(),
            None => {
                if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
                    let config_dir = proj_dirs.config_dir();
                    create_config_folder(&config_dir.to_path_buf()).unwrap()
                } else {
                    create_config_folder(&PathBuf::from(".config/")).unwrap()
                }
            }
        };

        let settings = Config::builder()
            .add_source(File::with_name(config_file.to_str().unwrap()))
            .add_source(
                Environment::with_prefix("TERRAPHIM").try_parsing(true), // .separator("_")
            )
            .set_default("config_file", config_file.to_str())?;

        match settings.build() {
            Ok(config) => {
                log::warn!("Settings: {:?}", config);
                Ok(config.try_deserialize())?
            }
            Err(e) => {
                log::warn!("Error: {:?}", e);
                Err(e)
            }
        }
    }
}

fn create_config_folder(path: &PathBuf) -> Result<PathBuf, ConfigError> {
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }
    let filename = path.join("settings.toml");

    if filename.exists() {
        log::warn!("File exists");
        log::warn!("{:?}", filename);
    } else {
        log::warn!("File does not exist");
        std::fs::copy("default/settings.toml", &filename).unwrap();
    }
    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;
    use opendal::Result as OpenDalResult;
    use std::env;
    use std::fs;
    use test_log::test;

    #[test]
    fn test_load_from_toml() -> OpenDalResult<()> {
        let dir = tempfile::tempdir().unwrap();
        let tmpfile = dir.path().join("settings.toml");
        println!("Info {:?}", tmpfile);
        fs::write(
            &tmpfile,
            r#"
            server_url = "127.0.0.1:3000"
            api_endpoint="http://localhost:3000/api"
            
            [profiles.s3]
            type = "s3"
            bucket = "test"
            region = "us-east-1"
            endpoint = "http://rpi4node3:8333/"
            access_key_id = ""
            secret_access_key = ""
            
            [profiles.sled]
            type = "sled"
            datadir= "/tmp/opendal/sled"
            
            [profiles.dash]
            type = "dashmap"
            root = "/tmp/dashmaptest"
            
            [profiles.rock]
            type = "rocksdb"
            datadir = "/tmp/opendal/rocksdb"
"#,
        )
        .unwrap();
        let settings = Settings::new(Some(dir.path().to_path_buf())).unwrap();
        println!("{:?}", settings);
        println!("Profile {:?}", settings.profiles);
        println!("Profile s3 {:?}", settings.profiles.get("s3"));
        println!("Profile mys3 {:?}", settings.profiles.get("dash"));
        // let profile = cfg.profiles["mys3"].clone();
        // assert_eq!(profile["region"], "us-east-1");
        // assert_eq!(profile["access_key_id"], "foo");
        // assert_eq!(profile["enable_virtual_host_style"], "on");
        Ok(())
    }

    #[test]
    fn test_load_config_from_file_and_env() -> OpenDalResult<()> {
        let dir = tempfile::tempdir().unwrap();
        let tmpfile = dir.path().join("settings.toml");
        fs::write(
            &tmpfile,
            r#"
            server_url = "127.0.0.1:3000"
            api_endpoint="http://localhost:3000/api"
            
            [profiles.s3]
            type = "s3"
            bucket = "test"
            region = "us-east-1"
            endpoint = "http://rpi4node3:8333/"
            access_key_id = ""
            secret_access_key = ""
            
            [profiles.sled]
            type = "sled"
            datadir= "/tmp/opendal/sled"
            
            [profiles.dash]
            type = "dashmap"
            root = "/tmp/dashmaptest"
            
            [profiles.rock]
            type = "rocksdb"
            datadir = "/tmp/opendal/rocksdb"
    "#,
        )
        .unwrap();
        let env_vars = vec![
            ("TERRAPHIM_PROFILE_2MYS3_REGION", "us-west-1"),
            ("TERRAPHIM_PROFILE_2MYS3_ENABLE_VIRTUAL_HOST_STYLE", "on"),
        ];
        for (k, v) in &env_vars {
            env::set_var(k, v);
        }
        let settings = Settings::new(Some(dir.path().to_path_buf())).unwrap();
        println!("Settings {:#?}", settings);
        log::warn!("Settings profile keys {:?}", settings.profiles.keys());
        log::warn!("Settings names {:?}", settings.profiles.get("sled"));
        log::warn!("Settings 2my3 {:?}", settings.profiles.get("2mys3"));
        // assert_eq!(profile["region"], "us-west-1");
        // assert_eq!(profile["access_key_id"], "foo");
        // assert_eq!(profile["enable_virtual_host_style"], "on");

        for (k, _) in &env_vars {
            env::remove_var(k);
        }
        Ok(())
    }
}
