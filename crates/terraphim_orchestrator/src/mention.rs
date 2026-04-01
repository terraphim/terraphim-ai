//! Mention detection and resolution for @adf:name comments.
//!
//! Supports two addressing modes:
//! - Agent name: `@adf:security-sentinel` (exact match on agent name)
//! - Persona name: `@adf:vigil` (resolved via PersonaRegistry)
//!
//! Uses Thesaurus-based Aho-Corasick automaton for O(text + patterns) mention detection,
//! replacing the previous regex-based O(text * patterns) approach.

use crate::config::AgentDefinition;
use crate::persona::PersonaRegistry;
use std::collections::{HashMap, HashSet};
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

/// Pre-built capability Thesauri for agent scoring.
/// Built once at startup from all agents' capability lists.
pub struct CapabilityThesauri {
    thesauri: HashMap<String, Thesaurus>,
}

impl CapabilityThesauri {
    pub fn build(agents: &[AgentDefinition]) -> Self {
        let mut thesauri = HashMap::new();
        for agent in agents {
            let mut thesaurus = Thesaurus::new(format!("{}-caps", agent.name));
            for (idx, cap) in agent.capabilities.iter().enumerate() {
                let key = cap.to_lowercase();
                let term = NormalizedTerm::new(format!("cap-{}", idx), key.clone().into());
                thesaurus.insert(key.into(), term);
            }
            if !thesaurus.is_empty() {
                thesauri.insert(agent.name.clone(), thesaurus);
            }
        }
        Self { thesauri }
    }

    pub fn score(&self, agent_name: &str, context_lower: &str) -> usize {
        match self.thesauri.get(agent_name) {
            Some(thesaurus) => match find_matches(context_lower, thesaurus.clone(), false) {
                Ok(matches) => {
                    let matched_terms: HashSet<&str> =
                        matches.iter().map(|m| m.term.as_str()).collect();
                    matched_terms.len()
                }
                Err(_) => 0,
            },
            None => 0,
        }
    }
}

/// Thesaurus-based mention detection.
///
/// Builds a Thesaurus at startup where:
/// - Each agent name becomes: key="@adf:agent-name", NormalizedTerm { id: "agent-name", value: "@adf:agent-name" }
/// - Each unique persona becomes a synonym: key="@adf:persona-name", NormalizedTerm {
///   id: "agent-name", value: "@adf:persona-name", display_value: Some("persona:persona-name")
///   }
/// - For ambiguous personas (multiple agents share same persona name), tracked in ambiguous_personas
pub struct MentionThesaurus {
    thesaurus: Thesaurus,
    /// Persona names that map to multiple agents (need disambiguation)
    ambiguous_personas: HashSet<String>,
}

impl MentionThesaurus {
    /// Build a new MentionThesaurus from agent definitions and persona registry.
    ///
    /// Creates Thesaurus entries for:
    /// - All agent names as @adf:agent-name patterns
    /// - All persona names as @adf:persona-name patterns
    ///
    /// Personas shared by multiple agents are tracked in ambiguous_personas.
    pub fn build(agents: &[AgentDefinition], _personas: &PersonaRegistry) -> Self {
        let mut thesaurus = Thesaurus::new("mention-thesaurus".to_string());
        let mut ambiguous_personas: HashSet<String> = HashSet::new();
        let mut persona_agent_counts: HashMap<String, Vec<String>> = HashMap::new();

        // First pass: count how many agents share each persona
        for agent in agents {
            if let Some(ref persona) = agent.persona {
                let persona_lower = persona.to_lowercase();
                persona_agent_counts
                    .entry(persona_lower)
                    .or_default()
                    .push(agent.name.clone());
            }
        }

        // Add agent name patterns: @adf:agent-name -> agent-name
        for agent in agents {
            let key = format!("@adf:{}", agent.name.to_lowercase());
            let term = NormalizedTerm::new(agent.name.clone(), key.clone().into());
            thesaurus.insert(key.into(), term);
        }

        // Add persona patterns: @adf:persona-name -> agent-name
        // For ambiguous personas, we'll use capability scoring at detection time
        for (persona_lower, agent_names) in &persona_agent_counts {
            let key = format!("@adf:{}", persona_lower);

            if agent_names.len() > 1 {
                // Ambiguous persona - mark it and use first agent as default
                ambiguous_personas.insert(persona_lower.clone());
                // Store all agents for this persona (we'll disambiguate at detection time)
                // For now, use alphabetical first as default
                let default_agent = agent_names.iter().min().unwrap().clone();
                let term = NormalizedTerm::new(default_agent.clone(), key.clone().into())
                    .with_display_value(format!("persona:{}", persona_lower));
                thesaurus.insert(key.into(), term);
            } else {
                // Unambiguous persona - direct mapping
                let agent_name = agent_names[0].clone();
                let term = NormalizedTerm::new(agent_name.clone(), key.clone().into())
                    .with_display_value(format!("persona:{}", persona_lower));
                thesaurus.insert(key.into(), term);
            }
        }

        tracing::debug!(
            "Built MentionThesaurus with {} patterns ({} agents, {} personas, {} ambiguous)",
            thesaurus.len(),
            agents.len(),
            persona_agent_counts.len(),
            ambiguous_personas.len()
        );

        Self {
            thesaurus,
            ambiguous_personas,
        }
    }

