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
#[cfg(unix)]
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

type HmacSha256 = Hmac<Sha256>;

const DEFAULT_HOST: &str = "bigbox";
const DEFAULT_ENDPOINT: &str = "http://172.18.0.1:9091/webhooks/gitea";
const DEFAULT_LOCAL_ENDPOINT: &str = "http://127.0.0.1:9091/webhooks/gitea";
const DEFAULT_ORCHESTRATOR_TOML: &str = "/opt/ai-dark-factory/orchestrator.toml";
const DEFAULT_WAIT_TIMEOUT_SECS: u64 = 1200;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum DirectEventKind {
    Push,
    Pr,
}

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
        /// Agent or persona name (e.g. meta-learning, security-sentinel).
        /// Use `project/agent` for project-qualified dispatch.
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
        /// Synthetic event type for direct dispatch of event-only agents.
        #[arg(long, value_enum)]
        event: Option<DirectEventKind>,
        /// Git SHA for push events (used with --event push).
        #[arg(long)]
        sha: Option<String>,
        /// Git ref name for push events (e.g. refs/heads/main).
        #[arg(long)]
        ref_name: Option<String>,
        /// PR number for pull_request events (used with --event pr).
        #[arg(long)]
        pr: Option<u64>,
        /// Head SHA for pull_request events.
        #[arg(long)]
        head_sha: Option<String>,
        /// Author login for pull_request events.
        #[arg(long)]
        author: Option<String>,
        /// Title for pull_request events.
        #[arg(long)]
        title: Option<String>,
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
    /// Run a named flow from a local `.terraphim/flows/\<name\>.toml` file
    Flow {
        /// Flow name (e.g. adf-useful-work-proof)
        name: String,
        /// Key=value context passed to the flow (e.g. "issue=1890")
        #[arg(long, default_value = "")]
        context: String,
        /// Path to local orchestrator TOML (reserved for future flow runtime wiring)
        #[arg(long)]
        config: Option<String>,
    },
    /// Show ADF pipeline stage artefact completion for an issue
    PipelineStatus {
        /// Issue number (e.g. 1887)
        issue: String,
        /// Base directory containing per-issue artefact subdirectories
        #[arg(long, default_value = ".docs/adf")]
        base_dir: PathBuf,
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
            event,
            sha,
            ref_name,
            pr,
            head_sha,
            author,
            title,
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
                event,
                sha.as_deref(),
                ref_name.as_deref(),
                pr,
                head_sha.as_deref(),
                author.as_deref(),
                title.as_deref(),
            )
        }
        AdfSub::Status {
            host,
            since,
            format,
        } => cmd_status(local, &host, &since, format),
        AdfSub::Cancel { name, host } => cmd_cancel(local, &name, &host),
        AdfSub::Agents { host, format } => cmd_agents(local, &host, format),
        AdfSub::Flow {
            name,
            context,
            config,
        } => cmd_flow(&name, &context, config.as_deref()),
        AdfSub::PipelineStatus { issue, base_dir } => cmd_pipeline_status(&issue, &base_dir),
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

#[cfg(unix)]
const DEFAULT_SOCKET_PATH: &str = "/tmp/adf-ctl.sock";

#[cfg(unix)]
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

#[cfg(unix)]
fn parse_socket_path_from_toml(path: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(path).ok()?;
    let parsed: toml::Value = toml::from_str(&content).ok()?;
    let socket = parsed.get("direct_dispatch")?.get("socket_path")?;
    socket.as_str().map(PathBuf::from)
}

