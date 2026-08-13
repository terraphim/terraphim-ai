//! Terminal-lifecycle regressions for claimed tasks (Refs #3222 Task 2).
//!
//! Run 23137 was claimed by the runner, rejected by policy before execution, and
//! then only *logged* -- so Gitea held the job pending/in-progress for 12m52s
//! until the server-side zombie timeout. These tests pin the invariant that
//! every claimed task reaches a terminal result, whichever stage fails, and that
//! the poll loop keeps serving the next task.
//!
//! Ownership contract: `TaskWorker` is the sole finalizer for a task it was
//! handed. The poller logs the composite error and continues; it must never
//! send a second terminal `UpdateTask` for the same task. (The poller's
//! coexistence guard does release a task it never hands to a worker -- that path
//! is covered in `poller_reliability.rs` and is not a double-terminalization.)
//!
//! No mocks: a real axum server speaks Connect-JSON, and the real
//! client/poller/task-worker/host-executor run against it.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, extract::State, routing::post};
use base64::Engine;
use serde_json::{Value, json};
use terraphim_gitea_runner::TaxonomyPlanner;
use terraphim_gitea_runner::client::ReqwestRunnerClient;
use terraphim_gitea_runner::config::RunnerConfig;
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;

/// Terminal Gitea task result codes.
const UNSPECIFIED: i32 = 0;
const SUCCESS: i32 = 1;
const FAILURE: i32 = 2;

/// How the fake Gitea should misbehave, per scenario.
#[derive(Clone, Copy, Default, PartialEq)]
enum Fault {
    #[default]
    None,
    /// Every `UpdateLog` returns HTTP 500 (log delivery is broken).
    LogsRejected,
    /// The first terminal `UpdateTask` returns HTTP 500, later ones succeed.
    FirstTerminalUpdateRejected,
}

#[derive(Default)]
struct Recorded {
    fetch_calls: usize,
    /// Result codes the server actually accepted, in order.
    task_results: Vec<i32>,
    /// (task_id, result) for every accepted update -- proves exactly-once.
    accepted: Vec<(i64, i32)>,
    /// Terminal update attempts including ones the server rejected.
    terminal_attempts: usize,
    /// Accepted terminal updates that carried the required stoppedAt timestamp.
    stopped_task_ids: Vec<i64>,
    log_rows: Vec<String>,
    /// Tasks the server should hand out, in order.
    queue: Vec<Value>,
    fault: Fault,
}
type Shared = Arc<Mutex<Recorded>>;

fn payload(yaml: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(yaml)
}

fn one_step_yaml(run: &str) -> String {
    format!(
        "name: native-ci\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Step\n        run: {run}\n"
    )
}

/// A task with no `sha`, so the worker runs in the bare checkout dir without
/// attempting a checkout (the existing proof-task path).
fn task_no_checkout(id: i64, run: &str) -> Value {
    json!({
        "id": id,
        "workflowPayload": payload(&one_step_yaml(run)),
        "context": {"github": {"repository": "terraphim/proof"}},
        "secrets": {}, "vars": {}, "needs": {}
    })
}

/// A task carrying repository + sha, so the worker must check out before running.
fn task_with_checkout(id: i64, run: &str) -> Value {
    json!({
        "id": id,
        "workflowPayload": payload(&one_step_yaml(run)),
        "context": {"github": {"repository": "terraphim/proof", "sha": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}},
        "secrets": {}, "vars": {}, "needs": {}
    })
}

