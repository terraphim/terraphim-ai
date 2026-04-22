//! Mention detection and resolution for @adf:name comments.
//!
//! Supports two addressing modes:
//! - Agent name: `@adf:security-sentinel` (exact match on agent name)
//! - Persona name: `@adf:vigil` (resolved via PersonaRegistry)

use crate::config::AgentDefinition;
use crate::persona::PersonaRegistry;
use chrono::{DateTime, SecondsFormat, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;
use terraphim_tracker::IssueComment;

pub(crate) use crate::dispatcher::LEGACY_PROJECT_ID;

/// Regex for `@adf:[project/]name` mentions.
///
/// Captures an optional lowercase project prefix and a mandatory agent name.
/// Unqualified mentions (`@adf:developer`) keep the `project` capture as `None`.
/// Qualified mentions (`@adf:odilo/developer`) populate both.
static MENTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"@adf:(?:(?P<project>[a-z][a-z0-9-]{1,39})/)?(?P<agent>[a-z][a-z0-9-]{1,39})\b")
        .unwrap()
});

/// How a mention was resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum MentionResolution {
    AgentName,
    PersonaName { persona: String },
}

/// Parsed tokens of a single `@adf:[project/]name` mention.
#[derive(Debug, Clone, PartialEq)]
pub struct MentionTokens {
    pub project: Option<String>,
    pub agent: String,
}

/// Parse all `@adf:[project/]name` mentions in `text`, returning their
/// project prefix (if any) and bare agent name in order of appearance.
///
/// Unlike [`parse_mentions`] this is a pure syntactic pass — no lookup
/// against known agents or personas. Useful for tests and for the
/// multi-project poller which wants the raw tokens before resolution.
pub fn parse_mention_tokens(text: &str) -> Vec<MentionTokens> {
    MENTION_RE
        .captures_iter(text)
        .map(|caps| MentionTokens {
            project: caps.name("project").map(|m| m.as_str().to_string()),
            agent: caps
                .name("agent")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
        })
        .collect()
}

/// A detected and resolved mention.
#[derive(Debug, Clone)]
pub struct DetectedMention {
    pub issue_number: u64,
    pub comment_id: u64,
    pub raw_mention: String,
    pub agent_name: String,
    pub resolution: MentionResolution,
    pub comment_body: String,
    pub mentioner: String,
    pub timestamp: String,
    /// Project id the mention was detected in.
    ///
    /// Set to [`LEGACY_PROJECT_ID`] for legacy single-project mode or to the
    /// id of the project whose repo the enclosing comment was polled from.
    /// A qualified `@adf:<proj>/<name>` mention does not override this -- the
    /// detected project lives in [`MentionTokens`] from [`parse_mention_tokens`]
    /// and is consumed by [`resolve_mention`].
    pub project_id: String,
}

// ---------------------------------------------------------------------------
// MentionCursor: Persistent cursor for repo-wide comment polling
// ---------------------------------------------------------------------------

/// Persistent cursor for mention polling.
///
/// Stored via `terraphim_persistence` as JSON at key
/// `adf/mention_cursor/<project_id>` (per-project) or
/// `adf/mention_cursor/__global__` for legacy single-project mode.
/// The cursor tracks the `created_at` timestamp of the last processed comment,
/// ensuring we never replay historical mentions on restart.
///
/// # Startup Guard
///
/// If no cursor exists (first run), we create one set to `now` to skip
/// all historical mentions. This prevents the "mention replay storm" bug.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionCursor {
    /// ISO 8601 timestamp of the last processed comment.
    pub last_seen_at: String,
    /// Counter of dispatches in current tick (reset each poll cycle).
    #[serde(skip)]
    pub dispatches_this_tick: u32,
    /// Set of comment IDs already dispatched (persisted to prevent re-dispatch on restart).
    #[serde(default)]
    pub processed_comment_ids: std::collections::HashSet<u64>,
}

impl MentionCursor {
    /// Create a cursor set to "now" (skip all historical mentions).
    pub fn now() -> Self {
        Self {
            last_seen_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            dispatches_this_tick: 0,
            processed_comment_ids: std::collections::HashSet::new(),
        }
    }

    /// Check if a comment has already been dispatched.
    pub fn is_processed(&self, comment_id: u64) -> bool {
        self.processed_comment_ids.contains(&comment_id)
    }

