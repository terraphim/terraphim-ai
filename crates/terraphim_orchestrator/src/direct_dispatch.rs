//! Direct dispatch listener via Unix domain socket.
//!
//! Provides a low-latency dispatch path for `adf-ctl --local trigger --direct`
//! that bypasses the HTTP webhook roundtrip and HMAC verification.  The listener
//! accepts JSON commands on a Unix domain socket and forwards them to the
//! orchestrator's event loop as `WebhookDispatch::SpawnAgent` events.

use std::collections::HashSet;
use std::path::PathBuf;

use tokio::net::UnixListener;
use tracing::{error, info};

use crate::agent_runner::SyntheticEvent;
use crate::webhook::WebhookDispatch;

const MAX_COMMAND_SIZE: u64 = 8192;

/// JSON command received from adf-ctl over the Unix domain socket.
#[derive(Debug, serde::Deserialize)]
pub struct DispatchCommand {
    /// Agent name to spawn (must match a configured agent name).
    pub agent: String,
    /// Optional project hint for project-qualified agent resolution.
    #[serde(default)]
    pub project: Option<String>,
    /// Optional context string appended to the agent mention.
    #[serde(default)]
    pub context: Option<String>,
    /// Optional synthetic event for event-only agents.
    #[serde(default)]
    pub synthetic_event: Option<SyntheticEvent>,
}

/// Index of valid agent names for direct dispatch validation.
///
/// Allows the UDS listener to synchronously reject invalid project-qualified
/// dispatches rather than returning ok and later being dropped by the orchestrator.
#[derive(Debug, Clone)]
pub struct DirectDispatchAgentIndex {
    bare_names: HashSet<String>,
    qualified_names: HashSet<(String, String)>,
}

impl DirectDispatchAgentIndex {
    /// Build an index from a slice of agent definitions.
    pub fn from_agents(agents: &[crate::config::AgentDefinition]) -> Self {
        let bare_names: HashSet<String> = agents
            .iter()
            .filter(|a| a.project.is_none())
            .map(|a| a.name.clone())
            .collect();
        let qualified_names: HashSet<(String, String)> = agents
            .iter()
            .filter_map(|a| a.project.clone().map(|p| (p, a.name.clone())))
            .collect();
        Self {
            bare_names,
            qualified_names,
        }
    }

    /// Return `true` if `(project, agent)` names a known configured agent.
    pub fn is_valid(&self, project: Option<&str>, agent: &str) -> bool {
        match project {
            Some(p) => self
                .qualified_names
                .contains(&(p.to_string(), agent.to_string())),
            None => self.bare_names.contains(agent),
        }
    }
}

/// JSON response written back to adf-ctl.
#[derive(Debug, serde::Serialize)]
pub struct DispatchResponse {
    /// `"ok"` on success or `"error"` on failure.
    pub status: String,
    /// Human-readable error description; omitted from JSON when `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl DispatchResponse {
    /// Construct a successful response with no message.
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
        }
    }

    /// Construct an error response with the given human-readable message.
    pub fn error(msg: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.to_string()),
        }
    }
}

