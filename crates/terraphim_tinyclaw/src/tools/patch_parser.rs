//! V4A patch format parser.
//!
//! Port of Hermes `tools/patch_parser.py`. Parses the V4A patch format used by
//! codex/cline-style agents into structured [`PatchOperation`]s. Application is
//! handled separately via [`super::fuzzy_match::fuzzy_find_and_replace`].

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use serde::Serialize;

/// Patch operation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationType {
    Add,
    Update,
    Delete,
    Move,
}

/// A single line in a patch hunk.
#[derive(Debug, Clone, Serialize)]
pub struct HunkLine {
    /// `' '`, `'-'`, or `'+'`.
    pub prefix: char,
    pub content: String,
}

/// A group of changes within a file.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Hunk {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_hint: Option<String>,
    #[serde(default)]
    pub lines: Vec<HunkLine>,
}

/// A single operation in a V4A patch.
#[derive(Debug, Clone, Serialize)]
pub struct PatchOperation {
    pub operation: OperationType,
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>,
    #[serde(default)]
    pub hunks: Vec<Hunk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

enum Marker<'a> {
    Update(&'a str),
    Add(&'a str),
    Delete(&'a str),
    Move(&'a str, &'a str),
}

fn parse_marker(line: &str) -> Option<Marker<'_>> {
    let rest = line.strip_prefix("***")?.trim_start();
    if let Some(p) = rest.strip_prefix("Update File:") {
        Some(Marker::Update(p.trim()))
    } else if let Some(p) = rest.strip_prefix("Add File:") {
        Some(Marker::Add(p.trim()))
    } else if let Some(p) = rest.strip_prefix("Delete File:") {
        Some(Marker::Delete(p.trim()))
    } else if let Some(p) = rest.strip_prefix("Move File:") {
        let (from, to) = p.split_once("->")?;
        Some(Marker::Move(from.trim(), to.trim()))
    } else {
        None
    }
}

fn extract_hint(line: &str) -> Option<String> {
    let inner = line.strip_prefix("@@")?.trim();
    let inner = inner.strip_suffix("@@")?.trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

fn flush(current_op: &mut Option<PatchOperation>, current_hunk: &mut Option<Hunk>) {
    if let Some(op) = current_op.as_mut()
        && let Some(hunk) = current_hunk.take()
        && !hunk.lines.is_empty()
    {
        op.hunks.push(hunk);
    }
}

/// Parse a V4A-format patch.
///
/// Returns `(operations, error)`. On success `error` is `None`.
pub fn parse_v4a_patch(patch: &str) -> (Vec<PatchOperation>, Option<String>) {
    let lines: Vec<&str> = patch.split('\n').collect();
    let mut operations: Vec<PatchOperation> = Vec::new();

    // Locate patch boundaries (Begin/End markers).
    let mut start_idx: i64 = -1;
    let mut end_idx = lines.len();
    for (i, line) in lines.iter().enumerate() {
        if line.contains("*** Begin Patch") || line.contains("***Begin Patch") {
            start_idx = i as i64;
        } else if line.contains("*** End Patch") || line.contains("***End Patch") {
            end_idx = i;
            break;
        }
    }

    let mut i = (start_idx + 1) as usize;
    let mut current_op: Option<PatchOperation> = None;
    let mut current_hunk: Option<Hunk> = None;

    while i < end_idx {
        let line = lines[i];

        if let Some(marker) = parse_marker(line) {
            match marker {
                Marker::Update(path) => {
                    flush(&mut current_op, &mut current_hunk);
                    if let Some(op) = current_op.take() {
                        operations.push(op);
                    }
                    current_op = Some(PatchOperation {
                        operation: OperationType::Update,
                        file_path: path.to_string(),
                        new_path: None,
                        hunks: Vec::new(),
                        content: None,
                    });
                    current_hunk = None;
                }
                Marker::Add(path) => {
                    flush(&mut current_op, &mut current_hunk);
                    if let Some(op) = current_op.take() {
                        operations.push(op);
                    }
                    current_op = Some(PatchOperation {
                        operation: OperationType::Add,
                        file_path: path.to_string(),
                        new_path: None,
                        hunks: Vec::new(),
                        content: None,
                    });
                    current_hunk = Some(Hunk::default());
                }
                Marker::Delete(path) => {
                    flush(&mut current_op, &mut current_hunk);
                    if let Some(op) = current_op.take() {
                        operations.push(op);
                    }
                    operations.push(PatchOperation {
                        operation: OperationType::Delete,
                        file_path: path.to_string(),
                        new_path: None,
                        hunks: Vec::new(),
                        content: None,
                    });
                    current_op = None;
                    current_hunk = None;
                }
                Marker::Move(from, to) => {
                    flush(&mut current_op, &mut current_hunk);
                    if let Some(op) = current_op.take() {
                        operations.push(op);
                    }
                    operations.push(PatchOperation {
                        operation: OperationType::Move,
                        file_path: from.to_string(),
                        new_path: Some(to.to_string()),
                        hunks: Vec::new(),
                        content: None,
                    });
                    current_op = None;
                    current_hunk = None;
                }
            }
        } else if line.starts_with("@@") {
            if current_op.is_some() {
                flush(&mut current_op, &mut current_hunk);
                current_hunk = Some(Hunk {
                    context_hint: extract_hint(line),
                    lines: Vec::new(),
                });
            }
        } else if current_op.is_some() && !line.is_empty() {
            if current_hunk.is_none() {
                current_hunk = Some(Hunk::default());
            }
            if let Some(hunk) = current_hunk.as_mut() {
                if let Some(stripped) = line.strip_prefix('+') {
                    hunk.lines.push(HunkLine {
                        prefix: '+',
                        content: stripped.to_string(),
                    });
                } else if let Some(stripped) = line.strip_prefix('-') {
                    hunk.lines.push(HunkLine {
                        prefix: '-',
                        content: stripped.to_string(),
                    });
                } else if let Some(stripped) = line.strip_prefix(' ') {
                    hunk.lines.push(HunkLine {
                        prefix: ' ',
                        content: stripped.to_string(),
                    });
                } else if line.starts_with('\\') {
                    // "\ No newline at end of file" marker — skip.
                } else {
                    // Treat as context line (implicit space prefix).
                    hunk.lines.push(HunkLine {
                        prefix: ' ',
                        content: line.to_string(),
                    });
                }
            }
        }

        i += 1;
    }

    flush(&mut current_op, &mut current_hunk);
    if let Some(op) = current_op.take() {
        operations.push(op);
    }

    (operations, None)
}

/// Tool that parses a V4A patch and returns structured operations as JSON.
pub struct PatchParseTool;

impl PatchParseTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PatchParseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PatchParseTool {
    fn name(&self) -> &str {
        "patch_parse"
    }

    fn description(&self) -> &str {
        "Parse a V4A-format patch (*** Begin Patch / *** Update File / *** Add \
         File / *** Delete File / *** Move File / *** End Patch) into a list of \
         structured file operations. Use before applying a patch to understand \
         what files it will add, update, delete, or move."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "The V4A-format patch text to parse"
                }
            },
            "required": ["patch"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let patch = args["patch"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "patch_parse".to_string(),
                message: "Missing required 'patch' parameter".to_string(),
            })?;

        let (operations, _error) = parse_v4a_patch(patch);
        serde_json::to_string(&operations).map_err(|e| ToolError::ExecutionFailed {
            tool: "patch_parse".to_string(),
            message: format!("Failed to serialise result: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_update_file() {
        let patch = "*** Begin Patch\n\
*** Update File: src/lib.rs\n\
@@ context @@\n\
 fn foo() {\n\
-    old\n\
+    new\n\
 }\n\
*** End Patch\n";
        let (ops, err) = parse_v4a_patch(patch);
        assert!(err.is_none());
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation, OperationType::Update);
        assert_eq!(ops[0].file_path, "src/lib.rs");
        assert_eq!(ops[0].hunks.len(), 1);
        let hunk = &ops[0].hunks[0];
        assert_eq!(hunk.context_hint.as_deref(), Some("context"));
        assert_eq!(hunk.lines.len(), 4);
        assert_eq!(hunk.lines[0].prefix, ' ');
        assert_eq!(hunk.lines[0].content, "fn foo() {");
        assert_eq!(hunk.lines[1].prefix, '-');
        assert_eq!(hunk.lines[1].content, "    old");
        assert_eq!(hunk.lines[2].prefix, '+');
        assert_eq!(hunk.lines[2].content, "    new");
        assert_eq!(hunk.lines[3].prefix, ' ');
        assert_eq!(hunk.lines[3].content, "}");
    }

    #[test]
    fn parse_add_file() {
        let patch = "*** Begin Patch\n\
*** Add File: src/new.rs\n\
+line one\n\
+line two\n\
*** End Patch\n";
        let (ops, err) = parse_v4a_patch(patch);
        assert!(err.is_none());
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation, OperationType::Add);
        assert_eq!(ops[0].file_path, "src/new.rs");
        assert_eq!(ops[0].hunks[0].lines.len(), 2);
    }

    #[test]
    fn parse_delete_and_move() {
        let patch = "*** Begin Patch\n\
*** Delete File: src/old.rs\n\
*** Move File: src/a.rs -> src/b.rs\n\
*** End Patch\n";
        let (ops, err) = parse_v4a_patch(patch);
        assert!(err.is_none());
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].operation, OperationType::Delete);
        assert_eq!(ops[0].file_path, "src/old.rs");
        assert_eq!(ops[1].operation, OperationType::Move);
        assert_eq!(ops[1].file_path, "src/a.rs");
        assert_eq!(ops[1].new_path.as_deref(), Some("src/b.rs"));
    }

    #[test]
    fn parse_without_markers_uses_whole_input() {
        let patch = "*** Update File: a.txt\n\
+hello\n";
        let (ops, err) = parse_v4a_patch(patch);
        assert!(err.is_none());
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].operation, OperationType::Update);
    }

    #[test]
    fn parse_empty_patch() {
        let (ops, err) = parse_v4a_patch("");
        assert!(err.is_none());
        assert!(ops.is_empty());
    }

    #[tokio::test]
    async fn tool_parse_roundtrip() {
        let tool = PatchParseTool::new();
        let out = tool
            .execute(serde_json::json!({
                "patch": "*** Begin Patch\n*** Add File: x.txt\n+hi\n*** End Patch\n"
            }))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 1);
        assert_eq!(v[0]["operation"], "add");
        assert_eq!(v[0]["file_path"], "x.txt");
    }

    #[tokio::test]
    async fn tool_missing_patch_is_error() {
        let tool = PatchParseTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
