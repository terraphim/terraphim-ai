// Library modules are imported from the library crate, not re-declared locally.
// This avoids the "multiple different versions of crate" E0308 error.

use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use terraphim_mcp_search::{SkillEntry, mcp_search_skills};
use terraphim_tinyclaw::agent::agent_loop::{HybridLlmRouter, ToolCallingLoop};
use terraphim_tinyclaw::agent::proxy_client::ProxyClientConfig;
use terraphim_tinyclaw::bus::MessageBus;
use terraphim_tinyclaw::channel::{Channel, ChannelManager, build_channels_from_config};
use terraphim_tinyclaw::channels::cli::CliChannel;
use terraphim_tinyclaw::config::Config;
use terraphim_tinyclaw::credentials::{
    CredentialPool, CredentialSource, EnvFileSource, EnvVarSource, PoolEntry, ProviderClass,
    ProviderId,
};
use terraphim_tinyclaw::session::SessionManager;
use terraphim_tinyclaw::skills::{Skill, SkillExecutor};
use terraphim_tinyclaw::tools::{ParityConfig, create_default_registry_with_parity};

/// Routing decision for the session memory backend (#3227 review P1).
///
/// Pure decision function, kept separate from `select_session_backend`
/// so the sqlite gate can be unit-tested without opening a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionBackendChoice {
    /// Default jsonl persistence over the shared `SessionManager`.
    Jsonl,
    /// Opt-in sqlite persistence via `DeviceStorage`.
    Sqlite,
}

/// Decide which session backend to use, applying the
/// `memory.allow_sqlite_backend` safety gate (#3227 review P1).
///
/// The sqlite path persists session state through `DeviceStorage` while
/// session tools still read the jsonl `SessionManager` — a known
/// split-brain. Unless the user explicitly sets
/// `memory.allow_sqlite_backend = true`, a requested
/// `backend = "sqlite"` is rejected here with a prominent warning and
/// routed to jsonl instead of silently splitting session state.
fn choose_session_backend(config: &Config) -> SessionBackendChoice {
    if !config.memory.enabled || config.memory.backend != "sqlite" {
        return SessionBackendChoice::Jsonl;
    }
    if !config.memory.allow_sqlite_backend {
        log::warn!(
            "memory.allow_sqlite_backend is false; sqlite backend requested but disabled \
             (split-brain session state with session tools is unsupported). Falling back to \
             jsonl. Set memory.allow_sqlite_backend = true to enable sqlite."
        );
        return SessionBackendChoice::Jsonl;
    }
    SessionBackendChoice::Sqlite
}

/// Select the session memory backend for the agent loop (#3227, T4).
///
/// `memory.backend = "sqlite"` (with `memory.enabled = true` AND the
/// explicit opt-in `memory.allow_sqlite_backend = true`) routes
/// session persistence through `SqliteBackend` on the shared
/// `DeviceStorage`. Any other value — a disabled sqlite gate, or a
/// `DeviceStorage` initialisation failure — falls back to the default
/// `JsonlBackend` over the shared `SessionManager`, preserving the
/// existing on-disk layout so legacy session files keep loading.
async fn select_session_backend(
    config: &Config,
    sessions: Arc<tokio::sync::Mutex<SessionManager>>,
) -> terraphim_tinyclaw::memory::SharedBackend {
    use terraphim_tinyclaw::memory::jsonl::JsonlBackend;
    use terraphim_tinyclaw::memory::sqlite::SqliteBackend;

    if choose_session_backend(config) == SessionBackendChoice::Sqlite {
        match terraphim_persistence::DeviceStorage::arc_instance().await {
            Ok(storage) => {
                log::info!("Session memory backend: sqlite (DeviceStorage)");
                return Arc::new(SqliteBackend::new(storage, "tinyclaw"));
            }
            Err(e) => {
                log::warn!(
                    "DeviceStorage init failed ({e}); falling back to jsonl session backend"
                );
            }
        }
    }
    Arc::new(JsonlBackend::from_shared(sessions))
}

