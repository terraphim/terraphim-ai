//! Telemetry capability for `AgentOrchestrator` (output-event draining,
//! telemetry recording/persistence, stdout/stderr parsing). Split from lib.rs
//! as part of the Gitea #1910 god-file decomposition; behaviour unchanged.

use terraphim_spawner::output::OutputEvent;
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::{control_plane, cost_tracker, evolution, provider_budget, quickwit, AgentOrchestrator};

impl AgentOrchestrator {
    /// Drain broadcast output events from all active agents into nightwatch.
    /// Also parses CLI output for telemetry completion events.
    pub(crate) fn drain_output_events(
        &mut self,
    ) -> Vec<(String, control_plane::telemetry::CompletionEvent)> {
        let mut events: Vec<(String, OutputEvent)> = Vec::new();
        for (name, managed) in &mut self.active_agents {
            loop {
                match managed.output_rx.try_recv() {
                    Ok(event) => events.push((name.clone(), event)),
                    Err(broadcast::error::TryRecvError::Empty) => break,
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        warn!(agent = %name, skipped = n, "output events lagged");
                        break;
                    }
                    Err(broadcast::error::TryRecvError::Closed) => break,
                }
            }
        }

        let mut completion_events: Vec<(String, control_plane::telemetry::CompletionEvent)> =
            Vec::new();

