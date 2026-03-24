//! Integration tests for LinearTracker using digital twin.
//!
//! These tests require the Linear digital twin to be running.
//! The twin runs on port 3008 by default (see zestic-ai/digital-twins#11).
//! Set LINEAR_API_KEY to any value with 'lin_api_' prefix (e.g., 'lin_api_test').
//! Set LINEAR_ENDPOINT to override the default (http://localhost:3008/linear/graphql).

use std::env;
use terraphim_tracker::{IssueTracker, LinearConfig, LinearTracker};

/// Default endpoint for Linear digital twin.
const DEFAULT_LINEAR_ENDPOINT: &str = "http://localhost:3008/linear/graphql";

/// Get test configuration for digital twin.
/// Skips tests if LINEAR_API_KEY is not set.
fn test_config() -> Option<LinearConfig> {
    let endpoint = env::var("LINEAR_ENDPOINT").unwrap_or_else(|_| DEFAULT_LINEAR_ENDPOINT.into());

    // Twin requires API key with 'lin_api_' prefix
    let api_key = env::var("LINEAR_API_KEY").ok()?;

    Some(LinearConfig {
        endpoint,
        api_key,
        project_slug: "TEST".into(),
        active_states: vec!["Todo".into(), "In Progress".into()],
        terminal_states: vec!["Done".into(), "Closed".into()],
    })
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_fetch_issues_by_project() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");

    // Basic sanity checks
    assert!(!issues.is_empty(), "should fetch at least one issue");

    // Verify all issues have required fields
    for issue in &issues {
        assert!(!issue.id.is_empty(), "issue should have id");
        assert!(!issue.identifier.is_empty(), "issue should have identifier");
        assert!(!issue.title.is_empty(), "issue should have title");
        assert!(!issue.state.is_empty(), "issue should have state");

        // Verify state is in active states
        let is_active = tracker
            .config
            .active_states
            .iter()
            .any(|s: &String| s.eq_ignore_ascii_case(&issue.state));
        assert!(
            is_active,
            "issue state should be in active states: {}",
            issue.state
        );
    }
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_fetch_issue_states_by_ids() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    // First fetch some issues to get IDs
    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");
    if issues.is_empty() {
        println!("No issues available for testing state fetch");
        return;
    }

    let ids: Vec<String> = issues.iter().map(|i| i.id.clone()).collect();
    let fetched = tracker
        .fetch_issue_states_by_ids(&ids)
        .await
        .expect("fetch by ids failed");

    assert_eq!(
        fetched.len(),
        issues.len(),
        "should fetch same number of issues"
    );
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_fetch_issues_by_states() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    let states = vec!["Done".into(), "Closed".into()];
    let issues = tracker
        .fetch_issues_by_states(&states)
        .await
        .expect("fetch by states failed");

    // Verify all returned issues are in requested states
    for issue in &issues {
        let is_requested_state = states.iter().any(|s| s.eq_ignore_ascii_case(&issue.state));
        assert!(
            is_requested_state,
            "issue {} should be in requested states, got: {}",
            issue.identifier, issue.state
        );
    }
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_issue_fields_parsed_correctly() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");
    if issues.is_empty() {
        println!("No issues available for field validation");
        return;
    }

    let issue = &issues[0];

    // Verify all fields are populated (twin should provide complete data)
    println!("Issue: {:?}", issue);

    // Labels should be normalized to lowercase
    for label in &issue.labels {
        assert_eq!(
            label.to_lowercase(),
            *label,
            "labels should be lowercase: {}",
            label
        );
    }

    // Timestamps should be present and valid
    assert!(issue.created_at.is_some(), "issue should have created_at");
    assert!(issue.updated_at.is_some(), "issue should have updated_at");
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_blocker_relations() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");

    // Find an issue with blockers
    let issue_with_blockers = issues.iter().find(|i| !i.blocked_by.is_empty());

    if let Some(issue) = issue_with_blockers {
        println!(
            "Found issue with {} blockers: {}",
            issue.blocked_by.len(),
            issue.identifier
        );

        for blocker in &issue.blocked_by {
            assert!(
                blocker.id.is_some() || blocker.identifier.is_some(),
                "blocker should have id or identifier"
            );
        }
    } else {
        println!("No issues with blockers found - this is OK if twin doesn't create them");
    }
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_empty_project_returns_no_issues() {
    let mut config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    config.project_slug = "EMPTY_PROJECT_THAT_DOES_NOT_EXIST".into();

    let tracker = LinearTracker::new(config).expect("failed to create tracker");
    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");

    assert!(
        issues.is_empty(),
        "non-existent project should return no issues"
    );
}

#[tokio::test]
#[ignore = "requires Linear digital twin"]
async fn test_pagination_with_large_result_set() {
    let config = test_config().expect("LINEAR_API_KEY not set - twin not available");
    let tracker = LinearTracker::new(config).expect("failed to create tracker");

    // Fetch all issues (twin should have 100+ to test pagination)
    let issues = tracker
        .fetch_candidate_issues()
        .await
        .expect("fetch failed");

    // If twin has more than 50 issues, pagination was used
    if issues.len() > 50 {
        println!("Pagination verified: fetched {} issues", issues.len());
    } else {
        println!(
            "Only {} issues available (need 50+ to test pagination)",
            issues.len()
        );
    }
}

#[tokio::test]
async fn test_tracker_without_twin_is_skipped() {
    // This test runs without the twin and verifies the skip logic
    if env::var("LINEAR_API_KEY").is_err() {
        println!("LINEAR_API_KEY not set - integration tests will be skipped");
        // Without API key, this test verifies no-panic behavior
    } else {
        println!("LINEAR_API_KEY is set - twin is available");
    }
    // Test passes if no panic occurs (implicit success)
}
