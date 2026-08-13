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
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::cron::{CronJob, CronStore, Schedule};

/// Default bound on jobs listed per `list` call.
const LIST_LIMIT: usize = 100;
const AGENT_NAME_PREFIX: &str = "tinyclaw-";
const FRAGMENT_OWNER: &str = "terraphim_tinyclaw.scheduler";
const FRAGMENT_SCHEMA_VERSION: u16 = 1;
const FRAGMENT_MARKER_AGENT_NAME: &str = "tinyclaw-schedule-fragment-marker";
const FRAGMENT_OWNER_CAPABILITY: &str =
    "tinyclaw-schedule-fragment-owner:terraphim_tinyclaw.scheduler";
const FRAGMENT_SCHEMA_CAPABILITY: &str = "tinyclaw-schedule-fragment-schema:1";
const SCHEDULE_CAPABILITY: &str = "tinyclaw-schedule";
const FRAGMENT_LOCK_TIMEOUT: Duration = Duration::from_secs(10);
const FRAGMENT_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(10);

/// The scheduler tool.
pub struct ScheduleTool {
    backend: ScheduleBackend,
}

enum ScheduleBackend {
    Local(CronStore),
    Orchestrator(OrchestratorScheduleStore),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorScheduleFragment {
    #[serde(default)]
    agents: Vec<OrchestratorScheduleAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrchestratorScheduleAgent {
    name: String,
    layer: String,
    cli_tool: String,
    task: String,
    schedule: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    project: Option<String>,
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

impl OrchestratorScheduleFragment {
    fn empty_owned(project: Option<String>) -> Self {
        Self {
            agents: vec![marker_agent(project)],
        }
    }

    fn normalize_and_validate_owned(
        &mut self,
        path: &Path,
        project: Option<&str>,
    ) -> Result<(), ToolError> {
        let marker_count = self
            .agents
            .iter()
            .filter(|agent| agent.name == FRAGMENT_MARKER_AGENT_NAME)
            .count();
        if marker_count > 1 {
            return Err(unowned_fragment_error(
                path,
                format!(
                    "more than one '{FRAGMENT_MARKER_AGENT_NAME}' agent is present ({marker_count}); only one TinyClaw-owned marker is allowed per fragment"
                ),
            ));
        }

        let marker_idx = self
            .agents
            .iter()
            .position(|agent| agent.name == FRAGMENT_MARKER_AGENT_NAME);

        if let Some(idx) = marker_idx {
            validate_marker_agent(path, &self.agents[idx])?;
            normalize_agent_project(path, &mut self.agents[idx], project)?;
        }

        for agent in &mut self.agents {
            if is_marker_agent(agent) {
                continue;
            }
            if !agent.name.starts_with(AGENT_NAME_PREFIX)
                || !agent
                    .capabilities
                    .iter()
                    .any(|capability| capability == SCHEDULE_CAPABILITY)
            {
                return Err(unowned_fragment_error(
                    path,
                    format!(
                        "agent '{}' is not a TinyClaw-owned schedule agent",
                        agent.name
                    ),
                ));
            }
            if !agent
                .capabilities
                .iter()
                .any(|capability| capability == FRAGMENT_OWNER_CAPABILITY)
                || !agent
                    .capabilities
                    .iter()
                    .any(|capability| capability == FRAGMENT_SCHEMA_CAPABILITY)
            {
                return Err(unowned_fragment_error(
                    path,
                    format!(
                        "agent '{}' is missing TinyClaw ownership/schema capabilities",
                        agent.name
                    ),
                ));
            }
            normalize_agent_project(path, agent, project)?;
        }

        if marker_idx.is_none() {
            self.agents
                .insert(0, marker_agent(project.map(ToOwned::to_owned)));
        }
        Ok(())
    }
}

fn marker_agent(project: Option<String>) -> OrchestratorScheduleAgent {
    OrchestratorScheduleAgent {
        name: FRAGMENT_MARKER_AGENT_NAME.to_string(),
        layer: "Core".to_string(),
        cli_tool: "tinyclaw-scheduler-marker".to_string(),
        task: "TinyClaw scheduler fragment ownership marker".to_string(),
        schedule: "0 0 1 1 *".to_string(),
        project,
        model: None,
        skill_chain: Vec::new(),
        capabilities: vec![
            FRAGMENT_OWNER_CAPABILITY.to_string(),
            FRAGMENT_SCHEMA_CAPABILITY.to_string(),
        ],
        enabled: false,
    }
}

fn validate_marker_agent(path: &Path, marker: &OrchestratorScheduleAgent) -> Result<(), ToolError> {
    if marker.enabled {
        return Err(unowned_fragment_error(
            path,
            "TinyClaw ownership marker agent must be disabled",
        ));
    }
    if !marker
        .capabilities
        .iter()
        .any(|capability| capability == FRAGMENT_OWNER_CAPABILITY)
    {
        return Err(unowned_fragment_error(
            path,
            format!("owner marker capability must identify '{FRAGMENT_OWNER}'"),
        ));
    }
    if !marker
        .capabilities
        .iter()
        .any(|capability| capability == FRAGMENT_SCHEMA_CAPABILITY)
    {
        return Err(unowned_fragment_error(
            path,
            format!("schema marker capability must be version {FRAGMENT_SCHEMA_VERSION}"),
        ));
    }
    Ok(())
}

fn normalize_agent_project(
    path: &Path,
    agent: &mut OrchestratorScheduleAgent,
    project: Option<&str>,
) -> Result<(), ToolError> {
    match (project, agent.project.as_deref()) {
        (Some(expected), Some(actual)) if actual != expected => Err(unowned_fragment_error(
            path,
            format!(
                "agent '{}' belongs to project '{}' but scheduler.project is '{}'",
                agent.name, actual, expected
            ),
        )),
        (Some(expected), None) => {
            agent.project = Some(expected.to_string());
            Ok(())
        }
        (Some(_), Some(_)) | (None, None) => Ok(()),
        (None, Some(actual)) => Err(unowned_fragment_error(
            path,
            format!(
                "agent '{}' belongs to project '{}' but scheduler.project is not configured",
                agent.name, actual
            ),
        )),
    }
}

fn is_marker_agent(agent: &OrchestratorScheduleAgent) -> bool {
    agent.name == FRAGMENT_MARKER_AGENT_NAME
}

fn unowned_fragment_error(path: &Path, reason: impl Into<String>) -> ToolError {
    ToolError::ExecutionFailed {
        tool: "schedule".to_string(),
        message: format!(
            "refusing to mutate orchestrator schedule fragment {}: {}; configure a dedicated TinyClaw-owned fragment with a disabled '{FRAGMENT_MARKER_AGENT_NAME}' marker agent carrying capabilities '{FRAGMENT_OWNER_CAPABILITY}' and '{FRAGMENT_SCHEMA_CAPABILITY}'",
            path.display(),
            reason.into()
        ),
    }
}

/// Durable orchestrator schedule fragment store.
#[derive(Debug, Clone)]
pub struct OrchestratorScheduleStore {
    path: PathBuf,
    cli_tool: String,
    project: Option<String>,
}

#[derive(Debug)]
struct FragmentLockGuard {
    file: File,
}

impl Drop for FragmentLockGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

impl OrchestratorScheduleStore {
    /// Create a store writing generated schedule agents to `path`.
    ///
    /// No generated-agent CLI tool is assumed. Calls that create jobs fail
    /// until the caller supplies one with [`Self::with_cli_tool`] or
    /// [`Self::with_cli_tool_and_project`].
    pub fn new(path: PathBuf) -> Self {
        Self::with_cli_tool(path, "")
    }

    /// Create a store with an explicit generated-agent CLI tool.
    pub fn with_cli_tool(path: PathBuf, cli_tool: impl Into<String>) -> Self {
        Self {
            path,
            cli_tool: cli_tool.into(),
            project: None,
        }
    }

    /// Create a store with an explicit generated-agent project id.
    pub fn with_project(path: PathBuf, project: impl Into<String>) -> Self {
        Self::with_cli_tool_and_project(path, "", Some(project.into()))
    }

    /// Create a store with explicit generated-agent CLI tool and project id.
    pub fn with_cli_tool_and_project(
        path: PathBuf,
        cli_tool: impl Into<String>,
        project: Option<String>,
    ) -> Self {
        Self {
            path,
            cli_tool: cli_tool.into(),
            project,
        }
    }

    /// Resolve `self.path` to the single canonical filesystem path used
    /// by load/save/atomic-write/lock-derivation. Two stores addressing
    /// the same fragment through different aliases (file-level symlinks,
    /// parent-directory symlinks, `..`/`.`, redirected parents) collapse
    /// to the same target, so concurrent creates cannot race past the
    /// lock or clobber each other on rename.
    ///
    /// Strategy:
    ///   - If the fragment already exists, canonicalize it so a
    ///     file-level symlink alias resolves to the same inode.
    ///   - If the fragment does not exist yet, create its parent,
    ///     canonicalize the parent (which unifies `..`/`.` and
    ///     resolves parent-directory symlinks), and rejoin the filename.
    ///     Parent creation errors propagate rather than being silently
    ///     ignored: an unresolvable parent means we cannot safely
    ///     serialise writes.
    ///
    /// Residual limitation (documented honestly): POSIX `canonicalize`
    /// does NOT unify hard links. Two stores addressing the same
    /// fragment through different hard-link paths in different
    /// directories will acquire INDEPENDENT adjacent locks and can
    /// race. The lock identity is "adjacent to this canonical entry",
    /// not "this inode". Inode-level locking of the target would be
    /// invalidated by atomic replacement (the canonical entry can be
    /// swapped under us by a concurrent writer), so we do not attempt
    /// it. Callers that need hard-link protection must ensure only
    /// one path spelling reaches the scheduler.
    fn effective_path(&self) -> Result<PathBuf, ToolError> {
        if let Ok(canonical) = self.path.canonicalize() {
            return Ok(canonical);
        }

        let raw_parent = self
            .path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let canonical_parent = std::fs::create_dir_all(&raw_parent)
            .and_then(|_| raw_parent.canonicalize())
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "schedule".to_string(),
                message: format!(
                    "resolve orchestrator schedule fragment parent for {}: {e}",
                    self.path.display()
                ),
            })?;
        let file_name = self
            .path
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| "tinyclaw-schedules.toml".into());
        Ok(canonical_parent.join(file_name))
    }

