//! Reconciliation capability for `AgentOrchestrator`: periodic reconcile tick,
//! budget enforcement, wall-clock timeout polling, agent-exit handling, and
//! per-project circuit breakers. Split from lib.rs as part of the Gitea #1910
//! god-file decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Timelike;
use terraphim_types::*;
use tracing::{debug, error, info, warn};

use crate::agent_run_record::{AgentRunRecord, ExitClass, RunTrigger};
use crate::config::{AgentDefinition, AgentLayer};
use crate::cost_tracker::BudgetVerdict;
use crate::dispatcher::DispatchTask;
#[cfg(feature = "quickwit")]
use crate::quickwit;
use crate::{
    agent_key, control_plane, error_signatures, parse_reset_time, project_control, provider_budget,
    AgentOrchestrator, DEFAULT_RATE_LIMIT_BLOCK,
};

impl AgentOrchestrator {
    /// Periodic reconciliation: detect exits, check cron, evaluate drift, drain output.
    pub(crate) async fn reconcile_tick(&mut self) {
        let tick_start = Instant::now();

        self.provider_rate_limits.clean_expired();
        self.retry_counts
            .retain(|_, (_, ts)| ts.elapsed() < Duration::from_secs(3600));

        // Check wall-clock timeouts and respawn with fallback
        self.poll_wall_timeouts().await;

        // 1. Poll all active agents for exit and handle exits per layer
        self.poll_agent_exits().await;

        // 2. Restart pending Safety agents (cooldown-aware)
        self.restart_pending_safety_agents().await;

        // 3. Check cron schedules for Core agents
        self.check_cron_schedules().await;

        // 4. Drain output events to nightwatch and collect telemetry
        let telemetry_events = self.drain_output_events();
        if !telemetry_events.is_empty() {
            self.record_telemetry(telemetry_events).await;
        }

        // 5. Evaluate nightwatch drift (only during active hours)
        let nw_cfg = &self.config.nightwatch;
        let current_hour = chrono::Local::now().hour() as u8;
        let in_window = if nw_cfg.active_start_hour <= nw_cfg.active_end_hour {
            current_hour >= nw_cfg.active_start_hour && current_hour < nw_cfg.active_end_hour
        } else {
            // Wraps past midnight, e.g. start=22 end=6
            current_hour >= nw_cfg.active_start_hour || current_hour < nw_cfg.active_end_hour
        };
        if in_window {
            self.nightwatch.evaluate();
        }

        // 6. Sweep expired handoff buffer entries
        let swept = self.handoff_buffer.sweep_expired();
        if swept > 0 {
            info!(swept_count = swept, "swept expired handoff buffer entries");
        }

        // 7. Check monthly budget reset
        self.cost_tracker.monthly_reset_if_due();

        // 8. Enforce budget limits (pause exhausted agents)
        self.enforce_budgets().await;

        // 9. Poll active flows (non-blocking)
        let completed_flows: Vec<String> = self
            .active_flows
            .iter()
            .filter(|(_, handle)| handle.is_finished())
            .map(|(name, _)| name.clone())
            .collect();

        for name in completed_flows {
            if let Some(handle) = self.active_flows.remove(&name) {
                match handle.await {
                    Ok(state) => {
                        tracing::info!(flow = %name, status = ?state.status, "flow completed");
                        if let Some(ref dir) = self.config.flow_state_dir {
                            let _ = state.save_to_file(dir);
                        }
                        // Feed cost data from step envelopes into nightwatch for drift detection
                        for envelope in &state.step_envelopes {
                            if let (Some(cost), Some(input), Some(output)) = (
                                envelope.cost_usd,
                                envelope.input_tokens,
                                envelope.output_tokens,
                            ) {
                                self.nightwatch.observe_cost(
                                    &format!("flow-{}", name),
                                    cost,
                                    input,
                                    output,
                                    None, // Flows don't have individual budgets yet
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(flow = %name, error = %e, "flow task panicked");
                    }
                }
            }
        }

        // 9b. Poll active compound review (spawned in background so tick
        //     is not blocked by git worktree ops).
        if let Some(handle) = self.active_compound_review.take() {
            if handle.is_finished() {
                match handle.await {
                    Ok(Ok(result)) => {
                        info!(
                            findings = result.findings.len(),
                            pass = %result.pass,
                            duration = ?result.duration,
                            "compound review completed"
                        );

                        // 1. Post structured summary to Gitea
                        if let (Some(ref poster), Some(issue)) =
                            (&self.output_poster, self.config.compound_review.gitea_issue)
                        {
                            let report = result.format_report();
                            if let Err(e) = poster.post_raw(issue, &report).await {
                                warn!(error = %e, "failed to post compound review summary");
                            }

                            // 2. Auto-file issues for CRITICAL/HIGH findings
                            if self.config.compound_review.auto_file_issues {
                                let actionable = result.actionable_findings();
                                for finding in actionable {
                                    if let Err(e) =
                                        self.file_finding_issue(poster, &result, finding).await
                                    {
                                        warn!(error = %e, "failed to file finding issue");
                                    }
                                }
                            }

                            // 3. Trigger remediation agents for CRITICAL findings
                            if self.config.compound_review.auto_remediate {
                                let critical: Vec<_> = result
                                    .findings
                                    .iter()
                                    .filter(|f| f.severity == FindingSeverity::Critical)
                                    .collect();
                                for finding in critical {
                                    if let Err(e) = self.spawn_remediation_agent(finding).await {
                                        warn!(error = %e, "failed to spawn remediation agent");
                                    }
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!(error = %e, "compound review failed");
                    }
                    Err(e) => {
                        error!(error = %e, "compound review task panicked");
                    }
                }
            } else {
                // Still running, put it back
                self.active_compound_review = Some(handle);
            }
        }

        // 10. Check flow schedules
        self.check_flow_schedules().await;

        // 11. Poll for @adf: mentions in watched issues
        self.poll_mentions().await;

        // 12. D-4: Hot-reload KG routing rules if markdown files changed
        if let Some(ref mut router) = self.kg_router {
            router.reload_if_changed();
        }

        // 13. D-2: Re-probe providers if cached results are stale
        if self.provider_health.is_stale() {
            if let Some(ref kg_router) = self.kg_router {
                self.provider_health.probe_all(kg_router).await;
                if let Some(ref dir) = self
                    .config
                    .routing
                    .as_ref()
                    .and_then(|r| r.probe_results_dir.clone())
                {
                    let _ = self.provider_health.save_results(dir.as_path()).await;
                }

                // Send probe results to Quickwit for cost-aware routing
                if let Some(ref sink) = self.quickwit_sink {
                    let project_id = self
                        .config
                        .projects
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                    self.provider_health
                        .send_to_quickwit(sink, &project_id)
                        .await;
                }
            }
        }

        // 14. Update last_tick_time and increment tick counter
        self.last_tick_time = chrono::Utc::now();
        self.tick_count = self.tick_count.wrapping_add(1);

        // 15. Periodic telemetry persistence (every 60 ticks = ~5 min at 5s interval)
        if self.tick_count % 60 == 0 {
            self.persist_telemetry();
        }

        // 16. Flush provider-budget snapshot. The tracker accumulates in
        // memory via record_telemetry; persist here so hour/day counters
        // carry across restarts (cf. with_persistence at construction).
        // Skip when no tracker was configured.
        if let Some(tracker) = self.provider_budget_tracker.as_ref() {
            if let Err(e) = tracker.persist() {
                warn!(error = %e, "failed to persist provider budget snapshot");
            }
        }

        // Archive stale learnings periodically
        if self.tick_count % self.learning_config.consolidation_ticks == 0 {
            if let Some(ref store) = self.learning_store {
                match tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(store.archive_stale(self.learning_config.archive_days))
                }) {
                    Ok(archived) if archived > 0 => {
                        info!(archived, "archived stale learnings");
                    }
                    Err(e) => {
                        warn!(error = %e, "failed to archive stale learnings");
                    }
                    Ok(_) => {}
                }
            }

            // Evolution memory consolidation (promotes high-importance short-term to long-term).
            if self.evolution_manager.is_enabled() {
                let consolidated = self.evolution_manager.consolidate_all();
                if consolidated > 0 {
                    info!(consolidated, "evolution memory consolidation complete");
                }
            }
        }

        // 17. Drain the unified dispatch queue. Handlers for ReviewPr,
        // AutoMerge, and PostMergeTestGate are stubs; wiring happens in
        // Steps D, G, and H respectively.
        while let Some(task) = self.dispatcher.dequeue() {
            match task {
                DispatchTask::TimeDriven { name, project, .. } => {
                    tracing::debug!(name, project, "TimeDriven task drained from dispatcher");
                }
                DispatchTask::IssueDriven {
                    identifier,
                    project,
                    ..
                } => {
                    tracing::debug!(
                        identifier,
                        project,
                        "IssueDriven task drained from dispatcher"
                    );
                }
                DispatchTask::MentionDriven {
                    agent_name,
                    project,
                    ..
                } => {
                    tracing::debug!(
                        agent_name,
                        project,
                        "MentionDriven task drained from dispatcher"
                    );
                }
                task @ DispatchTask::ReviewPr { .. } => {
                    if let Err(e) = self.handle_review_pr(task).await {
                        warn!(error = %e, "handle_review_pr failed");
                    }
                }
                task @ DispatchTask::AutoMerge { .. } => {
                    if let Err(e) = self.handle_auto_merge(task).await {
                        warn!(error = %e, "handle_auto_merge failed");
                    }
                }
                task @ DispatchTask::PostMergeTestGate { .. } => {
                    if let Err(e) = self.handle_post_merge_test_gate(task).await {
                        tracing::error!(error = ?e, "handle_post_merge_test_gate failed");
                    }
                }
                task @ DispatchTask::Push { .. } => {
                    if let Err(e) = self.handle_push(task).await {
                        warn!(error = %e, "handle_push failed");
                    }
                }
            }
        }

        // 17.5. PR gate reconciliation: read actual commit statuses and
        // branch protection, classify each open PR head, and take action
        // (enqueue missing agents, open remediation issues, or clear for
        // auto-merge). Runs every N ticks to avoid excessive API load.
        if self.tick_count % self.config.gate_reconcile_interval_ticks as u64 == 0 {
            if let Err(e) = self.reconcile_pr_gates().await {
                warn!(error = %e, "reconcile_pr_gates failed");
            }
        }

        // 18. ROC v1 Step F: poll open PRs for reviewer verdicts and enqueue
        // AutoMerge for any PR that clears every gate. Runs AFTER the
        // dispatch drain so the newly enqueued AutoMerge tasks are serviced
        // on the NEXT tick (deterministic ordering).
        if let Err(e) = self.poll_pending_reviews().await {
            warn!(error = %e, "poll_pending_reviews failed");
        }

        let elapsed = tick_start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        if elapsed > std::time::Duration::from_secs(5) {
            warn!(
                tick = self.tick_count,
                elapsed_ms, "reconcile_tick SLOW: took > 5s, likely blocking agent polling"
            );
        } else {
            info!(
                tick = self.tick_count,
                elapsed_ms, "reconcile_tick complete"
            );
        }
    }

    /// Check all agent budgets and pause any that have exceeded their limits.
    async fn enforce_budgets(&mut self) {
        let actionable = self.cost_tracker.check_all();

        for (agent_name, verdict) in actionable {
            match verdict {
                BudgetVerdict::NearExhaustion {
                    spent_cents,
                    budget_cents,
                } => {
                    warn!(
                        agent = %agent_name,
                        spent_usd = spent_cents as f64 / 100.0,
                        budget_usd = budget_cents as f64 / 100.0,
                        pct = (spent_cents * 100 / budget_cents),
                        "budget warning: agent approaching monthly limit"
                    );
                }
                BudgetVerdict::Exhausted {
                    spent_cents,
                    budget_cents,
                } => {
                    error!(
                        agent = %agent_name,
                        spent_usd = spent_cents as f64 / 100.0,
                        budget_usd = budget_cents as f64 / 100.0,
                        "budget exhausted: pausing agent"
                    );
                    self.stop_agent(&agent_name).await;
                }
                _ => {}
            }
        }
    }

    /// Kill agents that have exceeded their wall-clock timeout and respawn with fallback.
    async fn poll_wall_timeouts(&mut self) {
        let mut timed_out: Vec<String> = Vec::new();
        for (name, managed) in &self.active_agents {
            if let Some(max_secs) = managed.definition.max_cpu_seconds {
                let elapsed = managed.started_at.elapsed();
                if elapsed > Duration::from_secs(max_secs) {
                    warn!(
                        agent = %name,
                        elapsed_secs = elapsed.as_secs(),
                        max_cpu_seconds = max_secs,
                        "AGENT EXCEEDED max_cpu_seconds: killing for fallback respawn"
                    );
                    timed_out.push(name.clone());
                }
            }
        }

        for name in timed_out {
            if let Some(managed) = self.active_agents.remove(&name) {
                let def = managed.definition.clone();

                // Kill the process
                if let Err(e) = managed.handle.kill().await {
                    error!(agent = %name, error = %e, "failed to kill timed-out agent");
                }

                // Try respawn with fallback if configured
                if def.fallback_provider.is_some() {
                    info!(
                        agent = %name,
                        fallback_model = ?def.fallback_model,
                        "respawning timed-out agent with fallback provider"
                    );
                    let mut fallback_def = def.clone();
                    // Swap provider to fallback (fallback_provider is a CLI tool path)
                    if let Some(ref fb_provider) = def.fallback_provider {
                        fallback_def.cli_tool = fb_provider.clone();
                    }
                    if let Some(ref fb_model) = def.fallback_model {
                        fallback_def.model = Some(fb_model.clone());
                    }
                    // Clear provider field so model composition uses the new cli_tool
                    fallback_def.provider = None;
                    // Clear fallback to prevent infinite loops
                    fallback_def.fallback_provider = None;
                    fallback_def.fallback_model = None;
                    // Honour the chosen cli_tool/model verbatim: skip KG re-routing
                    // which would otherwise re-pick the primary provider that just
                    // timed out.
                    fallback_def.bypass_kg_routing = true;

                    if let Err(e) = self.spawn_agent(&fallback_def).await {
                        error!(agent = %name, error = %e, "failed to respawn with fallback");
                    }
                } else {
                    info!(agent = %name, "no fallback configured, agent timed out permanently");
                }
            }
        }
    }

    /// Poll all active agents for exit and handle exits per layer.
    pub(crate) async fn poll_agent_exits(&mut self) {
        // Collect exited agents first to avoid borrow conflict
        let mut exited: Vec<(String, AgentDefinition, std::process::ExitStatus)> = Vec::new();
        // Collect agents that exceeded their wall-clock timeout
        let mut timed_out: Vec<String> = Vec::new();

        for (name, managed) in &mut self.active_agents {
            match managed.handle.try_wait() {
                Ok(Some(status)) => {
                    exited.push((name.clone(), managed.definition.clone(), status));
                }
                Ok(None) => {
                    // Still running -- check wall-clock timeout
                    if let Some(max_secs) = managed.definition.max_cpu_seconds {
                        let elapsed = managed.started_at.elapsed();
                        if elapsed > Duration::from_secs(max_secs) {
                            warn!(
                                agent = %name,
                                elapsed_secs = elapsed.as_secs(),
                                max_cpu_seconds = max_secs,
                                "AGENT EXCEEDED max_cpu_seconds: killing"
                            );
                            timed_out.push(name.clone());
                        }
                    }
                }
                Err(e) => {
                    warn!(agent = %name, error = %e, "try_wait failed");
                }
            }
        }

        // Kill timed-out agents
        for name in timed_out {
            if let Some(mut managed) = self.active_agents.remove(&name) {
                let grace = Duration::from_secs(managed.definition.grace_period_secs.unwrap_or(5));
                match managed.handle.shutdown(grace).await {
                    Ok(graceful) => {
                        info!(
                            agent = %name,
                            graceful = graceful,
                            "timed-out agent terminated"
                        );
                    }
                    Err(e) => {
                        warn!(agent = %name, error = %e, "failed to kill timed-out agent");
                    }
                }
                // Handle exit based on layer (similar to handle_agent_exit but for timeout)
                if managed.definition.layer == AgentLayer::Safety {
                    let key = agent_key(&managed.definition);
                    let restart_count = self.increment_restart_count(&key);
                    self.restart_cooldowns.insert(key, Instant::now());
                    info!(
                        agent = %name,
                        restart_count,
                        "safety agent timed out, will restart after cooldown"
                    );
                } else {
                    info!(agent = %name, layer = ?managed.definition.layer, "agent timed out");
                }
            }
        }

        // Drain output from exiting agents BEFORE removing them
        let mut quota_exits: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut exit_telemetry: Vec<(String, control_plane::telemetry::CompletionEvent)> =
            Vec::new();
        let mut drained_outputs: std::collections::HashMap<String, (Vec<String>, String)> =
            std::collections::HashMap::new();
        for (name, def, status) in &exited {
            let mut stdout_lines: Vec<String> = Vec::new();
            let mut stderr_lines: Vec<String> = Vec::new();
            let mut output_lines: Vec<String> = Vec::new();
            let mut agent_tmp_path: Option<PathBuf> = None;
            let mut cli_tool = String::new();
            if let Some(managed) = self.active_agents.get_mut(name) {
                cli_tool = managed.definition.cli_tool.clone();
                let session_id = managed.session_id.clone();
                let model = managed
                    .routed_model
                    .clone()
                    .or_else(|| managed.definition.model.clone())
                    .unwrap_or_default();
                agent_tmp_path = managed.output_tmp_path.take();

                while let Ok(event) = managed.output_rx.try_recv() {
                    self.nightwatch.observe(name, &event);
                    match &event {
                        crate::OutputEvent::Stdout { line, .. } => {
                            stdout_lines.push(line.clone());
                            output_lines.push(line.clone());
                            if let Some(ce) = Self::parse_stdout_for_telemetry(
                                &cli_tool,
                                line,
                                &session_id,
                                &model,
                            ) {
                                exit_telemetry.push((name.clone(), ce));
                            }
                        }
                        crate::OutputEvent::Stderr { line, .. } => {
                            stderr_lines.push(line.clone());
                            output_lines.push(format!("[stderr] {}", line));
                            if let Some(ce) =
                                Self::parse_stderr_for_telemetry(line, &session_id, &model)
                            {
                                exit_telemetry.push((name.clone(), ce));
                            }
                        }
                        _ => {}
                    }
                }
            }

            let gate_drain_lines = agent_tmp_path
                .as_deref()
                .and_then(read_non_empty_log_lines)
                .unwrap_or_else(|| output_lines.clone());
            drained_outputs
                .entry(name.clone())
                .or_insert((gate_drain_lines, cli_tool.clone()));

            // Classify the exit using KG-boosted pattern matching
            let classification =
                self.exit_classifier
                    .classify(status.code(), &stdout_lines, &stderr_lines);

            let wall_time_secs = self
                .active_agents
                .get(name)
                .map(|m| m.started_at.elapsed().as_secs_f64())
                .unwrap_or(0.0);

            let routed_model = self
                .active_agents
                .get(name)
                .and_then(|m| m.routed_model.clone());

            let (mention_chain_id, mention_depth, mention_parent_agent) = self
                .active_agents
                .get(name)
                .map(|m| {
                    (
                        m.mention_chain_id.clone(),
                        m.mention_depth,
                        m.mention_parent_agent.clone(),
                    )
                })
                .unwrap_or((None, None, None));

            let trigger = if self
                .active_agents
                .get(name)
                .is_some_and(|m| m.spawned_by_mention)
            {
                RunTrigger::Mention
            } else {
                RunTrigger::Cron
            };

            let record = AgentRunRecord {
                run_id: uuid::Uuid::new_v4(),
                agent_name: name.clone(),
                started_at: chrono::Utc::now()
                    - chrono::Duration::milliseconds((wall_time_secs * 1000.0) as i64),
                ended_at: chrono::Utc::now(),
                exit_code: status.code(),
                exit_class: classification.exit_class,
                model_used: routed_model.clone().or_else(|| def.model.clone()),
                was_fallback: false,
                wall_time_secs,
                output_summary: AgentRunRecord::summarise_output(&stdout_lines),
                error_summary: AgentRunRecord::summarise_errors(&stderr_lines),
                trigger,
                matched_patterns: classification.matched_patterns.clone(),
                confidence: classification.confidence,
                mention_chain_id,
                mention_depth,
                mention_parent_agent,
                consecutive_config_errors: 0,
            };

            // Config-error circuit-breaker: quarantine after 3 consecutive failures.
            let record = if record.exit_class == ExitClass::ConfigError {
                let count = self
                    .config_error_counters
                    .entry(name.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
                let new_count = *count;
                if new_count >= 3 && !self.quarantined_agents.contains(name.as_str()) {
                    self.quarantined_agents.insert(name.clone());
                    warn!(
                        target: "adf.agent.quarantined",
                        agent = %name,
                        consecutive_config_errors = new_count,
                        "agent quarantined after consecutive ConfigError exits"
                    );
                    if std::env::var("ADF_QUARANTINE_PERSIST").as_deref() == Ok("1") {
                        // TODO: persist enabled=false to conf.d TOML (issue #1817).
                    }
                }
                AgentRunRecord {
                    consecutive_config_errors: new_count,
                    ..record
                }
            } else {
                self.config_error_counters.remove(name.as_str());
                record
            };

            if record.exit_class == ExitClass::RateLimit {
                quota_exits.insert(name.clone());
            }

            info!(
                agent = %name,
                exit_code = ?status.code(),
                exit_class = %record.exit_class,
                confidence = record.confidence,
                matched_patterns = ?record.matched_patterns,
                wall_time_secs = record.wall_time_secs,
                "agent exit classified"
            );

            // Record learning outcome for injected lessons
            if let Some(ref store) = self.learning_store {
                if let Some(ids) = self.injected_learning_ids.get(name) {
                    for id in ids {
                        let result = match record.exit_class {
                            ExitClass::Success | ExitClass::EmptySuccess => {
                                tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current()
                                        .block_on(store.record_effective(id, name))
                                })
                            }
                            ExitClass::Timeout
                            | ExitClass::RateLimit
                            | ExitClass::ModelError
                            | ExitClass::CompilationError
                            | ExitClass::TestFailure
                            | ExitClass::NetworkError
                            | ExitClass::ResourceExhaustion
                            | ExitClass::Crash => tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current()
                                    .block_on(store.record_applied(id, name))
                            }),
                            _ => continue,
                        };
                        if let Err(e) = result {
                            debug!(agent = %name, learning_id = %id, error = %e, "failed to record learning outcome");
                        }
                    }
                }
                self.injected_learning_ids.remove(name);
            }

            let mut quota_provider_recorded: Option<String> = None;
            if record.exit_class == ExitClass::RateLimit {
                let stderr_text = stderr_lines.join("\n");
                let output_text = output_lines.join("\n");
                let _secondary_confirms =
                    control_plane::output_parser::parse_stderr_for_limit_errors(&stderr_text)
                        .is_some()
                        || control_plane::telemetry::is_subscription_limit_error(&output_text);
                // Attribution: prefer routed model -> canonical key, fallback to config provider
                let effective_provider = routed_model
                    .as_deref()
                    .map(provider_budget::canonical_key_for_model_or_provider)
                    .or_else(|| {
                        def.provider
                            .as_deref()
                            .map(provider_budget::canonical_quota_key)
                    });

                if let Some(provider_key) = effective_provider {
                    warn!(
                        agent = %name,
                        provider = %provider_key,
                        model = ?routed_model,
                        "quota exit detected; recording provider failure and blocking"
                    );
                    self.provider_health.record_failure(provider_key);
                    if let Some(tracker) = self.provider_budget_tracker.as_ref() {
                        tracker.force_exhaust(provider_key);
                    }

                    let quota_line = stderr_lines
                        .iter()
                        .chain(stdout_lines.iter())
                        .find(|l| l.to_lowercase().contains("resets "))
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if let Some(reset_time) = parse_reset_time(quota_line) {
                        info!(
                            provider = %provider_key,
                            "blocking provider until rate-limit window expires"
                        );
                        self.provider_rate_limits
                            .block_until(provider_key, reset_time);
                    } else {
                        // Safety floor: classifier says RateLimit but no parseable
                        // reset window in stderr. Apply a conservative block so the
                        // next cron firing skips this provider while we iterate on
                        // the parser. The warn-log carries a representative stderr
                        // tail so future format drift is discoverable.
                        let stderr_sample: String = stderr_lines
                            .iter()
                            .rev()
                            .take(3)
                            .rev()
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" | ");
                        warn!(
                            provider = %provider_key,
                            block_secs = DEFAULT_RATE_LIMIT_BLOCK.as_secs(),
                            stderr_tail = %stderr_sample,
                            "rate-limit detected but reset window unparseable; applying conservative safety-floor block"
                        );
                        self.provider_rate_limits
                            .block_until(provider_key, Instant::now() + DEFAULT_RATE_LIMIT_BLOCK);
                    }
                    quota_provider_recorded = Some(provider_key.to_string());
                }
            }

            // D-3: Feed exit classification into provider health circuit breaker
            if let Some(ref provider) = def.provider {
                let canonical = provider_budget::canonical_quota_key(provider);
                let already_recorded_by_quota =
                    quota_provider_recorded.as_deref() == Some(canonical);
                match record.exit_class {
                    ExitClass::ModelError | ExitClass::RateLimit =>
                    {
                        #[allow(clippy::collapsible_match)]
                        if !already_recorded_by_quota {
                            self.provider_health.record_failure(canonical);
                        }
                    }
                    ExitClass::Success | ExitClass::EmptySuccess => {
                        self.provider_health.record_success(canonical);
                    }
                    _ => {} // Other exit classes don't affect provider health
                }

                // issue #7: per-provider stderr-signature classification. This
                // runs on top of the KG-driven ExitClass match above so that
                // providers whose CLI exits 0 on quota hits ("returning partial
                // output") still trip the breaker and force budget exhaustion
                // when their stderr matches a configured throttle pattern.
                let sigs = self.provider_error_signatures.get(provider);
                let sig_kind = error_signatures::classify_lines(&stderr_lines, sigs);
                match sig_kind {
                    error_signatures::ErrorKind::Throttle => {
                        warn!(
                            agent = %name,
                            provider = %canonical,
                            model = ?record.model_used,
                            "stderr classified as throttle; tripping breaker + exhausting budget"
                        );
                        self.provider_health.record_failure(canonical);
                        if let Some(tracker) = self.provider_budget_tracker.as_ref() {
                            tracker.force_exhaust(canonical);
                        }
                    }
                    error_signatures::ErrorKind::Flake => {
                        info!(
                            agent = %name,
                            provider = %provider,
                            "stderr classified as flake; routing will retry next pool entry"
                        );
                    }
                    error_signatures::ErrorKind::Unknown => {
                        // Only escalate when there *is* stderr text to classify
                        // AND the run itself looks like a real failure. A clean
                        // exit with empty stderr must never page fleet-meta.
                        let looked_like_failure = !matches!(
                            record.exit_class,
                            ExitClass::Success | ExitClass::EmptySuccess
                        );
                        if looked_like_failure && !stderr_lines.is_empty() {
                            // Soft-failure accounting so a pathological provider
                            // that spews unclassified errors still eventually
                            // opens the breaker.
                            self.provider_health.record_failure(provider);
                            self.escalate_unknown_error(
                                provider,
                                record.model_used.as_deref(),
                                &stderr_lines,
                            )
                            .await;
                        }
                    }
                }
            }

            // Post output to Gitea if configured, routed by the agent's
            // owning project so multi-project fleets land comments in the
            // correct owner/repo.
            if let (Some(poster), Some(issue)) = (&self.output_poster, def.gitea_issue) {
                if def.event_only {
                    // Defence-in-depth: the dispatch gate at handle_webhook_dispatch
                    // should have prevented an event-only agent from acquiring a
                    // gitea_issue. If we reach here the gate has a hole; treat as a
                    // should-never-happen alert and skip the post.
                    error!(
                        agent = %name,
                        issue = issue,
                        "skipping output post: agent is event-only but gitea_issue is set; this indicates a missed dispatch gate"
                    );
                } else {
                    let exit_code = status.code();
                    let project = def
                        .project
                        .clone()
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                    if let Err(e) = poster
                        .post_agent_output_for_project(
                            &project,
                            name,
                            issue,
                            &output_lines,
                            exit_code,
                        )
                        .await
                    {
                        warn!(
                            agent = %name,
                            project = %project,
                            issue = issue,
                            error = %e,
                            "failed to post output to Gitea"
                        );
                    }
                }
            }

            // Finalise the agent output log: prepend header to the temp file
            // written by the background drain task, then rename to final path.
            {
                let _ = std::fs::create_dir_all(&self.agent_log_dir);
                let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
                let filename = format!("{}-{}.log", name, ts);
                let final_path = self.agent_log_dir.join(&filename);

                if let Some(ref tmp_path) = agent_tmp_path {
                    // The drain task wrote raw output lines.  Read them back,
                    // prepend the header, and write the final file.
                    let body = std::fs::read_to_string(tmp_path).unwrap_or_default();
                    let header = format!(
                        "# agent: {}\n# exit_code: {:?}\n# exit_class: {}\n# wall_time: {:.1}s\n# model: {}\n\n",
                        name,
                        status.code(),
                        record.exit_class,
                        record.wall_time_secs,
                        record.model_used.as_deref().unwrap_or("n/a"),
                    );
                    let final_content = format!("{}{}", header, body);
                    if let Err(e) = std::fs::write(&final_path, &final_content) {
                        warn!(agent = %name, path = %final_path.display(), error = %e, "failed to write agent output log");
                    } else {
                        debug!(agent = %name, path = %final_path.display(), "wrote agent output log");
                    }
                    let _ = std::fs::remove_file(tmp_path);
                } else {
                    // Fallback: no drain task (shouldn't happen), write from
                    // the in-memory output_lines instead.
                    let mut content = String::with_capacity(output_lines.len() * 120);
                    content.push_str(&format!(
                        "# agent: {}\n# exit_code: {:?}\n# exit_class: {}\n# wall_time: {:.1}s\n# model: {}\n\n",
                        name,
                        status.code(),
                        record.exit_class,
                        record.wall_time_secs,
                        record.model_used.as_deref().unwrap_or("n/a"),
                    ));
                    for line in &output_lines {
                        content.push_str(line);
                        content.push('\n');
                    }
                    if let Err(e) = std::fs::write(&final_path, &content) {
                        warn!(agent = %name, path = %final_path.display(), error = %e, "failed to write agent output log (fallback)");
                    }
                }
            }

            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let exit_code = status.code();
                let level = if exit_code.unwrap_or(1) == 0 {
                    "INFO"
                } else {
                    "WARN"
                };
                let doc = quickwit::LogDocument {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    project_id: def
                        .project
                        .clone()
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
                    level: level.into(),
                    agent_name: name.clone(),
                    layer: format!("{:?}", def.layer),
                    source: "orchestrator".into(),
                    message: format!("agent exited: {}", record.exit_class),
                    model: routed_model.clone().or_else(|| def.model.clone()),
                    exit_code,
                    wall_time_secs: Some(record.wall_time_secs),
                    extra: Some(serde_json::json!({
                        "exit_class": record.exit_class.to_string(),
                        "confidence": record.confidence,
                        "matched_patterns": record.matched_patterns,
                    })),
                    ..Default::default()
                };
                let _ = sink.send(doc).await;
            }
        }

