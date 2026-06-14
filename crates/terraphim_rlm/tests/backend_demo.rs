//! Demonstration: RLM running locally (LocalExecutor) and via Docker (DockerExecutor).
//! Run: cargo test -p terraphim_rlm --test backend_demo -- --nocapture

use terraphim_rlm::config::{BackendType, RlmConfig};
use terraphim_rlm::TerraphimRlm;

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
    println!("  [Python] 2+2 = {} (exit {})", r.stdout.trim(), r.exit_code);

    // Bash
    let r = rlm.execute_command(&session.id, "echo hello-from-local").await.unwrap();
    println!("  [Bash]   echo = {} (exit {})", r.stdout.trim(), r.exit_code);

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
    println!("  [Python] 2+2 = {} (exit {})", r.stdout.trim(), r.exit_code);

    // Bash
    let r = rlm.execute_command(&session.id, "echo hello-from-docker").await.unwrap();
    println!("  [Bash]   echo = {} (exit {})", r.stdout.trim(), r.exit_code);

    // Show backend type
    let status = rlm.get_session_status(&session.id, false).await.unwrap();
    println!("  [Backend] {:?}", status.backend_type);

    // Show Docker isolation
    let r = rlm.execute_command(&session.id, "whoami").await.unwrap();
    println!("  [Container user] {}", r.stdout.trim());

    let r = rlm.execute_command(&session.id, "hostname").await.unwrap();
    println!("  [Container hostname] {}", r.stdout.trim());

    // Show Python version inside container
    let r = rlm.execute_code(&session.id, "import sys; print(sys.version)").await.unwrap();
    println!("  [Python version] {}", r.stdout.trim());

    // Show container filesystem
    let r = rlm.execute_command(&session.id, "ls / | head -5").await.unwrap();
    println!("  [Container root]\n{}", r.stdout);

    rlm.destroy_session(&session.id).await.unwrap();
    println!("  Docker executor works.\n");
}
