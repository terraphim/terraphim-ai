//! Cross-boundary contract test: the updater consumes the coordinator-owned
//! canonical `release-manifest-v1` layout.
//!
//! The fixture under `tests/fixtures/release-manifest/` is validated against
//! `.release/release-manifest.schema.json` by the Python contract suite
//! (`tests/release_manifest_validator_test.py` and
//! `tests/updater_r2_manifest_contract_test.py`). This test consumes the same
//! bytes through the updater's parser and asset selection, so any drift
//! between the published schema and the updater fails on one side or both.

use std::path::PathBuf;

use terraphim_update::r2;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/release-manifest")
        .join(name)
}

#[test]
fn canonical_fixture_parses_and_selects_agent_and_grep_assets_exactly() {
    let raw = std::fs::read_to_string(fixture_path("valid-v1.json"))
        .expect("canonical fixture must exist");
    let manifest = r2::parse_release_manifest(&raw)
        .expect("canonical fixture must parse as release-manifest-v1");

    assert_eq!(manifest.schema_version, "1.0.0");
    assert_eq!(manifest.release_version, "1.2.3");
    assert_eq!(manifest.release_tag, "v1.2.3");

    let targets = vec!["x86_64-unknown-linux-gnu".to_string()];

    for (component, expected_name, expected_sha_prefix) in [
        (
            "terraphim-agent",
            "terraphim-agent-1.2.3-linux-x86_64.tar.gz",
            "bbbb",
        ),
        (
            "terraphim-grep",
            "terraphim-grep-1.2.3-linux-x86_64.tar.gz",
            "cccc",
        ),
    ] {
        let asset = manifest
            .select_asset(component, &targets)
            .unwrap_or_else(|| panic!("fixture must carry a selectable {component} asset"));
        assert_eq!(asset.name, expected_name, "{component} asset name");
        assert_eq!(asset.component, component);
        assert_eq!(asset.format, "tar.gz");
        assert_eq!(asset.target, "x86_64-unknown-linux-gnu");
        assert_eq!(asset.arch, "x86_64");
        assert_eq!(asset.os, "linux");
        assert!(
            asset.sha256.starts_with(expected_sha_prefix) && asset.sha256.len() == 64,
            "{component} payload sha256 must be the exact 64-hex digest from the manifest"
        );

        // The download URL is constructed from trusted parts (never taken
        // verbatim from the manifest), and the fixture parts must be valid.
        let url = r2::github_release_asset_url(
            "terraphim",
            "terraphim-ai",
            &manifest.release_tag,
            &asset.name,
        )
        .expect("fixture asset must yield a canonical download URL");
        assert!(url.starts_with("https://github.com/"));
        assert!(url.ends_with(&asset.name));
    }
}

#[test]
fn manifest_signature_context_constant_matches_coordinator_contract() {
    // The coordinator signs with `zipsign sign separate --context
    // terraphim-release-manifest-v1` (release_coordinator.py /
    // r2-publish-manifest.sh). The updater must verify under the same context.
    assert_eq!(
        r2::MANIFEST_SIGNATURE_CONTEXT,
        "terraphim-release-manifest-v1"
    );
}
