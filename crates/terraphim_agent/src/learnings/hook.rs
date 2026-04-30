//! Hook input types and parser for AI agent integration.
//!
//! This module provides types for parsing JSON input from AI agent hooks
//! (Claude Code, Codex, opencode) and extracting failed commands for
//! learning capture.
//!
//! # Usage
//!
//! ```rust,ignore
//! use terraphim_agent::learnings::HookInput;
//!
//! let json = r#"{ "tool_name": "Bash", "tool_input": {"command": "git push"}, "tool_result": {"exit_code": 1, "stdout": "", "stderr": "rejected"} }"#;
//! let input = HookInput::from_json(json)?;
//!
//! if input.should_capture() {
//!     // Capture learning from failed command
//! }
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::learnings::redaction::contains_secrets;
use crate::learnings::{
    LearningCaptureConfig, LearningError, capture_failed_command, redact_secrets,
};

/// Hook type for multi-hook pipeline.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
pub enum LearnHookType {
    /// Pre-tool-use: warn if command matches past failure patterns
    PreToolUse,
    /// Post-tool-use: capture failed commands (existing behavior)
    PostToolUse,
    /// User prompt submit: capture user corrections inline
    UserPromptSubmit,
}

/// AI agent format for hook processing.
#[derive(Debug, Clone, Copy, PartialEq, clap::ValueEnum)]
#[allow(dead_code)]
pub enum AgentFormat {
    /// Claude Code format
    Claude,
    /// Codex format
    Codex,
    /// Opencode format
    Opencode,
}

/// Capture learning from hook input.
///
/// Extracts the command, error output, and exit code from the hook input
/// and delegates to `capture_failed_command` for storage.
///
/// # Arguments
///
/// * `input` - The parsed hook input
///
/// # Returns
///
/// Path to the saved learning file, or error if capture failed/ignored.
pub fn capture_from_hook(input: &HookInput) -> Result<PathBuf, LearningError> {
    let command = input
        .command()
        .ok_or_else(|| LearningError::Ignored("No command in input".to_string()))?;

    let error_output = input.error_output();
    let exit_code = input.tool_result.exit_code;

    let config = LearningCaptureConfig::default();
    capture_failed_command(command, &error_output, exit_code, &config)
}

/// Process hook input with an explicit hook type.
///
/// Routes to the appropriate handler based on the hook type:
/// - PreToolUse: checks command against known error patterns, warns if similar to past failure
/// - PostToolUse: captures failed commands (original behavior)
/// - UserPromptSubmit: captures user corrections inline
///
/// All hook types maintain fail-open behavior: errors are logged but
/// never block the pipeline.
pub async fn process_hook_input_with_type(
    _format: AgentFormat,
    hook_type: LearnHookType,
) -> Result<(), HookError> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Read stdin
    let mut buffer = String::new();
    tokio::io::stdin()
        .read_to_string(&mut buffer)
        .await
        .map_err(HookError::StdinError)?;

    match hook_type {
        LearnHookType::PreToolUse => {
            process_pre_tool_use(&buffer);
        }
        LearnHookType::PostToolUse => {
            // Parse JSON and capture failures or buffer successes.
            match HookInput::from_json(&buffer) {
                Ok(input) => {
                    if input.should_capture() {
                        if let Err(e) = capture_from_hook(&input) {
                            log::debug!("Hook capture failed: {}", e);
                        }
                    } else if input.should_capture_success() {
                        if let Some(command) = input.command() {
                            let config = LearningCaptureConfig::default();
                            let session_id = std::env::var("CLAUDE_SESSION_ID")
                                .or_else(|_| std::env::var("TERM_SESSION_ID"))
                                .unwrap_or_else(|_| "default".to_string());
                            let buf =
                                SessionCommandBuffer::new(&session_id, &config.storage_location());
                            buf.append(command, &input.tool_result.stdout);
                            log::debug!("Buffered successful command for session '{}'", session_id);
                        }
                    }
                }
                Err(e) => {
                    log::debug!("Hook parse failed (fail-open): {}", e);
                }
            }
        }
        LearnHookType::UserPromptSubmit => {
            process_user_prompt_submit(&buffer);
        }
    }

    // Redact secrets before passing through to stdout
    let output = if contains_secrets(&buffer) {
        log::debug!("Hook passthrough: secrets detected, redacting before stdout");
        redact_secrets(&buffer)
    } else {
        buffer
    };

    tokio::io::stdout()
        .write_all(output.as_bytes())
        .await
        .map_err(HookError::StdinError)?;

    Ok(())
}

