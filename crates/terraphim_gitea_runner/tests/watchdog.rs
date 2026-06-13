//! #2119 watchdog tests: poll timeout and error-streak tracking.
//!
//! Tests use a real axum fake-server (no internal mocks) to verify that
//! `run_forever` does not hang when `FetchTask` is slow or erroneous.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{Json, Router, extract::State, routing::post};
use serde_json::{Value, json};
use terraphim_gitea_runner::client::ReqwestRunnerClient;
use terraphim_gitea_runner::config::RunnerConfig;
use terraphim_gitea_runner::policy::DeterministicPlanner;
use terraphim_gitea_runner::poller::Poller;
use terraphim_gitea_runner::state::RunnerState;
use tokio::time::Instant;

fn runner_state() -> RunnerState {
    RunnerState {
        uuid: "uuid-1".into(),
        token: "tok-1".into(),
        name: "fake".into(),
        version: "0.1.0".into(),
        labels: vec!["terraphim-native".into()],
        ephemeral: false,
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
async fn update_task(Json(_b): Json<Value>) -> Json<Value> {
    Json(json!({"tasksVersion": 1, "sentOutputs": {}}))
}
async fn update_log(Json(b): Json<Value>) -> Json<Value> {
    let ack = b["index"].as_i64().unwrap_or(0)
        + b["rows"].as_array().map(|a| a.len() as i64).unwrap_or(0);
    Json(json!({"ackIndex": ack}))
}

#[derive(Default, Clone)]
struct Counters {
    fetch_calls: usize,
}
type Shared = Arc<Mutex<Counters>>;

/// Serve a slow FetchTask that sleeps longer than the client timeout.
async fn fetch_slow(State(s): State<Shared>, Json(_b): Json<Value>) -> Json<Value> {
    s.lock().unwrap().fetch_calls += 1;
    tokio::time::sleep(Duration::from_millis(500)).await;
    Json(json!({"tasksVersion": 0}))
}

/// Serve a fast FetchTask that immediately returns no task.
async fn fetch_fast(State(s): State<Shared>, Json(_b): Json<Value>) -> Json<Value> {
    s.lock().unwrap().fetch_calls += 1;
    Json(json!({"tasksVersion": 0}))
}

async fn spawn_server(shared: Shared, fetch: axum::routing::MethodRouter<Shared>) -> String {
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

/// When FetchTask takes longer than `http_request_timeout`, `poll_once` returns
/// an error (reqwest timeout) and `run_forever` continues without hanging.
#[tokio::test(flavor = "multi_thread")]
async fn poll_timeout_does_not_hang_forever() {
    let shared: Shared = Arc::new(Mutex::new(Counters::default()));
    // Use a short HTTP timeout: slow server sleeps 500ms, we timeout at 100ms.
    let http_timeout = Duration::from_millis(100);
    let http = reqwest::Client::builder()
        .timeout(http_timeout)
        .build()
        .unwrap();
    let url = spawn_server(shared.clone(), post(fetch_slow)).await;
    let config = RunnerConfig {
        instance_url: url.clone(),
        poll_interval: Duration::from_millis(10),
        poll_timeout: Duration::from_millis(200),
        http_request_timeout: http_timeout,
        ..RunnerConfig::default()
    };
    let client =
        Arc::new(terraphim_gitea_runner::client::ReqwestRunnerClient::with_http(url, http));
    let _tmpdir = tempfile::tempdir().unwrap();
    let poller = Poller::new(
        client,
        Arc::new(DeterministicPlanner::default()),
        config,
        _tmpdir.path(),
    );
    let st = runner_state();

    // run_forever loops without end; we let it run for 600ms and verify it
    // made multiple attempts (not frozen waiting on the slow server).
    let start = Instant::now();
    let _ = tokio::time::timeout(Duration::from_millis(600), poller.run_forever(&st)).await;
    let elapsed = start.elapsed();

    // Must have returned within the outer timeout, not the server's 500ms sleep.
    assert!(
        elapsed < Duration::from_millis(700),
        "run_forever must not hang on slow FetchTask (elapsed={elapsed:?})"
    );
    // Must have made at least 2 poll attempts, proving it recovered and retried.
    let calls = shared.lock().unwrap().fetch_calls;
    assert!(
        calls >= 2,
        "at least 2 FetchTask attempts expected (got {calls})"
    );
}

/// `new_with_timeout` builds a client whose underlying reqwest client has the
/// configured timeout — verified by checking that a slow server causes an error
/// response from `poll_once` faster than the server sleeps.
#[tokio::test]
async fn new_with_timeout_sets_request_timeout() {
    let shared: Shared = Arc::new(Mutex::new(Counters::default()));
    let url = spawn_server(shared.clone(), post(fetch_slow)).await;

    // Client with 100ms timeout; server sleeps 500ms.
    let client = Arc::new(ReqwestRunnerClient::new_with_timeout(
        &url,
        Duration::from_millis(100),
    ));
    let config = RunnerConfig {
        instance_url: url,
        poll_timeout: Duration::from_secs(10),
        http_request_timeout: Duration::from_millis(100),
        ..RunnerConfig::default()
    };
    let _tmpdir2 = tempfile::tempdir().unwrap();
    let poller = Poller::new(
        client,
        Arc::new(DeterministicPlanner::default()),
        config,
        _tmpdir2.path(),
    );
    let st = runner_state();

    let start = Instant::now();
    let result = poller.poll_once(&st, 0).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "poll_once must error when server is slow");
    assert!(
        elapsed < Duration::from_millis(300),
        "poll_once must time out before the server's 500ms sleep (elapsed={elapsed:?})"
    );
}

/// A fast server + quick poll_interval: `run_forever` should call FetchTask
/// multiple times within a short window, demonstrating the normal loop works.
#[tokio::test(flavor = "multi_thread")]
async fn run_forever_polls_repeatedly_on_no_task() {
    let shared: Shared = Arc::new(Mutex::new(Counters::default()));
    let url = spawn_server(shared.clone(), post(fetch_fast)).await;
    let config = RunnerConfig {
        instance_url: url.clone(),
        poll_interval: Duration::from_millis(10),
        http_request_timeout: Duration::from_secs(5),
        poll_timeout: Duration::from_secs(10),
        ..RunnerConfig::default()
    };
    let client = Arc::new(ReqwestRunnerClient::new(url));
    let _tmpdir3 = tempfile::tempdir().unwrap();
    let poller = Poller::new(
        client,
        Arc::new(DeterministicPlanner::default()),
        config,
        _tmpdir3.path(),
    );
    let st = runner_state();

    let _ = tokio::time::timeout(Duration::from_millis(120), poller.run_forever(&st)).await;

    let calls = shared.lock().unwrap().fetch_calls;
    assert!(
        calls >= 5,
        "at least 5 FetchTask polls in 120ms with 10ms interval (got {calls})"
    );
}
