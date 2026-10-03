# Release Channels

This document defines which release channel is **canonical** for fleet-internal agents consuming
`terraphim_grep` and `terraphim_agent`, and how to interpret a version mismatch between channels.

Read this before reporting a "release failure". A crates.io or GitHub Releases version that trails
the private registry is expected and is **not** a failure for fleet-internal work.

## Channel precedence

| Rank | Channel | Role | Authority |
|------|---------|------|-----------|
| 1 | Gitea private cargo registry (owner `terraphim`) | **Canonical for the fleet** | Blessed version — agents install from here |
| 2 | crates.io | Public mirror | Lags the canonical channel; informational |
| 3 | GitHub Releases (`terraphim/terraphim-ai` tags) | Public binary distribution | Lags the canonical channel; informational |

The canonical channel is the **Gitea private cargo registry**, owner `terraphim`, hosted at
`$GITEA_URL`. Public channels are downstream publication targets, not the source of truth for what
the fleet is expected to run.

## Currently blessed versions

As verified on 2026-08-08:

| Crate | Gitea (canonical) | crates.io | GitHub release tag |
|-------|-------------------|-----------|--------------------|
| `terraphim_grep` | **1.21.1** | 1.20.5 | v1.20.5 |
| `terraphim_agent` | **1.21.1** | 1.20.5 | v1.20.5 |

The Gitea git tag is `v1.21.0`; the blessed **package** version is 1.21.1. Package names in the
registry use underscores (`terraphim_grep`, `terraphim_agent`), not hyphens.

## Querying the canonical channel

Per-package (returns a **JSON array**, one object per published version):

```bash
curl -s -H "Authorization: token $GITEA_TOKEN" \
  "$GITEA_URL/api/v1/packages/terraphim/cargo/terraphim_grep" | jq -r '.[].version'
```

List all cargo packages for the owner:

```bash
curl -s -H "Authorization: token $GITEA_TOKEN" \
  "$GITEA_URL/api/v1/packages/terraphim?type=cargo"
```

Each array element carries `id`, `owner.login`, `repository` (null), `creator`, `type` (`"cargo"`),
`name`, `version`, `html_url`, `created_at`.

> **There is no `latest` field on this API.** The blessed version is the **maximum version present
> in the array** — compute it by semver ordering. Any agent or script that reads `.latest` is
> reading a field that does not exist and will produce a false negative.

Never paste a token value into documentation, commit messages, or issue comments. Use the
`$GITEA_TOKEN` and `$GITEA_URL` environment variables (loaded via `source ~/.profile`).

## Interpreting channel lag

Public-channel lag is **CONDITIONAL, not a failure**, for fleet-internal purposes:

- Gitea canonical version present and installable → fleet release is **good**.
- crates.io / GitHub Releases behind the canonical version → **warning only**. It is a publication
  backlog item, tracked separately as ops work; it does not block fleet-internal consumers.
- Report a release failure only when the **canonical** channel is missing the expected version or
  the package cannot be installed from it.

Continuous verification of these channels is owned by `release-guardian`, which lives outside this
repository in the `cto-executive-system` project. Do not add automated registry checking here.

Evidence for the 2026-08-08 assessment (paths within the `release-guardian` project, not links):

- `reports/2026-08-08-terraphim-ai-clients.md`
- `adf-ledger.jsonl`

The recorded verdict was `CONDITIONAL_PASS`: checks `R6`, `R6cargo` and `R6cargo-rel` were WARN
(public-channel lag), while `R6canon` PASSed — i.e. the fleet bless of 1.21.1 stands. Install smokes
`R7:*` PASSed.

## Install smoke testing

When the private sparse index is not reachable from the environment under test, the supported smoke
is a **path install from the `terraphim-clients` checkout, into a temporary root**:

- Install path: `path=terraphim-clients` (local path install, not registry install).
- Destination: a **temp root only** — never the developer's or agent's real `~/.cargo/bin`.

This keeps the smoke hermetic and prevents a smoke run from silently changing which binary the host
resolves. Do not substitute a crates.io install to "work around" an unreachable private index; that
tests a different, lagging artifact.

## See also

- [Release Process](./release-process.md) — how releases are built and published.
