---
title: "Introducing Multi-Haystack Roles: Local + Global Code Search"
date: 2026-02-16
author: Terraphim Team
tags: [release, features, grepapp, haystack]
---

# Introducing Multi-Haystack Roles: Local + Global Code Search

Today we're excited to announce a significant enhancement to Terraphim's engineer roles: **multi-haystack support**. FrontEnd Engineers and Python Engineers now have access to both local code search (via Ripgrep) and global GitHub search (via GrepApp) in a single query.

## The Problem: Siloed Search

Traditionally, developers face a frustrating choice when searching for code:

1. **Search locally** - Fast, but limited to your own codebase
2. **Search GitHub** - Broad, but requires leaving your workflow

When you're stuck on "how do I use this library?" or "what's the idiomatic way to solve this?", you need both: your local context for project-specific code, and global examples from the broader community.

## The Solution: Dual Haystacks

Starting with Terraphim v1.8.1, select engineer roles now come with **dual haystacks**:

### FrontEnd Engineer
- **Haystack 1**: Local Ripgrep search (`~/projects` or custom path)
- **Haystack 2**: GrepApp global search (filtered to JavaScript)

### Python Engineer
- **Haystack 1**: Local Ripgrep search (`~/projects` or custom path)
- **Haystack 2**: GrepApp global search (filtered to Python)

### Rust Engineer v2 (already available)
- **Haystack 1**: QueryRs (docs.rs search)
- **Haystack 2**: Local Ripgrep search

## How It Works

When you search as a FrontEnd or Python Engineer, Terraphim queries both sources simultaneously:

```
Your Query: "how to use useEffect"
    ↓
┌─────────────────┐     ┌──────────────────┐
│  Local Ripgrep  │     │  GrepApp         │
│  (your code)    │     │  (GitHub repos)  │
└────────┬────────┘     └────────┬─────────┘
         │                       │
         └───────────┬───────────┘
                     ↓
         Combined Results
         (ranked by relevance)
```

The results are merged and ranked according to your role's relevance function:
- **FrontEnd Engineer**: BM25Plus ranking
- **Python Engineer**: BM25F field-weighted ranking

This means you get the best of both worlds: your project's specific implementations AND real-world examples from popular repositories.

## Real-World Use Cases

### Learning a New Library

You're trying to use `react-query` for the first time. A single search shows:
- How you've used it elsewhere in your codebase (local)
- How Facebook uses it in their production apps (global)

### Debugging Common Patterns

You're debugging a Python asyncio issue. Your search returns:
- Your current implementation (local)
- How aiohttp, FastAPI, and Django handle similar patterns (global)

### Discovering Idioms

You want to know the "Pythonic" way to do something. Instead of reading docs, you can see:
- How the standard library implements it
- How popular open-source projects handle similar cases

## Technical Implementation

Under the hood, this uses our existing GrepApp integration:

```rust
Haystack {
    location: "https://grep.app".to_string(),
    service: ServiceType::GrepApp,
    read_only: true,
    fetch_content: false,
    extra_parameters: {
        let mut params = HashMap::new();
        params.insert("language".to_string(), "python".to_string());
        params
    },
}
```

The `extra_parameters` field allows language-specific filtering:
- FrontEnd Engineer: `language=javascript`
- Python Engineer: `language=python`

GrepApp returns structured results from millions of GitHub repositories, complete with:
- Repository name and file path
- Code snippet with context
- Direct link to view on GitHub
- Branch information

## Graceful Degradation

We know network availability isn't guaranteed. If GrepApp is unreachable:
- Local search continues working normally
- No errors or interruptions
- You still get your project-specific results

This follows our philosophy: **local-first, enhance with global**.

## What's Next

This is just the beginning. We're planning:

1. **More roles with dual haystacks**:
   - Go Engineer (GrepApp + local)
   - TypeScript-specific role
   - Language-agnostic search role

2. **Configurable filters**:
   - Filter by specific repositories
   - Filter by file paths
   - Filter by popularity/stars

3. **Enhanced ranking**:
   - Boost results from starred repositories
   - Learn from your click patterns
   - Personalized ranking per engineer

## Try It Now

If you're already using Terraphim v1.8.1+, you can try the new roles immediately:

```bash
# Switch to FrontEnd Engineer (with dual haystack)
terraphim-agent onboard --role frontend-engineer

# Switch to Python Engineer (with dual haystack)
terraphim-agent onboard --role python-engineer

# Search as usual - both local and global results appear
terraphim-agent search "async def"
```

## The Bigger Picture

Multi-haystack roles represent a fundamental shift in how we think about code search:

**Old model**: One haystack per role, choose your scope
**New model**: Multiple haystacks per role, get comprehensive results

This aligns with how developers actually work: they don't want to choose between "my code" and "the world's code" - they want both, intelligently combined.

As we expand this pattern to more roles and add more haystack types (MCP servers, Atomic Data, AI assistants), Terraphim becomes not just a search tool, but a **knowledge synthesis engine** for developers.

## Feedback Welcome

Have ideas for other haystack combinations? Want to see this pattern applied to other roles? Let us know:

- GitHub Issues: [github.com/terraphim/terraphim-ai/issues](https://github.com/terraphim/terraphim-ai/issues)
- Discussions: [github.com/terraphim/terraphim-ai/discussions](https://github.com/terraphim/terraphim-ai/discussions)

---

**Related Reading:**
- [Terraphim v1.8.1: Native Hook Support](/blog/2026-02-16-native-hook-support)
- [Introducing Multi-Haystack Roles](/docs/guides/multi-haystack-roles)
- [GrepApp Integration Deep Dive](/docs/integrations/grepapp)

*Terraphim: Search your world.*
