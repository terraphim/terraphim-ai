//! Voice transcription tool using Whisper.
//!
//! This tool downloads voice messages, converts them to the format
//! required by Whisper, and transcribes them to text.

use crate::config::VoiceToolConfig;
use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use reqwest::Url;
#[cfg(feature = "voice")]
use serde_json::Value;
use std::path::{Path, PathBuf};
#[cfg(feature = "voice")]
use std::time::Duration;

const TOOL_NAME: &str = "voice_transcribe";
const FALLBACK_DISABLED_BY_CONFIG: &str =
    "[Voice transcription unavailable: tool is disabled by configuration]";
const FALLBACK_PROVIDER_NOT_CONFIGURED: &str =
    "[Voice transcription unavailable: provider is not configured]";
#[cfg(not(feature = "voice"))]
const FALLBACK_FEATURE_DISABLED: &str =
    "[Voice transcription unavailable: binary built without 'voice' feature]";
#[cfg(feature = "voice")]
const FALLBACK_EMPTY_TRANSCRIPT: &str =
    "[Voice transcription unavailable: backend returned no transcript text]";

/// Tool for transcribing voice messages to text.
pub struct VoiceTranscribeTool {
    config: VoiceToolConfig,
    temp_dir: PathBuf,
}

impl VoiceTranscribeTool {
    /// Create a new voice transcription tool.
    pub fn new() -> Self {
        Self::with_config(VoiceToolConfig::default())
    }

    /// Create a new voice transcription tool with explicit configuration.
    pub fn with_config(config: VoiceToolConfig) -> Self {
        let temp_dir = config
            .temp_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("terraphim_tinyclaw"));

