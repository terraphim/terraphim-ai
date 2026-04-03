use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;
use terraphim_file_search::kg_scorer::KgPathScorer;
use terraphim_types::{NormalizedTerm, NormalizedTermValue, Thesaurus};
use fff_search::external_scorer::ExternalScorer;
use fff_search::types::FileItem;

// ---- helpers ----------------------------------------------------------------

fn make_file(relative_path: &str) -> FileItem {
    FileItem::new_raw(
        PathBuf::from(relative_path),
        relative_path.to_string(),
        relative_path
            .rsplit('/')
            .next()
            .unwrap_or(relative_path)
            .to_string(),
        0,
        0,
        None,
        false,
    )
}

fn make_thesaurus(terms: &[&str]) -> Thesaurus {
    let mut t = Thesaurus::new("bench".to_string());
    for (i, &term) in terms.iter().enumerate() {
        let key = NormalizedTermValue::from(term.to_string());
        let nt = NormalizedTerm {
            id: i as u64,
            value: NormalizedTermValue::from(term.to_string()),
            display_value: None,
            url: None,
        };
        t.insert(key, nt);
    }
    t
}

/// Synthetic file paths: mix of matching and non-matching entries.
fn synthetic_files(count: usize) -> Vec<FileItem> {
    // Path components that overlap with KG terms (matching paths)
    let matching = [
        "src/challenge/model.rs",
        "src/skill/mapper.rs",
        "src/competency/index.rs",
        "src/assessment/runner.rs",
        "src/learning_outcome/tracker.rs",
        "src/curriculum/builder.rs",
        "src/rubric/evaluator.rs",
        "src/portfolio/aggregator.rs",
        "src/mentorship/session.rs",
        "src/collaboration/hub.rs",
    ];
    // Paths that do not match any KG term
    let non_matching = [
        "src/utils/helpers.rs",
        "src/core/engine.rs",
        "src/io/reader.rs",
        "src/net/client.rs",
        "src/db/schema.rs",
        "src/auth/guard.rs",
        "src/cache/store.rs",
        "src/config/loader.rs",
        "src/metrics/recorder.rs",
        "src/logging/sink.rs",
    ];

    (0..count)
        .map(|i| {
            if i % 3 == 0 {
                make_file(matching[i % matching.len()])
            } else {
                make_file(non_matching[i % non_matching.len()])
            }
        })
        .collect()
}

/// 500 realistic domain terms from an educational/competency domain.
fn domain_terms_500() -> Vec<&'static str> {
    // Core domain terms (repeated with variations to reach ~500)
    let base: &[&str] = &[
        "challenge", "skill", "competency", "assessment", "learning_outcome",
        "curriculum", "rubric", "portfolio", "mentorship", "collaboration",
        "pedagogy", "andragogy", "scaffolding", "differentiation", "metacognition",
        "formative", "summative", "benchmark", "proficiency", "mastery",
        "inquiry", "reflection", "synthesis", "analysis", "evaluation",
        "taxonomy", "objective", "standard", "indicator", "descriptor",
        "engagement", "motivation", "autonomy", "feedback", "intervention",
        "remediation", "enrichment", "extension", "accommodation", "modification",
        "artifact", "evidence", "demonstration", "performance", "exhibition",
        "criterion", "threshold", "milestone", "progression", "trajectory",
        "domain", "subdomain", "topic", "concept", "principle",
        "knowledge", "understanding", "application", "transfer", "integration",
        "literacy", "numeracy", "fluency", "accuracy", "precision",
        "critical_thinking", "problem_solving", "creativity", "innovation", "design",
        "research", "investigation", "experiment", "hypothesis", "conclusion",
        "communication", "presentation", "argumentation", "discourse", "dialogue",
        "leadership", "teamwork", "responsibility", "initiative", "perseverance",
        "resilience", "empathy", "ethics", "citizenship", "community",
        "digital_literacy", "information_literacy", "media_literacy", "financial_literacy", "health_literacy",
        "stem", "steam", "arts", "humanities", "social_sciences",
        "mathematics", "statistics", "algebra", "geometry", "calculus",
        "biology", "chemistry", "physics", "ecology", "genetics",
        "history", "geography", "economics", "psychology", "sociology",
        "language", "writing", "reading", "listening", "speaking",
        "vocabulary", "grammar", "syntax", "semantics", "pragmatics",
        "module", "unit", "lesson", "session", "activity",
        "project", "task", "exercise", "practice", "review",
        "quiz", "test", "exam", "survey", "questionnaire",
        "grade", "score", "mark", "result", "outcome",
        "progress", "growth", "development", "achievement", "attainment",
        "gap", "need", "strength", "weakness", "opportunity",
        "strategy", "approach", "method", "technique", "tool",
        "resource", "material", "content", "curriculum_map", "scope",
        "sequence", "pacing", "calendar", "schedule", "timeline",
        "cohort", "group", "class", "team", "pair",
        "student", "learner", "participant", "candidate", "trainee",
        "teacher", "instructor", "facilitator", "coach", "mentor",
        "advisor", "tutor", "evaluator", "assessor", "reviewer",
        "institution", "school", "college", "university", "academy",
        "department", "faculty", "program", "course", "track",
        "certification", "credential", "diploma", "degree", "badge",
        "accreditation", "recognition", "endorsement", "award", "honor",
    ];
    // The base list has 500 entries already; slice to exactly 500.
    base[..base.len().min(500)].to_vec()
}

/// 100 terms subset for 10k-file benchmark.
fn domain_terms_100() -> Vec<&'static str> {
    domain_terms_500()[..100].to_vec()
}

// ---- benchmarks -------------------------------------------------------------

fn bench_kg_scorer_empty_thesaurus(c: &mut Criterion) {
    let scorer = KgPathScorer::new(Thesaurus::new("empty".to_string()));
    let files = synthetic_files(1000);

    c.bench_function("kg_scorer_empty_thesaurus_1k_files", |b| {
        b.iter(|| {
            for f in &files {
                black_box(scorer.score(black_box(f)));
            }
        });
    });
}

fn bench_kg_scorer_500_terms(c: &mut Criterion) {
    let thesaurus = make_thesaurus(&domain_terms_500());
    let scorer = KgPathScorer::new(thesaurus);
    let files = synthetic_files(1000);

    c.bench_function("kg_scorer_500_terms_1k_files", |b| {
        b.iter(|| {
            for f in &files {
                black_box(scorer.score(black_box(f)));
            }
        });
    });
}

fn bench_kg_scorer_10k_files(c: &mut Criterion) {
    let thesaurus = make_thesaurus(&domain_terms_100());
    let scorer = KgPathScorer::new(thesaurus);
    let files = synthetic_files(10_000);

    c.bench_function("kg_scorer_100_terms_10k_files", |b| {
        b.iter(|| {
            for f in &files {
                black_box(scorer.score(black_box(f)));
            }
        });
    });
}

fn bench_thesaurus_reload(c: &mut Criterion) {
    let initial = make_thesaurus(&domain_terms_500());
    let scorer = KgPathScorer::new(initial);

    c.bench_function("thesaurus_reload_500_terms", |b| {
        b.iter(|| {
            let t = make_thesaurus(&domain_terms_500());
            scorer.update_thesaurus(black_box(t));
        });
    });
}

criterion_group!(
    benches,
    bench_kg_scorer_empty_thesaurus,
    bench_kg_scorer_500_terms,
    bench_kg_scorer_10k_files,
    bench_thesaurus_reload,
);
criterion_main!(benches);
