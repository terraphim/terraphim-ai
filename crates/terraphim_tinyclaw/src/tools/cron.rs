use crate::bus::OutboundMessage;
use crate::config::CronConfig;
use crate::session::{ChatMessage, MessageRole, SessionManager};
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio::time::{Duration, MissedTickBehavior, interval};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_MESSAGE_LEN: usize = 4000;
const MAX_EVERY_SECONDS: u64 = 60 * 60 * 24 * 7;

fn split_session_key(key: &str) -> Option<(&str, &str)> {
    key.split_once(':')
}

fn read_string_arg<'a>(args: &'a Value, primary: &str, secondary: &str) -> Option<&'a str> {
    args.get(primary)
        .and_then(Value::as_str)
        .or_else(|| args.get(secondary).and_then(Value::as_str))
}

fn ensure_same_channel(
    requester: Option<&str>,
    target: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    let Some(requester) = requester else {
        return Ok(());
    };

    let (requester_channel, _) =
        split_session_key(requester).ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Invalid requester session key '{}'", requester),
        })?;
    let (target_channel, _) =
        split_session_key(target).ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: format!("Invalid target session key '{}'", target),
        })?;

    if requester_channel != target_channel {
        return Err(ToolError::Blocked {
            tool: tool_name.to_string(),
            reason: format!(
                "Cross-channel cron targets are blocked (requester: {}, target: {})",
                requester_channel, target_channel
            ),
        });
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CronSchedule {
    At { at: DateTime<Utc> },
    Every { every_seconds: u64 },
}

impl CronSchedule {
    fn first_run_at(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Self::At { at } => Some(at.to_owned()),
            Self::Every { every_seconds } => {
                Some(now + ChronoDuration::seconds(*every_seconds as i64))
            }
        }
    }

    fn next_after_run(
        &self,
        now: DateTime<Utc>,
        previous_next: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        match self {
            Self::At { .. } => None,
            Self::Every { every_seconds } => {
                let step = ChronoDuration::seconds(*every_seconds as i64);
                let mut next = previous_next + step;
                while next <= now {
                    next += step;
                }
                Some(next)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CronJob {
    id: String,
    name: Option<String>,
    session_key: String,
    message: String,
    schedule: CronSchedule,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    next_run_at: Option<DateTime<Utc>>,
    last_run_at: Option<DateTime<Utc>>,
    run_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CronStore {
    version: u32,
    jobs: Vec<CronJob>,
}

impl Default for CronStore {
    fn default() -> Self {
        Self {
            version: 1,
            jobs: Vec::new(),
        }
    }
}

struct CronRuntime {
    storage_path: PathBuf,
    max_jobs: usize,
    jobs: Mutex<Vec<CronJob>>,
}

impl CronRuntime {
    fn load(storage_path: PathBuf, max_jobs: usize) -> Self {
        let jobs = load_store(&storage_path)
            .map(|store| store.jobs)
            .unwrap_or_else(|error| {
                log::warn!(
                    "Failed to load cron store {}: {}",
                    storage_path.display(),
                    error
                );
                Vec::new()
            });

        Self {
            storage_path,
            max_jobs: max_jobs.max(1),
            jobs: Mutex::new(jobs),
        }
    }

    fn persist_jobs(&self, jobs: &[CronJob]) -> anyhow::Result<()> {
        let parent = self
            .storage_path
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(parent)?;

        let store = CronStore {
            version: 1,
            jobs: jobs.to_vec(),
        };
        let payload = serde_json::to_string_pretty(&store)?;
        std::fs::write(&self.storage_path, payload)?;
        Ok(())
    }
}

fn load_store(path: &Path) -> anyhow::Result<CronStore> {
    if !path.exists() {
        return Ok(CronStore::default());
    }

    let content = std::fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(CronStore::default());
    }

    if let Ok(store) = serde_json::from_str::<CronStore>(&content) {
        return Ok(store);
    }

    // Backward-compatible fallback for a plain array payload.
    if let Ok(jobs) = serde_json::from_str::<Vec<CronJob>>(&content) {
        return Ok(CronStore { version: 1, jobs });
    }

    anyhow::bail!("invalid cron store format")
}

async fn dispatch_job(
    job: &CronJob,
    sessions: &Arc<Mutex<SessionManager>>,
    outbound_tx: &mpsc::Sender<OutboundMessage>,
) {
    let Some((channel, chat_id)) = split_session_key(&job.session_key) else {
        log::warn!(
            "Skipping cron job '{}' with invalid session_key '{}'",
            job.id,
            job.session_key
        );
        return;
    };

    {
        let mut sessions_guard = sessions.lock().await;
        let session = sessions_guard.get_or_create(&job.session_key);

        let mut metadata = HashMap::new();
        metadata.insert("source".to_string(), "cron".to_string());
        metadata.insert("cron_job_id".to_string(), job.id.clone());

        session.add_message(ChatMessage {
            role: MessageRole::System,
            content: job.message.clone(),
            sender_id: Some("cron".to_string()),
            timestamp: Utc::now(),
            metadata,
        });

        let snapshot = session.clone();
        if let Err(error) = sessions_guard.save(&snapshot) {
            log::warn!("Failed saving cron session update: {}", error);
        }
    }

    if let Err(error) = outbound_tx
        .send(OutboundMessage::new(channel, chat_id, job.message.clone()))
        .await
    {
        log::warn!("Failed dispatching cron job '{}': {}", job.id, error);
    }
}

async fn scheduler_tick(
    runtime: &Arc<CronRuntime>,
    sessions: &Arc<Mutex<SessionManager>>,
    outbound_tx: &mpsc::Sender<OutboundMessage>,
) {
    let now = Utc::now();
    let mut due_jobs = Vec::new();

    {
        let mut jobs = runtime.jobs.lock().await;
        let mut changed = false;

        for job in jobs.iter_mut() {
            if !job.enabled {
                continue;
            }

            let Some(next_run_at) = job.next_run_at else {
                continue;
            };

            if next_run_at <= now {
                due_jobs.push(job.clone());
                job.run_count += 1;
                job.last_run_at = Some(now);
                job.updated_at = now;
                job.next_run_at = job.schedule.next_after_run(now, next_run_at);
                if job.next_run_at.is_none() {
                    job.enabled = false;
                }
                changed = true;
            }
        }

        if changed {
            if let Err(error) = runtime.persist_jobs(&jobs) {
                log::warn!("Failed persisting cron schedule changes: {}", error);
            }
        }
    }

    for job in due_jobs {
        dispatch_job(&job, sessions, outbound_tx).await;
    }
}

async fn scheduler_loop(
    runtime: Arc<CronRuntime>,
    sessions: Arc<Mutex<SessionManager>>,
    outbound_tx: mpsc::Sender<OutboundMessage>,
    cancel: CancellationToken,
    tick_seconds: u64,
) {
    let mut ticker = interval(Duration::from_secs(tick_seconds.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                break;
            }
            _ = ticker.tick() => {
                scheduler_tick(&runtime, &sessions, &outbound_tx).await;
            }
        }
    }
}

fn parse_schedule(args: &Value, tool_name: &str) -> Result<CronSchedule, ToolError> {
    let now = Utc::now();
    let schedule = args
        .get("schedule")
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: "Missing 'schedule' parameter".to_string(),
        })?;

    let kind = schedule
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: "schedule.kind must be set to 'at' or 'every'".to_string(),
        })?
        .to_ascii_lowercase();

    match kind.as_str() {
        "at" => {
            let at_raw = schedule.get("at").and_then(Value::as_str).ok_or_else(|| {
                ToolError::InvalidArguments {
                    tool: tool_name.to_string(),
                    message: "schedule.at must be an RFC3339 timestamp".to_string(),
                }
            })?;

            let at = chrono::DateTime::parse_from_rfc3339(at_raw)
                .map_err(|error| ToolError::InvalidArguments {
                    tool: tool_name.to_string(),
                    message: format!("Invalid schedule.at value: {}", error),
                })?
                .with_timezone(&Utc);

            if at <= now {
                return Err(ToolError::InvalidArguments {
                    tool: tool_name.to_string(),
                    message: "schedule.at must be in the future".to_string(),
                });
            }

            Ok(CronSchedule::At { at })
        }
        "every" => {
            let every_seconds = schedule
                .get("every_seconds")
                .or_else(|| schedule.get("everySeconds"))
                .and_then(Value::as_u64)
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: tool_name.to_string(),
                    message: "schedule.every_seconds must be an integer".to_string(),
                })?;

            if !(1..=MAX_EVERY_SECONDS).contains(&every_seconds) {
                return Err(ToolError::InvalidArguments {
                    tool: tool_name.to_string(),
                    message: format!(
                        "schedule.every_seconds must be between 1 and {}",
                        MAX_EVERY_SECONDS
                    ),
                });
            }

            Ok(CronSchedule::Every { every_seconds })
        }
        _ => Err(ToolError::InvalidArguments {
            tool: tool_name.to_string(),
            message: "schedule.kind must be 'at' or 'every'".to_string(),
        }),
    }
}

