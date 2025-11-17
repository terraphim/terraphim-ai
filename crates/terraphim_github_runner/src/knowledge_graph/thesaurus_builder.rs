//! Thesaurus builder for knowledge graph construction
//!
//! Converts parsed build terms into thesaurus entries compatible with terraphim_automata.

use crate::{BuildTerm, RunnerResult, RunnerError, TermType};
use crate::parsers::{BuildFileParser, EarthfileParser, DockerfileParser, WorkflowParser, ActionParser};
use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Thesaurus entry compatible with terraphim_automata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThesaurusEntry {
    /// Unique identifier
    pub id: String,
    /// Normalized term
    pub nterm: String,
    /// Reference URL
    pub url: String,
}

/// Builder for CI/CD knowledge graph thesaurus
pub struct ThesaurusBuilder {
    /// Collected terms
    terms: Vec<BuildTerm>,
    /// Base URL for the repository
    base_url: String,
    /// Term deduplication map
    term_map: AHashMap<String, BuildTerm>,
}

impl ThesaurusBuilder {
    /// Create a new thesaurus builder
    pub fn new(base_url: &str) -> Self {
        Self {
            terms: Vec::new(),
            base_url: base_url.to_string(),
            term_map: AHashMap::new(),
        }
    }

    /// Add an Earthfile to the thesaurus
    pub fn add_earthfile(&mut self, path: &Path) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, path.display());
        let parser = EarthfileParser::new(&file_url);
        let terms = parser.parse_file(path)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add a Dockerfile to the thesaurus
    pub fn add_dockerfile(&mut self, path: &Path) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, path.display());
        let parser = DockerfileParser::new(&file_url);
        let terms = parser.parse_file(path)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add a workflow file to the thesaurus
    pub fn add_workflow(&mut self, path: &Path) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, path.display());
        let parser = WorkflowParser::new(&file_url);
        let terms = parser.parse_file(path)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add an action.yml file to the thesaurus
    pub fn add_action(&mut self, path: &Path) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, path.display());
        let parser = ActionParser::new(&file_url);
        let terms = parser.parse_file(path)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add terms from content directly
    pub fn add_earthfile_content(&mut self, content: &str, name: &str) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, name);
        let parser = EarthfileParser::new(&file_url);
        let terms = parser.parse(content)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add workflow content directly
    pub fn add_workflow_content(&mut self, content: &str, name: &str) -> RunnerResult<&mut Self> {
        let file_url = format!("{}/{}", self.base_url, name);
        let parser = WorkflowParser::new(&file_url);
        let terms = parser.parse(content)?;
        self.add_terms(terms);
        Ok(self)
    }

    /// Add raw terms
    fn add_terms(&mut self, terms: Vec<BuildTerm>) {
        for term in terms {
            // Deduplicate by nterm
            if !self.term_map.contains_key(&term.nterm) {
                self.term_map.insert(term.nterm.clone(), term.clone());
                self.terms.push(term);
            }
        }
    }

    /// Add built-in CI/CD terms for common patterns
    pub fn add_builtin_terms(&mut self) -> &mut Self {
        let builtins = vec![
            // Common GitHub Actions
            ("actions/checkout", "https://github.com/actions/checkout", TermType::Action),
            ("actions/setup-node", "https://github.com/actions/setup-node", TermType::Action),
            ("actions/setup-python", "https://github.com/actions/setup-python", TermType::Action),
            ("actions/setup-go", "https://github.com/actions/setup-go", TermType::Action),
            ("actions/setup-java", "https://github.com/actions/setup-java", TermType::Action),
            ("actions/cache", "https://github.com/actions/cache", TermType::Action),
            ("actions/upload-artifact", "https://github.com/actions/upload-artifact", TermType::Action),
            ("actions/download-artifact", "https://github.com/actions/download-artifact", TermType::Action),
            // Common commands
            ("npm", "https://docs.npmjs.com/cli", TermType::Command),
            ("yarn", "https://yarnpkg.com/cli", TermType::Command),
            ("cargo", "https://doc.rust-lang.org/cargo/", TermType::Command),
            ("pip", "https://pip.pypa.io/", TermType::Command),
            ("go", "https://go.dev/doc/", TermType::Command),
            ("docker", "https://docs.docker.com/engine/reference/run/", TermType::Command),
            ("git", "https://git-scm.com/docs", TermType::Command),
            ("make", "https://www.gnu.org/software/make/manual/", TermType::Command),
            // Docker instructions
            ("FROM", "https://docs.docker.com/engine/reference/builder/#from", TermType::DockerInstruction),
            ("RUN", "https://docs.docker.com/engine/reference/builder/#run", TermType::DockerInstruction),
            ("COPY", "https://docs.docker.com/engine/reference/builder/#copy", TermType::DockerInstruction),
            ("ENV", "https://docs.docker.com/engine/reference/builder/#env", TermType::DockerInstruction),
            ("WORKDIR", "https://docs.docker.com/engine/reference/builder/#workdir", TermType::DockerInstruction),
            ("EXPOSE", "https://docs.docker.com/engine/reference/builder/#expose", TermType::DockerInstruction),
            ("ENTRYPOINT", "https://docs.docker.com/engine/reference/builder/#entrypoint", TermType::DockerInstruction),
            ("CMD", "https://docs.docker.com/engine/reference/builder/#cmd", TermType::DockerInstruction),
        ];

        for (nterm, url, term_type) in builtins {
            if !self.term_map.contains_key(nterm) {
                let term = BuildTerm {
                    id: uuid::Uuid::new_v4().to_string(),
                    nterm: nterm.to_string(),
                    url: url.to_string(),
                    term_type,
                    parent: None,
                    related: Vec::new(),
                };
                self.term_map.insert(nterm.to_string(), term.clone());
                self.terms.push(term);
            }
        }

        self
    }

    /// Build the thesaurus entries
    pub fn build(&self) -> Vec<ThesaurusEntry> {
        self.terms
            .iter()
            .map(|term| ThesaurusEntry {
                id: term.id.clone(),
                nterm: term.nterm.clone(),
                url: term.url.clone(),
            })
            .collect()
    }

    /// Export thesaurus to JSON format for terraphim_automata
    pub fn to_json(&self) -> RunnerResult<String> {
        let entries = self.build();
        serde_json::to_string_pretty(&entries)
            .map_err(|e| RunnerError::KnowledgeGraph(format!("Failed to serialize thesaurus: {}", e)))
    }

    /// Save thesaurus to file
    pub fn save_to_file(&self, path: &Path) -> RunnerResult<()> {
        let json = self.to_json()?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Get all terms
    pub fn terms(&self) -> &[BuildTerm] {
        &self.terms
    }

    /// Get term count
    pub fn len(&self) -> usize {
        self.terms.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Get terms by type
    pub fn terms_by_type(&self, term_type: TermType) -> Vec<&BuildTerm> {
        self.terms.iter().filter(|t| t.term_type == term_type).collect()
    }

    /// Get related terms for a given term
    pub fn get_related(&self, nterm: &str) -> Vec<&BuildTerm> {
        if let Some(term) = self.term_map.get(nterm) {
            term.related
                .iter()
                .filter_map(|r| self.term_map.get(r))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get child terms (terms with this as parent)
    pub fn get_children(&self, nterm: &str) -> Vec<&BuildTerm> {
        self.terms
            .iter()
            .filter(|t| t.parent.as_deref() == Some(nterm))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thesaurus_builder() {
        let mut builder = ThesaurusBuilder::new("https://github.com/owner/repo");

        let earthfile = r#"
VERSION 0.8

build:
    FROM rust:1.75
    RUN cargo build
"#;

        builder
            .add_earthfile_content(earthfile, "Earthfile")
            .unwrap()
            .add_builtin_terms();

        assert!(!builder.is_empty());

        // Check for parsed terms
        assert!(builder.terms().iter().any(|t| t.nterm == "+build"));

        // Check for builtin terms
        assert!(builder.terms().iter().any(|t| t.nterm == "cargo"));

        // Test JSON export
        let json = builder.to_json().unwrap();
        assert!(json.contains("nterm"));
    }

    #[test]
    fn test_term_relationships() {
        let mut builder = ThesaurusBuilder::new("https://github.com/owner/repo");

        let workflow = r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: npm test
"#;

        builder.add_workflow_content(workflow, ".github/workflows/ci.yml").unwrap();

        // Get children of job:build
        let children = builder.get_children("job:build");
        assert!(!children.is_empty());

        // Should have uses and run as children
        assert!(children.iter().any(|t| t.nterm.starts_with("uses:")));
        assert!(children.iter().any(|t| t.nterm.starts_with("run:")));
    }
}
