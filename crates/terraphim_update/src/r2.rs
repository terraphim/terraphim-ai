//! R2 bucket manifest backend for self-updates.
//!
//! This module implements the **R2-first** update contract documented at
//! <https://terraphim.ai/releases/>: the self-update backend is served from our
//! R2 bucket (`downloads.terraphim.ai`) with Ed25519 signature verification,
//! and GitHub Releases is an automatic fallback if R2 is unreachable.
//!
//! Prior to this module the crate only used
//! [`self_update::backends::github`](self_update::backends::github) and silently
//! diverged from the documented contract (Gitea #3096). This module restores
//! fidelity: `check_update`/`update` now consult the R2 manifest first and only
//! fall back to GitHub when R2 is unreachable or the manifest is malformed.
//!
//! The manifest format is intentionally minimal and additive: every asset is a
//! single platform triple with an absolute download URL and an optional
//! detached-signature URL. Signed `.tar.gz` archives (zipsign-embedded) remain
//! the preferred path, reused from [`crate::signature`].

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Default R2 base URL serving the update manifest + binaries.
///
/// Matches the public documentation ("served from our R2 bucket
/// `downloads.terraphim.ai`"). Kept as a `const` so tests and callers agree on
/// the contract; override via [`crate::UpdaterConfig::with_r2_base_url`].
pub const DEFAULT_R2_BASE_URL: &str = "https://downloads.terraphim.ai";

/// A single downloadable asset described by the R2 manifest.
///
/// One entry per supported platform triple (e.g. `x86_64-unknown-linux-gnu`).
/// `signature_url` is optional: signed `.tar.gz` archives carry the signature
/// embedded via zipsign, in which case a detached file is not required.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct R2Asset {
    /// Rust target triple the asset was built for
    /// (e.g. `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`).
    pub target: String,

    /// Absolute URL to the downloadable binary/archive on R2.
    pub url: String,

    /// Optional absolute URL to a detached signature file.
    ///
    /// When `None`, the archive is expected to carry a zipsign-embedded
    /// signature that [`crate::signature::verify_archive_signature`] can read.
    pub signature_url: Option<String>,

    /// Optional hex-encoded SHA-256 of the asset for integrity checks.
    pub sha256: Option<String>,
}

/// R2 release manifest for one binary.
///
/// Serialised as JSON at `{r2_base_url}/{bin_name}/manifest.json`.
/// Designed to be forward-compatible: unknown fields are ignored, optional
/// fields degrade gracefully.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct R2Manifest {
    /// Latest version string (semver, optional `v` prefix tolerated).
    pub version: String,

    /// ISO-8601 release date (informational only).
    #[serde(default)]
    pub release_date: Option<String>,

    /// Platform assets. Order is not significant; selection is by target.
    #[serde(default)]
    pub assets: Vec<R2Asset>,
}

impl R2Manifest {
    /// Select the first asset matching any of the candidate target triples,
    /// in candidate order (i.e. callers pass preferred targets first — e.g.
    /// GNU before MUSL).
    ///
    /// Returns `None` when no asset matches, so callers can decide to fall
    /// back to GitHub rather than failing hard.
    pub fn select_asset<'a>(&'a self, targets: &[String]) -> Option<&'a R2Asset> {
        for target in targets {
            if let Some(asset) = self.assets.iter().find(|a| a.target == *target) {
                return Some(asset);
            }
        }
        None
    }
}

/// Construct the canonical manifest URL for a binary.
///
/// `{base_url}/{bin_name}/manifest.json` — the contract documented at
/// <https://terraphim.ai/releases/>. Kept as a pure function so URL
/// construction is unit-testable without any network access.
pub fn r2_manifest_url(base_url: &str, bin_name: &str) -> String {
    // Trim a trailing slash so callers may or may not include one.
    let trimmed_base = base_url.trim_end_matches('/');
    format!("{}/{}/manifest.json", trimmed_base, bin_name)
}

/// Fetch and parse the R2 manifest for `bin_name`.
///
/// Uses a short, fail-fast timeout so the R2→GitHub fallback decision stays
/// snappy when R2 is unreachable (mirrors the documented "only fall back if R2
/// is unreachable"). Any error — network, non-200, malformed JSON — is
/// surfaced as `Err` so the caller can decide to fall back rather than treat
/// a transient blip as "no update".
pub fn fetch_r2_manifest(base_url: &str, bin_name: &str, timeout: Duration) -> Result<R2Manifest> {
    let url = r2_manifest_url(base_url, bin_name);
    info!("Fetching R2 manifest from {}", url);

    // Reuse the crate's HTTP agent (ureq) for TLS/timeout parity with the
    // downloader, but treat HTTP errors as errors (we want the status code).
    let agent_config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(true)
        .build();
    let agent = ureq::Agent::new_with_config(agent_config);

    let response = agent
        .get(&url)
        .call()
        .map_err(|e| anyhow!("R2 manifest request failed for {}: {}", url, e))?;

    let body = response
        .into_body()
        .read_to_string()
        .map_err(|e| anyhow!("Failed reading R2 manifest body from {}: {}", url, e))?;

    let manifest: R2Manifest = serde_json::from_str(&body)
        .with_context(|| format!("Failed parsing R2 manifest JSON from {}", url))?;

    debug!(
        "R2 manifest for {}: version {} with {} asset(s)",
        bin_name,
        manifest.version,
        manifest.assets.len()
    );

    Ok(manifest)
}

