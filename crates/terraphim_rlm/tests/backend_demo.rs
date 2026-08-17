//! Demonstration: RLM running locally (LocalExecutor) and via Docker (DockerExecutor).
//! Run: cargo test -p terraphim_rlm --test backend_demo -- --nocapture

use terraphim_rlm::TerraphimRlm;
use terraphim_rlm::config::{BackendType, RlmConfig};

#[tokio::test]
async fn demo_local_executor() {
    println!("\n═══════════════════════════════════════════");
    println!("  LOCAL EXECUTOR (no isolation)");
    println!("═══════════════════════════════════════════\n");

    let config = RlmConfig {
        backend_preference: vec![BackendType::Local],
        ..RlmConfig::minimal()
    };
    let rlm = TerraphimRlm::new(config).await.unwrap();
    let session = rlm.create_session().await.unwrap();

    // Python
    let r = rlm.execute_code(&session.id, "print(2+2)").await.unwrap();
    println!(
        "  [Python] 2+2 = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );

    // Bash
    let r = rlm
        .execute_command(&session.id, "echo hello-from-local")
        .await
        .unwrap();
    println!(
        "  [Bash]   echo = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );

    // Show backend type
    let status = rlm.get_session_status(&session.id, false).await.unwrap();
    println!("  [Backend] {:?}", status.backend_type);

    // Show that it runs on host
    let r = rlm.execute_command(&session.id, "whoami").await.unwrap();
    println!("  [Host user] {}", r.stdout.trim());

    let r = rlm.execute_command(&session.id, "hostname").await.unwrap();
    println!("  [Host name] {}", r.stdout.trim());

    rlm.destroy_session(&session.id).await.unwrap();
    println!("\n  Local executor works.\n");
}

#[tokio::test]
#[ignore = "requires Docker daemon with python:3.11-slim image; run with --ignored to enable"]
async fn demo_docker_executor() {
    println!("\n═══════════════════════════════════════════");
    println!("  DOCKER EXECUTOR (container isolation)");
    println!("═══════════════════════════════════════════\n");

    let config = RlmConfig {
        backend_preference: vec![BackendType::Docker, BackendType::Local],
        ..RlmConfig::minimal()
    };
    let rlm = TerraphimRlm::new(config).await.unwrap();
    let session = rlm.create_session().await.unwrap();

    // Python
    let r = rlm.execute_code(&session.id, "print(2+2)").await.unwrap();
    println!(
        "  [Python] 2+2 = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );

    // Bash
    let r = rlm
        .execute_command(&session.id, "echo hello-from-docker")
        .await
        .unwrap();
    println!(
        "  [Bash]   echo = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );

    // Show backend type
    let status = rlm.get_session_status(&session.id, false).await.unwrap();
    println!("  [Backend] {:?}", status.backend_type);

    // Show Docker isolation
    let r = rlm.execute_command(&session.id, "whoami").await.unwrap();
    println!("  [Container user] {}", r.stdout.trim());

    let r = rlm.execute_command(&session.id, "hostname").await.unwrap();
    println!("  [Container hostname] {}", r.stdout.trim());

    // Show Python version inside container
    let r = rlm
        .execute_code(&session.id, "import sys; print(sys.version)")
        .await
        .unwrap();
    println!("  [Python version] {}", r.stdout.trim());

    // Show container filesystem
    let r = rlm
        .execute_command(&session.id, "ls / | head -5")
        .await
        .unwrap();
    println!("  [Container root]\n{}", r.stdout);

    rlm.destroy_session(&session.id).await.unwrap();
    println!("  Docker executor works.\n");
}

/// Real Apple Container evidence. Requires Apple silicon, macOS 26,
/// `brew install container`, `container system start`, and `python:3.11-slim`
/// pre-pulled (`container image pull python:3.11-slim`) — `container run` pulls
/// implicitly, but that pull runs inside the 60s lifecycle timeout, so an
/// uncached image surfaces as a `container run` failure.
///
/// ```bash
/// cargo test -p terraphim_rlm --features apple-container-backend \
///     --test backend_demo apple_container -- --ignored --nocapture
/// ```
#[cfg(feature = "apple-container-backend")]
#[tokio::test]
#[ignore = "requires Apple silicon macOS 26 with `container` installed and started; run with --ignored"]
async fn demo_apple_container_executor() {
    use std::sync::Arc;
    use terraphim_rlm::executor::{AppleContainerExecutor, ExecutionContext, ExecutionEnvironment};
    use terraphim_rlm::types::SessionId;

    println!("\n═══════════════════════════════════════════");
    println!("  APPLE CONTAINER EXECUTOR (VM per container)");
    println!("═══════════════════════════════════════════\n");

    let exec = Arc::new(AppleContainerExecutor::new(RlmConfig::minimal(), None).unwrap());
    exec.probe()
        .await
        .expect("`container` must be installed and `container system start` already run");

    let ctx = ExecutionContext {
        session_id: SessionId::new(),
        timeout_ms: 120_000,
        ..Default::default()
    };

    // Python.
    let r = exec.execute_code("print(2+2)", &ctx).await.unwrap();
    println!(
        "  [Python] 2+2 = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );
    assert!(r.is_success());
    assert_eq!(r.stdout.trim(), "4");

    // Bash.
    let r = exec
        .execute_command("echo hello-from-apple-container", &ctx)
        .await
        .unwrap();
    println!("  [Bash]   {} (exit {})", r.stdout.trim(), r.exit_code);
    assert_eq!(r.stdout.trim(), "hello-from-apple-container");

    // Non-zero exit mapping.
    let r = exec.execute_command("exit 42", &ctx).await.unwrap();
    println!("  [Exit]   exit 42 -> {}", r.exit_code);
    assert_eq!(r.exit_code, 42);
    assert!(!r.is_success());

    // Same-session state: the marker file survives across calls, proving
    // affinity to one container.
    exec.execute_command("echo marker > /tmp/affinity", &ctx)
        .await
        .unwrap();
    let r = exec
        .execute_command("cat /tmp/affinity", &ctx)
        .await
        .unwrap();
    println!("  [Affinity] /tmp/affinity = {}", r.stdout.trim());
    assert_eq!(r.stdout.trim(), "marker");

    // Timeout fails closed and the next call gets a fresh container.
    let short = ExecutionContext {
        timeout_ms: 2_000,
        ..ctx.clone()
    };
    let r = exec.execute_command("sleep 600", &short).await.unwrap();
    println!("  [Timeout] timed_out={}", r.timed_out);
    assert!(r.timed_out);
    let r = exec
        .execute_command("cat /tmp/affinity", &ctx)
        .await
        .unwrap();
    println!("  [Recreate] post-timeout marker read exit {}", r.exit_code);
    assert_ne!(
        r.exit_code, 0,
        "timeout must destroy the container, so the marker is gone"
    );

    // Teardown.
    exec.end_session(&ctx.session_id).await.unwrap();
    exec.cleanup().await.unwrap();

    // No leaked resources.
    let list = std::process::Command::new("container")
        .args(["list", "--all", "--format", "json"])
        .output()
        .expect("container list");
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(
        !listing.contains("terraphim-rlm-"),
        "leaked containers after teardown:\n{listing}"
    );

    println!("\n  Apple Container executor works, no leaked containers.\n");
}

/// Real Apple Container evidence **through `select_executor`**, the path
/// production actually takes. Constructing `AppleContainerExecutor` directly
/// (as `demo_apple_container_executor` does) proves the executor works but says
/// nothing about selection: a backend that is never chosen, or that is chosen
/// with a config the direct constructor bypasses, would leave that test green.
///
/// Same host requirements as `demo_apple_container_executor`.
///
/// ```bash
/// cargo test -p terraphim_rlm --features apple-container-backend \
///     --test backend_demo apple_container_via_select -- --ignored --nocapture
/// ```
#[cfg(feature = "apple-container-backend")]
#[tokio::test]
#[ignore = "requires Apple silicon macOS 26 with `container` installed and started; run with --ignored"]
async fn demo_apple_container_via_select_executor() {
    use terraphim_rlm::executor::{ExecutionContext, select_executor};
    use terraphim_rlm::types::SessionId;

    println!("\n═══════════════════════════════════════════");
    println!("  APPLE CONTAINER via select_executor()");
    println!("═══════════════════════════════════════════\n");

    // Only Apple Container is offered: selection must not silently fall through
    // to a less isolated backend and still pass.
    let config = RlmConfig {
        backend_preference: vec![BackendType::AppleContainer],
        ..RlmConfig::minimal()
    };
    let exec = select_executor(&config)
        .await
        .expect("select_executor must choose Apple Container on a prepared host");
    assert_eq!(exec.backend_type(), BackendType::AppleContainer);
    println!("  [Select]  backend = {}", exec.backend_type());

    let ctx = ExecutionContext {
        session_id: SessionId::new(),
        timeout_ms: 120_000,
        ..Default::default()
    };

    let r = exec.execute_code("print(2+2)", &ctx).await.unwrap();
    println!(
        "  [Python]  2+2 = {} (exit {})",
        r.stdout.trim(),
        r.exit_code
    );
    assert!(r.is_success());
    assert_eq!(r.stdout.trim(), "4");

    // `bash -c`, not `-lc`: no guest profile is sourced, so the `--env` value
    // this backend passes in is what the command sees.
    let r = exec
        .execute_command(
            "echo \"$RLM_MARKER\"",
            &ctx.clone().with_env("RLM_MARKER", "via-select"),
        )
        .await
        .unwrap();
    println!("  [Env]     RLM_MARKER = {}", r.stdout.trim());
    assert_eq!(r.stdout.trim(), "via-select");

    // The result identifies where it ran.
    assert_eq!(
        r.metadata.get("backend").map(String::as_str),
        Some("apple-container")
    );
    assert!(
        r.metadata["container"].starts_with("terraphim-rlm-"),
        "{:?}",
        r.metadata
    );

    exec.end_session(&ctx.session_id).await.unwrap();
    exec.cleanup().await.unwrap();

    let list = std::process::Command::new("container")
        .args(["list", "--all", "--format", "json"])
        .output()
        .expect("container list");
    let listing = String::from_utf8_lossy(&list.stdout);
    assert!(
        !listing.contains("terraphim-rlm-"),
        "leaked containers after teardown:\n{listing}"
    );

    println!("\n  select_executor picked Apple Container, no leaked containers.\n");
}
