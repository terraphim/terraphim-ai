# Reddit Posts for Terraphim AI

## r/LocalLLaMA Post

**Title:** Terraphim AI: Privacy-first local AI assistant with knowledge graphs - now adding X/Twitter integration

**Body:**

Hey r/LocalLLaMA,

I've been working on Terraphim AI, an open-source privacy-first AI assistant that runs entirely locally. Thought you'd appreciate the architecture and would love feedback.

### The Problem

Knowledge workers waste ~20% of their time searching for information across fragmented tools. Existing AI assistants require uploading your data to external servers - not ideal when you're dealing with proprietary code, private notes, or confidential documents.

### What Terraphim Does

It's a local search engine with knowledge graph semantics that unifies search across:

- Local filesystem (Markdown, code files)
- Personal knowledge bases (Obsidian, Logseq, Notion)
- Team tools (Confluence, Jira)
- Public sources (StackOverflow, GitHub, Reddit)
- Email (JMAP)

Everything runs locally. Your data never leaves your machine.

### Technical Highlights

**Knowledge Graph System:**
- Custom Aho-Corasick automata for fast text matching
- Multiple relevance functions (BM25, BM25F, BM25Plus, TitleScorer, TerraphimGraph)
- Semantic thesaurus with concept expansion
- Role-based personalization (different knowledge graphs for different contexts)

**LLM Integration:**
- Full Ollama support (local models)
- OpenRouter integration (optional)
- Document summarization
- Context-aware AI chat

**Architecture:**
- Rust backend with async/await (tokio)
- Svelte + Tauri desktop app
- Terminal UI with REPL
- MCP (Model Context Protocol) server for AI tool integration
- Firecracker microVM support for secure execution (sub-2s boot times)

### Why X/Twitter Integration?

We're building X API integration to index:
- Your bookmarked technical threads
- Discussions from accounts you follow
- Domain-specific conversations

All stored in your local knowledge graph. When you search "async cancellation patterns", you get your notes + StackOverflow + that brilliant Twitter thread you saved 6 months ago.

### Code Quality

- Pre-commit hooks for fmt/lint
- No mocks in tests (real integration testing)
- Feature flags for optional functionality
- Multi-platform CI/CD (GitHub Actions + Docker Buildx)

### Try It

```bash
# Quick install
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash

# Or Docker
docker run ghcr.io/terraphim/terraphim-server:latest

# Or build from source
git clone https://github.com/terraphim/terraphim-ai
cargo build --release
```

**GitHub:** https://github.com/terraphim/terraphim-ai
**Discord:** https://discord.gg/VPJXB6BGuY

### Looking for Feedback

- How do you currently manage scattered knowledge?
- What local LLM models work best for your use cases?
- Interest in other integrations? (Slack, Discord, etc.)

Apache 2.0 licensed. PRs welcome.

---

## r/rust Post

**Title:** [Show Reddit] Terraphim AI - Privacy-first knowledge search with Rust backend, knowledge graphs, and Firecracker VM integration

**Body:**

Built a privacy-first AI assistant in Rust. Here's the interesting technical bits:

### Architecture

29 library crates in a Cargo workspace:
- `terraphim_service` - Main service layer
- `terraphim_automata` - Aho-Corasick text matching + autocomplete
- `terraphim_rolegraph` - Knowledge graph implementation
- `terraphim_middleware` - Search orchestration
- `terraphim_persistence` - Storage abstraction (memory, dashmap, sqlite, redb)
- `terraphim_firecracker` - Firecracker microVM integration

### Key Rust Patterns Used

**Async everywhere with tokio:**
```rust
// Bounded channels for backpressure
let (tx, rx) = tokio::sync::mpsc::channel(100);

// Select for concurrent operations
tokio::select! {
    result = search_task => handle_search(result),
    timeout = tokio::time::sleep(Duration::from_secs(5)) => handle_timeout(),
}
```

**Knowledge Graph with Automata:**
```rust
// Fast text matching using Aho-Corasick
pub fn find_matches(&self, text: &str) -> Vec<Match> {
    self.automaton.find_overlapping_iter(text)
        .map(|m| Match::new(m.pattern(), m.start(), m.end()))
        .collect()
}
```

**WASM Support:**
`terraphim_automata` compiles to WebAssembly with `wasm-pack` for browser autocomplete.

```bash
# Build WASM module
./scripts/build-wasm.sh web release
```

**Error Handling:**
- `thiserror` for custom error types
- `anyhow` for application errors
- Result propagation with `?`
- Graceful degradation (empty results vs panics)

**Testing Philosophy:**
- `tokio::test` for async tests
- No mocks - real integration tests
- Feature-gated live tests
- `tokio::time::pause` for time-dependent tests

### Firecracker Integration

Sub-2 second VM boot times for secure command execution:
- VM pooling for fast allocation
- Knowledge graph validation before execution
- Isolated file and web operations

### Performance Considerations

- Concurrent API calls with `tokio::join!`
- Bounded channels for backpressure
- Non-blocking operations throughout
- Cache automata to avoid expensive rebuilds

### X/Twitter Integration Plans

Adding X API to index bookmarked technical threads into the local knowledge graph. Interesting challenges:
- Rate limiting with backoff
- Incremental indexing
- Thread reconstruction
- Semantic concept extraction from tweets

### Links

GitHub: https://github.com/terraphim/terraphim-ai

Looking for feedback on:
- Architecture decisions
- Error handling patterns
- Testing strategies for async code
- Firecracker integration patterns

Apache 2.0. Contributions welcome.

---

## r/selfhosted Post

**Title:** Terraphim AI: Self-hosted privacy-first AI assistant that searches across local files, Notion, Obsidian, and more

**Body:**

For those who want AI assistance without cloud dependencies:

### What is it?

Terraphim AI is a self-hosted knowledge search engine that:
- Runs 100% locally on your hardware
- Searches across multiple knowledge sources from one interface
- Uses knowledge graphs for semantic search (not just keywords)
- Integrates with local LLMs (Ollama)

### Supported Sources

- Local filesystem (Markdown, code)
- Obsidian, Logseq, Notion
- Confluence, Jira
- StackOverflow, GitHub, Reddit
- Email (JMAP)
- Custom sources via API

### Installation

**Docker:**
```bash
docker run ghcr.io/terraphim/terraphim-server:latest
```

**Direct install:**
```bash
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash
```

**Homebrew:**
```bash
brew install terraphim/terraphim-ai/terraphim-ai
```

### Hardware Requirements

- Works on ARM and x86_64
- Docker images for linux/amd64, linux/arm64, linux/arm/v7
- Minimal resource usage (Rust backend)
- Ollama for local LLM (optional)

### Why Self-Host?

- Your private notes stay private
- Proprietary code never leaves your network
- No API costs or rate limits
- Deterministic, reproducible behavior
- Complete control over your AI assistant

### Coming Soon

X/Twitter API integration to index your bookmarked technical threads locally. All the knowledge from Twitter discussions, stored in your personal knowledge graph.

### Links

- GitHub: https://github.com/terraphim/terraphim-ai
- Discord: https://discord.gg/VPJXB6BGuY
- Apache 2.0 licensed

Anyone running similar setups? How do you handle knowledge fragmentation?
