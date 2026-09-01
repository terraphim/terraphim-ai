//! PR-handling capability for `AgentOrchestrator`: review-PR dispatch,
//! pr-reviewer and build-runner spawning, commit-status posting, and push
//! handling. Split from lib.rs as part of the Gitea #1910 god-file
//! decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use std::time::Instant;

use terraphim_spawner::{ResourceLimits, SpawnRequest};
use tracing::{debug, info, warn};

use crate::config;
use crate::{
    agent_key, build_spawn_context_for_agent, control_plane, dispatcher, pr_dispatch,
    pr_gate_context, pr_gate_prompt, AgentOrchestrator, CommitStatusPost, ManagedAgent,
    OrchestratorError,
};

const CANONICAL_PR_GATE_AGENTS: [&str; 3] = ["pr-verifier", "pr-validator", "pr-reviewer"];

pub(crate) fn is_canonical_pr_gate_agent(agent_name: &str) -> bool {
    CANONICAL_PR_GATE_AGENTS.contains(&agent_name)
}

/// `active_agents` key for a PR gate agent.
///
/// Gate names such as `pr-validator` are legitimately reused across
/// projects, so the in-process identity must carry the project: otherwise
/// one project's running gate starves the other's duplicate guard, receives
/// its mention-chain metadata, and is removed on the wrong exit.
pub(crate) fn pr_gate_agent_key(project: &str, agent_name: &str) -> String {
    format!("{project}/{agent_name}")
}

#[derive(Debug, serde::Deserialize)]
struct GiteaPullForGateMention {
    number: u64,
    head: GiteaPullRefForGateMention,
    user: GiteaPullUserForGateMention,
    title: String,
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
}

#[derive(Debug, serde::Deserialize)]
struct GiteaPullRefForGateMention {
    sha: String,
    #[serde(rename = "ref")]
    ref_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct GiteaPullUserForGateMention {
    login: String,
}

fn valid_pr_gate_mention_request(
    project: &str,
    agent_name: &str,
    issue_number: u64,
    req: &pr_dispatch::ReviewPrRequest,
) -> bool {
    if req.project != project {
        warn!(
            expected_project = project,
            actual_project = %req.project,
            agent = %agent_name,
            issue = issue_number,
            "PR gate mention rejected: project mismatch"
        );
        return false;
    }
    if req.pr_number != issue_number || req.head_sha.trim().is_empty() {
        warn!(
            project,
            agent = %agent_name,
            issue = issue_number,
            pr_number = req.pr_number,
            head_sha = %req.head_sha,
            "PR gate mention rejected: PR number/head mismatch"
        );
        return false;
    }
    true
}

#[cfg(test)]
pub(crate) fn duplicate_gate_config_for_tests() -> crate::OrchestratorConfig {
    let mut config = crate::OrchestratorConfig::from_toml(
        r#"
working_dir = "/tmp/adf-root"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp"

[[projects]]
id = "terraphim-llm-proxy"
working_dir = "/tmp/proxy"
[projects.gitea]
base_url = "http://127.0.0.1:1"
token = "proxy-token"
owner = "proxy-owner"
repo = "proxy-repo"

[[projects]]
id = "digital-twins"
working_dir = "/tmp/digital"
[projects.gitea]
base_url = "http://127.0.0.1:1"
token = "digital-token"
owner = "digital-owner"
repo = "digital-repo"

[[agents]]
name = "pr-validator"
layer = "Growth"
cli_tool = "echo"
task = "proxy validator"
model = "sonnet"
project = "terraphim-llm-proxy"

[[agents]]
name = "pr-validator"
layer = "Growth"
cli_tool = "echo"
task = "digital validator"
model = "sonnet"
project = "digital-twins"
"#,
    )
    .expect("test config parses");
    config.pr_dispatch_per_project.insert(
        "terraphim-llm-proxy".to_string(),
        config::PrDispatchConfig {
            agents_on_pr_open: vec![config::PrDispatchEntry {
                name: "pr-validator".to_string(),
                context: "adf/proxy-validation".to_string(),
            }],
        },
    );
    config.pr_dispatch_per_project.insert(
        "digital-twins".to_string(),
        config::PrDispatchConfig {
            agents_on_pr_open: vec![config::PrDispatchEntry {
                name: "pr-validator".to_string(),
                context: "adf/digital-validation".to_string(),
            }],
        },
    );
    config
}

impl AgentOrchestrator {
    /// Handle a `DispatchTask::ReviewPr` dispatch: run the routing engine,
    /// enforce the C1/C3 provider allow-list, and spawn the pr-reviewer agent
    /// with `ADF_PR_*` env overrides carrying the per-dispatch context.
    ///
    /// The task is a no-op (with a warn log) when no `pr-reviewer` agent is
    /// configured for the project yet. Step E adds the canonical
    /// `pr-reviewer.toml` fragment; until then this method must not crash the
    /// reconcile loop.
    ///
    /// Unlike [`spawn_agent`], this path skips persona composition, skill
    /// chain injection, and worktree creation. The pr-reviewer is review-tier
    /// (read-only), so the heavyweight scaffolding from the implementation
    /// spawn path is intentionally left out.
    ///
    /// [`spawn_agent`]: AgentOrchestrator::spawn_agent
    pub(crate) async fn handle_review_pr(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha, head_ref, author_login, title, diff_loc) = match task {
            dispatcher::DispatchTask::ReviewPr {
                pr_number,
                project,
                head_sha,
                head_ref,
                author_login,
                title,
                diff_loc,
            } => (
                pr_number,
                project,
                head_sha,
                head_ref,
                author_login,
                title,
                diff_loc,
            ),
            other => {
                warn!(task = ?other, "handle_review_pr invoked with non-ReviewPr task; ignoring");
                return Ok(());
            }
        };