    /// Detect all @adf: mentions in the given text.
    ///
    /// Returns Vec of (agent_name, raw_mention, is_ambiguous) tuples.
    /// Uses find_matches() for O(text + patterns) performance.
    pub fn detect(&self, text: &str) -> Vec<(String, String, bool)> {
        let mut results = Vec::new();

        // Use find_matches to detect all patterns in one pass
        let matches = match find_matches(text, self.thesaurus.clone(), true) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("find_matches failed: {}", e);
                return results;
            }
        };

        for matched in matches {
            let raw_mention = matched.term;
            let agent_name = matched.normalized_term.id.clone();

            // Check if this is a persona mention by looking at the pattern
            let is_ambiguous = if let Some(name_part) = raw_mention.strip_prefix("@adf:") {
                self.ambiguous_personas.contains(name_part)
            } else {
                false
            };

            results.push((agent_name, raw_mention, is_ambiguous));
        }

        results
    }

    /// Check if a persona name is ambiguous (maps to multiple agents).
    pub fn is_ambiguous(&self, persona: &str) -> bool {
        self.ambiguous_personas.contains(&persona.to_lowercase())
    }

    /// Get all agents that share an ambiguous persona.
    pub fn get_ambiguous_agents<'a>(
        &self,
        persona: &str,
        all_agents: &'a [AgentDefinition],
    ) -> Vec<&'a AgentDefinition> {
        let persona_lower = persona.to_lowercase();
        if !self.ambiguous_personas.contains(&persona_lower) {
            return Vec::new();
        }

        all_agents
            .iter()
            .filter(|a| {
                a.persona
                    .as_ref()
                    .map(|p| p.eq_ignore_ascii_case(&persona_lower))
                    .unwrap_or(false)
            })
            .collect()
    }
}