        for (name, event) in &events {
            self.nightwatch.observe(name, event);

            // Feed output to evolution system if enabled.
            if self.evolution_manager.is_enabled() {
                let is_evo = self
                    .active_agents
                    .get(name)
                    .map(|m| m.definition.evolution_enabled)
                    .unwrap_or(false);
                if is_evo {
                    let (line_content, event_type) = match event {
                        crate::OutputEvent::Stdout { line, .. } => (line.clone(), "stdout"),
                        crate::OutputEvent::Stderr { line, .. } => (line.clone(), "stderr"),
                        _ => continue,
                    };
                    let _ = self
                        .evolution_manager
                        .record_output(evolution::EvolutionOutput {
                            agent_id: name.clone(),
                            content: line_content,
                            event_type: event_type.to_string(),
                            importance: "medium".to_string(),
                        });
                }
            }

            match event {
                OutputEvent::Stdout { line, .. } => {
                    let (cli_tool, session_id, model) = self
                        .active_agents
                        .get(name)
                        .map(|m| {
                            (
                                m.definition.cli_tool.clone(),
                                m.session_id.clone(),
                                m.routed_model
                                    .clone()
                                    .or_else(|| m.definition.model.clone())
                                    .unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();

                    if let Some(ce) =
                        Self::parse_stdout_for_telemetry(&cli_tool, line, &session_id, &model)
                    {
                        completion_events.push((name.clone(), ce));
                    }
                }
                OutputEvent::Stderr { line, .. } => {
                    let (session_id, model) = self
                        .active_agents
                        .get(name)
                        .map(|m| {
                            (
                                m.session_id.clone(),
                                m.routed_model
                                    .clone()
                                    .or_else(|| m.definition.model.clone())
                                    .unwrap_or_default(),
                            )
                        })
                        .unwrap_or_default();
                    if let Some(ce) = Self::parse_stderr_for_telemetry(line, &session_id, &model) {
                        completion_events.push((name.clone(), ce));
                    }
                }
                _ => {}
            }

            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let (level, source, line) = match event {
                    crate::OutputEvent::Stdout { line, .. } => ("INFO", "stdout", line.as_str()),
                    crate::OutputEvent::Stderr { line, .. } => ("WARN", "stderr", line.as_str()),
                    _ => continue,
                };
                let layer = self
                    .active_agents
                    .get(name)
                    .map(|m| format!("{:?}", m.definition.layer))
                    .unwrap_or_default();
                let project_id = self
                    .active_agents
                    .get(name)
                    .and_then(|m| m.definition.project.clone())
                    .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
                let model = self.active_agents.get(name).and_then(|m| {
                    m.routed_model
                        .clone()
                        .or_else(|| m.definition.model.clone())
                });
                let doc = quickwit::LogDocument {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    project_id,
                    level: level.into(),
                    agent_name: name.clone(),
                    layer,
                    source: source.into(),
                    message: line.to_owned(),
                    model,
                    ..Default::default()
                };
                let _ = sink.try_send(doc);
            }
        }

        completion_events
    }

    /// Record parsed telemetry events into the telemetry store, per-agent
    /// cost tracker, and (when configured) the provider-level hour/day
    /// budget tracker.
    ///
    /// Cost accounting is performed per-agent before the batch write so that
    /// agent-level spend is still tracked individually. The telemetry store
    /// write uses a single lock acquisition via `record_batch`. Provider
    /// budget spend is folded in during the same iteration so Layer 3 of
    /// the subscription gate actually sees real dispatch cost.
    pub(crate) async fn record_telemetry(
        &self,
        events: Vec<(String, control_plane::telemetry::CompletionEvent)>,
    ) {
        for (agent_name, event) in &events {
            if event.cost_usd > 0.0 {
                self.cost_tracker.record_cost(agent_name, event.cost_usd);

                if let Some(tracker) = self.provider_budget_tracker.as_ref() {
                    if let Some(provider_key) =
                        provider_budget::provider_key_for_model(&event.model)
                    {
                        let verdict = tracker.record_cost(provider_key, event.cost_usd);
                        if matches!(
                            verdict,
                            cost_tracker::BudgetVerdict::Exhausted { .. }
                                | cost_tracker::BudgetVerdict::NearExhaustion { .. }
                        ) {
                            warn!(
                                provider = provider_key,
                                agent = agent_name.as_str(),
                                cost_usd = event.cost_usd,
                                verdict = ?verdict,
                                "provider budget pressure recorded"
                            );
                        }
                    }
                }
            }

            // Send to Quickwit for cost-aware routing analytics
            if let Some(ref sink) = self.quickwit_sink {
                let doc = quickwit::LogDocument {
                    timestamp: event.completed_at.to_rfc3339(),
                    project_id: self
                        .config
                        .projects
                        .first()
                        .map(|p| p.id.clone())
                        .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
                    level: if event.success {
                        "INFO".to_string()
                    } else {
                        "WARN".to_string()
                    },
                    agent_name: agent_name.clone(),
                    layer: "Core".to_string(),
                    source: "telemetry".to_string(),
                    message: event
                        .error
                        .clone()
                        .unwrap_or_else(|| "completion recorded".to_string()),
                    model: Some(event.model.clone()),
                    cost_usd: Some(event.cost_usd),
                    latency_ms: Some(event.latency_ms),
                    tokens_input: Some(event.tokens.input),
                    tokens_output: Some(event.tokens.output),
                    exit_class: if event.success {
                        Some("success".to_string())
                    } else {
                        Some("error".to_string())
                    },
                    is_free: event.cost_usd == 0.0,
                    ..Default::default()
                };
                if let Err(e) = sink.send(doc).await {
                    warn!(error = %e, "failed to send telemetry to Quickwit");
                }
            }
        }
        // Write all events in one lock acquisition.
        let completion_events: Vec<control_plane::telemetry::CompletionEvent> =
            events.into_iter().map(|(_, e)| e).collect();
        self.telemetry_store.record_batch(completion_events).await;
    }

    /// Attempt to restore persisted telemetry summary from durable storage.
    ///
    /// Best-effort: if no summary exists or loading fails, logs and continues
    /// with an empty telemetry store. Called once at the start of `run()`.
    pub(crate) async fn restore_telemetry(&self) {
        use terraphim_persistence::Persistable;
        let mut summary = control_plane::TelemetrySummary::new("telemetry_summary".to_string());
        match summary.load().await {
            Ok(loaded) => {
                self.telemetry_store.import_summary(loaded).await;
                info!("restored persisted telemetry summary");
            }
            Err(_) => {
                info!("no persisted telemetry summary found, starting fresh");
            }
        }
    }

    /// Persist telemetry summary to durable storage via fire-and-forget spawn.
    ///
    /// Clones the Arc-backed store and moves both export and save into the
    /// spawned task so the reconcile loop is not blocked by the read lock.
    pub(crate) fn persist_telemetry(&self) {
        let store = self.telemetry_store.clone();
        tokio::spawn(async move {
            use terraphim_persistence::Persistable;
            let summary = store.export_summary().await;
            if let Err(e) = summary.save().await {
                tracing::warn!(error = %e, "failed to persist telemetry summary");
            }
        });
    }

    /// Parse a stdout line from a CLI tool into a CompletionEvent, if the line
    /// represents a completed agent session.
    ///
    /// Returns `None` for lines that do not carry completion telemetry (tool
    /// calls, status updates, ignored formats, or unrecognised cli_tool).
    pub(crate) fn parse_stdout_for_telemetry(
        cli_tool: &str,
        line: &str,
        session_id: &str,
        model: &str,
    ) -> Option<control_plane::telemetry::CompletionEvent> {
        let cli_basename = std::path::Path::new(cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(cli_tool);
        let parsed = match cli_basename {
            "opencode" => {
                control_plane::output_parser::parse_opencode_line(line, session_id, model, None)
            }
            "claude" | "claude-code" => {
                control_plane::output_parser::parse_claude_line(line, session_id, model)
            }
            "pi-rust" | "pi" => {
                control_plane::output_parser::parse_pi_rust_line(line, session_id, model)
            }
            _ => control_plane::output_parser::ParsedOutput::Ignored,
        };
        match parsed {
            control_plane::output_parser::ParsedOutput::Completion(ce) => Some(ce),
            _ => None,
        }
    }

    /// Parse a stderr line into a CompletionEvent representing a subscription
    /// limit error.
    ///
    /// Returns `None` when the line does not match any known limit-error
    /// pattern.
    pub(crate) fn parse_stderr_for_telemetry(
        line: &str,
        session_id: &str,
        model: &str,
    ) -> Option<control_plane::telemetry::CompletionEvent> {
        control_plane::output_parser::parse_stderr_for_limit_errors(line)?;
        Some(control_plane::telemetry::CompletionEvent {
            model: model.to_string(),
            session_id: session_id.to_string(),
            completed_at: chrono::Utc::now(),
            latency_ms: 0,
            success: false,
            tokens: control_plane::telemetry::TokenBreakdown::default(),
            cost_usd: 0.0,
            error: Some(line.to_string()),
        })
    }
}
