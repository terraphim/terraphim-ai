# Automata Paragraph Extraction

Goal: Given a thesaurus-backed automata, find paragraphs in a text starting from matched automata terms.

## Plan
- Build/obtain a `Thesaurus` for the automata
- Use `find_matches(text, thesaurus, return_positions=true)` to get term matches with positions
- For each match:
  - Determine paragraph start: either at the start of the term, or right after the term
  - Determine paragraph end: the earliest blank-line separator (`\n\n`, `\r\n\r\n`, or `\r\r`) after the term; fallback to end-of-text
  - Slice the substring and return alongside the `Matched` metadata
- Return `Vec<(Matched, String)>` for downstream consumers

## Function
Implemented in `crates/terraphim_automata/src/matcher.rs`:

- `extract_paragraphs_from_automata(text, thesaurus, include_term)`
- `find_paragraph_end(text, from_index)`

## Example
```rust
use terraphim_automata::matcher::extract_paragraphs_from_automata;
use terraphim_types::{Thesaurus, NormalizedTerm, NormalizedTermValue};

let mut thesaurus = Thesaurus::new("test".to_string());
let norm = NormalizedTerm::new(1, NormalizedTermValue::from("lorem"));
thesaurus.insert(NormalizedTermValue::from("lorem"), norm);

let text = "Intro\n\nlorem ipsum dolor sit amet,\nconsectetur adipiscing elit.\n\nNext paragraph starts here.";
let results = extract_paragraphs_from_automata(text, thesaurus, true)?;
assert!(!results.is_empty());
```

## Tests
Run crate tests (tokio is only needed for remote loading; these tests do not require async):
```bash
cargo test -p terraphim_automata
```

## Benchmarks
Paragraph extraction benchmark is added to the existing Criterion suite:
```bash
cargo bench -p terraphim_automata --bench autocomplete_bench
```
Look for: "extract_paragraphs_from_automata_small_text" in the output.