        let req = pr_dispatch::ReviewPrRequest {
            pr_number,
            project: project.clone(),
            head_sha: head_sha.clone(),
            head_ref,
            author_login,
            title,
            diff_loc,
        };

        // ADF Phase 2 (issue #944): fan-out over the configured
        // `agents_on_pr_open` list. Each entry is gated independently
        // (subscription allow-list + per-agent monthly budget). Only
        // entries that successfully spawn get a `pending` commit status
        // — a `pending` from a skipped agent would block the PR forever.
        // When `[pr_dispatch]` is absent the legacy default ships a single
        // pr-reviewer entry, preserving pre-Phase-2 behaviour.
        let entries = self.config.agents_on_pr_open_for_project(&project);
        for entry in entries {
            let spawned = match entry.name.as_str() {
                "build-runner" => {
                    self.dispatch_build_runner_for_pr(&req, &entry.context)
                        .await?
                }
                _ => {
                    self.dispatch_pr_reviewer_for_pr(&req, &entry.name, &entry.context)
                        .await?
                }
            };
            if spawned {
                self.post_pending_status(
                    &head_sha,
                    pr_number,
                    &project,
                    &entry.name,
                    &entry.context,
                    &format!("{} dispatched", entry.name),
                )
                .await;
            }
        }

        Ok(())
    }

    /// Dispatch a project-scoped PR gate from a comment mention on a pull
    /// request issue. This deliberately reuses the ReviewPr spawn machinery so
    /// mention-triggered gates get the same bounded evidence prompt,
    /// `PrGateMeta`, pending status, and fail-closed terminal parsing as
    /// pull_request webhook fan-out.
    pub(crate) async fn dispatch_pr_gate_mention_for_issue(
        &mut self,
        project: &str,
        agent_name: &str,
        issue_number: u64,
    ) -> Result<bool, OrchestratorError> {
        let Some(entry) = self.pr_gate_dispatch_entry(project, agent_name) else {
            return Ok(false);
        };

        if self
            .agent_registry
            .lookup_project(project, agent_name)
            .is_none()
        {
            warn!(
                project,
                agent = %agent_name,
                issue = issue_number,
                "PR gate mention rejected: agent not configured for project"
            );
            return Ok(false);
        }

        let Some(req) = self
            .review_pr_request_from_project_issue(project, issue_number)
            .await?
        else {
            warn!(
                project,
                agent = %agent_name,
                issue = issue_number,
                "PR gate mention rejected: issue is not a resolvable pull request"
            );
            return Ok(false);
        };

        if !valid_pr_gate_mention_request(project, agent_name, issue_number, &req) {
            return Ok(false);
        }

        let spawned = self
            .dispatch_pr_reviewer_for_pr(&req, agent_name, &entry.context)
            .await?;
        if spawned {
            self.post_pending_status(
                &req.head_sha,
                req.pr_number,
                project,
                agent_name,
                &entry.context,
                &format!("{agent_name} dispatched by mention"),
            )
            .await;
        }
        Ok(spawned)
    }

