//! Public-entrypoint controls for self-managed updater behavior.

use terraphim_update::{TerraphimUpdater, UpdateStatus, UpdaterConfig};

const BIN_NAME: &str = "terraphim_server";
const POISON_R2_URL: &str = "not a usable update URL";

#[tokio::test]
async fn self_managed_public_check_update_attempts_update_backend() {
    let config = UpdaterConfig::new(BIN_NAME)
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false);
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_update().await.expect("check_update");

    assert!(
        matches!(status, UpdateStatus::Failed(_)),
        "SelfManaged control should attempt the poisoned update backend, got {status:?}"
    );
}

#[test]
fn updater_config_has_no_public_policy_downgrade_bypass() {
    let source = include_str!("../src/lib.rs");

    assert!(!source.contains("pub fn with_policy"));
    assert!(!source.contains("pub fn with_current_exe_path"));
    assert!(!source.contains("pub policy: policy::UpdatePolicy"));
    assert!(!source.contains("pub async fn check_for_updates_auto_with_policy"));
}