        Self { config, temp_dir }
    }

    /// Initialize Whisper model (downloads on first use).
    pub async fn init_whisper(&mut self) -> anyhow::Result<()> {
        log::info!("Whisper initialization would happen here with voice feature enabled");
        Ok(())
    }

    /// Validate audio URL input.
    fn validate_audio_url(audio_url: &str) -> Result<Url, ToolError> {
        let url = Url::parse(audio_url).map_err(|e| ToolError::InvalidArguments {
            tool: TOOL_NAME.to_string(),
            message: format!("audio_url must be a valid URL: {}", e),
        })?;

        match url.scheme() {
            "http" | "https" => Ok(url),
            _ => Err(ToolError::InvalidArguments {
                tool: TOOL_NAME.to_string(),
                message: "audio_url must use HTTP(S)".to_string(),
            }),
        }
    }

    /// Download audio file from URL.
    async fn download_audio(&self, url: &Url) -> Result<PathBuf, ToolError> {
        log::info!("Downloading audio from: {}", url);

        // Create temp directory if it doesn't exist
        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to create temp directory: {}", e),
            })?;

        // Generate temp file path
        let extension = Path::new(url.path())
            .extension()
            .and_then(|ext| ext.to_str())
            .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()))
            .unwrap_or("bin");
        let file_name = format!("voice_{}.{}", uuid::Uuid::new_v4(), extension);
        let file_path = self.temp_dir.join(&file_name);

        // Download file
        let response = reqwest::get(url.clone())
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to download audio: {}", e),
            })?
            .error_for_status()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Audio download returned non-success status: {}", e),
            })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to read audio bytes: {}", e),
            })?;

        tokio::fs::write(&file_path, bytes)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to write audio file: {}", e),
            })?;

        log::debug!("Downloaded audio to: {:?}", file_path);
        Ok(file_path)
    }

    /// Convert audio to WAV format (16kHz, mono) for Whisper.
    async fn convert_to_wav(&self, input: &Path) -> Result<PathBuf, ToolError> {
        // Placeholder implementation returns the input path unchanged.
        log::info!("Audio conversion would happen here with voice feature enabled");
        Ok(input.to_path_buf())
    }

    async fn transcribe_audio(
        &self,
        audio_path: &Path,
        language: &str,
    ) -> Result<String, ToolError> {
        if !self.config.enabled {
            return Ok(FALLBACK_DISABLED_BY_CONFIG.to_string());
        }

        let provider = match self
            .config
            .provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(provider) => provider,
            None => return Ok(FALLBACK_PROVIDER_NOT_CONFIGURED.to_string()),
        };

        #[cfg(feature = "voice")]
        {
            self.transcribe_with_voice_feature(audio_path, language, provider)
                .await
        }

        #[cfg(not(feature = "voice"))]
        {
            let _ = (audio_path, language, provider);
            Ok(FALLBACK_FEATURE_DISABLED.to_string())
        }
    }

    #[cfg(feature = "voice")]
    async fn transcribe_with_voice_feature(
        &self,
        audio_path: &Path,
        language: &str,
        provider: &str,
    ) -> Result<String, ToolError> {
        match provider {
            "whisper" | "openai" | "openai_compatible" => self
                .transcribe_openai_compatible(audio_path, language)
                .await
                .or_else(|error| {
                    log::warn!("Voice transcription backend error: {}", error);
                    Ok(self.fallback_from_error(&error))
                }),
            other => Ok(format!(
                "[Voice transcription unavailable: provider '{other}' is not supported]"
            )),
        }
    }

    #[cfg(feature = "voice")]
    async fn transcribe_openai_compatible(
        &self,
        audio_path: &Path,
        language: &str,
    ) -> Result<String, ToolError> {
        let endpoint = self
            .config
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|base| format!("{}/audio/transcriptions", base.trim_end_matches('/')))
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: "voice.base_url is not configured".to_string(),
            })?;

        let model = self
            .config
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: "voice.model is not configured".to_string(),
            })?;

        let bytes = tokio::fs::read(audio_path)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to read downloaded audio for transcription: {}", e),
            })?;

        let file_name = audio_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("voice-audio.bin")
            .to_string();

        let mut form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(bytes).file_name(file_name),
            );

        let language = language.trim();
        if !language.is_empty() && language != "auto" {
            form = form.text("language", language.to_string());
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_seconds.max(1)))
            .build()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to initialize HTTP client: {}", e),
            })?;

        let mut request = client.post(&endpoint).multipart(form);
        if let Some(api_key) = self
            .config
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            request = request.bearer_auth(api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Transcription request failed: {}", e),
            })?;

        let response = response
            .error_for_status()
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Transcription API returned non-success status: {}", e),
            })?;

        let payload: Value = response
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: TOOL_NAME.to_string(),
                message: format!("Failed to parse transcription response JSON: {}", e),
            })?;

        let transcript = payload
            .get("text")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        Ok(transcript.unwrap_or_else(|| FALLBACK_EMPTY_TRANSCRIPT.to_string()))
    }

    #[cfg(feature = "voice")]
    fn fallback_from_error(&self, error: &ToolError) -> String {
        match error {
            ToolError::ExecutionFailed { message, .. } => {
                format!("[Voice transcription unavailable: {}]", message)
            }
            _ => format!("[Voice transcription unavailable: {}]", error),
        }
    }

    async fn cleanup_temp_files(&self, download_path: &Path, converted_path: Option<&Path>) {
        self.cleanup_file(download_path).await;

        if let Some(path) = converted_path {
            if path != download_path {
                self.cleanup_file(path).await;
            }
        }
    }

    async fn cleanup_file(&self, path: &Path) {
        if let Err(error) = tokio::fs::remove_file(path).await {
            if error.kind() != std::io::ErrorKind::NotFound {
                log::warn!(
                    "Failed to clean up temporary voice file {:?}: {}",
                    path,
                    error
                );
            }
        }
    }
}

impl Default for VoiceTranscribeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for VoiceTranscribeTool {
    fn name(&self) -> &str {
        TOOL_NAME
    }