/// Parse and resolve all @adf:name mentions from a comment using Thesaurus-based detection.
///
/// This function uses a Thesaurus with find_matches() for O(text + patterns) performance,
/// replacing the previous regex-based O(text * patterns) approach.
pub fn parse_mentions(
    comment: &IssueComment,
    issue_number: u64,
    agents: &[AgentDefinition],
    _personas: &PersonaRegistry,
    mention_thesaurus: &MentionThesaurus,
    cap_thesauri: &CapabilityThesauri,
) -> Vec<DetectedMention> {
    let mut mentions = Vec::new();
    let body = &comment.body;

    // Use the thesaurus to detect all mentions in one pass
    let detected = mention_thesaurus.detect(body);

    for (default_agent_name, raw_mention, is_ambiguous) in detected {
        // Determine final agent name and resolution type
        let (final_agent_name, resolution) = if is_ambiguous {
            // Extract persona name from raw mention (e.g., "@adf:vigil" -> "vigil")
            let persona_name = if let Some(name) = raw_mention.strip_prefix("@adf:") {
                name.to_string()
            } else {
                raw_mention.clone()
            };

            // Get all agents with this persona
            let matching_agents: Vec<&AgentDefinition> = agents
                .iter()
                .filter(|a| {
                    a.persona
                        .as_ref()
                        .map(|p| p.eq_ignore_ascii_case(&persona_name))
                        .unwrap_or(false)
                })
                .collect();

            // Disambiguate using capability scoring
            let context_lower = body.to_lowercase();
            let mut best_agent = matching_agents[0];
            let mut best_score = 0usize;

            for agent in &matching_agents {
                let score = cap_thesauri.score(&agent.name, &context_lower);
                if score > best_score || (score == best_score && agent.name < best_agent.name) {
                    best_score = score;
                    best_agent = agent;
                }
            }

            (
                best_agent.name.clone(),
                MentionResolution::PersonaName {
                    persona: persona_name,
                },
            )
        } else if let Some(name_part) = raw_mention.strip_prefix("@adf:") {
            // Check if this is a direct agent name match
            if agents
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case(name_part))
            {
                (default_agent_name, MentionResolution::AgentName)
            } else {
                // It's a persona mention (unambiguous)
                let persona_name = name_part.to_string();
                (
                    default_agent_name,
                    MentionResolution::PersonaName {
                        persona: persona_name,
                    },
                )
            }
        } else {
            (default_agent_name, MentionResolution::AgentName)
        };

        mentions.push(DetectedMention {
            issue_number,
            comment_id: comment.id,
            raw_mention: raw_mention.clone(),
            agent_name: final_agent_name,
            resolution,
            comment_body: comment.body.clone(),
            mentioner: comment.user.login.clone(),
            timestamp: comment.created_at.clone(),
        });
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

    fn test_cap_thesauri() -> CapabilityThesauri {
        CapabilityThesauri::build(&test_agents())
    }

    fn test_mention_thesaurus() -> MentionThesaurus {
        let agents = test_agents();
        let personas = test_personas();
        MentionThesaurus::build(&agents, &personas)
    }

    #[test]
    fn test_mention_thesaurus_build() {
        let agents = test_agents();
        let personas = test_personas();
        let thesaurus = MentionThesaurus::build(&agents, &personas);

        // Should have entries for 4 agents + 3 personas = 7 total
        // But we need to verify by checking detection
        assert_eq!(thesaurus.thesaurus.len(), 7);

        // Vigil should be marked as ambiguous (2 agents share it)
        assert!(thesaurus.is_ambiguous("Vigil"));
        assert!(thesaurus.is_ambiguous("vigil"));

        // Carthos and Lux should not be ambiguous
        assert!(!thesaurus.is_ambiguous("Carthos"));
        assert!(!thesaurus.is_ambiguous("Lux"));
    }

    #[test]
    fn test_mention_thesaurus_direct_name_detection() {
        let thesaurus = test_mention_thesaurus();

        // Test direct agent name detection
        let results = thesaurus.detect("Please @adf:security-sentinel review this");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "security-sentinel");
        assert_eq!(results[0].1, "@adf:security-sentinel");
        assert!(!results[0].2); // Not ambiguous
    }

    #[test]
    fn test_mention_thesaurus_persona_synonym() {
        let thesaurus = test_mention_thesaurus();

        // Test unambiguous persona detection (Carthos -> spec-validator)
        let results = thesaurus.detect("@adf:carthos check the spec");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "spec-validator");
        assert_eq!(results[0].1, "@adf:carthos");
        assert!(!results[0].2); // Not ambiguous
    }

    #[test]
    fn test_mention_thesaurus_ambiguous_persona() {
        let thesaurus = test_mention_thesaurus();

        // Test ambiguous persona detection (Vigil is shared)
        let results = thesaurus.detect("@adf:vigil check security");
        assert_eq!(results.len(), 1);
        // Should return one of the agents (default is alphabetical first)
        assert!(results[0].0 == "compliance-watchdog" || results[0].0 == "security-sentinel");
        assert_eq!(results[0].1, "@adf:vigil");
        assert!(results[0].2); // Is ambiguous
    }

    #[test]
    fn test_mention_thesaurus_multiple_mentions() {
        let thesaurus = test_mention_thesaurus();

        // Test multiple mentions in one comment
        let results = thesaurus.detect("@adf:security-sentinel and @adf:carthos please review");
        assert_eq!(results.len(), 2);

        let agent_names: Vec<_> = results.iter().map(|r| r.0.clone()).collect();
        assert!(agent_names.contains(&"security-sentinel".to_string()));
        assert!(agent_names.contains(&"spec-validator".to_string()));
    }

    #[test]
    fn test_mention_thesaurus_no_mentions() {
        let thesaurus = test_mention_thesaurus();

        // Test no false positives
        let results = thesaurus.detect("No mentions here at all");
        assert!(results.is_empty());

        // Test non-@adf mentions are ignored
        let results = thesaurus.detect("@alice please review");
        assert!(results.is_empty());

        // Test unknown agent names are ignored
        let results = thesaurus.detect("@adf:unknown-agent please help");
        assert!(results.is_empty());
    }

    #[test]
    fn test_parse_single_mention_agent_name() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(1, "Please @adf:security-sentinel review this code", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "security-sentinel");
        assert_eq!(mentions[0].resolution, MentionResolution::AgentName);
        assert_eq!(mentions[0].raw_mention, "@adf:security-sentinel");
    }

    #[test]
    fn test_parse_single_mention_persona() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        // "vigil" persona resolves to an agent. With "security" in context,
        // should prefer security-sentinel over compliance-watchdog.
        let comment = make_comment(2, "@adf:vigil check for security vulnerabilities", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

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
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(3, "@adf:vigil and @adf:carthos please review", "bob");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn test_parse_no_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(4, "No mentions here", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert!(mentions.is_empty());
    }

    #[test]
    fn test_parse_ignores_regular_mentions() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(5, "@alice please review", "bob");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert!(mentions.is_empty());
    }

    #[test]
    fn test_resolve_persona_single_agent() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        // Lux has only one agent: product-development
        let comment = make_comment(1, "@adf:lux help with frontend", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "product-development");
        assert!(matches!(
            mentions[0].resolution,
            MentionResolution::PersonaName { .. }
        ));
    }

    #[test]
    fn test_resolve_persona_multiple_agents_keyword_match() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        // Vigil shared by security-sentinel and compliance-watchdog
        // Context mentions "license" -> should pick compliance-watchdog
        let comment = make_comment(1, "@adf:vigil check license compliance", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );

        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "compliance-watchdog");
    }

    #[test]
    fn test_case_insensitive_mention() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        // Test various casing
        let comment1 = make_comment(1, "@adf:SECURITY-SENTINEL check this", "alice");
        let mentions1 = parse_mentions(
            &comment1,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );
        assert_eq!(mentions1.len(), 1);
        assert_eq!(mentions1[0].agent_name, "security-sentinel");

        let comment2 = make_comment(2, "@adf:Vigil review please", "bob");
        let mentions2 = parse_mentions(
            &comment2,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );
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
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(1, "Please review, @adf:spec-validator.", "alice");
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );
        // The period after spec-validator is a word boundary, so it should match
        assert_eq!(mentions.len(), 1);
        assert_eq!(mentions[0].agent_name, "spec-validator");
    }

    #[test]
    fn test_multiple_mentions_same_comment() {
        let agents = test_agents();
        let personas = test_personas();
        let cap_thesauri = test_cap_thesauri();
        let mention_thesaurus = test_mention_thesaurus();

        let comment = make_comment(
            1,
            "@adf:security-sentinel please audit. @adf:carthos check the design.",
            "alice",
        );
        let mentions = parse_mentions(
            &comment,
            42,
            &agents,
            &personas,
            &mention_thesaurus,
            &cap_thesauri,
        );
        assert_eq!(mentions.len(), 2);

        let agent_names: Vec<_> = mentions.iter().map(|m| m.agent_name.as_str()).collect();
        assert!(agent_names.contains(&"security-sentinel"));
        assert!(agent_names.contains(&"spec-validator"));
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
}
