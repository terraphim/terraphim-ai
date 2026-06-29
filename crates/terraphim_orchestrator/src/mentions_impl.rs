//! Mention-polling capability for `AgentOrchestrator`: scanning Gitea
//! comments for agent mentions, per-project mention polling, and resolving
//! the mention chain for nested conversations. Split from lib.rs as part of
//! the Gitea #1910 god-file decomposition; behaviour unchanged.
#![allow(clippy::too_many_lines)]

use tracing::{info, warn};

use crate::{config, dispatcher, mention, mention_chain, AgentOrchestrator, ScheduleEvent};

impl AgentOrchestrator {
    /// Uses repo-wide comments endpoint with `since` cursor. On first run
    /// (no persisted cursor), cursor is set to `now` to skip all historical
    /// mentions — preventing the mention replay storm.
    pub(crate) async fn poll_mentions(&mut self) {
        // Build the list of (project_id, gitea_cfg, mention_cfg) targets.
        //
        // - Legacy mode (no `[[projects]]`): one pass under the synthetic
        //   `__global__` id using the top-level `gitea` and `mentions`.
        // - Multi-project mode: one pass per configured project that
        //   declares a `gitea` block. Per-project `mentions` override the
        //   top-level `mentions`, which in turn falls back to
        //   `MentionConfig::default()` so operators need not repeat caps
        //   in every project.
        let targets: Vec<(String, config::GiteaOutputConfig, config::MentionConfig)> =
            if self.config.projects.is_empty() {
                match (self.config.mentions.clone(), self.config.gitea.clone()) {
                    (Some(m), Some(g)) => {
                        vec![(dispatcher::LEGACY_PROJECT_ID.to_string(), g, m)]
                    }
                    _ => {
                        tracing::debug!(
                            "mention polling skipped: legacy mode but no Gitea/mentions config"
                        );
                        return;
                    }
                }
            } else {
                let global_mentions = self.config.mentions.clone();
                self.config
                    .projects
                    .iter()
                    .filter_map(|project| {
                        if project.gitea.is_none() {
                            tracing::debug!(
                                project = project.id.as_str(),
                                "skipping mention poll: project has no gitea config"
                            );
                        }
                        let gitea = project.gitea.clone()?;
                        let mentions = project
                            .mentions
                            .clone()
                            .or_else(|| global_mentions.clone())
                            .unwrap_or_default();
                        Some((project.id.clone(), gitea, mentions))
                    })
                    .collect()
            };

        if targets.is_empty() {
            tracing::debug!("mention polling skipped: no projects with Gitea config");
            return;
        }

        for (project_id, gitea_cfg, mention_cfg) in targets {
            self.poll_mentions_for_project(&project_id, &gitea_cfg, &mention_cfg)
                .await;
        }
    }

