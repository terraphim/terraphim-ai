//! Real server executable, protocol and files: JSON is tracked only for export safety.
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use tower_lsp::lsp_types::Url;

const COMMAND: &str = "terraphim.export_knowledge_graph";

struct Wire {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Value>,
    observed: Vec<Value>,
}

impl Wire {
    fn start() -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_terraphim-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("real LSP binary");
        let input = child.stdin.take().unwrap();
        let output = child.stdout.take().unwrap();
        let (sender, messages) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
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
        let mut wire = Self {
            child,
            input,
            messages,
            observed: Vec::new(),
        };
        wire.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId":null,"rootUri":null,"capabilities":{"workspace":{
                "applyEdit":true,"workspaceEdit":{"documentChanges":true,"resourceOperations":["create"]}
            }},"initializationOptions":{"lab":{"trigger":"save","actions":["hedges_and_filler"]},"reanchorOnSave":true}
        }}));
        let result = wire.response(1);
        assert!(
            result["result"]["capabilities"]["executeCommandProvider"]["commands"]
                .as_array()
                .unwrap()
                .contains(&json!(COMMAND)),
            "{result}"
        );
        wire.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
        wire
    }
    fn send(&mut self, value: Value) {
        let body = value.to_string();
        write!(self.input, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.input.flush().unwrap();
    }
    fn next(&mut self) -> Value {
        let value = self
            .messages
            .recv_timeout(Duration::from_secs(20))
            .expect("server response");
        self.observed.push(value.clone());
        value
    }
    fn response(&mut self, id: i32) -> Value {
        loop {
            let value = self.next();
            if value["id"] == id && value.get("method").is_none() {
                return value;
            }
            assert_ne!(
                value["method"], "workspace/applyEdit",
                "unexpected edit: {value}"
            );
        }
    }
    fn apply_request(&mut self) -> Value {
        loop {
            let value = self.next();
            if value["method"] == "workspace/applyEdit" {
                return value;
            }
            assert!(value.get("error").is_none(), "unexpected failure: {value}");
        }
    }
    fn open(&mut self, uri: &Url, language: &str, text: &str, version: i32) {
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri,"languageId":language,"version":version,"text":text}
            }}),
        );
        self.barrier(uri);
    }
    fn barrier(&mut self, uri: &Url) {
        self.send(
            json!({"jsonrpc":"2.0","id":9,"method":"textDocument/diagnostic","params":{
                "textDocument":{"uri":uri}
            }}),
        );
        assert!(self.response(9).get("error").is_none());
    }
    fn change(&mut self, uri: &Url, text: &str, version: i32) {
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
                "textDocument":{"uri":uri,"version":version},"contentChanges":[{"text":text}]
            }}),
        );
    }
    fn export(&mut self, source: &Url, version: i32) {
        self.send(
            json!({"jsonrpc":"2.0","id":10,"method":"workspace/executeCommand","params":{
                "command":COMMAND,"arguments":[{"uri":source,"version":version}]
            }}),
        );
    }
    fn close(&mut self, uri: &Url) {
        self.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}));
        self.barrier(uri);
    }
    fn acknowledge(&mut self, request: &Value, applied: bool) {
        self.send(json!({"jsonrpc":"2.0","id":request["id"],"result":{"applied":applied}}));
    }
}

