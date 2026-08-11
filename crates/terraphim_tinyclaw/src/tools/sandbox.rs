//! `SandboxTool` — Hermes-parity sandboxed execution (#3146).
//!
//! Wraps `terraphim_rlm` for isolated code/shell execution. Operations:
//! - `execute_code` — run Python code in a session
//! - `execute_bash` — run a shell command in a session
//! - `recursive_query` — recursive LLM query (requires LLM client wiring)
//! - `session_create` / `session_status` / `session_destroy` — lifecycle
//!
//! Design notes (from `docs/plans/research-tinyclaw-parity-tools.md`):
//! - RLM sessions live in an in-memory `DashMap` (`session.rs`), so a CLI
//!   bridge cannot hold a session across processes — the crate must be
//!   called in-process. That's why this tool embeds `TerraphimRlm`.
//! - KG validation is configured `Permissive` with no thesaurus so ordinary
//!   code is not rejected as "unknown terms" (`validator.rs:239-241` skips
//!   validation in that configuration).
//! - Backend preference comes from config (`local` default, `docker`
//!   optional); `select_executor` falls back through the list.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use terraphim_rlm::config::{BackendType, KgStrictness, RlmConfig};
use terraphim_rlm::rlm::TerraphimRlm;
use terraphim_rlm::types::SessionId;

/// Configuration for the sandbox tool, converted from
/// [`crate::config::SandboxConfig`].
#[derive(Debug, Clone)]
pub struct SandboxToolConfig {
    /// Backend preference: `"local"` (default) or `"docker"`.
    pub backend: String,
    /// Per-execution timeout in seconds (RLM time budget).
    pub timeout_secs: u64,
    /// Maximum output bytes surfaced per execution result.
    pub max_output_bytes: usize,
}

impl From<&crate::config::SandboxConfig> for SandboxToolConfig {
    fn from(cfg: &crate::config::SandboxConfig) -> Self {
        Self {
            backend: cfg.backend.clone(),
            timeout_secs: cfg.timeout_secs,
            max_output_bytes: cfg.max_output_bytes,
        }
    }
}

impl SandboxToolConfig {
    /// Build an `RlmConfig` with sandbox-appropriate settings.
    pub fn to_rlm_config(&self) -> RlmConfig {
        RlmConfig {
            // Permissive + no thesaurus => validator passes code through.
            kg_strictness: KgStrictness::Permissive,
            thesaurus: None,
            // Backend preference from config; `select_executor` falls back
            // to the next available backend if the first can't initialize.
            backend_preference: match self.backend.as_str() {
                "docker" => vec![BackendType::Docker, BackendType::Local],
                _ => vec![BackendType::Local, BackendType::Docker],
            },
            // Time budget in ms.
            time_budget_ms: self.timeout_secs.saturating_mul(1000),
            ..Default::default()
        }
    }
}

/// The sandbox tool. All operations are async and return JSON strings.
pub struct SandboxTool {
    /// The RLM engine (shared — sessions persist across calls in-process).
    rlm: Arc<TerraphimRlm>,
    /// Config snapshot for output bounding.
    config: SandboxToolConfig,
    /// Lazily created default session for execute_* calls without an
    /// explicit session_id.
    default_session: tokio::sync::Mutex<Option<SessionId>>,
}

impl SandboxTool {
    /// Create a sandbox tool with a pre-built RLM engine.
    pub fn new(rlm: TerraphimRlm, config: SandboxToolConfig) -> Self {
        Self {
            rlm: Arc::new(rlm),
            config,
            default_session: tokio::sync::Mutex::new(None),
        }
    }