    /// Mark a comment as processed.
    pub fn mark_processed(&mut self, comment_id: u64) {
        self.processed_comment_ids.insert(comment_id);
        // Cap the set to prevent unbounded growth (keep last 10000 entries).
        if self.processed_comment_ids.len() > 10_000 {
            // Remove oldest entries by keeping a fresh set (no ordering in HashSet,
            // so just drain half). In practice this set grows slowly.
            let to_remove: Vec<u64> = self
                .processed_comment_ids
                .iter()
                .take(5_000)
                .copied()
                .collect();
            for id in to_remove {
                self.processed_comment_ids.remove(&id);
            }
        }
    }

    /// Load from persistence or create "now" cursor.
    ///
    /// On first run (no persisted cursor), returns a cursor set to the
    /// current time, effectively skipping all historical mentions.
    /// Get the SQLite operator for persistent storage.
    async fn sqlite_op() -> Option<opendal::Operator> {
        let storage = terraphim_persistence::DeviceStorage::instance()
            .await
            .ok()?;
        let (op, _) = storage.ops.get("sqlite")?;
        Some(op.clone())
    }

    /// Persistence key for a project's cursor.
    ///
    /// Multi-project installations use one cursor per project id; legacy
    /// single-project installations pass [`LEGACY_PROJECT_ID`].
    fn cursor_key(project_id: &str) -> String {
        format!("adf/mention_cursor/{}", project_id)
    }

    pub async fn load_or_now(project_id: &str) -> Self {
        let key = Self::cursor_key(project_id);

        if let Some(op) = Self::sqlite_op().await {
            if let Ok(bs) = op.read(&key).await {
                if let Ok(cursor) = serde_json::from_slice::<Self>(&bs.to_vec()) {
                    tracing::info!(
                        project = project_id,
                        last_seen_at = %cursor.last_seen_at,
                        "loaded MentionCursor from persistence"
                    );
                    return cursor;
                }
                tracing::warn!(
                    project = project_id,
                    "failed to deserialize MentionCursor, starting fresh"
                );
            } else {
                tracing::info!(
                    project = project_id,
                    "no persisted MentionCursor found, starting fresh"
                );
            }
        } else {
            tracing::warn!(
                project = project_id,
                "DeviceStorage sqlite not available, using in-memory cursor"
            );
        }

        Self::now()
    }

    /// Save to persistence under the given project's cursor key.
    pub async fn save(&self, project_id: &str) {
        let key = Self::cursor_key(project_id);

        if let Some(op) = Self::sqlite_op().await {
            if let Ok(json) = serde_json::to_string(self) {
                if let Err(e) = op.write(&key, json).await {
                    tracing::warn!(project = project_id, ?e, "failed to save MentionCursor");
                } else {
                    tracing::debug!(
                        project = project_id,
                        last_seen_at = %self.last_seen_at,
                        "saved MentionCursor"
                    );
                }
            } else {
                tracing::warn!(project = project_id, "failed to serialize MentionCursor");
            }
        } else {
            tracing::warn!(
                project = project_id,
                "DeviceStorage sqlite not available, cursor not persisted"
            );
        }
    }

    /// Advance cursor past a comment's timestamp.
    ///
    /// Converts any RFC3339-ish timestamp to UTC Z format for Gitea compat.
    pub fn advance_to(&mut self, timestamp: &str) {
        // Parse any RFC3339 timestamp and convert to UTC Z format
        if let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp) {
            let utc = parsed.with_timezone(&Utc);
            let utc_str = utc.to_rfc3339_opts(SecondsFormat::Secs, true);

            // Only advance if newer than current cursor
            if let Ok(current) = DateTime::parse_from_rfc3339(&self.last_seen_at) {
                if utc > current.with_timezone(&Utc) {
                    self.last_seen_at = utc_str;
                }
            } else {
                self.last_seen_at = utc_str;
            }
        }
    }
}

impl Default for MentionCursor {
    fn default() -> Self {
        Self::now()
    }
}

