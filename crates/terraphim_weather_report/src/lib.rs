//! Pure logic for the ADF model "weather report".
//!
//! This module contains no I/O beyond reading the taxonomy directory, so the
//! classification, condition-mapping, and report-assembly logic is unit-testable
//! without any CLI tooling or network access.
//!
//! It reuses three Terraphim crates:
//! * [`terraphim_automata`] -- parses the tier taxonomy markdown into directives.
//! * [`terraphim_types`] -- the [`RouteDirective`] / [`MarkdownDirectives`] types.
//! * [`terraphim_orchestrator`] -- the [`ProbeResult`] / [`ProbeStatus`] types
//!   produced by the ADF provider probe.

use std::path::Path;

use serde::{Deserialize, Serialize};
use terraphim_orchestrator::provider_probe::{ProbeResult, ProbeStatus};
use terraphim_types::{MarkdownDirectives, RouteDirective};

/// Coarse classification of a routing tier, mapping the ADF taxonomy tiers to
/// the two poles the user cares about -- "thinking" vs "fast and cheap" -- plus
/// a middle "workhorse" band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierKind {
    /// Deep reasoning models: planning, decision, research, design.
    Thinking,
    /// Balanced mid-range models: implementation, build.
    Workhorse,
    /// Fast, cheap models: review, verification, validation.
    FastCheap,
}

impl TierKind {
    /// Short uppercase label suitable for table headers and log lines.
    pub fn label(&self) -> &'static str {
        match self {
            TierKind::Thinking => "THINKING",
            TierKind::Workhorse => "WORKHORSE",
            TierKind::FastCheap => "FAST & CHEAP",
        }
    }

    /// One-line description of what the tier is for.
    pub fn blurb(&self) -> &'static str {
        match self {
            TierKind::Thinking => {
                "deep reasoning -- strongest models for architecture, planning, decisions"
            }
            TierKind::Workhorse => "balanced -- mid-range models for implementation and review",
            TierKind::FastCheap => "fast and cheap -- verification, validation, lightweight checks",
        }
    }
}

/// Classify a tier from its concept name (the taxonomy filename stem, e.g.
/// `planning_tier`) with a priority fallback for unknown concepts.
pub fn classify_tier(concept: &str, priority: Option<u8>) -> TierKind {
    let c = concept.to_ascii_lowercase();
    if c.contains("planning")
        || c.contains("decision")
        || c.contains("research")
        || c.contains("design")
    {
        return TierKind::Thinking;
    }
    if c.contains("review") || c.contains("verif") || c.contains("valid") || c.contains("check") {
        return TierKind::FastCheap;
    }
    if c.contains("implement") || c.contains("build") || c.contains("code") {
        return TierKind::Workhorse;
    }
    match priority {
        Some(p) if p >= 60 => TierKind::Thinking,
        Some(p) if p >= 45 => TierKind::Workhorse,
        _ => TierKind::FastCheap,
    }
}

/// A weather-style condition for a single model route.
///
/// Token labels are uppercase ASCII (no emoji) so they render cleanly in a
/// monospace terminal and stay stable for JSON consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherCondition {
    /// Probe succeeded quickly: model is up and responsive.
    Sunny,
    /// Probe succeeded but slowly: up, but lethargic.
    Fair,
    /// Rate-limited / throttled: up but metered.
    Cloudy,
    /// Timed out: unreachable within the probe window.
    Stormy,
    /// Genuine provider error: the API itself is down or erroring.
    Offline,
    /// Environment/config error: could not measure (missing CLI, allow-list,
    /// no action template). Not a provider-health statement.
    Unknown,
    /// No probe was run (--no-probe); only the configured roster is shown.
    Configured,
}

impl WeatherCondition {
    /// Fixed-width (7 char) token for human-aligned table output.
    pub fn token(&self) -> &'static str {
        match self {
            WeatherCondition::Sunny => "SUNNY",
            WeatherCondition::Fair => "FAIR",
            WeatherCondition::Cloudy => "CLOUDY",
            WeatherCondition::Stormy => "STORMY",
            WeatherCondition::Offline => "OFFLINE",
            WeatherCondition::Unknown => "UNKNOWN",
            WeatherCondition::Configured => "CONFIG",
        }
    }

    /// Map an optional probe result to a condition.
    ///
    /// `None` means the route was not probed (returns [`WeatherCondition::Configured`]).
    /// `slow_threshold_ms` separates [`WeatherCondition::Sunny`] from
    /// [`WeatherCondition::Fair`] among successful probes.
    pub fn from_probe(probe: Option<&ProbeResult>, slow_threshold_ms: u64) -> Self {
        let Some(p) = probe else {
            return WeatherCondition::Configured;
        };
        match p.status {
            ProbeStatus::Success => match p.latency_ms {
                Some(ms) if ms > slow_threshold_ms => WeatherCondition::Fair,
                _ => WeatherCondition::Sunny,
            },
            ProbeStatus::RateLimited => WeatherCondition::Cloudy,
            ProbeStatus::Timeout => WeatherCondition::Stormy,
            ProbeStatus::Error => {
                if is_environment_error(p.error.as_deref().unwrap_or("")) {
                    WeatherCondition::Unknown
                } else {
                    WeatherCondition::Offline
                }
            }
        }
    }
}

