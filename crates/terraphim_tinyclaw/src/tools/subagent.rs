//! `SubagentTool` — Hermes-parity isolated subagents (#3145).
//!
//! Wraps `terraphim_spawner`'s `AgentPool` so chat users can delegate tasks
//! to isolated subagents (own process, working dir, output capture).
//! Operations:
//! - `spawn` {task, model?} — spawn a subagent, returns its id
//! - `status` {id} — health status of a subagent
//! - `list` — all tracked subagents
//! - `terminate` {id} — graceful shutdown (SIGTERM then SIGKILL)
//! - `collect` {id} — captured output lines so far
//!
//! Handles are tracked in a `Mutex<HashMap<String, AgentHandle>>` so
//! multi-turn conversations can reference subagents by id (per issue:
//! "Track active handles in Session or a dedicated registry").

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use terraphim_persistence::DeviceStorage;
use terraphim_spawner::{AgentHandle, AgentSpawner, SpawnContext};
use terraphim_types::capability::{Capability, Provider, ProviderType};
use tokio::sync::Mutex;

/// Configuration for the subagent tool, converted from
/// [`crate::config::SubagentConfig`].
#[derive(Debug, Clone)]
pub struct SubagentToolConfig {
    /// Provider id (e.g. `"claude-code"`). Maps to a cli command.
    pub provider: String,
    /// Optional default model.
    pub model: Option<String>,
    /// Timeout for waiting on spawned agents.
    pub timeout_secs: u64,
}

impl From<&crate::config::SubagentConfig> for SubagentToolConfig {
    fn from(cfg: &crate::config::SubagentConfig) -> Self {
        Self {
            provider: cfg.provider.clone(),
            model: cfg.model.clone(),
            timeout_secs: cfg.timeout_secs,
        }
    }
}

/// Map a provider id to a CLI command. Known ids resolve to their
/// standard commands; unknown ids are used verbatim as the command.
fn provider_cli_command(provider_id: &str) -> &str {
    match provider_id {
        "claude-code" | "claude" => "claude",
        "codex" => "codex",
        "opencode" => "opencode",
        other => other,
    }
}

/// Build a `Provider` from config (Agent-type provider with CLI command).
pub fn provider_from_config(cfg: &SubagentToolConfig) -> Provider {
    Provider::new(
        format!("@{}", cfg.provider),
        cfg.provider.clone(),
        ProviderType::Agent {
            agent_id: format!("@{}", cfg.provider),
            cli_command: provider_cli_command(&cfg.provider).to_string(),
            working_dir: PathBuf::from("/tmp"),
        },
        vec![Capability::CodeGeneration],
    )
}

/// The subagent tool.
pub struct SubagentTool {
    spawner: AgentSpawner,
    provider: Provider,
    default_model: Option<String>,
    grace: Duration,
    /// Bridge to a persistent current-thread runtime where spawns run
    /// (the spawner's future is !Send; capture tasks need a live runtime).
    bridge: Arc<SpawnBridge>,
    /// id -> live handle registry.
    handles: Arc<Mutex<HashMap<String, AgentHandle>>>,
    /// Optional durable registry (`terraphim_persistence::DeviceStorage`)
    /// so spawned-subagent metadata survives restarts. None = memory only.
    persist: Option<SubagentRegistry>,
}

impl SubagentTool {
    /// Create a subagent tool with explicit spawner/provider (test-friendly).
    pub fn with_spawner(
        spawner: AgentSpawner,
        provider: Provider,
        default_model: Option<String>,
        timeout_secs: u64,
    ) -> Self {
        Self {
            spawner,
            provider,
            default_model,
            grace: Duration::from_secs(timeout_secs),
            bridge: Arc::new(SpawnBridge::start()),
            handles: Arc::new(Mutex::new(HashMap::new())),
            persist: None,
        }
    }

