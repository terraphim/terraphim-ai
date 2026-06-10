//! Auto-merge and PR-gate reconciliation capability for `AgentOrchestrator`:
//! polling pending reviews, executing auto-merge, the post-merge test gate,
//! PR-gate reconciliation, and remediation-issue creation. Split from lib.rs
//! as part of the Gitea #1910 god-file decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use tracing::{info, warn};

use crate::dispatcher::DispatchTask;
#[cfg(feature = "quickwit")]
use crate::quickwit;
use crate::{
    config, dispatcher, post_merge_gate, pr_gate, pr_poller, pr_review, truncate_for_issue,
    AgentOrchestrator, OrchestratorError,
};

impl AgentOrchestrator {
    /// Poll every project with a Gitea config for open PRs, parse the latest
    /// structural-pr-review comment, and enqueue [`dispatcher::DispatchTask::AutoMerge`]
    /// for any PR that clears every gate in
    /// [`pr_review::AutoMergeCriteria::default`].
    ///
    /// Called once per reconcile tick after the
    /// dispatcher has been drained so AutoMerge tasks enqueued here are
    /// serviced on the next tick (deterministic ordering). The method is a
    /// no-op when no project has a `gitea` config.
    ///
    /// This is ROC v1 Step F — it enqueues auto-merge but does **not**
    /// actually merge the PR; that lands in Step G. Dedupe is process-local
    /// via [`pr_poller::AutoMergeDedupeSet`]; durable tracking is Step I.
    pub async fn poll_pending_reviews(&mut self) -> Result<(), OrchestratorError> {
        // Build the list of (project_id, gitea_cfg) targets. Mirrors the
        // legacy/multi-project split used by [`Self::poll_mentions`] so the
        // two pollers stay aligned as the config surface evolves.
        let targets: Vec<(String, config::GiteaOutputConfig)> = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) => vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g)],
                None => {
                    tracing::debug!(
                        "verdict polling skipped: legacy mode with no top-level gitea config"
                    );
                    return Ok(());
                }
            }
        } else {
            self.config
                .projects
                .iter()
                .filter_map(|project| {
                    let gitea = project.gitea.clone()?;
                    Some((project.id.clone(), gitea))
                })
                .collect()
        };

        if targets.is_empty() {
            tracing::debug!("verdict polling skipped: no projects with Gitea config");
            return Ok(());
        }

        let criteria = pr_review::AutoMergeCriteria::default();

        for (project_id, gitea_cfg) in targets {
            let tracker_cfg = terraphim_tracker::GiteaConfig {
                base_url: gitea_cfg.base_url.clone(),
                token: gitea_cfg.token.clone(),
                owner: gitea_cfg.owner.clone(),
                repo: gitea_cfg.repo.clone(),
                active_states: vec!["open".to_string()],
                terminal_states: vec!["closed".to_string()],
                use_robot_api: false,
                robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
                claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
            };
            let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
                Ok(t) => pr_poller::GiteaPrTracker::new(
                    t,
                    gitea_cfg.owner.clone(),
                    gitea_cfg.repo.clone(),
                ),
                Err(e) => {
                    tracing::warn!(
                        project = %project_id,
                        error = %e,
                        "failed to create GiteaTracker for verdict polling"
                    );
                    continue;
                }
            };

            self.poll_pending_reviews_for_project(&project_id, &tracker, &criteria)
                .await;
        }

        Ok(())
    }

    /// Inner per-project verdict poll. Accepts a generic [`pr_poller::PrTracker`]
    /// so integration tests can drive it with an in-memory tracker.
    pub async fn poll_pending_reviews_for_project<T: pr_poller::PrTracker + ?Sized>(
        &mut self,
        project_id: &str,
        tracker: &T,
        criteria: &pr_review::AutoMergeCriteria,
    ) {
        let prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                tracing::warn!(
                    project = %project_id,
                    error = %e,
                    "failed to list open PRs"
                );
                return;
            }
        };

        let now = std::time::Instant::now();
        for pr in prs {
            if !self.pr_poll_rate_limiter.allow(project_id, pr.number, now) {
                tracing::trace!(
                    project = %project_id,
                    pr = pr.number,
                    "skipping PR: poll rate limited"
                );
                continue;
            }

            let comments = match tracker.fetch_pr_comments(pr.number).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        error = %e,
                        "failed to fetch PR comments"
                    );
                    continue;
                }
            };

            let head_statuses = match tracker.fetch_head_commit_statuses(&pr.head_sha).await {
                Ok(statuses) => statuses,
                Err(e) => {
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        error = %e,
                        "failed to fetch commit statuses"
                    );
                    continue;
                }
            };

            let outcome =
                pr_poller::evaluate_pr_gates(&pr, &comments, &head_statuses, project_id, criteria);

            // Emit PrReviewed for any outcome that resolved a parsed verdict.
            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let has_verdict = matches!(
                    outcome,
                    pr_poller::EvaluationOutcome::Merge { .. }
                        | pr_poller::EvaluationOutcome::HumanReviewNeeded { .. }
                );
                if has_verdict {
                    if let Some(rc) = pr_poller::latest_reviewer_comment(&comments) {
                        if let Ok(v) = pr_review::parse_verdict(&rc.body, rc.id) {
                            let verdict_str = match &outcome {
                                pr_poller::EvaluationOutcome::Merge { .. } => "GO",
                                _ if v.p0_count > 0 => "NO-GO",
                                _ => "CONDITIONAL",
                            };
                            let event = quickwit::OrchestratorEvent::PrReviewed {
                                pr_number: pr.number,
                                project: project_id.to_string(),
                                head_sha: pr.head_sha.clone(),
                                reviewer_login: rc.user_login.clone(),
                                confidence: v.confidence,
                                p0_count: v.p0_count,
                                p1_count: v.p1_count,
                                verdict: verdict_str.to_string(),
                            };
                            let _ = sink.emit_event(project_id, event).await;
                        }
                    }
                }
            }

            match outcome {
                pr_poller::EvaluationOutcome::Merge { head_sha } => {
                    if !self
                        .auto_merge_enqueued
                        .record_if_new(project_id, pr.number, &head_sha)
                    {
                        tracing::debug!(
                            project = %project_id,
                            pr = pr.number,
                            head = %head_sha,
                            "auto-merge already enqueued for this revision"
                        );
                        continue;
                    }
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        head = %head_sha,
                        "enqueuing AutoMerge for PR that cleared every gate"
                    );
                    self.dispatcher.enqueue(DispatchTask::AutoMerge {
                        pr_number: pr.number,
                        project: project_id.to_string(),
                        head_sha,
                    });
                }
                pr_poller::EvaluationOutcome::HumanReviewNeeded { reason } => {
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "PR requires human review"
                    );
                }
                pr_poller::EvaluationOutcome::AwaitingGates { reason } => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "PR gates not ready for auto-merge yet"
                    );
                }
                pr_poller::EvaluationOutcome::StaleGates { reason } => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "stale gate results; waiting for re-review"
                    );
                }
                pr_poller::EvaluationOutcome::GateParseError { reason } => {
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        reason = %reason,
                        "gate-result comment failed to parse; skipping"
                    );
                }
            }
        }
    }

    /// Execute a [`DispatchTask::AutoMerge`] task — ROC v1 Step G.
    ///
    /// Builds the per-project [`pr_poller::GiteaPrTracker`] from config and
    /// delegates to [`AgentOrchestrator::handle_auto_merge_for_project`].
    /// The task's `project` field must match a configured project with a
    /// `gitea` block (or, for legacy configs, the top-level `gitea`);
    /// otherwise the call logs-and-skips so the dispatcher keeps draining.
    pub async fn handle_auto_merge(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha) = match &task {
            dispatcher::DispatchTask::AutoMerge {
                pr_number,
                project,
                head_sha,
            } => (*pr_number, project.clone(), head_sha.clone()),
            other => {
                warn!(task = ?other, "handle_auto_merge invoked with non-AutoMerge task; ignoring");
                return Ok(());
            }
        };

        // Resolve the Gitea config for this project. Mirrors the legacy /
        // multi-project split used by `poll_pending_reviews`.
        let gitea_cfg: config::GiteaOutputConfig = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) if project == dispatcher::LEGACY_PROJECT_ID => g,
                Some(_) => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: legacy mode but task project id does not match LEGACY_PROJECT_ID"
                    );
                    return Ok(());
                }
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: legacy mode with no top-level gitea config"
                    );
                    return Ok(());
                }
            }
        } else {
            match self
                .config
                .projects
                .iter()
                .find(|p| p.id == project)
                .and_then(|p| p.gitea.clone())
            {
                Some(g) => g,
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "AutoMerge skipped: project has no gitea config"
                    );
                    return Ok(());
                }
            }
        };

        let tracker_cfg = terraphim_tracker::GiteaConfig {
            base_url: gitea_cfg.base_url.clone(),
            token: gitea_cfg.token.clone(),
            owner: gitea_cfg.owner.clone(),
            repo: gitea_cfg.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
            Ok(t) => {
                pr_poller::GiteaPrTracker::new(t, gitea_cfg.owner.clone(), gitea_cfg.repo.clone())
            }
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "AutoMerge skipped: failed to create GiteaTracker"
                );
                return Ok(());
            }
        };

        self.handle_auto_merge_for_project(task, &tracker).await
    }

    /// Inner AutoMerge executor. Accepts any [`pr_poller::AutoMergeExecutor`]
    /// so integration tests can drive the full handler with an in-memory
    /// tracker. Real production code funnels through
    /// [`AgentOrchestrator::handle_auto_merge`].
    ///
    /// Steps:
    /// 1. Defensive re-check: list open PRs on the project. Skip when the
    ///    PR is absent (already closed/merged) or the HEAD SHA has moved.
    /// 2. Attempt the merge.
    /// 3. On success — enqueue [`DispatchTask::PostMergeTestGate`], record
    ///    the `(pr, head_sha)` in the dedupe set so late polls never
    ///    re-enqueue the same revision.
    /// 4. On failure — open an `[ADF]` tracking issue with the failure
    ///    reason via [`pr_poller::AutoMergeExecutor::open_failure_issue`];
    ///    do **not** enqueue a post-merge gate.
    pub async fn handle_auto_merge_for_project<T: pr_poller::AutoMergeExecutor + ?Sized>(
        &mut self,
        task: dispatcher::DispatchTask,
        tracker: &T,
    ) -> Result<(), OrchestratorError> {
        let (pr_number, project, head_sha) = match task {
            dispatcher::DispatchTask::AutoMerge {
                pr_number,
                project,
                head_sha,
            } => (pr_number, project, head_sha),
            other => {
                warn!(task = ?other, "handle_auto_merge_for_project invoked with non-AutoMerge task; ignoring");
                return Ok(());
            }
        };

        // 1. Defensive re-check: ensure the PR is still open and the HEAD
        // SHA matches what the verdict was computed against. If either has
        // moved, the merge decision is stale — skip silently.
        let open_prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "AutoMerge skipped: failed to list open PRs for head_sha re-check"
                );
                return Ok(());
            }
        };

        let live = match open_prs.iter().find(|p| p.number == pr_number) {
            Some(p) => p,
            None => {
                info!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    "AutoMerge skipped: PR no longer in open list (closed/merged already)"
                );
                return Ok(());
            }
        };
        if live.head_sha != head_sha {
            info!(
                pr_number,
                project = %project,
                expected_head = %head_sha,
                live_head = %live.head_sha,
                "AutoMerge skipped: PR HEAD SHA moved since verdict (stale auto-merge decision)"
            );
            return Ok(());
        }

        // 2. Merge.
        match tracker.merge_pr(pr_number).await {
            Ok(outcome) => {
                info!(
                    pr_number,
                    project = %project,
                    merge_sha = %outcome.merge_commit_sha,
                    "pr_auto_merged"
                );

                #[cfg(feature = "quickwit")]
                if let Some(ref sink) = self.quickwit_sink {
                    let event = quickwit::OrchestratorEvent::PrAutoMerged {
                        pr_number,
                        project: project.clone(),
                        merge_sha: outcome.merge_commit_sha.clone(),
                        title: outcome.title.clone(),
                    };
                    let _ = sink.emit_event(&project, event).await;
                }

                // 3a. Defensive dedupe write — covers AutoMerge tasks that
                // reached the handler by a path other than the poller
                // (webhook, manual enqueue, etc.). `record_if_new` is a
                // no-op when the entry already exists.
                let _ = self
                    .auto_merge_enqueued
                    .record_if_new(&project, pr_number, &head_sha);

                // 3b. Enqueue the post-merge test gate (Step H stub).
                self.dispatcher
                    .enqueue(dispatcher::DispatchTask::PostMergeTestGate {
                        pr_number,
                        project: project.clone(),
                        merge_sha: outcome.merge_commit_sha,
                        title: outcome.title,
                    });

                Ok(())
            }
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    head = %head_sha,
                    error = %e,
                    "pr_auto_merge_failed"
                );

                // 4. Open an [ADF] tracking issue with the failure reason,
                //    unless we already created one for this (project, pr, sha)
                //    within the TTL window.
                if self
                    .auto_merge_failure_dedupe
                    .is_recent(&project, pr_number, &head_sha)
                {
                    info!(
                        pr_number,
                        project = %project,
                        head = %head_sha,
                        "AutoMerge failure issue already exists for this PR/SHA; skipping duplicate"
                    );
                } else {
                    let title = format!("[ADF] Auto-merge failed for PR #{pr_number}");
                    let body = format!(
                        "AutoMerge handler failed to merge PR #{pr_number} on project `{project}`.\n\n\
                         Head SHA: `{head_sha}`\n\n\
                         Error: {e}\n\n\
                         The PR was left open; a human needs to investigate (merge conflict, \
                         protected branch, permissions, transient API failure).\n\n\
                         Refs: ROC v1 Step G handler, adf-fleet#35."
                    );
                    let labels = ["adf", "auto-merge-failed", "status/needs-triage"];
                    self.auto_merge_failure_dedupe
                        .record(&project, pr_number, &head_sha);
                    match tracker.open_failure_issue(&title, &body, &labels).await {
                        Ok(_issue_number) => {}
                        Err(issue_err) => {
                            warn!(
                                pr_number,
                                project = %project,
                                error = %issue_err,
                                "AutoMerge failure issue creation also failed; nothing to retry automatically"
                            );
                        }
                    }
                }

                Ok(())
            }
        }
    }

    /// Execute a [`DispatchTask::PostMergeTestGate`] task — ROC v1 Step H.
    ///
    /// Defers the heavy lifting to [`post_merge_gate::run_workspace_tests`]
    /// and [`post_merge_gate::revert_merge`] so those helpers stay fully
    /// testable without orchestrator state. This method resolves the
    /// project's `working_dir` as `repo_root`, constructs the [`post_merge_gate::GateConfig`]
    /// (picking up any overrides from `[post_merge_gate]` in
    /// orchestrator.toml), and funnels the result through the inner
    /// `handle_post_merge_test_gate_for_project` helper which takes a
    /// [`post_merge_gate::CommandRunner`] so integration tests can drive the
    /// full handler with a scripted runner.
    pub async fn handle_post_merge_test_gate(
        &mut self,
        task: dispatcher::DispatchTask,
    ) -> Result<(), OrchestratorError> {
        let runner = post_merge_gate::TokioCommandRunner;
        self.handle_post_merge_test_gate_with_runner(task, &runner)
            .await
    }

    /// Inner handler that accepts any [`post_merge_gate::CommandRunner`].
    /// Integration tests use a [`post_merge_gate::ScriptedRunner`] here to
    /// assert on the exact `cargo test` / `git revert` / `git push` call
    /// sequence without spawning real processes.
    ///
    /// On green: logs `post_merge_gate_verified` at info.
    /// On red: classifies the failure, runs `git revert`, pushes to the
    /// configured remote, opens an `[ADF] post-merge test gate reverted`
    /// tracking issue on the project's Gitea repo, and logs
    /// `post_merge_gate_reverted` at warn. Returns `Ok(())` in every
    /// case the dispatcher should continue draining — only hard I/O
    /// errors that prevent even the attempt return `Err`.
    pub async fn handle_post_merge_test_gate_with_runner<R>(
        &mut self,
        task: dispatcher::DispatchTask,
        runner: &R,
    ) -> Result<(), OrchestratorError>
    where
        R: post_merge_gate::CommandRunner + ?Sized,
    {
        let (pr_number, project, merge_sha, title) = match task {
            dispatcher::DispatchTask::PostMergeTestGate {
                pr_number,
                project,
                merge_sha,
                title,
            } => (pr_number, project, merge_sha, title),
            other => {
                warn!(task = ?other, "handle_post_merge_test_gate invoked with non-PostMergeTestGate task; ignoring");
                return Ok(());
            }
        };

        // Resolve repo_root + gitea tracking target for this project.
        // Legacy mode uses the top-level `working_dir` and `gitea`.
        let (repo_root, gitea_cfg) = if self.config.projects.is_empty() {
            if project != dispatcher::LEGACY_PROJECT_ID {
                warn!(
                    pr_number,
                    project = %project,
                    "PostMergeTestGate skipped: legacy mode but task project id does not match LEGACY_PROJECT_ID"
                );
                return Ok(());
            }
            (self.config.working_dir.clone(), self.config.gitea.clone())
        } else {
            match self.config.projects.iter().find(|p| p.id == project) {
                Some(p) => (p.working_dir.clone(), p.gitea.clone()),
                None => {
                    warn!(
                        pr_number,
                        project = %project,
                        "PostMergeTestGate skipped: no project entry for id"
                    );
                    return Ok(());
                }
            }
        };

        // Build GateConfig from orchestrator overrides (if any).
        let gate_override = self.config.post_merge_gate.clone().unwrap_or_default();
        let cfg = post_merge_gate::GateConfig {
            repo_root,
            merge_sha: merge_sha.clone(),
            max_test_duration: std::time::Duration::from_secs(gate_override.max_test_duration_secs),
            revert_push_remote: gate_override.revert_push_remote,
            revert_push_branch: gate_override.revert_push_branch,
        };

        info!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            max_test_duration_secs = cfg.max_test_duration.as_secs(),
            title = %title,
            "post_merge_gate_start"
        );

        let outcome = match post_merge_gate::run_workspace_tests(runner, &cfg).await {
            Ok(o) => o,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    merge_sha = %merge_sha,
                    error = %e,
                    "post_merge_gate: run_workspace_tests failed before producing an outcome"
                );
                return Ok(());
            }
        };

        if outcome.passed {
            info!(
                pr_number,
                project = %project,
                merge_sha = %merge_sha,
                wall_time_secs = outcome.wall_time.as_secs_f64(),
                "post_merge_gate_verified"
            );
            #[cfg(feature = "quickwit")]
            if let Some(ref sink) = self.quickwit_sink {
                let event = quickwit::OrchestratorEvent::PrAutoMergedVerified {
                    pr_number,
                    project: project.clone(),
                    merge_sha: merge_sha.clone(),
                    wall_time_secs: outcome.wall_time.as_secs_f64(),
                };
                let _ = sink.emit_event(&project, event).await;
            }
            return Ok(());
        }

        let classification = post_merge_gate::classify_failure(&outcome);
        warn!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            kind = ?classification.kind,
            failing_tests = ?classification.failing_tests,
            wall_time_secs = outcome.wall_time.as_secs_f64(),
            "post_merge_gate_failed"
        );

        let revert = match post_merge_gate::revert_merge(runner, &cfg).await {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    pr_number,
                    project = %project,
                    merge_sha = %merge_sha,
                    error = %e,
                    "post_merge_gate: revert_merge failed — manual intervention required"
                );
                // Still try to file the tracking issue below so a human notices.
                post_merge_gate::RevertOutcome {
                    revert_sha: String::new(),
                    pushed: false,
                }
            }
        };

        warn!(
            pr_number,
            project = %project,
            merge_sha = %merge_sha,
            revert_sha = %revert.revert_sha,
            pushed = revert.pushed,
            reason = ?classification.kind,
            "post_merge_gate_reverted"
        );
        #[cfg(feature = "quickwit")]
        if let Some(ref sink) = self.quickwit_sink {
            let event = quickwit::OrchestratorEvent::PrAutoReverted {
                pr_number,
                project: project.clone(),
                merge_sha: merge_sha.clone(),
                revert_sha: revert.revert_sha.clone(),
                reason: format!("{:?}", classification.kind),
                stderr_tail_bytes: outcome.stderr_tail.len() as u32,
            };
            let _ = sink.emit_event(&project, event).await;
        }

        // Open an [ADF] tracking issue. Best-effort — a failure here is
        // logged but does not propagate: the revert has already landed.
        if let Some(gitea) = gitea_cfg {
            let issue_title =
                format!("[ADF] post-merge test gate reverted PR #{pr_number}: {title}");
            let stderr_excerpt = truncate_for_issue(&outcome.stderr_tail, 4000);
            let failing_list = if classification.failing_tests.is_empty() {
                "(none parsed)".to_string()
            } else {
                classification
                    .failing_tests
                    .iter()
                    .map(|t| format!("- `{t}`"))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            let body = format!(
                "Auto-merged PR #{pr_number} on project `{project}` failed the post-merge test gate.\n\n\
                 Merge SHA: `{merge_sha}`\n\
                 Revert SHA: `{}`\n\
                 Revert pushed: {}\n\
                 Failure kind: `{:?}`\n\
                 Wall time: {:.1}s\n\n\
                 Failing tests:\n\n{failing_list}\n\n\
                 stderr tail (truncated):\n\n```\n{stderr_excerpt}\n```\n\n\
                 Refs: ROC v1 Step H, adf-fleet#36.",
                revert.revert_sha,
                revert.pushed,
                classification.kind,
                outcome.wall_time.as_secs_f64(),
            );
            let labels = ["adf", "post-merge-gate", "status/needs-triage"];
            let tracker_cfg = terraphim_tracker::GiteaConfig {
                base_url: gitea.base_url.clone(),
                token: gitea.token.clone(),
                owner: gitea.owner.clone(),
                repo: gitea.repo.clone(),
                active_states: vec!["open".to_string()],
                terminal_states: vec!["closed".to_string()],
                use_robot_api: false,
                robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
                claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
            };
            match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
                Ok(tracker) => {
                    if let Err(e) = tracker.create_issue(&issue_title, &body, &labels).await {
                        warn!(
                            pr_number,
                            project = %project,
                            error = %e,
                            "post_merge_gate: failed to open [ADF] tracking issue"
                        );
                    }
                }
                Err(e) => {
                    warn!(
                        pr_number,
                        project = %project,
                        error = %e,
                        "post_merge_gate: failed to construct tracker for [ADF] issue"
                    );
                }
            }
        } else {
            warn!(
                pr_number,
                project = %project,
                "post_merge_gate: no gitea config for project; skipping [ADF] issue creation"
            );
        }

        Ok(())
    }

    /// PR gate reconciliation: for every project with Gitea config, read
    /// actual commit statuses and branch protection rules, classify each
    /// open PR head via [`pr_gate::reconcile_pr_gate`], and take action.
    ///
    /// Actions:
    /// - `ReadyForPolicy`: no action (Step 18 will handle it).
    /// - `EnqueueMissingChecks`: log which agents need dispatching.
    /// - `AwaitingChecks`: log and skip (rechecked next interval).
    /// - `BlockedByFailedChecks`: open deduplicated remediation issue.
    /// - `FactoryFault`: open deduplicated remediation issue with error.
    ///
    /// Remediation issues are deduplicated using [`pr_gate::remediation_key`]
    /// by searching for existing open issues containing the key.
    pub(crate) async fn reconcile_pr_gates(&mut self) -> Result<(), OrchestratorError> {
        let targets: Vec<(String, config::GiteaOutputConfig)> = if self.config.projects.is_empty() {
            match self.config.gitea.clone() {
                Some(g) => vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g)],
                None => return Ok(()),
            }
        } else {
            self.config
                .projects
                .iter()
                .filter_map(|p| p.gitea.clone().map(|g| (p.id.clone(), g)))
                .collect()
        };

        if targets.is_empty() {
            return Ok(());
        }

        for (project_id, gitea_cfg) in &targets {
            if let Err(e) = self
                .reconcile_pr_gates_for_project(project_id, gitea_cfg)
                .await
            {
                warn!(
                    project = %project_id,
                    error = %e,
                    "reconcile_pr_gates_for_project failed"
                );
            }
        }

        Ok(())
    }

    /// Inner per-project PR gate reconciliation.
    async fn reconcile_pr_gates_for_project(
        &mut self,
        project_id: &str,
        gitea_cfg: &config::GiteaOutputConfig,
    ) -> Result<(), OrchestratorError> {
        let tracker_cfg = terraphim_tracker::GiteaConfig {
            base_url: gitea_cfg.base_url.clone(),
            token: gitea_cfg.token.clone(),
            owner: gitea_cfg.owner.clone(),
            repo: gitea_cfg.repo.clone(),
            active_states: vec!["open".to_string()],
            terminal_states: vec!["closed".to_string()],
            use_robot_api: false,
            robot_path: std::path::PathBuf::from("/home/alex/go/bin/gitea-robot"),
            claim_strategy: terraphim_tracker::gitea::ClaimStrategy::PreferRobot,
        };
        let tracker = match terraphim_tracker::GiteaTracker::new(tracker_cfg) {
            Ok(t) => t,
            Err(e) => {
                warn!(project = %project_id, error = %e, "failed to create GiteaTracker for PR gate reconciliation");
                return Ok(());
            }
        };

        let protection = match tracker
            .get_branch_protection(&gitea_cfg.owner, &gitea_cfg.repo, "main")
            .await
        {
            Ok(p) => p,
            Err(e) => {
                warn!(
                    project = %project_id,
                    error = %e,
                    "failed to get branch protection; skipping PR gate reconciliation"
                );
                return Ok(());
            }
        };

        if !protection.enable_status_check || protection.status_check_contexts.is_empty() {
            tracing::debug!(
                project = %project_id,
                "branch protection has no required status checks; skipping"
            );
            return Ok(());
        }

        let required_contexts = protection.status_check_contexts.clone();

        let prs = match tracker.list_open_prs().await {
            Ok(prs) => prs,
            Err(e) => {
                warn!(project = %project_id, error = %e, "failed to list open PRs for gate reconciliation");
                return Ok(());
            }
        };

        for pr in prs {
            if pr.head_sha.is_empty() {
                continue;
            }

            let statuses = match tracker
                .list_commit_statuses(&gitea_cfg.owner, &gitea_cfg.repo, &pr.head_sha)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    warn!(
                        project = %project_id,
                        pr = pr.number,
                        sha = %pr.head_sha,
                        error = %e,
                        "failed to list commit statuses"
                    );
                    continue;
                }
            };

            let head_statuses: Vec<pr_gate::CommitStatusSummary> = statuses
                .into_iter()
                .map(|s| pr_gate::CommitStatusSummary {
                    context: s.context,
                    state: pr_gate::CommitStatusState::from_api_str(&s.state),
                    created_at_unix: s.created_at.and_then(|ts| ts.parse::<i64>().ok()),
                })
                .collect();

            let snapshot = pr_gate::PrGateSnapshot {
                pr_number: pr.number,
                head_sha: pr.head_sha.clone(),
                base_branch: pr.base_ref.clone(),
                required_contexts: required_contexts.clone(),
                head_statuses,
                now_unix: chrono::Utc::now().timestamp(),
            };

            let decision = pr_gate::reconcile_pr_gate(&snapshot);

            match &decision {
                pr_gate::PrGateDecision::ReadyForPolicy => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        "PR gate: all required contexts green"
                    );
                }
                pr_gate::PrGateDecision::EnqueueMissingChecks { missing } => {
                    tracing::info!(
                        project = %project_id,
                        pr = pr.number,
                        missing = ?missing,
                        "PR gate: missing required contexts"
                    );
                }
                pr_gate::PrGateDecision::AwaitingChecks { pending } => {
                    tracing::debug!(
                        project = %project_id,
                        pr = pr.number,
                        pending = ?pending,
                        "PR gate: awaiting pending checks"
                    );
                }
                pr_gate::PrGateDecision::BlockedByFailedChecks { failed } => {
                    let key =
                        pr_gate::remediation_key(project_id, pr.number, &pr.head_sha, &decision);
                    tracing::warn!(
                        project = %project_id,
                        pr = pr.number,
                        failed = ?failed,
                        key = %key,
                        "PR gate: blocked by failed checks"
                    );
                    if let Err(e) = self
                        .open_remediation_issue_if_needed(
                            &tracker,
                            project_id,
                            pr.number,
                            &pr.head_sha,
                            &key,
                            &format!(
                                "PR #{} blocked by failed required contexts: {}",
                                pr.number,
                                failed
                                    .iter()
                                    .map(|(ctx, state)| format!("{ctx}={state}"))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        )
                        .await
                    {
                        warn!(error = %e, "failed to open remediation issue");
                    }
                }
                pr_gate::PrGateDecision::FactoryFault { error } => {
                    let key =
                        pr_gate::remediation_key(project_id, pr.number, &pr.head_sha, &decision);
                    tracing::error!(
                        project = %project_id,
                        pr = pr.number,
                        error = %error,
                        key = %key,
                        "PR gate: factory fault"
                    );
                    if let Err(e) = self
                        .open_remediation_issue_if_needed(
                            &tracker,
                            project_id,
                            pr.number,
                            &pr.head_sha,
                            &key,
                            &format!("PR #{} factory fault: {error}", pr.number),
                        )
                        .await
                    {
                        warn!(error = %e, "failed to open remediation issue");
                    }
                }
            }
        }

        Ok(())
    }

    /// Open a deduplicated remediation issue. Searches for existing open issues
    /// containing the remediation key before creating a new one.
    async fn open_remediation_issue_if_needed(
        &self,
        tracker: &terraphim_tracker::GiteaTracker,
        project_id: &str,
        pr_number: u64,
        head_sha: &str,
        dedup_key: &str,
        body: &str,
    ) -> Result<(), OrchestratorError> {
        let existing = tracker.search_issues_by_title(dedup_key).await;
        match existing {
            Ok(ids) if !ids.is_empty() => {
                tracing::debug!(
                    project = %project_id,
                    key = %dedup_key,
                    existing_count = ids.len(),
                    "remediation issue already exists; skipping"
                );
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(
                    project = %project_id,
                    error = %e,
                    "failed to search for existing remediation issues; creating anyway"
                );
            }
            Ok(_) => {}
        }

        let title = format!("[ADF] PR gate remediation: {dedup_key}");
        let full_body = format!(
            "{body}\n\n\
             Project: `{project_id}`\n\
             PR: #{pr_number}\n\
             Head SHA: `{head_sha}`\n\
             Dedup key: `{dedup_key}`\n\n\
             This issue was auto-created by the PR gate reconciler.\
             It will be auto-closed when the gate clears."
        );
        let labels = ["adf", "pr-gate", "status/needs-triage"];

        tracker
            .create_issue(&title, &full_body, &labels)
            .await
            .map_err(|e| {
                OrchestratorError::Config(format!("failed to create remediation issue: {e}"))
            })?;

        tracing::info!(
            project = %project_id,
            pr = pr_number,
            key = %dedup_key,
            "opened PR gate remediation issue"
        );

        Ok(())
    }
}