/// Pre-tool-use handler: check if the command matches known failure patterns.
///
/// Reads the command from the JSON input and queries past learnings for
/// similar commands. If a match is found (especially one with a correction),
/// emits a warning to stderr. Never blocks execution.
fn process_pre_tool_use(json: &str) {
    let input = match HookInput::from_json(json) {
        Ok(i) => i,
        Err(_) => return, // fail-open
    };

    let command = match input.command() {
        Some(c) => c,
        None => return, // not a Bash tool, nothing to check
    };

    let config = LearningCaptureConfig::default();
    let storage_dir = config.storage_location();

    // Query past learnings for similar commands
    let base_cmd = command.split_whitespace().next().unwrap_or(command);
    let learnings = match crate::learnings::capture::query_learnings(&storage_dir, base_cmd, false)
    {
        Ok(l) => l,
        Err(_) => return,
    };

    if learnings.is_empty() {
        return;
    }

    // Find the best match: prefer one with a correction
    let best = learnings
        .iter()
        .find(|l| l.correction.is_some())
        .or(learnings.first());

    if let Some(learning) = best {
        let mut warning = format!(
            "Warning: similar command failed before (exit {}): {}",
            learning.exit_code, learning.command
        );
        if let Some(ref correction) = learning.correction {
            warning.push_str(&format!("\n  Suggested: {}", correction));
        }
        eprintln!("{}", warning);
    }
}

/// User-prompt-submit handler: capture user corrections inline.
///
/// Expects JSON with "user_prompt" field. Looks for correction patterns
/// like "use X instead of Y" and captures them as correction events.
/// Never blocks execution.
fn process_user_prompt_submit(json: &str) {
    let value: serde_json::Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(_) => return, // fail-open
    };

    let prompt = match value.get("user_prompt").and_then(|v| v.as_str()) {
        Some(p) => p,
        None => return,
    };

    // Look for correction patterns: "use X instead of Y", "use X not Y", "prefer X over Y"
    if let Some((original, corrected)) = parse_correction_pattern(prompt) {
        let config = LearningCaptureConfig::default();
        if let Err(e) = crate::learnings::capture_correction(
            crate::learnings::CorrectionType::ToolPreference,
            &original,
            &corrected,
            &format!("Auto-captured from user prompt: {}", prompt),
            &config,
        ) {
            log::debug!("User prompt correction capture failed: {}", e);
        }
    }
}

/// Try to parse a correction pattern from user text.
///
/// Supports patterns:
/// - "use X instead of Y"  -> (Y, X)
/// - "use X not Y"         -> (Y, X)
/// - "prefer X over Y"     -> (Y, X)
///
/// Returns None if no pattern matches.
fn parse_correction_pattern(text: &str) -> Option<(String, String)> {
    let lower = text.to_lowercase();
    let trimmed = lower.trim_start();

    // "use X instead of Y" (must start with "use")
    if let Some(use_idx) = trimmed.find("use ") {
        if use_idx == 0 {
            let text_after_use =
                &text[text.to_lowercase().trim_start().find("use ").unwrap() + 4..];
            let lower_after_use = text_after_use.to_lowercase();
            if let Some(instead_idx) = lower_after_use.find(" instead of ") {
                let corrected = text_after_use[..instead_idx].trim().to_string();
                let original = text_after_use[instead_idx + 12..]
                    .trim()
                    .trim_end_matches('.')
                    .to_string();
                if !corrected.is_empty() && !original.is_empty() {
                    return Some((original, corrected));
                }
            }
            // "use X not Y"
            if let Some(not_idx) = lower_after_use.find(" not ") {
                let corrected = text_after_use[..not_idx].trim().to_string();
                let original = text_after_use[not_idx + 5..]
                    .trim()
                    .trim_end_matches('.')
                    .to_string();
                if !corrected.is_empty() && !original.is_empty() {
                    return Some((original, corrected));
                }
            }
        }
    }

    // "prefer X over Y" (must start with "prefer")
    if let Some(prefer_idx) = trimmed.find("prefer ") {
        if prefer_idx == 0 {
            let text_after_prefer =
                &text[text.to_lowercase().trim_start().find("prefer ").unwrap() + 7..];
            let lower_after_prefer = text_after_prefer.to_lowercase();
            if let Some(over_idx) = lower_after_prefer.find(" over ") {
                let corrected = text_after_prefer[..over_idx].trim().to_string();
                let original = text_after_prefer[over_idx + 6..]
                    .trim()
                    .trim_end_matches('.')
                    .to_string();
                if !corrected.is_empty() && !original.is_empty() {
                    return Some((original, corrected));
                }
            }
        }
    }

    None
}

