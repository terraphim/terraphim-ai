//! weather-report: ADF model-tier "weather report".
//!
//! Shows which LLM models are currently available across the ADF routing
//! tiers, classifying each tier as THINKING (deep reasoning), WORKHORSE
//! (balanced implementation), or FAST & CHEAP (verification/review), and
//! reporting a live "weather" condition per model.
//!
//! Reuses the same Terraphim crates the ADF orchestrator uses:
//!   * `terraphim_orchestrator::kg_router::KgRouter` -- the tier routing rules.
//!   * `terraphim_orchestrator::provider_probe::ProviderHealthMap` -- live
//!     probing of every `(cli, provider, model)` route via its action template.
//!   * `terraphim_automata` / `terraphim_types` -- taxonomy parsing and the
//!     `RouteDirective` type.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};

use terraphim_orchestrator::kg_router::KgRouter;
use terraphim_orchestrator::provider_probe::{ProbeResult, ProviderHealthMap};

use terraphim_weather_report::{
    TierKind, WeatherReport, build_report, filter_by_kind, load_tier_routes,
};

/// Default taxonomy directory, relative to the repo root.
const DEFAULT_TAXONOMY_REL: &str = "docs/taxonomy/routing_scenarios/adf";

/// Latency above which a successful probe is "FAIR" rather than "SUNNY".
const DEFAULT_SLOW_THRESHOLD_MS: u64 = 3000;

/// Provider probe per-model timeout (fixed by the orchestrator at 15s).
const PROBE_TIMEOUT_SECS: u64 = 15;

#[derive(Parser, Debug)]
#[command(
    name = "weather-report",
    version,
    about = "ADF model-tier weather report: what's available across thinking / workhorse / fast-and-cheap tiers"
)]
struct Cli {
    /// Path to the routing taxonomy directory (markdown tier files).
    #[arg(long, global = true, env = "ADF_TAXONOMY_DIR", value_name = "PATH")]
    taxonomy: Option<PathBuf>,

    /// Fire live probes (executes each route's action template; real API
    /// calls). Off by default -- the default run shows the configured roster.
    #[arg(long, global = true)]
    probe: bool,

    /// Latency (ms) above which a successful probe is reported as FAIR.
    #[arg(long, global = true, default_value_t = DEFAULT_SLOW_THRESHOLD_MS, value_name = "MS")]
    slow_threshold: u64,

    /// Output format.
    #[arg(long, global = true, value_enum, default_value_t)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Option<ReportCommand>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Subcommand, Debug)]
enum ReportCommand {
    /// Full report across all tiers (default).
    Report,
    /// Only the THINKING tiers (planning, decision).
    Thinking,
    /// Only the FAST & CHEAP tiers (review, verification).
    Fast,
    /// Only the WORKHORSE tiers (implementation).
    Workhorse,
    /// List configured tiers and their model rosters without probing.
    Tiers,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let taxonomy = resolve_taxonomy(cli.taxonomy.as_deref())?;
    let slow = cli.slow_threshold;
    let format = cli.format;

    let entries = load_tier_routes(&taxonomy)
        .with_context(|| format!("failed to load taxonomy from {}", taxonomy.display()))?;

    if entries.is_empty() {
        anyhow::bail!(
            "no tier routes found in {}; expected markdown files with `route::` directives",
            taxonomy.display()
        );
    }

    let command = cli.command.unwrap_or(ReportCommand::Report);
    // `tiers` always lists the roster without probing; otherwise probe only
    // when --probe is set.
    let list_only = matches!(command, ReportCommand::Tiers) || !cli.probe;

    // Probe the full taxonomy (not the filtered subset) so the summary stays
    // consistent regardless of which subcommand view the user picked.
    let probes: Vec<ProbeResult> = if list_only {
        Vec::new()
    } else {
        eprintln!(
            "Probing {} route(s) across {} tier(s); up to {PROBE_TIMEOUT_SECS}s per model...",
            entries.iter().map(|(_, d)| d.routes.len()).sum::<usize>(),
            entries.len()
        );
        run_probes(&taxonomy)?
    };

    let probed = !list_only;
    let mut report = build_report(&taxonomy, &entries, &probes, probed, slow);

    match command {
        ReportCommand::Thinking => report = filter_by_kind(report, TierKind::Thinking),
        ReportCommand::Fast => report = filter_by_kind(report, TierKind::FastCheap),
        ReportCommand::Workhorse => report = filter_by_kind(report, TierKind::Workhorse),
        ReportCommand::Report | ReportCommand::Tiers => {}
    }

    match format {
        OutputFormat::Json => print_json(&report)?,
        OutputFormat::Human => print_human(&report, command),
    }

