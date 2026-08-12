//! `ScheduleTool` — Hermes-parity cron scheduling surface (#3147).
//!
//! Lets the agent loop create recurring schedules in conversation, backed
//! in production by the `terraphim_orchestrator` config contract:
//! scheduled tasks are persisted as `[[agents]]` entries with `schedule`
//! fields in an orchestrator include fragment. Operations:
//! - `create` {prompt, schedule, skills?, deliver?, model?} — validate the
//!   schedule expression and persist a new job, returning its id
//! - `list` — all stored jobs (id, schedule, state, next run)
//! - `delete` {id} — remove a job by id
//!
//! The CLI subcommand (`terraphim-tinyclaw schedule …`) shares the same
//! helper functions, so the CLI and the tool cannot drift.
//!
//! `ScheduleTool::new` remains available for explicit local/test use over
//! TinyClaw's `CronStore`; `from_config` requires
//! `scheduler.orchestrator_schedule_file` and fails fast when the
//! orchestrator integration is not configured.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::cron::{CronJob, CronStore, Schedule};

/// Default bound on jobs listed per `list` call.
const LIST_LIMIT: usize = 100;
const AGENT_NAME_PREFIX: &str = "tinyclaw-";

/// The scheduler tool.
pub struct ScheduleTool {
    backend: ScheduleBackend,
}

