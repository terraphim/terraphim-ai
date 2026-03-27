use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use terraphim_symphony::runner::protocol::{FindingCategory, ReviewAgentOutput, ReviewFinding};

use crate::config::CompoundReviewConfig;
use crate::error::OrchestratorError;
use crate::scope::WorktreeManager;

// Embed prompt templates at compile time to avoid CWD-dependent file loading.
// The ADF binary may run from /opt/ai-dark-factory/ but templates live in the
// source tree. Embedding eliminates the path resolution issue entirely.
const PROMPT_SECURITY: &str = include_str!("../prompts/review-security.md");
const PROMPT_ARCHITECTURE: &str = include_str!("../prompts/review-architecture.md");
const PROMPT_PERFORMANCE: &str = include_str!("../prompts/review-performance.md");
const PROMPT_QUALITY: &str = include_str!("../prompts/review-quality.md");
const PROMPT_DOMAIN: &str = include_str!("../prompts/review-domain.md");
const PROMPT_DESIGN_QUALITY: &str = include_str!("../prompts/review-design-quality.md");

/// Definition of a single review group (1 agent per group).
#[derive(Debug, Clone)]
pub struct ReviewGroupDef {
    /// Name of the agent (e.g., "security-sentinel").
    pub agent_name: String,
    /// Category of findings this agent produces.
    pub category: FindingCategory,
    /// LLM tier to use (e.g., "Quick", "Deep").
    pub llm_tier: String,
    /// CLI tool to invoke (e.g., "opencode", "claude").
    pub cli_tool: String,
    /// Optional model override.
    pub model: Option<String>,
    /// Path to prompt template file (retained for logging/debug).
    pub prompt_template: String,
    /// Embedded prompt content (compile-time via include_str).
    pub prompt_content: &'static str,
    /// Whether this agent only runs on visual/design changes.
    pub visual_only: bool,
    /// Persona identity for this review agent (e.g., "Vigil", "Carthos").
    pub persona: Option<String>,
}

impl ReviewGroupDef {
    /// Load the prompt template content from file.
    pub fn prompt(&self) -> &str {
        self.prompt_content
    }
}

/// Configuration for the review swarm.
#[derive(Debug, Clone)]
pub struct SwarmConfig {
    /// Review group definitions (6 groups).
    pub groups: Vec<ReviewGroupDef>,
    /// Timeout for agent execution.
    pub timeout: Duration,
    /// Root directory for worktrees.
    pub worktree_root: PathBuf,
    /// Path to the git repository.
    pub repo_path: PathBuf,
    /// Base branch for comparison.
    pub base_branch: String,
    /// Maximum number of concurrent agents.
    pub max_concurrent_agents: usize,
    /// Whether to create PRs with findings.
    pub create_prs: bool,
}

impl SwarmConfig {
    /// Create a SwarmConfig from CompoundReviewConfig and add default groups.
    pub fn from_compound_config(config: &CompoundReviewConfig) -> Self {
        let mut groups = default_groups();

        // Override cli_tool and model from CompoundReviewConfig when present.
        if let Some(ref cli_tool) = config.cli_tool {
            for group in &mut groups {
                group.cli_tool = cli_tool.clone();
            }
        }
        if let Some(ref model) = config.model {
            // If provider is also set and CLI is opencode, compose provider/model
            let composed = if let Some(ref provider) = config.provider {
                let cli_tool_name = config.cli_tool.as_deref().unwrap_or("");
                let cli_name = std::path::Path::new(cli_tool_name)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(cli_tool_name);
                if cli_name == "opencode" {
                    format!("{}/{}", provider, model)
                } else {
                    model.clone()
                }
            } else {
                model.clone()
            };
            for group in &mut groups {
                group.model = Some(composed.clone());
            }
        }

        Self {
            groups,
            timeout: Duration::from_secs(config.max_duration_secs),
            worktree_root: config.worktree_root.clone(),
            repo_path: config.repo_path.clone(),
            base_branch: config.base_branch.clone(),
            max_concurrent_agents: config.max_concurrent_agents,
            create_prs: config.create_prs,
        }
    }

