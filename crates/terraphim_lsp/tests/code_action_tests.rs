//! `textDocument/codeAction` "Replace with X" through the tower-lsp service.
//!
//! No mocks: the server is built from the real fixture thesaurus committed
//! with `terraphim_lsp_core` and driven through `LanguageServer` directly, as
//! in `lsp_integration_tests.rs`.

use tower_lsp::lsp_types::*;
use tower_lsp::{ClientSocket, LanguageServer, LspService};

use terraphim_lsp::TerraphimLspServer;
use terraphim_lsp::core::{LineIndex, LinePosition};
use terraphim_types::Thesaurus;

const THESAURUS_JSON: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/writing_thesaurus.json");
const SAMPLE_DOC: &str =
    include_str!("../../terraphim_lsp_core/tests/fixtures/alternatives_doc.md");

fn build_service() -> (LspService<TerraphimLspServer>, ClientSocket) {
    let thesaurus: Thesaurus = serde_json::from_str(THESAURUS_JSON).expect("fixture thesaurus");
    LspService::new(move |client| TerraphimLspServer::new(client, thesaurus.clone()))
}

fn uri() -> Url {
    Url::parse("file:///tmp/alternatives.md").unwrap()
}

/// Open `text` in a fresh server and request code actions at `position`.
async fn code_actions(
    text: &str,
    position: Position,
    only: Option<Vec<CodeActionKind>>,
) -> Option<Vec<CodeAction>> {
    let (service, _socket) = build_service();
    let server = service.inner();
    server
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
    let response = server
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            range: Range {
                start: position,
                end: position,
            },
            context: CodeActionContext {
                diagnostics: vec![],
                only,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap()?;
    Some(
        response
            .into_iter()
            .map(|action| match action {
                CodeActionOrCommand::CodeAction(action) => action,
                CodeActionOrCommand::Command(command) => panic!("unexpected command {command:?}"),
            })
            .collect(),
    )
}

/// The LSP position (UTF-16 column) of the first `needle` in `text`.
fn position_of(text: &str, needle: &str) -> Position {
    let byte = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in text"));
    let LinePosition { line, character } = LineIndex::new(text).position(byte);
    Position { line, character }
}

fn titles(actions: &[CodeAction]) -> Vec<&str> {
    actions.iter().map(|a| a.title.as_str()).collect()
}

fn edits_of(action: &CodeAction) -> &[TextEdit] {
    let changes = action
        .edit
        .as_ref()
        .and_then(|edit| edit.changes.as_ref())
        .expect("workspace edit with changes");
    assert_eq!(changes.len(), 1, "one document per edit");
    &changes[&uri()]
}

/// Apply an action's edits to `text` the way an editor would.
fn apply(text: &str, action: &CodeAction) -> String {
    let index = LineIndex::new(text);
    let byte = |p: Position| {
        index.byte_offset(LinePosition {
            line: p.line,
            character: p.character,
        })
    };
    let mut out = text.to_string();
    let mut edits: Vec<&TextEdit> = edits_of(action).iter().collect();
    edits.sort_by_key(|edit| std::cmp::Reverse(byte(edit.range.start)));
    for edit in edits {
        out.replace_range(byte(edit.range.start)..byte(edit.range.end), &edit.new_text);
    }
    out
}

fn action<'a>(actions: &'a [CodeAction], title: &str) -> &'a CodeAction {
    actions
        .iter()
        .find(|a| a.title == title)
        .unwrap_or_else(|| panic!("{title:?} not in {:?}", titles(actions)))
}

#[tokio::test]
async fn initialize_advertises_code_actions() {
    let (service, _) = build_service();
    let response = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    match response.capabilities.code_action_provider {
        Some(CodeActionProviderCapability::Options(options)) => {
            assert_eq!(
                options.code_action_kinds,
                Some(vec![CodeActionKind::REFACTOR_REWRITE])
            );
        }
        other => panic!("expected code action options, got {other:?}"),
    }
}

#[tokio::test]
async fn offers_every_other_synonym_on_the_fixture_doc() {
    let position = position_of(SAMPLE_DOC, "choice");
    let actions = code_actions(SAMPLE_DOC, position, None).await.unwrap();
    assert_eq!(
        titles(&actions),
        [
            "Replace with decision",
            "Replace with judgment",
            "Replace with option",
        ]
    );
    for action in &actions {
        assert_eq!(action.kind, Some(CodeActionKind::REFACTOR_REWRITE));
    }
    let replaced = apply(SAMPLE_DOC, action(&actions, "Replace with judgment"));
    assert!(
        replaced.contains("Every judgment is a judgment."),
        "{replaced}"
    );
}

#[tokio::test]
async fn current_form_is_excluded() {
    let text = "the decision stands";
    let actions = code_actions(text, position_of(text, "decision"), None)
        .await
        .unwrap();
    assert!(!titles(&actions).contains(&"Replace with decision"));
    assert_eq!(actions.len(), 3);
}

