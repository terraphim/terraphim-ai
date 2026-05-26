//! Direct dispatch listener via Unix domain socket.
//!
//! Provides a low-latency dispatch path for `adf-ctl --local trigger --direct`
//! that bypasses the HTTP webhook roundtrip and HMAC verification.  The listener
//! accepts JSON commands on a Unix domain socket and forwards them to the
//! orchestrator's event loop as `WebhookDispatch::SpawnAgent` events.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::webhook::WebhookDispatch;

/// JSON command received from adf-ctl over the Unix domain socket.
#[derive(Debug, serde::Deserialize)]
pub struct DispatchCommand {
    /// Agent name to spawn (must match a configured agent name).
    pub agent: String,
    /// Optional context string appended to the agent mention.
    #[serde(default)]
    pub context: Option<String>,
}

/// JSON response written back to adf-ctl.
#[derive(Debug, serde::Serialize)]
pub struct DispatchResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl DispatchResponse {
    pub fn ok() -> Self {
        Self {
            status: "ok".to_string(),
            message: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.to_string()),
        }
    }
}

/// Start the Unix domain socket listener for direct dispatch.
//
///
///
/// The listener task:
///
/// 1. Removes any stale socket file at `socket_path`.
/// 2. Binds and listens on the socket path.
/// 3. For each incoming connection:
///    a. Reads a single JSON command from the stream.
///    b. Validates the agent name against `agent_names`.
///    c. Sends `WebhookDispatch::SpawnAgent` to `dispatch_tx`.
///    d. Writes a JSON response back to the client.
/// 4. Logs errors and continues accepting connections.
///
/// The socket is cleaned up automatically when the listener task is dropped.
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

pub fn start_direct_dispatch_listener(
    socket_path: PathBuf,
    dispatch_tx: tokio::sync::mpsc::Sender<WebhookDispatch>,
    agent_names: HashSet<String>,
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
                    let agent_names = agent_names.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, &dispatch_tx, &agent_names).await
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
    agent_names: &HashSet<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();

    // Read one JSON line (adf-ctl sends a single JSON object terminated by newline).
    let bytes_read = reader.read_line(&mut line).await?;
    if bytes_read == 0 {
        return Ok(());
    }

    let cmd: DispatchCommand = match serde_json::from_str(line.trim()) {
        Ok(cmd) => cmd,
        Err(e) => {
            let response = DispatchResponse::error(&format!("invalid JSON: {}", e));
            write_response(&mut reader, response).await?;
            return Ok(());
        }
    };

    if !agent_names.contains(&cmd.agent) {
        let response = DispatchResponse::error(&format!("unknown agent: {}", cmd.agent));
        write_response(&mut reader, response).await?;
        return Ok(());
    }

    let dispatch = WebhookDispatch::SpawnAgent {
        agent_name: cmd.agent,
        detected_project: None,
        issue_number: 0,
        comment_id: 0,
        context: cmd.context.unwrap_or_default(),
    };

    if dispatch_tx.send(dispatch).await.is_err() {
        let response = DispatchResponse::error("orchestrator channel closed");
        write_response(&mut reader, response).await?;
        return Ok(());
    }

    let response = DispatchResponse::ok();
    write_response(&mut reader, response).await?;
    Ok(())
}

async fn write_response(
    reader: &mut tokio::io::BufReader<tokio::net::UnixStream>,
    response: DispatchResponse,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    let stream = reader.get_mut();
    let json = serde_json::to_string(&response)?;
    stream.write_all(json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

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
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-socket.txt");
        std::fs::write(&path, "hello").unwrap();
        let result = super::remove_stale_socket_if_present(&path);
        assert!(result.is_err(), "regular file should not be removed");
        assert_eq!(
            path.exists(),
            true,
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
}