/// Multi-channel AI assistant powered by Terraphim.
#[derive(Parser, Debug)]
#[command(name = "terraphim-tinyclaw")]
#[command(about = "Multi-channel AI assistant for Telegram, Discord, and CLI")]
#[command(version)]
struct Cli {
    /// Path to configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Enable verbose logging.
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run in interactive CLI mode.
    Agent {
        /// Path to SYSTEM.md file
        #[arg(short, long)]
        system_prompt: Option<PathBuf>,
    },
    /// Run as gateway server with all enabled channels.
    Gateway,
    /// Manage skills (workflows).
    Skill {
        #[command(subcommand)]
        command: SkillCommands,
    },
    /// Manage recurring schedules (Hermes parity, #3147).
    Schedule {
        #[command(subcommand)]
        command: ScheduleCommands,
    },
    /// Start MCP server on stdio (9-tool channel bridge).
    Mcp {
        /// Run in server mode (default).
        #[arg(long, default_value_t = true)]
        serve: bool,
    },
}

#[derive(Subcommand, Debug)]
enum SkillCommands {
    /// Save a skill from a JSON file.
    Save {
        /// Path to the skill JSON file
        path: PathBuf,
    },
    /// Load and display a skill.
    Load {
        /// Name of the skill to load
        name: String,
    },
    /// List all saved skills.
    List,
    /// Run a skill with optional inputs.
    Run {
        /// Name of the skill to run
        name: String,
        /// Input values as key=value pairs (e.g., name=Alice message=hello)
        #[arg(value_name = "INPUTS")]
        inputs: Vec<String>,
    },
    /// Cancel the currently running skill.
    Cancel,
    /// Search saved skills by free-text query.
    ///
    /// Uses Aho-Corasick pattern matching over `name + description + tags`
    /// (the same engine as `mcp_search_skills`). Empty queries return all
    /// skills (sorted by name). Non-empty queries return skills whose
    /// search text contains the query keywords.
    Search {
        /// Free-text search query (whitespace-split into keywords).
        query: String,
        /// Maximum number of results to display. 0 = no limit.
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
}

/// Schedule subcommands (Hermes-parity cron surface, #3147).
#[derive(Subcommand, Debug)]
enum ScheduleCommands {
    /// Create a recurring schedule. Returns the job id.
    Create {
        /// Task prompt to run when the schedule fires.
        prompt: String,
        /// Schedule expression: cron ('0 9 * * *'), 'every 30m',
        /// RFC3339 timestamp, or relative delay ('2h').
        schedule: String,
        /// Skill(s) to inject at job start (repeatable).
        #[arg(long)]
        skill: Vec<String>,
        /// Delivery target (e.g. telegram chat id).
        #[arg(long)]
        deliver: Option<String>,
        /// Model override for the job.
        #[arg(long)]
        model: Option<String>,
    },
    /// List all stored schedules.
    List,
    /// Delete a schedule by id.
    Delete {
        /// Job id (from `schedule list`).
        id: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }

    log::info!("terraphim-tinyclaw starting");

    // Load configuration
    let config_path = cli.config.or_else(Config::default_path);

    let config = match config_path {
        Some(path) if path.exists() => {
            log::info!("Loading configuration from {}", path.display());
            Config::from_file_with_env(&path)?
        }
        Some(path) => {
            log::warn!(
                "Config file not found at {}, using defaults",
                path.display()
            );
            Config::default()
        }
        None => {
            log::warn!("Could not determine config path, using defaults");
            Config::default()
        }
    };

    match cli.command {
        Commands::Agent { system_prompt } => {
            log::info!("Starting in agent mode (CLI)");
            run_agent_mode(config, system_prompt).await?;
        }
        Commands::Gateway => {
            log::info!("Starting in gateway mode");
            run_gateway_mode(config).await?;
        }
        Commands::Skill { command } => {
            log::info!("Executing skill command");
            run_skill_command(command).await?;
        }
        Commands::Schedule { command } => {
            log::info!("Executing schedule command");
            run_schedule_command(command).await?;
        }
        Commands::Mcp { serve } => {
            log::info!("Starting MCP server mode");
            run_mcp_mode(config, serve).await?;
        }
    }

