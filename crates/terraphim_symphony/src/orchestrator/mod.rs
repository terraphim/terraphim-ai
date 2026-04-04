//! Symphony orchestrator.
//!
//! Owns the poll loop, dispatch decisions, reconciliation, and retry scheduling.
//! All state mutations are serialised through this component.

pub mod dispatch;
pub mod reconcile;
pub mod state;

pub use state::{OrchestratorRuntimeState, StateSnapshot};

use crate::config::ServiceConfig;
use crate::config::template::render_prompt;
use crate::error::Result;
use crate::runner::TokenCounts;
use crate::runner::claude_code::ClaudeCodeSession;
use crate::runner::protocol::AgentEvent;
use crate::runner::session::{CodexSession, WorkerOutcome};
use crate::tracker::IssueTracker;
use crate::workspace::WorkspaceManager;

use chrono::Utc;
use state::{LiveSession, RetryEntry, RunningEntry};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Worker exit message sent to the orchestrator.
struct WorkerExit {
    issue_id: String,
    identifier: String,
    outcome: WorkerOutcome,
    started_at: chrono::DateTime<Utc>,
    /// Issue environment variables for after_run hooks.
    issue_env: HashMap<String, String>,
}

/// The main Symphony orchestrator.
pub struct SymphonyOrchestrator {
    state: OrchestratorRuntimeState,
    config: ServiceConfig,
    tracker: Box<dyn IssueTracker>,
    workspace_mgr: WorkspaceManager,
    /// Channel for worker exit notifications.
    worker_exit_tx: mpsc::Sender<WorkerExit>,
    worker_exit_rx: mpsc::Receiver<WorkerExit>,
    /// Channel for agent events.
    agent_event_tx: mpsc::Sender<(String, AgentEvent)>,
    agent_event_rx: mpsc::Receiver<(String, AgentEvent)>,
    /// Channel for retry timer fires.
    retry_fire_tx: mpsc::Sender<String>,
    retry_fire_rx: mpsc::Receiver<String>,
}

impl SymphonyOrchestrator {
    /// Create a new orchestrator.
    pub fn new(
        config: ServiceConfig,
        tracker: Box<dyn IssueTracker>,
        workspace_mgr: WorkspaceManager,
    ) -> Self {
        let (worker_exit_tx, worker_exit_rx) = mpsc::channel(64);
        let (agent_event_tx, agent_event_rx) = mpsc::channel(256);
        let (retry_fire_tx, retry_fire_rx) = mpsc::channel(64);

        let state = OrchestratorRuntimeState::new(
            config.poll_interval_ms(),
            config.max_concurrent_agents(),
        );

        Self {
            state,
            config,
            tracker,
            workspace_mgr,
            worker_exit_tx,
            worker_exit_rx,
            agent_event_tx,
            agent_event_rx,
            retry_fire_tx,
            retry_fire_rx,
        }
    }

    /// Run the orchestrator main loop.
    pub async fn run(&mut self) -> Result<()> {
        // Startup validation
        self.config.validate_for_dispatch()?;

        // Startup terminal workspace cleanup
        self.startup_terminal_cleanup().await;

        let mut poll_tick =
            tokio::time::interval(Duration::from_millis(self.state.poll_interval_ms));
        // Immediate first tick
        poll_tick.tick().await;

        info!(
            poll_interval_ms = self.state.poll_interval_ms,
            max_concurrent = self.state.max_concurrent_agents,
            "orchestrator started"
        );

        loop {
            tokio::select! {
                _ = poll_tick.tick() => {
                    self.on_tick().await;
                }
                Some(exit) = self.worker_exit_rx.recv() => {
                    self.on_worker_exit(exit).await;
                }
                Some((issue_id, event)) = self.agent_event_rx.recv() => {
                    self.on_agent_event(&issue_id, event);
                }
                Some(issue_id) = self.retry_fire_rx.recv() => {
                    self.on_retry_timer(&issue_id).await;
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("shutdown signal received");
                    break;
                }
            }
        }

        self.shutdown_all().await;
        Ok(())
    }

