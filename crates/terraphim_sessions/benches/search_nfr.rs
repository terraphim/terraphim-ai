//! NFR benchmark for session search (issue #3014).
//!
//! Proves (or refutes) the performance claims in
//! `docs/specifications/terraphim-agent-session-search-spec.md`:
//!
//! - G1 (line 29 / NFR table line 544): "Search latency <100ms for 10K sessions"
//! - F4 §Performance (line 374): "BM25 over 10K sessions is <10ms in benchmarks
//!   (well under 100ms target)"
//!
//! The benchmark seeds exactly 10,000 synthetic sessions and times a single
//! `search_sessions()` query (the unit the NFR is stated for). `search_sessions`
//! rebuilds the `OkapiBM25Scorer` on every call, so this measures the realistic
//! cold-path latency an operator would observe.
//!
//! Corpus is deterministic (seeded, no RNG) so every run is reproducible --
//! faithful-mirror verification, zero deviation between runs.

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use terraphim_sessions::model::{Message, MessageRole, Session, SessionMetadata};
use terraphim_sessions::search::search_sessions;

/// Number of sessions the G1 / F4 NFRs are quantified at.
const NFR_SESSION_COUNT: usize = 10_000;

/// Build a synthetic session that mirrors the `make_session` test helper shape
/// in `src/search.rs` (the exact input type `search_sessions` consumes).
fn make_session(id: usize, title: &str, messages: Vec<(&str, MessageRole, &str)>) -> Session {
    let id_str = id.to_string();
    Session {
        id: id_str.clone(),
        source: "bench".to_string(),
        external_id: id_str.clone(),
        title: if title.is_empty() {
            None
        } else {
            Some(title.to_string())
        },
        source_path: std::path::PathBuf::from(format!("/sessions/{id_str}.jsonl")),
        started_at: None,
        ended_at: None,
        messages: messages
            .into_iter()
            .enumerate()
            .map(|(i, (role, role_type, content))| {
                let mut msg = Message::text(i, role_type, content);
                msg.author = Some(role.to_string());
                msg
            })
            .collect(),
        metadata: SessionMetadata::default(),
    }
}

/// Deterministic 10K-session corpus. Mixes query-relevant sessions (so the
/// result set is non-trivial) with filler sessions (so the BM25 scorer iterates
/// the full corpus, not just hits).
fn build_corpus(n: usize) -> Vec<Session> {
    // Fixed vocabulary -- deterministic, no RNG.
    let titles = [
        "Rust async tokio help",
        "Python web scraping",
        "Rust error handling anyhow",
        "Database SQL optimization",
        "React component state",
        "Cargo workspace setup",
        "Tauri desktop command",
        "Docker compose bind",
        "BM25 search ranking",
        "Session search latency",
    ];
    let bodies = [
        "How do I use async await in Rust with tokio runtime",
        "Best library for web scraping with python requests",
        "Handling errors in Rust with anyhow and thiserror",
        "Optimizing slow SQL queries with indexes and EXPLAIN",
        "Managing React component state with hooks and context",
        "Setting up a cargo workspace with shared dependencies",
        "Registering a tauri command and invoking from svelte",
        "Binding docker compose ports to loopback only",
        "Tuning BM25 okapi scorer parameters for ranking",
        "Measuring session search latency under load",
    ];

    let mut sessions = Vec::with_capacity(n);
    for i in 0..n {
        let pick = i % titles.len();
        // Vary the body slightly by index so sessions are not byte-identical
        // (keeps BM25 term-frequency math non-degenerate) but stay deterministic.
        let body = format!("{} [session {}]", bodies[pick], i);
        sessions.push(make_session(
            i,
            titles[pick],
            vec![
                ("user", MessageRole::User, body.as_str()),
                (
                    "assistant",
                    MessageRole::Assistant,
                    "Here is a helpful response about the topic.",
                ),
            ],
        ));
    }
    sessions
}

fn bench_search_sessions_10k(c: &mut Criterion) {
    let corpus = build_corpus(NFR_SESSION_COUNT);
    assert_eq!(
        corpus.len(),
        NFR_SESSION_COUNT,
        "corpus must be exactly the NFR-stated 10K sessions"
    );

    // "rust async" appears in titles[0]/bodies[0] -- ~1000/10000 sessions match,
    // so the result set is non-trivial and BM25 must score the whole corpus.
    let query = "rust async";

    let mut group = c.benchmark_group("search_nfr");
    group.sample_size(10); // 10K-session init is heavy; criterion min is 10.
    group.bench_function("search_sessions_10k", |b| {
        b.iter(|| {
            let results = search_sessions(black_box(&corpus), black_box(query));
            black_box(results);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_search_sessions_10k);
criterion_main!(benches);
