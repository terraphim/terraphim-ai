//! Lab-mark diagnostics and trim candidates from terraphim-editor's own Lab
//! engine (`terraphim_lab`), on a committed fixture document.
//!
//! No mocks: the engine runs with its embedded default lists
//! (`LabConfig::with_defaults`), exactly as the editor runs it without a
//! role selected.

use terraphim_lsp_core::{
    DiagnosticCode, DiagnosticTag, LabAction, LabConfig, LineIndex, Severity, TrimLevel,
    apply_edits, lab_findings, split_annotation_block, trim_preview,
};

const LAB_DOC: &str = include_str!("fixtures/lab_doc.md");

fn config() -> LabConfig {
    LabConfig::with_defaults().expect("embedded Lab lists load")
}

fn codes_and_texts(text: &str, actions: &[LabAction]) -> Vec<(DiagnosticCode, String)> {
    lab_findings(text, &config(), actions)
        .into_iter()
        .map(|f| {
            (
                f.diagnostic.code,
                text[f.diagnostic.range.bytes()].to_string(),
            )
        })
        .collect()
}

#[test]
fn one_code_per_mark_kind_with_the_engine_reason() {
    let found = lab_findings(LAB_DOC, &config(), &LabAction::ALL);
    let codes: Vec<DiagnosticCode> = found.iter().map(|f| f.diagnostic.code).collect();
    for expected in [
        DiagnosticCode::LabTypo,
        DiagnosticCode::LabPunctuation,
        DiagnosticCode::LabFiller,
        DiagnosticCode::LabHedge,
        DiagnosticCode::LabWeakSentence,
        DiagnosticCode::LabLongSentence,
        DiagnosticCode::LabConvolutedSentence,
    ] {
        assert!(codes.contains(&expected), "{expected:?} in {codes:?}");
    }
    let typo = &found[0];
    assert_eq!(typo.diagnostic.message, "typo: \"recieve\" -> \"receive\"");
    assert_eq!(typo.diagnostic.severity, Severity::Information);
    assert_eq!(typo.action, LabAction::TyposAndPunctuation);
    assert!(typo.diagnostic.tags.is_empty());
    let hedge = found
        .iter()
        .find(|f| f.diagnostic.code == DiagnosticCode::LabHedge)
        .unwrap();
    assert_eq!(hedge.diagnostic.severity, Severity::Hint);
    assert!(hedge.fix.is_none());
}

#[test]
fn ranges_are_utf16_and_skip_the_annotation_block() {
    let found = lab_findings(LAB_DOC, &config(), &LabAction::ALL);
    let body_end = split_annotation_block(LAB_DOC).body_end.byte;
    // The block's overflow has a typo and filler of its own: never marked.
    assert!(
        found
            .iter()
            .all(|f| f.diagnostic.range.end.byte <= body_end)
    );
    // "basically" follows "café" (2 UTF-8 bytes, 1 UTF-16 unit).
    let filler = found
        .iter()
        .find(|f| f.diagnostic.code == DiagnosticCode::LabFiller)
        .unwrap();
    let range = filler.diagnostic.range;
    assert_eq!(&LAB_DOC[range.bytes()], "basically");
    assert_eq!(range.end.byte - range.start.byte, 9);
    assert_eq!(
        range.start.byte - range.start.utf16,
        1,
        "one two-byte char before"
    );
}

#[test]
fn fixes_replace_exactly_the_marked_range() {
    let found = lab_findings(LAB_DOC, &config(), &[LabAction::TyposAndPunctuation]);
    let titles: Vec<&str> = found
        .iter()
        .map(|f| f.fix.as_ref().unwrap().title.as_str())
        .collect();
    assert_eq!(titles, ["Apply fix: receive", "Apply fix: ,"]);
    let fixed = apply_edits(LAB_DOC, &[found[0].fix.clone().unwrap().edit]);
    assert!(fixed.contains("We receive the café report ,and"));
    // The fix edits nothing but its range: no a/an fix-up is added.
    assert_eq!(fixed.len(), LAB_DOC.len());
}

#[test]
fn one_action_marks_only_its_kinds() {
    let marked = codes_and_texts(LAB_DOC, &[LabAction::HedgesAndFiller]);
    assert_eq!(
        marked,
        [
            (DiagnosticCode::LabFiller, "basically".to_string()),
            (DiagnosticCode::LabHedge, "I think".to_string()),
            (DiagnosticCode::LabHedge, "perhaps".to_string()),
            (DiagnosticCode::LabFiller, "quite".to_string()),
            (DiagnosticCode::LabFiller, "really".to_string()),
        ]
    );
}

#[test]
fn crlf_documents_map_to_the_same_text() {
    let crlf = LAB_DOC.replace('\n', "\r\n");
    assert_eq!(
        codes_and_texts(&crlf, &LabAction::ALL),
        codes_and_texts(LAB_DOC, &LabAction::ALL)
    );
    let found = lab_findings(&crlf, &config(), &[LabAction::HedgesAndFiller]);
    let index = LineIndex::new(&crlf);
    let really = found.last().unwrap().diagnostic.range;
    assert_eq!(&crlf[really.bytes()], "really");
    let end = index.position(really.end.byte);
    assert_eq!(end.line, 2);
}

#[test]
fn trim_candidates_are_faded_hints_nested_by_level() {
    let slight = trim_preview(LAB_DOC, &config(), TrimLevel::Slight);
    let sharper = trim_preview(LAB_DOC, &config(), TrimLevel::Sharper);
    assert_eq!(slight.level, TrimLevel::Slight);
    assert_eq!(slight.status, "61 \u{2192} 55 words \u{b7} \u{2212}10%");
    for d in slight.diagnostics.iter().chain(&sharper.diagnostics) {
        assert_eq!(d.code, DiagnosticCode::TrimCandidate);
        assert_eq!(d.severity, Severity::Hint);
        assert_eq!(d.tags, [DiagnosticTag::Unnecessary]);
    }
    let texts = |preview: &terraphim_lsp_core::TrimPreview| -> Vec<String> {
        preview
            .diagnostics
            .iter()
            .map(|d| LAB_DOC[d.range.bytes()].to_string())
            .collect()
    };
    assert_eq!(
        texts(&slight),
        [" basically", "I think ", " perhaps", " quite", ", really"]
    );
    // The weak sentence swallows the fillers inside it: only the outermost
    // cut is reported.
    assert_eq!(
        texts(&sharper),
        [
            " basically",
            " I think the plan is perhaps quite good, really.",
            ", which owns its DOM,"
        ]
    );
    assert_eq!(
        slight.diagnostics[0].message,
        "Slight trim: filler \"basically\""
    );
    assert!(
        trim_preview(LAB_DOC, &config(), TrimLevel::Original)
            .diagnostics
            .is_empty()
    );
}
