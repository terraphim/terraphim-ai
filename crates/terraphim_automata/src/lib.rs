pub mod matcher;

pub use matcher::{find_matches, replace_matches, Matched};
use std::io::prelude::*;
use std::path::Path;
use std::{fs::File, path::PathBuf};

use terraphim_types::Thesaurus;

#[derive(thiserror::Error, Debug)]
pub enum TerraphimAutomataError {
    #[error("Reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

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

/// A KnowledgeGraphBuilder receives a path containing
/// resources (e.g. files) with key-value pairs and returns a `Thesaurus`
/// (a dictionary with synonyms which map to higher-level concepts)
trait KnowledgeGraphBuilder {
    /// - `kg_path`: The path to the knowledge graph input (e.g. a directory of Markdown files)
    // TODO: This could be generalized to take a `Read` trait object
    // or a `Resource` or take a glob of inputs?
    async fn build(&self, kg_path: PathBuf) -> Result<Thesaurus>;
}

/// A builder for a knowledge graph, which can handle Markdown inputs.
struct MarkdownKnowledgeGraphBuilder {}

impl MarkdownKnowledgeGraphBuilder {
    /// Create a new knowledge graph builder from a data source.
    pub fn new() -> Self {
        Self {}
    }
}

impl KnowledgeGraphBuilder for MarkdownKnowledgeGraphBuilder {
    /// Build the knowledge graph from the data source.
    ///
    /// This uses a service for parsing the data source and returns a
    /// `Thesaurus` which is the knowledge graph
    async fn build(&self, kg_path: PathBuf) -> Result<Thesaurus> {
        todo!();
        // Initialize a logseq service for parsing the data source
        // let logseq_service = terraphim_middleware::LogseqService::default();
        // let mut thesaurus = Thesaurus::new();
        // let mut parser = terraphim_markdown::Parser::new();
        // let entries = parser.parse_directory(&self.kg_path).await?;
        // for entry in entries {
        //     thesaurus.insert(entry);
        // }
        // Ok(thesaurus)
    }
}

/// Load a thesaurus from a file or URL
///
/// This loads the output of the knowledge graph builder
pub async fn load_thesaurus(url_or_file: &str) -> Result<Thesaurus> {
    async fn read_url(url: &str) -> Result<String> {
        let response = reqwest::Client::new()
            .get(url)
            .header("Accept", "application/json")
            .send()
            .await?;

        let text = response.text().await?;

        Ok(text)
    }
    let contents = if url_or_file.starts_with("http") {
        read_url(url_or_file).await?
    } else {
        let mut file = File::open(Path::new(url_or_file))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        contents
    };

    let thesaurus = serde_json::from_str(&contents)?;
    Ok(thesaurus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_automata_from_file() {
        let thesaurus = load_thesaurus("tests/test_data.csv.gz").await.unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(thesaurus.get("foo").unwrap().id, 1);
        assert_eq!(thesaurus.get("bar").unwrap().id, 2);
        assert_eq!(thesaurus.get("baz").unwrap().id, 3);

        // TODO: No `parent` field in type `NormalizedTerm`
        // assert_eq!(thesaurus.get("foo").unwrap().parent, None);
        // assert_eq!(thesaurus.get("bar").unwrap().parent, Some("1".to_string()));
        // assert_eq!(thesaurus.get("baz").unwrap().parent, Some("2".to_string()));
    }

    #[tokio::test]
    async fn test_load_automata_from_url() {
        let thesaurus = load_thesaurus(
            "https://raw.githubusercontent.com/github/copilot-sample-code/main/test_data.csv.gz",
        )
        .await
        .unwrap();
        assert_eq!(thesaurus.len(), 3);
        assert_eq!(thesaurus.get("foo").unwrap().id, 1);
        assert_eq!(thesaurus.get("bar").unwrap().id, 2);
        assert_eq!(thesaurus.get("baz").unwrap().id, 3);

        // TODO: No `parent` field in type `NormalizedTerm`
        // assert_eq!(thesaurus.get("foo").unwrap().parent, None);
        // assert_eq!(thesaurus.get("bar").unwrap().parent, Some("1".to_string()));
        // assert_eq!(thesaurus.get("baz").unwrap().parent, Some("2".to_string()));
    }
}
