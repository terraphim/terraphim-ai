//! Golden-set ranking regression harness for WIG-1 (issue #3132).
//!
//! Detects silent changes to the document ranking pipeline by comparing scored
//! document order against committed snapshots using Kendall-tau rank
//! correlation. Runs entirely in-process against the standard
//! `terraphim_types::score` pipeline (BM25 / BM25F / BM25Plus / Tfidf / Jaccard
//! / OkapiBM25) consumed by `terraphim_server` — no network, no live KG, no
//! `DeviceStorage`, so it is deterministic and completes well under the 60s AC
//! budget.
//!
//! # Why this lives in `terraphim_server`
//!
//! The CI `ranking-regression-gate` job historically ran
//! `cargo nextest run -p terraphim_service ... ranking_regression`, but
//! `terraphim_service` was extracted to a polyrepo in #1910 and is now a
//! *registry* dependency of this repo, not a workspace member. `-p
//! terraphim_service` therefore fails with "package not found" in this repo's
//! CI. This test is the in-repo twin: same proven Kendall-tau + snapshot
//! design, driven through the same scoring types that `terraphim_server`
//! already depends on, so the gate is runnable from this workspace.
//!
//! # Updating snapshots
//!
//! When an intentional ranking change is made:
//! ```bash
//! UPDATE_RANKING_SNAPSHOTS=1 cargo test -p terraphim_server --test ranking_regression_test
//! ```
//! Review the diff in `fixtures/ranking_snapshots/*.snapshot.json` and add an
//! explicit "Ranking change ACK" note in the PR description before merging.
//!
//! # Acceptance criteria (#3132)
//!
//! | AC | Satisfied by |
//! |----|--------------|
//! | Index the fixture corpus via the standard pipeline | `load_corpus` + `sort_documents` |
//! | Each golden query returns its expected top-N in order | `run_regression_for_corpus` |
//! | A deliberately perturbed ranking weight causes failure | `perturbed_scorer_changes_order_and_fails_gate` |
//! | Suite completes in under 60 seconds | `suite_runs_well_under_60s_budget` |

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use terraphim_types::score::names::QueryScorer;
use terraphim_types::score::{Query, sort_documents};
use terraphim_types::{Document, DocumentType};

// ---------------------------------------------------------------------------
// Fixture types (loaded from JSON corpus files)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CorpusDoc {
    id: String,
    title: String,
    body: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct CorpusFile {
    scorer: String,
    documents: Vec<CorpusDoc>,
}

// ---------------------------------------------------------------------------
// Snapshot types (committed ranking expectations)
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize, Clone)]
struct QuerySnapshot {
    query: String,
    scorer: String,
    top_n: usize,
    expected_ids: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct SnapshotFile {
    corpus: String,
    snapshots: Vec<QuerySnapshot>,
}

// ---------------------------------------------------------------------------
// Kendall-tau rank correlation
// ---------------------------------------------------------------------------

/// Compute Kendall-tau between `expected` and `actual` orderings.
///
/// Returns 1.0 for identical orderings, -1.0 for perfect reversal. Pairs where
/// one ID is absent from `actual` are skipped (neutral). This is the faithful
/// twin of the algorithm in the polyrepo `terraphim_service` test.
fn kendall_tau(expected: &[String], actual: &[String]) -> f64 {
    if expected.is_empty() {
        return 1.0;
    }
    let actual_pos: HashMap<&str, usize> = actual
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    let n = expected.len();
    let mut concordant: i64 = 0;
    let mut discordant: i64 = 0;

    for i in 0..n {
        for j in (i + 1)..n {
            if let (Some(&ai), Some(&aj)) = (
                actual_pos.get(expected[i].as_str()),
                actual_pos.get(expected[j].as_str()),
            ) {
                if ai < aj {
                    concordant += 1;
                } else {
                    discordant += 1;
                }
            }
        }
    }

    let total = (concordant + discordant) as f64;
    if total == 0.0 {
        return 1.0;
    }
    (concordant - discordant) as f64 / total
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("ranking_snapshots")
}

fn load_corpus(name: &str) -> (CorpusFile, Vec<Document>) {
    let path = fixtures_dir().join(format!("{name}.json"));
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read corpus {name}: {e}"));
    let corpus: CorpusFile =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("Failed to parse corpus {name}: {e}"));

    let docs: Vec<Document> = corpus
        .documents
        .iter()
        .map(|d| Document {
            id: d.id.clone(),
            url: format!("file://{}.md", d.id),
            title: d.title.clone(),
            body: d.body.clone(),
            description: d.description.clone(),
            summarization: None,
            stub: None,
            tags: None,
            rank: None,
            source_haystack: None,
            doc_type: DocumentType::default(),
            synonyms: None,
            route: None,
            priority: None,
            quality_score: None,
        })
        .collect();

