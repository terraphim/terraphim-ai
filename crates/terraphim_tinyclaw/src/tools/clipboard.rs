//! System clipboard image extraction.
//!
//! Port of Hermes `hermes_cli/clipboard.py` (macOS + Linux subset). Checks
//! the system clipboard for image data and saves it to a PNG file. Uses only
//! OS-level CLI tools (osascript on macOS, wl-paste/xclip on Linux).

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::Path;

/// Check whether the clipboard currently contains an image.
pub async fn has_clipboard_image() -> bool {
    if cfg!(target_os = "macos") {
        return macos_has_image().await;
    }
    if cfg!(target_os = "linux") {
        return linux_has_image().await;
    }
    false
}

/// Extract an image from the clipboard and save it to `dest` as PNG.
/// Returns true if an image was found and saved.
pub async fn save_clipboard_image(dest: &Path) -> bool {
    if let Some(parent) = dest.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return false;
    }
    if cfg!(target_os = "macos") {
        return macos_save(dest).await;
    }
    if cfg!(target_os = "linux") {
        return linux_save(dest).await;
    }
    false
}

// ---------------------------------------------------------------------------
// macOS (osascript)
// ---------------------------------------------------------------------------

async fn macos_has_image() -> bool {
    let out = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg("clipboard info")
        .output()
        .await;
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.contains("«class PNGf»") || stdout.contains("«class TIFF»")
        }
        Err(_) => false,
    }
}

async fn macos_save(dest: &Path) -> bool {
    if !macos_has_image().await {
        return false;
    }
    let dest_str = dest.to_string_lossy();
    let script = format!(
        "try\n\
         \x20 set imgData to the clipboard as «class PNGf»\n\
         \x20 set f to open for access POSIX file \"{dest_str}\" with write permission\n\
         \x20 write imgData to f\n\
         \x20 close access f\n\
         on error\n\
         \x20 return \"fail\"\n\
         end try\n"
    );
    let out = tokio::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .await;
    match out {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            o.status.success()
                && !stdout.contains("fail")
                && dest.exists()
                && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Linux (wl-paste for Wayland, xclip for X11)
// ---------------------------------------------------------------------------

async fn linux_has_image() -> bool {
    // Prefer wl-paste if WAYLAND_DISPLAY is set, else xclip.
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return wayland_has_image().await;
    }
    xclip_has_image().await
}

async fn linux_save(dest: &Path) -> bool {
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return wayland_save(dest).await;
    }
    xclip_save(dest).await
}

async fn wayland_has_image() -> bool {
    let out = tokio::process::Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .any(|t| t.starts_with("image/")),
        _ => false,
    }
}

async fn wayland_save(dest: &Path) -> bool {
    // Determine MIME type, preferring PNG.
    let types = tokio::process::Command::new("wl-paste")
        .arg("--list-types")
        .output()
        .await
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut mime = None;
    for preferred in [
        "image/png",
        "image/jpeg",
        "image/bmp",
        "image/gif",
        "image/webp",
    ] {
        if types.lines().any(|t| t == preferred) {
            mime = Some(preferred);
            break;
        }
    }
    let Some(mime) = mime else {
        return false;
    };

    let out = tokio::process::Command::new("wl-paste")
        .arg("--type")
        .arg(mime)
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => {
            std::fs::write(dest, o.stdout).is_ok()
                && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
        }
        _ => false,
    }
}

async fn xclip_has_image() -> bool {
    let out = tokio::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "TARGETS", "-o"])
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).contains("image/png"),
        _ => false,
    }
}

async fn xclip_save(dest: &Path) -> bool {
    if !xclip_has_image().await {
        return false;
    }
    let out = tokio::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-t", "image/png", "-o"])
        .output()
        .await;
    match out {
        Ok(o) if o.status.success() => {
            std::fs::write(dest, o.stdout).is_ok()
                && dest.metadata().map(|m| m.len() > 0).unwrap_or(false)
        }
        _ => false,
    }
}

/// The `clipboard` tool. Checks or extracts a clipboard image.
pub struct ClipboardTool;

impl ClipboardTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClipboardTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ClipboardTool {
    fn name(&self) -> &str {
        "clipboard"
    }

    fn description(&self) -> &str {
        "Check whether the system clipboard contains an image, or save the \
         clipboard image to a PNG file. Actions: 'check' (has image?) and \
         'save' (write to a file path)."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["check", "save"]
                },
                "dest": {
                    "type": "string",
                    "description": "Output PNG path (required for 'save')"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "clipboard".to_string(),
                message: "Missing required 'action' parameter".to_string(),
            })?;

        let result = match action {
            "check" => serde_json::json!({
                "has_image": has_clipboard_image().await,
            }),
            "save" => {
                let dest = args["dest"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArguments {
                        tool: "clipboard".to_string(),
                        message: "'dest' is required for save".to_string(),
                    })?;
                let saved = save_clipboard_image(Path::new(dest)).await;
                serde_json::json!({"saved": saved, "dest": dest})
            }
            other => {
                return Err(ToolError::InvalidArguments {
                    tool: "clipboard".to_string(),
                    message: format!("Unknown clipboard action: {other}"),
                });
            }
        };

        serde_json::to_string(&result).map_err(|e| ToolError::ExecutionFailed {
            tool: "clipboard".to_string(),
            message: format!("Failed to serialise result: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tool_check_returns_boolean() {
        let tool = ClipboardTool::new();
        let out = tool
            .execute(serde_json::json!({"action": "check"}))
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["has_image"].is_boolean());
    }

    #[tokio::test]
    async fn tool_save_requires_dest() {
        let tool = ClipboardTool::new();
        let result = tool.execute(serde_json::json!({"action": "save"})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn tool_unknown_action_is_error() {
        let tool = ClipboardTool::new();
        let result = tool.execute(serde_json::json!({"action": "bogus"})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[test]
    fn schema_shape() {
        let tool = ClipboardTool::new();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let actions = schema["properties"]["action"]["enum"].as_array().unwrap();
        assert_eq!(actions.len(), 2);
    }
}