/// One-shot migration of the legacy top-level `adf/mention_cursor` key
/// to per-project keys `adf/mention_cursor/<project_id>`.
///
/// Behaviour:
///
/// - If the legacy key does not exist (or storage is unavailable), the call
///   is a no-op.
/// - If it does exist, the cursor is copied to every project id in
///   `projects` **and** to [`LEGACY_PROJECT_ID`]. A target key is only
///   written when it does not already exist, so repeated invocations
///   never clobber a cursor the poller has already advanced.
/// - After copying, the legacy key is deleted so subsequent restarts
///   skip this path entirely.
///
/// `projects` is the current orchestrator config's `projects` list.
/// An empty list means legacy single-project mode; the legacy cursor is
/// then simply renamed to the `__global__` key.
///
/// Logged but non-fatal on any storage error -- the poller will create
/// fresh per-project cursors on first use if migration fails.
pub async fn migrate_legacy_mention_cursor(projects: &[crate::config::Project]) {
    let legacy_key = "adf/mention_cursor";

    let Some(op) = MentionCursor::sqlite_op().await else {
        tracing::debug!("mention cursor migration skipped: no sqlite operator");
        return;
    };

    let legacy_bytes = match op.read(legacy_key).await {
        Ok(bs) => bs,
        Err(_) => {
            tracing::debug!("no legacy mention cursor at `adf/mention_cursor`, nothing to migrate");
            return;
        }
    };

    let Ok(cursor) = serde_json::from_slice::<MentionCursor>(&legacy_bytes.to_vec()) else {
        tracing::warn!(
            "legacy mention cursor is unparsable; deleting it so per-project keys start clean"
        );
        let _ = op.delete(legacy_key).await;
        return;
    };

    let Ok(json) = serde_json::to_string(&cursor) else {
        tracing::warn!("failed to serialize legacy mention cursor during migration");
        return;
    };

    let mut targets: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
    targets.push(LEGACY_PROJECT_ID.to_string());

    let mut written = 0usize;
    for pid in &targets {
        let key = MentionCursor::cursor_key(pid);
        match op.stat(&key).await {
            Ok(_) => {
                tracing::debug!(
                    project = pid.as_str(),
                    "skipping legacy-cursor migration: per-project cursor already present"
                );
            }
            Err(_) => {
                if let Err(e) = op.write(&key, json.clone()).await {
                    tracing::warn!(
                        project = pid.as_str(),
                        ?e,
                        "failed to write migrated MentionCursor"
                    );
                } else {
                    written += 1;
                }
            }
        }
    }

    match op.delete(legacy_key).await {
        Ok(()) => tracing::info!(
            migrated_to = written,
            last_seen_at = %cursor.last_seen_at,
            "migrated legacy mention cursor to per-project keys"
        ),
        Err(e) => tracing::warn!(?e, "failed to delete legacy mention cursor after migration"),
    }
}

/// Resolve a raw mention to an agent name via persona lookup.
///
/// 1. If raw matches an agent name exactly -> AgentName
/// 2. If raw matches a persona name -> PersonaName (pick best-fit agent)
/// 3. No match -> None
///
/// This is the legacy single-project resolver used by the compound-review
/// persona dispatch path. Multi-project resolution lives in [`resolve_mention`].
pub fn resolve_persona_mention(
    raw: &str,
    agents: &[AgentDefinition],
    personas: &PersonaRegistry,
    context: &str,
) -> Option<(String, MentionResolution)> {
    // 1. Direct agent name match
    if let Some(agent) = agents.iter().find(|a| a.name == raw) {
        return Some((agent.name.clone(), MentionResolution::AgentName));
    }

    // 2. Persona name match
    if personas.get(raw).is_some() {
        let matching_agents: Vec<&AgentDefinition> = agents
            .iter()
            .filter(|a| {
                a.persona
                    .as_ref()
                    .map(|p| p.eq_ignore_ascii_case(raw))
                    .unwrap_or(false)
            })
            .collect();

        match matching_agents.len() {
            0 => return None,
            1 => {
                return Some((
                    matching_agents[0].name.clone(),
                    MentionResolution::PersonaName {
                        persona: raw.to_string(),
                    },
                ));
            }
            _ => {
                // Multiple agents share this persona. Pick by keyword overlap with context.
                let context_lower = context.to_lowercase();
                let mut best_agent = &matching_agents[0];
                let mut best_score = 0usize;

                for agent in &matching_agents {
                    let score = agent
                        .capabilities
                        .iter()
                        .filter(|cap| context_lower.contains(&cap.to_lowercase()))
                        .count();
                    if score > best_score || (score == best_score && agent.name < best_agent.name) {
                        best_score = score;
                        best_agent = agent;
                    }
                }

                return Some((
                    best_agent.name.clone(),
                    MentionResolution::PersonaName {
                        persona: raw.to_string(),
                    },
                ));
            }
        }
    }

    None
}

