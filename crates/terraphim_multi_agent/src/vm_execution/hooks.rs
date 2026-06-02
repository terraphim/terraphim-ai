use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::models::*;

/// Decision returned by a hook after inspecting tool or LLM invocations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookDecision {
    /// Allow the operation to proceed unchanged.
    Allow,
    /// Block the operation, providing a human-readable reason.
    Block {
        /// Explanation of why the operation was blocked.
        reason: String,
    },
    /// Allow the operation but replace the submitted code with a transformed version.
    Modify {
        /// The replacement code to execute instead of the original.
        transformed_code: String,
    },
    /// Pause execution and ask the user for guidance before proceeding.
    AskUser {
        /// The question or prompt to present to the user.
        prompt: String,
    },
}

/// Context passed to pre-tool hooks before code is executed in a VM.
#[derive(Debug, Clone)]
pub struct PreToolContext {
    /// Source code submitted for execution.
    pub code: String,
    /// Programming language of the submitted code (e.g. `"python"`, `"bash"`).
    pub language: String,
    /// Identifier of the agent requesting execution.
    pub agent_id: String,
    /// Identifier of the target VM.
    pub vm_id: String,
    /// Arbitrary key-value metadata attached to the request.
    pub metadata: std::collections::HashMap<String, String>,
}

/// Context passed to post-tool hooks after code has been executed in a VM.
#[derive(Debug, Clone)]
pub struct PostToolContext {
    /// The original source code that was executed.
    pub original_code: String,
    /// Captured standard output (and stderr) from the execution.
    pub output: String,
    /// Process exit code; `0` conventionally indicates success.
    pub exit_code: i32,
    /// Wall-clock execution time in milliseconds.
    pub duration_ms: u64,
    /// Identifier of the agent that requested execution.
    pub agent_id: String,
    /// Identifier of the VM where execution ran.
    pub vm_id: String,
}

/// Context passed to pre-LLM hooks before a prompt is sent to a language model.
#[derive(Debug, Clone)]
pub struct PreLlmContext {
    /// The prompt text about to be submitted.
    pub prompt: String,
    /// Identifier of the agent issuing the LLM request.
    pub agent_id: String,
    /// Prior conversation turns included in the request.
    pub conversation_history: Vec<String>,
    /// Estimated token count for the full request.
    pub token_count: usize,
}

/// Context passed to post-LLM hooks after a language model has responded.
#[derive(Debug, Clone)]
pub struct PostLlmContext {
    /// The prompt that was submitted.
    pub prompt: String,
    /// The model's response text.
    pub response: String,
    /// Identifier of the agent that made the LLM request.
    pub agent_id: String,
    /// Total tokens consumed by the request and response.
    pub token_count: usize,
    /// Name or identifier of the model that generated the response.
    pub model: String,
}

/// Trait implemented by all hooks that intercept VM tool and LLM invocations.
///
/// Each method has a default implementation that returns [`HookDecision::Allow`],
/// so implementors only need to override the phases they care about.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Returns the unique name of this hook, used in log messages.
    fn name(&self) -> &str;

    /// Called before code is executed in a VM.
    ///
    /// Returns a decision indicating whether to allow, block, or modify the
    /// execution.
    async fn pre_tool(&self, _context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    /// Called after code has been executed in a VM.
    ///
    /// Returns a decision that may, for example, block the output from being
    /// returned to the caller.
    async fn post_tool(
        &self,
        _context: &PostToolContext,
    ) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    /// Called before a prompt is sent to a language model.
    ///
    /// Returns a decision that may block or modify the prompt.
    async fn pre_llm(&self, _context: &PreLlmContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    /// Called after a language model has produced a response.
    ///
    /// Returns a decision that may, for example, block a response that
    /// contains sensitive information.
    async fn post_llm(&self, _context: &PostLlmContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }
}

/// Manages an ordered list of hooks and dispatches execution lifecycle events to them.
///
/// Hooks are evaluated in registration order. The first non-`Allow` decision
/// short-circuits the chain and is returned immediately.
pub struct HookManager {
    hooks: Vec<Arc<dyn Hook>>,
}

impl std::fmt::Debug for HookManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookManager")
            .field("hook_count", &self.hooks.len())
            .finish()
    }
}

