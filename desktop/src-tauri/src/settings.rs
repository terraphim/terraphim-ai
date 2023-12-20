#![allow(unused)] // silence unused warnings while exploring (to comment out)
use std::env;
use std::fs;
use std::path::Path;
extern crate directories;
use directories::ProjectDirs;
use std::collections::HashMap;

use config::{ConfigError, Config, File, Environment};
use serde_derive::{Serialize,Deserialize};


lazy_static! {
    pub static ref CONFIG: Settings =
        Settings::new().expect("config can be loaded");
}


/// This is what we're going to decode into. Each field is optional, meaning
/// that it doesn't have to be present in TOML.
/// 
#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
    pub global_shortcut: String,
    pub role: HashMap<String,Role>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    pub name: String,
    pub relevance_function: String,
    pub theme: String,
    pub plugins: Vec<Plugin>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Plugin {
    pub name: String,
    pub hackstack: String,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {

    let mut s = Config::default();

    // Start off by merging in the "default" configuration file
    s.merge(File::with_name("config/default"))?;
    let env = env::var("RUN_MODE").unwrap_or_else(|_| "development".into());
    println!("env: {}", env);
    if let Some(proj_dirs) = ProjectDirs::from("com", "aks",  "terraphim") {
        proj_dirs.config_dir();
        println!("Project Dir {:?}", proj_dirs.config_dir());
        println!("Create folder if doesn't exist");
        std::fs::create_dir_all(proj_dirs.config_dir()).unwrap();
        
        let filename= proj_dirs.config_dir().join("config.toml");
        
        if filename.exists() {
            println!("File exists");
            println!("{:?}", filename);
            s.merge(File::with_name(filename.to_str().unwrap()))?;
        } else {
            println!("File does not exist");
            std::fs::copy("config/default.toml", filename).unwrap();
    
        }  
        

    }
    s.merge(Environment::with_prefix("app"))?;
    s.try_deserialize()
    }
}