/// Try R2 then, on failure, return a sentinel so callers can branch.
///
/// Convenience wrapper: attempts [`fetch_r2_manifest`] and logs a warning on
/// failure, returning the error so the caller can apply its fallback policy.
pub fn try_fetch_r2_manifest(
    base_url: &str,
    bin_name: &str,
    timeout: Duration,
) -> Result<R2Manifest> {
    match fetch_r2_manifest(base_url, bin_name, timeout) {
        Ok(m) => Ok(m),
        Err(e) => {
            warn!("R2 manifest unavailable, will fall back to GitHub: {}", e);
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
    fn r2_manifest_parses_minimal_json() {
        let json = r#"{ "version": "1.21.8" }"#;
        let manifest: R2Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.21.8");
        assert!(manifest.assets.is_empty());
        assert!(manifest.release_date.is_none());
    }

    #[test]
    fn r2_manifest_parses_full_json() {
        let json = r#"{
            "version": "1.21.8",
            "release_date": "2026-07-10",
            "assets": [
                {
                    "target": "x86_64-unknown-linux-gnu",
                    "url": "https://downloads.terraphim.ai/terraphim-grep/terraphim-grep-1.21.8-x86_64-unknown-linux-gnu.tar.gz",
                    "signature_url": null,
                    "sha256": "deadbeef"
                },
                {
                    "target": "aarch64-apple-darwin",
                    "url": "https://downloads.terraphim.ai/terraphim-grep/terraphim-grep-1.21.8-aarch64-apple-darwin.tar.gz"
                }
            ]
        }"#;
        let manifest: R2Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.21.8");
        assert_eq!(manifest.release_date.as_deref(), Some("2026-07-10"));
        assert_eq!(manifest.assets.len(), 2);
        assert_eq!(manifest.assets[0].target, "x86_64-unknown-linux-gnu");
        assert!(manifest.assets[0].signature_url.is_none());
        assert_eq!(manifest.assets[0].sha256.as_deref(), Some("deadbeef"));
        assert!(manifest.assets[1].signature_url.is_none());
        assert!(manifest.assets[1].sha256.is_none());
    }

    #[test]
    fn r2_manifest_roundtrip_preserves_fields() {
        let manifest = R2Manifest {
            version: "2.0.0".to_string(),
            release_date: Some("2026-08-01".to_string()),
            assets: vec![R2Asset {
                target: "x86_64-pc-windows-msvc".to_string(),
                url: "https://example.com/x.zip".to_string(),
                signature_url: Some("https://example.com/x.zip.sig".to_string()),
                sha256: None,
            }],
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: R2Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }

    #[test]
    fn r2_manifest_ignores_unknown_fields() {
        let json = r#"{ "version": "1.0.0", "future_field": 42 }"#;
        let manifest: R2Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn r2_manifest_rejects_missing_version() {
        let json = r#"{ "assets": [] }"#;
        let result: Result<R2Manifest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn select_asset_picks_first_matching_target() {
        let manifest = R2Manifest {
            version: "1.0.0".to_string(),
            release_date: None,
            assets: vec![
                R2Asset {
                    target: "x86_64-unknown-linux-gnu".to_string(),
                    url: "https://e.com/gnu.tar.gz".to_string(),
                    signature_url: None,
                    sha256: None,
                },
                R2Asset {
                    target: "x86_64-unknown-linux-musl".to_string(),
                    url: "https://e.com/musl.tar.gz".to_string(),
                    signature_url: None,
                    sha256: None,
                },
            ],
        };

        // GNU preferred over MUSL (passed first) and selected.
        let targets = vec![
            "x86_64-unknown-linux-gnu".to_string(),
            "x86_64-unknown-linux-musl".to_string(),
        ];
        let asset = manifest.select_asset(&targets).unwrap();
        assert_eq!(asset.target, "x86_64-unknown-linux-gnu");
        assert_eq!(asset.url, "https://e.com/gnu.tar.gz");
    }

    #[test]
    fn select_asset_falls_through_to_secondary_target() {
        let manifest = R2Manifest {
            version: "1.0.0".to_string(),
            release_date: None,
            assets: vec![R2Asset {
                target: "x86_64-unknown-linux-musl".to_string(),
                url: "https://e.com/musl.tar.gz".to_string(),
                signature_url: None,
                sha256: None,
            }],
        };

        // No GNU asset — MUSL fallback should be selected.
        let targets = vec![
            "x86_64-unknown-linux-gnu".to_string(),
            "x86_64-unknown-linux-musl".to_string(),
        ];
        let asset = manifest.select_asset(&targets).unwrap();
        assert_eq!(asset.target, "x86_64-unknown-linux-musl");
    }

    #[test]
    fn select_asset_returns_none_when_no_target_matches() {
        let manifest = R2Manifest {
            version: "1.0.0".to_string(),
            release_date: None,
            assets: vec![R2Asset {
                target: "aarch64-apple-darwin".to_string(),
                url: "https://e.com/mac.tar.gz".to_string(),
                signature_url: None,
                sha256: None,
            }],
        };

        let targets = vec!["x86_64-unknown-linux-gnu".to_string()];
        assert!(manifest.select_asset(&targets).is_none());
    }

    #[test]
    fn select_asset_returns_none_for_empty_assets() {
        let manifest = R2Manifest {
            version: "1.0.0".to_string(),
            release_date: None,
            assets: vec![],
        };
        let targets = vec!["x86_64-unknown-linux-gnu".to_string()];
        assert!(manifest.select_asset(&targets).is_none());
    }

    #[test]
    fn fetch_r2_manifest_fails_fast_on_unreachable_host() {
        // No network dependency: an unroutable host must fail quickly so the
        // documented GitHub fallback remains snappy.
        let result = fetch_r2_manifest(
            "http://127.0.0.1:9",
            "terraphim-grep",
            Duration::from_millis(500),
        );
        assert!(result.is_err(), "expected error for unreachable host");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("manifest") || err.contains("R2") || err.contains("connect"),
            "error should mention manifest/R2/connect: {}",
            err
        );
    }
}
