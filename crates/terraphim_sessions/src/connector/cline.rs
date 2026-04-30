//! Cline session connector
//!
//! Parses Cline's (VS Code extension) session data into Terraphim Session objects.
//! Supports the `cline-connector` feature flag.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::{ConnectorStatus, ImportOptions, SessionConnector};
use crate::model::{Message, MessageRole, Session, SessionMetadata};

/// Session connector for the Cline VS Code extension.
#[derive(Debug)]
pub struct ClineConnector {
    default_path: PathBuf,
}

#[derive(Debug, Deserialize, Serialize)]
struct HistoryItem {
    id: String,
    #[serde(rename = "ulid")]
    ulid: Option<String>,
    ts: i64,
    task: String,
    #[serde(rename = "tokensIn")]
    tokens_in: Option<i64>,
    #[serde(rename = "tokensOut")]
    tokens_out: Option<i64>,
    #[serde(rename = "cacheWrites")]
    cache_writes: Option<i64>,
    #[serde(rename = "cacheReads")]
    cache_reads: Option<i64>,
    #[serde(rename = "totalCost")]
    total_cost: Option<f64>,
    #[serde(rename = "cwdOnTaskInitialization")]
    cwd: Option<String>,
    #[serde(rename = "conversationHistoryDeletedRange")]
    deleted_range: Option<Vec<i64>>,
    #[serde(rename = "isFavorited")]
    is_favorited: Option<bool>,
    #[serde(rename = "checkpointManagerErrorMessage")]
    checkpoint_error: Option<String>,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ApiMessage {
    role: String,
    content: serde_json::Value,
}

/// A single message entry from a Cline conversation file.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClineMessage {
    ts: Option<i64>,
    #[serde(rename = "type")]
    message_type: String,
    ask: Option<String>,
    say: Option<String>,
    text: Option<String>,
    reasoning: Option<String>,
    images: Option<Vec<String>>,
    files: Option<Vec<String>>,
    partial: Option<bool>,
    #[serde(rename = "conversationHistoryIndex")]
    conversation_history_index: Option<i64>,
    #[serde(rename = "conversationHistoryDeletedRange")]
    conversation_history_deleted_range: Option<Vec<i64>>,
    model_info: Option<ModelInfo>,
}

/// Model identity information embedded in a Cline message.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    #[serde(rename = "providerId")]
    provider_id: Option<String>,
    #[serde(rename = "modelId")]
    model_id: Option<String>,
    mode: Option<String>,
}

impl ClineConnector {
    /// Creates a new connector pointing at the default Cline global storage path.
    pub fn new() -> Self {
        Self {
            default_path: cline_global_storage_path(),
        }
    }

    fn task_history_path(&self, base: &Path) -> PathBuf {
        base.join("state").join("taskHistory.json")
    }

    fn task_dir(&self, base: &Path, task_id: &str) -> PathBuf {
        base.join("tasks").join(task_id)
    }

    fn api_history_path(&self, task_dir: &Path) -> PathBuf {
        task_dir.join("api_conversation_history.json")
    }

    fn ui_messages_path(&self, task_dir: &Path) -> PathBuf {
        task_dir.join("ui_messages.json")
    }