    /// Attach a durable registry backed by `DeviceStorage`.
    pub fn with_persistence(mut self, storage: Arc<DeviceStorage>, key: impl Into<String>) -> Self {
        self.persist = Some(SubagentRegistry::new(storage, key));
        self
    }

    /// Create a subagent tool from config.
    pub fn from_config(cfg: &crate::config::SubagentConfig) -> Self {
        let tool_cfg = SubagentToolConfig::from(cfg);
        let provider = provider_from_config(&tool_cfg);
        let mut tool = Self::with_spawner(
            AgentSpawner::new(),
            provider,
            tool_cfg.model,
            tool_cfg.timeout_secs,
        );
        // Attach the durable registry when DeviceStorage is available
        // (graceful degradation: tool stays fully functional in-memory).
        tool = tool.with_persistence_from_env();
        tool
    }

    /// Try to attach `terraphim_persistence` storage; no-op on failure.
    fn with_persistence_from_env(mut self) -> Self {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(_) => return self,
        };
        let storage = rt.block_on(async {
            match DeviceStorage::arc_instance().await {
                Ok(s) => Some(s),
                Err(e) => {
                    log::warn!("subagent registry disabled: {}", e);
                    None
                }
            }
        });
        match storage {
            Some(s) => {
                self.persist = Some(SubagentRegistry::new(s, "tinyclaw/subagents"));
                self
            }
            None => self,
        }
    }

    /// Id for a spawn (uuid v4 without dashes, shortened).
    fn new_handle_id() -> String {
        uuid::Uuid::new_v4().simple().to_string()[..16].to_string()
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &str {
        "subagent"
    }

    fn description(&self) -> &str {
        "Spawn and manage isolated subagents (own process + output capture). \
         Operations: spawn {task, model?}, status {id}, list {}, \
         terminate {id}, collect {id}."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["spawn", "status", "list", "terminate", "collect"],
                    "description": "Operation to perform"
                },
                "task": { "type": "string", "description": "Task prompt (spawn)" },
                "model": { "type": "string", "description": "Optional model override (spawn)" },
                "id": { "type": "string", "description": "Subagent id" }
            },
            "required": ["op"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let op =
            args.get("op")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "subagent".to_string(),
                    message: "missing required 'op' field".to_string(),
                })?;

        match op {
            "spawn" => {
                let task = args.get("task").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "subagent".to_string(),
                        message: "spawn requires 'task'".to_string(),
                    }
                })?;
                let model = args
                    .get("model")
                    .and_then(|v| v.as_str())
                    .or(self.default_model.as_deref());
                let id = Self::new_handle_id();

                // The spawner holds a tracing `EnteredSpan` across an await
                // point inside spawn_with_model (lib.rs:671), making the
                // future !Send. Spawns run on a persistent current-thread
                // runtime (SpawnBridge) so the Tool trait's Send future
                // contract is preserved AND the output-capture tasks stay
                // alive for the handle's lifetime.
                let handle = self
                    .bridge
                    .spawn(
                        self.spawner.clone(),
                        self.provider.clone(),
                        task.to_string(),
                        model.map(|m| m.to_string()),
                    )
                    .await?;

                self.handles.lock().await.insert(id.clone(), handle);

                // Persist metadata so the spawn survives restarts (best effort).
                if let Some(reg) = &self.persist {
                    let pid = self
                        .handles
                        .lock()
                        .await
                        .get(&id)
                        .map(|h| h.process_id().0)
                        .unwrap_or(0);
                    let record = SubagentRecord {
                        id: id.clone(),
                        pid,
                        provider: self.provider.id.clone(),
                        task: task.to_string(),
                        model: model.map(|m| m.to_string()),
                        spawned_at: chrono::Utc::now().timestamp(),
                    };
                    let _ = reg.upsert(record).await;
                }

                Ok(json!({
                    "op": "spawn",
                    "id": id,
                    "provider": self.provider.id,
                    "model": model,
                })
                .to_string())
            }
            "status" => {
                let id = require_id(&args)?;
                let handles = self.handles.lock().await;
                let handle = handles.get(&id).ok_or_else(|| unknown_id(id.clone()))?;
                let status = handle.health_status();
                let pid = handle.process_id().0;
                Ok(json!({
                    "op": "status",
                    "id": id,
                    "pid": pid,
                    "health": format!("{status:?}"),
                })
                .to_string())
            }
            "list" => {
                let handles = self.handles.lock().await;
                let mut agents: Vec<Value> = handles
                    .iter()
                    .map(|(id, h)| {
                        json!({
                            "id": id,
                            "pid": h.process_id().0,
                            "health": format!("{:?}", h.health_status()),
                            "live": true,
                        })
                    })
                    .collect();
                // Include persisted-but-not-live records (e.g. before a
                // restart) with live=false so users can see they existed.
                if let Some(reg) = &self.persist
                    && let Ok(records) = reg.all().await
                {
                    let live_ids: std::collections::HashSet<&str> =
                        handles.keys().map(|s| s.as_str()).collect();
                    for r in records {
                        if !live_ids.contains(r.id.as_str()) {
                            agents.push(json!({
                                "id": r.id,
                                "pid": r.pid,
                                "provider": r.provider,
                                "task": r.task,
                                "spawned_at": r.spawned_at,
                                "live": false,
                            }));
                        }
                    }
                }
                Ok(json!({
                    "op": "list",
                    "count": agents.len(),
                    "agents": agents,
                })
                .to_string())
            }
            "terminate" => {
                let id = require_id(&args)?;
                let mut handles = self.handles.lock().await;
                let mut handle = handles.remove(&id).ok_or_else(|| unknown_id(id.clone()))?;
                let graceful =
                    handle
                        .shutdown(self.grace)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed {
                            tool: "subagent".to_string(),
                            message: format!("terminate failed: {e}"),
                        })?;
                // Remove from durable registry.
                if let Some(reg) = &self.persist {
                    let _ = reg.remove(&id).await;
                }
                Ok(json!({
                    "op": "terminate",
                    "id": id,
                    "graceful": graceful,
                })
                .to_string())
            }
            "collect" => {
                let id = require_id(&args)?;
                let handles = self.handles.lock().await;
                let handle = handles.get(&id).ok_or_else(|| unknown_id(id.clone()))?;
                let events = handle.output_capture().captured_events();
                let lines: Vec<String> = events
                    .iter()
                    .map(|e| match e {
                        terraphim_spawner::output::OutputEvent::Stdout { line, .. }
                        | terraphim_spawner::output::OutputEvent::Stderr { line, .. } => {
                            line.clone()
                        }
                        _ => String::new(),
                    })
                    .filter(|l| !l.is_empty())
                    .collect();
                Ok(json!({
                    "op": "collect",
                    "id": id,
                    "lines": lines,
                    "count": lines.len(),
                })
                .to_string())
            }
            other => Err(ToolError::InvalidArguments {
                tool: "subagent".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}

fn require_id(args: &Value) -> Result<String, ToolError> {
    args.get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: "subagent".to_string(),
            message: "operation requires 'id'".to_string(),
        })
}

