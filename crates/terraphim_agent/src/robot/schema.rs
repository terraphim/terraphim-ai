//! Response schemas for robot mode
//!
//! Structured types for JSON responses that AI agents can parse reliably.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Standard response envelope for all robot mode outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotResponse<T: Serialize> {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response metadata
    pub meta: ResponseMeta,
    /// The actual data payload (None on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// List of errors (empty on success)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<RobotError>,
}

impl<T: Serialize> RobotResponse<T> {
    /// Create a successful response
    pub fn success(data: T, meta: ResponseMeta) -> Self {
        Self {
            success: true,
            meta,
            data: Some(data),
            errors: vec![],
        }
    }

    /// Create an error response
    pub fn error(errors: Vec<RobotError>, meta: ResponseMeta) -> RobotResponse<()> {
        RobotResponse {
            success: false,
            meta,
            data: None,
            errors,
        }
    }
}

/// Metadata about the response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMeta {
    /// Command that was executed
    pub command: String,
    /// Time taken in milliseconds
    pub elapsed_ms: u64,
    /// When the response was generated
    pub timestamp: DateTime<Utc>,
    /// Version of terraphim-agent
    pub version: String,
    /// Auto-correction info if command was corrected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_corrected: Option<AutoCorrection>,
    /// Pagination info if results were paginated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
    /// Token budget info if budget was applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<TokenBudget>,
}

impl ResponseMeta {
    /// Create new metadata for a command
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            elapsed_ms: 0,
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            auto_corrected: None,
            pagination: None,
            token_budget: None,
        }
    }

    /// Set elapsed time
    pub fn with_elapsed(mut self, elapsed_ms: u64) -> Self {
        self.elapsed_ms = elapsed_ms;
        self
    }

    /// Set auto-correction info
    pub fn with_auto_correction(mut self, auto_corrected: AutoCorrection) -> Self {
        self.auto_corrected = Some(auto_corrected);
        self
    }

    /// Set pagination info
    pub fn with_pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = Some(pagination);
        self
    }

    /// Set token budget info
    pub fn with_token_budget(mut self, token_budget: TokenBudget) -> Self {
        self.token_budget = Some(token_budget);
        self
    }
}

/// Information about auto-corrected commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCorrection {
    /// Original (possibly misspelled) command
    pub original: String,
    /// Corrected command that was executed
    pub corrected: String,
    /// Edit distance between original and corrected
    pub distance: usize,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Total number of results
    pub total: usize,
    /// Number of results returned
    pub returned: usize,
    /// Offset from start
    pub offset: usize,
    /// Whether there are more results
    pub has_more: bool,
}

impl Pagination {
    /// Create pagination info
    pub fn new(total: usize, returned: usize, offset: usize) -> Self {
        Self {
            total,
            returned,
            offset,
            has_more: offset + returned < total,
        }
    }
}

/// Token budget tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Maximum tokens allowed
    pub max_tokens: usize,
    /// Estimated tokens used
    pub estimated_tokens: usize,
    /// Whether output was truncated to fit budget
    pub truncated: bool,
}

impl TokenBudget {
    /// Create a new token budget tracker
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            estimated_tokens: 0,
            truncated: false,
        }
    }

    /// Update with estimated token count
    pub fn with_estimate(mut self, estimated_tokens: usize) -> Self {
        self.estimated_tokens = estimated_tokens;
        self.truncated = estimated_tokens >= self.max_tokens;
        self
    }
}