    /// Create a sandbox tool from config, building the RLM engine.
    pub async fn from_config(cfg: &crate::config::SandboxConfig) -> Result<Self, ToolError> {
        let tool_cfg = SandboxToolConfig::from(cfg);
        let rlm_config = tool_cfg.to_rlm_config();
        let rlm = TerraphimRlm::new(rlm_config)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "sandbox".to_string(),
                message: format!("failed to initialize RLM: {e}"),
            })?;
        Ok(Self::new(rlm, tool_cfg))
    }

    /// Bounded output: truncate at max_output_bytes on a char boundary.
    fn bound_output(&self, s: &str) -> String {
        let max = self.config.max_output_bytes;
        if s.len() <= max {
            return s.to_string();
        }
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}… (truncated, {} bytes)", &s[..end], s.len())
    }

    /// Ensure a session exists; returns the session id to use.
    async fn ensure_session(&self, explicit: Option<&str>) -> Result<SessionId, ToolError> {
        if let Some(sid) = explicit {
            return SessionId::from_string(sid).map_err(|_| ToolError::InvalidArguments {
                tool: "sandbox".to_string(),
                message: format!("invalid session_id '{sid}' (must be a ULID)"),
            });
        }
        let mut guard = self.default_session.lock().await;
        if let Some(sid) = guard.as_ref() {
            return Ok(*sid);
        }
        let info = self
            .rlm
            .create_session()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "sandbox".to_string(),
                message: format!("failed to create session: {e}"),
            })?;
        let sid = info.id;
        *guard = Some(sid);
        Ok(sid)
    }

    /// Parse a session id from the args, requiring it.
    fn require_session_id(&self, sid: Option<&str>) -> Result<SessionId, ToolError> {
        let sid = sid.ok_or_else(|| ToolError::InvalidArguments {
            tool: "sandbox".to_string(),
            message: "operation requires 'session_id'".to_string(),
        })?;
        SessionId::from_string(sid).map_err(|_| ToolError::InvalidArguments {
            tool: "sandbox".to_string(),
            message: format!("invalid session_id '{sid}' (must be a ULID)"),
        })
    }
}

/// Run an RLM query on a dedicated current-thread runtime.
///
/// `TerraphimRlm::query` builds a `QueryLoop` containing `Cell<u32>`
/// (query_loop.rs), which makes the future `!Send`. The `Tool` trait's
/// `execute` must return a `Send` future, so we bounce the call through a
/// dedicated thread running a current-thread tokio runtime with a
/// `LocalSet` (which permits `!Send` futures), and hand the result back
/// over a oneshot channel.
async fn run_query_on_dedicated_thread(
    rlm: Arc<TerraphimRlm>,
    session_id: SessionId,
    prompt: String,
) -> Result<terraphim_rlm::query_loop::QueryLoopResult, ToolError> {
    let (tx, rx) = tokio::sync::oneshot::channel::<
        Result<terraphim_rlm::query_loop::QueryLoopResult, terraphim_rlm::error::RlmError>,
    >();
    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                let _ = tx.send(Err(terraphim_rlm::error::RlmError::Internal {
                    message: format!("failed to build query runtime: {e}"),
                }));
                return;
            }
        };
        let local = tokio::task::LocalSet::new();
        local.block_on(&rt, async move {
            let result = rlm.query(&session_id, &prompt).await;
            let _ = tx.send(result);
        });
    });
    rx.await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "sandbox".to_string(),
            message: format!("recursive_query channel closed: {e}"),
        })?
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "sandbox".to_string(),
            message: format!("recursive_query failed: {e}"),
        })
}

#[async_trait]
impl Tool for SandboxTool {
    fn name(&self) -> &str {
        "sandbox"
    }

