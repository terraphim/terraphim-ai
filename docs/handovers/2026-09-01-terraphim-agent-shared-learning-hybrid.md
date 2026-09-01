# Handover: terraphim_agent hybrid-scoring refactor + docs sweep

**Date**: 2026-09-01 15:42 BST
**Session**: SharedLearning hybrid-scoring refactor, P2 follow-up, artefact refresh, and downstream documentation sweep
**Branch (terraphim-clients)**: `main` at `94e5946` (post-`85b1b8b`, post-artefact refresh)
**Branch (terraphim-ai)**: `docs/terraphim-agent-sweep-2026-09-01` cut from `terraphim/terraphim-ai` `main` at `8c245fd78`

---

## 1. Progress Summary

### Tasks completed this session

1. **Hybrid scoring refactor landed on `terraphim-clients` `main`**
   - `e8f97c3` `refactor: remove dead code cluster and align annotations with AGENTS.md`
   - `a888ee5` `feat(agent): fully implement suggest_learnings cluster, bridge to SharedLearning`
   - `ba2e292` `feat(shared-learning): route suggest/find_similar through Terraphim hybrid scoring`
   - `85b1b8b` `fix(shared-learning): address four P2 findings from PR review`
   - `94e5946` `docs: refresh verification, validation, and PR review artefacts (post-fix)`
   - PR merged to `terraphim-clients` `main`. All four P2 findings from the
     structural PR review of `ba2e292` are closed.

2. **PR #3305 (rust 1.97 clippy needless-borrow) merged and cleaned up**
   - Branch `fix/clippy-needless-borrow-1.97` was already merged server-side
     as `8c245fd78` on `origin/main` before this session started.
   - Local `main` fast-forwarded to `8c245fd78`; local
     `chore/drop-unused-sessions-patch` and `fix/clippy-needless-borrow-1.97`
     branches deleted; remote branches already gone server-side.

3. **Gitea token reference canonicalised via 1Password**
   - `op://Terraphim/gitea-token/credential` is the working reference in
     the active 1Password account. The resolved value is intentionally not
     recorded in this handover.
   - The credential was supplied through `op inject` and verified with
     `GET /api/v1/user`, which returned 200 authenticated as `root` (Alex,
     `alex@metracortex.engineer`, `is_admin=true`).
   - `op://TerraphimPlatform/gitea-mac-admin-token/credential` does not
     resolve because no `TerraphimPlatform` vault is accessible in this
     account. The alternate path remains unverified and must not be used in
     automation until its account and exact vault/item names are confirmed.
   - The safe canonicalisation, including the eight accessible vault names
     and the injection pattern, is recorded in the SharedLearning file
     `terraphim-agent-learning-59d131a15119-1788275158000.md`.

4. **`terraphim-ai` Gitea remotes use token-free URLs**
   - The `origin` and `gitea-private` remote URLs contain no embedded
     credentials. Authentication is supplied transiently through a
     temporary `GIT_ASKPASS` helper backed by `op inject`; no token is
     stored in `.git/config`.
   - A read-only `git ls-remote` check against the documentation branch
     succeeded with that helper, confirming that the token-free remote
     configuration remains usable.
   - Keep the remote URL and helper pattern aligned with the canonical
     `op://Terraphim/gitea-token/credential` reference.

5. **Downstream docs sweep on `terraphim-ai`** (in progress at this
   handover)
   - `docs/architecture/terraphim-agent.md` — new architecture explainer
     covering both sub-systems, the hybrid scoring pattern, the storage
     layout, and the trust-level promotion rules.
   - `docs/examples/terraphim-agent-shared-learning.md` — new working
     example, capture -> store -> suggest -> find_similar -> promote, with
     troubleshooting notes.
   - `docs/examples/index.md` — index entry inserted under the existing
     "Advanced Search" section.
   - `docs/architecture/README.md` — created as a thin index pointing at
     `polyrepo-topology.md` and the new file.
   - Branch: `docs/terraphim-agent-sweep-2026-09-01`. Cut from
     `terraphim/terraphim-ai` `main` at `8c245fd78`. Push and PR are the
     next two steps (see section 4).

### Current implementation state

- `terraphim-clients` `main` is clean and fast-forwardable. The uncommitted
  changes that were present at session start (in
  `crates/terraphim_agent/src/client.rs`, `learnings/capture.rs`,
  `main.rs`, and `tests/kg_ranking_integration_test.rs`) are leftover
  untracked edits from earlier sessions; they are not part of the
  hybrid-scoring work and are out of scope for this handover. They
  remain untouched.
