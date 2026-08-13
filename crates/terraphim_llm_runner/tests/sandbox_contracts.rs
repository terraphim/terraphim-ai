use std::fs;

use std::error::Error;

use terraphim_llm_runner::{StrictDockerSandboxError, strict_docker_diagnostics_sandbox};
use terraphim_rlm::config::BackendType;
use terraphim_rlm::executor::ExecutionEnvironment;

#[test]
fn strict_docker_sandbox_rejects_invalid_checkout_without_leaking_source_chain() {
    let file_dir = tempfile::tempdir().expect("file tempdir");
    let file_path = file_dir.path().join("file");
    fs::write(&file_path, "not a checkout directory").expect("test file");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime");
    let error = runtime
        .block_on(strict_docker_diagnostics_sandbox(&file_path))
        .expect_err("file checkout rejected");

    assert!(matches!(error, StrictDockerSandboxError::InvalidCheckout));
    assert!(!format!("{error:?}").contains(file_path.to_string_lossy().as_ref()));
    assert!(
        !error
            .to_string()
            .contains(file_path.to_string_lossy().as_ref())
    );
    assert!(
        error.source().is_none(),
        "strict sandbox errors must not expose backend/path sources"
    );
}

#[test]
fn strict_docker_sandbox_public_api_is_opaque() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/strict_sandbox_no_host_config.rs");
    tests.compile_fail("tests/ui/strict_sandbox_no_public_profile.rs");
    tests.compile_fail("tests/ui/strict_sandbox_no_raw_constructor.rs");
}

#[tokio::test]
#[ignore = "requires a reachable Docker daemon"]
async fn strict_docker_sandbox_constructs_docker_executor_only() {
    let checkout = tempfile::tempdir().expect("checkout tempdir");

    let executor = strict_docker_diagnostics_sandbox(checkout.path())
        .await
        .expect("strict docker executor");

    assert_eq!(executor.backend_type(), BackendType::Docker);
}
