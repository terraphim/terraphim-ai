//! #2185 reliability tests against a fake Gitea `RunnerService` (Connect-JSON
//! over a real axum server). No internal mocks -- the real client/poller run.
//!
//! - Fix A (stuck runs): a fake server that GATES like Gitea (only offers the
//!   Waiting job when the runner's `tasks_version` differs from latest) proves
//!   that polling with version 0 picks a job a cached-version poll would miss.
//! - Fix B (orphan-on-skip): a task for a repo not in `active_repos` is reported
//!   SKIPPED via UpdateTask (result 4) rather than silently dropped.
//!
//! Plus the #3222 lifecycle-ownership guarantees: the coexistence skip is
//! finalized through `TaskWorker` (terminal, `stoppedAt`, bounded retry), and the
//! daemon's poll timeout bounds `FetchTask` only -- never a claimed task's run.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{Json, Router, extract::State, routing::post};
use base64::Engine;
use serde_json::{Value, json};
use terraphim_gitea_runner::TaxonomyPlanner;
use terraphim_gitea_runner::client::ReqwestRunnerClient;
use terraphim_gitea_runner::config::RunnerConfig;
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;

#[derive(Default)]
struct Recorded {
    fetch_calls: usize,
    fetch_versions: Vec<i64>,
    task_results: Vec<i32>,
    executed_log_rows: usize,
    /// Every `UpdateTask` POST the server saw, including ones it rejected.
    update_attempts: usize,
    /// `stoppedAt` presence for each *accepted* terminal (non-`UNSPECIFIED`)
    /// update, in arrival order.
    terminal_stopped_at: Vec<bool>,
}

impl Recorded {
    /// Terminal (non-`UNSPECIFIED`) results the server accepted.
    fn terminal_results(&self) -> Vec<i32> {
        self.task_results
            .iter()
            .copied()
            .filter(|r| *r != 0)
            .collect()
    }
}
type Shared = Arc<Mutex<Recorded>>;

const LATEST_VERSION: i64 = 5;

/// Poll timeout used by the #3222 cancellation test. Deliberately shorter than
/// [`SLOW_UPDATE_DELAY`] so a timeout wrapping worker execution would fire.
const SHORT_POLL_TIMEOUT: Duration = Duration::from_millis(150);

/// How long the fake server stalls the first in-progress `UpdateTask`.
const SLOW_UPDATE_DELAY: Duration = Duration::from_millis(600);

