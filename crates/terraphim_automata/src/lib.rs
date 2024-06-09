pub mod matcher;

pub use matcher::{find_matches, Matched};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
// use reqwest::Url;

use terraphim_types::Thesaurus;

#[derive(thiserror::Error, Debug)]
pub enum TerraphimAutomataError {
    #[error("Invalid thesaurus: {0}")]
    InvalidThesaurus(String),

    #[error("Serde deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Dict error: {0}")]
    Dict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Aho-Corasick build error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),
}

pub type Result<T> = std::result::Result<T, TerraphimAutomataError>;

/// AutomataPath is a path to the automata file
///
/// It can either be a local file path or a URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutomataPath {
    Local(PathBuf),
    Remote(String),
}

impl Display for AutomataPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutomataPath::Local(path) => write!(f, "Local Path: {:?}", path),
            AutomataPath::Remote(url) => write!(f, "Remote URL: {:?}", url),
        }
    }
}

impl AutomataPath {
    /// Create a new AutomataPath from a URL
    pub fn from_remote(url: &str) -> Result<Self> {
        // let url = reqwest::Url::parse(url)?;
        println!("{url}");
        if !url.starts_with("http") || !url.starts_with("https") {
            return Err(TerraphimAutomataError::Dict(format!(
                "Invalid URL scheme. Only `http` and `https` are supported right now. Got {}",
                url
            )));
        }

        Ok(AutomataPath::Remote(url.to_string()))
    }

    /// Create a new AutomataPath from a file
    pub fn from_local<P: AsRef<std::path::Path>>(file: P) -> Self {
        AutomataPath::Local(file.as_ref().to_path_buf())
    }

    /// Local example for testing
    pub fn local_example() -> Self {
        AutomataPath::from_local("data/term_to_id_simple.json")
    }

    /// Create a sample remote AutomataPath for testing
    pub fn remote_example() -> Self {
        AutomataPath::from_remote(
            "https://system-operator.s3.eu-west-2.amazonaws.com/term_to_id.json",
        )
        .unwrap()
    }
}

/// `load_automata` loads output of the knowledge graph builder
// pub async fn load_automata(url_or_file: &str) -> Result<matcher::Automata> {
//     let thesaurus = load_thesaurus(url_or_file).await?;
//     let automata = Automata::new(thesaurus);
//     Ok(automata)
// }

/// Load a thesaurus from a file or URL
pub async fn load_thesaurus(automata_path: &AutomataPath) -> Result<Thesaurus> {
    async fn read_url(url: String) -> Result<String> {
        log::debug!("Reading thesaurus from remote: {url}");
        let response = reqwest::Client::new()
            .get(url.clone())
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| {
                TerraphimAutomataError::InvalidThesaurus(format!(
                    "Failed to fetch thesaurus from remote {url}. Error: {e:#?}",
                ))
            })?;

        let status = response.status();
        let headers = response.headers().clone(); // Clone headers for error reporting
        let body = response.text().await;

        match body {
            Ok(text) => Ok(text),
            Err(e) => {
                let error_info = format!(
                    "Failed to read thesaurus from remote {url}. Status: {status}. Headers: {headers:#?}. Error: {e:#?}",
                );
                Err(TerraphimAutomataError::InvalidThesaurus(error_info))
            }
        }
    }

    log::debug!("Reading thesaurus from {automata_path}");
    let contents = match automata_path {
        AutomataPath::Local(path) => fs::read_to_string(path)?,
        AutomataPath::Remote(url) => read_url(url.clone()).await?,
    };

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

#[cfg(test)]
mod tests {
    use terraphim_types::{Id, NormalizedTermValue};

    use super::*;

    #[tokio::test]
    async fn test_load_thesaurus_from_file() {
        let automata_path = AutomataPath::from_local("data/term_to_id_simple.json");
        let thesaurus = load_thesaurus(&automata_path).await.unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("foo")).unwrap().id,
            Id::from(1)
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("bar")).unwrap().id,
            Id::from(2)
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("baz")).unwrap().id,
            Id::from(1)
        );
    }

    #[tokio::test]
    async fn test_load_thesaurus_from_url() {
        let automata_path = AutomataPath::remote_example();
        let thesaurus = load_thesaurus(&automata_path).await.unwrap();
        assert_eq!(thesaurus.len(), 1725);
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from("@risk a user guide"))
                .unwrap()
                .id,
            Id::from(661)
        );
    }
}
