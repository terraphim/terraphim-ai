//! Agent-spawning capability for `AgentOrchestrator`: spawning agents (with
//! and without lifecycle events), remediation-agent spawning, and git
//! worktree creation/removal/commit. Split from lib.rs as part of the Gitea
//! #1910 god-file decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use terraphim_spawner::{ResourceLimits, SpawnRequest};
use terraphim_types::*;
use tracing::{debug, error, info, warn};

use crate::config::AgentDefinition;
#[cfg(feature = "quickwit")]
use crate::quickwit;
use crate::{
    agent_key, build_spawn_context_for_agent, control_plane, kg_router, project_control,
    requires_isolated_worktree, AgentOrchestrator, ManagedAgent, OrchestratorError, PreCheckResult,
    SyntheticEvent,
};

impl AgentOrchestrator {
    /// Spawn an agent from its definition.
    ///
    /// Model selection: if the agent has an explicit `model` field, use it.
    /// Otherwise, route the task prompt through the RoutingEngine to select
    /// a model based on keyword matching.
    pub(crate) async fn spawn_agent(
        &mut self,
        def: &AgentDefinition,
    ) -> Result<(), OrchestratorError> {
        self.spawn_agent_with_event(def, None).await
    }

    pub(crate) async fn spawn_agent_with_event(
        &mut self,
        def: &AgentDefinition,
        synthetic_event: Option<&SyntheticEvent>,
    ) -> Result<(), OrchestratorError> {
        // === PROJECT PAUSE GATE ===
        // Operators and the project circuit breaker can block all dispatches
        // for a given project by creating a sentinel file at
        // `<pause_dir>/<project_id>`. The gate is project-scoped; legacy /
        // global agents (`def.project == None`) are never blocked here.
        if project_control::is_project_paused(&self.pause_dir, def.project.as_deref()) {
            info!(
                agent = %def.name,
                project = ?def.project,
                pause_dir = %self.pause_dir.display(),
                "skipping spawn: project is paused"
            );
            return Ok(());
        }

        // === DISK SPACE GUARD ===
        let threshold = self.config.disk_usage_threshold;
        if threshold < 100 {
            if let Some(usage) = Self::check_disk_usage_percent() {
                if usage >= threshold {
                    error!(
                        agent = %def.name,
                        disk_usage_percent = usage,
                        threshold,
                        "refusing to spawn agent: disk usage above threshold"
                    );
                    return Err(OrchestratorError::Config(format!(
                        "disk usage {}% >= {}% threshold, refusing to spawn {}",
                        usage, threshold, def.name
                    )));
                }
            }
        }

        // === BUDGET GATE ===
        // Skip spawn entirely if the agent's monthly budget is exhausted.
        // CostTracker::check is already called during routing (for budget
        // pressure scoring), but routing only deprioritises cheaper models;
        // it does not short-circuit dispatch. A fully exhausted agent must
        // not run at all this cycle.
        let budget_check = self.cost_tracker.check(&def.name);
        if budget_check.should_pause() {
            warn!(
                agent = %def.name,
                verdict = %budget_check,
                "skipping spawn: monthly budget exhausted"
            );
            return Ok(());
        }
        if budget_check.should_warn() {
            warn!(
                agent = %def.name,
                verdict = %budget_check,
                "budget near exhaustion; routing will prefer cheaper models"
            );
        }

        // === PRE-CHECK GATE ===
        let pre_check_result = self.run_pre_check(def).await;
        let findings = match pre_check_result {
            PreCheckResult::NoFindings => {
                info!(agent = %def.name, "skipping spawn: pre-check found nothing actionable");
                return Ok(());
            }
            PreCheckResult::Findings(f) if f.is_empty() => None,
            PreCheckResult::Findings(f) => Some(f),
            PreCheckResult::Failed(reason) => {
                warn!(agent = %def.name, reason = %reason,
                      "pre-check failed, spawning anyway (fail-open)");
                None
            }
        };

        // Select model via keyword routing or explicit config.
        // Skip keyword routing for CLIs that use OAuth and don't support -m
        // (e.g. codex with ChatGPT account). Only apply routed models when the
        // CLI tool is known to accept --model flags with arbitrary model IDs.
        let cli_name = std::path::Path::new(&def.cli_tool)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&def.cli_tool);
        let supports_model_flag = matches!(
            cli_name,
            "claude" | "claude-code" | "opencode" | "pi-rust" | "pi"
        );

        // Track KG decision for CLI override (set inside the routing block below)
        let mut kg_cli_override: Option<String> = None;