/// Resolve a `@adf:[project/]name` mention to a concrete [`AgentDefinition`]
/// using the poller's project hint and optional qualified prefix.
///
/// Resolution rules (in order):
///
/// 1. If `detected_project` is `Some("p")` — an explicit `@adf:p/name` —
///    find the unique agent whose `name == agent_name` **and** `project == Some("p")`.
///    Mismatch between `detected_project` and `hinted_project` is permitted:
///    a user in repo A may address an agent defined against project B, as long
///    as that agent exists. If zero or more than one agent matches, return `None`.
///
/// 2. If `detected_project` is `None` — an unqualified `@adf:name`:
///
///    - If `hinted_project == `[`LEGACY_PROJECT_ID`] (single-project legacy mode),
///      match on `name == agent_name` only, ignoring the agent's `project` field.
///
///    - Otherwise, prefer an agent defined for `hinted_project`
///      (`name == agent_name` and `project == Some(hinted_project)`).
///
///    - Fallback: accept a project-less agent (`project == None`) with a matching name.
///      Cross-project defaulting (matching an agent whose project differs from
///      the hint) is never allowed -- that would let an unqualified mention
///      silently spawn an agent from another repo.
///
/// Returns `None` if no agent satisfies these rules or if multiple agents do.
///
/// The caller is expected to have obtained `detected_project` from
/// [`parse_mention_tokens`] or [`MENTION_RE`] and `hinted_project` from the
/// poller's current iteration over `config.projects` (or [`LEGACY_PROJECT_ID`]
/// for the legacy top-level gitea path).
pub fn resolve_mention(
    detected_project: Option<&str>,
    hinted_project: &str,
    agent_name: &str,
    agents: &[AgentDefinition],
) -> Option<AgentDefinition> {
    if let Some(proj) = detected_project {
        // Qualified form: `@adf:<proj>/<name>` — exact (name, project) match.
        let matches: Vec<&AgentDefinition> = agents
            .iter()
            .filter(|a| a.name == agent_name && a.project.as_deref() == Some(proj))
            .collect();
        return match matches.len() {
            1 => Some(matches[0].clone()),
            _ => None,
        };
    }

    // Unqualified form: `@adf:<name>`.
    if hinted_project == LEGACY_PROJECT_ID {
        // Legacy single-project mode: ignore the agent's project field.
        let matches: Vec<&AgentDefinition> =
            agents.iter().filter(|a| a.name == agent_name).collect();
        return match matches.len() {
            1 => Some(matches[0].clone()),
            _ => None,
        };
    }

    // Multi-project mode: prefer an agent bound to the hinted project.
    let hinted: Vec<&AgentDefinition> = agents
        .iter()
        .filter(|a| a.name == agent_name && a.project.as_deref() == Some(hinted_project))
        .collect();
    if hinted.len() == 1 {
        return Some(hinted[0].clone());
    }
    if hinted.len() > 1 {
        return None;
    }

    // Fallback: project-less agent with matching name.
    let unbound: Vec<&AgentDefinition> = agents
        .iter()
        .filter(|a| a.name == agent_name && a.project.is_none())
        .collect();
    match unbound.len() {
        1 => Some(unbound[0].clone()),
        _ => None,
    }
}

/// Parse and resolve all @adf:name mentions from a comment.
///
/// `hinted_project` is the id of the project whose repo the comment was
/// polled from, or [`LEGACY_PROJECT_ID`] in single-project mode. It is
/// stamped on every produced [`DetectedMention`] for downstream dispatch.
pub fn parse_mentions(
    comment: &IssueComment,
    issue_number: u64,
    agents: &[AgentDefinition],
    personas: &PersonaRegistry,
    hinted_project: &str,
) -> Vec<DetectedMention> {
    let mut mentions = Vec::new();

    for cap in MENTION_RE.captures_iter(&comment.body) {
        let raw_agent = cap.name("agent").map(|m| m.as_str()).unwrap_or_default();
        if let Some((agent_name, resolution)) =
            resolve_persona_mention(raw_agent, agents, personas, &comment.body)
        {
            mentions.push(DetectedMention {
                issue_number,
                comment_id: comment.id,
                raw_mention: raw_agent.to_string(),
                agent_name,
                resolution,
                comment_body: comment.body.clone(),
                mentioner: comment.user.login.clone(),
                timestamp: comment.created_at.clone(),
                project_id: hinted_project.to_string(),
            });
        } else {
            tracing::warn!(
                raw_mention = raw_agent,
                issue = issue_number,
                "unresolved @adf mention"
            );
        }
    }

    mentions
}

