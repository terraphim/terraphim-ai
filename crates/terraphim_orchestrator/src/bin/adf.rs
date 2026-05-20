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

    // Implementation and code generation: kimi-for-coding/k2p6
    // Subscription: Moonshot subscription via kimi-for-coding prefix.
    router.add_provider(Provider {
        id: "kimi-for-coding/k2p6".into(),
        name: "Kimi K2.6 (Moonshot subscription)".into(),
        provider_type: ProviderType::Llm {
            model_id: "k2p6".into(),
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

    // Deep thinking, architecture, security: zai-coding-plan/glm-5.1
    // Subscription: ZAI subscription via zai-coding-plan prefix.
    router.add_provider(Provider {
        id: "zai-coding-plan/glm-5.1".into(),
        name: "GLM-5.1 (ZAI subscription)".into(),
        provider_type: ProviderType::Llm {
            model_id: "glm-5.1".into(),
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
    LocalCheck,
    Version,
    Help,
}

fn parse_args() -> Result<Cli, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut check: Option<PathBuf> = None;
    let mut local_check = false;
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
            "--local" => {
                if iter.next().as_deref() == Some("--check") {
                    local_check = true;
                } else {
                    return Err(
                        "--local must be followed by --check (use: adf --local --check)"
                            .to_string(),
                    );
                }
            }
            "-h" | "--help" => return Ok(Cli::Help),
            "-V" | "--version" => return Ok(Cli::Version),
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

    if local_check {
        Ok(Cli::LocalCheck)
    } else if let Some(config) = check {
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
    println!("    adf [CONFIG]                Run the orchestrator");
    println!("    adf --check CONFIG          Validate config + print agent routing table");
    println!("    adf --local --check         Discover .terraphim/adf.toml and validate");
    println!("    adf --help                  Show this message");
    println!("    adf --version               Show version");
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

fn run_local_check(cwd: PathBuf) -> ExitCode {
    let adf_config = match terraphim_orchestrator::ProjectAdfConfig::discover_and_load(&cwd) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            eprintln!(
                "adf --local --check: no .terraphim/adf.toml found at or above {}",
                cwd.display()
            );
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("adf --local --check FAILED: {e}");
            return ExitCode::from(1);
        }
    };

    println!(
        "Discovered {}: {} agents from {}",
        adf_config.project_id,
        adf_config.agents.len(),
        adf_config.discovered_path.display()
    );

    let (project, agents) = match (&adf_config).try_into() {
        Ok(tuple) => tuple,
        Err(e) => {
            eprintln!("adf --local --check FAILED conversion: {e}");
            return ExitCode::from(1);
        }
    };

    let working_dir = adf_config
        .discovered_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| cwd.clone());

    let nightwatch = terraphim_orchestrator::config::NightwatchConfig::default();
    let compound_review = terraphim_orchestrator::config::CompoundReviewConfig {
        schedule: "0 2 * * *".to_string(),
        repo_path: working_dir.clone(),
        ..Default::default()
    };

    let mut config = OrchestratorConfig {
        working_dir,
        nightwatch,
        compound_review,
        workflow: None,
        agents,
        restart_cooldown_secs: 60,
        max_restart_count: 10,
        restart_budget_window_secs: 43_200,
        disk_usage_threshold: 90,
        tick_interval_secs: 30,
        gate_reconcile_interval_ticks: 20,
        handoff_buffer_ttl_secs: Some(86400),
        persona_data_dir: None,
        skill_data_dir: None,
        flows: vec![],
        flow_state_dir: None,
        gitea: None,
        mentions: None,
        webhook: None,
        role_config_path: None,
        routing: None,
        #[cfg(feature = "quickwit")]
        quickwit: None,
        projects: vec![project],
        include: vec![],
        providers: vec![],
        provider_budget_state_file: None,
        pause_dir: None,
        project_circuit_breaker_threshold: 5,
        fleet_escalation_owner: None,
        fleet_escalation_repo: None,
        post_merge_gate: None,
        learning: terraphim_orchestrator::config::LearningConfig::default(),
        evolution: terraphim_orchestrator::config::EvolutionConfig::default(),
        pr_dispatch: adf_config.pr_dispatch,
        pr_dispatch_per_project: std::collections::HashMap::new(),
        gitea_skill_repo: None,
    };

    config.substitute_env_vars();

    if let Err(e) = config.validate() {
        eprintln!("adf --local --check FAILED validation: {e}");
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
        Cli::Version => {
            println!("adf {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Cli::LocalCheck => run_local_check(std::env::current_dir().unwrap_or_default()),
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
                        if qw_config.use_es_bulk {
                            use terraphim_orchestrator::quickwit_bulk::QuickwitEsBulkSink;
                            let _sink = QuickwitEsBulkSink::new(
                                qw_config.endpoint.clone(),
                                qw_config.index_id.clone(),
                            );
                            // Wrap ES bulk sink in a fleet-compatible interface
                            // For now, we use the native sink as a fallback
                            tracing::info!(
                                project = %project_id,
                                endpoint = %qw_config.endpoint,
                                index = %qw_config.index_id,
                                "Quickwit ES bulk logging enabled for project"
                            );
                            // Note: Full ES bulk integration requires updating QuickwitFleetSink
                            // to support both sink types. For now, fall back to native.
                            let native_sink = QuickwitSink::new(
                                qw_config.endpoint.clone(),
                                qw_config.index_id.clone(),
                                qw_config.batch_size,
                                qw_config.flush_interval_secs,
                            );
                            fleet.insert_project(project_id, native_sink);
                        } else {
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