/// Remove a stale socket file at `socket_path` if it exists.
///
/// Returns an error if the path exists but is not a socket, leaving it
/// untouched to avoid accidental data loss.
#[cfg(unix)]
fn remove_stale_socket_if_present(socket_path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::FileTypeExt;
    match std::fs::symlink_metadata(socket_path) {
        Ok(metadata) if metadata.file_type().is_socket() => std::fs::remove_file(socket_path),
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "direct dispatch path exists and is not a socket",
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Start the Unix domain socket listener for direct dispatch.
///
/// Spawns a Tokio task that:
/// 1. Removes any stale socket file at `socket_path`.
/// 2. Binds and listens on the socket path.
/// 3. For each connection: reads a JSON [`DispatchCommand`], validates the
///    agent name against `agent_index`, and forwards a
///    `WebhookDispatch::SpawnAgent` to `dispatch_tx`.
/// 4. Writes a JSON [`DispatchResponse`] back to the caller.
pub fn start_direct_dispatch_listener(
    socket_path: PathBuf,
    dispatch_tx: tokio::sync::mpsc::Sender<WebhookDispatch>,
    agent_index: DirectDispatchAgentIndex,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if let Err(e) = remove_stale_socket_if_present(&socket_path) {
            error!(
                path = %socket_path.display(),
                error = %e,
                "failed to prepare direct dispatch socket path"
            );
            return;
        }

        let listener = match UnixListener::bind(&socket_path) {
            Ok(l) => l,
            Err(e) => {
                error!(
                    path = %socket_path.display(),
                    error = %e,
                    "failed to bind direct dispatch socket"
                );
                return;
            }
        };

        // Apply restrictive permissions: owner read/write only (0600).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) =
                std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
            {
                tracing::warn!(
                    path = %socket_path.display(),
                    error = %e,
                    "could not set permissions on direct dispatch socket"
                );
            }
        }

        info!(
            path = %socket_path.display(),
            "direct dispatch socket listening"
        );

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let dispatch_tx = dispatch_tx.clone();
                    let agent_index = agent_index.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, &dispatch_tx, &agent_index).await
                        {
                            error!(error = %e, "direct dispatch connection error");
                        }
                    });
                }
                Err(e) => {
                    error!(error = %e, "failed to accept direct dispatch connection");
                }
            }
        }
    })
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    dispatch_tx: &tokio::sync::mpsc::Sender<WebhookDispatch>,
    agent_index: &DirectDispatchAgentIndex,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt};

    let (read_half, write_half) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(read_half.take(MAX_COMMAND_SIZE));
    let mut line = String::new();

    let bytes_read = reader.read_line(&mut line).await?;
    if bytes_read == 0 {
        return Ok(());
    }

    let cmd: DispatchCommand = match serde_json::from_str(line.trim()) {
        Ok(cmd) => cmd,
        Err(e) => {
            let response = DispatchResponse::error(&format!("invalid JSON: {}", e));
            write_response(write_half, response).await?;
            return Ok(());
        }
    };

    if !agent_index.is_valid(cmd.project.as_deref(), &cmd.agent) {
        let msg = match cmd.project.as_deref() {
            Some(p) => format!("unknown project-qualified agent: {}/{}", p, cmd.agent),
            None => format!("unknown agent: {}", cmd.agent),
        };
        let response = DispatchResponse::error(&msg);
        write_response(write_half, response).await?;
        return Ok(());
    }

    let dispatch = WebhookDispatch::SpawnAgent {
        agent_name: cmd.agent.clone(),
        detected_project: cmd.project.clone(),
        issue_number: 0,
        comment_id: 0,
        context: cmd.context.unwrap_or_default(),
        synthetic_event: cmd.synthetic_event.clone(),
    };

    if dispatch_tx.send(dispatch).await.is_err() {
        let response = DispatchResponse::error("orchestrator channel closed");
        write_response(write_half, response).await?;
        return Ok(());
    }

    let response = DispatchResponse::ok();
    write_response(write_half, response).await?;
    Ok(())
}

