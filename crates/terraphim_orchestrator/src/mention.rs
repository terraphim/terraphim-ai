//! Mention detection and resolution for @adf:name comments.
//!
//! Supports two addressing modes:
//! - Agent name: `@adf:security-sentinel` (exact match on agent name)
//! - Persona name: `@adf:vigil` (resolved via PersonaRegistry)

use crate::config::AgentDefinition;
use crate::persona::PersonaRegistry;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use terraphim_tracker::gitea_write::IssueComment;

/// Regex for @adf:name mentions.
static MENTION_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@adf:([a-z][a-z0-9-]{1,39})\b").unwrap());

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
pub fn parse_and_resolve_mentions(
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

/// Tracks processed mentions to prevent re-triggers and infinite loops.
pub struct MentionTracker {
    processed: HashSet<(u64, u64, String)>,
    max_depth: u32,
    depth_counters: HashMap<u64, u32>,
}

impl MentionTracker {
    pub fn new(max_depth: u32) -> Self {
        Self {
            processed: HashSet::new(),
            max_depth,
            depth_counters: HashMap::new(),
        }
    }

    pub fn is_processed(&self, mention: &DetectedMention) -> bool {
        self.processed.contains(&(
            mention.issue_number,
            mention.comment_id,
            mention.agent_name.clone(),
        ))
    }

    pub fn mark_processed(&mut self, mention: &DetectedMention) {
        self.processed.insert((
            mention.issue_number,
            mention.comment_id,
            mention.agent_name.clone(),
        ));
    }

    pub fn depth_exceeded(&self, issue_number: u64) -> bool {
        self.depth_counters
            .get(&issue_number)
            .map(|&d| d >= self.max_depth)
            .unwrap_or(false)
    }

    pub fn increment_depth(&mut self, issue_number: u64) {
        *self.depth_counters.entry(issue_number).or_insert(0) += 1;
    }
}

/// Structured mention request emitted by an agent in its output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentionRequest {
    /// Target agent or persona name to mention.
    #[serde(rename = "mention")]
    pub target: String,
    /// Issue number to post the mention on.
    pub issue: u64,
    /// Context/request message for the target agent.
    pub message: String,
}

/// Extract mention requests from agent output text.
///
/// Scans for JSON objects matching: {"mention": "name", "issue": N, "message": "..."}
/// Returns all valid mention requests found.
pub fn extract_mention_requests(output: &str) -> Vec<MentionRequest> {
    let mut requests = Vec::new();
    // Scan for JSON objects in the output
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') && trimmed.contains("\"mention\"") {
            if let Ok(req) = serde_json::from_str::<MentionRequest>(trimmed) {
                requests.push(req);
            }
        }
    }
    // Also try to find embedded JSON in larger text blocks
    if let Some(start) = output.find("{\"mention\"") {
        if let Some(end) = output[start..].find('}') {
            let candidate = &output[start..=start + end];
            if let Ok(req) = serde_json::from_str::<MentionRequest>(candidate) {
                if !requests
                    .iter()
                    .any(|r| r.target == req.target && r.issue == req.issue)
                {
                    requests.push(req);
                }
            }
        }
    }
    requests
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AgentLayer;
    use terraphim_tracker::gitea_write::CommentUser;
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
            user: CommentUser {
                login: login.into(),
                id: 1,
            },
            created_at: "2026-03-30T00:00:00Z".into(),
            updated_at: "2026-03-30T00:00:00Z".into(),
        }
    }

    #[test]
    fn test_parse_single_mention_agent_name() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(1, "Please @adf:security-sentinel review this code", "alice");
        let mentions = parse_and_resolve_mentions(&comment, 42, &agents, &personas);
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
        let mentions = parse_and_resolve_mentions(&comment, 42, &agents, &personas);
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
        let mentions = parse_and_resolve_mentions(&comment, 42, &agents, &personas);
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn test_parse_no_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(4, "No mentions here", "alice");
        let mentions = parse_and_resolve_mentions(&comment, 42, &agents, &personas);
        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_ignores_regular_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let comment = make_comment(5, "@alice please review", "bob");
        let mentions = parse_and_resolve_mentions(&comment, 42, &agents, &personas);
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
    fn test_mention_tracker_dedup() {
        let mut tracker = MentionTracker::new(3);
        let mention = DetectedMention {
            issue_number: 42,
            comment_id: 1,
            raw_mention: "vigil".into(),
            agent_name: "security-sentinel".into(),
            resolution: MentionResolution::PersonaName {
                persona: "vigil".into(),
            },
            comment_body: "test".into(),
            mentioner: "alice".into(),
            timestamp: "2026-03-30T00:00:00Z".into(),
        };

        assert!(!tracker.is_processed(&mention));
        tracker.mark_processed(&mention);
        assert!(tracker.is_processed(&mention));
    }

    #[test]
    fn test_mention_depth_limit() {
        let mut tracker = MentionTracker::new(3);
        assert!(!tracker.depth_exceeded(42));
        tracker.increment_depth(42);
        tracker.increment_depth(42);
        tracker.increment_depth(42);
        assert!(tracker.depth_exceeded(42));
    }

    #[test]
    fn test_extract_mention_request_from_line() {
        let output = r#"Analysis complete.
{"mention": "vigil", "issue": 42, "message": "Please review security implications"}
Done."#;
        let requests = extract_mention_requests(output);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].target, "vigil");
        assert_eq!(requests[0].issue, 42);
        assert_eq!(requests[0].message, "Please review security implications");
    }

    #[test]
    fn test_extract_multiple_mention_requests() {
        let output = r#"Found issues.
{"mention": "vigil", "issue": 42, "message": "Security review needed"}
{"mention": "carthos", "issue": 42, "message": "Architecture review needed"}
"#;
        let requests = extract_mention_requests(output);
        assert_eq!(requests.len(), 2);
    }

    #[test]
    fn test_extract_no_mention_requests() {
        let output = "Just plain text output with no JSON";
        let requests = extract_mention_requests(output);
        assert!(requests.is_empty());
    }

    #[test]
    fn test_extract_malformed_json_ignored() {
        let output = r#"{"mention": "vigil", "issue": "not_a_number"}"#;
        let requests = extract_mention_requests(output);
        assert!(requests.is_empty());
    }
}