    /// Adjacent lock file path for a fragment at `canonical`. The lock
    /// sits next to the canonical fragment so both alias stores
    /// observe the same lock file.
    fn lock_path_for(canonical: &Path) -> PathBuf {
        let mut file_name = canonical
            .file_name()
            .map(|name| name.to_os_string())
            .unwrap_or_else(|| "tinyclaw-schedules.toml".into());
        file_name.push(".lock");
        canonical
            .parent()
            .map(|parent| parent.join(&file_name))
            .unwrap_or_else(|| PathBuf::from(file_name))
    }

    fn load_fragment(&self) -> Result<OrchestratorScheduleFragment, ToolError> {
        let canonical = self.effective_path()?;
        if !canonical.exists() {
            return Ok(OrchestratorScheduleFragment::empty_owned(
                self.project.clone(),
            ));
        }
        let content = std::fs::read_to_string(&canonical)?;
        let mut fragment: OrchestratorScheduleFragment =
            toml::from_str(&content).map_err(|e| ToolError::ExecutionFailed {
            tool: "schedule".to_string(),
            message: format!(
                "parse TinyClaw-owned orchestrator schedule fragment {}: {e}; refusing to rewrite because unknown or unowned fields would otherwise be lost",
                canonical.display()
            ),
        })?;
        fragment.normalize_and_validate_owned(&canonical, self.project.as_deref())?;
        Ok(fragment)
    }