    fn extract_text(content: &serde_json::Value) -> String {
        if let Some(s) = content.as_str() {
            s.to_string()
        } else if let Some(arr) = content.as_array() {
            arr.iter()
                .filter_map(|block| {
                    if block.get("type")?.as_str()? == "text" {
                        block.get("text")?.as_str()
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            String::new()
        }
    }

    fn extract_tools(content: &serde_json::Value) -> Vec<String> {
        content
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| {
                        block
                            .get("type")
                            .and_then(|t| t.as_str())
                            .filter(|t| *t == "tool_use")
                            .and_then(|_| block.get("name"))
                            .and_then(|n| n.as_str())
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_api_messages(&self, path: &Path) -> Result<Vec<Message>> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read api_conversation_history.json")?;
        let messages: Vec<ApiMessage> = serde_json::from_str(&content)
            .context("Failed to parse api_conversation_history.json")?;

        let mut result = Vec::new();
        for (idx, msg) in messages.iter().enumerate() {
            let role = match msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            let text = Self::extract_text(&msg.content);
            let tools = Self::extract_tools(&msg.content);
            let mut extra = serde_json::json!({});
            if !tools.is_empty() {
                extra["tools"] = serde_json::json!(tools);
            }

            result.push(Message {
                idx,
                role,
                author: None,
                content: text,
                created_at: None,
                extra,
                blocks: Vec::new(),
            });
        }
        Ok(result)
    }

    fn parse_ui_messages(&self, path: &Path) -> Result<Vec<Message>> {
        let content = std::fs::read_to_string(path).context("Failed to read ui_messages.json")?;
        let messages: Vec<ClineMessage> =
            serde_json::from_str(&content).context("Failed to parse ui_messages.json")?;

        let mut result = Vec::new();
        for (idx, msg) in messages.iter().enumerate() {
            let role = match msg.message_type.as_str() {
                "ask" => MessageRole::User,
                "say" => MessageRole::Assistant,
                _ => continue,
            };

            let created_at = msg
                .ts
                .and_then(|ts| jiff::Timestamp::from_millisecond(ts).ok());

            let mut extra = serde_json::json!({});
            if let Some(ref ask) = msg.ask {
                extra["ask_type"] = serde_json::json!(ask);
            }
            if let Some(ref say) = msg.say {
                extra["say_type"] = serde_json::json!(say);
            }
            if msg.partial == Some(true) {
                extra["is_partial"] = serde_json::json!(true);
            }
            if msg.images.is_some() {
                extra["has_images"] = serde_json::json!(true);
            }
            if let Some(ref reasoning) = msg.reasoning {
                extra["reasoning"] = serde_json::json!(reasoning);
            }

            let author = msg.model_info.as_ref().and_then(|m| m.model_id.clone());

            result.push(Message {
                idx,
                role,
                author,
                content: msg.text.clone().unwrap_or_default(),
                created_at,
                extra,
                blocks: Vec::new(),
            });
        }
        Ok(result)
    }
}

fn cline_global_storage_path() -> PathBuf {
    if let Some(base) = dirs::data_dir() {
        let vscode_path = base
            .join("Code")
            .join("User")
            .join("globalStorage")
            .join("saoudrizwan.claude-dev");
        if vscode_path.exists() {
            return vscode_path;
        }

        let claude_dev_path = base.join("saoudrizwan.claude-dev");
        if claude_dev_path.exists() {
            return claude_dev_path;
        }

        if let Some(home) = dirs::home_dir() {
            let cline_path = home.join(".cline");
            if cline_path.exists() {
                return cline_path;
            }
        }

        base.join("saoudrizwan.claude-dev")
    } else {
        PathBuf::from("~/.cline")
    }
}

impl Default for ClineConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionConnector for ClineConnector {
    fn source_id(&self) -> &str {
        "cline"
    }

    fn display_name(&self) -> &str {
        "Cline"
    }

    fn detect(&self) -> ConnectorStatus {
        if let Some(path) = self.default_path() {
            if path.exists() {
                let count = WalkDir::new(&path)
                    .max_depth(3)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .file_name()
                            .is_some_and(|n| n == "taskHistory.json")
                    })
                    .count();
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
        Some(self.default_path.clone())
    }

    async fn import(&self, options: &ImportOptions) -> Result<Vec<Session>> {
        let base_path = options
            .path
            .clone()
            .or_else(|| self.default_path())
            .ok_or_else(|| anyhow::anyhow!("No path specified and default not found"))?;

        let history_path = self.task_history_path(&base_path);
        if !history_path.exists() {
            return Ok(Vec::new());
        }

        let content =
            std::fs::read_to_string(&history_path).context("Failed to read taskHistory.json")?;
        let history: Vec<HistoryItem> =
            serde_json::from_str(&content).context("Failed to parse taskHistory.json")?;

        let mut sessions = Vec::new();

        for item in history {
            let task_dir = self.task_dir(&base_path, &item.id);
            let api_path = self.api_history_path(&task_dir);
            let ui_path = self.ui_messages_path(&task_dir);

            if !task_dir.exists() {
                continue;
            }

            let mut messages = if api_path.exists() {
                self.parse_api_messages(&api_path).unwrap_or_default()
            } else {
                Vec::new()
            };

            if ui_path.exists() {
                let ui_messages = self.parse_ui_messages(&ui_path).unwrap_or_default();
                messages.extend(ui_messages);
            }

            messages.sort_by_key(|m| m.idx);

            let started_at = jiff::Timestamp::from_millisecond(item.ts)
                .expect("Invalid timestamp in taskHistory");

            let mut metadata = SessionMetadata {
                project_path: item.cwd.clone(),
                ..Default::default()
            };

            let mut extra = serde_json::json!({});
            if let Some(ref ulid) = item.ulid {
                extra["ulid"] = serde_json::json!(ulid);
            }
            if let Some(tokens_in) = item.tokens_in {
                extra["tokens_in"] = serde_json::json!(tokens_in);
            }
            if let Some(tokens_out) = item.tokens_out {
                extra["tokens_out"] = serde_json::json!(tokens_out);
            }
            if let Some(cost) = item.total_cost {
                extra["total_cost"] = serde_json::json!(cost);
            }
            if item.is_favorited == Some(true) {
                extra["is_favorited"] = serde_json::json!(true);
            }
            if let Some(ref err) = item.checkpoint_error {
                extra["checkpoint_error"] = serde_json::json!(err);
            }
            if let Some(ref model) = item.model_id {
                extra["model_id"] = serde_json::json!(model);
            }
            metadata.extra = extra;

            sessions.push(Session {
                id: item.id.clone(),
                external_id: item.id.clone(),
                title: Some(item.task.clone()),
                source: "cline".to_string(),
                source_path: task_dir,
                started_at: Some(started_at),
                ended_at: None,
                messages,
                metadata,
            });
        }

        Ok(sessions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_text_from_string() {
        let content = serde_json::json!("Hello world");
        assert_eq!(ClineConnector::extract_text(&content), "Hello world");
    }

    #[test]
    fn test_extract_text_from_array() {
        let content = serde_json::json!([
            {"type": "text", "text": "First"},
            {"type": "tool_use", "name": "read_file"},
            {"type": "text", "text": "Second"}
        ]);
        assert_eq!(ClineConnector::extract_text(&content), "First\nSecond");
    }

    #[test]
    fn test_extract_tools() {
        let content = serde_json::json!([
            {"type": "text", "text": "Hello"},
            {"type": "tool_use", "name": "read_file", "input": {}},
            {"type": "tool_use", "name": "write_file", "input": {}}
        ]);
        let tools = ClineConnector::extract_tools(&content);
        assert_eq!(tools, vec!["read_file", "write_file"]);
    }

    #[test]
    fn test_cline_message_parse() {
        let json = r#"{
            "ts": 1712345678901,
            "type": "say",
            "say": "task",
            "text": "Implement auth",
            "modelInfo": {"providerId": "anthropic", "modelId": "claude-3-5-sonnet-20241022", "mode": "act"}
        }"#;
        let msg: ClineMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.message_type, "say");
        assert_eq!(msg.text.as_ref().unwrap(), "Implement auth");
    }

    #[test]
    fn test_history_item_parse() {
        let json = r#"{
            "id": "1712345678901",
            "ulid": "01HV8J3K2M4N5P6Q7R8S9T0UV",
            "ts": 1712345678901,
            "task": "Implement auth",
            "tokensIn": 15000,
            "tokensOut": 8000,
            "totalCost": 0.45,
            "cwdOnTaskInitialization": "/home/user/project",
            "modelId": "claude-3-5-sonnet-20241022"
        }"#;
        let item: HistoryItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "1712345678901");
        assert_eq!(item.task, "Implement auth");
        assert!(item.ulid.is_some());
    }
}
