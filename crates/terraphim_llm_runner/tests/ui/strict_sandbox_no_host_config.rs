use terraphim_llm_runner::strict_docker_diagnostics_sandbox;

#[tokio::main]
async fn main() {
    let checkout = tempfile::tempdir().unwrap();
    let sandbox = strict_docker_diagnostics_sandbox(checkout.path())
        .await
        .unwrap();

    let _ = sandbox.with_host_config(Default::default());
}
