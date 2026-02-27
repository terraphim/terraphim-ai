use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

fn resolve_cli_command(agent_type: &str) -> Option<&'static str> {
    match agent_type.trim().to_ascii_lowercase().as_str() {
        "codex" => Some("codex"),
        "opencode" => Some("opencode"),
        "claude" | "claude-code" => Some("claude"),
        "echo" => Some("echo"),
        _ => None,
    }
}

/// Spawn external agent CLIs as a baseline for issue #560 integration.
pub struct AgentSpawnTool {
    default_workdir: PathBuf,
}

impl AgentSpawnTool {
    pub fn new(default_workdir: PathBuf) -> Self {
        Self { default_workdir }
    }
}

#[async_trait]
impl Tool for AgentSpawnTool {
    fn name(&self) -> &str {
        "agent_spawn"
    }

    fn description(&self) -> &str {
        "Spawn an external agent process (baseline for terraphim_spawner integration)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "agent_type": {
                    "type": "string",
                    "description": "Agent runtime type (codex, opencode, claude-code)"
                },
                "task": {
                    "type": "string",
                    "description": "Task prompt passed to the spawned agent"
                },
                "working_directory": {
                    "type": "string",
                    "description": "Optional working directory override"
                },
                "wait_seconds": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "How long to wait for completion/output (default: 20)"
                },
                "detach": {
                    "type": "boolean",
                    "description": "Keep process running and return immediately"
                }
            },
            "required": ["agent_type", "task"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let agent_type = args
            .get("agent_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'agent_type' parameter".to_string(),
            })?
            .trim()
            .to_string();

        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'task' parameter".to_string(),
            })?
            .to_string();

        let cli_command =
            resolve_cli_command(&agent_type).ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!(
                    "Unsupported agent_type '{}'. Supported: codex, opencode, claude-code",
                    agent_type
                ),
            })?;

        let working_directory = args
            .get("working_directory")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_workdir.clone());

        if !working_directory.exists() {
            return Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!(
                    "Working directory does not exist: {}",
                    working_directory.display()
                ),
            });
        }

        // Validate runtime command availability before attempting spawn.
        let which_output = tokio::process::Command::new("which")
            .arg(cli_command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to validate command availability: {}", e),
            })?;

        if !which_output.status.success() {
            return Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Agent command '{}' was not found on PATH", cli_command),
            });
        }

        let wait_seconds = args
            .get("wait_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(20)
            .min(300);
        let detach = args
            .get("detach")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let mut command = tokio::process::Command::new(cli_command);
        command
            .current_dir(working_directory)
            .arg(&task)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());

        let mut child = command.spawn().map_err(|e| ToolError::ExecutionFailed {
            tool: self.name().to_string(),
            message: format!("Failed to spawn agent process: {}", e),
        })?;

        let process_id = child.id();
        if detach {
            return Ok(serde_json::json!({
                "status": "spawned",
                "agent_type": agent_type,
                "process_id": process_id,
                "detached": true
            })
            .to_string());
        }

        let timeout = Duration::from_secs(wait_seconds.max(1));
        let wait_result = tokio::time::timeout(timeout, child.wait()).await;

        match wait_result {
            Ok(Ok(status)) => Ok(serde_json::json!({
                "status": "completed",
                "agent_type": agent_type,
                "process_id": process_id,
                "exit_code": status.code(),
                "output_preview": []
            })
            .to_string()),
            Ok(Err(e)) => Err(ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Agent process failed: {}", e),
            }),
            Err(_) => {
                let _ = child.kill().await;
                Err(ToolError::Timeout {
                    tool: self.name().to_string(),
                    seconds: wait_seconds.max(1),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_agent_spawn_rejects_unknown_agent_type() {
        let temp_dir = TempDir::new().unwrap();
        let tool = AgentSpawnTool::new(temp_dir.path().to_path_buf());

        let err = tool
            .execute(serde_json::json!({
                "agent_type": "unknown",
                "task": "hello"
            }))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::InvalidArguments { .. }));
    }

    #[tokio::test]
    async fn test_agent_spawn_echo_baseline() {
        let temp_dir = TempDir::new().unwrap();
        let tool = AgentSpawnTool::new(temp_dir.path().to_path_buf());

        let output = tool
            .execute(serde_json::json!({
                "agent_type": "echo",
                "task": "spawn baseline",
                "wait_seconds": 1
            }))
            .await
            .unwrap();

        let payload: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(payload["status"], "completed");
        assert!(payload["process_id"].is_number() || payload["process_id"].is_null());
    }
}
