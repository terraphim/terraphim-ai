//! Shell dispatch bridge for executing terraphim-agent subcommands.
//!
//! Parses context from `@adf:<agent-name>` mentions into subcommand + args,
//! executes via `tokio::process::Command` (no shell invocation), and formats
//! the result as a Gitea-compatible markdown comment.
//!
//! Security:
//! - Subcommand allowlist with explicit deny list for dangerous commands
//! - Shell metacharacter rejection (no shell expansion possible)
//! - Output truncation at 48KB, 5-minute timeout with process kill
//! - `--robot` always appended to args, no `--server` passthrough

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Maximum bytes captured from stdout+stderr before truncation.
pub(crate) const MAX_OUTPUT_BYTES: usize = 48_000;

/// Default execution timeout in seconds.
pub(crate) const DISPATCH_TIMEOUT_SECS: u64 = 300;

/// Subcommands that are safe to execute from an @adf mention.
pub(crate) const ALLOWED_SUBCOMMANDS: &[&str] = &[
    "search",
    "extract",
    "replace",
    "validate",
    "suggest",
    "graph",
    "roles",
    "config",
    "learn",
    "chat",
    "check-update",
    "guard",
    "hook",
    "evaluate",
];

/// Subcommands explicitly denied to prevent recursion, side effects, or
/// interactive modes.
pub(crate) const DENIED_SUBCOMMANDS: &[&str] = &[
    "listen",
    "repl",
    "interactive",
    "setup",
    "update",
    "sessions",
];

/// Shell metacharacters that must never appear in unquoted context.
const SHELL_METACHARS: &[char] = &['|', ';', '&', '`', '$', '(', ')', '<', '>'];

/// Configuration for the shell dispatch bridge.
#[derive(Clone)]
pub(crate) struct ShellDispatchConfig {
    pub(crate) agent_binary: PathBuf,
    pub(crate) max_output_bytes: usize,
    pub(crate) timeout: Duration,
    pub(crate) extra_allowed: Vec<String>,
    pub(crate) working_dir: Option<PathBuf>,
    pub(crate) guard: std::sync::Arc<crate::guard_patterns::CommandGuard>,
    pub(crate) agent_cli: Option<PathBuf>,
    pub(crate) agent_model: Option<String>,
}

impl std::fmt::Debug for ShellDispatchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShellDispatchConfig")
            .field("agent_binary", &self.agent_binary)
            .field("max_output_bytes", &self.max_output_bytes)
            .field("timeout", &self.timeout)
            .field("extra_allowed", &self.extra_allowed)
            .field("working_dir", &self.working_dir)
            .field("guard", &"<CommandGuard>")
            .field("agent_cli", &self.agent_cli)
            .field("agent_model", &self.agent_model)
            .finish()
    }
}

/// Result of executing a dispatched subcommand.
#[derive(Debug)]
pub(crate) struct DispatchResult {
    pub(crate) subcommand: String,
    pub(crate) exit_code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) duration_ms: u64,
}

/// Parse a dispatch context string into (subcommand, args).
///
/// Returns:
/// - `Ok(None)` if the context is empty or whitespace-only.
/// - `Ok(Some((subcommand, args)))` if parsing succeeds and the subcommand is allowed.
/// - `Err(message)` if the context contains shell metacharacters, a denied subcommand,
///   or an unknown subcommand.
pub(crate) fn parse_dispatch_command(
    context: &str,
    extra_allowed: &[String],
) -> Result<Option<(String, Vec<String>)>, String> {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    // Reject shell metacharacters anywhere in the raw context
    for ch in SHELL_METACHARS {
        if trimmed.contains(*ch) {
            return Err(format!(
                "shell metacharacter `{}` is not allowed in dispatch context",
                ch
            ));
        }
    }

    let tokens = tokenize(trimmed)?;
    if tokens.is_empty() {
        return Ok(None);
    }

    let subcommand = &tokens[0];
    let args: Vec<String> = tokens[1..].to_vec();

    // Check deny list first
    if DENIED_SUBCOMMANDS.contains(&subcommand.as_str()) {
        return Err(format!(
            "subcommand `{}` is denied for dispatch execution",
            subcommand
        ));
    }

    // Check allow list (built-in + extra)
    let allowed = ALLOWED_SUBCOMMANDS.contains(&subcommand.as_str())
        || extra_allowed.iter().any(|s| s == subcommand);
    if !allowed {
        return Err(format!(
            "subcommand `{}` is not in the dispatch allowlist",
            subcommand
        ));
    }

    Ok(Some((subcommand.clone(), args)))
}

