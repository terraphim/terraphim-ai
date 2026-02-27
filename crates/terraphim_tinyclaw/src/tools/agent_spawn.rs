use crate::config::{SpawnerAgentEntry, SpawnerConfig};
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use terraphim_spawner::{AgentHandle, AgentSpawner, OutputEvent};
use terraphim_types::capability::{Capability, Provider, ProviderType};
use tokio::sync::broadcast::{Receiver, error::TryRecvError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const MAX_CAPTURED_LINES: usize = 64;
const MAX_CAPTURED_MENTIONS: usize = 32;
const MAX_WAIT_SECONDS: u64 = 1800;

fn normalize_agent_name(name: &str) -> String {
    name.trim().to_ascii_lowercase().replace('_', "-")
}

fn parse_capability(value: &str) -> Option<Capability> {
    match value.trim().to_ascii_lowercase().as_str() {
        "deep_thinking" | "deep-thinking" => Some(Capability::DeepThinking),
        "fast_thinking" | "fast-thinking" => Some(Capability::FastThinking),
        "code_generation" | "code-generation" => Some(Capability::CodeGeneration),
        "code_review" | "code-review" => Some(Capability::CodeReview),
        "architecture" => Some(Capability::Architecture),
        "testing" => Some(Capability::Testing),
        "refactoring" => Some(Capability::Refactoring),
        "documentation" => Some(Capability::Documentation),
        "explanation" => Some(Capability::Explanation),
        "security_audit" | "security-audit" => Some(Capability::SecurityAudit),
        "performance" => Some(Capability::Performance),
        _ => None,
    }
}

fn parse_capabilities(values: &[String]) -> Vec<Capability> {
    let mut parsed = Vec::new();
    for value in values {
        if let Some(capability) = parse_capability(value) {
            if !parsed.contains(&capability) {
                parsed.push(capability);
            }
        }
    }

    if parsed.is_empty() {
        parsed.push(Capability::CodeGeneration);
    }

    parsed
}

fn builtin_agent_entries() -> Vec<SpawnerAgentEntry> {
    vec![
        SpawnerAgentEntry {
            name: "codex".to_string(),
            command: "codex".to_string(),
            working_directory: None,
            capabilities: vec!["code_generation".to_string(), "code_review".to_string()],
            health_check_interval_secs: 30,
        },
        SpawnerAgentEntry {
            name: "opencode".to_string(),
            command: "opencode".to_string(),
            working_directory: None,
            capabilities: vec!["code_generation".to_string(), "code_review".to_string()],
            health_check_interval_secs: 30,
        },
        SpawnerAgentEntry {
            name: "claude-code".to_string(),
            command: "claude".to_string(),
            working_directory: None,
            capabilities: vec!["deep_thinking".to_string(), "code_generation".to_string()],
            health_check_interval_secs: 30,
        },
        // Test-safe local baseline.
        SpawnerAgentEntry {
            name: "echo".to_string(),
            command: "echo".to_string(),
            working_directory: None,
            capabilities: vec!["code_generation".to_string()],
            health_check_interval_secs: 30,
        },
    ]
}

fn entry_working_dir(entry: &SpawnerAgentEntry, default_workdir: &std::path::Path) -> PathBuf {
    match &entry.working_directory {
        Some(path) if path.is_absolute() => path.clone(),
        Some(path) => default_workdir.join(path),
        None => default_workdir.to_path_buf(),
    }
}

fn provider_from_entry(entry: &SpawnerAgentEntry, default_workdir: &std::path::Path) -> Provider {
    let normalized = normalize_agent_name(&entry.name);
    let agent_id = if entry.name.starts_with('@') {
        entry.name.clone()
    } else {
        format!("@{}", normalized)
    };

    Provider::new(
        agent_id.clone(),
        entry.name.clone(),
        ProviderType::Agent {
            agent_id,
            cli_command: entry.command.clone(),
            working_dir: entry_working_dir(entry, default_workdir),
        },
        parse_capabilities(&entry.capabilities),
    )
}

fn register_provider_aliases(
    providers: &mut HashMap<String, Provider>,
    name: &str,
    provider: Provider,
) {
    let normalized = normalize_agent_name(name);
    providers.insert(normalized.clone(), provider.clone());

    if normalized == "claude" {
        providers.insert("claude-code".to_string(), provider);
    } else if normalized == "claude-code" {
        providers.insert("claude".to_string(), provider);
    }
}

fn provider_with_workdir(provider: &Provider, working_dir: PathBuf) -> Provider {
    match &provider.provider_type {
        ProviderType::Agent {
            agent_id,
            cli_command,
            ..
        } => Provider::new(
            provider.id.clone(),
            provider.name.clone(),
            ProviderType::Agent {
                agent_id: agent_id.clone(),
                cli_command: cli_command.clone(),
                working_dir,
            },
            provider.capabilities.clone(),
        ),
        _ => provider.clone(),
    }
}

fn push_capped(items: &mut Vec<String>, value: String, cap: usize) {
    if items.len() < cap {
        items.push(value);
    }
}

fn drain_output_events(
    receiver: &mut Receiver<OutputEvent>,
    stdout_lines: &mut Vec<String>,
    stderr_lines: &mut Vec<String>,
    mentions: &mut Vec<serde_json::Value>,
) {
    loop {
        match receiver.try_recv() {
            Ok(OutputEvent::Stdout { line, .. }) => {
                push_capped(stdout_lines, line, MAX_CAPTURED_LINES);
            }
            Ok(OutputEvent::Stderr { line, .. }) => {
                push_capped(stderr_lines, line, MAX_CAPTURED_LINES);
            }
            Ok(OutputEvent::Mention {
                target, message, ..
            }) => {
                if mentions.len() < MAX_CAPTURED_MENTIONS {
                    mentions.push(serde_json::json!({
                        "target": target,
                        "message": message,
                    }));
                }
            }
            Ok(OutputEvent::Completed { .. }) => {}
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Lagged(_)) => continue,
            Err(TryRecvError::Closed) => break,
        }
    }
}

