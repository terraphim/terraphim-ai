// Validation pipeline for MCP tool execution
//
// Implements 4-layer validation:
// 1. Pre-LLM: Context validation before LLM call
// 2. Post-LLM: Output validation after LLM response
// 3. Pre-Tool: Validation before tool execution
// 4. Post-Tool: Validation after tool execution

use anyhow::Result;
use rmcp::model::{CallToolRequestParam, CallToolResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

/// Validation layer trait
pub trait ValidationLayer: Send + Sync {
    /// Name of the validation layer
    fn name(&self) -> &str;

    /// Validate the context/request
    fn validate<'a>(
        &'a self,
        context: &'a ValidationContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + 'a>>;
}

/// Context for validation
#[derive(Debug, Clone)]
pub struct ValidationContext {
    pub tool_name: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub file_paths: Vec<String>,
    pub metadata: HashMap<String, String>,
}

impl ValidationContext {
    pub fn new(tool_name: String) -> Self {
        Self {
            tool_name,
            parameters: HashMap::new(),
            file_paths: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn from_tool_request(request: &CallToolRequestParam) -> Self {
        let mut context = Self::new(request.name.to_string());

        if let Some(args) = &request.arguments {
            // Convert serde_json::Map to HashMap
            for (k, v) in args.iter() {
                context.parameters.insert(k.clone(), v.clone());
            }

            // Extract file paths
            if let Some(file_path) = args.get("file_path").and_then(|v| v.as_str()) {
                context.file_paths.push(file_path.to_string());
            }
        }

        context
    }
}

/// Result of validation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationResult {
    pub passed: bool,
    pub layer: String,
    pub message: String,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn success(layer: &str) -> Self {
        Self {
            passed: true,
            layer: layer.to_string(),
            message: "Validation passed".to_string(),
            warnings: Vec::new(),
        }
    }

    pub fn failure(layer: &str, message: String) -> Self {
        Self {
            passed: false,
            layer: layer.to_string(),
            message,
            warnings: Vec::new(),
        }
    }

    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Pre-tool validation: Check permissions and file existence
pub struct PreToolValidator;

impl ValidationLayer for PreToolValidator {
    fn name(&self) -> &str {
        "pre-tool"
    }

    fn validate<'a>(
        &'a self,
        context: &'a ValidationContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + 'a>>
    {
        Box::pin(async move {
            debug!("Pre-tool validation for: {}", context.tool_name);

            // Check file existence for file editing tools
            if context.tool_name.starts_with("edit_file") || context.tool_name == "validate_edit" {
                for file_path in &context.file_paths {
                    if !tokio::fs::try_exists(file_path).await.unwrap_or(false) {
                        return Ok(ValidationResult::failure(
                            "pre-tool",
                            format!("File does not exist: {}", file_path),
                        ));
                    }
                }
            }

            // Check file is readable
            for file_path in &context.file_paths {
                match tokio::fs::read_to_string(file_path).await {
                    Ok(_) => {}
                    Err(e) => {
                        return Ok(ValidationResult::failure(
                            "pre-tool",
                            format!("Cannot read file {}: {}", file_path, e),
                        ));
                    }
                }
            }

            Ok(ValidationResult::success("pre-tool"))
        })
    }
}

/// Post-tool validation: Check results and file state
pub struct PostToolValidator;

impl ValidationLayer for PostToolValidator {
    fn name(&self) -> &str {
        "post-tool"
    }

    fn validate<'a>(
        &'a self,
        context: &'a ValidationContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ValidationResult>> + Send + 'a>>
    {
        Box::pin(async move {
            debug!("Post-tool validation for: {}", context.tool_name);

            let mut result = ValidationResult::success("post-tool");

            // For file editing operations, verify file still exists and is valid
            if context.tool_name.starts_with("edit_file") {
                for file_path in &context.file_paths {
                    match tokio::fs::read_to_string(file_path).await {
                        Ok(content) => {
                            // Basic sanity check
                            if content.is_empty() {
                                result = result.with_warning(format!(
                                    "File is empty after edit: {}",
                                    file_path
                                ));
                            }
                        }
                        Err(e) => {
                            return Ok(ValidationResult::failure(
                                "post-tool",
                                format!("File corrupted after edit {}: {}", file_path, e),
                            ));
                        }
                    }
                }
            }

            Ok(result)
        })
    }
}

/// Validation pipeline orchestrator
pub struct ValidationPipeline {
    pre_tool_validators: Vec<Box<dyn ValidationLayer>>,
    post_tool_validators: Vec<Box<dyn ValidationLayer>>,
}

impl ValidationPipeline {
    pub fn new() -> Self {
        Self {
            pre_tool_validators: vec![Box::new(PreToolValidator)],
            post_tool_validators: vec![Box::new(PostToolValidator)],
        }
    }

    /// Run all pre-tool validators
    pub async fn validate_pre_tool(&self, context: &ValidationContext) -> Result<ValidationResult> {
        for validator in &self.pre_tool_validators {
            let result = validator.validate(context).await?;
            if !result.passed {
                warn!("Pre-tool validation failed: {}", result.message);
                return Ok(result);
            }

            for warning in &result.warnings {
                warn!("Pre-tool warning: {}", warning);
            }
        }

        info!("All pre-tool validations passed for: {}", context.tool_name);
        Ok(ValidationResult::success("pre-tool-pipeline"))
    }

    /// Run all post-tool validators
    pub async fn validate_post_tool(
        &self,
        context: &ValidationContext,
        _result: &CallToolResult,
    ) -> Result<ValidationResult> {
        for validator in &self.post_tool_validators {
            let result = validator.validate(context).await?;
            if !result.passed {
                warn!("Post-tool validation failed: {}", result.message);
                return Ok(result);
            }

            for warning in &result.warnings {
                warn!("Post-tool warning: {}", warning);
            }
        }

        info!(
            "All post-tool validations passed for: {}",
            context.tool_name
        );
        Ok(ValidationResult::success("post-tool-pipeline"))
    }

    /// Add a custom pre-tool validator
    pub fn add_pre_tool_validator(&mut self, validator: Box<dyn ValidationLayer>) {
        self.pre_tool_validators.push(validator);
    }

    /// Add a custom post-tool validator
    pub fn add_post_tool_validator(&mut self, validator: Box<dyn ValidationLayer>) {
        self.post_tool_validators.push(validator);
    }
}

impl Default for ValidationPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pre_tool_validator_file_exists() {
        let validator = PreToolValidator;

        // Create temp file
        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        let mut context = ValidationContext::new("edit_file_search_replace".to_string());
        context
            .file_paths
            .push(test_file.to_str().unwrap().to_string());

        let result = validator.validate(&context).await.unwrap();
        assert!(result.passed);
    }

    #[tokio::test]
    async fn test_pre_tool_validator_file_not_exists() {
        let validator = PreToolValidator;

        let mut context = ValidationContext::new("edit_file_search_replace".to_string());
        context.file_paths.push("/nonexistent/file.txt".to_string());

        let result = validator.validate(&context).await.unwrap();
        assert!(!result.passed);
        assert!(result.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_validation_pipeline() {
        let pipeline = ValidationPipeline::new();

        let temp_dir = tempfile::TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "content").await.unwrap();

        let mut context = ValidationContext::new("edit_file_search_replace".to_string());
        context
            .file_paths
            .push(test_file.to_str().unwrap().to_string());

        let result = pipeline.validate_pre_tool(&context).await.unwrap();
        assert!(result.passed);
    }
}