/// A task with no `sha`, so no checkout is attempted: these tests cover the
/// fetch/dispatch gating, and since #3222 a task naming a repository *and* a sha
/// must check out successfully or fail closed (this fake Gitea has no git side).
fn echo_task(repo: &str) -> Value {
    let yaml = "name: CI\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Greet\n        run: echo hello-2185\n";
    let payload = base64::engine::general_purpose::STANDARD.encode(yaml);
    json!({
        "id": 77,
        "workflowPayload": payload,
        "context": {"github": {"repository": repo}},
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
        Json(json!({"task": echo_task("terraphim/proof"), "tasksVersion": LATEST_VERSION}))
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

// --- Servers for the #3222 lifecycle-ownership tests ---

/// Offer one task for a repo that IS in `active_repos`, then report no work.
async fn fetch_proof_once(State(s): State<Shared>, Json(_b): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.fetch_calls += 1;
    if g.fetch_calls == 1 {
        Json(json!({"task": echo_task("terraphim/proof"), "tasksVersion": 2}))
    } else {
        Json(json!({"tasksVersion": 2}))
    }
}

/// Record an accepted `UpdateTask`, tracking `stoppedAt` on terminal results.
fn record_accepted(s: &Shared, b: &Value) {
    let mut g = s.lock().unwrap();
    if let Some(r) = b["state"]["result"].as_i64() {
        if r != 0 {
            g.terminal_stopped_at
                .push(b["state"]["stoppedAt"].is_string());
        }
        g.task_results.push(r as i32);
    }
}

/// Reject the first `UpdateTask` with a transient 5xx, accept the rest. Proves a
/// bounded retry is applied to terminal delivery (Refs #3222).
async fn update_task_flaky_first(
    State(s): State<Shared>,
    Json(b): Json<Value>,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    let attempt = {
        let mut g = s.lock().unwrap();
        g.update_attempts += 1;
        g.update_attempts
    };
    if attempt == 1 {
        return (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "transient upstream failure",
        )
            .into_response();
    }
    record_accepted(&s, &b);
    Json(json!({"tasksVersion": LATEST_VERSION, "sentOutputs": {}})).into_response()
}

/// Stall the first (in-progress) `UpdateTask` well past `poll_timeout`, so a
/// timeout that wraps worker execution would cancel the task mid-flight.
async fn update_task_slow_first(State(s): State<Shared>, Json(b): Json<Value>) -> Json<Value> {
    let attempt = {
        let mut g = s.lock().unwrap();
        g.update_attempts += 1;
        g.update_attempts
    };
    if attempt == 1 {
        tokio::time::sleep(SLOW_UPDATE_DELAY).await;
    }
    record_accepted(&s, &b);
    Json(json!({"tasksVersion": LATEST_VERSION, "sentOutputs": {}}))
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
    s.lock().unwrap().update_attempts += 1;
    record_accepted(&s, &b);
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
    spawn_full(shared, fetch, post(update_task)).await
}

async fn spawn_full(
    shared: Shared,
    fetch: axum::routing::MethodRouter<Shared>,
    update: axum::routing::MethodRouter<Shared>,
) -> String {
    let base = "/api/actions/runner.v1.RunnerService";
    let app = Router::new()
        .route(&format!("{base}/Register"), post(register))
        .route(&format!("{base}/Declare"), post(declare))
        .route(&format!("{base}/FetchTask"), fetch)
        .route(&format!("{base}/UpdateTask"), update)
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
    Poller<ReqwestRunnerClient, TaxonomyPlanner>,
    tempfile::TempDir,
) {
    poller_cfg(
        url,
        RunnerConfig {
            active_repos: vec!["proof".into()],
            poll_interval: Duration::from_millis(10),
            ..RunnerConfig::default()
        },
    )
}

fn poller_cfg(
    url: String,
    config: RunnerConfig,
) -> (
    Poller<ReqwestRunnerClient, TaxonomyPlanner>,
    tempfile::TempDir,
) {
    let tmp = tempfile::tempdir().unwrap();
    let p = Poller::new(
        Arc::new(ReqwestRunnerClient::new(url)),
        Arc::new(TaxonomyPlanner::default_policy(true)),
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
    assert_eq!(
        g.terminal_stopped_at,
        vec![true],
        "the SKIPPED result must carry stoppedAt so Gitea moves the job out of running"
    );
}

/// #3222 (P1 A): the coexistence guard must finalize an already-claimed task
/// through the same owner and the same terminal-update policy as any other
/// claimed task -- `stoppedAt` plus a bounded retry. A single best-effort
/// `UpdateTask` strands the claimed job on the first transient 5xx.
#[tokio::test]
async fn skipped_task_retries_transient_terminal_update_failure() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    let url = spawn_full(
        shared.clone(),
        post(fetch_otherrepo),
        post(update_task_flaky_first),
    )
    .await;
    let (p, _tmp) = poller(url); // active_repos = ["proof"], task is for "other"
    let st = state();

    p.poll_once(&st, 0).await.unwrap();

    let g = shared.lock().unwrap();
    assert_eq!(
        g.update_attempts, 2,
        "the rejected terminal update must be retried exactly once more (bounded), \
         not abandoned; attempts: {}",
        g.update_attempts
    );
    assert_eq!(
        g.terminal_results(),
        vec![4],
        "exactly one accepted terminal SKIPPED result; results: {:?}",
        g.task_results
    );
    assert_eq!(
        g.terminal_stopped_at,
        vec![true],
        "the accepted SKIPPED result must carry stoppedAt"
    );
    assert_eq!(g.executed_log_rows, 0, "the skipped task must NOT execute");
}

/// #3222 (P1 B): `poll_timeout` bounds fetching, not execution. A claimed task
/// whose run outlasts `poll_timeout` must not be cancelled by the daemon loop --
/// it must still reach exactly one terminal result carrying `stoppedAt`.
#[tokio::test]
async fn worker_outliving_poll_timeout_is_not_cancelled_before_terminalizing() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    let url = spawn_full(
        shared.clone(),
        post(fetch_proof_once),
        post(update_task_slow_first),
    )
    .await;
    let (p, _tmp) = poller_cfg(
        url,
        RunnerConfig {
            active_repos: vec!["proof".into()],
            poll_interval: Duration::from_millis(10),
            poll_timeout: SHORT_POLL_TIMEOUT,
            ..RunnerConfig::default()
        },
    );
    let st = state();

    // Drive the real daemon loop: it is the only place the poll timeout is
    // applied, so `poll_once` alone cannot exercise the cancellation boundary.
    let _ = tokio::time::timeout(Duration::from_secs(5), p.run_forever(&st)).await;

    let g = shared.lock().unwrap();
    assert_eq!(
        g.terminal_results(),
        vec![1],
        "the task must reach exactly one terminal SUCCESS despite running longer \
         than poll_timeout ({SHORT_POLL_TIMEOUT:?}); results: {:?}",
        g.task_results
    );
    assert_eq!(
        g.terminal_stopped_at,
        vec![true],
        "the terminal result must carry stoppedAt"
    );
    assert!(
        g.executed_log_rows > 0,
        "the job actually executed (logs streamed)"
    );
}
