use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

use super::models::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HookDecision {
    Allow,
    Block { reason: String },
    Modify { transformed_code: String },
    AskUser { prompt: String },
}

#[derive(Debug, Clone)]
pub struct PreToolContext {
    pub code: String,
    pub language: String,
    pub agent_id: String,
    pub vm_id: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct PostToolContext {
    pub original_code: String,
    pub output: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub agent_id: String,
    pub vm_id: String,
}

#[derive(Debug, Clone)]
pub struct PreLlmContext {
    pub prompt: String,
    pub agent_id: String,
    pub conversation_history: Vec<String>,
    pub token_count: usize,
}

#[derive(Debug, Clone)]
pub struct PostLlmContext {
    pub prompt: String,
    pub response: String,
    pub agent_id: String,
    pub token_count: usize,
    pub model: String,
}

#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &str;

    async fn pre_tool(&self, _context: &PreToolContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    async fn post_tool(
        &self,
        _context: &PostToolContext,
    ) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    async fn pre_llm(&self, _context: &PreLlmContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }

    async fn post_llm(&self, _context: &PostLlmContext) -> Result<HookDecision, VmExecutionError> {
        Ok(HookDecision::Allow)
    }
}

pub struct HookManager {
    hooks: Vec<Arc<dyn Hook>>,
}

impl HookManager {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    pub fn add_hook(&mut self, hook: Arc<dyn Hook>) {
        info!("Registered hook: {}", hook.name());
        self.hooks.push(hook);
    }

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

pub struct DangerousPatternHook {
    patterns: Vec<regex::Regex>,
}

impl DangerousPatternHook {
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

pub struct SyntaxValidationHook {
    supported_languages: Vec<String>,
}

impl SyntaxValidationHook {
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

pub struct DependencyInjectorHook {
    inject_imports: bool,
}

impl DependencyInjectorHook {
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
