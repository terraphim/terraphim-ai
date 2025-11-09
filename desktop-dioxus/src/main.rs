use dioxus::desktop::{Config as WindowConfig, WindowBuilder};
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;
use terraphim_settings::DeviceSettings;
use std::sync::Arc;
use tokio::sync::Mutex;
use tray_icon::menu::MenuEvent;

mod app;
mod components;
mod state;
mod services;
mod routes;
mod utils;
mod system_tray;

use app::App;
use system_tray::{SystemTrayManager, handle_menu_event, TrayEvent};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    tracing::info!("Starting Terraphim Desktop (Dioxus)...");

    // Load device settings
    let device_settings = match DeviceSettings::load_from_env_and_file(None) {
        Ok(settings) => {
            tracing::info!("Loaded device settings: {:?}", settings);
            settings
        }
        Err(e) => {
            tracing::warn!("Failed to load device settings: {:?}, using defaults", e);
            DeviceSettings::new()
        }
    };

    // Load or create configuration
    let config = load_or_create_config().await?;
    tracing::info!("Configuration loaded for role: {:?}", config.selected_role);

    // Set up system tray
    let mut tray_manager = SystemTrayManager::new()?;
    tray_manager.build_menu(&config)?;

    // Create tray icon
    let icon_path = "assets/icons/icon.png";
    tray_manager.create_tray_icon(icon_path)?;
    tracing::info!("System tray created");

    // Handle tray menu events in background
    tokio::spawn(async move {
        let menu_channel = MenuEvent::receiver();
        loop {
            if let Ok(event) = menu_channel.recv() {
                if let Some(tray_event) = handle_menu_event(event) {
                    match tray_event {
                        TrayEvent::Toggle => {
                            tracing::info!("Toggle window requested");
                            // TODO: Implement window toggle
                        }
                        TrayEvent::RoleChanged(role_name) => {
                            tracing::info!("Role changed to: {}", role_name);
                            // TODO: Update config with new role
                        }
                        TrayEvent::Quit => {
                            tracing::info!("Quit requested");
                            std::process::exit(0);
                        }
                    }
                }
            }
        }
    });

    // Launch Dioxus app
    tracing::info!("Launching Dioxus application...");
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            WindowConfig::new()
                .with_window_size((1024.0, 768.0))
        )
        .launch(App);

    Ok(())
}

async fn load_or_create_config() -> anyhow::Result<terraphim_config::Config> {
    match ConfigBuilder::new_with_id(ConfigId::Desktop).build() {
        Ok(mut config) => match config.load().await {
            Ok(loaded) => Ok(loaded),
            Err(e) => {
                tracing::warn!("Failed to load config: {:?}, creating default", e);
                ConfigBuilder::new()
                    .build_default_desktop()
                    .build()
                    .map_err(|e| anyhow::anyhow!("Failed to build default config: {:?}", e))
            }
        },
        Err(e) => {
            tracing::warn!("Failed to build config: {:?}, creating default", e);
            ConfigBuilder::new()
                .build_default_desktop()
                .build()
                .map_err(|e| anyhow::anyhow!("Failed to build default config: {:?}", e))
        }
    }
}