/// Cron orchestration tool with a local scheduler.
pub struct CronTool {
    enabled: bool,
    tick_seconds: u64,
    runtime: Arc<CronRuntime>,
    scheduler_cancel: CancellationToken,
    scheduler_handle: StdMutex<Option<JoinHandle<()>>>,
}

impl CronTool {
    pub fn new(
        config: CronConfig,
        workspace: PathBuf,
        sessions: Arc<Mutex<SessionManager>>,
        outbound_tx: mpsc::Sender<OutboundMessage>,
    ) -> Self {
        let storage_path = config.jobs_path(&workspace);
        let runtime = Arc::new(CronRuntime::load(storage_path, config.max_jobs));
        let scheduler_cancel = CancellationToken::new();
        let tick_seconds = config.tick_seconds.max(1);

        let scheduler_handle = if config.enabled {
            let runtime_clone = runtime.clone();
            let sessions_clone = sessions.clone();
            let outbound_clone = outbound_tx.clone();
            let cancel_signal = scheduler_cancel.clone();
            Some(tokio::spawn(async move {
                scheduler_loop(
                    runtime_clone,
                    sessions_clone,
                    outbound_clone,
                    cancel_signal,
                    tick_seconds,
                )
                .await;
            }))
        } else {
            None
        };

        Self {
            enabled: config.enabled,
            tick_seconds,
            runtime,
            scheduler_cancel,
            scheduler_handle: StdMutex::new(scheduler_handle),
        }
    }