fn unknown_id(id: String) -> ToolError {
    ToolError::ExecutionFailed {
        tool: "subagent".to_string(),
        message: format!("unknown subagent id '{id}' (use list to see live agents)"),
    }
}

/// Durable record for a spawned subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentRecord {
    /// Handle id (16 hex chars).
    pub id: String,
    /// OS process id.
    pub pid: u64,
    /// Provider id.
    pub provider: String,
    /// Task prompt.
    pub task: String,
    /// Optional model.
    pub model: Option<String>,
    /// Unix timestamp of spawn.
    pub spawned_at: i64,
}

/// JSON-document registry for subagent metadata, persisted via
/// `terraphim_persistence::DeviceStorage` (same pattern as `cron/store.rs`).
#[derive(Clone)]
pub struct SubagentRegistry {
    storage: Arc<DeviceStorage>,
    key: String,
}

impl SubagentRegistry {
    pub fn new(storage: Arc<DeviceStorage>, key: impl Into<String>) -> Self {
        Self {
            storage,
            key: key.into(),
        }
    }

    async fn load(&self) -> Result<Vec<SubagentRecord>, ToolError> {
        match self.storage.fastest_op.read(&self.key).await {
            Ok(bytes) => serde_json::from_slice(bytes.to_bytes().as_ref()).map_err(|e| {
                ToolError::ExecutionFailed {
                    tool: "subagent".to_string(),
                    message: format!("parse registry: {e}"),
                }
            }),
            Err(_) => Ok(Vec::new()),
        }
    }