    /// Create a SwarmConfig from CompoundReviewConfig with no review groups.
    /// Useful for testing orchestrator lifecycle without spawning agents.
    pub fn from_compound_config_empty(config: &CompoundReviewConfig) -> Self {
        Self {
            groups: vec![],
            timeout: Duration::from_secs(300),
            worktree_root: config.worktree_root.clone(),
            repo_path: config.repo_path.clone(),
            base_branch: config.base_branch.clone(),
            max_concurrent_agents: config.max_concurrent_agents,
            create_prs: config.create_prs,
        }
    }
}

/// Result of a compound review cycle.
#[derive(Debug, Clone)]
pub struct CompoundReviewResult {
    /// Correlation ID for this review run.
    pub correlation_id: Uuid,
    /// All findings from all agents (deduplicated).
    pub findings: Vec<ReviewFinding>,
    /// Individual agent outputs.
    pub agent_outputs: Vec<ReviewAgentOutput>,
    /// Overall pass/fail status.
    pub pass: bool,
    /// Duration of the review.
    pub duration: Duration,
    /// Number of agents that ran.
    pub agents_run: usize,
    /// Number of agents that failed.
    pub agents_failed: usize,
}

/// Nightly compound review workflow with 6-agent swarm.
///
/// Dispatches review agents in parallel, collects findings,
/// and optionally creates PRs with results.
#[derive(Debug)]
pub struct CompoundReviewWorkflow {
    config: SwarmConfig,
    worktree_manager: WorktreeManager,
}

impl CompoundReviewWorkflow {
    /// Create a new compound review workflow from swarm config.
    pub fn new(config: SwarmConfig) -> Self {
        let worktree_manager = WorktreeManager::with_base(&config.repo_path, &config.worktree_root);
        Self {
            config,
            worktree_manager,
        }
    }

    /// Create from CompoundReviewConfig (legacy compatibility).
    pub fn from_compound_config(config: CompoundReviewConfig) -> Self {
        let swarm_config = SwarmConfig::from_compound_config(&config);
        Self::new(swarm_config)
    }