    log::info!("terraphim-tinyclaw shutting down");
    Ok(())
}

async fn run_agent_mode(config: Config, system_prompt_path: Option<PathBuf>) -> anyhow::Result<()> {
    println!("TinyClaw Agent Mode");
    println!("===================");

    // Load system prompt
    let system_prompt = if let Some(path) = system_prompt_path {
        tokio::fs::read_to_string(path)
            .await
            .unwrap_or_else(|_| "You are TinyClaw, a helpful AI assistant.".to_string())
    } else if let Ok(content) = tokio::fs::read_to_string(&config.agent.system_prompt_path()).await
    {
        content
    } else {
        "You are TinyClaw, a helpful AI assistant.".to_string()
    };

    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create session manager (wrapped in Arc<Mutex> for sharing)
    let sessions_dir = config.agent.workspace.join("sessions");
    let sessions = Arc::new(tokio::sync::Mutex::new(SessionManager::new(sessions_dir)));

    // Create tool registry with session manager + Hermes-parity tools
    // (sandbox / subagent / browser / scheduler, each gated by config).
    let web_tools_config = config.tools.web.as_ref();
    let memory_config = if config.memory.enabled {
        Some(&config.memory)
    } else {
        None
    };
    let tools = Arc::new(
        create_default_registry_with_parity(
            Some(sessions.clone()),
            web_tools_config,
            memory_config,
            ParityConfig {
                sandbox: Some(&config.sandbox),
                subagent: Some(&config.subagent),
                browser: Some(&config.browser),
                scheduler: Some(&config.scheduler),
                homeassistant: Some(&config.homeassistant),
                vision: Some(&config.vision),
                image_gen: Some(&config.image_gen),
                tts: Some(&config.tts),
                moa: Some(&config.moa),
                rl: Some(&config.rl),
            },
        )
        .await,
    );

    // Create hybrid LLM router
    let router = build_router(&config)?;

    // Create agent loop
    let backend = select_session_backend(&config, sessions).await;
    let agent = ToolCallingLoop::with_backend(
        &config.agent,
        router,
        tools,
        backend,
        system_prompt,
        memory_config,
    )
    .with_evolution_config(&config.evolution);

    // Spawn agent loop in background
    let bus_clone = bus.clone();
    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run(bus_clone).await {
            log::error!("Agent loop error: {}", e);
        }
    });

    // Create and run CLI channel
    let cli_channel = CliChannel::new();
    cli_channel.start(bus).await?;

    // Shutdown agent when CLI exits
    agent_handle.abort();

    Ok(())
}

