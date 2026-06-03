use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;
use std::str::FromStr;

use crate::config::{AgentDefinition, OrchestratorConfig};
use crate::{
    validate_agent_runtime, AgentRunRequest, AgentRuntimeValidationReport, ModeResult,
    OrchestratorError, SyntheticEvent, TriggerMode,
};
use cron::Schedule;
use serde::Serialize;

const LEGACY_PROJECT: &str = "<legacy>";

fn parse_cron(expr: &str) -> Result<Schedule, OrchestratorError> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    let full_expr = match parts.len() {
        5 => format!("0 {} *", expr),
        6 => format!("{} *", expr),
        7 => expr.to_string(),
        _ => {
            return Err(OrchestratorError::Config(format!(
                "invalid cron '{}': expected 5, 6, or 7 fields, got {}",
                expr,
                parts.len()
            )));
        }
    };
    Schedule::from_str(&full_expr)
        .map_err(|e| OrchestratorError::Config(format!("invalid cron '{}': {}", expr, e)))
}

/// Return the list of trigger modes applicable to the given agent definition.
pub fn applicable_modes(agent: &AgentDefinition) -> Vec<TriggerMode> {
    let mut modes = vec![TriggerMode::Local];
    if agent.schedule.is_some() {
        modes.push(TriggerMode::Cron);
    }
    if agent.event_only {
        modes.push(TriggerMode::PullRequest);
        modes.push(TriggerMode::Push);
    } else {
        modes.push(TriggerMode::Mention);
    }
    modes.push(TriggerMode::Webhook);
    modes
}

/// Return the cron schedule string for the named agent, if configured.
pub fn schedule_for_agent(config: &OrchestratorConfig, agent_name: &str) -> Option<String> {
    config
        .agents
        .iter()
        .find(|a| a.name == agent_name)
        .and_then(|a| a.schedule.clone())
}

/// Return true if the cron expression can be parsed into a valid schedule.
pub fn is_cron_schedule_valid(expr: &str) -> bool {
    parse_cron(expr).is_ok()
}

fn validate_agent_mode(
    _config: &OrchestratorConfig,
    agent: &AgentDefinition,
    mode: TriggerMode,
) -> ModeResult {
    let mut warnings = Vec::new();
    let runnable = match mode {
        TriggerMode::Cron => {
            if let Some(ref expr) = agent.schedule {
                if is_cron_schedule_valid(expr) {
                    true
                } else {
                    warnings.push(format!("invalid cron expression: {}", expr));
                    false
                }
            } else {
                warnings.push("agent has no cron schedule".to_string());
                false
            }
        }
        TriggerMode::PullRequest | TriggerMode::Push => {
            if agent.event_only {
                true
            } else {
                warnings.push("agent is not event-only".to_string());
                false
            }
        }
        TriggerMode::Mention => {
            if agent.event_only {
                warnings.push("event-only agent cannot be mention-dispatched".to_string());
                false
            } else {
                true
            }
        }
        TriggerMode::Local => !agent.cli_tool.trim().is_empty(),
        TriggerMode::Webhook => true,
    };

    let cli_tool_probe = if !agent.cli_tool.trim().is_empty() {
        Some(crate::probe_cli_tool(&agent.cli_tool).unwrap_or(false))
    } else {
        None
    };

    let model_probe = agent
        .model
        .as_ref()
        .map(|m| crate::probe_model_available(m, agent.provider.as_deref()).unwrap_or(false));

    ModeResult {
        trigger_mode: mode,
        runnable,
        cli_tool_probe,
        model_probe,
        synthetic_event_ok: None,
        warnings,
    }
}

