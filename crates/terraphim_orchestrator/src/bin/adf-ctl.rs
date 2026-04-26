//! adf-ctl: CLI control for the AI Dark Factory orchestrator.
//!
//! Triggers agents, queries status, and cancels running agents via SSH+curl
//! to the orchestrator webhook endpoint. Requires SSH access to bigbox.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use hmac::{Hmac, Mac};
use jiff::Timestamp;
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
        AdfSub::Status { host, since } => cmd_status(&host, &since),
        AdfSub::Cancel { name, host } => cmd_cancel(&name, &host),
        AdfSub::Agents { host } => cmd_agents(&host),
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

fn cmd_status(host: &str, since: &str) -> Result<()> {
    println!("[best-effort via SSH process scan; not authoritative without admin socket]");
    println!();

    println!("=== Recent agent activity (last {}) ===", since);
    let journal_cmd = format!(
        "journalctl -u adf-orchestrator --since '{} ago' --no-pager 2>/dev/null \
         | grep -E 'exit classified|spawning agent|Agent spawned' | tail -30",
        since
    );
    let (stdout, stderr, _) = ssh_run(host, &journal_cmd)?;
    if !stderr.is_empty() {
        eprintln!("ssh stderr: {}", stderr);
    }
    if stdout.trim().is_empty() {
        println!("(no recent activity found)");
    } else {
        print!("{}", stdout);
    }

    println!();
    println!("=== Running claude processes ===");
    let pgrep_cmd = "ps -o pid,etimes,cputime,comm -p $(pgrep -d, claude 2>/dev/null) 2>/dev/null \
                     || echo '(no claude processes running)'";
    let (stdout, _, _) = ssh_run(host, pgrep_cmd)?;
    print!("{}", stdout);

    Ok(())
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

fn cmd_agents(host: &str) -> Result<()> {
    let cmd = "grep '^name = ' /opt/ai-dark-factory/conf.d/*.toml \
               /opt/ai-dark-factory/orchestrator.toml 2>/dev/null \
               | awk -F'\"' '{print $2}' | sort -u";
    let (stdout, stderr, code) = ssh_run(host, cmd)?;
    if code != 0 && stdout.trim().is_empty() {
        eprintln!("ssh stderr: {}", stderr);
        bail!("Failed to list agents from {}", host);
    }
    println!("Configured agents on {}:", host);
    for agent in stdout.lines() {
        let a = agent.trim();
        if !a.is_empty() {
            println!("  {}", a);
        }
    }
    Ok(())
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
    fn test_resolve_secret_explicit() {
        std::env::remove_var("ADF_WEBHOOK_SECRET");
        let result = resolve_secret(Some("mysecret"), "unused-host-in-unit-test");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mysecret");
    }

    #[test]
    fn test_resolve_secret_from_env() {
        std::env::set_var("ADF_WEBHOOK_SECRET", "env-secret");
        let result = resolve_secret(None, "unused-host-in-unit-test");
        std::env::remove_var("ADF_WEBHOOK_SECRET");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "env-secret");
    }
}
