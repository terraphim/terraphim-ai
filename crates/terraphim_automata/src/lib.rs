pub mod matcher;

pub use matcher::{find_matches, Matched, replace_matches, LinkType};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;

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
        log::debug!("Current folder {:?}", std::env::current_dir());
        AutomataPath::from_local("data/term_to_id_simple.json")
    }
    /// Full Local example for testing
    pub fn local_example_full() -> Self {
        AutomataPath::from_local("data/term_to_id.json")
    }

    /// Create a sample remote AutomataPath for testing
    pub fn remote_example() -> Self {
        AutomataPath::from_remote("https://staging-storage.terraphim.io/thesaurus_Default.json")
            .unwrap()
    }
}

pub async fn load_thesaurus_from_json(json_str: &str) -> Result<Thesaurus> {
    let thesaurus: Thesaurus = serde_json::from_str(json_str)?;
    Ok(thesaurus)
}


/// load thesaurus from JSON string and replace terms using streaming matcher
pub async fn load_thesaurus_from_json_and_replace(json_str: &str, content: &str, link_type: LinkType) -> Result<Vec<u8>> {
    let thesaurus = load_thesaurus_from_json(json_str).await?;
    let replaced = replace_matches(content, thesaurus, link_type)?;
    Ok(replaced)    
}

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
    use terraphim_types::NormalizedTermValue;

    use super::*;

    #[tokio::test]
    async fn test_load_thesaurus_from_file() {
        let automata_path = AutomataPath::from_local("data/term_to_id_simple.json");
        let thesaurus = load_thesaurus(&automata_path).await.unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("foo")).unwrap().id,
            1_u64
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("bar")).unwrap().id,
            2_u64
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("baz")).unwrap().id,
            1_u64
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
            661_u64
        );
    }

    #[tokio::test]
    async fn test_load_thesaurus_from_json() {
        let json_str = r#"
{
  "name": "Engineering",
  "data": {
    "project management framework tailoring": {
      "id": 1,
      "nterm": "project tailoring strategy",
      "url": "https://example.com/project-tailoring-strategy"
    },
    "strategy documents": {
      "id": 2,
      "nterm": "strategy documents",
      "url": "https://example.com/strategy-documents"
    },
    "project constraints": {
      "id": 3,
      "nterm": "project constraints",
      "url": "https://example.com/project-constraints"
    }
  }
}"#;

        let thesaurus = load_thesaurus_from_json(json_str).await.unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("project management framework tailoring")).unwrap().id,
            1_u64
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("strategy documents")).unwrap().id,
            2_u64
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("project constraints")).unwrap().id,
            3_u64
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("project management framework tailoring")).unwrap().url,
            Some("https://example.com/project-tailoring-strategy".to_string())
        );
        assert_eq!(
            thesaurus.get(&NormalizedTermValue::from("strategy documents")).unwrap().url,
            Some("https://example.com/strategy-documents".to_string())
        );
        
    }

    #[tokio::test]
    async fn test_load_thesaurus_from_json_and_replace() {
        let json_str = r#"
{
  "name": "Engineering",
  "data": {
    "project management framework tailoring": {
      "id": 1,
      "nterm": "project tailoring strategy",
      "url": "https://example.com/project-tailoring-strategy"
    },
    "strategy documents": {
      "id": 2,
      "nterm": "strategy documents",
      "url": "https://example.com/strategy-documents"
    },
    "project constraints": {
      "id": 3,
      "nterm": "project constraints",
      "url": "https://example.com/project-constraints"
    }
  }
}"#;

        let content = "I like project constraints and strategy documents.";
        let replaced = load_thesaurus_from_json_and_replace(json_str, content, LinkType::MarkdownLinks).await.unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(replaced_str, "I like [project constraints](https://example.com/project-constraints) and [strategy documents](https://example.com/strategy-documents).");

        // Test HTMLLinks
        let replaced = load_thesaurus_from_json_and_replace(json_str, content, LinkType::HTMLLinks).await.unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(replaced_str, "I like <a href=\"https://example.com/project-constraints\">project constraints</a> and <a href=\"https://example.com/strategy-documents\">strategy documents</a>.");

        // Test WikiLinks
        let replaced = load_thesaurus_from_json_and_replace(json_str, content, LinkType::WikiLinks).await.unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(replaced_str, "I like [[project constraints]] and [[strategy documents]].");
    }

    #[tokio::test]
    async fn test_load_thesaurus_from_json_invalid() {
        let invalid_json = "{invalid_json}";
        let result = load_thesaurus_from_json(invalid_json).await;
        assert!(result.is_err());
    }
}
