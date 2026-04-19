use std::path::PathBuf;
use std::process::ExitCode;

use terraphim_orchestrator::config::OrchestratorConfig;
use terraphim_orchestrator::AgentOrchestrator;
use terraphim_types::capability::{Capability, CostLevel, Latency, Provider, ProviderType};
use tracing_subscriber::EnvFilter;

/// Register LLM providers for keyword-based model routing.
///
/// These providers are matched against task prompts via KeywordRouter.
/// The selected model_id is passed to the CLI tool via -m/--model flags.
fn register_providers(orchestrator: &mut AgentOrchestrator) {
    let router = orchestrator.router_mut();

    // Deep thinking: o3 (OpenAI reasoning model)
    router.add_provider(Provider {
        id: "openai-o3".into(),
        name: "OpenAI o3".into(),
        provider_type: ProviderType::Llm {
            model_id: "o3".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::DeepThinking,
            Capability::SecurityAudit,
            Capability::Architecture,
        ],
        cost_level: CostLevel::Expensive,
        latency: Latency::Slow,
        keywords: vec![
            "reason".into(),
            "think".into(),
            "analyze deeply".into(),
            "security".into(),
            "vulnerability".into(),
            "CVE".into(),
            "architecture".into(),
        ],
    });

    // General coding: o4-mini (fast, capable)
    router.add_provider(Provider {
        id: "openai-o4-mini".into(),
        name: "OpenAI o4-mini".into(),
        provider_type: ProviderType::Llm {
            model_id: "o4-mini".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::CodeGeneration,
            Capability::CodeReview,
            Capability::Testing,
            Capability::Refactoring,
        ],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec![
            "implement".into(),
            "code".into(),
            "review".into(),
            "test".into(),
            "refactor".into(),
            "check".into(),
        ],
    });

    // Fast/cheap: gpt-4.1-nano
    router.add_provider(Provider {
        id: "openai-4.1-nano".into(),
        name: "OpenAI GPT-4.1 Nano".into(),
        provider_type: ProviderType::Llm {
            model_id: "gpt-4.1-nano".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![
            Capability::FastThinking,
            Capability::Explanation,
            Capability::Documentation,
        ],
        cost_level: CostLevel::Cheap,
        latency: Latency::Fast,
        keywords: vec![
            "quick".into(),
            "fast".into(),
            "summary".into(),
            "explain".into(),
            "document".into(),
        ],
    });

    // Performance optimization: o4-mini (good at analysis)
    router.add_provider(Provider {
        id: "openai-o4-mini-perf".into(),
        name: "OpenAI o4-mini (perf)".into(),
        provider_type: ProviderType::Llm {
            model_id: "o4-mini".into(),
            api_endpoint: "https://api.openai.com".into(),
        },
        capabilities: vec![Capability::Performance],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec!["performance".into(), "optimize".into(), "benchmark".into()],
    });

    tracing::info!("registered 4 LLM providers for keyword routing");
}

enum Cli {
    Run { config: PathBuf },
    Check { config: PathBuf },
    Help,
}

fn parse_args() -> Result<Cli, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut check: Option<PathBuf> = None;
    let mut positional: Option<PathBuf> = None;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--check" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--check requires a config path".to_string())?;
                check = Some(PathBuf::from(path));
            }
            "-h" | "--help" => return Ok(Cli::Help),
            other if other.starts_with("--") => {
                return Err(format!("unknown flag: {other}"));
            }
            other => {
                if positional.is_some() {
                    return Err(format!("unexpected positional argument: {other}"));
                }
                positional = Some(PathBuf::from(other));
            }
        }
    }

    if let Some(config) = check {
        Ok(Cli::Check { config })
    } else {
        let config =
            positional.unwrap_or_else(|| PathBuf::from("/opt/ai-dark-factory/orchestrator.toml"));
        Ok(Cli::Run { config })
    }
}

fn print_help() {
    println!("adf -- AI Dark Factory orchestrator");
    println!();
    println!("USAGE:");
    println!("    adf [CONFIG]           Run the orchestrator");
    println!("    adf --check CONFIG     Validate config + print agent routing table");
    println!("    adf --help             Show this message");
}

