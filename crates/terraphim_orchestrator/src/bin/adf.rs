use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use terraphim_orchestrator::config::{OrchestratorConfig, check_file_permissions};
use terraphim_orchestrator::AgentOrchestrator;
use terraphim_orchestrator::{
    parse_agent_args, run_synthetic, run_validate, run_validate_all, AgentSubcommand,
};
use terraphim_spawner::{AgentSpawner, ResourceLimits, SpawnContext};
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
    Run { config: PathBuf, strict_permissions: bool },
    Check { config: PathBuf, strict_permissions: bool },
    LocalCheck,
    LocalAgent { agent_name: String },
    Agent { sub_args: Vec<String> },
    Version,
    Help,
}

fn parse_args() -> Result<Cli, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut check: Option<PathBuf> = None;
    let mut local_mode: Option<String> = None;
    let mut positional: Option<PathBuf> = None;
    let mut strict_permissions = false;

    let mut iter = args.into_iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--strict-permissions" => {
                strict_permissions = true;
            }
            "--check" => {
                let path = iter
                    .next()
                    .ok_or_else(|| "--check requires a config path".to_string())?;
                check = Some(PathBuf::from(path));
            }
            "--local" => {
                let next = iter.next();
                match next.as_deref() {
                    Some("--check") => {
                        local_mode = Some("--check".to_string());
                    }
                    Some("--agent") => {
                        let agent_name = iter
                            .next()
                            .ok_or_else(|| "--local --agent requires an agent name".to_string())?;
                        local_mode = Some(agent_name.clone());
                    }
                    _ => {
                        return Err(
                            "--local must be followed by --check or --agent (use: adf --local --check or adf --local --agent NAME)"
                                .to_string(),
                        );
                    }
                }
            }
            "-h" | "--help" => return Ok(Cli::Help),
            "-V" | "--version" => return Ok(Cli::Version),
            "agent" => {
                let sub_args: Vec<String> = iter.collect();
                return Ok(Cli::Agent { sub_args });
            }
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

    if local_mode.as_deref() == Some("--check") {
        Ok(Cli::LocalCheck)
    } else if let Some(agent_name) = &local_mode {
        Ok(Cli::LocalAgent {
            agent_name: agent_name.clone(),
        })
    } else if let Some(config) = check {
        Ok(Cli::Check { config, strict_permissions })
    } else {
        let config =
            positional.unwrap_or_else(|| PathBuf::from("/opt/ai-dark-factory/orchestrator.toml"));
        Ok(Cli::Run { config, strict_permissions })
    }
}

fn print_help() {
    println!("adf -- AI Dark Factory orchestrator");
    println!();
    println!("USAGE:");
    println!("    adf [CONFIG]                Run the orchestrator");
    println!("    adf --check CONFIG          Validate config + print agent routing table");
    println!("    adf --local --check         Discover .terraphim/adf.toml and validate");
    println!("    adf --local --agent NAME    Run named agent from .terraphim/adf.toml locally");
    println!("    adf agent validate [NAME]    Validate agent runtime (or all agents)");
    println!("    adf agent validate-all       Validate all agents across trigger modes");
    println!("    adf agent run NAME           Run agent with synthetic event injection");
    println!("    adf --help                  Show this message");
    println!("    adf --version               Show version");
    println!();
    println!("OPTIONS:");
    println!("    --strict-permissions        Exit with error if config file is group/world-readable");
    println!();
    println!("AGENT SUB_COMMANDS:");
    println!("    adf agent validate [NAME] [--project ID] [--format json|human]");
    println!("    adf agent validate-all --config CONFIG [--format json|human]");
    println!("    adf agent run NAME --synthetic-event <pr|push> [--pr N] [--project ID]");
}

