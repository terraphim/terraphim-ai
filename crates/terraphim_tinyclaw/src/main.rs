#![allow(dead_code)]
mod agent;
mod bus;
mod channel;
mod channels;
mod config;
mod format;
mod session;
mod skills;
mod tools;

use crate::agent::agent_loop::{HybridLlmRouter, ToolCallingLoop};
use crate::agent::proxy_client::ProxyClientConfig;
use crate::bus::MessageBus;
use crate::channel::{Channel, ChannelManager, build_channels_from_config};
use crate::channels::cli::CliChannel;
use crate::config::Config;
use crate::session::SessionManager;
use crate::skills::{Skill, SkillExecutor};
use crate::tools::create_default_registry;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

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

    // Create tool registry
    let tools = Arc::new(create_default_registry());

    // Create session manager
    let sessions_dir = config.agent.workspace.join("sessions");
    let sessions = SessionManager::new(sessions_dir);

    // Create hybrid LLM router
    let proxy_config = ProxyClientConfig {
        base_url: config.llm.proxy.base_url.clone(),
        api_key: config.llm.proxy.api_key.clone(),
        timeout_ms: config.llm.proxy.timeout_ms,
        model: config.llm.proxy.model.clone(),
        retry_after_secs: config.llm.proxy.retry_after_secs,
    };
    let router = HybridLlmRouter::new(proxy_config, config.llm.direct.clone());

    // Create agent loop
    let agent = ToolCallingLoop::new(&config.agent, router, tools, sessions, system_prompt);

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

    // Create tool registry
    let tools = Arc::new(create_default_registry());

    // Create session manager
    let sessions_dir = config.agent.workspace.join("sessions");
    let sessions = SessionManager::new(sessions_dir);

    // Create hybrid LLM router
    let proxy_config = ProxyClientConfig {
        base_url: config.llm.proxy.base_url.clone(),
        api_key: config.llm.proxy.api_key.clone(),
        timeout_ms: config.llm.proxy.timeout_ms,
        model: config.llm.proxy.model.clone(),
        retry_after_secs: config.llm.proxy.retry_after_secs,
    };
    let router = HybridLlmRouter::new(proxy_config, config.llm.direct.clone());

    // Create agent loop
    let agent = ToolCallingLoop::new(&config.agent, router, tools, sessions, system_prompt);

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

    // Wait for shutdown signal
    match tokio::signal::ctrl_c().await {
        Ok(()) => {
            log::info!("Received shutdown signal");
        }
        Err(err) => {
            log::error!("Error setting up signal handler: {}", err);
        }
    }

    // Graceful shutdown
    channel_manager.stop_all().await?;

    Ok(())
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
                    crate::skills::SkillStep::Tool { tool, .. } => format!("tool: {}", tool),
                    crate::skills::SkillStep::Llm { .. } => "llm".to_string(),
                    crate::skills::SkillStep::Shell { .. } => "shell".to_string(),
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
    }

    Ok(())
}
