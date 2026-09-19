//! R2 bucket manifest backend for self-updates.
//!
//! This module implements the **R2-first** update contract documented at
//! <https://terraphim.ai/releases/>: the self-update backend is served from our
//! R2 bucket (`downloads.terraphim.ai`) with Ed25519 signature verification,
//! and GitHub Releases is an automatic fallback if R2 is unreachable.
//!
//! # Coherent trust contract (coordinator-owned, P1-1/P1-2)
//!
//! There is exactly **one** authoritative release manifest: the canonical
//! `release-manifest-v1` produced, signed, and published by
//! `release-coordinator.yml` (see `.release/release-manifest.schema.json`).
//! The coordinator publishes two R2 objects from the *same signed bytes*:
//!
//! - `releases/<tag>/manifest.json` (+ `.sig`) — immutable, conditional-put,
//!   approval-bound release evidence;
//! - `<component>/manifest.json` (+ `.sig`) — the **signed stable discovery
//!   pointer** this module fetches (byte-identical content, so the single
//!   detached signature verifies both).
//!
//! Before manifest bytes may influence update selection, version reporting,
//! or download URLs, they are cryptographically authenticated: the detached
//! zipsign/Ed25519 signature (`manifest.json.sig`) is fetched and verified
//! against the embedded release key under the context
//! [`MANIFEST_SIGNATURE_CONTEXT`]. A missing, unreadable, or invalid
//! signature is a hard error (fail closed) — there is no unsigned path.
//!
//! Asset payloads are *not* located by manifest URLs (the canonical schema
//! deliberately carries none). The download URL is constructed from trusted
//! configuration plus validated manifest parts via
//! [`github_release_asset_url`], the payload SHA-256 must equal the signed
//! manifest's exact digest, and the zipsign-embedded archive signature is
//! still verified before install (defense in depth, unchanged).

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Default R2 base URL serving the signed update manifest pointer.
///
/// Matches the public documentation ("served from our R2 bucket
/// `downloads.terraphim.ai`"). Kept as a `const` so tests and callers agree on
/// the contract; override via [`crate::UpdaterConfig::with_r2_base_url`].
pub const DEFAULT_R2_BASE_URL: &str = "https://downloads.terraphim.ai";

/// zipsign context the release coordinator signs the canonical
/// `release-manifest-v1` under (`zipsign sign separate --context ...` in
/// `release_coordinator.py` / `r2-publish-manifest.sh`). Verification must
/// use exactly this context; anything else fails closed.
pub const MANIFEST_SIGNATURE_CONTEXT: &str = "terraphim-release-manifest-v1";

/// The only `release-manifest-v1` schema version this consumer understands.
pub const SUPPORTED_SCHEMA_VERSION: &str = "1.0.0";

/// The archive format the self-updater installs.
const INSTALLABLE_FORMAT: &str = "tar.gz";

/// A single asset entry of the canonical release manifest.
///
/// Consumer-side view of the `asset` definition in
/// `.release/release-manifest.schema.json`. Unknown manifest fields are
/// ignored for forward compatibility, but every field the updater relies on
/// is required and strictly validated by [`parse_release_manifest`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseAsset {
    /// Asset file name on the GitHub release (e.g.
    /// `terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz`).
    pub name: String,

    /// Owning component (`terraphim-server`, `terraphim-agent`,
    /// `terraphim-grep`).
    pub component: String,

    /// Package format; the updater only installs `tar.gz` archives.
    pub format: String,

    /// Rust target triple the asset was built for
    /// (e.g. `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`).
    pub target: String,

    /// CPU architecture (`x86_64`, `aarch64`, `universal`).
    pub arch: String,

    /// Operating system (`linux`, `macos`, `windows`).
    pub os: String,

    /// Exact hex-encoded SHA-256 of the payload bytes (enforced after
    /// download — this is a live integrity check, not schema theater).
    pub sha256: String,
}

