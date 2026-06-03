//! Integration test: drive the native runner against a fake Gitea
//! `RunnerService` (Connect-JSON over a real axum HTTP server). No internal
//! mocks -- the runner's real client/poller/task-worker/host-executor run.

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
    registered: bool,
    declared: bool,
    fetch_calls: usize,
    task_results: Vec<i32>, // result codes received via UpdateTask
    log_rows: Vec<String>,
    log_no_more: bool,
}

type Shared = Arc<Mutex<Recorded>>;

async fn register(State(s): State<Shared>, Json(_body): Json<Value>) -> Json<Value> {
    s.lock().unwrap().registered = true;
    Json(
        json!({"runner": {"id": 1, "uuid": "uuid-1", "token": "tok-1",
        "name": "fake", "version": "0.1.0", "labels": ["terraphim-native"], "ephemeral": false}}),
    )
}

async fn declare(State(s): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    s.lock().unwrap().declared = true;
    Json(json!({"version": body["version"], "labels": body["labels"]}))
}

async fn fetch_task(State(s): State<Shared>, Json(_body): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.fetch_calls += 1;
    if g.fetch_calls == 1 {
        // SingleWorkflow YAML with one host step + one cargo step (cargo routes to rch,
        // but `rch` may be absent on the test host -> we use a plain echo to assert success).
        let yaml = "name: CI\njobs:\n  build:\n    runs-on: terraphim-native\n    steps:\n      - name: Greet\n        run: echo hello-from-native-runner\n";
        let payload = base64::engine::general_purpose::STANDARD.encode(yaml);
        Json(json!({
            "task": {
                "id": 42,
                "workflowPayload": payload,
                "context": {"github": {"repository": "terraphim/proof", "sha": "deadbeef"}},
                "secrets": {}, "vars": {}, "needs": {}
            },
            "tasksVersion": 2
        }))
    } else {
        Json(json!({"tasksVersion": 2}))
    }
}

async fn update_task(State(s): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    if let Some(r) = body["state"]["result"].as_i64() {
        s.lock().unwrap().task_results.push(r as i32);
    }
    Json(json!({"tasksVersion": 2, "sentOutputs": {}}))
}

async fn update_log(State(s): State<Shared>, Json(body): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    if let Some(rows) = body["rows"].as_array() {
        for row in rows {
            if let Some(c) = row["content"].as_str() {
                g.log_rows.push(c.to_string());
            }
        }
    }
    if body["noMore"].as_bool() == Some(true) {
        g.log_no_more = true;
    }
    let ack = body["index"].as_i64().unwrap_or(0)
        + body["rows"].as_array().map(|a| a.len() as i64).unwrap_or(0);
    Json(json!({"ackIndex": ack}))
}

async fn spawn_fake_gitea(shared: Shared) -> String {
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
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn runner_register_declare_fetch_execute_cycle() {
    let shared: Shared = Arc::new(Mutex::new(Recorded::default()));
    let url = spawn_fake_gitea(shared.clone()).await;

    let client = Arc::new(ReqwestRunnerClient::new(url));
    let state = RunnerState {
        uuid: "uuid-1".into(),
        token: "tok-1".into(),
        name: "fake".into(),
        version: "0.1.0".into(),
        labels: vec!["terraphim-native".into()],
        ephemeral: false,
    };
    let tmp = tempfile::tempdir().unwrap();
    let config = RunnerConfig {
        active_repos: vec!["proof".into()],
        poll_interval: Duration::from_millis(10),
        ..RunnerConfig::default()
    };
    let poller = Poller::new(
        client,
        Arc::new(DeterministicPlanner::default()),
        config,
        tmp.path(),
    );

    // First poll fetches + executes the task; second poll sees no task.
    let v = poller.poll_once(&state, 0).await.unwrap();
    assert_eq!(v, 2, "tasks_version advances");
    let v2 = poller.poll_once(&state, v).await.unwrap();
    assert_eq!(v2, 2);

    let g = shared.lock().unwrap();
    assert!(g.fetch_calls >= 2);
    // in-progress (UNSPECIFIED=0) heartbeat then terminal success (1)
    assert_eq!(
        g.task_results,
        vec![0, 1],
        "in-progress(UNSPECIFIED) then success"
    );
    assert!(g.log_no_more, "log stream was closed");
    assert!(
        g.log_rows
            .iter()
            .any(|l| l.contains("hello-from-native-runner")),
        "command stdout streamed to UpdateLog, got: {:?}",
        g.log_rows
    );
}
