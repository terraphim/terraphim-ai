//! Terraphim Graph Embeddings + Graph Ranking (RoleGraph) - "Learnings" Demo
//!
//! This example is intentionally end-to-end:
//! - Build a thesaurus from a Markdown knowledge graph (`synonyms::` directives).
//! - Index "learnings" notes into a `RoleGraph` (co-occurrence graph).
//! - Query and rank results.
//! - Add a *new KG term* and show how rankings/retrieval change.
//!
//! Run:
//!   cargo run -p terraphim_rolegraph --example terraphim_graph_embeddings_learnings
//!
//! Notes:
//! - This example uses the repo's sample files under `examples/learnings_kg/` and
//!   `examples/learnings_docs/`, but operates on a temporary copy so it can add
//!   a KG term without modifying your working tree.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use terraphim_automata::builder::{Logseq, ThesaurusBuilder};
use terraphim_rolegraph::RoleGraph;
use terraphim_types::{Document, DocumentType, LogicalOperator, RoleName};

use tokio::fs;

const NEW_KG_TERM_FILE_NAME: &str = "learnings.md";
const NEW_KG_TERM_CONTENT: &str = r#"# Learnings

This concept captures terms used to store and retrieve lessons learned from
incidents, experiments, and deep work.

synonyms:: learnings, lessons learned, postmortem, retrospective, after action review, action items

When this term is present in the knowledge graph, searches for "learnings" or
"postmortem" will map to the same concept, improving retrieval for learnings
notes that use different phrasing.
"#;