        // Record telemetry from exiting agent output
        if !exit_telemetry.is_empty() {
            self.record_telemetry(exit_telemetry).await;
        }

        // Process natural exits

        // NOW remove from active_agents and handle exits.
        // Capture worktree_path before removing so we can commit + clean up.
        for (name, def, status) in exited {
            let (worktree_path, commit_status_post, gate_meta) = {
                let agent = self.active_agents.get(&name);
                (
                    agent.and_then(|m| m.worktree_path.clone()),
                    agent.and_then(|m| m.commit_status_post.clone()),
                    agent.and_then(|m| m.gate_meta.clone()),
                )
            };

            // Post terminal commit status if this agent was dispatched with one.
            let mut status_override: Option<(terraphim_tracker::StatusState, String)> = None;
            if let Some((ref head_sha, ref context)) = commit_status_post {
                if let Some(ref meta) = gate_meta {
                    if meta.context == *context && meta.head_sha == *head_sha {
                        let (drain_lines, drain_cli) = drained_outputs
                            .get(&name)
                            .map(|(lines, cli)| (lines.as_slice(), cli.as_str()))
                            .unwrap_or((&[], ""));
                        if let Some(override_status) = self
                            .derive_pr_gate_status(
                                meta,
                                drain_lines,
                                drain_cli,
                                self.output_poster.as_ref(),
                            )
                            .await
                        {
                            status_override = Some(override_status);
                        }
                    }
                }

                let (state, description) = if let Some((state, desc)) = status_override.take() {
                    (state, desc)
                } else {
                    let exit_success = status.success();
                    let state = if exit_success {
                        terraphim_tracker::StatusState::Success
                    } else {
                        terraphim_tracker::StatusState::Failure
                    };
                    let description = if exit_success {
                        format!("{} passed", def.name)
                    } else {
                        format!("{} failed (exit {})", def.name, status.code().unwrap_or(-1))
                    };
                    (state, description)
                };
                self.post_terminal_commit_status(head_sha, context, state, &description)
                    .await;
            }

            // Disarm worktree guard on success so it doesn't conflict with
            // the explicit cleanup below.
            if status.success() {
                if let Some(agent) = self.active_agents.get_mut(&name) {
                    if let Some(guard) = agent.worktree_guard.take() {
                        guard.keep();
                    }
                }
            }

            self.active_agents.remove(&name);

            if quota_exits.contains(&name) {
                let mut local_unhealthy = self.provider_health.unhealthy_providers();
                local_unhealthy.extend(self.provider_rate_limits.blocked_providers());

                let respawned = if let Some(ref kg_router) = self.kg_router {
                    if let Some(decision) =
                        kg_router.route_agent_with_tier(&def.task, def.default_tier.as_deref())
                    {
                        if let Some(healthy_route) = decision.first_healthy_route(&local_unhealthy)
                        {
                            let retry_count = self
                                .retry_counts
                                .entry(name.clone())
                                .or_insert((0, Instant::now()));
                            if retry_count.0 < 3 {
                                retry_count.0 += 1;
                                retry_count.1 = Instant::now();
                                let retry_name = format!("{}-retry-{}", name, retry_count.0);
                                let mut fallback_def = def.clone();
                                fallback_def.name = retry_name;
                                fallback_def.model = Some(healthy_route.model.clone());
                                fallback_def.provider = Some(healthy_route.provider.clone());
                                // Extract CLI tool from the action template so the retry uses
                                // the correct CLI for the fallback provider (e.g. opencode for kimi).
                                if let Some(ref action) = healthy_route.action {
                                    if let Some(cli) = action.split_whitespace().next() {
                                        fallback_def.cli_tool = cli.to_string();
                                    }
                                }
                                fallback_def.fallback_provider = None;
                                fallback_def.fallback_model = None;
                                // KG router already filtered for healthy routes;
                                // re-running KG selection inside spawn_agent could
                                // drift if breaker state has changed (e.g. a probe
                                // succeeded). Lock the chosen route in.
                                fallback_def.bypass_kg_routing = true;
                                info!(
                                    agent = %name,
                                    retry_name = %fallback_def.name,
                                    fallback_model = %healthy_route.model,
                                    fallback_provider = %healthy_route.provider,
                                    "respawning quota-exited agent via KG fallback route"
                                );
                                if let Err(e) = self.spawn_agent(&fallback_def).await {
                                    error!(agent = %fallback_def.name, error = %e, "failed to spawn KG fallback agent");
                                }
                                true
                            } else {
                                warn!(agent = %name, retries = retry_count.0, "max retries reached, agent exits permanently");
                                self.retry_counts.remove(&name);
                                false
                            }
                        } else {
                            info!(agent = %name, "no healthy KG route available, agent exits permanently");
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };

                if !respawned {
                    // KG routing failed or no healthy route - try configured fallback before giving up
                    if def.fallback_provider.is_some() {
                        info!(
                            agent = %name,
                            fallback_provider = ?def.fallback_provider,
                            fallback_model = ?def.fallback_model,
                            "KG routing failed, respawning with configured fallback provider"
                        );
                        let mut fallback_def = def.clone();
                        if let Some(ref fb_provider) = def.fallback_provider {
                            fallback_def.cli_tool = fb_provider.clone();
                        }
                        if let Some(ref fb_model) = def.fallback_model {
                            fallback_def.model = Some(fb_model.clone());
                        }
                        fallback_def.provider = None;
                        fallback_def.fallback_provider = None;
                        fallback_def.fallback_model = None;
                        // The whole point of this branch is to escape KG routing
                        // (it just told us "no healthy route"). Honour the
                        // operator-chosen cli_tool/model verbatim instead of
                        // letting spawn_agent re-evaluate KG and re-pick the
                        // primary that drove us here.
                        fallback_def.bypass_kg_routing = true;
                        if let Err(e) = self.spawn_agent(&fallback_def).await {
                            error!(agent = %name, error = %e, "failed to respawn with fallback");
                            self.handle_agent_exit(&name, &def, status);
                        }
                    } else {
                        self.handle_agent_exit(&name, &def, status);
                    }
                }
            } else {
                self.handle_agent_exit(&name, &def, status);
            }
            self.record_project_meta_exit(&def, status).await;

            // Evolution: record lesson and snapshot on agent exit.
            if def.evolution_enabled && self.evolution_manager.is_enabled() {
                #[cfg(feature = "evolution")]
                {
                    let exit_desc = format!("Agent {} exited with status: {}", name, status);
                    let exit_code = status.code().unwrap_or(-1);
                    let category = if status.success() {
                        terraphim_agent_evolution::LessonCategory::SuccessPattern
                    } else {
                        terraphim_agent_evolution::LessonCategory::Failure
                    };
                    let _ = self.evolution_manager.record_lesson(
                        &name,
                        &format!("{} run completed", name),
                        &exit_desc,
                        &format!("Exit code: {}", exit_code),
                        category,
                    );
                }
                if let Some(snapshot_key) = self.evolution_manager.snapshot_on_exit(&name) {
                    info!(agent = %name, key = %snapshot_key, "evolution snapshot created");
                }
            }

            // Auto-commit in the agent's working directory (worktree or shared)
            let project_repo: &Path = def
                .project
                .as_deref()
                .and_then(|pid| self.config.project_by_id(pid))
                .map(|p| p.working_dir.as_path())
                .unwrap_or(&self.config.working_dir);
            let commit_dir = worktree_path.as_deref().unwrap_or(project_repo);
            if status.success() {
                self.try_commit_agent_work(&name, commit_dir).await;
            }

            // Clean up the worktree after committing
            if let Some(ref wt) = worktree_path {
                self.remove_agent_worktree(&name, wt, project_repo).await;
            }
        }
    }

    /// Feed a `project-meta` exit into the per-project circuit breaker. On
    /// the trip transition (Nth consecutive failure) this touches the pause
    /// flag and opens an `[ADF]` escalation issue on the configured
    /// fleet-escalation repository.
    async fn record_project_meta_exit(
        &mut self,
        def: &AgentDefinition,
        status: std::process::ExitStatus,
    ) {
        if !project_control::is_project_meta_agent(def) {
            return;
        }
        let Some(project_id) = def.project.clone() else {
            return;
        };
        let success = status.success();
        let verdict = self
            .project_failure_counter
            .record_project_meta_result(&project_id, success);
        let count = self.project_failure_counter.count(&project_id);
        let threshold = self.project_failure_counter.threshold();
        info!(
            project = %project_id,
            agent = %def.name,
            success,
            consecutive_failures = count,
            threshold,
            "project-meta exit recorded"
        );
        if verdict != project_control::ShouldPause::Yes {
            return;
        }
        self.trip_project_circuit_breaker(&project_id, &def.name, count, threshold)
            .await;
    }

    /// Touch the pause flag and open an `[ADF]` escalation issue for a
    /// project that has exceeded the `project-meta` failure threshold.
    async fn trip_project_circuit_breaker(
        &self,
        project_id: &str,
        agent_name: &str,
        consecutive_failures: u32,
        threshold: u32,
    ) {
        match project_control::touch_pause_flag(&self.pause_dir, project_id) {
            Ok(path) => {
                warn!(
                    project = %project_id,
                    pause_flag = %path.display(),
                    consecutive_failures,
                    threshold,
                    "project circuit breaker tripped; pause flag created"
                );
            }
            Err(e) => {
                error!(
                    project = %project_id,
                    pause_dir = %self.pause_dir.display(),
                    error = %e,
                    "failed to create project pause flag"
                );
            }
        }

        let (Some(owner), Some(repo)) = self.fleet_escalation_target() else {
            warn!(
                project = %project_id,
                "no fleet-escalation owner/repo configured; skipping issue creation"
            );
            return;
        };
        let Some(gitea_cfg) = self.config.gitea.as_ref() else {
            warn!(
                project = %project_id,
                "no top-level gitea config; cannot open escalation issue"
            );
            return;
        };
        let tracker_cfg = terraphim_tracker::GiteaConfig {
            base_url: gitea_cfg.base_url.clone(),
            token: gitea_cfg.token.clone(),
            owner: owner.clone(),
            repo: repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
            Ok(t) => t,
            Err(e) => {
                error!(
                    project = %project_id,
                    owner = %owner,
                    repo = %repo,
                    error = %e,
                    "failed to construct escalation GiteaTracker"
                );
                return;
            }
        };
        let title = format!(
            "[ADF] project-meta on {project_id} failed {consecutive_failures} consecutively"
        );
        let body = format!(
            "Project `{project_id}` has been paused by the circuit breaker after \
{consecutive_failures} consecutive `project-meta` failures (threshold: {threshold}).\n\n\
Agent: `{agent_name}`\n\n\
Pause flag: `{pause}`\n\n\
Remove the pause flag once the underlying failure is resolved:\n\n\
```\nrm {pause}\n```\n",
            pause = self.pause_dir.join(project_id).display()
        );
        let labels = ["adf", "circuit-breaker", "priority/high"];
        match tracker.create_issue(&title, &body, &labels).await {
            Ok(issue) => info!(
                project = %project_id,
                owner = %owner,
                repo = %repo,
                issue = issue.number,
                "opened [ADF] escalation issue"
            ),
            Err(e) => error!(
                project = %project_id,
                owner = %owner,
                repo = %repo,
                error = %e,
                "failed to open [ADF] escalation issue"
            ),
        }
    }

    /// Resolve the `(owner, repo)` pair for fleet-level escalation issues.
    /// Falls back to the configured Gitea output target when the dedicated
    /// `fleet_escalation_*` fields are unset.
    fn fleet_escalation_target(&self) -> (Option<String>, Option<String>) {
        let owner = self
            .config
            .fleet_escalation_owner
            .clone()
            .or_else(|| self.config.gitea.as_ref().map(|g| g.owner.clone()));
        let repo = self
            .config
            .fleet_escalation_repo
            .clone()
            .or_else(|| self.config.gitea.as_ref().map(|g| g.repo.clone()));
        (owner, repo)
    }

    /// Handle an agent exit based on its layer.
    fn handle_agent_exit(
        &mut self,
        name: &str,
        def: &AgentDefinition,
        status: std::process::ExitStatus,
    ) {
        let key = agent_key(def);
        match def.layer {
            AgentLayer::Safety => {
                // Only count non-zero exits toward restart limit.
                // A successful exit (code 0) means the agent completed its task;
                // punishing it for succeeding makes no sense.
                if !status.success() {
                    let restart_count = self.increment_restart_count(&key);
                    self.restart_cooldowns.insert(key, Instant::now());
                    if restart_count <= self.config.max_restart_count {
                        info!(
                            agent = %name,
                            exit_status = %status,
                            restart_count,
                            cooldown_secs = self.config.restart_cooldown_secs,
                            window_secs = self.config.restart_budget_window_secs,
                            "safety agent failed, will restart after cooldown"
                        );
                    } else {
                        error!(
                            agent = %name,
                            exit_status = %status,
                            restart_count,
                            max = self.config.max_restart_count,
                            "safety agent exceeded max restarts, permanently stopped"
                        );
                    }
                } else {
                    self.restart_cooldowns.insert(key, Instant::now());
                    info!(
                        agent = %name,
                        exit_status = %status,
                        cooldown_secs = self.config.restart_cooldown_secs,
                        "safety agent completed successfully, will restart after cooldown"
                    );
                }
            }
            AgentLayer::Core => {
                info!(agent = %name, exit_status = %status, "core agent completed");
            }
            AgentLayer::Growth => {
                info!(agent = %name, exit_status = %status, "growth agent completed");
            }
        }
    }

    /// Restart Safety agents that have exited and passed their cooldown.
    pub(crate) async fn restart_pending_safety_agents(&mut self) {
        let cooldown = Duration::from_secs(self.config.restart_cooldown_secs);
        let max_restarts = self.config.max_restart_count;

        // Age out stale restart counters before restart eligibility checks.
        let safety_keys: Vec<(String, String)> = self
            .config
            .agents
            .iter()
            .filter(|def| def.layer == AgentLayer::Safety)
            .map(agent_key)
            .collect();
        for key in &safety_keys {
            let _ = self.current_restart_count(key);
        }

        // Find Safety agents that need restarting
        let to_restart: Vec<AgentDefinition> = self
            .config
            .agents
            .iter()
            .filter(|def| {
                // Must be Safety layer
                if def.layer != AgentLayer::Safety {
                    return false;
                }
                // Must not be currently active
                if self.active_agents.contains_key(&def.name) {
                    return false;
                }
                let key = agent_key(def);
                // Must have a restart cooldown entry (meaning it exited)
                let last_exit = match self.restart_cooldowns.get(&key) {
                    Some(t) => t,
                    None => return false,
                };
                // Must have passed the cooldown
                if last_exit.elapsed() < cooldown {
                    return false;
                }
                // Must be under max restart count
                let count = self.restart_counts.get(&key).copied().unwrap_or(0);
                count <= max_restarts
            })
            .cloned()
            .collect();

        for def in to_restart {
            let key = agent_key(&def);
            info!(
                agent = %def.name,
                restart_count = self.restart_counts.get(&key).copied().unwrap_or(0),
                "restarting safety agent after cooldown"
            );
            if let Err(e) = self.spawn_agent(&def).await {
                error!(agent = %def.name, error = %e, "failed to restart safety agent");
            }
        }
    }

    /// Derive a terminal commit status override for a PR gate agent.
    ///
    /// Returns `None` when the agent has no `gate_meta` (handled by the
    /// caller falling back to exit-code status) or when the gate result
    /// cannot be extracted/validated (treated as gate failure to keep
    /// branch protection trustworthy).
    async fn derive_pr_gate_status(
        &self,
        meta: &crate::pr_gate_result::PrGateMeta,
        drain_lines: &[String],
        cli_tool: &str,
        output_poster: Option<&crate::output_poster::OutputPoster>,
    ) -> Option<(terraphim_tracker::StatusState, String)> {
        let GateEvaluation {
            status,
            comment_body,
        } = derive_pr_gate_status_from_output(meta, drain_lines, cli_tool);

        if let Some(poster) = output_poster.filter(|_| !comment_body.trim().is_empty()) {
            if let Err(e) = poster
                .post_raw_as_agent_for_project(
                    meta.project.as_str(),
                    meta.agent_name.as_str(),
                    meta.pr_number,
                    &comment_body,
                )
                .await
            {
                warn!(
                    project = %meta.project,
                    agent = %meta.agent_name,
                    pr_number = meta.pr_number,
                    error = %e,
                    "pr gate result comment post failed; status still posted"
                );
            }
        }

        Some(status)
    }
}

struct GateEvaluation {
    status: (terraphim_tracker::StatusState, String),
    comment_body: String,
}

fn read_non_empty_log_lines(path: &Path) -> Option<Vec<String>> {
    let body = std::fs::read_to_string(path).ok()?;
    let lines: Vec<String> = body.lines().map(str::to_string).collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines)
    }
}

