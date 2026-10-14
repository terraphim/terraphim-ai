//! Folding the annotation block, re-anchoring it on save (R-9.2) and the
//! clean Markdown export (R-9.3), driven as a Zed-shaped client.
//!
//! No mocks: the server runs on real documents and is driven through its tower
//! `Service`, so tower-lsp really sends `workspace/applyEdit`, `window/showDocument`
//! and `window/logMessage` to the client. The test plays the client: it reads
//! every server message off the client socket and answers the server's requests
//! through the same socket.

use std::collections::VecDeque;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tower::{Service, ServiceExt};
use tower_lsp::jsonrpc::{ErrorCode, Request, Response};
use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, Loopback, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::commands;
use terraphim_lsp::core::{TextRange, apply_edits, ghost_range};
use terraphim_types::Thesaurus;

type ResponseSink = <ClientSocket as Loopback>::ResponseSink;

const THESAURUS_JSON: &str = include_str!("../tests/fixtures/writing_thesaurus.json");
const LAB_DOC: &str = include_str!("fixtures/lab_actions_doc.md");
const BODY: &str = "Alpha beta gamma delta.\nSecond line here.";

fn uri() -> Url {
    Url::parse("file:///tmp/notes.md").unwrap()
}

fn range_of(text: &str, needle: &str) -> TextRange {
    let start = text.find(needle).expect("needle in text");
    TextRange::from_bytes(text, start, start + needle.len())
}

/// `body` with "gamma" ghosted, written by the editor's own writer.
fn ghosted(body: &str) -> String {
    let edited = ghost_range(body, range_of(body, "gamma")).unwrap();
    apply_edits(body, std::slice::from_ref(&edited.edit))
}

/// A document whose block went stale: text was added in front of the body.
fn stale() -> String {
    ghosted(BODY).replacen("Alpha", "Preface. Alpha", 1)
}

fn parse_block(text: &str) -> terraphim_alternatives::Document {
    terraphim_alternatives::parse(text).expect("a well-formed block")
}

fn ghost_start(text: &str) -> usize {
    parse_block(text).annotations.ghosts[0].anchor.start
}

// ---------------------------------------------------------------- client --

#[derive(Clone, Copy)]
struct Caps {
    apply_edit: bool,
    create_file: bool,
    document_changes: bool,
    show_document: bool,
    pull: bool,
}

const ZED: Caps = Caps {
    apply_edit: true,
    create_file: true,
    document_changes: true,
    show_document: true,
    pull: false,
};

fn capabilities(caps: Caps) -> Value {
    let mut value = json!({
        "workspace": {
            "applyEdit": caps.apply_edit,
            "workspaceEdit": {
                "documentChanges": caps.document_changes,
                "resourceOperations": if caps.create_file { json!(["create"]) } else { json!([]) },
            },
        },
        "window": {"showDocument": {"support": caps.show_document}},
    });
    if caps.pull {
        value["textDocument"] = json!({"diagnostic": {"dynamicRegistration": false}});
    }
    value
}

/// The client half: what the server sent, and the socket that answers it.
struct Client {
    inbox: UnboundedReceiver<Request>,
    replies: ResponseSink,
    pending: VecDeque<Request>,
}

impl Client {
    /// The next server message with `method`; others stay pending.
    async fn next(&mut self, method: &str) -> Request {
        if let Some(at) = self.pending.iter().position(|m| m.method() == method) {
            return self.pending.remove(at).unwrap();
        }
        loop {
            let request = tokio::time::timeout(Duration::from_secs(20), self.inbox.recv())
                .await
                .unwrap_or_else(|_| panic!("no {method} from the server"))
                .expect("socket open");
            if request.method() == method {
                return request;
            }
            self.pending.push_back(request);
        }
    }

    /// Everything the server sends until it has been quiet for a moment.
    async fn quiet(&mut self) -> Vec<Request> {
        while let Ok(Some(request)) =
            tokio::time::timeout(Duration::from_millis(400), self.inbox.recv()).await
        {
            self.pending.push_back(request);
        }
        self.pending.iter().cloned().collect()
    }

    async fn reply(&mut self, request: &Request, result: Value) {
        let id = request.id().cloned().expect("a request");
        self.replies
            .send(Response::from_ok(id, result))
            .await
            .unwrap();
    }

    /// Answer the next `method` request with `result`; its params.
    async fn answer(&mut self, method: &str, result: Value) -> Value {
        let request = self.next(method).await;
        self.reply(&request, result).await;
        request.params().cloned().unwrap()
    }
}

struct Harness {
    service: LspService<TerraphimLspServer>,
    client: Client,
    version: i32,
    text: String,
    /// Another open document whose `didChange` the client sends between
    /// the reply and the saved document's change.
    noise: Option<Url>,
}

