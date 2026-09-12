//! Fake-root tests for `terraphim_server` package-manager receipt detection.

use std::fs;
use std::path::{Path, PathBuf};

use terraphim_update::policy::{
    PackageManager, UpdatePolicy, detect_update_policy, inferred_prefix,
};

const BIN_NAME: &str = "terraphim_server";

fn write_file(path: &Path, contents: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    fs::write(path, contents).expect("write file");
}

fn install_binary(root: &Path, prefix: &str, bin_name: &str) -> (PathBuf, PathBuf) {
    let prefix = root.join(prefix);
    let exe = prefix.join("bin").join(bin_name);
    write_file(&exe, b"binary");
    (prefix, exe)
}

fn write_receipt(prefix: &Path, bin_name: &str, contents: &[u8]) {
    write_file(
        &prefix
            .join("share/terraphim/package-manager.d")
            .join(bin_name),
        contents,
    );
}

fn assert_managed(policy: UpdatePolicy, manager: PackageManager, command: &str) {
    match policy {
        UpdatePolicy::PackageManaged {
            manager: actual,
            update_command,
        } => {
            assert_eq!(actual, manager);
            assert_eq!(update_command, command);
        }
        other => panic!("expected PackageManaged, got {other:?}"),
    }
}

#[test]
fn terraphim_server_receipt_under_resolved_prefix_is_package_managed() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    write_receipt(&prefix, BIN_NAME, b"dpkg\n");

    let policy = detect_update_policy(&exe);

    assert_managed(
        policy,
        PackageManager::Dpkg,
        "sudo apt update && sudo apt upgrade",
    );
}

#[test]
fn managed_receipt_path_uses_terraphim_server_key() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    write_receipt(&prefix, "terraphim-server", b"rpm\n");

    let policy = detect_update_policy(&exe);

    assert_eq!(policy, UpdatePolicy::SelfManaged);
}

#[test]
fn supported_server_receipt_values_are_accepted() {
    let cases = [
        (
            b"pacman".as_slice(),
            PackageManager::Pacman,
            "sudo pacman -Syu",
        ),
        (
            b"pacman\n".as_slice(),
            PackageManager::Pacman,
            "sudo pacman -Syu",
        ),
        (
            b"pacman\r\n".as_slice(),
            PackageManager::Pacman,
            "sudo pacman -Syu",
        ),
        (
            b"dpkg".as_slice(),
            PackageManager::Dpkg,
            "sudo apt update && sudo apt upgrade",
        ),
        (
            b"dpkg\n".as_slice(),
            PackageManager::Dpkg,
            "sudo apt update && sudo apt upgrade",
        ),
        (
            b"dpkg\r\n".as_slice(),
            PackageManager::Dpkg,
            "sudo apt update && sudo apt upgrade",
        ),
        (b"rpm".as_slice(), PackageManager::Rpm, "sudo dnf upgrade"),
        (b"rpm\n".as_slice(), PackageManager::Rpm, "sudo dnf upgrade"),
        (
            b"rpm\r\n".as_slice(),
            PackageManager::Rpm,
            "sudo dnf upgrade",
        ),
        (
            b"homebrew".as_slice(),
            PackageManager::Homebrew,
            "brew upgrade terraphim_server",
        ),
        (
            b"homebrew\n".as_slice(),
            PackageManager::Homebrew,
            "brew upgrade terraphim_server",
        ),
        (
            b"homebrew\r\n".as_slice(),
            PackageManager::Homebrew,
            "brew upgrade terraphim_server",
        ),
    ];

    for (contents, manager, command) in cases {
        let root = tempfile::tempdir().expect("tempdir");
        let (prefix, exe) = install_binary(root.path(), "opt/terraphim", BIN_NAME);
        write_receipt(&prefix, BIN_NAME, contents);

        let policy = detect_update_policy(&exe);

        assert_managed(policy, manager, command);
    }
}

#[test]
fn pacman_receipt_is_supported_for_parser_parity() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    write_receipt(&prefix, BIN_NAME, b"pacman\n");

    let policy = detect_update_policy(&exe);

    assert_managed(policy, PackageManager::Pacman, "sudo pacman -Syu");
}