    async fn status(&self) -> Result<String, ToolError> {
        let jobs = self.runtime.jobs.lock().await;
        let total = jobs.len();
        let enabled_count = jobs.iter().filter(|job| job.enabled).count();

        Ok(serde_json::json!({
            "status": "ok",
            "scheduler": if self.enabled { "running" } else { "disabled" },
            "tick_seconds": self.tick_seconds,
            "storage_path": self.runtime.storage_path,
            "jobs_total": total,
            "jobs_enabled": enabled_count,
        })
        .to_string())
    }

    async fn list_jobs(&self, include_disabled: bool) -> Result<String, ToolError> {
        let jobs = self.runtime.jobs.lock().await;
        let mut rows: Vec<&CronJob> = jobs
            .iter()
            .filter(|job| include_disabled || job.enabled)
            .collect();
        rows.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(serde_json::json!({
            "status": "ok",
            "count": rows.len(),
            "jobs": rows,
        })
        .to_string())
    }

    async fn add_job(&self, args: Value) -> Result<String, ToolError> {
        if !self.enabled {
            return Err(ToolError::Blocked {
                tool: self.name().to_string(),
                reason: "cron scheduling is disabled by configuration".to_string(),
            });
        }

        let requester = read_string_arg(&args, "requester_session_key", "requesterSessionKey")
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let target_session_key = read_string_arg(&args, "session_key", "sessionKey")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| requester.map(str::to_string))
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'session_key' (or requester_session_key context) for cron target"
                    .to_string(),
            })?;

        ensure_same_channel(requester, &target_session_key, self.name())?;

        if split_session_key(&target_session_key).is_none() {
            return Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!("Invalid session_key '{}'", target_session_key),
            });
        }

        let message = read_string_arg(&args, "message", "text")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: "Missing 'message' parameter".to_string(),
            })?
            .to_string();

        if message.len() > MAX_MESSAGE_LEN {
            return Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!("message is too long (max {} chars)", MAX_MESSAGE_LEN),
            });
        }

        let schedule = parse_schedule(&args, self.name())?;
        let now = Utc::now();
        let next_run_at =
            schedule
                .first_run_at(now)
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: self.name().to_string(),
                    message: "Unable to compute next run time from schedule".to_string(),
                })?;

        let job = CronJob {
            id: format!("job-{}", Uuid::new_v4().simple()),
            name: read_string_arg(&args, "name", "label").map(str::to_string),
            session_key: target_session_key,
            message,
            schedule,
            enabled: true,
            created_at: now,
            updated_at: now,
            next_run_at: Some(next_run_at),
            last_run_at: None,
            run_count: 0,
        };

        {
            let mut jobs = self.runtime.jobs.lock().await;
            if jobs.len() >= self.runtime.max_jobs {
                return Err(ToolError::Blocked {
                    tool: self.name().to_string(),
                    reason: format!(
                        "cron job limit reached ({}) - remove old jobs first",
                        self.runtime.max_jobs
                    ),
                });
            }

            jobs.push(job.clone());
            self.runtime
                .persist_jobs(&jobs)
                .map_err(|error| ToolError::ExecutionFailed {
                    tool: self.name().to_string(),
                    message: format!("Failed to persist cron store: {}", error),
                })?;
        }

        Ok(serde_json::json!({
            "status": "scheduled",
            "job": job,
        })
        .to_string())
    }

    async fn remove_job(&self, id: &str) -> Result<String, ToolError> {
        let mut removed = None;
        {
            let mut jobs = self.runtime.jobs.lock().await;
            if let Some(position) = jobs.iter().position(|job| job.id == id) {
                removed = Some(jobs.remove(position));
                self.runtime
                    .persist_jobs(&jobs)
                    .map_err(|error| ToolError::ExecutionFailed {
                        tool: self.name().to_string(),
                        message: format!("Failed to persist cron store: {}", error),
                    })?;
            }
        }

        Ok(serde_json::json!({
            "status": "ok",
            "removed": removed.is_some(),
            "job": removed,
        })
        .to_string())
    }
}

