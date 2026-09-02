//! `ScheduleTool` — Hermes-parity cron scheduling surface (#3147).
//!
//! Lets the agent loop create recurring schedules in conversation, backed
//! by the Wave-3 cron subsystem (`crate::cron::CronStore` over
//! `terraphim_persistence::DeviceStorage`). Operations:
//! - `create` {prompt, schedule, skills?, deliver?, model?} — validate the
//!   schedule expression and persist a new job, returning its id
//! - `list` — all stored jobs (id, schedule, state, next run)
//! - `delete` {id} — remove a job by id
//!
//! The CLI subcommand (`terraphim-tinyclaw schedule …`) shares the same
//! helper functions, so the CLI and the tool cannot drift.
//!
//! Deviation from issue #3147: the issue proposed validating via
//! `terraphim_orchestrator::is_cron_schedule_valid` and persisting through
//! the orchestrator. The orchestrator is excluded from this workspace
//! (registry-only, two versions in the lock) and its process is down;
//! tinyclaw's own `Schedule::parse` validates cron via the same `cron`
//! crate, and `CronStore` provides durable persistence. See
//! `docs/plans/research-tinyclaw-cron-and-jmap.md`.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{Value, json};
use terraphim_persistence::DeviceStorage;

use crate::cron::{CronJob, CronStore, Schedule};

/// Default bound on jobs listed per `list` call.
const LIST_LIMIT: usize = 100;

/// The scheduler tool.
pub struct ScheduleTool {
    store: CronStore,
}

impl ScheduleTool {
    /// Create a scheduler tool over an explicit store (test-friendly).
    pub fn new(store: CronStore) -> Self {
        Self { store }
    }

    /// Create a scheduler tool with the default production storage.
    pub async fn from_config(cfg: &crate::config::SchedulerConfig) -> Result<Self, ToolError> {
        let storage =
            DeviceStorage::arc_instance()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "schedule".to_string(),
                    message: format!("device storage unavailable: {e}"),
                })?;
        Ok(Self::new(CronStore::new(storage, cfg.store_key.clone())))
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
        let schedule = Schedule::parse(schedule_expr).map_err(|e| ToolError::InvalidArguments {
            tool: "schedule".to_string(),
            message: format!("invalid schedule '{schedule_expr}': {e}"),
        })?;
        let mut job = CronJob::new(prompt, schedule);
        job.skills = skills;
        job.deliver = deliver;
        job.model = model;

        let job_id = job.id.clone();
        let mut jobs = self
            .store
            .load_all()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "schedule".to_string(),
                message: format!("load jobs failed: {e}"),
            })?;
        jobs.push(job);
        self.store
            .save_all(&jobs)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "schedule".to_string(),
                message: format!("save jobs failed: {e}"),
            })?;
        Ok(job_id)
    }

    /// List stored jobs. Shared with the CLI subcommand.
    pub async fn list_jobs(&self) -> Result<Vec<CronJob>, ToolError> {
        let jobs = self
            .store
            .load_all()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "schedule".to_string(),
                message: format!("load jobs failed: {e}"),
            })?;
        Ok(jobs.into_iter().take(LIST_LIMIT).collect())
    }

    /// Delete a job by id. Returns `false` when the id is unknown.
    pub async fn delete_job(&self, id: &str) -> Result<bool, ToolError> {
        let mut jobs = self
            .store
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
            self.store
                .save_all(&jobs)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "schedule".to_string(),
                    message: format!("save jobs failed: {e}"),
                })?;
        }
        Ok(removed)
    }
}

#[async_trait]
impl Tool for ScheduleTool {
    fn name(&self) -> &str {
        "schedule"
    }

    fn description(&self) -> &str {
        "Create, list and delete recurring schedules. Operations: \
         create {prompt, schedule, skills?, deliver?, model?}, list {}, \
         delete {id}. 'schedule' accepts cron expressions ('0 9 * * *'), \
         intervals ('every 30m'), RFC3339 timestamps, or relative delays \
         ('2h')."
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
                "schedule": { "type": "string", "description": "Cron expression or interval (create)" },
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
                            "schedule": format!("{:?}", j.schedule),
                            "state": format!("{:?}", j.state),
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
