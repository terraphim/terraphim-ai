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

    
        let mut config = match ConfigBuilder::new_with_id(ConfigId::Server).build_default_server().build() {
            Ok(mut local_config) => match local_config.load().await {
                Ok(config) => config,
                Err(e) => {
                    log::info!("Failed to load config: {:?}", e);
                    ConfigBuilder::new().build_default_server().build().unwrap()
                }
            },
        Err(e) => panic!("Failed to build config: {e:?}"),
    };
    let config_state = ConfigState::new(&mut config)
        .await
        .context("Failed to load config")?;

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
