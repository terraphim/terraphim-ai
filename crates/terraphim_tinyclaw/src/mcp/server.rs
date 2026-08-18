//! MCP server exposing TinyClaw's conversations, messages, and events as MCP tools.
//!
//! Wave 2 of the Hermes parity arc (epic #3160). Matches Hermes' `mcp_serve.py`
//! 9-tool bridge surface (pinned commit `846b14ab`).

use super::tools::*;
use crate::agent::evo_apply::{self, EvolutionApplyOutcome};
use crate::bus::{MessageBus, OutboundMessage};
use crate::commands::CommandRegistry;
use crate::session::{MessageRole, SessionManager};
use crate::tools::approval;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ServerCapabilities, ServerInfo};
use rmcp::{ServerHandler, ServiceExt};
use std::sync::Arc;
use terraphim_engine_events::{Disposition, EvolutionApprove};
use tokio::sync::Mutex;

/// MCP server for TinyClaw channel bridge.
#[derive(Clone)]
pub struct TinyClawMcpServer {
    sessions: Arc<Mutex<SessionManager>>,
    bus: Arc<MessageBus>,
    commands: Arc<Mutex<CommandRegistry>>,
    workspace: std::path::PathBuf,
    tool_router: ToolRouter<Self>,
}

impl TinyClawMcpServer {
    /// Borrow the shared `Arc<Mutex<CommandRegistry>>`. Used by tests
    /// and by the in-process composition in `run_gateway_mode` to verify
    /// the agent loop and the MCP server apply evolution-authored
    /// behaviour commands to the same in-memory registry.
    pub fn commands_arc(&self) -> Arc<Mutex<CommandRegistry>> {
        Arc::clone(&self.commands)
    }

    /// Create a new MCP server.
    #[deprecated(
        since = "0.21.4-r12",
        note = "P1#4 fix: callers must construct the server with explicit commands/workspace (use `with_commands`).
                The private-fallback `new()` cannot safely share state across processes."
    )]
    pub fn new(sessions: Arc<Mutex<SessionManager>>, bus: Arc<MessageBus>) -> Self {
        Self::with_commands(
            sessions,
            bus,
            Arc::new(Mutex::new(CommandRegistry::with_defaults())),
            std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
        )
    }

    /// Create a new MCP server with an explicit command registry/workspace.
    pub fn with_commands(
        sessions: Arc<Mutex<SessionManager>>,
        bus: Arc<MessageBus>,
        commands: Arc<Mutex<CommandRegistry>>,
        workspace: std::path::PathBuf,
    ) -> Self {
        Self {
            sessions,
            bus,
            commands,
            workspace,
            tool_router: Self::tool_router(),
        }
    }

    /// Convert a session key to a conversation summary.
    fn session_to_summary(
        &self,
        key: &str,
        session: &crate::session::Session,
    ) -> ConversationSummary {
        let channel = key.split(':').next().unwrap_or("unknown").to_string();
        let display_name = session.metadata.get("display_name").cloned();
        let last_message_at = session.messages.last().map(|m| m.timestamp.to_rfc3339());
        ConversationSummary {
            id: key.to_string(),
            channel,
            display_name,
            last_message_at,
            message_count: session.messages.len(),
        }
    }

    /// Convert a ChatMessage to a ConversationMessage.
    fn chat_to_conversation(msg: &crate::session::ChatMessage) -> ConversationMessage {
        ConversationMessage {
            id: uuid::Uuid::new_v4().to_string(),
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                MessageRole::System => "system".to_string(),
                MessageRole::Tool => "tool".to_string(),
            },
            content: msg.content.clone(),
            timestamp: msg.timestamp.to_rfc3339(),
        }
    }
}