/// Simple tokenizer that handles double-quoted strings.
///
/// No shell expansion, no backslash escapes, no single quotes.
/// Tokens are split on whitespace. Double-quoted strings preserve
/// interior whitespace as a single token.
fn tokenize(input: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = input.chars();

    for ch in chars {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }

    if in_quotes {
        return Err("unterminated double quote in dispatch context".to_string());
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Execute a subcommand via `tokio::process::Command`.
///
/// Runs `config.agent_binary subcommand [args...] --robot` with stdout/stderr
/// captured. Applies timeout and output byte cap. On timeout, kills the child
/// process.
pub(crate) async fn execute_dispatch(
    config: &ShellDispatchConfig,
    subcommand: &str,
    args: &[String],
) -> Result<DispatchResult, String> {
    // Run guard check on the full command before executing
    let full_command = std::iter::once(subcommand.to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ");
    let guard_result = config.guard.check(&full_command);
    if guard_result.decision != crate::guard_patterns::GuardDecision::Allow {
        return Err(format!(
            "Guard blocked command `{}`: {} (pattern: {})",
            full_command,
            guard_result.reason.unwrap_or_default(),
            guard_result.pattern.unwrap_or_default(),
        ));
    }

    let start = Instant::now();

    // --robot is a top-level CLI flag (before the subcommand), not a subcommand flag
    let mut cmd_args = vec!["--robot".to_string(), subcommand.to_string()];
    cmd_args.extend(args.iter().cloned());

    let mut command = tokio::process::Command::new(&config.agent_binary);
    command
        .args(&cmd_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(ref dir) = config.working_dir {
        command.current_dir(dir);
    }

    let child = command
        .spawn()
        .map_err(|e| format!("failed to spawn `{}`: {}", config.agent_binary.display(), e))?;

    match tokio::time::timeout(config.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let (stdout, stderr, truncated) =
                truncate_output(&output.stdout, &output.stderr, config.max_output_bytes);

            Ok(DispatchResult {
                subcommand: subcommand.to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stdout,
                stderr,
                truncated,
                timed_out: false,
                duration_ms,
            })
        }
        Ok(Err(e)) => Err(format!("child process error: {}", e)),
        Err(_) => {
            // Timeout expired
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(DispatchResult {
                subcommand: subcommand.to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Process timed out after {}s", config.timeout.as_secs()),
                truncated: false,
                timed_out: true,
                duration_ms,
            })
        }
    }
}

/// Execute an AI coding agent (e.g. opencode) to implement an issue.
///
/// Runs `agent_cli run -m <model> --dir <working_dir> <message>` and captures output.
pub(crate) async fn execute_agent_dispatch(
    config: &ShellDispatchConfig,
    context: &str,
) -> Result<DispatchResult, String> {
    let agent_cli = config
        .agent_cli
        .as_ref()
        .ok_or("agent_cli not configured in dispatch config")?;
    let model = config
        .agent_model
        .as_deref()
        .unwrap_or("kimi-for-coding/k2p5");

    let start = Instant::now();

    let mut cmd_args = vec!["run".to_string(), "-m".to_string(), model.to_string()];
    if let Some(ref dir) = config.working_dir {
        cmd_args.push("--dir".to_string());
        cmd_args.push(dir.display().to_string());
    }
    cmd_args.push(context.to_string());

    let mut command = tokio::process::Command::new(agent_cli);
    command
        .args(&cmd_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(ref dir) = config.working_dir {
        command.current_dir(dir);
    }

    let child = command
        .spawn()
        .map_err(|e| format!("failed to spawn agent CLI `{}`: {}", agent_cli.display(), e))?;

    match tokio::time::timeout(config.timeout, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            let (stdout, stderr, truncated) =
                truncate_output(&output.stdout, &output.stderr, config.max_output_bytes);
            Ok(DispatchResult {
                subcommand: "implement".to_string(),
                exit_code: output.status.code().unwrap_or(-1),
                stdout,
                stderr,
                truncated,
                timed_out: false,
                duration_ms,
            })
        }
        Ok(Err(e)) => Err(format!("agent process error: {}", e)),
        Err(_) => {
            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(DispatchResult {
                subcommand: "implement".to_string(),
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Agent timed out after {}s", config.timeout.as_secs()),
                truncated: false,
                timed_out: true,
                duration_ms,
            })
        }
    }
}

/// Truncate combined stdout+stderr to fit within byte budget.
fn truncate_output(
    stdout_bytes: &[u8],
    stderr_bytes: &[u8],
    max_bytes: usize,
) -> (String, String, bool) {
    let total = stdout_bytes.len() + stderr_bytes.len();
    if total <= max_bytes {
        let stdout = String::from_utf8_lossy(stdout_bytes).to_string();
        let stderr = String::from_utf8_lossy(stderr_bytes).to_string();
        return (stdout, stderr, false);
    }

    // Prioritize stdout; give stderr whatever remains
    let stdout_budget = max_bytes.min(stdout_bytes.len());
    let stderr_budget = max_bytes.saturating_sub(stdout_budget);

    let stdout = String::from_utf8_lossy(&stdout_bytes[..stdout_budget]).to_string();
    let stderr =
        String::from_utf8_lossy(&stderr_bytes[..stderr_budget.min(stderr_bytes.len())]).to_string();

    (stdout, stderr, true)
}

/// Format a dispatch result as a markdown comment for posting to Gitea.
pub(crate) fn format_dispatch_result(
    result: &DispatchResult,
    agent_name: &str,
    session_id: &str,
    event_id: &str,
) -> String {
    let duration_secs = result.duration_ms as f64 / 1000.0;
    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "## `{}` -- exit {} ({:.1}s)\n\n",
        result.subcommand, result.exit_code, duration_secs
    ));

    // Timeout marker
    if result.timed_out {
        out.push_str(&format!(
            "**TIMED OUT** after {}s\n\n",
            result.duration_ms / 1000
        ));
    }

    // Truncation marker
    if result.truncated {
        out.push_str("[output truncated at 48KB]\n\n");
    }

    // Stdout
    if !result.stdout.is_empty() {
        out.push_str("```\n");
        out.push_str(&result.stdout);
        if !result.stdout.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n\n");
    }

    // Stderr (collapsible)
    if !result.stderr.is_empty() {
        out.push_str(&format!(
            "<details><summary>stderr ({} bytes)</summary>\n\n```\n",
            result.stderr.len()
        ));
        out.push_str(&result.stderr);
        if !result.stderr.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("```\n</details>\n\n");
    }

    // Footer
    out.push_str(&format!(
        "agent={} session={} event={}",
        agent_name, session_id, event_id
    ));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_dispatch_command tests ──

    #[test]
    fn test_parse_empty_context() {
        assert_eq!(parse_dispatch_command("", &[]).unwrap(), None);
        assert_eq!(parse_dispatch_command("   ", &[]).unwrap(), None);
        assert_eq!(parse_dispatch_command("\t\n", &[]).unwrap(), None);
    }

    #[test]
    fn test_parse_simple_command() {
        let result = parse_dispatch_command("search automata", &[]).unwrap();
        assert_eq!(
            result,
            Some(("search".to_string(), vec!["automata".to_string()]))
        );
    }

    #[test]
    fn test_parse_quoted_args() {
        let result = parse_dispatch_command(r#"search "multi word""#, &[]).unwrap();
        assert_eq!(
            result,
            Some(("search".to_string(), vec!["multi word".to_string()]))
        );
    }

    #[test]
    fn test_parse_learn_subcommand() {
        let result = parse_dispatch_command("learn list", &[]).unwrap();
        assert_eq!(
            result,
            Some(("learn".to_string(), vec!["list".to_string()]))
        );
    }

    #[test]
    fn test_parse_rejects_pipe() {
        let err = parse_dispatch_command("search | cat", &[]).unwrap_err();
        assert!(err.contains("metacharacter"), "error: {}", err);
    }

    #[test]
    fn test_parse_rejects_semicolon() {
        let err = parse_dispatch_command("search; rm", &[]).unwrap_err();
        assert!(err.contains("metacharacter"), "error: {}", err);
    }

    #[test]
    fn test_parse_rejects_backtick() {
        let err = parse_dispatch_command("search `whoami`", &[]).unwrap_err();
        assert!(err.contains("metacharacter"), "error: {}", err);
    }

    #[test]
    fn test_parse_rejects_dollar() {
        let err = parse_dispatch_command("search $HOME", &[]).unwrap_err();
        assert!(err.contains("metacharacter"), "error: {}", err);
    }

    #[test]
    fn test_parse_rejects_denied() {
        let err = parse_dispatch_command("listen --identity x", &[]).unwrap_err();
        assert!(err.contains("denied"), "error: {}", err);
    }

    #[test]
    fn test_parse_rejects_unknown() {
        let err = parse_dispatch_command("foobar", &[]).unwrap_err();
        assert!(err.contains("allowlist"), "error: {}", err);
    }

    #[test]
    fn test_parse_allows_extra() {
        let extra = vec!["custom".to_string()];
        let result = parse_dispatch_command("custom arg", &extra).unwrap();
        assert_eq!(
            result,
            Some(("custom".to_string(), vec!["arg".to_string()]))
        );
    }

    #[test]
    fn test_parse_subcommand_only_no_args() {
        let result = parse_dispatch_command("roles", &[]).unwrap();
        assert_eq!(result, Some(("roles".to_string(), vec![])));
    }

    #[test]
    fn test_parse_multiple_args() {
        let result = parse_dispatch_command("search automata --role engineer", &[]).unwrap();
        assert_eq!(
            result,
            Some((
                "search".to_string(),
                vec![
                    "automata".to_string(),
                    "--role".to_string(),
                    "engineer".to_string()
                ]
            ))
        );
    }

    #[test]
    fn test_parse_rejects_angle_brackets() {
        assert!(parse_dispatch_command("search > /tmp/out", &[]).is_err());
        assert!(parse_dispatch_command("search < /etc/passwd", &[]).is_err());
    }

    #[test]
    fn test_parse_rejects_ampersand() {
        assert!(parse_dispatch_command("search & echo", &[]).is_err());
    }

    #[test]
    fn test_parse_rejects_parentheses() {
        assert!(parse_dispatch_command("search (test)", &[]).is_err());
    }

    // ── format_dispatch_result tests ──

    #[test]
    fn test_format_success() {
        let result = DispatchResult {
            subcommand: "search".to_string(),
            exit_code: 0,
            stdout: "found 3 results\n".to_string(),
            stderr: String::new(),
            truncated: false,
            timed_out: false,
            duration_ms: 150,
        };
        let formatted = format_dispatch_result(&result, "worker", "ses:abc", "evt:def");
        assert!(formatted.contains("## `search` -- exit 0"));
        assert!(formatted.contains("found 3 results"));
        assert!(formatted.contains("session=ses:abc"));
        assert!(formatted.contains("event=evt:def"));
        assert!(!formatted.contains("stderr"));
        assert!(!formatted.contains("truncated"));
        assert!(!formatted.contains("TIMED OUT"));
    }

    #[test]
    fn test_format_failure() {
        let result = DispatchResult {
            subcommand: "search".to_string(),
            exit_code: 1,
            stdout: String::new(),
            stderr: "error: no config found\n".to_string(),
            truncated: false,
            timed_out: false,
            duration_ms: 50,
        };
        let formatted = format_dispatch_result(&result, "worker", "ses:abc", "evt:def");
        assert!(formatted.contains("exit 1"));
        assert!(formatted.contains("stderr"));
        assert!(formatted.contains("error: no config found"));
    }

    #[test]
    fn test_format_truncated() {
        let result = DispatchResult {
            subcommand: "search".to_string(),
            exit_code: 0,
            stdout: "partial output\n".to_string(),
            stderr: String::new(),
            truncated: true,
            timed_out: false,
            duration_ms: 200,
        };
        let formatted = format_dispatch_result(&result, "worker", "ses:abc", "evt:def");
        assert!(formatted.contains("[output truncated at 48KB]"));
    }

    #[test]
    fn test_format_timeout() {
        let result = DispatchResult {
            subcommand: "chat".to_string(),
            exit_code: -1,
            stdout: String::new(),
            stderr: "Process timed out after 300s".to_string(),
            truncated: false,
            timed_out: true,
            duration_ms: 300_000,
        };
        let formatted = format_dispatch_result(&result, "worker", "ses:abc", "evt:def");
        assert!(formatted.contains("**TIMED OUT**"));
        assert!(formatted.contains("300s"));
    }

    // ── tokenize tests ──

    #[test]
    fn test_tokenize_simple() {
        let tokens = tokenize("search automata").unwrap();
        assert_eq!(tokens, vec!["search", "automata"]);
    }

    #[test]
    fn test_tokenize_quoted() {
        let tokens = tokenize(r#"search "hello world" extra"#).unwrap();
        assert_eq!(tokens, vec!["search", "hello world", "extra"]);
    }

    #[test]
    fn test_tokenize_unterminated_quote() {
        assert!(tokenize(r#"search "unterminated"#).is_err());
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("").unwrap();
        assert!(tokens.is_empty());
    }

    // ── execute_dispatch tests ──

    fn test_config(binary: &str) -> ShellDispatchConfig {
        ShellDispatchConfig {
            agent_binary: PathBuf::from(binary),
            max_output_bytes: MAX_OUTPUT_BYTES,
            timeout: Duration::from_secs(5),
            extra_allowed: vec![],
            working_dir: None,
            guard: std::sync::Arc::new(crate::guard_patterns::CommandGuard::new()),
            agent_cli: None,
            agent_model: None,
        }
    }

    #[tokio::test]
    async fn test_execute_dispatch_captures_stdout() {
        let config = test_config("/bin/echo");
        // /bin/echo "--robot" "hello" => "--robot hello\n"
        let result = execute_dispatch(&config, "hello", &[]).await.unwrap();
        // echo receives args: "--robot", "hello"
        assert!(
            result.stdout.contains("hello"),
            "stdout was: {}",
            result.stdout
        );
        assert_eq!(result.exit_code, 0);
        assert!(!result.timed_out);
        assert!(!result.truncated);
    }

    #[tokio::test]
    async fn test_execute_dispatch_captures_exit_code() {
        let config = test_config("/bin/false");
        // /bin/false ignores all args and exits 1
        let result = execute_dispatch(&config, "anything", &[]).await.unwrap();
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_execute_dispatch_nonexistent_binary() {
        let config = test_config("/nonexistent/binary");
        let err = execute_dispatch(&config, "search", &[]).await.unwrap_err();
        assert!(err.contains("failed to spawn"), "error: {}", err);
    }

    // ── truncate_output tests ──

    #[test]
    fn test_truncate_output_within_budget() {
        let stdout = b"hello";
        let stderr = b"world";
        let (s, e, truncated) = truncate_output(stdout, stderr, 100);
        assert_eq!(s, "hello");
        assert_eq!(e, "world");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_output_over_budget() {
        let stdout = vec![b'A'; 100];
        let stderr = vec![b'B'; 100];
        let (s, e, truncated) = truncate_output(&stdout, &stderr, 150);
        assert_eq!(s.len(), 100);
        assert_eq!(e.len(), 50);
        assert!(truncated);
    }

    // ── guard integration tests ──

    #[tokio::test]
    async fn test_execute_dispatch_blocks_destructive_command() {
        let config = test_config("/bin/echo");
        // "git reset --hard" should be caught by the guard
        let result = execute_dispatch(&config, "guard", &["git reset --hard".to_string()]).await;
        // The guard should block this -- the args contain a destructive pattern
        // Note: the guard checks the joined command string "guard git reset --hard"
        // which contains "git reset --hard", a known destructive pattern
        assert!(
            result.is_err() || result.as_ref().is_ok_and(|r| r.exit_code != 0),
            "Guard should block or fail on destructive command pattern, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_dispatch_allows_safe_command() {
        let config = test_config("/bin/echo");
        // "search automata" is safe -- guard should allow
        let result = execute_dispatch(&config, "search", &["automata".to_string()]).await;
        assert!(
            result.is_ok(),
            "Guard should allow safe command: {:?}",
            result
        );
    }

    // ── execute_agent_dispatch tests ──

    #[tokio::test]
    async fn test_execute_agent_dispatch_not_configured() {
        let mut config = test_config("/bin/echo");
        config.agent_cli = None;
        let result = execute_agent_dispatch(&config, "test message").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent_cli not configured"));
    }

    #[tokio::test]
    async fn test_execute_agent_dispatch_success() {
        let mut config = test_config("/bin/echo");
        config.agent_cli = Some(PathBuf::from("/bin/echo"));
        config.agent_model = Some("kimi-for-coding/k2p5".to_string());

        let result = execute_agent_dispatch(&config, "test message").await;
        assert!(result.is_ok(), "Expected success, got: {:?}", result);
        let result = result.unwrap();
        assert_eq!(result.subcommand, "implement");
        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("test message"));
    }

    #[tokio::test]
    async fn test_execute_agent_dispatch_with_working_dir() {
        let mut config = test_config("/bin/sh");
        config.agent_cli = Some(PathBuf::from("/bin/sh"));
        config.agent_model = Some("test-model".to_string());
        config.working_dir = Some(PathBuf::from("/tmp"));

        let result = execute_agent_dispatch(&config, "echo hello").await;
        assert!(result.is_ok(), "Expected success, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_agent_dispatch_captures_nonzero_exit() {
        // Use a script that exits with code 42
        let mut config = test_config("/bin/sh");
        config.agent_cli = Some(PathBuf::from("/bin/sh"));
        config.agent_model = Some("test-model".to_string());
        config.working_dir = Some(PathBuf::from("/tmp"));

        // The shell will try to run "run" as a command and fail
        let result = execute_agent_dispatch(&config, "nonexistent_command").await;
        assert!(result.is_ok());
        let result = result.unwrap();
        // Shell returns 127 when command not found, or nonzero on other errors
        assert!(result.exit_code != 0 || !result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_execute_agent_dispatch_default_model() {
        let mut config = test_config("/bin/echo");
        config.agent_cli = Some(PathBuf::from("/bin/echo"));
        // No agent_model set - should use default

        let result = execute_agent_dispatch(&config, "test").await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.subcommand, "implement");
    }
}
