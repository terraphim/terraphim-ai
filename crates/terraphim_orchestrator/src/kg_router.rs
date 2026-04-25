//! KG-driven model routing using markdown-defined rules.
//!
//! Loads routing rules from markdown files in a taxonomy directory.
//! Each file defines `route::` + `action::` pairs with `synonyms::` for
//! Aho-Corasick matching against agent task descriptions.
//!
//! Reuses [`terraphim_automata::find_matches`] for pattern matching and
//! [`terraphim_automata::markdown_directives::parse_markdown_directives_dir`]
//! for loading rules.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use terraphim_automata::markdown_directives::parse_markdown_directives_dir;
use terraphim_types::{
    MarkdownDirectives, NormalizedTerm, NormalizedTermValue, RouteDirective, Thesaurus,
};
use tracing::{debug, info, warn};

/// A routing decision from KG matching.
#[derive(Debug, Clone)]
pub struct KgRouteDecision {
    /// Provider name (e.g., "kimi", "anthropic")
    pub provider: String,
    /// Model identifier (e.g., "kimi-for-coding/k2p5", "claude-opus-4-6")
    pub model: String,
    /// CLI action template with `{{ model }}` and `{{ prompt }}` placeholders
    pub action: Option<String>,
    /// Match confidence (0.0-1.0)
    pub confidence: f64,
    /// Concept that matched (filename stem)
    pub matched_concept: String,
    /// Priority from the matched rule (0-100)
    pub priority: u8,
    /// All routes from the matched file (primary + fallbacks)
    pub fallback_routes: Vec<RouteDirective>,
}

impl KgRouteDecision {
    /// Render the action template by substituting `{{ model }}` and `{{ prompt }}`.
    pub fn render_action(&self, prompt: &str) -> Option<String> {
        self.action.as_ref().map(|template| {
            template
                .replace("{{ model }}", &self.model)
                .replace("{{model}}", &self.model)
                .replace("{{ prompt }}", prompt)
                .replace("{{prompt}}", prompt)
        })
    }

    /// Get the next fallback route, skipping providers in the exclude set.
    pub fn first_healthy_route(&self, unhealthy_providers: &[String]) -> Option<&RouteDirective> {
        self.fallback_routes
            .iter()
            .find(|r| !unhealthy_providers.contains(&r.provider))
    }
}

/// A routing rule loaded from a markdown file.
#[derive(Debug, Clone)]
struct RoutingRule {
    /// Concept name (file stem, e.g., "security_audit")
    concept: String,
    /// Parsed directives from the markdown file
    directives: MarkdownDirectives,
}

/// KG-based model router that loads routing rules from markdown files.
///
/// Uses the same directive format as the rest of the terraphim KG system:
/// `route::`, `action::`, `priority::`, `synonyms::`, `trigger::`.
#[derive(Clone)]
pub struct KgRouter {
    /// Loaded routing rules indexed by concept name
    rules: Vec<RoutingRule>,
    /// Thesaurus built from all synonyms across all rules.
    /// Maps synonym → concept name for Aho-Corasick matching.
    thesaurus: Thesaurus,
    /// Path being watched
    taxonomy_path: PathBuf,
    /// Latest mtime of any file in the taxonomy directory (for change detection).
    last_mtime: Option<SystemTime>,
}

impl std::fmt::Debug for KgRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KgRouter")
            .field("taxonomy_path", &self.taxonomy_path)
            .field("rules_count", &self.rules.len())
            .field("thesaurus_size", &self.thesaurus.len())
            .finish()
    }
}

