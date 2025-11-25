//! Terraphim REPL - Offline-capable semantic knowledge graph search
//!
//! A standalone REPL (Read-Eval-Print-Loop) interface for searching and exploring
//! knowledge graphs using semantic search. Works offline with embedded defaults.

use anyhow::{Context, Result};
use rust_embed::RustEmbed;
use std::path::PathBuf;

mod repl;
mod service;

use service::TuiService;

/// Embedded default assets (config and thesaurus)
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Assets;

/// Get or create the terraphim config directory
fn get_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::home_dir()
        .context("Could not find home directory")?
        .join(".terraphim");

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .context("Failed to create config directory")?;
    }

    Ok(config_dir)
}

/// Initialize default configuration if not present
fn init_default_config() -> Result<()> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.json");

    // Only create if it doesn't exist
    if !config_path.exists() {
        if let Some(default_config) = Assets::get("default_config.json") {
            std::fs::write(&config_path, default_config.data.as_ref())
                .context("Failed to write default config")?;
            println!("✓ Created default configuration at {}", config_path.display());
        }
    }

    Ok(())
}

/// Initialize default thesaurus if not present
fn init_default_thesaurus() -> Result<()> {
    let config_dir = get_config_dir()?;
    let thesaurus_path = config_dir.join("default_thesaurus.json");

    // Only create if it doesn't exist
    if !thesaurus_path.exists() {
        if let Some(default_thesaurus) = Assets::get("default_thesaurus.json") {
            std::fs::write(&thesaurus_path, default_thesaurus.data.as_ref())
                .context("Failed to write default thesaurus")?;
            println!("✓ Created default thesaurus at {}", thesaurus_path.display());
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize default assets on first run
    init_default_config()?;
    init_default_thesaurus()?;

    // Initialize the service (offline mode)
    let service = TuiService::new()
        .await
        .context("Failed to initialize Terraphim service")?;

    // Launch REPL
    let mut handler = repl::ReplHandler::new_offline(service);
    handler.run().await
}
