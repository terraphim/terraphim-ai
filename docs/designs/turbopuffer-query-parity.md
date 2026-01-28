# turbopuffer `ContainsAnyToken` vs Terraphim graph/embedding search

This note compares turbopuffer's full‑text filter parameter
[`ContainsAnyToken`](https://turbopuffer.com/docs/query#param-ContainsAnyToken)
with Terraphim's search stack (BM25 family + TerraphimGraph rolegraph).

## What turbopuffer `ContainsAnyToken` means

From turbopuffer docs:

- `ContainsAnyToken` **matches documents that contain _any_ of the tokens** present in the filter input string.
- Requires the attribute to be configured for **full‑text search**.

In practice this is a **token‑level OR filter**:

- Input: `"lazy walrus"`
- Matches a doc that contains `lazy` **or** `walrus` (tokenization rules apply).

## Closest Terraphim equivalent (document filtering)

Terraphim's closest equivalent of token‑OR matching is:

- `SearchQuery` with multiple terms and `LogicalOperator::Or`.
- `SearchQuery::get_operator()` defaults to **OR** for multi‑term queries.

Implementation details:

- `terraphim_service::Service::apply_logical_operators_to_documents()`
  - For `LogicalOperator::Or`: keeps a document if it matches **ANY** term.
  - Uses `term_matches_with_word_boundaries()` (regex `\bterm\b`) to avoid partial matches.

### Important differences

- **Tokenization**: turbopuffer uses its internal tokenizer for FTS attributes;
  Terraphim currently uses a **word boundary regex** match against concatenated
  `title + body + description`.
- **Unicode / punctuation**: `\b` boundaries are ASCII/regex‑word‑boundary semantics;
  turbopuffer tokenization will differ (especially for emojis, hyphenated words, CJK, etc.).
- **Field awareness**: turbopuffer applies `ContainsAnyToken` to a specific FTS attribute;
  Terraphim applies OR matching across a merged text.

## Closest Terraphim equivalent (TerraphimGraph / rolegraph)

For TerraphimGraph, the analogue is not “tokens” but **thesaurus/ontology terms**.

- `terraphim_rolegraph::RoleGraph::find_matching_node_ids(text)`
  - Uses Aho‑Corasick over **thesaurus entries/synonyms**.
  - Returns all node IDs that match anywhere in `text`.

This is closer to:

- “contains any **known concept**”

…rather than “contains any token”. It’s concept‑level matching.

### Why this matters

This distinction is central to Terraphim’s graph embeddings design:

- turbopuffer `ContainsAnyToken` = **lexical token filter** (recall depends on tokenizer).
- Terraphim rolegraph matching = **ontology/concept filter** (recall depends on thesaurus coverage).

You can (and often should) combine both:

- Use a lexical scorer (BM25/BM25F/BM25+) to get broad recall.
- Use rolegraph (TerraphimGraph) to re‑rank / enrich based on concept connectivity.

## Proposed alignment opportunities (future work)

If we want Terraphim to have a closer mental model to turbopuffer FTS filters:

1. Introduce a **real tokenizer** for OR/AND filtering (Unicode‑aware word breaks).
2. Apply OR/AND matching **per field**, optionally with weights.
3. Add an explicit “token filter” abstraction that parallels:
   - `ContainsAnyToken`
   - `ContainsAllTokens`
   - `ContainsTokenSequence`

Key takeaway: turbopuffer’s `ContainsAnyToken` maps cleanly to Terraphim’s
multi‑term OR filter at the document layer, while TerraphimGraph is a different
(concept) layer.
