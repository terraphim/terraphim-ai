# BM25+ Scorer

BM25+ is an improved version of the BM25 ranking function that addresses the lower-bounding problem for term frequency normalization. It provides more accurate relevance scoring, especially for short documents and rare terms.

## Overview

BM25+ enhances the classic BM25 algorithm by adding a small constant (delta) to the term frequency component. This modification ensures that even rare terms contribute meaningfully to the relevance score, addressing a known limitation in the original BM25 algorithm.

## How It Works

BM25+ works by:

1. Calculating term frequencies in the document body
2. Applying document length normalization
3. Computing the Inverse Document Frequency (IDF) for each query term
4. Adding a small constant (delta) to ensure a lower bound on term frequency
5. Combining these factors to produce a final relevance score

The formula for BM25+ is:

```
score(D,Q) = ∑(t in Q) IDF(t) * ((tf(t,D) * (K1 + 1)) / (K1 * length_norm + tf(t,D)) + delta)
```

Where:
- `D` is a document
- `Q` is the query
- `t` is a term in the query
- `IDF(t)` is the inverse document frequency of term `t`
- `tf(t,D)` is the term frequency of `t` in document `D`
- `K1` is a parameter that controls term frequency saturation (default: 1.2)
- `length_norm` is the document length normalization factor
- `delta` is a small constant that ensures a lower bound (default: 1.0)

## When to Use BM25+

BM25+ is ideal for:

- General-purpose document search where accuracy is important
- Collections with varying document lengths
- When rare terms should contribute meaningfully to relevance
- When you want to improve upon basic BM25 without the complexity of field weighting

## Configuration

The BM25+ scorer can be configured with custom parameters:

- `k1`: Controls term frequency saturation (default: 1.2)
- `b`: Controls document length normalization (default: 0.75)
- `delta`: Controls the lower bound for term frequency contribution (default: 1.0)

## Comparison with Other Scorers

- **vs. TitleScorer**: BM25+ considers the entire document content, not just the title, and applies sophisticated term weighting.
- **vs. TerraphimGraph**: BM25+ uses statistical term weighting rather than graph-based relevance.
- **vs. BM25F**: BM25+ focuses on improving the lower-bounding problem of the original BM25 algorithm, while BM25F handles multiple fields with different weights.

## Example

When searching for "rust programming" across documents about programming languages, BM25+ will:

1. Calculate the frequency of "rust" and "programming" in each document
2. Apply length normalization to avoid bias toward longer documents
3. Ensure that even rare occurrences of terms contribute to the relevance score
4. Produce a ranked list of documents based on their relevance to the query

## Technical Details

The delta parameter in BM25+ addresses a theoretical issue with the original BM25 algorithm where the contribution of a term can be arbitrarily close to zero when the term frequency is low and the document is long. By adding a small constant, BM25+ ensures that every matching term contributes at least a minimum amount to the final score. 