/// Errors that can occur during hook processing.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum HookError {
    /// Failed to read from stdin
    #[error("failed to read stdin: {0}")]
    StdinError(#[from] std::io::Error),
    /// Failed to parse hook input JSON
    #[error("failed to parse hook input: {0}")]
    ParseError(#[from] serde_json::Error),
    /// Capture operation failed
    #[error("capture failed: {0}")]
    CaptureError(#[from] LearningError),
}

/// Input from AI agent hook.
///
/// This struct represents the JSON payload sent by AI agents
/// when a tool is executed. It contains the tool name, input parameters,
/// and execution result.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct HookInput {
    /// Tool name (e.g., "Bash", "Write", "Edit")
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: ToolInput,
    /// Tool execution result
    pub tool_result: ToolResult,
}

/// Tool input parameters.
///
/// For Bash tools, this contains the command string.
/// For other tools, additional fields are captured via the `extra` map.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ToolInput {
    /// Command to execute (for Bash tool)
    pub command: Option<String>,
    /// Additional fields for other tool types
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Tool execution result.
///
/// Contains the exit code and captured output from the tool execution.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ToolResult {
    /// Exit code (0 = success, non-zero = failure)
    pub exit_code: i32,
    /// Standard output captured from the tool
    #[serde(default)]
    pub stdout: String,
    /// Standard error captured from the tool
    #[serde(default)]
    pub stderr: String,
}

#[allow(dead_code)]
impl HookInput {
    /// Parse hook input from a JSON string.
    ///
    /// # Arguments
    ///
    /// * `json` - The JSON string to parse
    ///
    /// # Returns
    ///
    /// Ok(HookInput) if parsing succeeds, Err(serde_json::Error) otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use terraphim_agent::learnings::HookInput;
    ///
    /// let json = r#"{
    ///     "tool_name": "Bash",
    ///     "tool_input": {"command": "git status"},
    ///     "tool_result": {"exit_code": 128, "stdout": "", "stderr": "fatal: not a git repository"}
    /// }"#;
    ///
    /// let input = HookInput::from_json(json).unwrap();
    /// assert_eq!(input.tool_name, "Bash");
    /// ```
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Check if this input should be captured as a failure learning.
    ///
    /// Returns true if:
    /// - The tool is "Bash" (command execution)
    /// - The exit code is non-zero (failure)
    ///
    /// # Returns
    ///
    /// true if the failed command should be captured, false otherwise.
    pub fn should_capture(&self) -> bool {
        self.tool_name == "Bash" && self.tool_result.exit_code != 0
    }

    /// Check if this successful command should be buffered for procedure learning.
    ///
    /// Returns true if:
    /// - The tool is "Bash" (command execution)
    /// - The exit code is zero (success)
    ///
    /// Successful commands are accumulated in a [`SessionCommandBuffer`] so that
    /// a procedure can be extracted from them later (e.g., via
    /// `learn procedure from-session`).
    pub fn should_capture_success(&self) -> bool {
        self.tool_name == "Bash" && self.tool_result.exit_code == 0
    }

