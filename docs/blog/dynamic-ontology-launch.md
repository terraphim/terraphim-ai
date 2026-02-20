# Introducing Dynamic Ontology: Schema-First Knowledge Graphs

Knowledge graphs are powerful, but building them has always been the hard part. Traditional approaches require either expensive vector embeddings or manual schema design. Today, we're launching **Dynamic Ontology** - a schema-first approach that makes knowledge graph construction practical without the complexity.

## The Problem

Building knowledge graphs has been stuck in a rut:

1. **Vector embeddings are expensive** - Running BERT/embeddings for every entity is costly and slow
2. **Schema design is hard** - Deciding what entities and relationships to capture upfront is nearly impossible
3. **Quality is invisible** - How do you know if your extraction is actually working?

We've all been there - you build a knowledge graph, but you have no idea if it's actually capturing what's important.

## The Solution: Dynamic Ontology

Dynamic Ontology flips the script. Instead of designing a perfect schema upfront, it:

1. **Starts with your documents** - Pull entities and terms directly from your existing knowledge base
2. **Uses existing automata** - Our Aho-Corasick + fuzzy matching does the heavy lifting (no embeddings needed)
3. **Measures quality continuously** - Coverage signals tell you how well your ontology is working

### How It Works

```
Your Documents → Extract Terms → Build Ontology → Extract Entities → Quality Signal
```

The magic is in the feedback loop. Coverage signals tell you what you're missing, so you can expand your ontology intelligently.

## Under the Hood

We implemented this with a multi-agent architecture:

- **Extraction Agent** - LLM-based entity extraction with schema signals
- **Normalization Agent** - Grounds entities to canonical terms using graph + fuzzy matching
- **Coverage Agent** - Computes quality governance signals
- **Review Agent** - QA for low-confidence matches

### Feature Gates for Flexibility

We built this with Rust's feature gates so you only pay for what you need:

- **`ontology`** (default) - Core generic types work for any domain
- **`medical`** - Medical/oncology entity types when you need them
- **`hgnc`** - HGNC gene normalization (EGFR, TP53, KRAS, etc.)

```rust
// Just want the basics?
terraphim_types = { features = ["ontology"] }

// Building a medical knowledge graph?
terraphim_types = { features = ["hgnc"] }
```

### No Vector Embeddings Required

Here's the thing - we don't use vector embeddings at all. The existing Terraphim automata (Aho-Corasick + fuzzy matching) combined with graph ranking does the job. This means:

- Fast extraction (milliseconds, not seconds)
- No ML infrastructure to manage
- Deterministic results you can explain

## Real Results

We tested Dynamic Ontology on a corpus of 1,013 documents from our knowledge base:

- **4,090 terms** extracted into the ontology
- **Domain concepts** like "knowledge graphs", "ontology", "schema", "inference" correctly identified
- **Coverage signals** correctly flag when quality needs attention

The system knows what it knows - and more importantly, knows what it doesn't know.

## Get Started

```bash
# Run the knowledge graph normalization example
cargo run --example kg_normalization -p terraphim_types

# Or with HGNC gene normalization
cargo run --example ontology_usage -p terraphim_types --features hgnc
```

Check out the [documentation](./dynamic-ontology.md) for full details.

## What's Next

This is just the beginning. We're working on:

- Automatic ontology expansion based on coverage gaps
- Integration with external ontologies (UniProt, ChEBI)
- Batch processing for large document sets

The foundation is solid - now we build on it.

---

*Dynamic Ontology is available now in terraphim_types 1.4+. Star us on GitHub to follow the journey.*