/// Consumer view of the canonical coordinator-owned release manifest
/// (`release-manifest-v1`).
///
/// Served at `{r2_base_url}/{bin_name}/manifest.json` (the signed stable
/// discovery pointer) and authenticated by the detached signature at
/// `{r2_base_url}/{bin_name}/manifest.json.sig` before parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseManifest {
    /// Manifest schema version; must equal [`SUPPORTED_SCHEMA_VERSION`].
    pub schema_version: String,

    /// Release version without `v` prefix (semver).
    pub release_version: String,

    /// Release tag (`v<release_version>`), used to locate the immutable
    /// GitHub release holding the payload bytes.
    pub release_tag: String,

    /// All release assets. Selection is exact by component, format, and
    /// target triple (see [`ReleaseManifest::select_asset`]).
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseManifest {
    /// Select the asset for `component` in the installable `tar.gz` format
    /// matching any of the candidate target triples, in candidate order
    /// (callers pass preferred targets first — e.g. GNU before MUSL).
    ///
    /// Returns `None` when no asset matches, so callers can decide to fall
    /// back to GitHub rather than failing hard.
    pub fn select_asset<'a>(
        &'a self,
        component: &str,
        targets: &[String],
    ) -> Option<&'a ReleaseAsset> {
        for target in targets {
            if let Some(asset) = self.assets.iter().find(|a| {
                a.component == component && a.format == INSTALLABLE_FORMAT && a.target == *target
            }) {
                return Some(asset);
            }
        }
        None
    }
}

/// Validate a release tag against the release-tag grammar:
/// `vMAJOR.MINOR.PATCH` with optional `-prerelease` / `+build` suffixes.
/// Anything else (path traversal, empty, missing `v`) is rejected so the tag
/// can be safely interpolated into a download URL path segment.
fn is_valid_release_tag(tag: &str) -> bool {
    let Some(rest) = tag.strip_prefix('v') else {
        return false;
    };
    let (without_build, build) = match rest.split_once('+') {
        Some((core, build)) => (core, Some(build)),
        None => (rest, None),
    };
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (without_build, None),
    };
    let parts: Vec<&str> = core.split('.').collect();
    let core_ok = parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.len() <= 10 && p.bytes().all(|b| b.is_ascii_digit()));
    if !core_ok {
        return false;
    }
    let suffix_ok = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
    };
    prerelease.is_none_or(suffix_ok) && build.is_none_or(suffix_ok)
}

/// Validate an asset file name for safe URL path-segment interpolation:
/// printable basename characters only, no separators, no traversal.
fn is_valid_asset_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && !name.contains("..")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
        && name
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphanumeric())
}

/// Validate a hex-encoded lowercase SHA-256 digest.
fn is_valid_sha256_hex(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Construct the canonical manifest URL for a binary.
///
/// `{base_url}/{bin_name}/manifest.json` — the signed stable discovery
/// pointer published by the release coordinator (byte-identical to the
/// immutable `releases/<tag>/manifest.json`). Kept as a pure function so URL
/// construction is unit-testable without any network access.
pub fn r2_manifest_url(base_url: &str, bin_name: &str) -> String {
    // Trim a trailing slash so callers may or may not include one.
    let trimmed_base = base_url.trim_end_matches('/');
    format!("{}/{}/manifest.json", trimmed_base, bin_name)
}

/// Construct the detached-signature URL for a binary's manifest pointer.
pub fn r2_manifest_signature_url(base_url: &str, bin_name: &str) -> String {
    format!("{}.sig", r2_manifest_url(base_url, bin_name))
}

/// Parse and validate canonical `release-manifest-v1` bytes.
///
/// Fail closed on malformed JSON, an unknown `schema_version`, a release tag
/// outside the tag grammar, an unparseable release version, or any asset with
/// an unsafe name or a non-SHA-256 digest. Callers must authenticate the
/// bytes with [`verify_detached_signature`] *before* parsing.
pub fn parse_release_manifest(json: &str) -> Result<ReleaseManifest> {
    let manifest: ReleaseManifest =
        serde_json::from_str(json).context("Failed parsing release manifest JSON")?;

    if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(anyhow!(
            "unsupported release manifest schema_version {:?} (expected {:?})",
            manifest.schema_version,
            SUPPORTED_SCHEMA_VERSION
        ));
    }
    if !is_valid_release_tag(&manifest.release_tag) {
        return Err(anyhow!(
            "release_tag {:?} is outside the vX.Y.Z tag grammar",
            manifest.release_tag
        ));
    }
    semver::Version::parse(&manifest.release_version).map_err(|e| {
        anyhow!(
            "release_version {:?} is not valid semver: {}",
            manifest.release_version,
            e
        )
    })?;
    if manifest.release_tag != format!("v{}", manifest.release_version) {
        return Err(anyhow!(
            "release_tag {:?} does not equal v<release_version> {:?}",
            manifest.release_tag,
            manifest.release_version
        ));
    }
    for asset in &manifest.assets {
        if !is_valid_asset_name(&asset.name) {
            return Err(anyhow!(
                "asset name {:?} is unsafe for URL construction",
                asset.name
            ));
        }
        if !is_valid_sha256_hex(&asset.sha256) {
            return Err(anyhow!(
                "asset {:?} does not carry an exact lowercase SHA-256 digest",
                asset.name
            ));
        }
    }

    Ok(manifest)
}

