//! `linear_haystack` — a haystack provider for Terraphim that searches
//! Linear issues via the Linear GraphQL API.
//!
//! Implements [`haystack_core::HaystackProvider`] over [`LinearHaystack`].
//! One round-trip per search (title + description + labels + top-3 comments
//! in a single query). Uses Linear's `searchableContent: { contains: ... }`
//! filter (verified via API probes 2026-07-29).
//!
//! See:
//! - `.docs/research-linear-haystack-and-tinyclaw-events.md` (Phase 1)
//! - `.docs/spike-report-phase-1.5-linear.md` (Phase 1.5)
//! - `.docs/design-linear-haystack-and-tinyclaw-events.md` (Phase 2)

pub mod client;
pub mod error;
pub mod mapping;

pub use client::{LinearClient, LinearComment, LinearIssue};
pub use error::LinearError;
pub use mapping::{DEFAULT_COMMENT_LIMIT, build_searchable_content, issue_to_document};

use haystack_core::HaystackProvider;
use terraphim_types::{Document, SearchQuery};

/// Haystack adapter for Linear. Wraps a [`LinearClient`] and translates
/// `SearchQuery` → `Vec<Document>` via single-round-trip GraphQL.
#[derive(Debug, Clone)]
pub struct LinearHaystack {
    client: LinearClient,
}

impl LinearHaystack {
    /// Construct a haystack with an explicit API key.
    pub fn new(api_key: impl Into<String>) -> anyhow::Result<Self> {
        Ok(Self {
            client: LinearClient::new(api_key)?,
        })
    }

    /// Construct a haystack with a custom endpoint (used by tests).
    pub fn with_endpoint(
        api_key: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            client: LinearClient::with_endpoint(api_key, endpoint)?,
        })
    }

    /// Resolve the API key via `terraphim-linear-auth` (3 sources: file →
    /// env → `op read`) and construct a haystack.
    #[cfg(feature = "linear")]
    pub fn from_auth() -> anyhow::Result<Self> {
        let client = LinearClient::from_auth()?;
        Ok(Self { client })
    }

    /// Direct access to the wrapped client (for callers who want to
    /// search with explicit `searchable_content` rather than the haystack
    /// `SearchQuery` shape).
    pub fn client(&self) -> &LinearClient {
        &self.client
    }
}

impl HaystackProvider for LinearHaystack {
    type Error = LinearError;

    async fn search(&self, query: &SearchQuery) -> Result<Vec<Document>, Self::Error> {
        let term = match build_searchable_content(&query.search_term.to_string()) {
            Some(t) => t,
            None => return Ok(Vec::new()),
        };
        let limit = query.limit.unwrap_or(25) as u32;
        let issues = self.client.search(&term, limit).await?;
        Ok(issues.iter().map(issue_to_document).collect())
    }
}
