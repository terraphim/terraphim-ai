use serde_json as json;
use terraphim_config::TerraphimConfigError;

mod command;
pub mod thesaurus;
pub mod indexer;

pub use indexer::search_haystacks;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Serde deserialization error: {0}")]
    Json(#[from] json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Indexation error: {0}")]
    Indexation(String),

    #[error("Config error: {0}")]
    Config(#[from] TerraphimConfigError),
}

pub type Result<T> = std::result::Result<T, Error>;