async fn run_gateway_mode(config: Config) -> anyhow::Result<()> {
    println!("TinyClaw Gateway Mode");
    println!("=====================");

    // Load system prompt
    let system_prompt =
        if let Ok(content) = tokio::fs::read_to_string(&config.agent.system_prompt_path()).await {
            content
        } else {
            "You are TinyClaw, a helpful AI assistant.".to_string()
        };

    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create session manager (wrapped in Arc<Mutex> for sharing)
    let sessions_dir = config.agent.workspace.join("sessions");
    let sessions = Arc::new(tokio::sync::Mutex::new(SessionManager::new(sessions_dir)));

    // Create tool registry with session manager
    let web_tools_config = config.tools.web.as_ref();
    let memory_config_gw = if config.memory.enabled {
        Some(&config.memory)
    } else {
        None
    };
    let tools = Arc::new(
        create_default_registry_with_parity(
            Some(sessions.clone()),
            web_tools_config,
            memory_config_gw,
            ParityConfig {
                sandbox: Some(&config.sandbox),
                subagent: Some(&config.subagent),
                browser: Some(&config.browser),
                scheduler: Some(&config.scheduler),
                homeassistant: Some(&config.homeassistant),
                vision: Some(&config.vision),
                image_gen: Some(&config.image_gen),
                tts: Some(&config.tts),
                moa: Some(&config.moa),
                rl: Some(&config.rl),
            },
        )
        .await,
    );

    // Create hybrid LLM router
    let router = build_router(&config)?;

    // Create agent loop
    let backend = select_session_backend(&config, sessions).await;
    let agent = ToolCallingLoop::with_backend(
        &config.agent,
        router,
        tools,
        backend,
        system_prompt,
        memory_config_gw,
    )
    .with_evolution_config(&config.evolution);

    // Create channel manager and register enabled channels
    let mut channel_manager = ChannelManager::new();

    // Build channels from config
    let channels = build_channels_from_config(&config.channels)?;
    for channel in channels {
        channel_manager.register(channel);
    }

    // Start all channels
    let bus_clone = bus.clone();
    channel_manager.start_all(bus_clone).await?;

    // Start agent loop
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        if let Err(e) = agent.run(bus_clone).await {
            log::error!("Agent loop error: {}", e);
        }
    });

    // Dispatch outbound messages to channels
    let bus_clone = bus.clone();
    tokio::spawn(async move {
        let mut outbound_rx = bus_clone.outbound_rx.lock().await;
        while let Some(msg) = outbound_rx.recv().await {
            log::debug!("Dispatching outbound to channel: {}", msg.channel);
            if let Err(e) = channel_manager.send(msg).await {
                log::error!("Failed to dispatch outbound message: {}", e);
            }
        }
    });

    // Wait for shutdown signal
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            log::info!("Received shutdown signal");
        }
        Err(err) => {
            log::error!("Error setting up signal handler: {}", err);
        }
    }

    Ok(())
}

/// Run in MCP server mode (9-tool channel bridge over stdio).
async fn run_mcp_mode(config: Config, serve: bool) -> anyhow::Result<()> {
    if !serve {
        anyhow::bail!("MCP client mode is not yet implemented; use --serve");
    }

    if !config.mcp.enabled {
        log::warn!("mcp.enabled = false; MCP server is disabled in config");
        println!("MCP server is disabled. Set mcp.enabled = true in config to enable.");
        return Ok(());
    }

    println!("TinyClaw MCP Server");
    println!("===================");

    // Create message bus
    let bus = Arc::new(MessageBus::new());

    // Create session manager
    let sessions_dir = config.agent.workspace.join("sessions");
    let sessions = Arc::new(tokio::sync::Mutex::new(SessionManager::new(sessions_dir)));

    log::info!("Starting MCP server on stdio");
    terraphim_tinyclaw::mcp::server::serve_mcp_stdio(sessions, bus).await?;

    Ok(())
}

/// Build the hybrid LLM router from configuration.
///
/// When `config.credentials.enabled` is `true` and a `provider_class` is
/// configured, build a `CredentialPool` backed by either an env-file source
/// (`pool_file`) or the process environment. The router will acquire a live
/// token before each proxy request, fall back to the static `proxy.api_key`
/// when the pool is exhausted, and report success/throttle events so the
/// pool can rotate credentials.
fn build_router(config: &Config) -> anyhow::Result<HybridLlmRouter> {
    let proxy_config = ProxyClientConfig {
        base_url: config.llm.proxy.base_url.clone(),
        api_key: config.llm.proxy.api_key.clone(),
        timeout_ms: config.llm.proxy.timeout_ms,
        model: config.llm.proxy.model.clone(),
        retry_after_secs: config.llm.proxy.retry_after_secs,
    };

    if config.credentials.enabled {
        if let Some(class) = config
            .credentials
            .provider_class
            .as_deref()
            .filter(|s| !s.is_empty())
        {
            let source: Arc<dyn CredentialSource> =
                if let Some(path) = &config.credentials.pool_file {
                    Arc::new(EnvFileSource::load(path)?)
                } else {
                    Arc::new(EnvVarSource::new())
                };

            let pool = Arc::new(CredentialPool::with_default_cooldown(
                std::time::Duration::from_secs(config.credentials.cooldown_secs),
            ));

            for entry in &config.credentials.entries {
                pool.add(PoolEntry {
                    provider: ProviderId::from(entry.provider.clone()),
                    class: ProviderClass::from(entry.class.clone()),
                    token_ref: entry.token_ref.clone().into(),
                });
            }

            log::info!(
                "Credential pool enabled for class '{}' with {} entries",
                class,
                pool.len()
            );

            return Ok(HybridLlmRouter::with_credential_pool(
                proxy_config,
                config.llm.direct.clone(),
                pool,
                class,
                Some(source),
            ));
        } else {
            log::warn!(
                "credentials.enabled = true but provider_class is missing or empty; \
                 falling back to static proxy.api_key"
            );
        }
    }

    Ok(HybridLlmRouter::new(
        proxy_config,
        config.llm.direct.clone(),
    ))
}