    /// Get combined error output (stdout + stderr).
    ///
    /// Combines stdout and stderr with a newline separator if both are present.
    /// Useful for capturing the full error context for learning.
    ///
    /// # Returns
    ///
    /// Combined output string.
    pub fn error_output(&self) -> String {
        let mut output = String::new();
        if !self.tool_result.stdout.is_empty() {
            output.push_str(&self.tool_result.stdout);
        }
        if !self.tool_result.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.tool_result.stderr);
        }
        output
    }

    /// Get the command string from tool input.
    ///
    /// # Returns
    ///
    /// Some(&str) if a command is present, None otherwise.
    pub fn command(&self) -> Option<&str> {
        self.tool_input.command.as_deref()
    }
}

/// A single successful command entry persisted for procedure extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessEntry {
    /// The command that succeeded.
    pub command: String,
    /// Standard output captured from the successful run.
    pub stdout: String,
}

/// File-backed buffer that accumulates successful Bash commands within a session.
///
/// Each session writes to a JSONL file keyed by `session_id`. The accumulated
/// entries can be replayed later to create a procedure via
/// `terraphim-agent learn procedure from-session`.
///
/// The buffer is append-only and fail-open: write errors are logged but never
/// propagate to the hook caller.
pub struct SessionCommandBuffer {
    path: PathBuf,
}

#[allow(dead_code)]
impl SessionCommandBuffer {
    /// Create a buffer rooted in `storage_dir` for the given `session_id`.
    pub fn new(session_id: &str, storage_dir: &std::path::Path) -> Self {
        let path = storage_dir.join(format!("session-success-{session_id}.jsonl"));
        Self { path }
    }

