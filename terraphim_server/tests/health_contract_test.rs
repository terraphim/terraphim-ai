//! Contract test for the `GET /health` readiness endpoint.
//!
//! Guards the response *shape* (issue #3010), not just the status code:
//! any regression to a free-form body (e.g. the historical plain `"OK"`
//! string), a missing/renamed `status` field, or a non-JSON content type
//! fails this test. Integration test harnesses and Tauri parity checks
//! rely on this stable readiness signal.
//!
//! Reuses the same ephemeral-port + `axum_server` harness pattern as the
//! sibling `server.rs` integration tests.

#![cfg(test)]

use std::net::SocketAddr;
use std::time::Duration;

use serial_test::serial;
use terraphim_automata::AutomataPath;
use terraphim_config::{
    Config, ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
    ServiceType,
};
use terraphim_server::axum_server;
use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

/// Build a minimal valid `Config` so `axum_server` can boot. Mirrors the
/// fixture-based config used by `tests/server.rs::sample_config`, trimmed to
/// a single role (the `/health` route is role-agnostic).
fn minimal_config() -> Config {
    let automata_path = AutomataPath::from_local("fixtures/term_to_id.json");
    let haystack = "fixtures/haystack".to_string();
    let system_operator_pages = tempfile::Builder::new()
        .prefix("health_contract_pages")
        .tempdir()
        .expect("failed to create tempdir")
        .keep();

    ConfigBuilder::new()
        .global_shortcut("Ctrl+X")
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(automata_path),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_pages,
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: haystack,
                    service: ServiceType::Ripgrep,
                    read_only: false,
                    atomic_server_secret: None,
                    extra_parameters: std::collections::HashMap::new(),
                    fetch_content: false,
                }],
                terraphim_it: false,
                ..Default::default()
            },
        )
        .build()
        .expect("failed to build minimal config")
}

/// Start `axum_server` on an ephemeral port and return its address.
async fn start_server() -> SocketAddr {
    let port = portpicker::pick_unused_port().expect("Failed to find unused port");
    let server_hostname = SocketAddr::from(([127, 0, 0, 1], port));

    let mut config = minimal_config();
    let config_state = ConfigState::new(&mut config)
        .await
        .expect("Failed to create config state");

    tokio::spawn(async move {
        if let Err(e) = axum_server(server_hostname, config_state).await {
            eprintln!("Server error: {:?}", e);
        }
    });

    server_hostname
}

/// Poll `/health` until it responds 200, mirroring `server.rs`'s readiness
/// wait so we never race the router startup.
async fn wait_for_server_ready(address: SocketAddr) {
    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");
    let health_url = format!("http://{address}/health");

    let mut attempts = 0;
    loop {
        match client.get(&health_url).send().await {
            Ok(response) if response.status() == 200 => break,
            _ => {
                if attempts >= 20 {
                    panic!("Server did not become ready in time at {address}");
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
                attempts += 1;
            }
        }
    }
}

/// AC #3010 (Gherkin):
///   Given the server test harness is running
///   When a GET request is made to /health
///   Then the response status is 200
///   And the response content-type is application/json
///   And the response body contains a "status" field equal to "ok"
///   And the test fails if any of these fields are missing or changed
#[tokio::test]
#[serial]
async fn get_health_returns_200_json_with_status_ok() {
    let server = start_server().await;
    wait_for_server_ready(server).await;

    let client = terraphim_service::http_client::create_default_client()
        .expect("Failed to create HTTP client");

    let response = client
        .get(format!("http://{server}/health"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .expect("GET /health request failed");

    // AC: status is 200
    assert_eq!(
        response.status(),
        200,
        "/health must return HTTP 200, got {}",
        response.status()
    );

    // AC: content-type is application/json. Axum sets this from Json<T>;
    // compare on the media type only (allow `; charset=utf-8` suffix).
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap_or("").to_ascii_lowercase())
        .unwrap_or_default();
    assert!(
        content_type.starts_with("application/json"),
        "/health content-type must be application/json, got {content_type:?}"
    );

    // AC: body is JSON with a "status" field equal to "ok", and the test
    // fails if the field is missing or changed.
    let body: serde_json::Value = response
        .json()
        .await
        .expect("/health body must be valid JSON");
    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .expect(r#"/health body must contain a string "status" field"#);
    assert_eq!(
        status, "ok",
        r#"/health body "status" must be "ok", got {status:?}"#
    );
}
