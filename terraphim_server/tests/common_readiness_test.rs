//! Self-tests for `tests/common/mod.rs` readiness helper (issue #2998).
//!
//! Exercises both observable branches of [`common::wait_for_server_ready`]:
//!  * happy path — returns promptly once `/health` answers 200;
//!  * exhaustion — panics with an attributed message once the probe budget
//!    is spent against an address where nothing is listening.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use serial_test::serial;
use terraphim_server::axum_server;

use common::{READY_PROBE_INTERVAL, wait_for_server_ready_with_budget};

/// Boot `axum_server` on an ephemeral port against a minimal config, then
/// assert the readiness helper returns once `/health` is live.
#[tokio::test]
#[serial]
async fn wait_for_server_ready_returns_once_health_is_live() {
    let address = spawn_minimal_server().await;

    // Happy path: must return without panicking. Use a tight budget so the
    // test fails fast if readiness is never reached.
    wait_for_server_ready_with_budget(address, 80, READY_PROBE_INTERVAL).await;
}

/// Against an address where nothing is listening, the helper must panic with
/// a message that mentions both the address and the readiness budget, so
/// boot failures surface as attributed test failures.
#[tokio::test]
#[serial]
async fn wait_for_server_ready_panics_with_attributed_message_on_exhaustion() {
    // An unused port where nothing is listening.
    let port = portpicker::pick_unused_port().expect("no unused port available");
    let dead_address = SocketAddr::from(([127, 0, 0, 1], port));

    // Two probes at 1ms each keeps the test instant while still exercising the
    // panic path (request fails because nothing answers → budget exhausted).
    let result = std::panic::AssertUnwindSafe(async {
        wait_for_server_ready_with_budget(dead_address, 2, Duration::from_millis(1)).await;
    });

    use futures_util::FutureExt;
    let outcome = result.catch_unwind().await;

    let msg = outcome.expect_err("helper must panic when /health never answers");
    let msg = msg
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| msg.downcast_ref::<&'static str>().copied())
        .unwrap_or("<non-string panic payload>");

    assert!(
        msg.contains(&dead_address.to_string()),
        "panic message must name the unreachable address {dead_address}; got: {msg}"
    );
    assert!(
        msg.contains("/health"),
        "panic message must reference the /health readiness probe; got: {msg}"
    );
}

/// Boot a real `axum_server` on an ephemeral port with a minimal Default role
/// config (mirrors `health_contract_test.rs::minimal_config` but is kept
/// inline so these self-tests stay independent of sibling test files).
async fn spawn_minimal_server() -> SocketAddr {
    use terraphim_automata::AutomataPath;
    use terraphim_config::{
        ConfigBuilder, ConfigState, Haystack, KnowledgeGraph, KnowledgeGraphLocal, Role,
        ServiceType,
    };
    use terraphim_types::{KnowledgeGraphInputType, RelevanceFunction};

    let port = portpicker::pick_unused_port().expect("no unused port available");
    let address = SocketAddr::from(([127, 0, 0, 1], port));

    let system_operator_pages = tempfile::Builder::new()
        .prefix("common_readiness_pages")
        .tempdir()
        .expect("failed to create tempdir")
        .keep();

    let mut config = ConfigBuilder::new()
        .global_shortcut("Ctrl+X")
        .add_role(
            "Default",
            Role {
                shortname: Some("Default".to_string()),
                name: "Default".into(),
                relevance_function: RelevanceFunction::TitleScorer,
                theme: "spacelab".to_string(),
                kg: Some(KnowledgeGraph {
                    automata_path: Some(AutomataPath::from_local("fixtures/term_to_id.json")),
                    knowledge_graph_local: Some(KnowledgeGraphLocal {
                        input_type: KnowledgeGraphInputType::Markdown,
                        path: system_operator_pages,
                    }),
                    public: true,
                    publish: true,
                }),
                haystacks: vec![Haystack {
                    location: "fixtures/haystack".to_string(),
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
        .expect("failed to build minimal config");

    let config_state = ConfigState::new(&mut config)
        .await
        .expect("failed to create config state");

    tokio::spawn(async move {
        if let Err(e) = axum_server(address, config_state).await {
            eprintln!("Server error in readiness self-test: {e:?}");
        }
    });

    address
}
