# Design: Enhance claude-log-analyzer with terraphim_automata

## Problem Statement

The claude-log-analyzer crate has terraphim_automata integration behind a feature flag, but underutilizes its capabilities:
- Only uses `find_matches()` for basic pattern matching
- Hardcodes concept definitions instead of dynamic extraction
- Clones thesaurus on every call (inefficient)
- Doesn't use fuzzy matching, autocomplete, or graph connectivity features

## Goals

1. Better text processing using terraphim_automata capabilities
2. Dynamic concept/pattern learning from observed tool usage
3. Efficient thesaurus management with caching
4. Leverage graph connectivity for relationship inference

## Implementation Plan

### Step 1: Improve TerraphimMatcher Pattern Matching

**File**: `src/patterns/matcher.rs`

Current:
```rust
terraphim_find_matches(text, thesaurus.clone(), true)
```

Changes:
- Cache compiled automata instead of rebuilding
- Use fuzzy matching for typo tolerance
- Add context extraction for better pattern understanding

### Step 2: Dynamic Concept Building in KnowledgeGraphBuilder

**File**: `src/kg/builder.rs`

Current: Hardcoded concept lists (BUN, NPM, INSTALL, etc.)

Changes:
- Learn concepts dynamically from observed patterns
- Use terraphim_automata's thesaurus capabilities
- Build hierarchical concept relationships

### Step 3: Enhanced Search with Graph Connectivity

**File**: `src/kg/search.rs`

Current: Manual proximity-based result merging

Changes:
- Use `is_all_terms_connected_by_path()` for semantic relationships
- Improve relevance scoring with concept graph distances
- Cache search results for repeated queries

### Step 4: Integrate Learned Patterns with Terraphim

**File**: `src/patterns/knowledge_graph.rs`

Current: Separate learning system from terraphim

Changes:
- Store learned patterns in terraphim thesaurus
- Use graph structure for relationship inference
- Export/import learned knowledge

## File Changes

| File | Action | Purpose |
|------|--------|---------|
| `src/patterns/matcher.rs:159-290` | Modify | Cache automata, add fuzzy matching |
| `src/kg/builder.rs` | Modify | Dynamic concept learning |
| `src/kg/search.rs` | Modify | Graph connectivity for search |
| `src/patterns/knowledge_graph.rs` | Modify | Integrate with terraphim learning |

## Acceptance Criteria

1. Pattern matching uses cached automata (no clone per call)
2. Concepts learned dynamically from tool observation
3. Fuzzy matching available for typo-tolerant search
4. Tests pass with `--features terraphim`
