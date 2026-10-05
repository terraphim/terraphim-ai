# terraphim_lsp_core

Pure knowledge-graph analysis core shared by `terraphim_lsp` (the language
server) and terraphim-editor (in-process, compiled to WebAssembly).

No LSP, async runtime or I/O dependencies: the crate builds for
`wasm32-unknown-unknown`.

## What it does

In a Terraphim knowledge graph every synonym of a concept shares the
concept's id, so the synonyms are the alternatives for a matched word.

- `KgEngine::analyse(text)`: KG terms with concept id, normalised concept
  name (`nterm`), the matched key and text, and byte plus UTF-16 ranges.
- `KgEngine::alternatives_at(text, byte)` / `alternatives_at_utf16` /
  `alternatives_for(text, &term)`: the other terms of the concept, current
  form excluded, as `Replacement`s that keep the original capitalisation
  (lower, Sentence, Title Words, ALL CAPS; capitals are only ever added) and
  switch a preceding `a`/`an` in the same set of edits.
- `split_annotation_block(text)`: locates the trailing
  `terraphim-alternatives` fenced block so it is never analysed. A malformed
  block (no closing fence, text after it, or invalid JSON) yields exactly one
  diagnostic; the body is never modified. Schema validation stays with
  terraphim-editor's block parser.

Matching and the concept -> synonyms index are `terraphim_automata`'s
`CompiledMatcher` and `ConceptIndex`. All API types are plain data with serde
derives, so they pass through `wasm-bindgen` as JSON.

## Example

```rust
use terraphim_lsp_core::{KgEngine, apply_edits};

let engine = KgEngine::from_json(r#"{"name": "demo", "data": {
    "eraser": {"id": 1, "nterm": "eraser"},
    "rubber": {"id": 1, "nterm": "eraser"}
}}"#)?;
let text = "An eraser helps.";
let set = engine.alternatives_at(text, 4).unwrap();
assert_eq!(apply_edits(text, &set.replacements[0].edits), "A rubber helps.");
# Ok::<(), terraphim_lsp_core::CoreError>(())
```

## Checks

```bash
cargo test -p terraphim_lsp_core
cargo build -p terraphim_lsp_core --target wasm32-unknown-unknown
cargo bench -p terraphim_lsp_core
```