/// Verify a detached zipsign/Ed25519 signature over `data`.
///
/// Replicates `zipsign verify separate --context terraphim-release-manifest-v1`
/// against the embedded release key: the signature file is a zipsign
/// signature block (magic header + count + Ed25519 signatures) over the
/// SHA-512 prehash of `data`, verified with the manifest context as the
/// Ed25519 context string.
///
/// Fails closed: any malformed input, wrong key, wrong context, or missing
/// signature is an error; there is no "unsigned but proceed" outcome.
pub fn verify_detached_signature(
    data: &[u8],
    signature_bytes: &[u8],
    public_key_b64: &str,
) -> Result<()> {
    if signature_bytes.is_empty() {
        return Err(anyhow!(
            "detached manifest signature is missing (fail closed)"
        ));
    }

    let key_bytes = base64::engine::general_purpose::STANDARD
        .decode(public_key_b64)
        .context("Failed to decode release public key base64")?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow!("release public key must be exactly 32 bytes"))?;
    let keys = zipsign_api::verify::collect_keys(std::iter::once(Ok(key_array)))
        .context("Failed to parse release public key")?;

    let signatures = zipsign_api::verify::read_signatures(&mut Cursor::new(signature_bytes))
        .context("detached manifest signature block is malformed")?;
    let prehashed = zipsign_api::Prehash::calculate(&mut Cursor::new(data))
        .context("Failed to prehash manifest bytes")?;

    zipsign_api::verify::find_match(
        &keys,
        &signatures,
        &prehashed,
        Some(MANIFEST_SIGNATURE_CONTEXT.as_bytes()),
    )
    .map_err(|_| {
        anyhow!(
            "detached manifest signature does not verify under context {:?} with the embedded release key",
            MANIFEST_SIGNATURE_CONTEXT
        )
    })?;
    Ok(())
}

/// Compute the lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

/// Enforce the exact payload digest from the signed manifest.
pub fn verify_payload_sha256(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let actual = sha256_hex(bytes);
    if actual != expected_hex {
        return Err(anyhow!(
            "payload SHA-256 mismatch: signed manifest declares {}, downloaded bytes hash to {} (fail closed)",
            expected_hex,
            actual
        ));
    }
    Ok(())
}

