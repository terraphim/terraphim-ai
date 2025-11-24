use anyhow::Result;
use std::sync::Arc;
use terraphim_config::{Config, ConfigState};
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
    pub async fn new(mut config: Config) -> Result<Self> {
        let config_state = ConfigState::new(&mut config).await?;
        let service = Arc::new(TerraphimService::new(config_state));

        log::info!("SearchService initialized with {} roles", config.roles.len());

        Ok(Self { service, config })
    }

    /// Create search service from config file path
    pub async fn from_config_file(path: &str) -> Result<Self> {
        let config = Config::from_file(path)?;
        Self::new(config).await
    }

    /// Perform search
    pub async fn search(&self, query: &str, options: SearchOptions) -> Result<SearchResults> {
        log::info!(
            "Searching for '{}' with role '{}' (limit: {})",
            query,
            options.role,
            options.limit
        );

        let search_query = SearchQuery {
            query: query.to_string(),
            role: options.role.clone(),
            relevance_function: options.relevance_function.clone(),
            limit: Some(options.limit),
            skip: Some(options.skip),
            smart_search: Some(false),
        };

        let documents = self.service.search(&search_query).await?;
        let total = documents.len();

        log::info!("Found {} documents for query '{}'", total, query);

        Ok(SearchResults {
            documents,
            total,
            query: query.to_string(),
            relevance_function: options.relevance_function,
        })
    }

    /// Parse query string into structured query
    pub fn parse_query(query: &str) -> ParsedQuery {
        let mut terms = Vec::new();
        let mut operator = None;

        // Simple parsing: split by AND/OR operators
        if query.contains(" AND ") {
            operator = Some(LogicalOperator::And);
            terms = query
                .split(" AND ")
                .map(|s| s.trim().to_string())
                .collect();
        } else if query.contains(" OR ") {
            operator = Some(LogicalOperator::Or);
            terms = query
                .split(" OR ")
                .map(|s| s.trim().to_string())
                .collect();
        } else {
            terms = vec![query.to_string()];
        }

        ParsedQuery { terms, operator }
    }

    /// Get available roles
    pub fn list_roles(&self) -> Vec<String> {
        self.config.roles.keys().cloned().collect()
    }

    /// Get current config
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

#[derive(Clone, Debug)]
pub struct ParsedQuery {
    pub terms: Vec<String>,
    pub operator: Option<LogicalOperator>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogicalOperator {
    And,
    Or,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_query_single_term() {
        let parsed = SearchService::parse_query("rust");
        assert_eq!(parsed.terms.len(), 1);
        assert_eq!(parsed.terms[0], "rust");
        assert!(parsed.operator.is_none());
    }

    #[test]
    fn test_parse_query_and_operator() {
        let parsed = SearchService::parse_query("rust AND tokio");
        assert_eq!(parsed.terms.len(), 2);
        assert_eq!(parsed.terms[0], "rust");
        assert_eq!(parsed.terms[1], "tokio");
        assert_eq!(parsed.operator, Some(LogicalOperator::And));
    }

    #[test]
    fn test_parse_query_or_operator() {
        let parsed = SearchService::parse_query("rust OR python");
        assert_eq!(parsed.terms.len(), 2);
        assert_eq!(parsed.terms[0], "rust");
        assert_eq!(parsed.terms[1], "python");
        assert_eq!(parsed.operator, Some(LogicalOperator::Or));
    }

    #[test]
    fn test_parse_query_multiple_and() {
        let parsed = SearchService::parse_query("rust AND tokio AND async");
        assert_eq!(parsed.terms.len(), 3);
        assert_eq!(parsed.operator, Some(LogicalOperator::And));
    }

    #[test]
    fn test_parse_query_with_whitespace() {
        let parsed = SearchService::parse_query("rust  AND  tokio");
        assert_eq!(parsed.terms[0], "rust");
        assert_eq!(parsed.terms[1], "tokio");
    }

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert_eq!(options.role, "default");
        assert_eq!(options.limit, 10);
        assert_eq!(options.skip, 0);
        assert_eq!(options.relevance_function, RelevanceFunction::TerraphimGraph);
    }
}
