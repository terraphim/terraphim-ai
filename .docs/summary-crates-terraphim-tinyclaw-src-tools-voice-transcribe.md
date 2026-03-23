# crates/terraphim_tinyclaw/src/tools/voice_transcribe.rs

## Purpose
Implements the `voice_transcribe` tool for `terraphim_tinyclaw`, allowing the agent loop to turn remote audio attachments into text using Whisper when the `voice` feature is enabled.

## Key Functionality
- Exposes a `VoiceTranscribeTool` that conforms to the shared `Tool` trait and registers under the name `voice_transcribe`.
- Downloads audio from an HTTP(S) URL into a temp workspace under the system temp directory.
- Detects or preserves supported source formats such as `ogg`, `mp3`, `wav`, `m4a`, and `webm`.
- Under the `voice` feature, decodes media with `symphonia`, mixes channels to mono, resamples to 16 kHz, and writes a float WAV file via `hound`.
- Lazily resolves and caches the Whisper model path with `OnceCell`, checking `WHISPER_MODEL_PATH`, a local filename, the user data directory, and `/usr/share/terraphim/`.
- Runs Whisper inference in blocking tasks via `whisper-rs`, concatenates segment text, and returns a no-speech fallback if transcription yields nothing.
- Cleans up temporary audio artifacts after execution.

## Important Details
- Without the `voice` feature, conversion is skipped and transcription returns a clear compile-time capability message instead of failing obscurely.
- URL validation is basic by design: HTTP(S) is required, while missing audio extensions only trigger a warning.
- The `language` argument is accepted in the JSON schema but is not yet wired into Whisper parameters; transcription still uses auto-detection.
- CPU-heavy decode and transcription work is isolated in `spawn_blocking`, which protects the async runtime from long synchronous work.
- Tests cover tool identity, schema shape, invalid or missing URLs, and the non-voice fallback path.

## Integration Points
- Registered by `create_default_registry()` in `crates/terraphim_tinyclaw/src/tools/mod.rs`.
- Invoked indirectly when `build_media_augmented_content()` in `crates/terraphim_tinyclaw/src/agent/agent_loop.rs` appends instructions telling the LLM to call `voice_transcribe` for audio URLs.
- Depends on feature-gated audio and Whisper crates, plus the shared `ToolError` contract for consistent error reporting.
