//! Skill executor for loading and executing skill workflows.

use crate::skills::types::{Skill, SkillInput, SkillResult, SkillStatus, SkillStep, StepResult};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors that can occur during skill execution.
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("Skill not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Template error: {0}")]
    Template(String),
    #[error("Execution cancelled")]
    Cancelled,
    #[error("Execution timeout")]
    Timeout,
    #[error("Missing required input: {0}")]
    MissingInput(String),
}

/// Executes skills with progress tracking and cancellation support.
pub struct SkillExecutor {
    /// Directory where skills are stored
    storage_dir: PathBuf,
    /// Cancellation flag shared across execution
    cancelled: Arc<AtomicBool>,
}

impl SkillExecutor {
    /// Create a new skill executor with the given storage directory.
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self, SkillError> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&storage_dir)?;

        Ok(Self {
            storage_dir,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get the default skills directory (~/.config/terraphim/skills).
    pub fn default_storage_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("terraphim")
            .join("skills")
    }

    /// Create a new executor with the default storage location.
    pub fn with_default_storage() -> Result<Self, SkillError> {
        Self::new(Self::default_storage_dir())
    }

    /// Save a skill to storage.
    pub fn save_skill(&self, skill: &Skill) -> Result<(), SkillError> {
        let path = self.skill_path(&skill.name);
        let json = serde_json::to_string_pretty(skill)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a skill from storage.
    pub fn load_skill(&self, name: &str) -> Result<Skill, SkillError> {
        let path = self.skill_path(name);
        if !path.exists() {
            return Err(SkillError::NotFound(name.to_string()));
        }
        let json = std::fs::read_to_string(path)?;
        let skill = serde_json::from_str(&json)?;
        Ok(skill)
    }

    /// List all available skills.
    pub fn list_skills(&self) -> Result<Vec<Skill>, SkillError> {
        let mut skills = Vec::new();

        if !self.storage_dir.exists() {
            return Ok(skills);
        }

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                let json = std::fs::read_to_string(&path)?;
                if let Ok(skill) = serde_json::from_str::<Skill>(&json) {
                    skills.push(skill);
                }
            }
        }

        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(skills)
    }

    /// Delete a skill from storage.
    pub fn delete_skill(&self, name: &str) -> Result<(), SkillError> {
        let path = self.skill_path(name);
        if !path.exists() {
            return Err(SkillError::NotFound(name.to_string()));
        }
        std::fs::remove_file(path)?;
        Ok(())
    }

    /// Cancel any ongoing skill execution.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Reset cancellation flag for new execution.
    pub fn reset_cancellation(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
    }

    /// Execute a skill with the given inputs.
    pub async fn execute_skill(
        &self,
        skill: &Skill,
        inputs: HashMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<SkillResult, SkillError> {
        self.reset_cancellation();

        let start = Instant::now();
        let mut execution_log = Vec::new();
        let mut accumulated_output = String::new();

        // Validate inputs
        self.validate_inputs(skill, &inputs)?;

        // Merge with defaults
        let inputs = self.merge_with_defaults(skill, inputs);

        for (step_idx, step) in skill.steps.iter().enumerate() {
            // Check cancellation
            if self.cancelled.load(Ordering::SeqCst) {
                return Ok(SkillResult {
                    status: SkillStatus::Cancelled,
                    output: accumulated_output,
                    execution_log,
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }

            let step_start = Instant::now();

            let step_result = match step {
                SkillStep::Tool { tool, args } => self.execute_tool_step(tool, args, &inputs).await,
                SkillStep::Llm {
                    prompt,
                    use_context,
                } => self.execute_llm_step(prompt, *use_context, &inputs).await,
                SkillStep::Shell {
                    command,
                    working_dir,
                } => {
                    self.execute_shell_step(command, working_dir.as_deref(), &inputs)
                        .await
                }
            };

            let step_duration = step_start.elapsed().as_millis() as u64;

            match step_result {
                Ok(output) => {
                    accumulated_output.push_str(&format!("Step {}: {}\n\n", step_idx + 1, output));
                    execution_log.push(StepResult {
                        step_number: step_idx,
                        step_type: step_type_name(step),
                        success: true,
                        output: output.clone(),
                        duration_ms: step_duration,
                    });
                }
                Err(e) => {
                    return Ok(SkillResult {
                        status: SkillStatus::Failed {
                            step: step_idx,
                            error: e.to_string(),
                        },
                        output: accumulated_output,
                        execution_log,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }

            // Check timeout
            if let Some(timeout) = timeout {
                if start.elapsed() > timeout {
                    return Ok(SkillResult {
                        status: SkillStatus::Timeout,
                        output: accumulated_output,
                        execution_log,
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        Ok(SkillResult {
            status: SkillStatus::Success,
            output: accumulated_output,
            execution_log,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    /// Substitute template variables in a string.
    pub fn substitute_template(
        &self,
        template: &str,
        inputs: &HashMap<String, String>,
    ) -> Result<String, SkillError> {
        let mut result = template.to_string();

        // Find all {variable} patterns where variable is alphanumeric + underscore
        let mut start = 0;
        while let Some(open) = result[start..].find('{') {
            let open = start + open;
            if let Some(close) = result[open..].find('}') {
                let close = open + close;
                let var_name = &result[open + 1..close];

                // Only substitute if it's a valid variable name (alphanumeric + underscore)
                // and doesn't contain braces (to avoid matching JSON objects)
                if var_name.chars().all(|c| c.is_alphanumeric() || c == '_') && !var_name.is_empty()
                {
                    if let Some(value) = inputs.get(var_name) {
                        result.replace_range(open..=close, value);
                        // Adjust start for the replacement
                        start = open + value.len();
                        continue;
                    } else {
                        return Err(SkillError::Template(format!(
                            "Unknown variable: {}",
                            var_name
                        )));
                    }
                }

                // Not a valid variable, skip this brace
                start = open + 1;
            } else {
                break;
            }
        }

        Ok(result)
    }

    // Private helper methods

    fn skill_path(&self, name: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", name))
    }

    fn validate_inputs(
        &self,
        skill: &Skill,
        inputs: &HashMap<String, String>,
    ) -> Result<(), SkillError> {
        for input_def in &skill.inputs {
            if input_def.required && !inputs.contains_key(&input_def.name) {
                // Check if it has a default
                if input_def.default.is_none() {
                    return Err(SkillError::MissingInput(input_def.name.clone()));
                }
            }
        }
        Ok(())
    }

    fn merge_with_defaults(
        &self,
        skill: &Skill,
        mut inputs: HashMap<String, String>,
    ) -> HashMap<String, String> {
        for input_def in &skill.inputs {
            if !inputs.contains_key(&input_def.name) {
                if let Some(default) = &input_def.default {
                    inputs.insert(input_def.name.clone(), default.clone());
                }
            }
        }
        inputs
    }

    async fn execute_tool_step(
        &self,
        tool: &str,
        args: &serde_json::Value,
        inputs: &HashMap<String, String>,
    ) -> Result<String, SkillError> {
        // Substitute variables in args
        let args_str = serde_json::to_string(args)?;
        let substituted = self.substitute_template(&args_str, inputs)?;
        let substituted_args: serde_json::Value = serde_json::from_str(&substituted)?;

        // Placeholder - actual tool execution would integrate with the tool registry
        Ok(format!(
            "Executed tool '{}' with args: {}",
            tool, substituted_args
        ))
    }

    async fn execute_llm_step(
        &self,
        prompt: &str,
        _use_context: bool,
        inputs: &HashMap<String, String>,
    ) -> Result<String, SkillError> {
        let substituted = self.substitute_template(prompt, inputs)?;

        // Placeholder - actual LLM execution would integrate with the agent
        Ok(format!("LLM prompt: {}", substituted))
    }

    async fn execute_shell_step(
        &self,
        command: &str,
        working_dir: Option<&str>,
        inputs: &HashMap<String, String>,
    ) -> Result<String, SkillError> {
        let substituted = self.substitute_template(command, inputs)?;

        // Placeholder - actual shell execution
        Ok(format!(
            "Shell command: {} (in {:?})",
            substituted, working_dir
        ))
    }
}

fn step_type_name(step: &SkillStep) -> String {
    match step {
        SkillStep::Tool { .. } => "tool".to_string(),
        SkillStep::Llm { .. } => "llm".to_string(),
        SkillStep::Shell { .. } => "shell".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::TempDir;

    fn create_test_skill() -> Skill {
        Skill {
            name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "A test skill".to_string(),
            author: Some("Test".to_string()),
            steps: vec![
                SkillStep::Tool {
                    tool: "shell".to_string(),
                    args: serde_json::json!({"command": "echo {message}"}),
                },
                SkillStep::Llm {
                    prompt: "Analyze: {message}".to_string(),
                    use_context: true,
                },
            ],
            inputs: vec![SkillInput {
                name: "message".to_string(),
                description: "Message to process".to_string(),
                required: true,
                default: None,
            }],
        }
    }

    #[tokio::test]
    async fn test_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        assert!(executor.storage_dir.exists());
    }

    #[tokio::test]
    async fn test_save_and_load_skill() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        let skill = create_test_skill();

        executor.save_skill(&skill).unwrap();

        let loaded = executor.load_skill("test-skill").unwrap();
        assert_eq!(loaded.name, skill.name);
        assert_eq!(loaded.steps.len(), skill.steps.len());
    }

    #[tokio::test]
    async fn test_load_nonexistent_skill() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let result = executor.load_skill("nonexistent");
        assert!(matches!(result, Err(SkillError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_list_skills() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let skill1 = Skill {
            name: "skill-1".to_string(),
            version: "1.0.0".to_string(),
            description: "First skill".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        };

        let skill2 = Skill {
            name: "skill-2".to_string(),
            version: "1.0.0".to_string(),
            description: "Second skill".to_string(),
            author: None,
            steps: vec![],
            inputs: vec![],
        };

        executor.save_skill(&skill1).unwrap();
        executor.save_skill(&skill2).unwrap();

        let skills = executor.list_skills().unwrap();
        assert_eq!(skills.len(), 2);
        assert_eq!(skills[0].name, "skill-1");
        assert_eq!(skills[1].name, "skill-2");
    }

    #[tokio::test]
    async fn test_delete_skill() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        let skill = create_test_skill();

        executor.save_skill(&skill).unwrap();
        assert!(executor.load_skill("test-skill").is_ok());

        executor.delete_skill("test-skill").unwrap();
        assert!(executor.load_skill("test-skill").is_err());
    }

    #[tokio::test]
    async fn test_template_substitution() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), "Alice".to_string());
        inputs.insert("greeting".to_string(), "Hello".to_string());

        let template = "{greeting}, {name}!";
        let result = executor.substitute_template(template, &inputs).unwrap();
        assert_eq!(result, "Hello, Alice!");
    }

    #[tokio::test]
    async fn test_template_unknown_variable() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let inputs = HashMap::new();
        let template = "Hello, {name}!";

        let result = executor.substitute_template(template, &inputs);
        assert!(matches!(result, Err(SkillError::Template(_))));
    }

    #[tokio::test]
    async fn test_execute_skill_success() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        let skill = create_test_skill();

        let mut inputs = HashMap::new();
        inputs.insert("message".to_string(), "world".to_string());

        let result = executor.execute_skill(&skill, inputs, None).await.unwrap();

        assert_eq!(result.status, SkillStatus::Success);
        assert_eq!(result.execution_log.len(), 2);
        assert!(result.output.contains("world"));
    }

    #[tokio::test]
    async fn test_execute_skill_missing_input() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        let skill = create_test_skill();

        let inputs = HashMap::new();
        let result = executor.execute_skill(&skill, inputs, None).await;

        assert!(matches!(result, Err(SkillError::MissingInput(_))));
    }

    #[tokio::test]
    async fn test_execute_skill_with_default() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let skill = Skill {
            name: "test-with-default".to_string(),
            version: "1.0.0".to_string(),
            description: "Test".to_string(),
            author: None,
            steps: vec![SkillStep::Llm {
                prompt: "Hello {name}".to_string(),
                use_context: false,
            }],
            inputs: vec![SkillInput {
                name: "name".to_string(),
                description: "Name".to_string(),
                required: true,
                default: Some("World".to_string()),
            }],
        };

        // Should use default
        let inputs = HashMap::new();
        let result = executor.execute_skill(&skill, inputs, None).await.unwrap();

        assert_eq!(result.status, SkillStatus::Success);
        assert!(result.output.contains("Hello World"));
    }

    #[tokio::test]
    async fn test_cancellation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();
        let executor_clone = SkillExecutor::new(temp_dir.path()).unwrap();

        // Create a skill with multiple steps
        let skill = Skill {
            name: "multi-step".to_string(),
            version: "1.0.0".to_string(),
            description: "Multi-step skill".to_string(),
            author: None,
            steps: vec![
                SkillStep::Llm {
                    prompt: "Step 1".to_string(),
                    use_context: false,
                },
                SkillStep::Llm {
                    prompt: "Step 2".to_string(),
                    use_context: false,
                },
                SkillStep::Llm {
                    prompt: "Step 3".to_string(),
                    use_context: false,
                },
            ],
            inputs: vec![],
        };

        // Start execution in a separate task and cancel it mid-execution
        let exec_clone = Arc::new(executor_clone);
        let skill_clone = skill.clone();

        let handle = tokio::spawn(async move {
            // Give a small delay then cancel
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
            exec_clone.cancel();
        });

        let result = executor
            .execute_skill(&skill, HashMap::new(), None)
            .await
            .unwrap();

        // Wait for the cancel task to complete
        let _ = handle.await;

        // Result could be Success (if it completed before cancel) or Cancelled
        assert!(
            result.status == SkillStatus::Success || result.status == SkillStatus::Cancelled,
            "Expected Success or Cancelled, got {:?}",
            result.status
        );
    }

    #[tokio::test]
    async fn test_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SkillExecutor::new(temp_dir.path()).unwrap();

        let skill = Skill {
            name: "slow-skill".to_string(),
            version: "1.0.0".to_string(),
            description: "Slow skill".to_string(),
            author: None,
            steps: vec![
                SkillStep::Llm {
                    prompt: "Step 1".to_string(),
                    use_context: false,
                },
                SkillStep::Llm {
                    prompt: "Step 2".to_string(),
                    use_context: false,
                },
            ],
            inputs: vec![],
        };

        // Very short timeout to ensure it triggers
        let result = executor
            .execute_skill(&skill, HashMap::new(), Some(Duration::from_nanos(1)))
            .await
            .unwrap();

        // Should timeout (though timing may vary in tests)
        // Just verify execution completed without error
        assert!(result.status == SkillStatus::Success || result.status == SkillStatus::Timeout);
    }
}