/// Validate an agent definition against all applicable trigger modes.
pub fn validate_agent_all_modes(
    config: &OrchestratorConfig,
    agent: &AgentDefinition,
) -> (
    AgentRuntimeValidationReport,
    HashMap<TriggerMode, ModeResult>,
) {
    let request = match &agent.project {
        Some(p) => AgentRunRequest::new(&agent.name).with_project(p),
        None => AgentRunRequest::new(&agent.name),
    };

    let runtime_report =
        validate_agent_runtime(config, &request).unwrap_or_else(|_| AgentRuntimeValidationReport {
            agent_name: agent.name.clone(),
            project: agent
                .project
                .clone()
                .unwrap_or_else(|| LEGACY_PROJECT.to_string()),
            layer: format!("{:?}", agent.layer),
            schedule: agent.schedule.clone(),
            cli_tool: agent.cli_tool.clone(),
            model: agent.model.clone(),
            working_dir: config.working_dir_for_agent(agent).display().to_string(),
            repo_ok: config.working_dir_for_agent(agent).is_dir(),
            gitea_target: None,
            evolution_requested: agent.evolution_enabled,
            evolution_available: config.evolution.enabled && agent.evolution_enabled,
            runnable: false,
            cli_tool_probe: None,
            model_probe: None,
            warnings: vec!["validation failed".to_string()],
        });

    let modes = applicable_modes(agent);
    let mode_results: HashMap<TriggerMode, ModeResult> = modes
        .into_iter()
        .map(|m| (m, validate_agent_mode(config, agent, m)))
        .collect();

    let all_runnable = mode_results.values().all(|r| r.runnable);

    let report = AgentRuntimeValidationReport {
        runnable: runtime_report.runnable && all_runnable,
        ..runtime_report
    };

    (report, mode_results)
}

/// Parsed agent CLI subcommand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSubcommand {
    /// Validate one or all agents from the loaded config.
    Validate {
        /// Optional name of a specific agent to validate.
        agent_name: Option<String>,
        /// Optional project scope for the agent.
        project: Option<String>,
        /// Output format for the validation report.
        format: OutputFormat,
        /// Skip probing whether the model is available.
        skip_model_probe: bool,
    },
    /// Validate all agents from an explicit config file.
    ValidateAll {
        /// Path to the config file to validate.
        config: PathBuf,
        /// Output format for the validation report.
        format: OutputFormat,
        /// Skip probing whether the model is available.
        skip_model_probe: bool,
    },
    /// Run an agent with a synthetic event for local testing.
    RunSynthetic {
        /// Name of the agent to run.
        agent_name: String,
        /// Optional project scope for the agent.
        project: Option<String>,
        /// Synthetic event to inject.
        event: SyntheticEvent,
        /// Output format for the run report.
        format: OutputFormat,
    },
}

/// Output format for validation and run reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable text output.
    Human,
    /// Machine-readable JSON output.
    #[default]
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            _ => Err(format!("unknown format: {s}")),
        }
    }
}

/// Aggregated validation report for all agents in the config.
#[derive(Debug, Clone, Serialize)]
pub struct AgentValidateAllReport {
    /// Per-agent runtime validation reports keyed by agent name.
    pub agents: HashMap<String, AgentRuntimeValidationReport>,
    /// Per-agent mode validation results keyed by agent name.
    pub mode_results: HashMap<String, HashMap<TriggerMode, ModeResult>>,
    /// Total number of agents validated.
    pub total: usize,
    /// Number of agents that are runnable in all modes.
    pub runnable: usize,
    /// Number of agents that failed validation in at least one mode.
    pub failed: usize,
    /// True when every agent is runnable across all modes.
    pub all_modes_runnable: bool,
}