async fn run_skill_command(command: SkillCommands) -> anyhow::Result<()> {
    let executor = SkillExecutor::with_default_storage()
        .map_err(|e| anyhow::anyhow!("Failed to initialize skill executor: {}", e))?;

    match command {
        SkillCommands::Save { path } => {
            let json = tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read skill file: {}", e))?;

            let skill: Skill = serde_json::from_str(&json)
                .map_err(|e| anyhow::anyhow!("Invalid skill JSON: {}", e))?;

            executor
                .save_skill(&skill)
                .map_err(|e| anyhow::anyhow!("Failed to save skill: {}", e))?;

            println!(
                "✓ Skill '{}' saved successfully (v{})",
                skill.name, skill.version
            );
        }

        SkillCommands::Load { name } => {
            let skill = executor
                .load_skill(&name)
                .map_err(|e| anyhow::anyhow!("Failed to load skill: {}", e))?;

            println!("Skill: {}", skill.name);
            println!("Version: {}", skill.version);
            println!("Description: {}", skill.description);
            if let Some(author) = skill.author {
                println!("Author: {}", author);
            }

            if !skill.inputs.is_empty() {
                println!("\nInputs:");
                for input in &skill.inputs {
                    let req = if input.required {
                        "required"
                    } else {
                        "optional"
                    };
                    let default = input
                        .default
                        .as_ref()
                        .map(|d| format!(" (default: {})", d))
                        .unwrap_or_default();
                    println!(
                        "  - {}: {} [{}]{}",
                        input.name, input.description, req, default
                    );
                }
            }

            println!("\nSteps ({} total):", skill.steps.len());
            for (i, step) in skill.steps.iter().enumerate() {
                let step_type = match step {
                    terraphim_tinyclaw::skills::SkillStep::Tool { tool, .. } => {
                        format!("tool: {}", tool)
                    }
                    terraphim_tinyclaw::skills::SkillStep::Llm { .. } => "llm".to_string(),
                    terraphim_tinyclaw::skills::SkillStep::Shell { .. } => "shell".to_string(),
                    terraphim_tinyclaw::skills::SkillStep::Schedule { cron, .. } => {
                        format!("schedule: {}", cron)
                    }
                };
                println!("  {}. {}", i + 1, step_type);
            }
        }

        SkillCommands::List => {
            let skills = executor
                .list_skills()
                .map_err(|e| anyhow::anyhow!("Failed to list skills: {}", e))?;

            if skills.is_empty() {
                println!("No skills saved. Use 'skill save <file>' to add one.");
            } else {
                println!("Saved skills ({} total):", skills.len());
                for skill in skills {
                    println!(
                        "  • {} (v{}) - {}",
                        skill.name, skill.version, skill.description
                    );
                }
            }
        }

        SkillCommands::Run { name, inputs } => {
            let skill = executor
                .load_skill(&name)
                .map_err(|e| anyhow::anyhow!("Failed to load skill: {}", e))?;

            // Parse inputs
            let mut input_map = HashMap::new();
            for input in inputs {
                if let Some((key, value)) = input.split_once('=') {
                    input_map.insert(key.to_string(), value.to_string());
                } else {
                    eprintln!(
                        "Warning: Invalid input format '{}', expected key=value",
                        input
                    );
                }
            }

            println!("Running skill '{}'...", skill.name);

            let result = executor
                .execute_skill(&skill, input_map, None)
                .await
                .map_err(|e| anyhow::anyhow!("Skill execution failed: {}", e))?;

            println!("\nStatus: {:?}", result.status);
            println!("Duration: {}ms", result.duration_ms);

            if !result.output.is_empty() {
                println!("\nOutput:\n{}", result.output);
            }

            if !result.execution_log.is_empty() {
                println!("\nExecution Log:");
                for log in &result.execution_log {
                    let status = if log.success { "✓" } else { "✗" };
                    println!(
                        "  {} Step {} ({}): {}ms - {}",
                        status,
                        log.step_number + 1,
                        log.step_type,
                        log.duration_ms,
                        log.output.chars().take(50).collect::<String>()
                    );
                }
            }
        }

        SkillCommands::Cancel => {
            executor.cancel();
            println!("Cancellation signal sent.");
        }

        SkillCommands::Search { query, limit } => {
            // Load all saved skills and project each into a SkillEntry for the
            // search engine. SkillEntry is the discovery projection used by
            // mcp_search_skills — name + description + version + author + tags.
            //
            // Steps/inputs are intentionally NOT included in the search index:
            // they're workflow-shape, not discoverability-shape. A future
            // enhancement could add step-content to tags via a `step_indexer`
            // helper, but that's out of scope for the first cut.
            let all_skills = executor
                .list_skills()
                .map_err(|e| anyhow::anyhow!("Failed to list skills: {}", e))?;

            if all_skills.is_empty() {
                println!("No skills saved. Use 'skill save <file>' to add one.");
                return Ok(());
            }

            // Project to SkillEntry. Tags are derived from step types present
            // in the workflow, giving a useful discoverability hook without
            // requiring explicit `tags` field on the Skill struct itself.
            let entries: Vec<SkillEntry> = all_skills
                .iter()
                .map(|s| {
                    let mut tags: Vec<String> = s
                        .steps
                        .iter()
                        .map(|step| match step {
                            terraphim_tinyclaw::skills::SkillStep::Tool { tool, .. } => {
                                format!("tool:{}", tool)
                            }
                            terraphim_tinyclaw::skills::SkillStep::Llm { .. } => "llm".to_string(),
                            terraphim_tinyclaw::skills::SkillStep::Shell { .. } => {
                                "shell".to_string()
                            }
                            terraphim_tinyclaw::skills::SkillStep::Schedule { .. } => {
                                "schedule".to_string()
                            }
                        })
                        .collect();
                    if let Some(author) = &s.author {
                        tags.push(format!("author:{}", author));
                    }
                    SkillEntry {
                        name: s.name.clone(),
                        description: s.description.clone(),
                        version: s.version.clone(),
                        author: s.author.clone(),
                        tags,
                    }
                })
                .collect();

            let hits = mcp_search_skills(&query, &entries);

            if hits.is_empty() {
                println!("No skills match query '{}'.", query);
                return Ok(());
            }

            let total = hits.len();
            let display: Vec<&SkillEntry> = if limit > 0 && limit < hits.len() {
                hits.iter().take(limit).collect()
            } else {
                hits.iter().collect()
            };

            println!(
                "Skills matching '{}' (showing {} of {}):",
                query,
                display.len(),
                total
            );
            for entry in display {
                println!(
                    "  - {} (v{}) - {}{}",
                    entry.name,
                    entry.version,
                    entry.description,
                    if entry.tags.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", entry.tags.join(", "))
                    }
                );
            }
        }
    }

    Ok(())
}