impl KgRouter {
    /// Load routing rules from a taxonomy directory.
    ///
    /// Scans all `.md` files, parses directives, and builds a thesaurus
    /// from all `synonyms::` entries for Aho-Corasick matching.
    pub fn load(taxonomy_path: impl Into<PathBuf>) -> Result<Self, KgRouterError> {
        let taxonomy_path = taxonomy_path.into();
        if !taxonomy_path.exists() {
            return Err(KgRouterError::TaxonomyNotFound(
                taxonomy_path.display().to_string(),
            ));
        }

        let parse_result = parse_markdown_directives_dir(&taxonomy_path)
            .map_err(|e| KgRouterError::ParseError(e.to_string()))?;

        for w in &parse_result.warnings {
            warn!(
                path = %w.path.display(),
                line = ?w.line,
                msg = %w.message,
                "KG routing rule warning"
            );
        }

        let mut rules = Vec::new();
        let mut thesaurus = Thesaurus::new("kg_router".to_string());
        let mut term_id: u64 = 1;

        for (concept, directives) in &parse_result.directives {
            // Only include files that have at least one route
            if directives.routes.is_empty() {
                debug!(concept = %concept, "skipping KG file with no routes");
                continue;
            }

            // Build thesaurus entries: each synonym maps to this concept
            for synonym in &directives.synonyms {
                let key = NormalizedTermValue::from(synonym.clone());
                let term = NormalizedTerm {
                    id: term_id,
                    value: NormalizedTermValue::from(concept.clone()),
                    display_value: None,
                    url: None,
                    action: None,
                    priority: None,
                    trigger: None,
                    pinned: false,
                };
                thesaurus.insert(key, term);
                term_id += 1;
            }

            rules.push(RoutingRule {
                concept: concept.clone(),
                directives: directives.clone(),
            });
        }

        info!(
            path = %taxonomy_path.display(),
            rules = rules.len(),
            synonyms = thesaurus.len(),
            "KG router loaded"
        );

        let last_mtime = Self::dir_mtime(&taxonomy_path);

        Ok(Self {
            rules,
            thesaurus,
            taxonomy_path,
            last_mtime,
        })
    }

    /// Route an agent task description to the best provider+model.
    ///
    /// Uses [`terraphim_automata::find_matches`] to match task text against
    /// KG synonyms, then returns the highest-priority matched rule's primary route.
    pub fn route_agent(&self, task_description: &str) -> Option<KgRouteDecision> {
        if self.thesaurus.is_empty() {
            return None;
        }

        // Use terraphim_automata's find_matches for Aho-Corasick matching
        let matches = match terraphim_automata::find_matches(
            task_description,
            self.thesaurus.clone(),
            false,
        ) {
            Ok(m) if !m.is_empty() => m,
            Ok(_) => {
                debug!(task = %task_description.chars().take(80).collect::<String>(), "no KG synonym match");
                return None;
            }
            Err(e) => {
                warn!(error = %e, "KG router find_matches failed");
                return None;
            }
        };

        // Group matches by concept and find the best one
        let mut best: Option<(&RoutingRule, f64)> = None;

        for matched in &matches {
            // matched.normalized_term.value is the concept name
            let concept = matched.normalized_term.value.to_string();
            if let Some(rule) = self.rules.iter().find(|r| r.concept == concept) {
                let priority = rule.directives.priority.unwrap_or(50) as f64;
                // Score = priority (higher is better)
                // Multiple matches to the same concept don't stack
                let score = priority;

                match &best {
                    Some((_, best_score)) if score <= *best_score => {}
                    _ => best = Some((rule, score)),
                }
            }
        }

        let (rule, score) = best?;
        let primary = rule.directives.routes.first()?;

        let confidence = score / 100.0; // Normalise to 0.0-1.0

        info!(
            concept = %rule.concept,
            provider = %primary.provider,
            model = %primary.model,
            confidence = confidence,
            "KG route matched"
        );

        Some(KgRouteDecision {
            provider: primary.provider.clone(),
            model: primary.model.clone(),
            action: primary.action.clone(),
            confidence,
            matched_concept: rule.concept.clone(),
            priority: rule.directives.priority.unwrap_or(50),
            fallback_routes: rule.directives.routes.clone(),
        })
    }

    /// Reload rules from the taxonomy directory.
    pub fn reload(&mut self) -> Result<(), KgRouterError> {
        let reloaded = Self::load(&self.taxonomy_path)?;
        self.rules = reloaded.rules;
        self.thesaurus = reloaded.thesaurus;
        self.last_mtime = reloaded.last_mtime;
        info!(path = %self.taxonomy_path.display(), "KG router reloaded");
        Ok(())
    }

