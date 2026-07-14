# Implementation Plan: R2 update fallback (Gitea #3096)

**Status**: Draft → Implementing
**Research**: Gitea issue #3096 + source analysis of `crates/terraphim_update`
**Author**: Echo (implementation-swarm)

## Summary
The crate documents an R2-first update contract (`downloads.terraphim.ai/<binary>/manifest.json`,
Ed25519 verification, GitHub fallback) but only implements the GitHub backend. This plan adds
an R2 manifest backend and wires it as the primary source, with the existing GitHub path kept
verbatim as fallback.

## Scope
**In:**
- New `r2` module: `R2Manifest`, `R2Asset`, `fetch_manifest`, asset selection per target.
- New `UpdaterConfig` fields: `r2_base_url`, `github_fallback` (default true).
- R2-first flow in `check_update` + `update`; GitHub retained as fallback.
- Reuse `downloader::download_with_retry`, `signature::verify_archive_signature`,
  `install_verified_archive`.

**Out (Avoid-At-All-Cost):**
- New HTTP client / TLS stack (reuse `ureq` via `downloader`).
- New signature scheme (reuse zipsign embedded key).
- Multipart/signature-file protocol (zipsign embeds signature in archive).
- Touching the `scheduler`/`notification`/`state` modules.
- Changing `update_with_verification` (already a manual GitHub flow; leave alone).

## API design
```rust
pub mod r2;

/// R2 manifest entry: one per platform asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Asset {
    pub target: String,          // e.g. "x86_64-unknown-linux-gnu"
    pub url: String,             // absolute download URL
    pub signature_url: Option<String>, // optional detached sig
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Manifest {
    pub version: String,
    pub release_date: Option<String>,
    pub assets: Vec<R2Asset>,
}

pub fn r2_manifest_url(base_url: &str, bin_name: &str) -> String;
pub fn fetch_r2_manifest(base_url: &str, bin_name: &str, timeout: Duration) -> Result<R2Manifest>;
pub fn select_asset<'a>(manifest: &'a R2Manifest, targets: &[String]) -> Option<&'a R2Asset>;

// UpdaterConfig additions:
pub struct UpdaterConfig {
    // ...existing...
    pub r2_base_url: String,     // default "https://downloads.terraphim.ai"
    pub github_fallback: bool,   // default true
}
```

## Fallback semantics
- `check_update`: try R2 first. On any R2 error (network, parse, 404), if `github_fallback`,
  fall back to GitHub. Return R2 result on success.
- `update`: try R2 first. On R2 success → download + verify + install. On R2 failure → GitHub path.

## Test strategy
- Unit: manifest JSON parse/roundtrip, asset selection ordering, URL construction.
- Fallback decision: R2 error → GitHub (mocked by pointing at unreachable host).
- Integration: `fetch_r2_manifest` against unreachable host returns Err (no network reliance).

## Steps
1. Add `r2.rs` module + wire into `lib.rs`.
2. Add `UpdaterConfig` fields + builder methods.
3. Implement R2-first `check_update`.
4. Implement R2-first `update`.
5. Tests + quality gates.
