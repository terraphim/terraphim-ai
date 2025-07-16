//! terraphim API (AXUM) server
#![warn(
    clippy::all,
    clippy::pedantic,
    absolute_paths_not_starting_with_crate,
    rustdoc::invalid_html_tags,
    missing_copy_implementations,
    missing_debug_implementations,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_extern_crates,
    variant_size_differences,
    clippy::missing_const_for_fn
)]
#![deny(anonymous_parameters, macro_use_extern_crate)]
#![deny(missing_docs)]


use anyhow::Context;
use std::net::SocketAddr;
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;
use terraphim_config::ConfigState;
use terraphim_server::{axum_server, Result};
use terraphim_settings::DeviceSettings;

#[tokio::main]
async fn main() -> Result<()> {
    match run_server().await {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("Error: {e:#?}");
            std::process::exit(1)
        }
    }
}

async fn run_server() -> Result<()> {
    // Set up logger for the server
    env_logger::init();

    let server_settings =
        DeviceSettings::load_from_env_and_file(None).context("Failed to load settings")?;
    log::info!(
        "Device settings hostname: {:?}",
        server_settings.server_hostname
    );

    let server_hostname = server_settings
        .server_hostname
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| {
            let port = portpicker::pick_unused_port().expect("Failed to find unused port");
            SocketAddr::from(([127, 0, 0, 1], port))
        });

    
    let mut config = {
        // Try to load terraphim_engineer_config.json first
        let engineer_config_path = std::path::Path::new("terraphim_server/default/terraphim_engineer_config.json");
        if engineer_config_path.exists() {
            log::info!("Loading Terraphim Engineer configuration from {:?}", engineer_config_path);
            match std::fs::read_to_string(engineer_config_path) {
                Ok(config_content) => {
                    match serde_json::from_str::<terraphim_config::Config>(&config_content) {
                        Ok(mut engineer_config) => {
                            log::info!("✅ Successfully loaded Terraphim Engineer configuration");
                            // Try to load saved config, but if it fails, use the engineer config as fallback
                            match engineer_config.load().await {
                                Ok(saved_config) => {
                                    log::info!("✅ Successfully loaded saved configuration from disk");
                                    saved_config
                                },
                                Err(e) => {
                                    log::info!("No saved config found, using Terraphim Engineer config: {:?}", e);
                                    engineer_config
                                }
                            }
                        },
                        Err(e) => {
                            log::warn!("Failed to parse Terraphim Engineer config: {:?}", e);
                            log::info!("Falling back to default server configuration");
                            match ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build() {
                                Ok(mut local_config) => match local_config.load().await {
                                    Ok(config) => {
                                        log::info!("Successfully loaded saved configuration from disk");
                                        config
                                    },
                                    Err(e) => {
                                        log::info!("Failed to load saved config, using default: {:?}", e);
                                        ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build().unwrap()
                                    }
                                },
                                Err(e) => panic!("Failed to build config: {e:?}"),
                            }
                        }
                    }
                },
                Err(e) => {
                    log::warn!("Failed to read Terraphim Engineer config file: {:?}", e);
                    log::info!("Falling back to default server configuration");
                    match ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build() {
                        Ok(mut local_config) => match local_config.load().await {
                            Ok(config) => {
                                log::info!("Successfully loaded saved configuration from disk");
                                config
                            },
                            Err(e) => {
                                log::info!("Failed to load saved config, using default: {:?}", e);
                                ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build().unwrap()
                            }
                        },
                        Err(e) => panic!("Failed to build config: {e:?}"),
                    }
                }
            }
        } else {
            log::info!("Terraphim Engineer config not found at {:?}", engineer_config_path);
            log::info!("Using default server configuration");
            match ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build() {
                Ok(mut local_config) => match local_config.load().await {
                    Ok(config) => {
                        log::info!("Successfully loaded saved configuration from disk");
                        config
                    },
                    Err(e) => {
                        log::info!("Failed to load saved config, using default: {:?}", e);
                        ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build().unwrap()
                    }
                },
                Err(e) => panic!("Failed to build config: {e:?}"),
            }
        }
    };

    let config_state = ConfigState::new(&mut config)
        .await
        .context("Failed to load config")?;

    // Log available roles for debugging
    log::info!("Server started with {} roles:", config.roles.len());
    for (role_name, role) in &config.roles {
        log::info!("  - Role: {} (relevance: {:?}, kg: {})", 
                   role_name, 
                   role.relevance_function,
                   role.kg.is_some());
    }

    // Example of adding a role for testing
    // let role = "system operator2".to_string();
    // let thesaurus = load_thesaurus(&AutomataPath::remote_example()).await?;
    // let rolegraph = RoleGraph::new(role.clone(), thesaurus).await?;
    // config_state
    //     .roles
    //     .insert(role, RoleGraphSync::from(rolegraph));
    // log::info!(
    //     "Config Roles: {:?}",
    //     config_state.roles.keys().collect::<Vec<&String>>()
    // );

    axum_server(server_hostname, config_state).await?;

    Ok(())
}