#[rmcp::tool_router(router = tool_router)]
impl TinyClawMcpServer {
    /// List conversations across platforms.
    #[rmcp::tool(description = "List conversations across platforms")]
    pub async fn conversations_list(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let keys = sessions
            .list_sessions()
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))?;

        let mut summaries = Vec::new();
        for key in keys {
            if let Some(session) = sessions.get(&key) {
                summaries.push(self.session_to_summary(&key, session));
            }
        }

        // Hermes contract: wrap in {"count": N, "conversations": [...]}
        let body = serde_json::json!({
            "count": summaries.len(),
            "conversations": summaries,
        });
        Ok(json_result(&body))
    }

    /// Get a single conversation by ID.
    #[rmcp::tool(description = "Get a single conversation by ID")]
    pub async fn conversation_get(
        &self,
        params: Parameters<ConversationGetParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let session = match sessions.get(&params.0.conversation_id) {
            Some(s) => s,
            None => {
                // Hermes contract: missing session returns error JSON, not Err
                let body = serde_json::json!({
                    "error": format!("Conversation not found: {}", params.0.conversation_id),
                });
                return Ok(json_result(&body));
            }
        };

        let messages: Vec<ConversationMessage> = session
            .messages
            .iter()
            .map(Self::chat_to_conversation)
            .collect();

        let summary = self.session_to_summary(&params.0.conversation_id, session);
        let body = serde_json::json!({
            "session_key": params.0.conversation_id,
            "messages": messages,
            "summary": summary,
        });
        Ok(json_result(&body))
    }

    /// Read message history for a conversation.
    #[rmcp::tool(description = "Read message history for a conversation")]
    pub async fn messages_read(
        &self,
        params: Parameters<MessagesReadParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let sessions = self.sessions.lock().await;
        let session = sessions.get(&params.0.conversation_id).ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                format!("conversation not found: {}", params.0.conversation_id),
                None,
            )
        })?;

        let limit = params.0.limit.unwrap_or(50);
        let start = session.messages.len().saturating_sub(limit);
        let messages: Vec<ConversationMessage> = session.messages[start..]
            .iter()
            .map(Self::chat_to_conversation)
            .collect();

        Ok(json_result(&messages))
    }

    /// Fetch attachments for a conversation.
    #[rmcp::tool(description = "Fetch attachments for a conversation")]
    pub async fn attachments_fetch(
        &self,
        params: Parameters<ConversationGetParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        // TinyClaw stores media URLs in InboundMessage.media, not in the session.
        // For Wave 2, return an empty list — attachments are ephemeral in the bus.
        let _ = params;
        Ok(json_result(&Vec::<String>::new()))
    }

    /// Poll for live events.
    #[rmcp::tool(description = "Poll for live events")]
    pub async fn events_poll(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let mut rx = self.bus.inbound_rx.lock().await;
        let events: Vec<serde_json::Value> = match rx.try_recv() {
            Ok(msg) => {
                let event = serde_json::json!({
                    "type": "message",
                    "channel": msg.channel,
                    "chat_id": msg.chat_id,
                    "sender_id": msg.sender_id,
                    "content": msg.content,
                });
                vec![event]
            }
            Err(_) => Vec::new(),
        };
        let body = serde_json::json!({
            "count": events.len(),
            "events": events,
        });
        Ok(json_result(&body))
    }

    /// Wait for live events (long-poll).
    #[rmcp::tool(description = "Wait for live events (long-poll)")]
    pub async fn events_wait(
        &self,
        params: Parameters<EventsWaitParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let timeout_ms = params.0.timeout_ms.unwrap_or(30_000);
        let timeout = std::time::Duration::from_millis(timeout_ms);

        let mut rx = self.bus.inbound_rx.lock().await;
        match tokio::time::timeout(timeout, rx.recv()).await {
            Ok(Some(msg)) => {
                let event = serde_json::json!({
                    "type": "message",
                    "channel": msg.channel,
                    "chat_id": msg.chat_id,
                    "sender_id": msg.sender_id,
                    "content": msg.content,
                });
                Ok(json_result(&vec![event]))
            }
            Ok(None) => Ok(json_result(&Vec::<serde_json::Value>::new())),
            Err(_) => Ok(json_result(&Vec::<serde_json::Value>::new())),
        }
    }

    /// Send a message to a conversation.
    #[rmcp::tool(description = "Send a message to a conversation")]
    pub async fn messages_send(
        &self,
        params: Parameters<MessagesSendParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let conversation_id = &params.0.conversation_id;
        let parts: Vec<&str> = conversation_id.split(':').collect();
        if parts.len() < 2 {
            // Hermes contract: invalid format returns error JSON, not Err
            let body = serde_json::json!({
                "status": "error",
                "error": format!(
                    "invalid conversation_id format: expected 'channel:chat_id', got '{}'",
                    conversation_id
                ),
            });
            return Ok(json_result(&body));
        }

        let channel = parts[0].to_string();
        let chat_id = parts[1..].join(":");

        let msg = OutboundMessage::new(channel, chat_id, params.0.content.clone());
        match self.bus.outbound_sender().send(msg).await {
            Ok(()) => {
                let body = serde_json::json!({
                    "status": "sent",
                    "conversation_id": conversation_id,
                });
                Ok(json_result(&body))
            }
            Err(e) => {
                let body = serde_json::json!({
                    "status": "error",
                    "error": e.to_string(),
                });
                Ok(json_result(&body))
            }
        }
    }

    /// List open approval requests.
    ///
    /// A request id is "open" iff the *latest* record for that id in the
    /// JSONL queue has status [`approval::PendingStatus::Pending`]. This
    /// last-wins-per-id read closes the ghost-pending hole that the round-10
    /// review surfaced (round-11 fix): a resolved record stays in the
    /// append-only JSONL for audit, but does not re-appear as an open
    /// request. The all-records read path
    /// (`approval::read_pending_evolution`) is retained for the audit
    /// log; operator surfaces use
    /// [`approval::latest_pending_evolution`].
    #[rmcp::tool(description = "List open approval requests")]
    pub async fn permissions_list_open(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        let permissions: Vec<serde_json::Value> =
            approval::latest_pending_evolution(&self.workspace)
                .into_values()
                .filter(|r| r.status == approval::PendingStatus::Pending)
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "tool_name": "evolution.apply",
                        "arguments": {
                            "signature": r.proposal.signature,
                            "target_kind": r.proposal.target_kind,
                            "target_ref": r.proposal.target_ref,
                        },
                        "requested_at": r.requested_at,
                        "proposal": r.proposal,
                    })
                })
                .collect();
        let body = serde_json::json!({
            "count": permissions.len(),
            "permissions": permissions,
        });
        Ok(json_result(&body))
    }

    /// Respond to an approval request.
    ///
    /// The operator's `disposition` (or `approved` boolean for backwards
    /// compatibility) is **constructed into an `EvolutionApprove` at
    /// decision time** from the persisted proposal. The matching
    /// `apply_approved_proposal` path then runs the audit and apply
    /// consistently — including the `Reject|RejectAlways` paths that
    /// write `evo.reject` audit records (`#3229` P1#3, r8–r9).
    ///
    /// On any apply failure (transient I/O, validation mismatch), the
    /// pending request is **requeued** by appending a fresh `Pending`
    /// record (the original is replaced via a log-and-replay marker),
    /// so a failed apply does not consume the operator's approval.
    #[rmcp::tool(description = "Respond to an approval request")]
    pub async fn permissions_respond(
        &self,
        params: Parameters<PermissionsRespondParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let request_id = params.0.request_id;

        // Resolve disposition: prefer explicit `disposition`, fall back
        // to the boolean `approved` (backwards compat).
        let disposition = match (params.0.disposition, params.0.approved) {
            (Some(d), _) => d.to_engine_disposition(),
            (None, approved) => {
                if approved {
                    Disposition::AllowOnce
                } else {
                    Disposition::Reject
                }
            }
        };

        // Find the pending record (cross-process: read from the JSONL
        // queue with last-wins-per-id semantics). If the latest record for
        // this id is not Pending (i.e. it has been resolved, or never
        // existed), the request is treated as not-found: ghost-pending is
        // not allowed (round-11 fix).
        let pending = approval::latest_pending_evolution(&self.workspace)
            .get(&request_id)
            .filter(|r| r.status == approval::PendingStatus::Pending)
            .cloned();
        let Some(pending) = pending else {
            let body = serde_json::json!({
                "status": "error",
                "request_id": request_id,
                "error": format!("approval request not found: {request_id}"),
            });
            return Ok(json_result(&body));
        };

        // Construct the approval at decision time from the proposal +
        // the operator's disposition (P1#2 fix: agent-loop self-mint is
        // removed; the human gate now supplies the disposition).
        let approval = EvolutionApprove {
            signature: pending.proposal.signature.clone(),
            target_kind: pending.proposal.target_kind,
            target_ref: pending.proposal.target_ref.clone(),
            trust_level: pending.proposal.trust_level,
            disposition,
        };

        // Persist the resolution record (audit-trail of the operator's
        // decision into the pending queue, before doing the apply).
        if let Err(e) = approval::append_resolved_evolution(&self.workspace, &request_id, &approval)
        {
            log::warn!("failed to persist resolved evolution {}: {}", request_id, e);
        }

        let mut commands = self.commands.lock().await;
        match evo_apply::apply_approved_proposal(
            &self.workspace,
            &mut commands,
            &pending.proposal,
            &approval,
        ) {
            Ok(EvolutionApplyOutcome::Applied { audit_ref, .. }) => Ok(json_result(
                &serde_json::json!({"status": "applied", "request_id": request_id, "audit_ref": audit_ref}),
            )),
            Ok(EvolutionApplyOutcome::Deferred { audit_ref, reason }) => Ok(json_result(
                &serde_json::json!({"status": "deferred", "request_id": request_id, "audit_ref": audit_ref, "reason": reason}),
            )),
            Ok(EvolutionApplyOutcome::Rejected { audit_ref, reason }) => Ok(json_result(
                &serde_json::json!({"status": "rejected", "request_id": request_id, "audit_ref": audit_ref, "reason": reason}),
            )),
            Err(e) => {
                // Apply failed (transient or validation mismatch).
                // Requeue the pending request so the operator can retry
                // without re-proposing. The original record remains in the
                // JSONL for audit (file is append-only).
                let e_str = e.to_string();
                if let Err(requeue_err) = approval::submit_pending_evolution(
                    &self.workspace,
                    &request_id,
                    &pending.proposal,
                ) {
                    log::warn!(
                        "failed to requeue {} after apply error: {}",
                        request_id,
                        requeue_err
                    );
                }
                Ok(json_result(&serde_json::json!({
                    "status": "error",
                    "request_id": request_id,
                    "error": e_str,
                    "requeued": true,
                })))
            }
        }
    }

    /// List connected channels.
    #[rmcp::tool(description = "List connected channels")]
    pub async fn channels_list(&self) -> Result<CallToolResult, rmcp::ErrorData> {
        // TinyClaw channels are configured at startup; we can't enumerate them
        // from the bus alone. Return the channels we know about from config.
        // For Wave 2, return a static list based on feature flags.
        let channels = vec!["cli"];
        let body = serde_json::json!({
            "count": channels.len(),
            "channels": channels,
        });
        Ok(json_result(&body))
    }
}