    /// Run a full compound review cycle.
    ///
    /// 1. Get changed files between git_ref and base_ref
    /// 2. Filter groups based on visual changes
    /// 3. Spawn agents in parallel
    /// 4. Collect results with timeout
    /// 5. Deduplicate findings
    /// 6. Return structured result
    pub async fn run(
        &self,
        git_ref: &str,
        base_ref: &str,
    ) -> Result<CompoundReviewResult, OrchestratorError> {
        let start = Instant::now();
        let correlation_id = Uuid::new_v4();

        info!(
            correlation_id = %correlation_id,
            git_ref = %git_ref,
            base_ref = %base_ref,
            "starting compound review swarm"
        );

        // Get changed files
        let changed_files = self.get_changed_files(git_ref, base_ref).await?;
        debug!(count = changed_files.len(), "found changed files");

        // Filter groups based on visual changes
        let has_visual = has_visual_changes(&changed_files);
        let active_groups: Vec<&ReviewGroupDef> = self
            .config
            .groups
            .iter()
            .filter(|g| !g.visual_only || has_visual)
            .collect();

        info!(
            total_groups = self.config.groups.len(),
            active_groups = active_groups.len(),
            has_visual_changes = has_visual,
            "filtered review groups"
        );

        // Create worktree for this review
        let worktree_name = format!("review-{}", correlation_id);
        let worktree_path = self
            .worktree_manager
            .create_worktree(&worktree_name, git_ref)
            .await
            .map_err(|e| {
                OrchestratorError::CompoundReviewFailed(format!("failed to create worktree: {}", e))
            })?;

        // Channel for collecting agent outputs
        let (tx, mut rx) = mpsc::channel::<AgentResult>(active_groups.len().max(1));

        // Spawn agents in parallel
        let mut spawned_count = 0;
        for group in active_groups {
            let tx = tx.clone();
            let group = group.clone();
            let worktree_path = worktree_path.clone();
            let changed_files = changed_files.clone();
            let timeout = self.config.timeout;
            let cli_tool = group.cli_tool.clone();

            tokio::spawn(async move {
                let result = run_single_agent(
                    &group,
                    &worktree_path,
                    &changed_files,
                    correlation_id,
                    timeout,
                    &cli_tool,
                )
                .await;
                let _ = tx.send(result).await;
            });
            spawned_count += 1;
        }

        // Collect results with deadline-based timeout
        drop(tx);
        let mut agent_outputs = Vec::new();
        let mut failed_count = 0;
        let collect_deadline =
            tokio::time::Instant::now() + self.config.timeout + Duration::from_secs(10);

        loop {
            match tokio::time::timeout_at(collect_deadline, rx.recv()).await {
                Ok(Some(result)) => match result {
                    AgentResult::Success(output) => {
                        info!(agent = %output.agent, findings = output.findings.len(), "agent completed");
                        agent_outputs.push(output);
                    }
                    AgentResult::Failed { agent_name, reason } => {
                        warn!(agent = %agent_name, error = %reason, "agent failed");
                        failed_count += 1;
                        agent_outputs.push(ReviewAgentOutput {
                            agent: agent_name,
                            findings: vec![],
                            summary: format!("Agent failed: {}", reason),
                            pass: false,
                        });
                    }
                },
                Ok(None) => break, // channel closed, all senders dropped
                Err(_) => {
                    warn!("collection deadline exceeded, using partial results");
                    break;
                }
            }
        }

        // Cleanup worktree
        if let Err(e) = self.worktree_manager.remove_worktree(&worktree_name).await {
            warn!(error = %e, "failed to cleanup worktree");
        }

        // Collect all findings and deduplicate
        let all_findings: Vec<ReviewFinding> = agent_outputs
            .iter()
            .flat_map(|o| o.findings.clone())
            .collect();
        let deduplicated = terraphim_symphony::runner::protocol::deduplicate_findings(all_findings);

        // Determine overall pass/fail
        let pass = agent_outputs.iter().all(|o| o.pass) && failed_count == 0;

        let duration = start.elapsed();
        info!(
            correlation_id = %correlation_id,
            agents_run = spawned_count,
            agents_failed = failed_count,
            total_findings = deduplicated.len(),
            pass = %pass,
            duration = ?duration,
            "compound review completed"
        );

        Ok(CompoundReviewResult {
            correlation_id,
            findings: deduplicated,
            agent_outputs,
            pass,
            duration,
            agents_run: spawned_count,
            agents_failed: failed_count,
        })
    }

    /// Get the default review groups (6 groups).
    pub fn default_groups() -> Vec<ReviewGroupDef> {
        default_groups()
    }

    /// Check if there are visual changes in the changed files.
    pub fn has_visual_changes(changed_files: &[String]) -> bool {
        has_visual_changes(changed_files)
    }

    /// Extract ReviewAgentOutput from agent stdout.
    pub fn extract_review_output(
        stdout: &str,
        agent_name: &str,
        category: FindingCategory,
    ) -> ReviewAgentOutput {
        extract_review_output(stdout, agent_name, category)
    }

