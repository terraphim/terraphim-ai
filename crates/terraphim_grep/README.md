# terraphim_grep

Intelligent hybrid grep with RLM (Retrieval-Language Model) fallback and knowledge-graph curation.

## Overview

`terraphim_grep` is a search crate that combines local code search, knowledge-graph (KG) concept boosting, and optional LLM synthesis into a single pipeline. It runs code retrieval and KG concept lookup in parallel, applies KG-aware boosting so chunks matching your thesaurus rank above generic matches, and falls back to an LLM only when local sufficiency is below threshold.

## Features

- **Hybrid Search**: Parallel execution of code search and KG concept lookup
- **KG-Aware Boosting**: Chunks matching thesaurus concepts rank higher than generic matches
- **Sufficiency Judging**: Heuristic-based decision on whether results are sufficient or need LLM synthesis
- **RLM Fallback**: Graceful degradation to search-only mode when no LLM is configured
- **KG Curation**: Optional extraction and indexing of new concepts from LLM responses
- **CLI Binary**: Standalone `terraphim-grep` binary with JSON output support

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
terraphim_grep = { path = "../terraphim_grep" }
```

### Feature Flags

| Feature       | Default | Description                                        |
|---------------|---------|----------------------------------------------------|
| `llm`         | Yes     | Enable LLM integration via `terraphim_service`     |
| `code-search` | No      | Enable `fff-search` code retrieval backend          |
| `openrouter`  | No      | Enable OpenRouter provider for live LLM tests       |

## Usage

### Library

```rust
use std::sync::Arc;
use terraphim_grep::{
    GrepOptions, Haystack, HybridSearcher, SufficiencyJudge, TerraphimGrep,
};
use terraphim_types::Thesaurus;

#[tokio::main]
async fn main() -> terraphim_grep::Result<()> {
    let hybrid = HybridSearcher::new(
        "my-role".to_string(),
        Thesaurus::new("my-thesaurus".to_string()),
    )?.with_search_path("src".into());

    let grep = TerraphimGrep::new(
        Arc::new(hybrid),
        Arc::new(SufficiencyJudge::default()),
    );

    let result = grep.search(
        "error handling",
        GrepOptions {
            haystack: Haystack::Code,
            max_results: 20,
            ..GrepOptions::default()
        },
    ).await?;

    for chunk in &result.chunks {
        println!("{}: {}", chunk.source, chunk.content.chars().take(80).collect::<String>());
    }

    Ok(())
}
```

### CLI

```bash
# Basic search
terraphim-grep "async fn spawn"

# Search with context lines and JSON output
terraphim-grep "error handling" -C 3 --json

# Force LLM synthesis
terraphim-grep "explain token budget" --force-rlm --answer

# Search specific paths
terraphim-grep "struct Config" --paths src/ crates/
```

## Architecture

```
Query
  │
  ▼
┌──────────────────┐
│  HybridSearcher  │──→ Code search (fff-search) ──┐
│  (parallel)      │──→ KG concept lookup  ────────┤
└──────────────────┘                               │
  │                                                ▼
  ▼                                    ┌──────────────────┐
┌──────────────────┐                   │  RetrievedChunk  │
│ SufficiencyJudge │◄─────────────────│  + KgConcept      │
│  (heuristics)    │                   └──────────────────┘
└──────────────────┘
  │
  ├── Sufficient ──→ Return chunks (SearchOnly)
  ├── NeedsSynthesis ──→ RLM fallback (if LLM configured)
  ├── NeedsExpansion ──→ RLM fallback with additional chunks
  └── Insufficient ──→ Return empty (RlmInsufficient)
```

## Key Types

- **`TerraphimGrep`** — Main entry point combining search, sufficiency judging, and optional LLM
- **`HybridSearcher`** — Parallel code search and KG concept lookup with boosting
- **`SufficiencyJudge`** — Heuristic thresholds for deciding if results need LLM synthesis
- **`GrepOptions`** — Query configuration (haystack, context lines, max results, force RLM)
- **`GrepResult`** — Search output with chunks, optional answer, concepts, and stats

## Testing

```bash
# Run all tests
cargo test -p terraphim_grep

# Run with code-search feature
cargo test -p terraphim_grep --features code-search

# Run benchmarks
cargo bench -p terraphim_grep --features code-search
```

## License

MIT