/// Spawn external agent CLIs via `terraphim_spawner`.
pub struct AgentSpawnTool {
    spawner: AgentSpawner,
    providers: HashMap<String, Provider>,
    enabled: bool,
    default_timeout_secs: u64,
    shutdown_grace_secs: u64,
    semaphore: Arc<Semaphore>,
}

impl AgentSpawnTool {
    pub fn new(default_workdir: PathBuf) -> Self {
        Self::with_config(default_workdir, SpawnerConfig::default())
    }

    pub fn with_config(default_workdir: PathBuf, config: SpawnerConfig) -> Self {
        let mut providers = HashMap::new();

        for entry in builtin_agent_entries() {
            let provider = provider_from_entry(&entry, &default_workdir);
            register_provider_aliases(&mut providers, &entry.name, provider);
        }

        for entry in &config.agents {
            let provider = provider_from_entry(entry, &default_workdir);
            register_provider_aliases(&mut providers, &entry.name, provider);
        }

        let spawner = AgentSpawner::new()
            .with_working_dir(default_workdir)
            .with_auto_restart(true)
            .with_max_restarts(3);

        Self {
            spawner,
            providers,
            enabled: config.enabled,
            default_timeout_secs: config.default_timeout_secs.max(1),
            shutdown_grace_secs: config.shutdown_grace_secs.max(1),
            semaphore: Arc::new(Semaphore::new(config.max_concurrent.max(1))),
        }
    }

    async fn spawn_agent(&self, provider: &Provider, task: &str) -> Result<AgentHandle, ToolError> {
        self.spawner
            .spawn(provider, task)
            .await
            .map_err(|error| ToolError::ExecutionFailed {
                tool: self.name().to_string(),
                message: format!("Failed to spawn agent: {}", error),
            })
    }

    async fn monitor_detached(
        mut handle: AgentHandle,
        mut output_rx: Receiver<OutputEvent>,
        _permit: OwnedSemaphorePermit,
    ) {
        loop {
            // Drain output to avoid unbounded lag accumulation in broadcast channel.
            loop {
                match output_rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Lagged(_)) => continue,
                    Err(TryRecvError::Empty) | Err(TryRecvError::Closed) => break,
                }
            }