    /// Get list of changed files between two git refs.
    async fn get_changed_files(
        &self,
        git_ref: &str,
        base_ref: &str,
    ) -> Result<Vec<String>, OrchestratorError> {
        let output = tokio::process::Command::new("git")
            .args([
                "-C",
                self.config.repo_path.to_str().unwrap_or("."),
                "diff",
                "--name-only",
                base_ref,
                git_ref,
            ])
            .env_remove("GIT_INDEX_FILE")
            .output()
            .await
            .map_err(|e| {
                OrchestratorError::CompoundReviewFailed(format!("git diff failed: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OrchestratorError::CompoundReviewFailed(format!(
                "git diff returned non-zero: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.to_string())
            .collect();

        Ok(files)
    }

    /// Check if the compound review is in dry-run mode.
    pub fn is_dry_run(&self) -> bool {
        !self.config.create_prs
    }
}

/// Result from a single agent execution.
enum AgentResult {
    Success(ReviewAgentOutput),
    Failed { agent_name: String, reason: String },
}

/// Run a single review agent.
async fn run_single_agent(
    group: &ReviewGroupDef,
    worktree_path: &Path,
    changed_files: &[String],
    _correlation_id: Uuid,
    timeout: Duration,
    cli_tool: &str,
) -> AgentResult {
    let agent_name = &group.agent_name;

    // Use embedded prompt content (no filesystem access needed)
    let prompt = group.prompt_content;

    // Build the command with CLI-specific argument formatting
    let mut cmd = tokio::process::Command::new(cli_tool);

    // Determine CLI name for argument format selection
    let cli_name = std::path::Path::new(cli_tool)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cli_tool);

    match cli_name {
        "opencode" => {
            cmd.arg("run").arg("--format").arg("json");
            if let Some(ref model) = group.model {
                cmd.arg("-m").arg(model);
            }
            cmd.arg(prompt);
        }
        "claude" | "claude-code" => {
            cmd.arg("-p").arg(prompt);
            if let Some(ref model) = group.model {
                cmd.arg("--model").arg(model);
            }
        }
        "codex" => {
            cmd.arg("exec").arg("--full-auto");
            if let Some(ref model) = group.model {
                cmd.arg("-m").arg(model);
            }
            cmd.arg(prompt);
        }
        _ => {
            cmd.arg(prompt);
        }
    }
    cmd.current_dir(worktree_path);

    // Add changed files as arguments
    for file in changed_files {
        cmd.arg(file);
    }

    debug!(
        agent = %agent_name,
        command = ?cmd,
        "spawning review agent"
    );

    // Run with timeout
    let result = tokio::time::timeout(timeout, cmd.output()).await;

    match result {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let review_output = extract_review_output(&stdout, agent_name, group.category);
            AgentResult::Success(review_output)
        }
        Ok(Err(e)) => AgentResult::Failed {
            agent_name: agent_name.clone(),
            reason: format!("command execution failed: {}", e),
        },
        Err(_) => AgentResult::Failed {
            agent_name: agent_name.clone(),
            reason: "timeout exceeded".to_string(),
        },
    }
}

/// Extract ReviewAgentOutput from agent stdout.
/// Scans stdout for JSON matching ReviewAgentOutput schema.
/// Graceful fallback: empty output with pass: true if no valid JSON found.
fn extract_review_output(
    stdout: &str,
    agent_name: &str,
    _category: FindingCategory,
) -> ReviewAgentOutput {
    // Try to find JSON objects in stdout
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try to parse as ReviewAgentOutput
        if let Ok(output) = serde_json::from_str::<ReviewAgentOutput>(trimmed) {
            return output;
        }

        // Try to parse inside markdown code blocks
        if trimmed.starts_with("```json") {
            let json_content = trimmed
                .strip_prefix("```json")
                .and_then(|s| s.strip_suffix("```"))
                .or_else(|| {
                    trimmed
                        .strip_prefix("```json")
                        .map(|s| s.trim_end_matches("```"))
                });

            if let Some(content) = json_content {
                let clean_content = content.trim();
                if let Ok(output) = serde_json::from_str::<ReviewAgentOutput>(clean_content) {
                    return output;
                }
            }
        }
    }

    // Fallback: try to parse entire stdout as JSON
    if let Ok(output) = serde_json::from_str::<ReviewAgentOutput>(stdout) {
        return output;
    }

    // No parseable output means agent did not produce a valid review
    ReviewAgentOutput {
        agent: agent_name.to_string(),
        findings: vec![],
        summary: "No structured output found in agent response".to_string(),
        pass: false,
    }
}

/// Check if there are visual/design changes in the changed files.
fn has_visual_changes(changed_files: &[String]) -> bool {
    let visual_patterns = get_visual_patterns();

    for file in changed_files {
        for pattern in &visual_patterns {
            if glob_matches(file, pattern) {
                return true;
            }
        }
    }

    false
}

/// Get visual file detection patterns.
fn get_visual_patterns() -> Vec<&'static str> {
    vec![
        "*.css",
        "*.scss",
        "tokens.*",
        "DESIGN.md",
        "*.svelte",
        "*.tsx",
        "*.vue",
        "src/components/*",
        "src/ui/*",
        "design-system/*",
    ]
}

