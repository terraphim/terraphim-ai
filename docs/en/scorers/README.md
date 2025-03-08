# Terraphim Scorers

Terraphim AI provides multiple scoring algorithms to rank search results based on different criteria. Each scorer has its own strengths and is suitable for different types of searches.

## Available Scorers

### TitleScorer

The TitleScorer is a simple scorer that ranks documents based on the similarity between the search query and the document title. It uses the Levenshtein distance to calculate similarity.

[Learn more about TitleScorer](./Title-Scorer.md)

### TerraphimGraph

The TerraphimGraph scorer uses Terraphim's unique graph embeddings to rank documents. The rank of a term is defined by the number of synonyms connected to the concept in the knowledge graph.

[Learn more about TerraphimGraph](./terraphim-graph.md)

### BM25F

BM25F is an extension of the BM25 algorithm that handles multiple fields with different weights for structured documents. It's particularly well-suited for documents with fields like title, body, description, and tags.

Key features:
- Weights different document fields differently (title, body, description, tags)
- Applies document length normalization
- Considers term frequency across all fields

[Learn more about BM25F](./bm25f.md)

### BM25+

BM25+ is an improved version of the BM25 algorithm that addresses the lower-bounding problem for term frequency normalization. It provides more accurate relevance scoring, especially for short documents and rare terms.

Key features:
- Adds a small constant (delta) to ensure a lower bound on term frequency
- Handles rare terms better than standard BM25
- Provides more accurate scoring for documents of varying lengths

[Learn more about BM25+](./bm25-plus.md)

## Choosing a Scorer

- **TitleScorer**: Use when you want to prioritize matches in document titles.
- **TerraphimGraph**: Use when you want to leverage Terraphim's knowledge graph for ranking.
- **BM25F**: Use when you have structured documents with multiple fields and want to weight them differently.
- **BM25+**: Use when you need better handling of rare terms and documents of varying lengths.

## Rescoring Results from Multiple Haystacks

Terraphim AI supports rescoring results from multiple haystacks (data sources) using any of the available scorers. This allows you to:

1. Search across multiple data sources
2. Combine the results
3. Apply a consistent ranking algorithm to the combined results

This is particularly useful when you have data spread across different repositories or when you want to compare results from different sources using the same ranking criteria. 