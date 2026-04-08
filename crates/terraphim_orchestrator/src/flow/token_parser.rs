//! Token usage parser for CLI tool output.
//!
//! Extracts input_tokens, output_tokens, and cost_usd from various CLI tool outputs.
//! Supports: opencode, claude (anthropic), codex, and generic patterns.

use regex::Regex;
use std::sync::LazyLock;

/// Token usage data extracted from CLI output.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
}

// Regex patterns for different CLI tools
static RE_OPENAI_USAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:usage|tokens)[:\s]*\n?\s*(?:input|prompt)[:\s]*(\d+)[,\s]*(?:output|completion)[:\s]*(\d+)").unwrap()
});

static RE_ANTHROPIC_USAGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:input[_\s]?tokens?|tokens?[_\s]?in)[:\s]*(\d+)[,\s]*(?:output[_\s]?tokens?|tokens?[_\s]?out)[:\s]*(\d+)").unwrap()
});

static RE_TOTAL_TOKENS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:total[_\s]?tokens?)[:\s]*(\d+)").unwrap());

static RE_COST_USD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:cost|price)[:\s]*\$?([\d.]+)").unwrap());

static RE_JSON_USAGE: LazyLock<Regex> = LazyLock::new(|| {
    // Match JSON-like structures: {"input_tokens": 123, "output_tokens": 456}
    Regex::new(r#"["']?input[_\s]?tokens?["']?\s*[:=]\s*(\d+)[,\s]*["']?output[_\s]?tokens?["']?\s*[:=]\s*(\d+)"#).unwrap()
});

/// Parse token usage from CLI output (stdout/stderr combined).
pub fn parse_token_usage(output: &str) -> TokenUsage {
    let mut usage = TokenUsage::default();

    // Try OpenAI/Anthropic style patterns first
    if let Some(caps) = RE_OPENAI_USAGE.captures(output) {
        if let Ok(input) = caps[1].parse::<u64>() {
            usage.input_tokens = Some(input);
        }
        if let Ok(output) = caps[2].parse::<u64>() {
            usage.output_tokens = Some(output);
        }
    }

    // Try Anthropic specific pattern if not found
    if usage.input_tokens.is_none() || usage.output_tokens.is_none() {
        if let Some(caps) = RE_ANTHROPIC_USAGE.captures(output) {
            if let Ok(input) = caps[1].parse::<u64>() {
                usage.input_tokens = Some(input);
            }
            if let Ok(output) = caps[2].parse::<u64>() {
                usage.output_tokens = Some(output);
            }
        }
    }

    // Try JSON pattern
    if usage.input_tokens.is_none() || usage.output_tokens.is_none() {
        if let Some(caps) = RE_JSON_USAGE.captures(output) {
            if let Ok(input) = caps[1].parse::<u64>() {
                usage.input_tokens = Some(input);
            }
            if let Ok(output) = caps[2].parse::<u64>() {
                usage.output_tokens = Some(output);
            }
        }
    }

    // Extract total tokens if present
    if let Some(caps) = RE_TOTAL_TOKENS.captures(output) {
        if let Ok(total) = caps[1].parse::<u64>() {
            usage.total_tokens = Some(total);
        }
    }

    // Calculate total if we have input and output
    if usage.total_tokens.is_none() {
        if let (Some(input), Some(output)) = (usage.input_tokens, usage.output_tokens) {
            usage.total_tokens = Some(input + output);
        }
    }

    // Extract cost
    if let Some(caps) = RE_COST_USD.captures(output) {
        if let Ok(cost) = caps[1].parse::<f64>() {
            usage.cost_usd = Some(cost);
        }
    }

    usage
}

/// Parse token usage from specific CLI tool outputs.
pub fn parse_opencode_output(output: &str) -> TokenUsage {
    // Opencode typically outputs usage in a specific format
    // Example: "Usage: Input: 1234, Output: 567, Total: 1701"
    parse_token_usage(output)
}

pub fn parse_claude_output(output: &str) -> TokenUsage {
    // Claude CLI may output usage differently
    // Example: "Input tokens: 1234, Output tokens: 567"
    parse_token_usage(output)
}

pub fn parse_codex_output(output: &str) -> TokenUsage {
    // Codex/OpenAI CLI output
    parse_token_usage(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai_style_usage() {
        let output = r#"
Some response text here.

Usage:
  Input: 1250
  Output: 450
Total tokens: 1700
Cost: $0.023
"#;
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, Some(1250));
        assert_eq!(usage.output_tokens, Some(450));
        assert_eq!(usage.total_tokens, Some(1700));
        assert_eq!(usage.cost_usd, Some(0.023));
    }

    #[test]
    fn test_parse_anthropic_style_usage() {
        let output = r#"
Response content here.

Input tokens: 2000
Output tokens: 800
Cost: $0.045
"#;
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, Some(2000));
        assert_eq!(usage.output_tokens, Some(800));
        assert_eq!(usage.total_tokens, Some(2800));
        assert_eq!(usage.cost_usd, Some(0.045));
    }

    #[test]
    fn test_parse_json_style_usage() {
        let output = r#"
{"input_tokens": 1500, "output_tokens": 600, "cost_usd": 0.030}
"#;
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, Some(1500));
        assert_eq!(usage.output_tokens, Some(600));
        assert_eq!(usage.total_tokens, Some(2100));
    }

    #[test]
    fn test_parse_no_usage() {
        let output = "Just some regular output without token data.";
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, None);
        assert_eq!(usage.output_tokens, None);
        assert_eq!(usage.cost_usd, None);
    }

    #[test]
    fn test_parse_partial_usage() {
        let output = r#"
Total tokens: 5000
Cost: $0.10
"#;
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, None);
        assert_eq!(usage.output_tokens, None);
        assert_eq!(usage.total_tokens, Some(5000));
        assert_eq!(usage.cost_usd, Some(0.10));
    }

    #[test]
    fn test_parse_inline_usage() {
        let output = "Usage: Input: 100, Output: 50, done!";
        let usage = parse_token_usage(output);
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(usage.total_tokens, Some(150));
    }
}
