use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;
use crate::config::OrchestratorConfig;
use crate::{
    validate_agent_runtime, AgentRunRequest, AgentRuntimeValidationReport,
    SyntheticEvent,
};

const LEGACY_PROJECT: &str = "<legacy>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentSubcommand {
    Validate {
        agent_name: Option<String>,
        project: Option<String>,
        format: OutputFormat,
        skip_model_probe: bool,
    },
    ValidateAll {
        config: PathBuf,
        format: OutputFormat,
        skip_model_probe: bool,
    },
    RunSynthetic {
        agent_name: String,
        project: Option<String>,
        event: SyntheticEvent,
        format: OutputFormat,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    Human,
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

#[derive(Debug, Clone, Serialize)]
pub struct AgentValidateAllReport {
    pub agents: HashMap<String, AgentRuntimeValidationReport>,
    pub total: usize,
    pub runnable: usize,
    pub failed: usize,
}

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
                if subcommand.is_some() {
                    return Err(format!("multiple subcommands: {} and {}", subcommand.unwrap(), arg));
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
                config = Some(PathBuf::from(iter.next().ok_or("--config requires a value")?));
            }
            "--synthetic-event" => {
                let ev = iter.next().ok_or("--synthetic-event requires an event type")?;
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
        let mut runnable = 0;
        let mut failed = 0;
        let mut reports: HashMap<String, AgentRuntimeValidationReport> = HashMap::new();
        for agent in &config.agents {
            let proj = agent.project.clone();
            let request = match &proj {
                Some(p) => AgentRunRequest::new(&agent.name).with_project(p),
                None => AgentRunRequest::new(&agent.name),
            };
            match validate_agent_runtime(config, &request) {
                Ok(report) => {
                    if report.runnable {
                        runnable += 1;
                    } else {
                        failed += 1;
                    }
                    reports.insert(agent.name.clone(), report);
                }
                Err(e) => {
                    eprintln!("validate failed for {}: {e}", agent.name);
                    failed += 1;
                }
            }
        }
        let all_report = AgentValidateAllReport {
            agents: reports,
            total: runnable + failed,
            runnable,
            failed,
        };
        print_validate_all_report(&all_report, format);
        if failed > 0 {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }
}

pub fn run_validate_all(config: PathBuf, format: OutputFormat, _skip_model_probe: bool) -> ExitCode {
    let config = match OrchestratorConfig::from_file(&config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load config: {e}");
            return ExitCode::from(1);
        }
    };
    let mut reports: HashMap<String, AgentRuntimeValidationReport> = HashMap::new();
    let mut runnable = 0;
    let mut failed = 0;
    for agent in &config.agents {
        let proj = agent.project.clone();
        let request = match &proj {
            Some(p) => AgentRunRequest::new(&agent.name).with_project(p),
            None => AgentRunRequest::new(&agent.name),
        };
        match validate_agent_runtime(&config, &request) {
            Ok(report) => {
                if report.runnable {
                    runnable += 1;
                } else {
                    failed += 1;
                }
                reports.insert(agent.name.clone(), report);
            }
            Err(e) => {
                eprintln!("validate failed for {}: {e}", agent.name);
                failed += 1;
            }
        }
    }
    let all_report = AgentValidateAllReport {
        agents: reports,
        total: runnable + failed,
        runnable,
        failed,
    };
    print_validate_all_report(&all_report, format);
    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

pub fn run_synthetic(
    _config: &OrchestratorConfig,
    agent_name: &str,
    project: Option<String>,
    _event: SyntheticEvent,
    format: OutputFormat,
) -> ExitCode {
    eprintln!("synthetic run not yet implemented for agent: {}", agent_name);
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
            println!();
            for (name, r) in &report.agents {
                println!(
                    "  [{}] {} ({})",
                    if r.runnable { "OK" } else { "FAIL" },
                    name,
                    r.project
                );
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
        assert!(matches!(cmd, AgentSubcommand::Validate { agent_name: None, .. }));
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
        let AgentSubcommand::RunSynthetic { agent_name, event, .. } = cmd else {
            panic!("expected RunSynthetic")
        };
        assert_eq!(agent_name, "security-sentinel");
        assert!(matches!(event, SyntheticEvent::PullRequest { number: 42, .. }));
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
        let AgentSubcommand::RunSynthetic { agent_name, event, .. } = cmd else {
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
        let AgentSubcommand::Validate { agent_name, project, .. } = cmd else {
            panic!("expected Validate")
        };
        assert_eq!(agent_name, Some("security-sentinel".to_string()));
        assert_eq!(project, Some("terraphim".to_string()));
    }

    #[test]
    fn output_format_parsing() {
        assert!(matches!("human".parse::<OutputFormat>(), Ok(OutputFormat::Human)));
        assert!(matches!("json".parse::<OutputFormat>(), Ok(OutputFormat::Json)));
        assert!("yaml".parse::<OutputFormat>().is_err());
    }
}