impl HookManager {
    /// Creates a new `HookManager` with no registered hooks.
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Registers a hook, appending it to the end of the chain.
    pub fn add_hook(&mut self, hook: Arc<dyn Hook>) {
        info!("Registered hook: {}", hook.name());
        self.hooks.push(hook);
    }

    /// Runs all pre-tool hooks against `context` in registration order.
    ///
    /// Returns on the first non-`Allow` decision, or `Allow` if all hooks pass.
    pub async fn run_pre_tool(
        &self,
        context: &PreToolContext,
    ) -> Result<HookDecision, VmExecutionError> {
        for hook in &self.hooks {
            debug!("Running pre-tool hook: {}", hook.name());

            match hook.pre_tool(context).await? {
                HookDecision::Allow => continue,
                decision => {
                    info!("Hook {} returned decision: {:?}", hook.name(), decision);
                    return Ok(decision);
                }
            }
        }

        Ok(HookDecision::Allow)
    }

    /// Runs all post-tool hooks against `context` in registration order.
    ///
    /// Returns on the first non-`Allow` decision, or `Allow` if all hooks pass.
    pub async fn run_post_tool(
        &self,
        context: &PostToolContext,
    ) -> Result<HookDecision, VmExecutionError> {
        for hook in &self.hooks {
            debug!("Running post-tool hook: {}", hook.name());

            match hook.post_tool(context).await? {
                HookDecision::Allow => continue,
                decision => {
                    info!("Hook {} returned decision: {:?}", hook.name(), decision);
                    return Ok(decision);
                }
            }
        }

        Ok(HookDecision::Allow)
    }

    /// Runs all pre-LLM hooks against `context` in registration order.
    ///
    /// Returns on the first non-`Allow` decision, or `Allow` if all hooks pass.
    pub async fn run_pre_llm(
        &self,
        context: &PreLlmContext,
    ) -> Result<HookDecision, VmExecutionError> {
        for hook in &self.hooks {
            debug!("Running pre-LLM hook: {}", hook.name());

            match hook.pre_llm(context).await? {
                HookDecision::Allow => continue,
                decision => {
                    info!("Hook {} returned decision: {:?}", hook.name(), decision);
                    return Ok(decision);
                }
            }
        }

        Ok(HookDecision::Allow)
    }