impl Harness {
    async fn start(caps: Caps, options: Value, uri: Url, text: &str) -> Self {
        let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
        let (mut service, socket) =
            LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
        let (mut stream, replies) = socket.split();
        let (sender, inbox) = unbounded_channel();
        tokio::spawn(async move {
            while let Some(request) = stream.next().await {
                if sender.send(request).is_err() {
                    break;
                }
            }
        });
        let params = json!({
            "processId": null,
            "rootUri": null,
            "capabilities": capabilities(caps),
            "initializationOptions": options,
        });
        let initialize = Request::build("initialize").params(params).id(1).finish();
        service
            .ready()
            .await
            .unwrap()
            .call(initialize)
            .await
            .unwrap();
        let initialized = Request::build("initialized").params(json!({})).finish();
        service
            .ready()
            .await
            .unwrap()
            .call(initialized)
            .await
            .unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: text.to_string(),
                },
            })
            .await;
        Self {
            service,
            client: Client {
                inbox,
                replies,
                pending: VecDeque::new(),
            },
            version: 1,
            text: text.to_string(),
            noise: None,
        }
    }

    fn server(&self) -> &TerraphimLspServer {
        self.service.inner()
    }

    /// Edit the document as the client does after applying an edit.
    async fn change(&mut self, text: String) {
        self.version += 1;
        self.text = text;
        self.server()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version: self.version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: self.text.clone(),
                }],
            })
            .await;
    }

    /// Save, answering the server's `workspace/applyEdit` the way a client
    /// does: the edit is applied, `didChange` follows, then the reply. The
    /// edit's versioned text edits, if the server sent one.
    async fn save_applying(&mut self) -> Option<(Option<i32>, Vec<TextEdit>)> {
        self.save_applying_in(true).await
    }

    /// [`Self::save_applying`], with the client's `didChange` sent before the
    /// reply or (`change_first` false) after it: both orders are legal.
    async fn save_applying_in(
        &mut self,
        change_first: bool,
    ) -> Option<(Option<i32>, Vec<TextEdit>)> {
        let Self {
            service,
            client,
            version,
            text,
            noise,
        } = self;
        let server = service.inner();
        let save = server.did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            text: None,
        });
        let respond = async {
            let request = tokio::time::timeout(
                Duration::from_millis(1500),
                client.next("workspace/applyEdit"),
            )
            .await
            .ok()?;
            let params: ApplyWorkspaceEditParams =
                serde_json::from_value(request.params().cloned().unwrap()).unwrap();
            let sent = versioned_edits(&params.edit);
            *text = apply_edits_lsp(text, &sent.1);
            *version += 1;
            let change = DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version: *version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: text.clone(),
                }],
            };
            if change_first {
                server.did_change(change).await;
                client.reply(&request, json!({"applied": true})).await;
            } else {
                client.reply(&request, json!({"applied": true})).await;
                if let Some(other) = noise.as_ref() {
                    server
                        .did_change(DidChangeTextDocumentParams {
                            text_document: VersionedTextDocumentIdentifier {
                                uri: other.clone(),
                                version: 2,
                            },
                            content_changes: vec![TextDocumentContentChangeEvent {
                                range: None,
                                range_length: None,
                                text: "Unrelated edit.".to_string(),
                            }],
                        })
                        .await;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
                server.did_change(change).await;
            }
            Some(sent)
        };
        let (_, sent) = tokio::join!(save, respond);
        sent
    }

    async fn execute(
        &mut self,
        command: &str,
        arguments: Vec<Value>,
    ) -> tower_lsp::jsonrpc::Result<Option<Value>> {
        self.server()
            .execute_command(ExecuteCommandParams {
                command: command.to_string(),
                arguments,
                work_done_progress_params: Default::default(),
            })
            .await
    }
}

fn versioned_edits(edit: &WorkspaceEdit) -> (Option<i32>, Vec<TextEdit>) {
    match edit.document_changes.as_ref().expect("documentChanges") {
        DocumentChanges::Edits(edits) => {
            let [one] = edits.as_slice() else {
                panic!("one document edit")
            };
            (
                one.text_document.version,
                one.edits
                    .iter()
                    .map(|e| match e {
                        OneOf::Left(edit) => edit.clone(),
                        OneOf::Right(annotated) => annotated.text_edit.clone(),
                    })
                    .collect(),
            )
        }
        other => panic!("expected document edits, got {other:?}"),
    }
}

fn apply_edits_lsp(text: &str, edits: &[TextEdit]) -> String {
    use terraphim_lsp::core::{LineIndex, LinePosition};
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    let mut out = String::new();
    let mut copied = 0;
    for edit in edits {
        let (start, end) = (byte(edit.range.start), byte(edit.range.end));
        out.push_str(&text[copied..start]);
        out.push_str(&edit.new_text);
        copied = end;
    }
    out.push_str(&text[copied..]);
    out
}

// --------------------------------------------------------------- folding --

async fn fold(harness: &Harness, uri: Url) -> Option<Vec<FoldingRange>> {
    harness
        .server()
        .folding_range(FoldingRangeParams {
            text_document: TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn folding_is_advertised() {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).unwrap();
    let (service, _socket) =
        LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()));
    let response = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    assert_eq!(
        response.capabilities.folding_range_provider,
        Some(FoldingRangeProviderCapability::Simple(true))
    );
}

#[tokio::test]
async fn the_block_folds_as_one_region() {
    let text = ghosted(BODY);
    let harness = Harness::start(ZED, json!({}), uri(), &text).await;
    let ranges = fold(&harness, uri()).await.expect("a fold");
    // Body is two lines, a blank line, then fence, JSON, fence.
    let block_lines = text.lines().count() as u32;
    let [range] = ranges.as_slice() else {
        panic!("one range, got {ranges:?}")
    };
    assert_eq!(range.kind, Some(FoldingRangeKind::Region));
    assert_eq!(range.start_line, 3);
    assert_eq!(range.end_line, block_lines - 1);
    assert!(text.lines().nth(3).unwrap().starts_with("```terraphim"));
    assert_eq!(text.lines().nth(range.end_line as usize), Some("```"));
}

#[tokio::test]
async fn nothing_folds_without_a_block_or_for_an_unopened_document() {
    let harness = Harness::start(ZED, json!({}), uri(), BODY).await;
    assert_eq!(fold(&harness, uri()).await, None);
    assert_eq!(
        fold(&harness, Url::parse("file:///tmp/other.md").unwrap()).await,
        None
    );
}

#[tokio::test]
async fn the_fold_follows_edits() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &ghosted(BODY)).await;
    let first = fold(&harness, uri()).await.unwrap()[0].start_line;
    harness
        .change(format!("New first line\n{}", harness.text))
        .await;
    let second = fold(&harness, uri()).await.unwrap()[0].start_line;
    assert_eq!(second, first + 1);
}

// ------------------------------------------------------------ save: anchor --

#[tokio::test]
async fn saving_reanchors_the_block_with_one_versioned_edit() {
    let text = stale();
    assert_eq!(ghost_start(&text), 11, "the stored offset is stale");
    let mut harness = Harness::start(ZED, json!({}), uri(), &text).await;
    let (version, edits) = harness.save_applying().await.expect("an edit on save");
    assert_eq!(version, Some(1), "rejected by the client if stale");
    let [edit] = edits.as_slice() else {
        panic!("one edit, got {edits:?}")
    };
    // Only the block: the edit starts after the body.
    let body_lines = parse_block(&text).body.lines().count() as u32;
    assert!(edit.range.start.line >= body_lines - 1, "{edit:?}");
    assert_eq!(ghost_start(&harness.text), 20);
    assert_eq!(
        parse_block(&harness.text).body,
        parse_block(&text).body,
        "the body is untouched"
    );
}

#[tokio::test]
async fn saving_again_changes_nothing() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &stale()).await;
    harness.save_applying().await.expect("the first save edits");
    let again = harness.save_applying().await;
    assert!(again.is_none(), "idempotent: {again:?}");
    let messages = harness.client.quiet().await;
    assert!(
        messages.iter().all(|m| m.method() != "workspace/applyEdit"),
        "no loop"
    );
}

