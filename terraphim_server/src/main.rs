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
use clap::Parser;
use std::net::SocketAddr;
use terraphim_config::ConfigState;
use terraphim_config::{ConfigBuilder, ConfigId};
use terraphim_persistence::Persistable;
use terraphim_server::{axum_server, Result};
use terraphim_settings::DeviceSettings;
// TODO: Re-enable auto-update when terraphim_update is added to workspace
// use terraphim_update::{check_for_updates, update_binary};

/// Terraphim AI server with role-based deployment support
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Role configuration to use (Default, `RustEngineer`, `TerraphimEngineer`, Combined)
    #[arg(long, default_value = "TerraphimEngineer")]
    role: String,

    /// Custom config file path (overrides role selection)
    #[arg(long)]
    config: Option<String>,

    /// Check for updates without installing
    #[arg(long)]
    check_update: bool,

    /// Update to latest version if available
    #[arg(long)]
    update: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Handle update operations before starting server
    if args.check_update || args.update {
        return handle_update_commands(&args).await;
    }

    match run_server(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            log::error!("Error: {e:#?}");
            std::process::exit(1)
        }
    }
}

/// Handle update-related commands
#[allow(clippy::unused_async)]
async fn handle_update_commands(_args: &Args) -> Result<()> {
    // TODO: Re-enable when terraphim_update is added to workspace
    eprintln!("Auto-update feature temporarily disabled");
    std::process::exit(1);
}