impl Drop for Wire {
    fn drop(&mut self) {
        // A failed assertion must not leave the real server running.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn authored(alternative: &str) -> String {
    let body = "Risk: delivery may slip.\n";
    let added = terraphim_lsp::core::add_alternative(
        body,
        terraphim_lsp::core::TextRange::from_bytes(body, 0, 4),
        alternative,
        None,
    )
    .unwrap();
    terraphim_lsp::core::apply_edits(body, &[added.edit])
}

#[test]
fn json_and_jsonc_never_receive_markdown_analysis_or_commands() {
    for language in ["json", "jsonc"] {
        let directory = tempfile::tempdir().unwrap();
        let uri = Url::from_file_path(directory.path().join("graph.json")).unwrap();
        let mut wire = Wire::start();
        let text =
            "Risk is actually really probably possible.\n```terraphim-alternatives\n{invalid}\n```";
        wire.open(&uri, language, text, 1);
        for (method, extra) in [
            (
                "textDocument/hover",
                json!({"position":{"line":0,"character":1}}),
            ),
            (
                "textDocument/completion",
                json!({"position":{"line":0,"character":1}}),
            ),
            (
                "textDocument/codeAction",
                json!({"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":4}},"context":{"diagnostics":[]}}),
            ),
            (
                "textDocument/inlayHint",
                json!({"range":{"start":{"line":0,"character":0},"end":{"line":1,"character":0}}}),
            ),
            ("textDocument/foldingRange", json!({})),
        ] {
            let mut params = extra;
            params["textDocument"] = json!({"uri":uri});
            wire.send(json!({"jsonrpc":"2.0","id":20,"method":method,"params":params}));
            let response = wire.response(20);
            assert_eq!(
                response["result"],
                Value::Null,
                "{language} {method}: {response}"
            );
        }
        wire.send(json!({"jsonrpc":"2.0","method":"textDocument/didSave","params":{"textDocument":{"uri":uri}}}));
        wire.send(json!({"jsonrpc":"2.0","method":"workspace/didChangeConfiguration","params":{"settings":{"lab":{"trigger":"save"}}}}));
        wire.send(json!({"jsonrpc":"2.0","id":21,"method":"textDocument/diagnostic","params":{"textDocument":{"uri":uri}}}));
        assert_eq!(wire.response(21)["result"]["items"], json!([]));
        for (command, extra) in [
            ("terraphim.export", json!({})),
            ("terraphim.export_knowledge_graph", json!({})),
            ("terraphim.lab.mark", json!({"action":"hedges_and_filler"})),
            ("terraphim.trim.preview", json!({"level":"slight"})),
            ("terraphim.trim.make_cuts", json!({})),
            (
                "terraphim.alternative.add",
                json!({"text":"Concern","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":4}}}),
            ),
            (
                "terraphim.alternative.add_and_apply",
                json!({"text":"Concern","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":4}}}),
            ),
        ] {
            let mut argument = extra;
            argument["uri"] = json!(uri);
            argument["version"] = json!(1);
            wire.send(
                json!({"jsonrpc":"2.0","id":22,"method":"workspace/executeCommand","params":{
                    "command":command,"arguments":[argument]
                }}),
            );
            let result = wire.response(22);
            assert!(
                result["error"]["message"]
                    .as_str()
                    .is_some_and(|message| message.ends_with("is not open")),
                "{command} must reject JSON source eligibility, not malformed arguments: {result}"
            );
        }
        assert!(
            wire.observed.iter().all(|message| message["method"]
                != "textDocument/publishDiagnostics"
                || message["params"]["uri"] != json!(uri)
                || message["params"]["diagnostics"] == json!([])),
            "JSON must not receive Markdown findings"
        );
    }
}

#[test]
fn dirty_json_and_jsonc_refuse_without_touching_real_files() {
    for language in ["json", "jsonc"] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("brief.md");
        let target_path = directory.path().join("brief.knowledge-graph.json");
        let source = Url::from_file_path(&source_path).unwrap();
        let target = Url::from_file_path(&target_path).unwrap();
        let body = authored("Concern");
        let disk = "{\"name\":\"Original user graph\",\"data\":{}}\n";
        std::fs::write(&source_path, &body).unwrap();
        std::fs::write(&target_path, disk).unwrap();
        let mut wire = Wire::start();
        wire.open(&source, "markdown", &body, 1);
        wire.open(&target, language, disk, 1);
        let dirty = disk.replace("Original", "UNSAVED INTERIOR") + "UNSAVED APPEND\n";
        wire.change(&target, &dirty, 2);
        wire.barrier(&target);
        wire.export(&source, 1);
        let result = wire.response(10);
        assert_eq!(result["result"]["written"], false, "{result}");
        // Notifications and command responses use separate output queues.
        // Await the actual warning rather than assuming it preceded the result.
        while !wire.observed.iter().any(|message| {
            message["method"] == "window/showMessage"
                && message["params"]["message"]
                    .as_str()
                    .is_some_and(|text| text.contains("unconfirmed changes"))
        }) {
            let message = wire.next();
            assert_ne!(message["method"], "workspace/applyEdit");
        }
        assert_eq!(std::fs::read_to_string(&target_path).unwrap(), disk);
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), body);
        // Older changes cannot erase the known pending edit.
        wire.change(&target, disk, 1);
        wire.barrier(&target);
        wire.export(&source, 1);
        assert_eq!(wire.response(10)["result"]["written"], false);
    }
}

#[test]
fn closed_passive_buffer_changes_cannot_create_a_markdown_source() {
    let directory = tempfile::tempdir().unwrap();
    let uri = Url::from_file_path(directory.path().join("graph.json")).unwrap();
    let mut wire = Wire::start();
    wire.open(&uri, "json", "{}", 7);
    wire.close(&uri);
    wire.change(&uri, &authored("Concern"), 8);
    wire.barrier(&uri);
    wire.export(&uri, 8);
    assert!(wire.response(10).get("error").is_some());
    wire.open(&uri, "jsonc", "{}", 0);
    wire.export(&uri, 0);
    assert!(wire.response(10).get("error").is_some());
}