/// Two saves can arrive before either acknowledgement or didChange. A client
/// that rebases versioned edits must never receive the same rewrite twice.
#[tokio::test]
async fn overlapping_saves_send_one_reanchor_per_document_version() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &stale()).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let save = || {
        server.did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            text: None,
        })
    };
    let respond = async {
        let first = client.next("workspace/applyEdit").await;
        let extra = client.quiet().await;
        let duplicates: Vec<_> = extra
            .iter()
            .filter(|m| m.method() == "workspace/applyEdit")
            .cloned()
            .collect();
        // Drain and reject duplicates so a regression fails the assertion,
        // rather than hanging on an unanswered request.
        for duplicate in &duplicates {
            client.reply(duplicate, json!({"applied": false})).await;
        }
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(first.params().cloned().unwrap()).unwrap();
        let (version, edits) = versioned_edits(&params.edit);
        assert_eq!(version, Some(1));
        let result = apply_edits_lsp(&stale(), &edits);
        server
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version: 2,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: result.clone(),
                }],
            })
            .await;
        client.reply(&first, json!({"applied": true})).await;
        assert!(
            duplicates.is_empty(),
            "duplicate rewrites of the same version: {duplicates:?}"
        );
        assert_eq!(parse_block(&result).body, parse_block(&stale()).body);
        assert_eq!(ghost_start(&result), 20);
    };
    tokio::join!(save(), save(), respond);
}

#[tokio::test]
async fn old_save_ack_cannot_release_a_new_revision_or_reopened_document() {
    for reopen in [false, true] {
        for old_applied in [false, true] {
            let mut harness = Harness::start(ZED, json!({}), uri(), &stale()).await;
            let (server, client) = (harness.service.inner(), &mut harness.client);
            let save = || {
                server.did_save(DidSaveTextDocumentParams {
                    text_document: TextDocumentIdentifier { uri: uri() },
                    text: None,
                })
            };
            let old = save();
            tokio::pin!(old);
            let first = tokio::select! {
                first = client.next("workspace/applyEdit") => first,
                _ = &mut old => panic!("save returned before requesting the edit"),
            };
            let version = if reopen {
                server
                    .did_close(DidCloseTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri: uri() },
                    })
                    .await;
                server
                    .did_open(DidOpenTextDocumentParams {
                        text_document: TextDocumentItem {
                            uri: uri(),
                            language_id: "markdown".into(),
                            version: 1,
                            text: stale(),
                        },
                    })
                    .await;
                1
            } else {
                server
                    .did_change(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri(),
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: stale(),
                        }],
                    })
                    .await;
                2
            };
            let current = save();
            tokio::pin!(current);
            let second = tokio::select! {
                second = client.next("workspace/applyEdit") => second,
                _ = &mut current => panic!("new incarnation/revision must be rewriteable"),
            };
            client.reply(&first, json!({"applied": old_applied})).await;
            old.await;
            let respond = async {
                let duplicates: Vec<_> = client
                    .quiet()
                    .await
                    .into_iter()
                    .filter(|m| m.method() == "workspace/applyEdit")
                    .collect();
                for duplicate in &duplicates {
                    client.reply(duplicate, json!({"applied": false})).await;
                }
                assert!(duplicates.is_empty(), "old ACK released new reservation");
                let params: ApplyWorkspaceEditParams =
                    serde_json::from_value(second.params().cloned().unwrap()).unwrap();
                let (sent_version, edits) = versioned_edits(&params.edit);
                assert_eq!(sent_version, Some(version));
                let output = apply_edits_lsp(&stale(), &edits);
                assert_eq!(ghost_start(&output), 20);
                server
                    .did_change(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri: uri(),
                            version: version + 1,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: output,
                        }],
                    })
                    .await;
                client.reply(&second, json!({"applied": true})).await;
            };
            tokio::join!(save(), respond, current);
        }
    }
}

#[tokio::test]
async fn rejected_reanchor_can_be_retried_at_the_same_version() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &stale()).await;
    {
        let (server, client) = (harness.service.inner(), &mut harness.client);
        tokio::join!(
            server.did_save(DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                text: None,
            }),
            client.answer("workspace/applyEdit", json!({"applied": false}))
        );
    }
    harness
        .save_applying()
        .await
        .expect("rejected rewrite must be retryable");
    assert_eq!(ghost_start(&harness.text), 20);
}

#[tokio::test]
async fn a_current_block_is_not_rewritten() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &ghosted(BODY)).await;
    assert!(harness.save_applying().await.is_none());
}

#[tokio::test]
async fn the_setting_turns_reanchoring_off() {
    let mut harness = Harness::start(ZED, json!({"reanchorOnSave": false}), uri(), &stale()).await;
    assert!(harness.save_applying().await.is_none());
    // Nested under `terraphim`, as Zed passes settings.
    let mut nested = Harness::start(
        ZED,
        json!({"terraphim": {"reanchorOnSave": false}}),
        uri(),
        &stale(),
    )
    .await;
    assert!(nested.save_applying().await.is_none());
}

#[tokio::test]
async fn a_client_that_cannot_apply_edits_is_left_alone() {
    let caps = Caps {
        apply_edit: false,
        ..ZED
    };
    let mut harness = Harness::start(caps, json!({}), uri(), &stale()).await;
    assert!(harness.save_applying().await.is_none());
}

#[tokio::test]
async fn a_lost_anchor_leaves_the_block_alone_and_says_so() {
    let lost = "Totally different.\n\n```terraphim-alternatives\n{\"version\": 1, \"spans\": [], \"ghosts\": [{\"id\": \"g1\", \"anchor\": {\"start\": 0, \"end\": 4, \"text\": \"gone\"}}]}\n```\n";
    let mut harness = Harness::start(ZED, json!({}), uri(), lost).await;
    assert!(harness.save_applying().await.is_none());
    let messages = harness.client.quiet().await;
    let logged = messages
        .iter()
        .find(|m| m.method() == "window/logMessage")
        .expect("a log message");
    assert!(
        logged.params().unwrap()["message"]
            .as_str()
            .unwrap()
            .contains("no longer match the body")
    );
}

#[tokio::test]
async fn a_malformed_block_is_left_alone() {
    let malformed = format!("{BODY}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    let mut harness = Harness::start(ZED, json!({}), uri(), &malformed).await;
    assert!(harness.save_applying().await.is_none());
}

#[tokio::test]
async fn a_client_that_rejects_the_edit_changes_nothing() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &stale()).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let save = server.did_save(DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri() },
        text: None,
    });
    let respond = client.answer(
        "workspace/applyEdit",
        json!({"applied": false, "failureReason": "stale"}),
    );
    tokio::join!(save, respond);
    assert_eq!(ghost_start(&harness.text), 11);
}

