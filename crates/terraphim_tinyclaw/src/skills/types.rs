//! Skill type definitions for JSON serialization.

use serde::{Deserialize, Serialize};

/// A skill definition containing sequential steps.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Skill {
    /// Skill name (unique identifier)
    pub name: String,
    /// Semantic version
    pub version: String,
    /// Human-readable description
    pub description: String,
    /// Optional author information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Sequential steps to execute
    pub steps: Vec<SkillStep>,
    /// Input parameters the skill accepts
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<SkillInput>,
}

/// An individual step in a skill workflow.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "type")]
pub enum SkillStep {
    /// Execute a tool
    #[serde(rename = "tool")]
    Tool {
        /// Tool name
        tool: String,
        /// Arguments for the tool
        args: serde_json::Value,
    },
    /// Send a prompt to the LLM
    #[serde(rename = "llm")]
    Llm {
        /// Prompt text
        prompt: String,
        /// Whether to include conversation history
        #[serde(default = "default_true")]
        use_context: bool,
    },
    /// Execute a shell command
    #[serde(rename = "shell")]
    Shell {
        /// Command to execute
        command: String,
        /// Working directory for the command
        #[serde(skip_serializing_if = "Option::is_none")]
        working_dir: Option<String>,
    },
}

/// Input parameter definition for a skill.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SkillInput {
    /// Parameter name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Whether this parameter is required
    #[serde(default = "default_true")]
    pub required: bool,
    /// Default value if not provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Result of skill execution.
#[derive(Debug, Clone)]
pub struct SkillResult {
    /// Final status
    pub status: SkillStatus,
    /// Output text
    pub output: String,
    /// Step-by-step execution log
    pub execution_log: Vec<StepResult>,
    /// Execution duration
    pub duration_ms: u64,
}

/// Status of skill execution.
#[derive(Debug, Clone, PartialEq)]
pub enum SkillStatus {
    /// Successfully completed all steps
    Success,
    /// Failed at a specific step
    Failed { step: usize, error: String },
    /// Cancelled by user
    Cancelled,
    /// Timed out
    Timeout,
}

/// Result of a single step execution.
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Step number (0-indexed)
    pub step_number: usize,
    /// Step type (tool/llm/shell)
    pub step_type: String,
    /// Success or failure
    pub success: bool,
    /// Output from this step
    pub output: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_serialization() {
        let skill = Skill {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A test skill".to_string(),
            author: Some("Test Author".to_string()),
            steps: vec![SkillStep::Tool {
                tool: "shell".to_string(),
                args: serde_json::json!({"command": "echo hello"}),
            }],
            inputs: vec![],
        };

        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("test-skill"));
        assert!(json.contains("1.0.0"));

        let deserialized: Skill = serde_json::from_str(&json).unwrap();
        assert_eq!(skill, deserialized);
    }

    #[test]
    fn test_skill_step_tool() {
        let step = SkillStep::Tool {
            tool: "filesystem".to_string(),
            args: serde_json::json!({
                "operation": "read_file",
                "path": "/tmp/test.txt"
            }),
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("tool"));
        assert!(json.contains("filesystem"));

        let deserialized: SkillStep = serde_json::from_str(&json).unwrap();
        match deserialized {
            SkillStep::Tool { tool, .. } => assert_eq!(tool, "filesystem"),
            _ => assert!(false, "Expected Tool step"),
        }
    }

    #[test]
    fn test_skill_step_llm() {
        let step = SkillStep::Llm {
            prompt: "Analyze this".to_string(),
            use_context: true,
        };

        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("llm"));
        assert!(json.contains("Analyze this"));
    }

    #[test]
    fn test_skill_with_inputs() {
        let skill = Skill {
            name: "analyze-repo".to_string(),
            version: "1.0.0".to_string(),
            description: "Analyze a git repository".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![
                SkillInput {
                    name: "repo_url".to_string(),
                    description: "URL of the repository".to_string(),
                    required: true,
                    default: None,
                },
                SkillInput {
                    name: "branch".to_string(),
                    description: "Branch to analyze".to_string(),
                    required: false,
                    default: Some("main".to_string()),
                },
            ],
        };

        let json = serde_json::to_string(&skill).unwrap();
        assert!(json.contains("repo_url"));
        assert!(json.contains("branch"));
    }
}
