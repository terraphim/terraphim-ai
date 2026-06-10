//! #2185 reliability tests against a fake Gitea `RunnerService` (Connect-JSON
//! over a real axum server). No internal mocks -- the real client/poller run.
//!
//! - Fix A (stuck runs): a fake server that GATES like Gitea (only offers the
//!   Waiting job when the runner's `tasks_version` differs from latest) proves
//!   that polling with version 0 picks a job a cached-version poll would miss.
//! - Fix B (orphan-on-skip): a task for a repo not in `active_repos` is reported
//!   SKIPPED via UpdateTask (result 4) rather than silently dropped.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{Json, Router, extract::State, routing::post};
use base64::Engine;
use serde_json::{Value, json};
use terraphim_gitea_runner::client::ReqwestRunnerClient;
use terraphim_gitea_runner::config::RunnerConfig;
use terraphim_gitea_runner::policy::DeterministicPlanner;
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;

#[derive(Default)]
struct Recorded {
    fetch_calls: usize,
    fetch_versions: Vec<i64>,
    task_results: Vec<i32>,
    executed_log_rows: usize,
}
type Shared = Arc<Mutex<Recorded>>;

const LATEST_VERSION: i64 = 5;

/// A task that includes a repository + SHA, triggering a checkout attempt.
fn echo_task(repo: &str) -> Value {
    let yaml = "name: CI\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Greet\n        run: echo hello-2185\n";
    let payload = base64::engine::general_purpose::STANDARD.encode(yaml);
    json!({
        "id": 77,
        "workflowPayload": payload,
        "context": {"github": {"repository": repo, "sha": "cafef00d"}},
        "secrets": {}, "vars": {}, "needs": {}
    })
}

/// A task with no repository/SHA: runs directly in `checkout_dir` without a
/// checkout step. Used in tests that verify polling mechanics, not checkout
/// behaviour, so they do not require a real git server.
fn echo_task_no_repo() -> Value {
    let yaml = "name: CI\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Greet\n        run: echo hello-2185\n";
    let payload = base64::engine::general_purpose::STANDARD.encode(yaml);
    json!({
        "id": 77,
        "workflowPayload": payload,
        "context": {},
        "secrets": {}, "vars": {}, "needs": {}
    })
}

// --- Fix A server: gate the job on version inequality (like Gitea) ---
async fn fetch_gated(State(s): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    let incoming = body["tasksVersion"].as_i64().unwrap_or(0);
    let mut g = s.lock().unwrap();
    g.fetch_calls += 1;
    g.fetch_versions.push(incoming);
    if incoming != LATEST_VERSION {
        // Runner's version differs from latest -> a Waiting job is offered.
        // Use a no-repo task so the test doesn't require a real git server.
        Json(json!({"task": echo_task_no_repo(), "tasksVersion": LATEST_VERSION}))
    } else {
        // Cached-version poll: server reports no new work (the stuck-run gate).
        Json(json!({"tasksVersion": LATEST_VERSION}))
    }
}

// --- Fix B server: always offer a task for a repo NOT in active_repos ---
async fn fetch_otherrepo(State(s): State<Shared>, Json(_b): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.fetch_calls += 1;
    if g.fetch_calls == 1 {
        Json(json!({"task": echo_task("terraphim/other"), "tasksVersion": 2}))
    } else {
        Json(json!({"tasksVersion": 2}))
    }
}

async fn register(Json(_b): Json<Value>) -> Json<Value> {
    Json(
        json!({"runner": {"id": 1, "uuid": "uuid-1", "token": "tok-1",
        "name": "fake", "version": "0.1.0", "labels": ["terraphim-native"], "ephemeral": false}}),
    )
}
async fn declare(Json(b): Json<Value>) -> Json<Value> {
    Json(json!({"version": b["version"], "labels": b["labels"]}))
}
async fn update_task(State(s): State<Shared>, Json(b): Json<Value>) -> Json<Value> {
    if let Some(r) = b["state"]["result"].as_i64() {
        s.lock().unwrap().task_results.push(r as i32);
    }
    Json(json!({"tasksVersion": LATEST_VERSION, "sentOutputs": {}}))
}
async fn update_log(State(s): State<Shared>, Json(b): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    if let Some(rows) = b["rows"].as_array() {
        g.executed_log_rows += rows.len();
    }
    let ack = b["index"].as_i64().unwrap_or(0)
        + b["rows"].as_array().map(|a| a.len() as i64).unwrap_or(0);
    Json(json!({"ackIndex": ack}))
}

