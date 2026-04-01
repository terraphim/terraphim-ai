//! Mention detection and resolution for @adf:name comments.
//!
//! Supports two addressing modes:
//! - Agent name: `@adf:security-sentinel` (exact match on agent name)
//! - Persona name: `@adf:vigil` (resolved via PersonaRegistry)
//!
//! Uses Aho-Corasick automaton for O(text + patterns) mention detection,
//! replacing the previous regex-based O(text * patterns) approach.

use crate::config::AgentDefinition;
use crate::persona::PersonaRegistry;
use aho_corasick::AhoCorasick;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use terraphim_automata::find_matches;
use terraphim_tracker::IssueComment;
use terraphim_types::{NormalizedTerm, Thesaurus};

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

/// Aho-Corasick-based automaton for efficient mention detection.
///
/// Builds an automaton from agent names and persona names at construction time,
/// enabling O(text + patterns) mention detection instead of O(text * patterns).
pub struct MentionAutomaton {
    /// The Aho-Corasick automaton for pattern matching
    automaton: Arc<AhoCorasick>,
    /// Map from pattern index to resolution info
    pattern_map: Vec<PatternInfo>,
}

/// Information about a pattern in the automaton
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct PatternInfo {
    /// The raw pattern string (agent name or persona name)
    pattern: String,
    /// Whether this is an agent name or persona name
    kind: PatternKind,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum PatternKind {
    AgentName,
    PersonaName { persona: String },
}

impl MentionAutomaton {
    /// Build a new automaton from agent definitions and personas.
    ///
    /// Creates patterns for:
    /// - All agent names (e.g., "security-sentinel")
    /// - All persona names (e.g., "vigil")
    ///
    /// The automaton is case-insensitive and matches leftmost-longest patterns.
    pub fn new(agents: &[AgentDefinition], personas: &PersonaRegistry) -> Option<Self> {
        let mut patterns = Vec::new();
        let mut pattern_map = Vec::new();

        // Add agent name patterns
        for agent in agents {
            patterns.push(agent.name.clone());
            pattern_map.push(PatternInfo {
                pattern: agent.name.clone(),
                kind: PatternKind::AgentName,
            });
        }

        // Add persona name patterns
        for persona_name in personas.iter_names() {
            // Skip if persona name is already an agent name (avoid duplicates)
            if !agents
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case(persona_name))
            {
                patterns.push(persona_name.to_string());
                pattern_map.push(PatternInfo {
                    pattern: persona_name.to_string(),
                    kind: PatternKind::PersonaName {
                        persona: persona_name.to_string(),
                    },
                });
            }
        }

        if patterns.is_empty() {
            tracing::warn!("No agent or persona patterns available for mention detection");
            return None;
        }

        tracing::debug!(
            "Building MentionAutomaton with {} patterns ({} agents, {} personas)",
            patterns.len(),
            agents.len(),
            pattern_map.len() - agents.len()
        );

        let automaton = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&patterns)
            .map_err(|e| {
                tracing::error!("Failed to build Aho-Corasick automaton: {}", e);
                e
            })
            .ok()?;

        Some(Self {
            automaton: Arc::new(automaton),
            pattern_map,
        })
    }

    /// Find all @adf: mentions in the given text.
    ///
    /// Returns an iterator over (start, end, pattern_info) tuples.
    fn find_mentions<'a>(
        &'a self,
        text: &'a str,
    ) -> impl Iterator<Item = (usize, usize, &'a PatternInfo)> + 'a {
        self.automaton.find_iter(text).map(|mat| {
            let info = &self.pattern_map[mat.pattern()];
            (mat.start(), mat.end(), info)
        })
    }
}