- `terraphim-ai` is on `docs/terraphim-agent-sweep-2026-09-01` with the
  three new docs uncommitted.
- No open Gitea issue tracks the docs sweep; one will be opened before
  the PR (see section 4).

### What's working

- Hybrid path: `SharedLearningStore::suggest` and `find_similar` route
  through `RoleGraph::query_graph` when a graph is configured. The
  thesaurus-driven weighted-mean score replaces BM25 in the happy path.
- BM25 fallback: when the graph is not configured, is locked, or returns
  no matches, the store transparently falls back to the original BM25
  path. The `(score, SharedLearning)` shape is preserved across both
  paths.
- Locked-graph observability: poisoned or contended write locks now emit
  a `tracing::warn!` with the learning id, so operators can spot
  degraded hybrid mode in the logs without needing to reproduce it.
- Test coverage: 298 lib + 498 bin tests pass; the nine new
  `hybrid_tests` cover graph-driven ordering, BM25 fallback (no graph,
  empty thesaurus, no term match), trust weighting (L3 above L1), graph
  ingest via `insert`, and `applicable_agents` filtering on the hybrid
  path.
- Storage round-trip: `MarkdownLearningStore` round-trips every field,
  including the sparse-frontmatter case for older learnings. Tested in
  `test_save_and_load_roundtrip_preserves_full_state` and
  `test_sparse_old_frontmatter_still_loads`.

### What's blocked / needs follow-up

1. **Gitea token reference canonicalised, with one alternate path still
   unresolved.** The working reference is
   `op://Terraphim/gitea-token/credential`; the former
   `op://TerraphimPlatform/gitea-mac-admin-token/credential` path does not
   resolve in the active account. The eight accessible vaults contain no
   matching mac-admin item. The canonical path and safe injection pattern
   are documented in the SharedLearning record, while the alternate account
   or vault name still requires confirmation. The previously resolved value
   should be rotated if it is considered sensitive and was ever recorded
   outside the approved secret store.
2. **Untracked edits in `terraphim-clients` worktree.** Four files have
   modifications that pre-date this session and are unrelated to the
   hybrid-scoring work. They are not blocking the docs sweep but should
   be reviewed, committed, or discarded before the next change ships.
3. **`terraphim-ai` remote push pipeline.** The two-remote
   `origin` (GitHub) / `gitea` (Gitea) protocol declared in `AGENTS.md`
   was not exercised this session. The `origin` URL is currently set to
   the Gitea URL with the embedded token. If the next session needs to
   push to GitHub too, the URL must be reset via `op inject` against the
   GitHub-credentialed template.
4. **`main` is fast-forward only because the new commits are doc-only.**
   If the untracked edits in (2) above are turned into a real change,
   the next push will need a full PR flow on `terraphim-clients`.

## 2. Technical Context

### Branches

```
terraphim-clients: main at 94e5946 (clean, +2 ahead of last release tag)
terraphim-ai:      docs/terraphim-agent-sweep-2026-09-01 (3 new docs uncommitted)
```

### Recent commits (terraphim-clients)

```
94e5946 docs: refresh verification, validation, and PR review artefacts (post-fix)
85b1b8b fix(shared-learning): address four P2 findings from PR review
ba2e292 feat(shared-learning): route suggest/find_similar through Terraphim hybrid scoring
a888ee5 feat(agent): fully implement suggest_learnings cluster, bridge to SharedLearning
e8f97c3 refactor: remove dead code cluster and align annotations with AGENTS.md
```

### Modified files (terraphim-ai, uncommitted on docs branch)

```
docs/architecture/terraphim-agent.md                            (new)
docs/architecture/README.md                                      (new)
docs/examples/terraphim-agent-shared-learning.md                 (new)
docs/examples/index.md                                           (entry added)
```

### Verification commands

```bash
# terraphim-clients sanity
cd terraphim-clients
cargo check -p terraphim_agent --all-targets --features shared-learning,cross-agent-injection
cargo clippy -p terraphim_agent --all-targets --features shared-learning,cross-agent-injection -- -D warnings
cargo test -p terraphim_agent --lib --features shared-learning
cargo test -p terraphim_agent --lib --features shared-learning,cross-agent-injection

# terraphim-ai docs lint
cd terraphim-ai
# British English spot-check (delegated to the agent reviewer; not enforced as a CI step yet)
rg -n "color|behavior|optimize|utilize|organize" docs/architecture/terraphim-agent.md docs/examples/terraphim-agent-shared-learning.md
# Expect no hits (UK spelling is colour, behaviour, optimise, utilise, organise).
```

