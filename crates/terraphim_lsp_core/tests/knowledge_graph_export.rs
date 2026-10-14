//! Real directive/type registration, persistence and ambiguity checks.
use terraphim_lsp_core::{TextRange, add_alternative, apply_edits, export_knowledge_graph};

fn add(source: &str, original: &str, alternative: &str) -> String {
    let start = source.find(original).unwrap();
    let result = add_alternative(
        source,
        TextRange::from_bytes(source, start, start + original.len()),
        alternative,
        None,
    )
    .unwrap();
    apply_edits(source, &[result.edit])
}

#[test]
fn actual_registration_saved_reopen_and_history_snapshots() {
    let original = "Risk: delivery may slip.\n";
    let saved = add(original, "Risk", "Concern");
    let document = terraphim_alternatives::parse(&saved).unwrap();
    assert_eq!(
        document.synonym_groups()[0].markdown,
        "synonyms:: Risk, Concern\n"
    );
    let graph = export_knowledge_graph(&saved).unwrap();
    assert_eq!(
        graph.get(&"risk".into()).unwrap().id,
        graph.get(&"concern".into()).unwrap().id
    );
    assert!(export_knowledge_graph(original).unwrap().is_empty()); // native undo source
    assert_eq!(export_knowledge_graph(&saved).unwrap(), graph); // native redo source
    assert_eq!(
        export_knowledge_graph(&terraphim_alternatives::write(&document)).unwrap(),
        graph
    );
    assert_eq!(document.body, original);
}

#[test]
fn commas_are_typed_synonyms_and_ambiguous_groups_are_refused() {
    let saved = add("Risk. Decision.", "Risk", "Risk, exposure");
    let graph = export_knowledge_graph(&saved).unwrap();
    assert!(graph.get(&"risk, exposure".into()).is_some());
    assert!(graph.get(&"exposure".into()).is_none());
    let conflict = add(&saved, "Decision", "Risk");
    assert!(export_knowledge_graph(&conflict).is_err());
}

#[test]
fn malformed_or_detached_annotations_cannot_register_terms() {
    assert!(export_knowledge_graph("Risk\n\n```terraphim-alternatives\n{invalid}\n```\n").is_err());
    let saved = add("Risk", "Risk", "Concern");
    let detached = saved.replacen("Risk\n", "Unrelated\n", 1);
    assert!(export_knowledge_graph(&detached).unwrap().is_empty());
}