/// Score an agent's capabilities against context using terraphim_automata.
///
/// Builds a Thesaurus from agent capabilities and uses find_matches() for
/// efficient Aho-Corasick matching against the context.
fn score_agent_capabilities(agent: &AgentDefinition, context_lower: &str) -> usize {
    let mut thesaurus = Thesaurus::new(format!("{}-caps", agent.name));
    for (idx, cap) in agent.capabilities.iter().enumerate() {
        let key = cap.to_lowercase();
        let term = NormalizedTerm::new(format!("cap-{}", idx), key.clone().into());
        thesaurus.insert(key.into(), term);
    }
    if thesaurus.is_empty() {
        return 0;
    }
    match find_matches(context_lower, thesaurus, false) {
        Ok(matches) => {
            let matched_terms: HashSet<&str> = matches.iter().map(|m| m.term.as_str()).collect();
            matched_terms.len()
        }
        Err(_) => 0,
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
    // 1. Direct agent name match (case-insensitive)
    if let Some(agent) = agents.iter().find(|a| a.name.eq_ignore_ascii_case(raw)) {
        return Some((agent.name.clone(), MentionResolution::AgentName));
    }

    // 2. Persona name match (case-insensitive)
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
                    let score = score_agent_capabilities(agent, &context_lower);
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

/// Parse and resolve all @adf:name mentions from a comment using Aho-Corasick automaton.
///
/// This function uses an Aho-Corasick automaton for O(text + patterns) performance,
/// replacing the previous regex-based O(text * patterns) approach.
pub fn parse_mentions(
    comment: &IssueComment,
    issue_number: u64,
    agents: &[AgentDefinition],
    personas: &PersonaRegistry,
) -> Vec<DetectedMention> {
    let mut mentions = Vec::new();

    // Build automaton for this parse (agents/personas may change between calls)
    let Some(automaton) = MentionAutomaton::new(agents, personas) else {
        return mentions;
    };

    // Scan for @adf: prefixes followed by known agent/persona names
    let body = &comment.body;
    let body_lower = body.to_lowercase();

    // Find all @adf: occurrences
    for (pos, _) in body_lower.match_indices("@adf:") {
        // Look for a pattern match immediately after @adf:
        let after_prefix = pos + 5; // Skip "@adf:"
        if after_prefix >= body.len() {
            continue;
        }

        // Check if there's a valid identifier after @adf:
        // Valid identifier: [a-z][a-z0-9-]{1,39}
        let remaining = &body[after_prefix..];
        let _remaining_lower = remaining.to_lowercase();

        // Use automaton to find matching patterns at this position
        for (start, end, info) in automaton.find_mentions(remaining) {
            // Ensure the match starts at the beginning of the remaining text
            if start != 0 {
                continue;
            }

            let raw = &info.pattern;
            let matched_text = &remaining[..end];

            // Verify this is a valid identifier (starts with letter, alphanumeric/ hyphens)
            if !is_valid_identifier(&matched_text.to_lowercase()) {
                continue;
            }

            // Check if this is a direct agent name match first (case-insensitive)
            if let Some(agent) = agents.iter().find(|a| a.name.eq_ignore_ascii_case(raw)) {
                mentions.push(DetectedMention {
                    issue_number,
                    comment_id: comment.id,
                    raw_mention: raw.to_string(),
                    agent_name: agent.name.clone(),
                    resolution: MentionResolution::AgentName,
                    comment_body: comment.body.clone(),
                    mentioner: comment.user.login.clone(),
                    timestamp: comment.created_at.clone(),
                });
            } else if let Some((agent_name, resolution)) =
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

            // Only process the first match at this position
            break;
        }
    }

    mentions
}

/// Check if a string is a valid identifier for @adf: mentions.
/// Valid: starts with lowercase letter, followed by lowercase letters, digits, or hyphens.
fn is_valid_identifier(s: &str) -> bool {
    if s.len() < 2 || s.len() > 40 {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    // First character must be a lowercase letter
    if !first.is_ascii_lowercase() {
        return false;
    }

    // Remaining characters must be lowercase letters, digits, or hyphens
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
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
    fn test_valid_identifier() {
        assert!(is_valid_identifier("security-sentinel"));
        assert!(is_valid_identifier("a1"));
        assert!(is_valid_identifier("test-123-abc"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("1abc"));
        assert!(!is_valid_identifier("Test"));
        assert!(!is_valid_identifier("test_123"));
        assert!(!is_valid_identifier("a"));
    }

    #[test]
    fn test_automaton_creation() {
        let agents = test_agents();
        let personas = test_personas();

        let automaton = MentionAutomaton::new(&agents, &personas);
        assert!(automaton.is_some());

        let auto = automaton.unwrap();
        assert_eq!(auto.pattern_map.len(), 7); // 4 agents + 3 personas
    }

    #[test]
    fn test_automaton_empty_agents() {
        let agents: Vec<AgentDefinition> = vec![];
        let personas = test_personas();

        let automaton = MentionAutomaton::new(&agents, &personas);
        assert!(automaton.is_some()); // Should still work with just personas
    }

    #[test]
    fn test_automaton_empty() {
        let agents: Vec<AgentDefinition> = vec![];
        let personas = PersonaRegistry::new();

        let automaton = MentionAutomaton::new(&agents, &personas);
        assert!(automaton.is_none()); // No patterns available
    }

    #[test]
    fn test_case_insensitive_mention() {
        let agents = test_agents();
        let personas = test_personas();

        // Test various casing
        let comment1 = make_comment(1, "@adf:SECURITY-SENTINEL check this", "alice");
        let mentions1 = parse_mentions(&comment1, 42, &agents, &personas);
        assert_eq!(mentions1.len(), 1);
        assert_eq!(mentions1[0].agent_name, "security-sentinel");

        let comment2 = make_comment(2, "@adf:Vigil review please", "bob");
        let mentions2 = parse_mentions(&comment2, 42, &agents, &personas);
        assert_eq!(mentions2.len(), 1);
        // Vigil persona is shared by security-sentinel and compliance-watchdog
        // The specific agent returned depends on keyword matching or alphabetical tie-break
        assert!(
            mentions2[0].agent_name == "security-sentinel"
                || mentions2[0].agent_name == "compliance-watchdog"
        );
        assert!(matches!(
            mentions2[0].resolution,
            MentionResolution::PersonaName { .. }
        ));
    }

    #[test]
    fn test_mention_at_end_of_sentence() {
        let agents = test_agents();
        let personas = test_personas();

        let comment = make_comment(1, "Please review, @adf:spec-validator.", "alice");
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        // The period after spec-validator is a word boundary, so it should match
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "spec-validator");
    }

    #[test]
    fn test_multiple_mentions_same_comment() {
        let agents = test_agents();
        let personas = test_personas();

        let comment = make_comment(
            1,
            "@adf:security-sentinel please audit. @adf:carthos check the design.",
            "alice",
        );
        let mentions = parse_mentions(&comment, 42, &agents, &personas);
        assert_eq!(mentions.len(), 2);

        let agent_names: Vec<_> = mentions.iter().map(|m| m.agent_name.as_str()).collect();
        assert!(agent_names.contains(&"security-sentinel"));
        assert!(agent_names.contains(&"spec-validator"));
    }

    #[test]
    fn test_score_agent_capabilities_basic() {
        let agent = test_agents()
            .into_iter()
            .find(|a| a.name == "security-sentinel")
            .unwrap();
        let score = score_agent_capabilities(&agent, "run a security audit on this code");
        assert!(score >= 2);
    }

    #[test]
    fn test_score_agent_capabilities_empty_caps() {
        let mut agent = test_agents()[0].clone();
        agent.capabilities = vec![];
        let score = score_agent_capabilities(&agent, "anything");
        assert_eq!(score, 0);
    }

    #[test]
    fn test_score_agent_capabilities_no_match() {
        let agent = test_agents()
            .into_iter()
            .find(|a| a.name == "security-sentinel")
            .unwrap();
        let score = score_agent_capabilities(&agent, "hello world");
        assert_eq!(score, 0);
    }
}
