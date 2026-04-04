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

/// Regex for @adf:name mentions.
static MENTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@adf:([a-z][a-z0-9-]{1,39})\b").unwrap());

/// How a mention was resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum MentionResolution {
    AgentName,
    PersonaName { persona: String },
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
}

// ---------------------------------------------------------------------------
// MentionCursor: Persistent cursor for repo-wide comment polling
// ---------------------------------------------------------------------------

/// Persistent cursor for mention polling.
///
/// Stored via `terraphim_persistence` as JSON at key `adf/mention_cursor`.
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

    pub async fn load_or_now() -> Self {
        let key = "adf/mention_cursor";

        if let Some(op) = Self::sqlite_op().await {
            if let Ok(bs) = op.read(key).await {
                if let Ok(cursor) = serde_json::from_slice::<Self>(&bs.to_vec()) {
                    tracing::info!(
                        last_seen_at = %cursor.last_seen_at,
                        "loaded MentionCursor from persistence"
                    );
                    return cursor;
                }
                tracing::warn!("failed to deserialize MentionCursor, starting fresh");
            } else {
                tracing::info!("no persisted MentionCursor found, starting fresh");
            }
        } else {
            tracing::warn!("DeviceStorage sqlite not available, using in-memory cursor");
        }

        Self::now()
    }

    /// Save to persistence.
    pub async fn save(&self) {
        let key = "adf/mention_cursor";

        if let Some(op) = Self::sqlite_op().await {
            if let Ok(json) = serde_json::to_string(self) {
                if let Err(e) = op.write(key, json).await {
                    tracing::warn!(?e, "failed to save MentionCursor");
                } else {
                    tracing::debug!(last_seen_at = %self.last_seen_at, "saved MentionCursor");
                }
            } else {
                tracing::warn!("failed to serialize MentionCursor");
            }
        } else {
            tracing::warn!("DeviceStorage sqlite not available, cursor not persisted");
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

/// Resolve a raw mention to an agent name.
///
/// 1. If raw matches an agent name exactly -> AgentName
/// 2. If raw matches a persona name -> PersonaName (pick best-fit agent)
/// 3. No match -> None
pub fn resolve_mention(
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

/// Parse and resolve all @adf:name mentions from a comment.
pub fn parse_mentions(
    comment: &IssueComment,
    issue_number: u64,
    agents: &[AgentDefinition],
    personas: &PersonaRegistry,
) -> Vec<DetectedMention> {
    let mut mentions = Vec::new();

    for cap in MENTION_RE.captures_iter(&comment.body) {
        let raw = &cap[1];
        if let Some((agent_name, resolution)) =
            resolve_mention(raw, agents, personas, &comment.body)
        {
            mentions.push(DetectedMention {
                issue_number,
                comment_id: comment.id,
                raw_mention: raw.to_string(),
                agent_name,
                resolution,
                comment_body: comment.body.clone(),
                mentioner: comment.user.login.clone(),
                timestamp: comment.created_at.clone(),
            });
        } else {
            tracing::warn!(
                raw_mention = raw,
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
                persona: Some("Lux".into()),
                capabilities: vec!["typescript".into(), "frontend".into(), "ui".into()],
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
        registry.insert(test_persona_definition("Lux"));
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
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "security-sentinel");
        assert_eq!(mentions[0].resolution, MentionResolution::AgentName);
        assert_eq!(mentions[0].raw_mention, "security-sentinel");
    }

    #[test]
    fn test_parse_single_mention_persona() {
        let agents = test_agents();
        let personas = test_personas();
        // "vigil" persona resolves to an agent. With "security" in context,
        // should prefer security-sentinel over compliance-watchdog.
        let comment = make_comment(2, "@adf:vigil check for security vulnerabilities", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
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
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn test_parse_no_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(4, "No mentions here", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_ignores_regular_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(5, "@alice please review", "bob");
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_resolve_persona_single_agent() {
        let agents = test_agents();
        let personas = test_personas();
        // Lux has only one agent: product-development
        let result = resolve_mention("lux", &agents, &personas, "some context");
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
        let result = resolve_mention("vigil", &agents, &personas, "check license compliance");
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "compliance-watchdog");
    }

    #[test]
    fn test_resolve_unknown_name() {
        let agents = test_agents();
        let personas = test_personas();
        let result = resolve_mention("nonexistent", &agents, &personas, "context");
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