/// Parse CLI arguments into an `AgentSubcommand`.
pub fn parse_agent_args(args: &[String]) -> Result<AgentSubcommand, String> {
    let mut iter = args.iter();
    let mut subcommand: Option<String> = None;
    let mut agent_name: Option<String> = None;
    let mut project: Option<String> = None;
    let mut format: OutputFormat = OutputFormat::default();
    let mut skip_model_probe = false;
    let mut config: Option<PathBuf> = None;
    let mut event: Option<SyntheticEvent> = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "validate" | "validate-all" | "run" => {
                if let Some(prev) = &subcommand {
                    return Err(format!("multiple subcommands: {} and {}", prev, arg));
                }
                subcommand = Some(arg.clone());
            }
            "--project" | "-p" => {
                project = Some(iter.next().ok_or("--project requires a value")?.clone());
            }
            "--format" | "-f" => {
                let val = iter.next().ok_or("--format requires a value")?;
                format = val.parse().map_err(|e: String| e)?;
            }
            "--skip-model-probe" => {
                skip_model_probe = true;
            }
            "--config" | "-c" => {
                config = Some(PathBuf::from(
                    iter.next().ok_or("--config requires a value")?,
                ));
            }
            "--synthetic-event" => {
                let ev = iter
                    .next()
                    .ok_or("--synthetic-event requires an event type")?;
                event = match ev.as_str() {
                    "pr" | "pull_request" => Some(SyntheticEvent::PullRequest {
                        number: 1,
                        head_sha: "HEAD_SHA".to_string(),
                        author: "test".to_string(),
                        title: "Test PR".to_string(),
                        diff_loc: 100,
                    }),
                    "push" => Some(SyntheticEvent::Push {
                        sha: "HEAD_SHA".to_string(),
                        ref_name: "refs/heads/main".to_string(),
                        pusher: "test".to_string(),
                        files: vec![],
                    }),
                    _ => return Err(format!("unknown synthetic event type: {ev}")),
                };
            }
            "--pr" => {
                let num: u64 = iter
                    .next()
                    .ok_or("--pr requires a number")?
                    .parse()
                    .map_err(|e| format!("--pr requires a number: {e}"))?;
                event = Some(SyntheticEvent::PullRequest {
                    number: num,
                    head_sha: "HEAD_SHA".to_string(),
                    author: "test".to_string(),
                    title: "Test PR".to_string(),
                    diff_loc: 100,
                });
            }
            "--push" => {
                event = Some(SyntheticEvent::Push {
                    sha: "HEAD_SHA".to_string(),
                    ref_name: "refs/heads/main".to_string(),
                    pusher: "test".to_string(),
                    files: vec![],
                });
            }
            _ if arg.starts_with("-") => {
                return Err(format!("unknown flag: {arg}"));
            }
            _ => {
                if agent_name.is_some() {
                    return Err(format!("unexpected positional argument: {arg}"));
                }
                agent_name = Some(arg.clone());
            }
        }
    }

    let subcommand = subcommand.unwrap_or("validate".to_string());
    match subcommand.as_str() {
        "validate" => Ok(AgentSubcommand::Validate {
            agent_name,
            project,
            format,
            skip_model_probe,
        }),
        "validate-all" => Ok(AgentSubcommand::ValidateAll {
            config: config.ok_or("--config required for validate-all")?,
            format,
            skip_model_probe,
        }),
        "run" => Ok(AgentSubcommand::RunSynthetic {
            agent_name: agent_name.ok_or("run requires an agent name")?,
            project,
            event: event.ok_or("run requires --synthetic-event")?,
            format,
        }),
        _ => Err(format!("unknown subcommand: {subcommand}")),
    }
}

