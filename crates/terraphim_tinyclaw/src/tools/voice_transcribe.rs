//! Voice transcription tool using Whisper.
//!
//! This tool downloads voice messages, converts them to the format
//! required by Whisper, and transcribes them to text.
//!
//! Note: Full functionality requires the `voice` feature to be enabled.

use crate::tools::{Tool, ToolError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
#[cfg(feature = "voice")]
use tokio::sync::OnceCell;

/// Tool for transcribing voice messages to text.
pub struct VoiceTranscribeTool {
    temp_dir: PathBuf,
    #[cfg(feature = "voice")]
    model_path: OnceCell<PathBuf>,
}

impl VoiceTranscribeTool {
    /// Create a new voice transcription tool.
    pub fn new() -> Self {
        Self {
            temp_dir: std::env::temp_dir().join("terraphim_tinyclaw"),
            #[cfg(feature = "voice")]
            model_path: OnceCell::new(),
        }
    }

    /// Find and cache the Whisper model path (lazy, called on first use).
    #[cfg(feature = "voice")]
    async fn ensure_model(&self) -> Result<&PathBuf, ToolError> {
        self.model_path
            .get_or_try_init(|| async {
                let model_paths = [
                    std::env::var("WHISPER_MODEL_PATH").ok(),
                    Some("ggml-base.bin".to_string()),
                    dirs::data_local_dir().map(|d| {
                        d.join("terraphim/ggml-base.bin")
                            .to_string_lossy()
                            .into_owned()
                    }),
                    Some("/usr/share/terraphim/ggml-base.bin".to_string()),
                ];

                for path in model_paths.iter().flatten() {
                    let path = Path::new(path);
                    if path.exists() {
                        log::info!("Found Whisper model at: {:?}", path);
                        return Ok(path.to_path_buf());
                    }
                }

                Err(ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: "Whisper model not found. Download from \
                        https://huggingface.co/ggerganov/whisper.cpp and set \
                        WHISPER_MODEL_PATH or place at \
                        ~/.local/share/terraphim/ggml-base.bin"
                        .to_string(),
                })
            })
            .await
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

        // Generate temp file path with appropriate extension
        let ext = url
            .split('.')
            .next_back()
            .and_then(|e| e.split('?').next()) // Remove query params
            .filter(|e| ["ogg", "mp3", "wav", "m4a", "webm"].contains(&e.to_lowercase().as_str()))
            .unwrap_or("ogg");

        let file_name = format!("voice_{}.{}", uuid::Uuid::new_v4(), ext);
        let file_path = self.temp_dir.join(&file_name);

        // Download file
        let response = reqwest::get(url)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to download audio: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("HTTP error {} downloading audio", response.status()),
            });
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to read audio bytes: {}", e),
            })?;

        if bytes.is_empty() {
            return Err(ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: "Downloaded audio file is empty".to_string(),
            });
        }

        let bytes_len = bytes.len();
        tokio::fs::write(&file_path, bytes)
            .await
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to write audio file: {}", e),
            })?;

        log::debug!("Downloaded {} bytes to: {:?}", bytes_len, file_path);
        Ok(file_path)
    }

    /// Convert audio to WAV format (16kHz, mono) for Whisper.
    #[cfg(feature = "voice")]
    async fn convert_to_wav(&self, input: &Path) -> Result<PathBuf, ToolError> {
        use symphonia::core::audio::{AudioBufferRef, SampleBuffer, Signal};
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let output_path = self
            .temp_dir
            .join(format!("voice_{}.wav", uuid::Uuid::new_v4()));

        // Run conversion in blocking task
        let input_path = input.to_path_buf();
        let output = output_path.clone();

        tokio::task::spawn_blocking(move || {
            // Open the media source
            let file =
                std::fs::File::open(&input_path).map_err(|e| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Failed to open audio file: {}", e),
                })?;

            let mss = MediaSourceStream::new(Box::new(file), Default::default());

            // Create a probe hint using the file extension
            let mut hint = Hint::new();
            if let Some(ext) = input_path.extension().and_then(|e| e.to_str()) {
                hint.with_extension(ext);
            }

            // Probe the format
            let format_opts: FormatOptions = Default::default();
            let meta_opts: MetadataOptions = Default::default();
            let probed = symphonia::default::get_probe()
                .format(&hint, mss, &format_opts, &meta_opts)
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Failed to probe audio format: {}", e),
                })?;

            let mut format = probed.format;

            // Find the first audio track
            let track = format
                .tracks()
                .iter()
                .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
                .ok_or_else(|| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: "No audio track found in file".to_string(),
                })?;

            let track_id = track.id;
            let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);
            let channels = track.codec_params.channels.map(|c| c.count()).unwrap_or(2);

            log::debug!(
                "Audio: {} Hz, {} channels, sample rate",
                sample_rate,
                channels
            );

            // Create decoder
            let decoder_opts: DecoderOptions = Default::default();
            let mut decoder = symphonia::default::get_codecs()
                .make(&track.codec_params, &decoder_opts)
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Failed to create decoder: {}", e),
                })?;

            // Decode and collect samples
            let mut samples: Vec<f32> = Vec::new();

            loop {
                let packet = match format.next_packet() {
                    Ok(packet) => packet,
                    Err(symphonia::core::errors::Error::ResetRequired) => continue,
                    Err(_) => break,
                };

                if packet.track_id() != track_id {
                    continue;
                }

                match decoder.decode(&packet) {
                    Ok(decoded) => {
                        // Convert to mono f32 samples
                        match decoded {
                            AudioBufferRef::F32(buf) => {
                                let num_frames = buf.frames();
                                let num_channels = buf.spec().channels.count();
                                for frame_idx in 0..num_frames {
                                    // Average channels for mono
                                    let sum: f32 =
                                        (0..num_channels).map(|ch| buf.chan(ch)[frame_idx]).sum();
                                    samples.push(sum / num_channels as f32);
                                }
                            }
                            AudioBufferRef::S16(buf) => {
                                let num_frames = buf.frames();
                                let num_channels = buf.spec().channels.count();
                                for frame_idx in 0..num_frames {
                                    let sum: i16 =
                                        (0..num_channels).map(|ch| buf.chan(ch)[frame_idx]).sum();
                                    samples
                                        .push(sum as f32 / i16::MAX as f32 / num_channels as f32);
                                }
                            }
                            _ => {
                                // Convert other formats
                                let mut sample_buf: SampleBuffer<f32> =
                                    SampleBuffer::new(decoded.capacity() as u64, *decoded.spec());
                                sample_buf.copy_interleaved_ref(decoded);
                                for chunk in sample_buf.samples().chunks(channels) {
                                    let sum: f32 = chunk.iter().sum();
                                    samples.push(sum / channels as f32);
                                }
                            }
                        }
                    }
                    Err(_) => continue,
                }
            }

            // Resample to 16kHz if needed using simple linear interpolation
            let target_rate = 16000;
            let resampled = if sample_rate != target_rate {
                log::debug!("Resampling from {} Hz to {} Hz", sample_rate, target_rate);
                let ratio = target_rate as f64 / sample_rate as f64;
                let new_len = (samples.len() as f64 * ratio) as usize;
                let mut resampled = Vec::with_capacity(new_len);
                for i in 0..new_len {
                    let src_idx = i as f64 / ratio;
                    let idx_low = src_idx.floor() as usize;
                    let idx_high = (idx_low + 1).min(samples.len() - 1);
                    let frac = src_idx - idx_low as f64;
                    let val =
                        samples[idx_low] as f64 * (1.0 - frac) + samples[idx_high] as f64 * frac;
                    resampled.push(val as f32);
                }
                resampled
            } else {
                samples
            };

            // Write WAV file
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: target_rate,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            };

            let mut writer = hound::WavWriter::create(&output, spec).map_err(|e| {
                ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Failed to create WAV writer: {}", e),
                }
            })?;

            for sample in resampled {
                writer.write_sample(sample).ok();
            }

            writer.finalize().map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to finalize WAV file: {}", e),
            })?;

            Ok::<_, ToolError>(output)
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "voice_transcribe".to_string(),
            message: format!("Audio conversion task panicked: {}", e),
        })?
    }

    #[cfg(not(feature = "voice"))]
    async fn convert_to_wav(&self, input: &Path) -> Result<PathBuf, ToolError> {
        // Without voice feature, just return the input and let transcribe handle the fallback
        log::debug!("Voice feature not enabled, skipping audio conversion");
        Ok(input.to_path_buf())
    }

    /// Transcribe audio file using Whisper.
    #[cfg(feature = "voice")]
    async fn transcribe(&self, audio_path: &Path) -> Result<String, ToolError> {
        use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

        let model_path = self.ensure_model().await?;

        // Read WAV file
        let wav_path = audio_path.to_path_buf();
        let model_path_clone = model_path.clone();
        let (samples, sample_rate) = tokio::task::spawn_blocking(move || {
            let mut reader =
                hound::WavReader::open(&wav_path).map_err(|e| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Failed to open WAV file: {}", e),
                })?;

            let spec = reader.spec();
            let samples: Vec<f32> = reader.samples::<f32>().filter_map(|s| s.ok()).collect();

            Ok::<_, ToolError>((samples, spec.sample_rate))
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "voice_transcribe".to_string(),
            message: format!("WAV reading task panicked: {}", e),
        })??;

        if sample_rate != 16000 {
            return Err(ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Expected 16kHz audio, got {} Hz", sample_rate),
            });
        }

        // Run transcription in blocking task
        let text = tokio::task::spawn_blocking(move || {
            // Load Whisper context
            let ctx = WhisperContext::new_with_params(
                model_path_clone.to_str().unwrap(),
                whisper_rs::WhisperContextParameters::default(),
            )
            .map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to load Whisper model: {}", e),
            })?;

            // Create state for transcription
            let mut state = ctx.create_state().map_err(|e| ToolError::ExecutionFailed {
                tool: "voice_transcribe".to_string(),
                message: format!("Failed to create Whisper state: {}", e),
            })?;

            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_n_threads(4);
            params.set_translate(false);
            params.set_language(Some("auto"));

            state
                .full(params, &samples)
                .map_err(|e| ToolError::ExecutionFailed {
                    tool: "voice_transcribe".to_string(),
                    message: format!("Whisper transcription failed: {}", e),
                })?;

            let num_segments = state.full_n_segments();

            let mut text = String::new();
            for i in 0..num_segments {
                if let Some(segment) = state.get_segment(i) {
                    if let Ok(seg_text) = segment.to_str_lossy() {
                        text.push_str(&seg_text);
                        text.push(' ');
                    }
                }
            }

            Ok::<_, ToolError>(text.trim().to_string())
        })
        .await
        .map_err(|e| ToolError::ExecutionFailed {
            tool: "voice_transcribe".to_string(),
            message: format!("Transcription task panicked: {}", e),
        })??;

        if text.is_empty() {
            return Ok("[No speech detected in audio]".to_string());
        }

        Ok(text)
    }

    #[cfg(not(feature = "voice"))]
    async fn transcribe(&self, _audio_path: &Path) -> Result<String, ToolError> {
        Ok(
            "[Voice transcription requires voice feature enabled at compile time. \
             Rebuild with: cargo build --features voice]"
                .to_string(),
        )
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
         and returns transcribed text. \
         Requires 'voice' feature for actual transcription."
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

        // Additional URL validation - basic check for file extension
        let lower_url = audio_url.to_lowercase();
        let valid_extensions = [".ogg", ".mp3", ".wav", ".m4a", ".oga", ".webm"];
        let has_valid_ext = valid_extensions.iter().any(|ext| lower_url.contains(ext));

        if !has_valid_ext && !lower_url.contains("?") {
            log::warn!("Audio URL doesn't have a recognized audio file extension");
        }

        log::info!("Transcribing voice from: {}", audio_url);

        // Download audio
        let audio_path = self.download_audio(audio_url).await?;

        // Convert to WAV (16kHz mono) - required by Whisper
        let wav_path = self.convert_to_wav(&audio_path).await?;

        // Transcribe
        let text = self.transcribe(&wav_path).await?;

        // Cleanup temp files (ignore errors)
        let _ = tokio::fs::remove_file(&audio_path).await;
        if wav_path != audio_path {
            let _ = tokio::fs::remove_file(&wav_path).await;
        }

        Ok(text)
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

    #[tokio::test]
    async fn test_voice_feature_disabled_message() {
        let tool = VoiceTranscribeTool::new();
        // This test verifies the graceful fallback message when voice feature is not enabled
        let result = tool.transcribe(Path::new("/dev/null")).await;
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("voice feature"));
    }
}