fn repo_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` = `.../crates/terraphim_rolegraph`
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

async fn copy_markdown_files(src_dir: &Path, dst_dir: &Path) -> std::io::Result<usize> {
    fs::create_dir_all(dst_dir).await?;

    let mut copied = 0usize;
    let mut entries = fs::read_dir(src_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            fs::copy(&path, dst_dir.join(entry.file_name())).await?;
            copied += 1;
        }
    }

    Ok(copied)
}

fn title_from_markdown(stem: &str, body: &str) -> String {
    for line in body.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("# ") {
            let title = rest.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    // Reasonable fallback for printing.
    stem.replace('-', " ")
}

async fn load_documents_from_dir(docs_dir: &Path) -> std::io::Result<HashMap<String, Document>> {
    let mut docs = HashMap::new();

    let mut entries = fs::read_dir(docs_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.extension().is_some_and(|ext| ext == "md") {
            continue;
        }

        let body = fs::read_to_string(&path).await?;
        let stem = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let doc = Document {
            id: stem.clone(),
            url: path.to_string_lossy().to_string(),
            title: title_from_markdown(&stem, &body),
            body,
            description: None,
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: Some("examples/learnings_docs".to_string()),
            doc_type: DocumentType::Document,
            synonyms: None,
            route: None,
            priority: None,
        };

        docs.insert(stem, doc);
    }

    Ok(docs)
}

async fn build_rolegraph(
    role_name: &RoleName,
    kg_dir: &Path,
    docs: &HashMap<String, Document>,
) -> Result<RoleGraph, Box<dyn std::error::Error>> {
    let logseq = Logseq::default();
    let thesaurus = logseq
        .build(role_name.as_lowercase().to_string(), kg_dir.to_path_buf())
        .await?;

    let mut rolegraph = RoleGraph::new(role_name.clone(), thesaurus).await?;

    for doc in docs.values() {
        rolegraph.insert_document(&doc.id, doc.clone());
    }

    Ok(rolegraph)
}

fn print_stats(label: &str, rolegraph: &RoleGraph) {
    let stats = rolegraph.get_graph_stats();
    println!("{label}:");
    println!("  thesaurus terms: {}", stats.thesaurus_size);
    println!("  nodes: {}", stats.node_count);
    println!("  edges: {}", stats.edge_count);
    println!("  indexed documents: {}", stats.document_count);
}

fn print_query(
    label: &str,
    rolegraph: &RoleGraph,
    docs: &HashMap<String, Document>,
    query: &str,
    limit: usize,
) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    println!("\n{label} query: \"{query}\"");
    let results = rolegraph.query_graph(query, Some(0), Some(limit))?;

    if results.is_empty() {
        println!("  (no results)");
        return Ok(vec![]);
    }

    let mut out = Vec::with_capacity(results.len());
    for (i, (doc_id, indexed_doc)) in results.iter().enumerate() {
        let title = docs
            .get(doc_id)
            .map(|d| d.title.as_str())
            .unwrap_or(doc_id.as_str());
        println!(
            "  {:>2}. {} (id: {}, rank: {})",
            i + 1,
            title,
            doc_id,
            indexed_doc.rank
        );
        out.push((doc_id.clone(), indexed_doc.rank));
    }

    Ok(out)
}

fn rank_for(results: &[(String, u64)], doc_id: &str) -> Option<u64> {
    results
        .iter()
        .find(|(id, _)| id.as_str() == doc_id)
        .map(|(_, rank)| *rank)
}

fn print_and_query(
    label: &str,
    rolegraph: &RoleGraph,
    docs: &HashMap<String, Document>,
    terms: &[&str],
    limit: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{label} AND query: {:?}", terms);
    let results =
        rolegraph.query_graph_with_operators(terms, &LogicalOperator::And, Some(0), Some(limit))?;

    if results.is_empty() {
        println!("  (no results)");
        return Ok(());
    }

    for (i, (doc_id, indexed_doc)) in results.iter().enumerate() {
        let title = docs
            .get(doc_id)
            .map(|d| d.title.as_str())
            .unwrap_or(doc_id.as_str());
        println!(
            "  {:>2}. {} (id: {}, rank: {})",
            i + 1,
            title,
            doc_id,
            indexed_doc.rank
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = repo_root();
    let base_kg = root.join("examples/learnings_kg");
    let base_docs = root.join("examples/learnings_docs");

    if !base_kg.exists() {
        return Err(format!(
            "Missing example knowledge graph directory: {}",
            base_kg.display()
        )
        .into());
    }
    if !base_docs.exists() {
        return Err(format!(
            "Missing example learnings docs directory: {}",
            base_docs.display()
        )
        .into());
    }

    let temp_dir = tempfile::TempDir::new()?;
    let temp_kg = temp_dir.path().join("kg");
    let temp_docs = temp_dir.path().join("docs");

    let copied_kg = copy_markdown_files(&base_kg, &temp_kg).await?;
    let copied_docs = copy_markdown_files(&base_docs, &temp_docs).await?;
    if copied_kg == 0 {
        return Err(format!("No markdown KG files found in {}", base_kg.display()).into());
    }
    if copied_docs == 0 {
        return Err(format!("No markdown learning docs found in {}", base_docs.display()).into());
    }

    let docs = load_documents_from_dir(&temp_docs).await?;
    let role_name = RoleName::new("Learning Assistant");

    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║ Terraphim Graph Ranking Demo: Learnings Role + KG Expansion       ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
    println!("Role: {}", role_name.as_str());
    println!("KG (base): {}", base_kg.display());
    println!("Docs:      {}", base_docs.display());

    // BEFORE
    println!("\n== 1) BEFORE adding new KG term ==");
    let rolegraph_before = build_rolegraph(&role_name, &temp_kg, &docs).await?;
    print_stats("Graph stats (before)", &rolegraph_before);

    let before_learnings = print_query("Before", &rolegraph_before, &docs, "learnings", 5)?;
    let _before_postmortem = print_query("Before", &rolegraph_before, &docs, "postmortem", 5)?;
    let before_raft = print_query("Before", &rolegraph_before, &docs, "raft", 5)?;
    print_and_query(
        "Before",
        &rolegraph_before,
        &docs,
        &["postmortem", "raft"],
        5,
    )?;

    // Add a KG term, then rebuild (simulates "restart" / "rebuild automata")
    println!("\n== 2) Add KG term and rebuild ==");
    let new_term_path = temp_kg.join(NEW_KG_TERM_FILE_NAME);
    fs::write(&new_term_path, NEW_KG_TERM_CONTENT).await?;
    println!("Added KG term: {}", new_term_path.display());

    // AFTER
    println!("\n== 3) AFTER adding new KG term ==");
    let rolegraph_after = build_rolegraph(&role_name, &temp_kg, &docs).await?;
    print_stats("Graph stats (after)", &rolegraph_after);

    let after_learnings = print_query("After", &rolegraph_after, &docs, "learnings", 5)?;
    let _after_postmortem = print_query("After", &rolegraph_after, &docs, "postmortem", 5)?;
    let after_raft = print_query("After", &rolegraph_after, &docs, "raft", 5)?;
    print_and_query("After", &rolegraph_after, &docs, &["postmortem", "raft"], 5)?;

    // Comparison (show the specific change the user cares about)
    println!("\n== 4) What changed? ==");
    println!(
        "Query \"learnings\": {} results -> {} results",
        before_learnings.len(),
        after_learnings.len()
    );
    if before_learnings.is_empty() && !after_learnings.is_empty() {
        println!("  ✓ Retrieval improved: \"learnings\" started returning documents.");
    }

    // Show a concrete rank change for a specific note.
    let target_doc = "incident-retrospective-raft";
    let before_target_rank = rank_for(&before_raft, target_doc);
    let after_target_rank = rank_for(&after_raft, target_doc);

    match (before_target_rank, after_target_rank) {
        (Some(b), Some(a)) if a != b => {
            println!(
                "Query \"raft\": rank({target_doc}) changed {b} -> {a} ({:+})",
                a as i64 - b as i64
            );
        }
        (Some(b), Some(a)) => {
            println!("Query \"raft\": rank({target_doc}) stayed {b} (after: {a})");
        }
        _ => {
            println!(
                "Query \"raft\": could not compare rank for doc id \"{target_doc}\" (is the file present?)"
            );
        }
    }

    println!("\nDone.");
    Ok(())
}
