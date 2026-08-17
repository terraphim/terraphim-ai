//! Interactive clarification tool.
//!
//! Port of Hermes `tools/clarify_tool.py`. Lets the agent present a structured
//! question (optionally with up to 4 multiple-choice options) to the user.
//! The actual user interaction lives in the channel layer; this tool validates
//! the request and returns a structured `awaiting_user` payload that the
//! channel/loop can surface, or invokes a callback when one is wired.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;

/// Maximum predefined choices (a 5th "Other" is always implied by the UI).
pub const MAX_CHOICES: usize = 4;

/// Optional callback that performs the actual UI interaction.
/// Signature: `(question, choices) -> user_response`.
type ClarifyCallback = dyn Fn(&str, Option<Vec<String>>) -> Result<String, String> + Send + Sync;

/// The `clarify` tool.
pub struct ClarifyTool {
    callback: Option<std::sync::Arc<ClarifyCallback>>,
}

impl ClarifyTool {
    pub fn new() -> Self {
        Self { callback: None }
    }

    /// Attach a platform-provided callback for actual user interaction.
    pub fn with_callback<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str, Option<Vec<String>>) -> Result<String, String> + Send + Sync + 'static,
    {
        self.callback = Some(std::sync::Arc::new(callback));
        self
    }
}

impl Default for ClarifyTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ClarifyTool {
    fn name(&self) -> &str {
        "clarify"
    }

    fn description(&self) -> &str {
        "Ask the user a question when you need clarification, feedback, or a \
         decision before proceeding. Supports multiple choice (up to 4 choices) \
         or open-ended (omit choices). Use when the task is ambiguous or a \
         decision has meaningful trade-offs. Do NOT use for simple yes/no \
         confirmation of dangerous commands."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to present to the user."
                },
                "choices": {
                    "type": "array",
                    "items": { "type": "string" },
                    "maxItems": MAX_CHOICES,
                    "description": "Up to 4 answer choices. Omit for an open-ended question."
                }
            },
            "required": ["question"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let question = args["question"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "clarify".to_string(),
                message: "Question text is required.".to_string(),
            })?;

        // Validate and trim choices.
        let choices: Option<Vec<String>> = match args.get("choices") {
            None => None,
            Some(serde_json::Value::Array(arr)) => {
                let trimmed: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .take(MAX_CHOICES)
                    .collect();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            }
            Some(_) => {
                return Err(ToolError::InvalidArguments {
                    tool: "clarify".to_string(),
                    message: "choices must be a list of strings.".to_string(),
                });
            }
        };

        let payload = match &self.callback {
            Some(cb) => match cb(question, choices.clone()) {
                Ok(response) => serde_json::json!({
                    "question": question,
                    "choices_offered": choices,
                    "user_response": response.trim(),
                }),
                Err(e) => serde_json::json!({
                    "question": question,
                    "choices_offered": choices,
                    "error": e,
                }),
            },
            None => serde_json::json!({
                "status": "awaiting_user",
                "question": question,
                "choices_offered": choices,
                "user_response": null,
            }),
        };

        serde_json::to_string(&payload).map_err(|e| ToolError::ExecutionFailed {
            tool: "clarify".to_string(),
            message: format!("Failed to serialise result: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn missing_question_is_error() {
        let tool = ClarifyTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn blank_question_is_error() {
        let tool = ClarifyTool::new();
        let result = tool.execute(serde_json::json!({"question": "   "})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn open_ended_returns_awaiting_user() {
        let tool = ClarifyTool::new();
        let out = tool
            .execute(serde_json::json!({"question": "How did that work?"}))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "awaiting_user");
        assert_eq!(v["question"], "How did that work?");
        assert!(v["user_response"].is_null());
    }

    #[tokio::test]
    async fn choices_trimmed_and_capped() {
        let tool = ClarifyTool::new();
        let out = tool
            .execute(serde_json::json!({
                "question": "Pick one",
                "choices": [" A ", "", "B", "C", "D", "E"]
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let choices = v["choices_offered"].as_array().unwrap();
        assert_eq!(choices.len(), MAX_CHOICES); // capped at 4, empty dropped
        assert_eq!(choices[0], "A");
        assert_eq!(choices[1], "B");
    }

    #[tokio::test]
    async fn non_array_choices_is_error() {
        let tool = ClarifyTool::new();
        let result = tool
            .execute(serde_json::json!({"question": "q", "choices": "not-array"}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn callback_invoked_when_wired() {
        let tool = ClarifyTool::new().with_callback(|q, _choices| {
            assert_eq!(q, "Pick one");
            Ok("answer".to_string())
        });
        let out = tool
            .execute(serde_json::json!({"question": "Pick one"}))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["user_response"], "answer");
    }

    #[test]
    fn schema_shape() {
        let tool = ClarifyTool::new();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(
            schema["required"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("question"))
        );
        assert_eq!(schema["properties"]["choices"]["maxItems"], MAX_CHOICES);
    }
}