#[tokio::test]
async fn lab_marks_survive_the_block_edit_of_a_save() {
    for change_first in [true, false] {
        lab_marks_survive(change_first).await;
    }
}

async fn lab_marks_survive(change_first: bool) {
    // The Lab marks computed on save must survive the block edit's change.
    let text = format!(
        "{LAB_DOC}\n\n```terraphim-alternatives\n{{\"version\": 1, \"spans\": [], \"ghosts\": [{{\"id\": \"g1\", \"anchor\": {{\"start\": 0, \"end\": 5, \"text\": \"We re\"}}}}]}}\n```\n"
    );
    let mut harness = Harness::start(
        Caps { pull: true, ..ZED },
        json!({"lab": {"actions": ["off_tone"]}}),
        uri(),
        &text,
    )
    .await;
    harness.client.quiet().await;
    harness.client.pending.clear();
    let sent = harness.save_applying_in(change_first).await;
    assert!(sent.is_some(), "the stale ghost is re-anchored");
    let report = harness
        .server()
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();
    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) = report
    else {
        panic!("a full report")
    };
    assert!(
        full.full_document_diagnostic_report
            .items
            .iter()
            .any(|d| d.code == Some(NumberOrString::String("lab-off-tone".to_string()))),
        "the Lab marks survived the block edit (change first: {change_first})"
    );
}

// ---------------------------------------------------------------- export --

fn export_args(version: Option<i32>) -> Vec<Value> {
    vec![json!({"uri": uri(), "version": version})]
}

/// A document with a ghost and overflow.
fn exportable() -> String {
    let json = r#"{"version": 1, "spans": [], "ghosts": [{"id": "g1", "anchor": {"start": 11, "end": 21, "text": "Drop this."}}], "overflow": "private notes"}"#;
    format!("Keep this. Drop this.\n\n```terraphim-alternatives\n{json}\n```\n")
}

#[tokio::test]
async fn export_is_advertised() {
    assert!(commands::ALL.contains(&commands::EXPORT));
    assert!(commands::ALL.contains(&commands::EXPORT_KNOWLEDGE_GRAPH));
}

fn authored_graph_document() -> String {
    let body = "Risk: delivery may slip.\n";
    let added = terraphim_lsp::core::add_alternative(
        body,
        TextRange::from_bytes(body, 0, 4),
        "Concern",
        None,
    )
    .unwrap();
    apply_edits(body, &[added.edit])
}

#[tokio::test]
async fn knowledge_graph_export_uses_actual_registration_and_rejects_stale_versions() {
    let mut harness = Harness::start(
        Caps {
            apply_edit: false,
            ..ZED
        },
        json!({}),
        uri(),
        &authored_graph_document(),
    )
    .await;
    let result = harness
        .execute(commands::EXPORT_KNOWLEDGE_GRAPH, export_args(Some(1)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        result["graph"]["data"]["risk"]["id"],
        result["graph"]["data"]["concern"]["id"]
    );
    assert!(result["graph"]["data"]["risk"]["id"].is_number());
    assert_eq!(result["written"], false);
    assert_eq!(result["uri"], "file:///tmp/notes.knowledge-graph.json");
    let error = harness
        .execute(commands::EXPORT_KNOWLEDGE_GRAPH, export_args(Some(7)))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ContentModified);
}

#[tokio::test]
async fn knowledge_graph_export_client_writes_actual_file_and_preserves_source() {
    let directory = tempfile::tempdir().unwrap();
    let source_path = directory.path().join("brief.md");
    let source = Url::from_file_path(&source_path).unwrap();
    let target = directory.path().join("brief.knowledge-graph.json");
    let original = authored_graph_document();
    std::fs::write(&source_path, &original).unwrap();
    let mut harness = Harness::start(ZED, json!({}), source.clone(), &original).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let run = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT_KNOWLEDGE_GRAPH.into(),
        arguments: vec![json!({"uri": source, "version": 1})],
        work_done_progress_params: Default::default(),
    });
    let respond = async {
        let request = client.next("workspace/applyEdit").await;
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
        let DocumentChanges::Operations(operations) = params.edit.document_changes.unwrap() else {
            panic!("operations expected")
        };
        let [
            DocumentChangeOperation::Op(ResourceOp::Create(create)),
            DocumentChangeOperation::Edit(edit),
        ] = operations.as_slice()
        else {
            panic!("create and edit expected")
        };
        assert_eq!(create.uri.to_file_path().unwrap(), target);
        assert_eq!(create.options.as_ref().unwrap().overwrite, Some(false));
        let edits: Vec<_> = edit
            .edits
            .iter()
            .map(|edit| match edit {
                OneOf::Left(edit) => edit.clone(),
                _ => panic!("plain edit expected"),
            })
            .collect();
        let output = apply_edits_lsp("", &edits);
        std::fs::write(&target, &output).unwrap();
        let actual: Value =
            serde_json::from_str(&std::fs::read_to_string(&target).unwrap()).unwrap();
        assert_eq!(actual["data"]["concern"]["nterm"], "risk");
        client.reply(&request, json!({"applied": true})).await;
        client
            .answer("window/showDocument", json!({"success": true}))
            .await;
    };
    let (result, ()) = tokio::join!(run, respond);
    assert_eq!(result.unwrap().unwrap()["written"], true);
    assert_eq!(std::fs::read_to_string(source_path).unwrap(), original);
    let actual: Value = serde_json::from_slice(&std::fs::read(target).unwrap()).unwrap();
    assert_eq!(
        actual["data"]["risk"]["id"],
        actual["data"]["concern"]["id"]
    );
}

