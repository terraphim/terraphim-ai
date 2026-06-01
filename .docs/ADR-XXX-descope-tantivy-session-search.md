# ADR-XXX: Descope Tantivy Session Search in Favour of Hybrid FFF-Based Search

**Status**: Accepted
**Date**: 2026-05-30
**Author**: Alex
**Deciders**: Terraphim Team

## Context

During the 2026-05-30 planning session, we reviewed the backlog including issue #2288 which proposed implementing Tantivy full-text indexing for session search. The existing codebase already contains an FFF (Frecency, Fuzzy, Fast) search implementation in `terraphim_automata` which provides hybrid search capabilities combining:

- **Frecency**: Frequency-weighted results based on usage patterns
- **Fuzzy**: Fuzzy matching for typo tolerance
- **Fast**: Low-latency prefix-based autocomplete

### Current FFF Implementation

The `terraphim_automata` crate provides:
- `fffindexer` for building FFF indices
- Fuzzy autocomplete with Jaro-Winkler similarity
- Aho-Corasick multi-pattern matching
- Prefix-based term suggestions

### Tantivy Proposal (#2288)

The rejected proposal suggested adding:
- Full-text inverted index via Tantivy
- BM25 scoring
- Complex query parsing
- Additional memory and CPU overhead

## Decision

**Tantivy is descoped for session search.** The existing FFF-based hybrid search is retained and will be enhanced instead.

## Rationale

### FFF Advantages for Session Search

| Factor | FFF | Tantivy |
|--------|-----|---------|
| Latency | <10ms prefix search | 50-200ms queries |
| Memory | Minimal (FST-based) | Large (inverted index) |
| Fuzzy matching | Built-in Jaro-Winkler | Requires custom |
| Prefix autocomplete | Native | Not supported |
| Typo tolerance | Excellent | Requires fuzzing |
| Integration | Already in tree | New dependency |

### Session Search Requirements

Session search typically requires:
1. Fast prefix matching for autocomplete
2. Fuzzy matching for imperfect recall
3. Recent results weighted higher (frecency)
4. Low latency for interactive UIs

Tantivy's strengths (complex boolean queries, BM25 ranking, large corpus indexing) are not primary session search requirements.

### Risk of Adding Tantivy

| Risk | Impact |
|------|--------|
| Dependency bloat | New crate with own memory model |
| Query complexity | BM25 tuning parameters |
| Build overhead | Additional compilation time |
| Runtime memory | Inverted index size scales with corpus |
| Maintenance | Two search systems to maintain |

## Consequences

### Positive
- Single search implementation (FFF) reduces maintenance
- Lower memory footprint for session search
- Faster autocomplete responses
- Leverage existing investment in FFF

### Negative
- Some advanced query features (boolean operators) not available
- Exact-match BM25 ranking not available for long文档

### Mitigations
- FFF fuzzy matching handles most real-world typos
- Prefix search enables "search as you type"
- For complex queries, the broader terraphim search (which may use different backends) can be used

## Alternatives Considered

### 1. Keep Both (Rejected)
- **Option**: Use FFF for prefix/autocomplete, Tantivy for full-text
- **Rejected**: Increases complexity, two systems to maintain, unclear when to use which

### 2. Extend FFF with BM25 (Deferred)
- **Option**: Add BM25-style scoring to FFF
- **Rejected**: Not needed for current session search requirements; can revisit if gaps emerge

### 3. Hybrid FFF Enhancement (Chosen)
- **Option**: Extend FFF with better phrase matching and scoring
- **Accepted**: Aligns with existing architecture, low additional complexity

## Implementation Notes

### FFF Enhancement Roadmap

1. **Phase 1**: Ensure FFF index covers session content fully
2. **Phase 2**: Add positional information for phrase matching
3. **Phase 3**: Consider field-weighted scoring if needed

### Session Data Structure

Sessions should be indexed in FFF with:
- Session ID (unique key)
- Timestamp (for recency weighting)
- Role/context (for filtering)
- Content summary (for matching)

## References

- Issue #2288: "feat(sessions): implement Tantivy full-text index for session search"
- `crates/terraphim_automata/src/` - FFF implementation
- `crates/terraphim_automata/src/fffindexer.rs` - FFF indexer

---

**Note**: If future requirements demand advanced full-text features (boolean operators, phrase proximity, BM25), this decision should be revisited. The FFF approach should be profiled against real session search patterns before adding Tantivy.