    fn description(&self) -> &str {
        "Transcribe voice messages to text using Whisper. \
         Downloads audio from URL, converts format if needed, \
         and returns transcribed text."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "audio_url": {
                    "type": "string",
                    "description": "URL to download the audio file from"
                },
                "language": {
                    "type": "string",
                    "description": "Language code for transcription (e.g., 'en', 'es', 'auto' for auto-detect)",
                    "default": "auto"
                }
            },
            "required": ["audio_url"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let audio_url = args["audio_url"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments {
                tool: TOOL_NAME.to_string(),
                message: "Missing 'audio_url' parameter".to_string(),
            })?;

        let language = args["language"].as_str().unwrap_or("auto");
        let parsed_audio_url = Self::validate_audio_url(audio_url)?;

        log::info!("Transcribing voice from: {}", parsed_audio_url);

        // Download audio
        let download_path = self.download_audio(&parsed_audio_url).await?;
        let mut converted_path: Option<PathBuf> = None;

        let transcription = match self.convert_to_wav(&download_path).await {
            Ok(path) => {
                converted_path = Some(path.clone());
                self.transcribe_audio(&path, language).await
            }
            Err(error) => Err(error),
        };

        self.cleanup_temp_files(&download_path, converted_path.as_deref())
            .await;

        transcription
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_tool_name() {
        let tool = VoiceTranscribeTool::new();
        assert_eq!(tool.name(), "voice_transcribe");
    }

    #[test]
    fn test_voice_tool_schema() {
        let tool = VoiceTranscribeTool::new();
        let schema = tool.parameters_schema();

        assert!(schema["properties"]["audio_url"].is_object());
        assert!(schema["properties"]["language"].is_object());

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("audio_url")));
    }

    #[tokio::test]
    async fn test_voice_tool_rejects_invalid_url() {
        let tool = VoiceTranscribeTool::new();
        let args = serde_json::json!({
            "audio_url": "not-a-valid-url"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("valid URL"));
    }

    #[tokio::test]
    async fn test_voice_tool_missing_url() {
        let tool = VoiceTranscribeTool::new();
        let args = serde_json::json!({
            "language": "en"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_audio_url_rejects_non_http_scheme() {
        let result = VoiceTranscribeTool::validate_audio_url("ftp://example.com/file.ogg");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP(S)"));
    }

    #[test]
    fn test_validate_audio_url_accepts_https() {
        let result = VoiceTranscribeTool::validate_audio_url("https://example.com/audio.ogg");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_voice_tool_fallback_when_disabled_in_config() {
        let cfg = VoiceToolConfig {
            enabled: false,
            ..VoiceToolConfig::default()
        };
        let tool = VoiceTranscribeTool::with_config(cfg);
        let temp = tempfile::NamedTempFile::new().unwrap();

        let output = tool.transcribe_audio(temp.path(), "auto").await.unwrap();
        assert_eq!(output, FALLBACK_DISABLED_BY_CONFIG);
    }

    #[tokio::test]
    async fn test_voice_tool_fallback_when_provider_missing() {
        let cfg = VoiceToolConfig {
            provider: None,
            ..VoiceToolConfig::default()
        };
        let tool = VoiceTranscribeTool::with_config(cfg);
        let temp = tempfile::NamedTempFile::new().unwrap();

        let output = tool.transcribe_audio(temp.path(), "auto").await.unwrap();
        assert_eq!(output, FALLBACK_PROVIDER_NOT_CONFIGURED);
    }

    #[cfg(feature = "voice")]
    #[tokio::test]
    async fn test_voice_tool_fallback_for_unsupported_provider() {
        let mut cfg = VoiceToolConfig::default();
        cfg.provider = Some("unsupported".to_string());
        let tool = VoiceTranscribeTool::with_config(cfg);
        let temp = tempfile::NamedTempFile::new().unwrap();

        let output = tool.transcribe_audio(temp.path(), "auto").await.unwrap();
        assert!(output.contains("provider 'unsupported' is not supported"));
    }

    #[cfg(not(feature = "voice"))]
    #[tokio::test]
    async fn test_voice_tool_fallback_when_voice_feature_disabled() {
        let tool = VoiceTranscribeTool::new();
        let temp = tempfile::NamedTempFile::new().unwrap();

        let output = tool.transcribe_audio(temp.path(), "auto").await.unwrap();
        assert_eq!(output, FALLBACK_FEATURE_DISABLED);
    }
}
