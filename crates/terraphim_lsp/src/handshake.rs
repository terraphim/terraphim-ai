//! Reading the client's `initialize` request ahead of tower-lsp.
//!
//! lsp-types 0.94 (and 0.95) model the workspace diagnostics capability as
//! `workspace.diagnostic`, but LSP 3.17 and the clients that follow it (Zed)
//! send `workspace.diagnostics`, which the typed params silently drop. Without
//! this module a server could not tell whether a pulling client accepts
//! `workspace/diagnostic/refresh`.
//!
//! [`peek_initialize`] reads the first LSP frame from the transport, extracts
//! the hint from the raw JSON and returns every byte it consumed, so the
//! caller can replay them in front of the rest of the stream and tower-lsp
//! sees the unmodified conversation. The spec requires `initialize` to be the
//! first request; any other first frame passes through with no hint.

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt};

/// The longest header block read before giving up and passing the stream
/// through untouched.
const MAX_HEADER_BYTES: usize = 8 * 1024;

/// The longest `initialize` body read; a larger first frame is passed
/// through unparsed.
const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

/// What the first frame said about `workspace/diagnostic/refresh`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InitializeHint {
    /// `workspace.diagnostics.refreshSupport` (the spec key), else the legacy
    /// singular `workspace.diagnostic.refreshSupport`; the spec key wins when
    /// both are present. `None` when neither is a boolean, or the frame is
    /// malformed or not `initialize`.
    pub diagnostic_refresh: Option<bool>,
}

/// Read the first frame from `reader` and return its hint together with
/// every byte consumed (header, body and, on a short read, whatever
/// arrived), to be replayed in front of `reader`.
pub async fn peek_initialize<R: AsyncRead + Unpin>(reader: &mut R) -> (InitializeHint, Vec<u8>) {
    let mut buffered = Vec::new();
    let hint = read_first_frame(reader, &mut buffered)
        .await
        .map(|body| hint_from_body(&body))
        .unwrap_or_default();
    (hint, buffered)
}

async fn read_first_frame<R: AsyncRead + Unpin>(
    reader: &mut R,
    buffered: &mut Vec<u8>,
) -> Option<Vec<u8>> {
    // Header block: read byte by byte up to the blank line, so nothing past
    // the frame is consumed.
    while !buffered.ends_with(b"\r\n\r\n") {
        if buffered.len() >= MAX_HEADER_BYTES {
            return None;
        }
        buffered.push(reader.read_u8().await.ok()?);
    }
    let length = content_length(&buffered[..])?;
    if length > MAX_BODY_BYTES {
        return None;
    }
    let start = buffered.len();
    buffered.resize(start + length, 0);
    let mut filled = 0;
    while filled < length {
        match reader.read(&mut buffered[start + filled..]).await {
            Ok(0) | Err(_) => {
                buffered.truncate(start + filled);
                return None;
            }
            Ok(read) => filled += read,
        }
    }
    Some(buffered[start..].to_vec())
}

fn content_length(header: &[u8]) -> Option<usize> {
    std::str::from_utf8(header).ok()?.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.trim()
            .eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse().ok())?
    })
}

