//! Fast text matching and autocomplete engine for knowledge graphs.
//!
//! `terraphim_automata` provides high-performance text processing using Aho-Corasick
//! automata and finite state transducers (FST). It powers Terraphim's autocomplete
//! and knowledge graph linking features.
//!
//! # Features
//!
//! - **Fast Autocomplete**: Prefix-based search with fuzzy matching (Levenshtein/Jaro-Winkler)
//! - **Text Matching**: Find and replace terms using Aho-Corasick automata
//! - **Link Generation**: Convert matched terms to Markdown, HTML, or Wiki links
//! - **Paragraph Extraction**: Extract context around matched terms
//! - **WASM Support**: Browser-compatible autocomplete with TypeScript bindings
//! - **Remote Loading**: Async loading of thesaurus files from HTTP (feature-gated)
//!
//! # Architecture
//!
//! - **Autocomplete Index**: FST-based prefix search with metadata
//! - **Aho-Corasick Matcher**: Multi-pattern matching for link generation
//! - **Thesaurus Builder**: Parse knowledge graphs from JSON/Markdown
//!
//! # Cargo Features
//!
//! - `remote-loading`: Enable async HTTP loading of thesaurus files (requires tokio)
//! - `tokio-runtime`: Add tokio runtime support
//! - `typescript`: Generate TypeScript definitions via tsify
//! - `wasm`: Enable WebAssembly compilation
//!
//! # Examples
//!
//! ## Autocomplete with Fuzzy Matching
//!
//! ```rust
//! use terraphim_automata::{build_autocomplete_index, fuzzy_autocomplete_search};
//! use terraphim_types::{Thesaurus, NormalizedTermValue, NormalizedTerm};
//!
//! // Create a simple thesaurus
//! let mut thesaurus = Thesaurus::new("programming".to_string());
//! thesaurus.insert(
//!     NormalizedTermValue::from("rust"),
//!     NormalizedTerm::new(1, NormalizedTermValue::from("rust"))
//! );
//! thesaurus.insert(
//!     NormalizedTermValue::from("rust async"),
//!     NormalizedTerm::new(2, NormalizedTermValue::from("rust async"))
//! );
//!
//! // Build autocomplete index
//! let index = build_autocomplete_index(thesaurus, None).unwrap();
//!
//! // Fuzzy search (returns Result)
//! let results = fuzzy_autocomplete_search(&index, "rast", 0.8, Some(5)).unwrap();
//! assert!(!results.is_empty());
//! ```
//!
//! ## Text Matching and Link Generation
//!
//! ```rust
//! use terraphim_automata::{load_thesaurus_from_json, replace_matches, LinkType};
//!
//! let json = r#"{
//!   "name": "test",
//!   "data": {
//!     "rust": {
//!       "id": 1,
//!       "nterm": "rust programming",
//!       "url": "https://rust-lang.org"
//!     }
//!   }
//! }"#;
//!
//! let thesaurus = load_thesaurus_from_json(json).unwrap();
//! let text = "I love rust!";
//!
//! // Replace matches with Markdown links
//! let linked = replace_matches(text, thesaurus, LinkType::MarkdownLinks).unwrap();
//! let result = String::from_utf8(linked).unwrap();
//! println!("{}", result); // "I love [rust](https://rust-lang.org)!"
//! ```
//!
//! ## Loading Thesaurus Files
//!
//! ```no_run
//! use terraphim_automata::{AutomataPath, load_thesaurus};
//!
//! # #[cfg(feature = "remote-loading")]
//! # async fn example() {
//! // Load from local file
//! let local_path = AutomataPath::from_local("thesaurus.json");
//! let thesaurus = load_thesaurus(&local_path).await.unwrap();
//!
//! // Load from remote URL (requires 'remote-loading' feature)
//! let remote_path = AutomataPath::from_remote("https://example.com/thesaurus.json").unwrap();
//! let thesaurus = load_thesaurus(&remote_path).await.unwrap();
//! # }
//! ```
//!
//! # WASM Support
//!
//! Build for WebAssembly:
//!
//! ```bash
//! wasm-pack build --target web --features wasm
//! ```
//!
//! See the [WASM example](wasm-test/) for browser usage.

pub use self::builder::{Logseq, ThesaurusBuilder};
pub mod autocomplete;
pub mod builder;
pub mod matcher;
pub mod url_protector;

pub use autocomplete::{
    autocomplete_search, build_autocomplete_index, deserialize_autocomplete_index,
    fuzzy_autocomplete_search, fuzzy_autocomplete_search_levenshtein, serialize_autocomplete_index,
    AutocompleteConfig, AutocompleteIndex, AutocompleteMetadata, AutocompleteResult,
};
pub use matcher::{
    extract_paragraphs_from_automata, find_matches, replace_matches, LinkType, Matched,
};

