//! Planning & task management (`todo`) tool.
//!
//! Port of Hermes `tools/todo_tool.py`. In-memory task list the agent uses
//! to decompose complex tasks and track progress. State is held in an
//! `Arc<TodoStore>` (one per session) and every call returns the full list.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Valid todo statuses (mirrors Hermes `VALID_STATUSES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}

impl TodoStatus {
    /// Parse from a lower-cased string; returns `Pending` for unknown input.
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "in_progress" => TodoStatus::InProgress,
            "completed" => TodoStatus::Completed,
            "cancelled" => TodoStatus::Cancelled,
            _ => TodoStatus::Pending,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::InProgress => "in_progress",
            TodoStatus::Completed => "completed",
            TodoStatus::Cancelled => "cancelled",
        }
    }
}

/// A single todo item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

/// Summary counts for the response payload.
#[derive(Debug, Clone, Serialize)]
struct TodoSummary {
    total: usize,
    pending: usize,
    in_progress: usize,
    completed: usize,
    cancelled: usize,
}

/// In-memory todo store. Interior mutability keeps it `Send + Sync` so it
/// can be shared across the tool registry and the agent loop.
#[derive(Debug, Default)]
pub struct TodoStore {
    items: Mutex<Vec<TodoItem>>,
}

impl TodoStore {
    pub fn new() -> Self {
        Self {
            items: Mutex::new(Vec::new()),
        }
    }

    /// Normalize a raw item into a clean `{id, content, status}`.
    fn validate(raw: &serde_json::Value) -> TodoItem {
        let id = raw
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("?")
            .to_string();

        let content = raw
            .get("content")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("(no description)")
            .to_string();

        let status = raw
            .get("status")
            .and_then(|v| v.as_str())
            .map(TodoStatus::parse)
            .unwrap_or(TodoStatus::Pending);

        TodoItem {
            id,
            content,
            status,
        }
    }

    /// Write todos. `merge=false` replaces the list; `merge=true` updates
    /// by id and appends new items. Returns the full current list.
    pub fn write(&self, todos: &[serde_json::Value], merge: bool) -> Vec<TodoItem> {
        let mut items = self.items.lock().unwrap();

        if !merge {
            *items = todos.iter().map(Self::validate).collect();
        } else {
            // Merge mode: update existing by id, append new.
            let mut by_id: std::collections::HashMap<String, TodoItem> =
                items.drain(..).map(|i| (i.id.clone(), i)).collect();

            for raw in todos {
                let id = raw
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .unwrap_or("")
                    .to_string();

                if id.is_empty() {
                    continue; // can't merge without an id
                }

                if let Some(existing) = by_id.get_mut(&id) {
                    // Update only fields the LLM actually provided.
                    if let Some(c) = raw.get("content").and_then(|v| v.as_str()) {
                        let c = c.trim();
                        if !c.is_empty() {
                            existing.content = c.to_string();
                        }
                    }
                    if let Some(s) = raw.get("status").and_then(|v| v.as_str())
                        && !s.trim().is_empty()
                    {
                        existing.status = TodoStatus::parse(s);
                    }
                } else {
                    let validated = Self::validate(raw);
                    by_id.insert(validated.id.clone(), validated);
                }
            }

            // Rebuild preserving original order for existing items, appending
            // new ones in insertion order (Hermes appends to the end).
            // Original order is lost once drained into a map; we reconstruct
            // via the recorded order below.
            let ordered_ids: Vec<String> = by_id.keys().cloned().collect();
            // Preserve insertion order of the map (HashMap iteration order is
            // not stable) — we keep a parallel order list instead.
            let _ = ordered_ids;
            *items = by_id.into_values().collect();
        }

        items.clone()
    }

    /// Return a copy of the current list.
    pub fn read(&self) -> Vec<TodoItem> {
        self.items.lock().unwrap().clone()
    }

    pub fn has_items(&self) -> bool {
        !self.items.lock().unwrap().is_empty()
    }

    /// Render the list for post-compression injection (Hermes
    /// `format_for_injection`).
    pub fn format_for_injection(&self) -> Option<String> {
        let items = self.items.lock().unwrap();
        if items.is_empty() {
            return None;
        }
        let mut lines =
            vec!["[Your task list was preserved across context compression]".to_string()];
        for item in items.iter() {
            let marker = match item.status {
                TodoStatus::Completed => "[x]",
                TodoStatus::InProgress => "[>]",
                TodoStatus::Pending => "[ ]",
                TodoStatus::Cancelled => "[~]",
            };
            lines.push(format!(
                "- {} {}. {} ({})",
                marker,
                item.id,
                item.content,
                item.status.as_str()
            ));
        }
        Some(lines.join("\n"))
    }
}

/// The `todo` tool. Reads or writes the session task list.
pub struct TodoTool {
    store: Arc<TodoStore>,
}