/// Construct the download URL for a manifest asset from trusted parts.
///
/// The canonical schema intentionally carries no asset URLs, so the URL is
/// built from the caller's trusted repository coordinates plus the
/// cryptographically authenticated release tag and a strictly validated
/// asset name. The result is always an `https://github.com/...` URL —
/// insecure schemes and arbitrary hosts can never be introduced through
/// manifest content.
pub fn github_release_asset_url(
    repo_owner: &str,
    repo_name: &str,
    release_tag: &str,
    asset_name: &str,
) -> Result<String> {
    if !is_valid_release_tag(release_tag) {
        return Err(anyhow!(
            "release tag {:?} is outside the vX.Y.Z tag grammar",
            release_tag
        ));
    }
    if !is_valid_asset_name(asset_name) {
        return Err(anyhow!(
            "asset name {:?} is unsafe for URL construction",
            asset_name
        ));
    }
    for part in [repo_owner, repo_name] {
        if part.is_empty()
            || !part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
        {
            return Err(anyhow!("repository coordinate {:?} is invalid", part));
        }
    }
    Ok(format!(
        "https://github.com/{}/{}/releases/download/{}/{}",
        repo_owner, repo_name, release_tag, asset_name
    ))
}

/// Fetch, authenticate, and parse the signed release manifest for `bin_name`.
///
/// Both the manifest pointer and its detached signature must fetch
/// successfully; the signature is verified against the embedded release key
/// under [`MANIFEST_SIGNATURE_CONTEXT`] before the bytes are parsed. Uses a
/// short, fail-fast timeout so the R2→GitHub fallback decision stays snappy
/// when R2 is unreachable. Any error — network, non-200, missing/invalid
/// signature, malformed or off-schema JSON — is surfaced as `Err` so the
/// caller can apply its fallback policy; unauthenticated bytes never reach
/// update selection.
pub fn fetch_signed_manifest(
    base_url: &str,
    bin_name: &str,
    timeout: Duration,
) -> Result<ReleaseManifest> {
    let url = r2_manifest_url(base_url, bin_name);
    let signature_url = r2_manifest_signature_url(base_url, bin_name);
    info!("Fetching signed R2 manifest from {}", url);

    // Reuse the crate's HTTP agent (ureq) for TLS/timeout parity with the
    // downloader, but treat HTTP errors as errors (we want the status code).
    let agent_config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(true)
        .build();
    let agent = ureq::Agent::new_with_config(agent_config);

    let fetch = |url: &str| -> Result<Vec<u8>> {
        agent
            .get(url)
            .call()
            .map_err(|e| anyhow!("R2 request failed for {}: {}", url, e))?
            .into_body()
            .read_to_vec()
            .map_err(|e| anyhow!("Failed reading R2 body from {}: {}", url, e))
    };

    let body = fetch(&url)?;
    // Fail closed: without the detached signature the manifest bytes are
    // unauthenticated and must never influence update decisions.
    let signature_bytes = fetch(&signature_url)
        .map_err(|e| anyhow!("detached manifest signature unavailable: {}", e))?;

    verify_detached_signature(
        &body,
        &signature_bytes,
        crate::signature::get_embedded_public_key(),
    )?;

    let body_text = String::from_utf8(body).context("R2 manifest is not valid UTF-8")?;
    let manifest = parse_release_manifest(&body_text)
        .with_context(|| format!("Invalid signed R2 manifest from {}", url))?;

    debug!(
        "Signed R2 manifest for {}: release {} ({}) with {} asset(s)",
        bin_name,
        manifest.release_version,
        manifest.release_tag,
        manifest.assets.len()
    );

    Ok(manifest)
}

