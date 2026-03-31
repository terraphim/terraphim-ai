use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub name: String,
    #[serde(default)]
    pub schedule: Option<String>, // cron expression
    pub repo_path: String,
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    /// Global flow timeout in seconds. If the entire flow exceeds this, it is aborted.
    #[serde(default = "default_flow_timeout")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub steps: Vec<FlowStepDef>,
}

fn default_base_branch() -> String {
    "main".to_string()
}

fn default_flow_timeout() -> u64 {
    3600 // 1 hour default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowStepDef {
    pub name: String,
    pub kind: StepKind,
    /// Shell command (for action steps).
    #[serde(default)]
    pub command: Option<String>,
    /// CLI tool binary (for agent steps).
    #[serde(default)]
    pub cli_tool: Option<String>,
    /// Model override (for agent steps).
    #[serde(default)]
    pub model: Option<String>,
    /// Inline task/prompt (for agent steps).
    #[serde(default)]
    pub task: Option<String>,
    /// Path to external markdown prompt file (for agent steps). Takes precedence over task.
    #[serde(default)]
    pub task_file: Option<String>,
    /// Gate condition expression (for gate steps).
    #[serde(default)]
    pub condition: Option<String>,
    /// Timeout in seconds for this step.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// What to do when this step fails.
    #[serde(default)]
    pub on_fail: FailStrategy,
    /// LLM provider (for agent steps).
    #[serde(default)]
    pub provider: Option<String>,
    /// Persona name (for agent steps).
    #[serde(default)]
    pub persona: Option<String>,
}