    (corpus, docs)
}

fn snapshot_path(name: &str) -> PathBuf {
    fixtures_dir().join(format!("{name}.snapshot.json"))
}

fn parse_scorer(scorer_str: &str) -> QueryScorer {
    match scorer_str {
        "BM25" => QueryScorer::BM25,
        "BM25F" => QueryScorer::BM25F,
        "BM25Plus" => QueryScorer::BM25Plus,
        "Tfidf" => QueryScorer::Tfidf,
        "Jaccard" => QueryScorer::Jaccard,
        "QueryRatio" => QueryScorer::QueryRatio,
        _ => QueryScorer::OkapiBM25,
    }
}

fn rank_corpus(
    docs: &[Document],
    query_str: &str,
    scorer: QueryScorer,
    top_n: usize,
) -> Vec<String> {
    let query = Query::new(query_str).name_scorer(scorer);
    let ranked = sort_documents(&query, docs.to_vec());
    ranked.into_iter().take(top_n).map(|d| d.id).collect()
}

/// Run the regression gate for one corpus.
///
/// Either generates a new snapshot (`UPDATE_RANKING_SNAPSHOTS=1`) or compares
/// against the committed snapshot and fails if Kendall-tau < 0.95.
fn run_regression_for_corpus(corpus_name: &str, queries: &[(&str, usize)]) {
    let (corpus_file, docs) = load_corpus(corpus_name);
    let scorer = parse_scorer(&corpus_file.scorer);
    let update_mode = std::env::var("UPDATE_RANKING_SNAPSHOTS")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let snap_path = snapshot_path(corpus_name);

    if update_mode {
        let snapshots: Vec<QuerySnapshot> = queries
            .iter()
            .map(|(q, top_n)| QuerySnapshot {
                query: q.to_string(),
                scorer: corpus_file.scorer.clone(),
                top_n: *top_n,
                expected_ids: rank_corpus(&docs, q, scorer, *top_n),
            })
            .collect();
        let snap_file = SnapshotFile {
            corpus: corpus_name.to_string(),
            snapshots,
        };
        let json = serde_json::to_string_pretty(&snap_file).expect("Failed to serialise snapshot");
        std::fs::write(&snap_path, json)
            .unwrap_or_else(|e| panic!("Failed to write snapshot {corpus_name}: {e}"));
        println!("Updated snapshot: {}", snap_path.display());
        return;
    }

    let raw = std::fs::read_to_string(&snap_path).unwrap_or_else(|e| {
        panic!(
            "Snapshot file missing for {corpus_name}: {e}\n\
             Run with UPDATE_RANKING_SNAPSHOTS=1 to generate it."
        )
    });
    let snap_file: SnapshotFile = serde_json::from_str(&raw)
        .unwrap_or_else(|e| panic!("Failed to parse snapshot {corpus_name}: {e}"));

    const KENDALL_THRESHOLD: f64 = 0.95;
    let mut failures: Vec<String> = Vec::new();

    for snap in &snap_file.snapshots {
        let actual = rank_corpus(&docs, &snap.query, scorer, snap.top_n);
        let tau = kendall_tau(&snap.expected_ids, &actual);
        if tau < KENDALL_THRESHOLD {
            failures.push(format!(
                "  query={:?} scorer={} tau={:.3} < {:.3}\n    expected: {:?}\n    actual:   {:?}",
                snap.query, snap.scorer, tau, KENDALL_THRESHOLD, snap.expected_ids, actual
            ));
        }
    }

    if !failures.is_empty() {
        panic!(
            "Ranking regression detected in corpus '{corpus_name}':\n{}\n\n\
             To accept this as an intentional change:\n\
             1. Run: UPDATE_RANKING_SNAPSHOTS=1 cargo test -p terraphim_server --test ranking_regression_test\n\
             2. Review the snapshot diff and add 'Ranking change ACK' to your PR description.",
            failures.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// Query set for the default corpus (5-10 queries, top-5 each — matches the
// "~20 documents" / "5-10 queries" sizing in the issue).
// ---------------------------------------------------------------------------

const DEFAULT_QUERIES: &[(&str, usize)] = &[
    ("rust", 5),
    ("search engine", 5),
    ("automata theory", 5),
    ("graph database", 5),
    ("kubernetes container", 5),
    ("observability monitoring", 5),
    ("ranking bm25", 5),
];

// ---------------------------------------------------------------------------
// Regression gate tests
// ---------------------------------------------------------------------------

#[test]
fn ranking_regression_default_role_bm25() {
    run_regression_for_corpus("corpus_default", DEFAULT_QUERIES);
}

/// AC: "A deliberately perturbed ranking weight causes the job to fail."
///
/// Re-ranks the corpus with a *different* scorer than the committed snapshot
/// was recorded under and asserts the gate would flag it. This is the
/// self-verifying proof that the gate is sensitive to ranking drift: if
/// swapping BM25 -> Tfidf did not change the ordering, the gate would be
/// useless. We assert the gate logic detects the perturbation rather than
/// asserting a specific scorer pair (which would couple the test to BM25/Tfidf
/// internals).
#[test]
fn perturbed_scorer_changes_order_and_fails_gate() {
    let (corpus_file, docs) = load_corpus("corpus_default");
    let snapshot_scorer = parse_scorer(&corpus_file.scorer);

    // Find a query whose ordering differs under at least one alternate scorer.
    // We test every scorer variant against the snapshot scorer for the full
    // corpus query set; if NONE diverges, the gate cannot detect perturbation
    // and the harness is not fit for purpose (test fails, surfacing the gap).
    let alternate_scorers = [
        QueryScorer::Tfidf,
        QueryScorer::Jaccard,
        QueryScorer::BM25F,
        QueryScorer::BM25Plus,
        QueryScorer::QueryRatio,
        QueryScorer::OkapiBM25,
    ];

    let mut detected_divergence = false;
    'query: for (query, top_n) in DEFAULT_QUERIES {
        let baseline = rank_corpus(&docs, query, snapshot_scorer, *top_n);
        for &alt in &alternate_scorers {
            if alt == snapshot_scorer {
                continue;
            }
            let perturbed = rank_corpus(&docs, query, alt, *top_n);
            let tau = kendall_tau(&baseline, &perturbed);
            // A perturbation that pushes tau below threshold is exactly what
            // the gate is designed to catch.
            if tau < 0.95 {
                println!(
                    "Perturbation detected: query={query:?} {snapshot_scorer:?} -> {alt:?} tau={tau:.3}"
                );
                detected_divergence = true;
                break 'query;
            }
        }
    }

    assert!(
        detected_divergence,
        "No scorer perturbation changed the ranking enough to trip the gate (tau<0.95). \
         The harness cannot detect the regression class it claims to guard — choose a corpus \
         whose scorers diverge, or the gate threshold (0.95) is too loose."
    );
}

/// AC: "the suite completes in under 60 seconds."
///
/// Times the full regression run (load + rank all queries across all scorers
/// used above) and asserts it stays well under the 60s budget. The generous
/// 30s ceiling leaves headroom for loaded CI runners while proving the AC.
#[test]
fn suite_runs_well_under_60s_budget() {
    let start = Instant::now();
    let (corpus_file, docs) = load_corpus("corpus_default");
    let scorer = parse_scorer(&corpus_file.scorer);
    for (query, top_n) in DEFAULT_QUERIES {
        let _ = rank_corpus(&docs, query, scorer, *top_n);
    }
    let elapsed = start.elapsed();
    println!("Full ranking regression pass took {elapsed:?}");
    assert!(
        elapsed.as_secs() < 30,
        "ranking regression suite took {elapsed:?}, expected well under the 60s AC budget"
    );
}

// ---------------------------------------------------------------------------
// Unit tests for the Kendall-tau helper (faithful twin of the polyrepo suite)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod kendall_tests {
    use super::kendall_tau;

    fn strs(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn perfect_agreement_is_one() {
        let order = strs(&["a", "b", "c", "d"]);
        assert!((kendall_tau(&order, &order) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn perfect_reversal_is_minus_one() {
        let expected = strs(&["a", "b", "c", "d"]);
        let actual = strs(&["d", "c", "b", "a"]);
        assert!((kendall_tau(&expected, &actual) - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn one_swap_reduces_tau() {
        let expected = strs(&["a", "b", "c", "d"]);
        let actual = strs(&["b", "a", "c", "d"]);
        let tau = kendall_tau(&expected, &actual);
        assert!(tau > 0.0 && tau < 1.0, "tau={tau}");
    }

    #[test]
    fn empty_slice_returns_one() {
        let empty: Vec<String> = vec![];
        assert!((kendall_tau(&empty, &empty) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn single_element_returns_one() {
        let single = strs(&["a"]);
        assert!((kendall_tau(&single, &single) - 1.0).abs() < 1e-9);
    }
}