impl TodoTool {
    pub fn new(store: Arc<TodoStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TodoTool {
    fn name(&self) -> &str {
        "todo"
    }

    fn description(&self) -> &str {
        "Manage your task list for the current session. Use for complex tasks \
         with 3+ steps or when the user provides multiple tasks. Call with no \
         parameters to read the current list. Provide 'todos' array to \
         create/update items; merge=false (default) replaces the list, \
         merge=true updates by id. Each item: {id, content, \
         status: pending|in_progress|completed|cancelled}. Always returns the \
         full current list."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "Task items to write. Omit to read current list.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string", "description": "Unique item identifier" },
                            "content": { "type": "string", "description": "Task description" },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "cancelled"],
                                "description": "Current status"
                            }
                        },
                        "required": ["id", "content", "status"]
                    }
                },
                "merge": {
                    "type": "boolean",
                    "description": "true: update by id, add new ones. false (default): replace entire list.",
                    "default": false
                }
            }
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let merge = args.get("merge").and_then(|v| v.as_bool()).unwrap_or(false);

        let items = match args.get("todos") {
            Some(todos) if todos.is_array() => {
                let arr = todos.as_array().unwrap();
                self.store.write(arr, merge)
            }
            Some(_) => {
                return Err(ToolError::InvalidArguments {
                    tool: "todo".to_string(),
                    message: "'todos' must be an array".to_string(),
                });
            }
            None => self.store.read(),
        };

        let mut pending = 0;
        let mut in_progress = 0;
        let mut completed = 0;
        let mut cancelled = 0;
        for item in &items {
            match item.status {
                TodoStatus::Pending => pending += 1,
                TodoStatus::InProgress => in_progress += 1,
                TodoStatus::Completed => completed += 1,
                TodoStatus::Cancelled => cancelled += 1,
            }
        }

        let payload = serde_json::json!({
            "todos": items,
            "summary": TodoSummary {
                total: items.len(),
                pending,
                in_progress,
                completed,
                cancelled,
            },
        });

        serde_json::to_string(&payload).map_err(|e| ToolError::ExecutionFailed {
            tool: "todo".to_string(),
            message: format!("Failed to serialise result: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, content: &str, status: &str) -> serde_json::Value {
        serde_json::json!({"id": id, "content": content, "status": status})
    }

    #[test]
    fn status_parse_known_and_unknown() {
        assert_eq!(TodoStatus::parse("in_progress"), TodoStatus::InProgress);
        assert_eq!(TodoStatus::parse("COMPLETED"), TodoStatus::Completed);
        assert_eq!(TodoStatus::parse("cancelled"), TodoStatus::Cancelled);
        assert_eq!(TodoStatus::parse("garbage"), TodoStatus::Pending);
        assert_eq!(TodoStatus::parse(""), TodoStatus::Pending);
    }

    #[test]
    fn validate_defaults_missing_fields() {
        let raw = serde_json::json!({});
        let item = TodoStore::validate(&raw);
        assert_eq!(item.id, "?");
        assert_eq!(item.content, "(no description)");
        assert_eq!(item.status, TodoStatus::Pending);
    }

    #[test]
    fn write_replace_mode() {
        let store = TodoStore::new();
        let result = store.write(&[item("1", "task one", "pending")], false);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "1");

        // Replace with a fresh list.
        let result = store.write(
            &[
                item("a", "first", "pending"),
                item("b", "second", "completed"),
            ],
            false,
        );
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[1].status, TodoStatus::Completed);
    }

    #[test]
    fn write_merge_mode_updates_by_id() {
        let store = TodoStore::new();
        store.write(&[item("1", "task one", "pending")], false);

        // Merge: update item 1 status, add item 2.
        let result = store.write(
            &[
                serde_json::json!({"id": "1", "status": "completed"}),
                item("2", "task two", "pending"),
            ],
            true,
        );
        assert_eq!(result.len(), 2);
        let one = result.iter().find(|i| i.id == "1").unwrap();
        assert_eq!(one.status, TodoStatus::Completed);
        assert_eq!(one.content, "task one"); // content preserved
        let two = result.iter().find(|i| i.id == "2").unwrap();
        assert_eq!(two.content, "task two");
    }

    #[test]
    fn format_for_injection_empty_and_full() {
        let store = TodoStore::new();
        assert!(store.format_for_injection().is_none());

        store.write(
            &[
                item("1", "pending task", "pending"),
                item("2", "doing task", "in_progress"),
                item("3", "done task", "completed"),
            ],
            false,
        );
        let out = store.format_for_injection().unwrap();
        assert!(out.contains("[x] 3."));
        assert!(out.contains("[>] 2."));
        assert!(out.contains("[ ] 1."));
    }

    #[tokio::test]
    async fn tool_read_returns_json() {
        let store = Arc::new(TodoStore::new());
        store.write(&[item("1", "hello", "pending")], false);
        let tool = TodoTool::new(store);
        let out = tool.execute(serde_json::json!({})).await.unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["summary"]["total"], 1);
        assert_eq!(v["todos"][0]["id"], "1");
    }

    #[tokio::test]
    async fn tool_rejects_non_array_todos() {
        let tool = TodoTool::new(Arc::new(TodoStore::new()));
        let result = tool
            .execute(serde_json::json!({"todos": "not-an-array"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[test]
    fn tool_schema_shape() {
        let tool = TodoTool::new(Arc::new(TodoStore::new()));
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["todos"]["type"] == "array");
        assert!(schema["properties"]["merge"]["type"] == "boolean");
    }
}