    /// Reload rules only if any file in the taxonomy directory has been modified.
    ///
    /// Checks the latest mtime of all `.md` files against the cached mtime.
    /// Returns `true` if a reload was performed.
    pub fn reload_if_changed(&mut self) -> bool {
        let current_mtime = Self::dir_mtime(&self.taxonomy_path);
        if current_mtime != self.last_mtime {
            match self.reload() {
                Ok(()) => {
                    info!(path = %self.taxonomy_path.display(), "KG routing rules hot-reloaded");
                    return true;
                }
                Err(e) => {
                    warn!(error = %e, "KG router hot-reload failed, keeping old rules");
                }
            }
        }
        false
    }

    /// Get the latest mtime of any `.md` file in a directory.
    fn dir_mtime(path: &Path) -> Option<SystemTime> {
        std::fs::read_dir(path)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "md")
                    .unwrap_or(false)
            })
            .filter_map(|e| e.metadata().ok()?.modified().ok())
            .max()
    }

    /// Get the taxonomy path.
    pub fn taxonomy_path(&self) -> &Path {
        &self.taxonomy_path
    }

    /// Number of loaded routing rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Iterate all unique route directives across all rules (for probing).
    pub fn all_routes(&self) -> Vec<&RouteDirective> {
        self.rules
            .iter()
            .flat_map(|r| r.directives.routes.iter())
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KgRouterError {
    #[error("taxonomy directory not found: {0}")]
    TaxonomyNotFound(String),
    #[error("failed to parse taxonomy: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_rule(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(format!("{name}.md")), content).unwrap();
    }

    #[test]
    fn routes_to_primary_by_synonym_match() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "implementation",
            r#"# Implementation
priority:: 50
synonyms:: implement, build, code, fix
route:: kimi, kimi-for-coding/k2p5
action:: opencode -m {{ model }} -p "{{ prompt }}"
route:: anthropic, claude-sonnet-4-6
action:: claude --model {{ model }} -p "{{ prompt }}"
"#,
        );

        let router = KgRouter::load(dir.path()).unwrap();
        let decision = router.route_agent("implement the new feature").unwrap();

        assert_eq!(decision.provider, "kimi");
        assert_eq!(decision.model, "kimi-for-coding/k2p5");
        assert_eq!(decision.matched_concept, "implementation");
        assert_eq!(decision.fallback_routes.len(), 2);
    }

    #[test]
    fn higher_priority_wins() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "implementation",
            "priority:: 50\nsynonyms:: implement, build, review code\nroute:: kimi, k2p5\n",
        );
        write_rule(
            dir.path(),
            "code_review",
            "priority:: 70\nsynonyms:: code review, architecture review\nroute:: anthropic, opus\n",
        );

        let router = KgRouter::load(dir.path()).unwrap();
        // "code review" matches code_review rule (priority 70)
        // "review code" would match implementation rule (priority 50)
        // code_review's higher priority should win
        let decision = router
            .route_agent("do a code review of the architecture")
            .unwrap();

        assert_eq!(decision.provider, "anthropic");
        assert_eq!(decision.matched_concept, "code_review");
    }

    #[test]
    fn no_match_returns_none() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "security",
            "priority:: 60\nsynonyms:: security audit, CVE\nroute:: kimi, k2p5\n",
        );

        let router = KgRouter::load(dir.path()).unwrap();
        assert!(router.route_agent("write documentation").is_none());
    }

    #[test]
    fn render_action_substitutes_placeholders() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "impl",
            r#"synonyms:: build
