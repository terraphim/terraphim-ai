//! MCP tools integration for REPL interface
//! Requires 'repl-mcp' feature
//!
//! Provides access to terraphim_automata functionality through the TuiService,
//! including autocomplete, text matching, paragraph extraction, and thesaurus lookup.

#[cfg(feature = "repl-mcp")]
use crate::service::TuiService;
#[cfg(feature = "repl-mcp")]
use std::sync::Arc;
#[cfg(feature = "repl-mcp")]
use terraphim_automata::LinkType;
#[cfg(feature = "repl-mcp")]
use terraphim_types::RoleName;

#[cfg(feature = "repl-mcp")]
pub struct McpToolsHandler {
    service: Arc<TuiService>,
}

#[cfg(feature = "repl-mcp")]
#[allow(dead_code)]
impl McpToolsHandler {
    /// Create a new McpToolsHandler with a reference to the TuiService
    pub fn new(service: Arc<TuiService>) -> Self {
        Self { service }
    }

    /// Get the currently selected role from the service
    async fn get_role(&self) -> RoleName {
        self.service.get_selected_role().await
    }

    /// Autocomplete terms based on the query prefix
    ///
    /// Returns a list of matching terms from the thesaurus for the current role.
    pub async fn autocomplete_terms(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<Vec<String>> {
        let role = self.get_role().await;
        let results = self.service.autocomplete(&role, query, limit).await?;
        Ok(results.into_iter().map(|r| r.term).collect())
    }

    /// Extract paragraphs from text that contain thesaurus terms
    ///
    /// Returns pairs of (matched_term, paragraph_text).
    pub async fn extract_paragraphs(
        &self,
        text: &str,
        exclude_term: bool,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let role = self.get_role().await;
        self.service
            .extract_paragraphs(&role, text, exclude_term)
            .await
    }

    /// Find all thesaurus term matches in the given text
    ///
    /// Returns a list of matched terms.
    pub async fn find_matches(&self, text: &str) -> anyhow::Result<Vec<String>> {
        let role = self.get_role().await;
        let matches = self.service.find_matches(&role, text).await?;
        Ok(matches.into_iter().map(|m| m.term).collect())
    }

    /// Replace matched terms in text with links
    ///
    /// The format parameter specifies the link format:
    /// - "html" for HTML anchor tags
    /// - "markdown" or None for Markdown links
    pub async fn replace_matches(
        &self,
        text: &str,
        format: Option<String>,
    ) -> anyhow::Result<String> {
        let role = self.get_role().await;
        let link_type = match format.as_deref() {
            Some("html") => LinkType::HTMLLinks,
            Some("markdown") | None => LinkType::MarkdownLinks,
            _ => LinkType::MarkdownLinks,
        };
        self.service.replace_matches(&role, text, link_type).await
    }

    /// Get thesaurus entries for a role
    ///
    /// Returns pairs of (normalized_term, url).
    /// If role is None, uses the currently selected role.
    pub async fn get_thesaurus(
        &self,
        role: Option<String>,
    ) -> anyhow::Result<Vec<(String, String)>> {
        let role_name = match role {
            Some(r) => RoleName::new(&r),
            None => self.get_role().await,
        };
        let thesaurus = self.service.get_thesaurus(&role_name).await?;

        // Convert thesaurus to (term, url) pairs
        let pairs: Vec<_> = thesaurus
            .into_iter()
            .map(|(_, nterm)| {
                (
                    nterm.value.to_string(),
                    nterm.url.clone().unwrap_or_default(),
                )
            })
            .collect();
        Ok(pairs)
    }
}