    fn save_fragment(&self, fragment: &OrchestratorScheduleFragment) -> Result<(), ToolError> {
        let canonical = self.effective_path()?;
        let content = toml::to_string_pretty(fragment).map_err(|e| ToolError::ExecutionFailed {
            tool: "schedule".to_string(),
            message: format!("serialise orchestrator schedule fragment: {e}"),
        })?;
        atomic_write(&canonical, content.as_bytes())?;
        Ok(())
    }

    fn acquire_fragment_lock(&self) -> Result<FragmentLockGuard, ToolError> {
        let canonical = self.effective_path()?;
        let lock_path = Self::lock_path_for(&canonical);
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        let started = Instant::now();

        loop {
            match FileExt::try_lock_exclusive(&file) {
                Ok(()) => {
                    file.set_len(0)?;
                    file.seek(SeekFrom::Start(0))?;
                    writeln!(
                        file,
                        "pid={} path={}",
                        std::process::id(),
                        canonical.display()
                    )?;
                    file.flush()?;
                    return Ok(FragmentLockGuard { file });
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    if started.elapsed() >= FRAGMENT_LOCK_TIMEOUT {
                        return Err(ToolError::ExecutionFailed {
                            tool: "schedule".to_string(),
                            message: format!(
                                "timed out acquiring orchestrator schedule fragment lock {} after {:?}",
                                lock_path.display(),
                                FRAGMENT_LOCK_TIMEOUT
                            ),
                        });
                    }
                    std::thread::sleep(FRAGMENT_LOCK_RETRY_INTERVAL);
                }
                Err(err) => return Err(err.into()),
            }
        }
    }

