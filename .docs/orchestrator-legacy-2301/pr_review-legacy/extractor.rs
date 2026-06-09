//! Extract assistant-visible review text from captured agent drain logs.
//!
//! The orchestrator writes complete stdout/stderr streams to drain log files.
//! Review verdict posting should consume those files instead of relying on a
//! bounded broadcast receiver, which can lag during long reviews.

use serde_json::Value;

/// Extract the final assistant text from captured drain lines.
///
/// Different CLIs use different streaming JSON formats. The extractor accepts
/// known JSON event shapes first and falls back to non-empty plain text lines.
pub fn extract_final_assistant_text(lines: &[String], cli_tool: &str) -> String {
    let cli = cli_tool.rsplit('/').next().unwrap_or(cli_tool);
    let extracted = if cli.contains("claude") {
        extract_claude_text(lines)
    } else if cli.contains("opencode") {
        extract_opencode_text(lines)
    } else {
        extract_message_content_text(lines)
    };

    if !extracted.trim().is_empty() {
        return extracted.trim().to_string();
    }

    lines
        .iter()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('{') || trimmed.starts_with('#') {
                None
            } else {
                Some(trimmed)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_claude_text(lines: &[String]) -> String {
    let mut chunks = Vec::new();
    for value in json_values(lines) {
        if value.get("type").and_then(Value::as_str) == Some("content_block_delta") {
            if let Some(text) = value
                .get("delta")
                .and_then(|delta| delta.get("text"))
                .and_then(Value::as_str)
            {
                chunks.push(text.to_string());
            }
        }
    }
    chunks.join("")
}

fn extract_opencode_text(lines: &[String]) -> String {
    let mut chunks = Vec::new();
    for value in json_values(lines) {
        if value.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                chunks.push(text.to_string());
            }
        }
        if let Some(text) = value
            .get("part")
            .and_then(|part| part.get("text"))
            .and_then(Value::as_str)
        {
            chunks.push(text.to_string());
        }
    }
    chunks.join("")
}

fn extract_message_content_text(lines: &[String]) -> String {
    let mut chunks = Vec::new();
    for value in json_values(lines) {
        if let Some(text) = value.get("text").and_then(Value::as_str) {
            chunks.push(text.to_string());
        }
        if let Some(text) = value
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
        {
            chunks.push(text.to_string());
        }
        if let Some(parts) = value
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_array)
        {
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    chunks.push(text.to_string());
                }
            }
        }
    }
    chunks.join("\n")
}

fn json_values(lines: &[String]) -> impl Iterator<Item = Value> + '_ {
    lines
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_claude_streaming_deltas() {
        let lines = vec![
            r#"{"type":"content_block_delta","delta":{"text":"<h3>Summary</h3>\n"}}"#.to_string(),
            r#"{"type":"content_block_delta","delta":{"text":"<h3>Confidence Score: 5/5</h3>"}}"#
                .to_string(),
        ];

        let text = extract_final_assistant_text(&lines, "/home/alex/.local/bin/claude");

        assert!(text.contains("<h3>Summary</h3>"));
        assert!(text.contains("Confidence Score: 5/5"));
    }

    #[test]
    fn extracts_opencode_text_parts() {
        let lines = vec![
            r#"{"type":"text","text":"<h3>Summary</h3>\n"}"#.to_string(),
            r#"{"part":{"text":"<h3>Inline Findings</h3>"}}"#.to_string(),
        ];

        let text = extract_final_assistant_text(&lines, "/home/alex/.bun/bin/opencode");

        assert!(text.contains("<h3>Summary</h3>"));
        assert!(text.contains("<h3>Inline Findings</h3>"));
    }

    #[test]
    fn extracts_message_content_array_text() {
        let lines = vec![
            r#"{"message":{"content":[{"type":"text","text":"<h3>Summary</h3>"}]}}"#.to_string(),
            r#"{"message":{"content":[{"type":"text","text":"<sub>Last reviewed commit: deadbee</sub>"}]}}"#.to_string(),
        ];

        let text = extract_final_assistant_text(&lines, "/home/alex/.local/bin/pi-rust");

        assert!(text.contains("<h3>Summary</h3>"));
        assert!(text.contains("Last reviewed commit: deadbee"));
    }

    #[test]
    fn falls_back_to_plain_text_lines() {
        let lines = vec![
            "# agent: pr-reviewer".to_string(),
            "".to_string(),
            "<h3>Summary</h3>".to_string(),
            "<h3>Confidence Score: 4/5</h3>".to_string(),
        ];

        let text = extract_final_assistant_text(&lines, "unknown-cli");

        assert_eq!(text, "<h3>Summary</h3>\n<h3>Confidence Score: 4/5</h3>");
    }
}