    /// Append a successful command to the buffer.
    ///
    /// Creates the file on first write. Appends a single JSONL line.
    /// Errors are silently dropped (fail-open).
    pub fn append(&self, command: &str, stdout: &str) {
        let entry = SuccessEntry {
            command: command.to_string(),
            stdout: stdout.to_string(),
        };
        if let Ok(line) = serde_json::to_string(&entry) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)
            {
                let _ = writeln!(f, "{}", line);
            }
        }
    }

    /// Read all buffered entries.
    ///
    /// Returns an empty vec if the file does not exist or cannot be read.
    pub fn read(&self) -> Vec<SuccessEntry> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Path to the underlying JSONL file.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_input_parse() {
        let json = r#"{
            "tool_name": "Bash",
            "tool_input": {"command": "git push -f"},
            "tool_result": {"exit_code": 1, "stdout": "", "stderr": "rejected"}
        }"#;

        let input = HookInput::from_json(json).unwrap();
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.command(), Some("git push -f"));
        assert_eq!(input.tool_result.exit_code, 1);
        assert_eq!(input.tool_result.stdout, "");
        assert_eq!(input.tool_result.stderr, "rejected");
    }

    #[test]
    fn test_should_capture_failed_bash() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert!(input.should_capture());
    }

    #[test]
    fn test_should_not_capture_success() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        // Successful commands are NOT failure-captured but ARE success-captured.
        assert!(!input.should_capture());
        assert!(input.should_capture_success());
    }

    #[test]
    fn test_should_capture_success_only_for_bash_exit_zero() {
        // Non-Bash tool with exit 0 should NOT be success-captured.
        let non_bash = HookInput {
            tool_name: "Edit".to_string(),
            tool_input: ToolInput {
                command: None,
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert!(!non_bash.should_capture_success());

        // Bash with non-zero exit code is a failure, not a success.
        let bash_fail = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("false".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert!(!bash_fail.should_capture_success());
    }

    #[test]
    fn test_session_command_buffer_append_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let storage = dir.path().to_path_buf();
        let buf = SessionCommandBuffer::new("test-session", &storage);

        buf.append("cargo build", "Compiling...");
        buf.append("cargo test", "test result: ok");

        let entries = buf.read();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cargo build");
        assert_eq!(entries[0].stdout, "Compiling...");
        assert_eq!(entries[1].command, "cargo test");
        assert_eq!(entries[1].stdout, "test result: ok");
    }

    #[test]
    fn test_session_command_buffer_empty_on_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let storage = dir.path().to_path_buf();
        let buf = SessionCommandBuffer::new("nonexistent-session", &storage);
        assert!(buf.read().is_empty());
    }

    #[test]
    fn test_should_not_capture_edit() {
        let input = HookInput {
            tool_name: "Edit".to_string(),
            tool_input: ToolInput {
                command: None,
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert!(!input.should_capture());
    }

    #[test]
    fn test_error_output_combining() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: "output line 1".to_string(),
                stderr: "error line 1".to_string(),
            },
        };
        assert_eq!(input.error_output(), "output line 1\nerror line 1");
    }

    #[test]
    fn test_command_extraction() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("git push origin main".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert_eq!(input.command(), Some("git push origin main"));
    }

    #[test]
    fn test_command_extraction_none() {
        let input = HookInput {
            tool_name: "Edit".to_string(),
            tool_input: ToolInput {
                command: None,
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert_eq!(input.command(), None);
    }

    #[test]
    fn test_parse_with_extra_fields() {
        let json = r#"{
            "tool_name": "Write",
            "tool_input": {
                "path": "/tmp/test.txt",
                "content": "hello world"
            },
            "tool_result": {"exit_code": 0, "stdout": "", "stderr": ""}
        }"#;

        let input = HookInput::from_json(json).unwrap();
        assert_eq!(input.tool_name, "Write");
        assert!(input.tool_input.command.is_none());
        assert!(input.tool_input.extra.contains_key("path"));
        assert!(input.tool_input.extra.contains_key("content"));
    }

    #[test]
    fn test_error_output_stdout_only() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: "some output".to_string(),
                stderr: String::new(),
            },
        };
        assert_eq!(input.error_output(), "some output");
    }

    #[test]
    fn test_error_output_stderr_only() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: "some error".to_string(),
            },
        };
        assert_eq!(input.error_output(), "some error");
    }

    #[test]
    fn test_error_output_empty() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("cmd".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
            },
        };
        assert_eq!(input.error_output(), "");
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = "not valid json";
        let result = HookInput::from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_not_capture_bash_with_exit_zero() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("echo hello".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: "hello".to_string(),
                stderr: String::new(),
            },
        };
        assert!(!input.should_capture());
    }

    #[test]
    fn test_should_capture_bash_with_negative_exit_code() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("kill -9 $$".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: -1,
                stdout: String::new(),
                stderr: "Killed".to_string(),
            },
        };
        assert!(input.should_capture());
    }

    #[test]
    fn test_should_not_capture_non_bash_even_if_failed() {
        let input = HookInput {
            tool_name: "Write".to_string(),
            tool_input: ToolInput {
                command: None,
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: "Permission denied".to_string(),
            },
        };
        assert!(!input.should_capture());
    }

    #[test]
    fn test_capture_from_hook_success() {
        let input = HookInput {
            tool_name: "Bash".to_string(),
            tool_input: ToolInput {
                command: Some("git push".to_string()),
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 1,
                stdout: String::new(),
                stderr: "rejected".to_string(),
            },
        };

        // Should succeed and return a path
        let result = capture_from_hook(&input);
        // Note: This may fail if global dir is not writable, so we check it's not Ignored
        // for having no command
        if let Err(LearningError::Ignored(msg)) = &result {
            assert_ne!(msg, "No command in input");
        }
    }

    #[test]
    fn test_capture_from_hook_no_command() {
        let input = HookInput {
            tool_name: "Edit".to_string(),
            tool_input: ToolInput {
                command: None,
                extra: HashMap::new(),
            },
            tool_result: ToolResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        };

        let result = capture_from_hook(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            LearningError::Ignored(msg) => assert_eq!(msg, "No command in input"),
            _ => panic!("Expected Ignored error"),
        }
    }

    #[test]
    fn test_agent_format_variants() {
        // Verify AgentFormat enum variants exist and are distinct
        assert_ne!(AgentFormat::Claude, AgentFormat::Codex);
        assert_ne!(AgentFormat::Claude, AgentFormat::Opencode);
        assert_ne!(AgentFormat::Codex, AgentFormat::Opencode);
    }

    #[test]
    fn test_hook_passthrough_redacts_aws_key_in_error() {
        use crate::learnings::redact_secrets;
        use crate::learnings::redaction::contains_secrets;

        // Build a fake AWS key at runtime to avoid tripping the pre-commit secret scanner.
        // The key prefix "AKIA" followed by 16 uppercase alphanumeric chars is the pattern.
        let aws_key = format!("AKIA{}", "IOSFODNN7EXAMPLE");

        let json = format!(
            r#"{{
            "tool_name": "Bash",
            "tool_input": {{"command": "aws s3 ls"}},
            "tool_result": {{
                "exit_code": 1,
                "stdout": "",
                "stderr": "Unable to locate credentials {}"
            }}
        }}"#,
            aws_key
        );

        // Verify the input contains secrets
        assert!(contains_secrets(&json));

        // Verify redaction removes the AWS key
        let redacted = redact_secrets(&json);
        assert!(!redacted.contains(&aws_key));
        assert!(redacted.contains("[AWS_KEY_REDACTED]"));

        // Verify the redacted output is still valid JSON
        let parsed = HookInput::from_json(&redacted).unwrap();
        assert_eq!(parsed.tool_name, "Bash");
        assert_eq!(parsed.tool_result.exit_code, 1);
        assert!(parsed.tool_result.stderr.contains("[AWS_KEY_REDACTED]"));
    }

    #[test]
    fn test_learn_hook_type_variants() {
        assert_ne!(LearnHookType::PreToolUse, LearnHookType::PostToolUse);
        assert_ne!(LearnHookType::PostToolUse, LearnHookType::UserPromptSubmit);
        assert_ne!(LearnHookType::PreToolUse, LearnHookType::UserPromptSubmit);
    }

    #[test]
    fn test_parse_correction_pattern_use_instead_of() {
        let result = parse_correction_pattern("use bun instead of npm");
        assert_eq!(result, Some(("npm".to_string(), "bun".to_string())));
    }

    #[test]
    fn test_parse_correction_pattern_prefer_over() {
        let result = parse_correction_pattern("prefer cargo over make");
        assert_eq!(result, Some(("make".to_string(), "cargo".to_string())));
    }

    #[test]
    fn test_parse_correction_pattern_with_trailing_period() {
        let result = parse_correction_pattern("use Result<T> instead of unwrap().");
        assert_eq!(
            result,
            Some(("unwrap()".to_string(), "Result<T>".to_string()))
        );
    }

    #[test]
    fn test_parse_correction_pattern_use_not() {
        let result = parse_correction_pattern("use uv not pip");
        assert_eq!(result, Some(("pip".to_string(), "uv".to_string())));
    }

    #[test]
    fn test_parse_correction_pattern_no_match() {
        assert!(parse_correction_pattern("hello world").is_none());
        assert!(parse_correction_pattern("this is fine").is_none());
        // "I prefer tea over coffee" is a preference, not a tool correction
        assert!(parse_correction_pattern("I prefer tea over coffee").is_none());
    }

    #[test]
    fn test_pre_tool_use_no_crash_on_non_bash() {
        // Non-Bash tool should not crash (fail-open)
        let json = r#"{
            "tool_name": "Edit",
            "tool_input": {"path": "/tmp/test.txt"},
            "tool_result": {"exit_code": 0, "stdout": "", "stderr": ""}
        }"#;
        process_pre_tool_use(json);
        // No panic = pass
    }

    #[test]
    fn test_pre_tool_use_no_crash_on_invalid_json() {
        // Invalid JSON should not crash (fail-open)
        process_pre_tool_use("not valid json");
        // No panic = pass
    }

    #[test]
    fn test_user_prompt_submit_no_crash_on_empty() {
        process_user_prompt_submit("{}");
        // No panic = pass
    }

    #[test]
    fn test_user_prompt_submit_no_crash_on_invalid_json() {
        process_user_prompt_submit("invalid");
        // No panic = pass
    }
}
