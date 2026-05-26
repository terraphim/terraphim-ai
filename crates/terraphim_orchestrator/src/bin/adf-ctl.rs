//! adf-ctl: CLI control for the AI Dark Factory orchestrator.
//!
//! Triggers agents, queries status, and cancels running agents via SSH+curl
//! to the orchestrator webhook endpoint, or directly in local mode with
//! `--local`. Requires SSH access to bigbox when not using local mode.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use hmac::{Hmac, Mac};
use jiff::Timestamp;
use serde::Serialize;
use sha2::Sha256;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_HOST: &str = "bigbox";
const DEFAULT_ENDPOINT: &str = "http://172.18.0.1:9091/webhooks/gitea";
const DEFAULT_LOCAL_ENDPOINT: &str = "http://127.0.0.1:9091/webhooks/gitea";
const DEFAULT_ORCHESTRATOR_TOML: &str = "/opt/ai-dark-factory/orchestrator.toml";
const DEFAULT_WAIT_TIMEOUT_SECS: u64 = 1200;

#[derive(Parser, Debug)]
#[command(name = "adf-ctl", about = "Control the AI Dark Factory orchestrator")]
struct Cli {
    /// Run commands directly on local machine instead of via SSH
    #[arg(long, global = true)]
    local: bool,

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
        /// Webhook endpoint URL (defaults to remote or local endpoint based on --local)
        #[arg(long)]
        endpoint: Option<String>,
        /// Webhook HMAC secret (default: auto-resolve from env/TOML)
        #[arg(long)]
        secret: Option<String>,
        /// Wait for agent to complete before returning
        #[arg(long, default_value_t = false)]
        wait: bool,
        /// Timeout in seconds when --wait is used
        #[arg(long, default_value_t = DEFAULT_WAIT_TIMEOUT_SECS)]
        timeout: u64,
        /// Dispatch directly via Unix domain socket (local mode only).
        /// Bypasses HTTP webhook and HMAC verification.
        #[arg(long, default_value_t = false)]
        direct: bool,
    },
    /// Show running agents and recent exits [best-effort via SSH or local]
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
    /// Kill a running agent by name [best-effort via SSH pgrep or local]
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
    run(cli.local, cli.command)
}

fn run(local: bool, sub: AdfSub) -> Result<()> {
    match sub {
        AdfSub::Trigger {
            name,
            context,
            host,
            endpoint,
            secret,
            wait,
            timeout,
            direct,
        } => {
            let resolved_endpoint = resolve_endpoint(local, endpoint.as_deref());
            cmd_trigger(
                local,
                &name,
                &context,
                &host,
                &resolved_endpoint,
                secret.as_deref(),
                wait,
                timeout,
                direct,
            )
        }
        AdfSub::Status {
            host,
            since,
            format,
        } => cmd_status(local, &host, &since, format),
        AdfSub::Cancel { name, host } => cmd_cancel(local, &name, &host),
        AdfSub::Agents { host, format } => cmd_agents(local, &host, format),
    }
}

// --- Endpoint resolution ---

/// Resolve the webhook endpoint: explicit arg takes precedence, otherwise
/// uses the local endpoint when `local` is true, or the remote default.
fn resolve_endpoint(local: bool, explicit: Option<&str>) -> String {
    if let Some(ep) = explicit {
        return ep.to_string();
    }
    if local {
        DEFAULT_LOCAL_ENDPOINT.to_string()
    } else {
        DEFAULT_ENDPOINT.to_string()
    }
}

// --- Local config discovery ---