fn hint_from_body(body: &[u8]) -> InitializeHint {
    let Ok(message) = serde_json::from_slice::<Value>(body) else {
        return InitializeHint::default();
    };
    if message.get("method").and_then(Value::as_str) != Some("initialize") {
        return InitializeHint::default();
    }
    let workspace = &message["params"]["capabilities"]["workspace"];
    let flag = |key: &str| workspace[key]["refreshSupport"].as_bool();
    InitializeHint {
        diagnostic_refresh: flag("diagnostics").or_else(|| flag("diagnostic")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(body: &str) -> Vec<u8> {
        format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn initialize(workspace: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"capabilities":{{"workspace":{workspace}}}}}}}"#
        )
    }

    /// Peek `input` followed by `tail`; the replay must be the input as is.
    async fn peek(input: &[u8], tail: &[u8]) -> InitializeHint {
        let mut stream = [input, tail].concat();
        let mut reader = &stream[..];
        let (hint, buffered) = peek_initialize(&mut reader).await;
        assert_eq!(
            [buffered.as_slice(), reader].concat(),
            stream,
            "replaying the buffer then the rest restores the stream"
        );
        stream.clear();
        hint
    }

    #[tokio::test]
    async fn zed_shaped_plural_key_is_detected() {
        let body = initialize(r#"{"diagnostics":{"refreshSupport":true}}"#);
        assert_eq!(
            peek(&frame(&body), b"next").await.diagnostic_refresh,
            Some(true)
        );
    }

    #[tokio::test]
    async fn legacy_singular_key_is_detected() {
        let body = initialize(r#"{"diagnostic":{"refreshSupport":true}}"#);
        assert_eq!(
            peek(&frame(&body), b"").await.diagnostic_refresh,
            Some(true)
        );
    }

    #[tokio::test]
    async fn explicit_false_is_kept_and_the_spec_key_wins() {
        for (workspace, expected) in [
            (r#"{"diagnostics":{"refreshSupport":false}}"#, Some(false)),
            (
                r#"{"diagnostics":{"refreshSupport":false},"diagnostic":{"refreshSupport":true}}"#,
                Some(false),
            ),
            (
                r#"{"diagnostics":{"refreshSupport":true},"diagnostic":{"refreshSupport":false}}"#,
                Some(true),
            ),
            (
                r#"{"diagnostics":{},"diagnostic":{"refreshSupport":true}}"#,
                Some(true),
            ),
        ] {
            let body = initialize(workspace);
            assert_eq!(
                peek(&frame(&body), b"").await.diagnostic_refresh,
                expected,
                "{workspace}"
            );
        }
    }

    #[tokio::test]
    async fn absent_or_malformed_gives_no_hint() {
        for workspace in [
            "{}",
            r#"{"diagnostics":{}}"#,
            r#"{"diagnostics":{"refreshSupport":"yes"}}"#,
        ] {
            let body = initialize(workspace);
            assert_eq!(
                peek(&frame(&body), b"").await.diagnostic_refresh,
                None,
                "{workspace}"
            );
        }
        let no_workspace =
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#;
        assert_eq!(
            peek(&frame(no_workspace), b"").await.diagnostic_refresh,
            None
        );
    }

    #[tokio::test]
    async fn a_non_initialize_first_frame_passes_through_without_a_hint() {
        let body = r#"{"jsonrpc":"2.0","method":"workspace/diagnostics","params":{"capabilities":{"workspace":{"diagnostics":{"refreshSupport":true}}}}}"#;
        assert_eq!(peek(&frame(body), b"tail").await.diagnostic_refresh, None);
        assert_eq!(peek(&frame("not json"), b"").await.diagnostic_refresh, None);
    }

    #[tokio::test]
    async fn header_only_and_partial_reads_are_replayed() {
        let body = initialize(r#"{"diagnostics":{"refreshSupport":true}}"#);
        let whole = frame(&body);
        let header_end = whole.len() - body.len();
        for cut in [
            0,
            5,
            header_end - 1,
            header_end,
            header_end + 10,
            whole.len() - 1,
        ] {
            assert_eq!(
                peek(&whole[..cut], b"").await.diagnostic_refresh,
                None,
                "cut at {cut}"
            );
        }
    }

    #[tokio::test]
    async fn malformed_headers_pass_through() {
        for input in [
            &b"Content-Length: nope\r\n\r\n{}"[..],
            b"X-Other: 1\r\n\r\n{}",
        ] {
            assert_eq!(peek(input, b"").await.diagnostic_refresh, None);
        }
        let huge = format!("Content-Length: {}\r\n\r\n", MAX_BODY_BYTES + 1);
        assert_eq!(peek(huge.as_bytes(), b"xyz").await.diagnostic_refresh, None);
        let endless = vec![b'a'; MAX_HEADER_BYTES + 100];
        assert_eq!(peek(&endless, b"").await.diagnostic_refresh, None);
    }

    #[tokio::test]
    async fn content_type_header_and_case_are_tolerated() {
        let body = initialize(r#"{"diagnostics":{"refreshSupport":true}}"#);
        let input = format!(
            "content-type: application/vscode-jsonrpc; charset=utf-8\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
        assert_eq!(
            peek(input.as_bytes(), b"").await.diagnostic_refresh,
            Some(true)
        );
    }
}