#[cfg(unix)]
fn direct_dispatch_via_socket(
    socket_path: &Path,
    agent_name: &str,
    project: Option<&str>,
    context: Option<&str>,
    synthetic_event: Option<serde_json::Value>,
) -> Result<()> {
    let mut payload_map = serde_json::Map::new();
    payload_map.insert(
        "agent".to_string(),
        serde_json::Value::String(agent_name.to_string()),
    );
    if let Some(p) = project {
        payload_map.insert(
            "project".to_string(),
            serde_json::Value::String(p.to_string()),
        );
    }
    if let Some(c) = context.filter(|c| !c.is_empty()) {
        payload_map.insert(
            "context".to_string(),
            serde_json::Value::String(c.to_string()),
        );
    }
    if let Some(se) = synthetic_event {
        payload_map.insert("synthetic_event".to_string(), se);
    }
    let payload = serde_json::Value::Object(payload_map);

    let mut stream = std::os::unix::net::UnixStream::connect(socket_path)
        .with_context(|| format!("failed to connect to {}", socket_path.display()))?;

    // Send newline-terminated JSON.
    writeln!(stream, "{payload}").context("failed to write to direct dispatch socket")?;

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

fn split_project_agent(input: &str) -> (Option<String>, String) {
    match input.split_once('/') {
        Some((project, agent)) => (Some(project.to_string()), agent.to_string()),
        None => (None, input.to_string()),
    }
}

fn build_synthetic_event(
    event: Option<DirectEventKind>,
    sha: Option<&str>,
    ref_name: Option<&str>,
    pr: Option<u64>,
    head_sha: Option<&str>,
    author: Option<&str>,
    title: Option<&str>,
) -> Result<Option<serde_json::Value>> {
    let event = match event {
        Some(e) => e,
        None => return Ok(None),
    };

    let value = match event {
        DirectEventKind::Push => {
            let sha = sha.unwrap_or("0000000000000000000000000000000000000000");
            let ref_name = ref_name.unwrap_or("refs/heads/main");
            serde_json::json!({
                "Push": {
                    "sha": sha,
                    "ref_name": ref_name,
                    "pusher": author.unwrap_or("local-user"),
                    "files": <Vec<String>>::new(),
                }
            })
        }
        DirectEventKind::Pr => {
            let number = pr.unwrap_or(0);
            let head_sha = head_sha.unwrap_or("0000000000000000000000000000000000000000");
            let author = author.unwrap_or("local-user");
            let title = title.unwrap_or("Local direct dispatch");
            serde_json::json!({
                "PullRequest": {
                    "number": number,
                    "head_sha": head_sha,
                    "author": author,
                    "title": title,
                    "diff_loc": 0usize,
                }
            })
        }
    };

    Ok(Some(value))
}

fn validate_agent_name_for_shell(name: &str) -> Result<String> {
    if name.is_empty() {
        bail!("agent name cannot be empty");
    }
    if name.len() > 64 {
        bail!("agent name too long (max 64 chars)");
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        bail!(
            "agent name '{}' contains invalid characters (only alphanumeric, '-', '_' allowed)",
            name
        );
    }
    Ok(name.to_string())
}

fn validate_since_for_shell(since: &str) -> Result<String> {
    if since.is_empty() {
        bail!("--since value cannot be empty");
    }
    let mut chars = since.chars();
    match (chars.next(), chars.next_back()) {
        (Some(n), Some(u))
            if n.is_ascii_digit() && "smhdw".contains(u) && chars.all(|c| c.is_ascii_digit()) =>
        {
            Ok(since.to_string())
        }
        _ => {
            bail!(
                "--since '{}' must match ^[0-9]+[smhdw]$ (e.g. 30m, 1h, 2d, 1w)",
                since
            );
        }
    }
}

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
    event: Option<DirectEventKind>,
    sha: Option<&str>,
    ref_name: Option<&str>,
    pr: Option<u64>,
    head_sha: Option<&str>,
    author: Option<&str>,
    title: Option<&str>,
) -> Result<()> {
    if direct && !local {
        anyhow::bail!("--direct requires --local");
    }

    #[cfg(not(unix))]
    if direct {
        anyhow::bail!("--direct dispatch requires Unix (UDS not available on this platform)");
    }

    if local {
        println!("[local mode]");
    }

    #[cfg(unix)]
    if direct {
        let socket_path = resolve_socket_path()?;
        let synthetic_event =
            build_synthetic_event(event, sha, ref_name, pr, head_sha, author, title)?;
        let (project, agent_name) = split_project_agent(name);
        direct_dispatch_via_socket(
            &socket_path,
            &agent_name,
            project.as_deref(),
            Some(context),
            synthetic_event,
        )?;
        if wait {
            println!(
                "Waiting for agent '{}' to complete (timeout: {}s)...",
                name, timeout
            );
            wait_for_agent_exit(local, &agent_name, host, timeout)?;
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
    let validated_name = validate_agent_name_for_shell(name)?;
    let start = std::time::Instant::now();
    let poll_interval = std::time::Duration::from_secs(10);

    loop {
        if start.elapsed().as_secs() >= timeout_secs {
            bail!(
                "Timed out waiting for agent '{}' to complete after {}s",
                validated_name,
                timeout_secs
            );
        }

        let elapsed_secs = start.elapsed().as_secs() + 1;
        let since = format!("{} seconds ago", elapsed_secs);
        let cmd = format!(
            "journalctl -u adf-orchestrator --since '{}' --no-pager 2>/dev/null \
             | grep 'exit classified agent={}'",
            since, validated_name
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
                println!("Agent '{}' completed successfully.", validated_name);
                return Ok(());
            } else {
                bail!(
                    "Agent '{}' exited with non-success status:\n{}",
                    validated_name,
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
    let since = validate_since_for_shell(since)?;
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
                since: &since,
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
    let validated_name = validate_agent_name_for_shell(name)?;
    if local {
        println!("[local mode]");
    }
    if !local {
        println!("[best-effort via SSH process scan; not authoritative without admin socket]");
    }
    println!(
        "Searching for agent '{}' processes on {}...",
        validated_name, host
    );

    let find_cmd = if local {
        format!("ls .worktrees/ 2>/dev/null | grep '^{}-'", validated_name)
    } else {
        format!(
            "ls /tmp/adf-worktrees/ 2>/dev/null | grep '^{}-'",
            validated_name
        )
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
            validated_name
        );
        return Ok(());
    }

    if !worktrees.trim().is_empty() {
        println!("Active worktrees for '{}':", validated_name);
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

/// Discover a flow definition file from `.terraphim/flows/<name>.toml`, walking
/// up from the current directory so nested workspace commands still work.
fn discover_flow_file(cwd: &Path, name: &str) -> Option<PathBuf> {
    let mut current = Some(cwd.to_path_buf());
    while let Some(dir) = current {
        let flow_path = dir
            .join(".terraphim")
            .join("flows")
            .join(format!("{}.toml", name));
        if flow_path.is_file() {
            return Some(flow_path);
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn validate_flow_context_atom(label: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("flow context {} cannot be empty", label);
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        bail!(
            "flow context {} '{}' contains invalid characters (only alphanumeric, '-', '_', '.' allowed)",
            label,
            value
        );
    }
    Ok(())
}

fn parse_context(context: &str) -> Result<std::collections::HashMap<String, String>> {
    let mut map = std::collections::HashMap::new();
    for token in context.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            if !key.is_empty() && !value.is_empty() {
                validate_flow_context_atom("key", key)?;
                validate_flow_context_atom("value", value)?;
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    Ok(map)
}

/// Run a named flow definition locally. The current proof path is intentionally
/// one-slot (`k=1`) via a committed fixture, so it proves executor behaviour
/// without requiring runtime matrix rewriting.
fn cmd_flow(name: &str, context: &str, _config_path: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current working directory")?;

    let flow_path = discover_flow_file(&cwd, name).with_context(|| {
        format!(
            "flow '{}' not found in any .terraphim/flows/ directory from {} upward",
            name,
            cwd.display()
        )
    })?;

    println!("Loading flow from: {}", flow_path.display());

    let flow_content = std::fs::read_to_string(&flow_path)
        .with_context(|| format!("failed to read {}", flow_path.display()))?;

    let flow: terraphim_orchestrator::flow::config::FlowDefinition = toml::from_str(&flow_content)
        .with_context(|| format!("failed to parse flow TOML from {}", flow_path.display()))?;

    println!("Flow '{}' loaded: {} step(s)", flow.name, flow.steps.len());

    let ctx_map = parse_context(context)?;
    let issue = ctx_map.get("issue").cloned();

    let mut state = terraphim_orchestrator::flow::state::FlowRunState::new(&flow.name);
    if let Some(ref issue) = issue {
        println!("Issue context: {}", issue);
        state = state.with_issue(issue.clone());
    }

    let flow_state_dir = cwd.join(".terraphim").join("flow-state");
    std::fs::create_dir_all(&flow_state_dir).with_context(|| {
        format!(
            "failed to create flow state dir {}",
            flow_state_dir.display()
        )
    })?;

    let project_runtime = terraphim_orchestrator::flow::executor::ProjectRuntime {
        working_dir: cwd.clone(),
        gitea_owner: Some("terraphim".to_string()),
        gitea_repo: Some("terraphim-ai".to_string()),
    };

    let executor =
        terraphim_orchestrator::flow::executor::FlowExecutor::new(cwd.clone(), flow_state_dir)
            .with_projects(std::collections::HashMap::from([(
                flow.project.clone(),
                project_runtime,
            )]));

    println!("Running flow '{}'...", flow.name);
    let rt = tokio::runtime::Runtime::new().context("failed to create Tokio runtime")?;

    let final_state = rt
        .block_on(async { executor.run(&flow, Some(state)).await })
        .map_err(|e| anyhow::anyhow!("flow '{}' failed: {}", flow.name, e))?;

    println!();
    println!("Flow '{}' finished: {:?}", flow.name, final_state.status);
    if let Some(ref err) = final_state.error {
        println!("Error: {}", err);
    }
    println!(
        "Steps completed: {}/{}",
        final_state.next_step_index,
        flow.steps.len()
    );
    let matrix_slots: usize = final_state.matrix_envelopes.values().map(Vec::len).sum();
    if matrix_slots > 0 {
        println!("Matrix slots completed: {}", matrix_slots);
    }

    Ok(())
}

/// Canonical ADF disciplined-development stage artefact filenames (in order).
const STAGE_ARTEFACTS: &[&str] = &["research.md", "design.md", "implementation.md", "review.md"];

/// Print ADF pipeline artefact completion summary for a given issue.
///
/// Reads `.docs/adf/<issue>/` (or `base_dir/<issue>/`) and shows each
/// canonical stage artefact with its completion status, line count, and
/// last-modified timestamp.  Returns `Ok(())` when the directory exists
/// (even with partial artefacts); bails with exit code 1 when the directory
/// is missing.
fn cmd_pipeline_status(issue: &str, base_dir: &Path) -> Result<()> {
    let issue_dir = base_dir.join(issue);

    if !issue_dir.exists() {
        bail!(
            "artefact directory '{}' does not exist",
            issue_dir.display()
        );
    }

    println!("Issue: #{}", issue);
    println!("Directory: {}/", issue_dir.display());
    println!();
    println!("Stage Artefacts:");

    let mut complete: usize = 0;
    let total = STAGE_ARTEFACTS.len();

    for &stage in STAGE_ARTEFACTS {
        let path = issue_dir.join(stage);
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let lines = content.lines().count();
            let meta = std::fs::metadata(&path)
                .with_context(|| format!("failed to stat {}", path.display()))?;
            let mtime = meta
                .modified()
                .with_context(|| format!("failed to get mtime for {}", path.display()))?;
            let ts = jiff::Timestamp::try_from(mtime)
                .map(|t| t.to_string())
                .unwrap_or_else(|_| "unknown".to_string());
            println!("  {:<20} | COMPLETE | {:>5} lines | {}", stage, lines, ts);
            complete += 1;
        } else {
            println!("  {:<20} | MISSING  | {:>5}       | -", stage, "-");
        }
    }

    println!();
    println!("Summary: {}/{} artefacts complete", complete, total);

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
    fn test_parse_context_key_values() {
        let parsed = parse_context("issue=1890 k=1").unwrap();
        assert_eq!(parsed.get("issue").map(String::as_str), Some("1890"));
        assert_eq!(parsed.get("k").map(String::as_str), Some("1"));
    }

    #[test]
    fn test_parse_context_rejects_shell_metacharacters() {
        let result = parse_context("issue=1890;rm");
        assert!(result.is_err());
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
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_err(), "direct without local should fail");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("--direct requires --local"),
            "error message should mention --direct requires --local: {}",
            err
        );
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
    #[test]
    fn test_parse_socket_path_from_toml_missing_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestrator.toml");
        std::fs::write(&path, "agents = []\n").unwrap();
        let result = super::parse_socket_path_from_toml(&path);
        assert_eq!(result, None);
    }

    #[cfg(unix)]
    #[test]
    fn test_parse_socket_path_from_toml_missing_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orchestrator.toml");
        std::fs::write(&path, "[direct_dispatch]\nother_field = \"value\"\n").unwrap();
        let result = super::parse_socket_path_from_toml(&path);
        assert_eq!(result, None);
    }

    #[test]
    fn test_split_project_agent_bare() {
        let (project, agent) = super::split_project_agent("meta-learning");
        assert_eq!(project, None);
        assert_eq!(agent, "meta-learning");
    }

    #[test]
    fn test_split_project_agent_qualified() {
        let (project, agent) = super::split_project_agent("terraphim-ai/build-runner");
        assert_eq!(project, Some("terraphim-ai".to_string()));
        assert_eq!(agent, "build-runner");
    }

    #[test]
    fn test_validate_agent_name_for_shell_valid() {
        super::validate_agent_name_for_shell("meta-learning").unwrap();
        super::validate_agent_name_for_shell("build_runner").unwrap();
        super::validate_agent_name_for_shell("agent-123").unwrap();
    }

    #[test]
    fn test_validate_agent_name_for_shell_rejects_slash() {
        let result = super::validate_agent_name_for_shell("project/agent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("invalid characters"),
            "error should mention invalid characters: {}",
            err
        );
    }

    #[test]
    fn test_validate_agent_name_for_shell_rejects_empty() {
        let result = super::validate_agent_name_for_shell("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_agent_name_for_shell_rejects_too_long() {
        let long_name = "a".repeat(65);
        let result = super::validate_agent_name_for_shell(&long_name);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("too long"),
            "error should mention too long: {}",
            err
        );
    }

    #[test]
    fn test_validate_since_for_shell_valid() {
        super::validate_since_for_shell("30m").unwrap();
        super::validate_since_for_shell("1h").unwrap();
        super::validate_since_for_shell("2d").unwrap();
        super::validate_since_for_shell("1w").unwrap();
        super::validate_since_for_shell("10s").unwrap();
    }

    #[test]
    fn test_validate_since_for_shell_rejects_empty() {
        let result = super::validate_since_for_shell("");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("cannot be empty"),
            "error should mention empty: {}",
            err
        );
    }

    #[test]
    fn test_validate_since_for_shell_rejects_now() {
        let result = super::validate_since_for_shell("now");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_since_for_shell_rejects_injection() {
        let result = super::validate_since_for_shell("1h'; rm -rf /");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must match"),
            "error should mention grammar: {}",
            err
        );
    }

    #[test]
    fn test_validate_since_for_shell_rejects_no_unit() {
        let result = super::validate_since_for_shell("30");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("must match"),
            "error should mention grammar: {}",
            err
        );
    }

    #[test]
    fn test_pipeline_status_missing_directory_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();
        let result = super::cmd_pipeline_status("9999", base);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("does not exist"),
            "error should mention missing directory: {}",
            msg
        );
    }

    #[test]
    fn test_pipeline_status_all_artefacts_present() {
        let dir = tempfile::tempdir().unwrap();
        let issue_dir = dir.path().join("42");
        std::fs::create_dir_all(&issue_dir).unwrap();
        for stage in super::STAGE_ARTEFACTS {
            std::fs::write(
                issue_dir.join(stage),
                format!("# {}\n\nContent line.\n", stage),
            )
            .unwrap();
        }
        let result = super::cmd_pipeline_status("42", dir.path());
        assert!(result.is_ok(), "should succeed when all artefacts present");
    }

    #[test]
    fn test_pipeline_status_partial_artefacts() {
        let dir = tempfile::tempdir().unwrap();
        let issue_dir = dir.path().join("100");
        std::fs::create_dir_all(&issue_dir).unwrap();
        // Only write research.md; design.md, implementation.md, review.md are missing
        std::fs::write(issue_dir.join("research.md"), "# Research\n\nDone.\n").unwrap();
        let result = super::cmd_pipeline_status("100", dir.path());
        assert!(result.is_ok(), "should succeed even with partial artefacts");
    }

    #[test]
    fn test_pipeline_status_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let issue_dir = dir.path().join("0");
        std::fs::create_dir_all(&issue_dir).unwrap();
        // No artefacts at all — directory exists but is empty
        let result = super::cmd_pipeline_status("0", dir.path());
        assert!(
            result.is_ok(),
            "should succeed when directory exists with no artefacts"
        );
    }
}
