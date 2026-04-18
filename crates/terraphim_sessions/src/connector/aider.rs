//! Aider session connector
//!
//! Parses Aider's `.aider.chat.history.md` files into Terraphim Session objects.
//! Leverages `terraphim-markdown-parser` for heading extraction and AST parsing.

use std::path::PathBuf;

use crate::connector::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{ContentBlock, Message, MessageRole, Session, SessionMetadata};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use regex::Regex;
use terraphim_markdown_parser::{extract_first_heading, normalize_markdown};
use tracing::{debug, info, warn};

/// Aider session connector
#[derive(Debug, Default)]
pub struct AiderConnector;

/// Parsed state for the Aider chat history parser
#[derive(Debug, Clone, PartialEq)]
enum ParseState {
    /// Looking for session start or user message
    Idle,
    /// Reading a user message (lines after `#### `)
    UserMessage,
    /// Reading assistant/tool output (lines after `> `)
    AssistantOutput,
}

/// A parsed message from Aider chat history
#[derive(Debug, Clone)]
struct AiderMessage {
    role: MessageRole,
    content: String,
    timestamp: Option<DateTime<Utc>>,
}

#[async_trait]
impl SessionConnector for AiderConnector {
    fn source_id(&self) -> &str {
        "aider"
    }

    fn display_name(&self) -> &str {
        "Aider"
    }

    fn detect(&self) -> ConnectorStatus {
        // Aider stores history in project directories, not a global location
        // We check for .aider.chat.history.md in common project locations
        if let Some(path) = self.default_path() {
            if path.exists() {
                let count = count_aider_history_files(&path);
                ConnectorStatus::Available {
                    path,
                    sessions_estimate: Some(count),
                }
            } else {
                ConnectorStatus::NotFound
            }
        } else {
            ConnectorStatus::NotFound
        }
    }

    fn default_path(&self) -> Option<PathBuf> {
        // Aider stores files in individual project directories
        // We use the current working directory as a starting point
        std::env::current_dir().ok()
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let base_path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        info!("Scanning for Aider sessions in: {}", base_path.display());

        let mut sessions = Vec::new();
        let history_files = find_aider_history_files(&base_path, options.limit).await?;

        info!("Found {} Aider history files", history_files.len());

        for (idx, file_path) in history_files.iter().enumerate() {
            if idx > 0 && idx % 10 == 0 {
                info!("Parsed {}/{} Aider sessions...", idx, history_files.len());
            }

            match self.parse_history_file(file_path).await {
                Ok(session) => sessions.push(session),
                Err(e) => warn!("Failed to parse {}: {}", file_path.display(), e),
            }
        }

        info!("Successfully imported {} Aider sessions", sessions.len());
        Ok(sessions)
    }
}

impl AiderConnector {
    /// Parse a single `.aider.chat.history.md` file into a Session
    async fn parse_history_file(&self, file_path: &std::path::Path) -> Result<Session> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let messages = parse_chat_history(&content)?;

        // Extract metadata from the file path and content
        let project_name = file_path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let session_id = format!(
            "aider-{}-{}",
            project_name,
            file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
        );

        // Extract session start time from first message or use file metadata
        let created_at = messages
            .first()
            .and_then(|m| m.timestamp)
            .unwrap_or_else(|| {
                std::fs::metadata(file_path)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| DateTime::from_timestamp(d.as_secs() as i64, 0))
                    .flatten()
                    .unwrap_or_else(Utc::now)
            });

        let terraphim_messages: Vec<Message> = messages
            .into_iter()
            .enumerate()
            .map(|(idx, m)| Message::text(idx, m.role, m.content))
            .collect();

        let metadata = SessionMetadata {
            project_path: file_path
                .parent()
                .and_then(|p| p.to_str())
                .map(|s| s.to_string()),
            model: None,
            tags: vec!["aider".to_string(), "chat".to_string()],
            extra: serde_json::json!({
                "title": format!("Aider session: {}", project_name),
                "description": format!("Aider chat session from {}", file_path.display()),
            }),
        };

        let started_at = jiff::Timestamp::from_second(created_at.timestamp()).ok();

        Ok(Session {
            id: session_id.clone(),
            source: "aider".to_string(),
            external_id: session_id,
            title: Some(format!("Aider session: {}", project_name)),
            source_path: file_path.to_path_buf(),
            started_at,
            ended_at: None,
            messages: terraphim_messages,
            metadata,
        })
    }
}