enum ScheduleBackend {
    Local(CronStore),
    Orchestrator(OrchestratorScheduleStore),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestratorScheduleFragment {
    #[serde(default)]
    agents: Vec<OrchestratorScheduleAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OrchestratorScheduleAgent {
    name: String,
    layer: String,
    cli_tool: String,
    task: String,
    schedule: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    skill_chain: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

/// Durable orchestrator schedule fragment store.
#[derive(Debug, Clone)]
pub struct OrchestratorScheduleStore {
    path: PathBuf,
    cli_tool: String,
}

impl OrchestratorScheduleStore {
    /// Create a store writing generated schedule agents to `path`.
    pub fn new(path: PathBuf) -> Self {
        Self::with_cli_tool(path, "terraphim-tinyclaw")
    }

    /// Create a store with an explicit generated-agent CLI tool.
    pub fn with_cli_tool(path: PathBuf, cli_tool: impl Into<String>) -> Self {
        Self {
            path,
            cli_tool: cli_tool.into(),
        }
    }

    fn load_fragment(&self) -> Result<OrchestratorScheduleFragment, ToolError> {
        if !self.path.exists() {
            return Ok(OrchestratorScheduleFragment { agents: Vec::new() });
        }
        let content = std::fs::read_to_string(&self.path)?;
        toml::from_str(&content).map_err(|e| ToolError::ExecutionFailed {
            tool: "schedule".to_string(),
            message: format!(
                "parse orchestrator schedule fragment {}: {e}",
                self.path.display()
            ),
        })
    }

    fn save_fragment(&self, fragment: &OrchestratorScheduleFragment) -> Result<(), ToolError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(fragment).map_err(|e| ToolError::ExecutionFailed {
            tool: "schedule".to_string(),
            message: format!("serialise orchestrator schedule fragment: {e}"),
        })?;
        atomic_write(&self.path, content.as_bytes())?;
        Ok(())
    }

    fn create_job(
        &self,
        prompt: String,
        schedule_expr: &str,
        skills: Vec<String>,
        _deliver: Option<String>,
        model: Option<String>,
    ) -> Result<String, ToolError> {
        if !terraphim_orchestrator::is_cron_schedule_valid(schedule_expr) {
            return Err(ToolError::InvalidArguments {
                tool: "schedule".to_string(),
                message: format!(
                    "invalid orchestrator cron schedule '{schedule_expr}': expected 5, 6, or 7 cron fields"
                ),
            });
        }

        let id = uuid::Uuid::new_v4().simple().to_string();
        let agent = OrchestratorScheduleAgent {
            name: format!("{AGENT_NAME_PREFIX}{id}"),
            layer: "Core".to_string(),
            cli_tool: self.cli_tool.clone(),
            task: prompt,
            schedule: schedule_expr.to_string(),
            model,
            skill_chain: skills,
            capabilities: vec!["tinyclaw-schedule".to_string()],
            enabled: true,
        };
        let mut fragment = self.load_fragment()?;
        fragment.agents.push(agent);
        self.save_fragment(&fragment)?;
        Ok(id)
    }

    fn list_jobs(&self) -> Result<Vec<ScheduledJob>, ToolError> {
        let fragment = self.load_fragment()?;
        Ok(fragment
            .agents
            .into_iter()
            .filter_map(ScheduledJob::from_orchestrator_agent)
            .take(LIST_LIMIT)
            .collect())
    }

    fn delete_job(&self, id: &str) -> Result<bool, ToolError> {
        let mut fragment = self.load_fragment()?;
        let before = fragment.agents.len();
        let expected_name = format!("{AGENT_NAME_PREFIX}{id}");
        fragment.agents.retain(|agent| agent.name != expected_name);
        let removed = fragment.agents.len() != before;
        if removed {
            self.save_fragment(&fragment)?;
        }
        Ok(removed)
    }
}

impl ScheduleTool {
    /// Create a scheduler tool over an explicit store (test-friendly).
    pub fn new(store: CronStore) -> Self {
        Self {
            backend: ScheduleBackend::Local(store),
        }
    }

    /// Create a scheduler tool over an orchestrator schedule fragment.
    pub fn new_orchestrator(store: OrchestratorScheduleStore) -> Self {
        Self {
            backend: ScheduleBackend::Orchestrator(store),
        }
    }

    /// Create a scheduler tool with the default production storage.
    pub async fn from_config(cfg: &crate::config::SchedulerConfig) -> Result<Self, ToolError> {
        let path =
            cfg.orchestrator_schedule_file
                .clone()
                .ok_or_else(|| ToolError::BackendUnavailable {
                    tool: "schedule".to_string(),
                    message: "scheduler.orchestrator_schedule_file is required; configure an orchestrator include fragment and include it from orchestrator.toml".to_string(),
                })?;
        Ok(Self::new_orchestrator(
            OrchestratorScheduleStore::with_cli_tool(path, cfg.cli_tool.clone()),
        ))
    }

    /// Create a job. Shared with the CLI subcommand.
    pub async fn create_job(
        &self,
        prompt: String,
        schedule_expr: &str,
        skills: Vec<String>,
        deliver: Option<String>,
        model: Option<String>,
    ) -> Result<String, ToolError> {
        match &self.backend {
            ScheduleBackend::Local(store) => {
                let schedule =
                    Schedule::parse(schedule_expr).map_err(|e| ToolError::InvalidArguments {
                        tool: "schedule".to_string(),
                        message: format!("invalid schedule '{schedule_expr}': {e}"),
                    })?;
                let mut job = CronJob::new(prompt, schedule);
                job.skills = skills;
                job.deliver = deliver;
                job.model = model;

                let job_id = job.id.clone();
                let mut jobs = store
                    .load_all()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "schedule".to_string(),
                        message: format!("load jobs failed: {e}"),
                    })?;
                jobs.push(job);
                store
                    .save_all(&jobs)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "schedule".to_string(),
                        message: format!("save jobs failed: {e}"),
                    })?;
                Ok(job_id)
            }
            ScheduleBackend::Orchestrator(store) => {
                store.create_job(prompt, schedule_expr, skills, deliver, model)
            }
        }
    }

    /// List stored jobs. Shared with the CLI subcommand.
    pub async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, ToolError> {
        match &self.backend {
            ScheduleBackend::Local(store) => {
                let jobs = store
                    .load_all()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "schedule".to_string(),
                        message: format!("load jobs failed: {e}"),
                    })?;
                Ok(jobs
                    .into_iter()
                    .take(LIST_LIMIT)
                    .map(ScheduledJob::from_cron_job)
                    .collect())
            }
            ScheduleBackend::Orchestrator(store) => store.list_jobs(),
        }
    }

    /// Delete a job by id. Returns `false` when the id is unknown.
    pub async fn delete_job(&self, id: &str) -> Result<bool, ToolError> {
        match &self.backend {
            ScheduleBackend::Local(store) => {
                let mut jobs = store
                    .load_all()
                    .await
                    .map_err(|e| ToolError::ExecutionFailed {
                        tool: "schedule".to_string(),
                        message: format!("load jobs failed: {e}"),
                    })?;
                let before = jobs.len();
                jobs.retain(|j| j.id != id);
                let removed = jobs.len() != before;
                if removed {
                    store
                        .save_all(&jobs)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed {
                            tool: "schedule".to_string(),
                            message: format!("save jobs failed: {e}"),
                        })?;
                }
                Ok(removed)
            }
            ScheduleBackend::Orchestrator(store) => store.delete_job(id),
        }
    }
}

