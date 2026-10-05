//! Behaviour of the KG core against a real, committed fixture thesaurus.
//!
//! No mocks: every test loads `tests/fixtures/writing_thesaurus.json` through
//! `terraphim_automata`'s own loader and runs the real matcher and concept
//! index.

use terraphim_lsp_core::{
    AlternativeSet, Capitalisation, DiagnosticCode, KgEngine, TermMatch, Thesaurus, apply_edits,
};

const THESAURUS_JSON: &str = include_str!("fixtures/writing_thesaurus.json");
const SAMPLE_DOC: &str = include_str!("fixtures/alternatives_doc.md");

fn engine() -> KgEngine {
    KgEngine::from_json(THESAURUS_JSON).expect("fixture thesaurus loads")
}

/// The alternatives for the first occurrence of `needle` in `text`.
fn alternatives_of(engine: &KgEngine, text: &str, needle: &str) -> AlternativeSet {
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in {text:?}"));
    engine
        .alternatives_at(text, start)
        .unwrap_or_else(|| panic!("{needle:?} should be a KG term in {text:?}"))
}

fn offered(set: &AlternativeSet) -> Vec<&str> {
    set.replacements.iter().map(|r| r.text.as_str()).collect()
}

/// Apply the replacement whose text is `choice`.
fn replace_with(text: &str, set: &AlternativeSet, choice: &str) -> String {
    let replacement = set
        .replacements
        .iter()
        .find(|r| r.text == choice)
        .unwrap_or_else(|| panic!("{choice:?} not offered: {:?}", offered(set)));
    apply_edits(text, &replacement.edits)
}

fn utf16_slice(text: &str, start: usize, end: usize) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[start..end]).unwrap()
}

#[test]
fn fixture_round_trips_through_the_types_serde() {
    let thesaurus: Thesaurus =
        terraphim_automata::load_thesaurus_from_json(THESAURUS_JSON).unwrap();
    assert_eq!(thesaurus.len(), 15);
    let reserialised = serde_json::to_string(&thesaurus).unwrap();
    let reloaded = terraphim_automata::load_thesaurus_from_json(&reserialised).unwrap();
    assert_eq!(reloaded, thesaurus);
    assert!(engine().skipped_patterns().is_empty());
}

#[test]
fn analysis_keeps_concept_id_and_nterm() {
    let analysis = engine().analyse(SAMPLE_DOC);
    let found: Vec<(&str, &str, u64)> = analysis
        .matches
        .iter()
        .map(|m| (m.text.as_str(), m.nterm.as_str(), m.concept_id))
        .collect();
    assert_eq!(
        found,
        [
            ("choice", "decision", 1),
            ("judgment", "decision", 1),
            ("choice", "decision", 1),
            ("café", "café", 4),
            ("honour", "honour", 5),
        ]
    );
    assert!(analysis.diagnostics.is_empty());
    assert!(analysis.block.is_none());
    assert_eq!(analysis.body_end.byte, SAMPLE_DOC.len());
}

#[test]
fn analysis_reports_term_key_description_and_url() {
    let analysis = engine().analyse("I write Rust daily.");
    let rust: &TermMatch = &analysis.matches[0];
    assert_eq!(rust.term, "rust");
    assert_eq!(rust.text, "Rust");
    assert_eq!(rust.nterm, "rust programming language");
    assert_eq!(
        rust.description.as_deref(),
        Some("Rust programming language")
    );
    assert_eq!(rust.url.as_deref(), Some("https://rust-lang.org"));
}

#[test]
fn utf16_offsets_are_correct_for_multi_byte_and_astral_text() {
    // '😀' is 4 bytes / 2 UTF-16 units, 'é' 2 bytes / 1 unit, '—' 3 bytes / 1 unit.
    let text = "😀 Café — a choice";
    let analysis = engine().analyse(text);
    assert_eq!(analysis.matches.len(), 2);

    let cafe = &analysis.matches[0];
    assert_eq!(cafe.text, "Café");
    assert_eq!((cafe.range.start.byte, cafe.range.end.byte), (5, 10));
    assert_eq!((cafe.range.start.utf16, cafe.range.end.utf16), (3, 7));
    assert_eq!(utf16_slice(text, 3, 7), "Café");

    let choice = &analysis.matches[1];
    assert_eq!(&text[choice.range.bytes()], "choice");
    assert_eq!((choice.range.start.utf16, choice.range.end.utf16), (12, 18));
    assert_eq!(utf16_slice(text, 12, 18), "choice");
    assert_eq!(analysis.body_end.utf16, text.encode_utf16().count());
}