fn derive_pr_gate_status_from_output(
    meta: &crate::pr_gate_result::PrGateMeta,
    drain_lines: &[String],
    cli_tool: &str,
) -> GateEvaluation {
    let extracted = crate::pr_gate_result::extract_assistant_text(drain_lines, cli_tool);
    if extracted.trim().is_empty() {
        let reason = "no assistant output to parse";
        return GateEvaluation {
            status: failure_status(meta, reason),
            comment_body: canonical_failure_gate_comment(meta, reason, ""),
        };
    }

    let gate = match crate::pr_gate_result::extract_gate_result(&extracted) {
        Ok(gate) => gate,
        Err(e) => {
            let reason = format!("gate result parse failed: {e}");
            return GateEvaluation {
                status: failure_status(meta, &reason),
                comment_body: canonical_failure_gate_comment(meta, &reason, &extracted),
            };
        }
    };

    if let Err(e) = crate::pr_gate_result::validate_gate_result(&gate, meta) {
        let reason = format!("gate result invalid: {e}");
        return GateEvaluation {
            status: failure_status(meta, &reason),
            comment_body: canonical_failure_gate_comment(meta, &reason, &extracted),
        };
    }

    GateEvaluation {
        status: crate::pr_gate_result::status_from_gate_result(&gate),
        comment_body: limit_gate_comment(&extracted),
    }
}