        #[allow(clippy::manual_let_else)]
        let model = if self
            .config
            .routing
            .as_ref()
            .is_some_and(|r| r.use_routing_engine)
        {
            let kg_arc = self
                .kg_router
                .as_ref()
                .map(|r| std::sync::Arc::new(r.clone()));
            let unhealthy = self.provider_health.unhealthy_providers();
            let telemetry_arc = std::sync::Arc::new(self.telemetry_store.clone());
            let strategy = self
                .config
                .routing
                .as_ref()
                .map(|r| r.route_selection_strategy)
                .unwrap_or(crate::control_plane::RouteSelectionStrategy::Fastest);
            let engine = control_plane::RoutingDecisionEngine::with_provider_budget_and_strategy(
                kg_arc,
                unhealthy,
                terraphim_router::Router::new(),
                Some(telemetry_arc),
                self.provider_budget_tracker.clone(),
                strategy,
            );
            let ctx = control_plane::DispatchContext {
                agent_name: def.name.clone(),
                task: def.task.clone(),
                static_model: def.model.clone(),
                cli_tool: def.cli_tool.clone(),
                layer: def.layer,
                session_id: None,
                default_tier: def.default_tier.clone(),
            };
            let budget_verdict = self.cost_tracker.check(&def.name);
            let decision = engine.decide_route(&ctx, &budget_verdict).await;
            info!(
                agent = %def.name,
                rationale = %decision.rationale,
                telemetry_influenced = decision.telemetry_influenced,
                "routing engine selected model"
            );
            if decision.candidate.model.is_empty() {
                None
            } else {
                // Extract CLI tool override from routing decision so that
                // anthropic models routed via KG use claude CLI, not opencode.
                if decision.candidate.cli_tool != def.cli_tool {
                    kg_cli_override = Some(decision.candidate.cli_tool.clone());
                }
                Some(decision.candidate.model)
            }
        } else if supports_model_flag && def.bypass_kg_routing {
            // Fallback respawn (quota exit, wall-clock timeout, or KG-fallback
            // route already selected): the caller has explicitly chosen
            // `cli_tool` and `model`, and re-running KG tier routing here
            // would override their decision and route the spawn back to the
            // just-blocked primary. Honour the static config verbatim.
            info!(
                agent = %def.name,
                "bypassing KG tier routing per agent definition (fallback respawn)"
            );
            def.model.clone()
        } else if supports_model_flag {
            // KG routing first (phase-aware tier selection from markdown rules).
            // Takes priority over static model config so tier routing controls selection.
            let mut unhealthy = self.provider_health.unhealthy_providers();
            unhealthy.extend(self.provider_rate_limits.blocked_providers());
            let kg_decision = self.kg_router.as_ref().and_then(|router| {
                let decision =
                    router.route_agent_with_tier(&def.task, def.default_tier.as_deref())?;
                // If primary provider is unhealthy, try fallback routes
                if !unhealthy.is_empty() {
                    if let Some(healthy_route) = decision.first_healthy_route(&unhealthy) {
                        info!(
                            agent = %def.name,
                            concept = %decision.matched_concept,
                            provider = %healthy_route.provider,
                            model = %healthy_route.model,
                            skipped_unhealthy = ?unhealthy,
                            "KG routed to fallback (primary unhealthy)"
                        );
                        return Some(kg_router::KgRouteDecision {
                            provider: healthy_route.provider.clone(),
                            model: healthy_route.model.clone(),
                            action: healthy_route.action.clone(),
                            confidence: decision.confidence * 0.9,
                            matched_concept: decision.matched_concept,
                            priority: decision.priority,
                            fallback_routes: decision.fallback_routes,
                        });
                    }
                }
                Some(decision)
            });

            if let Some(ref kg) = kg_decision {
                info!(
                    agent = %def.name,
                    concept = %kg.matched_concept,
                    provider = %kg.provider,
                    model = %kg.model,
                    confidence = kg.confidence,
                    "model selected via KG tier routing"
                );
                // Extract CLI tool from action template (first word = CLI path)
                if let Some(ref action) = kg.action {
                    if let Some(cli) = action.split_whitespace().next() {
                        kg_cli_override = Some(cli.to_string());
                    }
                }
                Some(kg.model.clone())
            } else if let Some(m) = &def.model {
                // Static config fallback when KG has no match
                info!(agent = %def.name, model = %m, "using static model (no KG tier match)");
                Some(m.clone())
            } else {
                // Fall back to keyword routing engine
                let context = terraphim_router::RoutingContext::default();
                match self.router.route(&def.task, &context) {
                    Ok(decision) => {
                        if let terraphim_types::capability::ProviderType::Llm { model_id, .. } =
                            &decision.provider.provider_type
                        {
                            info!(
                                agent = %def.name,
                                model = %model_id,
                                confidence = decision.confidence,
                                "model selected via keyword routing"
                            );
                            Some(model_id.clone())
                        } else {
                            None
                        }
                    }
                    Err(_) => {
                        info!(agent = %def.name, "no model matched, using CLI default");
                        None
                    }
                }
            }
        } else {
            info!(agent = %def.name, cli = %def.cli_tool, "skipping model routing (CLI uses OAuth/default)");
            None
        };