#[test]
fn replacement_edits_carry_utf16_offsets() {
    let text = "😀 an eraser";
    let set = alternatives_of(&engine(), text, "eraser");
    let edits = &set.replacements[0].edits;
    assert_eq!(edits.len(), 2);
    assert_eq!(
        (edits[0].range.start.utf16, edits[0].range.end.utf16),
        (3, 5)
    );
    assert_eq!(
        (edits[1].range.start.utf16, edits[1].range.end.utf16),
        (6, 12)
    );
    assert_eq!(replace_with(text, &set, "rubber"), "😀 a rubber");
}

#[test]
fn alternatives_exclude_the_current_form() {
    let engine = engine();
    let set = alternatives_of(&engine, "a choice", "choice");
    assert_eq!(set.term.concept_id, 1);
    assert_eq!(offered(&set), ["decision", "judgment", "option"]);

    let set = alternatives_of(&engine, "the decision", "decision");
    assert_eq!(offered(&set), ["choice", "judgment", "option"]);
}

#[test]
fn single_term_concept_offers_nothing() {
    let set = alternatives_of(&engine(), "a paperclip", "paperclip");
    assert!(set.replacements.is_empty());
}

#[test]
fn lookup_by_utf16_offset_and_cursor_after_the_word() {
    let engine = engine();
    let text = "😀 café";
    // UTF-16 offset 3 is the 'c' of "café" (the emoji is two units).
    let set = engine.alternatives_at_utf16(text, 3).unwrap();
    assert_eq!(set.term.text, "café");
    // A cursor just after the word still finds it.
    assert!(engine.alternatives_at(text, text.len()).is_some());
    assert!(engine.alternatives_at(text, 1).is_none());
}

#[test]
fn capitalisation_is_preserved() {
    let engine = engine();
    let cases = [
        (
            "Choice matters.",
            "Choice",
            "Judgment matters.",
            Capitalisation::Sentence,
        ),
        (
            "CHOICE matters.",
            "CHOICE",
            "JUDGMENT matters.",
            Capitalisation::Upper,
        ),
        (
            "the choice matters",
            "choice",
            "the judgment matters",
            Capitalisation::Lower,
        ),
    ];
    for (text, needle, expected, case) in cases {
        let set = alternatives_of(&engine, text, needle);
        assert_eq!(set.capitalisation, case, "{text}");
        assert_eq!(
            apply_edits(text, &set.replacements[1].edits),
            expected,
            "{text}"
        );
    }
}

#[test]
fn title_case_capitalises_every_word() {
    let set = alternatives_of(&engine(), "Meet at the Coffee Shop", "Coffee Shop");
    assert_eq!(set.capitalisation, Capitalisation::Title);
    assert_eq!(offered(&set), ["Café", "Coffeehouse"]);
}

#[test]
fn display_form_survives_a_lowercase_original() {
    let set = alternatives_of(&engine(), "a large language model", "large language model");
    assert_eq!(offered(&set), ["LLM"]);
}

#[test]
fn article_switches_from_a_to_an_and_back() {
    let engine = engine();
    let text = "It was a choice.";
    let set = alternatives_of(&engine, text, "choice");
    let swapped = replace_with(text, &set, "option");
    assert_eq!(swapped, "It was an option.");

    let back = alternatives_of(&engine, &swapped, "option");
    assert_eq!(replace_with(&swapped, &back, "choice"), text);
}

#[test]
fn article_at_sentence_start_keeps_its_capital() {
    let engine = engine();
    let text = "A choice was made.";
    let set = alternatives_of(&engine, text, "choice");
    assert_eq!(replace_with(text, &set, "option"), "An option was made.");

    let text = "An eraser helps.";
    let set = alternatives_of(&engine, text, "eraser");
    assert_eq!(replace_with(text, &set, "rubber"), "A rubber helps.");

    let text = "An honour.";
    let set = alternatives_of(&engine, text, "honour");
    assert_eq!(replace_with(text, &set, "privilege"), "A privilege.");
}

#[test]
fn unchanged_article_produces_a_single_edit() {
    let text = "a choice";
    let set = alternatives_of(&engine(), text, "choice");
    let judgment = set
        .replacements
        .iter()
        .find(|r| r.text == "judgment")
        .unwrap();
    assert_eq!(judgment.edits.len(), 1);
    assert_eq!(apply_edits(text, &judgment.edits), "a judgment");
}

