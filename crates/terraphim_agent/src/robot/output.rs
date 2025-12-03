//! Output formatting for robot mode
//!
//! Handles JSON, JSONL, and other structured output formats.

use serde::Serialize;

/// Output format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Pretty-printed JSON (default for robot mode)
    #[default]
    Json,
    /// Newline-delimited JSON (streaming)
    Jsonl,
    /// Compact single-line JSON
    Minimal,
    /// Human-readable table (passthrough to existing formatters)
    Table,
}

impl OutputFormat {
    /// Parse format from string
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "jsonl" | "ndjson" => OutputFormat::Jsonl,
            "minimal" | "compact" => OutputFormat::Minimal,
            "table" | "human" | "text" => OutputFormat::Table,
            _ => OutputFormat::Json,
        }
    }

    /// Get format name for display
    pub fn name(&self) -> &'static str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Jsonl => "jsonl",
            OutputFormat::Minimal => "minimal",
            OutputFormat::Table => "table",
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "jsonl" | "ndjson" => Ok(OutputFormat::Jsonl),
            "minimal" | "compact" => Ok(OutputFormat::Minimal),
            "table" | "human" | "text" => Ok(OutputFormat::Table),
            _ => Err(format!(
                "Unknown format '{}'. Valid formats: json, jsonl, minimal, table",
                s
            )),
        }
    }
}

/// Field selection mode for output
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FieldMode {
    /// All fields including body content
    #[default]
    Full,
    /// Title, URL, description, score, concepts
    Summary,
    /// Title, URL, score only
    Minimal,
    /// Custom field selection
    Custom(Vec<String>),
}

impl FieldMode {
    /// Parse field mode from string
    pub fn from_str_loose(s: &str) -> Self {
        if s.starts_with("custom:") {
            let fields: Vec<String> = s
                .strip_prefix("custom:")
                .unwrap_or("")
                .split(',')
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect();
            return FieldMode::Custom(fields);
        }

        match s.to_lowercase().as_str() {
            "full" => FieldMode::Full,
            "summary" => FieldMode::Summary,
            "minimal" => FieldMode::Minimal,
            _ => FieldMode::Full,
        }
    }
}

/// Robot mode configuration
#[derive(Debug, Clone)]
pub struct RobotConfig {
    /// Output format
    pub format: OutputFormat,
    /// Maximum tokens to output (estimated)
    pub max_tokens: Option<usize>,
    /// Maximum results to return
    pub max_results: Option<usize>,
    /// Maximum content length before truncation
    pub max_content_length: Option<usize>,
    /// Field selection mode
    pub fields: FieldMode,
    /// Whether robot mode is enabled
    pub enabled: bool,
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self {
            format: OutputFormat::Json,
            max_tokens: None,
            max_results: Some(10),
            max_content_length: None,
            fields: FieldMode::Full,
            enabled: false,
        }
    }
}

impl RobotConfig {
    /// Create a new robot config with robot mode enabled
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Set output format
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set max results
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_results = Some(max_results);
        self
    }

    /// Set max content length
    pub fn with_max_content_length(mut self, max_content_length: usize) -> Self {
        self.max_content_length = Some(max_content_length);
        self
    }

    /// Set field mode
    pub fn with_fields(mut self, fields: FieldMode) -> Self {
        self.fields = fields;
        self
    }

    /// Check if this is robot mode
    pub fn is_robot_mode(&self) -> bool {
        self.enabled
    }
}

/// Formatter for robot mode output
pub struct RobotFormatter {
    config: RobotConfig,
}

impl RobotFormatter {
    /// Create a new formatter with config
    pub fn new(config: RobotConfig) -> Self {
        Self { config }
    }