#[tokio::test]
async fn capitalisation_is_preserved() {
    for (text, needle, title, expected) in [
        (
            "Choice matters.",
            "Choice",
            "Replace with Judgment",
            "Judgment matters.",
        ),
        (
            "CHOICE MATTERS",
            "CHOICE",
            "Replace with JUDGMENT",
            "JUDGMENT MATTERS",
        ),
        (
            "Meet at the Coffee Shop",
            "Coffee",
            "Replace with Coffeehouse",
            "Meet at the Coffeehouse",
        ),
        (
            "the large language model",
            "large",
            "Replace with LLM",
            "the LLM",
        ),
    ] {
        let actions = code_actions(text, position_of(text, needle), None)
            .await
            .unwrap();
        assert_eq!(apply(text, action(&actions, title)), expected, "{text}");
    }
}

#[tokio::test]
async fn article_is_fixed_in_the_same_workspace_edit() {
    // "A choice" on line 2 of the fixture doc becomes "An option".
    let position = position_of(SAMPLE_DOC, "choice made");
    let actions = code_actions(SAMPLE_DOC, position, None).await.unwrap();
    let option = action(&actions, "Replace with option");
    let edits = edits_of(option);
    assert_eq!(edits.len(), 2, "article and term in one edit");
    assert_eq!(edits[0].new_text, "An");
    assert_eq!(edits[1].new_text, "option");
    let replaced = apply(SAMPLE_DOC, option);
    assert!(replaced.contains("An option made in a café"), "{replaced}");

    // And back: "an eraser" -> "a rubber".
    let text = "Pass me an eraser.";
    let actions = code_actions(text, position_of(text, "eraser"), None)
        .await
        .unwrap();
    assert_eq!(
        apply(text, action(&actions, "Replace with rubber")),
        "Pass me a rubber."
    );
}

#[tokio::test]
async fn edit_ranges_use_utf16_columns() {
    // '😀' is two UTF-16 units, so "an" starts at column 3 and "eraser" at 6.
    let text = "😀 an eraser";
    let actions = code_actions(
        text,
        Position {
            line: 0,
            character: 7,
        },
        None,
    )
    .await
    .unwrap();
    let rubber = action(&actions, "Replace with rubber");
    let edits = edits_of(rubber);
    assert_eq!(
        edits[0].range.start,
        Position {
            line: 0,
            character: 3
        }
    );
    assert_eq!(
        edits[1].range.start,
        Position {
            line: 0,
            character: 6
        }
    );
    assert_eq!(
        edits[1].range.end,
        Position {
            line: 0,
            character: 12
        }
    );
    assert_eq!(apply(text, rubber), "😀 a rubber");
}

#[tokio::test]
async fn hover_uses_utf16_columns() {
    let (service, _socket) = build_service();
    let server = service.inner();
    server
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let text = "😀😀 rust";
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
    let hover = server
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri() },
                position: Position {
                    line: 0,
                    character: 6,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap()
        .expect("hover on rust");
    assert_eq!(
        hover.range,
        Some(Range {
            start: Position {
                line: 0,
                character: 5
            },
            end: Position {
                line: 0,
                character: 9
            },
        })
    );
}

#[tokio::test]
async fn no_actions_off_a_term_inside_the_block_or_for_other_kinds() {
    let text = "plain words";
    assert!(
        code_actions(text, position_of(text, "words"), None)
            .await
            .is_none()
    );

    let text = "A choice.\n\n```terraphim-alternatives\n{\"alts\": [\"choice\"]}\n```\n";
    let inside_block = position_of(text, "choice\"");
    assert!(code_actions(text, inside_block, None).await.is_none());

    let quickfix_only = Some(vec![CodeActionKind::QUICKFIX]);
    assert!(
        code_actions(text, position_of(text, "choice"), quickfix_only)
            .await
            .is_none()
    );
    let refactor = Some(vec![CodeActionKind::REFACTOR]);
    assert!(
        code_actions(text, position_of(text, "choice"), refactor)
            .await
            .is_some()
    );
}

#[tokio::test]
async fn malformed_block_is_reported_once() {
    let (service, _socket) = build_service();
    let server = service.inner();
    server
        .initialize(InitializeParams::default())
        .await
        .unwrap();
    let text = "A choice.\n\n```terraphim-alternatives\n{\"version\": 1}\n";
    server
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri(),
                language_id: "markdown".to_string(),
                version: 1,
                text: text.to_string(),
            },
        })
        .await;
    let report = server
        .diagnostic(DocumentDiagnosticParams {
            text_document: TextDocumentIdentifier { uri: uri() },
            identifier: None,
            previous_result_id: None,
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .await
        .unwrap();
    let items = match report {
        DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(full)) => {
            full.full_document_diagnostic_report.items
        }
        other => panic!("expected full report, got {other:?}"),
    };
    let block: Vec<&Diagnostic> = items
        .iter()
        .filter(|d| d.code == Some(NumberOrString::String("annotation-block-truncated".into())))
        .collect();
    assert_eq!(block.len(), 1, "{items:?}");
    assert_eq!(
        block[0].range.start,
        Position {
            line: 2,
            character: 0
        }
    );
}