/// A task whose workflow payload cannot be decoded at all.
fn task_malformed(id: i64) -> Value {
    json!({
        "id": id,
        "workflowPayload": "!!!! not base64 !!!!",
        "context": {"github": {"repository": "terraphim/proof"}},
        "secrets": {}, "vars": {}, "needs": {}
    })
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

async fn fetch_task(State(s): State<Shared>, Json(_b): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.fetch_calls += 1;
    match g.queue.first().cloned() {
        Some(task) => {
            g.queue.remove(0);
            Json(json!({"task": task, "tasksVersion": 2}))
        }
        None => Json(json!({"tasksVersion": 2})),
    }
}

async fn update_task(State(s): State<Shared>, Json(b): Json<Value>) -> axum::response::Response {
    let result = b["state"]["result"].as_i64().unwrap_or(0) as i32;
    let id = b["state"]["id"].as_i64().unwrap_or(0);
    let terminal = result != UNSPECIFIED;
    let mut g = s.lock().unwrap();
    if terminal {
        g.terminal_attempts += 1;
        if g.fault == Fault::FirstTerminalUpdateRejected && g.terminal_attempts == 1 {
            return (StatusCode::INTERNAL_SERVER_ERROR, "transient").into_response();
        }
        if b["state"]["stoppedAt"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
        {
            g.stopped_task_ids.push(id);
        }
    }
    g.task_results.push(result);
    g.accepted.push((id, result));
    Json(json!({"tasksVersion": 2, "sentOutputs": {}})).into_response()
}

async fn update_log(State(s): State<Shared>, Json(b): Json<Value>) -> axum::response::Response {
    let mut g = s.lock().unwrap();
    if g.fault == Fault::LogsRejected {
        return (StatusCode::INTERNAL_SERVER_ERROR, "log sink down").into_response();
    }
    if let Some(rows) = b["rows"].as_array() {
        for row in rows {
            if let Some(c) = row["content"].as_str() {
                g.log_rows.push(c.to_string());
            }
        }
    }
    let ack = b["index"].as_i64().unwrap_or(0)
        + b["rows"].as_array().map(|a| a.len() as i64).unwrap_or(0);
    Json(json!({"ackIndex": ack})).into_response()
}

async fn spawn(shared: Shared) -> String {
    let base = "/api/actions/runner.v1.RunnerService";
    let app = Router::new()
        .route(&format!("{base}/Register"), post(register))
        .route(&format!("{base}/Declare"), post(declare))
        .route(&format!("{base}/FetchTask"), post(fetch_task))
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

/// Build a harness whose fake Gitea will hand out `queue` in order.
async fn harness(
    queue: Vec<Value>,
    fault: Fault,
) -> (
    Shared,
    Poller<ReqwestRunnerClient, TaxonomyPlanner>,
    tempfile::TempDir,
) {
    let shared: Shared = Arc::new(Mutex::new(Recorded {
        queue,
        fault,
        ..Recorded::default()
    }));
    let url = spawn(shared.clone()).await;
    let tmp = tempfile::tempdir().unwrap();
    let config = RunnerConfig {
        active_repos: vec!["proof".into()],
        poll_interval: Duration::from_millis(10),
        instance_url: url.clone(),
        ..RunnerConfig::default()
    };
    let poller = Poller::new(
        Arc::new(ReqwestRunnerClient::new(url)),
        Arc::new(TaxonomyPlanner::default_policy(true)),
        config,
        tmp.path(),
    );
    (shared, poller, tmp)
}

/// Terminal results recorded for one task id.
fn terminals_for(g: &Recorded, id: i64) -> Vec<i32> {
    g.accepted
        .iter()
        .filter(|(tid, r)| *tid == id && *r != UNSPECIFIED)
        .map(|(_, r)| *r)
        .collect()
}

// --------------------------------------------------------------------------
// Pre-execution failures must terminalize
// --------------------------------------------------------------------------

/// The exact run-23137 shape: a claimed task whose command policy rejects before
/// execution must be reported FAILURE promptly, must not execute, must terminalize
/// exactly once, and must not stop the poll loop from serving the next task.
#[tokio::test]
async fn policy_rejection_is_terminal_and_polling_continues() {
    let (shared, poller, _tmp) = harness(
        vec![
            task_no_checkout(101, "./scripts/check-tinyclaw-test-hermeticity.sh"),
            task_no_checkout(102, "echo second-task-ran"),
        ],
        Fault::None,
    )
    .await;
    let st = state();

    poller.poll_once(&st, 0).await.expect("poll must not abort");
    {
        let g = shared.lock().unwrap();
        assert_eq!(
            terminals_for(&g, 101),
            vec![FAILURE],
            "a policy-rejected claim must terminalize exactly once as FAILURE; got {:?}",
            g.accepted
        );
        assert!(
            g.stopped_task_ids.contains(&101),
            "a terminal failure must populate stoppedAt; accepted={:?}",
            g.accepted
        );
        // The rejection diagnostic legitimately *names* the command; what must be
        // absent is evidence it ran. Executed steps emit `[Status] name (exit ..)`
        // header rows, so no log row may be a step header.
        assert!(
            !g.log_rows.iter().any(|l| l.starts_with('[')),
            "the rejected command must never have executed (no step rows); logs: {:?}",
            g.log_rows
        );
        assert!(
            g.log_rows
                .iter()
                .any(|l| l.starts_with("runner error: policy rejected command")),
            "the failure reason must reach the job log; logs: {:?}",
            g.log_rows
        );
    }

    // The loop must keep serving: the next task runs to success.
    poller.poll_once(&st, 0).await.expect("poll must continue");
    let g = shared.lock().unwrap();
    assert_eq!(
        terminals_for(&g, 102),
        vec![SUCCESS],
        "the next task must be claimed and completed after a failed one; got {:?}",
        g.accepted
    );
    assert!(
        g.log_rows.iter().any(|l| l.contains("second-task-ran")),
        "the second task actually executed"
    );
}

/// A payload that cannot be decoded is still a *claimed* task: it must be
/// terminalized rather than orphaned.
#[tokio::test]
async fn malformed_payload_is_terminal() {
    let (shared, poller, _tmp) = harness(vec![task_malformed(201)], Fault::None).await;
    poller.poll_once(&state(), 0).await.expect("poll survives");

    let g = shared.lock().unwrap();
    assert_eq!(
        terminals_for(&g, 201),
        vec![FAILURE],
        "an undecodable payload must terminalize as FAILURE; got {:?}",
        g.accepted
    );
}

/// Checkout must fail closed. Previously a failed checkout silently degraded to
/// the bare checkout root, so a build could "pass" without ever seeing the
/// commit under test. It must now terminalize as FAILURE and run nothing.
#[tokio::test]
async fn checkout_failure_fails_closed_and_is_terminal() {
    // The fake Gitea serves the RunnerService but no git endpoints, so the
    // checkout of terraphim/proof@deadbeef... genuinely fails.
    let (shared, poller, _tmp) = harness(
        vec![task_with_checkout(301, "echo must-not-run")],
        Fault::None,
    )
    .await;
    poller.poll_once(&state(), 0).await.expect("poll survives");

    let g = shared.lock().unwrap();
    assert_eq!(
        terminals_for(&g, 301),
        vec![FAILURE],
        "a failed checkout must terminalize as FAILURE; got {:?}",
        g.accepted
    );
    assert!(
        !g.log_rows.iter().any(|l| l.contains("must-not-run")),
        "the build must NOT execute from the fallback checkout root; logs: {:?}",
        g.log_rows
    );
}

// --------------------------------------------------------------------------
// Post-start failures must terminalize
// --------------------------------------------------------------------------

/// A command that exits non-zero keeps the established lifecycle: in-progress
/// heartbeat, then a single terminal FAILURE.
#[tokio::test]
async fn execution_failure_reports_unspecified_then_failure() {
    let (shared, poller, _tmp) =
        harness(vec![task_no_checkout(401, "bash -c 'exit 7'")], Fault::None).await;
    poller.poll_once(&state(), 0).await.expect("poll survives");

    let g = shared.lock().unwrap();
    assert_eq!(
        g.task_results,
        vec![UNSPECIFIED, FAILURE],
        "execution failure must be [UNSPECIFIED, FAILURE]; got {:?}",
        g.task_results
    );
}

/// If log delivery is broken the task must still terminalize. Losing logs is bad;
/// losing the terminal result strands the Gitea job.
#[tokio::test]
async fn log_delivery_failure_still_terminalizes() {
    let (shared, poller, _tmp) = harness(
        vec![task_no_checkout(501, "echo logs-are-broken")],
        Fault::LogsRejected,
    )
    .await;
    poller.poll_once(&state(), 0).await.expect("poll survives");

    let g = shared.lock().unwrap();
    assert_eq!(
        terminals_for(&g, 501),
        vec![FAILURE],
        "a task whose logs cannot be delivered must still reach a terminal result; got {:?}",
        g.accepted
    );
}

/// Commit-status posting is best-effort: a broken status endpoint must not
/// prevent the terminal result. The mirror writer points at the fake Gitea,
/// which serves no `/api/v1/repos/.../statuses` route.
#[tokio::test]
async fn status_reporting_failure_still_terminalizes() {
    let shared: Shared = Arc::new(Mutex::new(Recorded {
        queue: vec![task_no_checkout(601, "echo status-endpoint-missing")],
        ..Recorded::default()
    }));
    let url = spawn(shared.clone()).await;
    let tmp = tempfile::tempdir().unwrap();
    let config = RunnerConfig {
        active_repos: vec!["proof".into()],
        poll_interval: Duration::from_millis(10),
        instance_url: url.clone(),
        status_token: Some("status-token".into()),
        ..RunnerConfig::default()
    };
    let poller = Poller::new(
        Arc::new(ReqwestRunnerClient::new(url)),
        Arc::new(TaxonomyPlanner::default_policy(true)),
        config,
        tmp.path(),
    );

    poller.poll_once(&state(), 0).await.expect("poll survives");

    let g = shared.lock().unwrap();
    assert_eq!(
        terminals_for(&g, 601),
        vec![SUCCESS],
        "a failed commit-status post must not block the terminal result; got {:?}",
        g.accepted
    );
}

/// A transient failure of the terminal `UpdateTask` itself must be retried within
/// a bounded budget rather than abandoning the task.
#[tokio::test]
async fn transient_terminal_update_failure_is_retried() {
    let (shared, poller, _tmp) = harness(
        vec![task_no_checkout(701, "echo retry-me")],
        Fault::FirstTerminalUpdateRejected,
    )
    .await;
    let _ = poller.poll_once(&state(), 0).await;

    let g = shared.lock().unwrap();
    assert!(
        g.terminal_attempts >= 2,
        "the terminal update must be retried after a transient failure; attempts={}",
        g.terminal_attempts
    );
    assert_eq!(
        terminals_for(&g, 701),
        vec![SUCCESS],
        "the retry must deliver exactly one terminal result; got {:?}",
        g.accepted
    );
}

/// Exactly-once across the whole surface: no scenario may produce two terminal
/// updates for one task id (which would let the poller and the worker both
/// finalize and race the recorded conclusion).
#[tokio::test]
async fn no_task_is_terminalized_twice() {
    let (shared, poller, _tmp) = harness(
        vec![
            task_no_checkout(801, "./scripts/direct.sh"),
            task_malformed(802),
            task_with_checkout(803, "echo nope"),
            task_no_checkout(804, "bash -c 'exit 7'"),
            task_no_checkout(805, "echo fine"),
        ],
        Fault::None,
    )
    .await;
    let st = state();
    for _ in 0..5 {
        let _ = poller.poll_once(&st, 0).await;
    }

    let g = shared.lock().unwrap();
    for id in [801, 802, 803, 804, 805] {
        assert_eq!(
            terminals_for(&g, id).len(),
            1,
            "task {id} must be terminalized exactly once; accepted={:?}",
            g.accepted
        );
    }
}
