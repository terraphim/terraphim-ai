//! Client detection module for identifying LLM client types
//!
//! Detects which client is making requests (Claude Code, Codex CLI, OpenClaw, etc.)
//! based on HTTP headers and request patterns. This enables client-specific routing
//! and model mapping.

use axum::http::HeaderMap;

/// Detected client type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientType {
    /// Anthropic Claude Code
    ClaudeCode,
    /// OpenAI Codex CLI
    CodexCli,
    /// OpenClaw assistant
    OpenClaw,
    /// Generic OpenAI API client
    OpenAiGeneric,
    /// Unknown/generic client
    Unknown,
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::ClaudeCode => write!(f, "claude-code"),
            ClientType::CodexCli => write!(f, "codex-cli"),
            ClientType::OpenClaw => write!(f, "openclaw"),
            ClientType::OpenAiGeneric => write!(f, "openai-generic"),
            ClientType::Unknown => write!(f, "unknown"),
        }
    }
}

/// How the client was detected
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Matched on header pattern
    HeaderPattern,
    /// Matched on User-Agent string
    UserAgent,
    /// Matched on request path
    RequestPath,
    /// Inferred from heuristics
    Heuristic,
}

/// API format expected by the client
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiFormat {
    /// Anthropic API format (/v1/messages)
    Anthropic,
    /// OpenAI API format (/v1/chat/completions)
    OpenAI,
}

/// Client detection result
#[derive(Debug, Clone)]
pub struct ClientInfo {
    /// Detected client type
    pub client_type: ClientType,
    /// How the client was detected
    pub detected_by: DetectionMethod,
    /// API format expected by the client
    pub api_format: ApiFormat,
}

impl ClientInfo {
    /// Create new client info
    pub fn new(client_type: ClientType, detected_by: DetectionMethod) -> Self {
        let api_format = client_type.default_api_format();
        Self {
            client_type,
            detected_by,
            api_format,
        }
    }
}

impl ClientType {
    /// Get the default API format for this client type
    pub fn default_api_format(&self) -> ApiFormat {
        match self {
            ClientType::ClaudeCode => ApiFormat::Anthropic,
            ClientType::CodexCli => ApiFormat::OpenAI,
            ClientType::OpenClaw => ApiFormat::Anthropic, // OpenClaw can use either, default to Anthropic
            ClientType::OpenAiGeneric => ApiFormat::OpenAI,
            ClientType::Unknown => ApiFormat::Anthropic, // Default to Anthropic for backward compatibility
        }
    }

    /// Check if client expects Anthropic API format
    pub fn expects_anthropic_format(&self) -> bool {
        matches!(self.default_api_format(), ApiFormat::Anthropic)
    }

    /// Check if client expects OpenAI API format
    pub fn expects_openai_format(&self) -> bool {
        matches!(self.default_api_format(), ApiFormat::OpenAI)
    }
}

/// Detect client type from HTTP headers and request path
///
/// # Arguments
/// * `headers` - HTTP request headers
/// * `path` - Request path (e.g., "/v1/messages")
///
/// # Returns
/// Detected client information
///
/// # Examples
/// ```
/// use axum::http::HeaderMap;
/// use terraphim_llm_proxy::client_detection::{detect_client, ClientType};
///
/// let mut headers = HeaderMap::new();
/// headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
/// let info = detect_client(&headers, "/v1/messages");
/// assert_eq!(info.client_type, ClientType::ClaudeCode);
/// ```
pub fn detect_client(headers: &HeaderMap, path: &str) -> ClientInfo {
    // Priority 1: Check for anthropic-version header (Claude Code)
    if headers.contains_key("anthropic-version") {
        return ClientInfo::new(ClientType::ClaudeCode, DetectionMethod::HeaderPattern);
    }

    // Priority 2: Check User-Agent for specific clients
    if let Some(user_agent) = headers.get("user-agent") {
        if let Ok(ua_str) = user_agent.to_str() {
            let ua_lower = ua_str.to_lowercase();
            if ua_lower.contains("openclaw") {
                return ClientInfo::new(ClientType::OpenClaw, DetectionMethod::UserAgent);
            }
            if ua_lower.contains("claude-code") || ua_lower.contains("claude code") {
                return ClientInfo::new(ClientType::ClaudeCode, DetectionMethod::UserAgent);
            }
            if ua_lower.contains("codex") {
                return ClientInfo::new(ClientType::CodexCli, DetectionMethod::UserAgent);
            }
        }
    }

    // Priority 3: Check Authorization header pattern
    if let Some(auth) = headers.get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            // OpenAI format: "Bearer sk-..."
            if auth_str.starts_with("Bearer sk-") {
                // Could be Codex CLI or generic OpenAI client
                // Try to distinguish by checking other headers
                return ClientInfo::new(ClientType::OpenAiGeneric, DetectionMethod::HeaderPattern);
            }
        }
    }

    // Priority 4: Path-based detection as fallback
    match path {
        "/v1/messages" | "/v1/messages/count_tokens" => {
            ClientInfo::new(ClientType::ClaudeCode, DetectionMethod::RequestPath)
        }
        "/v1/chat/completions" | "/v1/completions" => {
            ClientInfo::new(ClientType::OpenAiGeneric, DetectionMethod::RequestPath)
        }
        _ => ClientInfo::new(ClientType::Unknown, DetectionMethod::Heuristic),
    }
}