    fn description(&self) -> &str {
        "Execute code or shell commands in an isolated sandbox (RLM backend: \
         local or docker), and manage sandbox sessions. Operations: \
         execute_code {code, session_id?}, execute_bash {command, session_id?}, \
         recursive_query {prompt, session_id?}, session_create {}, \
         session_status {session_id}, session_destroy {session_id}."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["execute_code", "execute_bash", "recursive_query",
                             "session_create", "session_status", "session_destroy"],
                    "description": "Operation to perform"
                },
                "code": { "type": "string", "description": "Python code (execute_code)" },
                "command": { "type": "string", "description": "Shell command (execute_bash)" },
                "prompt": { "type": "string", "description": "Prompt (recursive_query)" },
                "session_id": { "type": "string", "description": "Optional session id" }
            },
            "required": ["op"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let op =
            args.get("op")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "sandbox".to_string(),
                    message: "missing required 'op' field".to_string(),
                })?;

        let sid = args.get("session_id").and_then(|v| v.as_str());

        match op {
            "execute_code" => {
                let code = args.get("code").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "sandbox".to_string(),
                        message: "execute_code requires 'code'".to_string(),
                    }
                })?;
                let session_id = self.ensure_session(sid).await?;
                let result = self
                    .rlm
                    .execute_code(&session_id, code)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "sandbox".to_string(),
                        message: format!("execute_code failed: {e}"),
                    })?;
                Ok(json!({
                    "op": "execute_code",
                    "success": result.is_success(),
                    "stdout": self.bound_output(&result.stdout),
                    "stderr": self.bound_output(&result.stderr),
                    "session_id": session_id.to_string(),
                })
                .to_string())
            }
            "execute_bash" => {
                let command = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "sandbox".to_string(),
                        message: "execute_bash requires 'command'".to_string(),
                    })?;
                let session_id = self.ensure_session(sid).await?;
                let result = self
                    .rlm
                    .execute_command(&session_id, command)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "sandbox".to_string(),
                        message: format!("execute_bash failed: {e}"),
                    })?;
                Ok(json!({
                    "op": "execute_bash",
                    "success": result.is_success(),
                    "stdout": self.bound_output(&result.stdout),
                    "stderr": self.bound_output(&result.stderr),
                    "session_id": session_id.to_string(),
                })
                .to_string())
            }
            "recursive_query" => {
                let prompt = args.get("prompt").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "sandbox".to_string(),
                        message: "recursive_query requires 'prompt'".to_string(),
                    }
                })?;
                let session_id = self.ensure_session(sid).await?;
                // RLM's query loop uses `Cell<u32>` (query_loop.rs), making
                // the future !Send. Run it on a dedicated current-thread
                // runtime + LocalSet so the Tool trait's Send future
                // contract is preserved.
                let result =
                    run_query_on_dedicated_thread(self.rlm.clone(), session_id, prompt.to_string())
                        .await?;
                Ok(json!({
                    "op": "recursive_query",
                    "success": result.success,
                    "response": self.bound_output(result.result.as_deref().unwrap_or("")),
                    "iterations": result.iterations,
                    "session_id": session_id.to_string(),
                })
                .to_string())
            }
            "session_create" => {
                let info =
                    self.rlm
                        .create_session()
                        .await
                        .map_err(|e| ToolError::ExecutionFailed {
                            tool: "sandbox".to_string(),
                            message: format!("session_create failed: {e}"),
                        })?;
                Ok(json!({
                    "op": "session_create",
                    "session_id": info.id.to_string(),
                    "state": format!("{:?}", info.state),
                })
                .to_string())
            }
            "session_status" => {
                let session_id = self.require_session_id(sid)?;
                let status = self
                    .rlm
                    .get_session_status(&session_id, false)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "sandbox".to_string(),
                        message: format!("session_status failed: {e}"),
                    })?;
                Ok(json!({
                    "op": "session_status",
                    "session_id": session_id.to_string(),
                    "state": format!("{:?}", status.session_info.state),
                    "backend": status.backend_type.to_string(),
                    "snapshot_count": status.snapshot_count,
                })
                .to_string())
            }
            "session_destroy" => {
                let session_id = self.require_session_id(sid)?;
                self.rlm.destroy_session(&session_id).await.map_err(|e| {
                    ToolError::ExecutionFailed {
                        tool: "sandbox".to_string(),
                        message: format!("session_destroy failed: {e}"),
                    }
                })?;
                Ok(json!({
                    "op": "session_destroy",
                    "session_id": session_id.to_string(),
                    "destroyed": true,
                })
                .to_string())
            }
            other => Err(ToolError::InvalidArguments {
                tool: "sandbox".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}
