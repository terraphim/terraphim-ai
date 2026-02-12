use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::PathBuf;

/// Edit tool for file modifications with uniqueness guard.
pub struct EditTool;

impl EditTool {
    /// Create a new edit tool.
    pub fn new() -> Self {
        Self
    }

    /// Apply an edit to a file.
    /// Replaces old_string with new_string.
    /// Fails if old_string is not unique in the file.
    async fn apply_edit(
        &self,
        path: &PathBuf,
        old_string: &str,
        new_string: &str,
    ) -> Result<String, ToolError> {
        // Read the file
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "edit".to_string(),
                    message: format!("Failed to read file '{}': {}", path.display(), e),
                })?;

        // Count occurrences of old_string
        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            return Err(ToolError::ExecutionFailed {
                tool: "edit".to_string(),
                message: format!(
                    "String not found in file: '{}'\n\
                     Hint: The text you're trying to replace doesn't exist in the file.",
                    old_string
                ),
            });
        }

        if occurrences > 1 {
            return Err(ToolError::ExecutionFailed {
                tool: "edit".to_string(),
                message: format!(
                    "String appears {} times in file: '{}'\n\
                     Hint: Make the old_string more unique by including more context.",
                    occurrences, old_string
                ),
            });
        }

        // Apply the replacement
        let new_content = content.replacen(old_string, new_string, 1);

        // Write back
        tokio::fs::write(path, new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "edit".to_string(),
                message: format!("Failed to write file '{}': {}", path.display(), e),
            })?;

        Ok(format!(
            "Successfully edited {} (replaced 1 occurrence)",
            path.display()
        ))
    }

    /// Insert text at a specific line number.
    async fn insert_at_line(
        &self,
        path: &PathBuf,
        line_number: usize,
        text: &str,
    ) -> Result<String, ToolError> {
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "edit".to_string(),
                    message: format!("Failed to read file '{}': {}", path.display(), e),
                })?;

        let lines: Vec<&str> = content.lines().collect();

        if line_number > lines.len() {
            return Err(ToolError::InvalidArguments {
                tool: "edit".to_string(),
                message: format!(
                    "Line number {} is beyond file length of {} lines",
                    line_number,
                    lines.len()
                ),
            });
        }

        let mut new_lines = lines.clone();
        new_lines.insert(line_number, text);

        let new_content = new_lines.join("\n");
        tokio::fs::write(path, new_content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "edit".to_string(),
                message: format!("Failed to write file '{}': {}", path.display(), e),
            })?;

        Ok(format!(
            "Successfully inserted text at line {} in {}",
            line_number,
            path.display()
        ))
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "Edit files by replacing text. \
         Requires old_string to be unique in the file. \
         Supports: str_replace, insert_at_line operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "old_string": {
                    "type": "string",
                    "description": "The text to replace (must be unique in file)"
                },
                "new_string": {
                    "type": "string",
                    "description": "The text to replace with"
                },
                "operation": {
                    "type": "string",
                    "enum": ["str_replace", "insert_at_line"],
                    "description": "The edit operation to perform"
                },
                "line_number": {
                    "type": "integer",
                    "description": "Line number for insert operation (0-indexed)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to insert for insert_at_line operation"
                }
            },
            "required": ["path", "operation"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "edit".to_string(),
                message: "Missing 'path' parameter".to_string(),
            })?;

        let path = PathBuf::from(path_str);
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "edit".to_string(),
                message: "Missing 'operation' parameter".to_string(),
            })?;

        match operation {
            "str_replace" => {
                let old_string =
                    args["old_string"]
                        .as_str()
                        .ok_or_else(|| ToolError::InvalidArguments {
                            tool: "edit".to_string(),
                            message: "Missing 'old_string' parameter for str_replace".to_string(),
                        })?;
                let new_string =
                    args["new_string"]
                        .as_str()
                        .ok_or_else(|| ToolError::InvalidArguments {
                            tool: "edit".to_string(),
                            message: "Missing 'new_string' parameter for str_replace".to_string(),
                        })?;
                self.apply_edit(&path, old_string, new_string).await
            }
            "insert_at_line" => {
                let line_number =
                    args["line_number"]
                        .as_u64()
                        .ok_or_else(|| ToolError::InvalidArguments {
                            tool: "edit".to_string(),
                            message: "Missing or invalid 'line_number' parameter".to_string(),
                        })? as usize;
                let text = args["text"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "edit".to_string(),
                        message: "Missing 'text' parameter for insert_at_line".to_string(),
                    })?;
                self.insert_at_line(&path, line_number, text).await
            }
            _ => Err(ToolError::InvalidArguments {
                tool: "edit".to_string(),
                message: format!("Unknown operation: {}", operation),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_edit_tool_uniqueness_guard() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        // Write file with duplicate text
        tokio::fs::write(&test_file, "line1\nunique\nline3\nunique\nline5")
            .await
            .unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "path": test_file.to_str().unwrap(),
            "operation": "str_replace",
            "old_string": "unique",
            "new_string": "replaced"
        });

        // Should fail because "unique" appears twice
        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("appears 2 times"));
    }

    #[tokio::test]
    async fn test_edit_tool_successful_replace() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        tokio::fs::write(&test_file, "Hello, World!\nHow are you?")
            .await
            .unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "path": test_file.to_str().unwrap(),
            "operation": "str_replace",
            "old_string": "Hello, World!",
            "new_string": "Hello, Universe!"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Successfully edited"));

        // Verify the change
        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert!(content.contains("Hello, Universe!"));
        assert!(!content.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_edit_tool_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");

        tokio::fs::write(&test_file, "Hello, World!").await.unwrap();

        let tool = EditTool::new();
        let args = serde_json::json!({
            "path": test_file.to_str().unwrap(),
            "operation": "str_replace",
            "old_string": "This does not exist",
            "new_string": "Replacement"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