/// Execute a schedule subcommand (Hermes parity cron surface, #3147).
///
/// Persists via `terraphim_persistence::DeviceStorage` (same store type
/// as the dashboard cron CRUD); shares helpers with `ScheduleTool` so the
/// CLI and the agent-loop tool cannot drift.
async fn run_schedule_command(command: ScheduleCommands) -> anyhow::Result<()> {
    use terraphim_tinyclaw::cron::CronStore;
    use terraphim_tinyclaw::tools::scheduler::ScheduleTool;

    let storage = terraphim_persistence::DeviceStorage::arc_instance()
        .await
        .map_err(|e| anyhow::anyhow!("Device storage unavailable: {e}"))?;
    let store = CronStore::new(storage, "tinyclaw_schedules");
    let tool = ScheduleTool::new(store);

    match command {
        ScheduleCommands::Create {
            prompt,
            schedule,
            skill,
            deliver,
            model,
        } => {
            let id = tool
                .create_job(prompt.clone(), &schedule, skill, deliver, model)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("✓ Schedule created: {id}");
            println!("  prompt:   {prompt}");
            println!("  schedule: {schedule}");
        }
        ScheduleCommands::List => {
            let jobs = tool.list_jobs().await.map_err(|e| anyhow::anyhow!("{e}"))?;
            if jobs.is_empty() {
                println!("No schedules. Use 'schedule create' to add one.");
            } else {
                println!("Schedules ({} total):", jobs.len());
                for job in jobs {
                    println!(
                        "  • {} - {} | state={:?} | enabled={} | next={:?}",
                        job.id, job.prompt, job.state, job.enabled, job.next_run_at
                    );
                }
            }
        }
        ScheduleCommands::Delete { id } => {
            let removed = tool
                .delete_job(&id)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            if removed {
                println!("✓ Schedule deleted: {id}");
            } else {
                anyhow::bail!("Schedule '{id}' not found (use 'schedule list' to see ids)");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod backend_gate_tests {
    //! Routing-decision tests for the sqlite safety gate (#3227 review
    //! P1). These exercise `choose_session_backend` only — the pure
    //! decision function — so no sqlite database is ever opened.

    use super::*;

    /// Default config routes to jsonl even when nothing memory-related
    /// is configured.
    #[test]
    fn default_config_routes_to_jsonl() {
        let config = Config::default();
        assert_eq!(choose_session_backend(&config), SessionBackendChoice::Jsonl);
    }

    /// Gate closed: `backend = "sqlite"` + `allow_sqlite_backend = false`
    /// (the default) must fall back to jsonl instead of silently
    /// splitting session state between DeviceStorage and the jsonl
    /// SessionManager.
    #[test]
    fn sqlite_requested_but_gate_closed_falls_back_to_jsonl() {
        let mut config = Config::default();
        config.memory.enabled = true;
        config.memory.backend = "sqlite".to_string();
        // allow_sqlite_backend defaults to false.
        assert!(!config.memory.allow_sqlite_backend);
        assert_eq!(choose_session_backend(&config), SessionBackendChoice::Jsonl);
    }

    /// Gate open: explicit opt-in (`allow_sqlite_backend = true`) with
    /// `backend = "sqlite"` routes to sqlite.
    ///
    /// NOTE: the sqlite path still has the split-brain caveat — the
    /// agent loop persists via DeviceStorage while session tools read
    /// the jsonl SessionManager. The gate only prevents *silent*
    /// default-to-sqlite; enabling it is a deliberate acceptance of
    /// that caveat (#3227 review P1).
    #[test]
    fn sqlite_requested_and_gate_open_routes_to_sqlite() {
        let mut config = Config::default();
        config.memory.enabled = true;
        config.memory.backend = "sqlite".to_string();
        config.memory.allow_sqlite_backend = true;
        assert_eq!(
            choose_session_backend(&config),
            SessionBackendChoice::Sqlite
        );
    }

    /// Memory disabled: sqlite is never selected regardless of flags.
    #[test]
    fn memory_disabled_always_routes_to_jsonl() {
        let mut config = Config::default();
        config.memory.enabled = false;
        config.memory.backend = "sqlite".to_string();
        config.memory.allow_sqlite_backend = true;
        assert_eq!(choose_session_backend(&config), SessionBackendChoice::Jsonl);
    }
}
