# Hacker News Submission

## Primary Submission (Show HN)

**Title:** Show HN: Terraphim AI - Privacy-first local knowledge search with semantic graphs

**URL:** https://github.com/terraphim/terraphim-ai

**Text (for "Ask HN" variant):**

I built Terraphim AI to solve a problem I kept hitting: knowledge fragmentation.

My notes are in Obsidian. Team docs in Confluence. Code discussions in GitHub. Technical insights scattered across Twitter threads I bookmarked. When I need to find something, I'm searching 5+ tools hoping I remember where I saw it.

Terraphim unifies search across all these sources while keeping everything local. No uploading to cloud AI services. No privacy trade-offs.

**Technical approach:**

- Knowledge graph with semantic thesaurus (not just keyword search)
- Aho-Corasick automata for fast text matching
- Multiple scoring algorithms (BM25, BM25F, TerraphimGraph)
- Role-based personalization (different knowledge contexts)
- Rust backend with async/await throughout
- Ollama integration for local LLM
- WASM support for browser autocomplete

**Current integrations:** Local filesystem, Obsidian, Logseq, Notion, Confluence, Jira, StackOverflow, GitHub, Reddit, Email (JMAP)

**Building next:** X/Twitter API integration to index bookmarked technical threads into the local knowledge graph.

The privacy angle isn't just philosophical - it's practical. When you're searching proprietary code, client documents, or private research notes, uploading to external servers isn't acceptable.

Would love feedback on:
- The knowledge graph approach vs traditional vector embeddings
- Scoring function trade-offs (BM25 family vs graph-based)
- Interest in specific integrations (Slack? Discord? Linear?)

Apache 2.0. Written in Rust. Contributions welcome.

---

## Alternative Titles (A/B Testing)

1. "Show HN: Privacy-first AI assistant that searches local files, Notion, Obsidian without cloud upload"

2. "Show HN: Local knowledge graph search across 10+ sources - Rust, Ollama, no cloud required"

3. "Show HN: Terraphim - Semantic search across your fragmented knowledge (local-first)"

4. "Show HN: Building a local AI assistant with knowledge graphs instead of vector embeddings"

5. "Show HN: Open-source alternative to cloud AI assistants - runs 100% locally with semantic search"

---

## Comment Response Templates

### For "How is this different from X?" questions:

**vs. Perplexity/ChatGPT:**
The core difference is privacy architecture. Those services require sending your queries and documents to their servers. Terraphim runs entirely locally - your private notes, proprietary code, and confidential docs never leave your machine.

Beyond privacy, we use knowledge graphs with semantic thesaurus instead of just vector embeddings. This means:
- Explicit concept relationships (not just similarity)
- Deterministic scoring (reproducible results)
- Role-based personalization (different knowledge contexts)
- No embedding model training required

**vs. Obsidian/Logseq search:**
Terraphim searches across tools, not within one tool. Your Obsidian notes + Confluence docs + GitHub repos + StackOverflow + bookmarked Twitter threads - all from one search query.

Also, semantic graph search means "async cancellation" finds notes about "task cancellation" and "abort patterns" even if those exact words aren't present.

**vs. OpenAI/Anthropic APIs:**
Those are LLM APIs that process your data on their servers. Terraphim is a local search engine that optionally uses Ollama (local LLM) for summarization. Your data stays local.

### For technical architecture questions:

**Knowledge Graph vs Vector DB:**
We use Aho-Corasick automata built from a semantic thesaurus. Benefits:
- Explicit concept mapping (engineer knows what "relates to" what)
- Fast O(n) text matching
- Deterministic results
- No embedding model dependencies
- WASM-compatible (runs in browser)

Trade-offs:
- Requires building thesaurus (vs. auto-generated embeddings)
- Less "fuzzy" matching (precise concept boundaries)

For some use cases, vector DBs are better. For structured domain knowledge where you control the concept taxonomy, knowledge graphs give more predictable results.

**Why Rust?**
- Performance for text processing (matching millions of documents)
- Memory safety for long-running service
- WASM compilation for browser autocomplete
- Strong async ecosystem (tokio)
- Type safety across 29-crate workspace

### For integration questions:

**Twitter/X Integration:**
Planning to use X API v2 to:
- Index user's bookmarked tweets/threads
- Track specific accounts' technical discussions
- Extract concepts for thesaurus enrichment
- Reconstruct thread context

Challenges: Rate limits, thread reconstruction, incremental indexing. Will share updates on GitHub.

**Why not Slack/Discord first?**
Good question. X has unique value for public technical discourse - solutions shared in threads that aren't documented elsewhere. But Slack/Discord for team conversations is definitely on the roadmap. What would you prioritize?

---

## Follow-up Comment (Post-Submission)

**After a few hours, post this as a top-level comment:**

Author here. Few clarifications based on questions:

**Installation is simple:**
```bash
# One-liner
curl -fsSL https://raw.githubusercontent.com/terraphim/terraphim-ai/main/release/v0.2.3/install.sh | bash

# Or Docker
docker run ghcr.io/terraphim/terraphim-server:latest

# Or Homebrew
brew install terraphim/terraphim-ai/terraphim-ai
```

**LLM is optional:**
The core search works without any LLM. If you want AI features (summarization, chat), plug in Ollama locally. No OpenAI API key needed.

**Resource usage:**
Rust backend is lightweight. The expensive part is building automata from large thesauruses - we cache these. Runtime search is O(n) on document text.

**Mobile app?**
Not yet. Desktop (Tauri), web UI, and TUI currently. Mobile would require rethinking the local-first architecture.

**Commercial plans?**
Open source under Apache 2.0. Might build hosted version for teams eventually, but core stays open and local-first.

Happy to answer technical questions about the knowledge graph implementation or Rust architecture choices.