/// Structured error information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotError {
    /// Error code (e.g., "E001")
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    /// Suggested fix
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl RobotError {
    /// Create a new error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            suggestion: None,
        }
    }

    /// Add details
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    /// Add suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    // Common error constructors

    /// Unknown command error
    pub fn unknown_command(command: &str, suggestions: &[String]) -> Self {
        let mut err = Self::new("E001", format!("Unknown command: {}", command));
        if !suggestions.is_empty() {
            err = err.with_suggestion(format!("Did you mean: {}?", suggestions.join(", ")));
        }
        err
    }

    /// Invalid argument error
    pub fn invalid_argument(arg: &str, reason: &str) -> Self {
        Self::new("E002", format!("Invalid argument '{}': {}", arg, reason))
    }

    /// Missing argument error
    pub fn missing_argument(arg: &str) -> Self {
        Self::new("E003", format!("Missing required argument: {}", arg))
            .with_suggestion(format!("Provide the {} argument", arg))
    }

    /// Index not found error
    pub fn index_not_found(index_name: &str) -> Self {
        Self::new("E004", format!("Index not found: {}", index_name))
            .with_suggestion("Initialize the index first")
    }

    /// No results error
    pub fn no_results(query: &str) -> Self {
        Self::new("E005", format!("No results found for: {}", query))
            .with_suggestion("Try a broader search query")
    }

    /// Network error
    pub fn network_error(message: &str) -> Self {
        Self::new("E006", format!("Network error: {}", message))
    }

    /// Timeout error
    pub fn timeout_error(operation: &str, timeout_ms: u64) -> Self {
        Self::new(
            "E007",
            format!("Operation '{}' timed out after {}ms", operation, timeout_ms),
        )
    }

    /// Parse error
    pub fn parse_error(message: &str) -> Self {
        Self::new("E008", format!("Parse error: {}", message))
    }
}

/// Search results data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultsData {
    /// Search results
    pub results: Vec<SearchResultItem>,
    /// Total number of matches
    pub total_matches: usize,
    /// Concepts matched in the query
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concepts_matched: Vec<String>,
    /// Whether wildcard fallback was used
    #[serde(default)]
    pub wildcard_fallback: bool,
}

/// Individual search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    /// Rank in results
    pub rank: usize,
    /// Document/session ID
    pub id: String,
    /// Title or summary
    pub title: String,
    /// URL or path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Relevance score
    pub score: f64,
    /// Preview text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    /// Source (for sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Date (for sessions)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Whether preview was truncated
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub preview_truncated: bool,
}

/// Capabilities response data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesData {
    /// Agent name
    pub name: String,
    /// Agent version
    pub version: String,
    /// Description
    pub description: String,
    /// Available features
    pub features: FeatureFlags,
    /// Available commands
    pub commands: Vec<String>,
    /// Supported output formats
    pub supported_formats: Vec<String>,
    /// Index status (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_status: Option<IndexStatus>,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub search: bool,
    pub chat: bool,
    pub mcp_tools: bool,
    pub file_operations: bool,
    pub web_operations: bool,
    pub vm_execution: bool,
    pub session_search: bool,
    pub knowledge_graph: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            search: true,
            chat: cfg!(feature = "repl-chat"),
            mcp_tools: cfg!(feature = "repl-mcp"),
            file_operations: cfg!(feature = "repl-file"),
            web_operations: cfg!(feature = "repl-web"),
            vm_execution: true,
            session_search: false, // Will be true when sessions feature is added
            knowledge_graph: true,
        }
    }
}

/// Index status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    /// Number of documents indexed
    pub documents_indexed: usize,
    /// Number of sessions indexed
    pub sessions_indexed: usize,
    /// Last update timestamp
    pub last_updated: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_robot_response_success() {
        let meta = ResponseMeta::new("search");
        let response = RobotResponse::success("test data", meta);

        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.errors.is_empty());
    }

    #[test]
    fn test_robot_response_error() {
        let meta = ResponseMeta::new("search");
        let errors = vec![RobotError::no_results("test query")];
        let response = RobotResponse::<()>::error(errors, meta);

        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(!response.errors.is_empty());
    }

    #[test]
    fn test_pagination() {
        let pagination = Pagination::new(100, 10, 0);
        assert!(pagination.has_more);

        let pagination = Pagination::new(100, 10, 90);
        assert!(!pagination.has_more);
    }

    #[test]
    fn test_robot_error_serialization() {
        let error = RobotError::unknown_command("serach", &["search".to_string()]);
        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("E001"));
        assert!(json.contains("serach"));
    }
}