route:: kimi, k2p5
action:: opencode -m {{ model }} -p "{{ prompt }}"
"#,
        );

        let router = KgRouter::load(dir.path()).unwrap();
        let decision = router.route_agent("build it").unwrap();
        let rendered = decision.render_action("echo hello").unwrap();

        assert_eq!(rendered, r#"opencode -m k2p5 -p "echo hello""#);
    }

    #[test]
    fn first_healthy_route_skips_unhealthy() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "impl",
            "synonyms:: build\nroute:: kimi, k2p5\nroute:: anthropic, sonnet\n",
        );

        let router = KgRouter::load(dir.path()).unwrap();
        let decision = router.route_agent("build it").unwrap();

        let healthy = decision.first_healthy_route(&["kimi".to_string()]).unwrap();
        assert_eq!(healthy.provider, "anthropic");
    }

    #[test]
    fn review_tier_falls_back_to_kimi_when_anthropic_unhealthy() {
        let dir = tempdir().unwrap();
        write_rule(
            dir.path(),
            "review_tier",
            "priority:: 60\nsynonyms:: verify, validate, check results, drift detection\n\
             route:: anthropic, haiku\naction:: claude --model {{ model }} -p \"{{ prompt }}\" --max-turns 30\n\
             route:: kimi, kimi-for-coding/k2p5\naction:: opencode run -m {{ model }} --format json \"{{ prompt }}\"\n\
             route:: zai, zai-coding-plan/glm-5-turbo\naction:: opencode run -m {{ model }} --format json \"{{ prompt }}\"\n\
             route:: openai, openai/gpt-5.4-mini\naction:: opencode run -m {{ model }} --format json \"{{ prompt }}\"\n",
        );

        let router = KgRouter::load(dir.path()).unwrap();
        let decision = router
            .route_agent("verify and validate outputs, check results")
            .unwrap();

        // Primary: anthropic/haiku when all providers healthy
        assert_eq!(decision.provider, "anthropic");
        assert_eq!(decision.model, "haiku");

        // Anthropic unhealthy: kimi/k2p5 selected
        let healthy = decision
            .first_healthy_route(&["anthropic".to_string()])
            .unwrap();
        assert_eq!(healthy.provider, "kimi");
        assert_eq!(healthy.model, "kimi-for-coding/k2p5");

        // Anthropic + kimi unhealthy: zai selected
        let healthy2 = decision
            .first_healthy_route(&["anthropic".to_string(), "kimi".to_string()])
            .unwrap();
        assert_eq!(healthy2.provider, "zai");
        assert_eq!(healthy2.model, "zai-coding-plan/glm-5-turbo");

        // Four routes: anthropic + kimi + zai + openai
        assert_eq!(decision.fallback_routes.len(), 4);
    }

    #[test]
    fn empty_dir_loads_with_zero_rules() {
        let dir = tempdir().unwrap();
        let router = KgRouter::load(dir.path()).unwrap();
        assert_eq!(router.rule_count(), 0);
        assert!(router.route_agent("anything").is_none());
    }

    #[test]
    fn reload_picks_up_new_files() {
        let dir = tempdir().unwrap();
        let mut router = KgRouter::load(dir.path()).unwrap();
        assert_eq!(router.rule_count(), 0);

        write_rule(
            dir.path(),
            "security",
            "synonyms:: CVE\nroute:: kimi, k2p5\n",
        );
        router.reload().unwrap();
        assert_eq!(router.rule_count(), 1);
        assert!(router.route_agent("check CVE").is_some());
    }

    #[test]
    fn loads_real_adf_taxonomy_3_tiers() {
        let taxonomy = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/taxonomy/routing_scenarios/adf");
        if !taxonomy.exists() {
            return;
        }

        let router = KgRouter::load(&taxonomy).unwrap();
        assert_eq!(router.rule_count(), 3, "expected 3 tier files");

        // Every route should have an action template
        for route_directive in router.all_routes() {
            assert!(
                route_directive.action.is_some(),
                "route {}/{} missing action:: template",
                route_directive.provider,
                route_directive.model
            );
        }

        // Planning tier (priority 80)
        let d = router
            .route_agent("create a plan for strategic planning")
            .unwrap();
        assert_eq!(d.matched_concept, "planning_tier");
        assert_eq!(d.priority, 80);
        assert_eq!(d.provider, "anthropic");
        assert!(d.model.contains("opus"));

        // Review tier (priority 40) -- "verify" triggers review
        let d = router.route_agent("verify and validate results").unwrap();
        assert_eq!(d.matched_concept, "review_tier");
        assert_eq!(d.priority, 40);
        assert_eq!(d.provider, "anthropic");
        assert!(d.model.contains("haiku"));

        // Implementation tier (priority 50) -- "implement" triggers coding
        let d = router.route_agent("implement the new feature").unwrap();
        assert_eq!(d.matched_concept, "implementation_tier");
        assert_eq!(d.priority, 50);
        assert_eq!(d.provider, "anthropic");
        assert!(d.model.contains("sonnet"));
    }

    /// End-to-end: simulate ADF agent dispatch with phase-aware 3-tier routing.
    ///
    /// Each agent's task keywords determine its tier:
    /// - PLANNING (opus): strategic planning, architecture design, create a plan
    /// - REVIEW (haiku): verify, validate, check results, compliance check
    /// - IMPLEMENTATION (sonnet/kimi): implement, code, test, security audit
    #[test]
    fn e2e_all_adf_agents_route_to_correct_tier() {
        let taxonomy = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/taxonomy/routing_scenarios/adf");
        if !taxonomy.exists() {
            return;
        }

        let router = KgRouter::load(&taxonomy).unwrap();

        // (agent, task keywords, expected tier, expected primary provider)
        let agents: Vec<(&str, &str, &str, &str)> = vec![
            // PLANNING TIER (opus)
            (
                "meta-coordinator",
                "create a plan for strategic planning and cross-agent coordination",
                "planning_tier",
                "anthropic",
            ),
            (
                "product-development",
                "create a plan for product roadmap and feature prioritisation",
                "planning_tier",
                "anthropic",
            ),
            // REVIEW TIER (haiku)
            (
                "spec-validator",
                "verify and validate outputs, check results pass fail quality gate",
                "review_tier",
                "anthropic",
            ),
            (
                "quality-coordinator",
                "review code quality and verify test results for PR approval",
                "implementation_tier",
                "anthropic",
            ),
            (
                "compliance-watchdog",
                "verify compliance and check audit results against standards",
                "review_tier",
                "anthropic",
            ),
            (
                "drift-detector",
                "check drift detection and validate system state",
                "review_tier",
                "anthropic",
            ),
            (
                "merge-coordinator",
                "review merge verdict and evaluate GO NO-GO for PR approval",
                "implementation_tier",
                "anthropic",
            ),
            // IMPLEMENTATION TIER (sonnet)
            (
                "security-sentinel",
                "security audit cargo audit CVE vulnerability scan",
                "implementation_tier",
                "anthropic",
            ),
            (
                "test-guardian",
                "test QA regression integration test cargo test",
                "implementation_tier",
                "anthropic",
            ),
            (
                "implementation-swarm",
                "implement build code fix refactor feature PR",
                "implementation_tier",
                "anthropic",
            ),
            (
                "documentation-generator",
                "documentation readme changelog API docs technical writing",
                "implementation_tier",
                "anthropic",
            ),
            (
                "browser-qa",
                "test QA browser test end-to-end regression",
                "implementation_tier",
                "anthropic",
            ),
            (
                "log-analyst",
                "log analysis error pattern incident observability",
                "implementation_tier",
                "anthropic",
            ),
        ];

        let mut all_passed = true;
        for (agent, task, expected_tier, expected_provider) in &agents {
            match router.route_agent(task) {
                Some(decision) => {
                    let tier_ok = decision.matched_concept == *expected_tier;
                    let provider_ok = decision.provider == *expected_provider;
                    if !tier_ok || !provider_ok {
                        eprintln!(
                            "MISMATCH {}: got {}:{}/{} (expected {}:{})",
                            agent,
                            decision.matched_concept,
                            decision.provider,
                            decision.model,
                            expected_tier,
                            expected_provider,
                        );
                        all_passed = false;
                    } else {
                        eprintln!(
                            "OK {}: {} -> {}/{} (pri={})",
                            agent,
                            decision.matched_concept,
                            decision.provider,
                            decision.model,
                            decision.priority,
                        );
                    }
                }
                None => {
                    eprintln!("NO MATCH {}: task={}", agent, task);
                    all_passed = false;
                }
            }
        }
        assert!(all_passed, "some agents did not route to correct tier");
    }
}