/// Exercise the real server request and apply any emitted edit to real files.
/// A refusal must finish without sending an edit; before the alias guard this
/// client writes through the link and the source-preservation assertion fails.
async fn export_applying_to_disk(
    harness: &mut Harness,
    command: &str,
    source: &Url,
) -> (Value, usize) {
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let mut applied = 0;
    let run = server.execute_command(ExecuteCommandParams {
        command: command.into(),
        arguments: vec![json!({"uri": source, "version": 1})],
        work_done_progress_params: Default::default(),
    });
    let respond = async {
        loop {
            let request = client.next("workspace/applyEdit").await;
            let params: ApplyWorkspaceEditParams =
                serde_json::from_value(request.params().cloned().unwrap()).unwrap();
            let edits = match params.edit.document_changes.unwrap() {
                DocumentChanges::Edits(edits) => edits,
                DocumentChanges::Operations(operations) => operations
                    .into_iter()
                    .map(|operation| match operation {
                        DocumentChangeOperation::Edit(edit) => edit,
                        _ => panic!("an existing alias must not be recreated"),
                    })
                    .collect(),
            };
            for edit in edits {
                let path = edit.text_document.uri.to_file_path().unwrap();
                let before = std::fs::read_to_string(&path).unwrap();
                let edits: Vec<_> = edit
                    .edits
                    .into_iter()
                    .map(|edit| match edit {
                        OneOf::Left(edit) => edit,
                        _ => panic!("plain edit expected"),
                    })
                    .collect();
                std::fs::write(&path, apply_edits_lsp(&before, &edits)).unwrap();
                applied += 1;
            }
            client.reply(&request, json!({"applied": true})).await;
            client
                .answer("window/showDocument", json!({"success": true}))
                .await;
        }
    };
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::select! {
            result = run => result.unwrap().unwrap(),
            () = respond => unreachable!("client response loop does not end"),
        }
    })
    .await
    .expect("an alias refusal completes without waiting for a client edit");
    (result, applied)
}