/// Run the dry-run validator: load, validate, and print the routing table.
/// Returns exit code 0 on success, 1 on failure.
fn run_check(path: PathBuf) -> ExitCode {
    let config = match OrchestratorConfig::from_file(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("adf --check FAILED to load {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };

    if let Err(e) = config.validate() {
        eprintln!("adf --check FAILED validation for {}: {e}", path.display());
        return ExitCode::from(1);
    }

    print_routing_table(&config);
    ExitCode::SUCCESS
}

/// Print a sorted table of `(project_id, agent_name, model_or_fallback, layer)`.
fn print_routing_table(config: &OrchestratorConfig) {
    // Build rows: (project, agent, model_or_fallback, layer)
    let mut rows: Vec<(String, String, String, String)> = Vec::with_capacity(config.agents.len());
    for agent in &config.agents {
        let project = agent
            .project
            .clone()
            .unwrap_or_else(|| "<legacy>".to_string());
        let model = match (&agent.model, &agent.fallback_model) {
            (Some(m), _) => m.clone(),
            (None, Some(fb)) => format!("(fallback) {fb}"),
            (None, None) => "<unset>".to_string(),
        };
        rows.push((
            project,
            agent.name.clone(),
            model,
            format!("{:?}", agent.layer),
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let w_project = rows
        .iter()
        .map(|r| r.0.len())
        .chain(std::iter::once("PROJECT".len()))
        .max()
        .unwrap_or(7);
    let w_agent = rows
        .iter()
        .map(|r| r.1.len())
        .chain(std::iter::once("AGENT".len()))
        .max()
        .unwrap_or(5);
    let w_model = rows
        .iter()
        .map(|r| r.2.len())
        .chain(std::iter::once("MODEL".len()))
        .max()
        .unwrap_or(5);
    let w_layer = rows
        .iter()
        .map(|r| r.3.len())
        .chain(std::iter::once("LAYER".len()))
        .max()
        .unwrap_or(5);

    println!(
        "{:<w_project$}  {:<w_agent$}  {:<w_model$}  {:<w_layer$}",
        "PROJECT",
        "AGENT",
        "MODEL",
        "LAYER",
        w_project = w_project,
        w_agent = w_agent,
        w_model = w_model,
        w_layer = w_layer,
    );
    println!(
        "{}  {}  {}  {}",
        "-".repeat(w_project),
        "-".repeat(w_agent),
        "-".repeat(w_model),
        "-".repeat(w_layer),
    );
    for (project, agent, model, layer) in rows {
        println!(
            "{:<w_project$}  {:<w_agent$}  {:<w_model$}  {:<w_layer$}",
            project,
            agent,
            model,
            layer,
            w_project = w_project,
            w_agent = w_agent,
            w_model = w_model,
            w_layer = w_layer,
        );
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("error: {e}");
            print_help();
            return ExitCode::from(2);
        }
    };

    match cli {
        Cli::Help => {
            print_help();
            ExitCode::SUCCESS
        }
        Cli::Check { config } => run_check(config),
        Cli::Run { config } => {
            tracing::info!(config = %config.display(), "loading orchestrator config");

            let mut orchestrator = match AgentOrchestrator::from_config_file(&config) {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("failed to load config {}: {e}", config.display());
                    return ExitCode::from(1);
                }
            };

            register_providers(&mut orchestrator);

            #[cfg(feature = "quickwit")]
            {
                if let Some(qw_config) = orchestrator.quickwit_config().cloned() {
                    if qw_config.enabled {
                        let sink = terraphim_orchestrator::quickwit::QuickwitSink::new(
                            qw_config.endpoint.clone(),
                            qw_config.index_id.clone(),
                            qw_config.batch_size,
                            qw_config.flush_interval_secs,
                        );
                        orchestrator.set_quickwit_sink(sink);
                        tracing::info!(
                            endpoint = %qw_config.endpoint,
                            index = %qw_config.index_id,
                            "Quickwit logging enabled"
                        );
                    }
                }
            }

            let shutdown_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let flag = shutdown_flag.clone();

            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                tracing::info!("received shutdown signal");
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            });

            tracing::info!("starting AI Dark Factory orchestrator");
            if let Err(e) = orchestrator.run().await {
                eprintln!("orchestrator error: {e}");
                return ExitCode::from(1);
            }

            ExitCode::SUCCESS
        }
    }
}