    fn mutate_fragment<T>(
        &self,
        mutate: impl FnOnce(&mut OrchestratorScheduleFragment) -> Result<(T, bool), ToolError>,
    ) -> Result<T, ToolError> {
        let _lock = self.acquire_fragment_lock()?;
        let mut fragment = self.load_fragment()?;
        let (result, should_save) = mutate(&mut fragment)?;
        if should_save {
            self.save_fragment(&fragment)?;
        }
        Ok(result)
    }

    fn create_job(
        &self,
        prompt: String,
        schedule_expr: &str,
        skills: Vec<String>,
        deliver: Option<String>,
        model: Option<String>,
    ) -> Result<String, ToolError> {
        let cli_tool = self.cli_tool.trim();
        if cli_tool.is_empty() {
            return Err(ToolError::InvalidArguments {
                tool: "schedule".to_string(),
                message: "scheduler.cli_tool is required for orchestrator-backed schedules; configure a CLI that accepts the task as a positional prompt".to_string(),
            });
        }

        if let Some(deliver) = deliver.as_deref().filter(|value| !value.trim().is_empty()) {
            return Err(ToolError::InvalidArguments {
                tool: "schedule".to_string(),
                message: format!(
                    "deliver target '{deliver}' is not supported by the orchestrator schedule backend; omit deliver or use the local scheduler backend"
                ),
            });
        }

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
            cli_tool: cli_tool.to_string(),
            task: prompt,
            schedule: schedule_expr.to_string(),
            project: self.project.clone(),
            model,
            skill_chain: skills,
            capabilities: vec![
                SCHEDULE_CAPABILITY.to_string(),
                FRAGMENT_OWNER_CAPABILITY.to_string(),
                FRAGMENT_SCHEMA_CAPABILITY.to_string(),
            ],
            enabled: true,
        };
        self.mutate_fragment(|fragment| {
            fragment.agents.push(agent);
            Ok((id, true))
        })
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
        let expected_name = format!("{AGENT_NAME_PREFIX}{id}");
        self.mutate_fragment(|fragment| {
            let before = fragment.agents.len();
            fragment.agents.retain(|agent| agent.name != expected_name);
            let removed = fragment.agents.len() != before;
            Ok((removed, removed))
        })
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
        cfg.validate().map_err(|e| ToolError::InvalidArguments {
            tool: "schedule".to_string(),
            message: e.to_string(),
        })?;
        let path =
            cfg.orchestrator_schedule_file
                .clone()
                .ok_or_else(|| ToolError::BackendUnavailable {
                    tool: "schedule".to_string(),
                    message: "scheduler.orchestrator_schedule_file is required; configure an orchestrator include fragment and include it from orchestrator.toml".to_string(),
                })?;
        Ok(Self::new_orchestrator(
            OrchestratorScheduleStore::with_cli_tool_and_project(
                path,
                cfg.cli_tool.clone(),
                cfg.project.clone(),
            ),
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
        if is_marker_agent(&agent) {
            return None;
        }
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
