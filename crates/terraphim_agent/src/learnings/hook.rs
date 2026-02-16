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

use serde::Deserialize;
use thiserror::Error;

use crate::learnings::LearningError;

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

    /// Check if this input should be captured as a learning.
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
        assert!(!input.should_capture());
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
}
