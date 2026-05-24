# terraphim_grep v1.19.3 -- Hybrid Search with LLM Fallback

Tracks Gitea issue #1743, PR #1825 (branch `task/1743-terraphim-grep`).

## Highlights

- **KG-aware boost: your knowledge tops the results.** New `boost_chunks_with_kg`
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
- Criterion benchmarks for `code_only`, `hybrid_with_kg`, `fuse_and_rank`, and the new
  `kg_boost_overhead` -- under 25 us added per search at typical scale.

## Defects fixed

- **D001**: CLI never wired an `LlmClient`. Any query landing in `NeedsSynthesis` hard-errored.
- **D005**: With no LLM and `NeedsSynthesis`, grep failed instead of returning chunks.

## Verified

- `cargo test -p terraphim_grep --features "code-search openrouter"`: 24 passed
- `cargo clippy --tests --benches`: clean
- `cargo bench --bench hybrid_search -- --test`: all branches exercised
- Release binary smoke (`/tmp/grep-release-test`):
  - Checksum verifies
  - Search-only mode: 8 chunks, sufficiency=SearchOnly, 3ms
  - LLM mode against `liquid/lfm-2.5-1.2b-instruct:free`: 8 chunks,
    sufficiency=RlmSynthesis, ~1.4s end-to-end

## Build profile

```
cargo build --release -p terraphim_grep --features "code-search openrouter"
```

Binary: `target/release/terraphim-grep` (8.3 MB on darwin-arm64).

## Release artefact

```
target/release/terraphim-grep-v1.19.3-task1743-darwin-arm64.tar.gz
target/release/terraphim-grep.sha256
```

SHA-256 of tarball (darwin-arm64): `f4cd95f5a30a263145f82fca9895062cf95b1cc580358fa8b99397649d9b0118`

## Crates.io publishability

**Blocked** for the `code-search` feature. `fff-search` is currently consumed as a git
dependency (`github.com/AlexMikhalev/fff.nvim.git?branch=feat/external-scorer`), and
`cargo publish` refuses dependencies without a version requirement.

```
$ cargo publish -p terraphim_grep --dry-run --features code-search
error: all dependencies must have a version requirement specified when publishing.
       dependency `fff-search` does not specify a version
```

Paths forward (any one unblocks publish):
1. Land `fff-search` on crates.io and pin a version
2. Vendor the subset of `fff-search` we use into a workspace-local crate
3. Make `code-search` a non-default optional feature, publish the crate without that
   path, and document that downstream users must enable it as a git-source override

## Free OpenRouter models recommended

| Capability | Model | Notes |
|---|---|---|
| Code-aware synthesis | `qwen/qwen3-coder:free` | Best for code use cases |
| Multi-chunk reasoning | `meta-llama/llama-3.3-70b-instruct:free` | Strong general |
| Fast CI smoke | `liquid/lfm-2.5-1.2b-instruct:free` | 1.2B params, sub-second |

Free-tier limits: 20 req/min, 200 req/day (1000 if account has ever held >=$10).