/// Tracks dispatch counts per issue for flood protection.
///
/// With cursor-based polling, we no longer need the `processed` HashSet —
/// the cursor ensures we never see the same comment twice. This struct
/// now only tracks per-issue dispatch counts to prevent one noisy issue
/// from spawning too many agents.
pub struct MentionTracker {
    max_dispatches_per_issue: u32,
    dispatches_per_issue: HashMap<u64, u32>,
}

impl MentionTracker {
    pub fn new(max_dispatches_per_issue: u32) -> Self {
        Self {
            max_dispatches_per_issue,
            dispatches_per_issue: HashMap::new(),
        }
    }

    /// Check if an issue has exceeded its dispatch limit.
    pub fn limit_exceeded(&self, issue_number: u64) -> bool {
        self.dispatches_per_issue
            .get(&issue_number)
            .map(|&d| d >= self.max_dispatches_per_issue)
            .unwrap_or(false)
    }

    /// Record a dispatch for an issue.
    pub fn record_dispatch(&mut self, issue_number: u64) {
        *self.dispatches_per_issue.entry(issue_number).or_insert(0) += 1;
    }

    /// Reset dispatch counts (call at start of new poll cycle if desired).
    pub fn reset(&mut self) {
        self.dispatches_per_issue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentLayer;
    use terraphim_types::persona::PersonaDefinition;

    fn test_agent_default() -> AgentDefinition {
        AgentDefinition {
            name: String::new(),
            layer: AgentLayer::Growth,
            cli_tool: "echo".to_string(),
            task: "test task".to_string(),
            schedule: None,
            model: None,
            capabilities: vec![],
            max_memory_bytes: None,
            budget_monthly_cents: None,
            provider: None,
            persona: None,
            terraphim_role: None,
            skill_chain: vec![],
            sfia_skills: vec![],
            fallback_provider: None,
            fallback_model: None,
            grace_period_secs: None,
            max_cpu_seconds: None,
            pre_check: None,
            gitea_issue: None,
            project: None,
        }
    }

    fn test_agents() -> Vec<AgentDefinition> {
        vec![
            AgentDefinition {
                name: "security-sentinel".into(),
                persona: Some("Vigil".into()),
                capabilities: vec!["security".into(), "audit".into(), "vulnerability".into()],
                ..test_agent_default()
            },
            AgentDefinition {
                name: "compliance-watchdog".into(),
                persona: Some("Vigil".into()),
                capabilities: vec!["compliance".into(), "license".into(), "gdpr".into()],
                ..test_agent_default()
            },
            AgentDefinition {
                name: "spec-validator".into(),
                persona: Some("Carthos".into()),
                capabilities: vec![
                    "specification".into(),
                    "architecture".into(),
                    "domain".into(),
                ],
                ..test_agent_default()
            },
            AgentDefinition {
                name: "product-development".into(),
                persona: Some("Planner".into()),
                capabilities: vec![
                    "roadmap".into(),
                    "prioritisation".into(),
                    "backlog-analysis".into(),
                ],
                ..test_agent_default()
            },
        ]
    }

    fn test_persona_definition(name: &str) -> PersonaDefinition {
        PersonaDefinition {
            agent_name: name.to_string(),
            role_name: format!("{} Role", name),
            name_origin: format!("Test origin for {}", name),
            vibe: "Test vibe".to_string(),
            symbol: "T".to_string(),
            core_characteristics: vec![],
            speech_style: "Test style".to_string(),
            terraphim_nature: "Test nature".to_string(),
            sfia_title: format!("{} Engineer", name),
            primary_level: 4,
            guiding_phrase: "Test phrase".to_string(),
            level_essence: "Test essence".to_string(),
            sfia_skills: vec![],
        }
    }

    fn test_personas() -> PersonaRegistry {
        let mut registry = PersonaRegistry::new();
        registry.insert(test_persona_definition("Vigil"));
        registry.insert(test_persona_definition("Carthos"));
        registry.insert(test_persona_definition("Planner"));
        registry
    }

    fn make_comment(id: u64, body: &str, login: &str) -> IssueComment {
        IssueComment {
            id,
            body: body.into(),
            user: terraphim_tracker::CommentUser {
                login: login.into(),
            },
            issue_number: 0, // filled by caller via parse_mentions arg
            created_at: "2026-03-30T00:00:00Z".into(),
            updated_at: "2026-03-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_parse_single_mention_agent_name() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(1, "Please @adf:security-sentinel review this code", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, LEGACY_PROJECT_ID);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "security-sentinel");
        assert_eq!(mentions[0].resolution, MentionResolution::AgentName);
        assert_eq!(mentions[0].raw_mention, "security-sentinel");
        assert_eq!(mentions[0].project_id, LEGACY_PROJECT_ID);
    }

    #[test]
    fn test_parse_single_mention_persona() {
        let agents = test_agents();
        let personas = test_personas();
        // "vigil" persona resolves to an agent. With "security" in context,
        // should prefer security-sentinel over compliance-watchdog.
        let comment = make_comment(2, "@adf:vigil check for security vulnerabilities", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, LEGACY_PROJECT_ID);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "security-sentinel");
        assert!(matches!(
            mentions[0].resolution,
            MentionResolution::PersonaName { .. }
        ));
    }

    #[test]
    fn test_parse_multiple_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(3, "@adf:vigil and @adf:carthos please review", "bob");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, LEGACY_PROJECT_ID);
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn test_parse_no_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(4, "No mentions here", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, LEGACY_PROJECT_ID);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_ignores_regular_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(5, "@alice please review", "bob");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, LEGACY_PROJECT_ID);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_stamps_hinted_project_id() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(6, "Please @adf:security-sentinel look at this", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas, "odilo");
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].project_id, "odilo");
    }

    #[test]
    fn test_resolve_persona_single_agent() {
        let agents = test_agents();
        let personas = test_personas();
        // Planner has only one agent: product-development
        let result = resolve_persona_mention("planner", &agents, &personas, "some context");
        assert!(result.is_some());
        let (name, res) = result.unwrap();
        assert_eq!(name, "product-development");
        assert!(matches!(res, MentionResolution::PersonaName { .. }));
    }

    #[test]
    fn test_resolve_persona_multiple_agents_keyword_match() {
        let agents = test_agents();
        let personas = test_personas();
        // Vigil shared by security-sentinel and compliance-watchdog
        // Context mentions "license" -> should pick compliance-watchdog
        let result =
            resolve_persona_mention("vigil", &agents, &personas, "check license compliance");
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "compliance-watchdog");
    }

    #[test]
    fn test_resolve_unknown_name() {
        let agents = test_agents();
        let personas = test_personas();
        let result = resolve_persona_mention("nonexistent", &agents, &personas, "context");
        assert!(result.is_none());
    }

    #[test]
    fn test_mention_cursor_now() {
        let cursor = MentionCursor::now();
        // Should be set to approximately now
        let parsed = chrono::DateTime::parse_from_rfc3339(&cursor.last_seen_at);
        assert!(parsed.is_ok());
        assert_eq!(cursor.dispatches_this_tick, 0);
    }

    #[test]
    fn test_mention_cursor_advance() {
        let mut cursor = MentionCursor::now();
        cursor.last_seen_at = "2026-04-03T10:00:00Z".to_string();

        // Should advance to newer timestamp
        cursor.advance_to("2026-04-03T12:00:00Z");
        assert_eq!(cursor.last_seen_at, "2026-04-03T12:00:00Z");

        // Should NOT advance to older timestamp
        cursor.advance_to("2026-04-03T08:00:00Z");
        assert_eq!(cursor.last_seen_at, "2026-04-03T12:00:00Z");
    }

    #[test]
    fn test_mention_tracker_issue_limit() {
        let mut tracker = MentionTracker::new(3);
        assert!(!tracker.limit_exceeded(42));
        tracker.record_dispatch(42);
        tracker.record_dispatch(42);
        tracker.record_dispatch(42);
        assert!(tracker.limit_exceeded(42));

        // Different issue should not be affected
        assert!(!tracker.limit_exceeded(99));
    }

    #[test]
    fn test_mention_tracker_reset() {
        let mut tracker = MentionTracker::new(2);
        tracker.record_dispatch(42);
        tracker.record_dispatch(42);
        assert!(tracker.limit_exceeded(42));

        tracker.reset();
        assert!(!tracker.limit_exceeded(42));
    }
}