/// Walk up from the current working directory to find `.terraphim/adf.toml`.
/// Returns `None` if no such file exists in any ancestor directory.
fn discover_local_config() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        let candidate = current.join(".terraphim").join("adf.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Parse agent `name = "..."` entries from a TOML file using `strip_prefix`
/// and `strip_suffix` for safe extraction.
fn parse_agent_names_from_toml(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let mut names = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
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

fn resolve_secret(local: bool, explicit: Option<&str>, host: &str) -> Result<String> {
    if let Some(s) = explicit {
        return Ok(s.to_string());
    }
    if let Ok(s) = std::env::var("ADF_WEBHOOK_SECRET") {
        if !s.is_empty() {
            return Ok(s);
        }
    }
    if local {
        // Read secret from local config files
        if let Some(config_path) = discover_local_config() {
            let content = std::fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            for line in content.lines() {
                if let Some(rest) = line.trim().strip_prefix("secret = \"") {
                    if let Some(secret) = rest.strip_suffix('"') {
                        return Ok(secret.to_string());
                    }
                }
            }
        }
        bail!(
            "Could not read webhook secret from local config.\n\
             Set ADF_WEBHOOK_SECRET env var or pass --secret"
        );
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
    let code = output
        .status
        .code()
        .unwrap_or_else(|| ExitStatus::default().code().unwrap_or(-1));
    Ok((stdout, stderr, code))
}

// --- Direct local command runner ---

/// Run a command directly on the local machine (used in `--local` mode).
fn local_run(cmd: &str) -> Result<(String, String, i32)> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .with_context(|| format!("failed to run command: {}", cmd))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output
        .status
        .code()
        .unwrap_or_else(|| ExitStatus::default().code().unwrap_or(-1));
    Ok((stdout, stderr, code))
}

// --- Direct dispatch via Unix domain socket ---

const DEFAULT_SOCKET_PATH: &str = "/tmp/adf-ctl.sock";

/// Resolve the Unix domain socket path for direct dispatch.
///
/// Search order:
/// 1. `ADF_DIRECT_SOCKET` environment variable
/// 2. `socket_path` field in `.terraphim/adf.toml`
/// 3. `ADF_ORCHESTRATOR_TOML` env var / default orchestrator.toml → `direct_dispatch.socket_path`
/// 4. Default: `/tmp/adf-ctl.sock`
fn resolve_socket_path() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("ADF_DIRECT_SOCKET") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    if let Some(config_path) = discover_local_config() {
        if let Some(path) = parse_socket_path_from_toml(&config_path) {
            return Ok(path);
        }
    }
    if let Ok(orch_path) = std::env::var("ADF_ORCHESTRATOR_TOML") {
        if !orch_path.is_empty() {
            if let Some(path) = parse_socket_path_from_toml(Path::new(&orch_path)) {
                return Ok(path);
            }
        }
    }
    let orch_toml = Path::new("/opt/ai-dark-factory/orchestrator.toml");
    if let Some(path) = parse_socket_path_from_toml(orch_toml) {
        return Ok(path);
    }
    Ok(PathBuf::from(DEFAULT_SOCKET_PATH))
}

/// Parse `socket_path` from a TOML file's `[direct_dispatch]` section.
fn parse_socket_path_from_toml(path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    let socket = parsed.get("direct_dispatch")?.get("socket_path")?;
    socket.as_str().map(PathBuf::from)
}

/// Send a dispatch command to the orchestrator via Unix domain socket.
/// Returns `Ok(())` on success, `Err` with descriptive message on failure.
fn direct_dispatch_via_socket(
    socket_path: &Path,
    agent_name: &str,
    context: Option<&str>,
) -> Result<()> {
    let payload = serde_json::json!({
        "agent": agent_name,
        "context": context.filter(|c| !c.is_empty()),
    });

    let mut stream = std::os::unix::net::UnixStream::connect(socket_path)
        .with_context(|| format!("failed to connect to {}", socket_path.display()))?;

    // Send newline-terminated JSON.
    writeln!(stream, "{}", payload.to_string())
        .context("failed to write to direct dispatch socket")?;

    // Read response.
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .context("failed to read from direct dispatch socket")?;

    let response: serde_json::Value = serde_json::from_str(response.trim())
        .with_context(|| format!("invalid JSON from orchestrator: {}", response))?;

    match response.get("status").and_then(|s| s.as_str()) {
        Some("ok") => {
            println!("Agent dispatched via direct socket: {}", agent_name);
            println!("Monitor: journalctl -u adf-orchestrator -f");
            Ok(())
        }
        Some("error") => {
            let msg = response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error");
            bail!("Direct dispatch error: {}", msg);
        }
        _ => {
            bail!("Unexpected direct dispatch response: {}", response);
        }
    }
}

// --- Subcommand implementations ---

#[allow(clippy::too_many_arguments)]
fn cmd_trigger(
    local: bool,
    name: &str,
    context: &str,
    host: &str,
    endpoint: &str,
    secret: Option<&str>,
    wait: bool,
    timeout: u64,
    direct: bool,
) -> Result<()> {
    if direct && !local {
        anyhow::bail!("--direct requires --local");
    }

    if local {
        println!("[local mode]");
    }

    if direct {
        let socket_path = resolve_socket_path()?;
        direct_dispatch_via_socket(&socket_path, name, Some(context))?;
        if wait {
            println!(
                "Waiting for agent '{}' to complete (timeout: {}s)...",
                name, timeout
            );
            wait_for_agent_exit(local, name, host, timeout)?;
        }
        return Ok(());
    }

    let secret = resolve_secret(local, secret, host)?;
    let payload = build_payload(name, context);
    let sig = sign_payload(&secret, payload.as_bytes());

    if local {
        // Direct curl call (no SSH)
        let mut child = Command::new("curl")
            .arg("-s")
            .arg("-o")
            .arg("/dev/null")
            .arg("-w")
            .arg("%{http_code}")
            .arg("-X")
            .arg("POST")
            .arg(endpoint)
            .arg("-H")
            .arg("X-Gitea-Event: issue_comment")
            .arg("-H")
            .arg(format!("X-Gitea-Signature: sha256={}", sig))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("--data-binary")
            .arg("@-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("failed to spawn curl")?;

        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(payload.as_bytes())
            .context("failed to write payload to curl stdin")?;

        let output = child.wait_with_output().context("curl wait failed")?;
        let http_code = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if !stderr.is_empty() {
            eprintln!("curl stderr: {}", stderr);
        }

        match http_code.as_str() {
            "200" | "202" | "204" => {
                println!("Agent dispatched: @adf:{} (HTTP {})", name, http_code);
                println!("Monitor: journalctl -u adf-orchestrator -f");
            }
            "401" => bail!("Webhook authentication failed (check secret)"),
            "400" => bail!("Bad request (HTTP 400) - check payload format"),
            "503" => bail!("Orchestrator unavailable (HTTP 503)"),
            "" => bail!("No HTTP response - is the orchestrator running locally?"),
            code => bail!("Unexpected HTTP status: {}", code),
        }
    } else {
        // SSH-based dispatch
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
    }

    if wait {
        println!(
            "Waiting for agent '{}' to complete (timeout: {}s)...",
            name, timeout
        );
        wait_for_agent_exit(local, name, host, timeout)?;
    }

    Ok(())
}

fn wait_for_agent_exit(local: bool, name: &str, host: &str, timeout_secs: u64) -> Result<()> {
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

        let (stdout, _, _) = if local {
            local_run(&cmd)?
        } else {
            ssh_run(host, &cmd)?
        };

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

fn cmd_status(local: bool, host: &str, since: &str, format: OutputFormat) -> Result<()> {
    if local {
        println!("[local mode]");
    }
    let journal_cmd = format!(
        "journalctl -u adf-orchestrator --since '{} ago' --no-pager 2>/dev/null \
         | grep -E 'exit classified|spawning agent|Agent spawned' | tail -30",
        since
    );
    let (journal_stdout, journal_stderr, _) = if local {
        local_run(&journal_cmd)?
    } else {
        ssh_run(host, &journal_cmd)?
    };
    let journal_output = JournalOutput {
        stdout: journal_stdout,
        stderr: journal_stderr,
    };

    let pgrep_cmd = if local {
        "ps -o pid,etimes,cputime,comm -p $(pgrep -d, -x 'claude|opencode|pi' 2>/dev/null) 2>/dev/null \
         || echo '(no agent CLI processes running)'"
            .to_string()
    } else {
        "ps -o pid,etimes,cputime,comm -p $(pgrep -d, claude 2>/dev/null) 2>/dev/null \
         || echo '(no agent CLI processes running)'"
            .to_string()
    };
    let (pgrep_stdout, _, _) = if local {
        local_run(&pgrep_cmd)?
    } else {
        ssh_run(host, &pgrep_cmd)?
    };

    let activity = parse_journal_activity(&journal_output.stdout);
    let processes = parse_running_processes(&pgrep_stdout);

    match format {
        OutputFormat::Human => {
            if !local {
                println!(
                    "[best-effort via SSH process scan; not authoritative without admin socket]"
                );
            } else {
                println!("[best-effort via local process scan]");
            }
            println!();
            println!("=== Recent agent activity (last {}) ===", since);
            if !journal_output.stderr.is_empty() {
                eprintln!("stderr: {}", journal_output.stderr);
            }
            if journal_output.stdout.trim().is_empty() {
                println!("(no recent activity found)");
            } else {
                print!("{}", journal_output.stdout);
            }
            println!();
            if local {
                println!("=== Running agent CLI processes ===");
            } else {
                println!("=== Running claude processes ===");
            }
            print!("{}", pgrep_stdout);
        }
        OutputFormat::Json => {
            let report = StatusReport {
                host,
                since,
                recent_activity: activity,
                running_processes: processes,
                best_effort: true,
                note: if local {
                    "best-effort via local process scan"
                } else {
                    "best-effort via SSH process scan; not authoritative without admin socket"
                },
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

/// Raw output from a journalctl invocation, used in `cmd_status`.
#[derive(Debug)]
struct JournalOutput {
    stdout: String,
    stderr: String,
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
/// is the `(no agent CLI processes running)` fallback emitted by the shell when
/// `pgrep` matches nothing.
fn parse_running_processes(stdout: &str) -> Vec<ProcessInfo> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("PID") && !l.starts_with("(no agent CLI"))
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

fn cmd_cancel(local: bool, name: &str, host: &str) -> Result<()> {
    if local {
        println!("[local mode]");
    }
    if !local {
        println!("[best-effort via SSH process scan; not authoritative without admin socket]");
    }
    println!("Searching for agent '{}' processes on {}...", name, host);

    let find_cmd = if local {
        format!("ls .worktrees/ 2>/dev/null | grep '^{}-'", name)
    } else {
        format!("ls /tmp/adf-worktrees/ 2>/dev/null | grep '^{}-'", name)
    };
    let (worktrees, _, _) = if local {
        local_run(&find_cmd)?
    } else {
        ssh_run(host, &find_cmd)?
    };

    let pgrep_cmd = if local {
        "pgrep -a -x 'claude|opencode|pi' 2>/dev/null | grep -v defunct".to_string()
    } else {
        "pgrep -a claude 2>/dev/null | grep -v defunct".to_string()
    };
    let (procs, _, _) = if local {
        local_run(&pgrep_cmd)?
    } else {
        ssh_run(host, &pgrep_cmd)?
    };

    if worktrees.trim().is_empty() && procs.trim().is_empty() {
        println!(
            "No active worktrees or agent CLI processes found for '{}'.",
            name
        );
        return Ok(());
    }

    if !worktrees.trim().is_empty() {
        println!("Active worktrees for '{}':", name);
        for wt in worktrees.lines() {
            if local {
                println!("  .worktrees/{}", wt.trim());
            } else {
                println!("  /tmp/adf-worktrees/{}", wt.trim());
            }
        }
        println!();
    }

    if !procs.trim().is_empty() {
        println!("Running agent CLI processes:");
        for line in procs.lines() {
            println!("  {}", line);
        }
        println!();
        if local {
            println!("To kill a specific PID: kill <PID>");
        } else {
            println!("To kill a specific PID: ssh {} kill <PID>", host);
        }
        println!("(Phase 2 admin socket will provide authoritative cancel)");
    }

    Ok(())
}

fn cmd_agents(local: bool, host: &str, format: OutputFormat) -> Result<()> {
    if local {
        println!("[local mode]");
    }
    let agents = if local {
        // Discover local config and parse agent names from it
        let mut names = if let Some(config_path) = discover_local_config() {
            parse_agent_names_from_toml(&config_path)?
        } else {
            Vec::new()
        };
        // Fallback to orchestrator.toml if no local config found or it had no names
        if names.is_empty() {
            let orchestrator_toml = Path::new(DEFAULT_ORCHESTRATOR_TOML);
            if orchestrator_toml.exists() {
                names = parse_agent_names_from_toml(orchestrator_toml)?;
            }
        }
        names.sort();
        names.dedup();
        names
    } else {
        let cmd = "grep '^name = ' /opt/ai-dark-factory/conf.d/*.toml \
                   /opt/ai-dark-factory/orchestrator.toml 2>/dev/null \
                   | awk -F'\"' '{print $2}' | sort -u";
        let (stdout, stderr, code) = ssh_run(host, cmd)?;
        if code != 0 && stdout.trim().is_empty() {
            eprintln!("ssh stderr: {}", stderr);
            bail!("Failed to list agents from {}", host);
        }
        parse_agents_list(&stdout)
    };

    match format {
        OutputFormat::Human => {
            if local {
                println!("Configured agents (local):");
            } else {
                println!("Configured agents on {}:", host);
            }
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
    fn test_parse_running_processes_no_cli_placeholder() {
        let parsed = parse_running_processes("(no agent CLI processes running)\n");
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
        let result = resolve_secret(false, Some("mysecret"), "unused-host-in-unit-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mysecret");

        std::env::set_var("ADF_WEBHOOK_SECRET", "env-secret");
        let result = resolve_secret(false, None, "unused-host-in-unit-test");
        std::env::remove_var("ADF_WEBHOOK_SECRET");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "env-secret");
    }

    #[test]
    fn test_resolve_endpoint_local() {
        // Explicit overrides local
        let ep = resolve_endpoint(true, Some("http://custom:9090/webhook"));
        assert_eq!(ep, "http://custom:9090/webhook");
        // Local without explicit
        let ep = resolve_endpoint(true, None);
        assert_eq!(ep, DEFAULT_LOCAL_ENDPOINT);
    }

    #[test]
    fn test_resolve_endpoint_remote() {
        // Explicit overrides remote
        let ep = resolve_endpoint(false, Some("http://custom:9090/webhook"));
        assert_eq!(ep, "http://custom:9090/webhook");
        // Remote without explicit
        let ep = resolve_endpoint(false, None);
        assert_eq!(ep, DEFAULT_ENDPOINT);
    }

    #[test]
    fn test_parse_agent_names_from_toml_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.toml");
        std::fs::write(
            &path,
            r#"
            [agents]
            name = "meta-learning"
            name = "security-sentinel"
            name = "build-runner"
            "#,
        )
        .unwrap();
        let names = parse_agent_names_from_toml(&path).unwrap();
        assert_eq!(
            names,
            vec!["meta-learning", "security-sentinel", "build-runner"]
        );
    }

    #[test]
    fn test_parse_agent_names_from_toml_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-names.toml");
        std::fs::write(&path, "[other]\nkey = \"value\"\n").unwrap();
        let names = parse_agent_names_from_toml(&path).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_discover_local_config_not_found() {
        // In a typical test run there's unlikely to be .terraphim/adf.toml above
        let result = discover_local_config();
        // We just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_parse_agent_names_from_toml_strip_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("names.toml");
        std::fs::write(
            &path,
            "  name = \"test-agent\"  \n  # comment\n  name = \"other-agent\"\n",
        )
        .unwrap();
        let names = parse_agent_names_from_toml(&path).unwrap();
        assert_eq!(names, vec!["test-agent", "other-agent"]);
    }

    #[test]
    fn test_trigger_direct_requires_local() {
        let result = cmd_trigger(
            false,
            "meta-learning",
            "",
            "localhost",
            "http://localhost:9090/webhook",
            None,
            false,
            60,
            true,
        );
        assert!(result.is_err(), "direct without local should fail");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("--direct requires --local"),
            "error message should mention --direct requires --local: {}",
            err
        );
    }

    #[test]
    fn test_parse_socket_path_from_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestrator.toml");
        std::fs::write(
            &path,
            "[direct_dispatch]\nsocket_path = \"/var/run/adf-ctl.sock\"\n",
        )
        .unwrap();
        let result = super::parse_socket_path_from_toml(&path);
        assert_eq!(
            result,
            Some(std::path::PathBuf::from("/var/run/adf-ctl.sock"))
        );
    }

    #[test]
    fn test_parse_socket_path_from_toml_missing_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestrator.toml");
        std::fs::write(&path, "agents = []\n").unwrap();
        let result = super::parse_socket_path_from_toml(&path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_socket_path_from_toml_missing_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestrator.toml");
        std::fs::write(&path, "[direct_dispatch]\nother_field = \"value\"\n").unwrap();
        let result = super::parse_socket_path_from_toml(&path);
        assert_eq!(result, None);
    }
}