async fn assert_export_alias_refused(
    link: fn(&std::path::Path, &std::path::Path),
    other_open: bool,
    language: &str,
) {
    for (command, suffix) in [
        (commands::EXPORT_KNOWLEDGE_GRAPH, "knowledge-graph.json"),
        (commands::EXPORT, "clean.md"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("brief.md");
        let source = Url::from_file_path(&source_path).unwrap();
        let original = authored_graph_document();
        std::fs::write(&source_path, &original).unwrap();
        let mut harness = Harness::start(ZED, json!({}), source.clone(), &original).await;
        let protected_path = if other_open {
            let path = directory.path().join(format!("other.{language}"));
            std::fs::write(&path, "Saved other document.\n").unwrap();
            harness
                .server()
                .did_open(DidOpenTextDocumentParams {
                    text_document: TextDocumentItem {
                        uri: Url::from_file_path(&path).unwrap(),
                        language_id: language.into(),
                        version: 4,
                        text: "Unsaved changes in another document.\n".into(),
                    },
                })
                .await;
            path
        } else {
            source_path.clone()
        };
        let protected_bytes = std::fs::read(&protected_path).unwrap();
        let target = directory.path().join(format!("brief.{suffix}"));
        link(&protected_path, &target);
        let (result, applied) = export_applying_to_disk(&mut harness, command, &source).await;
        assert_eq!(std::fs::read(&source_path).unwrap(), original.as_bytes());
        assert_eq!(std::fs::read(&protected_path).unwrap(), protected_bytes);
        assert_eq!(std::fs::read(&target).unwrap(), protected_bytes);
        assert_eq!(
            applied, 0,
            "a protected alias must receive no workspace edit"
        );
        assert_eq!(result["written"], false);
        if command == commands::EXPORT_KNOWLEDGE_GRAPH {
            assert_eq!(result["graph"]["data"]["concern"]["nterm"], "risk");
        }
    }
}

#[tokio::test]
async fn graph_and_clean_exports_refuse_source_hard_links() {
    assert_export_alias_refused(
        |source, target| std::fs::hard_link(source, target).unwrap(),
        false,
        "markdown",
    )
    .await;
}

#[tokio::test]
async fn graph_and_clean_exports_refuse_dirty_open_document_hard_links() {
    assert_export_alias_refused(
        |source, target| std::fs::hard_link(source, target).unwrap(),
        true,
        "markdown",
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn graph_and_clean_exports_refuse_source_symlinks() {
    assert_export_alias_refused(
        |source, target| std::os::unix::fs::symlink(source, target).unwrap(),
        false,
        "markdown",
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn graph_and_clean_exports_refuse_dirty_open_document_symlinks() {
    assert_export_alias_refused(
        |source, target| std::os::unix::fs::symlink(source, target).unwrap(),
        true,
        "markdown",
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn graph_and_clean_exports_refuse_dangling_output_symlinks() {
    for (command, suffix) in [
        (commands::EXPORT_KNOWLEDGE_GRAPH, "knowledge-graph.json"),
        (commands::EXPORT, "clean.md"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("brief.md");
        let source = Url::from_file_path(&source_path).unwrap();
        let original = authored_graph_document();
        std::fs::write(&source_path, &original).unwrap();
        let mut harness = Harness::start(ZED, json!({}), source.clone(), &original).await;
        let target = directory.path().join(format!("brief.{suffix}"));
        let absent = directory.path().join("not-yet-created.md");
        std::os::unix::fs::symlink(&absent, &target).unwrap();
        let (result, applied) = export_applying_to_disk(&mut harness, command, &source).await;
        assert_eq!(result["written"], false);
        assert_eq!(applied, 0);
        assert_eq!(std::fs::read_to_string(source_path).unwrap(), original);
        assert!(!absent.exists());
        assert!(
            std::fs::symlink_metadata(target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }
}

#[test]
fn graph_and_clean_exports_refuse_source_changes_during_file_inspection() {
    // Occupy the real blocking pool so the actual filesystem inspection waits.
    // This schedules a real didChange/close/reopen after the export snapshot,
    // without replacing the filesystem or introducing a production test hook.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
        .block_on(async {
            for (command, suffix) in [
                (commands::EXPORT_KNOWLEDGE_GRAPH, "knowledge-graph.json"),
                (commands::EXPORT, "clean.md"),
            ] {
                for change in ["edit", "close", "reopen", "reopen_same"] {
                    let directory = tempfile::tempdir().unwrap();
                    let source_path = directory.path().join("brief.md");
                    let source = Url::from_file_path(&source_path).unwrap();
                    let original = authored_graph_document();
                    std::fs::write(&source_path, &original).unwrap();
                    let target = directory.path().join(format!("brief.{suffix}"));
                    let previous = "Previous output must remain unchanged.\n";
                    std::fs::write(&target, previous).unwrap();
                    let mut harness = Harness::start(
                        ZED,
                        json!({"lab": {"trigger": "command"}}),
                        source.clone(),
                        &original,
                    )
                    .await;
                    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
                    let (release_tx, release_rx) = std::sync::mpsc::channel();
                    let occupying = tokio::task::spawn_blocking(move || {
                        entered_tx.send(()).unwrap();
                        let _ = release_rx.recv();
                    });
                    entered_rx.await.unwrap();
                    let (server, client) = (harness.service.inner(), &mut harness.client);
                    let mut run = Box::pin(server.execute_command(ExecuteCommandParams {
                        command: command.into(),
                        arguments: vec![json!({"uri": source, "version": 1})],
                        work_done_progress_params: Default::default(),
                    }));
                    assert!(futures::poll!(&mut run).is_pending());
                    let updated = "Revised brief has no authored alternatives.\n";
                    if change == "edit" {
                        server
                            .did_change(DidChangeTextDocumentParams {
                                text_document: VersionedTextDocumentIdentifier {
                                    uri: source.clone(),
                                    version: 2,
                                },
                                content_changes: vec![TextDocumentContentChangeEvent {
                                    range: None,
                                    range_length: None,
                                    text: updated.into(),
                                }],
                            })
                            .await;
                    } else {
                        server
                            .did_close(DidCloseTextDocumentParams {
                                text_document: TextDocumentIdentifier {
                                    uri: source.clone(),
                                },
                            })
                            .await;
                        if change == "reopen" || change == "reopen_same" {
                            server
                                .did_open(DidOpenTextDocumentParams {
                                    text_document: TextDocumentItem {
                                        uri: source.clone(),
                                        language_id: "markdown".into(),
                                        version: 1,
                                        text: if change == "reopen_same" {
                                            original.clone()
                                        } else {
                                            updated.into()
                                        },
                                    },
                                })
                                .await;
                        }
                    }
                    release_tx.send(()).unwrap();
                    occupying.await.unwrap();
                    let respond = async {
                        let request = client.next("workspace/applyEdit").await;
                        let params: ApplyWorkspaceEditParams =
                            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
                        let (_, edits) = versioned_edits(&params.edit);
                        let output =
                            apply_edits_lsp(&std::fs::read_to_string(&target).unwrap(), &edits);
                        std::fs::write(&target, output).unwrap();
                        client.reply(&request, json!({"applied": true})).await;
                        client
                            .answer("window/showDocument", json!({"success": true}))
                            .await;
                        std::future::pending::<()>().await;
                    };
                    let result = tokio::time::timeout(Duration::from_secs(5), async {
                        tokio::select! {
                            result = run => result,
                            () = respond => unreachable!("response waits for export completion"),
                        }
                    })
                    .await
                    .expect("export must finish after the inspection is released");
                    assert_eq!(
                        std::fs::read_to_string(&target).unwrap(),
                        previous,
                        "{command}: {change}"
                    );
                    assert_eq!(std::fs::read_to_string(&source_path).unwrap(), original);
                    assert_eq!(
                        result.unwrap_err().code,
                        ErrorCode::ContentModified,
                        "{command}: {change}"
                    );
                    assert!(
                        client
                            .quiet()
                            .await
                            .iter()
                            .all(|request| request.method() != "workspace/applyEdit")
                    );
                }
            }
        });
}

#[tokio::test]
async fn export_writes_the_clean_file_beside_the_document() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &exportable()).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let run = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT.to_string(),
        arguments: export_args(Some(1)),
        work_done_progress_params: Default::default(),
    });
    let respond = async {
        let params = client
            .answer("workspace/applyEdit", json!({"applied": true}))
            .await;
        let shown = client
            .answer("window/showDocument", json!({"success": true}))
            .await;
        (params, shown)
    };
    let (result, (params, shown)) = tokio::join!(run, respond);
    let result = result.unwrap().unwrap();
    assert_eq!(result["markdown"], json!("Keep this."));
    assert_eq!(result["uri"], json!("file:///tmp/notes.clean.md"));
    assert_eq!(result["written"], json!(true));
    let edit: ApplyWorkspaceEditParams = serde_json::from_value(params).unwrap();
    let DocumentChanges::Operations(operations) = edit.edit.document_changes.unwrap() else {
        panic!("operations")
    };
    let [
        DocumentChangeOperation::Op(ResourceOp::Create(create)),
        DocumentChangeOperation::Edit(text),
    ] = operations.as_slice()
    else {
        panic!("create then edit, got {operations:?}")
    };
    assert_eq!(create.uri.as_str(), "file:///tmp/notes.clean.md");
    assert_eq!(
        create.options.as_ref().and_then(|o| o.overwrite),
        Some(false)
    );
    assert_eq!(text.text_document.uri, create.uri);
    let OneOf::Left(insert) = &text.edits[0] else {
        panic!("plain edit")
    };
    assert_eq!(insert.new_text, "Keep this.");
    assert_eq!(insert.range, Range::default());
    assert_eq!(shown["uri"], json!("file:///tmp/notes.clean.md"));
    assert_eq!(shown["takeFocus"], json!(true));
}

#[tokio::test]
async fn export_without_file_creation_returns_the_text_only() {
    for caps in [
        Caps {
            create_file: false,
            ..ZED
        },
        Caps {
            apply_edit: false,
            ..ZED
        },
    ] {
        let mut harness = Harness::start(caps, json!({}), uri(), &exportable()).await;
        let result = harness
            .execute(commands::EXPORT, export_args(None))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result["markdown"], json!("Keep this."));
        assert_eq!(result["written"], json!(false));
        let messages = harness.client.quiet().await;
        assert!(messages.iter().all(|m| m.method() != "workspace/applyEdit"));
    }
}

#[tokio::test]
async fn a_rejected_export_edit_reports_not_written() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &exportable()).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let run = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT.to_string(),
        arguments: export_args(None),
        work_done_progress_params: Default::default(),
    });
    let respond = client.answer("workspace/applyEdit", json!({"applied": false}));
    let (result, _) = tokio::join!(run, respond);
    assert_eq!(result.unwrap().unwrap()["written"], json!(false));
    let messages = harness.client.quiet().await;
    assert!(messages.iter().all(|m| m.method() != "window/showDocument"));
}

#[tokio::test]
async fn export_of_a_non_file_document_returns_the_text_only() {
    let untitled = Url::parse("untitled:Untitled-1").unwrap();
    let mut harness = Harness::start(ZED, json!({}), untitled.clone(), &exportable()).await;
    let result = harness
        .execute(commands::EXPORT, vec![json!({"uri": untitled})])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["markdown"], json!("Keep this."));
    assert_eq!(result["uri"], Value::Null);
    assert_eq!(result["written"], json!(false));
}

#[tokio::test]
async fn export_refuses_malformed_blocks_and_stale_versions() {
    let malformed = format!("{BODY}\n\n```terraphim-alternatives\n{{ nope\n```\n");
    let mut harness = Harness::start(ZED, json!({}), uri(), &malformed).await;
    let error = harness
        .execute(commands::EXPORT, export_args(None))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidParams);

    let mut harness = Harness::start(ZED, json!({}), uri(), &exportable()).await;
    let error = harness
        .execute(commands::EXPORT, export_args(Some(7)))
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ContentModified);
}