impl Drop for CronTool {
    fn drop(&mut self) {
        self.scheduler_cancel.cancel();
        if let Ok(mut guard) = self.scheduler_handle.lock() {
            if let Some(handle) = guard.take() {
                handle.abort();
            }
        }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Manage TinyClaw scheduled reminders (status, list, add, remove)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["status", "list", "add", "remove"],
                    "description": "Cron action to execute"
                },
                "include_disabled": {
                    "type": "boolean",
                    "description": "Include disabled jobs in list action"
                },
                "id": {
                    "type": "string",
                    "description": "Job ID for remove action"
                },
                "session_key": {
                    "type": "string",
                    "description": "Target session key for add action; defaults to requester"
                },
                "message": {
                    "type": "string",
                    "description": "Message payload for scheduled dispatch"
                },
                "schedule": {
                    "type": "object",
                    "description": "Schedule object: { kind: 'at', at: RFC3339 } or { kind: 'every', every_seconds: N }"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let action = read_string_arg(&args, "action", "op")
            .unwrap_or("status")
            .to_ascii_lowercase();

        match action.as_str() {
            "status" => self.status().await,
            "list" => {
                let include_disabled = args
                    .get("include_disabled")
                    .or_else(|| args.get("includeDisabled"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                self.list_jobs(include_disabled).await
            }
            "add" => self.add_job(args).await,
            "remove" => {
                let id = read_string_arg(&args, "id", "job_id")
                    .or_else(|| read_string_arg(&args, "jobId", "job_id"))
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: self.name().to_string(),
                        message: "Missing job 'id' for remove action".to_string(),
                    })?;
                self.remove_job(id).await
            }
            _ => Err(ToolError::InvalidArguments {
                tool: self.name().to_string(),
                message: format!(
                    "Unsupported cron action '{}'. Supported: status, list, add, remove",
                    action
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parse_schedule_every() {
        let schedule = parse_schedule(
            &serde_json::json!({
                "schedule": {
                    "kind": "every",
                    "every_seconds": 5
                }
            }),
            "cron",
        )
        .unwrap();

        assert!(matches!(schedule, CronSchedule::Every { every_seconds: 5 }));
    }

    #[tokio::test]
    async fn test_add_job_requires_enabled_scheduler() {
        let temp_dir = TempDir::new().unwrap();
        let (outbound_tx, _outbound_rx) = mpsc::channel(1);
        let tool = CronTool::new(
            CronConfig {
                enabled: false,
                ..Default::default()
            },
            temp_dir.path().to_path_buf(),
            Arc::new(Mutex::new(SessionManager::new(
                temp_dir.path().join("sessions"),
            ))),
            outbound_tx,
        );

        let err = tool
            .execute(serde_json::json!({
                "action": "add",
                "requester_session_key": "cli:source",
                "message": "hello",
                "schedule": {
                    "kind": "every",
                    "every_seconds": 1
                }
            }))
            .await
            .unwrap_err();

        assert!(matches!(err, ToolError::Blocked { .. }));
    }
}