// Re-export helpers for metadata iteration to support graph-embedding expansions in consumers
pub mod autocomplete_helpers {
    use super::autocomplete::{AutocompleteIndex, AutocompleteMetadata};
    pub fn iter_metadata(
        index: &AutocompleteIndex,
    ) -> impl Iterator<Item = (&str, &AutocompleteMetadata)> {
        index.metadata_iter()
    }
    pub fn get_metadata<'a>(
        index: &'a AutocompleteIndex,
        term: &str,
    ) -> Option<&'a AutocompleteMetadata> {
        index.metadata_get(term)
    }
}

#[cfg(feature = "remote-loading")]
pub use autocomplete::load_autocomplete_index;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs;
use std::path::PathBuf;
#[cfg(feature = "typescript")]
use tsify::Tsify;

use terraphim_types::Thesaurus;

/// Errors that can occur when working with automata and thesaurus operations.
#[derive(thiserror::Error, Debug)]
pub enum TerraphimAutomataError {
    /// Invalid thesaurus format or structure
    #[error("Invalid thesaurus: {0}")]
    InvalidThesaurus(String),

    /// JSON serialization/deserialization error
    #[error("Serde deserialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// Dictionary-related error
    #[error("Dict error: {0}")]
    Dict(String),

    /// File I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Aho-Corasick automata construction error
    #[error("Aho-Corasick build error: {0}")]
    AhoCorasick(#[from] aho_corasick::BuildError),

    /// Finite state transducer (FST) error
    #[error("FST error: {0}")]
    Fst(#[from] fst::Error),
}

/// Result type alias using `TerraphimAutomataError`.
pub type Result<T> = std::result::Result<T, TerraphimAutomataError>;

/// Path to a thesaurus/automata file, either local or remote.
///
/// Supports loading thesaurus files from local filesystem or HTTP URLs.
/// Remote loading requires the `remote-loading` feature to be enabled.
///
/// # Examples
///
/// ```
/// use terraphim_automata::AutomataPath;
///
/// // Local file path
/// let local = AutomataPath::from_local("thesaurus.json");
///
/// // Remote URL (requires 'remote-loading' feature)
/// let remote = AutomataPath::from_remote("https://example.com/thesaurus.json").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(Tsify))]
#[cfg_attr(feature = "typescript", tsify(into_wasm_abi, from_wasm_abi))]
pub enum AutomataPath {
    /// Local filesystem path
    Local(PathBuf),
    /// Remote HTTP/HTTPS URL
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
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let simple_path = if cwd.ends_with("terraphim_automata")
            || cwd.ends_with("terraphim_kg_orchestration")
            || cwd.ends_with("terraphim_task_decomposition")
            || cwd.ends_with("terraphim_kg_agents")
            || cwd.ends_with("terraphim_agent_registry")
        {
            "../../test-fixtures/term_to_id_simple.json"
        } else if cwd.file_name().is_some_and(|name| name == "terraphim-ai") {
            "test-fixtures/term_to_id_simple.json"
        } else {
            "data/term_to_id_simple.json" // fallback to old path
        };
        AutomataPath::from_local(simple_path)
    }
    /// Full Local example for testing
    pub fn local_example_full() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

        // Try multiple possible paths for the test fixtures
        let possible_paths = [
            "test-fixtures/term_to_id.json",       // from workspace root
            "../../test-fixtures/term_to_id.json", // from crate dir
            "../test-fixtures/term_to_id.json",    // from crates/ dir
            "data/term_to_id.json",                // legacy fallback
        ];

        let full_path = possible_paths
            .iter()
            .find(|path| cwd.join(path).exists())
            .unwrap_or(&"test-fixtures/term_to_id.json");

        AutomataPath::from_local(full_path)
    }

    /// Create a sample remote AutomataPath for testing
    pub fn remote_example() -> Self {
        AutomataPath::from_remote("https://staging-storage.terraphim.io/thesaurus_Default.json")
            .unwrap()
    }
}

/// Load thesaurus from JSON string (sync version for WASM compatibility)
pub fn load_thesaurus_from_json(json_str: &str) -> Result<Thesaurus> {
    let thesaurus: Thesaurus = serde_json::from_str(json_str)?;
    Ok(thesaurus)
}

/// Load thesaurus from JSON string and replace terms using streaming matcher
pub fn load_thesaurus_from_json_and_replace(
    json_str: &str,
    content: &str,
    link_type: LinkType,
) -> Result<Vec<u8>> {
    let thesaurus = load_thesaurus_from_json(json_str)?;
    let replaced = replace_matches(content, thesaurus, link_type)?;
    Ok(replaced)
}

/// Load thesaurus from JSON string (async version for compatibility)
#[cfg(feature = "remote-loading")]
pub async fn load_thesaurus_from_json_async(json_str: &str) -> Result<Thesaurus> {
    load_thesaurus_from_json(json_str)
}