async fn spawn(shared: Shared, fetch: axum::routing::MethodRouter<Shared>) -> String {
    let base = "/api/actions/runner.v1.RunnerService";
    let app = Router::new()
        .route(&format!("{base}/Register"), post(register))
        .route(&format!("{base}/Declare"), post(declare))
        .route(&format!("{base}/FetchTask"), fetch)
        .route(&format!("{base}/UpdateTask"), post(update_task))
        .route(&format!("{base}/UpdateLog"), post(update_log))
        .with_state(shared);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

fn state() -> RunnerState {
    RunnerState {
        uuid: "uuid-1".into(),
        token: "tok-1".into(),
        name: "fake".into(),
        version: "0.1.0".into(),
        labels: vec!["terraphim-native".into()],
        ephemeral: false,
    }
}
fn poller(
    url: String,
) -> (
    Poller<ReqwestRunnerClient, DeterministicPlanner>,
    tempfile::TempDir,
) {
    let tmp = tempfile::tempdir().unwrap();
    let config = RunnerConfig {
        active_repos: vec!["proof".into()],
        poll_interval: Duration::from_millis(10),
        ..RunnerConfig::default()
    };
    let p = Poller::new(
        Arc::new(ReqwestRunnerClient::new(url)),
        Arc::new(DeterministicPlanner::default()),
        config,
        tmp.path(),
    );
    (p, tmp)
}

/// Fix A: a job that is Waiting at `LATEST_VERSION` is NOT offered to a poll
/// that sends the cached version (==latest), but IS offered to a poll that
/// sends 0 (run_forever's behaviour after #2185). This is the stuck-run race.
#[tokio::test]
async fn version_zero_poll_picks_job_a_cached_version_poll_misses() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    let url = spawn(shared.clone(), post(fetch_gated)).await;
    let (p, _tmp) = poller(url);
    let st = state();

    // Cached-version poll (the pre-#2185 behaviour) sees no task -> stuck.
    p.poll_once(&st, LATEST_VERSION).await.unwrap();
    assert!(
        shared.lock().unwrap().task_results.is_empty(),
        "a cached-version poll must NOT receive the Waiting job (reproduces stuck-run)"
    );

    // Version-0 poll (the #2185 fix) receives + runs the job.
    p.poll_once(&st, 0).await.unwrap();
    let g = shared.lock().unwrap();
    assert!(
        g.task_results.contains(&1),
        "version-0 poll must fetch + complete the job (success=1); results: {:?}",
        g.task_results
    );
    assert!(
        g.executed_log_rows > 0,
        "the job actually executed (logs streamed)"
    );
}

/// P1-3: a task that includes repository + SHA but whose checkout fails (no real
/// git server) is reported as FAILURE (result 2) rather than silently degrading
/// to the bare checkout_dir and potentially building the wrong code.
#[tokio::test]
async fn checkout_failure_reports_failure_not_silent_degradation() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    // Use a task WITH a real-looking repo/sha so checkout is attempted.
    let task_with_repo = echo_task("terraphim/proof");
    let shared2 = shared.clone();
    let url = spawn(
        shared.clone(),
        post(move |_s: State<Shared>, Json(body): Json<Value>| {
            let task_val = task_with_repo.clone();
            let s2 = shared2.clone();
            async move {
                let incoming = body["tasksVersion"].as_i64().unwrap_or(0);
                let mut g = s2.lock().unwrap();
                g.fetch_calls += 1;
                if incoming != LATEST_VERSION {
                    Json(json!({"task": task_val, "tasksVersion": LATEST_VERSION}))
                } else {
                    Json(json!({"tasksVersion": LATEST_VERSION}))
                }
            }
        }),
    )
    .await;
    let (p, _tmp) = poller(url);
    let st = state();

    // The poll should succeed (poll_once returns Ok), but the task itself must
    // be reported as FAILURE (result 2) because checkout failed.
    p.poll_once(&st, 0).await.unwrap();
    let g = shared.lock().unwrap();
    assert!(
        g.task_results.contains(&2),
        "checkout failure must be reported as FAILURE (result 2); results: {:?}",
        g.task_results
    );
}

/// Fix B: a task for a repo not in `active_repos` is reported SKIPPED (result 4)
/// so Gitea marks it done, rather than being dropped and orphaned.
#[tokio::test]
async fn skipped_repo_task_is_released_not_orphaned() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    let url = spawn(shared.clone(), post(fetch_otherrepo)).await;
    let (p, _tmp) = poller(url); // active_repos = ["proof"], task is for "other"
    let st = state();

    p.poll_once(&st, 0).await.unwrap();

    let g = shared.lock().unwrap();
    assert_eq!(
        g.task_results,
        vec![4],
        "the unservable task must be reported SKIPPED (result 4), not orphaned"
    );
    assert_eq!(g.executed_log_rows, 0, "the skipped task must NOT execute");
}