#[allow(clippy::too_many_lines)]
async fn run_server(args: Args) -> Result<()> {
    // Set up logger for the server
    terraphim_service::logging::init_logging(terraphim_service::logging::detect_logging_config());

    let server_settings =
        DeviceSettings::load_from_env_and_file(None).context("Failed to load settings")?;
    log::info!(
        "Device settings hostname: {:?}",
        server_settings.server_hostname
    );

    // Pre-create directories for storage backends early in startup to prevent persistence errors
    log::info!("🔧 Pre-creating storage directories...");
    log::info!(
        "Found {} profiles: {:?}",
        server_settings.profiles.len(),
        server_settings.profiles.keys().collect::<Vec<_>>()
    );
    for (profile_name, profile) in &server_settings.profiles {
        let unknown = "unknown".to_string();
        let profile_type = profile.get("type").unwrap_or(&unknown);
        log::info!("Processing profile '{profile_name}' of type '{profile_type}'");
        match profile_type.as_str() {
            "sqlite" => {
                // Handle both datadir and connection_string formats
                if let Some(datadir) = profile.get("datadir") {
                    if !datadir.is_empty() {
                        log::info!("🔧 Creating SQLite directory: {datadir}");
                        if let Err(e) = std::fs::create_dir_all(datadir) {
                            log::warn!("Failed to create SQLite directory '{datadir}': {e}");
                        } else {
                            log::info!("✅ Created SQLite directory: {datadir}");
                        }
                    }
                } else if let Some(connection_string) = profile.get("connection_string") {
                    // Extract directory from connection_string path
                    if let Some(parent_dir) = std::path::Path::new(connection_string).parent() {
                        let dir_str = parent_dir.to_string_lossy();
                        if !dir_str.is_empty() {
                            log::info!("🔧 Creating SQLite directory: {dir_str}");
                            if let Err(e) = std::fs::create_dir_all(parent_dir) {
                                log::warn!("Failed to create SQLite directory '{dir_str}': {e}");
                            } else {
                                log::info!("✅ Created SQLite directory: {dir_str}");
                            }
                        }
                    }
                }
            }
            "redb" => {
                // ReDB datadir is the database file path - parent directory creation is handled by OpenDAL
                log::info!("ReDB database will be created by OpenDAL");
            }
            "dashmap" => {
                if let Some(root) = profile.get("root") {
                    if !root.is_empty() {
                        log::info!("🔧 Creating DashMap directory: {root}");
                        if let Err(e) = std::fs::create_dir_all(root) {
                            log::warn!("Failed to create DashMap directory '{root}': {e}");
                        } else {
                            log::info!("✅ Created DashMap directory: {root}");
                        }
                    }
                }
            }
            _ => {}
        }
    }
    log::info!("✅ Storage directory pre-creation completed");

    let server_hostname = server_settings
        .server_hostname
        .parse::<SocketAddr>()
        .unwrap_or_else(|_| {
            let port = portpicker::pick_unused_port().expect("Failed to find unused port");
            SocketAddr::from(([127, 0, 0, 1], port))
        });

    let mut config = {
        // Use custom config file if provided, otherwise determine from role
        let config_path = if let Some(custom_config) = args.config {
            std::path::PathBuf::from(custom_config)
        } else {
            // Determine config file based on role
            let config_filename = match args.role.to_lowercase().as_str() {
                "default" => "default_role_config.json",
                "rustengineer" => "rust_engineer_config.json",
                "terraphimengineer" => "terraphim_engineer_config.json",
                "combined" => "combined_roles_config.json",
                _ => {
                    log::warn!(
                        "Unknown role '{}', using terraphim_engineer_config.json",
                        args.role
                    );
                    "terraphim_engineer_config.json"
                }
            };
            std::path::Path::new("terraphim_server/default").join(config_filename)
        };

        log::info!(
            "Loading configuration from {} (role: {})",
            config_path.display(),
            args.role
        );

        if config_path.exists() {
            log::info!(
                "Loading {} role configuration from {}",
                args.role,
                config_path.display()
            );
            match std::fs::read_to_string(&config_path) {
                Ok(config_content) => {
                    match serde_json::from_str::<terraphim_config::Config>(&config_content) {
                        Ok(mut role_config) => {
                            log::info!("✅ Successfully loaded {} role configuration", args.role);
                            // Try to load saved config, but if it fails, use the role config as fallback
                            match role_config.load().await {
                                Ok(saved_config) => {
                                    log::info!(
                                        "✅ Successfully loaded saved configuration from disk"
                                    );
                                    saved_config
                                }
                                Err(e) => {
                                    log::info!(
                                        "No saved config found, using {} role config: {:?}",
                                        args.role,
                                        e
                                    );
                                    role_config
                                }
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse {} role config: {:?}", args.role, e);
                            log::info!("Falling back to default server configuration");
                            match ConfigBuilder::new_with_id(ConfigId::Server)
                                .build_default_server()
                                .build()
                            {
                                Ok(mut local_config) => match local_config.load().await {
                                    Ok(config) => {
                                        log::info!(
                                            "Successfully loaded saved configuration from disk"
                                        );
                                        config
                                    }
                                    Err(e) => {
                                        log::info!(
                                            "Failed to load saved config, using default: {e:?}"
                                        );
                                        ConfigBuilder::new_with_id(ConfigId::Server)
                                            .build_default_server()
                                            .build()
                                            .unwrap()
                                    }
                                },
                                Err(e) => panic!("Failed to build config: {e:?}"),
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to read {} role config file: {:?}", args.role, e);
                    log::info!("Falling back to default server configuration");
                    match ConfigBuilder::new_with_id(ConfigId::Server)
                        .build_default_server()
                        .build()
                    {
                        Ok(mut local_config) => match local_config.load().await {
                            Ok(config) => {
                                log::info!("Successfully loaded saved configuration from disk");
                                config
                            }
                            Err(e) => {
                                log::info!("Failed to load saved config, using default: {e:?}");
                                ConfigBuilder::new_with_id(ConfigId::Server)
                                    .build_default_server()
                                    .build()
                                    .unwrap()
                            }
                        },
                        Err(e) => panic!("Failed to build config: {e:?}"),
                    }
                }
            }
        } else {
            log::info!(
                "{} role config not found at {}",
                args.role,
                config_path.display()
            );
            log::info!("Using default server configuration");
            match ConfigBuilder::new_with_id(ConfigId::Server)
                .build_default_server()
                .build()
            {
                Ok(mut local_config) => match local_config.load().await {
                    Ok(config) => {
                        log::info!("Successfully loaded saved configuration from disk");
                        config
                    }
                    Err(e) => {
                        log::info!("Failed to load saved config, using default: {e:?}");
                        ConfigBuilder::new_with_id(ConfigId::Server)
                            .build_default_server()
                            .build()
                            .unwrap()
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
        log::info!(
            "  - Role: {} (relevance: {:?}, kg: {})",
            role_name,
            role.relevance_function,
            role.kg.is_some()
        );
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