    /// Runs all post-LLM hooks against `context` in registration order.
    ///
    /// Returns on the first non-`Allow` decision, or `Allow` if all hooks pass.
    pub async fn run_post_llm(
        &self,
        context: &PostLlmContext,
    ) -> Result<HookDecision, VmExecutionError> {
        for hook in &self.hooks {
            debug!("Running post-LLM hook: {}", hook.name());

            match hook.post_llm(context).await? {
                HookDecision::Allow => continue,
                decision => {
                    info!("Hook {} returned decision: {:?}", hook.name(), decision);
                    return Ok(decision);
                }
            }
        }

        Ok(HookDecision::Allow)
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-tool hook that blocks code containing known-dangerous shell patterns.
///
/// Patterns include destructive commands such as `rm -rf`, fork bombs, and
/// pipe-to-shell download patterns.
pub struct DangerousPatternHook {
    patterns: Vec<regex::Regex>,
}

impl DangerousPatternHook {
    /// Creates a new `DangerousPatternHook` with the built-in set of dangerous patterns.
    pub fn new() -> Self {
        let patterns = vec![
            regex::Regex::new(r"rm\s+-rf").unwrap(),
            regex::Regex::new(r"format\s+c:").unwrap(),
            regex::Regex::new(r"mkfs\.").unwrap(),
            regex::Regex::new(r"dd\s+if=").unwrap(),
            regex::Regex::new(r":\(\)\{\s*:\|:&\s*\}").unwrap(),
            regex::Regex::new(r"curl.*\|.*sh").unwrap(),
            regex::Regex::new(r"wget.*\|.*sh").unwrap(),
        ];

        Self { patterns }
    }
}

#[async_trait]
impl Hook for DangerousPatternHook {
    fn name(&self) -> &str {
        "dangerous_pattern"
    }

    async fn pre_tool(&self, context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        let code_lower = context.code.to_lowercase();

        for pattern in &self.patterns {
            if pattern.is_match(&code_lower) {
                warn!(
                    "Dangerous pattern detected in code: {} (agent: {})",
                    pattern.as_str(),
                    context.agent_id
                );

                return Ok(HookDecision::Block {
                    reason: format!(
                        "Code contains potentially dangerous pattern: {}",
                        pattern.as_str()
                    ),
                });
            }
        }

        Ok(HookDecision::Allow)
    }
}

impl Default for DangerousPatternHook {
    fn default() -> Self {
        Self::new()
    }
}

/// Pre-tool hook that validates that submitted code meets basic syntactic requirements.
///
/// Checks include: language is in the supported set, code is non-empty, and code
/// does not exceed the maximum allowed length.
pub struct SyntaxValidationHook {
    supported_languages: Vec<String>,
}

impl SyntaxValidationHook {
    /// Creates a new `SyntaxValidationHook` supporting Python, JavaScript, Bash, and Rust.
    pub fn new() -> Self {
        Self {
            supported_languages: vec![
                "python".to_string(),
                "javascript".to_string(),
                "bash".to_string(),
                "rust".to_string(),
            ],
        }
    }
}

#[async_trait]
impl Hook for SyntaxValidationHook {
    fn name(&self) -> &str {
        "syntax_validation"
    }

    async fn pre_tool(&self, context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        if !self.supported_languages.contains(&context.language) {
            return Ok(HookDecision::Block {
                reason: format!("Unsupported language: {}", context.language),
            });
        }

        if context.code.trim().is_empty() {
            return Ok(HookDecision::Block {
                reason: "Code cannot be empty".to_string(),
            });
        }

        if context.code.len() > 100000 {
            return Ok(HookDecision::Block {
                reason: "Code exceeds maximum length of 100,000 characters".to_string(),
            });
        }

        Ok(HookDecision::Allow)
    }
}

impl Default for SyntaxValidationHook {
    fn default() -> Self {
        Self::new()
    }
}

/// Hook that logs execution lifecycle events at the `INFO` level.
///
/// Emits a message before execution starts (pre-tool) and after it finishes
/// (post-tool), recording language, agent, VM, exit code, and duration.
pub struct ExecutionLoggerHook;

#[async_trait]
impl Hook for ExecutionLoggerHook {
    fn name(&self) -> &str {
        "execution_logger"
    }

    async fn pre_tool(&self, context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        info!(
            "Executing {} code for agent {} on VM {}",
            context.language, context.agent_id, context.vm_id
        );
        Ok(HookDecision::Allow)
    }

    async fn post_tool(&self, context: &PostToolContext) -> Result<HookDecision, VmExecutionError> {
        info!(
            "Execution completed for agent {} with exit code {} ({}ms)",
            context.agent_id, context.exit_code, context.duration_ms
        );
        Ok(HookDecision::Allow)
    }
}

/// Pre-tool hook that automatically injects common import statements into code
/// that does not already include them.
///
/// Currently supports Python (`sys`/`os`) and JavaScript (comment placeholder).
/// When injection is disabled at construction time the hook always returns `Allow`.
pub struct DependencyInjectorHook {
    inject_imports: bool,
}

impl DependencyInjectorHook {
    /// Creates a new `DependencyInjectorHook`.
    ///
    /// When `inject_imports` is `false` the hook is effectively a no-op.
    pub fn new(inject_imports: bool) -> Self {
        Self { inject_imports }
    }
}

#[async_trait]
impl Hook for DependencyInjectorHook {
    fn name(&self) -> &str {
        "dependency_injector"
    }

    async fn pre_tool(&self, context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        if !self.inject_imports {
            return Ok(HookDecision::Allow);
        }

        let transformed = match context.language.as_str() {
            "python" => {
                if !context.code.contains("import ") {
                    format!("import sys\nimport os\n\n{}", context.code)
                } else {
                    context.code.clone()
                }
            }
            "javascript" => {
                if !context.code.contains("require(") && !context.code.contains("import ") {
                    format!("// Auto-injected standard modules\n{}", context.code)
                } else {
                    context.code.clone()
                }
            }
            _ => context.code.clone(),
        };

        if transformed != context.code {
            debug!("Injected dependencies for {} code", context.language);
            Ok(HookDecision::Modify {
                transformed_code: transformed,
            })
        } else {
            Ok(HookDecision::Allow)
        }
    }
}

/// Post-tool hook that blocks execution output containing sensitive information.
///
/// Scans output for passwords, API keys, secrets, tokens, and e-mail addresses
/// using regular expressions. Any match causes the output to be blocked.
pub struct OutputSanitizerHook;

#[async_trait]
impl Hook for OutputSanitizerHook {
    fn name(&self) -> &str {
        "output_sanitizer"
    }

    async fn post_tool(&self, context: &PostToolContext) -> Result<HookDecision, VmExecutionError> {
        let sensitive_patterns = vec![
            regex::Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+").unwrap(),
            regex::Regex::new(r"(?i)(api[_-]?key|apikey)\s*[:=]\s*\S+").unwrap(),
            regex::Regex::new(r"(?i)(secret|token)\s*[:=]\s*\S+").unwrap(),
            regex::Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap(),
        ];

        for pattern in &sensitive_patterns {
            if pattern.is_match(&context.output) {
                warn!(
                    "Sensitive information detected in output for agent {}",
                    context.agent_id
                );

                return Ok(HookDecision::Block {
                    reason: "Output contains potential sensitive information".to_string(),
                });
            }
        }

        Ok(HookDecision::Allow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_hook_manager() {
        let mut manager = HookManager::new();
        manager.add_hook(Arc::new(ExecutionLoggerHook));

        let context = PreToolContext {
            code: "print('test')".to_string(),
            language: "python".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = manager.run_pre_tool(&context).await.unwrap();
        assert_eq!(decision, HookDecision::Allow);
    }

    #[tokio::test]
    async fn test_dangerous_pattern_hook() {
        let hook = DangerousPatternHook::new();

        let dangerous_context = PreToolContext {
            code: "rm -rf /".to_string(),
            language: "bash".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&dangerous_context).await.unwrap();
        assert!(matches!(decision, HookDecision::Block { .. }));

        let safe_context = PreToolContext {
            code: "echo 'hello'".to_string(),
            language: "bash".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&safe_context).await.unwrap();
        assert_eq!(decision, HookDecision::Allow);
    }

    #[tokio::test]
    async fn test_syntax_validation_hook() {
        let hook = SyntaxValidationHook::new();

        let empty_context = PreToolContext {
            code: "   ".to_string(),
            language: "python".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&empty_context).await.unwrap();
        assert!(matches!(decision, HookDecision::Block { .. }));

        let valid_context = PreToolContext {
            code: "print('test')".to_string(),
            language: "python".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&valid_context).await.unwrap();
        assert_eq!(decision, HookDecision::Allow);
    }

    #[tokio::test]
    async fn test_dependency_injector_hook() {
        let hook = DependencyInjectorHook::new(true);

        let context = PreToolContext {
            code: "print('hello')".to_string(),
            language: "python".to_string(),
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
            metadata: HashMap::new(),
        };

        let decision = hook.pre_tool(&context).await.unwrap();
        assert!(matches!(decision, HookDecision::Modify { .. }));
    }

    #[tokio::test]
    async fn test_output_sanitizer_hook() {
        let hook = OutputSanitizerHook;

        let sensitive_context = PostToolContext {
            original_code: "env".to_string(),
            output: "PASSWORD=secret123".to_string(),
            exit_code: 0,
            duration_ms: 100,
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
        };

        let decision = hook.post_tool(&sensitive_context).await.unwrap();
        assert!(matches!(decision, HookDecision::Block { .. }));

        let safe_context = PostToolContext {
            original_code: "echo test".to_string(),
            output: "test".to_string(),
            exit_code: 0,
            duration_ms: 100,
            agent_id: "test-agent".to_string(),
            vm_id: "test-vm".to_string(),
        };

        let decision = hook.post_tool(&safe_context).await.unwrap();
        assert_eq!(decision, HookDecision::Allow);
    }
}
