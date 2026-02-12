//! Voice transcription tool using Whisper.
//!
//! This tool downloads voice messages, converts them to the format
//! required by Whisper, and transcribes them to text.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::PathBuf;

/// Tool for transcribing voice messages to text.
pub struct VoiceTranscribeTool {
    temp_dir: PathBuf,
    #[cfg(feature = "voice")]
    whisper: Option<WhisperContext>,
}

impl VoiceTranscribeTool {
    /// Create a new voice transcription tool.
    pub fn new() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("terraphim_tinyclaw"),
            #[cfg(feature = "voice")]
            whisper: None,
        }
    }

    /// Initialize Whisper model (downloads on first use).
    #[cfg(feature = "voice")]
    pub async fn init_whisper(&mut self) -> anyhow::Result<()> {
        // TODO: Initialize Whisper context with base model
        // This requires whisper-rs which is currently disabled
        log::info!("Whisper initialization would happen here with voice feature enabled");
        Ok(())
    }

    /// Download audio file from URL.
    async fn download_audio(&self, url: &str) -> Result<PathBuf, ToolError> {
        log::info!("Downloading audio from: {}", url);

        // Create temp directory if it doesn't exist
        tokio::fs::create_dir_all(&self.temp_dir)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to create temp directory: {}", e),
            })?;

        // Generate temp file path
        let file_name = format!("voice_{}.ogg", uuid::Uuid::new_v4());
        let file_path = self.temp_dir.join(&file_name);

        // Download file
        let response = reqwest::get(url)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to download audio: {}", e),
            })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to read audio bytes: {}", e),
            })?;

        tokio::fs::write(&file_path, bytes)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to write audio file: {}", e),
            })?;

        log::debug!("Downloaded audio to: {:?}", file_path);
        Ok(file_path)
    }

    /// Convert audio to WAV format (16kHz, mono) for Whisper.
    #[cfg(feature = "voice")]
    async fn convert_to_wav(&self, input: &PathBuf) -> Result<PathBuf, ToolError> {
        use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        // TODO: Implement audio conversion using symphonia
        // This is a placeholder that just returns the input
        log::info!("Audio conversion would happen here with voice feature enabled");
        Ok(input.clone())
    }

    /// Transcribe audio file using Whisper.
    #[cfg(feature = "voice")]
    async fn transcribe(&self, audio_path: &PathBuf) -> Result<String, ToolError> {
        // TODO: Implement Whisper transcription
        // This requires whisper-rs which is currently disabled
        Ok("[Voice transcription requires voice feature enabled]".to_string())
    }

    /// Placeholder transcription without voice feature.
    #[cfg(not(feature = "voice"))]
    async fn transcribe_placeholder(&self, _audio_path: &PathBuf) -> Result<String, ToolError> {
        Ok("[Voice transcription is a placeholder - enable voice feature for actual transcription]".to_string())
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
        "voice_transcribe"
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
                tool: "voice_transcribe".to_string(),
                message: "Missing 'audio_url' parameter".to_string(),
            })?;

        let _language = args["language"].as_str().unwrap_or("auto");

        // Validate URL
        if !audio_url.starts_with("http://") && !audio_url.starts_with("https://") {
            return Err(ToolError::InvalidArguments {
                tool: "voice_transcribe".to_string(),
                message: "audio_url must be an HTTP(S) URL".to_string(),
            });
        }

        log::info!("Transcribing voice from: {}", audio_url);

        // Download audio
        let audio_path = self.download_audio(audio_url).await?;

        // Transcribe
        #[cfg(feature = "voice")]
        {
            let wav_path = self.convert_to_wav(&audio_path).await?;
            let text = self.transcribe(&wav_path).await?;

            // Cleanup temp files
            let _ = tokio::fs::remove_file(&audio_path).await;
            let _ = tokio::fs::remove_file(&wav_path).await;

            Ok(text)
        }

        #[cfg(not(feature = "voice"))]
        {
            let text = self.transcribe_placeholder(&audio_path).await?;

            // Cleanup temp file
            let _ = tokio::fs::remove_file(&audio_path).await;

            Ok(text)
        }
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
        assert!(result.unwrap_err().to_string().contains("HTTP(S)"));
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
}
