use std::path::PathBuf;
use std::sync::Arc;
use salvo::prelude::*;
use salvo::conn::tcp::TcpAcceptor;
use tokio::net::TcpListener;
use terraphim_config::{ConfigBuilder, Role, ServiceType, Haystack};
use terraphim_types::RelevanceFunction;
use crate::service::TerraphimResourceService;
use crate::handlers::resources::ResourceHandlers;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod schema;
mod service;
mod handlers;
mod docs;

#[tokio::main]
async fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    info!("Starting Terraphim MCP server...");

    // Create configuration
    let mut config = ConfigBuilder::new()
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: None,
                haystacks: vec![Haystack {
                    path: PathBuf::from("tests/test_data/sample_docs"),
                    service: ServiceType::Ripgrep,
                }],
                extra: Default::default(),
            },
        )
        .default_role("Default")
        .unwrap()
        .build()
        .unwrap();

    // Create service
    let service = TerraphimResourceService::new(config).await
        .expect("Failed to create service");

    // Create handlers
    let handlers = ResourceHandlers::new(Arc::new(service));
    let router = handlers.router();

    // Create server
    let addr = "127.0.0.1:5800";
    info!("Server running on http://{}", addr);
    let listener = TcpListener::bind(addr).await.unwrap();
    let acceptor = TcpAcceptor::try_from(listener).unwrap();
    Server::new(acceptor).serve(router).await;
}