#[tokio::test]
async fn export_cuts_a_ghost_whose_offsets_went_stale() {
    let mut harness = Harness::start(
        Caps {
            create_file: false,
            ..ZED
        },
        json!({}),
        uri(),
        &stale(),
    )
    .await;
    let result = harness
        .execute(commands::EXPORT, export_args(None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        result["markdown"],
        json!("Preface. Alpha beta delta.\nSecond line here.")
    );
}

#[tokio::test]
async fn saving_needs_documentchanges_so_the_edit_is_versioned() {
    let caps = Caps {
        document_changes: false,
        ..ZED
    };
    let mut harness = Harness::start(caps, json!({}), uri(), &stale()).await;
    assert!(harness.save_applying().await.is_none());
    let messages = harness.client.quiet().await;
    assert!(messages.iter().all(|m| m.method() != "workspace/applyEdit"));
}

#[tokio::test]
async fn export_needs_documentchanges_for_the_create_operation() {
    let caps = Caps {
        document_changes: false,
        ..ZED
    };
    let mut harness = Harness::start(caps, json!({}), uri(), &exportable()).await;
    let result = harness
        .execute(commands::EXPORT, export_args(None))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result["markdown"], json!("Keep this."));
    assert_eq!(result["written"], json!(false));
    let messages = harness.client.quiet().await;
    assert!(messages.iter().all(|m| m.method() != "workspace/applyEdit"));
}

#[tokio::test]
async fn a_change_to_another_document_does_not_end_the_wait() {
    // The Lab refresh must wait for the saved document's own change, not
    // for the unrelated document's.
    let text = format!(
        "{LAB_DOC}\n\n```terraphim-alternatives\n{{\"version\": 1, \"spans\": [], \"ghosts\": [{{\"id\": \"g1\", \"anchor\": {{\"start\": 0, \"end\": 5, \"text\": \"We re\"}}}}]}}\n```\n"
    );
    let mut harness = Harness::start(
        Caps { pull: true, ..ZED },
        json!({"lab": {"actions": ["off_tone"]}}),
        uri(),
        &text,
    )
    .await;
    let other = Url::parse("file:///tmp/other.md").unwrap();
    harness
        .server()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: other.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "Other.".to_string(),
            },
        })
        .await;
    harness.noise = Some(other);
    harness.client.quiet().await;
    harness.client.pending.clear();
    assert!(harness.save_applying_in(false).await.is_some());
    let report = harness
        .server()
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();
    let DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) = report
    else {
        panic!("a full report")
    };
    assert!(
        full.full_document_diagnostic_report
            .items
            .iter()
            .any(|d| d.code == Some(NumberOrString::String("lab-off-tone".to_string()))),
        "the Lab marks survived"
    );
}

#[tokio::test]
async fn repeated_export_replaces_open_output_with_one_versioned_edit() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &exportable()).await;
    let target = Url::parse("file:///tmp/notes.clean.md").unwrap();
    // First export establishes what this session generated. Then the real
    // client opens that buffer; CreateFile must never truncate it on repeat.
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let first = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT.to_string(),
        arguments: export_args(Some(1)),
        work_done_progress_params: Default::default(),
    });
    let acknowledge = async {
        client
            .answer("workspace/applyEdit", json!({"applied": true}))
            .await;
        client
            .answer("window/showDocument", json!({"success": true}))
            .await;
    };
    let (result, ()) = tokio::join!(first, acknowledge);
    let previous = result.unwrap().unwrap()["markdown"]
        .as_str()
        .unwrap()
        .to_string();
    harness
        .server()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: target.clone(),
                language_id: "markdown".into(),
                version: 7,
                text: previous.clone(),
            },
        })
        .await;
    harness
        .change(exportable().replacen("Keep this.", "Keep that.", 1))
        .await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let run = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT.to_string(),
        arguments: export_args(Some(2)),
        work_done_progress_params: Default::default(),
    });
    let respond = async {
        let request = client.next("workspace/applyEdit").await;
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
        let edits = match params.edit.document_changes.unwrap() {
            DocumentChanges::Operations(ops) => {
                assert!(
                    ops.iter()
                        .all(|op| matches!(op, DocumentChangeOperation::Edit(_))),
                    "open output must not be recreated"
                );
                let DocumentChangeOperation::Edit(edit) = ops.into_iter().next().unwrap() else {
                    panic!()
                };
                edit
            }
            DocumentChanges::Edits(mut edits) => edits.remove(0),
        };
        assert_eq!(edits.text_document.uri, target);
        assert_eq!(edits.text_document.version, Some(7));
        let plain: Vec<_> = edits
            .edits
            .into_iter()
            .map(|e| match e {
                OneOf::Left(e) => e,
                _ => panic!(),
            })
            .collect();
        assert_eq!(apply_edits_lsp(&previous, &plain), "Keep that.");
        client.reply(&request, json!({"applied": true})).await;
        client
            .answer("window/showDocument", json!({"success": true}))
            .await;
    };
    let (result, ()) = tokio::join!(run, respond);
    assert_eq!(result.unwrap().unwrap()["written"], true);
}

#[tokio::test]
async fn late_or_duplicate_changes_cannot_replace_a_newer_snapshot() {
    let mut harness = Harness::start(
        Caps {
            create_file: false,
            ..ZED
        },
        json!({}),
        uri(),
        BODY,
    )
    .await;
    harness.change("Newer 😀 body.\r\n".to_string()).await;
    for version in [1, 2] {
        harness
            .server()
            .did_change(DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier {
                    uri: uri(),
                    version,
                },
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: "Stale body.".into(),
                }],
            })
            .await;
        let actual = harness
            .execute(commands::EXPORT, export_args(Some(2)))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            actual["markdown"], "Newer 😀 body.\r\n",
            "version {version} must be ignored"
        );
    }
}

#[tokio::test]
async fn export_refuses_an_unconfirmed_dirty_open_output() {
    let mut harness = Harness::start(ZED, json!({}), uri(), &exportable()).await;
    harness
        .server()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: Url::parse("file:///tmp/notes.clean.md").unwrap(),
                language_id: "markdown".into(),
                version: 3,
                text: "My unsaved edits must survive.".into(),
            },
        })
        .await;
    let actual = harness
        .execute(commands::EXPORT, export_args(Some(1)))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(actual["written"], false);
    assert_eq!(actual["markdown"], "Keep this.");
    let messages = harness.client.quiet().await;
    assert!(messages.iter().all(|m| m.method() != "workspace/applyEdit"));
}

