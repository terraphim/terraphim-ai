//! Parsers for extracting knowledge from build files
//!
//! This module contains parsers for:
//! - Earthfiles
//! - Dockerfiles
//! - GitHub Actions workflows
//! - Action metadata (action.yml)

pub mod earthfile;
pub mod dockerfile;
pub mod workflow;
pub mod action;

pub use earthfile::EarthfileParser;
pub use dockerfile::DockerfileParser;
pub use workflow::WorkflowParser;
pub use action::ActionParser;

use crate::{BuildTerm, RunnerResult};

/// Common trait for build file parsers
pub trait BuildFileParser {
    /// Parse content and extract terms for knowledge graph
    fn parse(&self, content: &str) -> RunnerResult<Vec<BuildTerm>>;

    /// Parse from file path
    fn parse_file(&self, path: &std::path::Path) -> RunnerResult<Vec<BuildTerm>> {
        let content = std::fs::read_to_string(path)?;
        self.parse(&content)
    }
}
