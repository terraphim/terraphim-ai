//! Integration tests for the voice transcription tool pipeline.
//!
//! These tests exercise the public tool API (`ToolRegistry`) and verify:
//! - Download + transcribe pipeline behavior
//! - Temp-file cleanup after execution
//! - Error handling for invalid URLs
//! - Graceful fallback output when voice transcription is unavailable

use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;
use terraphim_tinyclaw::tools::{ToolCall, ToolError, create_default_registry};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::timeout;

/// Serialize tests that touch the shared temp directory used by VoiceTranscribeTool.
static TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn list_voice_temp_files(temp_dir: &Path) -> std::io::Result<HashSet<PathBuf>> {
    if !temp_dir.exists() {
        return Ok(HashSet::new());
    }

    let mut files = HashSet::new();
    for entry in std::fs::read_dir(temp_dir)? {
        let path = entry?.path();
        let is_voice_file = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("voice_") && name.ends_with(".ogg"))
            .unwrap_or(false);

        if is_voice_file {
            files.insert(path);
        }
    }

    Ok(files)
}

async fn start_local_audio_server(
    body: &'static [u8],
) -> std::io::Result<(String, JoinHandle<std::io::Result<()>>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let response_body = body.to_vec();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await?;

        // Consume the incoming request bytes before replying.
        let mut request_buf = [0_u8; 1024];
        let _ = socket.read(&mut request_buf).await?;

        let response_headers = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: audio/ogg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response_body.len()
        );
        socket.write_all(response_headers.as_bytes()).await?;
        socket.write_all(&response_body).await?;
        socket.shutdown().await?;
        Ok(())
    });

    Ok((format!("http://{addr}/voice.ogg"), handle))
}

fn is_local_bind_blocked(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::PermissionDenied || error.raw_os_error() == Some(1)
}

#[tokio::test]
async fn test_voice_download_transcribe_cleanup() {
    let _guard = TEST_LOCK.lock().await;
    let temp_dir = std::env::temp_dir().join("terraphim_tinyclaw");
    let before_files = list_voice_temp_files(&temp_dir).expect("failed to snapshot temp files");

    let (audio_url, server_handle) = match start_local_audio_server(b"OggS\x00tiny-fixture").await {
        Ok(value) => value,
        Err(error) if is_local_bind_blocked(&error) => {
            eprintln!("skipping test_voice_download_transcribe_cleanup: {error}");
            return;
        }
        Err(error) => panic!("failed to start local audio server: {error}"),
    };

    let registry = create_default_registry();
    let call = ToolCall {
        id: "voice_cleanup_1".to_string(),
        name: "voice_transcribe".to_string(),
        arguments: json!({
            "audio_url": audio_url
        }),
    };

    let result = registry
        .execute(&call)
        .await
        .expect("voice tool should complete with fallback output");
    assert_eq!(
        result,
        "[Voice transcription unavailable: binary built without 'voice' feature]"
    );

    let server_io_result = timeout(Duration::from_secs(2), server_handle)
        .await
        .expect("server should finish serving request in time")
        .expect("server task should not panic");
    server_io_result.expect("server IO should succeed");

    let after_files = list_voice_temp_files(&temp_dir).expect("failed to snapshot temp files");
    let leaked: Vec<_> = after_files.difference(&before_files).cloned().collect();
    assert!(
        leaked.is_empty(),
        "expected no leaked voice temp files, leaked: {leaked:?}"
    );
}

#[tokio::test]
async fn test_voice_invalid_url_error_path() {
    let _guard = TEST_LOCK.lock().await;
    let registry = create_default_registry();
    let call = ToolCall {
        id: "voice_invalid_url_1".to_string(),
        name: "voice_transcribe".to_string(),
        arguments: json!({
            "audio_url": "not-a-url"
        }),
    };

    let err = registry
        .execute(&call)
        .await
        .expect_err("invalid URL should return an error");

    match err {
        ToolError::InvalidArguments { tool, message } => {
            assert_eq!(tool, "voice_transcribe");
            assert!(
                message.contains("valid URL"),
                "unexpected message: {message}"
            );
        }
        other => panic!("expected InvalidArguments, got {other:?}"),
    }
}

#[tokio::test]
async fn test_voice_graceful_fallback_output() {
    let _guard = TEST_LOCK.lock().await;
    let (audio_url, server_handle) = match start_local_audio_server(b"OggS\x00fallback").await {
        Ok(value) => value,
        Err(error) if is_local_bind_blocked(&error) => {
            eprintln!("skipping test_voice_graceful_fallback_output: {error}");
            return;
        }
        Err(error) => panic!("failed to start local audio server: {error}"),
    };

    let registry = create_default_registry();
    let call = ToolCall {
        id: "voice_fallback_1".to_string(),
        name: "voice_transcribe".to_string(),
        arguments: json!({
            "audio_url": audio_url,
            "language": "auto"
        }),
    };

    let output = registry
        .execute(&call)
        .await
        .expect("voice tool should return a graceful fallback output");
    assert!(
        output.contains("Voice transcription unavailable"),
        "unexpected fallback output: {output}"
    );

    let server_io_result = timeout(Duration::from_secs(2), server_handle)
        .await
        .expect("server should finish serving request in time")
        .expect("server task should not panic");
    server_io_result.expect("server IO should succeed");
}
