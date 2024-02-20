use crate::indexer::LogseqIndexer;
use crate::Result;
use std::path::PathBuf;
use terraphim_config::{Config, ConfigState, ServiceType};
use terraphim_types::Thesaurus;

/// A KnowledgeGraphBuilder receives a path containing
/// resources (e.g. files) with key-value pairs and returns a `Thesaurus`
/// (a dictionary with synonyms which map to higher-level concepts)
trait KnowledgeGraphBuilder {
    /// `path` is path to the knowledge graph input
    /// (e.g. a directory of Markdown files)
    // TODO: This should be generalized (e.g. to take a `Read` trait object
    // or a `Resource` or a glob of inputs)
    async fn build(&self, path: PathBuf) -> Result<Thesaurus>;
}

/// A builder for a knowledge graph, which knows how to handle Markdown input.
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
    ///
    /// # Arguments
    ///
    /// - `path`: The path to the knowledge graph input (e.g. a directory of
    /// Markdown files)
    async fn build(&self, path: PathBuf) -> Result<Thesaurus> {
        // Initialize a logseq service for parsing the data source
        let mut config = Config::new(ServiceType::Logseq);
        let config_state = ConfigState::new(&mut config).await?;
        let logseq_service = LogseqIndexer::new(config_state);

        // let mut thesaurus = Thesaurus::new();
        // let mut parser = terraphim_markdown::Parser::new();
        // let entries = parser.parse_directory(&self.kg_path).await?;
        // for entry in entries {
        //     thesaurus.insert(entry);
        // }
        // Ok(thesaurus)

        todo!()
    }
}
