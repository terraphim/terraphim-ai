# Title-Scorer

Title scorer is a simple scorer for the "Default" role that matches documents by applying Levenshtein distance to document titles. This is the default scoring method for basic document ranking.

## Available Scoring Methods

### Similarity-Based Scoring (Default)
- **Levenshtein Distance**: Measures edit distance between query and document title
- **Jaro Distance**: String similarity based on character transpositions
- **Jaro-Winkler Distance**: Enhanced Jaro distance with prefix bonus

### BM25-Based Scoring (New)
Terraphim now supports multiple BM25 variants for more sophisticated document ranking:

- **BM25**: Standard Okapi BM25 ranking function
- **BM25F**: Fielded BM25 with different weights for title, body, description, tags
- **BM25Plus**: Enhanced BM25 with additional parameters for fine-tuning
- **TFIDF**: Traditional Term Frequency-Inverse Document Frequency
- **Jaccard**: Similarity-based scoring using term overlap
- **QueryRatio**: Ratio of query terms matched in document

## Configuration

You can select different scoring methods via the `QueryScorer` enum:

```json
{
  "relevance_function": "title-scorer",
  "query_scorer": "bm25f"
}
```

## Usage

The title scorer is automatically used when `RelevanceFunction::TitleScorer` is selected in your role configuration. The system will:

1. Index all documents using the selected scoring method
2. Apply the chosen algorithm to rank search results
3. Return documents sorted by relevance score

## When to Use

- **Similarity-based scoring**: For simple title matching and fuzzy search
- **BM25 variants**: For more sophisticated document ranking that considers term frequency and document structure
- **BM25F**: When you want to emphasize certain document fields (title, body, description, tags)
- **TFIDF**: For simple, interpretable scoring without length bias
- **Jaccard/QueryRatio**: When you care about term presence rather than frequency

For detailed information about BM25 scorers, see [BM25 Scorers](./bm25-scorers.md).