        // For opencode, compose "provider/model" format when both fields are set.
        // opencode requires `-m provider/model` whereas the TOML config stores them
        // separately (provider = "kimi-for-coding", model = "k2p5").
        // Skip composition if the model already contains a provider prefix (e.g.
        // from KG routing which returns full model ids like "kimi-for-coding/k2p5").
        let model = if cli_name == "opencode" {
            match (&def.provider, &model) {
                (Some(provider), Some(m)) if !m.contains('/') => {
                    let composed = format!("{}/{}", provider, m);
                    info!(agent = %def.name, composed_model = %composed, "composed provider/model for opencode");
                    Some(composed)
                }
                _ => model,
            }
        } else {
            model
        };

        // If KG routing selected a different CLI tool (e.g., claude instead of opencode),
        // use the KG-selected CLI to match the routed model.
        let effective_cli = kg_cli_override
            .as_deref()
            .unwrap_or(&def.cli_tool)
            .to_string();

        info!(agent = %def.name, layer = ?def.layer, cli = %effective_cli, model = ?model, "spawning agent");

        // Compose persona-enriched task prompt
        let (composed_task, persona_found) = if let Some(ref persona_name) = def.persona {
            if let Some(persona) = self.persona_registry.get(persona_name) {
                let composed = self.metaprompt_renderer.compose_prompt(persona, &def.task);
                info!(
                    agent = %def.name,
                    persona = %persona_name,
                    original_len = def.task.len(),
                    composed_len = composed.len(),
                    "composed persona-enriched prompt"
                );
                (composed, true)
            } else {
                warn!(
                    agent = %def.name,
                    persona = %persona_name,
                    "persona not found in registry, using bare task"
                );
                (def.task.clone(), false)
            }
        } else {
            (def.task.clone(), false)
        };

        // === FINDINGS INJECTION ===
        let composed_task = if let Some(ref findings) = findings {
            format!(
                "## Pre-flight findings (automated checks already ran)\n\n{}\n\n---\n\n{}",
                findings, composed_task
            )
        } else {
            composed_task
        };

        // Inject skill_chain content between persona preamble and task
        let skill_content = self.load_skill_chain_content(def);
        let composed_task = if skill_content.is_empty() {
            composed_task
        } else {
            info!(
                agent = %def.name,
                skills = def.skill_chain.len(),
                skill_bytes = skill_content.len(),
                "injecting skill_chain into prompt"
            );
            format!("{}{}", composed_task, skill_content)
        };

        // Inject prior lessons from shared learning store
        let (lessons_section, lesson_ids) = self.render_lessons_section(&def.name);
        let mut composed_task = if lessons_section.is_empty() {
            composed_task
        } else {
            info!(
                agent = %def.name,
                lessons = lesson_ids.len(),
                "injecting prior lessons into prompt"
            );
            self.injected_learning_ids
                .insert(def.name.clone(), lesson_ids);
            format!("{}\n\n{}", composed_task, lessons_section)
        };

        // Inject evolution memory context if enabled for this agent.
        if def.evolution_enabled && self.evolution_manager.is_enabled() {
            self.evolution_manager.ensure_agent(&def.name);
            let _ = self
                .evolution_manager
                .record_task_start(&def.name, &def.task);
            let evo_ctx = self.evolution_manager.render_context(&def.name);
            if !evo_ctx.is_empty() {
                info!(agent = %def.name, "injecting evolution memory context");
                composed_task = format!("{}\n\n{}", composed_task, evo_ctx);
            }
        }

