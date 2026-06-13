# Incomplete Handoff: Structural PR Review #2664

**Status**: In progress — analysis complete, final write-up and verification remaining.

**PR**: terraphim/terraphim-ai #2664  
**Head**: `0607a5732cd56d5d7bcd0bfb02ebed13b4fa4aa7`  
**Target**: `main` (Gitea)  
**Size**: 923 LOC across 20 files.

## What is done

1. Loaded `structural-pr-review` skill.
2. Fetched PR metadata and full diff from `git.terraphim.cloud` API:
   - `/tmp/pr2664.diff`
   - `.pr2664.diff`
3. Split the diff per file under `.review_tmp/pr2664/*.diff`.
4. Extracted full post-PR file contents under `.review_tmp/pr2664/files/` for:
   - `crates/terraphim_rlm/src/executor/firecracker.rs`
   - `crates/terraphim_rlm/src/executor/mod.rs`
   - `crates/terraphim_rlm/src/config.rs`
   - `crates/terraphim_rlm/src/validator.rs`
5. Read `crates/terraphim_rlm/src/executor/context.rs` at PR head to confirm `SnapshotId` semantics.
6. Identified candidate findings (see below).

## What remains

- Verify each candidate finding by compiling / reading cross-file context (e.g., does `KnowledgeGraphValidator` implement `Clone`? Does `rlm.rs` compile without `kg-validation`?).
- Run `cargo clippy -p terraphim_rlm` and `cargo test -p terraphim_rlm` if possible.
- Decide final severity calibration and confidence score.
- Produce the final single markdown block with:
  - `### Summary`
  - `### Confidence Score: N/5`
  - `### Inline Findings` (one line per finding prefixed `**P0 `, `**P1 `, or `**P2 `)
  - `<sub>Last reviewed commit: 0607a57</sub>`

## Candidate findings (to verify / finalise)

### Likely P1 — correctness

1. **`crates/terraphim_rlm/src/executor/firecracker.rs`, `list_snapshots()`**  
   Maps returned fc snapshots with `SnapshotId::new(s.name, *session_id)`. `SnapshotId::new` generates a fresh random ULID for the `id` field and only stores the user name. `delete_snapshot` and `restore_snapshot` use `id.id.to_string()` as the backend snapshot identifier, so snapshot IDs produced by `list_snapshots` will not match any backend snapshot and cannot be deleted/restored.  
   *Suggested fix*: carry the backend snapshot `id` (`s.id`) through `SnapshotId`; if the type mismatch with `Ulid` is blocking, change `SnapshotId.id` to a `String` or add a backend-id field.

2. **`crates/terraphim_rlm/src/executor/firecracker.rs`, `list_snapshots()`**  
   `snapshot_manager.list_snapshots(...).await.unwrap_or_default()` silently swallows backend errors and returns an empty list. This masks failures and makes callers believe no snapshots exist.

3. **`crates/terraphim_rlm/src/rlm.rs`**  
   Imports and stores `KnowledgeGraphValidator` unconditionally, but the `validator` module is gated by `#[cfg(feature = "kg-validation")]`. A `--no-default-features` build (or any feature set without `kg-validation`) will fail to compile.

### Likely P1/P2 — to verify

4. **`crates/terraphim_rlm/src/executor/mod.rs`, `select_executor()`**  
   A single `validator` value is built before the backend loop and moved into the first backend that consumes it (e.g., Firecracker). If that backend fails initialization and `continue`s, the validator is no longer available for the next backend arm. Verify whether this compiles (likely not unless `KnowledgeGraphValidator` is `Clone`).

### Likely P2

5. **`crates/terraphim_rlm/src/executor/firecracker.rs`, `cleanup()`**  
   Stops VMs sequentially and holds the `vm_manager` lock for the entire loop. Independent stop operations could be parallelised with `FuturesUnordered` / `join_all`, and shorter lock hold improves concurrency.

6. **`crates/terraphim_update/src/signature.rs`**  
   The embedded public-key comment now records the 1Password vault and item ID where the private key is stored. This is operational metadata exposure; consider keeping it out of the source tree.

7. **PR contains unrelated files**  
   `.docs/research-release-blockers.md` and `.sessions/session-20260613-134146.md` are included in the PR but unrelated to #2430/#2431. They should be removed from this branch or moved to a separate PR.

## Next-agent starting position

Continue from this handoff file. Use the artifacts in `.review_tmp/pr2664/` and `.pr2664.diff` to complete the review. Priority order:

1. Confirm or refute findings 1–4 above.
2. Run the RLM test/clippy gate if time permits.
3. Assign final severities, choose a confidence score, and emit the required final markdown block.

## Artifacts

- `.pr2664.diff` — full Gitea PR diff
- `.review_tmp/pr2664/*.diff` — per-file diffs
- `.review_tmp/pr2664/files/` — full file contents at PR head for key Rust files
- `.handoff/pr2664-review-handoff.md` — this file