/// Run the agent subcommand (validate, validate-all, run).
fn run_agent(sub_args: Vec<String>) -> ExitCode {
    let subcommand = match parse_agent_args(&sub_args) {
        Ok(cmd) => cmd,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    match subcommand {
        AgentSubcommand::Validate {
            agent_name,
            project: _,
            format,
            skip_model_probe,
        } => {
            let adf_config = match terraphim_orchestrator::ProjectAdfConfig::discover_and_load(
                &std::env::current_dir().unwrap_or_default(),
            ) {
                Ok(Some(cfg)) => cfg,
                Ok(None) => {
                    eprintln!(
                        "adf agent validate: no .terraphim/adf.toml found at or above current directory"
                    );
                    return ExitCode::from(1);
                }
                Err(e) => {
                    eprintln!("adf agent validate: failed to load config: {e}");
                    return ExitCode::from(1);
                }
            };

            let project_id = &adf_config.project_id;
            let (project_def, agents) = match (&adf_config).try_into() {
                Ok(tuple) => tuple,
                Err(e) => {
                    eprintln!("adf agent validate: config error: {e}");
                    return ExitCode::from(1);
                }
            };

            let working_dir = adf_config
                .discovered_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

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
                projects: vec![project_def.clone()],
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
                direct_dispatch: None,
            };

            config.substitute_env_vars();

            if let Err(e) = config.validate() {
                eprintln!("adf agent validate: config validation failed: {e}");
                return ExitCode::from(1);
            }

            run_validate(
                &config,
                agent_name,
                Some(project_id.clone()),
                format,
                skip_model_probe,
            )
        }
        AgentSubcommand::ValidateAll {
            config,
            format,
            skip_model_probe,
        } => run_validate_all(config, format, skip_model_probe),
        AgentSubcommand::RunSynthetic {
            agent_name,
            project: _,
            event,
            format,
        } => {
            let adf_config = match terraphim_orchestrator::ProjectAdfConfig::discover_and_load(
                &std::env::current_dir().unwrap_or_default(),
            ) {
                Ok(Some(cfg)) => cfg,
                Ok(None) => {
                    eprintln!(
                        "adf agent run: no .terraphim/adf.toml found at or above current directory"
                    );
                    return ExitCode::from(1);
                }
                Err(e) => {
                    eprintln!("adf agent run: failed to load config: {e}");
                    return ExitCode::from(1);
                }
            };

            let (project_def, agents) = match (&adf_config).try_into() {
                Ok(tuple) => tuple,
                Err(e) => {
                    eprintln!("adf agent run: config error: {e}");
                    return ExitCode::from(1);
                }
            };

            let working_dir = adf_config
                .discovered_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

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
                projects: vec![project_def.clone()],
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
                direct_dispatch: None,
            };

            config.substitute_env_vars();

            if let Err(e) = config.validate() {
                eprintln!("adf agent run: config validation failed: {e}");
                return ExitCode::from(1);
            }

            run_synthetic(
                &config,
                &agent_name,
                Some(project_def.id.clone()),
                event,
                format,
            )
        }
    }
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

    let working_dir = adf_config.project_root();

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
        direct_dispatch: None,
    };

    config.substitute_env_vars();

    if let Err(e) = config.validate() {
        eprintln!("adf --local --check FAILED validation: {e}");
        return ExitCode::from(1);
    }

    print_routing_table(&config);
    ExitCode::SUCCESS
}

/// Build a SpawnContext for a local agent from an OrchestratorConfig and agent definition.
/// Mirrors the logic of build_spawn_context_for_agent but for local (non-daemon) use.
fn build_local_spawn_context(
    config: &OrchestratorConfig,
    def: &terraphim_orchestrator::config::AgentDefinition,
) -> SpawnContext {
    let pid = match def.project.as_deref() {
        Some(p) => p,
        None => return SpawnContext::global(),
    };
    let project = match config.project_by_id(pid) {
        Some(p) => p,
        None => return SpawnContext::global(),
    };
    let working_dir_str = project.working_dir.to_string_lossy().into_owned();
    let mut ctx = SpawnContext::with_working_dir(project.working_dir.clone())
        .with_env("ADF_PROJECT_ID", pid)
        .with_env("ADF_WORKING_DIR", working_dir_str);
    if let Some(gitea) = project.gitea.as_ref() {
        ctx = ctx
            .with_env("GITEA_URL", gitea.base_url.clone())
            .with_env("GITEA_OWNER", gitea.owner.clone())
            .with_env("GITEA_REPO", gitea.repo.clone());
    }
    ctx
}

