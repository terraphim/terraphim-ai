use directories::ProjectDirs;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use twelf::reexports::toml;
use twelf::{Layer, config};

#[cfg(feature = "onepassword")]
use terraphim_onepassword_cli::{OnePasswordLoader, SecretLoader};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("config error: {0}")]
    ConfigError(#[from] twelf::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("env error: {0}")]
    EnvError(#[from] std::env::VarError),
    #[cfg(feature = "onepassword")]
    #[error("1Password error: {0}")]
    OnePasswordError(#[from] terraphim_onepassword_cli::OnePasswordError),
}

// Need to name it explicitly to avoid conflict with std::Result
// which gets used by the `#[config]` macro below.
pub type DeviceSettingsResult<T> = std::result::Result<T, Error>;

/// Default config path
pub const DEFAULT_CONFIG_PATH: &str = ".config";

/// Default settings file
pub const DEFAULT_SETTINGS: &str = include_str!("../default/settings_local_dev.toml");

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
    /// default data path
    pub default_data_path: String,
    /// configured storage backends available on device
    pub profiles: HashMap<String, HashMap<String, String>>,
}

impl Default for DeviceSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceSettings {
    /// Create a new DeviceSettings
    pub fn new() -> Self {
        Self::load_from_env_and_file(None).unwrap_or_else(|e| {
            log::warn!(
                "Failed to load device settings from file: {:?}, using defaults",
                e
            );
            Self::default_embedded()
        })
    }

    /// Load settings with 1Password secret resolution
    #[cfg(feature = "onepassword")]
    pub async fn load_with_onepassword(config_path: Option<PathBuf>) -> DeviceSettingsResult<Self> {
        log::info!("Loading device settings with 1Password integration...");
        let config_path = config_path.unwrap_or_else(Self::default_config_path);

        log::debug!("Settings path: {:?}", config_path);
        let config_file = init_config_file(&config_path)?;
        log::debug!("Loading config_file: {:?}", config_file);

        // Read the raw configuration file
        let raw_config = std::fs::read_to_string(&config_file)?;

        // Process 1Password references
        let loader = OnePasswordLoader::new();
        let processed_config = if loader.is_available().await {
            log::info!("1Password CLI available, processing secrets...");
            loader.process_config(&raw_config).await?
        } else {
            log::warn!("1Password CLI not available, using raw configuration");
            raw_config
        };

        // Parse the processed configuration
        let settings: DeviceSettings = toml::from_str(&processed_config).map_err(|e| {
            Error::IoError(std::io::Error::other(format!("TOML parsing error: {}", e)))
        })?;

        log::info!("Successfully loaded settings with 1Password integration");
        Ok(settings)
    }

    /// Process a configuration string with 1Password secret resolution
    #[cfg(feature = "onepassword")]
    pub async fn process_config_with_secrets(config: &str) -> DeviceSettingsResult<String> {
        let loader = OnePasswordLoader::new();
        if loader.is_available().await {
            Ok(loader.process_config(config).await?)
        } else {
            log::warn!("1Password CLI not available, returning raw configuration");
            Ok(config.to_string())
        }
    }

    /// Create default embedded DeviceSettings without filesystem operations
    /// Used for embedded/offline mode where config files are not needed
    pub fn default_embedded() -> Self {
        use std::collections::HashMap;

        let mut profiles = HashMap::new();

        // Get user data directory for persistent storage
        // Use ProjectDirs for cross-platform paths, fallback to ~/.terraphim
        let data_dir = if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
            proj_dirs.data_dir().to_string_lossy().to_string()
        } else if let Ok(home) = std::env::var("HOME") {
            format!("{}/.terraphim", home)
        } else {
            // Fallback to /tmp only if we can't get a better path
            "/tmp/terraphim_embedded".to_string()
        };

        // Use only SQLite for persistent storage
        // DashMap disabled - causes role selections to be lost between CLI invocations
        let mut sqlite_profile = HashMap::new();
        sqlite_profile.insert("type".to_string(), "sqlite".to_string());
        sqlite_profile.insert("datadir".to_string(), format!("{}/sqlite", data_dir));
        sqlite_profile.insert(
            "connection_string".to_string(),
            format!("{}/sqlite/terraphim.db", data_dir),
        );
        sqlite_profile.insert("table".to_string(), "terraphim_kv".to_string());
        profiles.insert("sqlite".to_string(), sqlite_profile);

        Self {
            server_hostname: "127.0.0.1:8000".to_string(),
            api_endpoint: "http://localhost:8000/api".to_string(),
            initialized: true,
            default_data_path: data_dir,
            profiles,
        }
    }
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
    /// config path shall be a folder and not file
    pub fn load_from_env_and_file(config_path: Option<PathBuf>) -> DeviceSettingsResult<Self> {
        log::info!("Loading device settings...");
        let config_path = match config_path {
            Some(path) => path,
            None => DeviceSettings::default_config_path(),
        };

        log::debug!("Settings path: {:?}", config_path);
        let config_file = init_config_file(&config_path)?;
        log::debug!("Loading config_file: {:?}", config_file);

        Ok(DeviceSettings::with_layers(&[
            Layer::Toml(config_file),
            Layer::Env(Some(String::from("TERRAPHIM_"))),
        ])?)
    }
    pub fn update_initialized_flag(
        &mut self,
        settings_path: Option<PathBuf>,
        initialized: bool,
    ) -> Result<(), Error> {
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
        let serialized_settings =
            toml::to_string_pretty(self).map_err(|e| Error::IoError(std::io::Error::other(e)))?;

        std::fs::write(path, serialized_settings).map_err(Error::IoError)?;

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

    use envtestkit::lock::lock_test;

    #[test]
    fn test_env_variable() {
        let _lock = lock_test();
        // Test that config loading works with test settings
        let config =
            DeviceSettings::load_from_env_and_file(Some(PathBuf::from("./test_settings/")));

        log::debug!("Config: {:?}", config);

        // Verify config loads successfully and has expected structure
        let config = config.unwrap();
        assert!(config.profiles.contains_key("dashmap"));
        assert!(config.profiles.contains_key("sqlite"));

        // Verify dashmap profile has required fields
        let dashmap_profile = config.profiles.get("dashmap").unwrap();
        assert!(dashmap_profile.contains_key("root"));
        assert!(dashmap_profile.contains_key("type"));
        assert_eq!(dashmap_profile.get("type").unwrap(), "dashmap");
    }

    #[test]
    fn test_update_initialized_flag() {
        let test_config_path = PathBuf::from("./test_settings/");

        // Check if initialized is false
        let mut config =
            DeviceSettings::load_from_env_and_file(Some(test_config_path.clone())).unwrap();
        config.initialized = false;
        assert!(!config.initialized);

        // Update to true
        config
            .update_initialized_flag(Some(test_config_path.clone()), true)
            .unwrap();

        // Check if initialized is now true
        let config_copy =
            DeviceSettings::load_from_env_and_file(Some(test_config_path.clone())).unwrap();
        assert!(config_copy.initialized);
    }
}
