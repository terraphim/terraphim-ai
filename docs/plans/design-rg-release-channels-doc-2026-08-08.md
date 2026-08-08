# Design Gate — #3184 fleet-internal release bless = Gitea private cargo

## Problem
Agents treat crates.io / GitHub Releases as canonical for `terraphim_grep` and `terraphim_agent`.
Guardian blesses **1.21.1** on the Gitea private cargo registry while crates.io sits at 1.20.5 and the
GitHub release tag at v1.20.5 — so agents report false release failures. Nothing in-repo encodes the
channel precedence. Docs-only slice of ops issue #3183.

## Decision (exact touchpoints)
1. NEW `docs/src/release-channels.md` — policy doc. Placed under the mdBook `src` root (verified
   `docs/book.toml` → `src = "src"`) so it renders and sits beside existing `docs/src/release-process.md`.
   The issue's `docs/release-channels.md` would be outside the book; this satisfies the same AC.
2. EDIT `docs/src/SUMMARY.md` — add `- [Release Channels](./release-channels.md)` near the
   `Contributing`/`Branch Protection` block.
3. EDIT `README.md` `## Installation Methods` (~line 591), under `#### Direct Download` — note that
   GitHub Releases are **not** the fleet-canonical channel; link to the new doc.

## Ground truth (verified 2026-08-08, live)
- `GET {GITEA_URL}/api/v1/packages/terraphim/cargo/{name}` → HTTP 200, returns a **JSON array**, one
  object per version. Fields per object: `id`, `owner.login`, `repository` (null), `creator`,
  `type: "cargo"`, `name`, `version`, `html_url`, `created_at`. **There is no `latest` field** — the
  doc must say "max version in the array", not "read `.latest`". List form:
  `GET /api/v1/packages/terraphim?type=cargo`.
- Blessed: `terraphim_grep` 1.21.1 (3 versions), `terraphim_agent` 1.21.1 (3 versions). Package names
  use underscores.
- Lag confirmed: crates.io 1.20.5 both crates; GitHub release v1.20.5; Gitea tag v1.21.0.
- Guardian verdict `CONDITIONAL_PASS`; lag = WARN checks `R6`/`R6cargo`/`R6cargo-rel`; `R6canon` PASS
  = fleet bless 1.21.1. Smokes `R7:*` PASS, `path=terraphim-clients`, temp-root.
- Guardian lives at `/home/alex/projects/cto-executive-system/release-guardian/` (**not** `private/…`
  as the issue states) — external to this repo: reference by name, never a relative link.

## Acceptance criteria
- AC1 Doc states: canonical = Gitea cargo registry, owner `terraphim`; blessed 1.21.1 for grep+agent;
  crates.io/GitHub lag is **CONDITIONAL, not failure** for fleet-internal; install smoke is temp-root
  only with path install from `terraphim-clients` when the private sparse index is unavailable.
- AC2 README `Installation Methods` links to it.
- AC3 Doc names `release-guardian` for continuous checks and cites evidence
  `reports/2026-08-08-terraphim-ai-clients.md` and `adf-ledger.jsonl` (as paths, not links).
- AC4 PR to `main`; structural-pr-review 5/5, zero P1/P2.

## Non-goals
- Publishing to crates.io / GitHub Releases (ops #3183). No version bumps, no Cargo/CI changes.
- No automated registry checking in this repo (guardian owns it); no edits under
  `cto-executive-system/`; no re-running smokes.

## Test plan
- Unit: none — zero code changes; asserted by `git diff --stat` touching only `.md`.
- Links resolve (`ls` each target); `mdbook build docs` emits `release-channels.html`.
- Live (re-run and paste in PR): `curl -s -H "Authorization: token $GITEA_TOKEN" \
  "$GITEA_URL/api/v1/packages/terraphim/cargo/terraphim_grep" | jq -r '.[].version'` → includes 1.21.1.
- No wiremock: no in-repo code path consumes this API, so there is nothing to stub.

## Gates
```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings   # no-op, docs-only
cargo test --workspace                                   # unchanged
mdbook build docs                                        # book.toml is at docs/book.toml
```
Pre-commit hooks must pass: conventional commit `docs(release): … Refs #3184`; secret scan — never
paste a `GITEA_TOKEN` value into the doc, use `$GITEA_TOKEN` placeholders only.
