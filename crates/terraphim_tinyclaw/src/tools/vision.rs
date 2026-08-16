//! Vision analysis (multimodal image understanding).
//!
//! Port of Hermes `tools/vision_tools.py`. Sends an OpenAI-compatible
//! multimodal chat-completion request for an image (URL or local path) plus a
//! question. Images are base64-encoded data URLs (matching Hermes behaviour).

use crate::config::VisionConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Maximum size (bytes) of a remotely-fetched image before we refuse it.
const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;

/// Fetch a remote image with a hard size cap to bound SSRF/memory exposure.
async fn fetch_image_bytes(
    http: &reqwest::Client,
    url: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut resp = http.get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("image fetch failed: HTTP {}", resp.status()));
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if buf.len() + chunk.len() > max_bytes {
            return Err(format!("image exceeds {} byte limit", max_bytes));
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Minimal standard base64 encoder (with padding).
pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_CHARS[((n >> 18) & 63) as usize] as char);
        out.push(BASE64_CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64_CHARS[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_CHARS[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Determine MIME type from a file extension (defaults to image/jpeg).
pub fn determine_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("bmp") => "image/bmp",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => "image/jpeg",
    }
}

/// Read an image file and return a base64 data URL.
pub fn image_to_base64_data_url(path: &Path) -> std::io::Result<String> {
    let data = std::fs::read(path)?;
    let mime = determine_mime_type(path);
    Ok(format!("data:{};base64,{}", mime, base64_encode(&data)))
}

/// HTTP client for vision analysis.
pub struct VisionClient {
    http: reqwest::Client,
    model: String,
    base_url: String,
    api_key: String,
}

impl VisionClient {
    pub fn from_config(cfg: &VisionConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self {
            http,
            model: cfg.model.clone(),
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
        }
    }

    /// Analyse an image (URL or local path) against a question.
    pub async fn analyze(&self, image_url: &str, question: &str) -> Result<String, String> {
        // Resolve image to a base64 data URL.
        let data_url = if image_url.starts_with("http://") || image_url.starts_with("https://") {
            let bytes = fetch_image_bytes(&self.http, image_url, MAX_IMAGE_BYTES).await?;
            let mime =
                determine_mime_type(Path::new(image_url.split('?').next().unwrap_or(image_url)));
            format!("data:{};base64,{}", mime, base64_encode(&bytes))
        } else {
            image_to_base64_data_url(Path::new(image_url))
                .map_err(|e| format!("failed to read image: {}", e))?
        };

        let prompt = format!(
            "Fully describe and explain everything about this image, then answer the \
             following question:\n\n{}",
            question
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": prompt},
                    {"type": "image_url", "image_url": {"url": data_url}}
                ]
            }],
            "max_tokens": 1000
        });

        let url = format!("{}/chat/completions", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("vision analyze failed: HTTP {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let analysis = data["choices"][0]["message"]["content"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "vision response missing content".to_string())?;
        Ok(analysis)
    }
}

/// The `vision_analyze` tool.
pub struct VisionTool {
    client: VisionClient,
}

impl VisionTool {
    pub fn from_config(cfg: &VisionConfig) -> Self {
        Self {
            client: VisionClient::from_config(cfg),
        }
    }
}

#[async_trait]
impl Tool for VisionTool {
    fn name(&self) -> &str {
        "vision_analyze"
    }
    fn description(&self) -> &str {
        "Analyze an image (URL or local file path) with a vision model. \
         Provides a full description and answers a specific question."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "image_url": { "type": "string", "description": "Image URL (http/https) or local file path" },
                "question": { "type": "string", "description": "Question about the image" }
            },
            "required": ["image_url", "question"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let image_url = args["image_url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "vision_analyze".to_string(),
                message: "image_url is required".to_string(),
            })?;
        let question = args["question"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "vision_analyze".to_string(),
                message: "question is required".to_string(),
            })?;
        let analysis = self
            .client
            .analyze(image_url, question)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "vision_analyze".to_string(),
                message: e,
            })?;
        Ok(serde_json::json!({"success": true, "analysis": analysis}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encodes_correctly() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn mime_type_from_extension() {
        assert_eq!(determine_mime_type(Path::new("a.png")), "image/png");
        assert_eq!(determine_mime_type(Path::new("a.jpg")), "image/jpeg");
        assert_eq!(determine_mime_type(Path::new("a.webp")), "image/webp");
        assert_eq!(determine_mime_type(Path::new("a.unknown")), "image/jpeg");
    }

    #[test]
    fn data_url_roundtrip() {
        let dir = std::env::temp_dir().join("tinyclaw-vision-test");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("x.png");
        std::fs::write(&png, b"\x89PNG\r\n\x1a\nfake").unwrap();
        let url = image_to_base64_data_url(&png).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn config_available_requires_key() {
        let cfg = VisionConfig {
            enabled: true,
            api_key: "k".into(),
            ..Default::default()
        };
        assert!(cfg.available());
        let cfg2 = VisionConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(!cfg2.available());
    }

    #[tokio::test]
    async fn fetch_rejects_oversized_image() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut req = [0u8; 2048];
            let _ = sock.read(&mut req).await;
            let body = vec![b'a'; 64 * 1024];
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = sock.write_all(header.as_bytes()).await;
            let _ = sock.write_all(&body).await;
        });

        let http = reqwest::Client::new();
        let url = format!("http://{}/big", addr);
        // Cap far below the 64 KB body forces the oversize error.
        let err = fetch_image_bytes(&http, &url, 1024).await.unwrap_err();
        assert!(err.contains("exceeds"), "unexpected error: {}", err);
        let _ = server.await;
    }
}
