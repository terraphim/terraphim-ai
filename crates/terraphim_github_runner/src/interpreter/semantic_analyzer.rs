//! Semantic analysis of actions using LLM

use crate::{InterpretedAction, RunnerResult, RunnerError, LlmConfig};

/// Semantic analyzer for understanding action purpose
pub struct SemanticAnalyzer {
    /// LLM configuration
    llm_config: Option<LlmConfig>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new() -> Self {
        Self { llm_config: None }
    }

    /// Configure LLM for analysis
    pub fn with_llm(mut self, provider: &str, model: &str, base_url: Option<&str>) -> Self {
        self.llm_config = Some(LlmConfig {
            provider: provider.to_string(),
            model: model.to_string(),
            base_url: base_url.map(|s| s.to_string()),
            api_key: None,
        });
        self
    }

    /// Set API key for LLM
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        if let Some(config) = &mut self.llm_config {
            config.api_key = Some(api_key.to_string());
        }
        self
    }

    /// Check if LLM is configured
    pub fn has_llm(&self) -> bool {
        self.llm_config.is_some()
    }

    /// Analyze an action using LLM
    pub async fn analyze_action(
        &self,
        action_ref: &str,
        base_interpretation: &InterpretedAction,
    ) -> RunnerResult<InterpretedAction> {
        let config = self.llm_config.as_ref().ok_or_else(|| {
            RunnerError::LlmInterpretation("LLM not configured".to_string())
        })?;

        // Build prompt for semantic analysis
        let prompt = format!(
            r#"Analyze this GitHub Action and provide semantic understanding:

Action: {}
Current interpretation:
- Purpose: {}
- Commands: {:?}
- Prerequisites: {:?}
- Produces: {:?}

Enhance this interpretation with:
1. More accurate purpose description
2. Any missing prerequisites
3. Any missing outputs/artifacts
4. Whether this is deterministic (cacheable)

Respond in JSON format:
{{
  "purpose": "string",
  "prerequisites": ["string"],
  "produces": ["string"],
  "cacheable": boolean,
  "confidence": number
}}"#,
            action_ref,
            base_interpretation.purpose,
            base_interpretation.commands,
            base_interpretation.prerequisites,
            base_interpretation.produces
        );

        // In a real implementation, this would call the LLM API
        // For now, return an enhanced version of the base interpretation
        log::debug!("LLM analysis prompt: {}", prompt);
        log::debug!("Using LLM provider: {} model: {}", config.provider, config.model);

        // Placeholder: return base interpretation with higher confidence
        // Real implementation would parse LLM response
        Ok(InterpretedAction {
            original: base_interpretation.original.clone(),
            purpose: base_interpretation.purpose.clone(),
            prerequisites: base_interpretation.prerequisites.clone(),
            produces: base_interpretation.produces.clone(),
            cacheable: base_interpretation.cacheable,
            commands: base_interpretation.commands.clone(),
            required_env: base_interpretation.required_env.clone(),
            kg_terms: base_interpretation.kg_terms.clone(),
            confidence: 0.9, // Enhanced confidence after LLM analysis
        })
    }

    /// Analyze a shell command for semantic understanding
    pub async fn analyze_command(&self, command: &str) -> RunnerResult<InterpretedAction> {
        let config = self.llm_config.as_ref().ok_or_else(|| {
            RunnerError::LlmInterpretation("LLM not configured".to_string())
        })?;

        let prompt = format!(
            r#"Analyze this shell command for CI/CD execution:

Command: {}

Provide:
1. Purpose of the command
2. Prerequisites (tools, files, env vars needed)
3. What it produces (files, artifacts, outputs)
4. Whether output is deterministic (cacheable)

Respond in JSON format:
{{
  "purpose": "string",
  "prerequisites": ["string"],
  "produces": ["string"],
  "cacheable": boolean
}}"#,
            command
        );

        log::debug!("LLM command analysis prompt: {}", prompt);
        log::debug!("Using LLM provider: {} model: {}", config.provider, config.model);

        // Placeholder response
        Ok(InterpretedAction {
            original: command.to_string(),
            purpose: format!("Execute: {}", command.split_whitespace().next().unwrap_or("command")),
            prerequisites: Vec::new(),
            produces: Vec::new(),
            cacheable: false,
            commands: vec![command.to_string()],
            required_env: Vec::new(),
            kg_terms: Vec::new(),
            confidence: 0.7,
        })
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
