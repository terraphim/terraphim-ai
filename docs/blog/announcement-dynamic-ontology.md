# Dynamic Ontology Launch Announcement

## Twitter/X Thread

**Tweet 1:**
Just shipped Dynamic Ontology - a schema-first approach to knowledge graphs that actually works.

No vector embeddings. No expensive ML. Just your documents + automata + coverage signals.

Here's what makes it different...

**Tweet 2:**
The problem with knowledge graphs: you build one but have no idea if it's working.

Dynamic Ontology solves this with Coverage Signals - continuous quality governance that tells you exactly how well your extraction is performing.

**Tweet 3:**
We tested on 1,013 documents:
- 4,090 domain terms extracted
- Real concepts identified (ontology, schema, inference, entity)
- Quality measurable and actionable

Coverage: 70%+ = proceed, <20% = rethink approach

**Tweet 4:**
Built with Rust feature gates so you only pay for what you need:
- `ontology` - core generic types (default)
- `medical` - medical entity types
- `hgnc` - gene normalization (EGFR, TP53, KRAS...)

```bash
cargo run --example kg_normalization -p terraphim_types
```

**Tweet 5:**
The best part: no vector embeddings required. Uses existing Aho-Corasick + fuzzy matching. Fast, deterministic, explainable.

Docs: https://github.com/terraphim/terraphim-ai/tree/main/docs/src/dynamic-ontology.md

Star us to follow the journey.

---

## LinkedIn Post

Excited to announce Dynamic Ontology - a practical approach to knowledge graphs that makes extraction quality visible.

After months of research and implementation, we've shipped a schema-first methodology that:

1. Extracts entities from your existing documents
2. Builds ontology automatically
3. Measures coverage continuously
4. Grounds entities to canonical URIs

The key insight? You don't need vector embeddings. The existing automata (Aho-Corasick + fuzzy matching) combined with graph ranking does the job - faster, cheaper, and more explainable.

We tested on 1,013 documents and extracted 4,090 domain-specific terms. The system correctly identifies concepts like "knowledge graphs", "ontology", "schema", and "inference" - and importantly, tells you when it's missing something.

Coverage signals provide continuous governance:
- 70%+ coverage = ready to use
- 40-70% = minor review needed
- <20% = different approach needed

Built with Rust feature gates for flexibility - enable medical types or HGNC gene normalization when you need it.

Check out the docs and example in our GitHub repo.

#KnowledgeGraph #AI #DataScience #Rust

---

## Hacker News Submission

**Title:** Dynamic Ontology: Schema-first knowledge graphs without vector embeddings

**Body:**
We've been working on a practical approach to knowledge graphs that doesn't require expensive vector embeddings or perfect schema design upfront.

Key features:
- Extract entities from documents automatically
- Build ontology from your corpus
- Coverage signals measure extraction quality
- No ML infrastructure needed

The insight: existing Aho-Corasick automata + fuzzy matching handles normalization fine. What we needed was a feedback loop - coverage signals that tell you what you're missing so you can expand intelligently.

Tested on 1,013 documents - 4,090 terms extracted, domain concepts identified correctly.

https://github.com/terraphim/terraphim-ai/tree/main/docs/src/dynamic-ontology.md

---

## GitHub Release Notes

## Dynamic Ontology v1.4

We're excited to announce Dynamic Ontology - a schema-first approach to knowledge graph construction.

### What's New

- **GroundingMetadata** - Canonical URIs for normalized entities
- **CoverageSignal** - Quality governance signals
- **SchemaSignal** - Entity extraction with confidence scores
- **HgncNormalizer** - Gene normalization (EGFR, TP53, KRAS, etc.)

### Feature Gates

| Feature | Description |
|---------|-------------|
| `ontology` | Core generic types (default) |
| `medical` | Medical entity types |
| `hgnc` | HGNC gene normalization |

### Example

```bash
cargo run --example kg_normalization -p terraphim_types
cargo run --example ontology_usage -p terraphim_types --features hgnc
```

### Documentation

See [docs/src/dynamic-ontology.md](./docs/src/dynamic-ontology.md) for full documentation.

### Breaking Changes

- `ExtractedEntity.entity_type` is now `String` (was enum) - enables cross-domain use
- `ExtractedRelationship.relationship_type` is now `String` (was enum)
- Medical types moved to feature-gated `EntityType`/`RelationshipType` enums

---

*For additional announcement formats or localization, let me know.*