/// Determine whether a probe error represents a local environment/config issue
/// (missing CLI tool, C1 allow-list gate, missing action template) rather than a
/// genuine provider health failure. Mirrors the orchestrator's own classifier so
/// "Unknown" lines up with what the circuit breaker would skip.
fn is_environment_error(error: &str) -> bool {
    (error.contains("CLI tool") && error.contains("not found on PATH"))
        || error.contains("not in C1 allow-list")
        || error.contains("no action:: template defined")
        || error.contains("spawn failed")
}

/// A single model row in the report.
#[derive(Debug, Clone, Serialize)]
pub struct ModelRow {
    /// Provider identifier (e.g. `anthropic`, `openai`).
    pub provider: String,
    /// Model identifier within the provider (e.g. `claude-sonnet-4-6`).
    pub model: String,
    /// CLI tool basename used to reach this model (e.g. `claude`, `opencode`).
    pub cli: String,
    /// Whether the route is flagged as free-tier in the taxonomy.
    pub is_free: bool,
    /// Observed probe condition for this model.
    pub condition: WeatherCondition,
    /// Round-trip latency in milliseconds from the last probe, if measured.
    pub latency_ms: Option<u64>,
    /// Human-readable error or detail from the probe, if any.
    pub detail: Option<String>,
}

/// One tier section of the report (one taxonomy file).
#[derive(Debug, Clone, Serialize)]
pub struct TierSection {
    /// Taxonomy concept name (the markdown filename stem, e.g. `planning_tier`).
    pub concept: String,
    /// Human-readable heading from the taxonomy file's first `#` line.
    pub heading: String,
    /// Coarse tier classification derived from the concept name and priority.
    pub kind: TierKind,
    /// Priority value from the taxonomy directive, if present.
    pub priority: Option<u8>,
    /// All model rows in this tier, in taxonomy order.
    pub models: Vec<ModelRow>,
}

/// Aggregate counts per condition for the summary line.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ConditionSummary {
    /// Number of models reporting [`WeatherCondition::Sunny`] (fast and up).
    pub sunny: usize,
    /// Number of models reporting [`WeatherCondition::Fair`] (up but slow).
    pub fair: usize,
    /// Number of models reporting [`WeatherCondition::Cloudy`] (rate-limited).
    pub cloudy: usize,
    /// Number of models reporting [`WeatherCondition::Stormy`] (timed out).
    pub stormy: usize,
    /// Number of models reporting [`WeatherCondition::Offline`] (API error).
    pub offline: usize,
    /// Number of models reporting [`WeatherCondition::Unknown`] (env/config error).
    pub unknown: usize,
    /// Number of models reporting [`WeatherCondition::Configured`] (not probed).
    pub configured: usize,
}

impl ConditionSummary {
    fn record(&mut self, c: WeatherCondition) {
        match c {
            WeatherCondition::Sunny => self.sunny += 1,
            WeatherCondition::Fair => self.fair += 1,
            WeatherCondition::Cloudy => self.cloudy += 1,
            WeatherCondition::Stormy => self.stormy += 1,
            WeatherCondition::Offline => self.offline += 1,
            WeatherCondition::Unknown => self.unknown += 1,
            WeatherCondition::Configured => self.configured += 1,
        }
    }

    /// Number of models that are genuinely usable right now (up, regardless of
    /// speed). Excludes Unknown/Configured since those were not measured.
    pub fn available(&self) -> usize {
        self.sunny + self.fair + self.cloudy
    }
}

