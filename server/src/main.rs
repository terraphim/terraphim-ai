//! terraphim API server
#![warn(clippy::all, clippy::pedantic)]
#![warn(
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

use poem::{listener::TcpListener, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
mod settings;
use settings::Settings;
use terraphim_pipeline::{Document, RoleGraph};
use tokio::sync::Mutex;
use anyhow::{Context, Result};

mod api;
use api::Api;
mod types;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let settings = Settings::new().unwrap();
    println!("{:?}", settings);
    let bind_addr = settings.server_url.clone();
    let api_endpoint = settings.api_endpoint.clone();

    let role = "system operator".to_string();
    // let automata_url = "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json";
    let automata_url = "./data/term_to_id.json";
    let rolegraph = RoleGraph::new(role, automata_url).context(format!("Failed to create rolegraph from {automata_url}"))?;

    let api_service = OpenApiService::new(Api { rolegraph: Mutex::new(rolegraph) }, "Hello World", "1.0").server(api_endpoint);
    let ui = api_service.swagger_ui();
    let spec = api_service.spec();
    let route = Route::new()
        .nest("/api", api_service)
        .nest("/doc", ui)
        .at("/spec", poem::endpoint::make_sync(move |_| spec.clone()))
        // .with(Cors::new())
        .data(settings);

    Server::new(TcpListener::bind(bind_addr)).run(route).await?;

    Ok(())
}