    /// Get a snapshot of the current state.
    pub fn snapshot(&self) -> StateSnapshot {
        self.state.snapshot(Utc::now())
    }

    /// Handle a poll tick.
    async fn on_tick(&mut self) {
        // Reconcile first
        self.reconcile().await;

        // Re-validate config before dispatch
        if let Err(e) = self.config.validate_for_dispatch() {
            warn!("dispatch validation failed: {e}");
            return;
        }

        // Fetch candidates
        let issues = match self.tracker.fetch_candidate_issues().await {
            Ok(issues) => issues,
            Err(e) => {
                warn!("failed to fetch candidate issues: {e}");
                return;
            }
        };

        // Sort and dispatch
        let mut candidates = issues;
        dispatch::sort_for_dispatch(&mut candidates);

        let active_states = self.config.active_states();
        let terminal_states = self.config.terminal_states();
        let per_state_limits = self.config.max_concurrent_agents_by_state();

        for issue in candidates {
            if self.state.available_slots() == 0 {
                break;
            }

            if dispatch::is_dispatch_eligible(
                &issue,
                &self.state,
                &active_states,
                &terminal_states,
                &per_state_limits,
            ) {
                self.dispatch_issue(issue, None).await;
            }
        }
    }

    /// Build environment variables for hook processes from an issue.
    ///
    /// Provides `SYMPHONY_ISSUE_ID`, `SYMPHONY_ISSUE_IDENTIFIER`,
    /// `SYMPHONY_ISSUE_NUMBER`, and `SYMPHONY_ISSUE_TITLE` so that hooks
    /// can create per-issue branches, write meaningful commit messages, etc.
    fn build_issue_env(issue: &crate::tracker::Issue) -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("SYMPHONY_ISSUE_ID".into(), issue.id.clone());
        env.insert("SYMPHONY_ISSUE_IDENTIFIER".into(), issue.identifier.clone());
        env.insert("SYMPHONY_ISSUE_TITLE".into(), issue.title.clone());
        // Extract issue number from identifier (e.g. "owner/repo#7" -> "7")
        let number = issue
            .identifier
            .rsplit_once('#')
            .map(|(_, n)| n.to_string())
            .unwrap_or_default();
        env.insert("SYMPHONY_ISSUE_NUMBER".into(), number);
        env
    }

    /// Dispatch an issue: create workspace, build prompt, spawn worker.
    async fn dispatch_issue(&mut self, issue: crate::tracker::Issue, attempt: Option<u32>) {
        let issue_id = issue.id.clone();
        let identifier = issue.identifier.clone();

        info!(
            issue_id = issue_id,
            issue_identifier = identifier,
            attempt = ?attempt,
            "dispatching issue"
        );

        // Claim the issue
        self.state.claimed.insert(issue_id.clone());
        // Remove from retry queue if present
        if let Some(entry) = self.state.retry_attempts.remove(&issue_id) {
            entry.timer_handle.abort();
        }

        // Build issue environment variables for hooks
        let issue_env = Self::build_issue_env(&issue);

        // Prepare workspace
        let workspace = match self.workspace_mgr.prepare(&identifier, &issue_env).await {
            Ok(ws) => ws,
            Err(e) => {
                error!(
                    issue_identifier = identifier,
                    "workspace preparation failed: {e}"
                );
                self.schedule_retry(
                    &issue_id,
                    &identifier,
                    attempt.unwrap_or(0) + 1,
                    Some(e.to_string()),
                    false,
                );
                return;
            }
        };

        // Run before_run hook
        if let Err(e) = self
            .workspace_mgr
            .run_before_run_hook(&workspace, &issue_env)
            .await
        {
            error!(issue_identifier = identifier, "before_run hook failed: {e}");
            self.schedule_retry(
                &issue_id,
                &identifier,
                attempt.unwrap_or(0) + 1,
                Some(e.to_string()),
                false,
            );
            return;
        }

        // Build prompt
        let prompt = match render_prompt(self.config.prompt_template(), &issue, attempt) {
            Ok(p) => p,
            Err(e) => {
                error!(
                    issue_identifier = identifier,
                    "prompt rendering failed: {e}"
                );
                self.schedule_retry(
                    &issue_id,
                    &identifier,
                    attempt.unwrap_or(0) + 1,
                    Some(e.to_string()),
                    false,
                );
                return;
            }
        };

        // Spawn worker task
        let config = self.config.clone();
        let event_tx = self.agent_event_tx.clone();
        let exit_tx = self.worker_exit_tx.clone();
        let workspace_path = workspace.path.clone();
        let issue_clone = issue.clone();
        let prompt_clone = prompt.clone();
        let started_at = Utc::now();
        let issue_id_clone = issue_id.clone();
        let identifier_clone = identifier.clone();
        let issue_env_clone = issue_env.clone();

        // We need the tracker for multi-turn state checks.
        // Since we can't clone Box<dyn IssueTracker>, we create a channel-based
        // approach where the worker sends state-check requests back.
        // For simplicity in this initial implementation, we skip multi-turn
        // tracker checks in the worker and let the orchestrator handle it
        // via retry continuation.

        let runner_kind = self.config.runner_kind();

        let worker_handle = tokio::spawn(async move {
            let per_issue_event_tx = event_tx.clone();
            let issue_id_for_events = issue_id_clone.clone();

            // Create a dedicated event sender that tags events with the issue ID
            let (session_event_tx, mut session_event_rx) = mpsc::channel::<AgentEvent>(128);

            // Forward events with issue ID tag
            let forward_handle = tokio::spawn({
                let event_tx = per_issue_event_tx.clone();
                let issue_id = issue_id_for_events.clone();
                async move {
                    while let Some(event) = session_event_rx.recv().await {
                        if event_tx.send((issue_id.clone(), event)).await.is_err() {
                            break;
                        }
                    }
                }
            });

            let outcome = match runner_kind.as_str() {
                "claude-code" => {
                    // Claude Code runner: single-shot `claude -p` invocation
                    match ClaudeCodeSession::start(
                        &workspace_path,
                        &config,
                        &prompt_clone,
                        session_event_tx.clone(),
                    )
                    .await
                    {
                        Ok(session) => {
                            let turn_timeout =
                                Duration::from_millis(config.codex_turn_timeout_ms());
                            session
                                .run_to_completion(&session_event_tx, turn_timeout)
                                .await
                        }
                        Err(e) => WorkerOutcome::Failed {
                            reason: e.to_string(),
                            turn_count: 0,
                            tokens: TokenCounts::default(),
                        },
                    }
                }
                _ => {
                    // Codex runner: JSON-RPC handshake + single turn
                    match CodexSession::start(&workspace_path, &config, session_event_tx.clone())
                        .await
                    {
                        Ok(mut session) => {
                            let turn_timeout =
                                Duration::from_millis(config.codex_turn_timeout_ms());

                            match tokio::time::timeout(
                                turn_timeout,
                                session.run_turn_simple(
                                    &prompt_clone,
                                    &issue_clone,
                                    &session_event_tx,
                                ),
                            )
                            .await
                            {
                                Ok(Ok(turn_id)) => {
                                    debug!(turn_id, "single turn completed");
                                    session.stop().await;
                                    WorkerOutcome::Normal {
                                        turn_count: 1,
                                        tokens: session.accumulated_tokens(),
                                    }
                                }
                                Ok(Err(e)) => {
                                    let tokens = session.accumulated_tokens();
                                    session.stop().await;
                                    WorkerOutcome::Failed {
                                        reason: e.to_string(),
                                        turn_count: 1,
                                        tokens,
                                    }
                                }
                                Err(_) => {
                                    let tokens = session.accumulated_tokens();
                                    session.stop().await;
                                    WorkerOutcome::Failed {
                                        reason: format!(
                                            "turn timeout after {}ms",
                                            config.codex_turn_timeout_ms()
                                        ),
                                        turn_count: 1,
                                        tokens,
                                    }
                                }
                            }
                        }
                        Err(e) => WorkerOutcome::Failed {
                            reason: e.to_string(),
                            turn_count: 0,
                            tokens: TokenCounts::default(),
                        },
                    }
                }
            };

            forward_handle.abort();
            let _ = exit_tx
                .send(WorkerExit {
                    issue_id: issue_id_for_events,
                    identifier: identifier_clone,
                    outcome: match &outcome {
                        WorkerOutcome::Normal { turn_count, tokens } => WorkerOutcome::Normal {
                            turn_count: *turn_count,
                            tokens: tokens.clone(),
                        },
                        WorkerOutcome::Failed {
                            reason,
                            turn_count,
                            tokens,
                        } => WorkerOutcome::Failed {
                            reason: reason.clone(),
                            turn_count: *turn_count,
                            tokens: tokens.clone(),
                        },
                    },
                    started_at,
                    issue_env: issue_env_clone,
                })
                .await;

            outcome
        });

        // Track the running entry
        self.state.running.insert(
            issue_id.clone(),
            RunningEntry {
                issue: issue.clone(),
                session: LiveSession::default(),
                started_at,
                retry_attempt: attempt,
                worker_handle,
            },
        );
    }

    /// Handle worker exit.
    async fn on_worker_exit(&mut self, exit: WorkerExit) {
        let issue_id = &exit.issue_id;

        // Calculate runtime from the exit's started_at timestamp
        let elapsed = (Utc::now() - exit.started_at).num_milliseconds().max(0) as f64 / 1000.0;
        self.state.add_runtime_seconds(elapsed);

        // Remove from running
        self.state.running.remove(issue_id);

        // Run after_run hook best-effort
        let ws_key = crate::workspace::sanitise_workspace_key(&exit.identifier);
        let ws_path = self.workspace_mgr.root().join(&ws_key);
        if ws_path.exists() {
            let ws_info = crate::workspace::WorkspaceInfo {
                path: ws_path,
                workspace_key: ws_key,
                created_now: false,
            };
            self.workspace_mgr
                .run_after_run_hook(&ws_info, &exit.issue_env)
                .await;
        }

        match &exit.outcome {
            WorkerOutcome::Normal { turn_count, tokens } => {
                info!(
                    issue_id = exit.issue_id,
                    issue_identifier = exit.identifier,
                    turn_count,
                    "worker completed normally"
                );
                self.state.add_token_totals(tokens);
                self.state.completed.insert(exit.issue_id.clone());

                // Schedule continuation retry
                self.schedule_retry(&exit.issue_id, &exit.identifier, 1, None, true);
            }
            WorkerOutcome::Failed {
                reason,
                turn_count,
                tokens,
            } => {
                warn!(
                    issue_id = exit.issue_id,
                    issue_identifier = exit.identifier,
                    reason,
                    turn_count,
                    "worker failed"
                );
                self.state.add_token_totals(tokens);

                let current_attempt = self
                    .state
                    .running
                    .get(&exit.issue_id)
                    .and_then(|e| e.retry_attempt)
                    .unwrap_or(0);

                self.schedule_retry(
                    &exit.issue_id,
                    &exit.identifier,
                    current_attempt + 1,
                    Some(reason.clone()),
                    false,
                );
            }
        }
    }

    /// Handle an agent event update.
    fn on_agent_event(&mut self, issue_id: &str, event: AgentEvent) {
        if let Some(entry) = self.state.running.get_mut(issue_id) {
            let now = Utc::now();
            entry.session.last_timestamp = Some(now);

            match &event {
                AgentEvent::SessionStarted {
                    session_id,
                    thread_id,
                    turn_id,
                    pid,
                } => {
                    entry.session.session_id = session_id.clone();
                    entry.session.thread_id = thread_id.clone();
                    entry.session.turn_id = turn_id.clone();
                    entry.session.pid = *pid;
                    entry.session.last_event = Some("session_started".into());
                }
                AgentEvent::TurnCompleted {
                    turn_id,
                    turn_count,
                } => {
                    entry.session.turn_id = turn_id.clone();
                    entry.session.turn_count = *turn_count;
                    entry.session.last_event = Some("turn_completed".into());
                }
                AgentEvent::TurnFailed { turn_id, reason } => {
                    entry.session.turn_id = turn_id.clone();
                    entry.session.last_event = Some("turn_failed".into());
                    entry.session.last_message = reason.clone();
                }
                AgentEvent::TokenUsage {
                    input_tokens,
                    output_tokens,
                    total_tokens,
                } => {
                    entry.session.tokens = TokenCounts {
                        input_tokens: *input_tokens,
                        output_tokens: *output_tokens,
                        total_tokens: *total_tokens,
                    };
                }
                AgentEvent::Notification { message } => {
                    entry.session.last_event = Some("notification".into());
                    entry.session.last_message = message.clone();
                }
                AgentEvent::ApprovalAutoApproved { approval_type } => {
                    entry.session.last_event = Some("approval_auto_approved".into());
                    entry.session.last_message = approval_type.clone();
                }
                AgentEvent::UnsupportedToolCall { tool_name } => {
                    entry.session.last_event = Some("unsupported_tool_call".into());
                    entry.session.last_message = tool_name.clone();
                }
                AgentEvent::StartupFailed { reason } => {
                    entry.session.last_event = Some("startup_failed".into());
                    entry.session.last_message = reason.clone();
                }
            }
        }
    }

    /// Handle a retry timer fire.
    async fn on_retry_timer(&mut self, issue_id: &str) {
        let retry_entry = match self.state.retry_attempts.remove(issue_id) {
            Some(e) => e,
            None => return,
        };

        // Fetch current candidates
        let candidates = match self.tracker.fetch_candidate_issues().await {
            Ok(c) => c,
            Err(e) => {
                warn!(issue_id, "retry poll failed: {e}");
                self.schedule_retry(
                    issue_id,
                    &retry_entry.identifier,
                    retry_entry.attempt + 1,
                    Some(format!("retry poll failed: {e}")),
                    false,
                );
                return;
            }
        };

        // Find the specific issue
        let issue = candidates.into_iter().find(|i| i.id == issue_id);

        match issue {
            None => {
                // Issue no longer in candidates - release claim
                debug!(issue_id, "issue not in candidates, releasing claim");
                self.state.claimed.remove(issue_id);
            }
            Some(issue) => {
                if self.state.available_slots() == 0 {
                    self.schedule_retry(
                        issue_id,
                        &issue.identifier,
                        retry_entry.attempt + 1,
                        Some("no available orchestrator slots".into()),
                        false,
                    );
                } else {
                    self.dispatch_issue(issue, Some(retry_entry.attempt)).await;
                }
            }
        }
    }

    /// Schedule a retry for an issue.
    fn schedule_retry(
        &mut self,
        issue_id: &str,
        identifier: &str,
        attempt: u32,
        error: Option<String>,
        is_continuation: bool,
    ) {
        // Cancel any existing retry
        if let Some(old) = self.state.retry_attempts.remove(issue_id) {
            old.timer_handle.abort();
        }

        let delay_ms =
            state::retry_delay_ms(attempt, self.config.max_retry_backoff_ms(), is_continuation);
        let due_at = tokio::time::Instant::now() + Duration::from_millis(delay_ms);

        let issue_id_owned = issue_id.to_string();
        let tx = self.retry_fire_tx.clone();
        let timer_handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            let _ = tx.send(issue_id_owned).await;
        });

        debug!(
            issue_id,
            identifier, attempt, delay_ms, is_continuation, "scheduled retry"
        );

        self.state.retry_attempts.insert(
            issue_id.to_string(),
            RetryEntry {
                issue_id: issue_id.to_string(),
                identifier: identifier.to_string(),
                attempt,
                due_at,
                timer_handle,
                error,
            },
        );
    }

    /// Run reconciliation: stall detection + tracker state refresh.
    async fn reconcile(&mut self) {
        // Part A: Stall detection
        let stall_timeout = self.config.codex_stall_timeout_ms();
        let stalled = reconcile::find_stalled_issues(&self.state, stall_timeout, Utc::now());
        for issue_id in stalled {
            if let Some(entry) = self.state.running.remove(&issue_id) {
                warn!(
                    issue_id,
                    issue_identifier = entry.issue.identifier,
                    "terminating stalled session"
                );
                entry.worker_handle.abort();
                self.schedule_retry(
                    &issue_id,
                    &entry.issue.identifier,
                    entry.retry_attempt.unwrap_or(0) + 1,
                    Some("session stalled".into()),
                    false,
                );
            }
        }

        // Part B: Tracker state refresh
        let running_ids: Vec<String> = self.state.running.keys().cloned().collect();
        if running_ids.is_empty() {
            return;
        }

        let refreshed = match self.tracker.fetch_issue_states_by_ids(&running_ids).await {
            Ok(issues) => issues,
            Err(e) => {
                debug!("state refresh failed, keeping workers: {e}");
                return;
            }
        };

        let active_states = self.config.active_states();
        let terminal_states = self.config.terminal_states();

        for issue in refreshed {
            let action =
                reconcile::determine_action(&issue.state, &active_states, &terminal_states);

            match action {
                reconcile::ReconcileAction::TerminateAndCleanup => {
                    if let Some(entry) = self.state.running.remove(&issue.id) {
                        info!(
                            issue_id = issue.id,
                            issue_identifier = entry.issue.identifier,
                            "stopping run for terminal issue"
                        );
                        entry.worker_handle.abort();
                        self.state.claimed.remove(&issue.id);
                        // Cleanup workspace
                        if let Err(e) = self.workspace_mgr.cleanup(&entry.issue.identifier).await {
                            warn!(
                                issue_identifier = entry.issue.identifier,
                                "workspace cleanup failed: {e}"
                            );
                        }
                    }
                }
                reconcile::ReconcileAction::TerminateNoCleanup => {
                    if let Some(entry) = self.state.running.remove(&issue.id) {
                        info!(
                            issue_id = issue.id,
                            issue_identifier = entry.issue.identifier,
                            state = issue.state,
                            "stopping run for non-active issue"
                        );
                        entry.worker_handle.abort();
                        self.state.claimed.remove(&issue.id);
                    }
                }
                reconcile::ReconcileAction::KeepRunning { new_state } => {
                    if let Some(entry) = self.state.running.get_mut(&issue.id) {
                        if let Some(s) = new_state {
                            entry.issue.state = s;
                        }
                    }
                }
                reconcile::ReconcileAction::StallDetected => {
                    // Already handled above
                }
            }
        }
    }

    /// Startup terminal workspace cleanup.
    async fn startup_terminal_cleanup(&mut self) {
        let terminal_states = self.config.terminal_states();
        match self.tracker.fetch_issues_by_states(&terminal_states).await {
            Ok(issues) => {
                let identifiers: Vec<String> =
                    issues.iter().map(|i| i.identifier.clone()).collect();
                if !identifiers.is_empty() {
                    info!(count = identifiers.len(), "cleaning up terminal workspaces");
                    self.workspace_mgr
                        .cleanup_terminal_workspaces(&identifiers)
                        .await;
                }
            }
            Err(e) => {
                warn!("startup terminal cleanup failed: {e}");
            }
        }
    }

    /// Gracefully shut down all running workers.
    async fn shutdown_all(&mut self) {
        info!(
            running = self.state.running.len(),
            retrying = self.state.retry_attempts.len(),
            "shutting down orchestrator"
        );

        // Abort all retry timers
        for (_, entry) in self.state.retry_attempts.drain() {
            entry.timer_handle.abort();
        }

        // Abort all running workers
        for (issue_id, entry) in self.state.running.drain() {
            info!(issue_id, "aborting running worker");
            entry.worker_handle.abort();
        }

        self.state.claimed.clear();
    }
}
