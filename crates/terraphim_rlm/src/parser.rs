//! Command parser for LLM output.
//!
//! This module provides parsing logic to extract structured commands from LLM
//! responses. The LLM outputs commands in a specific format that allows the
//! RLM to understand what action to take.
//!
//! ## Supported Command Formats
//!
//! - `FINAL(result)` or `FINAL("result")` - Return final result and terminate
//! - `FINAL_VAR(variable_name)` - Return variable value and terminate
//! - `RUN(command)` or `RUN("command")` - Execute bash command
//! - `CODE(python_code)` or ```python ... ``` - Execute Python code
//! - `SNAPSHOT(name)` - Create named snapshot
//! - `ROLLBACK(name)` - Restore to named snapshot
//! - `QUERY_LLM(prompt)` - Recursive LLM call
//! - `QUERY_LLM_BATCHED([...])` - Batched recursive LLM calls

use crate::error::{RlmError, RlmResult};
use crate::types::{BashCommand, Command, LlmQuery, PythonCode};

/// Command parser for extracting structured commands from LLM output.
#[derive(Debug, Default)]
pub struct CommandParser {
    /// Whether to allow bare code blocks without CODE() wrapper.
    pub allow_bare_code_blocks: bool,
    /// Whether to be strict about command format (fail on unknown patterns).
    pub strict_mode: bool,
}

impl CommandParser {
    /// Create a new command parser with default settings.
    pub fn new() -> Self {
        Self {
            allow_bare_code_blocks: true,
            strict_mode: false,
        }
    }

    /// Create a strict parser that fails on unrecognized patterns.
    pub fn strict() -> Self {
        Self {
            allow_bare_code_blocks: false,
            strict_mode: true,
        }
    }

    /// Parse commands from LLM output.
    ///
    /// Returns a list of commands found in the output. The LLM may output
    /// multiple commands in a single response, though typically only one
    /// is expected.
    pub fn parse(&self, input: &str) -> RlmResult<Vec<Command>> {
        let mut commands = Vec::new();
        let input = input.trim();

        // Try parsing in order of specificity
        if let Some(cmd) = self.try_parse_final(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_final_var(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_run(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_code(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_snapshot(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_rollback(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_query_llm(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        if let Some(cmd) = self.try_parse_query_llm_batched(input)? {
            commands.push(cmd);
            return Ok(commands);
        }

        // Try bare code blocks if allowed
        if self.allow_bare_code_blocks {
            if let Some(cmd) = self.try_parse_bare_code_block(input)? {
                commands.push(cmd);
                return Ok(commands);
            }
        }

        // If strict mode and no command found, fail
        if self.strict_mode && commands.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: format!("No valid command found in output: {}", truncate(input, 100)),
            });
        }

        Ok(commands)
    }

    /// Parse a single command, returning an error if none or multiple found.
    pub fn parse_one(&self, input: &str) -> RlmResult<Command> {
        let commands = self.parse(input)?;
        match commands.len() {
            0 => Err(RlmError::CommandParseFailed {
                message: "No command found in LLM output".to_string(),
            }),
            1 => Ok(commands.into_iter().next().unwrap()),
            n => Err(RlmError::CommandParseFailed {
                message: format!("Expected 1 command, found {n}"),
            }),
        }
    }

    /// Try parsing FINAL command.
    ///
    /// Formats:
    /// - `FINAL(result)`
    /// - `FINAL("result")`
    /// - `FINAL('result')`
    /// - `FINAL('''multiline result''')`
    fn try_parse_final(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        // Check for FINAL prefix
        if !input.starts_with("FINAL(") {
            return Ok(None);
        }

        // Find matching close paren
        let content = extract_parens_content(input, "FINAL")?;
        let result = unquote_string(&content);

        Ok(Some(Command::Final(result)))
    }

    /// Try parsing FINAL_VAR command.
    ///
    /// Format: `FINAL_VAR(variable_name)`
    fn try_parse_final_var(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("FINAL_VAR(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "FINAL_VAR")?;
        let var_name = content.trim();

        if var_name.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "FINAL_VAR requires a variable name".to_string(),
            });
        }

        // Validate variable name (alphanumeric + underscore)
        if !var_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(RlmError::CommandParseFailed {
                message: format!("Invalid variable name: {var_name}"),
            });
        }

        Ok(Some(Command::FinalVar(var_name.to_string())))
    }

    /// Try parsing RUN command.
    ///
    /// Formats:
    /// - `RUN(command)`
    /// - `RUN("command")`
    /// - `RUN('command')`
    fn try_parse_run(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("RUN(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "RUN")?;
        let command = unquote_string(&content);

        if command.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "RUN requires a command".to_string(),
            });
        }

