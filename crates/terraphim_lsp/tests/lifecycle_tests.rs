//! The `shutdown` / `exit` lifecycle of the real `terraphim-lsp` binary
//! (LSP 3.17): the server exits when it receives `exit`, with code 0 if
//! `shutdown` came first and 1 if not, without waiting for stdin to close.
//! Zed holds the pipe open until the process is gone, so a server that waits
//! for end of input lingers. No mocks.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn frame(message: &serde_json::Value) -> Vec<u8> {
    let body = message.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

/// A started server whose stdin the test holds open.
struct Session {
    child: Child,
    stdin: Option<ChildStdin>,
    messages: mpsc::Receiver<serde_json::Value>,
}

impl Session {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_terraphim-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn terraphim-lsp");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
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
                if sender.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    return;
                }
            }
        });
        let mut session = Self {
            child,
            stdin: Some(stdin),
            messages,
        };
        session.send(
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
            "params": {"processId": null, "rootUri": null, "capabilities": {}}}),
        );
        session.response(1);
        session.send(serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
        session
    }

    fn send(&mut self, message: serde_json::Value) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin.write_all(&frame(&message)).unwrap();
        stdin.flush().unwrap();
    }

    fn response(&mut self, id: i64) -> serde_json::Value {
        let end = Instant::now() + Duration::from_secs(30);
        while Instant::now() < end {
            if let Ok(message) = self.messages.recv_timeout(Duration::from_millis(50))
                && message["id"] == id
                && message.get("method").is_none()
            {
                return message;
            }
        }
        panic!("no response {id}");
    }

    /// Poll for the process to exit; its status, or `None` if it is still
    /// running after `limit`.
    fn exit_within(&mut self, limit: Duration) -> Option<ExitStatus> {
        let end = Instant::now() + limit;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return Some(status);
            }
            if Instant::now() >= end {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Close stdin and reap the process, so a failing test leaves nothing
    /// running.
    fn reap(mut self) {
        self.stdin = None;
        if self.exit_within(Duration::from_secs(10)).is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Generous for a loaded CI machine, but far below what a hang would take.
const PROMPTLY: Duration = Duration::from_secs(5);

#[test]
fn exit_after_shutdown_ends_the_process_with_code_0_while_stdin_is_open() {
    let mut session = Session::start();
    session.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}));
    assert_eq!(session.response(2)["result"], serde_json::Value::Null);
    session.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
    let status = session.exit_within(PROMPTLY);
    assert_eq!(status.map(|s| s.code()), Some(Some(0)), "exits 0 on exit");
    session.reap();
}

#[test]
fn exit_without_shutdown_ends_the_process_with_code_1() {
    let mut session = Session::start();
    session.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
    let status = session.exit_within(PROMPTLY);
    assert_eq!(status.map(|s| s.code()), Some(Some(1)), "exits 1 on exit");
    session.reap();
}

#[test]
fn exit_without_shutdown_then_end_of_input_still_gives_code_1() {
    // A realistic teardown: the client says `exit`, then drops the pipe. The
    // code must not depend on which of the two the server notices first.
    for _ in 0..5 {
        let mut session = Session::start();
        session.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
        session.stdin = None;
        let status = session.exit_within(PROMPTLY);
        assert_eq!(status.map(|s| s.code()), Some(Some(1)));
        session.reap();
    }
}

#[test]
fn shutdown_exit_then_end_of_input_gives_code_0() {
    for _ in 0..5 {
        let mut session = Session::start();
        session.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}));
        session.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
        session.stdin = None;
        let status = session.exit_within(PROMPTLY);
        assert_eq!(status.map(|s| s.code()), Some(Some(0)));
        session.reap();
    }
}

#[test]
fn the_shutdown_response_is_delivered_before_the_process_exits() {
    let mut session = Session::start();
    session.send(serde_json::json!({"jsonrpc": "2.0", "id": 7, "method": "shutdown"}));
    session.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
    assert_eq!(session.response(7)["result"], serde_json::Value::Null);
    let status = session.exit_within(PROMPTLY);
    assert_eq!(status.map(|s| s.code()), Some(Some(0)));
    session.reap();
}

#[test]
fn end_of_input_still_ends_the_process() {
    let mut session = Session::start();
    session.stdin = None; // the client vanished
    assert!(session.exit_within(PROMPTLY).is_some());
    session.reap();
}

#[test]
fn requests_before_exit_are_answered_normally() {
    let mut session = Session::start();
    session.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "textDocument/hover",
        "params": {"textDocument": {"uri": "file:///tmp/none.md"},
                   "position": {"line": 0, "character": 0}}}),
    );
    // An unopened document: a null result, not a hang or a crash.
    assert_eq!(session.response(3)["result"], serde_json::Value::Null);
    assert!(
        session.exit_within(Duration::from_millis(200)).is_none(),
        "still serving"
    );
    session.reap();
}
