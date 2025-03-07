# BM25F Scorer

BM25F is an extension of the BM25 ranking function that handles multiple fields with different weights for structured documents. It's particularly well-suited for documents with fields like title, body, description, and tags.

## Overview

BM25F extends the classic BM25 algorithm by allowing different parts of a document to be weighted differently. For example, a match in the title field might be considered more important than a match in the body field.

The BM25F implementation in Terraphim AI assigns different weights to the following document fields:
- Title (weight: 3.0)
- Body (weight: 1.0)
- Description (weight: 2.0)
- Tags (weight: 2.5)

## How It Works

BM25F works by:

1. Calculating term frequencies across all fields, weighted by field importance
2. Applying document length normalization
3. Computing the Inverse Document Frequency (IDF) for each query term
4. Combining these factors to produce a final relevance score

The formula for BM25F is:

```
score(D,Q) = ∑(t in Q) IDF(t) * (weighted_tf(t,D) / (K1 * length_norm + weighted_tf(t,D)))
```

Where:
- `D` is a document
- `Q` is the query
- `t` is a term in the query
- `IDF(t)` is the inverse document frequency of term `t`
- `weighted_tf(t,D)` is the weighted term frequency of `t` in document `D`
- `K1` is a parameter that controls term frequency saturation (default: 1.2)
- `length_norm` is the document length normalization factor

## When to Use BM25F

BM25F is ideal for:

- Searching across structured documents with multiple fields
- When you want to prioritize matches in certain fields (like title or tags) over others
- When document length normalization is important for fair ranking

## Configuration

The BM25F scorer can be configured with custom parameters:

- `k1`: Controls term frequency saturation (default: 1.2)
- `b`: Controls document length normalization (default: 0.75)
- Field weights:
  - `title`: Weight for document title (default: 3.0)
  - `body`: Weight for document body (default: 1.0)
  - `description`: Weight for document description (default: 2.0)
  - `tags`: Weight for document tags (default: 2.5)

## Comparison with Other Scorers

- **vs. TitleScorer**: BM25F considers all document fields, not just the title, and applies sophisticated term weighting.
- **vs. TerraphimGraph**: BM25F uses statistical term weighting rather than graph-based relevance.
- **vs. BM25Plus**: BM25F handles multiple fields with different weights, while BM25+ focuses on improving the lower-bounding problem of the original BM25 algorithm.

## Example

When searching for "rust programming" across documents about programming languages, BM25F will:

1. Give higher weight to documents with "rust" and "programming" in the title
2. Consider matches in tags and description fields more important than body text
3. Apply length normalization to avoid bias toward longer documents
4. Produce a ranked list of documents based on their relevance to the query 