/// Check if client expects Anthropic API format
pub fn client_expects_anthropic_format(client_type: ClientType) -> bool {
    client_type.expects_anthropic_format()
}

/// Check if client expects OpenAI API format
pub fn client_expects_openai_format(client_type: ClientType) -> bool {
    client_type.expects_openai_format()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_detect_claude_code_by_anthropic_version_header() {
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert("x-api-key", HeaderValue::from_static("test-key"));

        let info = detect_client(&headers, "/v1/messages");

        assert_eq!(info.client_type, ClientType::ClaudeCode);
        assert_eq!(info.detected_by, DetectionMethod::HeaderPattern);
        assert_eq!(info.api_format, ApiFormat::Anthropic);
    }

    #[test]
    fn test_detect_claude_code_by_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("claude-code/1.0"));
        headers.insert("x-api-key", HeaderValue::from_static("test-key"));

        let info = detect_client(&headers, "/v1/messages");

        assert_eq!(info.client_type, ClientType::ClaudeCode);
        assert_eq!(info.detected_by, DetectionMethod::UserAgent);
    }

    #[test]
    fn test_detect_openclaw_by_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("OpenClaw/0.35"));
        headers.insert("authorization", HeaderValue::from_static("Bearer sk-test"));

        let info = detect_client(&headers, "/v1/chat/completions");

        assert_eq!(info.client_type, ClientType::OpenClaw);
        assert_eq!(info.detected_by, DetectionMethod::UserAgent);
        assert_eq!(info.api_format, ApiFormat::Anthropic);
    }

    #[test]
    fn test_detect_codex_cli_by_user_agent() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static("codex-cli/1.0"));
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-test123"),
        );

        let info = detect_client(&headers, "/v1/chat/completions");

        assert_eq!(info.client_type, ClientType::CodexCli);
        assert_eq!(info.detected_by, DetectionMethod::UserAgent);
        assert_eq!(info.api_format, ApiFormat::OpenAI);
    }

    #[test]
    fn test_detect_openai_generic_by_auth_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer sk-abc123"),
        );

        let info = detect_client(&headers, "/v1/chat/completions");

        assert_eq!(info.client_type, ClientType::OpenAiGeneric);
        assert_eq!(info.detected_by, DetectionMethod::HeaderPattern);
        assert_eq!(info.api_format, ApiFormat::OpenAI);
    }

    #[test]
    fn test_detect_by_path_anthropic() {
        let headers = HeaderMap::new();

        let info = detect_client(&headers, "/v1/messages");

        assert_eq!(info.client_type, ClientType::ClaudeCode);
        assert_eq!(info.detected_by, DetectionMethod::RequestPath);
    }

    #[test]
    fn test_detect_by_path_openai() {
        let headers = HeaderMap::new();

        let info = detect_client(&headers, "/v1/chat/completions");

        assert_eq!(info.client_type, ClientType::OpenAiGeneric);
        assert_eq!(info.detected_by, DetectionMethod::RequestPath);
    }

    #[test]
    fn test_detect_unknown() {
        let headers = HeaderMap::new();

        let info = detect_client(&headers, "/v1/models");

        assert_eq!(info.client_type, ClientType::Unknown);
        assert_eq!(info.detected_by, DetectionMethod::Heuristic);
    }

    #[test]
    fn test_client_type_display() {
        assert_eq!(ClientType::ClaudeCode.to_string(), "claude-code");
        assert_eq!(ClientType::CodexCli.to_string(), "codex-cli");
        assert_eq!(ClientType::OpenClaw.to_string(), "openclaw");
        assert_eq!(ClientType::OpenAiGeneric.to_string(), "openai-generic");
        assert_eq!(ClientType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_client_type_api_format() {
        assert!(ClientType::ClaudeCode.expects_anthropic_format());
        assert!(!ClientType::ClaudeCode.expects_openai_format());

        assert!(ClientType::CodexCli.expects_openai_format());
        assert!(!ClientType::CodexCli.expects_anthropic_format());

        assert!(ClientType::OpenClaw.expects_anthropic_format());
        assert!(ClientType::OpenAiGeneric.expects_openai_format());
    }

    #[test]
    fn test_anthropic_version_header_takes_priority() {
        let mut headers = HeaderMap::new();
        // Both headers present - anthropic-version should win
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        headers.insert("user-agent", HeaderValue::from_static("codex-cli/1.0"));

        let info = detect_client(&headers, "/v1/chat/completions");

        // Should detect as Claude Code due to anthropic-version header
        assert_eq!(info.client_type, ClientType::ClaudeCode);
        assert_eq!(info.detected_by, DetectionMethod::HeaderPattern);
    }

    #[test]
    fn test_openai_endpoint_with_anthropic_headers() {
        // This tests the case where a client uses OpenAI endpoint but sends Anthropic headers
        // (shouldn't happen in practice, but good to define behavior)
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));

        let info = detect_client(&headers, "/v1/chat/completions");

        // Header-based detection takes priority
        assert_eq!(info.client_type, ClientType::ClaudeCode);
    }
}