#[rmcp::tool_handler(router = self.tool_router)]
impl ServerHandler for TinyClawMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("TinyClaw MCP channel bridge".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio, sharing the agent's
/// `Arc<Mutex<CommandRegistry>>` and configured workspace.
///
/// **P1#4 fix**: the previous `serve_mcp_stdio(sessions, bus)` signature
/// constructed a private `CommandRegistry::with_defaults()` and used
/// `std::env::current_dir()` as the workspace, which the executing agent
/// loop never reads in production (the agent loop owns a different
/// `Arc<Mutex<CommandRegistry>>` in a different OS process). Evolution
/// applications written via the apply path landed in the wrong registry
/// and at the wrong filesystem location. The new signature requires the
/// caller to thread both the shared registry and the workspace
/// directory explicitly.
///
/// **Run in-process alongside the agent loop** (option A, fixes the
/// r9 cross-process topology finding): `run_gateway_mode` constructs a
/// single `TinyClawMcpServer::with_commands(...)` using the agent loop's
/// `commands_arc()` and `workspace()`, then spawns the MCP server in
/// the same process via `tokio::spawn`.
///
/// The `Commands::Mcp` (standalone) mode and `serve_mcp_stdio_no_sharing`
/// below are retained as escape hatches for deployments that intentionally
/// want a clean MCP-only process; in those deployments evolution-apply
/// is unsupported.
pub async fn serve_mcp_stdio(
    sessions: Arc<Mutex<SessionManager>>,
    bus: Arc<MessageBus>,
    commands: Arc<Mutex<CommandRegistry>>,
    workspace: std::path::PathBuf,
) -> Result<(), super::McpError> {
    use rmcp::transport::io::stdio;

    let server = TinyClawMcpServer::with_commands(sessions, bus, commands, workspace);
    let service = server
        .serve(stdio())
        .await
        .map_err(|e| super::McpError::Server(e.to_string()))?;

    service
        .waiting()
        .await
        .map_err(|e| super::McpError::Server(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionManager;
    use tempfile::TempDir;
    use terraphim_engine_events::{EvolutionPropose, TargetKind, TrustLevel};

    fn make_server() -> (TinyClawMcpServer, TempDir) {
        let dir = TempDir::new().unwrap();
        let sessions = Arc::new(Mutex::new(SessionManager::new(dir.path().to_path_buf())));
        let bus = Arc::new(MessageBus::new());
        let commands = Arc::new(Mutex::new(CommandRegistry::new()));
        (
            TinyClawMcpServer::with_commands(sessions, bus, commands, dir.path().to_path_buf()),
            dir,
        )
    }

    #[tokio::test]
    async fn test_conversations_list_empty() {
        // Hermes contract: conversations_list returns {"count": 0, "conversations": []}
        let (server, _dir) = make_server();
        let result = server.conversations_list().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["count"], 0);
        assert!(parsed["conversations"].is_array());
    }

    #[tokio::test]
    async fn test_conversation_get_not_found() {
        // Hermes contract: missing session returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(ConversationGetParams {
            conversation_id: "nonexistent".into(),
        });
        let result = server.conversation_get(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert!(parsed.get("error").is_some());
    }

    #[tokio::test]
    async fn test_messages_send_invalid_format() {
        // Hermes contract: invalid format returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(MessagesSendParams {
            conversation_id: "no-colon".into(),
            content: "hello".into(),
        });
        let result = server.messages_send(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "error");
        assert!(parsed["error"].as_str().unwrap().contains("invalid"));
    }

    #[tokio::test]
    async fn test_permissions_list_open_empty() {
        // Hermes contract: permissions_list_open returns {"count": 0, "permissions": []}
        let (server, _dir) = make_server();
        let result = server.permissions_list_open().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["count"], 0);
        assert!(parsed["permissions"].is_array());
    }

    #[tokio::test]
    async fn test_permissions_respond_not_found() {
        // Hermes contract: unknown request returns error JSON, NOT Err
        let (server, _dir) = make_server();
        let params = Parameters(PermissionsRespondParams {
            request_id: "req-123".into(),
            approved: true,
            disposition: None,
        });
        let result = server.permissions_respond(params).await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["request_id"], "req-123");
    }

    #[tokio::test]
    async fn permissions_respond_applies_pending_evolution_request() {
        let (server, dir) = make_server();
        let proposal = EvolutionPropose {
            signature: "prefer-rg-search".to_string(),
            target_kind: TargetKind::Tool,
            target_ref: Some("prefer-rg-search".to_string()),
            content: "Use rg for repository search.".to_string(),
            trust_level: TrustLevel::L1,
        };
        approval::submit_pending_evolution(dir.path(), "evo:prefer-rg-search", &proposal).unwrap();

        let result = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:prefer-rg-search".into(),
                approved: true,
                disposition: None,
            }))
            .await
            .unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "applied");
        let audit = std::fs::read_to_string(
            dir.path()
                .join(".terraphim")
                .join("evolution")
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("evo.applied"));
    }

    #[tokio::test]
    async fn permissions_respond_rejection_is_audited() {
        let (server, dir) = make_server();
        let proposal = EvolutionPropose {
            signature: "prefer-rg-search".to_string(),
            target_kind: TargetKind::Tool,
            target_ref: Some("prefer-rg-search".to_string()),
            content: "Use rg for repository search.".to_string(),
            trust_level: TrustLevel::L1,
        };
        approval::submit_pending_evolution(dir.path(), "evo:prefer-rg-search", &proposal).unwrap();

        let result = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:prefer-rg-search".into(),
                approved: false,
                disposition: Some(crate::mcp::tools::ApprovalDispositionParam::Reject),
            }))
            .await
            .unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "rejected");
        let audit = std::fs::read_to_string(
            dir.path()
                .join(".terraphim")
                .join("evolution")
                .join("audit.jsonl"),
        )
        .unwrap();
        assert!(audit.contains("evo.reject"));
    }

    #[tokio::test]
    async fn permissions_respond_apply_failure_requeues_pending() {
        let (server, dir) = make_server();
        // A proposal whose identity will fail validation: behaviour at L1.
        let proposal = EvolutionPropose {
            signature: "behaviour-at-l1".to_string(),
            target_kind: TargetKind::Behaviour,
            target_ref: Some("dangerous".to_string()),
            content: "rm -rf /".to_string(),
            trust_level: TrustLevel::L1,
        };
        approval::submit_pending_evolution(dir.path(), "evo:behaviour-at-l1", &proposal).unwrap();

        let result = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:behaviour-at-l1".into(),
                approved: true,
                disposition: None,
            }))
            .await
            .unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["requeued"], true);
        // The pending queue must contain a fresh Pending entry after the
        // requeue, so the operator can retry. (The original entry plus the
        // requeued entry both appear; the most recent entry for this
        // request_id must be Pending, otherwise the operator cannot
        // re-answer the request.)
        let pending = approval::read_pending_evolution(dir.path());
        let latest = pending
            .iter()
            .rev()
            .find(|p| p.id == "evo:behaviour-at-l1")
            .expect("queue must contain the request_id");
        assert_eq!(
            latest.status,
            approval::PendingStatus::Pending,
            "failed apply must requeue the pending request"
        );
    }

    #[tokio::test]
    async fn test_channels_list() {
        // Hermes contract: channels_list returns {"count": N, "channels": [...]}
        let (server, _dir) = make_server();
        let result = server.channels_list().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();

        assert!(parsed["channels"].is_array());
        assert!(
            parsed["channels"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("cli"))
        );
    }

    #[tokio::test]
    async fn permissions_list_open_hides_resolved_requests() {
        // Round-11 fix for the ghost-pending hole: after an operator
        // resolves a request, permissions_list_open must NOT show it as
        // an open request. The raw queue file keeps both records for
        // audit; the operator view uses last-wins-per-id semantics.
        let (server, dir) = make_server();
        let proposal = EvolutionPropose {
            signature: "prefer-rg-search".to_string(),
            target_kind: TargetKind::Tool,
            target_ref: Some("prefer-rg-search".to_string()),
            content: "Use rg for repository search.".to_string(),
            trust_level: TrustLevel::L1,
        };
        approval::submit_pending_evolution(dir.path(), "evo:prefer-rg-search", &proposal).unwrap();

        // Before resolution: 1 open.
        let result = server.permissions_list_open().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["count"], 1, "before resolution: 1 open request");
        assert_eq!(parsed["permissions"][0]["id"], "evo:prefer-rg-search");

        // Resolve (reject).
        let _ = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:prefer-rg-search".into(),
                approved: false,
                disposition: None,
            }))
            .await
            .unwrap();

        // After resolution: 0 open requests, even though the JSONL still
        // has both the original Pending and the appended Resolved record.
        let result = server.permissions_list_open().await.unwrap();
        let text = result.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(
            parsed["count"], 0,
            "after resolution: 0 open requests (ghost-pending fix)"
        );
    }

    #[tokio::test]
    async fn permissions_respond_rejects_duplicate_response() {
        // The same request id resolved twice must NOT double-apply: after
        // resolution, the latest record is Resolved, so the second response
        // returns "approval request not found".
        let (server, dir) = make_server();
        let proposal = EvolutionPropose {
            signature: "prefer-rg-search".to_string(),
            target_kind: TargetKind::Tool,
            target_ref: Some("prefer-rg-search".to_string()),
            content: "Use rg for repository search.".to_string(),
            trust_level: TrustLevel::L1,
        };
        approval::submit_pending_evolution(dir.path(), "evo:prefer-rg-search", &proposal).unwrap();

        // First response: applies.
        let first = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:prefer-rg-search".into(),
                approved: true,
                disposition: None,
            }))
            .await
            .unwrap();
        let text = first.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(parsed["status"], "applied");

        // Second response: not found (latest record is no longer Pending).
        let second = server
            .permissions_respond(Parameters(PermissionsRespondParams {
                request_id: "evo:prefer-rg-search".into(),
                approved: true,
                disposition: None,
            }))
            .await
            .unwrap();
        let text = second.content[0].as_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text.text).unwrap();
        assert_eq!(
            parsed["status"], "error",
            "second response must be error (no double-apply)"
        );
        assert!(
            parsed["error"].as_str().unwrap().contains("not found"),
            "error message must reference not-found, got: {}",
            parsed["error"]
        );
    }
}