        // Inject RLM session info if enabled for this agent.
        if def.rlm_enabled.unwrap_or(false) {
            info!(agent = %def.name, "injecting RLM sandboxed execution context");
            composed_task = format!(
                "{}\n\n## RLM Sandboxed Code Execution\n\
                 You have access to sandboxed code execution via terraphim_rlm. \
                 Use the terraphim-rlm MCP tools to execute code in an isolated environment \
                 when you need to run, test, or validate code changes. \
                 Sessions are resource-limited and automatically cleaned up.",
                composed_task
            );
        }

        // Use stdin only when persona was actually resolved (prompt is enriched)
        // or when the task exceeds ARG_MAX safety threshold.
        // Do NOT use stdin for unfound personas -- the bare task is small and
        // stdin delivery to short-lived processes (echo) causes broken pipe races.
        const STDIN_THRESHOLD: usize = 32_768; // 32 KB
        let use_stdin =
            persona_found || !skill_content.is_empty() || composed_task.len() > STDIN_THRESHOLD;

        // Create isolated git worktrees for AI/model-backed agents that may modify code.
        // Review-tier agents (haiku) and simple local commands used in tests do not need isolation.
        let needs_isolation = requires_isolated_worktree(def, model.as_deref());

        // Resolve the git repo directory for worktree operations. Project-bound
        // agents need a worktree from their own repo, not the orchestrator's.
        let repo_dir: &Path = if let Some(pid) = def.project.as_deref() {
            match self.config.project_by_id(pid) {
                Some(p) => p.working_dir.as_path(),
                None => {
                    warn!(
                        agent = %def.name,
                        project_id = %pid,
                        fallback = %self.config.working_dir.display(),
                        "project_by_id returned None, falling back to orchestrator working_dir"
                    );
                    &self.config.working_dir
                }
            }
        } else {
            &self.config.working_dir
        };

        let (worktree_path, worktree_guard) = if needs_isolation {
            let path = self.create_agent_worktree(&def.name, repo_dir).await?;
            let guard = crate::worktree_guard::WorktreeGuard::for_managed(repo_dir, &path);
            (Some(path), Some(guard))
        } else {
            (None, None)
        };
        let agent_working_dir = worktree_path.as_deref().unwrap_or(repo_dir).to_path_buf();

        // Build primary Provider from the agent definition for the spawner.
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: effective_cli.clone(),
                working_dir: agent_working_dir.clone(),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        // Build fallback Provider if fallback_provider is configured
        let fallback_provider = def.fallback_provider.as_ref().map(|fallback_cli| {
            terraphim_types::capability::Provider {
                id: format!("{}-fallback", def.name),
                name: format!("{} (fallback)", def.name),
                provider_type: terraphim_types::capability::ProviderType::Agent {
                    agent_id: format!("{}-fallback", def.name),
                    cli_command: fallback_cli.clone(),
                    working_dir: agent_working_dir.clone(),
                },
                capabilities: vec![],
                cost_level: terraphim_types::capability::CostLevel::Cheap,
                latency: terraphim_types::capability::Latency::Medium,
                keywords: def.capabilities.clone(),
            }
        });

        // Build the spawn request with primary and fallback
        let mut request = SpawnRequest::new(primary_provider, &composed_task)
            .with_primary_model(model.as_deref().unwrap_or(""));

        if let Some(fallback) = fallback_provider {
            request = request.with_fallback_provider(fallback);
            if let Some(fallback_model) = &def.fallback_model {
                request = request.with_fallback_model(fallback_model);
            }
        }

        if use_stdin {
            request = request.with_stdin();
        }

