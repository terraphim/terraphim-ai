//! No-network / no-write tests for package-managed `terraphim_server`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use terraphim_update::policy::{PackageManager, UpdatePolicy};
use terraphim_update::{
    TerraphimUpdater, UpdateStatus, UpdaterConfig, check_for_updates_auto_with_policy,
};

const BIN_NAME: &str = "terraphim_server";
const POISON_R2_URL: &str = "not a usable update URL";

fn package_managed_policy() -> UpdatePolicy {
    UpdatePolicy::PackageManaged {
        manager: PackageManager::Dpkg,
        update_command: "sudo apt update && sudo apt upgrade".to_string(),
    }
}

fn install_destination_candidates(bin_name: &str) -> Vec<PathBuf> {
    let dir = std::env::current_exe()
        .expect("current_exe")
        .parent()
        .expect("parent")
        .to_path_buf();
    vec![dir.join(bin_name), dir.join(bin_name.replace('_', "-"))]
}

fn write_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, contents).expect("write file");
}

fn install_managed_server(root: &Path) -> PathBuf {
    let prefix = root.join("usr");
    let exe = prefix.join("bin").join(BIN_NAME);
    write_file(&exe, b"binary");
    write_file(
        &prefix
            .join("share/terraphim/package-manager.d")
            .join(BIN_NAME),
        b"rpm\n",
    );
    exe
}

fn install_managed_binary(root: &Path, bin_name: &str, manager: &[u8]) -> PathBuf {
    let prefix = root.join("usr");
    let exe = prefix.join("bin").join(bin_name);
    write_file(&exe, b"binary");
    write_file(
        &prefix
            .join("share/terraphim/package-manager.d")
            .join(bin_name),
        manager,
    );
    exe
}

fn assert_package_managed(status: UpdateStatus) {
    assert!(
        matches!(status, UpdateStatus::PackageManaged { .. }),
        "expected PackageManaged, got {status:?}"
    );
}

#[tokio::test]
async fn package_managed_check_update_makes_zero_requests() {
    let config = UpdaterConfig::new(BIN_NAME)
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(package_managed_policy());
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_update().await.expect("check_update");

    assert_package_managed(status);
}

#[tokio::test]
async fn misspelled_config_bin_name_cannot_bypass_canonical_receipt() {
    let root = tempfile::tempdir().expect("tempdir");
    let exe = install_managed_binary(root.path(), "terraphim-server", b"rpm\n");
    let config = UpdaterConfig::new(BIN_NAME)
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(UpdatePolicy::SelfManaged)
        .with_current_exe_path(exe);
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_update().await.expect("check_update");

    assert_package_managed(status);
}

#[tokio::test]
async fn package_managed_update_makes_zero_requests_and_zero_writes() {
    let bin_name = format!("{BIN_NAME}_managed_update_{}", std::process::id());
    let destinations = install_destination_candidates(&bin_name);
    for dest in &destinations {
        assert!(!dest.exists(), "precondition: {dest:?} must not exist");
    }

    let config = UpdaterConfig::new(bin_name.as_str())
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(package_managed_policy());
    let updater = TerraphimUpdater::new(config);

    let status = updater.update().await.expect("update");

    assert_package_managed(status);
    for dest in &destinations {
        assert!(!dest.exists(), "update must not write {dest:?}");
    }
}

#[tokio::test]
async fn package_managed_check_and_update_makes_zero_requests_and_zero_writes() {
    let bin_name = format!("{BIN_NAME}_managed_full_{}", std::process::id());
    let destinations = install_destination_candidates(&bin_name);
    for dest in &destinations {
        assert!(!dest.exists(), "precondition: {dest:?} must not exist");
    }

    let config = UpdaterConfig::new(bin_name.as_str())
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(package_managed_policy());
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_and_update().await.expect("check_and_update");

    assert_package_managed(status);
    for dest in &destinations {
        assert!(!dest.exists(), "check_and_update must not write {dest:?}");
    }
}

#[tokio::test]
async fn self_managed_check_update_makes_request_control() {
    let root = tempfile::tempdir().expect("tempdir");
    let exe = root.path().join("local/bin").join(BIN_NAME);
    write_file(&exe, b"binary");
    let config = UpdaterConfig::new(BIN_NAME)
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(UpdatePolicy::SelfManaged)
        .with_current_exe_path(exe);
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_update().await.expect("check_update");

    assert!(
        matches!(status, UpdateStatus::Failed(_)),
        "SelfManaged control should attempt the poisoned update backend, got {status:?}"
    );
}

#[tokio::test]
async fn stale_self_managed_policy_redetects_before_request() {
    let root = tempfile::tempdir().expect("tempdir");
    let exe = install_managed_server(root.path());
    let config = UpdaterConfig::new(BIN_NAME)
        .with_r2_base_url(POISON_R2_URL)
        .with_github_fallback(false)
        .with_policy(UpdatePolicy::SelfManaged)
        .with_current_exe_path(exe);
    let updater = TerraphimUpdater::new(config);

    let status = updater.check_update().await.expect("check_update");

    assert_package_managed(status);
}

#[tokio::test]
async fn package_managed_update_with_verification_makes_zero_writes() {
    let bin_name = format!("{BIN_NAME}_verify_{}", std::process::id());
    let destinations = install_destination_candidates(&bin_name);
    for dest in &destinations {
        assert!(!dest.exists(), "precondition: {dest:?} must not exist");
    }

    let config = UpdaterConfig::new(bin_name.as_str()).with_policy(package_managed_policy());
    let updater = TerraphimUpdater::new(config);

    let status = tokio::time::timeout(Duration::from_secs(5), updater.update_with_verification())
        .await
        .expect("update_with_verification must short-circuit")
        .expect("update_with_verification");

    assert_package_managed(status);
    for dest in &destinations {
        assert!(
            !dest.exists(),
            "update_with_verification must not write {dest:?}"
        );
    }
}

#[tokio::test]
async fn startup_auto_check_with_managed_policy_short_circuits() {
    let policy = package_managed_policy();
    let status = tokio::time::timeout(
        Duration::from_secs(5),
        check_for_updates_auto_with_policy(BIN_NAME, "0.0.1", &policy),
    )
    .await
    .expect("startup check must short-circuit")
    .expect("startup check");

    match status {
        UpdateStatus::PackageManaged { update_command, .. } => {
            assert!(update_command.contains("sudo apt update"));
        }
        other => panic!("expected PackageManaged, got {other:?}"),
    }
}
