+++
title="FFF vs Ripgrep: Understanding the Difference"
date=2026-05-02
[taxonomies]
categories = ["Technical"]
tags = ["Terraphim", "FFF", "Ripgrep", "knowledge-graph", "haystack"]
[extra]
toc = true
+++

A common point of confusion in Terraphim is understanding the difference between FFF (fzf-like fuzzy file finder) and Ripgrep haystack. This post clarifies what each does and how they complement each other.

<!-- more -->

## TL;DR

**FFF is NOT a haystack.** FFF provides MCP tools for fuzzy file finding and content grep. Ripgrep is a haystack ServiceType for exhaustive content search. They serve different purposes and can be used together.

## Haystack Architecture

In Terraphim, a **haystack** is a searchable data source configured in your role definition:

```json
{
  "haystacks": [
    {
      "location": "/path/to/project",
      "service": "Ripgrep",
      "read_only": true
    }
  ]
}
```

**Available haystack ServiceTypes**:

| ServiceType | Purpose | KG Ranking |
|-------------|---------|------------|
| `Ripgrep` | Local content search | Via relevance function |
| `GrepApp` | GitHub code search | Via relevance function |
| `Atomic` | Atomic Server | Via relevance function |
| `QueryRs` | Rust API docs | Via relevance function |
| `Quickwit` | Log search | Via relevance function |
| `Jmap` | Email search | Via relevance function |

## FFF: MCP Tools, Not a Haystack

FFF (Fast File Finder) is a **separate system** that provides MCP tools for AI coding agents. It's not a haystack and cannot be configured as one.

### FFF MCP Tools

| Tool | Purpose | Output |
|------|---------|--------|
| `terraphim_find_files` | Fuzzy file path search | Ranked list of file paths |
| `terraphim_grep` | Content grep with KG ordering | Lines with matches |
| `terraphim_multi_grep` | Multi-pattern Aho-Corasick grep | Lines matching any pattern |

### FFF Architecture

```
Query → FilePicker (scan files) → fuzzy_search → KG scorer (path boost) → ranked results
Query → File list → KG scorer (sort by path score) → grep_search → matches
```

Key features:
- **Fuzzy matching** on file paths (not content)
- **Frecency tracking** (frequency + recency of file access)
- **KG path scoring** via `KgPathScorer` (ExternalScorer trait)
- **Aho-Corasick** multi-pattern grep

## Comparison Table

| Aspect | Ripgrep Haystack | FFF Tools |
|--------|------------------|-----------|
| **Type** | Haystack (configured in role) | MCP tools |
| **Content search** | Yes (regex, exhaustive) | Yes (fff grep engine) |
| **File path search** | No | Yes (fuzzy) |
| **Frecency** | No | Yes |
| **KG integration** | Relevance function | kg_scorer (path boost) |
| **Output format** | Ranked documents | Files + content lines |
| **Pagination** | No | Cursor-based |

## How They Work Together

### Ripgrep Haystack Flow

```
Query → terraphim search → role relevance function → Ripgrep → KG concept match → ranked docs
```

1. Query hits `terraphim search` command
2. Role's relevance function (e.g., `terraphim-graph`) scores documents
3. Ripgrep searches content
4. KG concept matching boosts conceptually relevant docs

### FFF MCP Tool Flow

```
Query → terraphim_find_files → fuzzy path match → KG scorer (path) → ranked paths
Query → terraphim_grep → KG scorer (sort files) → grep → matches
```

1. AI agent calls `terraphim_find_files` or `terraphim_grep` directly
2. FFF scans files and applies fuzzy matching
3. KG scorer boosts files with KG concept matches in path
4. Results returned directly to AI agent

## When to Use Each

### Use Ripgrep Haystack When:
- You want exhaustive content search
- You need regex support
- You're using `terraphim search` command
- You want unified ranking across multiple haystacks

### Use FFF Tools When:
- You want fuzzy file path finding (find files by name)
- You need frecency (smart ordering by access frequency)
- AI agent needs direct MCP tool access
- You're doing interactive file navigation

### Use Both When:
- Ripgrep haystack for content search
- FFF for file path navigation
- Terraphim REPL for exploration

## Configuration Example

### Role with Ripgrep Haystack

```json
{
  "roles": {
    "Frontend Developer": {
      "relevance_function": "terraphim-graph",
      "haystacks": [
        {
          "location": "/path/to/project",
          "service": "Ripgrep"
        }
      ]
    }
  }
}
```

### FFF MCP Tools

FFR tools are registered separately in OpenCode:

```json
{
  "mcp": {
    "terraphim": {
      "type": "local",
      "command": ["/path/to/terraphim_mcp_server"]
    }
  }
}
```

Tools available: `terraphim_find_files`, `terraphim_grep`, `terraphim_multi_grep`

## Key Takeaway

FFF and Ripgrep are **complementary**, not competing:

- **Ripgrep haystack** = structured haystack search with KG relevance
- **FFF tools** = fuzzy file finding + fast grep with KG path scoring

Both integrate with the knowledge graph but in different ways. Use Ripgrep haystack for thorough content search. Use FFF for quick file finding and when AI agents need direct tool access.
