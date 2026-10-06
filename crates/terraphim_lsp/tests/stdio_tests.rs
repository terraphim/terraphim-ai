//! The real `terraphim-lsp` binary over stdio: the raw `initialize` is read
//! ahead of tower-lsp (lsp-types drops the spec key
//! `workspace.diagnostics`), replayed intact, and decides whether a pulling
//! client gets `workspace/diagnostic/refresh`. No mocks.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

const LAB_DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/lab_doc.md");

fn frame(message: &serde_json::Value) -> Vec<u8> {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

/// Drive one session and return every server message, as JSON text, in
/// order. Ends the session once the trim command has been answered.
fn session(workspace: serde_json::Value) -> Vec<String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_terraphim-lsp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn terraphim-lsp");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; length];
            if reader.read_exact(&mut body).is_err() {
                return;
            }
            if sender.send(String::from_utf8(body).unwrap()).is_err() {
                return;
            }
        }
    });

    let uri = "file:///tmp/stdio.md";
    let messages = [
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "processId": null, "rootUri": null,
        "capabilities": {
            "textDocument": {"diagnostic": {"dynamicRegistration": false}},
            "workspace": workspace,
        }}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
            "textDocument": {"uri": uri, "languageId": "markdown", "version": 1, "text": LAB_DOC}}}),
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "workspace/executeCommand",
            "params": {"command": "terraphim.trim.preview",
                       "arguments": [{"uri": uri, "level": "tighten"}]}}),
    ];
    for message in &messages {
        stdin.write_all(&frame(message)).unwrap();
    }
    stdin.flush().unwrap();

    let mut seen = Vec::new();
    while let Ok(message) = receiver.recv_timeout(Duration::from_secs(30)) {
        let answered = message.contains(r#""id":2"#) && message.contains("candidates");
        seen.push(message);
        if answered {
            break;
        }
    }
    // Let anything already queued behind the answer arrive, then end.
    stdin
        .write_all(&frame(
            &serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown"}),
        ))
        .unwrap();
    stdin
        .write_all(&frame(
            &serde_json::json!({"jsonrpc": "2.0", "method": "exit"}),
        ))
        .unwrap();
    drop(stdin);
    while let Ok(message) = receiver.recv_timeout(Duration::from_secs(30)) {
        seen.push(message);
    }
    let _ = child.wait();
    seen
}

fn refreshes(seen: &[String]) -> usize {
    seen.iter()
        .filter(|m| m.contains("workspace/diagnostic/refresh"))
        .count()
}

#[test]
fn spec_key_in_the_raw_initialize_enables_refresh_and_the_stream_is_intact() {
    let seen = session(serde_json::json!({"diagnostics": {"refreshSupport": true}}));
    assert!(
        seen.iter()
            .any(|m| m.contains(r#""id":1"#) && m.contains("capabilities")),
        "initialize answered, so the replayed frame reached tower-lsp: {seen:?}"
    );
    assert!(
        seen.iter()
            .any(|m| m.contains(r#""id":2"#) && m.contains("candidates")),
        "the trim command ran: {seen:?}"
    );
    assert_eq!(refreshes(&seen), 1, "{seen:?}");
}

#[test]
fn no_advertised_refresh_means_none_is_sent() {
    for workspace in [
        serde_json::json!({}),
        serde_json::json!({"diagnostics": {"refreshSupport": false}}),
    ] {
        let seen = session(workspace);
        assert!(seen.iter().any(|m| m.contains(r#""id":2"#)), "{seen:?}");
        assert_eq!(refreshes(&seen), 0, "{seen:?}");
    }
}
