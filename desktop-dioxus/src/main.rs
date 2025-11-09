use dioxus::desktop::{Config as WindowConfig, WindowBuilder};
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;
use terraphim_settings::DeviceSettings;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tray_icon::menu::MenuEvent;

mod app;
mod components;
mod state;
mod services;
mod routes;
mod utils;
mod system_tray;
mod global_shortcuts;

use app::App;
use system_tray::{SystemTrayManager, handle_menu_event, TrayEvent};
use global_shortcuts::ShortcutManager;

// Global broadcast channel for tray events (allows multiple receivers)
static TRAY_EVENT_SENDER: once_cell::sync::OnceCell<broadcast::Sender<TrayEvent>> = once_cell::sync::OnceCell::new();

// Global tray manager for updating menu
static TRAY_MANAGER: once_cell::sync::OnceCell<Mutex<SystemTrayManager>> = once_cell::sync::OnceCell::new();

pub fn subscribe_to_tray_events() -> Option<broadcast::Receiver<TrayEvent>> {
    TRAY_EVENT_SENDER.get().map(|sender| sender.subscribe())
}

pub async fn update_tray_menu(config: &terraphim_config::Config) -> anyhow::Result<()> {
    if let Some(manager_mutex) = TRAY_MANAGER.get() {
        let mut manager = manager_mutex.lock().await;
        manager.update_menu(config)?;
    }
    Ok(())
}

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

    // Store tray manager globally for updates
    let _ = TRAY_MANAGER.set(Mutex::new(tray_manager));

    // Create tray event broadcast channel (capacity of 100 events)
    let (tray_tx, _tray_rx) = broadcast::channel::<TrayEvent>(100);
    let _ = TRAY_EVENT_SENDER.set(tray_tx.clone());

    // Set up global shortcuts
    let mut shortcut_manager = ShortcutManager::new()?;

    // Try to get custom shortcut from config, otherwise use default
    match config.extra.get("global_shortcut") {
        Some(shortcut_str) if !shortcut_str.is_empty() => {
            if let Err(e) = shortcut_manager.register_custom_shortcut(shortcut_str) {
                tracing::warn!("Failed to register custom shortcut '{}': {:?}, using default", shortcut_str, e);
                shortcut_manager.register_toggle_shortcut()?;
            }
        }
        _ => {
            shortcut_manager.register_toggle_shortcut()?;
        }
    }

    // Listen for global shortcut events
    let tray_tx_for_shortcuts = tray_tx.clone();
    tokio::spawn(async move {
        global_shortcuts::listen_for_shortcuts(move || {
            tracing::info!("Global shortcut pressed, sending Toggle event");
            let _ = tray_tx_for_shortcuts.send(TrayEvent::Toggle);
        }).await;
    });

    // Handle tray menu events in background
    tokio::spawn(async move {
        let menu_channel = MenuEvent::receiver();
        loop {
            if let Ok(event) = menu_channel.recv() {
                if let Some(tray_event) = handle_menu_event(event) {
                    match &tray_event {
                        TrayEvent::Toggle => {
                            tracing::info!("Toggle window requested");
                        }
                        TrayEvent::RoleChanged(role_name) => {
                            tracing::info!("Role changed to: {}", role_name);
                        }
                        TrayEvent::Quit => {
                            tracing::info!("Quit requested");
                            std::process::exit(0);
                        }
                    }

                    // Send event to Dioxus app (except Quit which is handled immediately)
                    if !matches!(tray_event, TrayEvent::Quit) {
                        if let Some(sender) = TRAY_EVENT_SENDER.get() {
                            let _ = sender.send(tray_event);
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
