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
#![deny(anonymous_parameters, macro_use_extern_crate, pointer_structural_match)]
#![deny(missing_docs)]

use anyhow::Context;
use clap::Parser;
use std::net::SocketAddr;
use terraphim_automata::load_thesaurus;
use terraphim_config::{Config, ConfigState};
use terraphim_rolegraph::RoleGraphSync;
use terraphim_server::{axum_server, Result};
use terraphim_settings::Settings;

/// TODO: Can't get Open API docs to work with axum consistently, given up for now.
use terraphim_rolegraph::RoleGraph;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// String to search for
    #[arg(short, long)]
    search_term: Option<String>,

    /// Role to use for search
    #[arg(short, long)]
    role: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logger for the server
    env_logger::init();

    let args = Args::parse();
    log::info!("Commandline arguments: {args:?}");
    let server_settings =
        Settings::load_from_env_and_file(None).context("Failed to load settings")?;
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

    // TODO: make the service type configurable
    // For now, we only support passing in the service type as an argument
    let mut config = Config::new();
    let mut config_state = ConfigState::new(&mut config)
        .await
        .context("Failed to load config")?;

    // Example of adding a role for testing
    let role = "system operator2".to_string();
    let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let thesaurus = load_thesaurus(automata_url).await?;
    let rolegraph = RoleGraph::new(role.clone(), thesaurus).await?;
    config_state
        .roles
        .insert(role, RoleGraphSync::from(rolegraph));
    log::info!(
        "Config Roles: {:?}",
        config_state.roles.keys().collect::<Vec<&String>>()
    );

    axum_server(server_hostname, config_state).await?;

    Ok(())
}