/// The full weather report.
#[derive(Debug, Clone, Serialize)]
pub struct WeatherReport {
    /// ISO-8601 timestamp when the report was generated.
    pub generated_at: String,
    /// Absolute path to the taxonomy directory that was loaded.
    pub taxonomy_path: String,
    /// `true` if live provider probes were run; `false` for `--no-probe` mode.
    pub probed: bool,
    /// Total number of model routes across all tiers.
    pub total_models: usize,
    /// Aggregate condition counts across all tiers.
    pub summary: ConditionSummary,
    /// Per-tier breakdowns, ordered by priority descending.
    pub tiers: Vec<TierSection>,
}

/// Load the tier taxonomy from a directory of markdown routing files.
///
/// Returns `(concept, directives)` pairs for files that declare at least one
/// route, sorted by priority descending (strongest tiers first) and then by
/// concept name for stable ordering.
pub fn load_tier_routes(taxonomy: &Path) -> anyhow::Result<Vec<(String, MarkdownDirectives)>> {
    let parsed = terraphim_automata::markdown_directives::parse_markdown_directives_dir(taxonomy)
        .map_err(|e| {
        anyhow::anyhow!("failed to parse taxonomy at {}: {e}", taxonomy.display())
    })?;

    let mut entries: Vec<(String, MarkdownDirectives)> = parsed
        .directives
        .into_iter()
        .filter(|(_, d)| !d.routes.is_empty())
        .collect();

    entries.sort_by(|a, b| {
        b.1.priority
            .unwrap_or(0)
            .cmp(&a.1.priority.unwrap_or(0))
            .then_with(|| a.0.cmp(&b.0))
    });
    Ok(entries)
}

/// Find the probe result (if any) that matches a route's `(cli, provider,
/// model)` triple. The orchestrator keys probe health the same way so that the
/// same model reached via two CLIs has independent health.
pub fn find_probe<'a>(
    probes: &'a [ProbeResult],
    route: &RouteDirective,
) -> Option<&'a ProbeResult> {
    let cli = route.cli_basename().unwrap_or("");
    probes
        .iter()
        .find(|p| p.cli_tool == cli && p.provider == route.provider && p.model == route.model)
}

/// Build a [`ModelRow`] from a route and an optional matching probe.
pub fn model_row(
    route: &RouteDirective,
    probe: Option<&ProbeResult>,
    slow_threshold_ms: u64,
) -> ModelRow {
    let condition = WeatherCondition::from_probe(probe, slow_threshold_ms);
    ModelRow {
        provider: route.provider.clone(),
        model: route.model.clone(),
        cli: route.cli_basename().unwrap_or("-").to_string(),
        is_free: route.is_free,
        latency_ms: probe.and_then(|p| p.latency_ms),
        detail: probe.and_then(|p| p.error.clone()),
        condition,
    }
}

/// Assemble the full report from loaded tier routes and probe results.
///
/// `probed` should be `false` when the caller passed `--no-probe`; in that case
/// `probes` is empty and every row resolves to [`WeatherCondition::Configured`].
pub fn build_report(
    taxonomy_path: &Path,
    entries: &[(String, MarkdownDirectives)],
    probes: &[ProbeResult],
    probed: bool,
    slow_threshold_ms: u64,
) -> WeatherReport {
    let mut tiers = Vec::with_capacity(entries.len());
    let mut summary = ConditionSummary::default();
    let mut total_models = 0usize;

    for (concept, directives) in entries {
        let kind = classify_tier(concept, directives.priority);
        let heading = directives
            .heading
            .clone()
            .unwrap_or_else(|| concept.clone());
        let mut models = Vec::with_capacity(directives.routes.len());
        for route in &directives.routes {
            let probe = find_probe(probes, route);
            let row = model_row(route, probe, slow_threshold_ms);
            summary.record(row.condition);
            total_models += 1;
            models.push(row);
        }
        tiers.push(TierSection {
            concept: concept.clone(),
            heading,
            kind,
            priority: directives.priority,
            models,
        });
    }

    WeatherReport {
        generated_at: now(),
        taxonomy_path: taxonomy_path.display().to_string(),
        probed,
        total_models,
        summary,
        tiers,
    }
}

/// Filter a report down to a single tier kind (used by the `thinking` /
/// `fast` subcommands). Re-computes the summary over the filtered set.
pub fn filter_by_kind(mut report: WeatherReport, kind: TierKind) -> WeatherReport {
    report.tiers.retain(|t| t.kind == kind);
    let mut summary = ConditionSummary::default();
    let mut total = 0;
    for tier in &report.tiers {
        for m in &tier.models {
            summary.record(m.condition);
            total += 1;
        }
    }
    report.total_models = total;
    report.summary = summary;
    report
}