        Ok(Some(Command::Run(BashCommand::new(command))))
    }

    /// Try parsing CODE command.
    ///
    /// Formats:
    /// - `CODE(python_code)`
    /// - `CODE("python_code")`
    /// - `CODE('''multiline code''')`
    fn try_parse_code(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("CODE(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "CODE")?;
        let code = unquote_string(&content);

        if code.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "CODE requires Python code".to_string(),
            });
        }

        Ok(Some(Command::Code(PythonCode::new(code))))
    }

    /// Try parsing bare code blocks (```python ... ```).
    fn try_parse_bare_code_block(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        // Check for Python code block
        if let Some(code) = extract_code_block(input, "python") {
            return Ok(Some(Command::Code(PythonCode::new(code))));
        }

        // Check for bash/shell code block
        if let Some(code) = extract_code_block(input, "bash") {
            return Ok(Some(Command::Run(BashCommand::new(code))));
        }
        if let Some(code) = extract_code_block(input, "sh") {
            return Ok(Some(Command::Run(BashCommand::new(code))));
        }
        if let Some(code) = extract_code_block(input, "shell") {
            return Ok(Some(Command::Run(BashCommand::new(code))));
        }

        Ok(None)
    }

    /// Try parsing SNAPSHOT command.
    fn try_parse_snapshot(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("SNAPSHOT(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "SNAPSHOT")?;
        let name = unquote_string(&content);

        if name.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "SNAPSHOT requires a name".to_string(),
            });
        }

        Ok(Some(Command::Snapshot(name)))
    }

    /// Try parsing ROLLBACK command.
    fn try_parse_rollback(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("ROLLBACK(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "ROLLBACK")?;
        let name = unquote_string(&content);

        if name.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "ROLLBACK requires a snapshot name".to_string(),
            });
        }

        Ok(Some(Command::Rollback(name)))
    }

    /// Try parsing QUERY_LLM command.
    fn try_parse_query_llm(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("QUERY_LLM(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "QUERY_LLM")?;
        let prompt = unquote_string(&content);

        if prompt.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "QUERY_LLM requires a prompt".to_string(),
            });
        }

        Ok(Some(Command::QueryLlm(LlmQuery::new(prompt))))
    }

    /// Try parsing QUERY_LLM_BATCHED command.
    fn try_parse_query_llm_batched(&self, input: &str) -> RlmResult<Option<Command>> {
        let input = input.trim();

        if !input.starts_with("QUERY_LLM_BATCHED(") {
            return Ok(None);
        }

        let content = extract_parens_content(input, "QUERY_LLM_BATCHED")?;

        // Expect a JSON array of prompts
        let prompts: Vec<String> =
            serde_json::from_str(&content).map_err(|e| RlmError::CommandParseFailed {
                message: format!("Invalid JSON array in QUERY_LLM_BATCHED: {e}"),
            })?;

        if prompts.is_empty() {
            return Err(RlmError::CommandParseFailed {
                message: "QUERY_LLM_BATCHED requires at least one prompt".to_string(),
            });
        }

        let queries: Vec<LlmQuery> = prompts.into_iter().map(LlmQuery::new).collect();
        Ok(Some(Command::QueryLlmBatched(queries)))
    }
}

