// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use tauri::command;
use tauri::State;

use terraphim_config::{Config, ConfigState};
use terraphim_middleware::search_haystacks;
use terraphim_middleware::thesaurus::create_thesaurus_from_haystack;
use terraphim_types::{merge_and_serialize, Article, IndexedDocument, SearchQuery};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RequestBody {
    id: i32,
    name: String,
}

// Everything we return from commands must implement `Serialize`.
// This includes Errors and `anyhow`'s `Error` type doesn't implement it.
// See https://github.com/tauri-apps/tauri/discussions/3913
#[derive(Debug, thiserror::Error)]
pub enum TerraphimTauriError {
    #[error("An error occurred: {0}")]
    MiddlewareError(#[from] terraphim_middleware::Error),
}

// Manually implement `Serialize` for our error type because some of the
// lower-level types don't implement it.
impl Serialize for TerraphimTauriError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}

pub type Result<T> = anyhow::Result<T, TerraphimTauriError>;

/// Search All TerraphimGraphs defined in a config by query param
#[command]
pub async fn search(
    config_state: State<'_, ConfigState>,
    search_query: SearchQuery,
) -> Result<Vec<Article>> {
    println!("Search called with {:?}", search_query);
    let current_config_state = config_state.inner().clone();

    // Build thesaurus and update knowledge graph automata_url
    log::debug!("Creating thesaurus from haystack");
    create_thesaurus_from_haystack(config_state.inner().clone(), search_query.clone()).await?;
    log::debug!("Thesaurus created");

    let cached_articles = search_haystacks(current_config_state, search_query.clone()).await?;
    let docs: Vec<IndexedDocument> = config_state.search_articles(search_query).await;
    let articles = merge_and_serialize(cached_articles, docs);

    Ok(articles)
}

#[command]
pub async fn get_config(
    config_state: tauri::State<'_, ConfigState>,
) -> Result<terraphim_config::Config> {
    println!("Get config called");
    let current_config = config_state.config.lock().await;
    println!("Get config called with {:?}", current_config);
    Ok(current_config.clone())
}

use persistence::Persistable;

#[command]
pub async fn update_config(
    config_state: tauri::State<'_, ConfigState>,
    config_new: Config,
) -> Result<terraphim_config::Config> {
    println!("Update config called with {:?}", config_new);
    let mut config_state = config_state.config.lock().await;
    config_state.update(config_new.clone());
    // FIXME: add tauri error TerraphimTauriError and use context
    let _ = config_state.save().await;
    Ok(config_state.clone())
}
// pub struct Port(u16);

// /// A command to get the unused port instead of 3000.
// #[tauri::command]
// pub fn _get_port(port: tauri::State<Port>) -> Result<String> {
//     Ok(format!("{}", port.0))
// }

// use std::net::SocketAddr;
// use terraphim_server::axum_server;

// /// Start the server, currently not in use, but will be important for OAuth etc in the future.
// #[tauri::command]
// async fn _start_server() -> Result<()> {
//     let port = portpicker::pick_unused_port().expect("failed to find unused port");
//     let addr = SocketAddr::from(([127, 0, 0, 1], port));
//     let mut config = Config::new();
//     let config_state = ConfigState::new(&mut config).await.unwrap();
//     tauri::async_runtime::spawn(async move {
//         if let Err(e) = axum_server(addr, config_state).await {
//             println!("Failed to start axum server: {e:?}");
//         }
//     });
//     Ok(())
// }
