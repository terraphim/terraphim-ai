//! Scheduling capability for `AgentOrchestrator`: checking cron and flow
//! schedules, handling schedule events, and reacting to nightwatch drift
//! alerts. Split from lib.rs as part of the Gitea #1910 god-file
//! decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use std::path::PathBuf;

use tracing::{debug, error, info, warn};

use crate::config::AgentDefinition;
use crate::{
    build_flow_project_runtimes, flow, AgentOrchestrator, CorrectionAction, DriftAlert,
    ScheduleEvent,
};

impl AgentOrchestrator {
    /// Check cron schedules and spawn due Core agents.
    pub(crate) async fn check_cron_schedules(&mut self) {
        let now = chrono::Utc::now();
        let scheduled = self.scheduler.scheduled_agents();

        // Collect agents that should fire
        let to_spawn: Vec<(AgentDefinition, chrono::DateTime<chrono::Utc>)> = scheduled
            .into_iter()
            .filter(|(def, _schedule)| {
                // Skip quarantined agents
                !self.quarantined_agents.contains(&def.name)
            })
            .filter(|(def, _schedule)| {
                // Skip if already active
                !self.active_agents.contains_key(&def.name)
            })
            .filter_map(|(def, schedule)| {
                // Get the next fire time after last_tick_time
                let next_fire = schedule.after(&self.last_tick_time).next()?;
                // Check if fire time is within window
                if next_fire > now {
                    return None;
                }
                // Skip if agent already fired at this schedule occurrence
                if let Some(last_fire) = self.last_cron_fire.get(&def.name) {
                    if next_fire <= *last_fire {
                        return None;
                    }
                }
                Some((def.clone(), next_fire))
            })
            .collect();

        for (def, fire_time) in to_spawn {
            info!(agent = %def.name, fire_time = %fire_time, "cron schedule fired");
            // Record fire time before spawning to prevent rapid re-trigger
            self.last_cron_fire.insert(def.name.clone(), fire_time);
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "cron spawn failed");
            }
        }

        // Also check compound review schedule
        if let Some(compound_sched) = self.scheduler.compound_review_schedule() {
            debug!(
                last_tick = %self.last_tick_time,
                last_fired = ?self.last_compound_review_fired_at,
                now = %now,
                "checking compound review schedule"
            );

            // Compute the earliest occurrence strictly after
            // `last_tick_time` that is also <= now. This is the same
            // occurrence the buggy code would have refired forever when
            // the reconcile-tick future was cancelled mid-await by the
            // 90 s `tokio::time::timeout` safety wrapper (#1562).
            let next_fire = compound_sched
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next();
            debug!(next_fire = ?next_fire, "compound schedule next fire");

            if let Some(fire_time) = next_fire {
                // Gate against re-firing the same occurrence. The
                // cursor `last_compound_review_fired_at` is the per-
                // occurrence dedup key, mirroring `last_cron_fire` for
                // per-agent crons. It is updated *before* the `.await`
                // below so a cancelled future cannot lose the update.
                let already_fired = self
                    .last_compound_review_fired_at
                    .map(|prev| fire_time <= prev)
                    .unwrap_or(false);

                if !already_fired {
                    // Record fire time BEFORE awaiting
                    // `handle_schedule_event` so that future
                    // cancellation cannot lose the update and
                    // re-trigger the same occurrence on the next tick.
                    self.last_compound_review_fired_at = Some(fire_time);
                    info!(
                        fire_time = %fire_time,
                        "compound review schedule fired, starting review"
                    );
                    self.handle_schedule_event(ScheduleEvent::CompoundReview)
                        .await;
                }
            }
        }
    }

    /// Check flow schedules and trigger due flows.
    pub(crate) async fn check_flow_schedules(&mut self) {
        let now = chrono::Utc::now();
        let mut to_trigger: Vec<flow::config::FlowDefinition> = Vec::new();

        for flow_def in &self.config.flows {
            let Some(ref schedule_str) = flow_def.schedule else {
                continue;
            };
            let Ok(schedule) = schedule_str.parse::<cron::Schedule>() else {
                continue;
            };

            // Overlap prevention: skip if this flow is already active
            if self.active_flows.contains_key(&flow_def.name) {
                tracing::info!(
                    flow = %flow_def.name,
                    "skipping cron trigger: flow already active"
                );
                continue;
            }

            let should_fire: bool = schedule
                .after(&self.last_tick_time)
                .take_while(|t| *t <= now)
                .next()
                .is_some();

            if should_fire {
                to_trigger.push(flow_def.clone());
            }
        }

        for flow_def in to_trigger {
            self.handle_schedule_event(ScheduleEvent::Flow(Box::new(flow_def)))
                .await;
        }
    }

    /// Handle a schedule event from the TimeScheduler.
    pub(crate) async fn handle_schedule_event(&mut self, event: ScheduleEvent) {
        match event {
            ScheduleEvent::Spawn(def) => {
                info!(agent = %def.name, "scheduled spawn");
                if let Err(e) = self.spawn_agent(&def).await {
                    error!(agent = %def.name, error = %e, "scheduled spawn failed");
                }
            }
            ScheduleEvent::Stop { agent_name } => {
                info!(agent = %agent_name, "scheduled stop");
                self.stop_agent(&agent_name).await;
            }
            ScheduleEvent::CompoundReview => {
                if self.active_compound_review.is_some() {
                    info!("compound review already running, skipping");
                    return;
                }
                info!("scheduled compound review starting (background task)");
                // For scheduled reviews, use HEAD against base_branch from config
                let git_ref = "HEAD".to_string();
                let base_ref = self.config.compound_review.base_branch.clone();
                let workflow = self.compound_workflow.clone();
                let handle = tokio::spawn(async move { workflow.run(&git_ref, &base_ref).await });
                self.active_compound_review = Some(handle);
            }
            ScheduleEvent::Flow(flow_def) => {
                let flow_name = flow_def.name.clone();
                let flow_state_dir = self
                    .config
                    .flow_state_dir
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("/tmp/flow-states"));
                let working_dir = self.config.compound_review.repo_path.clone();
                let project_runtimes = build_flow_project_runtimes(&self.config);
                let flow_def = *flow_def;
                let flow_name_for_closure = flow_name.clone();
                // FlowExecutor contains non-Send types (Regex via AgentSpawner),
                // so we use spawn_blocking + Handle::block_on as a Send-safe bridge.
                let rt_handle = tokio::runtime::Handle::current();
                let handle = tokio::task::spawn_blocking(move || {
                    let executor = flow::executor::FlowExecutor::new(working_dir, flow_state_dir)
                        .with_projects(project_runtimes);
                    rt_handle.block_on(async {
                        executor.run(&flow_def, None).await
                            .unwrap_or_else(|e| {
                                tracing::error!(flow = %flow_name_for_closure, error = %e, "flow execution failed");
                                flow::state::FlowRunState::failed(&flow_name_for_closure, &e.to_string())
                            })
                    })
                });
                self.active_flows.insert(flow_name.clone(), handle);
                tracing::info!(flow = %flow_name, "flow spawned as background task");
            }
        }
    }

    /// Handle a drift alert from the NightwatchMonitor.
    pub(crate) async fn handle_drift_alert(&mut self, alert: DriftAlert) {
        warn!(
            agent = %alert.agent_name,
            score = alert.drift_score.score,
            level = ?alert.drift_score.level,
            "drift alert"
        );

        match alert.recommended_action {
            CorrectionAction::LogWarning(msg) => {
                warn!(agent = %alert.agent_name, message = %msg, "drift warning");
            }
            CorrectionAction::RestartAgent => {
                info!(agent = %alert.agent_name, "restarting agent due to drift");
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);

                // Find definition and respawn
                if let Some(def) = self
                    .config
                    .agents
                    .iter()
                    .find(|a| a.name == alert.agent_name)
                    .cloned()
                {
                    if let Err(e) = self.spawn_agent(&def).await {
                        error!(
                            agent = %alert.agent_name,
                            error = %e,
                            "failed to restart agent after drift correction"
                        );
                    }
                }
            }
            CorrectionAction::PauseAndEscalate(msg) => {
                error!(
                    agent = %alert.agent_name,
                    message = %msg,
                    "CRITICAL: pausing agent and escalating to human"
                );
                self.stop_agent(&alert.agent_name).await;
                self.nightwatch.reset(&alert.agent_name);
            }
        }
    }
}