/// Check if a file path matches a glob pattern.
/// Supports: *.ext, prefix.*, directory/*, exact matches
fn glob_matches(file: &str, pattern: &str) -> bool {
    // Exact match
    if file == pattern {
        return true;
    }

    // Extension pattern: *.css
    if pattern.starts_with("*.") {
        let ext = &pattern[1..]; // .css
        if file.ends_with(ext) {
            return true;
        }
    }

    // Prefix pattern with wildcard: tokens.*
    if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 1]; // tokens.
        if file.starts_with(prefix) {
            return true;
        }
    }

    // Directory pattern: src/components/*
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 1]; // src/components/
        if file.starts_with(prefix) {
            return true;
        }
    }

    // Prefix pattern without wildcard
    if pattern.ends_with('/') && file.starts_with(pattern) {
        return true;
    }

    false
}

/// Get the default 6 review groups.
fn default_groups() -> Vec<ReviewGroupDef> {
    vec![
        ReviewGroupDef {
            agent_name: "security-sentinel".to_string(),
            category: FindingCategory::Security,
            llm_tier: "Quick".to_string(),
            cli_tool: "opencode".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-security.md".to_string(),
            prompt_content: PROMPT_SECURITY,
            visual_only: false,
            persona: Some("Vigil".to_string()),
        },
        ReviewGroupDef {
            agent_name: "architecture-strategist".to_string(),
            category: FindingCategory::Architecture,
            llm_tier: "Deep".to_string(),
            cli_tool: "claude".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-architecture.md"
                .to_string(),
            prompt_content: PROMPT_ARCHITECTURE,
            visual_only: false,
            persona: Some("Carthos".to_string()),
        },
        ReviewGroupDef {
            agent_name: "performance-oracle".to_string(),
            category: FindingCategory::Performance,
            llm_tier: "Deep".to_string(),
            cli_tool: "claude".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-performance.md"
                .to_string(),
            prompt_content: PROMPT_PERFORMANCE,
            visual_only: false,
            persona: Some("Ferrox".to_string()),
        },
        ReviewGroupDef {
            agent_name: "rust-reviewer".to_string(),
            category: FindingCategory::Quality,
            llm_tier: "Deep".to_string(),
            cli_tool: "claude".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-quality.md".to_string(),
            prompt_content: PROMPT_QUALITY,
            visual_only: false,
            persona: Some("Ferrox".to_string()),
        },
        ReviewGroupDef {
            agent_name: "domain-model-reviewer".to_string(),
            category: FindingCategory::Domain,
            llm_tier: "Quick".to_string(),
            cli_tool: "opencode".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-domain.md".to_string(),
            prompt_content: PROMPT_DOMAIN,
            visual_only: false,
            persona: Some("Carthos".to_string()),
        },
        ReviewGroupDef {
            agent_name: "design-fidelity-reviewer".to_string(),
            category: FindingCategory::DesignQuality,
            llm_tier: "Deep".to_string(),
            cli_tool: "claude".to_string(),
            model: None,
            prompt_template: "crates/terraphim_orchestrator/prompts/review-design-quality.md"
                .to_string(),
            prompt_content: PROMPT_DESIGN_QUALITY,
            visual_only: true,
            persona: Some("Lux".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use terraphim_symphony::runner::protocol::FindingSeverity;

    // ==================== Visual File Detection Tests ====================

    #[test]
    fn test_visual_file_detection_css() {
        let files = vec!["styles.css".to_string()];
        assert!(has_visual_changes(&files));
    }

    #[test]
    fn test_visual_file_detection_tsx() {
        let files = vec!["src/components/Button.tsx".to_string()];
        assert!(has_visual_changes(&files));
    }

    #[test]
    fn test_visual_file_detection_design_md() {
        let files = vec!["DESIGN.md".to_string()];
        assert!(has_visual_changes(&files));
    }

    #[test]
    fn test_visual_file_detection_rust_only() {
        let files = vec!["src/main.rs".to_string(), "src/lib.rs".to_string()];
        assert!(!has_visual_changes(&files));
    }

    #[test]
    fn test_visual_file_detection_component_dir() {
        let files = vec!["src/components/mod.rs".to_string()];
        assert!(has_visual_changes(&files));
    }

    #[test]
    fn test_visual_file_detection_tokens() {
        let files = vec!["tokens.json".to_string()];
        assert!(has_visual_changes(&files));
    }

    // ==================== Extract Review Output Tests ====================

    #[test]
    fn test_extract_review_output_valid_json() {
        let json = r#"{"agent":"test-agent","findings":[],"summary":"All good","pass":true}"#;
        let output = extract_review_output(json, "test-agent", FindingCategory::Quality);
        assert_eq!(output.agent, "test-agent");
        assert!(output.pass);
        assert_eq!(output.findings.len(), 0);
    }

    #[test]
    fn test_extract_review_output_mixed_output() {
        let mixed = r#"Some log output here
{"agent":"test-agent","findings":[{"file":"src/lib.rs","line":42,"severity":"high","category":"security","finding":"Test issue","confidence":0.9}],"summary":"Found 1 issue","pass":false}
More logs..."#;
        let output = extract_review_output(mixed, "test-agent", FindingCategory::Security);
        assert_eq!(output.agent, "test-agent");
        assert!(!output.pass);
        assert_eq!(output.findings.len(), 1);
        assert_eq!(output.findings[0].severity, FindingSeverity::High);
    }

    #[test]
    fn test_extract_review_output_no_json() {
        let no_json = "Just some plain text output without JSON";
        let output = extract_review_output(no_json, "test-agent", FindingCategory::Quality);
        assert_eq!(output.agent, "test-agent");
        assert!(!output.pass); // Unparseable output treated as failure
        assert_eq!(output.findings.len(), 0);
    }

    #[test]
    fn test_extract_review_output_markdown_code_block() {
        let markdown = r#"Here's my review:

```json
{"agent":"test-agent","findings":[],"summary":"No issues","pass":true}
```

Done!"#;
        let output = extract_review_output(markdown, "test-agent", FindingCategory::Quality);
        assert_eq!(output.agent, "test-agent");
        assert!(output.pass);
    }

    // ==================== Default Groups Tests ====================

    #[test]
    fn test_default_groups_count() {
        let groups = default_groups();
        assert_eq!(groups.len(), 6);
    }

    #[test]
    fn test_default_groups_one_visual_only() {
        let groups = default_groups();
        let visual_only_count = groups.iter().filter(|g| g.visual_only).count();
        assert_eq!(visual_only_count, 1);

        // Verify it's the design-fidelity-reviewer
        let visual_group = groups.iter().find(|g| g.visual_only).unwrap();
        assert_eq!(visual_group.agent_name, "design-fidelity-reviewer");
        assert_eq!(visual_group.category, FindingCategory::DesignQuality);
    }

    #[test]
    fn test_default_groups_categories() {
        let groups = default_groups();
        let categories: Vec<_> = groups.iter().map(|g| g.category).collect();

        assert!(categories.contains(&FindingCategory::Security));
        assert!(categories.contains(&FindingCategory::Architecture));
        assert!(categories.contains(&FindingCategory::Performance));
        assert!(categories.contains(&FindingCategory::Quality));
        assert!(categories.contains(&FindingCategory::Domain));
        assert!(categories.contains(&FindingCategory::DesignQuality));
    }

    // ==================== Glob Matching Tests ====================

    #[test]
    fn test_glob_matches_extension() {
        assert!(glob_matches("styles.css", "*.css"));
        assert!(glob_matches("app.scss", "*.scss"));
        assert!(glob_matches("Component.tsx", "*.tsx"));
        assert!(!glob_matches("main.rs", "*.css"));
    }

    #[test]
    fn test_glob_matches_directory() {
        assert!(glob_matches("src/components/Button.rs", "src/components/*"));
        assert!(glob_matches("src/ui/mod.rs", "src/ui/*"));
        assert!(!glob_matches("src/main.rs", "src/components/*"));
    }

    #[test]
    fn test_glob_matches_exact() {
        assert!(glob_matches("DESIGN.md", "DESIGN.md"));
        assert!(!glob_matches("README.md", "DESIGN.md"));
    }

    #[test]
    fn test_glob_matches_design_system() {
        assert!(glob_matches("design-system/tokens.css", "design-system/*"));
        assert!(glob_matches(
            "design-system/components/button.css",
            "design-system/*"
        ));
    }

    // ==================== Compound Review Integration Tests ====================

    #[tokio::test]
    async fn test_compound_review_dry_run() {
        let swarm_config = SwarmConfig {
            groups: default_groups(),
            timeout: Duration::from_secs(60),
            worktree_root: std::env::temp_dir().join("test-compound-review-worktrees"),
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            create_prs: false,
        };

        let workflow = CompoundReviewWorkflow::new(swarm_config);
        assert!(workflow.is_dry_run());
    }

    #[tokio::test]
    async fn test_get_changed_files_real_repo() {
        let swarm_config = SwarmConfig {
            groups: default_groups(),
            timeout: Duration::from_secs(60),
            worktree_root: std::env::temp_dir().join("test-compound-review-worktrees"),
            repo_path: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            create_prs: false,
        };

        let workflow = CompoundReviewWorkflow::new(swarm_config);

        // Test with HEAD vs HEAD~1 (should work in any repo with history)
        let result = workflow.get_changed_files("HEAD", "HEAD~1").await;

        // The result may fail if there's no history, but it should not panic
        match result {
            Ok(files) => {
                // If we have files, they should be valid paths
                for file in &files {
                    assert!(!file.is_empty());
                }
            }
            Err(_) => {
                // Error is acceptable in test environment without proper git setup
            }
        }
    }

    #[test]
    fn test_swarm_config_from_compound_config() {
        let compound_config = CompoundReviewConfig {
            schedule: "0 2 * * *".to_string(),
            max_duration_secs: 1800,
            repo_path: PathBuf::from("/tmp/repo"),
            create_prs: false,
            worktree_root: PathBuf::from("/tmp/worktrees"),
            base_branch: "main".to_string(),
            max_concurrent_agents: 3,
            cli_tool: None,
            provider: None,
            model: None,
        };

        let swarm_config = SwarmConfig::from_compound_config(&compound_config);

        assert_eq!(swarm_config.repo_path, PathBuf::from("/tmp/repo"));
        assert_eq!(swarm_config.worktree_root, PathBuf::from("/tmp/worktrees"));
        assert_eq!(swarm_config.base_branch, "main");
        assert_eq!(swarm_config.max_concurrent_agents, 3);
        assert!(!swarm_config.create_prs);
        assert_eq!(swarm_config.groups.len(), 6);
    }

    #[test]
    fn test_compound_review_result_structure() {
        let result = CompoundReviewResult {
            correlation_id: Uuid::new_v4(),
            findings: vec![],
            agent_outputs: vec![],
            pass: true,
            duration: Duration::from_secs(10),
            agents_run: 6,
            agents_failed: 0,
        };

        assert!(result.pass);
        assert_eq!(result.agents_run, 6);
        assert_eq!(result.agents_failed, 0);
    }

    // ==================== Persona Identity Tests ====================

    #[test]
    fn test_review_security_contains_vigil() {
        let prompt = include_str!("../prompts/review-security.md");
        assert!(
            prompt.contains("Vigil"),
            "review-security.md should contain 'Vigil'"
        );
        assert!(
            prompt.contains("Security Engineer"),
            "review-security.md should mention Security Engineer"
        );
    }

    #[test]
    fn test_review_architecture_contains_carthos() {
        let prompt = include_str!("../prompts/review-architecture.md");
        assert!(
            prompt.contains("Carthos"),
            "review-architecture.md should contain 'Carthos'"
        );
        assert!(
            prompt.contains("Domain Architect"),
            "review-architecture.md should mention Domain Architect"
        );
    }

    #[test]
    fn test_review_quality_contains_ferrox() {
        let prompt = include_str!("../prompts/review-quality.md");
        assert!(
            prompt.contains("Ferrox"),
            "review-quality.md should contain 'Ferrox'"
        );
        assert!(
            prompt.contains("Rust Engineer"),
            "review-quality.md should mention Rust Engineer"
        );
    }

    #[test]
    fn test_review_performance_contains_ferrox() {
        let prompt = include_str!("../prompts/review-performance.md");
        assert!(
            prompt.contains("Ferrox"),
            "review-performance.md should contain 'Ferrox'"
        );
        assert!(
            prompt.contains("Rust Engineer"),
            "review-performance.md should mention Rust Engineer"
        );
    }

    #[test]
    fn test_review_domain_contains_carthos() {
        let prompt = include_str!("../prompts/review-domain.md");
        assert!(
            prompt.contains("Carthos"),
            "review-domain.md should contain 'Carthos'"
        );
        assert!(
            prompt.contains("Domain Architect"),
            "review-domain.md should mention Domain Architect"
        );
    }

    #[test]
    fn test_review_design_contains_lux() {
        let prompt = include_str!("../prompts/review-design-quality.md");
        assert!(
            prompt.contains("Lux"),
            "review-design-quality.md should contain 'Lux'"
        );
        assert!(
            prompt.contains("TypeScript Engineer"),
            "review-design-quality.md should mention TypeScript Engineer"
        );
    }

    #[test]
    fn test_default_groups_all_have_persona() {
        let groups = default_groups();
        for group in &groups {
            assert!(
                group.persona.is_some(),
                "Group '{}' should have a persona set",
                group.agent_name
            );
        }

        // Verify specific persona mappings
        let vigil = groups
            .iter()
            .find(|g| g.agent_name == "security-sentinel")
            .unwrap();
        assert_eq!(vigil.persona.as_ref().unwrap(), "Vigil");

        let carthos_arch = groups
            .iter()
            .find(|g| g.agent_name == "architecture-strategist")
            .unwrap();
        assert_eq!(carthos_arch.persona.as_ref().unwrap(), "Carthos");

        let ferrox_perf = groups
            .iter()
            .find(|g| g.agent_name == "performance-oracle")
            .unwrap();
        assert_eq!(ferrox_perf.persona.as_ref().unwrap(), "Ferrox");

        let ferrox_qual = groups
            .iter()
            .find(|g| g.agent_name == "rust-reviewer")
            .unwrap();
        assert_eq!(ferrox_qual.persona.as_ref().unwrap(), "Ferrox");

        let carthos_domain = groups
            .iter()
            .find(|g| g.agent_name == "domain-model-reviewer")
            .unwrap();
        assert_eq!(carthos_domain.persona.as_ref().unwrap(), "Carthos");

        let lux = groups
            .iter()
            .find(|g| g.agent_name == "design-fidelity-reviewer")
            .unwrap();
        assert_eq!(lux.persona.as_ref().unwrap(), "Lux");
    }

    #[test]
    fn test_extract_review_output_with_persona_agent_name() {
        // Verify JSON output still parses when agent name includes persona
        let json = r#"{"agent":"Vigil-security-sentinel","findings":[{"file":"src/lib.rs","line":42,"severity":"high","category":"security","finding":"Test issue","confidence":0.9}],"summary":"Found 1 security issue","pass":false}"#;
        let output =
            extract_review_output(json, "Vigil-security-sentinel", FindingCategory::Security);
        assert_eq!(output.agent, "Vigil-security-sentinel");
        assert!(!output.pass);
        assert_eq!(output.findings.len(), 1);
    }
}