#[test]
fn clean_json_repeated_export_uses_versions_and_saves_actual_changed_graph() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("brief.md");
    let target_path = directory.path().join("brief.knowledge-graph.json");
    let source = Url::from_file_path(&source_path).unwrap();
    let target = Url::from_file_path(&target_path).unwrap();
    let mut wire = Wire::start();
    let mut current = "{}\n".to_string();
    std::fs::write(&target_path, &current).unwrap();
    wire.open(&target, "json", &current, 1);
    for (version, alternative) in [(1, "Concern"), (2, "Hazard")] {
        let body = authored(alternative);
        std::fs::write(&source_path, &body).unwrap();
        if version == 1 {
            wire.open(&source, "markdown", &body, version);
        } else {
            wire.change(&source, &body, version);
            wire.barrier(&source);
        }
        wire.export(&source, version);
        let request = wire.apply_request();
        let operations = request["params"]["edit"]["documentChanges"]
            .as_array()
            .unwrap();
        assert_eq!(
            operations.len(),
            1,
            "existing output must not be created/truncated"
        );
        let edit = &operations[0];
        assert_eq!(edit["textDocument"]["uri"], json!(target));
        assert_eq!(edit["textDocument"]["version"], version);
        let edits = edit["edits"].as_array().unwrap();
        assert_eq!(edits.len(), 1);
        let index = terraphim_lsp::core::LineIndex::new(&current);
        let end = &edits[0]["range"]["end"];
        assert_eq!(
            index.byte_offset(terraphim_lsp::core::LinePosition {
                line: end["line"].as_u64().unwrap() as u32,
                character: end["character"].as_u64().unwrap() as u32,
            }),
            current.len()
        );
        current = edits[0]["newText"].as_str().unwrap().to_string();
        std::fs::write(&target_path, &current).unwrap();
        wire.change(&target, &current, version + 1);
        wire.acknowledge(&request, true);
        assert_eq!(wire.response(10)["result"]["written"], true);
        wire.barrier(&target);
        let actual: Value = serde_json::from_slice(&std::fs::read(&target_path).unwrap()).unwrap();
        assert_eq!(
            actual["data"][alternative.to_lowercase()]["id"],
            actual["data"]["risk"]["id"]
        );
        assert!(actual["data"]["risk"]["id"].is_number());
        assert_eq!(std::fs::read_to_string(&source_path).unwrap(), body);
    }
    wire.close(&target);
    wire.open(
        &target,
        "json",
        &std::fs::read_to_string(&target_path).unwrap(),
        0,
    );
    wire.export(&source, 2);
    let request = wire.apply_request();
    assert_eq!(
        request["params"]["edit"]["documentChanges"][0]["textDocument"]["version"],
        0
    );
    wire.acknowledge(&request, false);
    assert_eq!(wire.response(10)["result"]["written"], false);
    assert_eq!(std::fs::read_to_string(&target_path).unwrap(), current);
}

#[test]
fn reopening_with_a_different_language_moves_the_buffer_kind() {
    let directory = tempfile::tempdir().unwrap();
    let uri = Url::from_file_path(directory.path().join("changed-language.json")).unwrap();
    let body = authored("Concern");
    let mut wire = Wire::start();
    for language in ["markdown", "json", "jsonc", "markdown"] {
        wire.open(&uri, language, &body, 0);
        wire.send(json!({"jsonrpc":"2.0","id":25,"method":"textDocument/codeAction","params":{
            "textDocument":{"uri":uri},"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":4}},"context":{"diagnostics":[]}
        }}));
        let result = wire.response(25);
        if language == "markdown" {
            assert!(
                result["result"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                "{result}"
            );
        } else {
            assert_eq!(result["result"], Value::Null);
        }
    }
}

#[test]
fn rejected_graph_payload_does_not_authorize_later_unsaved_output() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("brief.md");
    let target_path = directory.path().join("brief.knowledge-graph.json");
    let source = Url::from_file_path(&source_path).unwrap();
    let target = Url::from_file_path(&target_path).unwrap();
    let body = authored("Concern");
    let disk = "{}\n";
    std::fs::write(&source_path, &body).unwrap();
    std::fs::write(&target_path, disk).unwrap();
    let mut wire = Wire::start();
    wire.open(&source, "markdown", &body, 1);
    wire.open(&target, "json", disk, 1);
    wire.export(&source, 1);
    let request = wire.apply_request();
    let rejected = request["params"]["edit"]["documentChanges"][0]["edits"][0]["newText"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(rejected, disk);
    assert_eq!(
        request["params"]["edit"]["documentChanges"][0]["textDocument"]["version"],
        1
    );
    // The target changes after dispatch: the client refuses the version-1 edit.
    // This is new unsaved client input, not an acknowledged generated export.
    wire.change(&target, &rejected, 2);
    wire.barrier(&target);
    wire.acknowledge(&request, false);
    assert_eq!(wire.response(10)["result"]["written"], false);
    wire.export(&source, 1);
    assert_eq!(wire.response(10)["result"]["written"], false);
    assert_eq!(std::fs::read_to_string(target_path).unwrap(), disk);
    assert_eq!(std::fs::read_to_string(source_path).unwrap(), body);
}