    Ok(())
}

/// Probe every route in the taxonomy concurrently via the orchestrator's
/// `ProviderHealthMap`. Each route's `action::` template is executed with a
/// minimal test prompt by the orchestrator's own `probe_single`.
fn run_probes(taxonomy: &std::path::Path) -> Result<Vec<ProbeResult>> {
    let router = KgRouter::load(taxonomy).map_err(|e| {
        anyhow::anyhow!("failed to load KG router from {}: {e}", taxonomy.display())
    })?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to create tokio runtime")?;

    let results = rt.block_on(async move {
        let mut health = ProviderHealthMap::new(Duration::from_secs(1800));
        health.probe_all(&router).await;
        health.results().to_vec()
    });

    Ok(results)
}

/// Resolve the taxonomy directory: explicit arg > env > walk up from CWD
/// looking for the default relative path.
fn resolve_taxonomy(explicit: Option<&std::path::Path>) -> Result<PathBuf> {
    if let Some(p) = explicit {
        if !p.is_dir() {
            anyhow::bail!("taxonomy directory not found: {}", p.display());
        }
        return Ok(p.to_path_buf());
    }

    let mut cur = std::env::current_dir().context("failed to get current directory")?;
    loop {
        let candidate = cur.join(DEFAULT_TAXONOMY_REL);
        if candidate.is_dir() {
            return Ok(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    anyhow::bail!(
        "could not find taxonomy directory '{DEFAULT_TAXONOMY_REL}' walking up from CWD; \
         pass --taxonomy <PATH> or set ADF_TAXONOMY_DIR"
    );
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

fn print_json(report: &WeatherReport) -> Result<()> {
    let stdout = std::io::stdout();
    serde_json::to_writer_pretty(stdout.lock(), report)?;
    println!();
    Ok(())
}

fn print_human(report: &WeatherReport, command: ReportCommand) {
    let ts = now_timestamp();
    println!("Model Weather Report -- {ts}");
    println!(
        "Taxonomy: {} ({} tier(s), {} model(s))",
        report.taxonomy_path,
        report.tiers.len(),
        report.total_models
    );
    let mode = if report.probed {
        format!("live probe ({PROBE_TIMEOUT_SECS}s timeout per model)")
    } else {
        "configured roster (no live probe)".to_string()
    };
    println!("Mode: {mode}");
    println!();

    for tier in &report.tiers {
        print_tier(tier);
    }

    print_summary(report, command);
}

fn print_tier(tier: &terraphim_weather_report::TierSection) {
    println!("--- {} : {} ---", tier.kind.label(), tier.heading);
    println!("    {}", tier.kind.blurb());
    if let Some(p) = tier.priority {
        println!("    priority: {p}");
    }
    println!();

    // Column widths for alignment.
    for m in &tier.models {
        print_model_row(m);
    }
    println!();
}

fn print_model_row(m: &terraphim_weather_report::ModelRow) {
    let cond = m.condition.token();
    let latency = match m.latency_ms {
        Some(ms) => format!("{ms}ms"),
        None => match m.condition {
            terraphim_weather_report::WeatherCondition::Stormy => "timeout".to_string(),
            _ => "-".to_string(),
        },
    };
    let cost = if m.is_free { "FREE" } else { "paid" };
    println!(
        "  {cond:<7} {:<16} {:<24} {:<10} {latency:<8} {cost}",
        m.provider, m.model, m.cli
    );
    if let Some(detail) = &m.detail {
        if !detail.is_empty() {
            println!("          {detail}");
        }
    }
}

fn print_summary(report: &WeatherReport, command: ReportCommand) {
    let s = &report.summary;
    let mut parts: Vec<String> = Vec::new();
    if s.available() > 0 {
        parts.push(format!(
            "{} available ({} sunny, {} fair, {} cloudy)",
            s.available(),
            s.sunny,
            s.fair,
            s.cloudy
        ));
    }
    if s.stormy > 0 {
        parts.push(format!("{} stormy", s.stormy));
    }
    if s.offline > 0 {
        parts.push(format!("{} offline", s.offline));
    }
    if s.unknown > 0 {
        parts.push(format!("{} unknown (not measurable)", s.unknown));
    }
    if s.configured > 0 {
        parts.push(format!("{} configured (not probed)", s.configured));
    }
    let joined = if parts.is_empty() {
        "no models".to_string()
    } else {
        parts.join(" | ")
    };

    let scope = match command {
        ReportCommand::Thinking => " [thinking tiers]",
        ReportCommand::Fast => " [fast & cheap tiers]",
        ReportCommand::Workhorse => " [workhorse tiers]",
        ReportCommand::Report | ReportCommand::Tiers => "",
    };
    println!("Summary{scope}: {joined}");
}

fn now_timestamp() -> String {
    // Project standard is jiff (not chrono).
    jiff::Timestamp::now().to_string()
}
