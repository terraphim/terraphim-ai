//! MCP (Model Context Protocol) tools for RLM operations.
//!
//! This module provides 8 specialized MCP tools:
//! - `rlm_code`: Execute Python code in isolated VM
//! - `rlm_bash`: Execute bash commands in isolated VM
//! - `rlm_query`: Query LLM recursively
//! - `rlm_context`: Get/set context variables
//! - `rlm_snapshot`: Create/restore snapshots
//! - `rlm_status`: Get session status
//! - `mcp_search_tools`: Search a caller-supplied corpus of MCP tools by
//!   free-text query (powered by `terraphim_mcp_search::mcp_search_tools`)
//! - `mcp_search_skills`: Search a caller-supplied corpus of skill entries
//!   by free-text query (powered by `terraphim_mcp_search::mcp_search_skills`)
//!
//! ## Tool Search Tool compatibility (SEP-1821)
//!
//! `mcp_search_tools` and `mcp_search_skills` are the canonical
//! implementations of MCP's dynamic-tool-discovery pattern (SEP-1821 in the
//! Model Context Protocol spec). The MCP server SHOULD advertise this
//! capability via `ServerCapabilities.tools.listChanged = true` and accept
//! the `query` parameter on `tools/list` for clients that want native
//! integration. Today the implementation exposes the search via discrete
//! tools (more portable across the current MCP client ecosystem); the
//! capability advertisement is left for a follow-up when the
//! `rmcp` SDK ships SEP-1821 support upstream.

use std::sync::Arc;

use rmcp::model::{CallToolResult, Content, ErrorData, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use terraphim_mcp_search::{SkillEntry, mcp_search_skills, mcp_search_tools};
use terraphim_types::McpToolEntry;
use tokio::sync::RwLock;

use crate::rlm::TerraphimRlm;
use crate::types::SessionId;

// Note: McpError is in crate::error but we use RlmError.to_mcp_error()

/// Response payload for the `mcp_search_tools` MCP tool.
///
/// Returned as pretty-printed JSON inside a `CallToolResult::success`.
/// `query` echoes the input for client-side correlation; `count` is
/// pre-computed so the caller doesn't have to re-walk `tools`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSearchToolsResponse {
    /// The original query string (echoed back).
    pub query: String,
    /// Number of matching tools returned.
    pub count: usize,
    /// The matching tool entries, in original (Aho-Corasick) order.
    pub tools: Vec<McpToolEntry>,
}

/// Response payload for the `mcp_search_skills` MCP tool.
///
/// Parallel to [`McpSearchToolsResponse`] but for Terraphim skills. Same
/// serialisation pattern: pretty-printed JSON inside a single text content
/// block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSearchSkillsResponse {
    /// The original query string (echoed back).
    pub query: String,
    /// Number of matching skills returned.
    pub count: usize,
    /// The matching skill entries, in original (Aho-Corasick) order.
    pub skills: Vec<SkillEntry>,
}

/// RLM MCP service providing specialized tools for code execution.
#[derive(Clone)]
pub struct RlmMcpService {
    /// Reference to the RLM instance.
    rlm: Arc<RwLock<Option<TerraphimRlm>>>,
    /// Current session ID for tool operations.
    current_session: Arc<RwLock<Option<SessionId>>>,
}

impl RlmMcpService {
    /// Create a new RLM MCP service.
    pub fn new() -> Self {
        Self {
            rlm: Arc::new(RwLock::new(None)),
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    /// Initialize the service with an RLM instance.
    pub async fn initialize(&self, rlm: TerraphimRlm) {
        let mut guard = self.rlm.write().await;
        *guard = Some(rlm);
    }

    /// Set the current session for operations.
    pub async fn set_session(&self, session_id: SessionId) {
        let mut guard = self.current_session.write().await;
        *guard = Some(session_id);
    }

    /// Get tool definitions for RLM MCP tools.
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::rlm_code_tool(),
            Self::rlm_bash_tool(),
            Self::rlm_query_tool(),
            Self::rlm_context_tool(),
            Self::rlm_snapshot_tool(),
            Self::rlm_status_tool(),
            Self::mcp_search_tools_tool(),
            Self::mcp_search_skills_tool(),
        ]
    }

