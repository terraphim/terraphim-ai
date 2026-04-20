use std::path::PathBuf;
use std::process::ExitCode;

use terraphim_orchestrator::config::OrchestratorConfig;
use terraphim_orchestrator::AgentOrchestrator;
use terraphim_types::capability::{Capability, CostLevel, Latency, Provider, ProviderType};
use tracing_subscriber::EnvFilter;

/// Register LLM providers for keyword-based model routing.
///
/// Only subscription-based providers are registered here (C1 allow-list).
/// These providers are matched against task prompts via KeywordRouter.
/// The selected model_id is passed to the CLI tool via -m/--model flags.
fn register_providers(orchestrator: &mut AgentOrchestrator) {
    let router = orchestrator.router_mut();

    // Implementation and code generation: kimi-for-coding/k2p5
    // Subscription: Moonshot subscription via kimi-for-coding prefix.
    router.add_provider(Provider {
        id: "kimi-for-coding/k2p5".into(),
        name: "Kimi K2.5 (Moonshot subscription)".into(),
        provider_type: ProviderType::Llm {
            model_id: "k2p5".into(),
            api_endpoint: "https://api.moonshot.ai".into(),
        },
        capabilities: vec![Capability::CodeGeneration, Capability::FastThinking],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec![
            "implement".into(),
            "code".into(),
            "generate".into(),
            "write".into(),
            "build".into(),
        ],
    });

    // Explanation and documentation: minimax-coding-plan/MiniMax-M2.5
    // Subscription: MiniMax subscription via minimax-coding-plan prefix.
    router.add_provider(Provider {
        id: "minimax-coding-plan/MiniMax-M2.5".into(),
        name: "MiniMax M2.5 (MiniMax subscription)".into(),
        provider_type: ProviderType::Llm {
            model_id: "MiniMax-M2.5".into(),
            api_endpoint: "https://api.minimax.chat".into(),
        },
        capabilities: vec![Capability::Explanation, Capability::Documentation],
        cost_level: CostLevel::Cheap,
        latency: Latency::Fast,
        keywords: vec![
            "explain".into(),
            "document".into(),
            "summary".into(),
            "quick".into(),
            "fast".into(),
        ],
    });

    // Deep thinking, architecture, security: zai-coding-plan/glm-5-turbo
    // Subscription: ZAI subscription via zai-coding-plan prefix.
    router.add_provider(Provider {
        id: "zai-coding-plan/glm-5-turbo".into(),
        name: "GLM-5-Turbo (ZAI subscription)".into(),
        provider_type: ProviderType::Llm {
            model_id: "glm-5-turbo".into(),
            api_endpoint: "https://open.bigmodel.cn".into(),
        },
        capabilities: vec![
            Capability::DeepThinking,
            Capability::Architecture,
            Capability::SecurityAudit,
        ],
        cost_level: CostLevel::Moderate,
        latency: Latency::Medium,
        keywords: vec![
            "reason".into(),
            "think".into(),
            "architecture".into(),
            "security".into(),
            "vulnerability".into(),
            "CVE".into(),
            "analyze".into(),
            "performance".into(),
            "optimize".into(),
        ],
    });

    tracing::info!("registered 3 subscription LLM providers for keyword routing");
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
                use terraphim_orchestrator::quickwit::{QuickwitFleetSink, QuickwitSink};

                let fleet_configs = orchestrator.quickwit_fleet_configs();
                if !fleet_configs.is_empty() {
                    let mut fleet = QuickwitFleetSink::new_multi();
                    for (project_id, qw_config) in fleet_configs {
                        let sink = QuickwitSink::new(
                            qw_config.endpoint.clone(),
                            qw_config.index_id.clone(),
                            qw_config.batch_size,
                            qw_config.flush_interval_secs,
                        );
                        tracing::info!(
                            project = %project_id,
                            endpoint = %qw_config.endpoint,
                            index = %qw_config.index_id,
                            "Quickwit logging enabled for project"
                        );
                        fleet.insert_project(project_id, sink);
                    }
                    orchestrator.set_quickwit_sink(fleet);
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