    /// Single funnel for every comment-triggered canonical PR gate mention
    /// (webhook and polling, qualified and generic). Applies the
    /// project-scoped duplicate guard, dispatches through
    /// [`Self::dispatch_pr_gate_mention_for_issue`], and stamps the
    /// mention-chain metadata onto the agent stored under its
    /// [`pr_gate_agent_key`].
    pub(crate) async fn dispatch_canonical_pr_gate_mention_from_comment(
        &mut self,
        project: &str,
        agent_name: &str,
        issue_number: u64,
        chain_id: String,
        depth: u32,
        parent_agent: String,
    ) -> Result<bool, OrchestratorError> {
        if self.should_skip_local_dispatch(project, agent_name, issue_number) {
            return Ok(false);
        }

        let spawned = self
            .dispatch_pr_gate_mention_for_issue(project, agent_name, issue_number)
            .await?;
        if spawned {
            if let Some(agent) = self
                .active_agents
                .get_mut(&pr_gate_agent_key(project, agent_name))
            {
                agent.spawned_by_mention = true;
                agent.mention_chain_id = Some(chain_id);
                agent.mention_depth = Some(depth);
                agent.mention_parent_agent = if parent_agent.is_empty() {
                    None
                } else {
                    Some(parent_agent)
                };
            }
        }
        Ok(spawned)
    }

    pub(crate) fn pr_gate_dispatch_entry(
        &self,
        project: &str,
        agent_name: &str,
    ) -> Option<config::PrDispatchEntry> {
        if project == dispatcher::LEGACY_PROJECT_ID {
            return None;
        }
        self.config
            .agents_on_pr_open_for_project(project)
            .into_iter()
            .find(|entry| entry.name == agent_name && is_canonical_pr_gate_agent(&entry.name))
    }