    /// Format a value as output string
    pub fn format<T: Serialize>(&self, value: &T) -> Result<String, serde_json::Error> {
        match self.config.format {
            OutputFormat::Json => serde_json::to_string_pretty(value),
            OutputFormat::Jsonl | OutputFormat::Minimal => serde_json::to_string(value),
            OutputFormat::Table => {
                // For table format, we still return JSON but it's not used
                // The caller should handle table formatting separately
                serde_json::to_string_pretty(value)
            }
        }
    }

    /// Format multiple values as JSONL
    pub fn format_stream<T: Serialize, I: IntoIterator<Item = T>>(
        &self,
        values: I,
    ) -> Result<String, serde_json::Error> {
        let lines: Result<Vec<String>, _> = values
            .into_iter()
            .map(|v| serde_json::to_string(&v))
            .collect();
        Ok(lines?.join("\n"))
    }

    /// Truncate content if needed based on config
    pub fn truncate_content(&self, content: &str) -> (String, bool) {
        if let Some(max_len) = self.config.max_content_length {
            if content.len() > max_len {
                // Truncate at word boundary if possible
                let truncated = if let Some(pos) = content[..max_len].rfind(char::is_whitespace) {
                    &content[..pos]
                } else {
                    &content[..max_len]
                };
                return (format!("{}...", truncated), true);
            }
        }
        (content.to_string(), false)
    }

    /// Estimate token count (simple heuristic: ~4 chars per token)
    pub fn estimate_tokens(&self, text: &str) -> usize {
        text.len() / 4
    }

    /// Check if output would exceed token budget
    pub fn would_exceed_budget(&self, text: &str) -> bool {
        if let Some(max_tokens) = self.config.max_tokens {
            return self.estimate_tokens(text) > max_tokens;
        }
        false
    }

    /// Get the configuration
    pub fn config(&self) -> &RobotConfig {
        &self.config
    }
}

impl Default for RobotFormatter {
    fn default() -> Self {
        Self::new(RobotConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_parsing() {
        assert_eq!(OutputFormat::from_str_loose("json"), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str_loose("JSONL"), OutputFormat::Jsonl);
        assert_eq!(OutputFormat::from_str_loose("ndjson"), OutputFormat::Jsonl);
        assert_eq!(
            OutputFormat::from_str_loose("minimal"),
            OutputFormat::Minimal
        );
        assert_eq!(OutputFormat::from_str_loose("table"), OutputFormat::Table);
        assert_eq!(
            OutputFormat::from_str_loose("unknown"),
            OutputFormat::Json
        ); // Default
    }

    #[test]
    fn test_field_mode_parsing() {
        assert_eq!(FieldMode::from_str_loose("full"), FieldMode::Full);
        assert_eq!(FieldMode::from_str_loose("summary"), FieldMode::Summary);
        assert_eq!(FieldMode::from_str_loose("minimal"), FieldMode::Minimal);
        assert_eq!(
            FieldMode::from_str_loose("custom:title,url,score"),
            FieldMode::Custom(vec![
                "title".to_string(),
                "url".to_string(),
                "score".to_string()
            ])
        );
    }

    #[test]
    fn test_formatter_truncation() {
        let config = RobotConfig::new().with_max_content_length(20);
        let formatter = RobotFormatter::new(config);

        let (truncated, was_truncated) = formatter.truncate_content("This is a very long string that should be truncated");
        assert!(was_truncated);
        assert!(truncated.len() <= 23); // 20 + "..."

        let (not_truncated, was_truncated) = formatter.truncate_content("Short");
        assert!(!was_truncated);
        assert_eq!(not_truncated, "Short");
    }

    #[test]
    fn test_formatter_token_estimation() {
        let formatter = RobotFormatter::default();
        // ~4 chars per token
        assert_eq!(formatter.estimate_tokens("12345678"), 2);
        assert_eq!(formatter.estimate_tokens(""), 0);
    }

    #[test]
    fn test_formatter_json_output() {
        let formatter = RobotFormatter::new(RobotConfig::new());
        let data = serde_json::json!({"key": "value"});
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("key"));
        assert!(output.contains("value"));
    }
}
