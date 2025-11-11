use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, warn};

use super::models::*;

/// Extracts code blocks from LLM responses for potential execution
#[derive(Debug, Clone)]
pub struct CodeBlockExtractor {
    /// Patterns for detecting code blocks
    code_block_patterns: Vec<Regex>,
    /// Language-specific configurations
    language_configs: HashMap<String, LanguageConfig>,
    /// Execution intent detection patterns
    intent_patterns: Vec<Regex>,
    /// Default execution threshold
    default_threshold: f64,
}

impl CodeBlockExtractor {
    /// Create a new code block extractor with default patterns
    pub fn new() -> Self {
        let mut extractor = Self {
            code_block_patterns: Vec::new(),
            language_configs: HashMap::new(),
            intent_patterns: Vec::new(),
            default_threshold: 0.7,
        };

        extractor.initialize_patterns();
        extractor.initialize_language_configs();
        extractor.initialize_intent_patterns();
        extractor
    }

    /// Create extractor with custom threshold
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.default_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Extract code blocks from text
    pub fn extract_code_blocks(&self, text: &str) -> Vec<CodeBlock> {
        let mut code_blocks = Vec::new();

        debug!("Extracting code blocks from text (length: {})", text.len());

        // Extract fenced code blocks (```language)
        code_blocks.extend(self.extract_fenced_blocks(text));

        // Extract inline code that might be executable
        code_blocks.extend(self.extract_inline_executable_code(text));

        // Sort by execution confidence
        code_blocks.sort_by(|a, b| {
            b.execution_confidence
                .partial_cmp(&a.execution_confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        debug!("Extracted {} code blocks", code_blocks.len());
        code_blocks
    }

    /// Detect execution intent in text
    pub fn detect_execution_intent(&self, text: &str) -> ExecutionIntent {
        let text_lower = text.to_lowercase();
        let mut confidence: f64 = 0.0;
        let mut trigger_keywords = Vec::new();
        let mut context_clues = Vec::new();

        // Check for explicit execution keywords
        let execution_keywords = [
            "run this",
            "execute",
            "try this",
            "test this code",
            "run the code",
            "execute the script",
            "try running",
            "can you run",
            "please run",
            "execute this",
            "run it",
            "test it",
            "give it a try",
        ];

        for keyword in &execution_keywords {
            if text_lower.contains(keyword) {
                confidence += 0.3;
                trigger_keywords.push(keyword.to_string());
            }
        }

        // Check for question patterns about execution
        let question_patterns = [
            "what happens if",
            "what would this do",
            "does this work",
            "will this run",
            "can this execute",
            "is this correct",
        ];

        for pattern in &question_patterns {
            if text_lower.contains(pattern) {
                confidence += 0.2;
                context_clues.push(pattern.to_string());
            }
        }

        // Check for code modification context
        if text_lower.contains("here's the fix") || text_lower.contains("try this instead") {
            confidence += 0.2;
            context_clues.push("code modification context".to_string());
        }

        // Check for testing context
        if text_lower.contains("test")
            && (text_lower.contains("function") || text_lower.contains("script"))
        {
            confidence += 0.15;
            context_clues.push("testing context".to_string());
        }

        // Proximity to code blocks increases confidence
        if self.has_code_blocks(text) {
            confidence += 0.1;
            context_clues.push("code blocks present".to_string());
        }

        let suggested_action = if confidence > 0.8 {
            "High confidence - auto-execute".to_string()
        } else if confidence > 0.5 {
            "Medium confidence - ask user".to_string()
        } else if confidence > 0.2 {
            "Low confidence - show code only".to_string()
        } else {
            "No execution intent detected".to_string()
        };

        ExecutionIntent {
            confidence: confidence.min(1.0_f64),
            trigger_keywords,
            context_clues,
            suggested_action,
        }
    }

    /// Validate code before execution
    pub fn validate_code(&self, code_block: &CodeBlock) -> Result<(), VmExecutionError> {
        // Check code length
        if code_block.code.len() > 10000 {
            return Err(VmExecutionError::ValidationFailed(
                "Code exceeds maximum length of 10,000 characters".to_string(),
            ));
        }

        // Check for dangerous patterns
        let dangerous_patterns = [
            r"rm\s+-rf",
            r"format\s+c:",
            r"mkfs\.",
            r"dd\s+if=",
            r":\(\)\{\s*:\|\:&\s*\}", // Fork bomb
            r"curl.*\|.*sh",
            r"wget.*\|.*sh",
            r"eval\s*\(",
            r"exec\s*\(",
            r"system\s*\(",
            r"__import__.*os",
            r"subprocess\.",
            r"importlib\.",
        ];

        let code_lower = code_block.code.to_lowercase();
        for pattern in &dangerous_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(&code_lower) {
                    return Err(VmExecutionError::ValidationFailed(format!(
                        "Code contains potentially dangerous pattern: {}",
                        pattern
                    )));
                }
            }
        }

        // Language-specific validation
        if let Some(lang_config) = self.language_configs.get(&code_block.language) {
            for restriction in &lang_config.restrictions {
                if code_lower.contains(restriction) {
                    return Err(VmExecutionError::ValidationFailed(format!(
                        "Code violates language restriction: {}",
                        restriction
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get language configuration
    pub fn get_language_config(&self, language: &str) -> Option<&LanguageConfig> {
        self.language_configs.get(language)
    }

    /// Check if language is supported
    pub fn is_language_supported(&self, language: &str) -> bool {
        self.language_configs.contains_key(language)
    }

    // Private implementation methods

    fn initialize_patterns(&mut self) {
        let patterns = [
            // Fenced code blocks with language
            r"```(\w+)\n([\s\S]*?)\n```",
            // Fenced code blocks without language
            r"```\n([\s\S]*?)\n```",
            // Indented code blocks (4+ spaces)
            r"(?m)^(    .+\n)+",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                self.code_block_patterns.push(regex);
            } else {
                warn!("Failed to compile regex pattern: {}", pattern);
            }
        }
    }

    fn initialize_language_configs(&mut self) {
        // Python configuration
        self.language_configs.insert(
            "python".to_string(),
            LanguageConfig {
                name: "python".to_string(),
                extension: "py".to_string(),
                execute_command: "python3".to_string(),
                common_packages: vec![
                    "numpy".to_string(),
                    "pandas".to_string(),
                    "requests".to_string(),
                ],
                restrictions: vec![
                    "__import__".to_string(),
                    "exec(".to_string(),
                    "eval(".to_string(),
                ],
                timeout_multiplier: 1.0,
            },
        );

        // JavaScript configuration
        self.language_configs.insert(
            "javascript".to_string(),
            LanguageConfig {
                name: "javascript".to_string(),
                extension: "js".to_string(),
                execute_command: "node".to_string(),
                common_packages: vec![
                    "lodash".to_string(),
                    "axios".to_string(),
                    "moment".to_string(),
                ],
                restrictions: vec![
                    "eval(".to_string(),
                    "function(".to_string(),
                    "require(".to_string(),
                ],
                timeout_multiplier: 1.0,
            },
        );

        // Bash configuration
        self.language_configs.insert(
            "bash".to_string(),
            LanguageConfig {
                name: "bash".to_string(),
                extension: "sh".to_string(),
                execute_command: "bash".to_string(),
                common_packages: vec![],
                restrictions: vec![
                    "rm -rf".to_string(),
                    "format".to_string(),
                    "mkfs".to_string(),
                ],
                timeout_multiplier: 1.5,
            },
        );

        // Rust configuration
        self.language_configs.insert(
            "rust".to_string(),
            LanguageConfig {
                name: "rust".to_string(),
                extension: "rs".to_string(),
                execute_command: "rustc".to_string(),
                common_packages: vec!["serde".to_string(), "tokio".to_string(), "clap".to_string()],
                restrictions: vec!["unsafe".to_string(), "std::process".to_string()],
                timeout_multiplier: 3.0, // Compilation takes longer
            },
        );
    }

    fn initialize_intent_patterns(&mut self) {
        let patterns = [
            r"(?i)(run|execute|try)\s+(this|the|it)",
            r"(?i)(can|could|would)\s+you\s+(run|execute|try)",
            r"(?i)(test|check|verify)\s+(this|the)\s+code",
            r"(?i)(what\s+happens|what\s+would\s+happen)\s+if",
            r"(?i)(does|will)\s+(this|it)\s+(work|run)",
        ];

        for pattern in &patterns {
            if let Ok(regex) = Regex::new(pattern) {
                self.intent_patterns.push(regex);
            }
        }
    }

    fn extract_fenced_blocks(&self, text: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();

        // Pattern for ```language\ncode\n```
        let fenced_pattern = Regex::new(r"```(\w+)?\n([\s\S]*?)\n```").unwrap();

        for captures in fenced_pattern.captures_iter(text) {
            let language = captures
                .get(1)
                .map(|m| m.as_str().to_lowercase())
                .unwrap_or_else(|| "text".to_string());

            let code = captures.get(2).map(|m| m.as_str()).unwrap_or("");
            let start_pos = captures.get(0).unwrap().start();
            let end_pos = captures.get(0).unwrap().end();

            // Skip empty or very short code blocks
            if code.trim().len() < 3 {
                continue;
            }

            // Calculate execution confidence
            let confidence = self.calculate_execution_confidence(&language, code, text, start_pos);

            blocks.push(CodeBlock {
                language,
                code: code.to_string(),
                execution_confidence: confidence,
                start_pos,
                end_pos,
                metadata: None,
            });
        }

        blocks
    }

    fn extract_inline_executable_code(&self, text: &str) -> Vec<CodeBlock> {
        let mut blocks = Vec::new();

        // Look for single-line executable statements
        let executable_patterns = [
            (r"(?m)^python3?\s+(.+)$", "python"),
            (r"(?m)^node\s+(.+)$", "javascript"),
            (r"(?m)^bash\s+(.+)$", "bash"),
            (r"(?m)^cargo\s+run\s*(.*)$", "rust"),
            (r"(?m)^(\w+\s*=\s*.+)$", "python"), // Variable assignments
        ];

        for (pattern, language) in &executable_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                for captures in regex.captures_iter(text) {
                    if let Some(code_match) = captures.get(1) {
                        let code = code_match.as_str();
                        let start_pos = code_match.start();
                        let end_pos = code_match.end();

                        // Skip very short statements
                        if code.trim().len() < 5 {
                            continue;
                        }

                        let confidence =
                            self.calculate_execution_confidence(language, code, text, start_pos);

                        // Only include if confidence is reasonably high
                        if confidence > 0.3 {
                            blocks.push(CodeBlock {
                                language: language.to_string(),
                                code: code.to_string(),
                                execution_confidence: confidence,
                                start_pos,
                                end_pos,
                                metadata: Some(serde_json::json!({
                                    "type": "inline",
                                    "pattern": pattern
                                })),
                            });
                        }
                    }
                }
            }
        }

        blocks
    }

    fn calculate_execution_confidence(
        &self,
        language: &str,
        code: &str,
        full_text: &str,
        position: usize,
    ) -> f64 {
        let mut confidence: f64 = 0.0;

        // Base confidence by language
        match language {
            "python" | "javascript" | "bash" => confidence += 0.4,
            "rust" | "go" | "java" => confidence += 0.3,
            "text" | "plaintext" => confidence += 0.1,
            _ => confidence += 0.2,
        }

        // Code characteristics
        let code_lines = code.lines().count();
        if code_lines > 1 && code_lines < 50 {
            confidence += 0.2; // Multi-line but not too long
        }

        // Contains function definitions or statements
        if code.contains("def ") || code.contains("function ") || code.contains("fn ") {
            confidence += 0.1;
        }

        // Contains imports or requires
        if code.contains("import ") || code.contains("require(") || code.contains("use ") {
            confidence += 0.1;
        }

        // Surrounded by execution context
        let context_window = 200;
        let start = position.saturating_sub(context_window);
        let end = (position + context_window).min(full_text.len());
        let context = &full_text[start..end];

        for pattern in &self.intent_patterns {
            if pattern.is_match(context) {
                confidence += 0.2;
                break;
            }
        }

        // Proximity to execution keywords
        if context.to_lowercase().contains("run")
            || context.to_lowercase().contains("execute")
            || context.to_lowercase().contains("try")
        {
            confidence += 0.1;
        }

        confidence.min(1.0_f64)
    }

    fn has_code_blocks(&self, text: &str) -> bool {
        text.contains("```")
            || text
                .lines()
                .any(|line| line.starts_with("    ") && !line.trim().is_empty())
    }
}

impl Default for CodeBlockExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_python_code_block() {
        let extractor = CodeBlockExtractor::new();
        let text = r#"
Here's a simple Python script:

```python
print("Hello, World!")
x = 5 + 3
print(f"Result: {x}")
```

This should work fine.
        "#;

        let blocks = extractor.extract_code_blocks(text);
        // Extractor finds both fenced blocks and inline executable code (e.g., "x = 5 + 3")
        assert!(
            blocks.len() >= 1,
            "Expected at least 1 code block, got {}",
            blocks.len()
        );

        // First (highest confidence) should be the fenced Python block
        let python_block = blocks
            .iter()
            .find(|b| b.language == "python")
            .expect("Should have found Python fenced block");

        assert!(python_block.code.contains("Hello, World!"));
        assert!(python_block.execution_confidence > 0.0);
    }

    #[test]
    fn test_execution_intent_detection() {
        let extractor = CodeBlockExtractor::new();

        let high_intent = "Please run this code and see what happens";
        let intent = extractor.detect_execution_intent(high_intent);
        assert!(intent.confidence > 0.5);
        assert!(!intent.trigger_keywords.is_empty());

        let low_intent = "Here's some code for reference";
        let intent = extractor.detect_execution_intent(low_intent);
        assert!(intent.confidence < 0.3);
    }

    #[test]
    fn test_code_validation() {
        let extractor = CodeBlockExtractor::new();

        let safe_code = CodeBlock {
            language: "python".to_string(),
            code: "print('Hello')".to_string(),
            execution_confidence: 0.8,
            start_pos: 0,
            end_pos: 10,
            metadata: None,
        };

        assert!(extractor.validate_code(&safe_code).is_ok());

        let dangerous_code = CodeBlock {
            language: "bash".to_string(),
            code: "rm -rf /".to_string(),
            execution_confidence: 0.8,
            start_pos: 0,
            end_pos: 10,
            metadata: None,
        };

        assert!(extractor.validate_code(&dangerous_code).is_err());
    }

    #[test]
    fn test_language_support() {
        let extractor = CodeBlockExtractor::new();

        assert!(extractor.is_language_supported("python"));
        assert!(extractor.is_language_supported("javascript"));
        assert!(extractor.is_language_supported("bash"));
        assert!(extractor.is_language_supported("rust"));
        assert!(!extractor.is_language_supported("cobol"));
    }

    #[test]
    fn test_extract_multiple_languages() {
        let extractor = CodeBlockExtractor::new();
        let text = r#"
Here's Python:
```python
print("Python code")
```

And JavaScript:
```javascript
console.log("JavaScript code");
```

Try running both!
        "#;

        let blocks = extractor.extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);

        let languages: Vec<&String> = blocks.iter().map(|b| &b.language).collect();
        assert!(languages.contains(&&"python".to_string()));
        assert!(languages.contains(&&"javascript".to_string()));
    }
}