    async fn save(&self, records: &[SubagentRecord]) -> Result<(), ToolError> {
        let json = serde_json::to_vec(records).map_err(|e| ToolError::ExecutionFailed {
            tool: "subagent".to_string(),
            message: format!("serialise registry: {e}"),
        })?;
        self.storage
            .fastest_op
            .write(&self.key, json)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "subagent".to_string(),
                message: format!("write registry: {e}"),
            })?;
        Ok(())
    }

    /// Upsert a record, returning the full updated list.
    pub async fn upsert(&self, record: SubagentRecord) -> Result<Vec<SubagentRecord>, ToolError> {
        let mut records = self.load().await?;
        records.retain(|r| r.id != record.id);
        records.push(record);
        self.save(&records).await?;
        Ok(records)
    }

    /// Remove a record by id, returning the full updated list.
    pub async fn remove(&self, id: &str) -> Result<Vec<SubagentRecord>, ToolError> {
        let mut records = self.load().await?;
        records.retain(|r| r.id != id);
        self.save(&records).await?;
        Ok(records)
    }

    pub async fn all(&self) -> Result<Vec<SubagentRecord>, ToolError> {
        self.load().await
    }
}

/// Run a subagent spawn on a persistent dedicated current-thread runtime.
///
/// `spawn_with_model` holds a tracing `EnteredSpan` across an await point
/// (terraphim_spawner lib.rs:671), making its future `!Send`, and the
/// output-capture tasks it spawns (`tokio::spawn` in OutputCapture) must
/// outlive the spawn call or no output is ever recorded. `SpawnBridge`
/// runs a long-lived current-thread tokio runtime on a dedicated thread:
/// the runtime's `LocalSet` permits `!Send` futures, and staying alive
/// keeps capture tasks processing for the handle's whole lifetime.
struct SpawnBridge {
    tx: tokio::sync::mpsc::UnboundedSender<SpawnJob>,
}

struct SpawnJob {
    spawner: AgentSpawner,
    provider: Provider,
    task: String,
    model: Option<String>,
    reply: tokio::sync::oneshot::Sender<Result<AgentHandle, terraphim_spawner::SpawnerError>>,
}

impl SpawnBridge {
    fn start() -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<SpawnJob>();
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(_) => return,
            };
            let local = tokio::task::LocalSet::new();
            local.block_on(&rt, async move {
                while let Some(job) = rx.recv().await {
                    let result = job
                        .spawner
                        .spawn_with_model(
                            &job.provider,
                            &job.task,
                            job.model.as_deref(),
                            SpawnContext::global(),
                        )
                        .await;
                    let _ = job.reply.send(result);
                }
            });
        });
        Self { tx }
    }

    async fn spawn(
        &self,
        spawner: AgentSpawner,
        provider: Provider,
        task: String,
        model: Option<String>,
    ) -> Result<AgentHandle, ToolError> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .send(SpawnJob {
                spawner,
                provider,
                task,
                model,
                reply,
            })
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "subagent".to_string(),
                message: format!("spawn bridge closed: {e}"),
            })?;
        rx.await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "subagent".to_string(),
                message: format!("spawn channel closed: {e}"),
            })?
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "subagent".to_string(),
                message: format!("spawn failed: {e}"),
            })
    }
}