/// Run the validate subcommand and return an exit code.
pub fn run_validate(
    config: &OrchestratorConfig,
    agent_name: Option<String>,
    project: Option<String>,
    format: OutputFormat,
    _skip_model_probe: bool,
) -> ExitCode {
    if let Some(name) = agent_name {
        let request = match &project {
            Some(p) => AgentRunRequest::new(&name).with_project(p),
            None => AgentRunRequest::new(&name),
        };
        match validate_agent_runtime(config, &request) {
            Ok(report) => {
                print_validation_report(&report, format);
                if report.runnable {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(1)
                }
            }
            Err(e) => {
                eprintln!("validate failed: {e}");
                ExitCode::from(1)
            }
        }
    } else {
        let mut reports: HashMap<String, AgentRuntimeValidationReport> = HashMap::new();
        let mut mode_results: HashMap<String, HashMap<TriggerMode, ModeResult>> = HashMap::new();
        let mut runnable = 0;
        let mut failed = 0;
        for agent in &config.agents {
            let (report, modes) = validate_agent_all_modes(config, agent);
            if report.runnable {
                runnable += 1;
            } else {
                failed += 1;
            }
            reports.insert(agent.name.clone(), report);
            mode_results.insert(agent.name.clone(), modes);
        }
        let all_modes_runnable = failed == 0;
        let all_report = AgentValidateAllReport {
            agents: reports,
            mode_results,
            total: runnable + failed,
            runnable,
            failed,
            all_modes_runnable,
        };
        print_validate_all_report(&all_report, format);
        if !all_modes_runnable {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }
}

/// Run the validate-all subcommand loading config from the given path and return an exit code.
pub fn run_validate_all(
    config: PathBuf,
    format: OutputFormat,
    _skip_model_probe: bool,
) -> ExitCode {
    let config = match OrchestratorConfig::from_file(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::from(1);
        }
    };
    let mut reports: HashMap<String, AgentRuntimeValidationReport> = HashMap::new();
    let mut mode_results: HashMap<String, HashMap<TriggerMode, ModeResult>> = HashMap::new();
    let mut runnable = 0;
    let mut failed = 0;
    for agent in &config.agents {
        let (report, modes) = validate_agent_all_modes(&config, agent);
        if report.runnable {
            runnable += 1;
        } else {
            failed += 1;
        }
        reports.insert(agent.name.clone(), report);
        mode_results.insert(agent.name.clone(), modes);
    }
    let all_modes_runnable = failed == 0;
    let all_report = AgentValidateAllReport {
        agents: reports,
        mode_results,
        total: runnable + failed,
        runnable,
        failed,
        all_modes_runnable,
    };
    print_validate_all_report(&all_report, format);
    if !all_modes_runnable {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

/// Run an agent with a synthetic event and return an exit code.
pub fn run_synthetic(
    _config: &OrchestratorConfig,
    agent_name: &str,
    project: Option<String>,
    _event: SyntheticEvent,
    format: OutputFormat,
) -> ExitCode {
    eprintln!(
        "synthetic run not yet implemented for agent: {}",
        agent_name
    );
    let report = AgentRuntimeValidationReport {
        agent_name: agent_name.to_string(),
        project: project.unwrap_or_else(|| LEGACY_PROJECT.to_string()),
        layer: "unknown".to_string(),
        schedule: None,
        cli_tool: "".to_string(),
        model: None,
        working_dir: ".".to_string(),
        repo_ok: false,
        gitea_target: None,
        evolution_requested: false,
        evolution_available: false,
        runnable: false,
        cli_tool_probe: None,
        model_probe: None,
        warnings: vec!["synthetic run not yet implemented".to_string()],
    };
    print_validation_report(&report, format);
    ExitCode::from(1)
}

fn print_validation_report(report: &AgentRuntimeValidationReport, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report).unwrap());
        }
        OutputFormat::Human => {
            println!("Agent: {}", report.agent_name);
            println!("Project: {}", report.project);
            println!("Layer: {}", report.layer);
            println!("Schedule: {:?}", report.schedule);
            println!("CLI Tool: {}", report.cli_tool);
            println!("Model: {:?}", report.model);
            println!("Working Dir: {}", report.working_dir);
            println!("Repo OK: {}", report.repo_ok);
            println!("CLI Probe: {:?}", report.cli_tool_probe);
            println!("Model Probe: {:?}", report.model_probe);
            println!("Runnable: {}", report.runnable);
            if !report.warnings.is_empty() {
                println!("Warnings:");
                for w in &report.warnings {
                    println!("  - {w}");
                }
            }
        }
    }
}

