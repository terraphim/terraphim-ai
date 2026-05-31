# Terraphim Knowledge Graph Configuration

This directory contains role-based knowledge graph (KG) configurations for `terraphim_grep`.

## Structure

```
.terraphim/
├── config.toml              # Role definitions and settings
├── kg/                      # Knowledge graph concept files
│   ├── ai-engineer/         # AI/ML concepts
│   ├── devops/              # DevOps concepts
│   └── rust-engineer/       # Rust concepts
├── thesaurus-*.json         # Auto-generated Aho-Corasick thesauri
└── role-*.json              # Role-specific LLM configurations
```

## Roles

### DevOps
- 6 concepts: Caddy, Docker Compose, GitHub Actions, Monitoring, Secrets
- 37 synonyms for fast matching

### Rust Engineer
- 9 concepts: Concurrency, Error Handling, Ownership, Performance, Serde, Testing, Tokio, Traits, WebAssembly
- 92 synonyms for fast matching

### AI Engineer
- 6 concepts: Agent Patterns, Embedding, LLM, Prompt Engineering, Providers, RAG
- 61 synonyms for fast matching

## Thesaurus Generation

Thesaurus JSON files are auto-generated from KG markdown files. Each markdown file contains `synonyms::` directives that are extracted and compiled into an Aho-Corasick automaton for fast substring matching.

### Manual Regeneration

If you modify KG markdown files, regenerate the thesaurus:

```bash
cargo run --bin terraphim-grep -- generate-thesaurus --role rust-engineer
```

Or manually update the JSON file by extracting synonyms from all markdown files in the role directory.

### Format

Thesaurus JSON structure:
```json
{
  "concepts": [
    {
      "name": "Concept Name",
      "synonyms": ["synonym1", "synonym2"],
      "relationships": ["related1", "related2"]
    }
  ]
}
```

## KG Curation (Learning Loop)

When `terraphim_grep` is run with `--kg-path` and `--answer`, the RLM synthesis step extracts new concepts from the LLM response and persists them as markdown files:

```bash
terraphim-grep --role rust-engineer --kg-path .terraphim/kg/rust-engineer --answer "how does tokio work"
```

New concepts are written to `kg_path/learned-<slug>.md` with synonyms and relationships.

## Usage

### Search-only (no LLM)
```bash
terraphim-grep --role devops --thesaurus .terraphim/thesaurus-devops.json --paths . "pipeline"
```

### With RLM synthesis + KG curation
```bash
OPENROUTER_API_KEY=sk-or-... terraphim-grep --role devops \
  --thesaurus .terraphim/thesaurus-devops.json \
  --paths . --answer "pipeline"
```

## Contributing

When adding new concepts:
1. Create a markdown file in `kg/{role}/`
2. Include synonyms using the `synonyms::` directive
3. Regenerate the thesaurus JSON
4. Test with `terraphim-grep --role {role} --answer "your query"`