fn default_timeout() -> u64 {
    600
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    Action,
    Agent,
    Gate,
    Checkpoint,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailStrategy {
    #[default]
    Abort,
    SkipFailed,
    Continue,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_config_parse_minimal() {
        let toml_str = r#"
name = "test-flow"
repo_path = "/tmp/repo"

[[steps]]
name = "build"
kind = "action"
command = "cargo build"
"#;

        let flow: FlowDefinition = toml::from_str(toml_str).unwrap();
        assert_eq!(flow.name, "test-flow");
        assert_eq!(flow.repo_path, "/tmp/repo");
        assert_eq!(flow.base_branch, "main"); // default
        assert!(flow.schedule.is_none());
        assert_eq!(flow.steps.len(), 1);

        let step = &flow.steps[0];
        assert_eq!(step.name, "build");
        assert_eq!(step.kind, StepKind::Action);
        assert_eq!(step.command, Some("cargo build".to_string()));
        assert_eq!(step.timeout_secs, 600); // default
        assert_eq!(step.on_fail, FailStrategy::Abort); // default
    }

    #[test]
    fn test_flow_config_parse_full() {
        let toml_str = r#"
name = "compound-review-v2"
schedule = "0 2 * * *"
repo_path = "/home/user/project"
base_branch = "develop"

[[steps]]
name = "gather-changes"
kind = "action"
command = "git diff main..HEAD"
timeout_secs = 300
on_fail = "abort"

[[steps]]
name = "analyze-architecture"
kind = "agent"
cli_tool = "claude"
model = "sonnet"
task = "Review the architecture changes"
task_file = "/prompts/arch.md"
timeout_secs = 900
on_fail = "skip_failed"
provider = "anthropic"
persona = "architect"

[[steps]]
name = "check-quality"
kind = "agent"
cli_tool = "opencode"
model = "k2p5"
task = "Check code quality"
timeout_secs = 600
on_fail = "continue"
provider = "kimi-for-coding"

[[steps]]
name = "gate-approval"
kind = "gate"
condition = "steps.analyze-architecture.exit_code == 0"

[[steps]]
name = "checkpoint-state"
kind = "checkpoint"
"#;

        let flow: FlowDefinition = toml::from_str(toml_str).unwrap();
        assert_eq!(flow.name, "compound-review-v2");
        assert_eq!(flow.schedule, Some("0 2 * * *".to_string()));
        assert_eq!(flow.repo_path, "/home/user/project");
        assert_eq!(flow.base_branch, "develop");
        assert_eq!(flow.steps.len(), 5);

        // Action step
        let action_step = &flow.steps[0];
        assert_eq!(action_step.name, "gather-changes");
        assert_eq!(action_step.kind, StepKind::Action);
        assert_eq!(action_step.command, Some("git diff main..HEAD".to_string()));
        assert_eq!(action_step.timeout_secs, 300);
        assert_eq!(action_step.on_fail, FailStrategy::Abort);

        // Agent step with all fields
        let agent_step = &flow.steps[1];
        assert_eq!(agent_step.name, "analyze-architecture");
        assert_eq!(agent_step.kind, StepKind::Agent);
        assert_eq!(agent_step.cli_tool, Some("claude".to_string()));
        assert_eq!(agent_step.model, Some("sonnet".to_string()));
        assert_eq!(
            agent_step.task,
            Some("Review the architecture changes".to_string())
        );
        assert_eq!(agent_step.task_file, Some("/prompts/arch.md".to_string()));
        assert_eq!(agent_step.timeout_secs, 900);
        assert_eq!(agent_step.on_fail, FailStrategy::SkipFailed);
        assert_eq!(agent_step.provider, Some("anthropic".to_string()));
        assert_eq!(agent_step.persona, Some("architect".to_string()));

        // Second agent step
        let agent_step2 = &flow.steps[2];
        assert_eq!(agent_step2.name, "check-quality");
        assert_eq!(agent_step2.kind, StepKind::Agent);
        assert_eq!(agent_step2.on_fail, FailStrategy::Continue);

        // Gate step
        let gate_step = &flow.steps[3];
        assert_eq!(gate_step.name, "gate-approval");
        assert_eq!(gate_step.kind, StepKind::Gate);
        assert_eq!(
            gate_step.condition,
            Some("steps.analyze-architecture.exit_code == 0".to_string())
        );

        // Checkpoint step
        let checkpoint_step = &flow.steps[4];
        assert_eq!(checkpoint_step.name, "checkpoint-state");
        assert_eq!(checkpoint_step.kind, StepKind::Checkpoint);
    }

    #[test]
    fn test_step_kind_serde() {
        // Test serialization
        assert_eq!(
            serde_json::to_string(&StepKind::Action).unwrap(),
            "\"action\""
        );
        assert_eq!(
            serde_json::to_string(&StepKind::Agent).unwrap(),
            "\"agent\""
        );
        assert_eq!(serde_json::to_string(&StepKind::Gate).unwrap(), "\"gate\"");
        assert_eq!(
            serde_json::to_string(&StepKind::Checkpoint).unwrap(),
            "\"checkpoint\""
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<StepKind>("\"action\"").unwrap(),
            StepKind::Action
        );
        assert_eq!(
            serde_json::from_str::<StepKind>("\"agent\"").unwrap(),
            StepKind::Agent
        );
        assert_eq!(
            serde_json::from_str::<StepKind>("\"gate\"").unwrap(),
            StepKind::Gate
        );
        assert_eq!(
            serde_json::from_str::<StepKind>("\"checkpoint\"").unwrap(),
            StepKind::Checkpoint
        );
    }

    #[test]
    fn test_fail_strategy_default() {
        // Test that default is Abort
        let strategy: FailStrategy = Default::default();
        assert_eq!(strategy, FailStrategy::Abort);

        // Test deserialization with default
        let toml_str = r#"
name = "test"
kind = "action"
"#;
        let step: FlowStepDef = toml::from_str(toml_str).unwrap();
        assert_eq!(step.on_fail, FailStrategy::Abort);
    }

    #[test]
    fn test_fail_strategy_variants() {
        // Test all variants serialize/deserialize correctly
        assert_eq!(
            serde_json::to_string(&FailStrategy::Abort).unwrap(),
            "\"abort\""
        );
        assert_eq!(
            serde_json::to_string(&FailStrategy::SkipFailed).unwrap(),
            "\"skip_failed\""
        );
        assert_eq!(
            serde_json::to_string(&FailStrategy::Continue).unwrap(),
            "\"continue\""
        );

        assert_eq!(
            serde_json::from_str::<FailStrategy>("\"abort\"").unwrap(),
            FailStrategy::Abort
        );
        assert_eq!(
            serde_json::from_str::<FailStrategy>("\"skip_failed\"").unwrap(),
            FailStrategy::SkipFailed
        );
        assert_eq!(
            serde_json::from_str::<FailStrategy>("\"continue\"").unwrap(),
            FailStrategy::Continue
        );
    }
}
