use directories::ProjectDirs;
use std::collections::HashMap;
use std::path::PathBuf;
use twelf::{config, Layer};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use twelf::reexports::toml;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config error: {0}")]
    ConfigError(#[from] twelf::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("env error: {0}")]
    EnvError(#[from] std::env::VarError),
}

// Need to name it explicitly to avoid conflict with std::Result
// which gets used by the `#[config]` macro below.
pub type DeviceSettingsResult<T> = std::result::Result<T, Error>;

/// Default config path
pub const DEFAULT_CONFIG_PATH: &str = ".config";

/// Default settings file
pub const DEFAULT_SETTINGS: &str = include_str!("../default/settings_local.toml");

fn deserialize_bool_from_string<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    // This will accept both string and bool values
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BoolOrString {
        Bool(bool),
        String(String),
    }

    match BoolOrString::deserialize(deserializer)? {
        BoolOrString::Bool(b) => Ok(b),
        BoolOrString::String(s) => match s.to_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(de::Error::custom(format!("invalid boolean value: {}", s))),
        },
    }
}

/// Configuration settings for the device (i.e. the server or runtime).
///
/// These values are set when the server is initialized, and do not change while
/// running. These are constructed from default or local files and ENV
/// variables.
#[config]
#[derive(Debug, Serialize, Clone)]
pub struct DeviceSettings {
    /// The address to listen on
    pub server_hostname: String,
    /// API endpoint for the server
    pub api_endpoint: String,
    /// init completed
    #[serde(deserialize_with = "deserialize_bool_from_string")]
    pub initialized: bool,
    /// configured storage backends available on device
    pub profiles: HashMap<String, HashMap<String, String>>,
}

impl DeviceSettings {
    /// Get the default path for the config file
    ///
    /// This is the default path where the config file is stored.
    pub fn default_config_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
            let config_dir = proj_dirs.config_dir();
            config_dir.to_path_buf()
        } else {
            PathBuf::from(DEFAULT_CONFIG_PATH)
        }
    }

    /// Load settings from environment and file
    pub fn load_from_env_and_file(config_path: Option<PathBuf>) -> DeviceSettingsResult<Self> {
        log::info!("Loading device settings...");
        let config_path = match config_path {
            Some(path) => path,
            None => DeviceSettings::default_config_path(),
        };
        println!("Settings path: {:?}", config_path);
        log::debug!("Settings path: {:?}", config_path);
        let config_file = init_config_file(&config_path)?;
        log::debug!("Loading config_file: {:?}", config_file);

        Ok(DeviceSettings::with_layers(&[
            Layer::Toml(config_file),
            Layer::Env(Some(String::from("TERRAPHIM_"))),
        ])?)
    }
    pub fn update_initialized_flag(&mut self, settings_path: Option<PathBuf>, initialized: bool) -> Result<(), Error> {
        let settings_path = settings_path.unwrap_or_else(Self::default_config_path);
        let settings_path = settings_path.join("settings.toml");
        self.initialized = initialized;
        self.save(&settings_path)?;
        Ok(())
    }

    /// Save the current settings to a file
    pub fn save(&self, path: &PathBuf) -> Result<(), Error> {
        log::info!("Saving device settings to: {:?}", path);
        self.save_to_file(path)?;
        Ok(())
    }

    /// Save settings to a specified file
    fn save_to_file(&self, path: &PathBuf) -> Result<(), Error> {
        let serialized_settings = toml::to_string_pretty(self)
            .map_err(|e| Error::IoError(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        
        std::fs::write(path, serialized_settings)
            .map_err(Error::IoError)?;
        
        Ok(())
    }
}

/// Initialize the config file if it doesn't exist
fn init_config_file(path: &PathBuf) -> Result<PathBuf, std::io::Error> {
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    let config_file = path.join("settings.toml");
    if !config_file.exists() {
        log::info!("Initializing default config file at: {:?}", path);
        std::fs::write(&config_file, DEFAULT_SETTINGS)?;
    } else {
        log::debug!("Config file exists at: {:?}", config_file);
    }
    Ok(config_file)
}


/// To run test with logs and variables use:
/// RUST_LOG="info,warn" TERRAPHIM_API_ENDPOINT="test_endpoint" TERRAPHIM_PROFILE_S3_REGION="us-west-1" cargo test -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;
    use tempfile::tempdir;

    #[test]
    fn test_env_variable() {
        let env_vars = vec![
            ("TERRAPHIM_PROFILE_S3_REGION", "us-west-1"),
            ("TERRAPHIM_PROFILE_S3_ENABLE_VIRTUAL_HOST_STYLE", "on"),
        ];
        for (k, v) in &env_vars {
            std::env::set_var(k, v);
        }
        let config = DeviceSettings::load_from_env_and_file(None);
        println!("{:?}", config);

        assert_eq!(
            config
                .unwrap()
                .profiles
                .get("s3")
                .unwrap()
                .get("region")
                .unwrap(),
            &String::from("us-west-1")
        );
        for (k, _) in &env_vars {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn test_update_initialized_flag() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().to_path_buf();

        // Initialize with false
        update_initialized_flag(&config_path, false).unwrap();

        // Check if initialized is false
        let config = DeviceSettings::load_from_env_and_file(Some(config_path.clone())).unwrap();
        assert_eq!(config.initialized, false);

        // Update to true
        update_initialized_flag(&config_path, true).unwrap();

        // Check if initialized is now true
        let config = DeviceSettings::load_from_env_and_file(Some(config_path.clone())).unwrap();
        assert_eq!(config.initialized, true);
    }
}