    /// Handle tool call dispatch.
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        match name {
            "rlm_code" => self.handle_rlm_code(arguments).await,
            "rlm_bash" => self.handle_rlm_bash(arguments).await,
            "rlm_query" => self.handle_rlm_query(arguments).await,
            "rlm_context" => self.handle_rlm_context(arguments).await,
            "rlm_snapshot" => self.handle_rlm_snapshot(arguments).await,
            "rlm_status" => self.handle_rlm_status(arguments).await,
            "mcp_search_tools" => self.handle_mcp_search_tools(arguments),
            "mcp_search_skills" => self.handle_mcp_search_skills(arguments),
            _ => Err(ErrorData::internal_error(
                format!("Unknown RLM tool: {}", name),
                None,
            )),
        }
    }

    // Tool definitions

    fn rlm_code_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "Python code to execute in the isolated VM"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Optional execution timeout in milliseconds"
                }
            },
            "required": ["code"]
        });

        Tool {
            name: "rlm_code".into(),
            title: Some("Execute Python Code".into()),
            description: Some(
                "Execute Python code in an isolated Firecracker VM. \
                Returns stdout, stderr, and exit status."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn rlm_bash_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Bash command to execute in the isolated VM"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Optional execution timeout in milliseconds"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory relative to session root"
                }
            },
            "required": ["command"]
        });

        Tool {
            name: "rlm_bash".into(),
            title: Some("Execute Bash Command".into()),
            description: Some(
                "Execute a bash command in an isolated Firecracker VM. \
                Commands are validated against the knowledge graph before execution."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn rlm_query_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The prompt/query to send to the LLM"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "model": {
                    "type": "string",
                    "description": "Optional model override for this query"
                },
                "temperature": {
                    "type": "number",
                    "description": "Optional temperature override (0.0-2.0)"
                },
                "max_tokens": {
                    "type": "integer",
                    "description": "Optional max tokens override"
                }
            },
            "required": ["prompt"]
        });

        Tool {
            name: "rlm_query".into(),
            title: Some("Query LLM".into()),
            description: Some(
                "Query the LLM recursively from within an RLM session. \
                Consumes from the session's token budget."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn rlm_context_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "set", "list", "delete"],
                    "description": "The action to perform on context variables"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "key": {
                    "type": "string",
                    "description": "Variable key (required for get, set, delete)"
                },
                "value": {
                    "type": "string",
                    "description": "Variable value (required for set)"
                }
            },
            "required": ["action"]
        });

        Tool {
            name: "rlm_context".into(),
            title: Some("Manage Context Variables".into()),
            description: Some(
                "Manage context variables within an RLM session. \
                Variables persist across executions within the same session."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn rlm_snapshot_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "restore", "list", "delete"],
                    "description": "The snapshot action to perform"
                },
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "snapshot_name": {
                    "type": "string",
                    "description": "Name for the snapshot (required for create, restore, delete)"
                }
            },
            "required": ["action"]
        });

        Tool {
            name: "rlm_snapshot".into(),
            title: Some("Manage VM Snapshots".into()),
            description: Some(
                "Manage VM snapshots for rollback support. \
                Create checkpoints and restore to previous states."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    fn rlm_status_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "session_id": {
                    "type": "string",
                    "description": "Optional session ID (uses current session if not provided)"
                },
                "include_history": {
                    "type": "boolean",
                    "description": "Whether to include command history in the response"
                }
            },
            "required": []
        });

        Tool {
            name: "rlm_status".into(),
            title: Some("Get Session Status".into()),
            description: Some(
                "Get the status of an RLM session including budget usage, \
                VM state, and optionally command history."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    /// MCP tool definition for `mcp_search_tools`.
    ///
    /// Takes a free-text query and an inline array of tool entries to search
    /// across. Returns matching entries as JSON. The caller decides which
    /// corpus to feed; this service is stateless about tools.
    fn mcp_search_tools_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search query (whitespace-split into keywords)"
                },
                "tools": {
                    "type": "array",
                    "description": "Array of MCP tool entries to search across. \
                                    Each entry must have at least 'name' and 'description'; \
                                    'server_name' and 'tags' are optional.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "description": { "type": "string" },
                            "server_name": { "type": "string" },
                            "tags": { "type": "array", "items": { "type": "string" } },
                            "input_schema": { "type": "object" }
                        },
                        "required": ["name", "description"]
                    }
                }
            },
            "required": ["query", "tools"]
        });

        Tool {
            name: "mcp_search_tools".into(),
            title: Some("Search MCP Tools".into()),
            description: Some(
                "Search a caller-supplied corpus of MCP tool entries for \
                those matching the query. Uses Aho-Corasick pattern matching \
                over name + description + tags. NFR: < 70ms for 100 tools."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    /// MCP tool definition for `mcp_search_skills`.
    ///
    /// Parallel to `mcp_search_tools` but for Terraphim skills. Takes a
    /// free-text query and an inline array of skill entries to search
    /// across. Returns matching entries as JSON. Same stateless contract:
    /// the caller decides which corpus to feed; this service does not
    /// maintain its own skill registry.
    ///
    /// `SkillEntry` is a discovery projection (name + description + version
    /// + author + tags) — `inputs` and `steps` are intentionally NOT part
    ///   of the search index because they're workflow-shape, not
    ///   discoverability-shape.
    fn mcp_search_skills_tool() -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Free-text search query (whitespace-split into keywords)"
                },
                "skills": {
                    "type": "array",
                    "description": "Array of skill entries to search across. \
                                    Each entry must have at least 'name' and 'description'; \
                                    'version', 'author', and 'tags' are optional.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "description": { "type": "string" },
                            "version": { "type": "string" },
                            "author": { "type": "string" },
                            "tags": { "type": "array", "items": { "type": "string" } }
                        },
                        "required": ["name", "description"]
                    }
                }
            },
            "required": ["query", "skills"]
        });

        Tool {
            name: "mcp_search_skills".into(),
            title: Some("Search Skills".into()),
            description: Some(
                "Search a caller-supplied corpus of Terraphim skill entries for \
                those matching the query. Uses Aho-Corasick pattern matching \
                over name + description + tags. NFR: < 70ms for 100 skills."
                    .into(),
            ),
            input_schema: Arc::new(schema.as_object().unwrap().clone()),
            output_schema: None,
            annotations: None,
            icons: None,
            meta: None,
        }
    }

    // Tool handlers

    async fn handle_rlm_code(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments
            .ok_or_else(|| ErrorData::invalid_params("Missing arguments for rlm_code", None))?;

        let code = args
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'code' parameter", None))?;

        let session_id = self.resolve_session_id(&args).await?;
        // timeout_ms is available for future use when execution context supports it
        let _timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64());

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match rlm.execute_code(&session_id, code).await {
            Ok(result) => {
                let response = RlmCodeResponse {
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: result.exit_code,
                    execution_time_ms: result.execution_time_ms,
                    success: result.is_success(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let mcp_error = e.to_mcp_error();
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&mcp_error).unwrap(),
                )]))
            }
        }
    }

    async fn handle_rlm_bash(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments
            .ok_or_else(|| ErrorData::invalid_params("Missing arguments for rlm_bash", None))?;

        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'command' parameter", None))?;

        let session_id = self.resolve_session_id(&args).await?;
        // These are available for future use when execution context supports them
        let _timeout_ms = args.get("timeout_ms").and_then(|v| v.as_u64());
        let _working_dir = args.get("working_dir").and_then(|v| v.as_str());

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match rlm.execute_command(&session_id, command).await {
            Ok(result) => {
                let response = RlmBashResponse {
                    stdout: result.stdout.clone(),
                    stderr: result.stderr.clone(),
                    exit_code: result.exit_code,
                    execution_time_ms: result.execution_time_ms,
                    success: result.is_success(),
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&response).unwrap(),
                )]))
            }
            Err(e) => {
                let mcp_error = e.to_mcp_error();
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&mcp_error).unwrap(),
                )]))
            }
        }
    }

    async fn handle_rlm_query(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments
            .ok_or_else(|| ErrorData::invalid_params("Missing arguments for rlm_query", None))?;

        let prompt = args
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'prompt' parameter", None))?;

        let session_id = self.resolve_session_id(&args).await?;
        // These are available for future use when query_llm supports overrides
        let _model = args.get("model").and_then(|v| v.as_str());
        let _temperature = args
            .get("temperature")
            .and_then(|v| v.as_f64())
            .map(|t| t as f32);
        let _max_tokens = args
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .map(|t| t as u32);

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match rlm.query_llm(&session_id, prompt).await {
            Ok(response) => {
                let result = RlmQueryResponse {
                    response: response.response,
                    tokens_used: response.tokens_used,
                    model: response.model,
                };
                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => {
                let mcp_error = e.to_mcp_error();
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&mcp_error).unwrap(),
                )]))
            }
        }
    }

    async fn handle_rlm_context(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments
            .ok_or_else(|| ErrorData::invalid_params("Missing arguments for rlm_context", None))?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'action' parameter", None))?;

        let session_id = self.resolve_session_id(&args).await?;

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match action {
            "get" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'key' for get", None))?;

                match rlm.get_context_variable(&session_id, key) {
                    Ok(value) => {
                        let response = RlmContextResponse {
                            action: "get".to_string(),
                            key: Some(key.to_string()),
                            value,
                            variables: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            "set" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'key' for set", None))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'value' for set", None))?;

                match rlm.set_context_variable(&session_id, key, value) {
                    Ok(()) => {
                        let response = RlmContextResponse {
                            action: "set".to_string(),
                            key: Some(key.to_string()),
                            value: Some(value.to_string()),
                            variables: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            "list" => match rlm.list_context_variables(&session_id).await {
                Ok(variables) => {
                    let response = RlmContextResponse {
                        action: "list".to_string(),
                        key: None,
                        value: None,
                        variables: Some(variables),
                    };
                    Ok(CallToolResult::success(vec![Content::text(
                        serde_json::to_string_pretty(&response).unwrap(),
                    )]))
                }
                Err(e) => {
                    let mcp_error = e.to_mcp_error();
                    Ok(CallToolResult::error(vec![Content::text(
                        serde_json::to_string_pretty(&mcp_error).unwrap(),
                    )]))
                }
            },
            "delete" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ErrorData::invalid_params("Missing 'key' for delete", None))?;

                match rlm.delete_context_variable(&session_id, key).await {
                    Ok(()) => {
                        let response = RlmContextResponse {
                            action: "delete".to_string(),
                            key: Some(key.to_string()),
                            value: None,
                            variables: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            _ => Err(ErrorData::invalid_params(
                format!("Invalid action: {}", action),
                None,
            )),
        }
    }

    async fn handle_rlm_snapshot(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments
            .ok_or_else(|| ErrorData::invalid_params("Missing arguments for rlm_snapshot", None))?;

        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'action' parameter", None))?;

        let session_id = self.resolve_session_id(&args).await?;

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match action {
            "create" => {
                let snapshot_name = args
                    .get("snapshot_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'snapshot_name' for create", None)
                    })?;

                match rlm.create_snapshot(&session_id, snapshot_name).await {
                    Ok(snapshot_id) => {
                        let response = RlmSnapshotResponse {
                            action: "create".to_string(),
                            snapshot_name: Some(snapshot_name.to_string()),
                            snapshot_id: Some(snapshot_id.name),
                            snapshots: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            "restore" => {
                let snapshot_name = args
                    .get("snapshot_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'snapshot_name' for restore", None)
                    })?;

                match rlm.restore_snapshot(&session_id, snapshot_name).await {
                    Ok(()) => {
                        let response = RlmSnapshotResponse {
                            action: "restore".to_string(),
                            snapshot_name: Some(snapshot_name.to_string()),
                            snapshot_id: None,
                            snapshots: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            "list" => {
                match rlm.list_snapshots(&session_id).await {
                    Ok(snapshots) => {
                        // Convert Vec<SnapshotId> to Vec<String> (names)
                        let snapshot_names: Vec<String> =
                            snapshots.iter().map(|s| s.name.clone()).collect();
                        let response = RlmSnapshotResponse {
                            action: "list".to_string(),
                            snapshot_name: None,
                            snapshot_id: None,
                            snapshots: Some(snapshot_names),
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            "delete" => {
                let snapshot_name = args
                    .get("snapshot_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        ErrorData::invalid_params("Missing 'snapshot_name' for delete", None)
                    })?;

                match rlm.delete_snapshot(&session_id, snapshot_name).await {
                    Ok(()) => {
                        let response = RlmSnapshotResponse {
                            action: "delete".to_string(),
                            snapshot_name: Some(snapshot_name.to_string()),
                            snapshot_id: None,
                            snapshots: None,
                        };
                        Ok(CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&response).unwrap(),
                        )]))
                    }
                    Err(e) => {
                        let mcp_error = e.to_mcp_error();
                        Ok(CallToolResult::error(vec![Content::text(
                            serde_json::to_string_pretty(&mcp_error).unwrap(),
                        )]))
                    }
                }
            }
            _ => Err(ErrorData::invalid_params(
                format!("Invalid action: {}", action),
                None,
            )),
        }
    }

    async fn handle_rlm_status(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments.unwrap_or_default();

        let session_id = self.resolve_session_id(&args).await?;
        let include_history = args
            .get("include_history")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let rlm_guard = self.rlm.read().await;
        let rlm = rlm_guard
            .as_ref()
            .ok_or_else(|| ErrorData::internal_error("RLM not initialized", None))?;

        match rlm.get_session_status(&session_id, include_history).await {
            Ok(status) => Ok(CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&status).unwrap(),
            )])),
            Err(e) => {
                let mcp_error = e.to_mcp_error();
                Ok(CallToolResult::error(vec![Content::text(
                    serde_json::to_string_pretty(&mcp_error).unwrap(),
                )]))
            }
        }
    }

    /// Handler for the `mcp_search_tools` MCP tool.
    ///
    /// Stateless: takes a query string and an inline `tools` array, runs
    /// `terraphim_mcp_search::mcp_search_tools`, and returns matching entries
    /// as JSON. The caller decides what corpus to feed; this service does not
    /// maintain its own registry.
    ///
    /// # Arguments
    ///
    /// The expected JSON shape is:
    /// ```json
    /// {
    ///   "query": "search",
    ///   "tools": [
    ///     {"name": "search_files", "description": "Search for files",
    ///      "server_name": "filesystem", "tags": ["fs"]}
    ///   ]
    /// }
    /// ```
    ///
    /// `tools` is parsed loosely: extra fields are ignored, missing fields
    /// default sensibly (`tags: []`, `input_schema: None`).
    fn handle_mcp_search_tools(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments.ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'query' parameter", None))?;

        let tools_value = args
            .get("tools")
            .ok_or_else(|| ErrorData::invalid_params("Missing 'tools' parameter", None))?;

        let tools_array = tools_value.as_array().ok_or_else(|| {
            ErrorData::invalid_params("'tools' must be an array of tool entries", None)
        })?;

        // Deserialise loosely. We don't require the caller to send every
        // field on McpToolEntry — only name + description are mandatory.
        let tools: Vec<McpToolEntry> = tools_array
            .iter()
            .cloned()
            .map(serde_json::from_value::<McpToolEntry>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ErrorData::invalid_params(format!("Invalid tool entry: {}", e), None))?;

        let hits = mcp_search_tools(query, tools.as_slice());

        let response = McpSearchToolsResponse {
            query: query.to_string(),
            count: hits.len(),
            tools: hits,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap(),
        )]))
    }

    /// Handler for the `mcp_search_skills` MCP tool.
    ///
    /// Parallel to `handle_mcp_search_tools` but for Terraphim skills.
    /// Stateless: takes a query string and an inline `skills` array, runs
    /// `terraphim_mcp_search::mcp_search_skills`, and returns matching
    /// entries as JSON.
    ///
    /// # Arguments
    ///
    /// Expected JSON shape:
    /// ```json
    /// {
    ///   "query": "review",
    ///   "skills": [
    ///     {"name": "code-review", "description": "Automated review",
    ///      "version": "1.0.0", "author": "Terraphim", "tags": ["quality"]}
    ///   ]
    /// }
    /// ```
    ///
    /// `skills` is parsed loosely: only `name` + `description` are required;
    /// other fields default sensibly.
    fn handle_mcp_search_skills(
        &self,
        arguments: Option<Map<String, serde_json::Value>>,
    ) -> Result<CallToolResult, ErrorData> {
        let args = arguments.ok_or_else(|| ErrorData::invalid_params("Missing arguments", None))?;

        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ErrorData::invalid_params("Missing 'query' parameter", None))?;

        let skills_value = args
            .get("skills")
            .ok_or_else(|| ErrorData::invalid_params("Missing 'skills' parameter", None))?;

        let skills_array = skills_value.as_array().ok_or_else(|| {
            ErrorData::invalid_params("'skills' must be an array of skill entries", None)
        })?;

        let skills: Vec<SkillEntry> = skills_array
            .iter()
            .cloned()
            .map(serde_json::from_value::<SkillEntry>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| ErrorData::invalid_params(format!("Invalid skill entry: {}", e), None))?;

        let hits = mcp_search_skills(query, skills.as_slice());

        let response = McpSearchSkillsResponse {
            query: query.to_string(),
            count: hits.len(),
            skills: hits,
        };

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&response).unwrap(),
        )]))
    }

    // Helper methods

    async fn resolve_session_id(
        &self,
        args: &Map<String, serde_json::Value>,
    ) -> Result<SessionId, ErrorData> {
        if let Some(session_str) = args.get("session_id").and_then(|v| v.as_str()) {
            SessionId::from_string(session_str)
                .map_err(|e| ErrorData::invalid_params(format!("Invalid session_id: {}", e), None))
        } else {
            let guard = self.current_session.read().await;
            guard.ok_or_else(|| {
                ErrorData::invalid_params("No session_id provided and no current session set", None)
            })
        }
    }
}