/// Parse Aider chat history markdown content into messages
///
/// Uses `terraphim-markdown-parser` for initial AST parsing to extract headings
/// and structured blocks, then applies a state machine to disambiguate between
/// user messages and assistant/tool output (both use `> ` prefix in Aider).
fn parse_chat_history(content: &str) -> Result<Vec<AiderMessage>> {
    let mut messages = Vec::new();
    let mut state = ParseState::Idle;
    let mut current_content = String::new();
    let mut current_role = MessageRole::User;
    let mut session_start: Option<DateTime<Utc>> = None;

    // Extract session start time from h1 heading using terraphim-markdown-parser
    if let Some(heading) = extract_first_heading(content) {
        // Parse: "aider chat started at YYYY-MM-DD HH:MM:SS"
        if let Some(dt_str) = heading.strip_prefix("aider chat started at ") {
            session_start = DateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|dt| dt.with_timezone(&Utc));
        }
    }

    // Parse markdown for structured content extraction (used for headings)
    let _normalized = normalize_markdown(content)?;

    // Regex for session start line (fallback if heading parser misses it)
    let session_start_re =
        Regex::new(r"# aider chat started at (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for session start (fallback)
        if let Some(caps) = session_start_re.captures(trimmed) {
            if let Some(dt_str) = caps.get(1) {
                session_start = DateTime::parse_from_str(dt_str.as_str(), "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            }

            // Save previous message if any
            if !current_content.is_empty() {
                messages.push(AiderMessage {
                    role: current_role.clone(),
                    content: current_content.trim().to_string(),
                    timestamp: session_start,
                });
            }

            state = ParseState::Idle;
            current_content.clear();
            continue;
        }

        // Check for user message start (h4 heading)
        if trimmed.starts_with("#### ") {
            // Save previous message if any
            if !current_content.is_empty() {
                messages.push(AiderMessage {
                    role: current_role.clone(),
                    content: current_content.trim().to_string(),
                    timestamp: session_start,
                });
            }

            current_role = MessageRole::User;
            current_content = trimmed.strip_prefix("#### ").unwrap_or("").to_string();
            state = ParseState::UserMessage;
            continue;
        }

        // Check for assistant/tool output start (blockquote)
        if trimmed.starts_with("> ") {
            // In Aider's format, `> ` can be either:
            // 1. Assistant output (actual response)
            // 2. Tool output (file edits, git commands, etc.)
            //
            // Heuristic: If we're in Idle state and see `> `, it's likely assistant output.
            // If we're already in AssistantOutput state, continue accumulating.
            // If we're in UserMessage state, this is a new assistant message.
            match state {
                ParseState::UserMessage => {
                    // Save user message
                    if !current_content.is_empty() {
                        messages.push(AiderMessage {
                            role: current_role.clone(),
                            content: current_content.trim().to_string(),
                            timestamp: session_start,
                        });
                    }

                    current_role = MessageRole::Assistant;
                    current_content = trimmed.strip_prefix("> ").unwrap_or("").to_string();
                    state = ParseState::AssistantOutput;
                }
                ParseState::Idle | ParseState::AssistantOutput => {
                    // Continue or start assistant output
                    if state == ParseState::Idle {
                        current_role = MessageRole::Assistant;
                        current_content.clear();
                    }
                    if !current_content.is_empty() {
                        current_content.push('\n');
                    }
                    current_content.push_str(trimmed.strip_prefix("> ").unwrap_or(""));
                    state = ParseState::AssistantOutput;
                }
            }
            continue;
        }

        // Continue current message (blank lines or continuation)
        match state {
            ParseState::UserMessage | ParseState::AssistantOutput => {
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(trimmed);
            }
            ParseState::Idle => {
                // Skip empty lines in idle state
                if !trimmed.is_empty() {
                    debug!("Unexpected content in idle state: {}", trimmed);
                }
            }
        }
    }

    // Don't forget the last message
    if !current_content.is_empty() {
        messages.push(AiderMessage {
            role: current_role,
            content: current_content.trim().to_string(),
            timestamp: session_start,
        });
    }

    Ok(messages)
}

/// Find all `.aider.chat.history.md` files in a directory tree
async fn find_aider_history_files(
    base_path: &std::path::Path,
    limit: Option<usize>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs_to_visit = vec![base_path.to_path_buf()];

    while let Some(current_dir) = dirs_to_visit.pop() {
        let mut entries = match tokio::fs::read_dir(&current_dir).await {
            Ok(entries) => entries,
            Err(_) => continue, // Skip directories we can't read
        };

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_dir() {
                dirs_to_visit.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name == ".aider.chat.history.md")
            {
                files.push(path);
            }

            if let Some(max) = limit {
                if files.len() >= max {
                    return Ok(files);
                }
            }
        }
    }

    Ok(files)
}

/// Count Aider history files (for detection estimate)
fn count_aider_history_files(base_path: &std::path::Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_aider_history_files(&path);
            } else if path
                .file_name()
                .is_some_and(|name| name == ".aider.chat.history.md")
            {
                count += 1;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_chat() {
        let content = r#"# aider chat started at 2025-06-19 14:32:16

#### Add a function to calculate fibonacci

> I'll add a fibonacci function to the math module.
>
> ```rust
> pub fn fibonacci(n: u32) -> u32 {
>     match n {
>         0 => 0,
>         1 => 1,
>         _ => fibonacci(n - 1) + fibonacci(n - 2),
>     }
> }
> ```
>
> Applied edit to src/math.rs

#### Now add tests for it

> I'll add tests for the fibonacci function.
>
> ```rust
> #[test]
> fn test_fibonacci() {
>     assert_eq!(fibonacci(0), 0);
>     assert_eq!(fibonacci(1), 1);
>     assert_eq!(fibonacci(10), 55);
> }
> ```
"#;

        let messages = parse_chat_history(content).unwrap();
        assert_eq!(messages.len(), 4);
        assert_eq!(messages[0].role, MessageRole::User);
        assert_eq!(messages[0].content, "Add a function to calculate fibonacci");
        assert_eq!(messages[1].role, MessageRole::Assistant);
        assert!(messages[1].content.contains("fibonacci function"));
        assert_eq!(messages[2].role, MessageRole::User);
        assert_eq!(messages[2].content, "Now add tests for it");
        assert_eq!(messages[3].role, MessageRole::Assistant);
    }

    #[test]
    fn test_parse_with_metadata() {
        let content = r#"# aider chat started at 2025-06-19 14:32:16

> Model: ollama_chat/qwen3:8b with whole edit format
> Git repo: /home/user/project
> Repo-map: using 1024 tokens

#### What files are in this project?

> Let me check what files are in the project.
>
> The project contains:
> - src/main.rs
> - Cargo.toml
"#;

        let messages = parse_chat_history(content).unwrap();
        // The metadata lines at the start should be treated as assistant output
        assert!(messages.len() >= 2);
        assert_eq!(messages.last().unwrap().role, MessageRole::Assistant);
    }
}
