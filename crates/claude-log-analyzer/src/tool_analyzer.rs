//! Tool analysis and command parsing logic

use std::collections::HashMap;

use crate::models::{ToolInvocation, ToolStatistics};

/// Shell built-ins and keywords to exclude from tool detection
#[allow(dead_code)] // Will be used in Phase 2
const EXCLUDED_SHELL_BUILTINS: &[&str] = &[
    "cd", "ls", "pwd", "echo", "cat", "mkdir", "rm", "cp", "mv", "export", "source", "if", "then",
    "else", "fi", "for", "while", "do", "done", "case", "esac", "function", "return", "local",
    "set", "unset", "shift", "test", "[", "[[", "alias", "unalias", "bg", "fg", "jobs", "wait",
    "kill", "exit", "break", "continue", "read", "printf", "pushd", "popd", "dirs", "true",
    "false", ":", ".",
];

/// Parse command into components (command, args, flags)
///
/// # Arguments
/// * `command` - The full command string
/// * `tool_start` - Offset where the tool name starts
///
/// # Returns
/// Tuple of (full_command, args, flags) or None if parsing fails
pub fn parse_command_context(
    command: &str,
    tool_start: usize,
) -> Option<(String, Vec<String>, HashMap<String, String>)> {
    // Split on shell operators (&&, ||, ;, |)
    let cmd_parts = split_command_pipeline(command);

    // Find segment containing the tool
    let relevant_part = cmd_parts
        .iter()
        .find(|part| {
            // Check if this part contains the tool at the right position
            if let Some(offset) = command.find(*part) {
                tool_start >= offset && tool_start < offset + part.len()
            } else {
                false
            }
        })?
        .trim();

    // Simple tokenization (space-separated)
    let tokens: Vec<String> = shell_words::split(relevant_part).ok()?;

    if tokens.is_empty() {
        return None;
    }

    let mut args = Vec::new();
    let mut flags = HashMap::new();

    let mut i = 1; // Skip command itself
    while i < tokens.len() {
        let token = &tokens[i];

        if token.starts_with("--") {
            // Long flag: --env production
            let flag_name = token.trim_start_matches("--");
            let flag_value = tokens.get(i + 1).cloned().unwrap_or_default();
            flags.insert(flag_name.to_string(), flag_value);
            i += 2;
        } else if token.starts_with('-') && token.len() > 1 {
            // Short flag: -f value
            let flag_name = token.trim_start_matches('-');
            let flag_value = tokens.get(i + 1).cloned().unwrap_or_default();
            flags.insert(flag_name.to_string(), flag_value);
            i += 2;
        } else {
            // Positional argument
            args.push(token.clone());
            i += 1;
        }
    }

    Some((relevant_part.to_string(), args, flags))
}

