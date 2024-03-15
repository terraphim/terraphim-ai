use directories::ProjectDirs;
use std::collections::HashMap;
use std::path::PathBuf;
use twelf::{config, Layer};

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
pub type SettingsResult<T> = std::result::Result<T, Error>;

/// Configuration settings for the device or server.
///
/// These values are set when the server initializes, and do not change while
/// running. These are constructed from default or local files and ENV
/// variables.
#[config]
#[derive(Debug)]
pub struct Settings {
    /// The address to listen on
    pub server_hostname: String,
    /// API endpoint for the server
    pub api_endpoint: String,
    /// configured storage backends available on device
    pub profiles: HashMap<String, HashMap<String, String>>,
}

impl Settings {
    /// Load settings from environment and file
    pub fn load_from_env_and_file(config_path: Option<PathBuf>) -> SettingsResult<Self> {
        log::info!("Loading device settings...");
        let config_path = match config_path {
            Some(path) => path,
            None => {
                if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
                    let config_dir = proj_dirs.config_dir();
                    config_dir.to_path_buf()
                } else {
                    PathBuf::from(".config/")
                }
            }
        };
        let config_file = init_config_file(&config_path)?;
        log::info!("Using config_file: {:?}", config_file);

        Ok(Settings::with_layers(&[
            Layer::Toml(config_file),
            Layer::Env(Some(String::from("TERRAPHIM_"))),
        ])?)
    }
}

/// Initialize the config file if it doesn't exist
fn init_config_file(path: &PathBuf) -> Result<PathBuf, std::io::Error> {
    log::info!("Initializing config file at: {:?}", path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }
    log::info!("Checking for settings.toml");
    let config_file = path.join("settings.toml");
    if !config_file.exists() {
        log::warn!("Creating default config at: {:?}", config_file);
        let default_config = include_str!("../default/settings_local.toml");
        std::fs::write(&config_file, default_config)?;
    }
    Ok(config_file)
}

/// To run test with logs and variables use:
/// RUST_LOG="info,warn" TERRAPHIM_API_ENDPOINT="test_endpoint" TERRAPHIM_PROFILE_S3_REGION="us-west-1" cargo test -- --nocapture
#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn test_env_variable() {
        let env_vars = vec![
            ("TERRAPHIM_PROFILE_S3_REGION", "us-west-1"),
            ("TERRAPHIM_PROFILE_S3_ENABLE_VIRTUAL_HOST_STYLE", "on"),
        ];
        for (k, v) in &env_vars {
            std::env::set_var(k, v);
        }
        let config = Settings::load_from_env_and_file(None);
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
}