        // Thread resource limits from agent definition to spawner
        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        // === CONCURRENCY GATE ===
        let project_id = def
            .project
            .as_deref()
            .unwrap_or(crate::dispatcher::LEGACY_PROJECT_ID);
        let permit = self.concurrency_controller.acquire_any(project_id).await;
        if permit.is_none() {
            warn!(
                agent = %def.name,
                project = %project_id,
                active = self.active_agents.len(),
                "skipping spawn: global concurrency limit reached"
            );
            return Ok(());
        }

        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, def, self.output_poster.as_ref());
        spawn_ctx.working_dir = Some(agent_working_dir.clone());
        spawn_ctx = spawn_ctx.with_env(
            "ADF_WORKING_DIR",
            agent_working_dir.to_string_lossy().into_owned(),
        );
        if let Some(event) = synthetic_event {
            for (key, value) in event.env_vars() {
                spawn_ctx = spawn_ctx.with_env(key, value);
            }
        }

        // Pre-create temp log path so the spawner can write stderr directly
        // to disk, giving us a durable fallback if the broadcast drain lags.
        let _ = std::fs::create_dir_all(&self.agent_log_dir);
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
        let stderr_tmp_name = format!(".tmp-{}-{}.stderr.log", def.name, ts);
        let stderr_tmp_path = self.agent_log_dir.join(&stderr_tmp_name);
        spawn_ctx = spawn_ctx.with_stderr_log(&stderr_tmp_path);

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        // Subscribe to the output broadcast for nightwatch drain
        let output_rx = handle.subscribe_output();

        // Open a streaming log file and spawn a background drain task so
        // output is captured to disk even when the tick interval is long.
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);

        // Get the restart count from the orchestrator-level counter
        let restart_count = self
            .restart_counts
            .get(&agent_key(def))
            .copied()
            .unwrap_or(0);

        self.active_agents.insert(
            def.name.clone(),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count,
                output_rx,
                spawned_by_mention: false,
                worktree_path,
                worktree_guard,
                routed_model: model.clone(),
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: permit,
                commit_status_post: None,
                gate_meta: None,
                output_tmp_path,
            },
        );

        // === RECORD COMMIT FOR GIT-DIFF STRATEGY ===
        if let Ok(head) = self.get_current_head().await {
            self.last_run_commits.insert(def.name.clone(), head);
        }

        #[cfg(feature = "quickwit")]
        if let Some(ref sink) = self.quickwit_sink {
            let doc = quickwit::LogDocument {
                timestamp: chrono::Utc::now().to_rfc3339(),
                project_id: def
                    .project
                    .clone()
                    .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string()),
                level: "INFO".into(),
                agent_name: def.name.clone(),
                layer: format!("{:?}", def.layer),
                source: "orchestrator".into(),
                message: "agent spawned".into(),
                model: model.clone(),
                ..Default::default()
            };
            let _ = sink.send(doc).await;
        }

        Ok(())
    }
    /// Spawn a remediation agent for a CRITICAL finding.
    ///
    /// Looks up the finding's category in `compound_review.remediation_agents`
    /// to determine which agent to spawn. If no mapping exists, logs and skips.
    pub(crate) async fn spawn_remediation_agent(
        &mut self,
        finding: &ReviewFinding,
    ) -> Result<(), String> {
        let category_key = format!("{:?}", finding.category).to_lowercase();
        let agent_name = self
            .config
            .compound_review
            .remediation_agents
            .get(&category_key)
            .cloned();

        let agent_name = match agent_name {
            Some(name) => name,
            None => {
                debug!(
                    category = %category_key,
                    "no remediation agent mapped for category, skipping"
                );
                return Ok(());
            }
        };

        // Build a targeted fix prompt
        let mut prompt = "Fix this CRITICAL finding from compound review:\n\n".to_string();
        if !finding.file.is_empty() {
            prompt.push_str(&format!(
                "File: {}{}\n",
                finding.file,
                if finding.line > 0 {
                    format!(":{}", finding.line)
                } else {
                    String::new()
                }
            ));
        }
        prompt.push_str(&format!(
            "Severity: CRITICAL\nFinding: {}\n",
            finding.finding
        ));
        if let Some(ref suggestion) = finding.suggestion {
            if !suggestion.is_empty() {
                prompt.push_str(&format!("Suggested approach: {}\n", suggestion));
            }
        }
        prompt.push_str(
            "\nInstructions:\n\
1. Read the relevant file(s)\n\
2. Implement the fix\n\
3. Run cargo build && cargo test to verify\n\
4. Commit your changes\n",
        );

        // Look up the agent definition
        let agent_def = self
            .config
            .agents
            .iter()
            .find(|a| a.name == agent_name)
            .cloned();

        let agent_def = match agent_def {
            Some(def) => def,
            None => {
                warn!(
                    agent = %agent_name,
                    "remediation agent not found in fleet config, skipping"
                );
                return Ok(());
            }
        };

        // Spawn using the existing agent infrastructure
        // Build a modified agent def with our custom task prompt
        let mut fix_def = agent_def;
        fix_def.task = prompt;
        fix_def.pre_check = None; // Skip pre-check for remediation

        let spawned = self.spawn_agent(&fix_def).await;

        match spawned {
            Ok(_) => {
                info!(
                    agent = %agent_name,
                    file = %finding.file,
                    "spawned remediation agent for CRITICAL finding"
                );
                Ok(())
            }
            Err(e) => Err(format!(
                "failed to spawn remediation agent '{}': {}",
                agent_name, e
            )),
        }
    }

    /// Create a git worktree for an agent to work in isolation.
    ///
    /// `repo_dir` is the git repository root where `git worktree add` runs.
    /// For project-bound agents this is the project's working_dir; otherwise
    /// it is the orchestrator's top-level working_dir.
    ///
    /// Returns the worktree path if successful. Mutating agents fail closed when
    /// git cannot create an isolated worktree; they must not use the shared checkout.
    async fn create_agent_worktree(
        &self,
        agent_name: &str,
        repo_dir: &Path,
    ) -> Result<PathBuf, OrchestratorError> {
        let worktree_root = repo_dir.join(".worktrees");
        if let Err(e) = tokio::fs::create_dir_all(&worktree_root).await {
            return Err(OrchestratorError::WorktreeCreationFailed {
                agent: agent_name.to_string(),
                repo: repo_dir.display().to_string(),
                reason: format!("failed to create worktree root: {e}"),
            });
        }

        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let worktree_path = worktree_root.join(format!("{agent_name}-{id}"));

        let output = tokio::process::Command::new("git")
            .args([
                "worktree",
                "add",
                "--detach",
                &worktree_path.to_string_lossy(),
                "HEAD",
            ])
            .current_dir(repo_dir)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    repo = %repo_dir.display(),
                    "created isolated git worktree"
                );
                Ok(worktree_path)
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                Err(OrchestratorError::WorktreeCreationFailed {
                    agent: agent_name.to_string(),
                    repo: repo_dir.display().to_string(),
                    reason: stderr.chars().take(500).collect::<String>(),
                })
            }
            Err(e) => Err(OrchestratorError::WorktreeCreationFailed {
                agent: agent_name.to_string(),
                repo: repo_dir.display().to_string(),
                reason: format!("git worktree command failed: {e}"),
            }),
        }
    }

    /// Remove a git worktree after an agent finishes.
    pub(crate) async fn remove_agent_worktree(
        &self,
        agent_name: &str,
        worktree_path: &Path,
        repo_dir: &Path,
    ) {
        // Force-remove even if there are uncommitted changes (they were already
        // committed by try_commit_agent_work or are intentionally discarded).
        let output = tokio::process::Command::new("git")
            .args([
                "worktree",
                "remove",
                "--force",
                &worktree_path.to_string_lossy(),
            ])
            .current_dir(repo_dir)
            .output()
            .await;

        match output {
            Ok(o) if o.status.success() => {
                info!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    "removed agent worktree"
                );
            }
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                warn!(
                    agent = %agent_name,
                    path = %worktree_path.display(),
                    error = %stderr.chars().take(200).collect::<String>(),
                    "git worktree remove failed"
                );
            }
            Err(e) => {
                warn!(agent = %agent_name, error = %e, "git worktree remove command failed");
            }
        }
    }

    /// Attempt to commit any uncommitted working tree changes made by an agent.
    ///
    /// This runs `git add -A && git diff --cached --quiet` to check if there
    /// are changes, then commits with a standard message. Failures are logged
    /// but not propagated — agent work is best-effort.
    pub(crate) async fn try_commit_agent_work(&self, agent_name: &str, working_dir: &Path) {
        // Stage all changes
        let add = tokio::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(working_dir)
            .output()
            .await;

        if let Err(e) = add {
            tracing::debug!(agent = %agent_name, error = %e, "git add failed, skipping commit");
            return;
        }

        // Check if there are staged changes
        let diff_check = tokio::process::Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(working_dir)
            .status()
            .await;

        match diff_check {
            Ok(status) if status.success() => {
                // No changes to commit
                return;
            }
            Ok(_) => { /* changes exist */ }
            Err(e) => {
                tracing::debug!(agent = %agent_name, error = %e, "git diff failed, skipping commit");
                return;
            }
        }

        // Get current branch for commit message
        let branch = tokio::process::Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(working_dir)
            .output()
            .await;

        let branch_name = match branch {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(_) => "unknown".to_string(),
        };

        let msg = format!("feat({agent_name}): agent work [auto-commit]");

        let commit = tokio::process::Command::new("git")
            .args(["commit", "-m", &msg])
            .current_dir(working_dir)
            .output()
            .await;

        match commit {
            Ok(output) if output.status.success() => {
                tracing::info!(
                    agent = %agent_name,
                    branch = %branch_name,
                    "auto-committed agent working tree changes"
                );
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::debug!(
                    agent = %agent_name,
                    stderr = %stderr,
                    "git commit failed"
                );
            }
            Err(e) => {
                tracing::warn!(agent = %agent_name, error = %e, "failed to run git commit");
            }
        }
    }
}
