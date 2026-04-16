# Automata Evaluation Framework

The evaluation framework measures classification accuracy for automata-based term matching. It
compares `find_matches()` output against human-labeled ground-truth documents and reports
micro-averaged precision, recall, and F1, per-term breakdowns, and systematic error detection.

Source: `crates/terraphim_automata/src/evaluation.rs`

## When to use it

Use evaluation when you change a thesaurus and want to verify the automata still classify
documents correctly, or when you suspect a pattern is producing too many false positives.

## Ground-truth format

Create a JSON array of documents, each with an `id`, `text`, and `expected_terms` list:

```json
[
  {
    "id": "doc1",
    "text": "the rust language is fast and memory-safe",
    "expected_terms": [
      { "term": "rust", "category": null },
      { "term": "memory-safe", "category": "security" }
    ]
  },
  {
    "id": "doc2",
    "text": "tokio powers async rust applications",
    "expected_terms": [
      { "term": "tokio", "category": null },
      { "term": "rust", "category": null }
    ]
  }
]
```

The `term` field must match the normalized term value (`nterm`) in the thesaurus. The
`category` field is optional and reserved for future per-category metrics.

Load with `load_ground_truth(path: &Path)`.

## API

```rust
use terraphim_automata::evaluation::{evaluate, load_ground_truth};

let docs = load_ground_truth(Path::new("ground_truth.json"))?;
let result = evaluate(&docs, thesaurus);

println!("Precision: {:.2}", result.overall.precision);
println!("Recall:    {:.2}", result.overall.recall);
println!("F1:        {:.2}", result.overall.f1);
```

### Types

| Type | Description |
|------|-------------|
| `GroundTruthDocument` | Single document with `id`, `text`, and `expected_terms` |
| `ExpectedMatch` | Expected term with optional `category` |
| `EvaluationResult` | Top-level result: `overall`, `per_term`, `systematic_errors` |
| `ClassificationMetrics` | `precision`, `recall`, `f1`, `true_positives`, `false_positives`, `false_negatives` |
| `TermReport` | Per-term `ClassificationMetrics` with the `term` name |
| `SystematicError` | Term flagged as consistently producing false positives, with `document_ids` |

## Metrics explained

Metrics are **micro-averaged**: counts are summed across all documents before dividing.

- **Precision** = TP / (TP + FP) — of all matched terms, how many were expected
- **Recall** = TP / (TP + FN) — of all expected terms, how many were matched
- **F1** = harmonic mean of precision and recall

A term is flagged as a `SystematicError` when it appears as a false positive in 2 or more
documents. Common words added to the thesaurus accidentally (e.g. "the") will surface here.

## Matching rules

The automaton uses case-insensitive matching. "Rust" in the text matches the nterm "rust" in
the thesaurus. Each term is counted at most once per document regardless of how many times it
appears in the text.