/// Run a single named agent from .terraphim/adf.toml in the foreground.
/// Streams output to stdout and returns the agent's exit code.
async fn run_local_agent(agent_name: &str, cwd: PathBuf) -> ExitCode {
    let adf_config = match terraphim_orchestrator::ProjectAdfConfig::discover_and_load(&cwd) {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            eprintln!(
                "adf --local --agent {}: no .terraphim/adf.toml found at or above {}",
                agent_name,
                cwd.display()
            );
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("adf --local --agent {} FAILED: {e}", agent_name);
            return ExitCode::from(1);
        }
    };

    let project_id = &adf_config.project_id;
    let (project, agents) = match (&adf_config).try_into() {
        Ok(tuple) => tuple,
        Err(e) => {
            eprintln!("adf --local --agent {} FAILED conversion: {e}", agent_name);
            return ExitCode::from(1);
        }
    };

    let working_dir = adf_config.project_root();

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
        direct_dispatch: None,
    };

    config.substitute_env_vars();

    if let Err(e) = config.validate() {
        eprintln!("adf --local --agent {} FAILED validation: {e}", agent_name);
        return ExitCode::from(1);
    }

    let def = match config
        .agents
        .iter()
        .find(|a| a.name == agent_name && a.project.as_deref() == Some(project_id))
    {
        Some(d) => d.clone(),
        None => {
            eprintln!(
                "adf --local --agent {}: agent not found in project '{}'. \
                 Available agents: {}",
                agent_name,
                project_id,
                config
                    .agents
                    .iter()
                    .filter(|a| a.project.as_deref() == Some(project_id))
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return ExitCode::from(1);
        }
    };

    let ctx = build_local_spawn_context(&config, &def);
    let ctx = terraphim_orchestrator::local_skills::prepare_local_skill_loading(
        &def.cli_tool,
        &config.working_dir,
        ctx,
    );

    let primary_provider = Provider {
        id: def.name.clone(),
        name: def.name.clone(),
        provider_type: terraphim_types::capability::ProviderType::Agent {
            agent_id: def.name.clone(),
            cli_command: def.cli_tool.clone(),
            working_dir: config.working_dir_for_agent(&def),
        },
        capabilities: vec![],
        cost_level: CostLevel::Cheap,
        latency: Latency::Medium,
        keywords: def.capabilities.clone(),
    };

    let fallback_provider = def.fallback_provider.as_ref().map(|fb_cli| Provider {
        id: format!("{}-fallback", def.name),
        name: format!("{} (fallback)", def.name),
        provider_type: terraphim_types::capability::ProviderType::Agent {
            agent_id: format!("{}-fallback", def.name),
            cli_command: fb_cli.clone(),
            working_dir: config.working_dir_for_agent(&def),
        },
        capabilities: vec![],
        cost_level: CostLevel::Cheap,
        latency: Latency::Medium,
        keywords: def.capabilities.clone(),
    });

    let mut limits = ResourceLimits::default();
    if let Some(max_cpu) = def.max_cpu_seconds {
        limits.max_cpu_seconds = Some(max_cpu);
    }
    if let Some(max_mem) = def.max_memory_bytes {
        limits.max_memory_bytes = Some(max_mem);
    }

    let task = def.task.as_str();
    let spawner = AgentSpawner::new();

    let mut handle = match &def.model {
        Some(model) if !model.is_empty() => {
            let req = terraphim_spawner::SpawnRequest::new(primary_provider.clone(), task)
                .with_primary_model(model)
                .with_resource_limits(limits);
            match &fallback_provider {
                Some(fb) => {
                    let fb_model = def.fallback_model.clone();
                    let req = match fb_model {
                        Some(ref fm) => req
                            .with_fallback_provider(fb.clone())
                            .with_fallback_model(fm),
                        None => req.with_fallback_provider(fb.clone()),
                    };
                    match spawner.spawn_with_fallback(&req, ctx.clone()).await {
                        Ok(h) => h,
                        Err(e) => {
                            eprintln!("adf --local --agent {} FAILED to spawn: {e}", agent_name);
                            return ExitCode::from(1);
                        }
                    }
                }
                None => match spawner.spawn_with_fallback(&req, ctx.clone()).await {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("adf --local --agent {} FAILED to spawn: {e}", agent_name);
                        return ExitCode::from(1);
                    }
                },
            }
        }
        _ => {
            let req = terraphim_spawner::SpawnRequest::new(primary_provider.clone(), task)
                .with_resource_limits(limits);
            match fallback_provider {
                Some(ref fb) => {
                    let fb_model = def.fallback_model.clone();
                    let req = match fb_model {
                        Some(ref fm) => req
                            .with_fallback_provider(fb.clone())
                            .with_fallback_model(fm),
                        None => req.with_fallback_provider(fb.clone()),
                    };
                    match spawner.spawn_with_fallback(&req, ctx.clone()).await {
                        Ok(h) => h,
                        Err(e) => {
                            eprintln!("adf --local --agent {} FAILED to spawn: {e}", agent_name);
                            return ExitCode::from(1);
                        }
                    }
                }
                None => match spawner.spawn(&primary_provider, task, ctx.clone()).await {
                    Ok(h) => h,
                    Err(e) => {
                        eprintln!("adf --local --agent {} FAILED to spawn: {e}", agent_name);
                        return ExitCode::from(1);
                    }
                },
            }
        }
    };

    let output_rx = handle.subscribe_output();

    let output_task = tokio::spawn(async move {
        let mut rx = output_rx;
        while let Ok(event) = rx.recv().await {
            match event {
                terraphim_spawner::OutputEvent::Stdout { line, .. } => {
                    println!("{}", line);
                }
                terraphim_spawner::OutputEvent::Stderr { line, .. } => {
                    eprintln!("[stderr] {}", line);
                }
                terraphim_spawner::OutputEvent::Mention {
                    target, message, ..
                } => {
                    eprintln!("[@mention {}] {}", target, message);
                }
                terraphim_spawner::OutputEvent::Completed { .. } => {}
            }
        }
    });

    let exit_code = match handle.wait().await {
        Ok(status) => {
            tracing::info!(agent = %agent_name, status = %status, "agent exited");
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            output_task.abort();
            std::io::stdout().flush().ok();
            match status.code() {
                Some(0) => ExitCode::SUCCESS,
                Some(code) => ExitCode::from(code.min(127) as u8),
                None => ExitCode::from(1),
            }
        }
        Err(e) => {
            tracing::error!(agent = %agent_name, error = %e, "failed to wait on agent");
            output_task.abort();
            ExitCode::from(1)
        }
    };

    exit_code
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
        Cli::LocalAgent { agent_name } => {
            run_local_agent(&agent_name, std::env::current_dir().unwrap_or_default()).await
        }
        Cli::Agent { sub_args } => run_agent(sub_args),
        Cli::Check { config, strict_permissions } => {
            if strict_permissions {
                if let Err(e) = check_file_permissions(&config) {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
            run_check(config)
        }
        Cli::Run { config, strict_permissions } => {
            if strict_permissions {
                if let Err(e) = check_file_permissions(&config) {
                    eprintln!("{e}");
                    return ExitCode::from(1);
                }
            }
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