#[test]
fn edits_are_sorted_and_non_overlapping() {
    let text = "Here is an eraser and a choice.";
    let engine = engine();
    for needle in ["eraser", "choice"] {
        for replacement in alternatives_of(&engine, text, needle).replacements {
            for pair in replacement.edits.windows(2) {
                assert!(pair[0].range.end.byte <= pair[1].range.start.byte);
            }
        }
    }
}

const WELL_FORMED_BLOCK: &str = "```terraphim-alternatives\n{\"version\": 1, \"spans\": [{\"alts\": [\"choice\", \"judgment\"]}]}\n```\n";

#[test]
fn matches_inside_the_annotation_block_are_ignored() {
    let engine = engine();
    let body = "A choice.";
    let text = format!("{body}\n\n{WELL_FORMED_BLOCK}");
    let analysis = engine.analyse(&text);
    assert_eq!(analysis.matches.len(), 1);
    assert_eq!(analysis.matches[0].range.start.byte, 2);
    assert_eq!(&text[..analysis.body_end.byte], body);
    assert!(analysis.diagnostics.is_empty());
    let block = analysis.block.expect("block located");
    assert!(block.problem.is_none());

    let inside_block = text.rfind("judgment").unwrap();
    assert!(engine.alternatives_at(&text, inside_block).is_none());
}

#[test]
fn malformed_block_yields_one_diagnostic_and_the_body_is_unchanged() {
    let engine = engine();
    let body = "A choice is a judgment.";
    let malformed = [
        (
            format!(
                "{body}\n\n```terraphim-alternatives\n{{\"version\": 1, \"spans\": [\"choice\"]}}\n"
            ),
            DiagnosticCode::AnnotationBlockTruncated,
        ),
        (
            format!("{body}\n\n{WELL_FORMED_BLOCK}a choice after the block\n"),
            DiagnosticCode::AnnotationBlockNotTrailing,
        ),
        (
            format!("{body}\n\n```terraphim-alternatives\n{{\"version\": 1, choice\n```\n"),
            DiagnosticCode::AnnotationBlockInvalidJson,
        ),
    ];
    let body_only = engine.analyse(body);
    for (text, code) in malformed {
        let analysis = engine.analyse(&text);
        assert_eq!(analysis.diagnostics.len(), 1, "{text}");
        assert_eq!(analysis.diagnostics[0].code, code, "{text}");
        // The body is exactly the text before the block, analysed as if the
        // block were absent; nothing in or after the block is matched.
        assert_eq!(&text[..analysis.body_end.byte], body, "{text}");
        assert_eq!(analysis.matches, body_only.matches, "{text}");
    }
}

#[test]
fn alternatives_for_rejects_a_stale_match() {
    let engine = engine();
    let text = "a choice";
    let term = engine.analyse(text).matches.remove(0);
    assert!(engine.alternatives_for(text, &term).is_some());
    assert!(engine.alternatives_for("an option", &term).is_none());
}

#[test]
fn unusable_patterns_are_skipped_not_fatal() {
    let json = r#"{"name": "short", "data": {
        "x": {"id": 1, "nterm": "ex"},
        "ex": {"id": 1, "nterm": "ex"}
    }}"#;
    let engine = KgEngine::from_json(json).unwrap();
    assert_eq!(engine.skipped_patterns(), ["x"]);
    let set = alternatives_of(&engine, "an ex", "ex");
    assert_eq!(offered(&set), ["x"], "short keys may still be offered");
}

#[test]
fn empty_thesaurus_finds_nothing() {
    for engine in [
        KgEngine::new(&Thesaurus::new("empty".to_string())).unwrap(),
        KgEngine::empty(),
    ] {
        assert!(engine.analyse("a choice").matches.is_empty());
        assert!(engine.alternatives_at("a choice", 3).is_none());
    }
}

#[test]
fn invalid_thesaurus_json_is_an_error() {
    assert!(KgEngine::from_json("{not json").is_err());
}

#[test]
fn analysis_serialises_as_plain_json() {
    let analysis = engine().analyse("😀 a choice");
    let json = serde_json::to_value(&analysis).unwrap();
    assert_eq!(json["matches"][0]["concept_id"], 1);
    assert_eq!(json["matches"][0]["range"]["start"]["utf16"], 5);
    let back: terraphim_lsp_core::Analysis = serde_json::from_value(json).unwrap();
    assert_eq!(back, analysis);
}
