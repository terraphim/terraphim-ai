//! Text-to-speech.
//!
//! Port of Hermes `tools/tts_tool.py`. Three providers:
//! - `edge` (default): shells out to the `edge-tts` CLI (no API key).
//! - `openai`: OpenAI-compatible `/v1/audio/speech`.
//! - `elevenlabs`: ElevenLabs `/v1/text-to-speech/{voice_id}`.
//!
//! All write audio to an output file and return its path.

use crate::config::TtsConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::Path;
use std::time::Duration;

/// Generate a default output path under `output_dir`.
pub fn default_output_path(output_dir: &str, extension: &str) -> String {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%S");
    format!("{}/{}.{}", output_dir.trim_end_matches('/'), ts, extension)
}

/// HTTP + CLI client for TTS synthesis.
pub struct TtsClient {
    http: reqwest::Client,
    provider: String,
    voice: String,
    base_url: String,
    api_key: String,
    output_dir: String,
}

impl TtsClient {
    pub fn from_config(cfg: &TtsConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        Self {
            http,
            provider: cfg.provider.trim().to_lowercase(),
            voice: cfg.voice.clone(),
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key.clone(),
            output_dir: cfg.output_dir.clone(),
        }
    }

    /// Synthesize `text` into an audio file, returning the file path.
    pub async fn synthesize(
        &self,
        text: &str,
        output_path: Option<&str>,
        voice: Option<&str>,
    ) -> Result<String, String> {
        let text = text.trim();
        if text.is_empty() {
            return Err("text is required and must be non-empty".to_string());
        }
        let voice = voice.unwrap_or(&self.voice);

        // Determine output path + extension per provider.
        let (path, ext) = match output_path {
            Some(p) => {
                let ext = p.rsplit('.').next().unwrap_or("mp3").to_string();
                (p.to_string(), ext)
            }
            None => match self.provider.as_str() {
                "openai" | "elevenlabs" => {
                    (default_output_path(&self.output_dir, "mp3"), "mp3".into())
                }
                _ => (default_output_path(&self.output_dir, "mp3"), "mp3".into()),
            },
        };

        match self.provider.as_str() {
            "edge" => self.synthesize_edge(text, voice, &path).await?,
            "openai" => self.synthesize_openai(text, voice, &path).await?,
            "elevenlabs" => self.synthesize_elevenlabs(text, voice, &path).await?,
            other => {
                return Err(format!("unknown TTS provider: {}", other));
            }
        }

        if !Path::new(&path).exists() {
            return Err(format!(
                "TTS generation produced no output (provider: {})",
                self.provider
            ));
        }
        let _ = ext;
        Ok(path)
    }

    async fn synthesize_edge(&self, text: &str, voice: &str, path: &str) -> Result<(), String> {
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let out = tokio::process::Command::new("edge-tts")
            .arg("--voice")
            .arg(if voice.is_empty() {
                "en-US-JennyNeural"
            } else {
                voice
            })
            .arg("--text")
            .arg(text)
            .arg("--write-media")
            .arg(path)
            .output()
            .await
            .map_err(|e| {
                format!(
                    "edge-tts not available (install `pip install edge-tts`): {}",
                    e
                )
            })?;
        if !out.status.success() {
            return Err(format!(
                "edge-tts failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(())
    }

    async fn synthesize_openai(&self, text: &str, voice: &str, path: &str) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("OpenAI TTS selected but api_key not set".to_string());
        }
        let body = serde_json::json!({
            "model": "tts-1",
            "voice": if voice.is_empty() { "alloy" } else { voice },
            "input": text
        });
        let url = format!("{}/audio/speech", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("OpenAI TTS failed: HTTP {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    async fn synthesize_elevenlabs(
        &self,
        text: &str,
        voice: &str,
        path: &str,
    ) -> Result<(), String> {
        if self.api_key.is_empty() {
            return Err("ElevenLabs TTS selected but api_key not set".to_string());
        }
        let voice_id = if voice.is_empty() {
            "21m00Tcm4TlvDq8ikWAM"
        } else {
            voice
        };
        let url = format!("{}/v1/text-to-speech/{}", self.base_url, voice_id);
        let resp = self
            .http
            .post(&url)
            .header("xi-api-key", &self.api_key)
            .json(&serde_json::json!({
                "text": text,
                "model_id": "eleven_multilingual_v2"
            }))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("ElevenLabs TTS failed: HTTP {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }
}

/// The `text_to_speech` tool.
pub struct TextToSpeechTool {
    client: TtsClient,
}

impl TextToSpeechTool {
    pub fn from_config(cfg: &TtsConfig) -> Self {
        Self {
            client: TtsClient::from_config(cfg),
        }
    }
}

#[async_trait]
impl Tool for TextToSpeechTool {
    fn name(&self) -> &str {
        "text_to_speech"
    }
    fn description(&self) -> &str {
        "Convert text to speech audio. The model sends text; voice and provider \
         are configured by the user. Returns the saved audio file path."
    }
    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "text": { "type": "string", "description": "Text to speak" },
                "voice": { "type": "string", "description": "Voice name/id override" },
                "output_path": { "type": "string", "description": "Custom output file path" }
            },
            "required": ["text"]
        })
    }
    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let text = args["text"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: "text_to_speech".to_string(),
                message: "text is required".to_string(),
            })?;
        let output_path = args["output_path"].as_str();
        let voice = args["voice"].as_str();
        let path = self
            .client
            .synthesize(text, output_path, voice)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "text_to_speech".to_string(),
                message: e,
            })?;
        Ok(serde_json::json!({"success": true, "file": path}).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_path_in_output_dir() {
        let p = default_output_path("voice-memos", "mp3");
        assert!(p.starts_with("voice-memos/"));
        assert!(p.ends_with(".mp3"));
    }

    #[test]
    fn config_available_edge_no_key() {
        let cfg = TtsConfig {
            enabled: true,
            provider: "edge".into(),
            ..Default::default()
        };
        assert!(cfg.available());
    }

    #[test]
    fn config_available_openai_requires_key() {
        let cfg = TtsConfig {
            enabled: true,
            provider: "openai".into(),
            ..Default::default()
        };
        assert!(!cfg.available());
        let cfg2 = TtsConfig {
            enabled: true,
            provider: "openai".into(),
            api_key: "k".into(),
            ..Default::default()
        };
        assert!(cfg2.available());
    }

    #[tokio::test]
    async fn unknown_provider_errors() {
        let cfg = TtsConfig {
            enabled: true,
            provider: "bogus".into(),
            api_key: "k".into(),
            ..Default::default()
        };
        let client = TtsClient::from_config(&cfg);
        let err = client.synthesize("hello", None, None).await.unwrap_err();
        assert!(err.contains("unknown TTS provider"));
    }
}
