//! Integration tests for `pr_review` with real fixture bodies.
//!
//! Fixtures live under `tests/fixtures/pr_review/` and mirror the output of
//! the `structural-pr-review` Claude Code skill for realistic PR-review
//! comments. No mocks — each fixture is a full markdown + HTML body as
//! it would render on Gitea/GitHub.

use std::path::{Path, PathBuf};

use terraphim_orchestrator::pr_review::{
    author_is_agent, evaluate, parse_verdict, AutoMergeCriteria, AutoMergeDecision, PrMetadata,
    VerdictParseError,
};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pr_review")
}

fn load(name: &str) -> String {
    let path = fixture_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}

fn agent_pr(diff_loc: u32) -> PrMetadata {
    PrMetadata {
        pr_number: 631,
        author_login: "claude-code".to_string(),
        diff_loc,
        head_sha: "2ef451d8a9c0b1f2e3d4".to_string(),
        base_branch: "main".to_string(),
    }
}

#[test]
fn parse_verdict_parses_5_of_5_go() {
    let body = load("go_5_5_clean.md");
    let v = parse_verdict(&body, 101).expect("go fixture must parse");

    assert_eq!(v.confidence, 5);
    assert_eq!(v.p0_count, 0);
    assert_eq!(v.p1_count, 0);
    assert_eq!(v.p2_count, 2);
    assert!(v.all_criteria_met, "three ticked boxes should be all-met");
    assert_eq!(v.comment_id, 101);
    assert_eq!(v.commit_short_hash, "2ef451d8");
}

#[test]
fn parse_verdict_parses_3_of_5_conditional() {
    let body = load("conditional_3_5.md");
    let v = parse_verdict(&body, 202).expect("conditional fixture must parse");

    assert_eq!(v.confidence, 3);
    assert_eq!(v.p0_count, 0);
    assert_eq!(v.p1_count, 1);
    assert_eq!(v.p2_count, 2);
    assert!(
        !v.all_criteria_met,
        "one unchecked acceptance criterion must flip the flag to false"
    );
    assert_eq!(v.comment_id, 202);
    assert_eq!(v.commit_short_hash, "3be4e599");
}

#[test]
fn parse_verdict_rejects_no_confidence_header() {
    let body = load("malformed_no_confidence.md");
    let err = parse_verdict(&body, 303).unwrap_err();
    assert_eq!(err, VerdictParseError::MissingConfidence);
}

#[test]
fn parse_verdict_rejects_confidence_out_of_range() {
    // Inline body: confidence is 9/5, which is a template bug, not a valid
    // calibration.
    let body = "<h3>Summary</h3>\nok\n\n<h3>Confidence Score: 9/5</h3>\n- nope\n\n<h3>Inline Findings</h3>\nnothing.\n\n<sub>Last reviewed commit: deadbeef</sub>\n";
    let err = parse_verdict(body, 404).unwrap_err();
    assert_eq!(err, VerdictParseError::ConfidenceOutOfRange(9));
}

#[test]
fn parse_verdict_rejects_missing_findings() {
    let body = "<h3>Summary</h3>\nok\n\n<h3>Confidence Score: 5/5</h3>\n- fine\n\n<sub>Last reviewed commit: cafebabe</sub>\n";
    let err = parse_verdict(body, 405).unwrap_err();
    assert_eq!(err, VerdictParseError::MissingFindings);
}

#[test]
fn parse_verdict_rejects_malformed_footer() {
    let body = "<h3>Summary</h3>\nok\n\n<h3>Confidence Score: 5/5</h3>\n- fine\n\n<h3>Inline Findings</h3>\nnone.\n\n<sub>Reviews (1)</sub>\n";
    let err = parse_verdict(body, 406).unwrap_err();
    assert_eq!(err, VerdictParseError::MalformedFooter);
}

#[test]
fn parse_verdict_handles_multi_round_reviews() {
    let body = load("multi_round_reviews_2.md");
    let v = parse_verdict(&body, 505).expect("multi-round fixture must parse");

    assert_eq!(v.confidence, 4);
    assert_eq!(v.p0_count, 0);
    assert_eq!(v.p1_count, 0);
    assert_eq!(v.p2_count, 1);
    assert!(v.all_criteria_met);
    assert_eq!(v.commit_short_hash, "dcbc2f50");
    assert!(
        body.contains("Reviews (2)"),
        "fixture must use the multi-round footer form"
    );
}

#[test]
fn evaluate_approves_clean_pr() {
    let v = parse_verdict(&load("go_5_5_clean.md"), 101).unwrap();
    let pr = agent_pr(120);
    let decision = evaluate(&v, &pr, &AutoMergeCriteria::default());
    assert_eq!(decision, AutoMergeDecision::Merge);
}

#[test]
fn evaluate_rejects_human_author() {
    let v = parse_verdict(&load("go_5_5_clean.md"), 101).unwrap();
    let mut pr = agent_pr(120);
    pr.author_login = "alex".to_string();
    assert!(!author_is_agent(&pr.author_login));

    match evaluate(&v, &pr, &AutoMergeCriteria::default()) {
        AutoMergeDecision::HumanReviewNeeded(reason) => {
            assert!(
                reason.contains("not a recognised agent"),
                "reason should cite the agent-author gate, got: {reason}"
            );
        }
        AutoMergeDecision::Merge => panic!("human author must never auto-merge"),
    }
}

#[test]
fn evaluate_rejects_large_diff() {
    let v = parse_verdict(&load("go_5_5_clean.md"), 101).unwrap();
    let pr = agent_pr(1_024);
    match evaluate(&v, &pr, &AutoMergeCriteria::default()) {
        AutoMergeDecision::HumanReviewNeeded(reason) => {
            assert!(reason.contains("1024"), "reason should cite diff size");
            assert!(reason.contains("500"), "reason should cite the cap");
        }
        AutoMergeDecision::Merge => panic!("1024 LoC must exceed the 500 LoC cap"),
    }
}

#[test]
fn evaluate_rejects_p1_present() {
    let v = parse_verdict(&load("conditional_3_5.md"), 202).unwrap();
    assert_eq!(v.p1_count, 1);

    // Relax confidence so the P1 gate is exercised on its own merits.
    let criteria = AutoMergeCriteria {
        min_confidence: 3,
        ..AutoMergeCriteria::default()
    };
    let pr = agent_pr(120);
    match evaluate(&v, &pr, &criteria) {
        AutoMergeDecision::HumanReviewNeeded(reason) => {
            assert!(reason.contains("P1"), "reason should cite P1 gate");
        }
        AutoMergeDecision::Merge => panic!("a single P1 must block auto-merge"),
    }
}

#[test]
fn evaluate_rejects_low_confidence() {
    let v = parse_verdict(&load("conditional_3_5.md"), 202).unwrap();
    let pr = agent_pr(120);
    match evaluate(&v, &pr, &AutoMergeCriteria::default()) {
        AutoMergeDecision::HumanReviewNeeded(reason) => {
            assert!(
                reason.contains("3/5") && reason.contains("5/5"),
                "reason should cite the confidence gap, got: {reason}"
            );
        }
        AutoMergeDecision::Merge => panic!("3/5 confidence must not auto-merge at default cap"),
    }
}
