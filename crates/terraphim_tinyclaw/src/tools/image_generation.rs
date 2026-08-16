//! Image generation (FAL.ai FLUX).
//!
//! Port of Hermes `tools/image_generation_tool.py`. Sends a text prompt to a
//! FAL.ai-compatible image endpoint and returns the generated image URL.

use crate::config::ImageGenConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::time::Duration;

const ASPECT_RATIO_MAP: [(&str, &str); 3] = [
    ("landscape", "landscape_16_9"),
    ("square", "square_hd"),
    ("portrait", "portrait_16_9"),
];

/// Map an aspect-ratio name to a FAL image-size token (defaults to landscape).
pub fn aspect_ratio_to_image_size(aspect_ratio: &str) -> &'static str {
    let key = aspect_ratio.trim().to_lowercase();
    ASPECT_RATIO_MAP
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .unwrap_or("landscape_16_9")
}

/// HTTP client for FAL.ai image generation.
pub struct ImageGenerateClient {
    http: reqwest::Client,
    model: String,
    base_url: String,
    api_key: String,
}

impl ImageGenerateClient {
    pub fn from_config(cfg: &ImageGenConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self {
            http,
            model: cfg.model.clone(),
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
        }
    }

    /// Generate an image and return its URL.
    pub async fn generate(&self, prompt: &str, aspect_ratio: &str) -> Result<String, String> {
        let prompt = prompt.trim();
        if prompt.is_empty() {
            return Err("prompt is required and must be a non-empty string".to_string());
        }

        let body = serde_json::json!({
            "prompt": prompt,
            "image_size": aspect_ratio_to_image_size(aspect_ratio),
            "num_inference_steps": 50,
            "guidance_scale": 4.5,
            "num_images": 1,
            "output_format": "png",
            "enable_safety_checker": false,
            "safety_tolerance": "5"
        });

        let url = format!("{}/{}", self.base_url, self.model);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Key {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("image generation failed: HTTP {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        let image_url = data["images"][0]["url"]
            .as_str()
            .or_else(|| data["image"]["url"].as_str())
            .ok_or_else(|| "image generation response missing image URL".to_string())?;
        Ok(image_url.to_string())
    }
}

/// The `image_generate` tool.
pub struct ImageGenerateTool {
    client: ImageGenerateClient,
}

impl ImageGenerateTool {
    pub fn from_config(cfg: &ImageGenConfig) -> Self {
        Self {
            client: ImageGenerateClient::from_config(cfg),
        }
    }
}

#[async_trait]
impl Tool for ImageGenerateTool {
    fn name(&self) -> &str {
        "image_generate"
    }
    fn description(&self) -> &str {
        "Generate images from text prompts (FAL.ai FLUX). Returns an image URL; \
         display it with markdown: ![description](URL)."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "Detailed text prompt describing the image" },
                "aspect_ratio": {
                    "type": "string",
                    "enum": ["landscape", "square", "portrait"],
                    "description": "Image aspect ratio"
                }
            },
            "required": ["prompt"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let prompt = args["prompt"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "image_generate".to_string(),
                message: "prompt is required".to_string(),
            })?;
        let aspect_ratio = args["aspect_ratio"].as_str().unwrap_or("landscape");
        let image_url = self
            .client
            .generate(prompt, aspect_ratio)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "image_generate".to_string(),
                message: e,
            })?;
        Ok(serde_json::json!({"success": true, "image": image_url}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_mapping() {
        assert_eq!(aspect_ratio_to_image_size("landscape"), "landscape_16_9");
        assert_eq!(aspect_ratio_to_image_size("square"), "square_hd");
        assert_eq!(aspect_ratio_to_image_size("portrait"), "portrait_16_9");
        assert_eq!(aspect_ratio_to_image_size("Landscape"), "landscape_16_9");
        assert_eq!(aspect_ratio_to_image_size("bogus"), "landscape_16_9");
    }

    #[tokio::test]
    async fn empty_prompt_errors() {
        let cfg = ImageGenConfig {
            enabled: true,
            api_key: "k".into(),
            ..Default::default()
        };
        let client = ImageGenerateClient::from_config(&cfg);
        let err = client.generate("   ", "landscape").await.unwrap_err();
        assert!(err.contains("non-empty"));
    }

    #[test]
    fn config_available_requires_key() {
        let cfg = ImageGenConfig {
            enabled: true,
            api_key: "k".into(),
            ..Default::default()
        };
        assert!(cfg.available());
        let cfg2 = ImageGenConfig {
            enabled: true,
            ..Default::default()
        };
        assert!(!cfg2.available());
    }
}
