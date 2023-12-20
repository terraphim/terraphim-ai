use std::collections::HashMap;
use std::path::PathBuf;
use directories::ProjectDirs;
use twelf::reexports::serde::{Deserialize, Serialize};
use twelf::{config, Layer};


#[config]
#[derive(Debug, Default)]
pub struct Settings {
    /// The address to listen on
    pub server_url: Option<String>,
    /// API endpoint for the server
    pub api_endpoint: Option<String>,
    /// configured storage backends available on device
    pub profiles: HashMap<String, HashMap<String, String>>,
}
impl Settings {
    pub fn load_from_env_and_file(config_path:Option<PathBuf>) -> Self {
        let config_file = match config_path {
            Some(path) => create_config_folder(&path).unwrap(),
            None => if let Some(proj_dirs) = ProjectDirs::from("com", "aks", "terraphim") {
                let config_dir = proj_dirs.config_dir();
                create_config_folder(&config_dir.to_path_buf()).unwrap()
            }else{
                create_config_folder(&PathBuf::from(".config/")).unwrap()
            }
        };
        
        let config = Settings::with_layers(&[
            Layer::Toml(config_file),
            Layer::Env(Some(String::from("TERRAPHIM_"))),
        ])
        .unwrap();
        config
    }
}

fn create_config_folder(path: &PathBuf) -> Result<PathBuf, std::io::Error> {
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
            config.profiles.get("s3").unwrap().get("region").unwrap(),
            &String::from("us-west-1")
        );
        for (k, _) in &env_vars {
            std::env::remove_var(k);
        }
    }
}
