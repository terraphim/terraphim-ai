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

use ahash::AHashMap;
use anyhow::Context;
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use terraphim_automata::AutomataPath;
use terraphim_config::ConfigBuilder;
use terraphim_config::Haystack;
use terraphim_config::KnowledgeGraph;
use terraphim_config::Role;
use terraphim_config::ServiceType;
use terraphim_types::KnowledgeGraphInputType;
use terraphim_types::RelevanceFunction;
use url::Url;

use terraphim_config::ConfigState;
use terraphim_server::{axum_server, Result};
use terraphim_settings::DeviceSettings;

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
    match run_server().await {
        Ok(_) => Ok(()),
        Err(e) => {
            log::error!("Error: {e:#?}");
            std::process::exit(1)
        }
    }
}

async fn run_server() -> Result<()> {
    // Set up logger for the server
    env_logger::init();

    let args = Args::parse();
    log::info!("Commandline arguments: {args:?}");
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

    let automata_path = AutomataPath::from_local("data/term_to_id.json");

    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let system_operator_haystack = cwd.join("fixtures/haystack/");

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+X")
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
                kg: KnowledgeGraph {
                    automata_path: automata_path.clone(),
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("~/pkm"),
                    public: true,
                    publish: true,
                },
                haystacks: vec![Haystack {
                    path: PathBuf::from("localsearch"),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "Engineer",
            Role {
                shortname: Some("Engineer".to_string()),
                name: "Engineer".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "lumen".to_string(),
                server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
                kg: KnowledgeGraph {
                    automata_path: automata_path.clone(),
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("~/pkm"),
                    public: true,
                    publish: true,
                },
                haystacks: vec![Haystack {
                    path: PathBuf::from("localsearch"),
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .add_role(
            "System Operator",
            Role {
                shortname: Some("operator".to_string()),
                name: "System Operator".to_string(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "superhero".to_string(),
                server_url: Url::parse("http://localhost:8000/articles/search").unwrap(),
                kg: KnowledgeGraph {
                    automata_path,
                    input_type: KnowledgeGraphInputType::Markdown,
                    path: PathBuf::from("~/pkm"),
                    public: true,
                    publish: true,
                },
                haystacks: vec![Haystack {
                    path: system_operator_haystack,
                    service: ServiceType::Ripgrep,
                }],
                extra: AHashMap::new(),
            },
        )
        .build()
        .unwrap();
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
