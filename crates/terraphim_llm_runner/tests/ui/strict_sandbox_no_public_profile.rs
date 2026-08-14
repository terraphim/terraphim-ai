use terraphim_llm_runner::strict_docker_diagnostics_profile;
use terraphim_rlm::executor::StrictDockerDiagnosticsProfile;

fn main() {
    let checkout = tempfile::tempdir().unwrap();
    let profile = StrictDockerDiagnosticsProfile::new(checkout.path()).unwrap();
    let _ = profile.host_config();
    let _ = strict_docker_diagnostics_profile(checkout.path()).unwrap();
}