fn print_validate_all_report(report: &AgentValidateAllReport, format: OutputFormat) {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report).unwrap());
        }
        OutputFormat::Human => {
            println!("Validation Summary:");
            println!("  Total: {}", report.total);
            println!("  Runnable: {}", report.runnable);
            println!("  Failed: {}", report.failed);
            println!("  All Modes Runnable: {}", report.all_modes_runnable);
            println!();
            for (name, r) in &report.agents {
                println!(
                    "  [{}] {} ({})",
                    if r.runnable { "OK" } else { "FAIL" },
                    name,
                    r.project
                );
                if let Some(modes) = report.mode_results.get(name) {
                    for (mode, result) in modes {
                        println!(
                            "    {:?}: runnable={}, cli_probe={:?}, model_probe={:?}",
                            mode, result.runnable, result.cli_tool_probe, result.model_probe
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_validate_no_args() {
        let args = vec!["validate".to_string()];
        let cmd = parse_agent_args(&args).unwrap();
        assert!(matches!(
            cmd,
            AgentSubcommand::Validate {
                agent_name: None,
                ..
            }
        ));
    }

    #[test]
    fn parse_validate_with_agent() {
        let args = vec!["validate".to_string(), "security-sentinel".to_string()];
        let cmd = parse_agent_args(&args).unwrap();
        let AgentSubcommand::Validate { agent_name, .. } = cmd else {
            panic!("expected Validate")
        };
        assert_eq!(agent_name, Some("security-sentinel".to_string()));
    }

    #[test]
    fn parse_validate_all_requires_config() {
        let args = vec!["validate-all".to_string()];
        let err = parse_agent_args(&args).unwrap_err();
        assert!(err.contains("--config required"));
    }

    #[test]
    fn parse_validate_all_with_config() {
        let args = vec![
            "validate-all".to_string(),
            "--config".to_string(),
            "/path/to/config.toml".to_string(),
        ];
        let cmd = parse_agent_args(&args).unwrap();
        let AgentSubcommand::ValidateAll { config, .. } = cmd else {
            panic!("expected ValidateAll")
        };
        assert_eq!(config, PathBuf::from("/path/to/config.toml"));
    }

    #[test]
    fn parse_run_requires_agent_and_event() {
        let args = vec!["run".to_string()];
        let err = parse_agent_args(&args).unwrap_err();
        assert!(err.contains("run requires an agent name"));

        let args = vec!["run".to_string(), "security-sentinel".to_string()];
        let err = parse_agent_args(&args).unwrap_err();
        assert!(err.contains("run requires --synthetic-event"));
    }

    #[test]
    fn parse_run_with_synthetic_pr() {
        let args = vec![
            "run".to_string(),
            "security-sentinel".to_string(),
            "--synthetic-event".to_string(),
            "pr".to_string(),
            "--pr".to_string(),
            "42".to_string(),
        ];
        let cmd = parse_agent_args(&args).unwrap();
        let AgentSubcommand::RunSynthetic {
            agent_name, event, ..
        } = cmd
        else {
            panic!("expected RunSynthetic")
        };
        assert_eq!(agent_name, "security-sentinel");
        assert!(matches!(
            event,
            SyntheticEvent::PullRequest { number: 42, .. }
        ));
    }

    #[test]
    fn parse_run_with_synthetic_push() {
        let args = vec![
            "run".to_string(),
            "pr-reviewer".to_string(),
            "--synthetic-event".to_string(),
            "push".to_string(),
        ];
        let cmd = parse_agent_args(&args).unwrap();
        let AgentSubcommand::RunSynthetic {
            agent_name, event, ..
        } = cmd
        else {
            panic!("expected RunSynthetic")
        };
        assert_eq!(agent_name, "pr-reviewer");
        assert!(matches!(event, SyntheticEvent::Push { .. }));
    }

    #[test]
    fn parse_validate_with_project() {
        let args = vec![
            "validate".to_string(),
            "security-sentinel".to_string(),
            "--project".to_string(),
            "terraphim".to_string(),
        ];
        let cmd = parse_agent_args(&args).unwrap();
        let AgentSubcommand::Validate {
            agent_name,
            project,
            ..
        } = cmd
        else {
            panic!("expected Validate")
        };
        assert_eq!(agent_name, Some("security-sentinel".to_string()));
        assert_eq!(project, Some("terraphim".to_string()));
    }

    #[test]
    fn output_format_parsing() {
        assert!(matches!(
            "human".parse::<OutputFormat>(),
            Ok(OutputFormat::Human)
        ));
        assert!(matches!(
            "json".parse::<OutputFormat>(),
            Ok(OutputFormat::Json)
        ));
        assert!("yaml".parse::<OutputFormat>().is_err());
    }

    fn make_agent(name: &str, schedule: Option<&str>, event_only: bool) -> AgentDefinition {
        AgentDefinition {
            name: name.to_string(),
            layer: AgentLayer::Core,
            cli_tool: "echo".to_string(),
            task: "test".to_string(),
            schedule: schedule.map(String::from),
            model: Some("minimax-coding-plan/MiniMax-M2.5".to_string()),
            default_tier: None,
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
            event_only,
            project: None,
            evolution_enabled: false,
            rlm_enabled: None,
            bypass_kg_routing: false,
            enabled: true,
        }
    }

    #[test]
    fn applicable_modes_cron_agent() {
        let agent = make_agent("security-sentinel", Some("0 */6 * * *"), false);
        let modes = applicable_modes(&agent);
        assert!(modes.contains(&TriggerMode::Cron));
        assert!(modes.contains(&TriggerMode::Local));
        assert!(!modes.contains(&TriggerMode::PullRequest));
        assert!(!modes.contains(&TriggerMode::Push));
    }

    #[test]
    fn applicable_modes_event_only_agent() {
        let agent = make_agent("pr-reviewer", None, true);
        let modes = applicable_modes(&agent);
        assert!(modes.contains(&TriggerMode::PullRequest));
        assert!(modes.contains(&TriggerMode::Push));
        assert!(modes.contains(&TriggerMode::Local));
        assert!(!modes.contains(&TriggerMode::Cron));
    }

    #[test]
    fn is_cron_schedule_valid_valid() {
        assert!(is_cron_schedule_valid("0 */6 * * *"));
        assert!(is_cron_schedule_valid("15 0-10 * * *"));
        assert!(is_cron_schedule_valid("*/30 * * * *"));
    }

    #[test]
    fn is_cron_schedule_valid_invalid() {
        assert!(!is_cron_schedule_valid("not a cron"));
        assert!(!is_cron_schedule_valid(""));
        assert!(!is_cron_schedule_valid("60 0 * * *"));
    }

    use crate::config::{AgentDefinition, AgentLayer};
    use crate::OrchestratorConfig;
    use tempfile::TempDir;

    fn test_config(agents: Vec<AgentDefinition>) -> OrchestratorConfig {
        let tmp = TempDir::new().unwrap();
        OrchestratorConfig {
            working_dir: tmp.path().to_path_buf(),
            nightwatch: Default::default(),
            compound_review: Default::default(),
            workflow: None,
            agents,
            restart_cooldown_secs: 60,
            max_restart_count: 10,
            restart_budget_window_secs: 43_200,
            disk_usage_threshold: 90,
            tick_interval_secs: 30,
            gate_reconcile_interval_ticks: 20,
            handoff_buffer_ttl_secs: None,
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
            projects: vec![],
            include: vec![],
            providers: vec![],
            provider_budget_state_file: None,
            pause_dir: None,
            project_circuit_breaker_threshold: 3,
            fleet_escalation_owner: None,
            fleet_escalation_repo: None,
            post_merge_gate: None,
            learning: Default::default(),
            evolution: Default::default(),
            pr_dispatch: None,
            pr_dispatch_per_project: std::collections::HashMap::new(),
            gitea_skill_repo: None,
            direct_dispatch: None,
        }
    }

    #[test]
    fn schedule_for_agent_finds_schedule() {
        let agent = make_agent("security-sentinel", Some("0 */6 * * *"), false);
        let config = test_config(vec![agent]);
        assert_eq!(
            schedule_for_agent(&config, "security-sentinel"),
            Some("0 */6 * * *".to_string())
        );
    }

    #[test]
    fn schedule_for_agent_not_found() {
        let agent = make_agent("security-sentinel", Some("0 */6 * * *"), false);
        let config = test_config(vec![agent]);
        assert_eq!(schedule_for_agent(&config, "nonexistent"), None);
    }

    #[test]
    fn validate_agent_all_modes_cron_agent() {
        let agent = make_agent("security-sentinel", Some("0 */6 * * *"), false);
        let config = test_config(vec![agent.clone()]);
        let (report, mode_results) = validate_agent_all_modes(&config, &agent);
        assert_eq!(report.agent_name, "security-sentinel");
        assert!(mode_results.contains_key(&TriggerMode::Cron));
        assert!(mode_results.contains_key(&TriggerMode::Local));
        assert!(mode_results.get(&TriggerMode::Cron).unwrap().runnable);
    }

    #[test]
    fn validate_agent_all_modes_event_only_agent() {
        let agent = make_agent("pr-reviewer", None, true);
        let config = test_config(vec![agent.clone()]);
        let (_report, mode_results) = validate_agent_all_modes(&config, &agent);
        assert!(mode_results.contains_key(&TriggerMode::PullRequest));
        assert!(mode_results.contains_key(&TriggerMode::Push));
        assert!(mode_results.contains_key(&TriggerMode::Local));
    }
}
