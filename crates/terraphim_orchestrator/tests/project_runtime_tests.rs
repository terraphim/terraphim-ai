//! Integration tests for runtime project plumbing (issue terraphim/adf-fleet#4):
//! OutputPoster per-project routing, legacy-mode fallback, and concurrency /
//! dispatcher fairness seen through public APIs.

use terraphim_orchestrator::config::OrchestratorConfig;
use terraphim_orchestrator::dispatcher::{DispatchTask, Dispatcher, LEGACY_PROJECT_ID};
use terraphim_orchestrator::output_poster::OutputPoster;

fn two_project_config() -> OrchestratorConfig {
    let toml_str = r#"
working_dir = "/tmp/adf"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[gitea]
base_url = "https://git.example.test"
token = "legacy-token"
owner = "legacy-owner"
repo = "legacy-repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[projects.gitea]
base_url = "https://git.example.test"
token = "alpha-token"
owner = "alpha-owner"
repo = "alpha-repo"

[[projects]]
id = "beta"
working_dir = "/tmp/beta"

[projects.gitea]
base_url = "https://git.example.test"
token = "beta-token"
owner = "beta-owner"
repo = "beta-repo"

[[agents]]
name = "alpha-worker"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "alpha"

[[agents]]
name = "beta-worker"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "beta"
"#;
    OrchestratorConfig::from_toml(toml_str).unwrap()
}

#[test]
fn output_poster_routes_per_project_and_to_legacy_fallback() {
    let config = two_project_config();
    let poster =
        OutputPoster::from_orchestrator_config(&config).expect("expected poster to be constructed");

    // Agent lookups resolve to the correct project's tracker (owner/repo).
    let alpha = poster
        .tracker_for("alpha", "alpha-worker")
        .expect("alpha tracker");
    assert_eq!(alpha.owner(), "alpha-owner");
    assert_eq!(alpha.repo(), "alpha-repo");

    let beta = poster
        .tracker_for("beta", "beta-worker")
        .expect("beta tracker");
    assert_eq!(beta.owner(), "beta-owner");
    assert_eq!(beta.repo(), "beta-repo");

    // Unknown project ids fall back to the legacy project (top-level gitea).
    let legacy = poster
        .tracker_for(LEGACY_PROJECT_ID, "alpha-worker")
        .expect("legacy tracker");
    assert_eq!(legacy.owner(), "legacy-owner");
    assert_eq!(legacy.repo(), "legacy-repo");

    let unknown = poster
        .tracker_for("does-not-exist", "anybody")
        .expect("fallback tracker");
    assert_eq!(unknown.owner(), "legacy-owner");
}

#[test]
fn output_poster_legacy_single_project_still_addressable() {
    // Legacy single-project config: only top-level gitea, no [[projects]].
    let toml_str = r#"
working_dir = "/tmp/adf"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[gitea]
base_url = "https://git.example.test"
token = "legacy-token"
owner = "legacy-owner"
repo = "legacy-repo"

[[agents]]
name = "legacy"
layer = "Safety"
cli_tool = "claude"
task = "t"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    let poster = OutputPoster::from_orchestrator_config(&config).expect("legacy poster constructs");

    // Legacy project id resolves the top-level tracker.
    let tracker = poster
        .tracker_for(LEGACY_PROJECT_ID, "legacy")
        .expect("legacy tracker");
    assert_eq!(tracker.owner(), "legacy-owner");
    assert_eq!(tracker.repo(), "legacy-repo");

    // Unknown project ids also fall back to legacy.
    let fallback = poster
        .tracker_for("unknown", "legacy")
        .expect("fallback resolves to legacy");
    assert_eq!(fallback.owner(), "legacy-owner");
}

#[test]
fn output_poster_without_gitea_returns_none() {
    let toml_str = r#"
working_dir = "/tmp/adf"

[nightwatch]

[compound_review]
schedule = "0 2 * * *"
repo_path = "/tmp/repo"

[[projects]]
id = "alpha"
working_dir = "/tmp/alpha"

[[agents]]
name = "alpha-worker"
layer = "Safety"
cli_tool = "claude"
task = "t"
project = "alpha"
"#;
    let config = OrchestratorConfig::from_toml(toml_str).unwrap();
    assert!(OutputPoster::from_orchestrator_config(&config).is_none());
}

#[test]
fn dispatcher_round_robin_fairness_across_projects() {
    let mut dispatcher = Dispatcher::new();

    // Enqueue three alpha tasks and one beta task at the same layer/score.
    for name in ["a1", "a2", "a3"] {
        dispatcher.enqueue(DispatchTask::TimeDriven {
            name: name.into(),
            task: "t".into(),
            layer: terraphim_orchestrator::AgentLayer::Core,
            project: "alpha".into(),
        });
    }
    dispatcher.enqueue(DispatchTask::TimeDriven {
        name: "b1".into(),
        task: "t".into(),
        layer: terraphim_orchestrator::AgentLayer::Core,
        project: "beta".into(),
    });

    // First dequeue: alpha wins on FIFO.
    assert_eq!(dispatcher.dequeue().unwrap().project(), "alpha");
    // Second dequeue: beta jumps ahead via round-robin (never served yet).
    assert_eq!(dispatcher.dequeue().unwrap().project(), "beta");
    // Remaining two are alpha-only.
    assert_eq!(dispatcher.dequeue().unwrap().project(), "alpha");
    assert_eq!(dispatcher.dequeue().unwrap().project(), "alpha");
}

#[test]
fn dispatcher_by_project_stats_track_enqueue_dequeue() {
    let mut dispatcher = Dispatcher::new();
    dispatcher.enqueue(DispatchTask::TimeDriven {
        name: "a".into(),
        task: "t".into(),
        layer: terraphim_orchestrator::AgentLayer::Core,
        project: "alpha".into(),
    });
    dispatcher.enqueue(DispatchTask::TimeDriven {
        name: "b".into(),
        task: "t".into(),
        layer: terraphim_orchestrator::AgentLayer::Core,
        project: "beta".into(),
    });
    assert_eq!(dispatcher.stats().by_project.get("alpha"), Some(&1));
    assert_eq!(dispatcher.stats().by_project.get("beta"), Some(&1));

    dispatcher.dequeue();
    // Still one of the two remains.
    let total_remaining: u64 = dispatcher.stats().by_project.values().sum();
    assert_eq!(total_remaining, 1);
}
