# terraphim_grep v1.20.0 -- Hybrid Search with LLM Fallback

Tracks Gitea issue #1743, PR #1825 (branch `task/1743-terraphim-grep`).

Ships as part of the workspace v1.20.0 release. Also tagged as
`terraphim_grep-v1.20.0` for crate-focused references and Homebrew.

## Highlights

- **KG-aware boost: your knowledge tops the results.** `boost_chunks_with_kg`
  re-ranks chunks so files whose source path or content matches your thesaurus
  concepts move above generic matches. Default weight of `1.0` lets a fully-matched
  chunk roughly double its score; the boost is reflected in the JSON output so
  downstream tools can see why a chunk ranked where it did.
- End-to-end hybrid pipeline: `fff-search` code retrieval + parallel knowledge-graph
  concept extraction + KG-aware ranking boost + sufficiency judging + LLM synthesis
  with citations.
- CLI wires `terraphim_service::llm::build_llm_from_role` -- aligns grep with how the
  server, TUI, and RLM consume providers; whether routing through capability extraction
  kicks in is a role-config decision (`llm_router_enabled = true`).
- Graceful degradation: no LLM configured? You still get chunks. `force_rlm = true`
  still fails fast.
- Four-layer test pyramid, zero mocks (L1 inline, L2 router-capability,
  L3 e2e against free OpenRouter, L4 manual quality gate).
- Criterion benchmarks for `code_only`, `hybrid_with_kg`, `fuse_and_rank`, and
  `kg_boost_overhead` -- under 25 us added per search at typical scale.

## Dependencies bumped in v1.20.0

- `fff-search` git → **crates.io 0.8.2** (stable, published 2026-05-24). Unblocks
  `cargo publish -p terraphim_grep`. API changed since 0.5.1:
  - `grep_search` is now private; switched to `FilePicker::grep(&query, &options)`
  - `FilePickerOptions` dropped `warmup_mmap_cache`; manifest uses `..Default::default()`
  - `GrepSearchOptions` requires `abort_signal` and `trim_whitespace`; same approach
  - `FileItem::relative_path` now takes an arena reference; `&FilePicker` implements
    `FFFStringStorage`, so we pass `&picker` directly

## Defects fixed

- **D001**: CLI never wired an `LlmClient`. Any query landing in `NeedsSynthesis` hard-errored.
- **D005**: With no LLM and `NeedsSynthesis`, grep failed instead of returning chunks.

## Verified

- `cargo test -p terraphim_grep --features "code-search openrouter"`: 24 passed
- 3 router-capability tests pass (`tests/router_capability_routing.rs`)
- 1 L3 e2e test passes against `liquid/lfm-2.5-1.2b-instruct:free`
- `cargo clippy --tests --benches`: clean
- `cargo bench --bench hybrid_search -- --test`: all branches exercised
- `cargo publish --dry-run -p terraphim_grep --features code-search --allow-dirty`:
  manifest publish-valid (Packaging + Verifying both succeed). Actual `cargo publish`
  succeeds in CI via `scripts/publish-crates.sh` which publishes the full dependency
  chain at v1.20.0 in order.

## Install

### Homebrew

```bash
brew tap terraphim/terraphim
brew install terraphim-grep
```

### Cargo

```bash
cargo install terraphim_grep --features code-search
```

For the OpenRouter-backed synthesis path:

```bash
cargo install terraphim_grep --features "code-search openrouter"
export OPENROUTER_API_KEY=sk-or-v1-...
export OPENROUTER_MODEL=qwen/qwen3-coder:free
```

### Debian / Ubuntu

```bash
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.20.0/terraphim-grep_1.20.0_amd64.deb
sudo dpkg -i terraphim-grep_1.20.0_amd64.deb
```

### Binary tarball

```bash
wget https://github.com/terraphim/terraphim-ai/releases/download/v1.20.0/terraphim-grep-1.20.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf terraphim-grep-1.20.0-x86_64-unknown-linux-gnu.tar.gz
./terraphim-grep --version
```

macOS users with the universal binary outside Homebrew:

```bash
# After downloading terraphim-grep-universal-apple-darwin
xattr -d com.apple.quarantine ./terraphim-grep-universal-apple-darwin
./terraphim-grep-universal-apple-darwin --version
```

## Free OpenRouter models recommended

| Capability | Model | Notes |
|---|---|---|
| Code-aware synthesis | `qwen/qwen3-coder:free` | Best for code use cases |
| Multi-chunk reasoning | `meta-llama/llama-3.3-70b-instruct:free` | Strong general |
| Fast CI smoke | `liquid/lfm-2.5-1.2b-instruct:free` | 1.2B params, sub-second |

Free-tier limits: 20 req/min, 200 req/day (1000 if account has ever held >=$10).

## Rollback procedures per channel

| Channel | Recipe | Reversibility |
|---|---|---|
| crates.io | `cargo yank --vers 1.20.0 terraphim_grep` | Hard; blocks new resolutions, doesn't delete |
| GitHub Release | `gh release delete v1.20.0 --yes` (or with `--cleanup-tag`) | Soft; can re-publish |
| Homebrew | revert the formula commit in `terraphim/homebrew-terraphim` | Soft; users get revert on `brew update` |
| Debian | supersede with `terraphim_grep_1.20.1_*.deb` carrying the fix | Forward-only |

Prefer fix-forward (`1.20.1`) over rollback for any channel except crates.io.