async fn write_response(
    mut writer: tokio::net::unix::OwnedWriteHalf,
    response: DispatchResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    let json = serde_json::to_string(&response)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixStream;
    use tokio::sync::mpsc;

    #[cfg(unix)]
    async fn wait_for_socket(path: &std::path::Path) {
        use std::os::unix::fs::FileTypeExt;
        for _ in 0..50 {
            if path.exists()
                && path
                    .metadata()
                    .map(|m| m.file_type().is_socket())
                    .unwrap_or(false)
            {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("socket was not created at {}", path.display());
    }

    #[cfg(unix)]
    async fn send_command(path: &std::path::Path, json: &str) -> serde_json::Value {
        let stream =
            tokio::time::timeout(std::time::Duration::from_secs(2), UnixStream::connect(path))
                .await
                .expect("socket connect timed out")
                .expect("socket connect failed");

        let mut stream = tokio::io::BufReader::new(stream);
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let stream = stream.get_mut();
            stream
                .write_all(json.as_bytes())
                .await
                .expect("write failed");
            stream.write_all(b"\n").await.expect("newline failed");
            let mut response = String::new();
            stream
                .read_to_string(&mut response)
                .await
                .expect("read failed");
            serde_json::from_str(response.trim()).expect("invalid JSON response")
        })
        .await
        .expect("send_command timed out")
    }

    #[test]
    fn test_dispatch_command_deserialize() {
        let json = r#"{"agent": "meta-learning", "context": "test context"}"#;
        let cmd: DispatchCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.agent, "meta-learning");
        assert_eq!(cmd.context, Some("test context".to_string()));
    }

    #[test]
    fn test_dispatch_command_deserialize_no_context() {
        let json = r#"{"agent": "meta-learning"}"#;
        let cmd: DispatchCommand = serde_json::from_str(json).unwrap();
        assert_eq!(cmd.agent, "meta-learning");
        assert_eq!(cmd.context, None);
    }

    #[test]
    fn test_dispatch_response_ok() {
        let response = DispatchResponse::ok();
        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, r#"{"status":"ok"}"#);
    }

    #[test]
    fn test_dispatch_response_error() {
        let response = DispatchResponse::error("unknown agent: foo");
        let json = serde_json::to_string(&response).unwrap();
        assert_eq!(json, r#"{"status":"error","message":"unknown agent: foo"}"#);
    }

    #[cfg(unix)]
    #[test]
    fn test_remove_stale_socket_rejects_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-socket.txt");
        std::fs::write(&path, "hello").unwrap();
        let result = super::remove_stale_socket_if_present(&path);
        assert!(result.is_err(), "regular file should not be removed");
        assert!(
            path.exists(),
            "regular file must still exist after rejected removal"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_remove_stale_socket_removes_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("does-not-exist");
        let result = super::remove_stale_socket_if_present(&path);
        assert!(result.is_ok(), "nonexistent path should be fine");
    }

    #[cfg(unix)]
    #[test]
    fn test_dispatch_command_agent_validation_logic() {
        use std::collections::HashSet;
        let valid_agents: HashSet<String> =
            ["meta-learning".to_string(), "sentinel".to_string()].into();

        let cmd_valid: DispatchCommand =
            serde_json::from_str(r#"{"agent":"meta-learning","context":"test"}"#).unwrap();
        assert!(
            valid_agents.contains(&cmd_valid.agent),
            "meta-learning should be valid"
        );

        let cmd_unknown: DispatchCommand =
            serde_json::from_str(r#"{"agent":"unknown-agent","context":""}"#).unwrap();
        assert!(
            !valid_agents.contains(&cmd_unknown.agent),
            "unknown-agent should be rejected"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_direct_dispatch_socket_valid_agent_round_trip() {
        use std::collections::HashSet;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("adf.sock");
        let (tx, mut rx) = mpsc::channel::<WebhookDispatch>(1);
        let bare_names: HashSet<String> = ["meta-learning".to_string()].into_iter().collect();
        let agent_index = super::DirectDispatchAgentIndex {
            bare_names,
            qualified_names: HashSet::new(),
        };

        let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_index);
        wait_for_socket(&socket_path).await;

        let response = send_command(
            &socket_path,
            r#"{"agent":"meta-learning","context":"test"}"#,
        )
        .await;
        assert_eq!(response["status"], "ok", "expected ok response");

        let dispatch = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("dispatch receive timed out")
            .expect("dispatch channel closed");

        match dispatch {
            WebhookDispatch::SpawnAgent {
                agent_name,
                context,
                issue_number,
                comment_id,
                ..
            } => {
                assert_eq!(agent_name, "meta-learning");
                assert_eq!(context, "test");
                assert_eq!(issue_number, 0);
                assert_eq!(comment_id, 0);
            }
            other => panic!("unexpected dispatch: {other:?}"),
        }

        handle.abort();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_direct_dispatch_socket_unknown_agent_returns_error() {
        use std::collections::HashSet;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("adf.sock");
        let (tx, mut rx) = mpsc::channel::<WebhookDispatch>(1);
        let bare_names: HashSet<String> = ["meta-learning".to_string()].into_iter().collect();
        let agent_index = super::DirectDispatchAgentIndex {
            bare_names,
            qualified_names: HashSet::new(),
        };

        let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_index);
        wait_for_socket(&socket_path).await;

        let response = send_command(&socket_path, r#"{"agent":"unknown-agent"}"#).await;
        assert_eq!(
            response["status"], "error",
            "expected error response for unknown agent"
        );
        assert!(
            response["message"]
                .as_str()
                .unwrap()
                .contains("unknown agent"),
            "error message should mention unknown agent"
        );
        assert!(
            rx.try_recv().is_err(),
            "unknown agent must not emit a dispatch"
        );

        handle.abort();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_direct_dispatch_socket_project_qualified_agent_round_trip() {
        use std::collections::HashSet;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("adf.sock");
        let (tx, mut rx) = mpsc::channel::<WebhookDispatch>(1);
        let qualified_names: HashSet<(String, String)> =
            [("terraphim-ai".to_string(), "build-runner".to_string())]
                .into_iter()
                .collect();
        let agent_index = super::DirectDispatchAgentIndex {
            bare_names: HashSet::new(),
            qualified_names,
        };

        let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_index);
        wait_for_socket(&socket_path).await;

        let response = send_command(
            &socket_path,
            r#"{"project":"terraphim-ai","agent":"build-runner","context":"test"}"#,
        )
        .await;
        assert_eq!(response["status"], "ok", "expected ok response");

        let dispatch = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
            .await
            .expect("dispatch receive timed out")
            .expect("dispatch channel closed");

        let WebhookDispatch::SpawnAgent {
            agent_name,
            detected_project,
            context,
            ..
        } = dispatch
        else {
            unreachable!("direct dispatch emits only SpawnAgent variants");
        };
        assert_eq!(agent_name, "build-runner");
        assert_eq!(detected_project.as_deref(), Some("terraphim-ai"));
        assert_eq!(context, "test");

        handle.abort();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_direct_dispatch_socket_bad_project_returns_error() {
        use std::collections::HashSet;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("adf.sock");
        let (tx, mut rx) = mpsc::channel::<WebhookDispatch>(1);
        let qualified_names: HashSet<(String, String)> =
            [("terraphim-ai".to_string(), "build-runner".to_string())]
                .into_iter()
                .collect();
        let agent_index = super::DirectDispatchAgentIndex {
            bare_names: HashSet::new(),
            qualified_names,
        };

        let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_index);
        wait_for_socket(&socket_path).await;

        let response = send_command(
            &socket_path,
            r#"{"project":"bad-project","agent":"build-runner"}"#,
        )
        .await;
        assert_eq!(
            response["status"], "error",
            "expected error response for bad project"
        );
        assert!(
            response["message"]
                .as_str()
                .unwrap()
                .contains("unknown project-qualified agent: bad-project/build-runner"),
            "error message should mention project-qualified agent"
        );
        assert!(
            rx.try_recv().is_err(),
            "bad project-qualified agent must not emit a dispatch"
        );

        handle.abort();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_direct_dispatch_rejects_oversized_command() {
        use std::collections::HashSet;
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("adf.sock");
        let (tx, _rx) = mpsc::channel::<WebhookDispatch>(1);
        let bare_names: HashSet<String> = ["meta-learning".to_string()].into_iter().collect();
        let agent_index = super::DirectDispatchAgentIndex {
            bare_names,
            qualified_names: HashSet::new(),
        };

        let handle = start_direct_dispatch_listener(socket_path.clone(), tx, agent_index);
        wait_for_socket(&socket_path).await;

        let oversized = "x".repeat(16384);
        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            tokio::net::UnixStream::connect(&socket_path),
        )
        .await
        .expect("connect timed out")
        .expect("connect failed");

        use tokio::io::AsyncWriteExt;
        let (_, mut write_half) = stream.into_split();
        let _ = write_half.write_all(oversized.as_bytes()).await;
        drop(write_half);

        tokio::task::yield_now().await;

        let response = send_command(
            &socket_path,
            r#"{"agent":"meta-learning","context":"after-oversize"}"#,
        )
        .await;
        assert_eq!(
            response["status"], "ok",
            "listener must survive oversized input"
        );

        handle.abort();
    }

    #[test]
    fn test_direct_dispatch_agent_index_bare_agent() {
        let agents = vec![crate::config::AgentDefinition {
            name: "meta-learning".to_string(),
            layer: crate::config::AgentLayer::Growth,
            cli_tool: "claude".to_string(),
            task: "do stuff".to_string(),
            schedule: None,
            model: None,
            default_tier: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,
            gitea_issue: None,
            event_only: false,
            project: None,
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,
        }];
        let index = super::DirectDispatchAgentIndex::from_agents(&agents);
        assert!(index.is_valid(None, "meta-learning"));
        assert!(!index.is_valid(None, "unknown-agent"));
    }

    #[test]
    fn test_direct_dispatch_agent_index_qualified_agent() {
        let agents = vec![crate::config::AgentDefinition {
            name: "build-runner".to_string(),
            layer: crate::config::AgentLayer::Core,
            cli_tool: "claude".to_string(),
            task: "run builds".to_string(),
            schedule: None,
            model: None,
            default_tier: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,
            gitea_issue: None,
            event_only: false,
            project: Some("terraphim-ai".to_string()),
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,
        }];
        let index = super::DirectDispatchAgentIndex::from_agents(&agents);
        assert!(index.is_valid(Some("terraphim-ai"), "build-runner"));
        assert!(!index.is_valid(Some("terraphim-ai"), "unknown-agent"));
        assert!(!index.is_valid(Some("other-project"), "build-runner"));
        assert!(!index.is_valid(None, "build-runner"));
    }
}