fn failure_status(
    meta: &crate::pr_gate_result::PrGateMeta,
    reason: &str,
) -> (terraphim_tracker::StatusState, String) {
    (
        terraphim_tracker::StatusState::Failure,
        format!("{}: {reason}", meta.context),
    )
}

fn canonical_failure_gate_comment(
    meta: &crate::pr_gate_result::PrGateMeta,
    reason: &str,
    extracted: &str,
) -> String {
    let summary = truncate_for_summary(reason, 180);
    let result = crate::pr_gate_result::PrGateResult {
        schema_version: crate::pr_gate_result::GATE_RESULT_SCHEMA_VERSION,
        agent: meta.agent_name.clone(),
        context: meta.context.clone(),
        pr_number: meta.pr_number,
        head_sha: meta.head_sha.clone(),
        status: crate::pr_gate_result::GateStatus::Fail,
        confidence: 1,
        blocking_findings: 1,
        summary,
    };
    let json = serde_json::to_string_pretty(&result)
        .unwrap_or_else(|_| String::from(r#"{"schema_version":1,"status":"fail"}"#));
    let evidence = truncate_for_comment(extracted, 8_000);
    let evidence_section = if evidence.trim().is_empty() {
        "No assistant output was captured.".to_string()
    } else {
        format!("Captured output excerpt:\n\n```text\n{}\n```", evidence)
    };

    format!(
        "ADF PR gate failed closed.\n\nReason: {}\n\n{}\n\n<!-- adf:gate-result\n{}\n-->",
        reason, evidence_section, json
    )
}

fn limit_gate_comment(comment: &str) -> String {
    const MAX_COMMENT_CHARS: usize = 60_000;
    if comment.chars().count() <= MAX_COMMENT_CHARS {
        return comment.to_string();
    }
    let marker = "<!-- adf:gate-result";
    let suffix = comment
        .rfind(marker)
        .map(|idx| &comment[idx..])
        .unwrap_or_default();
    let prefix_budget = MAX_COMMENT_CHARS.saturating_sub(suffix.chars().count() + 80);
    let prefix = truncate_for_comment(comment, prefix_budget);
    if suffix.is_empty() {
        format!(
            "{}\n\n[ADF: comment truncated to {} characters]",
            prefix, MAX_COMMENT_CHARS
        )
    } else {
        format!(
            "{}\n\n[ADF: comment truncated to {} characters; canonical gate block preserved]\n\n{}",
            prefix, MAX_COMMENT_CHARS, suffix
        )
    }
}

fn truncate_for_comment(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push_str("\n...[truncated]");
    }
    out
}

fn truncate_for_summary(text: &str, max_chars: usize) -> String {
    let mut out: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> crate::pr_gate_result::PrGateMeta {
        crate::pr_gate_result::PrGateMeta {
            pr_number: 2318,
            project: "terraphim-ai".to_string(),
            agent_name: "pr-validator".to_string(),
            context: "adf/validation".to_string(),
            head_sha: "b71332d".to_string(),
        }
    }

    fn valid_output() -> Vec<String> {
        vec![
            "Validation report".to_string(),
            r#"<!-- adf:gate-result
{
  "schema_version": 1,
  "agent": "pr-validator",
  "context": "adf/validation",
  "pr_number": 2318,
  "head_sha": "b71332d",
  "status": "pass",
  "confidence": 5,
  "blocking_findings": 0,
  "summary": "Validation passed"
}
-->"#
                .to_string(),
        ]
    }

    #[test]
    fn pr_gate_status_fails_closed_when_output_is_empty() {
        let evaluation = derive_pr_gate_status_from_output(&meta(), &[], "");

        let (state, description) = evaluation.status;
        assert_eq!(state, terraphim_tracker::StatusState::Failure);
        assert!(description.contains("no assistant output"));
        assert!(evaluation
            .comment_body
            .contains("ADF PR gate failed closed"));
        assert!(evaluation.comment_body.contains("adf:gate-result"));
    }

    #[test]
    fn pr_gate_status_fails_closed_when_gate_block_is_missing() {
        let lines = vec!["Human report without the machine-readable block".to_string()];
        let evaluation = derive_pr_gate_status_from_output(&meta(), &lines, "unknown-cli");

        let (state, description) = evaluation.status;
        assert_eq!(state, terraphim_tracker::StatusState::Failure);
        assert!(description.contains("gate result parse failed"));
        assert!(description.contains("missing adf:gate-result block"));
        assert!(evaluation.comment_body.contains(&lines[0]));
        let gate = crate::pr_gate_result::extract_gate_result(&evaluation.comment_body)
            .expect("orchestrator-owned failure block must be valid");
        assert_eq!(gate.status, crate::pr_gate_result::GateStatus::Fail);
        assert_eq!(gate.blocking_findings, 1);
    }

    #[test]
    fn pr_gate_status_fails_closed_when_gate_head_is_stale() {
        let mut gate_meta = meta();
        gate_meta.head_sha = "new-head".to_string();

        let evaluation =
            derive_pr_gate_status_from_output(&gate_meta, &valid_output(), "unknown-cli");

        let (state, description) = evaluation.status;
        assert_eq!(state, terraphim_tracker::StatusState::Failure);
        assert!(description.contains("gate result invalid"));
        assert!(description.contains("head_sha"));
        let gate = crate::pr_gate_result::extract_gate_result(&evaluation.comment_body)
            .expect("orchestrator-owned invalid-result block must be valid");
        assert_eq!(gate.head_sha, "new-head");
    }

    #[test]
    fn pr_gate_status_uses_valid_gate_result() {
        let evaluation = derive_pr_gate_status_from_output(&meta(), &valid_output(), "unknown-cli");

        let (state, description) = evaluation.status;
        assert_eq!(state, terraphim_tracker::StatusState::Success);
        assert_eq!(description, "adf/validation pass (5/5)");
        assert!(evaluation.comment_body.contains("adf:gate-result"));
    }

    #[test]
    fn invalid_gate_comment_is_bounded_and_preserves_canonical_failure_block() {
        let oversized = format!("{}\n{}", "x".repeat(20_000), "<!-- adf:gate-result { -->");
        let lines = vec![oversized];

        let evaluation = derive_pr_gate_status_from_output(&meta(), &lines, "unknown-cli");

        assert!(evaluation.comment_body.len() < 12_000);
        let gate = crate::pr_gate_result::extract_gate_result(&evaluation.comment_body)
            .expect("synthesised failure block must parse");
        assert_eq!(gate.agent, "pr-validator");
        assert_eq!(gate.context, "adf/validation");
        assert_eq!(gate.status, crate::pr_gate_result::GateStatus::Fail);
    }

    #[test]
    fn oversized_valid_gate_comment_preserves_trailing_gate_block() {
        let body = format!(
            "{}\n{}",
            "large report\n".repeat(10_000),
            valid_output().join("\n")
        );

        let limited = limit_gate_comment(&body);

        assert!(limited.len() < body.len());
        let gate = crate::pr_gate_result::extract_gate_result(&limited)
            .expect("valid gate block should survive truncation");
        assert_eq!(gate.status, crate::pr_gate_result::GateStatus::Pass);
    }

    #[test]
    fn read_non_empty_log_lines_uses_file_backed_drain_log() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("agent.log");
        std::fs::write(&path, "line 1\nline 2\n").expect("log should be written");

        let lines = read_non_empty_log_lines(&path).expect("log lines should be loaded");

        assert_eq!(lines, vec!["line 1".to_string(), "line 2".to_string()]);
    }

    #[test]
    fn read_non_empty_log_lines_ignores_empty_file() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("agent.log");
        std::fs::write(&path, "").expect("log should be written");

        assert!(read_non_empty_log_lines(&path).is_none());
    }
}
