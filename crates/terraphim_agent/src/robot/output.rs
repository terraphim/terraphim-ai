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

impl std::str::FromStr for FieldMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("custom:") {
            let fields: Vec<String> = s
                .strip_prefix("custom:")
                .unwrap_or("")
                .split(',')
                .map(|f| f.trim().to_string())
                .filter(|f| !f.is_empty())
                .collect();
            if fields.is_empty() {
                return Err("custom: mode requires at least one field name".to_string());
            }
            return Ok(FieldMode::Custom(fields));
        }
        match s.to_lowercase().as_str() {
            "full" => Ok(FieldMode::Full),
            "summary" => Ok(FieldMode::Summary),
            "minimal" => Ok(FieldMode::Minimal),
            _ => Err(format!(
                "unknown field mode '{}'. Valid values: full, summary, minimal, custom:<f1>,<f2>",
                s
            )),
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

    /// Format a value as output string, applying field-mode filtering
    /// when the value is a search response with results.
    pub fn format<T: Serialize>(&self, value: &T) -> Result<String, serde_json::Error> {
        let filtered = self.apply_fields(value);
        match self.config.format {
            OutputFormat::Json => serde_json::to_string_pretty(&filtered),
            OutputFormat::Jsonl | OutputFormat::Minimal => serde_json::to_string(&filtered),
            OutputFormat::Table => serde_json::to_string_pretty(&filtered),
        }
    }

    /// Apply field-mode filtering. For non-`Full` modes, drops fields
    /// from each item in `data.results` that are not in the allowed set.
    /// Returns the original value unchanged when the value is not a
    /// search response or serialization fails.
    fn apply_fields<T: Serialize>(&self, value: &T) -> serde_json::Value {
        if matches!(self.config.fields, FieldMode::Full) {
            return serde_json::to_value(value).unwrap_or(serde_json::Value::Null);
        }
        let mut v = match serde_json::to_value(value) {
            Ok(v) => v,
            Err(_) => return serde_json::Value::Null,
        };
        if let Some(results) = v.get_mut("data").and_then(|d| d.get_mut("results")) {
            if let Some(arr) = results.as_array_mut() {
                let keep: Vec<&str> = match &self.config.fields {
                    FieldMode::Full => unreachable!(),
                    FieldMode::Summary => vec![
                        "rank",
                        "id",
                        "title",
                        "url",
                        "score",
                        "preview",
                        "source",
                        "date",
                        "preview_truncated",
                    ],
                    FieldMode::Minimal => vec!["rank", "id", "title", "url", "score"],
                    FieldMode::Custom(fields) => fields.iter().map(|s| s.as_str()).collect(),
                };
                for item in arr {
                    if let Some(obj) = item.as_object_mut() {
                        obj.retain(|k, _| keep.contains(&k.as_str()));
                    }
                }
            }
        }
        v
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
                let safe_boundary = if content.is_char_boundary(max_len) {
                    max_len
                } else {
                    content
                        .char_indices()
                        .take_while(|(i, _)| *i < max_len)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0)
                };
                let truncated =
                    if let Some(pos) = content[..safe_boundary].rfind(char::is_whitespace) {
                        &content[..pos]
                    } else {
                        &content[..safe_boundary]
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
        assert_eq!(OutputFormat::from_str_loose("unknown"), OutputFormat::Json);
        // Default
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
    fn test_field_mode_from_str_standard_variants() {
        use std::str::FromStr;
        assert_eq!(FieldMode::from_str("full").unwrap(), FieldMode::Full);
        assert_eq!(FieldMode::from_str("Full").unwrap(), FieldMode::Full);
        assert_eq!(FieldMode::from_str("summary").unwrap(), FieldMode::Summary);
        assert_eq!(FieldMode::from_str("minimal").unwrap(), FieldMode::Minimal);
    }

    #[test]
    fn test_field_mode_from_str_custom() {
        use std::str::FromStr;
        let mode = FieldMode::from_str("custom:title,score").unwrap();
        assert_eq!(
            mode,
            FieldMode::Custom(vec!["title".to_string(), "score".to_string()])
        );
    }

    #[test]
    fn test_field_mode_from_str_custom_empty_fields_error() {
        use std::str::FromStr;
        assert!(FieldMode::from_str("custom:").is_err());
    }

    #[test]
    fn test_field_mode_from_str_unknown_returns_error() {
        use std::str::FromStr;
        assert!(FieldMode::from_str("bogus").is_err());
    }

    #[test]
    fn test_robot_config_builder_all_flags() {
        let config = RobotConfig::new()
            .with_max_tokens(1000)
            .with_max_content_length(500)
            .with_fields(FieldMode::Summary);
        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.max_content_length, Some(500));
        assert_eq!(config.fields, FieldMode::Summary);
    }

    #[test]
    fn test_formatter_truncation() {
        let config = RobotConfig::new().with_max_content_length(20);
        let formatter = RobotFormatter::new(config);

        let (truncated, was_truncated) =
            formatter.truncate_content("This is a very long string that should be truncated");
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

    #[test]
    fn test_formatter_jsonl_streaming() {
        let formatter = RobotFormatter::new(RobotConfig::new().with_format(OutputFormat::Jsonl));
        let items = vec![serde_json::json!({"id": 1}), serde_json::json!({"id": 2})];
        let output = formatter.format_stream(items).unwrap();
        assert!(output.contains('\n'));
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_formatter_minimal_format() {
        let formatter = RobotFormatter::new(RobotConfig::new().with_format(OutputFormat::Minimal));
        let data = serde_json::json!({"key": "value"});
        let output = formatter.format(&data).unwrap();
        assert!(!output.contains('\n'));
    }

    #[test]
    fn test_formatter_table_format() {
        let formatter = RobotFormatter::new(RobotConfig::new().with_format(OutputFormat::Table));
        let data = serde_json::json!({"key": "value"});
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("key"));
    }

    #[test]
    fn test_would_exceed_budget() {
        let config = RobotConfig::new().with_max_tokens(5);
        let formatter = RobotFormatter::new(config);
        assert!(formatter.would_exceed_budget("this is more than twenty chars"));
        assert!(!formatter.would_exceed_budget("hi"));
    }

    #[test]
    fn test_no_budget_unlimited() {
        let formatter = RobotFormatter::new(RobotConfig::new());
        assert!(!formatter.would_exceed_budget(&"x".repeat(10000)));
    }

    #[test]
    fn test_output_format_from_str() {
        use std::str::FromStr;
        assert!(OutputFormat::from_str("json").is_ok());
        assert!(OutputFormat::from_str("jsonl").is_ok());
        assert!(OutputFormat::from_str("unknown_format").is_err());
    }

    #[test]
    fn test_output_format_name() {
        assert_eq!(OutputFormat::Json.name(), "json");
        assert_eq!(OutputFormat::Jsonl.name(), "jsonl");
        assert_eq!(OutputFormat::Minimal.name(), "minimal");
        assert_eq!(OutputFormat::Table.name(), "table");
    }

    #[test]
    fn test_field_mode_custom_empty() {
        let mode = FieldMode::from_str_loose("custom:");
        assert_eq!(mode, FieldMode::Custom(vec![]));
    }

    #[test]
    fn test_field_mode_unknown_defaults_full() {
        assert_eq!(FieldMode::from_str_loose("unknown"), FieldMode::Full);
    }

    #[test]
    fn test_robot_config_default() {
        let config = RobotConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.format, OutputFormat::Json);
        assert_eq!(config.max_results, Some(10));
    }

    #[test]
    fn test_robot_config_builder() {
        let config = RobotConfig::new()
            .with_format(OutputFormat::Jsonl)
            .with_max_tokens(100)
            .with_max_results(5)
            .with_max_content_length(200)
            .with_fields(FieldMode::Minimal);
        assert!(config.enabled);
        assert_eq!(config.format, OutputFormat::Jsonl);
        assert_eq!(config.max_tokens, Some(100));
        assert_eq!(config.max_results, Some(5));
        assert_eq!(config.max_content_length, Some(200));
        assert_eq!(config.fields, FieldMode::Minimal);
    }

    #[test]
    fn test_truncation_word_boundary() {
        let config = RobotConfig::new().with_max_content_length(10);
        let formatter = RobotFormatter::new(config);
        let (truncated, was_truncated) = formatter.truncate_content("hello world test");
        assert!(was_truncated);
        assert!(!truncated.contains("world"));
    }

    #[test]
    fn test_truncation_no_whitespace() {
        let config = RobotConfig::new().with_max_content_length(5);
        let formatter = RobotFormatter::new(config);
        let (truncated, was_truncated) = formatter.truncate_content("abcdefgh");
        assert!(was_truncated);
        assert!(truncated.starts_with("abcde"));
    }

    #[test]
    fn test_fields_full_includes_all() {
        let config = RobotConfig::new().with_fields(FieldMode::Full);
        let formatter = RobotFormatter::new(config);
        let data = serde_json::json!({
            "data": {
                "results": [
                    {"rank": 1, "id": "a", "title": "T", "url": "u", "score": 0.9, "preview": "p"}
                ]
            }
        });
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("\"preview\""));
        assert!(output.contains("\"rank\""));
    }

    #[test]
    fn test_fields_minimal_excludes_preview() {
        let config = RobotConfig::new().with_fields(FieldMode::Minimal);
        let formatter = RobotFormatter::new(config);
        let data = serde_json::json!({
            "data": {
                "results": [
                    {"rank": 1, "id": "a", "title": "T", "url": "u", "score": 0.9, "preview": "p"}
                ]
            }
        });
        let output = formatter.format(&data).unwrap();
        assert!(!output.contains("\"preview\""));
        assert!(output.contains("\"rank\""));
        assert!(output.contains("\"id\""));
    }

    #[test]
    fn test_fields_summary_includes_preview_excludes_body() {
        let config = RobotConfig::new().with_fields(FieldMode::Summary);
        let formatter = RobotFormatter::new(config);
        let data = serde_json::json!({
            "data": {
                "results": [
                    {"rank": 1, "id": "a", "title": "T", "preview": "p", "body": "full text"}
                ]
            }
        });
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("\"preview\""));
        assert!(!output.contains("\"body\""));
    }

    #[test]
    fn test_fields_custom_only_keeps_named() {
        let config =
            RobotConfig::new().with_fields(FieldMode::Custom(vec!["rank".into(), "id".into()]));
        let formatter = RobotFormatter::new(config);
        let data = serde_json::json!({
            "data": {
                "results": [
                    {"rank": 1, "id": "a", "title": "T", "url": "u", "score": 0.9}
                ]
            }
        });
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("\"rank\""));
        assert!(output.contains("\"id\""));
        assert!(!output.contains("\"title\""));
        assert!(!output.contains("\"url\""));
        assert!(!output.contains("\"score\""));
    }

    #[test]
    fn test_fields_full_noop_on_non_search_response() {
        let config = RobotConfig::new().with_fields(FieldMode::Minimal);
        let formatter = RobotFormatter::new(config);
        let data = serde_json::json!({"status": "ok", "msg": "hello"});
        let output = formatter.format(&data).unwrap();
        assert!(output.contains("\"msg\""));
        assert!(output.contains("\"status\""));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn format_never_panics(key: String, value: String) {
            let formatter = RobotFormatter::new(RobotConfig::new());
            let data = serde_json::json!({ key: value });
            let _ = formatter.format(&data);
        }

        #[test]
        fn truncate_never_panics(content: String) {
            let config = RobotConfig::new().with_max_content_length(50);
            let formatter = RobotFormatter::new(config);
            let (result, _) = formatter.truncate_content(&content);
            if content.len() > 50 {
                prop_assert!(result.contains("..."));
            }
        }

        #[test]
        fn from_str_loose_never_panics(input: String) {
            let _ = OutputFormat::from_str_loose(&input);
            let _ = FieldMode::from_str_loose(&input);
        }

        #[test]
        fn format_stream_empty_is_empty(_input: String) {
            let formatter = RobotFormatter::new(RobotConfig::new());
            let items: Vec<serde_json::Value> = vec![];
            let output = formatter.format_stream(items).unwrap();
            prop_assert!(output.is_empty());
        }
    }
}
