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

// ------------------------------------------------- trim review over stdio --

const REVIEW_DOC: &str = include_str!("../../terraphim_lsp_core/tests/fixtures/trim_review_doc.md");

/// A live stdio session with the real binary, answered message by message.
struct Wire {
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    messages: mpsc::Receiver<serde_json::Value>,
}

impl Wire {
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
                let value = serde_json::from_slice(&body).unwrap();
                if sender.send(value).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin,
            messages,
        }
    }

    fn send(&mut self, message: serde_json::Value) {
        self.stdin.write_all(&frame(&message)).unwrap();
        self.stdin.flush().unwrap();
    }

    /// The next message matching `wanted`; refresh requests on the way are
    /// answered, as Zed does.
    fn until(
        &mut self,
        what: &str,
        wanted: impl Fn(&serde_json::Value) -> bool,
    ) -> serde_json::Value {
        loop {
            let message = self
                .messages
                .recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| panic!("no {what}"));
            if wanted(&message) {
                return message;
            }
            if message["method"] == "workspace/diagnostic/refresh" {
                let id = message["id"].clone();
                self.send(serde_json::json!({"jsonrpc": "2.0", "id": id, "result": null}));
            }
        }
    }

    fn response(&mut self, id: i64) -> serde_json::Value {
        self.until(&format!("response {id}"), |m| {
            m["id"] == id && m.get("method").is_none()
        })["result"]
            .clone()
    }

    fn request(&mut self, method: &str) -> serde_json::Value {
        self.until(method, |m| m["method"] == method && m.get("id").is_some())
    }

    fn finish(mut self) {
        self.send(serde_json::json!({"jsonrpc": "2.0", "id": 99, "method": "shutdown"}));
        self.send(serde_json::json!({"jsonrpc": "2.0", "method": "exit"}));
        let Self {
            mut child, stdin, ..
        } = self;
        drop(stdin);
        let _ = child.wait();
    }
}

/// Apply LSP edits (UTF-16 positions) to `text`, last first.
fn apply_lsp_edits(text: &str, edits: &[serde_json::Value]) -> String {
    use terraphim_lsp::core::{LineIndex, LinePosition};
    let index = LineIndex::new(text);
    let byte = |p: &serde_json::Value| {
        index.byte_offset(LinePosition {
            line: p["line"].as_u64().unwrap() as u32,
            character: p["character"].as_u64().unwrap() as u32,
        })
    };
    let mut spans: Vec<(usize, usize, &str)> = edits
        .iter()
        .map(|edit| {
            (
                byte(&edit["range"]["start"]),
                byte(&edit["range"]["end"]),
                edit["newText"].as_str().unwrap(),
            )
        })
        .collect();
    spans.sort_by_key(|&(start, _, _)| std::cmp::Reverse(start));
    let mut out = text.to_string();
    for (start, end, new_text) in spans {
        out.replace_range(start..end, new_text);
    }
    out
}

#[test]
fn zed_shaped_client_previews_answers_the_card_and_makes_the_cuts() {
    let uri = "file:///tmp/review.md";
    let mut wire = Wire::start();
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "processId": null, "rootUri": null,
        "capabilities": {
            "textDocument": {"diagnostic": {"dynamicRegistration": false}},
            "workspace": {
                "applyEdit": true,
                "workspaceEdit": {"documentChanges": true},
                "diagnostics": {"refreshSupport": true},
            },
            "window": {
                "showMessage": {"messageActionItem": {"additionalPropertiesSupport": false}},
                "showDocument": {"support": true},
            },
        }}}),
    );
    wire.response(1);
    wire.send(serde_json::json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {
        "textDocument": {"uri": uri, "languageId": "markdown", "version": 1, "text": REVIEW_DOC}}}),
    );

    // The menu, as Zed asks for it.
    wire.send(serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction",
        "params": {"textDocument": {"uri": uri},
                   "range": {"start": {"line": 0, "character": 0}, "end": {"line": 0, "character": 0}},
                   "context": {"diagnostics": [], "only": ["refactor.terraphim.trim"]}}}));
    let actions = wire.response(2);
    let titles: Vec<&str> = actions
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles.len(), 7, "{titles:?}");
    let sharper = actions
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == "Trim: Even sharper ~30%")
        .unwrap()["command"]
        .clone();
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 3, "method": "workspace/executeCommand",
        "params": {"command": sharper["command"], "arguments": sharper["arguments"]}}),
    );
    let status = wire.response(3);
    let words_after = status["words_after"].as_u64().unwrap() as usize;

    // The card: three buttons; choose "Make the cuts".
    let card = wire.request("window/showMessageRequest");
    let buttons: Vec<&str> = card["params"]["actions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["title"].as_str().unwrap())
        .collect();
    assert_eq!(buttons, ["Make the cuts", "Walk through", "Done"]);
    assert!(
        card["params"]["message"]
            .as_str()
            .unwrap()
            .contains(status["status"].as_str().unwrap())
    );
    wire.send(serde_json::json!({"jsonrpc": "2.0", "id": card["id"], "result": {"title": "Make the cuts"}}));

    // The edit arrives, versioned; apply it and report the new text.
    let apply = wire.request("workspace/applyEdit");
    let change = &apply["params"]["edit"]["documentChanges"][0];
    assert_eq!(change["textDocument"]["version"], 1);
    let edited = apply_lsp_edits(REVIEW_DOC, change["edits"].as_array().unwrap());
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
        "textDocument": {"uri": uri, "version": 2},
        "contentChanges": [{"text": edited}]}}),
    );
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "id": apply["id"], "result": {"applied": true}}),
    );
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {
        "textDocument": {"uri": uri}}}),
    );

    // No fades remain, and the word count is the card's.
    wire.send(
        serde_json::json!({"jsonrpc": "2.0", "id": 4, "method": "textDocument/diagnostic",
        "params": {"textDocument": {"uri": uri}}}),
    );
    let report = wire.response(4);
    let fades = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["code"] == "trim-candidate")
        .count();
    assert_eq!(fades, 0, "{report}");
    let config = terraphim_lsp::core::LabConfig::with_defaults().unwrap();
    assert_eq!(
        terraphim_lsp::core::trim_plan_for(&edited, &config).total_words(),
        words_after
    );
    wire.finish();
}