    async fn review_pr_request_from_project_issue(
        &self,
        project: &str,
        issue_number: u64,
    ) -> Result<Option<pr_dispatch::ReviewPrRequest>, OrchestratorError> {
        let Some(gitea) = self.gitea_config_for_project(project) else {
            return Ok(None);
        };
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            gitea.base_url.trim_end_matches('/'),
            gitea.owner,
            gitea.repo,
            issue_number
        );
        let response = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("token {}", gitea.token))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| OrchestratorError::Config(format!("failed to fetch PR metadata: {e}")))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(OrchestratorError::Config(format!(
                "failed to fetch PR metadata for {project}#{issue_number}: HTTP {status}: {text}"
            )));
        }
        let pr: GiteaPullForGateMention = response
            .json()
            .await
            .map_err(|e| OrchestratorError::Config(format!("failed to parse PR metadata: {e}")))?;
        Ok(Some(pr_dispatch::ReviewPrRequest {
            pr_number: pr.number,
            project: project.to_string(),
            head_sha: pr.head.sha,
            head_ref: pr.head.ref_name,
            author_login: pr.user.login,
            title: pr.title,
            diff_loc: pr.additions.saturating_add(pr.deletions),
        }))
    }

    fn gitea_config_for_project(&self, project: &str) -> Option<&config::GiteaOutputConfig> {
        self.config
            .projects
            .iter()
            .find(|p| p.id == project)
            .and_then(|p| p.gitea.as_ref())
            .or(self.config.gitea.as_ref())
    }

    /// Phase 2 helper: spawn the LLM-style PR review agent (`pr-reviewer`
    /// or any future fan-out entry that runs through the routing engine).
    ///
    /// Returns `Ok(true)` when the agent was spawned and is now in
    /// `active_agents`; `Ok(false)` when it was gated out (no agent
    /// configured for the project, banned static or routed model, or
    /// budget exhausted). The caller posts a `pending` commit status only
    /// when this returns `true`.
    async fn dispatch_pr_reviewer_for_pr(
        &mut self,
        req: &pr_dispatch::ReviewPrRequest,
        agent_name: &str,
        commit_status_context: &str,
    ) -> Result<bool, OrchestratorError> {
        let pr_number = req.pr_number;
        let project = req.project.clone();
        let head_sha = req.head_sha.clone();

        // Look up the agent for this project. Missing entries in the
        // fan-out list must skip silently (no `pending` posted) — a
        // hung pending would block the PR forever.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), agent_name)
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    pr_number,
                    project = %project,
                    agent = %agent_name,
                    "ReviewPr skipped: no agent configured for project"
                );
                return Ok(false);
            }
        };

        // === STATIC ALLOW-LIST GATE (pre-routing) ===
        // Belt-and-braces: the load-time config validator rejects banned
        // providers, and `RoutingDecisionEngine` filters them from the
        // candidate pool, but this check guarantees the spawn never runs
        // against a banned static `model` even if the config was mutated
        // at runtime or a future refactor drops the routing filter.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    pr_number,
                    project = %project,
                    model = %static_model,
                    "ReviewPr skipped: static model rejected by subscription allow-list"
                );
                return Ok(false);
            }
        }

        // === BUDGET GATE ===
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                verdict = %budget_verdict,
                "ReviewPr skipped: monthly budget exhausted"
            );
            return Ok(false);
        }

        // === ROUTING ===
        // Build a DispatchContext off the per-PR task string so KG/keyword
        // routing can pick a model based on "review" keywords and PR shape.
        let task_string = pr_dispatch::build_review_task(req);
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
        let dispatch_ctx = control_plane::DispatchContext {
            agent_name: def.name.clone(),
            task: task_string.clone(),
            static_model: def.model.clone(),
            cli_tool: def.cli_tool.clone(),
            layer: def.layer,
            session_id: None,
            default_tier: def.default_tier.clone(),
        };
        let decision = engine.decide_route(&dispatch_ctx, &budget_verdict).await;
        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            model = %decision.candidate.model,
            rationale = %decision.rationale,
            "ReviewPr routing decision"
        );

        // === C1/C3 ALLOW-LIST GATE ===
        // Routing may suggest a banned provider (e.g. via stale KG rules); the
        // subscription-only allow-list must still short-circuit the spawn so
        // unsanctioned providers never launch.
        let routed_model = decision.candidate.model.clone();
        let effective_cli = if decision.candidate.cli_tool.is_empty() {
            def.cli_tool.clone()
        } else {
            decision.candidate.cli_tool.clone()
        };
        if !routed_model.is_empty() && !config::is_allowed_provider(&routed_model) {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                model = %routed_model,
                "ReviewPr skipped: routed model rejected by subscription allow-list"
            );
            return Ok(false);
        }

        // === SPAWN ===
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: effective_cli.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let fallback_provider = def.fallback_provider.as_ref().map(|fallback_cli| {
            terraphim_types::capability::Provider {
                id: format!("{}-fallback", def.name),
                name: format!("{} (fallback)", def.name),
                provider_type: terraphim_types::capability::ProviderType::Agent {
                    agent_id: format!("{}-fallback", def.name),
                    cli_command: fallback_cli.clone(),
                    working_dir: self.config.working_dir_for_agent(&def),
                },
                capabilities: vec![],
                cost_level: terraphim_types::capability::CostLevel::Cheap,
                latency: terraphim_types::capability::Latency::Medium,
                keywords: def.capabilities.clone(),
            }
        });

        // Keep the compact PR summary for routing/telemetry only. The spawned
        // gate receives a native bounded evidence prompt so it does not have to
        // discover PR context, read skills dynamically, or post statuses.
        let working_dir = self.config.working_dir_for_agent(&def);
        // Issue #3293: the PR gate dispatch contract is fail-closed. When
        // authoritative evidence cannot be assembled there is no
        // degraded evidence substitution and no spawn — the typed
        // error propagates with deterministic diagnostics.
        let evidence = pr_gate_context::build_pr_gate_evidence_pack(
            req,
            Some(working_dir.as_path()),
            pr_gate_context::PrGateEvidenceLimits::default(),
        )
        .await
        .map_err(|e| OrchestratorError::PrGateEvidenceUnavailable {
            project: project.clone(),
            agent: def.name.clone(),
            pr_number,
            head_sha: head_sha.clone(),
            reason: e.to_string(),
        })?;
        let gate_meta = crate::pr_gate_result::PrGateMeta {
            pr_number,
            project: project.clone(),
            agent_name: def.name.clone(),
            context: commit_status_context.to_string(),
            head_sha: head_sha.clone(),
            dispatch_id: crate::pr_gate_result::PrGateDispatchId::new(
                &project, pr_number, &head_sha, &def.name,
            ),
        };
        let gate_kind = pr_gate_prompt::PrGateKind::for_agent(&def.name);
        let gate_prompt = pr_gate_prompt::build_pr_gate_prompt(gate_kind, &gate_meta, &evidence);
        let mut request = SpawnRequest::new(primary_provider, gate_prompt)
            .with_stdin()
            .with_default_tools_disabled();
        if !routed_model.is_empty() {
            request = request.with_primary_model(&routed_model);
        }
        if let Some(fallback) = fallback_provider {
            request = request.with_fallback_provider(fallback);
            if let Some(fallback_model) = &def.fallback_model {
                request = request.with_fallback_model(fallback_model);
            }
        }

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        let base_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        let spawn_ctx = pr_dispatch::layer_pr_env(base_ctx, req)
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
            .copied()
            .unwrap_or(0);

        self.active_agents.insert(
            pr_gate_agent_key(&project, &def.name),
            ManagedAgent {
                definition: def.clone(),
                handle,
                started_at: Instant::now(),
                restart_count,
                output_rx,
                spawned_by_mention: false,
                worktree_path: None,
                worktree_guard: None,
                routed_model: if routed_model.is_empty() {
                    None
                } else {
                    Some(routed_model)
                },
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some(CommitStatusPost {
                    project: project.clone(),
                    head_sha: head_sha.clone(),
                    context: commit_status_context.to_string(),
                }),
                gate_meta: Some(gate_meta),
                output_tmp_path,
            },
        );

        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            head_sha = %head_sha,
            "ReviewPr spawned LLM review agent"
        );

        Ok(true)
    }

    /// Phase 2 helper: spawn the deterministic `build-runner` agent on a
    /// `pull_request.opened` event. Mirrors `handle_push`'s spawn pipeline
    /// but injects PR-shaped `ADF_PUSH_*` env (using
    /// `refs/pull/<n>/head` as the synthetic ref) so the same bash task
    /// script handles both push events and PR opens.
    ///
    /// Skips the routing engine — `build-runner` is bash-only (no LLM, no
    /// model) so a routing decision would invite a false-positive
    /// banned-provider check on an unset `def.model`. Logs a synthetic
    /// `model = "n/a"` row for parity with the LLM path.
    ///
    /// Returns `Ok(true)` on successful spawn (caller posts pending);
    /// `Ok(false)` when gated out.
    async fn dispatch_build_runner_for_pr(
        &mut self,
        req: &pr_dispatch::ReviewPrRequest,
        commit_status_context: &str,
    ) -> Result<bool, OrchestratorError> {
        let pr_number = req.pr_number;
        let project = req.project.clone();
        let head_sha = req.head_sha.clone();

        if self.active_agents.contains_key("build-runner") {
            info!(
                pr_number,
                project = %project,
                head_sha = %head_sha,
                "ReviewPr skipped build-runner: already active from concurrent push dispatch"
            );
            return Ok(false);
        }

        // Look up the build-runner agent for this project. Missing must
        // skip silently — no `pending` posted by the caller.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), "build-runner")
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    pr_number,
                    project = %project,
                    "ReviewPr skipped: no build-runner agent configured for project"
                );
                return Ok(false);
            }
        };

        // === STATIC ALLOW-LIST GATE ===
        // build-runner is bash-only (no LLM), so def.model is normally None
        // and this gate is a no-op. The check is retained for defence in
        // depth so a future config that mis-sets `model` cannot bypass C1/C3.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    pr_number,
                    project = %project,
                    model = %static_model,
                    "ReviewPr skipped: build-runner static model rejected by subscription allow-list"
                );
                return Ok(false);
            }
        }

        // === BUDGET GATE ===
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                pr_number,
                project = %project,
                verdict = %budget_verdict,
                "ReviewPr skipped: build-runner monthly budget exhausted"
            );
            return Ok(false);
        }

        // === ROUTING DECISION (observability only) ===
        // build-runner is bash; mirror handle_push's synthetic log row so
        // dashboards see one entry per dispatch. No call to decide_route —
        // an LLM router on an unset model would surface false positives.
        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            model = "n/a",
            cost_estimate_cents = 0,
            rationale = "deterministic build-runner (no LLM)",
            "ReviewPr routing decision"
        );

        // === SPAWN ===
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let task_string = format!(
            "Build/test verdict for PR #{} (head={}, {} LOC, project={}, author={})",
            pr_number, head_sha, req.diff_loc, project, req.author_login,
        );

        // Issue #1020: pass the TOML `task` body (the bash script that
        // does git fetch / rch exec / curl status post) to the spawner
        // -- not the runtime informational summary, which would have
        // been interpreted as `bash -c "Build/test verdict ..."` and
        // exited 127 on the first non-existent command.
        let mut request = SpawnRequest::new(primary_provider, &def.task);

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        // Layer ADF_PUSH_* env on top of the per-agent base context.
        // The ref is synthesised as `refs/pull/<n>/head` so the task
        // script can `git fetch origin <ref> && git checkout <sha>`
        // identically to a push-event dispatch. `ADF_PUSH_BEFORE_SHA`
        // is empty because the ReviewPr dispatch task does not carry
        // the PR base SHA — the build-runner script only requires
        // `ADF_PUSH_SHA` and `ADF_PUSH_REF`.
        // ADF_TASK_SUMMARY exposes the runtime summary so the task can
        // log it without a code change (issue #1020).
        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        spawn_ctx = spawn_ctx
            .with_env("ADF_PUSH_SHA", head_sha.clone())
            .with_env("ADF_PUSH_REF", format!("refs/pull/{}/head", pr_number))
            .with_env("ADF_PUSH_PROJECT", project.clone())
            .with_env("ADF_PUSH_BEFORE_SHA", String::new())
            .with_env("ADF_PUSH_PUSHER", req.author_login.clone())
            .with_env("ADF_PUSH_FILES", String::new())
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
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
                worktree_path: None,
                worktree_guard: None,
                routed_model: None,
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some(CommitStatusPost {
                    project: project.clone(),
                    head_sha: head_sha.clone(),
                    context: commit_status_context.to_string(),
                }),
                gate_meta: None,
                output_tmp_path,
            },
        );

        info!(
            agent = %def.name,
            pr_number,
            project = %project,
            head_sha = %head_sha,
            "ReviewPr spawned build-runner"
        );

        Ok(true)
    }

    /// Post a `pending` commit status for the given `context` against the
    /// PR head SHA.
    ///
    /// Generalised from Phase 1's `post_pr_reviewer_pending_status` so the
    /// Phase 2 PR-fan-out path can post one pending per dispatched agent
    /// (one row per `agents_on_pr_open` entry that successfully spawned).
    ///
    /// Best-effort: when the workflow tracker isn't configured (e.g. in
    /// unit tests) or the API call fails we log and return without
    /// surfacing the error. The agent itself owns the final state
    /// transition (success / failure / error).
    async fn post_pending_status(
        &mut self,
        head_sha: &str,
        pr_number: u64,
        project: &str,
        agent_name: &str,
        context: &str,
        description: &str,
    ) {
        let Some(tracker) = self.commit_status_tracker(project, agent_name) else {
            debug!(
                pr_number,
                project,
                agent = agent_name,
                context,
                "ReviewPr: no project tracker configured; skipping pending status"
            );
            return;
        };
        let owner = tracker.owner().to_string();
        let repo = tracker.repo().to_string();
        let result = tracker
            .set_commit_status(
                &owner,
                &repo,
                head_sha,
                terraphim_tracker::StatusState::Pending,
                context,
                description,
                None,
            )
            .await;
        match result {
            Ok(()) => {
                info!(
                    pr_number,
                    project, head_sha, context, "ReviewPr: posted pending status"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    pr_number,
                    project,
                    head_sha,
                    context,
                    "ReviewPr: failed to post pending status"
                );
            }
        }
    }

    /// Post a terminal (success/failure) commit status for an agent that
    /// exited. Best-effort: logs on failure but does not propagate errors.
    ///
    /// Resolves the tracker exactly like the pending status did, so the
    /// terminal state lands on the same project repository and context.
    pub(crate) async fn post_terminal_commit_status(
        &mut self,
        project: &str,
        agent_name: &str,
        head_sha: &str,
        context: &str,
        state: terraphim_tracker::StatusState,
        description: &str,
    ) {
        let Some(tracker) = self.commit_status_tracker(project, agent_name) else {
            debug!(
                project,
                agent = agent_name,
                head_sha,
                context,
                "post_terminal_commit_status: no project tracker; skipping"
            );
            return;
        };
        let owner = tracker.owner().to_string();
        let repo = tracker.repo().to_string();
        match tracker
            .set_commit_status(&owner, &repo, head_sha, state, context, description, None)
            .await
        {
            Ok(()) => {
                info!(
                    project,
                    owner, repo, head_sha, context, "posted terminal commit status"
                );
            }
            Err(e) => {
                warn!(
                    error = %e,
                    project,
                    owner,
                    repo,
                    head_sha,
                    context,
                    "failed to post terminal commit status"
                );
            }
        }
    }

    /// Tracker used for both pending and terminal commit statuses of a
    /// `(project, agent)` pair.
    ///
    /// With an [`OutputPoster`] configured the project's tracker is
    /// authoritative (per-agent token when one exists, otherwise the project
    /// default); a project the poster does not know is skipped rather than
    /// silently redirected to the workflow-global tracker. The workflow
    /// tracker is only used when no poster exists at all (legacy
    /// single-project deployments and unit tests).
    fn commit_status_tracker(
        &mut self,
        project: &str,
        agent_name: &str,
    ) -> Option<&terraphim_tracker::GiteaTracker> {
        if self.output_poster.is_some() {
            return self
                .output_poster
                .as_ref()
                .and_then(|poster| poster.tracker_for(project, agent_name));
        }
        self.get_or_init_pre_check_tracker()
    }

    /// Handle a `DispatchTask::Push` dispatch (Phase 3 — ADF replaces Gitea
    /// Actions): look up the project's `build-runner` agent, gate on the
    /// subscription allow-list and monthly budget, log a routing decision row
    /// for observability (even though `build-runner` is bash, not LLM), then
    /// spawn it with `ADF_PUSH_*` env injection so the bash task can shell
    /// out to `rch exec` for the deterministic cargo gates.
    ///
    /// The handler is a no-op (with warn log) when no `build-runner` agent is
    /// configured for the project — repos without build-runner must not break
    /// the orchestrator drain loop.
    pub(crate) async fn handle_push(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (project, ref_name, before_sha, after_sha, pusher_login, files_changed) = match task {
            dispatcher::DispatchTask::Push {
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            } => (
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            ),
            other => {
                warn!(task = ?other, "handle_push invoked with non-Push task; ignoring");
                return Ok(());
            }
        };

        // Look up the build-runner agent for this project. Repos without
        // build-runner shouldn't break the orchestrator -- log and skip.
        let def = match self
            .agent_registry
            .lookup_project(project.as_str(), "build-runner")
        {
            Some(agent) => agent.definition.clone(),
            None => {
                warn!(
                    project = %project,
                    after_sha = %after_sha,
                    "Push skipped: no build-runner agent configured for project"
                );
                return Ok(());
            }
        };

        if !def.enabled {
            info!(
                agent = %def.name,
                project = %project,
                "Push skipped: build-runner agent is disabled"
            );
            return Ok(());
        }

        if self.active_agents.contains_key("build-runner") {
            info!(
                project = %project,
                after_sha = %after_sha,
                "Push skipped build-runner: already active from concurrent dispatch"
            );
            return Ok(());
        }

        // === STATIC ALLOW-LIST GATE ===
        // build-runner is bash-only (no LLM), so def.model is normally None
        // and this gate is a no-op. The check is retained for defence in
        // depth so a future config that mis-sets `model` cannot bypass C1/C3.
        if let Some(static_model) = def.model.as_deref() {
            if !config::is_allowed_provider(static_model) {
                warn!(
                    agent = %def.name,
                    project = %project,
                    model = %static_model,
                    "Push skipped: static model rejected by subscription allow-list"
                );
                return Ok(());
            }
        }

        // === BUDGET GATE ===
        // build-runner has no LLM cost but the budget tracker still records
        // its dispatches; pause if the operator deliberately capped it.
        let budget_verdict = self.cost_tracker.check(&def.name);
        if budget_verdict.should_pause() {
            warn!(
                agent = %def.name,
                project = %project,
                verdict = %budget_verdict,
                "Push skipped: build-runner monthly budget exhausted"
            );
            return Ok(());
        }

        // === ROUTING DECISION (observability only) ===
        // Even though build-runner is bash, we still log a routing decision
        // row so the dashboard sees one entry per dispatch. Cost is 0 because
        // there is no LLM, and the model column reads "n/a".
        info!(
            agent = %def.name,
            project = %project,
            ref_name = %ref_name,
            after_sha = %after_sha,
            model = "n/a",
            cost_estimate_cents = 0,
            rationale = "deterministic build-runner (no LLM)",
            "Push routing decision"
        );

        // === SPAWN ===
        // build-runner is a plain bash agent: cli_tool from the def, no
        // primary model, no fallback model. Mirror the SpawnRequest shape
        // used by handle_review_pr but skip the LLM-specific overrides.
        let primary_provider = terraphim_types::capability::Provider {
            id: def.name.clone(),
            name: def.name.clone(),
            provider_type: terraphim_types::capability::ProviderType::Agent {
                agent_id: def.name.clone(),
                cli_command: def.cli_tool.clone(),
                working_dir: self.config.working_dir_for_agent(&def),
            },
            capabilities: vec![],
            cost_level: terraphim_types::capability::CostLevel::Cheap,
            latency: terraphim_types::capability::Latency::Medium,
            keywords: def.capabilities.clone(),
        };

        let task_string = format!(
            "Build/test verdict for push to {} ({} → {}, {} files changed) on project={}, pushed by {}",
            ref_name,
            before_sha,
            after_sha,
            files_changed.len(),
            project,
            pusher_login,
        );

        // Issue #1020: pass the TOML `task` body (build-runner bash
        // script) to the spawner -- not the runtime informational
        // summary. The summary is layered as ADF_TASK_SUMMARY env.
        let mut request = SpawnRequest::new(primary_provider, &def.task);

        let mut limits = ResourceLimits::default();
        if let Some(max_cpu) = def.max_cpu_seconds {
            limits.max_cpu_seconds = Some(max_cpu);
        }
        if let Some(max_mem) = def.max_memory_bytes {
            limits.max_memory_bytes = Some(max_mem);
        }
        request = request.with_resource_limits(limits);

        // Layer the ADF_PUSH_* env on top of the per-agent base context.
        let mut spawn_ctx =
            build_spawn_context_for_agent(&self.config, &def, self.output_poster.as_ref());
        spawn_ctx = spawn_ctx
            .with_env("ADF_PUSH_SHA", after_sha.clone())
            .with_env("ADF_PUSH_REF", ref_name.clone())
            .with_env("ADF_PUSH_PROJECT", project.clone())
            .with_env("ADF_PUSH_BEFORE_SHA", before_sha.clone())
            .with_env("ADF_PUSH_PUSHER", pusher_login.clone())
            .with_env("ADF_PUSH_FILES", files_changed.join("\n"))
            .with_env("ADF_TASK_SUMMARY", task_string.clone());

        let handle = self
            .spawner
            .spawn_with_fallback(&request, spawn_ctx)
            .await
            .map_err(|e| OrchestratorError::SpawnFailed {
                agent: def.name.clone(),
                reason: e.to_string(),
            })?;

        let output_rx = handle.subscribe_output();
        let output_tmp_path = self.start_output_log_drain(&def.name, &handle);
        let restart_count = self
            .restart_counts
            .get(&agent_key(&def))
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
                worktree_path: None,
                worktree_guard: None,
                routed_model: None,
                session_id: format!("{}-{}", def.name, ulid::Ulid::new()),
                mention_chain_id: None,
                mention_depth: None,
                mention_parent_agent: None,
                concurrency_permit: None,
                commit_status_post: Some(CommitStatusPost {
                    project: project.clone(),
                    head_sha: after_sha.clone(),
                    context: "adf/build".to_string(),
                }),
                gate_meta: None,
                output_tmp_path,
            },
        );

        self.post_pending_status(
            &after_sha,
            0,
            &project,
            &def.name,
            "adf/build",
            "build-runner dispatched",
        )
        .await;

        info!(
            agent = %def.name,
            project = %project,
            ref_name = %ref_name,
            after_sha = %after_sha,
            "Push spawned build-runner"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::AgentOrchestrator;

    #[test]
    fn qualified_gate_uses_project_definition_and_status_tracker() {
        let orchestrator =
            AgentOrchestrator::new(super::duplicate_gate_config_for_tests()).unwrap();

        let proxy_entry = orchestrator
            .pr_gate_dispatch_entry("terraphim-llm-proxy", "pr-validator")
            .expect("proxy pr-validator gate entry");
        assert_eq!(proxy_entry.context, "adf/proxy-validation");

        let digital_entry = orchestrator
            .pr_gate_dispatch_entry("digital-twins", "pr-validator")
            .expect("digital pr-validator gate entry");
        assert_eq!(digital_entry.context, "adf/digital-validation");

        let proxy_def = orchestrator
            .agent_registry
            .lookup_project("terraphim-llm-proxy", "pr-validator")
            .expect("proxy agent");
        assert_eq!(proxy_def.definition.task, "proxy validator");

        let poster = orchestrator.output_poster.as_ref().expect("output poster");
        let tracker = poster
            .tracker_for("terraphim-llm-proxy", "adf/proxy-validation")
            .expect("proxy tracker");
        assert_eq!(tracker.owner(), "proxy-owner");
        assert_eq!(tracker.repo(), "proxy-repo");
    }

    #[test]
    fn pr_gate_request_validation_rejects_wrong_project_or_head() {
        let req = crate::pr_dispatch::ReviewPrRequest {
            pr_number: 38,
            project: "digital-twins".to_string(),
            head_sha: "abc123".to_string(),
            head_ref: "feature".to_string(),
            author_login: "alice".to_string(),
            title: "test".to_string(),
            diff_loc: 10,
        };

        assert!(!super::valid_pr_gate_mention_request(
            "terraphim-llm-proxy",
            "pr-validator",
            38,
            &req
        ));

        let empty_head = crate::pr_dispatch::ReviewPrRequest {
            project: "terraphim-llm-proxy".to_string(),
            head_sha: " ".to_string(),
            ..req.clone()
        };
        assert!(!super::valid_pr_gate_mention_request(
            "terraphim-llm-proxy",
            "pr-validator",
            38,
            &empty_head
        ));

        let wrong_issue = crate::pr_dispatch::ReviewPrRequest {
            project: "terraphim-llm-proxy".to_string(),
            head_sha: "abc123".to_string(),
            ..req
        };
        assert!(!super::valid_pr_gate_mention_request(
            "terraphim-llm-proxy",
            "pr-validator",
            39,
            &wrong_issue
        ));
    }
}
