use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Filesystem tool for reading, writing, and listing files.
pub struct FilesystemTool;

impl FilesystemTool {
    /// Create a new filesystem tool.
    pub fn new() -> Self {
        Self
    }

    /// Read a file's contents.
    async fn read_file(&self, path: &Path) -> Result<String, ToolError> {
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "read_file".to_string(),
                    message: format!("Failed to read file '{}': {}", path.display(), e),
                })?;
        Ok(content)
    }

    /// Write content to a file.
    async fn write_file(&self, path: &Path, content: &str) -> Result<String, ToolError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "write_file".to_string(),
                    message: format!("Failed to create directory '{}': {}", parent.display(), e),
                })?;
        }

        tokio::fs::write(path, content)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "write_file".to_string(),
                message: format!("Failed to write file '{}': {}", path.display(), e),
            })?;

        Ok(format!("Successfully wrote to {}", path.display()))
    }

    /// List directory contents.
    async fn list_directory(&self, path: &Path) -> Result<String, ToolError> {
        let mut entries =
            tokio::fs::read_dir(path)
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "list_directory".to_string(),
                    message: format!("Failed to read directory '{}': {}", path.display(), e),
                })?;

        let mut result = Vec::new();
        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "list_directory".to_string(),
                    message: format!("Failed to read directory entry: {}", e),
                })?
        {
            let file_type = entry.file_type().await.ok();
            let type_str = if file_type.map(|t| t.is_dir()).unwrap_or(false) {
                "[DIR]"
            } else {
                "[FILE]"
            };
            result.push(format!(
                "{} {}",
                type_str,
                entry.file_name().to_string_lossy()
            ));
        }

        if result.is_empty() {
            Ok("Directory is empty".to_string())
        } else {
            Ok(result.join("\n"))
        }
    }
}

impl Default for FilesystemTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FilesystemTool {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn description(&self) -> &str {
        "Read, write, and list files and directories. \
         Supports: read_file, write_file, list_directory operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["read_file", "write_file", "list_directory"],
                    "description": "The filesystem operation to perform"
                },
                "path": {
                    "type": "string",
                    "description": "Path to the file or directory"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write (required for write_file)"
                }
            },
            "required": ["operation", "path"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let operation = args["operation"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "filesystem".to_string(),
                message: "Missing 'operation' parameter".to_string(),
            })?;

        let path_str = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "filesystem".to_string(),
                message: "Missing 'path' parameter".to_string(),
            })?;

        let path = PathBuf::from(path_str);

        match operation {
            "read_file" => self.read_file(&path).await,
            "write_file" => {
                let content =
                    args["content"]
                        .as_str()
                        .ok_or_else(|| ToolError::InvalidArguments {
                            tool: "filesystem".to_string(),
                            message: "Missing 'content' parameter for write_file".to_string(),
                        })?;
                self.write_file(&path, content).await
            }
            "list_directory" => self.list_directory(&path).await,
            _ => Err(ToolError::InvalidArguments {
                tool: "filesystem".to_string(),
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
    async fn test_tool_execute_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "Hello, World!").await.unwrap();

        let tool = FilesystemTool::new();
        let args = serde_json::json!({
            "operation": "read_file",
            "path": test_file.to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_tool_execute_read_missing_file() {
        let tool = FilesystemTool::new();
        let args = serde_json::json!({
            "operation": "read_file",
            "path": "/nonexistent/path/file.txt"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ToolError::ExecutionFailed { .. })));
    }

    #[tokio::test]
    async fn test_tool_execute_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("write_test.txt");

        let tool = FilesystemTool::new();
        let args = serde_json::json!({
            "operation": "write_file",
            "path": test_file.to_str().unwrap(),
            "content": "Test content"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Successfully wrote"));

        // Verify the file was written
        let content = tokio::fs::read_to_string(&test_file).await.unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_tool_execute_list_directory() {
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content")
            .await
            .unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir"))
            .await
            .unwrap();

        let tool = FilesystemTool::new();
        let args = serde_json::json!({
            "operation": "list_directory",
            "path": temp_dir.path().to_str().unwrap()
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("[FILE] file1.txt"));
        assert!(result.contains("[DIR] subdir"));
    }
}