    /// Run a single mention-poll pass for one project.
    ///
    /// Invoked by [`AgentOrchestrator::poll_mentions`] for each configured
    /// project (or once for legacy single-project mode under
    /// `__global__`). Loads/persists the project's cursor, honours the
    /// project's `MentionConfig`, and threads `project_id` onto every
    /// dispatched mention.
    async fn poll_mentions_for_project(
        &mut self,
        project_id: &str,
        gitea_cfg: &config::GiteaOutputConfig,
        mention_cfg: &config::MentionConfig,
    ) {
        // Respect poll_modulo to reduce API traffic.
        if self.tick_count % mention_cfg.poll_modulo != 0 {
            return;
        }

        // Count currently active mention-spawned agents for this project.
        //
        // We filter by the agent definition's project field so one noisy
        // project cannot exhaust the fleet-wide mention budget for others.
        // In legacy mode (project_id == "__global__") every agent
        // contributes because project binding isn't meaningful there.
        let active_mention_agents = if project_id == dispatcher::LEGACY_PROJECT_ID {
            self.active_agents
                .values()
                .filter(|a| a.spawned_by_mention)
                .count() as u32
        } else {
            self.active_agents
                .values()
                .filter(|a| {
                    a.spawned_by_mention && a.definition.project.as_deref() == Some(project_id)
                })
                .count() as u32
        };
        if active_mention_agents >= mention_cfg.max_concurrent_mention_agents {
            tracing::debug!(
                project = project_id,
                active = active_mention_agents,
                max = mention_cfg.max_concurrent_mention_agents,
                "mention agents at capacity, skipping poll"
            );
            return;
        }

        // Lazy-load the project's cursor.
        let mut cursor = match self.mention_cursors.remove(project_id) {
            Some(c) => c,
            None => mention::MentionCursor::load_or_now(project_id).await,
        };
        cursor.dispatches_this_tick = 0;

        // Create Gitea tracker for repo-wide comment polling
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
                tracing::warn!(
                    project = project_id,
                    error = %e,
                    "failed to create GiteaTracker for mention polling"
                );
                self.mention_cursors.insert(project_id.to_string(), cursor);
                return;
            }
        };

        // Single API call: all comments since cursor
        let comments = match tracker
            .fetch_repo_comments(Some(&cursor.last_seen_at), Some(50))
            .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    project = project_id,
                    error = %e,
                    "failed to fetch repo comments for mention polling"
                );
                self.mention_cursors.insert(project_id.to_string(), cursor);
                return;
            }
        };

        if comments.is_empty() {
            cursor.save(project_id).await;
            self.mention_cursors.insert(project_id.to_string(), cursor);
            return;
        }

        let agents = self.config.agents.clone();
        let persona_registry = self.persona_registry.clone();
        let max_dispatches = mention_cfg.max_dispatches_per_tick;

        // Build ADF command parser with known agents and personas
        let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
        let persona_names: Vec<String> = persona_registry
            .persona_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        let command_parser =
            crate::adf_commands::AdfCommandParser::new(&agent_names, &persona_names);

        let max_mention_depth = mention_cfg.max_mention_depth;

        for comment in &comments {
            if cursor.dispatches_this_tick >= max_dispatches {
                tracing::debug!(
                    dispatched = cursor.dispatches_this_tick,
                    max = max_dispatches,
                    "max dispatches per tick reached"
                );
                break;
            }

            // Skip already-processed comments (persisted dedup across restarts)
            if cursor.is_processed(comment.id) {
                tracing::debug!(
                    comment_id = comment.id,
                    issue = comment.issue_number,
                    "skipping already-processed comment"
                );
                cursor.advance_to(&comment.created_at);
                continue;
            }

            // Parse ADF commands using terraphim-automata Aho-Corasick
            let commands =
                command_parser.parse_commands(&comment.body, comment.issue_number, comment.id);

            // Handle qualified `@adf:project/name` mentions that AdfCommandParser cannot
            // see (its patterns are `@adf:{name}`; a `project/` prefix is not a substring).
            for token in mention::parse_mention_tokens(&comment.body) {
                if cursor.dispatches_this_tick >= max_dispatches {
                    break;
                }
                let proj = match token.project.as_deref() {
                    Some(p) => p,
                    None => continue, // unqualified mentions are handled by parse_commands below
                };
                match mention::resolve_mention(Some(proj), project_id, &token.agent, &agents) {
                    Some(def) => {
                        info!(
                            agent = %token.agent,
                            project = proj,
                            issue = comment.issue_number,
                            comment_id = comment.id,
                            "dispatching qualified mention-driven agent"
                        );
                        // Event-only agents (e.g. build-runner) must not be dispatched
                        // from comment mentions. Reject before any spawn-related work.
                        if def.event_only {
                            info!(
                                agent = %token.agent,
                                issue = comment.issue_number,
                                comment_id = comment.id,
                                "poll mention dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                            );
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }
                        if self
                            .should_skip_dispatch(&token.agent, comment.issue_number)
                            .await
                        {
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }

                        let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                            &comment.user.login,
                            &agent_names,
                            max_mention_depth,
                        );

                        if let Err(e) = mention_chain::MentionChainTracker::check(
                            depth,
                            &parent_agent,
                            &token.agent,
                            max_mention_depth,
                        ) {
                            warn!(
                                agent = %token.agent,
                                chain_id = %chain_id,
                                depth,
                                error = %e,
                                "mention chain check rejected dispatch"
                            );
                            if let Some(ref poster) = self.output_poster {
                                let body = format!(
                                    "## Mention Dispatch Blocked\n\n\
                                    Agent `{}` was not spawned: {}.\n\n\
                                    _Chain `{}` at depth {} exceeds the configured limit._",
                                    token.agent, e, chain_id, depth
                                );
                                if let Err(pe) = poster
                                    .post_raw_for_project(project_id, comment.issue_number, &body)
                                    .await
                                {
                                    warn!(error = %pe, "failed to post chain rejection comment");
                                }
                            }
                            cursor.dispatches_this_tick += 1;
                            continue;
                        }

                        let ctx_args = mention_chain::MentionContextArgs {
                            parent_agent: parent_agent.clone(),
                            issue_number: comment.issue_number,
                            comment_body: comment.body.clone(),
                            depth,
                            chain_id: chain_id.clone(),
                            available_agents: agent_names
                                .iter()
                                .filter(|n| *n != &token.agent)
                                .cloned()
                                .collect(),
                        };
                        let chain_ctx = mention_chain::MentionChainTracker::build_context(
                            &ctx_args,
                            max_mention_depth,
                        );

                        let mut mention_def = def.clone();
                        mention_def.task = format!("{}\n\n{}", def.task, chain_ctx);
                        mention_def.gitea_issue = Some(comment.issue_number);
                        if let Err(e) = self.spawn_agent(&mention_def).await {
                            tracing::error!(
                                agent = %token.agent,
                                project = proj,
                                issue = comment.issue_number,
                                error = %e,
                                "failed to spawn agent for qualified mention"
                            );
                        } else if let Some(active) = self.active_agents.get_mut(&mention_def.name) {
                            active.spawned_by_mention = true;
                            active.mention_chain_id = Some(chain_id);
                            active.mention_depth = Some(depth);
                            active.mention_parent_agent = if parent_agent.is_empty() {
                                None
                            } else {
                                Some(parent_agent)
                            };
                        }
                        cursor.dispatches_this_tick += 1;
                    }
                    None => {
                        tracing::warn!(
                            mention = format!("@adf:{}/{}", proj, token.agent),
                            project = project_id,
                            issue = comment.issue_number,
                            "qualified mention matched no agent"
                        );
                    }
                }
            }

            for cmd in commands {
                if cursor.dispatches_this_tick >= max_dispatches {
                    break;
                }

                match cmd {
                    crate::adf_commands::AdfCommand::CompoundReview {
                        issue_number,
                        comment_id,
                    } => {
                        info!(
                            issue = issue_number,
                            comment_id = comment_id,
                            "compound review triggered via @adf:compound-review mention"
                        );

                        // Trigger compound review
                        self.handle_schedule_event(ScheduleEvent::CompoundReview)
                            .await;

                        // Post acknowledgment
                        if let Some(ref poster) = self.output_poster {
                            let ack_body = format!(
                                "## 🔍 Compound Review Triggered\n\n\
                                Manual trigger received from issue #{} comment {}.\n\
                                Running 6-agent review swarm now...\n\n\
                                _Results will be posted to issue #{} when complete._",
                                issue_number,
                                comment_id,
                                self.config.compound_review.gitea_issue.unwrap_or(108)
                            );
                            if let Err(e) = poster.post_raw(issue_number, &ack_body).await {
                                warn!(error = %e, "failed to post compound review acknowledgment");
                            }
                        }

                        cursor.dispatches_this_tick += 1;
                    }
                    crate::adf_commands::AdfCommand::SpawnAgent {
                        agent_name,
                        issue_number,
                        comment_id,
                        context,
                    } => {
                        info!(
                            agent = %agent_name,
                            issue = issue_number,
                            comment_id = comment_id,
                            "dispatching mention-driven agent via terraphim-automata parser"
                        );

                        if let Some(def) =
                            mention::resolve_mention(None, project_id, &agent_name, &agents)
                        {
                            // Event-only agents (e.g. build-runner) must not be dispatched
                            // from comment mentions. Reject before any spawn-related work.
                            if def.event_only {
                                info!(
                                    agent = %agent_name,
                                    issue = issue_number,
                                    comment_id = comment_id,
                                    "poll mention dispatch rejected: agent is event-only (push/event-driven), not mention-dispatchable"
                                );
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }
                            if self.should_skip_dispatch(&agent_name, issue_number).await {
                                cursor.dispatches_this_tick += 1;
                                continue;
                            }

                            let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                                &comment.user.login,
                                &agent_names,
                                max_mention_depth,
                            );

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
                                    "mention chain check rejected dispatch"
                                );
                                if let Some(ref poster) = self.output_poster {
                                    let body = format!(
                                        "## Mention Dispatch Blocked\n\n\
                                        Agent `{}` was not spawned: {}.\n\n\
                                        _Chain `{}` at depth {} exceeds the configured limit._",
                                        agent_name, e, chain_id, depth
                                    );
                                    if let Err(pe) = poster
                                        .post_raw_for_project(project_id, issue_number, &body)
                                        .await
                                    {
                                        warn!(error = %pe, "failed to post chain rejection comment");
                                    }
                                }
                                cursor.dispatches_this_tick += 1;
                                continue;
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
                                tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                            } else if let Some(agent) =
                                self.active_agents.get_mut(&mention_def.name)
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

                            cursor.dispatches_this_tick += 1;
                        }
                    }
                    crate::adf_commands::AdfCommand::SpawnPersona {
                        persona_name,
                        issue_number,
                        comment_id: _,
                        context,
                    } => {
                        // Resolve persona to agent
                        if let Some((agent_name, _)) = mention::resolve_persona_mention(
                            &persona_name,
                            &agents,
                            &persona_registry,
                            &context,
                        ) {
                            info!(
                                persona = %persona_name,
                                agent = %agent_name,
                                issue = issue_number,
                                "dispatching persona-resolved agent via terraphim-automata parser"
                            );

                            if let Some(def) = agents.iter().find(|a| a.name == agent_name).cloned()
                            {
                                // Event-only agents (e.g. build-runner) must not be dispatched
                                // from persona mentions. Reject before any spawn-related work.
                                if def.event_only {
                                    info!(
                                        persona = %persona_name,
                                        agent = %agent_name,
                                        issue = issue_number,
                                        "poll mention dispatch rejected: persona-resolved agent is event-only (push/event-driven), not mention-dispatchable"
                                    );
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }
                                // Dedup: check Gitea assignment + active_agents before spawning
                                if self.should_skip_dispatch(&agent_name, issue_number).await {
                                    cursor.dispatches_this_tick += 1;
                                    continue;
                                }

                                let (chain_id, depth, parent_agent) = self.resolve_mention_chain(
                                    &comment.user.login,
                                    &agent_names,
                                    max_mention_depth,
                                );

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
                                        "mention chain check rejected persona dispatch"
                                    );
                                    if let Some(ref poster) = self.output_poster {
                                        let body = format!(
                                            "## Mention Dispatch Blocked\n\n\
                                            Agent `{}` (via persona) was not spawned: {}.\n\n\
                                            _Chain `{}` at depth {} exceeds the configured limit._",
                                            agent_name, e, chain_id, depth
                                        );
                                        if let Err(pe) = poster
                                            .post_raw_for_project(project_id, issue_number, &body)
                                            .await
                                        {
                                            warn!(error = %pe, "failed to post chain rejection comment");
                                        }
                                    }
                                    cursor.dispatches_this_tick += 1;
                                    continue;
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
                                    tracing::error!(agent = %agent_name, issue = issue_number, error = %e, "failed to spawn agent");
                                } else if let Some(agent) =
                                    self.active_agents.get_mut(&mention_def.name)
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

                                cursor.dispatches_this_tick += 1;
                            }
                        }
                    }
                    crate::adf_commands::AdfCommand::Unknown { raw } => {
                        warn!(raw = %raw, "unknown ADF command");
                    }
                }
            }

            // Mark comment as processed and advance cursor
            cursor.mark_processed(comment.id);
            cursor.advance_to(&comment.created_at);
        }

        // Persist cursor for next poll / restart
        cursor.save(project_id).await;
        self.mention_cursors.insert(project_id.to_string(), cursor);
    }

    /// Check if an agent is already assigned to this issue and currently active.
    ///
    /// Returns `true` if dispatch should be **skipped** (duplicate), `false` if
    /// dispatch should proceed. When dispatch proceeds, the issue is assigned
    /// to the agent as a side-effect.
    ///
    /// The dedup logic:
    /// - If the issue is already assigned to the agent AND the agent is currently
    ///   in `active_agents` -> skip (duplicate dispatch).
    /// - If assigned but agent is NOT active -> allow (agent crashed, re-dispatch).
    /// - If not assigned -> allow (first dispatch) and assign.
    pub(crate) async fn should_skip_dispatch(&self, agent_name: &str, issue_number: u64) -> bool {
        if issue_number == 0 {
            return false;
        }

        // Fast local check: if agent is already running, skip immediately.
        // This prevents races where the Gitea API returns stale assignee data
        // because a concurrent dispatch path just assigned the issue milliseconds ago.
        if self.active_agents.contains_key(agent_name) {
            warn!(
                agent = %agent_name,
                issue = issue_number,
                "skipping dispatch: agent already active (local guard)"
            );
            return true;
        }

        let Some(ref poster) = self.output_poster else {
            return false;
        };
        // Resolve the agent's owning project so the tracker uses the
        // correct owner/repo (multi-project) or falls back to legacy.
        let project = self
            .config
            .agents
            .iter()
            .find(|a| a.name == agent_name)
            .and_then(|a| a.project.clone())
            .unwrap_or_else(|| crate::dispatcher::LEGACY_PROJECT_ID.to_string());
        let Some(tracker) = poster.tracker_for(&project, agent_name) else {
            warn!(
                agent = %agent_name,
                project = %project,
                "no Gitea tracker for project; treating dispatch as not-duplicate"
            );
            return false;
        };

        // Remote check: if agent is assigned in Gitea but not active (crash recovery)
        match tracker.fetch_issue_assignees(issue_number).await {
            Ok(assignees) => {
                if assignees.iter().any(|a| a == agent_name) {
                    // Already assigned -- check if agent is actively running
                    if self.active_agents.contains_key(agent_name) {
                        warn!(
                            agent = %agent_name,
                            issue = issue_number,
                            "skipping duplicate dispatch: agent already assigned and active"
                        );
                        return true;
                    }
                    // Assigned but not active (crashed or completed) -- allow re-dispatch
                    info!(
                        agent = %agent_name,
                        issue = issue_number,
                        "agent assigned but not active, allowing re-dispatch"
                    );
                }
            }
            Err(e) => {
                // Fail open: if we can't check assignees, allow dispatch
                warn!(
                    agent = %agent_name,
                    issue = issue_number,
                    error = %e,
                    "failed to fetch assignees, allowing dispatch (fail-open)"
                );
            }
        }

        // Assign the issue to the agent
        if let Err(e) = tracker.assign_issue(issue_number, &[agent_name]).await {
            warn!(
                agent = %agent_name,
                issue = issue_number,
                error = %e,
                "failed to assign issue to agent"
            );
        } else {
            info!(
                agent = %agent_name,
                issue = issue_number,
                "assigned issue to agent"
            );
        }
        false
    }

    /// Resolve mention chain metadata from the comment author.
    ///
    /// If the comment was posted by a known agent, this is a nested mention:
    /// inherit the chain_id from the agent's current run and increment depth.
    /// If posted by a human, start a fresh chain with depth 0.
    fn resolve_mention_chain(
        &self,
        comment_author: &str,
        agent_names: &[String],
        _max_depth: u32,
    ) -> (String, u32, String) {
        if agent_names.iter().any(|n| n == comment_author) {
            if let Some(active) = self.active_agents.get(comment_author) {
                let parent_chain_id = active
                    .mention_chain_id
                    .clone()
                    .unwrap_or_else(|| ulid::Ulid::new().to_string());
                let parent_depth = active.mention_depth.unwrap_or(0).saturating_add(1);
                (parent_chain_id, parent_depth, comment_author.to_string())
            } else {
                let chain_id = ulid::Ulid::new().to_string();
                (chain_id, 1, comment_author.to_string())
            }
        } else {
            let chain_id = ulid::Ulid::new().to_string();
            (chain_id, 0, String::new())
        }
    }
}
