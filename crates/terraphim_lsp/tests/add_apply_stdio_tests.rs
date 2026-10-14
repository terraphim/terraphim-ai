//! Real stdio authoring: an LSP client applies the server request, saves the
//! annotation, reopens it and exports the resulting typed synonym graph.
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use tower_lsp::lsp_types::Url;

const BODY: &str = "Risk is manageable.\n";
const COMMAND: &str = "terraphim.alternative.add_and_apply";

struct Wire {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Value>,
}

impl Wire {
    fn start(apply: bool, versioned: bool) -> Self {
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
        };
        wire.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "processId":null,"rootUri":null,"capabilities":{"workspace":{
                "applyEdit":apply,"workspaceEdit":{"documentChanges":versioned,"resourceOperations":["create"]}
            }},"initializationOptions":{"lab":{"trigger":"command"}}
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
        self.messages
            .recv_timeout(Duration::from_secs(20))
            .expect("server response")
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
    fn open(&mut self, uri: &Url, text: &str, version: i32) {
        self.send(
            json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri,"languageId":"markdown","version":version,"text":text}
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
    fn add(&mut self, uri: &Url, version: i32) {
        self.send(
            json!({"jsonrpc":"2.0","id":10,"method":"workspace/executeCommand","params":{
                "command":COMMAND,"arguments":[{"uri":uri,"version":version,"text":"Concern",
                    "range":{"start":{"line":0,"character":0},"end":{"line":0,"character":4}}}]
            }}),
        );
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

fn uri() -> Url {
    Url::parse("file:///tmp/add-apply.md").unwrap()
}

fn edited(body: &str, request: &Value, uri: &Url, version: i32) -> String {
    let changes = request["params"]["edit"]["documentChanges"]
        .as_array()
        .unwrap();
    assert_eq!(changes.len(), 1);
    let change = &changes[0];
    assert_eq!(change["textDocument"]["uri"], json!(uri));
    assert_eq!(change["textDocument"]["version"], version);
    let index = terraphim_lsp::core::LineIndex::new(body);
    let offset = |p: &Value| {
        index.byte_offset(terraphim_lsp::core::LinePosition {
            line: p["line"].as_u64().unwrap() as u32,
            character: p["character"].as_u64().unwrap() as u32,
        })
    };
    let mut edits: Vec<_> = change["edits"].as_array().unwrap().iter().collect();
    edits.sort_by_key(|edit| std::cmp::Reverse(offset(&edit["range"]["start"])));
    let mut text = body.to_owned();
    for edit in edits {
        text.replace_range(
            offset(&edit["range"]["start"])..offset(&edit["range"]["end"]),
            edit["newText"].as_str().unwrap(),
        );
    }
    text
}

#[test]
fn applied_alternative_survives_saved_reopen_and_registers_exported_synonym() {
    for change_first in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("brief.md");
        std::fs::write(&path, BODY).unwrap();
        let uri = Url::from_file_path(&path).unwrap();
        let mut wire = Wire::start(true, true);
        wire.open(&uri, BODY, 1);
        wire.add(&uri, 1);
        let request = wire.apply_request();
        let text = edited(BODY, &request, &uri, 1);
        assert!(text.starts_with(BODY));
        let parsed = terraphim_alternatives::parse(&text).unwrap();
        assert!(!parsed.annotations.spans.is_empty());
        if change_first {
            wire.change(&uri, &text, 2);
        }
        wire.acknowledge(&request, true);
        if !change_first {
            wire.change(&uri, &text, 2);
        }
        let response = wire.response(10);
        assert_eq!(
            response["result"],
            json!({"applied":true,"uri":uri,"version":2})
        );
        assert!(
            response["result"].get("edit").is_none(),
            "a caller must not apply twice"
        );
        std::fs::write(&path, &text).unwrap();
        wire.send(json!({"jsonrpc":"2.0","method":"textDocument/didSave","params":{"textDocument":{"uri":uri}}}));
        wire.barrier(&uri);
        wire.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri}}}));
        wire.open(&uri, &std::fs::read_to_string(&path).unwrap(), 1);
        wire.send(
            json!({"jsonrpc":"2.0","id":11,"method":"workspace/executeCommand","params":{
                "command":"terraphim.export_knowledge_graph","arguments":[{"uri":uri,"version":1}]
            }}),
        );
        let export = wire.apply_request();
        let target = path.with_file_name("brief.knowledge-graph.json");
        let operations = export["params"]["edit"]["documentChanges"]
            .as_array()
            .unwrap();
        assert_eq!(operations[0]["kind"], "create");
        assert_eq!(
            operations[0]["uri"],
            json!(Url::from_file_path(&target).unwrap())
        );
        let graph_text = operations[1]["edits"][0]["newText"].as_str().unwrap();
        std::fs::write(&target, graph_text).unwrap();
        wire.acknowledge(&export, true);
        assert_eq!(wire.response(11)["result"]["written"], true);
        let graph: terraphim_types::Thesaurus =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(
            graph.get(&"risk".into()).unwrap().id,
            graph.get(&"concern".into()).unwrap().id
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn unsupported_capabilities_and_stale_input_never_request_edits() {
    for (apply, versioned) in [(false, true), (true, false)] {
        let mut wire = Wire::start(apply, versioned);
        wire.open(&uri(), BODY, 1);
        wire.add(&uri(), 1);
        assert_eq!(wire.response(10)["error"]["code"], -32602);
    }
    let mut wire = Wire::start(true, true);
    wire.open(&uri(), BODY, 2);
    wire.add(&uri(), 1);
    assert_eq!(wire.response(10)["error"]["code"], -32801);
}

#[test]
fn rejected_edit_is_distinguished_from_accepted_but_unobserved_edit() {
    for applied in [false, true] {
        let mut wire = Wire::start(true, true);
        wire.open(&uri(), BODY, 1);
        wire.add(&uri(), 1);
        let request = wire.apply_request();
        wire.acknowledge(&request, applied);
        let error = wire.response(10)["error"].clone();
        assert_eq!(error["code"], -32803);
        assert_eq!(
            error["data"]["status"],
            if applied {
                "accepted_not_observed"
            } else {
                "rejected"
            }
        );
        wire.send(
            json!({"jsonrpc":"2.0","id":12,"method":"workspace/executeCommand","params":{
                "command":"terraphim.export","arguments":[{"uri":uri(),"version":1}]
            }}),
        );
        let export = wire.apply_request();
        assert_eq!(
            export["params"]["edit"]["documentChanges"][1]["edits"][0]["newText"],
            BODY
        );
        wire.acknowledge(&export, false);
        assert_eq!(wire.response(12)["result"]["markdown"], BODY);
    }
}

#[test]
fn concurrent_change_or_reopen_cannot_confirm_the_old_edit() {
    for reopen in [false, true] {
        let mut wire = Wire::start(true, true);
        wire.open(&uri(), BODY, 1);
        wire.add(&uri(), 1);
        let request = wire.apply_request();
        let changed = if reopen {
            edited(BODY, &request, &uri(), 1)
        } else {
            "Other text.\n".into()
        };
        if reopen {
            wire.send(json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":uri()}}}));
            wire.open(&uri(), &changed, 2);
        } else {
            wire.change(&uri(), &changed, 2);
            wire.barrier(&uri());
        }
        wire.acknowledge(&request, true);
        assert_eq!(wire.response(10)["error"]["code"], -32801);
    }
}

#[test]
fn unanswered_apply_request_has_a_bounded_uncertain_outcome() {
    let mut wire = Wire::start(true, true);
    wire.open(&uri(), BODY, 1);
    wire.add(&uri(), 1);
    let _request = wire.apply_request();
    let start = std::time::Instant::now();
    let response = wire.response(10);
    assert_eq!(response["error"]["code"], -32803);
    assert_eq!(
        response["error"]["data"]["status"],
        "acknowledgement_timeout"
    );
    assert!(start.elapsed() < Duration::from_secs(15));
}

#[test]
fn late_change_after_timeout_is_observed_without_a_second_apply_request() {
    let mut wire = Wire::start(true, true);
    wire.open(&uri(), BODY, 1);
    wire.add(&uri(), 1);
    let request = wire.apply_request();
    let text = edited(BODY, &request, &uri(), 1);
    wire.acknowledge(&request, true);
    assert_eq!(
        wire.response(10)["error"]["data"]["status"],
        "accepted_not_observed"
    );
    // The edit may still arrive after an uncertain result. Do not retry it.
    wire.change(&uri(), &text, 2);
    wire.barrier(&uri());
    wire.send(
        json!({"jsonrpc":"2.0","id":13,"method":"workspace/executeCommand","params":{
            "command":"terraphim.export_knowledge_graph","arguments":[{"uri":uri(),"version":2}]
        }}),
    );
    let export = wire.apply_request();
    let graph: Value = serde_json::from_str(
        export["params"]["edit"]["documentChanges"][1]["edits"][0]["newText"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(graph["data"]["risk"]["id"], graph["data"]["concern"]["id"]);
    assert!(graph["data"]["concern"]["id"].is_number());
    wire.acknowledge(&export, false);
    let result = wire.response(13);
    assert_eq!(result["result"]["graph"], graph);
    assert_eq!(result["result"]["written"], false);
}