/// Stable list projection shared by local and orchestrator backends.
#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub id: String,
    pub name: Option<String>,
    pub prompt: String,
    pub schedule: String,
    pub state: String,
    pub enabled: bool,
    pub next_run_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ScheduledJob {
    fn from_cron_job(job: CronJob) -> Self {
        Self {
            id: job.id,
            name: job.name,
            prompt: job.prompt,
            schedule: format!("{:?}", job.schedule),
            state: format!("{:?}", job.state),
            enabled: job.enabled,
            next_run_at: job.next_run_at,
        }
    }

    fn from_orchestrator_agent(agent: OrchestratorScheduleAgent) -> Option<Self> {
        let id = agent.name.strip_prefix(AGENT_NAME_PREFIX)?.to_string();
        Some(Self {
            id,
            name: Some(agent.name),
            prompt: agent.task,
            schedule: agent.schedule,
            state: "Scheduled".to_string(),
            enabled: agent.enabled,
            next_run_at: None,
        })
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("toml")
    ));
    std::fs::write(&tmp_path, bytes)?;
    std::fs::rename(tmp_path, path)
}

#[async_trait]
impl Tool for ScheduleTool {
    fn name(&self) -> &str {
        "schedule"
    }

    fn description(&self) -> &str {
        "Create, list and delete recurring schedules. Operations: \
         create {prompt, schedule, skills?, deliver?, model?}, list {}, \
         delete {id}. Production scheduling is backed by \
         terraphim_orchestrator and accepts cron expressions such as \
         '0 9 * * *'."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "op": {
                    "type": "string",
                    "enum": ["create", "list", "delete"],
                    "description": "Operation to perform"
                },
                "prompt": { "type": "string", "description": "Task prompt (create)" },
                "schedule": { "type": "string", "description": "Cron expression (create)" },
                "skills": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Skills to inject at job start (create)"
                },
                "deliver": { "type": "string", "description": "Delivery target (create)" },
                "model": { "type": "string", "description": "Model override (create)" },
                "id": { "type": "string", "description": "Job id (delete)" }
            },
            "required": ["op"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let op =
            args.get("op")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "schedule".to_string(),
                    message: "missing required 'op' field".to_string(),
                })?;

        match op {
            "create" => {
                let prompt = args.get("prompt").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "schedule".to_string(),
                        message: "create requires 'prompt'".to_string(),
                    }
                })?;
                let schedule = args
                    .get("schedule")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "schedule".to_string(),
                        message: "create requires 'schedule'".to_string(),
                    })?;
                let skills = args
                    .get("skills")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let deliver = args
                    .get("deliver")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let model = args
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let id = self
                    .create_job(prompt.to_string(), schedule, skills, deliver, model)
                    .await?;
                Ok(json!({
                    "op": "create",
                    "id": id,
                    "schedule": schedule,
                    "status": "created",
                })
                .to_string())
            }
            "list" => {
                let jobs = self.list_jobs().await?;
                let items: Vec<Value> = jobs
                    .iter()
                    .map(|j| {
                        json!({
                            "id": j.id,
                            "name": j.name,
                            "prompt": j.prompt,
                            "schedule": j.schedule,
                            "state": j.state,
                            "enabled": j.enabled,
                            "next_run_at": j.next_run_at,
                        })
                    })
                    .collect();
                Ok(json!({
                    "op": "list",
                    "count": items.len(),
                    "jobs": items,
                })
                .to_string())
            }
            "delete" => {
                let id = args.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                    ToolError::InvalidArguments {
                        tool: "schedule".to_string(),
                        message: "delete requires 'id'".to_string(),
                    }
                })?;
                let removed = self.delete_job(id).await?;
                if !removed {
                    return Err(ToolError::ExecutionFailed {
                        tool: "schedule".to_string(),
                        message: format!("unknown job id '{id}' (use list to see jobs)"),
                    });
                }
                Ok(json!({
                    "op": "delete",
                    "id": id,
                    "status": "deleted",
                })
                .to_string())
            }
            other => Err(ToolError::InvalidArguments {
                tool: "schedule".to_string(),
                message: format!("unknown op '{other}'"),
            }),
        }
    }
}