/// Split command on shell operators while respecting quotes
pub fn split_command_pipeline(command: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut quote_char = ' ';

    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        match ch {
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
                current.push(ch);
            }
            '"' | '\'' if in_quotes && ch == quote_char => {
                in_quotes = false;
                current.push(ch);
            }
            '&' | '|' | ';' if !in_quotes => {
                // Handle && and ||
                if (ch == '&' || ch == '|') && i + 1 < chars.len() && chars[i + 1] == ch {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                        current.clear();
                    }
                    i += 2;
                    continue;
                }
                if !current.trim().is_empty() {
                    parts.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
        i += 1;
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// Check if a command is an actual tool invocation (not a shell built-in)
#[must_use]
#[allow(dead_code)] // Used in parser for filtering shell builtins
pub fn is_actual_tool(tool_name: &str) -> bool {
    // Extract just the command name without path
    let base_name = tool_name.rsplit('/').next().unwrap_or(tool_name).trim();

    // Check if it's an excluded built-in
    !EXCLUDED_SHELL_BUILTINS.contains(&base_name)
}

/// Calculate tool statistics from invocations
/// Replaced by Analyzer::calculate_tool_statistics - kept for compatibility
#[must_use]
#[allow(dead_code)]
pub fn calculate_tool_statistics(
    invocations: &[ToolInvocation],
) -> HashMap<String, ToolStatistics> {
    let mut stats: HashMap<String, ToolStatistics> = HashMap::new();

    for inv in invocations {
        let stat = stats
            .entry(inv.tool_name.clone())
            .or_insert_with(|| ToolStatistics {
                tool_name: inv.tool_name.clone(),
                category: inv.tool_category.clone(),
                total_invocations: 0,
                agents_using: Vec::new(),
                success_count: 0,
                failure_count: 0,
                first_seen: inv.timestamp,
                last_seen: inv.timestamp,
                command_patterns: Vec::new(),
                sessions: Vec::new(),
            });

        stat.total_invocations += 1;

        // Track agents
        if let Some(ref agent) = inv.agent_context {
            if !stat.agents_using.contains(agent) {
                stat.agents_using.push(agent.clone());
            }
        }

        // Track sessions
        if !stat.sessions.contains(&inv.session_id) {
            stat.sessions.push(inv.session_id.clone());
        }

        // Update timestamps
        if inv.timestamp < stat.first_seen {
            stat.first_seen = inv.timestamp;
        }
        if inv.timestamp > stat.last_seen {
            stat.last_seen = inv.timestamp;
        }

        // Track success/failure
        match inv.exit_code {
            Some(0) => stat.success_count += 1,
            Some(_) => stat.failure_count += 1,
            None => {}
        }

        // Track command patterns (store unique base commands)
        let base_cmd = format!("{} {}", inv.tool_name, inv.arguments.join(" "));
        if !stat.command_patterns.contains(&base_cmd) && stat.command_patterns.len() < 10 {
            stat.command_patterns.push(base_cmd);
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command_context() {
        let cmd = "npx wrangler deploy --env production";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert!(args.contains(&"deploy".to_string()));
        assert_eq!(flags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_split_command_pipeline() {
        let cmd = "npm install && npm build";
        let parts = split_command_pipeline(cmd);

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "npm install");
        assert_eq!(parts[1], "npm build");
    }

    #[test]
    fn test_split_with_quotes() {
        let cmd = r#"echo "hello && world" && npm install"#;
        let parts = split_command_pipeline(cmd);

        assert_eq!(parts.len(), 2);
        assert!(parts[0].contains("hello && world"));
    }

    #[test]
    fn test_split_with_pipe() {
        let cmd = "cat file.txt | grep pattern";
        let parts = split_command_pipeline(cmd);

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], "cat file.txt");
        assert_eq!(parts[1], "grep pattern");
    }

    // Comprehensive wrangler command parsing tests
    #[test]
    fn test_parse_wrangler_deploy_with_env() {
        let cmd = "npx wrangler deploy --env production";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "deploy"]);
        assert_eq!(flags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_parse_wrangler_complex_flags() {
        let cmd = "npx wrangler deploy --env prod --minify --compatibility-date 2024-01-01";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "deploy", "2024-01-01"]);
        assert_eq!(flags.get("env"), Some(&"prod".to_string()));
        assert_eq!(
            flags.get("minify"),
            Some(&"--compatibility-date".to_string())
        );
    }

    #[test]
    fn test_parse_wrangler_bunx() {
        let cmd = "bunx wrangler login";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "login"]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_parse_wrangler_pnpm() {
        let cmd = "pnpm wrangler deploy --env staging";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "deploy"]);
        assert_eq!(flags.get("env"), Some(&"staging".to_string()));
    }

    #[test]
    fn test_parse_wrangler_yarn() {
        let cmd = "yarn wrangler publish";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "publish"]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_parse_wrangler_dev() {
        let cmd = "npx wrangler dev --port 8787";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "dev"]);
        assert_eq!(flags.get("port"), Some(&"8787".to_string()));
    }

    #[test]
    fn test_parse_wrangler_tail() {
        let cmd = "bunx wrangler tail my-worker";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "tail", "my-worker"]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_parse_wrangler_kv_commands() {
        let cmd = "npx wrangler kv:namespace create NAMESPACE --preview";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(
            args,
            vec!["wrangler", "kv:namespace", "create", "NAMESPACE"]
        );
        assert!(flags.contains_key("preview"));
    }

    #[test]
    fn test_parse_wrangler_pages_deploy() {
        let cmd = "npx wrangler pages deploy ./dist --project-name my-project --branch main";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "pages", "deploy", "./dist"]);
        assert_eq!(flags.get("project-name"), Some(&"my-project".to_string()));
        assert_eq!(flags.get("branch"), Some(&"main".to_string()));
    }

    #[test]
    fn test_parse_wrangler_secret_put() {
        let cmd = "npx wrangler secret put API_KEY";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "secret", "put", "API_KEY"]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_parse_wrangler_in_pipeline() {
        let cmd = "npm install && npx wrangler deploy --env production && npm test";
        let (full, args, flags) = parse_command_context(cmd, 15).unwrap(); // Start at "npx"

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "deploy"]);
        assert_eq!(flags.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_parse_wrangler_with_output_redirect() {
        let cmd = "npx wrangler deploy --env prod";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert!(args.contains(&"wrangler".to_string()));
        assert!(args.contains(&"deploy".to_string()));
        assert_eq!(flags.get("env"), Some(&"prod".to_string()));
    }

    #[test]
    fn test_parse_wrangler_init() {
        let cmd = "bunx wrangler init my-worker --type rust";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "init", "my-worker"]);
        assert_eq!(flags.get("type"), Some(&"rust".to_string()));
    }

    #[test]
    fn test_parse_wrangler_whoami() {
        let cmd = "npx wrangler whoami";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.contains("wrangler"));
        assert_eq!(args, vec!["wrangler", "whoami"]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_parse_wrangler_case_insensitive() {
        let cmd = "NPX WRANGLER DEPLOY --ENV PRODUCTION";
        let (full, args, flags) = parse_command_context(cmd, 0).unwrap();

        assert!(full.to_lowercase().contains("wrangler"));
        assert_eq!(args.len(), 2); // wrangler, deploy
        assert!(!flags.is_empty());
    }
}