impl Default for RlmMcpService {
    fn default() -> Self {
        Self::new()
    }
}

// Response types

/// Response from rlm_code tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmCodeResponse {
    /// Standard output from the executed code.
    pub stdout: String,
    /// Standard error from the executed code.
    pub stderr: String,
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Wall-clock execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Whether the process exited with code 0.
    pub success: bool,
}

/// Response from rlm_bash tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmBashResponse {
    /// Standard output from the executed command.
    pub stdout: String,
    /// Standard error from the executed command.
    pub stderr: String,
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Wall-clock execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Whether the process exited with code 0.
    pub success: bool,
}

/// Response from rlm_query tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmQueryResponse {
    /// LLM-generated response text.
    pub response: String,
    /// Number of tokens consumed by this query.
    pub tokens_used: u64,
    /// Model identifier used for this query.
    pub model: String,
}

/// Response from rlm_context tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmContextResponse {
    /// Action that was performed (`"get"`, `"set"`, `"delete"`, `"list"`).
    pub action: String,
    /// Context variable key (present for `get`/`set`/`delete`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Context variable value (present for `get`/`set`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// All context variables (present for `list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<std::collections::HashMap<String, String>>,
}

/// Response from rlm_snapshot tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmSnapshotResponse {
    /// Action that was performed (`"create"`, `"restore"`, `"list"`, `"delete"`).
    pub action: String,
    /// Snapshot name (present for `create`/`restore`/`delete`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_name: Option<String>,
    /// Snapshot identifier assigned by the backend (present for `create`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    /// All snapshot names in the session (present for `list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshots: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tools() {
        let tools = RlmMcpService::get_tools();
        assert_eq!(tools.len(), 8);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"rlm_code"));
        assert!(names.contains(&"rlm_bash"));
        assert!(names.contains(&"rlm_query"));
        assert!(names.contains(&"rlm_context"));
        assert!(names.contains(&"rlm_snapshot"));
        assert!(names.contains(&"rlm_status"));
        assert!(names.contains(&"mcp_search_tools"));
        assert!(names.contains(&"mcp_search_skills"));
    }

    /// End-to-end test for the new MCP search tools. Builds a small corpus,
    /// calls each tool's handler, and asserts the deserialised response shape.
    #[tokio::test]
    async fn test_mcp_search_tools_e2e() {
        let service = RlmMcpService::new();

        let tools = vec![
            McpToolEntry::new("search_files", "Search for files", "filesystem"),
            McpToolEntry::new("read_file", "Read file contents", "filesystem"),
            McpToolEntry::new("grep_text", "Grep text with regex", "search"),
        ];

        // Invoke via the public dispatch path
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "query": "file",
                "tools": tools,
            }))
            .unwrap();

        let result = service
            .call_tool("mcp_search_tools", Some(args))
            .await
            .expect("call_tool mcp_search_tools");

        // Extract the JSON body from the first text content block.
        let body = match &result.content.first().expect("content present").raw {
            rmcp::model::RawContent::Text(t) => &t.text,
            _ => panic!("expected text content"),
        };
        let response: McpSearchToolsResponse = serde_json::from_str(body).unwrap();
        assert_eq!(response.query, "file");
        assert_eq!(response.count, 2);
        let names: Vec<&str> = response.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"search_files"));
        assert!(names.contains(&"read_file"));
    }

    #[tokio::test]
    async fn test_mcp_search_skills_e2e() {
        let service = RlmMcpService::new();

        let skills = vec![
            SkillEntry::new("code-review", "Automated code review")
                .with_tags(vec!["quality".into()]),
            SkillEntry::new("deploy", "Deploy to staging"),
        ];

        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({
                "query": "review",
                "skills": skills,
            }))
            .unwrap();

        let result = service
            .call_tool("mcp_search_skills", Some(args))
            .await
            .expect("call_tool mcp_search_skills");

        let body = match &result.content.first().expect("content present").raw {
            rmcp::model::RawContent::Text(t) => &t.text,
            _ => panic!("expected text content"),
        };
        let response: McpSearchSkillsResponse = serde_json::from_str(body).unwrap();
        assert_eq!(response.query, "review");
        assert_eq!(response.count, 1);
        assert_eq!(response.skills[0].name, "code-review");
    }

    #[tokio::test]
    async fn test_mcp_search_tools_invalid_args() {
        let service = RlmMcpService::new();

        // Missing 'tools' parameter.
        let args: serde_json::Map<String, serde_json::Value> =
            serde_json::from_value(serde_json::json!({"query": "anything"})).unwrap();
        let result = service.call_tool("mcp_search_tools", Some(args)).await;
        assert!(result.is_err(), "expected error for missing 'tools' param");

        // 'tools' is not an array.
        let args = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(
            serde_json::json!({"query": "x", "tools": "not-an-array"}),
        )
        .unwrap();
        let result = service.call_tool("mcp_search_tools", Some(args)).await;
        assert!(result.is_err(), "expected error for non-array 'tools'");
    }

    #[test]
    fn test_tool_schemas() {
        let tools = RlmMcpService::get_tools();

        for tool in &tools {
            // Each tool should have a valid JSON schema
            assert!(tool.input_schema.contains_key("type"));
            assert!(tool.input_schema.contains_key("properties"));
        }
    }
}