/// Extract content between parentheses for a command.
fn extract_parens_content(input: &str, cmd_name: &str) -> RlmResult<String> {
    let prefix = format!("{cmd_name}(");
    if !input.starts_with(&prefix) {
        return Err(RlmError::CommandParseFailed {
            message: format!("Expected {cmd_name}(...)"),
        });
    }

    // Handle nested parens, strings, and escapes
    let content_start = prefix.len();
    let chars: Vec<char> = input.chars().collect();
    let mut depth = 1;
    let mut in_string = false;
    let mut string_char = '"';
    let mut in_triple = false;
    let mut i = content_start;

    while i < chars.len() && depth > 0 {
        let c = chars[i];

        // Check for triple quotes
        if !in_string
            && i + 2 < chars.len()
            && (chars[i..i + 3] == ['\'', '\'', '\''] || chars[i..i + 3] == ['"', '"', '"'])
        {
            in_triple = true;
            in_string = true;
            string_char = c;
            i += 3;
            continue;
        }

        // Check for end of triple quotes
        if in_triple
            && i + 2 < chars.len()
            && chars[i] == string_char
            && chars[i + 1] == string_char
            && chars[i + 2] == string_char
        {
            in_triple = false;
            in_string = false;
            i += 3;
            continue;
        }

        // Handle single/double quotes
        if !in_string && (c == '"' || c == '\'') {
            in_string = true;
            string_char = c;
            i += 1;
            continue;
        }

        if in_string && !in_triple && c == string_char {
            // Check for escape
            if i > 0 && chars[i - 1] == '\\' {
                i += 1;
                continue;
            }
            in_string = false;
            i += 1;
            continue;
        }

        // Track parens (only outside strings)
        if !in_string {
            if c == '(' {
                depth += 1;
            } else if c == ')' {
                depth -= 1;
            }
        }

        i += 1;
    }

    if depth != 0 {
        return Err(RlmError::CommandParseFailed {
            message: format!("Unbalanced parentheses in {cmd_name} command"),
        });
    }

    // Extract content (exclude final closing paren)
    let content: String = chars[content_start..i - 1].iter().collect();
    Ok(content.trim().to_string())
}

/// Remove quotes from a string value.
fn unquote_string(s: &str) -> String {
    let s = s.trim();

    // Handle triple quotes
    if (s.starts_with("'''") && s.ends_with("'''"))
        || (s.starts_with("\"\"\"") && s.ends_with("\"\"\""))
    {
        return s[3..s.len() - 3].to_string();
    }

    // Handle single/double quotes
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        return s[1..s.len() - 1].to_string();
    }

    s.to_string()
}

/// Extract code from a markdown code block.
fn extract_code_block(input: &str, language: &str) -> Option<String> {
    let prefix = format!("```{language}");
    let alt_prefix = format!("```{language}\n");

    let start = if input.starts_with(&prefix) {
        Some(prefix.len())
    } else if input.starts_with(&alt_prefix) {
        Some(alt_prefix.len())
    } else {
        None
    }?;

    // Find closing ```
    let remaining = &input[start..];
    let end = remaining.find("```")?;

    Some(remaining[..end].trim().to_string())
}