/// Try R2 then, on failure, return the error so callers can branch.
///
/// Convenience wrapper: attempts [`fetch_signed_manifest`] and logs a warning
/// on failure, returning the error so the caller can apply its fallback
/// policy.
pub fn try_fetch_signed_manifest(
    base_url: &str,
    bin_name: &str,
    timeout: Duration,
) -> Result<ReleaseManifest> {
    match fetch_signed_manifest(base_url, bin_name, timeout) {
        Ok(m) => Ok(m),
        Err(e) => {
            warn!(
                "Signed R2 manifest unavailable, will fall back to GitHub: {}",
                e
            );
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r2_manifest_url_trims_trailing_slash() {
        let url = r2_manifest_url("https://downloads.terraphim.ai/", "terraphim-grep");
        assert_eq!(
            url,
            "https://downloads.terraphim.ai/terraphim-grep/manifest.json"
        );
    }

    #[test]
    fn r2_manifest_url_without_trailing_slash() {
        let url = r2_manifest_url("https://downloads.terraphim.ai", "terraphim-agent");
        assert_eq!(
            url,
            "https://downloads.terraphim.ai/terraphim-agent/manifest.json"
        );
    }

    #[test]
    fn r2_manifest_url_respects_custom_base() {
        let url = r2_manifest_url("https://staging.terraphim.dev/", "terraphim-grep");
        assert_eq!(
            url,
            "https://staging.terraphim.dev/terraphim-grep/manifest.json"
        );
    }

    #[test]
    fn r2_manifest_signature_url_appends_sig() {
        let url = r2_manifest_signature_url("https://downloads.terraphim.ai/", "terraphim-grep");
        assert_eq!(
            url,
            "https://downloads.terraphim.ai/terraphim-grep/manifest.json.sig"
        );
    }

    #[test]
    fn release_tag_validation_accepts_tag_grammar() {
        for tag in [
            "v1.2.3",
            "v0.0.1",
            "v1.21.8",
            "v1.2.3-rc.1",
            "v1.2.3+build.5",
        ] {
            assert!(is_valid_release_tag(tag), "{tag} must be accepted");
        }
    }

    #[test]
    fn release_tag_validation_rejects_non_tags() {
        for tag in [
            "",
            "1.2.3",
            "v1.2",
            "v1.2.3.4",
            "v1.2.x",
            "../v1.2.3",
            "v1.2.3/evil",
            "v1.2.3\n",
            "V1.2.3",
        ] {
            assert!(!is_valid_release_tag(tag), "{tag:?} must be rejected");
        }
    }

    #[test]
    fn asset_name_validation_rejects_traversal_and_injection() {
        for name in [
            "",
            "..",
            "../evil",
            "a/b",
            ".hidden",
            "name with spaces",
            "name\ninject",
            &"a".repeat(300),
        ] {
            assert!(!is_valid_asset_name(name), "{name:?} must be rejected");
        }
        for name in [
            "terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
            "checksums.txt",
            "terraphim_server",
        ] {
            assert!(is_valid_asset_name(name), "{name:?} must be accepted");
        }
    }

    #[test]
    fn sha256_validation_is_exact_lowercase_hex() {
        assert!(is_valid_sha256_hex(&"a".repeat(64)));
        assert!(!is_valid_sha256_hex(&"a".repeat(63)));
        assert!(!is_valid_sha256_hex(&"A".repeat(64)));
        assert!(!is_valid_sha256_hex(&"g".repeat(64)));
    }

    #[test]
    fn verify_payload_sha256_enforces_exact_digest() {
        let payload = b"payload-bytes";
        let digest = sha256_hex(payload);
        verify_payload_sha256(payload, &digest).unwrap();
        assert!(verify_payload_sha256(b"other", &digest).is_err());
        assert!(verify_payload_sha256(payload, &"0".repeat(64)).is_err());
    }

    // ------------------------------------------------------------------
    // P1-1/P1-2: signed canonical manifest contract tests.
    //
    // These tests pin the coordinator-owned `release-manifest-v1` trust
    // contract: the updater must cryptographically authenticate manifest
    // bytes (detached zipsign/Ed25519, context terraphim-release-manifest-v1)
    // before they can influence update selection, version reporting, or
    // download URLs, and must fail closed on any deviation.
    // ------------------------------------------------------------------

    /// Deterministic test signing key (never the production key).
    fn test_signing_key() -> zipsign_api::SigningKey {
        zipsign_api::SigningKey::from_bytes(&[7u8; 32])
    }

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    fn sign_detached(data: &[u8], key: &zipsign_api::SigningKey, context: &str) -> Vec<u8> {
        let prehash = zipsign_api::Prehash::calculate(&mut std::io::Cursor::new(data)).unwrap();
        zipsign_api::sign::gather_signature_data(
            std::slice::from_ref(key),
            &prehash,
            Some(context.as_bytes()),
        )
        .unwrap()
    }

    fn canonical_manifest_json() -> String {
        r#"{
            "schema_version": "1.0.0",
            "release_version": "1.21.8",
            "release_tag": "v1.21.8",
            "sources": {
                "terraphim-ai": {
                    "gitea_sha": "1111111111111111111111111111111111111111",
                    "github_sha": "1111111111111111111111111111111111111111",
                    "tree_sha": "2222222222222222222222222222222222222222",
                    "workflow_run_id": 33380001
                },
                "terraphim-clients": {
                    "gitea_sha": "3333333333333333333333333333333333333333",
                    "github_sha": "3333333333333333333333333333333333333333",
                    "tree_sha": "4444444444444444444444444444444444444444",
                    "workflow_run_id": 33360001
                }
            },
            "assets": [
                {
                    "name": "terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
                    "component": "terraphim-agent",
                    "format": "tar.gz",
                    "target": "x86_64-unknown-linux-gnu",
                    "arch": "x86_64",
                    "os": "linux",
                    "source_repo": "terraphim-clients",
                    "source_sha": "3333333333333333333333333333333333333333",
                    "sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "size_bytes": 1048576,
                    "signature": "embedded"
                },
                {
                    "name": "terraphim-agent-1.21.8-x86_64-unknown-linux-musl.tar.gz",
                    "component": "terraphim-agent",
                    "format": "tar.gz",
                    "target": "x86_64-unknown-linux-musl",
                    "arch": "x86_64",
                    "os": "linux",
                    "source_repo": "terraphim-clients",
                    "source_sha": "3333333333333333333333333333333333333333",
                    "sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                    "size_bytes": 1048576,
                    "signature": "embedded"
                },
                {
                    "name": "terraphim-grep-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
                    "component": "terraphim-grep",
                    "format": "tar.gz",
                    "target": "x86_64-unknown-linux-gnu",
                    "arch": "x86_64",
                    "os": "linux",
                    "source_repo": "terraphim-clients",
                    "source_sha": "3333333333333333333333333333333333333333",
                    "sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "size_bytes": 1048576,
                    "signature": "embedded"
                }
            ],
            "required_channels": [
                "github_release",
                "r2_stable_manifest",
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin"
            ],
            "central_channels": ["github_release", "r2_stable_manifest"],
            "downstream_channels": [
                "homebrew_tap_pr",
                "aur_terraphim_clients_bin",
                "omarchy_terraphim_clients_bin"
            ],
            "created_at": "2026-09-12T10:00:00Z"
        }"#
        .to_string()
    }

    #[test]
    fn detached_signature_valid_roundtrip() {
        let key = test_signing_key();
        let data = canonical_manifest_json().into_bytes();
        let sig = sign_detached(&data, &key, MANIFEST_SIGNATURE_CONTEXT);
        let public_b64 = b64(&key.verifying_key().to_bytes());
        verify_detached_signature(&data, &sig, &public_b64)
            .expect("valid detached signature must verify");
    }

    #[test]
    fn detached_signature_tampered_manifest_fails_closed() {
        let key = test_signing_key();
        let data = canonical_manifest_json().into_bytes();
        let sig = sign_detached(&data, &key, MANIFEST_SIGNATURE_CONTEXT);
        let public_b64 = b64(&key.verifying_key().to_bytes());
        let mut tampered = data.clone();
        tampered.extend_from_slice(b" ");
        assert!(
            verify_detached_signature(&tampered, &sig, &public_b64).is_err(),
            "tampered manifest bytes must not verify"
        );
    }

    #[test]
    fn detached_signature_missing_signature_fails_closed() {
        let key = test_signing_key();
        let data = canonical_manifest_json().into_bytes();
        let public_b64 = b64(&key.verifying_key().to_bytes());
        assert!(
            verify_detached_signature(&data, &[], &public_b64).is_err(),
            "missing signature bytes must fail closed"
        );
        assert!(
            verify_detached_signature(&data, b"not-a-zipsign-file", &public_b64).is_err(),
            "garbage signature bytes must fail closed"
        );
    }

    #[test]
    fn detached_signature_wrong_context_fails_closed() {
        let key = test_signing_key();
        let data = canonical_manifest_json().into_bytes();
        // Signed under a different context than the canonical manifest context.
        let sig = sign_detached(&data, &key, "terraphim-update-pointer-v1");
        let public_b64 = b64(&key.verifying_key().to_bytes());
        assert!(
            verify_detached_signature(&data, &sig, &public_b64).is_err(),
            "a signature under the wrong context must not authenticate the manifest"
        );
    }

    #[test]
    fn detached_signature_wrong_key_fails_closed() {
        let key = test_signing_key();
        let other_key = zipsign_api::SigningKey::from_bytes(&[9u8; 32]);
        let data = canonical_manifest_json().into_bytes();
        let sig = sign_detached(&data, &other_key, MANIFEST_SIGNATURE_CONTEXT);
        let public_b64 = b64(&key.verifying_key().to_bytes());
        assert!(
            verify_detached_signature(&data, &sig, &public_b64).is_err(),
            "a signature from a different key must not verify"
        );
    }

    #[test]
    fn parse_signed_manifest_accepts_canonical_layout() {
        let manifest = parse_release_manifest(&canonical_manifest_json())
            .expect("canonical release-manifest-v1 must parse");
        assert_eq!(manifest.schema_version, "1.0.0");
        assert_eq!(manifest.release_version, "1.21.8");
        assert_eq!(manifest.release_tag, "v1.21.8");
        assert_eq!(manifest.assets.len(), 3);
    }

    #[test]
    fn parse_release_manifest_rejects_wrong_schema_version() {
        let json = canonical_manifest_json().replace("\"1.0.0\"", "\"2.0.0\"");
        assert!(
            parse_release_manifest(&json).is_err(),
            "unknown schema_version must fail closed"
        );
    }

    #[test]
    fn parse_release_manifest_rejects_missing_release_version() {
        let json = r#"{
            "schema_version": "1.0.0",
            "release_tag": "v1.21.8",
            "assets": []
        }"#;
        assert!(
            parse_release_manifest(json).is_err(),
            "a manifest without release_version must fail closed"
        );
    }

    #[test]
    fn parse_release_manifest_rejects_malformed_release_tag() {
        let json = canonical_manifest_json().replace("\"v1.21.8\"", "\"v9.9.9\"");
        // release_tag/release_version must stay consistent (replay pinning).
        assert!(
            parse_release_manifest(&json).is_err(),
            "a release_tag inconsistent with release_version must fail closed"
        );
        let traversal = canonical_manifest_json().replace("\"v1.21.8\"", "\"v1.21.8/x\"");
        assert!(
            parse_release_manifest(&traversal).is_err(),
            "a release_tag outside the tag grammar must fail closed"
        );
    }

    #[test]
    fn parse_release_manifest_rejects_bad_asset_digest() {
        let json = canonical_manifest_json().replace(
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "deadbeef",
        );
        assert!(
            parse_release_manifest(&json).is_err(),
            "assets without an exact SHA-256 digest must fail closed"
        );
    }

    #[test]
    fn select_asset_is_exact_by_component_target_and_format() {
        let manifest = parse_release_manifest(&canonical_manifest_json()).unwrap();
        let targets = vec![
            "x86_64-unknown-linux-gnu".to_string(),
            "x86_64-unknown-linux-musl".to_string(),
        ];

        let agent = manifest
            .select_asset("terraphim-agent", &targets)
            .expect("agent asset must be selected");
        assert_eq!(agent.component, "terraphim-agent");
        assert_eq!(agent.target, "x86_64-unknown-linux-gnu");
        assert_eq!(agent.format, "tar.gz");
        assert_eq!(
            agent.sha256,
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        );

        let grep = manifest
            .select_asset("terraphim-grep", &targets)
            .expect("grep asset must be selected");
        assert_eq!(grep.component, "terraphim-grep");
        assert_eq!(
            grep.sha256,
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        );

        // GNU preference falls through to MUSL when only MUSL exists.
        let musl_only = vec!["x86_64-unknown-linux-musl".to_string()];
        let musl = manifest
            .select_asset("terraphim-agent", &musl_only)
            .unwrap();
        assert_eq!(musl.target, "x86_64-unknown-linux-musl");

        // A component with no matching asset selects nothing (fallback path).
        assert!(
            manifest
                .select_asset("terraphim-server", &targets)
                .is_none()
        );
        assert!(
            manifest
                .select_asset("terraphim-agent", &["aarch64-apple-darwin".to_string()])
                .is_none()
        );
    }

    #[test]
    fn select_asset_never_picks_non_installable_formats() {
        let json = canonical_manifest_json().replace(
            "\"name\": \"terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz\",\n                    \"component\": \"terraphim-agent\",\n                    \"format\": \"tar.gz\"",
            "\"name\": \"terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.deb\",\n                    \"component\": \"terraphim-agent\",\n                    \"format\": \"deb\"",
        );
        let manifest = parse_release_manifest(&json).unwrap();
        let targets = vec!["x86_64-unknown-linux-gnu".to_string()];
        // The deb must not be selected for self-update; the remaining gnu
        // asset is terraphim-grep's, which is a different component.
        assert!(manifest.select_asset("terraphim-agent", &targets).is_none());
    }

    #[test]
    fn github_release_asset_url_constructs_canonical_https_url() {
        let url = github_release_asset_url(
            "terraphim",
            "terraphim-ai",
            "v1.21.8",
            "terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
        )
        .unwrap();
        assert_eq!(
            url,
            "https://github.com/terraphim/terraphim-ai/releases/download/v1.21.8/terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz"
        );
    }

    #[test]
    fn github_release_asset_url_rejects_insecure_or_arbitrary_parts() {
        // Path traversal / host injection via asset name must fail closed.
        for bad_name in [
            "../evil.tar.gz",
            "a/b.tar.gz",
            "..",
            ".hidden",
            "name with spaces.tar.gz",
            "name\ninject.tar.gz",
        ] {
            assert!(
                github_release_asset_url("terraphim", "terraphim-ai", "v1.21.8", bad_name).is_err(),
                "asset name {bad_name:?} must be rejected"
            );
        }
        // Tag grammar is the release-tag grammar, nothing else.
        for bad_tag in ["1.21.8", "v1.2", "../../refs", "v1.2.3/evil", ""] {
            assert!(
                github_release_asset_url(
                    "terraphim",
                    "terraphim-ai",
                    bad_tag,
                    "terraphim-agent-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
                )
                .is_err(),
                "release tag {bad_tag:?} must be rejected"
            );
        }
    }

    #[test]
    fn fetch_signed_manifest_requires_sig_sidecar_and_fails_fast_on_unreachable_host() {
        // No network dependency: an unroutable host must fail quickly so the
        // documented GitHub fallback remains snappy.
        let result = fetch_signed_manifest(
            "http://127.0.0.1:9",
            "terraphim-grep",
            Duration::from_millis(500),
        );
        assert!(result.is_err(), "expected error for unreachable host");
    }

    #[test]
    fn legacy_unsigned_manifest_shape_is_not_accepted() {
        // The pre-contract per-binary manifest (`{"version": ...}`, no
        // schema_version/release_tag, no detached signature) must be rejected:
        // unauthenticated bytes can never drive update selection again.
        let legacy = r#"{ "version": "9.9.9", "assets": [] }"#;
        assert!(parse_release_manifest(legacy).is_err());
    }
}
