//! Webhook and direct-dispatch capability for `AgentOrchestrator`: handling
//! immediate dispatch requests from the webhook endpoint, direct dispatches,
//! and reading the current git HEAD. Split from lib.rs as part of the Gitea
//! #1910 god-file decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use std::time::Duration;

use tracing::{error, info, warn};

use crate::{
    dispatcher, mention, mention_chain, webhook, AgentOrchestrator, OrchestratorError,
    ScheduleEvent,
};

impl AgentOrchestrator {
    /// Get current HEAD commit hash.
    pub(crate) async fn get_current_head(&self) -> Result<String, OrchestratorError> {
        let output = tokio::time::timeout(
            Duration::from_secs(5),
            tokio::process::Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&self.config.working_dir)
                .output(),
        )
        .await
        .map_err(|_| OrchestratorError::Config("git rev-parse HEAD timed out after 5s".into()))?
        .map_err(OrchestratorError::from)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(OrchestratorError::Config(
                "git rev-parse HEAD failed".into(),
            ))
        }
    }

    /// Handle a dispatch request received from the webhook endpoint.
    /// This is the webhook equivalent of poll_mentions but immediate.
    pub(crate) async fn handle_webhook_dispatch(&mut self, dispatch: webhook::WebhookDispatch) {
        // Rate limiting: check concurrent mention-spawned agents
        let mention_cfg = match self.config.mentions.as_ref() {
            Some(cfg) => cfg,
            None => return,
        };

        let active_mention_agents = self
            .active_agents
            .values()
            .filter(|a| a.spawned_by_mention)
            .count() as u32;

        if active_mention_agents >= mention_cfg.max_concurrent_mention_agents {
            warn!(
                active = active_mention_agents,
                max = mention_cfg.max_concurrent_mention_agents,
                "webhook dispatch rejected: mention agents at capacity"
            );
            return;
        }

        let agents = self.config.agents.clone();
        let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
        let max_mention_depth = mention_cfg.max_mention_depth;

        match dispatch {
            webhook::WebhookDispatch::SpawnAgent {
                agent_name,
                detected_project,
                issue_number,
                comment_id,
                context,
                synthetic_event: _,
            } => {
                info!(
                    agent = %agent_name,
                    project = ?detected_project,
                    issue = issue_number,
                    comment_id = comment_id,
                    "webhook: dispatching agent spawn"
                );

                // Use project-aware resolver. For webhook dispatches we don't know which
                // project's repo the webhook came from, so we use LEGACY_PROJECT_ID as the
                // hint for unqualified mentions; qualified mentions carry detected_project.
                if let Some(def) = mention::resolve_mention(
                    detected_project.as_deref(),
                    dispatcher::LEGACY_PROJECT_ID,
                    &agent_name,
                    &agents,
                ) {
                    // Event-only agents (e.g. build-runner) must not be dispatched
                    // from comment mentions. They are spawned by handle_push or
                    // other event handlers with the appropriate context env vars.
                    // Rejecting here prevents ghost-issue posts and wasted spawns.
                    if def.event_only {
                        info!(
                            agent = %agent_name,
                            issue = issue_number,
                            comment_id = comment_id,
                            "webhook dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                        );
                        return;
                    }

                    // Dedup: check Gitea assignment + active_agents before spawning
                    if self.should_skip_dispatch(&agent_name, issue_number).await {
                        return;
                    }

                    let chain_id = ulid::Ulid::new().to_string();
                    let depth: u32 = 0;
                    let parent_agent = String::new();

                    if let Err(e) = mention_chain::MentionChainTracker::check(
                        depth,
                        &parent_agent,
                        &agent_name,
                        max_mention_depth,
                    ) {
                        warn!(
                            agent = %agent_name,
                            chain_id = %chain_id,
                            depth,
                            error = %e,
                            "webhook mention chain check rejected dispatch"
                        );
                        if let Some(ref poster) = self.output_poster {
                            let body = format!(
                                "## Mention Dispatch Blocked\n\n\
                                Agent `{}` was not spawned: {}.\n\n\
                                _Webhook chain `{}` blocked._",
                                agent_name, e, chain_id
                            );
                            if let Err(pe) = poster.post_raw(issue_number, &body).await {
                                warn!(error = %pe, "failed to post webhook chain rejection comment");
                            }
                        }
                        return;
                    }

                    let ctx_args = mention_chain::MentionContextArgs {
                        parent_agent: parent_agent.clone(),
                        issue_number,
                        comment_body: context.clone(),
                        depth,
                        chain_id: chain_id.clone(),
                        available_agents: agent_names
                            .iter()
                            .filter(|n| *n != &agent_name)
                            .cloned()
                            .collect(),
                    };
                    let chain_ctx = mention_chain::MentionChainTracker::build_context(
                        &ctx_args,
                        max_mention_depth,
                    );

                    let mut mention_def = def.clone();
                    mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                    mention_def.gitea_issue = Some(issue_number);

                    if let Err(e) = self.spawn_agent(&mention_def).await {
                        error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                    } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                        agent.spawned_by_mention = true;
                        agent.mention_chain_id = Some(chain_id);
                        agent.mention_depth = Some(depth);
                        agent.mention_parent_agent = None;
                    }
                }
            }
            webhook::WebhookDispatch::SpawnPersona {
                persona_name,
                issue_number,
                comment_id: _,
                context,
            } => {
                if let Some((agent_name, _)) = mention::resolve_persona_mention(
                    &persona_name,
                    &agents,
                    &self.persona_registry,
                    &context,
                ) {
                    info!(
                        persona = %persona_name,
                        agent = %agent_name,
                        issue = issue_number,
                        "webhook: dispatching persona-resolved agent"
                    );

                    if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned() {
                        // Event-only agents must not be dispatched via persona-mention
                        // either. Same rationale as the SpawnAgent arm.
                        if def.event_only {
                            info!(
                                persona = %persona_name,
                                agent = %agent_name,
                                issue = issue_number,
                                "webhook dispatch rejected: persona-resolved agent is event-only (push/event-driven), not mention-dispatchable"
                            );
                            return;
                        }

                        // Dedup: check Gitea assignment + active_agents before spawning
                        if self.should_skip_dispatch(&agent_name, issue_number).await {
                            return;
                        }

                        let chain_id = ulid::Ulid::new().to_string();
                        let depth: u32 = 0;
                        let parent_agent = String::new();

                        if let Err(e) = mention_chain::MentionChainTracker::check(
                            depth,
                            &parent_agent,
                            &agent_name,
                            max_mention_depth,
                        ) {
                            warn!(
                                agent = %agent_name,
                                chain_id = %chain_id,
                                depth,
                                error = %e,
                                "webhook mention chain check rejected persona dispatch"
                            );
                            if let Some(ref poster) = self.output_poster {
                                let body = format!(
                                    "## Mention Dispatch Blocked\n\n\
                                    Agent `{}` (via persona) was not spawned: {}.\n\n\
                                    _Webhook chain `{}` blocked._",
                                    agent_name, e, chain_id
                                );
                                if let Err(pe) = poster.post_raw(issue_number, &body).await {
                                    warn!(error = %pe, "failed to post webhook chain rejection comment");
                                }
                            }
                            return;
                        }

                        let ctx_args = mention_chain::MentionContextArgs {
                            parent_agent: parent_agent.clone(),
                            issue_number,
                            comment_body: context.clone(),
                            depth,
                            chain_id: chain_id.clone(),
                            available_agents: agent_names
                                .iter()
                                .filter(|n| *n != &agent_name)
                                .cloned()
                                .collect(),
                        };
                        let chain_ctx = mention_chain::MentionChainTracker::build_context(
                            &ctx_args,
                            max_mention_depth,
                        );

                        let mut mention_def = def.clone();
                        mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                        mention_def.gitea_issue = Some(issue_number);

                        if let Err(e) = self.spawn_agent(&mention_def).await {
                            error!(agent = %agent_name, issue = issue_number, error = %e, "webhook: failed to spawn agent");
                        } else if let Some(agent) = self.active_agents.get_mut(&mention_def.name) {
                            agent.spawned_by_mention = true;
                            agent.mention_chain_id = Some(chain_id);
                            agent.mention_depth = Some(depth);
                            agent.mention_parent_agent = None;
                        }
                    }
                }
            }
            webhook::WebhookDispatch::CompoundReview {
                issue_number,
                comment_id,
            } => {
                info!(
                    issue = issue_number,
                    comment_id = comment_id,
                    "webhook: compound review triggered"
                );
                self.handle_schedule_event(ScheduleEvent::CompoundReview)
                    .await;

                // Post acknowledgment via existing output_poster
                if let Some(ref poster) = self.output_poster {
                    let ack_body = format!(
                        "## Compound Review Triggered (webhook)\n\n\
                        Manual trigger received from issue #{} comment {}.\n\
                        Running 6-agent review swarm now...",
                        issue_number, comment_id
                    );
                    if let Err(e) = poster.post_raw(issue_number, &ack_body).await {
                        warn!(error = %e, "failed to post compound review acknowledgment");
                    }
                }
            }
            webhook::WebhookDispatch::ReviewPr {
                pr_number,
                project,
                head_sha,
                author_login,
                title,
                diff_loc,
            } => {
                info!(
                    pr = pr_number,
                    project = %project,
                    head_sha = %head_sha,
                    author = %author_login,
                    diff_loc = diff_loc,
                    "webhook: enqueuing ReviewPr dispatch task"
                );
                self.dispatcher.enqueue(dispatcher::DispatchTask::ReviewPr {
                    pr_number,
                    project,
                    head_sha,
                    author_login,
                    title,
                    diff_loc,
                });
            }
            webhook::WebhookDispatch::Push {
                project,
                ref_name,
                before_sha,
                after_sha,
                pusher_login,
                files_changed,
            } => {
                info!(
                    project = %project,
                    ref_name = %ref_name,
                    after_sha = %after_sha,
                    pusher = %pusher_login,
                    files = files_changed.len(),
                    "webhook: enqueuing Push dispatch task"
                );
                self.dispatcher.enqueue(dispatcher::DispatchTask::Push {
                    project,
                    ref_name,
                    before_sha,
                    after_sha,
                    pusher_login,
                    files_changed,
                });
            }
        }
    }

    pub(crate) async fn handle_direct_dispatch(&mut self, dispatch: webhook::WebhookDispatch) {
        match dispatch {
            webhook::WebhookDispatch::SpawnAgent {
                agent_name,
                detected_project,
                context,
                synthetic_event,
                ..
            } => {
                // Use project-aware resolution for qualified agent names.
                let def = mention::resolve_mention(
                    detected_project.as_deref(),
                    dispatcher::LEGACY_PROJECT_ID,
                    &agent_name,
                    &self.config.agents,
                );

                let def = match def {
                    Some(def) => def,
                    None => {
                        // Fallback to simple name lookup for legacy compatibility.
                        warn!(agent = %agent_name, "direct dispatch: agent not found in config");
                        return;
                    }
                };

                if !def.enabled {
                    info!(agent = %agent_name, "direct dispatch rejected: agent is disabled");
                    return;
                }

                let mut direct_def = def.clone();
                if !context.is_empty() {
                    direct_def.task =
                        format!("{}\n\n[direct dispatch context]\n{}", def.task, context);
                }

                if def.event_only {
                    info!(
                        agent = %agent_name,
                        event = ?synthetic_event,
                        "direct dispatch override: spawning event_only agent locally"
                    );
                } else {
                    info!(agent = %agent_name, "direct dispatch: spawning agent");
                }
                if let Err(e) = self
                    .spawn_agent_with_event(&direct_def, synthetic_event.as_ref())
                    .await
                {
                    error!(agent = %agent_name, error = %e, "direct dispatch: failed to spawn agent");
                }
            }
            other => {
                warn!(dispatch = ?other, "direct dispatch ignored unsupported dispatch type");
            }
        }
    }
}