/// Truncate a string for error messages.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_final_simple() {
        let parser = CommandParser::new();
        let result = parser.parse_one("FINAL(42)").unwrap();
        assert!(matches!(result, Command::Final(s) if s == "42"));
    }

    #[test]
    fn test_parse_final_quoted() {
        let parser = CommandParser::new();
        let result = parser.parse_one(r#"FINAL("hello world")"#).unwrap();
        assert!(matches!(result, Command::Final(s) if s == "hello world"));
    }

    #[test]
    fn test_parse_final_triple_quoted() {
        let parser = CommandParser::new();
        let result = parser.parse_one("FINAL('''multi\nline''')").unwrap();
        assert!(matches!(result, Command::Final(s) if s == "multi\nline"));
    }

    #[test]
    fn test_parse_final_var() {
        let parser = CommandParser::new();
        let result = parser.parse_one("FINAL_VAR(my_result)").unwrap();
        assert!(matches!(result, Command::FinalVar(s) if s == "my_result"));
    }

    #[test]
    fn test_parse_run() {
        let parser = CommandParser::new();
        let result = parser.parse_one("RUN(ls -la)").unwrap();
        assert!(matches!(result, Command::Run(cmd) if cmd.command == "ls -la"));
    }

    #[test]
    fn test_parse_run_quoted() {
        let parser = CommandParser::new();
        let result = parser.parse_one(r#"RUN("echo 'hello'")"#).unwrap();
        assert!(matches!(result, Command::Run(cmd) if cmd.command == "echo 'hello'"));
    }

    #[test]
    fn test_parse_code() {
        let parser = CommandParser::new();
        let result = parser.parse_one("CODE(print('hello'))").unwrap();
        assert!(matches!(result, Command::Code(code) if code.code == "print('hello')"));
    }

    #[test]
    fn test_parse_bare_python_block() {
        let parser = CommandParser::new();
        let input = "```python\nx = 1 + 1\nprint(x)\n```";
        let result = parser.parse_one(input).unwrap();
        assert!(matches!(result, Command::Code(code) if code.code.contains("x = 1 + 1")));
    }

    #[test]
    fn test_parse_bare_bash_block() {
        let parser = CommandParser::new();
        let input = "```bash\nls -la\n```";
        let result = parser.parse_one(input).unwrap();
        assert!(matches!(result, Command::Run(cmd) if cmd.command == "ls -la"));
    }

    #[test]
    fn test_parse_snapshot() {
        let parser = CommandParser::new();
        let result = parser.parse_one("SNAPSHOT(checkpoint1)").unwrap();
        assert!(matches!(result, Command::Snapshot(s) if s == "checkpoint1"));
    }

    #[test]
    fn test_parse_rollback() {
        let parser = CommandParser::new();
        let result = parser.parse_one("ROLLBACK(checkpoint1)").unwrap();
        assert!(matches!(result, Command::Rollback(s) if s == "checkpoint1"));
    }

    #[test]
    fn test_parse_query_llm() {
        let parser = CommandParser::new();
        let result = parser.parse_one("QUERY_LLM(what is 2+2?)").unwrap();
        assert!(matches!(result, Command::QueryLlm(q) if q.prompt == "what is 2+2?"));
    }

    #[test]
    fn test_parse_query_llm_batched() {
        let parser = CommandParser::new();
        let result = parser
            .parse_one(r#"QUERY_LLM_BATCHED(["q1", "q2", "q3"])"#)
            .unwrap();
        match result {
            Command::QueryLlmBatched(queries) => {
                assert_eq!(queries.len(), 3);
                assert_eq!(queries[0].prompt, "q1");
                assert_eq!(queries[1].prompt, "q2");
                assert_eq!(queries[2].prompt, "q3");
            }
            _ => panic!("Expected QueryLlmBatched"),
        }
    }

    #[test]
    fn test_parse_nested_parens() {
        let parser = CommandParser::new();
        let result = parser.parse_one("RUN(echo $(whoami))").unwrap();
        assert!(matches!(result, Command::Run(cmd) if cmd.command == "echo $(whoami)"));
    }

    #[test]
    fn test_strict_mode_fails_on_unknown() {
        let parser = CommandParser::strict();
        let result = parser.parse_one("random text");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_command_fails() {
        let parser = CommandParser::new();
        let result = parser.parse_one("RUN()");
        assert!(result.is_err());
    }

    #[test]
    fn test_unbalanced_parens_fails() {
        let parser = CommandParser::new();
        let result = parser.parse_one("FINAL(hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_var_name_fails() {
        let parser = CommandParser::new();
        let result = parser.parse_one("FINAL_VAR(my-var)");
        assert!(result.is_err());
    }

    #[test]
    fn test_whitespace_handling() {
        let parser = CommandParser::new();
        let result = parser.parse_one("  FINAL( result )  ").unwrap();
        assert!(matches!(result, Command::Final(s) if s == "result"));
    }
}