            match handle.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => tokio::time::sleep(Duration::from_millis(200)).await,
                Err(error) => {
                    log::warn!("Detached agent monitoring failed: {}", error);
                    break;
                }
            }
        }
    }

    async fn wait_for_completion(
        &self,
        mut handle: AgentHandle,
        mut output_rx: Receiver<OutputEvent>,
        wait_seconds: u64,
    ) -> Result<String, ToolError> {
        let deadline = Instant::now() + Duration::from_secs(wait_seconds);
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        let mut mentions = Vec::new();

        loop {
            drain_output_events(
                &mut output_rx,
                &mut stdout_lines,
                &mut stderr_lines,
                &mut mentions,
            );

            match handle.try_wait() {
                Ok(Some(status)) => {
                    return Ok(serde_json::json!({
                        "status": "completed",
                        "process_id": handle.process_id().0,
                        "agent_id": handle.provider.id,
                        "exit_code": status.code(),
                        "stdout": stdout_lines,
                        "stderr": stderr_lines,
                        "mentions": mentions,
                    })
                    .to_string());
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(ToolError::ExecutionFailed {
                        tool: self.name().to_string(),
                        message: format!("Failed while waiting for agent completion: {}", error),
                    });
                }
            }

            if Instant::now() >= deadline {
                let _ = handle
                    .shutdown(Duration::from_secs(self.shutdown_grace_secs))
                    .await;
                return Err(ToolError::Timeout {
                    tool: self.name().to_string(),
                    seconds: wait_seconds,
                });
            }

            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }
}

#[async_trait]
impl Tool for AgentSpawnTool {
    fn name(&self) -> &str {
        "agent_spawn"
    }

    fn description(&self) -> &str {
        "Spawn an external agent process via terraphim_spawner"
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
                    "minimum": 1,
                    "description": "How long to wait for completion/output (default from spawner config)"
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
        if !self.enabled {
            return Err(ToolError::Blocked {
                tool: self.name().to_string(),
                reason: "agent spawning is disabled by configuration".to_string(),
            });
        }

        let agent_type = args
            .get("agent_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'agent_type' parameter".to_string(),
            })?;
        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'task' parameter".to_string(),
            })?
            .trim()
            .to_string();

        if task.is_empty() {
            return Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "'task' cannot be empty".to_string(),
            });
        }

        let agent_key = normalize_agent_name(agent_type);
        let provider =
            self.providers
                .get(&agent_key)
                .cloned()
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: self.name().to_string(),
                    message: format!(
                        "Unsupported agent_type '{}'. Available: {}",
                        agent_type,
                        self.providers
                            .keys()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                })?;

        let provider =
            if let Some(working_dir) = args.get("working_directory").and_then(|v| v.as_str()) {
                let working_dir = PathBuf::from(working_dir);
                if !working_dir.exists() {
                    return Err(ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: format!(
                            "Working directory does not exist: {}",
                            working_dir.display()
                        ),
                    });
                }
                provider_with_workdir(&provider, working_dir)
            } else {
                provider
            };

        let wait_seconds = args
            .get("wait_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(self.default_timeout_secs)
            .clamp(1, MAX_WAIT_SECONDS);
        let detach = args
            .get("detach")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let permit =
            self.semaphore
                .clone()
                .try_acquire_owned()
                .map_err(|_| ToolError::Blocked {
                    tool: self.name().to_string(),
                    reason: "Maximum concurrent spawned agents reached".to_string(),
                })?;

        let handle = self.spawn_agent(&provider, &task).await?;
        let process_id = handle.process_id().0;
        let output_rx = handle.subscribe_output();

        if detach {
            tokio::spawn(Self::monitor_detached(handle, output_rx, permit));
            return Ok(serde_json::json!({
                "status": "spawned",
                "process_id": process_id,
                "agent_id": provider.id,
                "detached": true,
            })
            .to_string());
        }

        let result = self
            .wait_for_completion(handle, output_rx, wait_seconds)
            .await;
        drop(permit);
        result
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
    async fn test_agent_spawn_echo_via_spawner() {
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
        assert!(payload["process_id"].is_number());
    }

    #[tokio::test]
    async fn test_agent_spawn_respects_disabled_config() {
        let temp_dir = TempDir::new().unwrap();
        let tool = AgentSpawnTool::with_config(
            temp_dir.path().to_path_buf(),
            SpawnerConfig {
                enabled: false,
                ..Default::default()
            },
        );

        let err = tool
            .execute(serde_json::json!({
                "agent_type": "echo",
                "task": "blocked"
            }))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Blocked { .. }));
    }
}
