use anyhow::Result;
use std::sync::Arc;
use terraphim_config::Config;
use terraphim_middleware::Indexer;
use terraphim_service::TerraphimService;
use terraphim_types::{Document, RelevanceFunction, SearchQuery};

/// Search service integration layer
pub struct SearchService {
    service: Arc<TerraphimService>,
    config: Config,
}

#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub role: String,
    pub limit: usize,
    pub skip: usize,
    pub relevance_function: RelevanceFunction,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            role: "default".to_string(),
            limit: 10,
            skip: 0,
            relevance_function: RelevanceFunction::TerraphimGraph,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SearchResults {
    pub documents: Vec<Document>,
    pub total: usize,
    pub query: String,
    pub relevance_function: RelevanceFunction,
}

impl SearchService {
    /// Create search service from config
    pub fn new(config: Config) -> Result<Self> {
        let service = Arc::new(TerraphimService::new(config.clone())?);

        log::info!("SearchService initialized with {} roles", config.roles.len());

        Ok(Self { service, config })
    }

    /// Create from config file
    pub fn from_config_file(path: &str) -> Result<Self> {
        let config = Config::from_file(path)?;
        Self::new(config)
    }

    /// Search across knowledge sources
    pub async fn search(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<SearchResults> {
        if query.trim().is_empty() {
            return Ok(SearchResults {
                documents: vec![],
                total: 0,
                query: query.to_string(),
                relevance_function: options.relevance_function,
            });
        }

        log::info!("Searching for: '{}' (role: {})", query, options.role);

        let search_query = SearchQuery {
            query: query.to_string(),
            role: options.role.clone(),
            limit: options.limit,
            skip: options.skip,
            relevance_function: Some(options.relevance_function),
            ..Default::default()
        };

        let results = self.service.search(search_query).await?;

        log::info!(
            "Search completed: {} results (total: {})",
            results.documents.len(),
            results.total
        );

        Ok(SearchResults {
            documents: results.documents,
            total: results.total,
            query: query.to_string(),
            relevance_function: options.relevance_function,
        })
    }

    /// Parse query with operators (AND/OR)
    pub fn parse_query(query: &str) -> ParsedQuery {
        let query_lower = query.to_lowercase();

        if query_lower.contains(" and ") {
            let terms: Vec<String> = query_lower
                .split(" and ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            ParsedQuery {
                terms,
                operator: Some(LogicalOperator::And),
                original: query.to_string(),
            }
        } else if query_lower.contains(" or ") {
            let terms: Vec<String> = query_lower
                .split(" or ")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            ParsedQuery {
                terms,
                operator: Some(LogicalOperator::Or),
                original: query.to_string(),
            }
        } else {
            ParsedQuery {
                terms: vec![query.to_string()],
                operator: None,
                original: query.to_string(),
            }
        }
    }

    /// Get available roles
    pub fn available_roles(&self) -> Vec<String> {
        self.config.roles.keys().cloned().collect()
    }

    /// Get role configuration
    pub fn get_role_config(&self, role: &str) -> Option<&terraphim_config::Role> {
        self.config.roles.get(role)
    }
}

#[derive(Clone, Debug)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    pub operator: Option<LogicalOperator>,
    pub original: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOperator {
    And,
    Or,
}

impl ParsedQuery {
    pub fn is_complex(&self) -> bool {
        self.terms.len() > 1 && self.operator.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_query() {
        let query = SearchService::parse_query("rust");
        assert_eq!(query.terms.len(), 1);
        assert_eq!(query.terms[0], "rust");
        assert!(query.operator.is_none());
    }

    #[test]
    fn test_parse_and_query() {
        let query = SearchService::parse_query("rust AND tokio");
        assert_eq!(query.terms.len(), 2);
        assert_eq!(query.terms[0], "rust");
        assert_eq!(query.terms[1], "tokio");
        assert_eq!(query.operator, Some(LogicalOperator::And));
    }

    #[test]
    fn test_parse_or_query() {
        let query = SearchService::parse_query("rust OR async");
        assert_eq!(query.terms.len(), 2);
        assert_eq!(query.operator, Some(LogicalOperator::Or));
    }

    #[test]
    fn test_parse_case_insensitive() {
        let query = SearchService::parse_query("Rust and Tokio");
        assert_eq!(query.terms.len(), 2);
        assert_eq!(query.operator, Some(LogicalOperator::And));
    }
}