#[test]
fn malformed_or_missing_receipts_are_self_managed() {
    let invalid_contents: &[&[u8]] = &[
        b"",
        b" dpkg",
        b"dpkg ",
        b"dpkg\n\n",
        b"rpm extra",
        b"homebrew\t",
        b"homebrew\nextra",
        b"DPKG",
        b"apt",
        b"dnf",
        b"brew",
        b"rpm\xff",
    ];

    for contents in invalid_contents {
        let root = tempfile::tempdir().expect("tempdir");
        let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
        write_receipt(&prefix, BIN_NAME, contents);

        let policy = detect_update_policy(&exe);

        assert_eq!(
            policy,
            UpdatePolicy::SelfManaged,
            "expected SelfManaged for receipt content {contents:?}"
        );
    }

    let root = tempfile::tempdir().expect("tempdir");
    let (_prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    assert_eq!(detect_update_policy(&exe), UpdatePolicy::SelfManaged);
}

#[test]
fn mismatched_and_cross_prefix_receipts_are_self_managed() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    write_receipt(&prefix, "terraphim-agent", b"dpkg\n");
    assert_eq!(detect_update_policy(&exe), UpdatePolicy::SelfManaged);

    let root = tempfile::tempdir().expect("tempdir");
    let (_prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    let other_prefix = root.path().join("opt/terraphim");
    write_receipt(&other_prefix, BIN_NAME, b"rpm\n");
    assert_eq!(detect_update_policy(&exe), UpdatePolicy::SelfManaged);
}

#[test]
fn executable_name_mismatch_is_self_managed() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", "terraphim-server");
    write_receipt(&prefix, BIN_NAME, b"dpkg\n");

    let policy = detect_update_policy(&exe);

    assert_eq!(policy, UpdatePolicy::SelfManaged);
}

#[test]
fn caller_supplied_bin_name_cannot_bypass_canonical_receipt() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", "terraphim-server");
    write_receipt(&prefix, "terraphim-server", b"rpm\n");

    let policy = detect_update_policy(&exe);

    assert_managed(policy, PackageManager::Rpm, "sudo dnf upgrade");
}

#[test]
fn traversal_resolving_into_prefix_is_package_managed() {
    let root = tempfile::tempdir().expect("tempdir");
    let (prefix, exe) = install_binary(root.path(), "usr", BIN_NAME);
    let other = prefix.join("other");
    fs::create_dir_all(&other).expect("other dir");
    let traversal_exe = other.join("..").join("bin").join(BIN_NAME);
    assert_eq!(fs::canonicalize(&traversal_exe).expect("canonical"), exe);
    write_receipt(&prefix, BIN_NAME, b"rpm\n");

    let policy = detect_update_policy(&traversal_exe);

    assert_managed(policy, PackageManager::Rpm, "sudo dnf upgrade");
}

#[cfg(unix)]
#[test]
fn symlinked_prefix_uses_resolved_prefix_receipt() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let (real_prefix, _real_exe) = install_binary(root.path(), "opt/terraphim", BIN_NAME);
    let link_prefix = root.path().join("usr");
    fs::create_dir_all(&link_prefix).expect("link prefix");
    symlink(real_prefix.join("bin"), link_prefix.join("bin")).expect("symlink bin dir");
    write_receipt(&real_prefix, BIN_NAME, b"homebrew\n");

    let policy = detect_update_policy(&link_prefix.join("bin").join(BIN_NAME));

    assert_managed(
        policy,
        PackageManager::Homebrew,
        "brew upgrade terraphim_server",
    );
}

#[cfg(unix)]
#[test]
fn symlinked_prefix_ignores_link_prefix_receipt() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("tempdir");
    let (real_prefix, _real_exe) = install_binary(root.path(), "opt/terraphim", BIN_NAME);
    let link_prefix = root.path().join("usr");
    fs::create_dir_all(&link_prefix).expect("link prefix");
    symlink(real_prefix.join("bin"), link_prefix.join("bin")).expect("symlink bin dir");
    write_receipt(&link_prefix, BIN_NAME, b"dpkg\n");

    let policy = detect_update_policy(&link_prefix.join("bin").join(BIN_NAME));

    assert_eq!(policy, UpdatePolicy::SelfManaged);
}

#[test]
fn inferred_prefix_strips_bin_and_binary_name() {
    let exe = Path::new("/usr/local/bin/terraphim_server");

    let prefix = inferred_prefix(exe);

    assert_eq!(prefix, Some(PathBuf::from("/usr/local")));
}

#[test]
fn guidance_message_contains_manager_update_command() {
    let policy = UpdatePolicy::PackageManaged {
        manager: PackageManager::Rpm,
        update_command: "sudo dnf upgrade".to_string(),
    };
    let msg = terraphim_update::policy::guidance(&policy, BIN_NAME);

    assert!(msg.contains("sudo dnf upgrade"));
}