/// Load thesaurus from JSON string and replace terms using streaming matcher (async version)
#[cfg(feature = "remote-loading")]
pub async fn load_thesaurus_from_json_and_replace_async(
    json_str: &str,
    content: &str,
    link_type: LinkType,
) -> Result<Vec<u8>> {
    load_thesaurus_from_json_and_replace(json_str, content, link_type)
}

/// Load a thesaurus from a file or URL
///
/// Note: Remote loading requires the "remote-loading" feature to be enabled.
#[cfg(feature = "remote-loading")]
pub async fn load_thesaurus(automata_path: &AutomataPath) -> Result<Thesaurus> {
    async fn read_url(url: String) -> Result<String> {
        log::debug!("Reading thesaurus from remote: {url}");
        let response = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("Terraphim-Automata/1.0")
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
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

    let contents = match automata_path {
        AutomataPath::Local(path) => {
            // Check if file exists before attempting to read
            if !std::path::Path::new(path).exists() {
                return Err(TerraphimAutomataError::InvalidThesaurus(format!(
                    "Thesaurus file not found: {}",
                    path.display()
                )));
            }
            fs::read_to_string(path)?
        }
        AutomataPath::Remote(_) => {
            return Err(TerraphimAutomataError::InvalidThesaurus(
                "Remote loading is not supported. Enable the 'remote-loading' feature.".to_string(),
            ));
        }
    };

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

/// Load a thesaurus from a local file only (WASM-compatible version)
///
/// This version only supports local file loading and doesn't require async runtime.
#[cfg(not(feature = "remote-loading"))]
pub fn load_thesaurus(automata_path: &AutomataPath) -> Result<Thesaurus> {
    let contents = match automata_path {
        AutomataPath::Local(path) => fs::read_to_string(path)?,
        AutomataPath::Remote(_) => {
            return Err(TerraphimAutomataError::InvalidThesaurus(
                "Remote loading is not supported. Enable the 'remote-loading' feature.".to_string(),
            ));
        }
    };

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

#[cfg(test)]
mod tests {
    use terraphim_types::NormalizedTermValue;

    use super::*;

    #[cfg(feature = "remote-loading")]
    #[tokio::test]
    async fn test_load_thesaurus_from_file() {
        let automata_path = AutomataPath::local_example();
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

    #[cfg(feature = "remote-loading")]
    #[tokio::test]
    #[ignore]
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

    #[cfg(not(feature = "remote-loading"))]
    #[test]
    fn test_load_thesaurus_from_file_sync() {
        let automata_path = AutomataPath::local_example();
        let thesaurus = load_thesaurus(&automata_path).unwrap();
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

    #[cfg(feature = "remote-loading")]
    #[tokio::test]
    async fn test_load_thesaurus_from_file_async() {
        let automata_path = AutomataPath::local_example();
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

    #[test]
    fn test_load_thesaurus_from_json() {
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

        let thesaurus = load_thesaurus_from_json(json_str).unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from(
                    "project management framework tailoring"
                ))
                .unwrap()
                .id,
            1_u64
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from("strategy documents"))
                .unwrap()
                .id,
            2_u64
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from("project constraints"))
                .unwrap()
                .id,
            3_u64
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from(
                    "project management framework tailoring"
                ))
                .unwrap()
                .url,
            Some("https://example.com/project-tailoring-strategy".to_string())
        );
        assert_eq!(
            thesaurus
                .get(&NormalizedTermValue::from("strategy documents"))
                .unwrap()
                .url,
            Some("https://example.com/strategy-documents".to_string())
        );
    }

    #[test]
    fn test_load_thesaurus_from_json_and_replace() {
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
        let replaced =
            load_thesaurus_from_json_and_replace(json_str, content, LinkType::MarkdownLinks)
                .unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(replaced_str, "I like [project constraints](https://example.com/project-constraints) and [strategy documents](https://example.com/strategy-documents).");

        // Test HTMLLinks
        let replaced =
            load_thesaurus_from_json_and_replace(json_str, content, LinkType::HTMLLinks).unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(replaced_str, "I like <a href=\"https://example.com/project-constraints\">project constraints</a> and <a href=\"https://example.com/strategy-documents\">strategy documents</a>.");

        // Test WikiLinks
        let replaced =
            load_thesaurus_from_json_and_replace(json_str, content, LinkType::WikiLinks).unwrap();
        let replaced_str = String::from_utf8(replaced).unwrap();
        assert_eq!(
            replaced_str,
            "I like [[project constraints]] and [[strategy documents]]."
        );
    }

    #[test]
    fn test_load_thesaurus_from_json_invalid() {
        let invalid_json = "{invalid_json}";
        let result = load_thesaurus_from_json(invalid_json);
        assert!(result.is_err());
    }
}