#[tokio::test]
async fn export_replaces_existing_closed_file_without_truncating_it_first() {
    let dir = std::env::temp_dir().join(format!(
        "terraphim-export-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let source = Url::from_file_path(dir.join("source.md")).unwrap();
    let target = dir.join("source.clean.md");
    let previous = "Older 😀 text.\r\nSecond line.";
    std::fs::write(&target, previous).unwrap();
    let mut harness = Harness::start(ZED, json!({}), source.clone(), &exportable()).await;
    let (server, client) = (harness.service.inner(), &mut harness.client);
    let run = server.execute_command(ExecuteCommandParams {
        command: commands::EXPORT.into(),
        arguments: vec![json!({"uri":source,"version":1})],
        work_done_progress_params: Default::default(),
    });
    let respond = async {
        let request = client.next("workspace/applyEdit").await;
        let params: ApplyWorkspaceEditParams =
            serde_json::from_value(request.params().cloned().unwrap()).unwrap();
        let edits = match params.edit.document_changes.unwrap() {
            DocumentChanges::Edits(edits) => edits,
            DocumentChanges::Operations(ops) => ops
                .into_iter()
                .map(|op| match op {
                    DocumentChangeOperation::Edit(edit) => edit,
                    _ => panic!("no create/truncate for existing file"),
                })
                .collect(),
        };
        assert_eq!(edits.len(), 1);
        let edit = &edits[0];
        assert_eq!(edit.text_document.version, None);
        let plain: Vec<_> = edit
            .edits
            .iter()
            .map(|e| match e {
                OneOf::Left(e) => e.clone(),
                _ => panic!(),
            })
            .collect();
        let actual = apply_edits_lsp(previous, &plain);
        assert_eq!(actual, "Keep this.");
        std::fs::write(&target, &actual).unwrap();
        client.reply(&request, json!({"applied":true})).await;
        client
            .answer("window/showDocument", json!({"success":true}))
            .await;
    };
    let (result, ()) = tokio::join!(run, respond);
    assert_eq!(result.unwrap().unwrap()["written"], true);
    assert_eq!(std::fs::read_to_string(&target).unwrap(), "Keep this.");
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn graph_and_clean_exports_protect_passive_json_hard_links() {
    for language in ["json", "jsonc"] {
        assert_export_alias_refused(
            |source, target| std::fs::hard_link(source, target).unwrap(),
            true,
            language,
        )
        .await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn graph_and_clean_exports_protect_passive_json_symlinks() {
    for language in ["json", "jsonc"] {
        assert_export_alias_refused(
            |source, target| std::os::unix::fs::symlink(source, target).unwrap(),
            true,
            language,
        )
        .await;
    }
}

#[test]
fn exports_refuse_target_lifecycle_changes_during_file_inspection() {
    // The real filesystem inspection is delayed by occupying its blocking pool.
    // Notifications run after the actual target snapshot and before dispatch.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap()
        .block_on(async {
            for (command, suffix) in [
                (commands::EXPORT_KNOWLEDGE_GRAPH, "knowledge-graph.json"),
                (commands::EXPORT, "clean.md"),
            ] {
                for change in ["edit", "close", "reopen", "open", "kind"] {
                    let directory = tempfile::tempdir().unwrap();
                    let source_path = directory.path().join("brief.md");
                    let source = Url::from_file_path(&source_path).unwrap();
                    let original = authored_graph_document();
                    std::fs::write(&source_path, &original).unwrap();
                    let target_path = directory.path().join(format!("brief.{suffix}"));
                    let target = Url::from_file_path(&target_path).unwrap();
                    let previous = "{\"user\":\"preserve this output\"}\n";
                    std::fs::write(&target_path, previous).unwrap();
                    let mut harness = Harness::start(
                        ZED,
                        json!({"lab":{"trigger":"command"}}),
                        source.clone(),
                        &original,
                    )
                    .await;
                    let (server, client) = (harness.service.inner(), &mut harness.client);
                    if change != "open" {
                        server
                            .did_open(DidOpenTextDocumentParams {
                                text_document: TextDocumentItem {
                                    uri: target.clone(),
                                    language_id: "json".into(),
                                    version: 1,
                                    text: previous.into(),
                                },
                            })
                            .await;
                    }
                    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
                    let (release_tx, release_rx) = std::sync::mpsc::channel();
                    let occupying = tokio::task::spawn_blocking(move || {
                        entered_tx.send(()).unwrap();
                        let _ = release_rx.recv();
                    });
                    entered_rx.await.unwrap();
                    let mut run = Box::pin(server.execute_command(ExecuteCommandParams {
                        command: command.into(),
                        arguments: vec![json!({"uri":source,"version":1})],
                        work_done_progress_params: Default::default(),
                    }));
                    assert!(futures::poll!(&mut run).is_pending());
                    if change == "edit" {
                        server
                            .did_change(DidChangeTextDocumentParams {
                                text_document: VersionedTextDocumentIdentifier {
                                    uri: target.clone(),
                                    version: 2,
                                },
                                content_changes: vec![TextDocumentContentChangeEvent {
                                    range: None,
                                    range_length: None,
                                    text: previous.into(),
                                }],
                            })
                            .await;
                    } else {
                        if change != "open" {
                            server
                                .did_close(DidCloseTextDocumentParams {
                                    text_document: TextDocumentIdentifier {
                                        uri: target.clone(),
                                    },
                                })
                                .await;
                        }
                        if change != "close" {
                            server
                                .did_open(DidOpenTextDocumentParams {
                                    text_document: TextDocumentItem {
                                        uri: target.clone(),
                                        language_id: if change == "kind" {
                                            "jsonc"
                                        } else {
                                            "json"
                                        }
                                        .into(),
                                        version: 1,
                                        text: previous.into(),
                                    },
                                })
                                .await;
                        }
                    }
                    release_tx.send(()).unwrap();
                    occupying.await.unwrap();
                    let result = tokio::time::timeout(Duration::from_secs(5), run)
                        .await
                        .expect("refusal requires no client applyEdit")
                        .unwrap()
                        .unwrap();
                    assert_eq!(result["written"], false, "{command} {change}");
                    assert_eq!(std::fs::read_to_string(&target_path).unwrap(), previous);
                    assert_eq!(std::fs::read_to_string(&source_path).unwrap(), original);
                    assert!(
                        client
                            .quiet()
                            .await
                            .iter()
                            .all(|request| request.method() != "workspace/applyEdit")
                    );
                }
            }
        });
}