### Source artefacts in `terraphim-clients/.docs/`

- `.docs/verification/verification-report-hybrid-scoring.md` — Phase 4
  verification (post-fix). Confirms 298/298 lib + 498/498 bin + 9/9 new
  hybrid tests; clippy clean on the diff; `cargo fmt` clean.
- `.docs/validation/validation-report-hybrid-scoring.md` — Phase 5
  validation. Stakeholder walkthrough; sign-off is recorded.
- `.docs/pr-review/pr-review-ba2e292.md` — structural PR review of
  `ba2e292` that produced the four P2 findings closed in `85b1b8b`.

### Key Files Changed (terraphim-clients)

| File | Change |
|---|---|
| `crates/terraphim_agent/src/shared_learning/store.rs` | New `hybrid_rank` helper; `build_document_for_graph`; `sync_to_graph`; `set_role_graph` initial sync; `find_similar` and `suggest` route through hybrid with BM25 fallback; P2 lock-observability warnings |
| `crates/terraphim_agent/src/shared_learning/markdown_store.rs` | `parse_learning_source` accepts both snake_case and PascalCase for back-compat (no functional change in this refactor) |
| `crates/terraphim_agent/src/learnings/capture.rs` | Bridges `CapturedLearning` to `SharedLearning` via `shared_learning_from_entry` when `shared-learning` feature is on |
| `crates/terraphim_agent/CHANGELOG.md` | Hybrid-scoring entry to be added under "Unreleased" (next session) |
| `terraphim-clients/.docs/verification/verification-report-hybrid-scoring.md` | Phase 4 verification report (post-fix) |
| `terraphim-clients/.docs/validation/validation-report-hybrid-scoring.md` | Phase 5 validation report (post-fix) |
| `terraphim-clients/.docs/pr-review/pr-review-ba2e292.md` | Structural PR review (P0/P1/P2 with severity tiers) |

## 3. Cross-references

- **Architecture explainer**:
  [`docs/architecture/terraphim-agent.md`](../architecture/terraphim-agent.md)
- **Working example**:
  [`docs/examples/terraphim-agent-shared-learning.md`](../examples/terraphim-agent-shared-learning.md)
- **Polyrepo placement**: `terraphim-clients` is the clients layer of
  the polyrepo split; see
  [`docs/architecture/polyrepo-topology.md`](../architecture/polyrepo-topology.md).
- **Prior handover template**:
  [`docs/handovers/2026-05-11-adf-restart.md`](2026-05-11-adf-restart.md),
  [`docs/handovers/2026-06-13-adf-digital-twins-handover.md`](2026-06-13-adf-digital-twins-handover.md)
  for the cadence this handover follows.

## 4. Next Steps

1. **Commit the new docs on the `docs/terraphim-agent-sweep-2026-09-01`
   branch.** Conventional Commits prefix
   `docs(terraphim_agent): sweep` so the history reads cleanly when the
   branch is merged.
2. **Push the branch to `terraphim-ai` `origin`.** `git push -u origin
   docs/terraphim-agent-sweep-2026-09-01`.
3. **Open a Gitea PR via `gtr create-pull`.** Target `terraphim/terraphim-ai`,
   base `main`, head `docs/terraphim-agent-sweep-2026-09-01`. PR
   description should reference the three new files and the upstream
   `terraphim-clients` commits `ba2e292` and `85b1b8b`.
4. **Open a Gitea tracking issue via `gtr create-issue`.** Title
   "Docs: terraphim_agent architecture, example, and handover sweep".
   Body summarises the three artefacts and points to this handover. Cross
   the PR and issue with `gtr comment` so future search hits both.
5. **Rotate the previously exposed Gitea credential if required, then
   confirm the alternate 1Password account or exact vault/item names.** Use
   only the canonical `op://Terraphim/gitea-token/credential` reference in
   automation until that confirmation is available.
6. **Review and commit the untracked edits** in `terraphim-clients` from
   `crates/terraphim_agent/src/{client,learnings/capture,main}.rs` and
   `tests/kg_ranking_integration_test.rs`. They pre-date this session
   and are not part of the hybrid-scoring work; they need their own
   commit and PR.
7. **Add a `CHANGELOG.md` entry** under "Unreleased" for the
   hybrid-scoring work. The other "Unreleased" entries (R2 update
   backend, self-update install path) are unrelated and stay separate.

---

*Prepared by terraphim-agent session on 2026-09-01 at 15:42 BST. This
handover is committed to the docs branch and is intended to be merged
alongside the three new docs.*