fn now() -> String {
    // Project standard is jiff (not chrono). RFC3339 UTC.
    jiff::Timestamp::now().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_orchestrator::provider_probe::{ProbeResult, ProbeStatus};
    use terraphim_types::RouteDirective;

    fn route(provider: &str, model: &str, cli: &str, is_free: bool) -> RouteDirective {
        RouteDirective {
            provider: provider.into(),
            model: model.into(),
            action: Some(format!(
                "/home/x/.local/bin/{cli} --provider {{provider}} --model {{model}} -p \"{{prompt}}\""
            )),
            is_free,
        }
    }

    fn probe(status: ProbeStatus, latency_ms: Option<u64>, error: Option<&str>) -> ProbeResult {
        ProbeResult {
            provider: "p".into(),
            model: "m".into(),
            cli_tool: "c".into(),
            status,
            latency_ms,
            error: error.map(String::from),
            timestamp: "now".into(),
        }
    }

    #[test]
    fn classifies_known_tiers_by_name() {
        assert_eq!(classify_tier("planning_tier", Some(80)), TierKind::Thinking);
        assert_eq!(classify_tier("decision_tier", Some(65)), TierKind::Thinking);
        assert_eq!(
            classify_tier("implementation_tier", Some(50)),
            TierKind::Workhorse
        );
        assert_eq!(classify_tier("review_tier", Some(40)), TierKind::FastCheap);
        assert_eq!(
            classify_tier("disciplined-verification", None),
            TierKind::FastCheap
        );
    }

    #[test]
    fn classifies_unknown_tier_by_priority() {
        assert_eq!(classify_tier("weird", Some(70)), TierKind::Thinking);
        assert_eq!(classify_tier("weird", Some(50)), TierKind::Workhorse);
        assert_eq!(classify_tier("weird", Some(10)), TierKind::FastCheap);
        assert_eq!(classify_tier("weird", None), TierKind::FastCheap);
    }

    #[test]
    fn condition_from_probe_maps_all_statuses() {
        assert_eq!(
            WeatherCondition::from_probe(None, 3000),
            WeatherCondition::Configured
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&probe(ProbeStatus::Success, Some(100), None)), 3000),
            WeatherCondition::Sunny
        );
        assert_eq!(
            WeatherCondition::from_probe(
                Some(&probe(ProbeStatus::Success, Some(5000), None)),
                3000
            ),
            WeatherCondition::Fair
        );
        // Missing latency on success still counts as sunny (responsive).
        assert_eq!(
            WeatherCondition::from_probe(Some(&probe(ProbeStatus::Success, None, None)), 3000),
            WeatherCondition::Sunny
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&probe(ProbeStatus::RateLimited, None, None)), 3000),
            WeatherCondition::Cloudy
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&probe(ProbeStatus::Timeout, None, None)), 3000),
            WeatherCondition::Stormy
        );
    }

    #[test]
    fn condition_distinguishes_env_error_from_offline() {
        let env_missing = probe(
            ProbeStatus::Error,
            None,
            Some("probe skipped: CLI tool 'claude' not found on PATH"),
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&env_missing), 3000),
            WeatherCondition::Unknown
        );

        let env_allowlist = probe(
            ProbeStatus::Error,
            None,
            Some("probe skipped: provider not in C1 allow-list"),
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&env_allowlist), 3000),
            WeatherCondition::Unknown
        );

        let real_down = probe(
            ProbeStatus::Error,
            None,
            Some("HTTP 503 service unavailable"),
        );
        assert_eq!(
            WeatherCondition::from_probe(Some(&real_down), 3000),
            WeatherCondition::Offline
        );
    }

    #[test]
    fn find_probe_matches_on_cli_provider_model() {
        let r = route("anthropic", "opus", "claude", false);
        let probes = vec![ProbeResult {
            provider: "anthropic".into(),
            model: "opus".into(),
            cli_tool: "claude".into(),
            status: ProbeStatus::Success,
            latency_ms: Some(120),
            error: None,
            timestamp: "t".into(),
        }];
        assert!(find_probe(&probes, &r).is_some());

        let wrong_cli = route("anthropic", "opus", "opencode", false);
        assert!(find_probe(&probes, &wrong_cli).is_none());
    }

    #[test]
    fn build_report_summarises_conditions() {
        let r_fast = route("anthropic", "haiku", "claude", false);
        let r_down = route("openai", "gpt-99", "opencode", false);
        let directives = MarkdownDirectives {
            priority: Some(40),
            routes: vec![r_fast.clone(), r_down.clone()],
            heading: Some("Review Tier".into()),
            ..Default::default()
        };
        let entries = vec![("review_tier".to_string(), directives)];

        let probes = vec![
            ProbeResult {
                provider: "anthropic".into(),
                model: "haiku".into(),
                cli_tool: "claude".into(),
                status: ProbeStatus::Success,
                latency_ms: Some(200),
                error: None,
                timestamp: "t".into(),
            },
            ProbeResult {
                provider: "openai".into(),
                model: "gpt-99".into(),
                cli_tool: "opencode".into(),
                status: ProbeStatus::Timeout,
                latency_ms: None,
                error: None,
                timestamp: "t".into(),
            },
        ];

        let report = build_report(Path::new("/tmp/tax"), &entries, &probes, true, 3000);

        assert_eq!(report.tiers.len(), 1);
        assert_eq!(report.tiers[0].kind, TierKind::FastCheap);
        assert_eq!(report.total_models, 2);
        assert_eq!(report.summary.sunny, 1);
        assert_eq!(report.summary.stormy, 1);
        assert_eq!(report.summary.available(), 1);
        assert_eq!(report.tiers[0].models[0].condition, WeatherCondition::Sunny);
        assert_eq!(
            report.tiers[0].models[1].condition,
            WeatherCondition::Stormy
        );
    }

    #[test]
    fn no_probe_marks_everything_configured() {
        let r = route("zai-coding-plan", "glm-5.1", "pi-rust", true);
        let directives = MarkdownDirectives {
            priority: Some(80),
            routes: vec![r],
            ..Default::default()
        };
        let entries = vec![("planning_tier".to_string(), directives)];

        let report = build_report(Path::new("/tmp/tax"), &entries, &[], false, 3000);
        assert!(!report.probed);
        assert_eq!(report.summary.configured, 1);
        assert_eq!(report.summary.available(), 0);
    }

    #[test]
    fn filter_by_kind_recomputes_summary() {
        let make = |concept: &str, prio: u8, kind_check: TierKind| {
            assert_eq!(classify_tier(concept, Some(prio)), kind_check);
            let d = MarkdownDirectives {
                priority: Some(prio),
                routes: vec![route("p", "m", "c", false)],
                ..Default::default()
            };
            (concept.to_string(), d)
        };
        let entries = vec![
            make("planning_tier", 80, TierKind::Thinking),
            make("review_tier", 40, TierKind::FastCheap),
        ];
        // one successful probe for each
        let probes = vec![
            ProbeResult {
                provider: "p".into(),
                model: "m".into(),
                cli_tool: "c".into(),
                status: ProbeStatus::Success,
                latency_ms: Some(100),
                error: None,
                timestamp: "t".into(),
            };
            2
        ];
        let report = build_report(Path::new("/x"), &entries, &probes, true, 3000);
        assert_eq!(report.summary.sunny, 2);

        let thinking = filter_by_kind(report, TierKind::Thinking);
        assert_eq!(thinking.tiers.len(), 1);
        assert_eq!(thinking.total_models, 1);
        assert_eq!(thinking.summary.sunny, 1);
    }

    #[test]
    fn loads_real_adf_taxonomy() {
        // The canonical ADF taxonomy shipped in this repo.
        let path = Path::new("docs/taxonomy/routing_scenarios/adf");
        if !path.exists() {
            eprintln!("skipping: {path:?} not present in this checkout");
            return;
        }
        let entries = load_tier_routes(path).expect("taxonomy parses");
        assert!(!entries.is_empty(), "ADF taxonomy should define tiers");

        // Sorted by priority descending: planning (80) should come first.
        let priorities: Vec<u8> = entries
            .iter()
            .map(|(_, d)| d.priority.unwrap_or(0))
            .collect();
        let mut sorted = priorities.clone();
        sorted.sort_by_key(|&p| std::cmp::Reverse(p));
        assert_eq!(priorities, sorted);

        // Known tiers are classified into the expected bands.
        let kinds: Vec<TierKind> = entries
            .iter()
            .map(|(c, d)| classify_tier(c, d.priority))
            .collect();
        assert!(
            kinds.contains(&TierKind::Thinking),
            "expected a thinking tier"
        );
        assert!(
            kinds.contains(&TierKind::FastCheap),
            "expected a fast-and-cheap tier"
        );

        // Every tier declares at least one route.
        for (_, d) in &entries {
            assert!(!d.routes.is_empty());
        }
    }
}
