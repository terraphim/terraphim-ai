# Phase 1 §10 — Experimental Crate Register

Captured: 2026-05-10. Source: root `Cargo.toml` `[workspace] exclude` block plus directory inspection.

Workspace-active members from `cargo metadata` count: **49** (includes `crates/terraphim_rlm` which is commented in the exclude block, i.e. retained as active).

## Excluded entries (verbatim from Cargo.toml, with classification)

| # | Path | Comment | Classification | Rationale |
|---|------|---------|----------------|-----------|
| 1 | `crates/terraphim_agent_application` | "Experimental crates" | **deferred** | Experimental; await graduation. Re-evaluate at Phase 2 §13 topology decision. |
| 2 | `crates/terraphim_truthforge` | "Experimental crates" | **deferred** | Experimental; await graduation. |
| 3 | `crates/terraphim_automata_py` | listed TWICE in exclude block (Cargo.toml ll. 4 & 8) — first as "Experimental", second as "Python bindings" | **awaiting-graduation** + housekeeping fix | Real classification is Python binding; needs `maturin develop`. **Action: dedupe the entry in Cargo.toml** as part of Stage A housekeeping. |
| 4 | `crates/terraphim_rolegraph_py` | "Python bindings (need `maturin develop`)" | **awaiting-graduation** | Python binding; lifecycle differs from Rust crates; may move to its own repo when split alongside any future Rust-FFI strategy. |
| 5 | `desktop/src-tauri` | "Desktop built separately via Tauri CLI" | **already-extracted** | Lives in `terraphim-ai-desktop` repo per `docs/reports/desktop-extraction-crate-architecture-review-2026-02-25.md`. |
| 6 | `crates/terraphim_build_args` | "Planned future use, not needed for workspace builds" | **deferred** | Skeleton; no consumers; revisit when build-args orchestration is needed. |
| 7 | `crates/haystack_atlassian` | "Unused haystack providers (kept for future integration)" | **deferred** | Future haystack adapter; would join `terraphim-haystacks` repo (Phase 2 §13) on graduation. |
| 8 | `crates/haystack_discourse` | "Unused haystack providers" | **deferred** | Same as #7. |
| 9 | `crates/terraphim_repl` | "Superseded by terraphim_agent" | **graveyard** OR **deferred** | Per `REPL_EXTRACTION_PLAN.md` there is intent to extract a standalone REPL. Decision: **deferred** — keep until REPL extraction PR lands; archive in `terraphim-experiments` only if REPL plan is cancelled. |
| 10 | `crates/terraphim_symphony` | "Symphony orchestrator (build separately: cd crates/terraphim_symphony && cargo build)" | **deferred** | Has its own build process; is a candidate for its own repo when matured. |
| 11 | `crates/terraphim_github_runner` | "Firecracker-based crates (private git dependency fcctl-core)" | **private** | Depends on private `fcctl-core`; stays in private firecracker-rust repo. |
| 12 | `crates/terraphim_github_runner_server` | (firecracker stack) | **private** | Same as #11. |
| 13 | `terraphim_firecracker` | (firecracker stack) | **conflicting** — listed in BOTH `members` AND `exclude` of root Cargo.toml. `cargo metadata` reports it as a workspace member. **Action: resolve in Stage A** — pick one. Plan recommendation: keep in `members` (currently builds), drop the duplicated `exclude` line. |
| 14 | `crates/terraphim_lsp` | "Missing Cargo.toml" | **graveyard** | Skeleton without `Cargo.toml`; cannot build. Move to `terraphim-experiments` archive or delete. |
| 15 | `infrastructure/vm-templates` | "Infrastructure templates (not a crate)" | **not-a-crate** | Templates / config; lives in an ops/infrastructure repo, out of scope for the Rust split. |
| 16 | `infrastructure/rust-cache-stack` | "docker compose + systemd unit, no Cargo.toml" | **not-a-crate** | Same as #15. |
| 17 | `infrastructure/firecracker-rust-ci` | "shell scripts + overlay, no Cargo.toml" | **not-a-crate** | Same as #15. |
| 18 | `infrastructure/rch-bigbox` | "toml/service files, no Cargo.toml" | **not-a-crate** | Same as #15. |
| 19 | `infrastructure/fcctl-web` | "Lives in the private firecracker-rust repo" | **private** + already-extracted | Already lives elsewhere. |

## Classification summary

| Class | Count | Disposition |
|-------|-------|-------------|
| `deferred` | 7 | Re-evaluate at Phase 2 §13 |
| `awaiting-graduation` | 2 | Python bindings; separate maturin lifecycle |
| `already-extracted` | 2 | desktop, fcctl-web |
| `private` | 3 | firecracker stack — stays private |
| `graveyard` | 1 | terraphim_lsp (no Cargo.toml) |
| `not-a-crate` | 4 | infrastructure/* |
| `conflicting` | 1 | terraphim_firecracker — Stage A housekeeping |

## Phase-specific actions

- **Stage A housekeeping (cycle-break PR or earlier)**:
  - Dedupe `crates/terraphim_automata_py` in Cargo.toml `exclude`.
  - Resolve `terraphim_firecracker` `members` vs `exclude` conflict.
  - Either delete `crates/terraphim_lsp` or add a stub `Cargo.toml` placeholder; plan recommendation: delete and archive in a `terraphim-experiments` repo if anyone wants the source preserved.

- **No experimental crate joins the polyrepo split unless and until it graduates**. The plan accepts that `terraphim-experiments` (graveyard repo) and the Python bindings live on independent timelines.

## Active member count reconciliation

- `cargo metadata` reports 49 workspace_members (includes `terraphim_firecracker` despite exclude line — Cargo behaviour with overlapping `members`/`exclude` is to include).
- Brief stated 48; actual is 49. Difference is `terraphim_firecracker`. After Stage A housekeeping, the count will be 48 (firecracker either fully excluded as private, or kept in members with the duplicate exclude line removed).
